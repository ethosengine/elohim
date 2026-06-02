---
name: checkpoint-and-handoff-when-scope-explodes
description: "When inline execution of a plan surfaces unexpected substrate depth (>2 unforeseen architectural layers), operator preference is to checkpoint and write a fresh-context handoff at the end of the existing plan rather than push through indefinitely"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: de3918e5-34fe-4832-bc41-5a3ca933e9fc
cites:
  - .claude/skills/handoff/SKILL.md
---

When executing a written plan inline and discovering meaningful scope expansion that the plan didn't predict (typical signal: each "depth check" reveals another architectural layer that needs 1-2 hours of work), don't grind through to completion. Pause, write addenda to the existing plan documenting what was discovered, write a self-contained handoff prompt at the end of the plan, commit, and let a fresh context pick it up.

**Why:** The operator caught this during the 2026-05-26 substrate-rea-replication-fix work. After Tasks 1-6 + 6.5 landed and I'd surfaced two more unexpected substrate asymmetries (content side: lamad-types vs shefa-types, blob_cid vs blob_hash, no DHT-bootstrap for existing rows), they interrupted with "hey is the plan written? we should touch up the plan and finish with a prompt, so we can open a fresh context to get all the substrate work on REA in place."

**How to apply:**
- After 2 unexpected layers (or ~3-4 hours of inline execution), check in
- Add addenda to the plan capturing what was discovered (not the plan body — addenda preserve original intent + show evolution)
- Write a handoff prompt at the bottom of the plan that is self-contained: read order, current state (commits-ahead, what's done, what's untouched), first decision for operator, task entry points, gotchas, stop conditions
- Commit the plan update so the handoff is in git, not just in chat
- Surface the handoff prompt cleanly in the chat so the operator can copy/paste it into a fresh session
- The fresh context loads only the plan + recent commits, not the discovery-trail conversation

Related: [[memory_lifecycle_comet_shape]] (forgetting as design — execution context should melt, plan + handoff should persist).
