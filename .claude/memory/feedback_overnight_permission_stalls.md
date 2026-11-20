---
index: false
name: overnight-permission-stalls
title: Overnight permission stalls
id: feedback-overnight-permission-stalls
description: An idle overnight session may be blocked on a permission prompt (auth paths), not done; check the transcript tail and never race a blocked session.
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 6bd0f758-fe18-46cf-b0d0-8848acafeca0
  modified: 2026-11-19T13:11:52.988Z
cites:
  - .claude/skills/agentic-developer/SKILL.md
---

During the 2026-06-04 overnight integration, Sprint A (portal-handoff) stalled for ~1h mid-task on a permission prompt while writing a spec for an auth component (`threshold-login.component.spec.ts`). The session looked dead (idle transcript, no commits) but was actually blocked awaiting approval. The operator: "I've got to keep in mind what gets touched in an overnight shakeout."

**Why:** Permission prompts block silently — an idle-looking session may be waiting on approval, not done. Auth-path files are a likely prompt trigger.

**How to apply:**
- Before an overnight/autonomous run, anticipate which paths the work will touch (especially auth/security components) and confirm permission coverage — or have the operator pre-approve.
- When detecting "sprint done" via quiescence, distinguish *blocked-on-permission* from *finished*: check the transcript tail — a final assistant text that ends mid-action ("Writing the X spec:") with a pending tool call means blocked, not done.
- As integrator, don't race a blocked session to finish its work — if the operator approves the prompt, the session resumes and collides with your version. Verify first whether the session can be unblocked.

**Same stall in background subagents (2026-11-19, brit sprint 3, session 0e0924d5).** Two reviewer subagents sat "still running" for 3 h and 2 h; a scout sat 4.5 h. I (Claude) only looked after a sibling session reported the same thing on its side. What it looked like and what worked:
- Detect: compare each task output file's mtime with `date` (same container, so no clock skew), and list child processes of the session (`ps --ppid <claude pid>`). A subagent whose transcript ends in an assistant `tool_use` with no `tool_result` and that has NO child process never had its command run. Cause confirmed only for my own session: an Edit outside the working directory waited about two hours and resumed when the directory was added to the allowed list. For the two reviewers the approval hold is my inference, not verified (one later blamed a script of its own). All three subagent calls that were held that day (two reviewers, one of them twice) defined a shell function inline in the Bash command (`c(){…}`, `r(){…}`, `chk(){…}`); hundreds of other long compound commands without one ran. Brief agents: no inline function definitions — write a script file and run `timeout N bash <file>`.
- Recover: `TaskStop` the agent, then `SendMessage` it — it resumes with its context intact. Tell it the call never ran, not to re-issue it or reach the same effect another way, and to report from what it already established.
- A subagent waiting on its own background process is different: it has child shells. Read their command lines. I found nine `until ! pgrep -f '<script name>'` loops that matched themselves (see [[feedback_agent_fleet_and_harness]]), and one unbounded `while kill -0 <pid>` on an experiment that ran five hours.
- My interpretation, for the next multi-hour orchestration: check liveness on a schedule instead of repeating "still running" from the harness, and put three lines in every brief — bound every experiment with `timeout`, wait with `pgrep -x` or a pid file, never with `pgrep -f`, and keep every write inside the scratch directory.

Related: [[concurrent-sessions-shared-worktree]]
