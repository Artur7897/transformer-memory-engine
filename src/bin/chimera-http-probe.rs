use std::{
    collections::BTreeMap,
    env,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use transformer_memory_engine::{
    experience::EvidenceSource,
    learning_b_events::{ExecutionEvent, ExecutionOutcome},
    Memory, StateToken,
};

const URL: &str = "http://127.0.0.1:18765/";
const REQUEST: &str = "http.request.completed";
const VERIFY: &str = "http.response.verified";

fn now_ms() -> Result<i64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;

    i64::try_from(duration.as_millis()).map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    let task = env::args()
        .nth(1)
        .ok_or("Usage: chimera-http-probe TASK_ID")?;

    if task.trim().is_empty() {
        return Err("Task ID cannot be empty".into());
    }

    let memory_path =
        env::var("CHIMERA_PROBE_MEMORY").map_err(|_| "CHIMERA_PROBE_MEMORY is not set")?;

    let path = Path::new(&memory_path);

    let mut memory = if path.exists() {
        Memory::load(path).map_err(|e| e.to_string())?
    } else {
        Memory::new()
    };

    for id in [REQUEST, VERIFY] {
        if !memory.states.contains_key(id) {
            memory.insert(StateToken {
                id: id.into(),
                vector: vec![1.0, 0.0],
            })?;
        }
    }

    // Activate previously observed experience before execution.
    let previous = memory.learning_b_experience_activation(REQUEST, now_ms()?)?;

    println!("Task: {task}");
    println!("Previous observed neighbors: {}", previous.neighbors.len());

    for neighbor in &previous.neighbors {
        println!(
            "  {} | observations={} | tasks={}",
            neighbor.state_id, neighbor.observation_count, neighbor.task_count
        );
    }

    // Actual tool execution. No simulated success event.
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--max-time",
            "5",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}",
            URL,
        ])
        .output()
        .map_err(|e| format!("Cannot execute curl: {e}"))?;

    let request_time = now_ms()?;

    if !output.status.success() {
        return Err(format!(
            "HTTP request failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let http_code = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;

    if http_code.trim() != "200" {
        return Err(format!("Unexpected HTTP status: {http_code}"));
    }

    // The Rust runtime has verified the actual tool result.
    let verify_time = now_ms()?;

    let events = vec![
        ExecutionEvent {
            id: format!("{task}/request"),
            task_id: task.clone(),
            state_id: REQUEST.into(),
            outcome: ExecutionOutcome::Success,
            source: EvidenceSource::VerifiedToolResult,
            reference: format!("{task}/curl/http-200"),
        },
        ExecutionEvent {
            id: format!("{task}/verify"),
            task_id: task.clone(),
            state_id: VERIFY.into(),
            outcome: ExecutionOutcome::Success,
            source: EvidenceSource::VerifiedToolResult,
            reference: format!("{task}/runtime/status-verified"),
        },
    ];

    let times = BTreeMap::from([
        (events[0].id.clone(), request_time),
        (events[1].id.clone(), verify_time),
    ]);

    let report = memory.learning_b_record_execution_evidence(&events, &times, now_ms()?)?;

    memory.save(path).map_err(|e| e.to_string())?;

    println!("HTTP status: {http_code}");
    println!("New evidence records: {}", report.recorded_ids.len());
    println!(
        "Total evidence records: {}",
        memory.contextual_evidence.len()
    );
    println!("Memory persisted: {}", path.display());

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("CHIMERA probe error: {error}");
        std::process::exit(1);
    }
}
