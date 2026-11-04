# Stage 11 – Lifecycle and coordinated feedback bridge

This stage builds on Stage 10 and changes only `src/bin/text-ingestion-bridge.rs`.

## Scope

Added CLI commands using the existing state container (same five positional arguments after executable):

- `lifecycle-status`: read-only, final argument unused (pass `-`).
- `maintenance-request`: marks maintenance pending; final argument unused (pass `-`).
- `standby`: JSON `{"active_tasks":0}`. Only ACTIVE with zero tasks can transition.
- `sleep-begin`: JSON `{"llm_inactive":true}`. Soul/host must independently establish that the LLM is actually inactive; TME cannot verify external process state.
- `sleep-pass`: JSON `{"budget":20}`. Calls existing `workflow_sleep_pass` atomically via state-container update; only acknowledges deterministic operations, retains unresolved findings and returns to STANDBY.
- `wake`: returns to ACTIVE; final argument unused (`-`). The host must synchronize any in-flight work.
- `workflow-feedback`: JSON `{"session_id":"s1","feedback":"insufficient","signal":"routine","assessment_budget":20}`. Calls existing `workflow_feedback` atomically and returns the new acquisition batch plus Reflection maintenance findings. Requires ACTIVE.

### Intentional limits

This is *not* a scheduler, an LLM shutdown mechanism, an autonomous tool gateway, or a semantic merge engine. `sleep-pass` does not delete duplicate tokens; it acknowledges only validated pending operation counters. Abstraction requires a separate reviewed commit while ACTIVE.

## Suggested isolated validation on the Workspace Mini

Do not overwrite the original checkout or production state. Unpack to a temporary directory, inspect, then run:

```bash
cargo check --bin text-ingestion-bridge
cargo test --lib
cargo test --test clockwork_dynamics
```

For an end-to-end lifecycle smoke test, use an **isolated newly created** `.tme` file; do not reuse Lucy or Academy state. In case of a compilation failure, retain the complete diagnostic output for targeted corrections.

## Status

Source and archive have been prepared in this environment. No Rust toolchain was available here, so Cargo checks and runtime verification have not been performed.
