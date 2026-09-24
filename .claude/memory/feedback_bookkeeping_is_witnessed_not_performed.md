---
index: false
name: feedback_bookkeeping_is_witnessed_not_performed
title: "Bookkeeping is witnessed, not performed"
id: feedback-bookkeeping-is-witnessed-not-performed
description: "Operator 2026-09-23: participants (human or agent) act; the harness witnesses the act into eprfs. Manual bookkeeping is a harness gap, not a step."
metadata:
  node_type: memory
  type: feedback
cites:
  - .epr-meta/acts-attributed-to-participants.habit.md
  - .claude/hooks/valueflow-observer.py
---

Operator, 2026-09-23: taking part in the Elohim protocol should be a **passive experience most of the time**,
for AI agents as much as for humans. A participant makes a change; the harness **witnesses** it and records it
to the files (eprfs exists for this). Friction should only surface where validation genuinely needs it: to build
trust and to exercise accountability.

**Why:** the 2026-09-23 memory-ceremony close took four manual bookkeeping steps. (1) Claim an actor and re-run
`epr flow memory import`, because the earlier run had taken a stale `blind-reader` claim and rewritten 251
authors. (2) `sed` the author on 35 records the import skipped. (3) Add a missing `title`, which the pre-commit
gate caught at commit instead of at write. (4) Re-import and re-project MEMORY.md by hand. Every one of these is
work the harness should absorb.

**How to apply:** when you find yourself doing bookkeeping (re-pinning CIDs, fixing attribution, re-projecting
an index, re-sealing cites), do it to unblock the work, then name it as a harness gap. Don't write it up as a
procedure for the next agent to follow. The target shape is in [[acts-attributed-to-participants]]: a re-import
appends an act and never rewrites identity. Surface a refusal at write time and loudly, never as a bare exit at
commit. Related: [[project-eprfs-witnessed-interaction-primitive]], [[feedback_agent_fleet_and_harness]].
