# TME — Consolidation Architecture

Status: Architectural specification; not fully implemented.

## 1. System identity

Transformer Memory Engine is an adaptive memory management
system, not a Large Language Model.

Ordinary memory adaptation operates under defined control
mechanisms. Changes to those mechanisms are Parameter
Calibration, not neural Fine-Tuning.

## 2. Shared consolidation system

Reflect and Sleep are two operating modes of one
consolidation architecture.

### Reflect — active mode

- Event-driven, local and resource-bounded.
- Operates alongside normal agent activity.
- Inspects relevant evidence, state changes and conflicts.
- Identifies missing information and uncertain memories.
- May request external information through the agent.
- Produces revision-linked consolidation candidates.
- Does not commit structural weight reconciliation,
  irreversible deletion or global reorganization.

### Sleep — idle mode

- Starts during agent idle periods when relevant memory
  or calibration revisions require examination.
- Has no fixed completion deadline.
- Can pause or stop when agent activity resumes.
- Analyzes memory structure and coupled weight dimensions.
- Evaluates Reflect findings against current evidence.
- Prepares and validates candidate reorganizations.
- May legitimately conclude that no change is needed.

Sleep analysis must not mutate the active memory state.

## 3. Weight interaction rules

Weight dimensions are independent by default.

Each permitted coupling must specify:

- Source and target dimensions.
- Influence direction.
- Evidence or rationale.
- Conditions and control parameters.
- Bounds and applicable invariants.
- Potential direct or indirect feedback loops.
- Tests and measurable expected consequences.

A proposed structural weight change must account for
all applicable dependencies, including indirect effects.

Accounting for a dependency does not imply changing
its associated weight.

## 4. Operational versus structural adaptation

Routine operational state changes can occur under
explicitly calibrated control rules.

Coupled structural weight reconciliation is reserved
for validated Sleep consolidation.

Parameter Calibration changes control mechanisms,
thresholds or coupling rules. It does not directly
rewrite memory weights and is not Fine-Tuning.

## 5. Information gaps

For a consequential knowledge gap, TME reports
uncertainty and may request retrieval through an agent.

TME does not itself browse external resources.

Plausible completion of inconsequential details must
remain distinguishable from evidenced historical facts.

Lack of evidence must not be converted into certainty.

## 6. Sleep transaction

Sleep follows:

1. Analyze a consistent memory snapshot.
2. Prepare a proposed change set.
3. Validate dependencies, evidence and invariants.
4. Check compatibility with newer memory revisions.
5. Commit the complete validated state atomically,
   or leave the existing state unchanged.

A failed, interrupted or obsolete proposal must not
partially modify active memory.

Recovery after process or power failure must preserve
a complete valid memory state.

## 7. Availability

Agent activity takes priority over Sleep preparation.

The agent may continue using its valid active memory
while Sleep analyzes or prepares a future revision.

Commit duration and actual agent unavailability are
different metrics and must be measured separately.

No implementation-time or downtime guarantee is
claimed until measured.

## 8. Revision awareness

Track memory revisions and control-calibration revisions.

A completed analysis, including a no-change result,
is associated with the examined revisions.

Unchanged inputs must not trigger endless reanalysis.

New relevant information must invalidate or revalidate
affected pending consolidation proposals.

## 9. Evaluation requirements

Before enabling autonomous structural commits:

- Define the Weight Interaction Map.
- Test permitted and forbidden couplings.
- Test feedback stability and calibration robustness.
- Test interrupted Sleep and stale proposals.
- Verify atomicity, recovery and provenance.
- Compare against simpler memory baselines.
- Measure retrieval quality, latency, storage and RAM.
- Include long-horizon deterministic replay.

This specification describes target behavior.
Existing preview and retention modules do not yet
implement autonomous, integrated Sleep consolidation.
