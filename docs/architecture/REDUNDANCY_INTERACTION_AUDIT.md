# TME — Redundancy and Interaction Audit

Status: Preliminary source-based audit.
No new policy, coupling or dimension is authorized.

## 1. Existing numeric state

Activation:
- Stored in TokenDynamics.
- Operational activity, not factual reliability.
- Independent dimension candidate: retain.

Stability:
- Stored in TokenDynamics.
- Experimental caller-maintained value.
- Not empirically inferred from retention history.
- Investigate derived rather than separately
  maintained representations.

Relation weight:
- Stored on directed StateToken relations.
- Modified by reinforce and global decay.
- Not equivalent to activation or evidence quality.

Operation counters:
- Track reported processing operations.
- Not a memory quality or importance estimate.

## 2. Existing experimental coupling

Use:
  activation_after =
    activation + (1 - activation) * use_gain

ConsolidationReview:
  activation_after =
    activation * (1 - review_decay * (1 - stability))

Both currently produce read-only previews.

Question:
Does retention stability legitimately control
activation reduction, or should these properties
only meet in a later operational decision?

Do not activate this coupling before testing.

## 3. Retrieval

Attention uses semantic vector similarity.

RetrievalPriorityPreview shows activation alongside
similarity without changing rank.

Potential feedback:
retrieval -> activation -> priority -> retrieval.

Test retrieval interference and relevance loss
before introducing combined ranking.

## 4. Consolidation and decay

ConsolidationPreview combines activation and
descriptive retention observations.

It does not authorize persistence, deletion,
structural change or automatic Sleep execution.

Current global relation decay has no scoped
dependency-aware preservation policy.

Do not connect global decay directly to Sleep.

## 5. Redundancy candidates

- Stability stored independently of retention data.
- Multiple analyses traversing the same evidence.
- Separate traversal of relationship structures
  where shared indexed access may suffice.

None is confirmed redundant yet.

## 6. Shared hierarchical processing

Use existing identities and typed relations where
their semantics are sufficient.

Apply the same hierarchy to:
- Anchored retrieval
- Local Reflect analysis
- Bounded Sleep processing
- Higher-level consolidation of completed sections

A finished lower-level result can be reused only
while its input versions and dependencies remain valid.

## 7. Performance and correctness requirements

- Avoid repeated full scans in routine operations.
- Separate task scope, batch size and commit boundary.
- Preserve evidence provenance across consolidation.
- Prevent double-counting derived evidence.
- Support interruption and valid checkpoint recovery.
- Require termination for finite stable workloads.
- Measure CPU, RAM, storage and retrieval quality.

## 8. Next validation

Build an integrated deterministic dynamics simulator.

Compare:
- Independent vs directly coupled dimensions
- Stored vs derived stability
- Semantic-only vs combined retrieval
- Local vs global consolidation
- Hierarchical vs full-scan processing

Evaluate quality and resource costs together.

Use Parameter Calibration only for changes to
control parameters, thresholds and coupling rules.
