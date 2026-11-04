# Stage 8 — external handoff and active-state boundary

This overlay is cumulative: it contains the prior Stage 7 overlay plus Stage 8 changes, applied to the user's original 78-file source archive. It is **not yet compiled**.

## Changes

- `src/external_acquisition.rs`: explicit, read-only external handoff requiring prior `ExternalSourceCandidate`; allowed source classes `library`, `runbook`, `api`, `other_entity`, `web`. Soul/ToolGateway performs authorization, scheduling, real-world source selection, and any external I/O. This module **does not** retrieve external information or verify facts.
- `src/acquisition_session.rs`: mutating acquisition sessions (start, feedback, hint, close) now require `ACTIVE` to prevent bridge users from bypassing the workflow lifecycle. Read-only access remains possible.
- `src/bin/text-ingestion-bridge.rs`: new `external-handoff` command reads JSON with `session_id` and `allowed_source_kinds`, prints a typed request; no persistent write.
- Unit tests for source handoff and forbidden mutations in standby/sleep.

## Not implemented / not asserted

- No actual authenticated external tool execution, source-policy router, or Semantic LLM evaluator.
- No fully automatic Abstraction and redundancy resolution; no evidence of correctness from duplicate text.
- No claim that a full TME end-to-end test or Rust compilation has passed.
- Session changes remain backward-compatible through the default maps in the prior overlays; test on **isolated temp states** first.

## Deploy and verification, only when approved

1. Copy the patch to an isolated checkout of the original source.
2. Run `cargo test --lib`, `cargo check --bins` and the existing clockwork tests.
3. Verify state-container load/save on a synthetic state, not Lucy/Academy.

