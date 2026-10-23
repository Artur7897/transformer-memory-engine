# TME — State-Driven Sleep Architecture

Status: TARGET DEFINED / IMPLEMENTATION IN PROGRESS

## Architectural principle

TME uses a state-driven internal processing rhythm.

Sleep is an autonomous memory consolidation mode,
not a timer, scheduled task, or fixed operation counter.

No independent language acquisition in the initial TME.

## Sleep objective

Sleep selectively reorganizes existing memory by combining:

- Reflect: inspect evidence, conflicts and inconsistencies.
- Abstract: derive reusable patterns from experiences.
- Compress: reduce redundancy while preserving information.
- Slice: separate ambiguous or overloaded states.
- Adaptive Decay: update activation and stability according
  to operation results, not elapsed time.

Newly inferred relationships remain hypotheses until validated.

Original evidence and provenance must remain recoverable.

## Trigger objective

Sleep is initiated by internal consolidation demand,
potentially including:

- unresolved contradictions,
- excessive redundancy,
- unintegrated experiences,
- competing state representations,
- accumulated uncertainty or instability.

Trigger rules must be experimentally validated.

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
- [ ] M10 — Autonomous state-driven Sleep trigger
- [ ] M11 — Long-horizon tests and model independence

M01/M02 are basic implementations, not complete
validation of the final memory architecture.

## Completion criteria

Sleep is not complete until:

1. Its trigger depends on internal state.
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
