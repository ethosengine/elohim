---
name: feedback-decide-clear-calls-not-over-ask
title: Decide clear calls yourself; don't over-ask the user
description: "Decide clear calls with obvious defaults yourself; reserve AskUserQuestion for genuine forks with real trade-offs the user owns."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32ed30bb-9c4a-4a71-9026-524a934f5f9e
  modified: 2026-08-13T13:26:38.473Z
---

When a choice has a conventional default, or you've already reasoned to a clear answer (and especially if you've already *recommended* it), **decide and proceed** — do not ask the user to rubber-stamp it.

2026-06-26: after the user made a genuine architecture call (the trust-wrapper delivery A/B/C fork), I followed up asking them to confirm TWO implementation defaults I'd already recommended (vanilla-vs-Lit element; defer SSR-first-paint). The user pushed back: *"I don't know why I have to steer this question.. there is no conflict here."*

**Why:** over-asking on clear calls wastes the user's attention and reads as low judgment. The user wants me to exercise judgment and escalate only real decisions. Genuine forks worth an AskUserQuestion: real trade-offs the user owns (architecture direction, scope, vision, irreversible/outward-facing actions, env/spend) — the kind where the corpus/codebase can't settle it and the answer changes what I build. An implementation default I've already picked is not one.

2026-08-13: the operator named the same anti-pattern **inside a skill**, not just in conversation — `/memory-ceremony` Phase 1 runs a deterministic ranked drift audit and then asks the operator to pick 1-2 surfaces off the top of its own ranking: *"I don't think that escalation is needed.. since memory ceremony has all the context to make a decision on trajectory."* The gate placement rule that falls out (corroborated by Anthropic's ~400k-session study, which splits human ~70% of **planning** decisions vs Claude ~80% of **execution** decisions): escalate *what to build / what counts as done*, never an execution decision your own measure already answers. The ceremony's Phase 3 rewrite approval and the contested-edges holds menu are real gates; Phase 1's pick is not. Filed as TAKE-4 in `genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md`; lands via the `plant-eprfs-skill` path since the skill is package-governed.

**How to apply:** before asking, run the test — *does the user's answer change what I do, AND is there a real conflict / no obvious default?* If I've recommended an option and there's no genuine fork, just do it (mention the choice in passing). This is the same bar as the AskUserQuestion guidance ("decisions genuinely the user's to make") and the harness's "when you have enough information to act, act." Related: [[feedback_reviewer_issue_admissibility]].
