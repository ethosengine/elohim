---
name: feedback_valueflow_authorship_is_the_process
title: Valueflow authorship IS the process
description: Operator (2026-09-05) — authoring/connecting REA valueflows in context IS the protocol's process; its verbs are the designed friction, everything around them (tool discovery, prompt assembly, ledger duplication) must be frictionless — bites at every SDD/epic loop
metadata:
  type: feedback
---

The operator's meta note (2026-09-05, mid-epic): the REA valueflow authorship loop —
intend (mint gap) → commit (seal, brief) → produce → verify → fulfil (receipt) → note
(correction) → ratchet (habit delta, project) — is THE process the whole Elohim Protocol
is designed to support, from one human to the global network. "The process of the protocol
is the designed friction; everything else should be maximally frictionless."

**Why:** during the Holochain Evolution Epic run the protocol verbs cost the right tokens
(seals, review seats, evidence-gated flips) but the surround cost more than it should:
finding which command mints gap items (`epr flow project`, discovered by grepping the binary),
hand-assembling ~2k-token implementer/reviewer prompts per dispatch, three ledgers that do not
read each other (SDD progress.md · epic §11.4 · flows.jsonl), rulings as prose instead of
`epr flow note`, habit DELTAs typed by hand.
**How to apply:** keep the protocol verbs (`epr flow seal/note/fulfill/project`, the review
seat, the gate, evidence-only flips) and make everything else a skill, a script emission, or
a projection: dispatch prompts as parametrised skill templates (constants once, brief + rulings
per call); SDD scripts emit flow events (brief → claim, report → fulfil, review → verify);
ledgers as ONE flow with the others projected; an `epr context <atom>` one-screen brief
(open intents · seals · notes · habit · gate). Prefer building the verb into the tool surface
over training any model on the method. See [[project_rea_valueflows_are_our_workflow_layer]],
[[project_epr_flow_valueflow_projection]], [[feedback_role_coherence_orchestration_operator_executes]].

**Ownership (2026-09-05):** the operator moved the implementation of this surround to a separate session; the epic session keeps the protocol loop and does not build the tooling itself.

**The why (operator, 2026-09-05):** friction is purposeful where a STANDARD is held against
individual bias — seal, review seat, evidence-only flip, notarized path, and the floor/ceiling
bounds a scope declares on a stock (the inequality curve, [[project_inequality_curve_as_bounded_standard]]).
Frictionless everywhere else because the marginal cost of applied knowledge trends to zero and
the substrate's performance edge is what lets sensing (lamad · psephos · the register) run
continuously enough for policy to follow — slow sensing is stale policy.
