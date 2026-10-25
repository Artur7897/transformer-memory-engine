# TME — Adaptive Memory Lifecycle

Status: Architectural proposal.
Implementation and empirical validation pending.

## 1. Objective

Maintain useful, reliable and efficiently retrievable
memory over multi-year operation.

The goal is not a fixed storage capacity or maximal
deletion. The goal is to prevent irrelevant, redundant,
outdated or misleading information from degrading
memory quality and retrieval performance.

## 2. Independent properties

The following concepts must remain distinguishable:

- Activation: current operational activity.
- Accessibility: cost and likelihood of retrieval.
- Persistence: requirement for long-term preservation.
- Evidence: provenance and factual support.
- Semantic validity: applicability in time and context.
- Episodic significance: importance of an experience.
- Intent dependency: connection to unfinished work.
- Exactness: whether approximate recall is acceptable.
- Consequence: impact of an incorrect use.

Not every property requires a stored numerical weight.

## 3. Memory access tiers

Proposed logical tiers:

- Active: currently relevant working information.
- Consolidated: structured reusable memories.
- Deep: indexed, rarely accessed long-term memories.
- Archive candidate: retained pending further review.

Tiers describe access organization, not factual
correctness, importance or permission to delete.

Physical storage and caching implementations may
differ from these logical categories.

## 4. Memory operations

Reflect:
- Process relevant events during active operation.
- Identify gaps, conflicts, cues and dependencies.
- Produce findings without structural consolidation.

Slice:
- Identify separable information components.
- Preserve identities, provenance and relationships.

Compress:
- Reduce representation cost or redundancy.
- Distinguish lossless from lossy transformation.
- Never silently convert an approximation into fact.

Tier:
- Move representations between access levels.
- Preserve required retrieval anchors and links.

Sleep:
- Evaluate relevant dependencies and impact sets.
- Coordinate structural changes under a versioned policy.
- Validate proposed transformations before commit.
- Permit interruption and legitimate no-change results.

Forget:
- Require a separate preservation and dependency check.
- Never follow automatically from low activation or age.

## 5. Anchored associative retrieval

New observations may reactivate stored memory through
recognizable entities, contexts, relationships or cues.

Inputs may originate from prompts, terminal output,
APIs, structured records or other agent observations.

Adapters and agents normalize observations while
preserving source information.

TME matches normalized observations to state tokens
and traces valid relationships to relevant fragments.

An anchor match alone does not establish historical
identity or truth.

If the exact fragment was removed, it cannot be
recreated as a verified recollection.

## 6. Information reacquisition

TME does not independently search external systems.

When an important detail cannot be reliably retrieved,
TME may generate an information-need request for an agent.

Possible outcomes:
- Retry using other stored cues.
- Request clarification from a user.
- Check an authorized external information source.
- Continue with an explicitly qualified hypothesis,
  when consequences and context allow.
- Stop a consequential action until verification.

Do not confuse creative reconstruction with verified
episodic recall.

## 7. Reversible inference

Preserve the distinction between:

- Recorded observations.
- Derived hypotheses and interpretations.
- Operational decisions based on those hypotheses.

Contradictory evidence can invalidate a hypothesis.
Reflect must surface the conflict promptly.

Sleep may reconcile dependent representations after
validation, without falsifying original observations.

## 8. Context-dependent preservation

No information type is universally important.

For example, wallet addresses may be:
- Relevant to an active transaction.
- Referenced by an open obligation.
- Frequently used and operationally useful.
- Historically recorded but operationally obsolete.

High exactness requirements do not automatically
imply permanent active-memory retention.

Consequential operations require appropriate
independent validation of critical values.

## 9. Coupling model

Distinguish:
- Independent dimension estimation.
- Joint operation decisions.
- Directed weight-to-weight coupling.
- Hard integrity and safety constraints.

Couplings must be justified by purpose and tested
for indirect consequences and feedback instability.

Do not calibrate the integrated system before its
functional interactions are sufficiently specified.

## 10. Resource discipline

Ordinary operations should avoid scanning the entire
historical memory when indexed or local access suffices.

Sleep may examine global dependencies conceptually,
but should reuse indexes and incrementally process
affected subgraphs wherever correct.

Measure processing costs rather than assuming that
more dimensions or tiers always improve efficiency.

## 11. Required evaluation

Test:
- Six-year deterministic event replay.
- Growing volumes of historical observations.
- Relevant details inaccessible on first retrieval.
- Recovery through later contextual cues.
- Forgotten information requiring reacquisition.
- Open obligations preserving exact identifiers.
- Incorrect hypotheses and subsequent corrections.
- Retrieval interference and temporal confusion.
- Changes to coupling and tiering configurations.
- CPU, RAM, latency and storage overhead.

Compare against simpler indexed-memory baselines.

Success means reliable, useful, sustainable recall;
not achieving a predetermined memory size.

## 12. Open research questions

- Which dimensions need persistent numeric values?
- Which interactions are genuinely necessary?
- How are retrieval anchors selected and retained?
- What evidence justifies irreversible information loss?
- When does tiering improve total retrieval cost?
- How can derived hypotheses be revised efficiently?
- Which errors are tolerable in which contexts?
- How should system behavior degrade when sources
  or retrieval paths are unavailable?

No autonomous tiering, deletion, reacquisition or
complete Sleep reconciliation is implemented here.
