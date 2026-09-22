---
name: feedback_overnight_no_parked_decisions
title: Overnight shifts decide, never park decisions
id: feedback-overnight-no-parked-decisions
description: "Operator 2026-09-06: in a granted overnight arc, reversible fleet flips are the shift's call — never park them for morning."
metadata:
  type: feedback
cites:
  - .claude/skills/delivery-stasis/SKILL.md
---

**What the operator said (2026-09-06, 02:41 their time):** "why don't you canary flip James, and keep working on this..
Don't leave decisions to me in an overnight shift."

**Why:** the night's report had ended with the `=observe→canary` flip framed as the operator's call. Under an explicit
overnight grant ([[feedback_overnight_mandate_upgrade_epics_fair_game]]) that framing is timidity, the same shape as
[[feedback_shift_drive_cure_not_stage]] — a reversible data change with a designed revert path is the shift's to make.
**How to apply:** in a granted overnight arc, make the reversible calls (deployment data, channel modes, fixture cures,
driver fixes) and record them as interpretive decisions in the journal; reserve for the operator only the irreversible
class (wipes, re-keys, integrity-hash moves, spend, vision). Never close a night report with a decision menu.

**Reaffirmed 2026-09-13 (morning after the pipeline-e2e shift):** the night report closed with five "open decisions"
(pool-conductor capacity, ingress classes, a persona for a scenario, a DNA contract fix, where the operator's WIP lands).
The operator answered "Yes on all the above, I'm not sure why these are all open decisions." Every one of them was
already implied by the sprint plan or the standing mandate; none was irreversible. Treat plan-implied and
mandate-implied calls as decided: act, journal the interpretive decision, and report what was done, not what could be.
