use std::{env, error::Error, io, path::PathBuf};

use transformer_memory_engine::{
    experience::EvidenceSource, state_container, token_architecture::TokenKind, Memory, StateToken,
};

const SERIAL: &str = "ACADEMY";

fn main() {
    if let Err(e) = run() {
        eprintln!("ACADEMY ERROR: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        return Err("Usage: academy-tme-bridge \
             <init|record|recall|inspect> <state-dir> <uuid> [args]"
            .into());
    }

    let command = args[1].as_str();
    let directory = PathBuf::from(&args[2]);
    let state_id = &args[3];

    let path = directory.join(format!("{state_id}.tme"));

    match command {
        "init" => {
            if args.len() != 4 {
                return Err("init: unexpected arguments".into());
            }

            if path.exists() {
                return Err("State already exists".into());
            }

            let mut memory = Memory::new();

            let bootstrap = "academy-bootstrap";

            memory.insert(StateToken {
                id: bootstrap.into(),
                vector: vec![1.0, 0.0],
            })?;

            memory.set_token_kind(bootstrap, TokenKind::Skill)?;

            state_container::create(&path, state_id, SERIAL, &memory)?;

            println!("ACADEMY_STATE_CREATED {state_id}");
        }

        "record" => {
            if args.len() != 8 {
                return Err("record requires task, expected, observed, evidence-ref".into());
            }

            let task = &args[4];
            let expected = &args[5];
            let observed = &args[6];
            let reference = &args[7];

            if task.is_empty()
                || !task
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                || expected.trim().is_empty()
                || observed.trim().is_empty()
                || reference.trim().is_empty()
            {
                return Err("Invalid academy event".into());
            }

            let token_id = format!("task-{task}");

            state_container::update(&path, state_id, SERIAL, |memory| {
                if !memory.states.contains_key(&token_id) {
                    memory
                        .insert(StateToken {
                            id: token_id.clone(),
                            vector: vec![0.0, 1.0],
                        })
                        .map_err(io::Error::other)?;

                    memory
                        .set_token_kind(&token_id, TokenKind::Memory)
                        .map_err(io::Error::other)?;
                }

                let next = memory
                    .experiences
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("Experience counter overflow"))?;

                let event_id = format!("academy-event-{next:08}");

                memory
                    .begin_experience(&event_id, &token_id, expected)
                    .map_err(io::Error::other)?;

                memory
                    .complete_experience(
                        &event_id,
                        observed,
                        EvidenceSource::VerifiedToolResult,
                        Some(reference),
                    )
                    .map_err(io::Error::other)?;

                Ok(())
            })?;

            println!("RECORDED task={task}");
        }

        "recall" => {
            if args.len() != 5 {
                return Err("recall requires task".into());
            }

            let memory = state_container::load(&path, state_id, SERIAL)?;

            let token_id = format!("task-{}", args[4]);

            let mut found = false;

            for experience in memory.experiences.values() {
                if experience.state_id != token_id {
                    continue;
                }

                found = true;

                println!(
                    "event={} expected={} observed={} reference={}",
                    experience.id,
                    experience.expected,
                    experience.observed.as_deref().unwrap_or("PENDING"),
                    experience.reference.as_deref().unwrap_or("NONE"),
                );
            }

            if !found {
                println!("NO_MEMORY");
            }
        }

        "inspect" => {
            if args.len() != 4 {
                return Err("inspect: unexpected arguments".into());
            }

            let memory = state_container::load(&path, state_id, SERIAL)?;

            println!("TOKENS {}", memory.states.len());
            println!("EXPERIENCES {}", memory.experiences.len());
            println!("PENDING {}", memory.pending_experience_count());
        }

        _ => return Err("Unknown command".into()),
    }

    Ok(())
}
