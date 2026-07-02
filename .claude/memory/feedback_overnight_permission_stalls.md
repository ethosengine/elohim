---
name: overnight-permission-stalls
title: Overnight permission stalls
description: An idle overnight session may be blocked on a permission prompt (auth paths), not done; check the transcript tail and never race a blocked session.
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 6bd0f758-fe18-46cf-b0d0-8848acafeca0
---

During the 2026-06-04 overnight integration, Sprint A (portal-handoff) stalled for ~1h mid-task on a permission prompt while writing a spec for an auth component (`threshold-login.component.spec.ts`). The session looked dead (idle transcript, no commits) but was actually blocked awaiting approval. The operator: "I've got to keep in mind what gets touched in an overnight shakeout."

**Why:** Permission prompts block silently — an idle-looking session may be waiting on approval, not done. Auth-path files are a likely prompt trigger.

**How to apply:**
- Before an overnight/autonomous run, anticipate which paths the work will touch (especially auth/security components) and confirm permission coverage — or have the operator pre-approve.
- When detecting "sprint done" via quiescence, distinguish *blocked-on-permission* from *finished*: check the transcript tail — a final assistant text that ends mid-action ("Writing the X spec:") with a pending tool call means blocked, not done.
- As integrator, don't race a blocked session to finish its work — if the operator approves the prompt, the session resumes and collides with your version. Verify first whether the session can be unblocked.

Related: [[concurrent-sessions-shared-worktree]]
