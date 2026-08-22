---
index: false
name: project_alpha_conductor_spin_root_cause
title: Alpha conductor spin — sys-validation retry loop
description: 2026-08-21 — every alpha storage pod pegged at CPU quota for 48h+ = the conductor's sys-validation spinning on 353–2849 unfetchable dependencies; the pool saturation (Util 4037%) came from an UNBOUNDED join_all local re-lookup, not the network fetch. Cure 1 lives on fork branch fix/sys-validation-unfetchable-deps-backoff (pointer not bumped).
metadata:
  type: project
---

Symptom: `container_cpu_usage` == quota on every `elohim-node` pod regardless of limit (2/3/4/8 cores), CFS
throttled 97–100 %, ~1000 conductor log lines/s per pod (`holochain_sqlite::db::access: Database read
connection is saturated. Util 4037%`, `holochain_cascade: No peers to fetch record from
NoPeersForLocation`, `Sys validation sleeping for 10s, with 0 fetched of N missing dependencies`, N never
draining). Storage's own activity is ~0 (zome calls 0.1/s; admission in-flight 0–2/5).

Root cause (fork agent, file:line in backlog `alpha-conductor-sys-validation-spin-unfetchable-deps`):
no per-dependency retry budget (upstream TODO at the miss site); the LOCAL re-check pass filtered on
`!has(hash)` which is false for a known-missing dep, so every missing dep was re-looked-up locally via
`futures::future::join_all` — unbounded concurrency against an 8-reader pool (2849 × 2–4 stores); the
saturation line logged per acquisition attempt (rate = request rate). Full-arc (`target_arc_factor: 1`)
means every network get is `NoPeersForLocation`, so the set is permanently unfetchable (fossils from the
2026-07-24 re-key are the suspect).

Cure 1 (fork, branch `fix/sys-validation-unfetchable-deps-backoff`, `b9c7458ae` + `c9a6c4439`): per-dep
exponential backoff (local ≤60 s, network ≤1 h, jittered), `buffer_unordered(10)` local fan-out,
unfetchable after 12 network misses (reporting state; never dropped, never validated), saturation log
once/30 s with suppressed count, OTel metrics `hc.conductor.sys_validation.{missing,unfetchable}_dependencies`
+ `hc.db.connections.read_saturation`. Release binary: pool `crates/dev/release/holochain`. Ships via
the submodule pointer bump → `elohim-conductor` image (integrator's move).

**Open node B:** the fork bounds the sweep; it does not DRAIN fossils — `AwaitingSysDeps` ops never
integrate. Assertion needed: `unfetchable_dependencies` → 0 on every pod; actor = `DumpFullState` to
identify the set + lineage-preserving migration. Relief now: `RUST_LOG` directive dropping those two
targets below INFO. Mesh reproduction: `hc-mesh-chaos-rekey.sh` (staging v2: neighbours SIGSTOP'd, a
dependent chain authored, tail-only delivery, re-key) — the mesh conductors need `RUST_LOG=info` for the
diagnostic lines (default `hc sandbox run` logs ERROR only: the log leg is BLIND without it).
Related: [[project_conductor_storm_starves_storage_reads]], [[project_full_arc_authority_disables_network_get]],
[[project_ghost_declaration_deadlock_batch3]], [[feedback_mesh_is_the_proving_ground]].
