# TME Clockwork — Architecture & implementation status

**Status:** architecture draft agreed in discussion; technical status verified against source excerpts and test output supplied 2026-11-03.  
**Scope:** Transformer Memory Engine (TME), distinct from Soul (agent runtime), LLM, Library, Policy, ToolGateway and Signer.  
**Terminology:** Parameter calibration (*Parameterkalibrierung*), **not** LLM fine-tuning.  
**Rule:** Do not claim a planned mechanism is implemented merely because a preview, module or unit test exists.

## 1. Core principle

TME stores individual experience, semantic memories, meaningful skill steps and their relations. It is not a wordpiece tokenizer or a replacement for the LLM's general knowledge. Semantic units can be short statements or coherent longer passages, whichever best preserves meaning. A persistent anchor organizes associated facts, steps, stories and versions without requiring all details to be activated simultaneously.

**Experience is not necessarily truth.** Separate the result observed, the explanation hypothesized, the evidence supporting each claim, source provenance and the ability to verify it again.

## 2. Token kinds and anchors

Existing Rust `TokenKind`: `State`, `Memory`, `Skill`, `Llm` (in `src/token_architecture.rs`).

- **State:** represents/evaluates knowledge states; e.g. known / uncertain / unknown as conceptual evidence states. Do not assume all three are already discrete persisted Rust token values.
- **Memory:** individual experiences, noteworthy findings, historical claims and their time/version context.
- **Skill:** one meaningful reusable logical action step. A complete ability is a graph/sequence of related steps, often with a named skill anchor. Being stored or described does **not** equal demonstrated executable competence.
- **Llm:** model-related contextual links/hints useful for a particular model to execute a skill; model-specific preparation can be learned from verified outcomes. This is architectural intent, not a verified end-to-end feature.

**Anchor is a property, not a fifth token kind.** E.g. `BCE verwenden`, `Bitcoin Inscription erstellen`, `Frank in Dubai`. Anchor identity may persist with high inertia while linked detail memories adapt independently. The anchor flag itself does not raise inertia automatically; token-specific inertia must be assigned/calibrated.

Tokens may be reused across multiple anchors/runbooks, and conflicting claims must keep provenance/version boundaries. No forced deduplication of different events or versions.

## 3. Clockwork dynamics

`TokenInertia` has **three independent dimensions** (`activation`, `stability`, `retention`); there is also `relation_inertia` per relation. Existing helper:

```text
new_value = current + (target - current) * (1 - inertia)
```

Inertia is **resistance to change**, not truth, confidence, evidence strength, or storage tier. `inertia.stability` and `inertia.retention` already work in separate update paths. Avoid double-damping the same update. Activation should still be able to respond promptly to a current task; a stable anchor must not be impossible to retrieve.

### Memory level (persistent-state-derived, not task activation)

For `TokenKind::Memory` with an explicit retention estimate:

```text
score = (stability + retention) / 2
L1  if score >= 0.80
L2  if 0.40 <= score < 0.80
L3  if score < 0.40
```

Scores are continuous and unrounded for comparison. Zero is currently included in L3 by the Rust implementation; any deletion/garbage-collection policy is **separate and not implemented by this classification**. A missing retention estimate gives **no determined level**, not an invented value.

L3 may receive additional inertia on **actual updates**:

```text
effective_inertia = base + (1 - base) * l3_bonus
```

The current patch defaults `l3_bonus = 0` for backward compatibility, and makes it configurable for experiments. This protects against rapid mutation but must not make verified corrections or revival impossible.

**Important:** existing `learning_b::ActivationTier` (L1/L2/L3) is **query-relative graph traversal / activation**, and must never be conflated with the memory-level score. Skill tokens have **no persistent MemoryLevel**; a skill's L1/L2/L3 refers only to the query-relative *connection paths* from the selected skill anchor, not a whole tier of skills to load. A necessary step must not be skipped because it is reached through L3.

## 4. Higher mechanisms and responsibility boundaries

| Mechanism | Function / caution |
|---|---|
| Evidence | Whether an assertion is verified, uncertain, unknown; does not follow automatically from stability or popularity |
| Provenance | Original source, time, version, independence and ability to reacquire information |
| Reflection | Fast, bounded check; identify gaps, contradictions, wrong skill paths; label or queue unresolved items |
| Neubeschaffung (reacquisition) | Explicit retrieval action through Soul and available skills, then evidence validation; not a passive TME lookup alone |
| Compression | Consider safe semantic deduplication/condensation; do not erase incompatible versions or evidence |
| Abstraction | Derive higher-order, reusable relations and patterns, with their evidentiary limits |
| SLEEP | State-triggered, heavier review and consolidation; **not a wall-clock alarm or proven automated loop** |

Suggested cycle:

```text
Observed event / detected gap
  -> Evidence + lightweight Reflection
  -> provenance check
  -> (if material) Soul chooses an authorized retrieval skill
  -> evidence/validity check
  -> later state-triggered SLEEP: review / compression / abstraction
  -> qualified updates through existing token/relation inertia
```

A hallucinated fill or creative hypothesis may be allowed **only in inconsequential contexts** and must never silently be promoted to verified fact. Wallet addresses, transaction inputs, outputs, inscription IDs, payment destinations and other irreversible operations require independent current verification and policy/signing authorization. Even a known, frequently successful skill is not a substitute for checking the live system.

## 5. Reacquisition is learnable

A named capability anchor **`Information beschaffen`** should support reuse across domains. Initial/discoverable skill steps:

1. Recognize and report a gap: *Ich weiß es nicht* (state/decision, not a verified retrieved fact).
2. Search existing **Library** for capabilities, runbooks and source references.
3. Recover a missing step via graph traversal to L2/L3-linked steps; distinguish **missing token** from **retrieval failure**.
4. Ask an authorized known entity (e.g. Ada for technical analysis, Lucy for coordination); treat the response as a sourced assertion, not independent verification.
5. Browse a suitable current official source or query API documentation through an authorized tool; potentially consult other LLMs but verify important facts independently.
6. Check evidence, provenance, time validity, result and consequences. Strengthen reusable routes when verified successful; retain failure reasons.

These are **design decisions** until wired to real Library/Soul APIs. No secrets in prompts. Policy/ToolGateway/Signer enforce authorization externally to TME.

## 6. Inactivity reviews and calibration — NOT YET IMPLEMENTED

No use should **not automatically** cause ongoing weight change. A **qualified state-triggered review** may propose a small negative retention adjustment if there is relevant new inactivity/evidence context. Illustrative proposals (not hard-coded facts or final default values):

| Evidence status | Illustrative review proposal |
|---|---:|
| Unknown | −0.020 |
| Uncertain | −0.015 |
| Verified | −0.010 |

For ephemeral trader market observations, a much larger magnitude (e.g. −0.5 *before* inertia damping) is an experimental ceiling, **not** a blanket rule on all experiences. A verified historical event can remain valuable for years; verification of profit does not prove the historical trading rationale.

`delta_effective = proposed_delta * (1 - effective_inertia)` (only when using an additive-event convention; for target-based APIs convert to a target and damp exactly once). Re-running SLEEP on identical evidence must not repeatedly apply the same inactivity penalty. Evidence-state changes and irreversible safety checks are not delayed by inertia. Do not hand-edit individual memory weights to force benchmark goals; calibrate behavior through policy rates and inertia.

## 7. Observed implementation / baseline evidence

Source excerpts show:

- `TokenKind::{State,Memory,Skill,Llm}`, and `TokenInertia::{activation,stability,retention}` plus `relation_inertia`.
- `apply_stability_target` uses inertia; `apply_retention_event` requires explicit estimate, validates a minimum time gap and uses the retention inertia.
- Clockwork patch adds persistent anchor metadata, derived `MemoryLevel`, optional L3 inertia bonus, and validation. Existing state metadata remains backward compatible by default.
- 2026-11-03: `cargo test --lib` **218/218 pass**; `tests/clockwork_dynamics.rs` **4/4 pass**. These counts are from user-provided terminal output, not a test rerun in this document-generation environment.

Clockwork scenario observations:

| Scenario | Observed result |
|---|---|
| 70 inscription recall events over 14 days | 13 qualified gaps; stability stayed 0.01, retention rose 0.75 → 0.86496; derived L3 → L2 |
| One explicit L3 reactivation | Stability 0.01 → 0.366; derived L3 → L2 |
| Ordinary vs high-inertia anchor | Ordinary 0.78/0.86 (L1); anchor 0.36/0.335 (L3) after compared event |
| 180 days without events | Token remained 0.85/0.85 (L1): **no passive forgetting implemented** |

Separate **synthetic** capacity simulation: 10,950 logical records (30/day × 365); assumed weekly evidence-dependent compaction yielded 5,302 compacted and about **32.9% modelled storage reduction**. This is **not** a measurement of TME RAM, disk, correctness, compression or real retrieval quality. Modelled retention of logical record count does not certify information preservation.

## 8. Next implementation sequence

1. **Inventory read-only**: identify actual Library indexing/search and capability/runbook contracts; Soul/Rust/Node entrypoints, permissions, existing source timestamps and metadata. No duplicate search stack.
2. Define minimal **Library lookup** capability contract and distinguish `found`, `ambiguous`, `not_found`, `source_outdated`, `unauthorized`, and `tool_error`.
3. Wire **one** Soul-executable reacquisition route for the Academy in isolation, first with an explicit local Library fixture and an end-to-end verified result.
4. Preserve source/provenance and verified vs uncertain output; no automatic truth promotion by LLM response.
5. Test failed lookup, wrong related skill step (inscribe vs mint), L2/L3 traversal, correct retrieval, and repeat reuse.
6. Only afterwards implement qualified Evidence-weighted inactivity review/SLEEP and semantic compression/abstraction commits, with idempotency safeguards and regression tests.

## 9. Safety and non-goals

- Do not mutate or reset existing Academy-v1, Academy-v2 or CHIMERA state files.
- Do not overwrite untracked `src/bin/academy-tme-bridge.rs` or `src/bin/qwen-tme-bridge.rs`.
- No unverified claim that SLEEP, persistent compression, adaptive semantic ingestion, multi-agent communication or browser capability is operational.
- No production wallet/private-key handling in Academy tests; current authorized signer and ToolGateway checks are mandatory for real transactions.
- No promise that high inertia implies permanent truth; evidence correction must remain available.
