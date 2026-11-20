---
name: project_device_status_false_degraded_test_leases
title: "False 'degraded peer daemon' from a leaked test lease"
description: "A test daemon's lease in the HOST bin store made `epr device status` read degraded for days (2026-11-17..20); check lease owners before restarting the real peer."
metadata:
  node_type: memory
  title: "False \"degraded peer daemon\" from a leaked test lease"
  type: project
  originSessionId: a222db7c-0134-4e5c-abfa-6daeaede5ab1
  modified: 2026-11-20T14:09:52.136Z
---

`epr device status`'s peer-daemon line judges every live `daemon-<pid>-<start>.json` lease under `/projects/.claude-config/tools/bin-store/epr/sessions/`. The a2o scenario devices-share-one-berth (genesis/a2o/steps/devflow/devices-share-one-berth.steps.ts) leaked an `epr peer serve` (ppid 1, --root under /tmp/devices-share-one-berth-*) whose lease landed in the HOST store, because the bin-store root is not isolated by the fixture's HOME/XDG. Status read "running bafkr4iaatfp…; restart epr peer serve" from it for ~3 days while the real ark child was fine.

**Why:** a false "degraded" sends an operator to restart a healthy peer and hides the real state.

**How to apply:** when status says degraded peer daemon, read the lease named (`grep -l <cid> …/sessions/*`), check its ownerPid's cwd/environ before acting; a /tmp-rooted orphan is a test leak: SIGINT it (never SIGKILL without asking), and the lease then reads dead by itself. Backlog row recorded by shem (2026-11-20). Also: this session's pinned epr can predate status features; read status with the store's current binary by path when it matters.
