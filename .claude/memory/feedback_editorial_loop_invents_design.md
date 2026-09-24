---
name: feedback_editorial_loop_invents_design
title: Editorial loops invent design on normative docs
id: feedback-editorial-loop-invents-design
description: "Blind-reader loops on spec/law/whitepaper do not converge; editors invent rules to close findings. Cap rounds, diff-based drift check, defer by name."
metadata:
  node_type: memory
  title: Editorial loops invent design on normative docs
  type: feedback
  originSessionId: 5c45be19-a450-4d6a-b68d-bb9c809bb2f1
  modified: 2026-09-23T10:36:55.336Z
cites:
  - genesis/docs/content/elohim-protocol/.epr-meta
  - .epr-meta/elohim/packages/agents/blind-reader.json
---

**What happened (2026-09-23 editorial pass on README + protocol-specification, shefa, constitution,
social_medium epic, orchestrator README):** Opus 5.5 editors briefed with survey findings plus rulings
did good tic/leak cleanup, but each fresh context-isolated reader on a normative document returned a
new set of *design* questions (10-18 interpretability findings per round, not shrinking), and the
editors "closed" them by writing new normative content: a Kademlia public-only rule and a byte-access
rule in the spec, a "three kinds of value" structure in shefa, ten new Appendix C questions and
mechanism claims in the constitution, hop-by-hop correction mechanics in the epic. Must-survive
passages were also rewritten "to close a finding". A diff-based fact-drift reviewer (`git show
HEAD:path` vs working copy; facts lost / facts invented / voice flattened / residual tics) caught
all of it; a repair pass driven by that report restored the committed meaning.

**Why:** a blind reader's job is to surface what a stranger cannot recover; on a design document
that is unbounded, and an editor told to "close every interpretability finding" will invent the
missing design. The README (a routing document) converged; the canon did not.

**How to apply:**
- For vision/spec/law documents, cap the reader loop at two rounds, then **defer the remaining
  findings by name** to the operator (the `.epr-meta` rule allows it).
- Every editor round on a normative doc gets a **fact-drift diff check** before the next round, with
  the rule: a design gap becomes one honest "open question" sentence, never a new rule.
- Put the must-survive list in the editor brief AND in the drift reviewer's brief; verbatim means
  verbatim.
- Keep rulings explicit up front (vocabulary, what runs vs designed, which shape is normative); the
  editors follow rulings well and improvise badly.
- Related: [[feedback_readability_edit_by_codex_or_gemini]], [[feedback_verify_the_measure_before_the_ranking]],
  [[feedback_sealed_decisions_must_not_outrun_evidence]].
