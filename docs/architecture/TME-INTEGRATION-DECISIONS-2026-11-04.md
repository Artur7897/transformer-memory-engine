# TME Integration Decisions — 2026-11-04

Status: **Target architecture, implementation incomplete**. This document records the latest user decisions. It does not claim that all mechanisms are implemented or validated.

## Scope and invariants

The TME is a persistent, LLM-independent memory engine. The external Soul/Agent Runtime owns policy, tool access, and execution. Parameter Calibration changes control rules and thresholds, not neural weights. Graph activation L1/L2/L3 is task-relative and **not** the persistent memory-level classification. A memory can be accessible without being loaded in an LLM context window. Skills must remain independently retrievable: semantic compression of prose must not silently delete executable capability steps.

## Acquisition and feedback

Internal **Beschaffung** and external **Neubeschaffung** pursue the same informational objective. Internal retrieval uses Learning-B graph traversal, semantic relevance, previously delivered tokens, bounded budgets and feedback. External reacquisition is requested via an authorized runtime tool; the TME must not make unauthorized I/O itself. Neither source is unconditionally prioritized: acquisition cost, freshness, provenance, availability and task relevance matter.

Triggers: `insufficient`, `incorrect`, `contradictory`, `execution_error`; `sufficient` means that the caller reports enough context, not proof of correctness. Execution errors require operational state checks before retrying side-effecting tools. Further reachable graph relations indicate `expansion_available`, not `expansion_required`. Distinguish source exhaustion from budget exhaustion.

External evidence can retrigger internal search with updated vectors and entity/term cues. Reacquired information may initially create duplicate memory tokens. Retain provenance and do not count repeated references to one source as independent corroboration.

## Shared Reflection / SLEEP system

Reflection and SLEEP use one assessment and consolidation kernel with different execution profiles, rather than separate duplicate algorithms. Reflection is event-driven, near-real-time and usually locally bounded. SLEEP is state-driven, idle-compatible, resumable and usually broader/deeper, but not necessarily intrinsically more authoritative. Urgent events never wait for SLEEP. Both may perform **provably unambiguous** reversible or fully evidenced actions and may request Abstraction.

**Abstraction** is the component responsible for content-aware summarization/semantic merge and may collaborate with an LLM during active operation. It must preserve material details and source provenance, avoid converting hypotheses into facts, and provide a verifiable proposal before commit. SLEEP does **not** decide textual meaning on its own. When the LLM is disabled, semantically dependent tasks may be queued for the active runtime or handled by an explicitly authorized on-demand model invocation.

## Recovery and consistency

Write operations require a snapshot/revision check, complete invariant validation and atomic persistence. An active task can preempt SLEEP; partial work must be resumable and never create half-applied transitions. Keep prior Academy and Lucy states untouched during development. Avoid overlapping feedback loops that repeatedly strengthen duplicate evidence or inflate confidence; Activation, Stability and Retention remain distinct dimensions.

## Current code audit (source archive received 2026-11-04)

- `src/learning_b.rs`, `learning_b_weighted.rs`, `learning_b_semantic.rs`: graph activation and scoring previews exist, but no task-feedback-controlled end-to-end retrieval loop.
- `src/bin/text-ingestion-bridge.rs`: existing read-only `search`, persistent `create` and `ingest`, and ordered `recall` existed.
- `src/text_ingestion.rs`: text segmentation records and exact same-ID reingestion work; cross-ID semantic duplication and text Abstraction are **not** implemented.
- `src/reflect.rs`: primarily read-only state metrics, pending operations and conflicting transition indicators.
- `src/abstraction.rs`: generates evidence-bound contextual comparison candidates, not compressed text memories.
- `src/consolidation_preview.rs`: descriptive review-only; does not authorize structural commits.
- `docs/architecture/CONSOLIDATION_ARCHITECTURE.md`: older design disallows Reflection structural commits and says SLEEP must not mutate during analysis. This remains a conservative safety invariant for preparation. Latest design permits clearly validated, atomic commits by either mode after preparation, but this does **not** justify semantically ambiguous merges.
- `docs/architecture/HIERARCHICAL_SLEEP_EXECUTION.md`: describes pending resumable task/checkpoint semantics; not an implemented scheduler.

## Integration implementation sequence — no Academy performance tuning yet

1. Introduce a bounded feedback acquisition API reusing existing Learning-B previews. *Prototype included in patch; not yet compiler-tested in this environment.*
2. Connect source selection and external runtime requests, evidence provenance and query reactivation. Do not let TME perform external I/O directly.
3. Add shared Reflection/SLEEP assessment core, clear action permissions and lifecycle state signals.
4. Implement reversible deterministic consolidation and validated cross-ID duplicate handling with provenance.
5. Provide LLM-assisted Abstraction proposal/validation/commit protocol, and text lineage preserving original evidence.
6. Add revision-aware atomic SleepTask batching and restart handling; preserve existing state container semantics.
7. Only after integration, run Rust unit/integration tests, then end-to-end Academy/Qwen tests and controlled baselines.

## Explicit non-claims

The prototype below does not yet implement external source selection, automated semantic correctness judgments, duplicate consolidation, Abstraction execution, sleep checkpointing, or a functioning full acquisition loop around Qwen. No new formulas or independent memory weight dimensions are calibrated here.

## Integration patch 2: implemented interfaces (not yet runtime-verified)

- `src/acquisition.rs`: read-only internal retrieval escalation with feedback and a seen-token set; exposed through the existing text bridge as `acquire`.
- `src/maintenance.rs`: shared Reflection/SLEEP assessment, urgent signal and caller-provided inspection budget; no unsafe deletion.
- `src/text_abstraction.rs`: proposal validator for LLM-assisted text Abstraction; refuses Skill/anchor sources; does **not** commit or claim summary accuracy.

**Still outstanding:** external reacquisition provider orchestration, persistent retry/feedback sessions, duplicate graph rewiring and atomic merge, validated semantic Abstraction commits, resumable SLEEP checkpoints, production Soul wiring. These are explicit gaps, not completed functionality.
