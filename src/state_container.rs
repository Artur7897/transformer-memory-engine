//! CHIMERA binary state container v1.
//! Create-only initial implementation. Existing states are never overwritten.

use crate::Memory;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path};

const MAGIC: &[u8; 8] = b"CHIMTME1";
const VERSION: u32 = 1;
const HEADER_LEN: usize = 8 + 4 + 8 + 32;

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    state_id: String,
    entity_serial: String,
    memory: Memory,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn valid_uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit() && !b.is_ascii_uppercase()
            }
        })
}

fn validate_identity(path: &Path, state_id: &str, entity_serial: &str) -> io::Result<()> {
    if !valid_uuid(state_id) {
        return Err(invalid("Invalid state UUID"));
    }
    if entity_serial.is_empty()
        || !entity_serial
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("Invalid entity serial"));
    }
    let expected = format!("{state_id}.tme");
    if path.file_name().and_then(|s| s.to_str()) != Some(expected.as_str()) {
        return Err(invalid("TME filename does not match state ID"));
    }
    Ok(())
}

/// Creates a new .tme file, never overwriting an existing state.
/// The caller must provide a freshly assigned, stable state UUID.
pub fn create(path: &Path, state_id: &str, entity_serial: &str, memory: &Memory) -> io::Result<()> {
    validate_identity(path, state_id, entity_serial)?;

    let payload = Payload {
        state_id: state_id.to_owned(),
        entity_serial: entity_serial.to_owned(),
        memory: memory.clone(),
    };
    let body = serde_json::to_vec(&payload)?;
    let digest = Sha256::digest(&body);

    let mut bytes = Vec::with_capacity(HEADER_LEN + body.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&digest);
    bytes.extend_from_slice(&body);

    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Loads a container only if the filename, IDs, checksum and
/// existing CHIMERA memory validations all succeed.
pub fn load(
    path: &Path,
    expected_state_id: &str,
    expected_entity_serial: &str,
) -> io::Result<Memory> {
    validate_identity(path, expected_state_id, expected_entity_serial)?;

    let bytes = fs::read(path)?;
    if bytes.len() < HEADER_LEN {
        return Err(invalid("Truncated TME header"));
    }
    if &bytes[..8] != MAGIC {
        return Err(invalid("Unknown TME format"));
    }

    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if version != VERSION {
        return Err(invalid("Unsupported TME version"));
    }

    let body_len = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
    if body_len != (bytes.len() - HEADER_LEN) as u64 {
        return Err(invalid("Invalid TME payload length"));
    }

    let body = &bytes[HEADER_LEN..];
    let digest = Sha256::digest(body);
    if digest.as_slice() != &bytes[20..52] {
        return Err(invalid("TME checksum mismatch"));
    }

    let mut payload: Payload = serde_json::from_slice(body).map_err(|e| invalid(e.to_string()))?;

    if payload.state_id != expected_state_id || payload.entity_serial != expected_entity_serial {
        return Err(invalid("TME entity or state binding mismatch"));
    }

    payload.memory.prepare_retention_writes().map_err(invalid)?;
    payload.memory.validate_evidence_graph().map_err(invalid)?;

    Ok(payload.memory)
}

/// Atomically updates a validated state under an exclusive file lock.
/// The callback runs only after loading the latest persisted memory.
pub fn update<F>(path: &Path, state_id: &str, entity_serial: &str, change: F) -> io::Result<()>
where
    F: FnOnce(&mut Memory) -> io::Result<()>,
{
    use fs2::FileExt;

    validate_identity(path, state_id, entity_serial)?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Missing parent directory"))?;

    // This lock file must not be deleted while the state is in use.
    let lock_path = parent.join(format!(".{state_id}.lock"));
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_path)?;
    lock.lock_exclusive()?;

    let mut memory = load(path, state_id, entity_serial)?;
    change(&mut memory)?;

    memory.prepare_retention_writes().map_err(invalid)?;
    memory.validate_evidence_graph().map_err(invalid)?;

    // A same-filesystem staging directory lets us reuse create/load
    // while retaining the required <stateId>.tme filename.
    let staging = tempfile::Builder::new()
        .prefix(".chimera-stage-")
        .tempdir_in(parent)?;
    let staged_path = staging.path().join(format!("{state_id}.tme"));

    create(&staged_path, state_id, entity_serial, &memory)?;
    load(&staged_path, state_id, entity_serial)?;

    fs::rename(&staged_path, path)?;

    // Persist the directory entry on Unix filesystems.
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;

    // The lock remains held until this function returns.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateToken;

    const ID: &str = "7f83a2c9-1d4e-4b62-9a71-d508cf235b90";

    fn path() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "chimera-container-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!("{ID}.tme"))
    }

    #[test]
    fn update_roundtrip_preserves_identity() {
        let path = path();
        create(&path, ID, "TEST001", &Memory::new()).unwrap();

        update(&path, ID, "TEST001", |memory| {
            memory
                .insert(StateToken {
                    id: "learned".into(),
                    vector: vec![0.2, 0.7],
                })
                .map_err(invalid)
        })
        .unwrap();

        let restored = load(&path, ID, "TEST001").unwrap();
        assert!(restored.states.contains_key("learned"));
        assert_eq!(restored.states.len(), 1);
        assert!(load(&path, ID, "WRONG001").is_err());

        fs::remove_file(&path).unwrap();
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn failed_update_does_not_change_memory() {
        let path = path();
        create(&path, ID, "TEST001", &Memory::new()).unwrap();
        let before = fs::read(&path).unwrap();

        let result = update(&path, ID, "TEST001", |memory| {
            memory.cycles += 1;
            Err(invalid("Simulated failure"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(load(&path, ID, "TEST001").unwrap().cycles, 0);

        fs::remove_file(&path).unwrap();
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn concurrent_updates_preserve_all_changes() {
        let path = path();
        create(&path, ID, "TEST001", &Memory::new()).unwrap();

        let workers: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    update(&path, ID, "TEST001", |memory| {
                        memory.cycles += 1;
                        Ok(())
                    })
                })
            })
            .collect();

        for worker in workers {
            worker.join().unwrap().unwrap();
        }

        assert_eq!(load(&path, ID, "TEST001").unwrap().cycles, 8);

        fs::remove_file(&path).unwrap();
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn memory_roundtrip_and_binding() {
        let path = path();
        let mut memory = Memory::new();
        memory
            .insert(StateToken {
                id: "bitcoin".into(),
                vector: vec![0.5, 0.8],
            })
            .unwrap();

        create(&path, ID, "TEST001", &memory).unwrap();
        let restored = load(&path, ID, "TEST001").unwrap();

        assert_eq!(restored.states.len(), 1);
        assert!(restored.states.contains_key("bitcoin"));
        assert!(load(&path, ID, "WRONG001").is_err());
        assert!(create(&path, ID, "TEST001", &memory).is_err());

        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn corrupted_container_is_rejected() {
        let path = path();
        create(&path, ID, "TEST001", &Memory::new()).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(&path, bytes).unwrap();

        assert!(load(&path, ID, "TEST001").is_err());

        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
    }
}
