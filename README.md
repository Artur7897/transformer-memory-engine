# TME — Transformer Memory Engine

<!-- TME_RESEARCH_PREFACE_START -->

## Research Preface
### Adaptation, Stability, and the Organization of Memory

#### Nature Does Not Optimize for Everything

Nature does not produce organisms with universally optimal
abilities. Evolution shapes organisms through environmental
pressures, available resources, inherited structures, and
the demands of survival and reproduction.

Different environments favor different adaptations.

A dog's extraordinary sense of smell and the human capacity
for fine manipulation, tool use, and complex reasoning
illustrate different specializations. Neither represents
universal superiority. Each reflects a history of adaptation
under particular biological and environmental constraints.

Physical conditions also shape what is possible. Energy
availability, atmospheric composition, climate, gravity,
and ecological relationships influence which biological
structures and behaviors can persist.

Evolution is not a conscious search for perfection.
It operates through variation, selection, historical
constraints, and contingency.

The principle relevant to this research is therefore
not maximum capability, but **functional adaptation
within environmental and resource constraints**.

#### Stability Is Not the Absence of Change

A living system must preserve essential functions while
continuously responding to its environment.

Some structures remain stable because their operation is
critical. Others retain flexibility, enabling adaptation
when circumstances change.

This creates a fundamental tension:

**Preserve what remains essential while retaining the
capacity to change what no longer serves the system.**

Artificial learning systems face a related challenge,
commonly described as the stability-plasticity dilemma.

Excessive stability can prevent learning.
Excessive plasticity can destroy previously acquired
competence.

For long-lived artificial entities, neither extreme
is desirable.

#### Memory Is a Resource Allocation Problem

No physical system has unlimited memory, computation,
or energy.

Retaining every experience at maximum detail, continuously
activating every representation, and repeatedly processing
the entire history would create increasing costs.

Over long periods, uncontrolled accumulation may eventually
undermine the continuity that memory was intended to support.

A useful memory architecture must therefore do more than
store information.

It must distinguish between essential knowledge, useful
experience, redundant representations, uncertain assumptions,
and patterns that can be generalized.

Forgetting is not necessarily a failure.

Selective forgetting may reduce unnecessary processing
and representation costs. Abstraction may preserve useful
relationships without retaining every detail at the
same level of activation.

An imperfect memory can remain functional if important
capabilities survive and the system retains the ability
to correct itself.

**The objective is not perfect recall. It is durable
functional competence under bounded resources.**

#### Physical Processes and Internal Rhythms

Natural systems undergo changes of state through physical
processes and interactions.

Earth's rotation is a physical process. Dividing a day into
twenty-four hours is a human convention for measuring it.

Likewise, a fixed computational interval describes when
processing occurs according to a clock, but does not
necessarily describe when that processing is needed.

A memory system might encounter hundreds of meaningful
events in a few minutes, or very little relevant change
over several hours.

A rigid processing schedule cannot by itself distinguish
these conditions.

Biological systems also use internal clocks and
time-dependent regulation. This research does not claim
that time is irrelevant or that nature is exclusively
event-driven.

Instead, TME investigates whether internal memory operations
can be initiated primarily by changes in state, unresolved
contradictions, uncertainty, redundancy, and the need
for consolidation.

Time remains valuable as information about events,
durations, deadlines, and causal relationships.

**Time as information is different from time as the
mandatory trigger for memory reorganization.**

#### Learning, Forgetting, and Internal Reorganization

TME investigates a model-independent memory architecture
in which experiences can modify internal relationships
and representations.

Its proposed mechanisms include:

- **Attention:** selectively allocating processing resources
  according to current relevance.
- **Reflect:** reconsidering assumptions when new evidence
  conflicts with established expectations.
- **Abstract:** discovering reusable relationships across
  different experiences.
- **Compress:** reducing redundant representations while
  preserving useful information.
- **Slice:** separating ambiguous states into more precise
  representations.
- **Adaptive Decay:** updating activation and stability
  according to evidence and the results of internal
  operations, rather than indiscriminately forgetting
  information because time has passed.
- **Sleep:** an internally triggered consolidation mode
  combining memory review, reorganization, and selective
  reinforcement or reduction.
- **Evidence Seeking:** identifying consequential knowledge
  gaps and requesting external evidence rather than
  inventing unsupported answers.

Uncertainty is not automatically an error to eliminate.
It can identify what the system needs to investigate.

Similarly, internally generated associations may serve as
hypotheses, but must not silently become verified facts.

The original evidence and its provenance should remain
distinguishable from interpretations and hypotheses.

These mechanisms are architectural research objectives.
Their proposed combination has not yet been experimentally
validated.

#### The Long-Lived Entity

A language model may provide reasoning and communication
capabilities without being the sole location of an entity's
long-term memory or operational continuity.

TME investigates whether learned relationships, persistent
state, relevant experience, and operational context can be
maintained independently of a particular language model.

Under this approach, models may be replaced while the
external memory system preserves relevant continuity.

The goal is not an entity that simply remembers more.

It is an entity that can adapt without uncontrolled growth,
preserve important capabilities without becoming rigid,
and reorganize its own memory under limited resources.

### Project Background and Research Status

The Transformer Memory Engine is an independent, self-initiated
open-source research and engineering project.

It is not a university research program, doctoral dissertation,
commissioned academic study, or institutionally funded initiative.

TME originated from architectural questions encountered during
the development of OrdiFi, particularly the concept of modular
Bitcoin UTXO-based AI entities.

In this architecture, a language model is treated from the
beginning as a replaceable component, rather than as the entity
itself.

The project draws inspiration from established research in
cognitive science, neuroscience, adaptive systems, continual
learning, and artificial memory architectures.

It does not claim to have invented these underlying principles
or demonstrated superiority over existing approaches.

#### Research Approach and Practical Limitations

Although independently developed, TME aims to apply scientific
reasoning and experimental discipline wherever feasible.

This includes:

- Explicit and falsifiable research hypotheses.
- Documented architectural decisions.
- Reproducible experiments and source code.
- Comparisons with appropriate baseline systems.
- Transparent reporting of assumptions and limitations.
- Documentation of failures and negative results.
- Revision of architectural decisions when evidence requires it.

The project operates without dedicated research funding,
institutional laboratory infrastructure, formal academic
supervision, or guaranteed access to research partnerships.

Consequently, some experiments may not achieve the scale,
statistical power, methodological breadth, or external
validation expected of formal academic research.

Where established benchmarks are accessible and practical,
TME will seek to use them.

Where custom experiments are necessary, their methodology,
data, evaluation criteria, and limitations should be
documented to support independent examination and
reproduction where possible.

Preliminary experimental results will not be presented
as broadly validated scientific conclusions.

**The objective is not to imitate the institutional appearance
of academic research, but to apply scientific reasoning and
experimental discipline within the practical limits of an
independent open-source project.**

Constructive criticism, independent replication, and
future collaboration are welcome.

---

#### Central Research Hypothesis

> A model-independent, state-driven memory architecture may
> support long-term adaptive learning, selective forgetting,
> and functional continuity more efficiently than fixed-
> interval or indiscriminately accumulating memory systems
> under comparable resource constraints.

This hypothesis requires controlled experiments.

Success must be evaluated through retained competence,
adaptability, retrieval quality, information loss, memory
growth, processing cost, and continuity after model
replacement.

TME must be compared with simpler approaches, including
conventional context management, retrieval-augmented
generation, fixed-interval consolidation, and established
forgetting strategies.

If alternative methods perform better, the architecture
must remain open to revision.

#### Guiding Principle

**A memory system should not grow merely because it
experiences more. It should become better organized
in response to what it experiences.**

TME does not aim to reproduce the human brain.

It investigates whether principles of adaptation,
selective memory, continual learning, and consolidation
can help artificial systems remain stable, useful,
and resource-conscious over extended operational lifetimes.

*This preface describes research motivation and hypotheses,
not demonstrated scientific results.*

---

<!-- TME_RESEARCH_PREFACE_END -->

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
