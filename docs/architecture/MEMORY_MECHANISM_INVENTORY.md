# TME — Memory Mechanism Inventory

Status: Research and architectural definition in progress.

## Objective

Identify all required memory dimensions, mechanisms,
decisions, and interactions before calibrating the
integrated system.

This inventory distinguishes implemented behavior
from proposed architecture and experimental hypotheses.

## 1. Memory dimensions

### Activation
Status: Experimental numerical state.
Questions:
- What counts as meaningful activation?
- Which operational events change activation?
- How should activation affect retrieval?
- When is structural rebalancing justified?

### Retention Stability
Status: Experimental numerical state with separate
recall-event history and descriptive analysis.
Questions:
- How can retention reliability be estimated?
- How should separated successes and failures count?
- How is absence of testing distinguished from failure?

### Semantic Stability
Status: Proposed.
Questions:
- How are historical and currently valid facts separated?
- How are context and changing attributes represented?
- Which changes require information reacquisition?

### Evidence Quality
Status: Partial provenance and dependency analysis.
Questions:
- How are independent confirmations established?
- How are contradictions and source dependencies handled?
- Which decisions require additional verification?

### Episodic Significance
Status: Proposed.
Questions:
- How is a significant first occurrence recognized?
- How are rare events preserved?
- How is significance kept separate from frequency?

### Novelty and Habituation
Status: Proposed.
Questions:
- How are repetitive experiences distinguished?
- How does novelty change with additional observations?
- How can habituation coexist with episodic protection?

### Contextual Relevance
Status: Partial contextual analysis and retrieval.
Questions:
- How is task relevance determined?
- How does it affect retrieval selection?
- How is current relevance separated from preservation?

## 2. Operational mechanisms

- Attention and retrieval
- Recording experiences and observations
- Reflect
- Retention analysis
- Evidence and dependency analysis
- Abstract
- Compress
- Slice
- Adaptive Decay
- Sleep consolidation

For every mechanism, define:
- Inputs and provenance
- Observed or derived outputs
- Allowed operational changes
- Structural changes reserved for Sleep
- Direct and indirect dependencies
- Failure behavior and invariants
- Evaluation scenarios

## 3. Interaction categories

A. Independent observation or estimation
B. Shared decision using multiple dimensions
C. Directed coupling between dimensions
D. Hard invariant or preservation constraint
E. Feedback relationship requiring stability analysis

Not every co-occurrence is a causal dependency.

An interaction is neither accepted nor rejected
solely because it changes an example's outcome.

## 4. Architectural evaluation

Evaluate each proposed interaction for:

- Functional necessity
- Direction and causal rationale
- Effects on other decisions
- Stability and boundedness
- Simplicity relative to alternatives
- Observability and testability

## 5. Future simulation

The future TME Dynamics Simulator should support:

- Deterministic event replay
- Versioned policy configurations
- Dimension and coupling enable/disable comparisons
- Controlled parameter perturbations
- Complete intermediate state inspection
- Long-horizon stability testing
- Retrieval and preservation quality comparisons
- Ablation experiments
- Independent validation scenarios

Simulation results must distinguish demonstrated
behavior from assumptions and open questions.

## 6. Implementation boundary

No new numerical coupling or automatic structural
weight change is authorized by this inventory.

Complete the functional interaction model before
performing integrated parameter calibration.
