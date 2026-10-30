---
title: "Integration capacity grounding — the limit canon, the hard device limits, the fixture model, and where the bottleneck is"
id: integration-capacity-grounding-2026-10-29
status: active
class: research
date: 2026-10-29
steward: human:matthew
author: "Claude (orchestrator, Fable 5.1); grounding run wf_94c4bd4b-8c2 — seven seams (limit canon, hardware limits, alpha fixture limits, integration bottleneck, parallel Jenkins, native distribution, workspace contention), each adversarially verified before synthesis"
tags: [capacity, limitarian, carrying-capacity, vsm, jenkins, rakia, berth, ark, grounding]
cites:
  - genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md
  - genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md
  - genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md
  - genesis/agentic/pool-policy.json
  - genesis/manifests/cluster-state.yaml
  - genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md
---

> **How to read this.** Every number is tagged measured (M, with date and probe), declared (D, with the file) or unverified (U). Where a verifier corrected the reader, the corrected value stands. The ranked options in §6 fed the design spec `genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md`; its §10 carries the fix-in-flight list. Outputs (mint pass): the design spec's sprint slices S1a–S4 and its §10 rows are the backlog-bearing takes; nothing here is a standalone backlog spray.

# Compute limits, native reading, and how to widen the integration bottleneck without adding nodes

Synthesis of seven read-only seam groundings and their adversarial verifications, all run 2026-10-29 between ~02:05Z and ~02:30Z on `/projects/elohim` (HEAD `ee203b8b1`, remote `dev` at `e67c6cef8`). Where a verification corrected a grounding, the corrected value is used; where neither could settle a figure it is marked **unverified**. Numbers are tagged **M** (measured, with date/probe), **D** (declared in a file), or **U** (unverified).

---

## 1. The canon on limits

**The economic shape.** `genesis/docs/content/elohim-protocol/manifesto.md:313-323` states the Raworth ring: a floor nobody falls below and a ceiling nothing overshoots, where "*the curve between the two is a gradient of friction, not a wall: freedom is widest at the floor and tightens as an act approaches the ceiling*" (`:323`), and "*the ceiling is the part we cede*" to something whose position does not depend on the answer (`:315-319`). `:1016` names it limitarian "*graduated by friction rather than set as a flat cap*". `succession.md:72` fixes the two planes: "*never a computed payout at the ceiling, never judgment at the floor*"; `:341` adds that "*carrying capacity is erodible … overshoot becomes witnessed and priced, not … impossible*"; `:529` (Stance IV.2): "*To not respect the limits is to limit your own reach.*"

**The governor as code.** `genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md` is the mechanism: a scale-invariant concentration measure, super-linear friction, walls that are "*reject-at-write or they are nothing*" (`:339-351`), and a subsidiarity table that ratifies **compute** at Household→Community (`:390-398`). The walls are real and confirmed: `elohim/elohim-storage/src/services/limit_gradient_registry.rs:16-20` and `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs:888-946` (the reject-at-write validator — so the wisdom spec's and the coupling-delay spec's "zero `.rs` hits" claims at `2026-06-09-wisdom-layer…:39,217` and `coupling-delay…:112` are stale). `arc_actuator.rs:421` is `impl elohim_compute::Governor for ArcGovernor` — Rung 3 of the VISION-DESIGN ladder is landed for the arc lever (the VISION docs live at `genesis/docs/superpowers/specs/2026-06-14-elohim-substrate-passes/`, both still PROPOSAL). All three `limitarian-governor.feature` scenarios are `@wip`; the wall widths are "TBD-OPERATOR" by the registry's own comment (`:9`).

**Carrying capacity as a workspace discipline.** Two operator rulings carry the whole compute canon:

- `genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md:28-36` — "*the limits are our carrying capacities at the workspace level; registering who is doing what … has to be native.*"
- `:129-131` and `genesis/agentic/pool-policy.json` (`_dont_restrain_by_accident`, `:84`) — "*manage our own carrying capacities as a discipline of the VSM primitives in our compute, but make sure we don't restrain ourselves by accident*" / "*never restrain ourselves on a number the disk does not feel.*" The capacity that matters is **I/O latency headroom read as host PSI**, committed RAM second (`:38-50`).

The berth spec's VSM table (`:129-150`) places S1 = sessions/builds/mesh runs; S2 = leases (one mesh, two cargo, one disk-heavy; refusal names the holder, never a queue); S3 = the mooring (bounded delegation to a named agent); S3* = ledgers that "*audit, never act*"; algedonic = host PSI + committed memory; S4/S5 = the operator (topology, lease durations, what counts as capacity — "*failure: delegated to a daemon*"). The 2026-09-13 ruling (`/projects/.claude-config/projects/-projects-elohim/memory/feedback_limit_raises_are_design_signals_not_capacity.md:17-34`) completes it: "*those limits creeping up to patch over an emergent problem that needs a design pass*" — a raise is "*the algedonic signal and must reach the design plane (S3/S4), not be answered by S1.*" It is wired as rule `resource-limit-raise-design-signal` (`genesis/orchestrator/data/.epr-meta:17-107`, class `ask`) and lens `resource-limit-raise-recurrence` (`.claude/epr-meta/measures.yaml:1490-1509`, hard 2 → BREACH). One stale note: the rule's inline rationale (`.epr-meta:109-118`, "policies.yaml at 99.1% of 65,536") is obsolete — `policies.yaml` is 72,808 bytes under a 1 MiB registry cap (`eprfs-meta/src/lib.rs:44`).

The Meadows reading (`genesis/research/meadows-systems-dynamics-cross-pollination-2026-08-11.md:19,311-327`) names the gap this document keeps hitting: "*our limits are scalars where Meadows shows they must be rates against rates*" and "*a model's carrying capacity is a vector, not a window*" (Liebig's most-limiting input). The holonic pivot (`…/plans/2026-08-12-holonic-capacity-measure-convergence-pivot.md:335-376`) rules "*a verdict is a homeostat, not a gate*" with a revisit cadence — but `elohim/epr/src/verdict.rs:51-60` `ReferQuestion{layer, reason, note}` carries no `Cadence` field; that part is spec-only.

**The decision procedure when a limit is hit.** The canon never prescribes "raise". Assembled from where each step is written, in order: (1) **sense before the line** — band-edge `approaching` with hysteresis (algedonic spec `:62-88`; carrier is the `CommitmentByState` link, declared at `mishpat_integrity/src/lib.rs:394`, but `algedonic::should_emit` at `elohim/epr/src/algedonic.rs:381` still has no production caller); (2) **refuse-and-elevate naming the owner** — `Refusal{code, limit_owner, elevate}` (`elohim-compute/src/actuation.rs`), berth refusal by holder, arc actuator refuses a flip that opens a coverage gap; (3) **shed in declared order, witnessed** — tevah §5.2 / ram-guard tiers / device-footprint "cache → window → refuse new carrying; running, pinned, carried never shed"; (4) **read the raise as a design signal** (the three questions in `.epr-meta:100-107`); (5) **attenuate at the seam we own** (sender back-off, boot profiles, sequenced rolls, the arc lever); (6) **split rather than raise** ({0,1} leecher flips, light app per hosted human, measure-class runs to a peer, a second workspace = a second set of container slots); (7) **descend** — Meadows §6.3 "*quicken the response rate or slow the growth rate*"; (8) **revisit on cadence**; (9) **overflow emits outward** (RECURSION §3.1, aspirational). **It is written nowhere as one procedure**: it is distributed across the berth spec, tevah §5.2, device-footprint §5, the limit-raise rule, the saturation backlog atom and the holonic pivot §6b. Steps 1, 8 and 9 are implied or spec-only; 2–7 are live in at least one seam.

---

## 2. Hard device limits

Probes: Prometheus via the observability MCP (`datasourceUid: prometheus`), container cgroup files, `/proc`, `ram-guard status`, `io-guard status`, `cargo-pool status`, all 2026-10-29 ~02:05–02:30Z unless dated. 7-day windows end at probe time.

### ethosengine (192.168.86.100 — performance; dev workspace, Jenkins controller, alpha trio matthew/jessica/james)

| Axis | Value | Tag |
|---|---|---|
| CPU | Ryzen 9 3900X, 24 threads allocatable; node 7d peak 74.2%, load15 8.7–9.7; pod CPU requests 10.35 c, limits 25.4 c (106%) | M 2026-10-29 |
| RAM | 59.4 GiB capacity (MemTotal 62.3 GB); `memmap=2032M$16M` reserves 2 GB away from the OS; requests 19.8 GiB (33%); **limits 67.7 GiB = 114% of allocatable** (contradicts `compute-capacity.json` "memory is NOT overcommitted" — the ledger sums only humans); working set now 37–41 GB, 7d peak 47.1 GB; MemAvailable 7d min 15.0 GB; **73 OOM kills in 7d** | M |
| Disk | `/projects`+`/nix`+microk8s on md127 = RAID1 2× WD SN7100 4 TB, 29% used, 2.6 TiB free; root `/` and **swap (4 GB, 964 MB used)** still on the worn Crucial P1 QLC pair (md2/md1) | M |
| Write endurance | md127 26.1 MB/s 24h avg ≈ 2.25 TB/day ≈ 820 TB/yr per SN7100 (TBW rating **U** — not on disk); md2 3.7 MB/s still landing on QLC; QLC at 108%/130% of rated TBW per 2026-08-29 evidence (**U** today, no SMART from the pod) | M/U |
| I/O headroom | host PSI full 7d max 8.8% (vs 77% before the 2026-08-29 resets); now ~0.4–0.7% | M |
| Reliability | **new: 2,044 uncorrectable ECC deferred-scrub errors on `mc#0 csrow#3 channel#0` at 2026-10-28 ~19:52Z**, pages 0.9 MB→2047 MB; 2,029 of them inside the `memmap` window, 15 below it; CE=0; no panic. The 2026-09-27 "MCE bank-0" theory now has a more specific competitor: one DIMM. Which slot: **U** (no per-csrow sysfs, no rasdaemon from the pod). `softlockup_panic=1`, `panic=10`, kdump armed | M |
| Guards in front | none at host level readable from here; the Che cgroup below; k8s requests only (limits overcommitted) | — |

### shem (10.99.0.2 over wg0 — remote; adam/gertrude/susan/eve + doorway-B; booted 2026-10-25T22:19:51Z)

| Axis | Value | Tag |
|---|---|---|
| CPU | 24 threads (dual Ivy Bridge Xeon, x86-64-v2); node 7d peak **94.7%**, load15 ~20–21; requests 12.9 c; **limits 36.7 c = 153%** (above the ratified 125%) | M |
| RAM | 118.2 GB MemTotal (110 GiB allocatable; the memory file's "135 GB" is **not what the kernel sees**); requests 39.9 GB; limits 84.8 GiB; working set now 28–33 GB, 7d peak 51.8 GB; MemAvailable 7d min **6.45 GB**; **213 OOM kills in 7d** | M |
| Largest tenants | a second Che workspace `workspace0d7b60ba2d3247a9`: 23.8–25 GB working set, limit 41.25 GiB / 8.5 cpu, using ~8 cores (active or idle: **U**); elohim-alpha 11 pods, 30.9 GB requests (largest single commitment in the cluster) | M |
| Conductor CFS throttle (1 h) | eve 83–99%, gertrude 80–94%, susan 3–5%, adam 0.3–0.5% — moving; quote as a range | M |
| Disk | root LVM SATA 75% used; ZFS `tank` fill unreadable by node-exporter; `storage-data-elohim-adam-alpha-0` PVC **100%**, `elohim-adam-alpha-compute` 71%, holochain-data 31–34%; sdd/sde **81.2 MB/s each** (~7 TB/day; replication vs churn **U**) | M/U |
| Guards | `remote-wan=true:NoSchedule` taint (no Jenkins pod tolerates it → never a CI builder); archetype floors only via epr-meta `ask`; `devfile-shem.yaml` on dev but **ISA-blocked** (UBI10 x86-64-v3 image) — yet a workspace is running there (image provenance **U**) | D/M |

### CI builder pool (intel-nuc + thinkc-p0t/p0h/p1s; hp-micro10 serves the registry)

| Node | CPU / alloc RAM | 7d CPU peak | 7d min MemAvailable | 7d IO PSI full max | OOM 7d | Role |
|---|---|---|---|---|---|---|
| intel-nuc .102 | 8 c / 15.23 GiB | 99.99% | 1.45 GB | 8.1% | 0 | **de-facto builder**: every app/edge/DNA-main pod this week; doorway-A, mongodb, keycloak resident (~2 GiB req / 9.8 GiB lim) |
| thinkc-p0t .110 | 4 c / 7.55 GiB | 99.6% | **0.48 GB** | **79.8%** | 0 | orchestrator/genesis/storybook pods, sweettest shards, alpha site |
| thinkc-p0h .112 | 4 c / 7.55 GiB | 99.5% | **0.25 GB** | 57.4% | **18** | eclipse-che, argocd, shards |
| thinkc-p1s .111 | 4 c / 7.55 GiB | 36% | 3.5 GB | 7.8% | 0 | openebs, harbor-trivy |
| hp-micro10 .101 | 2 c / 15.0 GiB | 96.9% (93% now) | 11.8 GB | 46.3% | 0 | harbor registry (2.8 GB), nexus, garage; Jiva replicas — every CI pod pulls `:latest` with `imagePullPolicy: Always` |

All M (Prometheus, 2026-10-29). Builder pod envelopes (D): app 6Gi→10Gi, **no CPU request**; edge 4Gi/1c→8Gi/4c; DNA main 4Gi/1c→8Gi/4c; sweettest shards ×4 at 2Gi/1c→4Gi/2c (12/24 Gi ephemeral); eprfs/epr 4Gi→8Gi; steward 8Gi/2c→16Gi/8c; sophia 8Gi→16Gi on `node-type: performance` (ethosengine); genesis 2Gi→4Gi and orchestrator 0.75Gi on `node-type: edge`. Measured 7d working-set peaks (M): app builder 5.88 GiB (16 of the top 18 pods are app builds at 5.3–5.9), edge 5.85, DNA main 5.94, shard 3.3, eprfs 5.23, Jenkins controller 3.72 GiB — the 6Gi app request sits exactly at its peak; the 4Gi edge/DNA requests sit ~1.9 GiB under theirs. Max concurrent Jenkins agent pods in 7d: 11. Cluster totals: 70 c / 238 GB allocatable; requests 30.4 c / 73.9 GB; limits 86.3 c / 207 GB; microk8s v1.29.15 (EOL). `genesis/data/rakia/compute-capacity.json` is **three strata in one file**: node allocatables 2026-10-21T20:38:29Z, commitments/actuals 2026-09-05, humans block pre-2026-06 ("shem decommissioned", adam STRANDED) — cite the stratum.

### The Che workspace cgroup (this pod, `tools` container)

| Axis | Value | Tag |
|---|---|---|
| CPU | `cpu.max 1050000/100000` = **10.5 cores** (devfile `cpuLimit 10` + sidecar; `nproc` says 24 — the host's cpuset); throttled 1.1% of periods, ~32 min/day; average use 1.95 c | M |
| RAM | `memory.max` 31.0 GiB (devfile says 30Gi; the +1Gi is **U**); `memory.high` 29.14 GiB (94%, steered); `memory.peak` 10.7 MB **above** high; `memory.events high` 125,349 since 2026-10-23 10:47Z ≈ 22,200/day; `memory.swap.max 0`; committed 18.2–21.8 GB (58–66%); kernel 3.0 GiB (the grounding's 4.98 was the host root cgroup) | M |
| Disk | `/projects` 29% of 3.6 TiB; cargo pool **141 G of `pool_max_gb` 150** (94% of its own cap); `cargo-pool status` runs in <90 s | M |
| pids | 602 current; `pids.max` **U** | M/U |
| Guards + watermarks | ram-guard soft/high/hard **70/80/88%** of committed (22.2/25.4/27.9 GB), `memory.high` 94%, `oom.group=0` re-asserted every tick, tier1 cargo → tier2 JS → tier3 dev servers; critical/unknown never shed — the 7.3 GB `elohim-storage-seed-row-…` is **unknown** (comm truncated). io-guard PSI full avg10 high/hard **10/20%**, kill after 90 s, write bands 60/100/160 MB/s advisory. cargo-disk-guard: deny at PVC 92%, deny without `CARGO_TARGET_DIR`, deny at PSI full10 ≥20 / some60 ≥50, **max_concurrent_heavy 2**, jobs cap default 4. Pre-push: `cargo-pool enforce` at 88%, `DEFERRED-BY-PVC` at 92%. Berth: mesh 1, cargo 2, disk-heavy 1, device-ark 1; steward-scoped dev-integration / jenkins-dispatch / alpha-fleet / lane: 1 each; verify ttl 1800/3600 s; mooring ttl 10800 s. `CARGO_BUILD_JOBS=1` for elohim-storage at `elohim/holochain/build-manifest.json:319-322` | D (all in `genesis/agentic/pool-policy.json`) |

Guard activity (M): ram-guard 135 sheds 2026-08-29→**2026-10-07**, 0 in 14 days, 125 at `high`; cargo+clippy 123 events, median tree age 79 s, 4.5 h of compile discarded. io-guard 0 actions since 2026-09-14. Berth: 12 refusals in 7d (11 on `dev-integration` 2026-10-27 00:48–01:02); **`dev-integration` is held right now by shem's device since 02:16:47Z**; a `push` lease sits EXPIRED after 801 h of a 30-min verify ttl; 268 mooring files, 1 live.

---

## 3. Modeled fixture limits

### Cost model

| Quantity | Value | Date / source | M or modeled |
|---|---|---|---|
| Hosted human, household | 786 MB conductor heap (22.8 GB / 29 personas; ~157 MB × 5 cells) | 2026-09-11, `genesis/seeder/src/seed-humans.ts:102-115` | M once, a quotient |
| same, snapshot | 704 MB/agent, ~137 MiB/cell (matthew 16 agents = 12.47 GB) | 2026-10-02 shem handoff `:78-90` | M once |
| same, smaps | +185 MB `r-xp` (wasm duplicated per agent, ~26% executable) + 465–580 MB `rw-p`; +112 MB idle in 10 min | 2026-10-08, `arch-scale-risk-backlog.md` row 7/14 | M, household, fork c916eddb |
| Hosted install, fleet | ~0.7 GiB each; adam 1.17→3.18 GiB for three; matthew OOMKilled at 5120Mi 2026-10-15 19:52:53Z | `doorway-hosted-capacity-bound-from-pod-memory.md:55-60` | M |
| Counter-evidence | eve at ~10 hosted apps ≈ 1.7 GiB total — "must not size any decision" on 786 MB | `2026-10-03-plane-separation-design.md:81-84` | M; **the one-registration smaps probe (row 14) has never run** |
| Idle conductor | 1–1.5 GiB fleet; 292 MB right after restart (ark), ~2 GB loaded household | memory `project_conductor_arc_resources.md:26`; `hc-mesh.sh:3451` | M |
| Growth, stock 0.7 | ~2.0→4.3 GB per conductor over 6h46m (01:24→08:10Z 2026-09-04) ≈ 0.33 GB/h, 12.6 GB for three; "~14 GB overnight" | `arc-shrink-ineffective-memory-soak.md:36-40`; row 4 | M (stock); **fork `hc-fct-ccda82d3d` growth/h U** |
| Household at boot | conductors 2,382 + storage 1,065 + doorways 249 + mongo 147 + relay 7 = **3,850 MB**; + portal `ng serve` 792 MB (uncounted by `mesh_footprint`) | `household-dowell/logs/start.log:115-127` 2026-10-26; ram-guard event 2026-10-07 | M |
| Household loaded | 5.2–6.9 GB household-only; Oct logs print 7.3–13.8 GB because the T3 conductor + :8095 storage are co-counted | logs; `i1006/logs/start.log` | M |
| Storage gate | build peak 15.42 GB at default jobs (= 24 rustc on a 10.5-core quota, ~2.3× overcommit) → 11.6 (4) → 9.6 (2) → **5.28–5.49 GB at jobs=1**, +12.0% wall (813.9→~911 s); test phase 1.7 GB | T8 2026-09-08, `cargo-test-memory-shed-storage-gate.md` | M once |
| eprfs gate memory; a2o lane RSS (cucumber+chromium) | — | — | **never measured** |
| Orphaned fixture doorway pairs | 3 mongod + 6 doorway = 1.15 GB after a lane | `before-hook-timeout-orphans…:23-30` 2026-09-20 | M |

### Ceilings

| Ceiling | Value | Evidence |
|---|---|---|
| Doorway hosted-agent ceiling | code default **50** (`pool_map.rs:45`); alpha-B **32** (`manifests/doorway/alpha-b.yaml:407-408`); alpha-A unset → 50 — exactly the `(50/50)` refusals in genesis #1699–#1708; household A **14**, B/C 0 (`hc-mesh.sh:5572,5630,5702`) | D + M |
| Heap-derived bound | (5.0 − 1.5)/0.7 ≈ **5 hosted humans per 5120Mi conductor**, not 50; "a hosting conductor reaches its limit about every second genesis run" | 2026-10-16 atom, modeled from M |
| Arc-convergence stall | "~30 agents" (`main.rs:470`) ← ONE `Updating storage arc to full` in 14.5 h across ~35 agents | 2026-07-30, **0.6-era adam**; not re-measured on the 0.7 fork |
| Household DHT wedge | ~115–120k ops; three conductors ~5 cores; `NoPeersForLocation` ≈1,000/10 min | 2026-09-18 |
| Fresh joiner Empty→target | ≈56 min on a 197,467-op space | 2026-10-16 row 13 |
| Fleet 7d working-set peaks | matthew 4,914 / adam 4,780 of 5,120 MiB (96%/93%); 24h: susan 99.7%, eve 99.3%, gertrude 96.6% of limit; storage jessica 99.9%, matthew 98.8%, eve 98%, gertrude 96.9% | M Prometheus |
| Three-conductor RAM interplay (workspace) | floor 7.5–9.5 GB + household loaded 9–13 GB + capped storage gate 5.5 GB + lane ≥1.5 GB = **~27 GB ≈ 87% → hard**; at boot size 19.6 GB (63%), one hosted prologue (4.7 GB) or ~2 h of stock-conductor growth eats the margin to `high`; uncapped gate = 37 GB > `memory.max` | arithmetic on M |
| Conductor admission | permits = max(2·cpus, 8) − 3 = **5** on every storage pod; capacity 17→34 experiment: λ flat, W ×2.06 — "oversubscribed, not undersized" | `conductor_admission.rs:95-125`; M live |

### Serialization constraints

| Constraint | Household | Alpha fleet |
|---|---|---|
| Lease | `mesh` capacity 1; verify 1800/3600 s with `timeout` fence; measure refused (→ `just measure`); `mesh` daemon class | `alpha-fleet` steward-scoped capacity 1 (pre-push claims it for `[build:edge]`); Jenkins is **not** a berth participant (`ci-as-steward-channel-participant.md` open) |
| Restart churn | fork e0bfc6c7a: conductor swap → zome path live in **1m00s–1m10s**, cold 0m53s–1m00s (the 14–18 min figures are pre-fork 25dd2d0be); write-readiness 463–5757 ms after the 2026-10-18 fix (21–43 s before) | I6 "~20 min" addressing churn (2026-07-11); hours-scale past `divergentAnchor` ≈200 (2026-08-01: 2 h+; 2026-08-06 full roll: 2.5–3 h dark); roll gate ceiling 900 s/peer, rollout 600 s; edge #1747 2026-10-28: jessica RELEASED 333 s, james DEADLINE 591 s, adam DEADLINE 900 s |
| Quiesce gate | — (no CI stage touches the household) | `fleet-quiesce-gate.sh`: deadline 2700 s, poll 60, sustain 330, tolerance default 0 (caller sets 2 at `run-dataplane-validation.sh:96`); predicate on storage-A only; floor verdict 361–372 s; `no_measure` at 2649–2651 s on #1653/#1655/#1657 |
| Hosted growth reset | `MESH_RESET=1`; `conductors-restart` ~2 h | sequenced roll landed 2026-09-22 (`Jenkinsfile:1019-1045`), "not yet proven on the fleet" |
| Live state at probe | **household not running**; resident instead: T3 join-alpha conductor 4.5 GB @128% CPU + campaign-1.4 seed storage 7.3 GB @46% = 11.8 GB, no mooring names them | **all four shem storage containers (adam/eve/gertrude/susan `elohim-node`) crash-looping now** — ready=0, `last_terminated_reason=Error`, ~35 restarts accrued within 6 h; cause **U** (Loki not read; first suspect `self-heal-adam-storage-breaker-compute-worker-readiness-coupling.md`) |

---

## 4. Where the bottleneck actually is

**Critical path of one dev push (measured, 2026-10-24→29):**

push → **pre-push, local** (serialized by the `dev-integration` steward lease + a serial gate loop at `gate-runner.mjs:624` + RAM/PVC admission): **1,475 s (25 min) warm with reuse** … **4,407–7,126 s (73–119 min) cold** (`push-r3c/d.log`; live tap 2026-10-27/28: 787–5,331 s, **3 of 8 runs `interrupted`**); elohim-storage 642/2,613/2,602 s on consecutive days, eprfs 547–1,518 s, fails 13/20 locally → can reject at the ref lock after the window (2026-10-25) → **orchestrator queue** (`disableConcurrentBuilds()`, `genesis/orchestrator/Jenkinsfile:2478`): 0 s if idle, **79.2 min for #2174 today** (`blockedDurationMillis 4,749,740`, lock-bound, not capacity-bound) → orchestrator overhead ~8 min → **Level 0 in parallel**: eprfs 24–33 min; DNA 77–83 min (65 min sweettest, 2-CPU/4 GiB shards); **edge 72–154 min** = build 56–80 min on every commit (image tags are commit-derived) + `awaitDnaVerdict` 0 s today + alpha roll **14 min storage-only / 72 min with conductors** (#1747: 71.9 min deploy, two of seven gated peers burned their full deadline) → **Level 1 app 19–20 min** (readiness 5.4 s, release 615 s; adopt leg fails `APP-NOT-ADOPTED adam … HTTP 503`, circuit open → UNSTABLE, **0/10 delivered**) → **Level 2 genesis 33–46 min** (UNSTABLE on `conductor-1 at capacity (50/50)` and `create_human` zome timeouts ×24) → detached validate-only sibling 22 min (quiesce 6 min floor … 44 min no-measure). Execute Builds 210.7 min on a coupled wave; **p90 218.3 min, p50 132.9, delivered 5/10** (`delivery-series.mjs --window 10`, OUTSIDE BOUNDS). Per-stage splits inside edge/DNA/genesis come from one `wfapi` read and are **U**; job-level durations (154.3 / 71.8 / 77.2 / 35.7 / 218.3 min) are confirmed.

**Single longest wait:** on the wave, `elohim-edge` at 154 min — half CPU (the 56–80 min rebuild) and half fleet convergence (the 72 min conductor roll). For a push arriving mid-wave, the 79-min orchestrator queue is the longest wait and is purely lock-bound. Human-bound waits appear only afterwards (bypass rulings, re-dispatch).

**Verdict on "it's on the mesh":** **Yes for the alpha fleet, no for the household mesh, and only for the *quality* of delivery, not the wave miss rate.** Evidence for the fleet: (1) the only stage that swings 14→72 min between consecutive builds is the alpha conductor roll, with james and adam at `no-movement`/`no-status` deadlines; (2) all ten App builds and the last ten genesis builds ended UNSTABLE on fleet state (adam circuit open, 50/50 hosted capacity, zome timeouts); (3) storage-A changed instance IP 6 times in 24 h and `divergent_actionable>0` held for 10–84% of the day per peer; (4) four shem storage containers are crash-looping at probe time. **Correction that matters:** the five non-delivered waves are **#2163–#2166 (`elohim-eprfs` FAILURE — a Rust gate red) and #2173 (genesis #1709, npm ETIMEDOUT in 3 min)** — none failed on fleet state. The household mesh is on no CI stage; its cost is a pre-push-lane cost. And the CPU half is as large as the mesh half: edge rebuild 56–80 min per commit plus 25–119 min of local gates.

**The two red cost measures.** `delivery-cost@1` = **3.24 pipeline-hours (hard 1)** is live (row 2026-10-28): ten App builds × 19–20 min, 0 delivered → the hours carry as a lower bound; driven by the adam adopt leg (503) on every run. `stage-wallclock@1` = **122.5 min (hard 20)** is **a 32-day-old observation (2026-09-27)**: the sensor stopped recording when the junit phase split made `publishVerify n=0`; the stage reads **10.9 min p90 today** (`--stages`). The headline has carried a dead value since.

---

## 5. How we understand limits natively today

**Measures** (`.claude/epr-meta/measures.yaml`): `stage-wallclock@1` (`:1036`, sensor dead since 2026-09-27), `delivery-cost@1` (`:1057`, live), `operator-surfaced-pain@1` (`:1010`), `resource-limit-raises@1` (`:984`), `dev-berth-held-by-measure@1` (berth `:116`), `gate-cycle-seconds@1` (`gate-cycle.mjs:15` → `.eprfs/status/flows.jsonl`; **fragmented**: the main checkout's rows end 2026-10-18 while `.worktrees/device-runtime-m/.eprfs/status/flows.jsonl` holds 147 rows after 10-23 — `gate-cost.mjs` p90s are therefore older than the newest runs).
**Bounds** (all `binding-local`, nothing refuses on them): `stage-wallclock-ceiling` hard 20 (`:2115`), `delivery-cost-ceiling` hard 1 (`:2135`), `resource-limit-raise-recurrence` hard 2 (`:1490`, class `ask`, the only one with teeth).
**Habits** (`habits-status.py`: 58 declared, 17 green, 35 red, 6 unwired, **37 not measured**; active = `dataplane-convergence`, `push-delivers-within-budget`): `push-delivers-within-budget` red (`genesis/orchestrator/.epr-meta/`), `push-gate-pays-once` green, `devices-share-one-berth` green but `observed_status: not-measured` (`genesis/agentic/.epr-meta/`), `measure-runs-on-a-peer` red, `conductor-capacity-represented` green (`elohim/elohim-storage/.epr-meta/`), `dev-system-equilibrium` red ("window unmeasurable"), `sprint-lands-each-closure-once` red/inactive, `runtime-death-witnessed` red.
**Berth classes / leases**: `mesh|verify|measure`, `cargo.lsp`, `device-ark`, steward-scoped `dev-integration|jenkins-dispatch|alpha-fleet|lane:` (`pool-policy.json berth`; `genesis/agentic/bin/berth`; `epr peer leases --json`).
**Guards**: ram-guard, io-guard, `cargo-disk-guard.py`, `ram-guard.py` deny arms, pre-push PVC watermarks (`.husky/pre-push.bash:886-944`), `conductor_admission.rs` (5 permits, 503 backpressure), `peer-roll-gate.sh` (900 s ceiling), `fleet-quiesce-gate.sh`, `edge-deploy-budget.sh` (240/115), `GATE_DEFER` (`gate-cost.mjs:68-84` — **dead in practice**: storage fails 4/20, eprfs 13/20, and dev targets are excluded), `GATE_EVIDENCE=reuse` for six projects, `GATE_NARROW` (eprfs, shadow).
**Ledgers**: `/projects/.claude-config/{ram-guard,io-guard,berth}/*.jsonl`, `.claude/data/quiesce-timeline.jsonl` (fleet-side), `.claude/data/ci-findings.jsonl`, `.eprfs/status/gate-live/runs/`, `genesis/data/rakia/compute-capacity.json` (three strata), `deployments.json` raise stamps (`$cpuLimitBump` ×11, `$memoryBumpComment` ×10, `$cpuThrottleBumpComment` ×3), `genesis/a2o/reports/recovery/recovery-timeline.jsonl`, `genesis/a2o/reports/peer-stage/` (last 2026-09-27).

**Numb senses — limits we hit that nothing observes:**
- The pod's **10.5-core CFS quota** is read by no guard and no gate; cargo default jobs = 24 against it. No per-gate CPU, memory or IO is recorded (`gate-tap.mjs` records seconds only) — so the 642→2,613 s storage-gate spread (CPU contention with the 128%-CPU resident conductor? page-cache eviction under 22k/day `memory.high` throttles?) is undiagnosable from the ledgers.
- Jenkins queue wait, concurrency, builder-pod CPU/OOM: **no `jenkins_*` metric exists**; the cloud's `containerCap` and the Lockable-Resources/Throttle plugins are unreadable from the repo or the anonymous MCP.
- `compute-capacity.json` commitments are 54 days stale and still read "shem decommissioned"; `stewardCommitments.jenkins` declares 4c/8Gi agents but no concurrency budget derives from it.
- Conductor heap growth per hour on the **fork**; per-space gossip convergence on 0.7 (48 `hc_*` series exist and none are folded); accumulated hosted agents per conductor (the 108-agent reading is memory-only); the household's per-peer envelope (absent by design, backlog).
- Berth lease contention over time (how long `dev-integration` is held, by whom) is in the ledger but never folded; stale moorings are never reaped (268 files); an 801-hour expired `push` lease.
- The `(N/0)` hosted-capacity refusals in genesis — no manifest sets a 0 ceiling.
- Host-side: SN7100 TBW, the 2026-10-28 DIMM UE burst, shem ZFS fill (adam `storage-data` PVC at 100%), hp-micro10's registry load under `imagePullPolicy: Always`, 3 GiB of cgroup kernel memory invisible to `ps`.
- The two `Che` workspaces on ethosengine/shem are absent from every capacity ledger (ethosengine limits 114% because of it).

---

## 6. Options to widen without adding nodes

Ranked by expected minutes recovered per push against the measured wave (p90 218 min, Execute Builds 210 min) and per local push (25–119 min), with the arithmetic and the evidence each rests on.

1. **Build-without-deploy by default for anything that is not `dev`, and a roll only when a conductor-class change ships** — CI-side. Evidence: edge deploy = 14 min storage-only vs 72 min with the conductor roll (#1749–#1753 vs #1747); `GATE_ONLY` landed on dev as `afc8a755d` (2026-10-28, merged `a88cb17c7`) but is still an opt-in PR tag; PR-215/PR-216 ran **N=2** concurrent gate-only edge builds on intel-nuc on 2026-10-28 (53.5 and 107 min) without a crash. Gain: −58 min on every wave that does not touch conductors (most), and every PR/feature wave stops rolling the fleet it shares with dev. Risk: a coordinator/DNA change that needs the roll must be classified (the DNA-hash vs coordinator hot-swap seam in CLAUDE.md). First step: flip `applyGateOnlyOverride` (orchestrator dev:965) from tag to rule for PR/non-dev; remove the `feat-.+`/`claude/.+` deploy arms at `elohim/holochain/Jenkinsfile:3092-3093, 3287-3288`, root `Jenkinsfile:2026, 2065, 2140`, orchestrator `:1894`; add `branch 'dev'` to `app/elohim-library/Jenkinsfile:331`; guard `dna/Jenkinsfile:1077` so `dev-latest` moves only on dev.

2. **Stop rebuilding edge on every commit: key the 56–80 min edge build on its closure, not the commit** — CI-side, with a native analog. Evidence: `edge-quiesce-gate-timeout-aborts.md:45-47` ("both rebuilt because the image tag is commit-derived"); locally the same idea (`GATE_EVIDENCE=reuse`) took elohim-storage 5,757→56 s and the storage gate 2,669→26 s. Gain: up to −56…−80 min on waves that touch neither `doorway/**` nor `elohim-storage/**` (eprfs-only waves already run 24–33 min). Risk: a stale image if the closure misses a read (the `gate-input-audit.mjs` discipline already exists). First step: reuse `scripts/ci/storage-input-digest.sh` to derive the image tag and short-circuit `Build Storage`/`Build Doorway` when Harbor already holds it.

3. **Fix the adam adopt leg and the shem storage crash loop** — fleet-side, and the only thing that moves `delivery-cost@1` (3.24 → the ~0.33 h a delivered 20-min run would cost). Evidence: every App run's failed leg is `adam state=unreachable HTTP 503`; four shem `elohim-node` containers are crash-looping now; `self-heal-adam-storage-breaker-compute-worker-readiness-coupling.md`. Gain: App delivered 0/10 → 10/10 with no pipeline change; UNSTABLE marks on genesis also depend on adam. First step: Loki on `pod=~"elohim-(adam|eve|gertrude|susan)-alpha-0", container="elohim-node"` for the last line before exit; decouple readiness from the compute-worker container.

4. **Shed the measure-class residents from the dev workspace** — native (berth ruling). Evidence: the T3 join-alpha conductor (4.5 GB, 128% CPU) + campaign-1.4 seed storage (7.3 GB) = **11.8 GB, 39% of the cgroup**, no mooring; they alone put the floor at the 70% soft line and 11 of 13 soft crossings on 10-28 were pre-push runs on top of them; the pod averages 1.95 of 10.5 cores but the conductor burns 1.3 of them continuously. Gain: ~12 GB and ~1.5 cores back; the storage gate stops competing with a 100-thread conductor (the likeliest cause of 642→2,613 s, **U**). Risk: whoever launched them for the letter-race ceremony (~2026-10-25 16:02Z) may still need them. First step: `berth status` shows no owner — moor or stop; then classify the truncated `elohim-storage-` comm in `ram-guard` so it is at least named.

5. **Sibling-verdict reuse across the two devices (A8) and `ark task` derivations** — native, zero new compute plane. Evidence: D4 proved identical closure keys on ethosengine and shem for all six `gate_reuse` projects at `c827533ab`; A8 is implemented (`gate-runner.mjs:463-486`) with "a live folded two-device verdict" still owed; `ark task run` has **never run here** (no `ark` in the `dev` family slot — only `family/tt/elohim/dev/debug/ark`; `.eprfs/status/ark/tasks` absent). Gain: a gate shem already paid is ~26–60 s on ethosengine (storage 1,510 s p90 → ~1 min), i.e. up to −25 min per push when the other device pushed the same closure. Risk: governance-class changes are never sibling-judged (D5); eprfs/orchestrator keys are whole-tree and never hit. First step: `just gate elohim-ark` so `family/dev/elohim/dev/debug/ark` exists; run one push with shem's verdict present and record the first `independence: sibling` row.

6. **Parallel Jenkins branches — safe only as build-only, and only within intel-nuc's envelope** — CI-side. Evidence: the orchestrator serializes per branch job (dev queued 79 min; PR jobs run beside it — three orchestrator runs started within 4 ms at 09:00Z on 10-28 from the cron); root App and genesis have **no** `disableConcurrentBuilds`; intel-nuc is 15.23 GiB with ~2 GiB resident, 7d CPU peak 99.99%; two full Level-0 waves request 16.5 GiB before shards and spill to 7.55 GiB ThinkPads whose MemAvailable dipped to 0.25–0.48 GB with 18 OOM kills; the alpha namespace, `elohim-happ:dev-latest`, RWO cache PVCs and the sweettest shards are true exclusives. Verdict: **N=2 build-only is measured-safe; N≥2 deploy-bearing is unsafe without a lock**. Minimal change: option 1 plus `nodeAffinity` *required* `operations` (or preferred, as the app does) for the ≥4Gi builders (`elohim/holochain/Jenkinsfile:2340-2351`, `dna/Jenkinsfile:249-256`, eprfs/epr), a `ResourceQuota` on ns `jenkins` (~12Gi requests) so excess goes Pending, CPU requests on the app builder (`Jenkinsfile:1194-1200`), and scope the cron at `genesis/orchestrator/Jenkinsfile:2448-2470` to dev. Gain: PR waves stop waiting 79 min behind dev. Risk: no `lock('alpha-fleet')` exists and Jenkins is not a berth participant.

7. **Shrink the fixture per lane** — native (`hc-mesh.sh` knobs). Evidence: lane cast 6 personas (3 provision) ≈ 4.7 GB vs `all` 22.8 GB; `MESH_RESET=1` sheds accumulated agents; `conductors-restart` every ~2 h reclaims 3–6 GB of stock-conductor growth (0.33 GB/h/conductor measured on stock; fork rate **U**); fractional arcs are refused, so the only arc lever is {0,1}; `MESH_DOORWAY_MAX_AGENTS=14` already bounds doorway A. Gain: keeps a working session at ~20 GB instead of 27 GB so the capped storage gate (5.5 GB) fits under `high` instead of being shed; each shed costs a 25-min re-run. First step: measure the fork conductor's growth/h once (`ps` RSS over a 2-h lane) before fixing a recycle cadence.

8. **Lane scoping and local parallelism that already exists** — native. `just test mesh <scope>` keeps a verify lease under 1800 s; `HARNESS_JOBS=4` took the harnesses 77→36 s (2026-10-25); `GATE_NARROW=on` for eprfs; fix the eprfs gate's 13/20 local failure rate so `GATE_DEFER` (p90 ≥600 s, 0 fails) can ever fire on feature branches. Gain: −10…−25 min per feature-branch push. First step: open the eprfs reds in the worktree sidecars (`.worktrees/*/.eprfs/status/flows.jsonl`) and unify the cycle ledger (`28b29fead` reads main, writes worktree).

9. **Move the household mesh or heavy gates to shem's workspace** — native, **not yet**. Evidence: shem already runs a 24–25 GB / 8.5-core Che workspace beside eve/gertrude at 80–99% CFS throttle, 7d MemAvailable min 6.45 GB, 213 OOM kills; `devfile-shem.yaml` (40Gi/8 cpu) is on dev but ISA-blocked for the UBI10 image; shem is not a provider in `providers.json` and the `local-conductor` cap is `available: false`. The cheaper version is option 5: shem *gates*, ethosengine *reuses*. Risk: shem is the more saturated host. First step: identify what the shem workspace is doing (active session vs leftover) before adding to it.

10. **Peer-executed stages via `just measure`** — native, **gain ≈ 0 today**. Evidence: executor source is on the rakia pin (`39cd2eb`, 2026-10-21) but no built `compute-executor` exists under the pool; `cluster-state.yaml` (all three files) still says unavailable; `--on adam` refuses; the G2 grant expired 2026-10-29T04:54:39Z with no auto-renewal; `jenkins-peer-stage.json` disabled; rung H guest work is 12–15 s per delegable lane and "saved about 0.2 min"; the storage cargo gate is out of scope for the stage shape (§12). First step (if pursued): build per `rakia-executor/OPERATIONS.md`, reconcile the three disagreeing binary paths (`measure.sh:111`, OPERATIONS `:14`, stage spec), flip the three cluster-state entries.

11. **Shed tier-3 and housekeeping** — native, small: sonarqube-mcp java ×3–4 ≈ 1 GB, portal `ng serve` 0.8 GB (`MESH_PORTAL=0`), `epr peer serve` 0.4 GB vs its ≤150 MB target, reap 268 moorings and the 801-h `push` lease, re-run `snapshot-capacity.sh` so `compute-capacity.json` stops saying shem is decommissioned, fix the `stage-wallclock@1` sensor.

**Which single option moves the measured 122.5-min stage-wallclock the most:** none of them, because that number is dead — fixing the sensor (option 11) makes it read today's 10.9 min p90 at once. For the costs that are actually red, option **3** moves `delivery-cost@1` from 3.24 to ~0.33 pipeline-hours, and option **1** (with 2 behind it) moves the wave p90 the most: 218 → ~160 min on a non-conductor wave, and 218 → ~80–100 min once the closure-keyed edge build also skips.

---

## 7. Unresolved frontier

1. **Why four shem storage containers are crash-looping now** — Loki `{namespace="elohim-alpha", pod=~"elohim-(adam|eve|gertrude|susan)-alpha-0", container="elohim-node"}` last line before exit; `kube_pod_container_status_last_terminated_exitcode`; read `self-heal-adam-storage-breaker-compute-worker-readiness-coupling.md`.
2. **Whether a Jenkins pod ever waits for cloud capacity** — the kubernetes-cloud `containerCap`/`maxRequestsPerHost` and plugin inventory (operator-side, Jenkins config); plus `kube_pod_status_reason{namespace="jenkins"}` / OOMKilled over 7d to turn "spill to a ThinkPad" into a measurement.
3. **What stretches the storage gate 642→2,613 s** — per-gate `cpu.stat`/`io.pressure` deltas and `ps` RSS sampling in `gate-tap.mjs`; with and without the resident T3 conductor.
4. **The hosted-human heap on the fleet** — the unrun one-registration smaps probe (`arch-scale-risk-backlog.md` row 14) and E1 `disable_app` release; `deriv(container_memory_working_set_bytes[6h])` per conductor to date matthew's 4,914 MiB week-high.
5. **Per-space arc convergence on the 0.7 fork** — fold the 48 `hc_*` series (workflow/gossip) over one roll; the "~30 agents" ceiling is a 2026-07-30 0.6-era number.
6. **Who holds what, how long** — fold `/projects/.claude-config/berth/ledger.jsonl` by resource×holder×duration; whether `dev-integration` contention across ethosengine↔shem is the serializer the operator feels; reap policy for 268 moorings.
7. **The DIMM** — map `csrow#3 channel#0` to a slot (host `dmidecode`/`rasdaemon`), confirm `memmap=2032M$16M` was chosen for it (2,029/2,044 pages inside), and move swap off md1 (QLC) — the half-done "$0 steps".
8. **shem's 81 MB/s per disk and the 100% `storage-data-elohim-adam-alpha-0` PVC** — ZFS pool stats from the host (node-exporter cannot read them); whether the 2026-10-08 `refquota` move was applied.
9. **Who launched the T3 conductor + seed-row storage (~2026-10-25 15:40–16:02Z) and whether they are still needed** — no mooring, lease or note names either pid; the ceremony receipt at `genesis/local-dev/letter-race-for-intimacy/publish/ceremony-receipt.json:43` is the only trace.
10. **The `(N/0)` hosted-capacity refusals** — `pool_map.rs` ~190–230 where `({used}/{max})` is formatted; why non-hosting conductors report a 0 denominator.
11. **Is the rebuilt `rakia-executor` on pin `39cd2eb` the recovered source or a rewrite, and can shem's Che container run `worker.mjs` as root with the five caps** — `rakia-executor/tests/`, the absent `genesis/local-dev/jenkins-native-bdd/` logs (presumably on shem), and a `setpriv` probe on shem.
12. **The +1 Gi / +500m delta between `devfile.yaml` (30Gi/10) and the running container (31Gi/10.5)** — the cluster-side `devworkspace-config`, operator-readable only.