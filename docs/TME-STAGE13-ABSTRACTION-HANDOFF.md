# Stage 13 – Reflection/SLEEP → Abstraction handoff

`Memory::workflow_abstraction_handoffs(mode, signal, budget)` is a read-only
selection layer built on `maintenance_assess`. Both modes use the same code.
It exposes exact-content + same-source duplicate groups with their original
source records. It **does not** generate prose summaries, merge tokens, assign
truth, alter weights, or authorize commits. The caller may separately prepare
`TextAbstractionRequest` (with an LLM-proposed text), run `abstraction-preview`,
secure independent review, then use digest-bound `abstraction-commit` in ACTIVE.

The CLI command `abstraction-handoffs` requires a JSON file containing
`{ "mode":"reflection", "signal":"routine", "budget":100 }` for ACTIVE
or `mode:"sleep"` for SLEEP. It does not alter the state container.

Candidate discovery can be truncated by the maintenance finding budget. A
nonempty `findings_total` does not indicate that every finding is an Abstraction
candidate. Other findings retain their original responsibilities.

This is source-only and has not been Cargo-compiled in this environment.
