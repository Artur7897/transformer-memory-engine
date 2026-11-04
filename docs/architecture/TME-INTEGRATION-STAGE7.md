# TME Integration – Stage 7 (2026-11-04)

## Scope

This patch is incremental **on top of Stage 6** and the uploaded repository snapshot.
It does not independently replace any previous patches or mutate persistent states.

- `src/workflow.rs`: transactional `workflow_feedback` combines an existing
  persisted acquisition feedback step with a bounded Reflection assessment.
  It returns an external-lookup *candidate*, not an authorization or tool call.
- `workflow_sleep_pass`: one transactional SLEEP pass acknowledges only
  deterministic, pending operation counters using the shared maintenance
  implementation; it closes the pass but **does not** delete tokens, merge
  sources, decide semantic truth or suppress unresolved findings.
- `src/acquisition_session.rs`: fixes a hand-off bug. An exhausted internal
  search with `ExternalSourceCandidate` stays open so an authorized source
  hint can reactivate it. `Complete` explicitly closes the session.
- Unit tests target atomic failure, no unintended token writes, feedback
  expansion, SLEEP gating and external-hint reactivation after hand-off.

## Non-goals / still open

This is not an end-to-end Qwen runner. It does not implement ToolGateway,
external API retrieval, automatic deduplication, semantic Abstraction review,
or automatic scheduling. The TME never executes untrusted tools. LLM-derived
feedback is not proof of correctness. Reviewer-approved Abstract commits are
in Stage 4/6, with originals retained. The lifecycle still relies on Soul
for actual LLM process management.

## Validation status

Source-level edits only; cargo and rustc are unavailable in the authoring
container. **Compilation and tests must be performed on an isolated copy
on the user's Workspace Mini before merging.** Never run this against Lucy's
runtime or existing Academy states.
