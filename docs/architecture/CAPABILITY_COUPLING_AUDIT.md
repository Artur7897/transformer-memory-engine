# TME — Existing Capability and Coupling Audit

Status: Preliminary implementation audit.
No new weights or coupling rules are authorized.

## 1. Objective

Reuse existing TME capabilities before introducing
new dimensions, stored states or mechanisms.

Distinguish structural validity, evidence support,
hypothesis status and operational consequences.

## 2. Verified implementation capabilities

### Hypotheses
Module: hypotheses.rs

- Derives read-only candidates from mixed transitions.
- Labels candidates as unverified.
- Does not persist or revise hypotheses.
- Does not establish explanatory truth.

### Validation
Module: validation.rs

- Compares abstraction candidates with later evidence.
- Supports same, opposite, absent and insufficient
  outcome comparisons.
- Does not establish general factual verification.

### Evidence dependencies
Module: evidence_dependencies.rs

- Records explicit CopiedFrom and DerivedFrom links.
- Traverses recorded ancestors.
- Prevents declared dependency cycles.
- Does not prove that declared links are true.
- Does not automatically reconcile dependent hypotheses.

### Experience
Module: experience.rs

- Separates expectation from observed outcome.
- Preserves recorded source classification.
- Rejects silent replacement of completed outcomes.
- Does not implement importance or persistence policy.

### Contextual evidence
Module: contextual_evidence.rs

- Records timestamps, source, reference and context.
- Prevents duplicate evidence identifiers.
- Supports time-bounded observation access.
- Does not normalize arbitrary external observations.

### Evidence quality and graph integrity

- Source-reference repetition can be detected.
- Graph structure and dependency times are validated.
- Neither establishes factual truth or independence.

### Slice
Module: slice.rs

- Selects evidence identifiers by state, context and time.
- Is deterministic and read-only.
- Does not create persistent memory fragments.
- Does not implement indexed deep-memory retrieval.

## 3. Candidate reuse before expansion

Reversible inference:
- Reuse hypothesis candidates, validation results,
  contextual evidence and dependency ancestry.
- Missing: persistent revision-aware inference lifecycle
  and downstream impact reconciliation.

Associative recall:
- Reuse state IDs, relations and contextual selection.
- Missing: normalized entity resolution, typed
  retrieval anchors and selective deep retrieval.

Information reacquisition:
- Reuse evidence gaps and agent-facing reports.
- Missing: a deduplicated information-need lifecycle
  and a defined agent handoff protocol.

Intent-dependent preservation:
- Reuse state identity and explicit relations where
  the semantics are sufficient.
- Missing: a reliable representation of open and
  completed obligations and their dependencies.

Memory tiering:
- Reuse stable identifiers and evidence references.
- Missing: indexed logical tiers, reversible movement
  and safe physical storage boundaries.

## 4. Coupling review rules

Before adding a new dimension, establish:

1. A unique property not adequately represented.
2. Its required inputs and evidence basis.
3. Why existing mechanisms cannot express it.
4. Which operations consume or modify it.
5. Direct and indirect coupling effects.
6. Invariants and possible feedback instability.
7. CPU, RAM, storage and retrieval costs.
8. A deterministic test that distinguishes the
   proposed solution from a simpler alternative.

## 5. Important semantic boundaries

- Structural graph validity is not factual truth.
- Provenance does not automatically prove reliability.
- No observed failure is not proof of perfect retention.
- High activation is not high evidence quality.
- Approximate recall is not verified recollection.
- Information exactness is not permanent importance.
- An inactive memory is not necessarily forgotten.
- A new observation does not automatically justify
  structural weight changes during active operation.

## 6. Next phase

Build a complete operation-to-dimension interaction
matrix based on the existing implementation inventory.

Explicitly identify existing links, proposed couplings,
potential redundancies and genuinely missing behavior.

Do not begin integrated parameter calibration until
the functional interaction model can be evaluated
as a coherent system.
