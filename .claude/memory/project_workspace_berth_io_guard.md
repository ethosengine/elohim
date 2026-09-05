---
name: project_workspace_berth_io_guard
title: Workspace berth + io-guard
description: "berth (moor/claim/release/say — who is here, model@lab, what they hold) + io-guard (write budget 60/100/160 MB/s + PSI; pause/kill tier-1, never critical) live on dev; coordination is a ledger, not chat"
metadata: 
  node_type: memory
  title: "Workspace berth + io-guard — carrying capacity is a write budget; coordination is a ledger, not chat"
  type: project
  originSessionId: 0b015666-05d9-4a1d-bf90-6e1e121f70b3
  modified: 2026-09-03T16:22:24.434Z
---

**Operator ruling 2026-09-03:** sessions coordinating the mesh, the cargo slot and the disk by chat
("mesh taken/free", "hold every cargo") is the wrong organ. *The limits are our carrying capacities at
the workspace level; registering who is doing what, which model from which lab, has to be native; the
deliberation messages and their space belong in the protocol (ephemeral, low stakes).*

**What landed (local dev b870014ea + spec commit; spec
`genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md`):**
- `genesis/agentic/bin/berth` — `moor --session --model --lab --runtime --principal --task --writes`,
  `claim mesh|cargo|disk-heavy` (a live holder is REFUSED by name, exit 3 — never queued; stale holder
  taken over on the record), `release`, `say`, `who`, `status`, `ledger`. Store `$CLAUDE_CONFIG_DIR/berth`.
  Records are Ephemeral (C) in the field shape of their homes: mooring → `delegates-compute`
  Mishpat::Commitment (provider = human, recipient = {model, lab, runtime, session}, bounds = write set +
  ttl); lease → REA `use` commitment; note → witnessed interaction; shed → tevah §5.3 breach event.
- `genesis/agentic/bin/io-guard` — sibling of ram-guard, reuses its tier ladder/classify/kill_tree.
  Level = max(cgroup io.stat write MB/s over 30 s, host `/proc/pressure/io full avg10`); 60/100/160 MB/s,
  PSI 10/20 %; 20 s dwell; high = SIGSTOP the top-writing tier-1 tree (one/tick), hard = kill tier-1 then
  pause tier-2; critical/unknown never; auto-resume oldest-first after 30 s quiet; every action → its
  events.jsonl + a berth `shed` row attributed to the `cargo`/`disk-heavy` lease holder. `guard_mode` in
  pool-policy.json `io` (now `enforce`; `IO_GUARD_MODE` overrides). SessionStart hook (`ram-guard.py`)
  ensures both daemons, auto-moors the session (model unknown until the agent completes it), prints
  IO-GUARD + BERTH lines; UserPromptSubmit touches the mooring (liveness).
- First live reading: a `cargo test` the sensor called 215–257 MB/s (72–86 real — the first cut summed
  io.stat over md2 AND both mirror members, 3×; fixed to max-over-devices), host PSI full 23 % — hard on PSI.
  After enforce it shed two rustc trees of a just-relaunched build because dwell carried over the daemon
  restart (fixed: dwell resets on start); the owner relaunched with `CARGO_BUILD_JOBS=2`.

**Why (corrected):** the 10:20Z pod swap was NOT a write storm ([[project_devspace_recovery]] — dqlite SEGV
loop + a stale secretless ReplicaSet); the host's real I/O failure mode is latency on worn QLC NVMes, so PSI
is the guard's primary axis. The cargo dev profile was ALREADY trimmed (`debug = "line-tables-only"`, deps
`debug = false` in root `.cargo/config.toml`). The coordination gap the operator named stands regardless.

**How to apply:** before any heavy work in this workspace: `berth moor …` once per session, then
`berth claim cargo|mesh|disk-heavy` (obey a refusal), `berth release` after; use `CARGO_BUILD_JOBS=2` for
builds beside a running mesh; read `berth status`/`io-guard status` instead of asking peers by message.
A shed row with `holder: null` means someone skipped the claim. Graduation (spec): runtime-asserted
session identity → quota verb inside `ark` with `io.max` → moorings as signed `delegates-compute`
commitments → the workspace as matthew's device peer ([[project_tevah_compute_envelope_canonized]],
[[project_rea_compute_commitment_primitive]]). Operator-side fixes still open: lengthen the
devworkspace-controller leader-election lease/renew deadline, and move the k3s datastore or the
workspace PVC + cargo pool off the shared NVMe.
