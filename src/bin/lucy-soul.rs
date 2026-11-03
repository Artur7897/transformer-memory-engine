use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process,
};

use transformer_memory_engine::state_container;

const ENTITY_ID: &str = "A_26_99_01";
const ENTITY_SERIAL: &str = "A269901";
const RUNTIME_ID: &str = "7a2e4023-85c8-42c3-aadf-b66160cba4e6";
const STATE_ID: &str = "a1aae0c4-5da4-4c40-8ed1-ebc419916988";
const IDENTITY_INSCRIPTION: &str =
    "b01ff162629af23372efda371754cb7201cdcb18b48e4bd60b1545a70812ab91i0";

const STATE_FILENAME: &str = "a1aae0c4-5da4-4c40-8ed1-ebc419916988.tme";

fn discover(dir: &Path, depth: usize, found: &mut BTreeSet<PathBuf>) {
    if depth > 7 {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        let Ok(kind) = entry.file_type() else {
            continue;
        };

        // Symlinks are not followed during discovery.
        if kind.is_symlink() {
            continue;
        }

        if kind.is_file() {
            if path.file_name().and_then(|s| s.to_str()) == Some(STATE_FILENAME) {
                if let Ok(canonical) = fs::canonicalize(&path) {
                    found.insert(canonical);
                }
            }
        } else if kind.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if matches!(name.as_ref(), ".git" | "target" | "node_modules" | ".next") {
                continue;
            }

            discover(&path, depth + 1, found);
        }
    }
}

fn run() -> Result<(), String> {
    println!("OrdiFi Soul Runtime V1");
    println!("Entity: {}", ENTITY_ID);
    println!("Runtime ID: {}", RUNTIME_ID);
    println!("TME State ID: {}", STATE_ID);
    println!("Identity Inscription: {}", IDENTITY_INSCRIPTION);
    println!();

    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "--identity") {
        return Ok(());
    }

    if args.iter().any(|a| a == "--help") {
        println!("Usage:");
        println!("  A_26_99_01.runtime");
        println!("  A_26_99_01.runtime --identity");
        println!("  A_26_99_01.runtime --root DIRECTORY");
        return Ok(());
    }

    let exe = env::current_exe().map_err(|e| format!("Executable location: {e}"))?;

    let exe_dir = exe.parent().ok_or("Executable has no parent directory")?;

    let mut roots = Vec::new();

    // Search relative to the current location of the binary.
    roots.push(exe_dir.to_path_buf());

    if let Some(parent) = exe_dir.parent() {
        roots.push(parent.to_path_buf());
    }

    // Additional storage roots can be supplied at runtime.
    // No absolute workspace path is embedded in this binary.
    let mut i = 0;

    while i < args.len() {
        if args[i] == "--root" {
            let value = args.get(i + 1).ok_or("--root requires a directory")?;

            roots.push(PathBuf::from(value));
            i += 2;
        } else {
            return Err(format!("Unknown argument: {}", args[i]));
        }
    }

    let mut candidates = BTreeSet::new();

    for root in &roots {
        discover(root, 0, &mut candidates);
    }

    if candidates.is_empty() {
        return Err("TME State not found. Supply --root DIRECTORY.".into());
    }

    let mut valid = Vec::new();

    for path in &candidates {
        match state_container::load(path, STATE_ID, ENTITY_SERIAL) {
            Ok(memory) => {
                valid.push((path.clone(), memory));
            }
            Err(err) => {
                eprintln!("Rejected state candidate {}: {}", path.display(), err);
            }
        }
    }

    if valid.len() != 1 {
        return Err(format!(
            "Expected exactly one valid TME State, found {}",
            valid.len()
        ));
    }

    let (path, memory) = valid.remove(0);

    println!("TME State verified.");
    println!("State location: {}", path.display());
    println!("Memory tokens: {}", memory.states.len());
    println!("Relations: {}", memory.relations.len());
    println!("Cycles: {}", memory.cycles);
    println!();
    println!("Lucy Soul: identity and memory binding verified.");

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("SOUL ERROR: {error}");
        process::exit(1);
    }
}
