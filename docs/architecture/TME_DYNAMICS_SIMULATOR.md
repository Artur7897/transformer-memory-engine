# TME — Integrated Dynamics Simulator

Status: Experimental design specification.
No integrated simulator is implemented yet.

## 1. Purpose

Evaluate the interactions of TME memory mechanisms
as one evolving system over long operational periods.

Determine which couplings improve useful memory
behavior and which cause redundancy, instability,
retrieval interference or excessive resource cost.

The simulator must not silently modify production
memory or imply unvalidated policies are established.

## 2. Model boundaries

Core:
- Existing deterministic Rust memory operations.

Experimental models:
- Explicit and independently selectable policies.
- Alternative retrieval ranking strategies.
- Alternative consolidation and tiering rules.
- Alternative representations of retention stability.

Evaluation:
- Functional correctness.
- Recall precision and availability.
- Provenance and dependency integrity.
- CPU, RAM, storage and operation latency.

## 3. Deterministic event timeline

Replay timestamped synthetic scenarios including:
- Initial learning.
- Repeated use.
- Inactivity.
- Recall success and failure.
- Contradictory observations.
- Correction of hypotheses.
- Open and completed intentions.
- Associative cues and memory reactivation.
- Background consolidation opportunities.
- Sleep interruption and checkpoint recovery.
- Information loss and reacquisition requests.

All experimental runs must be reproducible.

## 4. Time horizons

Evaluate:
- Short-term sequences.
- Weeks and months.
- One year.
- Six-year sustained operation.

Simulated time is not real elapsed test time.

The timeline must include changing workloads,
new information and obsolete historical contexts.

## 5. Interaction experiments

Investigate:
- Independent versus directly coupled dimensions.
- Activation and retrieval feedback.
- Retention history versus stored stability.
- Semantic retrieval versus combined ranking.
- Hypotheses, validation and evidence dependencies.
- Anchored slicing and hierarchical access.
- Local and hierarchical Sleep.
- Versioned checkpoints and dependency invalidation.

A model that has not been implemented must be
marked as experimental, never as a core capability.

## 6. Baselines and ablations

Compare against:
- Simple indexed retrieval.
- Existing semantic-only attention.
- No consolidation.
- No activation influence.
- No direct stability-to-activation coupling.
- Local versus global processing.

Remove one mechanism or coupling at a time
to identify its actual contribution.

## 7. Evaluation criteria

Measure:
- Exact recall when required.
- Relevant candidate recall and precision.
- Retrieval interference.
- Recovery from contextual cues.
- Incorrect confidence or provenance claims.
- Preservation of unresolved obligations.
- Invalid or stale dependencies.
- Checkpoint and resume correctness.
- Per-operation latency.
- Peak working memory.
- Persistent storage size and write volume.

No single aggregate score defines success.

## 8. Performance methodology

Separate:
- Synthetic operation counts.
- CPU time.
- Wall-clock latency.
- Memory allocation and peak RSS.
- Storage reads and writes.
- Simulated history duration.

Use repeated benchmark runs and record hardware,
data size, configuration and variance.

Evaluate scalability rather than extrapolating
from a single small test.

## 9. Safety and consistency invariants

- Unverified hypotheses remain distinguishable.
- Copied evidence is not counted as independent.
- Critical exact values are never reconstructed
  probabilistically and represented as verified.
- Irreversible deletion requires validated policy.
- Sleep never commits stale dependent results.
- A completed checkpoint records actual progress.
- Finite stable Sleep tasks eventually complete.

## 10. Initial engineering milestones

1. Define typed scenario events.
2. Build deterministic replay runner.
3. Add a simple baseline and metric collector.
4. Run existing TME mechanisms without coupling.
5. Add isolated experimental policy variants.
6. Introduce multi-mechanism interaction cases.
7. Evaluate hierarchical Sleep execution models.
8. Run six-year simulated workloads.
9. Compare results and calibrate control parameters.

## 11. Open research questions

- Which memory quality metrics predict usefulness?
- Which existing dimensions are genuinely independent?
- Which couplings improve quality without instability?
- Can the hierarchy avoid significant full scans?
- Which Sleep checkpoint granularity is effective?
- Which workloads cause the worst interference?
- Which improvements justify implementation cost?
