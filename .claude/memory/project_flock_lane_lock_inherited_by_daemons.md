---
index: false
name: project-flock-lane-lock-inherited-by-daemons
title: "Never start mesh daemons inside a flock'd shell"
id: project-flock-lane-lock-inherited-by-daemons
description: "`just mesh start` inside `flock lane.lock …` makes every detached daemon inherit the lock fd — the lock never releases and all later lane waiters hang silently"
metadata: 
  node_type: memory
  title: "Never start mesh daemons inside a flock'd shell"
  type: project
  originSessionId: dcddc033-024b-4dfa-8d13-39fa5f75b9ef
  modified: 2026-09-13T08:34:06.626Z
cites:
  - app/elohim-app/scripts/hc-mesh.sh
---

On 2026-09-13 a cold household restart was run as `flock /tmp/elohim-local-mesh/lane.lock bash mesh-cold-remeasure.sh`; `just mesh start` detaches conductors, storage peers, doorways, beacons and the portal, and every one of them inherited the open lock file descriptor. When the chain exited, the daemons still held the lock, so every later `flock lane.lock just test …` blocked forever with an empty log and no child process (`fuser lane.lock` lists the daemons; the waiter shows no descendants).

**Why:** `flock(1)` locks the file description; a detached child that keeps the fd open keeps the lock for its lifetime.

**How to apply:** never run `just mesh start/stop/*-restart` (or `hc-mesh.sh` restarts) inside a flock'd shell; take the lane lock only around lanes and prologues. If it happens, stop the waiters (`pkill -f "flock <lock>"`), switch lane serialization to a fresh lock file, and leave the daemons alone. A hung waiter with no children is the signature. Related: [[project_local_mesh_binary_slot_and_restart]], [[project_devspace_recovery]].
