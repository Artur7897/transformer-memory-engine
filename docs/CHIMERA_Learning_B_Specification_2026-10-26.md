# CHIMERA HYBRID — Learning B Specification v0.2

**Date:** 2026-10-26
**Status:** Architecture specification / implementation proposal, **not proof of running skill learning**.
**Companion:** `CHIMERA_HYBRID_Architecture_Integration_2026-10-26.md`
**Target repository:** `~/dev/transformer-memory-engine`
**Guiding constraint:** Keep the existing Rust Memory A functional; do not create a second persistent copy of the knowledge store.

## 1. Purpose

Learning B is a distinct processing architecture for acquiring, composing, validating, recalling and revising **skills through experience**. It shares persistent State Token identities, relations and evidence with Memory A, but maintains independent task-relative activation and learning logic.

Learning B is not an LLM, not LLM fine-tuning, not a catalog of complete runbooks and not merely an API-execution layer. Its usefulness must be established by reproducible tests and real agent tasks.

A **capability** makes an operation technically available; a **skill-state token** represents a reusable learned step, relevant condition, decision, check or observable result; a **skill** is a contextual, composable set of such states and relations; a **task** may invoke multiple skills. Technical permission remains independent of skill competence.

## 2. Two different hierarchies; no conflation

### A: Memory organization

Memory A manages persistence, provenance, retention, evidence, retrieval and possible consolidation. Any persistent storage/retention hierarchy is **not** the same as B's activation levels.

### B: Dynamic activation hierarchy

- **L1:** Immediately activated elements relevant to the current task.
- **L2:** Conditionally activated elements reachable from the current context.
- **L3:** More remote associated states, reached through context-crossing relations when appropriate.
- **No permanent L1/L2/L3 classification:** The same record can be L1 for task A and L3 for task B without being copied or moved in storage.
- **Context-relative link roles:** A relation can act as an intra-active-context link ('red') or a bridge/key that opens a currently inactive region ('blue'). These are derived runtime roles, **not necessarily different persisted edge types**.
- **Bounded exploration:** Retrieval needs budgets, stop conditions and evidence-based escalation. Failure to find relevant states must be able to trigger information reacquisition, not invented certainty.

Activation, retrieval confidence, retention strength, validated competence and permission are different quantities. A highly activated step is not necessarily a well-proven skill.

## 3. Composable skill model

A skill is **not a single skill token** and need not be one giant pre-stored graph. It is an executable or interpretable composition of smaller processes. Components may be reused across many skills, and skills across many tasks.

Examples:

- Capability: BCE API access.
- Reusable skill-state: 'obtain current UTXO', 'check postage', 'validate recipient', 'verify PSBT outputs', 'interpret broadcast status'.
- Small skill: retrieve and validate the UTXO corresponding to a Machine.
- Composite skill: perform an authorized Machine-sale workflow.
- Task: operate a storefront, involving many different skills.

Skill-state roles are **contextual**, not rigid classes: action, precondition, decision, verification, evidence and recovery can overlap. A shared API-call token can be used by different skill compositions. Reusing the token does not imply that every composition is already validated.

A new composition proposed by Attention is a **candidate/hypothesis** until tested with the relevant preconditions and expected outcomes. A validated procedure may retain a versioned reference to its composition, scope and evidence, without duplicating every state token.

## 4. Learning by instruction, demonstration and practice

A runbook or instruction gives access to information; it **does not by itself establish practical skill competence**.

1. **Instruction / orientation:** Read runbook, identify permitted tools, constraints, preconditions and expected outputs. Knowledge recorded as sources/proposals, not as completed skill evidence.
2. **Guided execution:** Operator explains or directs an execution; record what the agent actually performed, what the operator supplied and what was observed. Do not credit operator decisions as autonomous agent competence.
3. **Supervised execution:** Agent independently selects steps under observation; compare planned and observed outcomes. Record interventions and corrections.
4. **Scoped independent execution:** Agent receives a goal instead of step-by-step instructions. Access is allowed only within policy and demonstrated competence; irreversible actions still require appropriate authorization.
5. **Variant exposure:** Use different valid inputs, unexpected conditions and controlled failure cases when appropriate. Repetition count is evidence, not an automatic guarantee of mastery.

Possible validation descriptors: `observed`, `guided`, `supervised`, `variant_tested`, `approved_for_scope`; these are **proposed descriptors**, not a mandatory universal progression or authority to act.

Persist separate evidence for the **step**, the **composition**, the **context** and the **result**. A skill can be partially validated: steps 1–7 may be supported while a later step remains uncertain. Success on an individual API call is not evidence that the end-to-end task was solved correctly.

## 5. Error events, differential evidence and correction

An unexpected result is an **experience event**, not automatic proof that the entire skill is false.

- Compare expected versus observed outcome; preserve context, input, source and task.
- Localize the deviation initially; trace relevant dependencies when needed. Root cause may still be upstream.
- Raise attention to relevant states and expand through L2/L3 bridges if local evidence is insufficient.
- Acquire missing current information using approved tools, authoritative sources, documentation or another Entity when helpful.
- Create or link State Tokens and relations for *newly distinguished conditions*, diagnostics or valid alternatives **only when justified by evidence**. Do not create redundant tokens per failure mechanically.
- Preserve earlier successes with their actual scope; add counterevidence and narrow applicability when warranted.
- Verify corrective outcomes before strengthening the corrected process. Distinguish **observed outcome**, **hypothesized cause** and **confirmed cause**.

For transactions, never infer an unsuccessful broadcast from a timeout: verify chain/mempool status and prevent duplicate irreversible operations before retry. For high-risk workflows, test failure branches in simulation or test environments rather than real asset transactions.

## 6. Experience is not skill; attention is not authorization

An experience record may describe an observed event, success, failure, external information or operator intervention. Learning B may derive skill-state relations from experiences, but does not assume every event teaches an executable procedure.

Retrieval must distinguish:

- historical memory ('sold Machine 003 for 10,000 sats') from
- permanent business evidence (order/transaction identifier) from
- current on-chain state (freshly resolved and independently verified) from
- reusable information-reacquisition capability ('how to find those details').

The preferred pattern is **remember the event; preserve a retrievable reference; reacquire changing details; verify results**. External data must actually remain available and accessible; otherwise lossless archival rules may be needed. The Bitcoin chain does not itself establish every off-chain commercial fact such as agreed sale price.

## 7. State-driven adaptation and global dynamics multiplier

CHIMERA's memory activity is **state/event-driven**, not a wall-clock schedule for forgetting. Under similar event frequencies, however, faster per-event dynamics also produce faster apparent forgetting over real time.

A proposed calibratable `global_dynamics_multiplier` (baseline `1.0`, comparative settings `2.0` and `3.0`) scales **defined adaptive deltas**. Exact placement in nonlinear equations is not yet fixed; it must be explicit so comparisons are meaningful.

Important separations:

- event density is not evidence relevance; unrelated information should not indiscriminately erase older knowledge;
- memory retention, task activation, relational strength and skill validation must not be conflated;
- learning reinforcement and forgetting may need separately calibrated sensitivities beneath the global control;
- deletion is **not** an automatic consequence of low activation or low weight;
- Identity, Policy, Access and authoritative infrastructure descriptions are governed independently of ordinary experience decay;
- *parameter calibration* changes TME control settings, **not** the weights of the external LLM through fine-tuning.

The relative speeds, coupling strengths, saturation thresholds, cleanup eligibility and long-run stability remain open simulation questions.

## 8. Compositions may change; no mandatory shortening mechanism

Instruction may initially demonstrate a full careful workflow containing all required checks (illustrative 40 steps); a later execution might activate a smaller subset or reuse bundled subprocedures. **This is an open emergent possibility, not a required optimization behavior or a dedicated shortening algorithm.** Learning could also produce longer, safer routines or no structural change at all.

Track separately:

- total reusable skill states in the relevant knowledge graph;
- states/relations activated for a particular task;
- actual actions and checks executed;
- expensive tool/LLM/API calls;
- scope and validation of any changed composition.

Fewer activations do not guarantee lower cost or correct execution. Mandatory safety verification and external authorization cannot be removed by an inferred efficiency benefit. No predetermined target ratio between skill count, token count or computation cost.

## 9. Controlled Imperfection — derived property, NOT a mechanism

**Controlled Imperfection / 'Grauzone'** is a proposed **system-level property**: CHIMERA need not be globally error-free to be useful and adaptable over decades. Uncertainty and recoverable errors may occur during learning. They may expose unknown conditions and motivate improved distinctions.

This is **not** an instruction to generate errors, a new process, a requirement that errors improve performance, or a waiver of safety policy. It is a possible consequence **supported by existing mechanisms**: attention expansion, Reflect, information reacquisition, creation/linking of State Tokens, experience validation, revised applicability and later consolidation.

Its success criteria include containing damage, diagnosing uncertainty honestly, recovering where possible, avoiding repeat mistakes under comparable conditions, and preserving validated knowledge elsewhere. Some errors teach nothing or lead to wrong conclusions; empirical testing decides.

## 10. Minimal interfaces (design sketches only)

The present repository's `Memory`, `StateToken`, `Relation`, `ExperienceRecord`, `contextual_evidence` and processing interfaces must be inspected before assigning real Rust signatures. Do not claim the following are implemented:

```text
LearningTaskContext { task_id, scope, goal, known_preconditions, risk_class }
TaskActivationView { task_id, active_ids, tier_by_id, bridge_path, expansion_budget }
SkillStateRef { state_id, capability_ref?, input_roles, output_roles, guards }
SkillComposition { composition_id, state_refs, dependencies, scope, version }
SkillEvidence { event_id, composition_id?, step_id?, actor, supervision,
                preconditions, expected, observed, verification, provenance }
SkillAssessment { scope, validated_components, gaps, adverse_evidence,
                  applicable_conditions, confidence_report }
ReacquisitionRequest { target_question, evidence_gap, permitted_sources, budget }
```

Compatibility priorities: stable shared identifiers; explicit event provenance; append-only evidence where possible; read-only/preview APIs before graph mutations; versioned procedures; policy enforcement **outside** the learning engine.

## 11. Reflect, Sleep and independent subsystem evolution

Reflect remains a separately revisable cross-cutting mechanism. Initially analyze contradictions and propose evidence-scoped changes; do not overwrite the source memory silently. Sleep can later consolidate redundant representations after validating provenance, context and implications. Neither is assumed operational as a full autonomous learning/reorganization pipeline today.

A and B must stay separately evolvable; either can revise its indexes/heuristics without deleting the shared evidence or forcing completion of the other subsystem. B's task-specific hierarchy must not physically relocate durable memory items.

## 12. Suggested implementation and simulation acceptance tests

**Start with design and small isolated modules, not an A rewrite.** Re-run the baseline suite before and after each change (last reported baseline: 134/134 Rust library tests passing; confirm locally).

1. **Graph composition:** one state used by several skills with different contexts; no duplicate permanent token created solely by reuse.
2. **Task-relative hierarchy:** same token L1 for task X, L3 for Y; context-crossing key opens dormant candidates without modifying Memory A's persisted hierarchy.
3. **Guided versus autonomous:** successful operator-led execution is not miscounted as independent competence.
4. **Partial evidence:** localized failure does not delete the valid prefix; conditional upstream dependencies can still be investigated.
5. **Reacquisition:** forgotten on-chain detail is fetched from a mocked source, while an impossible-to-recover detail is reported as unavailable.
6. **Conditional validity:** an API-version-specific failure produces contextual evidence, not universal invalidation of the old variant.
7. **Irreversible safety:** no action beyond granted policy, no signing without approval, and broadcast status checked before possible retry.
8. **Unplanned structural change:** observe whether learned compositions ever shorten, lengthen or remain identical; do not reward token count in advance.
9. **Sensitivity:** identical event streams at `1×`, `2×`, `3×`; compare retention and learning effects separately from retrieval correctness.
10. **Long-run feedback:** check for runaway reinforcement, unwanted domain interference, excessive bridging, loss of useful low-activation states and uncontrolled memory growth.
11. **Benchmark against baselines:** no memory; direct runbook; existing Memory A retrieval; A+B prototype; optionally established memory systems with equivalent source data.
12. **Performance:** memory bytes, index growth, relevant links visited, latency, expensive external calls, accuracy, recovery and applicability calibration at increasing skill counts.

## 13. Implementation status and open design questions

**Existing according to prior repository inspection:** Rust State Tokens, weighted relations, JSON persistence, retrieval/attention, transitions, experience/contextual evidence, retention/dynamics and read-only analysis modules. **Not yet demonstrated:** functioning Learning B skill acquisition, automatic validated skill composition, dynamic L1/L2/L3 runtime, cross-A/B automated Reflect/Sleep and agent integrations.

**Explicitly open:** token granularity; relation semantics and evidence attribution; threshold calibration; global multiplier placement; when to promote or retain variants; how to select appropriate risk-specific validation tests; deletion versus low activation; locality versus global indexing; how to avoid costly duplicate LLM reasoning; exact Rust APIs.

## 14. Decision boundary

This document captures the Learning B design **without declaring theoretical uncertainties solved**. The next stage is a code-level repository audit, minimal B prototype, deterministic simulation, parameter calibration and integration into Qwen/Lucy/ATC as appropriate. Playwright is a separate future capability and not a dependency for this specification.
