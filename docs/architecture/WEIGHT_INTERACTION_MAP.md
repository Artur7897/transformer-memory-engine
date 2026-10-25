# TME — Weight Interaction Map

Status: Architecture proposal v1.
No calibrated interaction coefficients are defined.

## 1. Purpose

Define how memory dimensions participate in adaptive
memory management without inventing dependencies.

TME is not a Large Language Model.
Its memory weights are not neural model parameters.

The system distinguishes:

1. Observed data and immutable historical evidence.
2. Derived estimates of memory properties.
3. Operational state and retrieval priorities.
4. Consolidation decisions.
5. Versioned control parameters.

These layers must not be treated as interchangeable.

## 2. Memory dimensions

### Activation

Represents recent or ongoing operational activation.

It does not establish truth, semantic validity,
long-term retention or episodic significance.

Current status: experimental stored dynamic value.

### Retention Stability

Represents evidence concerning reliable retrieval
over separated recall opportunities.

Successful recalls, failed attempts and temporal gaps
are relevant observations.

The absence of recall attempts is not retrieval failure.

Current status: experimental stability value plus
retention history and descriptive analysis.
No validated automatic stability estimator exists.

### Semantic Stability

Represents the durability of a proposition's meaning
or validity over time and context.

Historical facts, changing current attributes and
conditional relationships must be distinguished.

A fact becoming outdated as a current value does not
make its timestamped historical observation false.

Current status: proposed dimension.

### Evidence Quality

Represents the support for a proposition, including
provenance, independence, conflicts and verification.

Frequent repetition is not independent confirmation.
Storage priority cannot manufacture evidence quality.

Current status: partial provenance and dependency
analysis. No universal quality score is implemented.

### Episodic Significance

Represents contextual significance of an experience,
including important first occurrences and rare events.

Significance is not equivalent to frequency or truth.

Current status: proposed dimension.

### Novelty / Habituation

Describes whether an experience is distinctive
relative to previously observed experiences.

Repeated similar experiences may reduce novelty,
without invalidating the earlier experiences.

Current status: proposed dimension.

### Contextual Relevance

Represents usefulness for a current task or context.

It can change without changing an item's historical
truth, significance or underlying provenance.

Current status: partially supported by contextual
evidence and retrieval facilities; not fully modeled
as an independent calibrated weight.

## 3. Interaction categories

### Independent derivation

A dimension is estimated from its own relevant
observations without rewriting other dimensions.

Default for all dimensions.

### Joint decision

Multiple dimensions inform one consolidation
decision but do not directly modify one another.

Preferred when it produces the required behavior.

### Direct coupling

Changing one dimension influences another through
an explicit directed control rule.

Requires separate justification and testing.
No direct coupling is authorized by this document.

## 4. Proposed joint decisions

### Active retrieval priority

Possible inputs:
- Activation
- Contextual relevance
- Retention observations
- Task-specific importance

Truth and evidence quality must remain separate
from mere likelihood of retrieval.

Status: hypothesis; decision rule not calibrated.

### Archival and preservation

Possible inputs:
- Episodic significance
- Evidence quality
- Provenance dependencies
- Historical reconstructability
- Current contextual relevance
- Storage cost

Low activation alone cannot authorize deletion.

Status: proposed decision; deletion policy undefined.

### Semantic consolidation

Possible inputs:
- Repeated relational evidence
- Temporal changes in attributes
- Conflicts and independent sources
- Provenance of derived abstractions

An aggregate or approximate representation must
remain distinguishable from original evidence.

Status: proposed decision; semantic aggregation
not implemented by current exact-match compression.

### Information reacquisition

Possible inputs:
- Current relevance
- Consequence risk
- Evidence gaps
- Temporal uncertainty
- Cost of reacquisition

A high-consequence unknown should be reported
as unknown or referred to an external agent.

TME does not retrieve external information itself.

Status: proposed decision; agent signaling not complete.

## 5. Prohibited implicit influences

- Activation must not establish factual correctness.
- Repetition must not imply independent evidence.
- Episodic significance must not fabricate retention.
- Retention stability must not imply semantic validity.
- Age alone must not imply falsity.
- Low retrieval priority must not authorize deletion.
- Compression must not silently upgrade evidence quality.
- An inferred approximation must not become a verified fact.
- A Sleep proposal must not overwrite newer memory revisions.
- Parameter Calibration must not directly rewrite
  stored memory weights.

## 6. Reflect and Sleep responsibilities

Reflect:
- Runs lightweight, event-driven checks.
- Detects anomalies and potential consolidation needs.
- Records revision-linked findings.
- Can signal consequential information gaps to an agent.
- Does not commit coupled structural weight changes.

Sleep:
- Runs during idle periods on a consistent snapshot.
- Reviews applicable dimensions and dependencies.
- Prepares coordinated changes under a versioned policy.
- Validates invariants and revision compatibility.
- May perform no changes.
- Commits only a complete validated state.

## 7. Calibration requirements

Calibration modifies control parameters, thresholds,
bounds and approved coupling rules.

It is not LLM Fine-Tuning.

Calibration must be:
- Versioned.
- Reproducible.
- Tested against multiple scenarios.
- Evaluated for feedback stability.
- Robust to reasonable parameter perturbations.
- Compared against simpler baselines.

No numeric coefficient is justified solely because it
produces a desired result in one example.

## 8. Evaluation scenarios

At minimum, test:

1. Frequent short-interval recalls.
2. Successful recalls after long gaps.
3. Rare but consequential episodes.
4. Many repetitive low-significance experiences.
5. Changing attributes with valid historical values.
6. Contradictory evidence from dependent sources.
7. Missing details with low consequence.
8. Missing information with high consequence.
9. Long inactive periods without recall attempts.
10. Interrupted Sleep with intervening memory updates.

Measure memory quality, information loss,
integrity, latency, storage and calibration robustness.

## 9. Next engineering steps

1. Define machine-readable dimension identifiers.
2. Specify read-only joint decision previews.
3. Build a deterministic calibration simulation harness.
4. Compare independent and joint-decision baselines.
5. Justify any direct coupling before implementation.
6. Implement revision-aware Sleep planning only after
   dependency and consistency rules are validated.

No autonomous weight reconciliation, arbitrary deletion
or full Sleep commit is implemented by this specification.
