use std::{env, error::Error, path::PathBuf};

use transformer_memory_engine::{
    experience::EvidenceSource, state_container, token_architecture::TokenKind, Memory, StateToken,
};

const SERIAL: &str = "QWENTEST";

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        return Err(
            "Usage: qwen-tme-bridge <init|record|recall> <state-dir> <state-id> [args]".into(),
        );
    }

    let command = args[1].as_str();
    let directory = PathBuf::from(&args[2]);
    let state_id = &args[3];
    let path = directory.join(format!("{state_id}.tme"));

    match command {
        "init" => {
            if args.len() != 4 {
                return Err("init requires exactly 2 arguments".into());
            }

            let mut memory = Memory::new();

            for level in 1..=5 {
                let id = format!("door-level-{level}");

                memory.insert(StateToken {
                    id: id.clone(),
                    vector: vec![level as f32, 1.0],
                })?;

                memory.set_token_kind(&id, TokenKind::Memory)?;
            }

            state_container::create(&path, state_id, SERIAL, &memory)?;

            println!("CREATED {state_id}");
            println!("TOKENS {}", memory.states.len());
            println!("EXPERIENCES 0");
        }

        "record" => {
            if args.len() != 7 {
                return Err("record requires level, choice, outcome".into());
            }

            let level: usize = args[4].parse()?;
            let choice = args[5].as_str();
            let outcome = args[6].as_str();

            if !(1..=5).contains(&level)
                || !matches!(choice, "L" | "R")
                || !matches!(outcome, "open" | "closed")
            {
                return Err("Invalid door event".into());
            }

            let token_id = format!("door-level-{level}");

            state_container::update(&path, state_id, SERIAL, |memory| {
                let next = memory
                    .experiences
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| std::io::Error::other("Experience counter overflow"))?;

                let event_id = format!("door-event-{next:08}");

                memory
                    .begin_experience(&event_id, &token_id, choice)
                    .map_err(std::io::Error::other)?;

                memory
                    .complete_experience(
                        &event_id,
                        outcome,
                        EvidenceSource::VerifiedToolResult,
                        Some("two-doors-controller"),
                    )
                    .map_err(std::io::Error::other)?;

                Ok(())
            })?;

            println!("RECORDED level={level} choice={choice} outcome={outcome}");
        }

        "recall" => {
            if args.len() != 5 {
                return Err("recall requires level".into());
            }

            let level: usize = args[4].parse()?;

            if !(1..=5).contains(&level) {
                return Err("Invalid level".into());
            }

            let memory = state_container::load(&path, state_id, SERIAL)?;

            let token_id = format!("door-level-{level}");
            let mut count = 0usize;

            for experience in memory.experiences.values() {
                if experience.state_id != token_id {
                    continue;
                }

                if let Some(outcome) = &experience.observed {
                    println!("choice={} outcome={}", experience.expected, outcome);
                    count += 1;
                }
            }

            if count == 0 {
                println!("NO_MEMORY");
            }
        }

        _ => return Err("Unknown command".into()),
    }

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("BRIDGE ERROR: {error}");
        std::process::exit(1);
    }
}
