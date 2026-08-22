---
index: false
name: project_container_restart_recovery_drill
title: Overnight container restarts — recovery drill
description: The devworkspace container restarted twice on 2026-08-21 (~21:38, ~00:45 UTC), killing mesh + all background agents; /tmp and /home/user/bin wiped. Drill below.
metadata:
  type: project
---

The Che devworkspace container can restart mid-session (twice on 2026-08-21). Effects: every background
agent and shell dies (transcripts survive; UNCOMMITTED work survives in the tree), `/tmp` (mesh, reports,
scratchpad) and `/home/user/bin` (mongod) are wiped, git trips `dubious ownership`, MCP reconnects.

Recovery drill, in order:
1. `git config --global --add safe.directory /projects/elohim` (and `'*'`).
2. Restore mongod: version+sha ARGs in `che-devworkspaces/containers/udi-plus/Dockerfile`
   (`MONGOD_VERSION`/`MONGOD_SHA256`, rhel93 tarball → `/home/user/bin/mongod`).
3. `git status --short` — attribute stranded uncommitted diffs to their dead agents; respawn FINISHER
   agents per crate/tree (path-limited commits), never bulk-commit blind.
4. Regenerate: `hc-mesh.sh start` → wait 3× storage 200 → `prologue` → kill 8888/8889 + `start` again
   (boot-order: doorways must warm AFTER storage) → restart `hc-mesh-perf-watch.sh`.
5. Re-arm waiters/monitors (they die silently).

Lesson enforced by this: agents commit early and path-limited; long cargo builds are the main casualty.
Related: [[feedback_overnight_permission_stalls]], [[feedback_commit_only_integrator_pushes]].
