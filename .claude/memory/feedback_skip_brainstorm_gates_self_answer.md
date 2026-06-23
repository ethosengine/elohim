---
name: feedback_skip_brainstorm_gates_self_answer
description: "Operator prefers self-answered design questions + a single recommended-design summary, not the gated one-question-at-a-time brainstorming dialogue"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: bb6233b0-21d0-494c-8768-a211d858c47c
---

For design/brainstorming work, the operator does NOT want the typical interactive brainstorming flow (one question at a time, per-section approval gates). Instead: **develop the clarifying questions, answer them yourself from evidence, then present ONLY the summary of the recommended design with a short defense of the choices.** (Stated 2026-06-13 during the self-healing debug-view design.)

**Why:** The operator runs an operator-as-conveyor / autonomous style (see [[feedback_commit_only_integrator_pushes]], [[feedback_deterministic_flag_agent_canon_stasis_pattern]]) — they want me to do the reasoning end-to-end and bring back a decision-ready recommendation, not to be walked through it. Round-trip gates are friction.

**How to apply:** Still do the exploration (fan-out survey under ultracode) and still write the design doc artifact, but collapse the brainstorming skill's interactive gates into: explore → self-pose+self-answer the open questions → one concise recommended design + defense. The HARD-GATE on "no implementation before an approved design" still holds — present the design and get a go before building — but the *approval* is one summary review, not a section-by-section dialogue.
