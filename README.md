# TME — Transformer Memory Engine

> **Entity survives model replacement while preserving continuity.**

TME is an experimental persistent memory architecture for long-lived AI entities.

The central hypothesis is that an entity's continuity does not need to reside inside a specific language model. The language model can remain a replaceable reasoning component while persistent memory, state and identity are maintained externally.

## Core hypothesis

A long-lived entity should be able to:

- preserve relevant state across sessions,
- accumulate and restructure memory over time,
- compress older experience without losing essential information,
- selectively expose memory through attention,
- replace its underlying language model,
- and continue operating without resetting its accumulated continuity.

The model is a component of the entity — not the entity itself.

## Research goals

TME will investigate:

- persistent state representation,
- state tokens,
- memory attention,
- memory compression and abstraction,
- retrieval and relevance selection,
- contradiction handling,
- long-horizon continuity,
- model replacement,
- compute and token efficiency.

## Evaluation

TME is intended to be evaluated experimentally rather than by subjective demonstrations alone.

Planned comparisons include:

- LLM + conventional context
- LLM + RAG
- TME-backed entity

Candidate metrics include:

- retrieval accuracy,
- contradiction rate,
- continuity after model replacement,
- long-horizon task performance,
- token consumption,
- inference cost,
- memory growth,
- compression loss.

## Status

Early research and architecture phase.

No claim is currently made that the central hypothesis has been demonstrated.

## Related work

TME is being developed as part of research into persistent modular entities used by the OrdiFi project, but the memory engine itself is intended to remain independently usable.
