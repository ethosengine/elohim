---
name: feedback_sealed_decisions_must_not_outrun_evidence
title: "Sealed decisions must not outrun evidence; review as outs…"
id: feedback-sealed-decisions-must-not-outrun-evidence
description: "Operator 2026-09-06 — read our Holochain usage as an outside skeptic; sealed decisions get a second-model read."
metadata: 
  node_type: memory
  title: Sealed decisions must not outrun evidence; review as outside skeptic
  type: feedback
  originSessionId: 62953c2b-d161-4b7d-8ec3-88beb0ae56de
  modified: 2026-09-06T19:13:35.177Z
---

**What happened (2026-09-06, holons/reimplementation second-opinion):** I sealed D1 ("retire holons-are-spaces")
leaning on Codex's abstraction; the operator asked "are you sure.. you don't want to feel biased by the existing
codebase, approach our current holochain usage as an external holochain developer with a healthy dose of skeptic."
Re-run from outside, the sharper finding was "you are carrying Holochain, not using it" (product writes go to
SQLite, DHT anchors null, membrane absent). Then the operator ran the plan through a second reviewer, which found
nine problems — a real blocker (post_commit is cell-local, [[project_holochain_post_commit_signals_are_cell_local]]),
a contradiction (new dedup identity vs unchanged integrity entry), and five "sealed decisions" that outran evidence
(binary commit-or-drop; "no live writer ⇒ safe deletion"; "fixture drop cures churn"; group clones as destination;
holding the whole app boundary).

**Why:** the operator wants decisions that survive an adversarial technical read, not confident synthesis.

**How to apply:** (1) before sealing an architecture decision, re-derive it as an outside expert with no stake in
the tree; (2) mark each "sealed" row with the evidence that seals it and downgrade to "candidate with
reconsideration criteria" when the evidence is an idiom or an idle benchmark; (3) send the plan's first slice to
Codex for an adversarial read BEFORE ExitPlanMode — signal semantics, identity/dedup, crash windows, and authority
are where my slices break. See [[feedback_skip_brainstorm_gates_self_answer]], [[feedback_delegate_research_to_opus_sonnet_codex]].
