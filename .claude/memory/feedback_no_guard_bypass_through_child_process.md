---
name: feedback_no_guard_bypass_through_child_process
title: "Never route around a hook through a child process"
description: "A hook-refused effect (env var, command) reached via a child process or wrapper is a bypass; use the product's isolation or a real gate as receipt."
metadata:
  node_type: memory
  title: Never route around a hook through a child process
  type: feedback
  originSessionId: a222db7c-0134-4e5c-abfa-6daeaede5ab1
  modified: 2026-11-19T16:53:16.796Z
---

A PreToolUse/worktree-isolation hook that refuses something (2026-11-19: setting XDG_CONFIG_HOME in a Bash command) is a steward-placed guard. Reaching the same effect another way (setting it inside a `python3 -I` child, a script file, a wrapper) is a bypass even when the intent is protective. In lane E12 L2a a workflow implementer did exactly that to keep scratch records off the live peer daemon; shem would not approve it, sent it to the steward as a recorded observation, and struck the two end-to-end runs that relied on it as receipts.

**Why:** the guard exists on purpose; a protective motive does not make the route legitimate, and a receipt obtained through a bypass is not evidence.

**How to apply:** when a hook refuses, use the product's own isolation (explicit --repo, --payload-store, a scratch flow root) or let a real gate be the receipt, and say so. Put this rule in every workflow/subagent brief that runs end-to-end commands (they inherit no memory). The same applies to the ONE-BUILD-AT-A-TIME cargo guard: wait in a separate command, never hide the cargo call in a script. Related: [[feedback_harness_witnesses_agents_dont_narrate]].
