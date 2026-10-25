# TME — Reflect and Idle-Driven Sleep Roadmap

Status: TARGET DEFINED / IMPLEMENTATION IN PROGRESS

## Architectural principle

Reflect performs event-driven, resource-bounded analysis
during agent activity.

Sleep runs during idle periods when relevant memory
or calibration revisions require examination.
It is not driven by a timer, a fixed operation counter,
or a separate consolidation-demand threshold.
Sleep preparation is interruptible and has no fixed deadline.

No independent language acquisition in the initial TME.

## Sleep objective

Reflect identifies local findings and consolidation candidates.
Sleep verifies those findings and coordinates shared mechanisms,
including:
- Abstract: derive reusable patterns from experiences.
- Compress: reduce redundancy while preserving information.
- Slice: separate ambiguous or overloaded states.
- Adaptive Decay: update activation and stability according
  to operation results, not elapsed time.

Newly inferred relationships remain hypotheses until validated.

Original evidence and provenance must remain recoverable.

## Scheduling and revision objective

The agent runtime identifies idle periods.
Sleep may begin when memory or calibration revisions
have not yet been examined.

Reflect records local findings during active operation.
A completed no-change analysis is still recorded as examined.
Unchanged revisions must not cause endless reanalysis.

New agent work interrupts Sleep preparation.
Pending changes require revision validation before commit.

## Engineering requirements

- Model-independent memory state.
- Deterministic and reproducible tests.
- Auditable transformations.
- Consistent state snapshots.
- Safe rollback of failed consolidation.
- No fabricated relationships.
- No mandatory wall-clock trigger.
- Clear separation of observations and hypotheses.

## Development milestones

- [x] M01 — Persistent numerical state-token core
- [x] M02 — Positive/negative transition evidence
- [ ] M03 — State metrics and consolidation demand
- [ ] M04 — Reflect
- [ ] M05 — Abstract
- [ ] M06 — Compress
- [ ] M07 — Slice
- [ ] M08 — Operation-driven adaptive decay
- [ ] M09 — Integrated deterministic consolidation
- [ ] M10 — Event-driven Reflect and revision tracking
- [ ] M11 — Interruptible idle-driven Sleep
- [ ] M12 — Atomic consolidation and recovery
- [ ] M13 — Calibration simulation and long-horizon tests

M01/M02 are basic implementations, not complete
validation of the final memory architecture.

## Completion criteria

Sleep is not complete until:

1. Its execution respects idle availability and revisions.
2. It can coordinate the defined memory operations.
3. It preserves source evidence and provenance.
4. It can generate and distinguish tentative hypotheses.
5. Consolidation is reproducible and recoverable.
6. Long-horizon testing demonstrates measurable benefits
   without unacceptable information loss.

## Research constraints

A decline in memory size or improved retrieval score alone
does not prove genuine learning or autonomous cognition.

Any such claims require controlled comparative evaluation.

## Deferred

- Independent language learning
- Production integration with Lucy
- Claims of LLM-free autonomous intelligence
