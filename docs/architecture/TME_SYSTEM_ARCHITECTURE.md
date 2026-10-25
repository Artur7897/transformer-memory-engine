# TME — Integrated System Architecture

Status: Architecture draft.
Not an implementation or validated dynamics model.

## 1. Objective

Define the cooperation of existing TME mechanisms before
introducing additional states, weights or control rules.

The architecture must remain usable on ordinary hardware
and support long-term operation with growing memory.

## 2. Architectural layers

### Memory Data Model

Existing foundations:
- StateToken identities and vectors
- Relations and relation weights
- Contextual evidence and provenance
- Evidence dependencies
- Experiences and retention events

Identifiers, relationships and indexes are not
automatically adaptive weight dimensions.

### Memory Dynamics

Existing experimental dimensions:
- Activation
- Retention stability
- Relation weights
- Operation counters

Retention stability is not yet an empirically
validated estimate derived from recall history.

A derived evaluation should not be persisted as a
separate independent dimension without justification.

### Analysis and decision operations

Existing:
- Attention
- Reflect preview
- Hypothesis candidates
- Abstraction and validation
- Evidence quality and dependency analysis
- Retention analysis
- Compression and slice previews
- Consolidation preview

These operations have different responsibilities.
Most are read-only and do not implement autonomous
consolidation.

### Storage and execution

Existing:
- JSON persistence
- Structural integrity validation
- Retention ID index

Planned:
- Selective indexed retrieval
- Incremental persistence
- Versioned execution state
- Atomic consolidation commits

## 3. Shared hierarchical structure

The same identities, relations and dependency indexes
may support several different operations.

Retrieval:
- Identify relevant anchors.
- Follow appropriate associative relationships.
- Load additional details when needed.

Reflect:
- Process locally relevant changes.
- Inspect affected memory and evidence.
- Expand the scope only when dependencies require it.

Sleep:
- Examine selected subgraphs or a global scope.
- Reuse valid lower-level consolidation results.
- Coordinate changes across dependency boundaries.

Associative relevance and structural dependency are
not necessarily interchangeable relationship types.

## 4. Persistent Hierarchical Sleep

Sleep is a persistent task rather than one uninterrupted
execution session.

Terms:
- Task: a defined consolidation objective and scope.
- Session: an opportunity to perform work.
- Batch: a bounded unit of analysis.
- Commit group: changes that must be applied atomically.
- Checkpoint: durable progress with version references.

A task may span multiple sessions and checkpoints.

The system should:
- Pause at safe boundaries.
- Resume from valid checkpoints.
- Process memory hierarchically.
- Consolidate completed A and B into higher-level A+B.
- Invalidate dependent results after relevant changes.
- Avoid repeating unaffected completed work.
- Complete finite tasks under stable prerequisites.

A global consolidation can use small batches.
Global scope does not imply loading or writing
the complete memory simultaneously.

A completed task is not a claim that memory will
never require further consolidation.

## 5. Coupling categories

Classify every proposed interaction as:

- Data reference or structural relation
- Shared input to an operational decision
- Directed influence on another weight
- Feedback or recurrent influence
- Safety or integrity constraint

Do not introduce a new numeric dimension merely
because an operation uses existing information.

## 6. Candidate interactions to investigate

Activation + Retrieval:
- Risk: frequently retrieved memories dominate.

Retention History + Stability:
- Determine whether stability should be derived
  rather than separately maintained.

Evidence + Hypothesis + Validation:
- Reuse existing provenance and validation reports.
- Avoid treating descriptive agreement as proof.

Dependencies + Sleep:
- Determine safe impact boundaries and invalidation.
- Avoid circular reinforcement of derived evidence.

Slice + Retrieval + Tiering:
- Reuse stable identities and provenance.
- Prevent loss of retrieval anchors.

Compress + Sleep:
- Preserve evidence provenance and exactness.
- Distinguish physical compression from grouping.

## 7. Potentially undesirable behavior

Investigate:
- Self-reinforcing retrieval dominance
- Accidental loss of dormant but valuable memories
- Stale hypotheses treated as verified facts
- Double-counting derived or copied evidence
- Cross-slice identity confusion
- Nonterminating consolidation due to new inputs
- Repeated full scans on ordinary events
- Stale checkpoints after dependent memory changes

## 8. Implementation boundaries

TME is not an LLM.

Agent adapters interpret external observations.
TME maintains structured memory and its operations.

Policy revisions are Parameter Calibration,
not LLM fine-tuning.

Normal adaptive weight changes are runtime behavior,
not Parameter Calibration.

No automatic irreversible deletion, global Sleep
execution or validated combined retrieval policy
is implied by this document.

## 9. Next engineering stages

1. Complete redundancy and interaction audit.
2. Specify coupling invariants and dependencies.
3. Specify transaction-safe hierarchical Sleep.
4. Implement an integrated deterministic simulator.
5. Test long-term memory quality and resource costs.
6. Calibrate control parameters using test results.
7. Implement only demonstrated missing capabilities.
