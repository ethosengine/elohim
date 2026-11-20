---
name: feedback_waiting_turns_end_with_a_wakeup
title: "Every waiting turn ends with ScheduleWakeup"
description: "In a long unattended sprint, a turn that ends \"holding for notifications\" can idle for hours if the session is suspended; always arm a ScheduleWakeup (≤1800 s) before ending a waiting turn."
id: feedback-waiting-turns-end-with-a-wakeup
metadata:
  node_type: memory
  title: Every waiting turn ends with ScheduleWakeup
  type: feedback
  originSessionId: 09892ed7-62ff-451d-a4cc-7871ccfa5ccb
  modified: 2026-10-18T14:52:56.726Z
cites:
  - .claude/skills/agentic-developer/SKILL.md
---

On 2026-10-18, during the operator-granted 15 h autonomous sprint, I ended several turns with
"holding for the next completion" and no fallback wakeup. Between ~04:30Z and ~11:00Z the session
produced nothing: background tasks (a fork build, two implementers, a lane) resumed only when the
session resumed, and the completion notifications never re-invoked it. Roughly 6.5 of the 15 hours
were lost that way, and my journal clock stamps had drifted because I estimated them instead of
reading `date -u`.

**Why:** the harness re-invokes on notifications only while the session is live; a suspended or
paused session loses wall-clock silently, and nothing else wakes it.

**How to apply:** in any unattended loop, end every waiting turn with `ScheduleWakeup`
(1200–1800 s for CI waves, shorter for lanes/builds), carrying the plan file path in the prompt so
the wakeup re-reads the Journal; stamp the Journal from `date -u`, never from memory; and when the
clock jumps, treat every "pending" agent as possibly finished and read its output before waiting
again. Related: [[feedback_memory_index_is_projected_never_hand_edited]],
[[project_shem_workspace_runs_long_sprints]].
