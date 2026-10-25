# TME — Hierarchical Sleep Execution

Status: Architecture specification.
Not implemented or empirically validated.

## 1. Objective

Provide thorough, interruptible and resource-bounded
memory consolidation on ordinary hardware.

Sleep is a persistent task, not a time-limited session.

A complete consolidation may span multiple sessions,
batches, checkpoints and hierarchy levels.

## 2. Execution concepts

SleepTask:
- Defined objective and memory scope.
- Stable initial revision or snapshot.
- Explicit completion criteria.
- Persisted execution progress.

SleepSession:
- One available interval for processing.
- May end without finishing the task.
- Must not invalidate completed work.

Batch:
- Bounded processing unit.
- Selected by memory hierarchy and dependencies.
- Limited by configurable resource budgets.

CommitGroup:
- Mutations requiring atomic application.
- May contain multiple batches or sections.
- Must preserve global integrity invariants.

Checkpoint:
- Durable progress after a valid commit boundary.
- Records relevant input and dependency versions.
- Allows safe restart and selective invalidation.

These execution concepts are not adaptive memory
weights or additional semantic dimensions.

## 3. Hierarchical consolidation

Lower levels:
- Inspect individual memory fragments and relations.
- Validate provenance and dependencies.
- Prepare locally consistent results.

Intermediate levels:
- Combine previously consolidated sections.
- Evaluate cross-section relationships.
- Reuse validated lower-level outputs.

Global level:
- Verify consistency across the declared scope.
- Evaluate remaining cross-boundary dependencies.
- Complete the task when obligations are satisfied.

A+B consolidation must not automatically treat
summaries as independent new evidence.

Deeper records remain accessible when required
for correct cross-section decisions.

## 4. Scheduling and interruption

Sleep operates during available idle capacity.

It should:
- Pause at safe processing boundaries.
- Prioritize active-agent responsiveness.
- Resume from the latest valid checkpoint.
- Avoid unnecessary repetition.
- Respect configured CPU and memory budgets.

Interruption is normal, not a task failure.

A task must make measurable progress under
stable prerequisites and sufficient runtime.

## 5. Snapshot and revision semantics

A task operates against a defined initial view.

New agent observations remain recordable while
Sleep is running.

Before commit:
- Revalidate relevant inputs and dependencies.
- Detect changes affecting proposed mutations.
- Invalidate or recompute affected results.
- Avoid committing stale conclusions.

A completed task covers its declared baseline.
Later observations may require new incremental work.

## 6. Atomicity and persistence

Current Memory::save serializes the entire memory
as JSON and is not incremental record persistence.

The planned execution layer requires:
- Durable task and checkpoint records.
- Atomic commit of related memory mutations.
- Crash-safe recovery.
- Idempotent replay or guarded reapplication.
- Selective persistence of modified records.
- Storage integrity verification.

Do not simulate incremental storage by repeatedly
rewriting the full JSON database after every batch.

Storage engine selection remains an open decision.

## 7. Dependency handling

Reuse existing evidence dependencies and stable
identities wherever their semantics are sufficient.

Associative links and strict change dependencies
must remain distinguishable.

Commit impact assessment must account for:
- Modified input records.
- Downstream derived results.
- Cross-section constraints.
- Previously consolidated ancestor sections.

Avoid creating parallel dependency graphs unless
a genuinely different relationship is required.

## 8. Resource optimization

Scope, batch size and commit boundary are separate.

A global task may use small bounded batches.

Potential techniques:
- Incremental traversal.
- Indexed subgraph selection.
- Reuse of version-valid intermediate results.
- Selective dirty-section propagation.
- Bounded working sets.
- Batched storage commits.

Each optimization requires correctness and
performance validation.

## 9. Required invariants

- No partial externally visible commit group.
- No accepted result with stale dependencies.
- No loss of original evidence provenance.
- No double counting of derived evidence.
- No implicit promotion of hypotheses to facts.
- No irreversible deletion from weak evidence.
- No fabricated checkpoint completion.
- No infinite restart loop under stable inputs.

## 10. Evaluation scenarios

Test:
- Repeated pause and resume.
- Crash between analysis and commit.
- Crash during durable checkpoint writing.
- New observations during an active Sleep task.
- Cross-section invalidation of A+B.
- Reuse of unaffected consolidated sections.
- Deep-memory retrieval during consolidation.
- Large multi-year memory populations.
- Completion after many separate sessions.
- CPU, RAM, storage writes and latency.

Compare hierarchical incremental execution
against simpler full-scan baselines.

## 11. Open decisions

- Physical storage backend.
- Version granularity and snapshot design.
- Commit group determination.
- Checkpoint frequency.
- Scheduler fairness and budgets.
- Cross-section dependency indexing.
- Exact progress and completion metrics.

These decisions require implementation tests,
not only architectural assumptions.
