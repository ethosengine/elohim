---
name: feedback-fix-what-you-find-no-leaving-defects
title: "Fix what you find; never leave a defect recorded-but-open"
description: "Operator 2026-10-07: a defect found in flight is fixed in flight, not canonicalized and left for later — bites whenever a lane surfaces a side-defect."
id: feedback-fix-what-you-find-no-leaving-defects
metadata:
  node_type: memory
  title: Fix what you find; never leave a defect recorded-but-open
  type: feedback
  originSessionId: b792cf88-2ad3-4526-8b27-3869349776fe
  modified: 2026-10-07T21:41:53.039Z
cites:
  - genesis/data/timeline/backlog/CLUSTERS.md
---

"if you find it fix it.. don't leave defects" — operator, 2026-10-07, after I recorded two
household defects (the dead-anchor Held wedge reproduced on a recast household; the identity
seeder refusing its own fixture ids) in their backlog entries and reported them as findings.

**Why:** recording is not resolution. A backlog row with a repro and no fix is exactly the
"instrument with no reader" CLAUDE.md warns about; the operator's velocity rule
([[feedback_atomic_wins_compound_velocity]]) means the right-way fix lands now, while the
repro and the context are live, not in a later session that has to rebuild both.

**How to apply:** when a lane surfaces a defect outside its original scope, the default is to
fix it in that session (delegating to the equipped agent, verifying on the repro, committing
with its proof), and the backlog entry records the fix, not the deferral. Deferring is the
exception and needs the operator's say-so or a hard blocker (operator-owned cluster state, a
missing capability, a design question only the operator can rule on). "Blocked and
canonicalized" is not a terminal state for a defect the tree can fix.
