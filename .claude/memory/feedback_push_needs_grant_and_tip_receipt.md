---
index: false
name: feedback-push-needs-grant-and-tip-receipt
title: "Push needs a grant; receipt on the tip"
description: "Auto mode denies pushes to dev until granted; pre-push wants the deliverability receipt on the tip sha; rebase is tier-gated, merge is not."
id: feedback-push-needs-grant-and-tip-receipt
metadata:
  node_type: memory
  title: Push needs a grant; receipt on the tip
  type: feedback
  originSessionId: b085634d-eeb1-4db9-a819-02d1eb548f1c
  modified: 2026-10-06T15:34:31.001Z
cites:
  - .husky/pre-push.bash
  - .claude/hooks/capability-tier-gate.py
---

Observed 2026-10-06 (settling the device-consent landing; my own reading, not an operator ruling):

- The auto-mode classifier denies `git push … :dev` as a production deploy, and the denial is per action: the operator's `/permissions` grant covered the one push I had attempted, and a later, different push ran without a prompt. It also denied `pnpm elohim-agent:packages:verify` (gospel tooling) until granted, and denied `epr actor claim … --root /projects/elohim` outright as a bypass.
- The pre-push t2-receipt leg (.husky/pre-push.bash ~L1550) refuses a push touching serving paths unless the receipt script finds a complete current-source household serving receipt; mint it on the final tip after the last commit.
- `git rebase` and other history rewrites go through `capability-tier-gate.py`, which resolves the tier from `CLAUDE_MODEL` or the session's actor claim in the **home checkout's** `.eprfs/status/actors.jsonl`. With neither, the tier is `unknown` and the rewrite is denied. `git merge origin/dev` is not gated and dev already carries merge commits.

**Why:** each of these cost a round trip late in a long session, after the work was done.

**How to apply:** when a task will end in a push, say so in the first status and ask for the grant early; do every commit first, then mint the household serving receipt on the final tip, then push (under the CLAUDE.md landing exception: `GATE_EVIDENCE=reuse git push origin HEAD:dev`, then the separate notes push); if the remote moved, merge it in rather than plan a rebase. Related: [[feedback_push_branch_discipline]], [[project_receipt_lane_quiet_host_worktree_prep]], [[feedback_tiered_agent_capabilities_destructive_git]].
