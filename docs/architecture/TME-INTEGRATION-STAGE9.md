# TME Integration Stage 9 — activation-depth correctness

## Fixed

`Memory::acquisition_preview` now selects the shallowest **still-unseen** task-relative graph depth on negative feedback, and sends at most `batch_size` ranked results from that depth. Previously, after seeing any depth-1 result, the code opened depth 2 even while there were more unserved depth-1 tokens.

This is an activation-policy fix, not a change to the persistent Memory L1/L2/L3 classification. Semantic scores remain unchanged.

## Added regression tests

- `feedback_exhausts_l1_before_opening_l2`: two depth-1 candidates are delivered before depth 2, then external lookup becomes eligible only when all reachable candidates are exhausted.
- `missing_l1_candidates_are_not_skipped_after_relevance_changes`: changing relevance does not skip an as-yet-unseen direct neighbor.

## Explicit limitations

- User-described behavior ultimately needs a sufficiency signal: a plausible LLM output is not evidence of correctness.
- Full integrated Qwen/Soul controller, semantic Abstraction review, automatic duplicate consolidation and production-ready SLEEP execution remain pending.
- ToolGateway belongs outside TME.
- Stage 9 has **not** been compiled or tested against Cargo in this environment. Run on an isolated copy; do not overwrite user runtime or Academy states.

## Test invocation on a safe, separate working copy

`cargo test --lib` and `cargo test --test clockwork_dynamics`.
