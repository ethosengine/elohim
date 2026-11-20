---
id: chronicle-substrate-currency-memory-handoff-reconcile
kind: chronicle
contentType: chronicle-entry
contentFormat: markdown
title: "Memory hand-off reconcile, 2026-11-20"
slug: substrate-currency-memory-handoff-reconcile
written: '2026-11-20'
occurred_at: '2026-11-20'
author: "agent:librarian@claude-opus-5-5 (steward human:matthew)"
status: noted
significance: meaningful
relatedNodeIds: []
tags: [memory-ceremony]
ceremony: substrate-currency
surfaces_rewritten: 30 memory entries (+59 frontmatter-only title/id repairs)
diff_review_verdict: refute-checked per item (workflow wf_2cd1d4fd-22a); no independent post-apply diff review
coherence_verdict: not run
sampled_topic: stale vs kept memory and branches, hand-off to shem
corrections_closed: 50
measured: 228 workflow agents, ~18.2M subagent tokens, ~13.5 min wall-clock; main-loop tokens unknown
review_rounds: 1
rework: index reprojection reverted (native projector dropped 101 rows, later refused at 37 KB)
---

# Memory hand-off reconcile, 2026-11-20

The steward asked for this device's dirty work to be handed to shem, and for whatever was stale to be pruned, so the integrator receives current memory. Every judgment was made against origin/dev 44d80dfb6. Local dev is 266 commits behind it, and this pass did not integrate them.

**Branches.** 101 remote branches were deleted. 65 were plain ancestors of origin/dev. The other 36 had their unique commits found patch-equivalent or landed on origin/dev, and a refute agent failed to overturn that. Four delete verdicts were refuted, so those branches were kept. Fifteen live or unclear branches remain.

**Corrections.** Of the 96 unresolved witnessed corrections, 50 are now closed by native notes. 48 were already repaired on their surfaces and 2 were repaired here in memory. 36 are still stale. Most of those sit outside this session's write set: commands, skills, CLAUDE.md, source and docs. Six are unclear.

**Memory.** 79 uncommitted entries were judged. 49 were kept as they were and 27 got targeted fixes. Two drop candidates were left in place, because staleness alone does not authorize removal. Fifty entries had no `title:` and so could never project into the index; that is now fixed. Committed on `memory/ceremony-2026-11-20` and pushed without gates for the integrator.

**Frontier.** The fully titled corpus projects to 37 KB, past the 24 KB hard bound. The committed MEMORY.md was rebuilt by hand: the HEAD rows plus 26 newer entries, at 20.3 KB. Restoring a generated index needs a storyteller consolidation pass (umbrellas). The 36 still-stale corrections outside the write set are still open, and the semantic fold is 2,338 files behind.
