---
name: project_ram_guard_oom_group_kill
title: RAM guard — workspace OOM is a group kill
description: memory.oom.group=1 makes one hot rustc restart the whole workspace; ram-guard sheds builds first — bites when a build dies with signal 15
metadata:
  type: project
---

The devworkspace container cgroup ships with `memory.oom.group=1` (kubelet cgroup-v2 default), so
ONE OOM event at `memory.max` (31 GiB live; devfile says 30Gi) kills every process AND PID 1 —
the whole workspace restarts (2026-08-29 17:12Z: 35 processes, trigger = rustc 4.3G + 7 rust-lld
on top of 3 conductors). The pod is privileged (full caps, cgroupfs rw), so this is fixable inside.

Guard (built 2026-08-29): `genesis/agentic/bin/ram-guard` daemon + `.claude/hooks/ram-guard.py`,
policy `pool-policy.json` `ram` block (soft/high/hard 70/80/88 % of COMMITTED = anon+kernel+shmem;
page cache is reclaimable and must not count — `memory.current` alone over-reads by ~10 G).
Kernel steering: oom.group→0, memory.high→94 % max, oom_score_adj builds 1000 / critical 0.
Store: `/projects/.claude-config/ram-guard/`. `just status ram`, `ram-guard plan` (dry).

**Why:** the OOM-group kill is the single most disruptive event in the workspace — mesh churn
≈20 min, every agent session lost. Measuring `memory.current` would false-fire on page cache.

**How to apply:** when a build dies with `signal: 15`/`Terminated`, check `ram-guard status` —
it was shed, not flaky. Never move the cgroup steering into devfile postStart
([[feedback_no_brittle_commands_in_poststart]]); the SessionStart hook is the auto-start path.
The auto-mode classifier blocks `ram-guard start` from an agent Bash call — the hook or the
operator starts it. Related: [[project_cargo_pvc_disk_discipline]], [[project_devspace_recovery]].

**Genesis captured (2026-08-29):** the guard's six fields = the missing fields of a
`delegates-compute` compute envelope for virtual peers / rakia dispatch — backlog
`genesis/data/timeline/backlog/2026-08-29-compute-envelope-virtual-peer-contract.md`; rakia side in
`elohim/rakia/docs/plans/stage-2-canopy.md` (submodule — operator commits there); governance side
`epr:alpha-test-bench-compute-envelope`. Don't re-derive; pick up rung (a) from the entry's `shift_objective`.

**Telling an OOM from a node reboot (2026-08-29 20:54Z):** from inside the workspace both look the same — new pod UID, PID 1 minutes old, `/tmp` gone, every process dead. Discriminate before blaming memory: `cat /proc/uptime` (host seconds, not container) and Prometheus `node_boot_time_seconds{instance="192.168.86.100:9100"}`; the guard's `guard.log` shows the last LEVEL line and the cgroup `memory.events` restarts at zero either way. On 2026-08-29 the host itself rebooted at 20:54:40Z with the pod at ~22 G / 31 G and the guard never above 70 % since 20:23 — not an OOM. The SessionStart hook re-armed the guard in the new pod (`oom.group` 1→0, `memory.high` set) at 21:05:59 without anyone touching it.
