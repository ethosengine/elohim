---
name: feedback_overnight_no_parked_decisions
title: Overnight shifts decide, never park decisions
description: "Operator 2026-09-06 02:41 local: 'Don't leave decisions to me in an overnight shift' — in a granted overnight arc, reversible fleet data flips (e.g. observe→canary) are mine to make; bites whenever I end a night report with 'yours to decide'"
metadata:
  type: feedback
---

**What the operator said (2026-09-06, 02:41 their time):** "why don't you canary flip James, and keep working on this..
Don't leave decisions to me in an overnight shift."

**Why:** the night's report had ended with the `=observe→canary` flip framed as the operator's call. Under an explicit
overnight grant ([[feedback_overnight_mandate_upgrade_epics_fair_game]]) that framing is timidity, the same shape as
[[feedback_shift_drive_cure_not_stage]] — a reversible data change with a designed revert path is the shift's to make.
**How to apply:** in a granted overnight arc, make the reversible calls (deployment data, channel modes, fixture cures,
driver fixes) and record them as interpretive decisions in the journal; reserve for the operator only the irreversible
class (wipes, re-keys, integrity-hash moves, spend, vision). Never close a night report with a decision menu.
