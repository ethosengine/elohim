---
id: "backlog-performance-concern-index"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Performance concern index — every backlog entry tagged `performance`, by layer and mechanism, as the map for the profiling deep dive"
slug: "performance-concern-index"
written: "2026-10-02"
author: "claude-opus-5-5 (labelling pass, operator-directed)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "habit:zome-call-cost-bounded"
  - "habit:idle-is-free"
  - "habit:runtime-performance"
  - "habit:dataplane-convergence"
  - "habit:conductor-capacity-represented"
cites:
  - genesis/data/timeline/CONVENTIONS.md
  - genesis/docs/superpowers/sprints/2026-10-02-performance-deep-dive-shem-handoff.md
  - genesis/data/timeline/backlog/arch-scale-risk-backlog.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
tags: [performance, index, perf-telemetry, handoff, trust-gradient]
---

# Performance concern index

This is a map, not a register. The authority is the `performance` tag on each entry and the entry's
own evidence; this page is a dated snapshot of the query, written so the profiling deep dive can
start from one place. Vocabulary: `genesis/data/timeline/CONVENTIONS.md` §Performance. Bootstrap for
the deep dive: `genesis/docs/superpowers/sprints/2026-10-02-performance-deep-dive-shem-handoff.md`.

```bash
grep -lE '^tags:.*\bperformance\b' genesis/data/timeline/backlog/*.md          # the whole view
grep -lE '^tags:.*\bperf-latency\b' genesis/data/timeline/backlog/*.md         # one mechanism
grep -lE '^tags:.*\bperf-cpu\b' genesis/data/timeline/backlog/*.md | xargs grep -lE '^tags:.*\bconductor\b'
```

## What this pass did and did not do

Labelling only, 2026-10-02. Nothing was investigated, re-measured or re-ranked.

- **Swept:** 235 of the 675 backlog files — those whose frontmatter or body matched performance
  keywords. The other 440 were not read. An untagged entry is unclassified, not cleared.
- **Classified by a partial read** (frontmatter and roughly the first 60 lines) by three delegated
  readers, then spot-checked. Mechanism tags and the evidence column are a first reading.
- **Evidence column** is what the entry itself says: `measured`, `theory`, `cured-unverified`,
  `cured`, `design`. Many `measured` figures are weeks old and predate later cures. Reconciling
  each entry against the current code and conductor pin is the deep dive's first job.
- **Trust context and friction** were labelled in a second pass the same day, by one reader on the
  same partial read. `self` versus `declared` and `verify` versus `wait` are judgment calls worth
  re-checking on the entries you rank highest.
- **Figures are copied from the entries**, not re-derived. Open the entry before quoting one.


Indexed: 74 entries — 71 tagged at the source, 3 by reference (†). By mechanism: `perf-convergence` 25, `perf-queue` 24, `perf-latency` 17, `perf-telemetry` 15, `perf-cpu` 14, `perf-scale` 10, `perf-memory` 9, `perf-io` 8. By stated evidence: measured 49, theory 8, design 6, cured-unverified 6, cured 5.


## The trust gradient

Each entry also carries a trust context (who stands at each end of the costed path) and a friction
kind (CONVENTIONS.md §Trust context and friction). Trust should price compute, so the cells to read
first are a trustful relationship paying for re-verification or waiting. Mechanical friction is owed
whatever the relationship. Every path is trust-blind today: the dataplane handshake classes every
edge `public`.

| Trust context | verify | wait | mechanical | blind |
|---|---|---|---|---|
| **trustful-self** — one steward on both ends | 3 | 7 | 17 | · |
| **trustful-declared** — household, genesis pair, fleet peers | · | 7 | 9 | 3 |
| **trustful-earned** — earned standing, scoped delegation | · | · | · | · |
| **trustless** — strangers, first contact | · | 1 | · | · |
| mixed — not separated in the entry | 1 | · | 3 | · |
| no relationship on the path | · | 3 | 12 | 8 |

No performance entry sits in a trustful-earned context yet. Ten further entries that are *about* a
trustful context without reporting a cost carry the trust tag alone; `grep -lE '^tags:.*\btrustful-' genesis/data/timeline/backlog/*.md` lists them all.

### Trustful paths paying to re-verify

| Entry | The two ends | What shows it | Figures in the entry |
|---|---|---|---|
| [adopt-local-heal-second-guesses-arbitrated-winner](epr:adopt-local-heal-second-guesses-arbitrated-winner) | storage adopt_local ↔ its own conductor's arbitrated canonical answer | Re-derives forward-ordering proof the node's own conductor already settled; own-conductor local arm. | — |
| [conductor-cap-grant-scan-per-zome-call](epr:conductor-cap-grant-scan-per-zome-call) | storage zome calls ↔ its own conductor capability grants | Conductor re-reads 15k cap grants per zome call that storage itself minted. | ~47,000 SQL queries/call; CapGrant read 11,619-18,516 rows in 4.3-8.7 s; ~15,000 grants |
| [conductor-residual-cpu-full-chain-read-and-perpetual-republish](epr:conductor-residual-cpu-full-chain-read-and-perpetual-republish) | conductor ↔ its own source chain and CapGrant store | Full source-chain read per call and CapGrant read re-derive held facts; republish never completes receipts. | full-chain read rows_returned=33608, ~8 s; ranked by slow-statement seconds over 20 min |

### Trustful paths paying to wait

| Entry | Context | The two ends | What shows it |
|---|---|---|---|
| [2026-07-10-server-side-epr-read-path-catching-up-shed](2026-07-10-server-side-epr-read-path-catching-up-shed.md) | self | doorway /epr read path ↔ its own storage projector | Server-side read sheds 503 catching-up while the node's own projector drains. |
| [2026-08-10-adam-pull-loop-wedged-at-boot](2026-08-10-adam-pull-loop-wedged-at-boot.md) | self | adam projector puller ↔ adam's own conductor/doorway B reads | Pull loop never completes first pass; doorway B sheds catching-up on same-node reads. |
| [2026-08-10-fresh-head-nomination-and-declare-error-backoff](2026-08-10-fresh-head-nomination-and-declare-error-backoff.md) | self | storage head contest/declare ↔ its own conductor | Phantom-candidate loop and declare_error lacking backoff stall canonical convergence on one node's heal path. |
| [alpha-a-projector-chronic-catchup-flap](epr:alpha-a-projector-chronic-catchup-flap) | self | alpha-A doorway reads ↔ its own projector/storage | Projector flaps catching-up vs serving on one node, shedding same-node reads. |
| [heal-pointer-bytes-ordering-blocking-serve](epr:heal-pointer-bytes-ordering-blocking-serve) | self | doorway serve ↔ its own storage heal-on-read | Serving blocks unboundedly in heal-on-read waiting for bytes behind the pointer on one node. |
| [self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall](epr:self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall) | self | doorway SSR render ↔ its own cold fetches after bundle swap | One of 29 cold fetches exceeds soft budget for ~15 min after head swap. |
| [self-heal-doorway-startup-conductor-mint-serialization](epr:self-heal-doorway-startup-conductor-mint-serialization) | self | doorway startup ↔ its own per-conductor token mints | Startup serializes N synchronous token mints before listener binds, blowing startupProbe on same-node conductors. |
| [content-gap-limit-cycle-blocks-convergence](epr:content-gap-limit-cycle-blocks-convergence) | declared | fleet storage peers ↔ fleet peers via content gap discovery | Discovered-never-fetched ids oscillate across all pods; replicating peers fail to converge. |
| [content-projection-plateau-ethosengine-household](epr:content-projection-plateau-ethosengine-household) | declared | household peers james/jessica/matthew ↔ each other, content projection | divergent_anchor plateau across the household peers; declared household convergence. |
| [genesis-pair-cross-conductor-fetch-blocks-canonical-convergence](epr:genesis-pair-cross-conductor-fetch-blocks-canonical-convergence) | declared | adam conductor ↔ matthew conductor, genesis pair | Genesis pair cross-conductor fetch of REA commitments; cold-cell warm-up misattributed as retrieval failure. |
| [household-lamad-gossip-wedge-large-dht](epr:household-lamad-gossip-wedge-large-dht) | declared | household mesh peers ↔ each other, full-arc gossip | Full-arc gossip rounds do not complete within roundTimeout at ~120k ops among household peers. |
| [projection-reconcile-actionable-sawtooth](epr:projection-reconcile-actionable-sawtooth) | declared | fleet storage peers ↔ fleet peers, projection reconcile | ~50 actionable divergences reappear every 20-30 min across peers, making quiesce window a coin toss. |
| [self-heal-adam-projection-catchup-exhaustion-full-arc](epr:self-heal-adam-projection-catchup-exhaustion-full-arc) | declared | adam storage ↔ fleet authorities via conductor get_links | Cells not authorities until arc reconverges after restart; heal get_links leave box and time out. |
| [staggered-conductor-fleet-restarts](epr:staggered-conductor-fleet-restarts) | declared | alpha conductors ↔ each other, simultaneous roll | All 7 conductors restart at once; DHT routes dark for hours until convergence. |

These two tables are a reading order, not findings. Whether trust can compress any one of them is
for the deep dive to establish; the constraint is that ceremony is compressed, never skipped.

## Custody shape: who carries it

Sharing load is the other way cost falls (CONVENTIONS.md §Custody shape). Nine performance entries
are tagged `custody-everyone`: the cost is paid because every participant holds or validates all of
it. For each, the question is what the smallest holder set meeting the object's resilience need
would be.

- [fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11](epr:fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11)
- [household-lamad-gossip-wedge-large-dht](epr:household-lamad-gossip-wedge-large-dht)
- [self-heal-adam-projection-catchup-exhaustion-full-arc](epr:self-heal-adam-projection-catchup-exhaustion-full-arc)
- [staggered-conductor-fleet-restarts](epr:staggered-conductor-fleet-restarts)
- [inventory-gossip-amplifier-three-layer-idempotency](epr:inventory-gossip-amplifier-three-layer-idempotency)
- `2026-08-23-shard-level-inventory-gossip.md`, `2026-07-27-anti-entropy-egress-baseline.md`
- [dht-scale-envelope-and-web2-projection-at-planetary-scale](epr:dht-scale-envelope-and-web2-projection-at-planetary-scale)
- [arch-scale-risk-backlog](epr:arch-scale-risk-backlog) rows 7–8 (hosted-human heap; full-arc CPU)

Three things to hold beside that list:

- **The lever does not exist on the conductor line we run.** The arc is effectively zero or full
  with no runtime actuator ([dataplane-borrows](epr:arch-dataplane-borrows-backlog) row 18 priority
  2, tagged `custody-subset`). Storage custody can be shaped today; the DHT's can only be declared.
- **Breadth was ruled out as the driver in at least three entries**, by their own words:
  `conductor-memory-attribution-verdict.md` (arc-independent heap), `arc-shrink-ineffective-memory-soak.md`
  (arc factor 0 did not bound memory) and `genesis-pipeline-substrate-gated-adam-arc-saturation.md`
  (write amplification, not arc factor). Sharding would not have cured them.
- **Headcount is not resilience.** The operator's direction that a holder set be composed for
  independent failure domains at the least carrying cost is
  [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) row 30
  (`custody-diverse`); blob placement already ranks household diversity first.

These nine were assigned by me from the first pass's one-line summaries, not from a re-read.

## Planes: which part of the object is doing the work

Each entry names the plane whose work consumes the resource (CONVENTIONS.md §Plane). `fused-planes`
marks the entries where that price is charged on another plane's path. These are the ones to read
first: the cure is usually to let the cheap plane stop waiting on the expensive one.

By paying plane: no plane (build, tooling, gates) 25, notary 18, projection 13, custody 9, head 4, authority 3, bytes 1, reference 1.

| Paying plane → path it is charged on | Entries |
|---|---|
| projection → head (5) | `2026-08-10-adam-pull-loop-wedged-at-boot`; `conductor-slow-batch-starvation-jessica-class`; `content-gap-limit-cycle-blocks-convergence`; `projection-reconcile-actionable-sawtooth`; `storage-sqlite-locked-surfaces-as-500-despite-busy-timeout` |
| notary → head (5) | `adam-genesis-anchor-sustained-saturation-post-storm`; `conductor-admission-saturated-for-hours-after-restart`; `declare-route-sheds-harder-plus-no-chain-gate-leg2-findings`; `genesis-pair-cross-conductor-fetch-blocks-canonical-convergence`; `resolve-canonical-election-get-links-deadline` |
| custody → bytes (3) | `2026-08-23-shard-level-inventory-gossip`; `cluster-to-shem-p2p-request-starvation-11-peer-blackout`; `sovereign-peer-network-read-no-authorities` |
| notary → projection (3) | `2026-08-24-matthew-conductor-saturation-heal-leg-loop`; `self-heal-adam-projection-catchup-exhaustion-full-arc`; `staggered-conductor-fleet-restarts` |
| notary → bytes (3) | `alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes`; `alpha-conductor-sys-validation-spin-unfetchable-deps`; `doorway-conductor-reconnect-storm-matthew-edge` |
| projection → attention (3) | `elohim-render-incremental-hydration`; `feedback-discovery-sweep-is-o-n-in-history`; `self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall` |
| authority → head (2) | `conductor-cap-grant-scan-per-zome-call`; `head-authority-carried-with-content-sync-unit` |
| projection → bytes (1) | `2026-07-10-server-side-epr-read-path-catching-up-shed` |
| custody → projection (1) | `genesis-pipeline-substrate-gated-adam-arc-saturation` |
| head → bytes (1) | `heal-pointer-bytes-ordering-blocking-serve` |
| notary → custody (1) | `fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11` |

28 entries are marked fused. This pass read only each entry's title and opening lines, so it is the
shallowest of the label passes; treat the pairing as a prompt for the question, not an answer.

## Cost unit, phase and lane

Three more questions (CONVENTIONS.md §Cost unit, phase and lane): what the cost multiplies by, when it
is paid, and who is waiting.

By unit: item 16, unlabelled 15, call 15, once 12, peer 8, history 7, agent 1.

By phase: steady 33, transition 23, growth 9, unlabelled 9.

By lane: operator 23, interactive 21, background 16, borrowed 7, unlabelled 7.

| Unit | steady | transition | growth |
|---|---|---|---|
| history | 2 | · | 5 |
| agent | · | 1 | · |
| peer | 4 | 3 | 1 |
| item | 12 | 1 | 3 |
| call | 8 | 7 | · |
| once | · | 7 | · |

Two reading orders:

- **Cost that grows with history** (7): `arc-shrink-ineffective-memory-soak`; `arch-scale-risk-backlog`; `conductor-anon-leak-mechanism-smaps-verdict`; `conductor-memory-attribution-verdict`; `conductor-residual-cpu-full-chain-read-and-perpetual-republish`; `eprfs-status-perf`; `feedback-discovery-sweep-is-o-n-in-history`. A per-history cost on a path that runs per call is the first to name.
- **Background work standing in a foreground lane** (7): `2026-08-24-matthew-conductor-saturation-heal-leg-loop`; `alpha-conductor-sys-validation-spin-unfetchable-deps`; `conductor-publish-livelock-fk787`; `conductor-residual-cpu-full-chain-read-and-perpetual-republish`; `fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11`; `genesis-pipeline-substrate-gated-adam-arc-saturation`; `inventory-gossip-amplifier-three-layer-idempotency`.

A shallow pass: title and opening lines only. The unit in particular is the reader's inference from
incident text.

## Conductor (Holochain fork) (20)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [2026-08-24-matthew-conductor-saturation-heal-leg-loop](2026-08-24-matthew-conductor-saturation-heal-leg-loop.md) | queue, cpu | self | mechanical | measured | matthew's saturated conductor trips the circuit every sweep so projection-reconcile heal leg heals nothing for 2.5h. | healed:0 for 2.5h; to_resolve 829->1807->1766; 25s conductor timeouts; recv_validation_receipt elapsed_s=247.8 |
| [adam-genesis-anchor-sustained-saturation-post-storm](epr:adam-genesis-anchor-sustained-saturation-post-storm) | io, queue | declared | mechanical, wait | measured | adam genesis anchor sustains DB read-pool saturation and conductor unreachability after storm-pod deletion; arc-shrink lever does not apply to an anchor. | max_readers=8; read-pool util 650%-3300%; ~362-383 saturation msgs/sec |
| [alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes](epr:alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes) | io, queue | self | mechanical | measured | All seven alpha conductors crash-loop: holochain saturates its SQLite read pool, runs out of blocking threads and its admin listener dies. | 64 restarts in 08:35-10:35Z after 28h of none; 3-14 restarts/pod; CPU 0.01-0.06 cores |
| [alpha-conductor-sys-validation-spin-unfetchable-deps](epr:alpha-conductor-sys-validation-spin-unfetchable-deps) | cpu | declared | mechanical, wait | cured | Every alpha pod pegged at its CPU quota because sys-validation spins on unfetchable dependencies; the cure is later reported live on the 0.7 line. | adam 7.55 of 8 CPU; matthew 3.94 of 4, 99.9% throttled; 40x read-pool saturation; ~1000 log lines/s/pod |
| [arc-shrink-ineffective-memory-soak](epr:arc-shrink-ineffective-memory-soak) † | memory, telemetry | self | mechanical | cured | target_arc_factor=0 does not bound conductor memory (leecher soak); real driver hidden by the fused cgroup, later found as glibc arena retention and cured by jemalloc. | — |
| [arch-dataplane-borrows-backlog](epr:arch-dataplane-borrows-backlog) (rows 3, 13, 16, 17, 18) | queue, latency | mixed | mechanical | design | Flow-control and tail-latency borrows; row 17 conductor publish backpressure, row 18 fork resource bounds and recovery (six ranked priorities). | row 17: ~1.13 M DHT ops republished per 15 min on matthew; SQLite median 1.5 s (max 210 s) |
| [arch-scale-risk-backlog](epr:arch-scale-risk-backlog) (rows all (risk rows 1–10)) | scale | mixed | mechanical | design | Risk rows: landed-code shapes that grow badly with chain length, peer count or migration count, each with a measurable trigger. | — |
| [conductor-admission-saturated-for-hours-after-restart](epr:conductor-admission-saturated-for-hours-after-restart) | queue, convergence | self | mechanical, wait | measured | After a storage restart the conductor-admission gate stays full for hours, so no head can be authored and sheds spread across every storage caller. | admission capacity 5, in_flight 5 for ~4h (03:00-07:00Z) vs ~20 min runbook; shed after 5000ms |
| [conductor-anon-leak-mechanism-smaps-verdict](epr:conductor-anon-leak-mechanism-smaps-verdict) | memory | self | mechanical | cured | Conductor anon-memory leak mechanism via smaps; hypothesis ranking superseded, real cause glibc arena retention cured by jemalloc. | jemalloc flat ~2.1-2.9 GB past old ~5h OOM cadence; Go heap flat ~52 MB |
| [conductor-cap-grant-scan-per-zome-call](epr:conductor-cap-grant-scan-per-zome-call) | cpu, latency, scale | self | verify, mechanical | measured | Every zome call re-reads all capability grants (storage minted them), so each call costs tens of thousands of SQL queries and pegs conductor CPU and admission. | ~47,000 SQL queries/call; CapGrant read 11,619-18,516 rows in 4.3-8.7 s; ~15,000 grants |
| [conductor-memory-attribution-verdict](epr:conductor-memory-attribution-verdict) † | memory, telemetry | self | mechanical | cured | Conductor OOM climb is anonymous heap, not page cache or corpus, arc-independent; attributed to the conductor child and later cured by jemalloc. | anon share 94-98.3% (james 6.53 GB, matthew 5.39 GB, jessica 3.79 GB mid-climb) |
| [conductor-publish-livelock-fk787](epr:conductor-publish-livelock-fk787) | queue, cpu | self | mechanical | measured | Upstream Holochain 0.7.0 publish queue livelocks (SQLite FK 787): get_ops_to_publish LEFT JOIN plus batch-voiding record_published_op_hashes; confirmed locally. | — |
| [conductor-residual-cpu-full-chain-read-and-perpetual-republish](epr:conductor-residual-cpu-full-chain-read-and-perpetual-republish) | cpu, io, scale | self | verify, mechanical | measured | After the FK-787 cure alpha conductors stay near CPU limit: full source-chain read per call, bulk Entry fetch, CapGrant read and a republish that never completes receipts. | full-chain read rows_returned=33608, ~8 s; ranked by slow-statement seconds over 20 min |
| [dht-scale-envelope-and-web2-projection-at-planetary-scale](epr:dht-scale-envelope-and-web2-projection-at-planetary-scale) | scale | mixed | mechanical | design | Design question whether one DHT space and per-EPR replication scale to billions of users and hot content, and how trust projects to web2. | — |
| [fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11](epr:fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11) | cpu, queue | declared | mechanical, wait | measured | Every full-arc alpha conductor is CPU-pegged and throttled; kitsune2 publish has no sender back-off, gossip knobs are default, and the fleet rolls all peers at once. | adam 4.00 of 4 CPU 100% throttled ~24h; gertrude/susan/eve 1.50 of 1.5 for 7 days; matthew 55% throttled |
| [genesis-pair-cross-conductor-fetch-blocks-canonical-convergence](epr:genesis-pair-cross-conductor-fetch-blocks-canonical-convergence) | latency | declared | wait | measured | Sweettest red read as REA fetch regression; DELTA shows the 60s budget is spent on cold-cell first-call warm-up, the retrieval itself is fast. | first call 100.97 s; warmup export_schema_version 94.37 s; B retrieves commitment in 12 ms |
| [resolve-canonical-election-get-links-deadline](epr:resolve-canonical-election-get-links-deadline) | queue, io | self | mechanical | theory | Obey-path starvation from conductor DB-pool saturation on the local election read; dominant exit still unadjudicated; B2/GetStrategy recurrence disproven. | — |
| [runtime-sensing-gap-poller-unscheduled-no-throttle-alert-2026-09-11](epr:runtime-sensing-gap-poller-unscheduled-no-throttle-alert-2026-09-11) | telemetry, cpu | — | blind | measured | No alert watches conductor CFS throttling and the runtime harvester never polled, so a week of 100% throttled conductors reached no human. | CFS throttle ratio 1.0 for 7 days on gertrude/susan/eve; ~24h on adam; 66 polls, empty window |
| [self-heal-adam-projection-catchup-exhaustion-full-arc](epr:self-heal-adam-projection-catchup-exhaustion-full-arc) | convergence, queue | declared | wait, mechanical | measured | adam's post-restart projection catch-up never completes; conductor admission ceiling makes every heal get_links die on the 60s request timeout, doorway B answers 503 catching-up. | catch-up not done 80+ min vs ~20-min restart churn; 60s conductor request timeout |
| [staggered-conductor-fleet-restarts](epr:staggered-conductor-fleet-restarts) | convergence, queue | declared | wait | measured | Simultaneous restart of all 7 alpha conductors costs hours of DHT-route outage from fleet-wide arc catch-up and write-guard contention; stagger proposed. | ~2.5-3h outage; 7 conductors; PTxnGuard holds 0.6-1.9s |

## Kitsune2 / gossip (2)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [household-lamad-gossip-wedge-large-dht](epr:household-lamad-gossip-wedge-large-dht) | convergence, scale | declared | wait, mechanical | measured | Full-arc gossip on the household lamad DHT cannot complete rounds inside roundTimeoutMs at ~120k ops; fleet risk unconfirmed. | ~120k ops; rounds in 2.5h matthew None, jessica 1, james 2; 63-102 peer_timeouts per peer |
| [sovereign-peer-network-read-no-authorities](epr:sovereign-peer-network-read-no-authorities) | convergence | trustless | wait | measured | A freshly joined iroh peer cannot read fleet content for hours because live agent-infos advertise no storage arc and gossip fills slowly. | gossip ~90 KB/min (3.29 to 3.74 MB in 5 min); null in 7-10ms; not held after 15 min |

## elohim-storage (22)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [2026-07-27-anti-entropy-egress-baseline](2026-07-27-anti-entropy-egress-baseline.md) | telemetry, scale | declared | blind | theory | Three concurrent unbudgeted anti-entropy loops; two have zero counters so egress cannot be priced. | kitsune2 gossip 120-300s, inventory gossip 60s, Automerge 60s; Freenet single loop 53.7% of egress |
| [2026-08-10-adam-pull-loop-wedged-at-boot](2026-08-10-adam-pull-loop-wedged-at-boot.md) | convergence | self | wait, mechanical | measured | adam's projector pull loop never completes its first pass for hours, so doorway B sheds catching-up on every head-record read. | pull total=0 fetched=0 caughtUp=false for hours |
| [2026-08-10-fresh-head-nomination-and-declare-error-backoff](2026-08-10-fresh-head-nomination-and-declare-error-backoff.md) | queue, convergence | self | wait, mechanical | cured-unverified | Phantom-candidate loop stalls canonical convergence and declare_error lacked per-id backoff (retry amplification); backoff fix recorded as landed. | — |
| [2026-08-23-shard-level-inventory-gossip](2026-08-23-shard-level-inventory-gossip.md) | scale, io | declared | mechanical | design | Inventory gossip advertises N flat shard addresses per blob (64 for a 64 MiB blob); oversized snapshots hit the frame limit and stop advertising. Bitfield per composite proposed. | 64 addresses per 64 MiB chunked blob; 3.5 KB page budget; MessageTooLarge on household mesh |
| [adopt-local-heal-second-guesses-arbitrated-winner](epr:adopt-local-heal-second-guesses-arbitrated-winner) | convergence | self | verify, wait | theory | adopt_local re-derives forward-ordering proof the conductor already settled, so gossip-only peers converge slower than Declare-receiving peers. | — |
| [alpha-a-projector-chronic-catchup-flap](epr:alpha-a-projector-chronic-catchup-flap) | convergence | self | wait | measured | alpha-A projector oscillates catching-up and serving and never durably catches up, answering 503 to doorway reads. | p2p.divergentAnchor 1456 -> 2031 between runs ~90 min apart; retryAfter 30 |
| [conductor-slow-batch-starvation-jessica-class](epr:conductor-slow-batch-starvation-jessica-class) | queue, convergence | self | mechanical, wait | measured | Circuit breaker sheds the content heal leg before per-item classification on jessica, starving every adopt/contest arm so divergence never drains. | known_divergent{content} flat at 13 for 3+ h while matthew 13->2, james 14->1 |
| [content-gap-limit-cycle-blocks-convergence](epr:content-gap-limit-cycle-blocks-convergence) | convergence | declared | wait, mechanical | measured | Fleet-wide content-gap limit cycle: discovered-never-fetched ids oscillate with zero decay so the fleet never converges after a restart. | ~2.8k gaps/pod; ~30-min identical waveform 3h+ zero decay; divergent ~1245-3055 on adam |
| [content-projection-plateau-ethosengine-household](epr:content-projection-plateau-ethosengine-household) | convergence | declared | wait | measured | Content-projection divergent_anchor plateaus on the ethosengine household, independent of the inventory-snapshot storm and not touched by storage deploy. | — |
| [declare-route-sheds-harder-plus-no-chain-gate-leg2-findings](epr:declare-route-sheds-harder-plus-no-chain-gate-leg2-findings) | queue, convergence | self | mechanical, verify | measured | adam write-admission pool starves then oscillates, shedding 503 catching-up for hours so canonical-head declare route sheds harder than PATCH. | ~7h paced attempts; pool oscillates open every ~2-6 min |
| [feedback-discovery-sweep-is-o-n-in-history](epr:feedback-discovery-sweep-is-o-n-in-history) | scale, convergence | self | mechanical, wait | measured | Feedback projector visits 8 members per 60s sweep with no cursor, so correction convergence grows with whole mesh history rather than subscribed set. | convergence 107s to 333s across a day; 8 members per 60s sweep |
| [genesis-pipeline-substrate-gated-adam-arc-saturation](epr:genesis-pipeline-substrate-gated-adam-arc-saturation) | cpu, queue | declared | mechanical | cured-unverified | adam CPU storm from inventory-snapshot write amplification under WAN loss, not arc-factor; receive-side idempotency fix landed. | — |
| [head-authority-carried-with-content-sync-unit](epr:head-authority-carried-with-content-sync-unit) | convergence | mixed | verify, wait | measured | Design principle: carry signed head authority with its content sync unit; verification cost mentioned only as rationale. | adoption-trigger ladder 47.7 s / 58.8 s measured 2026-09-17 |
| [heal-pointer-bytes-ordering-blocking-serve](epr:heal-pointer-bytes-ordering-blocking-serve) | latency | self | wait, mechanical | measured | Heal converges the pointer before the bytes, so serving blocks unboundedly in heal-on-read instead of answering a degraded syncing status. | request held >30s with zero bytes; target <=5s; in-request heal ~2-5s |
| [inventory-gossip-amplifier-three-layer-idempotency](epr:inventory-gossip-amplifier-three-layer-idempotency) | queue, cpu, io | declared | mechanical, verify | measured | Inventory gossip plane carries ~500x design traffic; each re-apply of a 263-blob snapshot costs delete+reinsert plus re-score; receive-side idempotency landed, publish/gossip-id layers open. | ~53 applies/sec vs design ~0.1/sec (~500x); count=263 snapshots; sequence 1042-1303 |
| [inventory-refresh-pages-dropped-as-gaps](epr:inventory-refresh-pages-dropped-as-gaps) | convergence, queue | declared | mechanical, verify | measured | Bounded inventory refresh of 77 pages is received ~4 pages deep; out-of-order pages read as sequence gaps, are dropped and trigger snapshot requests. | 77 pages on 3513 count; receiver 184 hashes vs 2046 blob rows; cursor 2468 to 2541 |
| [no-latency-metric-for-change-doorbell](epr:no-latency-metric-for-change-doorbell) | telemetry, convergence | declared | blind | theory | No change-to-peer-notified latency metric exists on either plane, so announce-on-both cannot be measured; iroh-only peers wait on the 60s round driver. | 60s round driver |
| [projection-reconcile-actionable-sawtooth](epr:projection-reconcile-actionable-sawtooth) | convergence | declared | wait | measured | Projection reconcile never rests on alpha: actionable divergences reappear every 20-30 min on every peer, making the quiesce sustain window a coin toss. | ~50 actionable divergences per peer every 20-30 min (eve 0/51/51/14, susan 47, adam 49-53, jessica 51-65) |
| [storage-sqlite-locked-surfaces-as-500-despite-busy-timeout](epr:storage-sqlite-locked-surfaces-as-500-despite-busy-timeout) | io | self | mechanical | measured | Storage answers 500 'database is locked' on a head-moving PATCH during boot contention; the 30 s busy_timeout does not cover it. | 30 s busy_timeout; epr-app-deliverability passed 5/5 then failed 4/5 on same binaries |
| [susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors](epr:susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors) | queue | self | mechanical, blind | measured | susan's storage-conductor websocket is dead and HealCircuit pacing never trips on instant connection-closed errors, so heal attempts retry-storm at full speed. | heal_outcomes failed ~64,658 content + 1,849 rea per 12h on susan; others 60k-155k Ok(None) |
| [sync-edge-susan-timeouts-per-edge-observability](epr:sync-edge-susan-timeouts-per-edge-observability) | latency, telemetry | declared | blind, wait | measured | susan is the fleet's expensive sync edge: every pod times out to her and she to everyone, and nothing in the dataplane prices that edge. | 274 outbound sync failures/6h (susan 93, gertrude 53); susan working set 753 MB of 3 GiB |
| [transport-route-metrics-pretouch-zero](epr:transport-route-metrics-pretouch-zero) | telemetry | — | blind | cured-unverified | Transport route, path RTT and acquisition dispatch metric series are invisible on the fleet until first decision; pre-touch at zero landed locally, fleet read pending. | — |

## Doorway (4)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [2026-07-10-server-side-epr-read-path-catching-up-shed](2026-07-10-server-side-epr-read-path-catching-up-shed.md) | queue | self | wait, mechanical | measured | Server-side /epr read path sheds 503 catching-up under sustained load during E2E, a real read degradation twin of the cured write-path shed. | 503 catching-up retryAfter 30 |
| [doorway-breaker-trial-theft-fleet-verification](epr:doorway-breaker-trial-theft-fleet-verification) | queue | self | mechanical | cured-unverified | Storage /apps extraction-flight herd and breaker trial theft stacked into 503 half-open on all content routes; cures landed host-green, fleet verification open. | errorStreak 3 half-open circuit |
| [doorway-conductor-reconnect-storm-matthew-edge](epr:doorway-conductor-reconnect-storm-matthew-edge) | queue, cpu | self | mechanical | measured | Conductor closes app-ws sessions ~40/min, re-firing warm_stream replay and wedging the doorway runtime into watchdog restarts on matthew edge. | ~40 closes/min (476 in 12 min); 7 restarts in ~30 min; DB read pool util 500%, ~1.58M/13h |
| [self-heal-doorway-startup-conductor-mint-serialization](epr:self-heal-doorway-startup-conductor-mint-serialization) | latency | self | wait, mechanical | measured | Doorway startup serializes N synchronous per-conductor token mints with retry backoff before the listener binds, exceeding the startupProbe budget on a slow-DNS node. | up to N x ~12.5s serial; probe budget 24 x 5s = 120s; working set ~79MB of 1Gi |

## App and SSR (6)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [elohim-render-incremental-hydration](epr:elohim-render-incremental-hydration) | latency, cpu | — | mechanical | design | elohim-render opted out of Angular incremental hydration; whole component tree hydrates up front, costing low-power devices time-to-interactive. | — |
| [lamad-path-completion-enrichment](epr:lamad-path-completion-enrichment) | scale, latency | self | mechanical | theory | Path view completion computed as client-side O(N*M) intersection; nested-path summaries unwired (N+1 regressed to never-populated). | O(N*M) intersection |
| [self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall](epr:self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall) | latency | self | wait | measured | For ~15min after a bundle-head swap one of 29 cold-render fetches exceeds the 1200ms soft budget, so every visitor gets a degenerate render. | render degenerate 11/31 NEW renders = 0.35; 1200ms soft budget; ~15min window; 29 fetches per cold render |
| [self-heal-render-degenerate-cumulative-counter-false-positive](epr:self-heal-render-degenerate-cumulative-counter-false-positive) | telemetry | — | blind | cured-unverified | The SSR-saturation sensor reported an idle node as saturated (cumulative ratio, then delta with no denominator floor); a misleading metric, not a node cost. | render.degenerateRate 0.36 sustained >=3 polls |
| [ssr-server-bundle-zone-polyfill-split](epr:ssr-server-bundle-zone-polyfill-split) | cpu, latency | — | mechanical | theory | SSR server bundle ships and runs zone.js because the builder has one shared polyfills array; extra bundle size and runtime in the elohim-render isolate. | zone.js ~33 KB / 267 KB bundle figures in file |
| [ssr-staging-prod-pod-floor](epr:ssr-staging-prod-pod-floor) | memory | — | mechanical | theory | Staging and prod SSR manifests keep a 256Mi memory floor and no startupProbe, so V8 cold-start will OOM on the first SSR roll. | 256Mi floor; V8 cold-start 2s to 15s to 60s |

## Harness and gates (a2o, mesh, quiesce) (5)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [ci-edge-inventory-convergence-caughtup-sawtooth-flap](epr:ci-edge-inventory-convergence-caughtup-sawtooth-flap) | telemetry, convergence | — | blind, wait | measured | a2o scenario asserts p2p.caughtUp as an instant on a field that sawtooths, so the same fleet passes the quiesce gate then fails the scenario. | — |
| [fleet-quiesce-pass-not-convergence](epr:fleet-quiesce-pass-not-convergence) | telemetry, convergence | — | blind, wait | measured | Fleet-quiesce gate PASS tracks only probe A actionable convergence while B caughtUp stays False, so validation 503s minutes later. | PASS at 1999s; 35 poll lines at 60s cadence; B-caughtUp=False on every poll |
| [quiesce-gate-measurement-availability](epr:quiesce-gate-measurement-availability) | telemetry, convergence | — | blind, wait | measured | Fleet-quiesce gate never saw a quiet window against the non-converging A-side reconcile plateau, so three validate-only runs did not measure. | deadline 2700s; sustain 330s; 3 DID-NOT-MEASURE runs #1367-#1369 |
| [quiesce-preflight-absent-unmeasured-reads-as-pass](epr:quiesce-preflight-absent-unmeasured-reads-as-pass) | telemetry, convergence | — | blind | measured | The doc-level quiesce preflight leg cannot fail because an absent unmeasured Prometheus series reads as pass; a convergence probe that cannot measure. | — |
| [self-heal-alpha-projector-one-sweep-read-as-three](epr:self-heal-alpha-projector-one-sweep-read-as-three) | telemetry, convergence | — | blind | cured-unverified | The projector exhaustion predicate counted one published post-restart sweep as three, filing exhaustion on a projector that converged next sweep. | — |

## CI and builds (8)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [cargo-test-memory-shed-storage-gate](epr:cargo-test-memory-shed-storage-gate) | memory | — | mechanical | cured | elohim-storage gate build peaked at 15.4 GB and was shed by the RAM guard six times; manifest-declared CARGO_BUILD_JOBS=1 capped it. | 15.4 GB peak to 5.5 GB; 6 sheds; ~12% wall-clock cost; cargo test 7.6-14.5 GB |
| [ci-orchestrator-stale-baseline-supersedes-inflight-downstream](epr:ci-orchestrator-stale-baseline-supersedes-inflight-downstream) | queue, latency | — | mechanical | measured | Stale-baseline timer dispatch re-runs an app pipeline already building the same commit, discarding a delivery 79 minutes in after the shared 240-minute timeout. | 240-minute timeout; DNA 44 min; edge ~3h; app aborted at 79 min |
| [ci-projected-head-convergence-race](epr:ci-projected-head-convergence-race) | convergence, telemetry | — | wait, blind | measured | Deploy projected-head probe reads the served hash at one instant before the doorway's reconcile tick swaps the head, a convergence-window race. | 300s reconcile tick; four fingerprints |
| [ci-sweettest-shard-packing-contention](epr:ci-sweettest-shard-packing-contention) | cpu, queue | — | mechanical | measured | Sweettest 4-way shards pack 2-3 per node, saturating CPU and blowing 30s gossip-visibility deadlines. | 95%+ node CPU; 30s deadline; failed #1376/#1379/#1380, passed #1377/#1378 |
| [edge-quiesce-gate-timeout-aborts](epr:edge-quiesce-gate-timeout-aborts) | latency, convergence | — | wait | measured | The fleet-quiesce gate rides the edge build to global-timeout ABORT during catch-up windows, so healthy deploys read as ABORTED. | edge #1406/#1407/#1408 all ABORTED after deploy stages |
| [edge-rollout-walltimeout](epr:edge-rollout-walltimeout) | latency | — | wait, mechanical | measured | Edge cold-start statefulset rollouts flake on a 1-hour wall timeout; bump timeout, pre-pull or parallelize the three rollouts. | ~43% flake rate; 1-hour wall budget; 3 rollouts |
| [elohim-app-ng-build-oom-cascade-blocks-dispatch](epr:elohim-app-ng-build-oom-cascade-blocks-dispatch) | memory | — | mechanical | measured | ng build OOMKills (no pod memory limit) on elohim/dev and cascade-aborts the orchestrator graph so edge/genesis never dispatch. | builds #1654 and one more; container terminated OOMKilled exit 137 |
| [genesis-seed-conductor-hang-90min-timeout](epr:genesis-seed-conductor-hang-90min-timeout) | latency, queue | self | mechanical, wait | measured | Genesis Seed Database hangs in a conductor-path retry loop on an OOM-flapping conductor until the 90-min pipeline timeout; should be a fast skip. | aborted at 90m22s; adam retry ~4.8m+; 3,429 content rows stamped provenance-only |

## Devspace tooling (3)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [2026-08-29-compute-envelope-virtual-peer-contract](2026-08-29-compute-envelope-virtual-peer-contract.md) | memory | declared | mechanical | design | After a cgroup memory.max group-kill of the workspace, virtual mesh peers should run inside a declared resource envelope; RAM guard fields become delegates-compute fields. | one rustc 4.3 GB plus seven rust-lld linkers on three conductors hit memory.max |
| [eprfs-status-perf](epr:eprfs-status-perf) | latency, scale | — | mechanical | measured | epr flow status re-parses the whole JSONL sidecar on every call, taking ~25s in a debug cargo-run build. | ~25s wall-clock (real 0m25.076s); 4986 resources, 3485 intents, 2MB+ sidecar |
| [join-alpha-skips-local-dna-build](epr:join-alpha-skips-local-dna-build) | latency | — | mechanical | theory | join-alpha still builds all 5 WASM DNAs it will never install, costing minutes of cargo on a fresh clone; skip when join-alpha without FORCE_LOCAL_HAPP. | minutes of cargo; 5 WASM DNAs |

## Cluster and nodes (4)

| Entry | Mechanism | Trust | Friction | Evidence | What costs what | Figures in the entry |
|---|---|---|---|---|---|---|
| [break-503-operator-levers](epr:break-503-operator-levers) † | memory | — | mechanical | measured | Operator levers (matthew RAM 8Gi to 16Gi, anti-affinity) against conductor memory sawtooth and OOM, since arc-shrink did not bound memory. | jessica OOMs to 4Gi every ~40 min; matthew limit 8Gi to 16Gi; doorway 100 restarts |
| [cluster-pressure-rebalance](epr:cluster-pressure-rebalance) | cpu, memory | — | mechanical | measured | intel-nuc node oversubscribed on CPU and jessica edgenode OOM-flaps; nodeAffinity rebalance proposed so pressure stops causing evictions. | intel-nuc ~135% CPU |
| [cluster-to-shem-p2p-request-starvation-11-peer-blackout](epr:cluster-to-shem-p2p-request-starvation-11-peer-blackout) | queue, latency | declared | mechanical, wait | measured | matthew's sync and shard requests to 11 of 13 mesh peers time out while connected; starvation toward shem peers. | 2 of 13 peers answer; 11 unanswering over window 2026-07-01T23:49 to 07-02T11:49 |
| [ops-adam-pod-log-volume-saturates-loki](epr:ops-adam-pod-log-volume-saturates-loki) | io, telemetry | — | mechanical, blind | measured | adam alpha pod emits ~20x its sibling log volume, saturating Loki into 502s on every query. | 25.9 GB / 94,141,822 entries in 24h vs matthew 1.5 GB / 4,643,525 |

† No frontmatter, so the entry carries no tag; it is indexed here by reference only.

## Habits that carry a performance invariant

The acceptance boundary for any performance claim is a habit's own check, never this page.

| Habit | Home | Status | What it bounds |
|---|---|---|---|
| `zome-call-cost-bounded` | `elohim/holochain/.epr-meta/` | red | A zome call's conductor cost does not grow with the agent's history |
| `idle-is-free` | `elohim/elohim-storage/.epr-meta/` | red | A settled household asks its conductors for almost nothing |
| `runtime-performance` | `.epr-meta/` | red | A performance report never turns missing or incomparable evidence into a green |
| `conductor-capacity-represented` | `elohim/elohim-storage/.epr-meta/` | green | The conductor's DB read pool is admitted against, not discovered by timeout |
| `sync-scale-honesty` | `elohim/elohim-storage/.epr-meta/` | green | The sync plane's cost is sub-quadratic and measured |
| `dataplane-convergence` | `elohim/elohim-storage/.epr-meta/` | red | Peers converge on one head without per-host imperative writes |
| `push-delivers-within-budget` | `genesis/orchestrator/.epr-meta/` | red, active | A push reaches every planned pipeline inside a wall-clock budget |
| `measure-runs-on-a-peer` | `genesis/agentic/.epr-meta/` | red | Measure-class runs execute on a peer, never on the dev berth |

Statuses read from each atom on 2026-10-02; `just status habits --full` is the live reading.

## Keeping it current

Tag at the source when an entry is written or touched. Regenerate this page from the tag query when
a deep-dive pass closes; do not add rows here that have no entry behind them.
