# TME Architecture Addendum — Orientation, Information Reacquisition and Skill Consolidation

**Date:** 2026-10-26
**Status:** Conceptual design; not a claim of implemented capability.

## 1. Generalized information reacquisition

Information reacquisition is a common TME mechanism, not an independent problem solver. It can be triggered when information required for a task is unknown, inaccessible, insufficiently supported, contradictory, potentially outdated, or contradicted by an unexpected execution result. Triggering does not diagnose the cause. The system should identify the specific information gap, estimate whether resolving it matters for the current task, and select a suitable source or delegate to another entity. The response is evidence to be evaluated by the relevant memory/learning process; it does not automatically overwrite prior knowledge.

An empty initial memory may trigger an initial orientation request via the host's available terminal/prompt/interface. An entity may begin by asking **“Was kann ich tun?”** The interface and entity speak as the entity, not as “the TME”. Information requests should be bounded and should not become an endless questionnaire.

## 2. Entity identity and environmental orientation

The TME engine supplies processing mechanisms, not a preloaded domain identity. Its host entity can be introduced to its identity, organization, purpose, coworkers, responsibilities, tools, communication channels and permissions by initial information acquisition. Example: **“Du bist Lucy. Du bist eine Entity, die für OrdiFi Aufgaben erledigt. Die dafür benötigten Fähigkeiten wirst du künftig erlernen und regelmäßig anwenden.”**

Identity, organizational assignments and authorization are authoritative contextual records, governed differently from incidental experiences and not discarded by ordinary decay. Learned competence and authorization are separate: possessing a skill never grants permission to execute it.

## 3. Learning from runbooks and training layers

Runbooks and Training/Capability Layers are external teaching sources, not documents to be bulk-copied into memory. Repeated guided execution generates observations, actions, outcomes, confirmations, errors and relevant relationships. Experience-guided reflection/consolidation may progressively form reusable procedural knowledge. A **Skill State Token** serves as an entry point to a stable, structured capability, including applicability conditions, dependencies, parameter roles, validation criteria and links to evidence or relevant exceptions. It need not store a verbatim runbook.

The candidate learning progression is **guided acquisition → experience-based consolidation → validated independent execution**. Mere repetition or a fixed execution count does not prove competence. A runbook-free test should establish that the entity can reliably use the authorized technical interface under tested conditions. The external runbook remains available for future reacquisition or changes.

## 4. Errors as learning experiences

A failed action followed by targeted information reacquisition, diagnosed correction and independently verified success can produce more informative evidence than an uncomplicated success. The TME should distinguish (a) confirmation of an existing procedure from (b) new error-recognition, diagnosis, delegation or recovery relationships. These may have different effects on activation, retention stability and retrieval paths.

**Higher stability after a repaired failure is a testable hypothesis, not an unconditional rule.** Credit should depend on evidence of what changed and whether a causal lesson was actually established. A transient outage followed by a retry is not automatically evidence that the skill itself improved.

Example: Lucy's Inscribe API call fails; she asks Ada because software diagnosis falls within Ada's remit. Ada investigates or repairs the defect within her authority; Lucy verifies the outcome and learns a useful escalation/recovery association. Lucy need not learn Ada's code-repair skill. If a broadcast may have succeeded despite an error, verify status before retrying to avoid duplicate execution.

## 5. Protected procedures and adaptive dynamics

Once a procedure is adequately validated, its established execution definition can be versioned/protected against unrelated memories silently rewriting it. Only structurally corresponding successful executions, with compatible conditions, should directly reinforce that procedure. Activation, stability, evidence, applicability and operational validity remain adaptive. A failure triggers investigation and possible acquisition of new information; it does not immediately invalidate or erase the learned procedure. New variants or superseding versions require supporting evidence and preserved provenance.

## 6. Research boundary and proposed checks

These are design hypotheses rather than completed features of the current TME implementation. Test incrementally:

- Empty-memory orientation through the entity's host interface.
- A repeated guided task with persistent evidence and a later runbook-free execution test.
- Successful repetition versus error → reacquisition → correction → success, observing distinct memory relationships and retention effects.
- Scope validation: changed conditions, changed APIs, and appropriate referral to a specialist.
- Measure independent task success and the change in external LLM/runbook dependence, without treating fewer calls alone as evidence of correctness.

**Terminology:** TME is not an LLM; adjustments to its control rules are *Parameterkalibrierung*, not fine-tuning. Runtime weight/activation/retention updates within established rules remain adaptive operation.
