use std::{env, fs, path::PathBuf, process};
use transformer_memory_engine::{state_container, Memory};

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        return Err(
            "Usage: chimera-state-init <state-directory> <entity-serial> <state-uuid>".into(),
        );
    }

    let directory = PathBuf::from(&args[1]);
    let serial = &args[2];
    let state_id = &args[3];

    if !directory.is_dir() {
        return Err("State directory does not exist".into());
    }

    // Never initialize twice or silently replace an existing memory.
    for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().extension().is_some_and(|e| e == "tme") {
            return Err(format!(
                "Existing TME state found: {}",
                entry.path().display()
            ));
        }
    }

    let path = directory.join(format!("{state_id}.tme"));

    state_container::create(&path, state_id, serial, &Memory::new()).map_err(|e| e.to_string())?;

    state_container::load(&path, state_id, serial)
        .map_err(|e| format!("Created state failed verification: {e}"))?;

    println!("Entity: {serial}");
    println!("State ID: {state_id}");
    println!("Verified file: {}", path.display());

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ERROR: {e}");
        process::exit(1);
    }
}
