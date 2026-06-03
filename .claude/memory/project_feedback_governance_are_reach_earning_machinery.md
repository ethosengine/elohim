---
id: project-feedback-governance-are-reach-earning-machinery
name: project_feedback_governance_are_reach_earning_machinery
description: "The teleological reason every EPR must declare feedback + governance legs — they ARE the machinery by which reach is earned (evidence + adjudication); reach can't be self-asserted, so an EPR without them can't earn reach and the manifest schema rejects it"
metadata: 
  node_type: memory
  type: project
  originSessionId: 02a31f41-5f1f-4600-9d3f-7d0c9a341c9c
cites:
  - elohim/sdk/CLAUDE.md
---

The informal/nascent reason **every EPR needs feedback and governance** (surfaced 2026-05-30 while fixing the manifesto reach mis-grade): they are the apparatus by which **reach is earned rather than asserted**.

Reach is earned at authoring ([[project_social_reach_nervous_system]]) — but "earned" requires evidence and a grader, or every author would self-declare `commons` and claim maximal free distribution. The two legs supply exactly those:
- **Feedback = the evidence.** An EPR's claims + the positive/negative observations accumulating against them + validity horizons. This is "the homework." (SDK: every content type must declare claims — what outcomes it asserts, what would contradict them, validity horizon.)
- **Governance = the adjudication.** The process (qahal/mishpat, councils, the reach-gate returning `{Allowed, Blocked, Pending}` — [[project_reach_gate_is_elohim_mediated_matchmaking]]) that reads the evidence and CONFERS the reach grade.

This is the teleological reason the SDK manifest schema **rejects any content type lacking value + governance + claims/feedback legs** (`elohim/sdk/CLAUDE.md`, app-manifest.schema.json). The rule reads like arbitrary boilerplate until you see it: an EPR without feedback has no evidence to be graded on; without governance, no one to grade it; such an EPR *cannot earn reach*, so it isn't allowed to exist. **The three-leg requirement and the reach economy are the same fact from two sides.**

**genesis/seeder is the trusted-issuer bootstrap of this loop** ([[project_reach_earned_genesis_seeder_grades_homework]]): with no community yet, a trusted authority pre-confers the grade ("we grade our own homework"). The general case — a third party authoring an EPR — MUST route through its own feedback (does the content prove out?) + governance (does the qahal confer the reach?). Same gate, same earning, different grader. The bootstrap stands in for the loop until the loop can run itself.

Re-reads two existing entries: `{Allowed, Blocked, Pending}` — `Pending` = "evidence insufficient / not earned yet," not "forbidden." And [[project_trust_as_efficiency_signal]] — cheaper distribution is the *payoff* of earned reach; feedback+governance is the *price*; one economy.

**Why:** elevates the manifest's three-leg requirement from a validation rule to a load-bearing protocol telos — useful when designing any new EPR/content type (if you can't say what feedback grades it and what governance adjudicates it, it can't earn reach) and when explaining why self-asserted reach is structurally impossible. **How to apply:** when reviewing a new content-type/EPR design, check the feedback+governance legs answer "how does THIS earn its reach, and who grades it?" — not just "are the fields present." Pairs with [[project_standing_composes_multiple_evidence_streams]] (standing = composed evidence streams) and [[feedback_schema_first_ioc]] (legs are schema-enforced).
