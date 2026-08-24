---
title: "Transport self-awareness and the two-peer diversity harness — peers that know their own paths, proven locally before the fleet"
id: transport-self-awareness-diversity-harness-design
tier: spec
status: Draft
created: 2026-08-24
maintainers: Matthew Dowell + Claude Fable 5
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete (every row in §7 landed or superseded) OR superseded-by-implementation
domain: peer-hoster dataplane (T2 libp2p × iroh) × local mesh harness × quiescence measurement
habits: [dataplane-convergence]
topic: [transport, iroh, libp2p, dual, path-quality, race-fetch, selection, exploration, diversity-harness, two-peer, quiescence, churn, honest-absence, syncDocuments]
cites:
  - "substrate-trust-contract-runbook | the live-substrate invariants and per-red decision tree this arc's churn-injected quiescence measure must stay coherent with; its probes remain the authority when the local and fleet series disagree | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - "elohim-seam-map-concern-routing | routes this concern to the T2 substrate track (libp2p x iroh co-resident); seams are where capability is added, tracks are how a node participates — path selection is a track property, not a new seam | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - genesis/data/timeline/backlog/2026-08-24-matthew-conductor-saturation-heal-leg-loop.md
  - genesis/a2o/features/dataplane/transport-dual-plane.feature
  - genesis/a2o/features/dataplane/transport-comparison-matrix.feature
  - app/elohim-app/scripts/hc-mesh.sh
  - app/elohim-app/scripts/hc-mesh-transport-matrix.sh
  - scripts/ci/fleet-quiesce-gate.sh
  - scripts/ci/run-mesh-quiesce-stage.sh
  - genesis/scripts/quiesce-timeline.py
  - elohim/elohim-storage/src/p2p_iroh/sync_driver.rs
  - elohim/elohim-storage/src/p2p_iroh/peer_book.rs
  - elohim/elohim-storage/src/p2p/blob_fetch.rs
  - elohim/elohim-storage/src/sync/doc_store.rs
  - "iroh-libp2p-complementarity | the canonical selector this spec composes INSIDE: peer_map::select_transport rules 1/3 become the eligible set, Track3/NoShared stay verbatim, rule-2 iroh preference survives as the Unknown prior; its anti-capture clause is answered by the exploration floor | sha256:29235aeb35aff128 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md"
  - genesis/docs/superpowers/plans/2026-06-14-dataplane-transport-plan.md
  - elohim/elohim-storage/src/p2p_iroh/peer_map.rs
---

# Transport self-awareness and the two-peer diversity harness

> **Status:** Draft — design pass 2026-08-24, awaiting operator review before the plan.
> **Shape:** one arc, four cuts, in dependency order: (0) an honesty fix, (1) an ephemeral path-quality observation + one selection predicate, (2) a heterogeneous two-peer local harness, (3) a churn-injected quiescence measure shared by the local mesh and the fleet.
> **Theme:** a peer should know, from its own perspective, which path to another peer is fastest right now — and we prove it with the one scenario the resiliency saga already demands: one peer holds the other's full recoverable state; take a peer down, bring it back (or bring a new one in), and time how long until it holds everything again. Two peers, two doorways, every transport pair — nothing more is needed.

## 1. What today's measurement actually showed

The arc was scoped after measuring the alpha fleet and the local mesh on 2026-08-24. Three findings reframe the work and are the ground the design stands on.

1. **`/p2p/status.syncDocuments` is a reporting bug, not an empty store.** It calls `count_documents("_all")`, which `DocStore::count` turns into a key-prefix scan for `"_all:"` — but every projected doc lives under the `"elohim:"` namespace, so it reads `0` forever. `count_all()` sits directly beneath it. Ground truth from `GET /sync/v1/elohim/docs`: **504 docs on each local peer, 5,356 on the fleet.** The corpus back-fill ran everywhere (local: scanned ~485, skipped ~484 = docs already present and matching).
2. **iroh sync is working and is idle by correctness.** In `dual` mode both planes share one DocStore. libp2p converged it (`elohim_sync_in_sync_total` fires on essentially every round), so the iroh driver's `local_heads == remote.heads` short-circuit fires on every document and `sync_changes_applied_total` sits at ~1 per peer. Control plane is fully live (fleet: `iroh_peers_known`=6 everywhere, ~78k gossip-neighbor events, ~3.9k manifest announcements, 62–313 sync rounds per pod). The 2026-08-23 memory note that called the data plane "idle" was reading a converged corpus as a dead one.
3. **A converged corpus cannot show transport leverage.** Steady-state counters on a converged fleet measure nothing about which transport is faster. Leverage exists only *during churn* — a peer restart, a deploy, new content — which is exactly what the quiesce gate times. Every quiesce record from edge #1366–#1379 wins at a ~360s window because that is the sustain threshold quantised to the 60s poll; what varies is time-to-verdict, and on #1379 it was driven entirely by matthew's `divergent_actionable` spike cycle (0–2 baseline, 12–14 for ~10–15 min every ~30–50 min), not by any transport property.

Consequence: the measurement instrument for this arc is **time-to-converge under injected churn, per transport pair**, never a steady-state counter. And the honesty fix in (1) ships first, because a register that reads 0 while holding 5,356 docs will mislead every later reading.

## 2. Goals and non-goals

**Goals**

- A peer holds a *local, per-pair, per-transport* view of path quality and exposes it (`/p2p/status`, Prometheus) — the self-awareness the operator asked for, readable by a human and, later, by the peer itself.
- Latency-critical small operations race both live planes; bulk operations select one plane using the observations the races already produced; a fixed exploration fraction keeps the non-preferred plane sampled so recovery is detectable.
- The local mesh runs **exactly two peers and two doorways**. Two peers is sufficient, not merely minimal: with replication factor 2 each peer holds the other's *entire* recoverable state, so **peer loss + recovery from the survivor** exercises every feature the architecture claims (household forms, heads converge, custody witnessed, pull queue retires) *and* measures its performance in one run. The six transport pairs are the axis that recovery is timed along, on the same two process slots.
- The harness reports its own footprint (RSS/CPU per process) so "can we afford this locally" is a number, not a worry.
- Quiescence under churn is measured with **one reader** (`quiesce-timeline.py`) from **two emitters** (the local mesh stage and the fleet gate), so the before/after claim has the same shape locally and on the fleet.
- Sync-document verdicts are projected as *structured per-document reasons* in the peer's own status, so "no peer knows what to do with document X" becomes a readable genesis-pipeline signal — the lens the operator named.

**Non-goals (declared, with pointers)**

- **No path metric is notarized.** Self-asserted latency is unverifiable and per-pair × per-transport × continuous sampling is an unbounded head-plane resident. Ephemeral (C) throughout; see §5.
- **No new table, migration, or HTTP mutation route.**
- **No AI-inferencing repair of contracts.** The structured-reason surface is built so a future inferencing peer has something to read; the inferencing is a separate arc.
- **No simulacra orchestrator.** The eventual shape — Che (or any peer) minting peer slots with declared configurations across the elohim primitives, k8s-like power over p2p — is the destination this harness is *compatible with*: its unit is "a slot with a declared configuration." Building the orchestrator is out of scope; this arc proves the unit.
- **No single-conductor mesh.** The three local conductors are ~2 GB RSS each (~6 GB of a 6.9 GB mesh); collapsing them would cut a two-peer mesh from ~4 GB to ~2 GB, but the mesh binds one conductor per peer by design and `seed-household-formation` binds each human to *its own* conductor — changing that is a seeding-contract change, not an incremental one. Listed as a future lever.
- **No change to the fleet quiesce predicate.** Its four legs and sustain semantics stay as they are; this arc feeds it a second emitter, nothing more.

## 3. Design

### 3.0 Cut 0 — the honesty fix (ships first, alone)

`/p2p/status.syncDocuments` calls `sync_manager.count_all()` — the honest "what does this store hold" (`count_documents(PROJECTION_NAMESPACE)` would be equivalent today and silently wrong the day a second namespace lands). One-line change plus a regression test that seeds two namespaces and asserts the status projection counts both. This lands as its own commit before anything else so that every later reading of the field is trustworthy, and so the fleet register stops contradicting the HTTP route.

### 3.1 Cut 1 — `PathObservation` and `select_path`

**The core move: race the cheap operations, select the expensive ones — and the races *are* the probe.** No synthetic pings. Every small, latency-critical operation (head-record fetch, `list_documents`, blob ≤ a size threshold) fires on **both** live planes when both are known for the peer and takes the first verified answer. The duplicate bandwidth is negligible; the win is min-latency for free *and* a fresh RTT sample for both transports on every such operation. Bulk transfers (large blob, full sync payload) **select** a single plane using those samples, because duplicating a large transfer is genuinely expensive. Active probing would add load and measure a synthetic path; passive-only observation would starve the non-preferred plane of samples and freeze the first choice forever. Racing the small class gives continuous, representative samples on both planes at ~zero marginal cost; the exploration fraction on the bulk class keeps samples flowing where racing is off.

The blob layer already has this shape: `blob_fetch::race_fetch` is a `FuturesUnordered` first-responder-wins race across *libp2p candidates* (T19 regression `race_fetch_first_responder_wins`), and heal-on-read already races the iroh plane (T2 counter, edge #1379). Cut 1 extends that precedent across transports and *records* the outcome; it does not invent a second racing mechanism.

**`PathObservation`** — in-memory, per `(agent_cid, transport, op_class)`:

| field | meaning |
|---|---|
| `rtt_ewma_ms` | exponentially-weighted RTT of verified successes |
| `success_rate` | successes / attempts over the ring |
| `last_sample_age` | wall-clock since the last sample — the honest-absence carrier |
| `saturation_hint` | set when the remote signalled backpressure (queue-full, 503-class, receipt latency) |
| `state` | `Unknown` (no sample yet) · `Sampled` · `Degraded` (success rate below floor or hint set) |

Keyed by `agent_cid` resolved through `peer_transport_manifest` / `IrohPeerBook` — never by raw-comparing a libp2p PeerId against an iroh NodeId (the cross-namespace string-equality trap). Reconstruction strategy: cold start = `Unknown` everywhere, refilled by the first operations. Nothing persists.

**Composition with canon — this predicate lives *inside* the canonical selector, never beside it.** The complementarity canon defines `peer_map::select_transport(self, peer, plane) -> Iroh | Libp2p | Track3Bridge | NoSharedTransport` (implemented at `peer_map.rs:462`, called today from `http_blob_router` for the Blob plane): rules 1/3 derive which transports *both* peers support for the plane, rule 2 prefers iroh, rules 4/5 handle the hub bridge and the no-path case. Cut 1 keeps every rule and changes one thing: rules 1/3 now yield the **eligible set** (0, 1, or 2 planes) instead of collapsing it to one choice, and `select_path` decides *within* that set. Track3Bridge and NoSharedTransport are returned verbatim before `select_path` is ever consulted; with one eligible plane the behaviour is identical to today; canon's rule 2 ("prefer iroh") survives as the **prior** when both eligible planes are `Unknown`. The canon's anti-capture clause — transport monoculture is itself a capture vector — is answered directly: the exploration floor means a dynamic preference can never harden into monoculture, which a static "prefer iroh" could.

**`select_path(peer, eligible, op_class, observations) -> Route`** — a pure decision predicate registered in `elohim/elohim-storage/seam-registry.yaml`:

- `op_class = Small` and both planes known → `Route::Race(both)`.
- `op_class = Bulk` → `Route::Single(best)` where best = lowest `rtt_ewma_ms` among `Sampled` planes not `Degraded`; with probability `explore` (default 0.10) pick the other live plane instead.
- Any plane in `Unknown` → it is *included in the race* for `Small` and *is the exploration pick* for `Bulk` until it earns a sample. Unknown never sorts last.
- Only one plane known → `Route::Single(that)`; no plane known → `Route::None` with a reason.

Concern-canon answers for the predicate (registered at birth, contract tests named in the plan):

- **C4 honest absence — the trap of this design.** "No observation" is its own state and never coerces to "slow"; otherwise a freshly-joined peer is permanently deprioritised and never earns a sample. `Unknown` routes to *race*, not to last place. Contract test: a peer with one `Sampled` plane and one `Unknown` plane must race on `Small` and must explore the `Unknown` plane on `Bulk` within N picks.
- **C3 liveness.** The exploration fraction guarantees a non-preferred live plane keeps being sampled, so its recovery is detectable. Contract test: a plane marked `Degraded` that starts succeeding returns to `Sampled` without operator action.
- **C11 externally-imposed backpressure.** `saturation_hint` *lowers* selection weight; it never triggers retries. A saturated peer (matthew, 2026-08-24 morning) receives less traffic, not more attempts.
- **C6a bounded work.** Racing is capped at two in-flight futures per operation; the observation ring is fixed-size per key; keys are bounded by the peer book.
- **C7 advertise/serve symmetry — partial, named gap.** The fleet shows ~50% `manifest_announcements{result="stale"}`; advertisement and serving are already drifting. Cut 1 records `Degraded` when an advertised plane fails to serve, which surfaces the drift but does not cure it. Gap note in the registry row.
- **C8 observability-per-decision.** Every `Route` decision increments `elohim_transport_route_total{transport,op_class,reason}`; every sample updates `elohim_transport_path_rtt_ms{peer,transport,class}`.
- C0 plane location: T2 substrate, both swarms co-resident (`TransportBackend::Dual`). C1/C2/C5/C9/C10/C12/C13/C14: `n-a` — no authority, no identity lineage, no consent surface, no residual is minted; a route choice is a local, reversible, un-witnessed act.

**Surface.** `/p2p/status.transportPaths[]` — one row per `(peer, transport, class)` with the fields above — plus the two metric families. This is the peer's-eye view of its own network.

### 3.2 The sync-document verdict surface (the steering note, as a constraint)

The reconcile and back-fill paths already *know* why a document is not projected or not convergent — reach excluded, content type not re-authorable (`reanchor_backfill: skipping row with non-canonical content_type … "album"` on the fleet today, `dead_candidates=199` looping every ~2.5 min), no producer for the row, heads diverged with no peer able to serve. Today those are log lines. Cut 1 projects them as **structured per-document reasons** — `/p2p/status.syncVerdicts` (bounded: top-N by age, counts by reason) and `elohim_sync_verdict_total{reason}` — so that "every peer reports document X and none knows what to do with it" reads as a genesis-pipeline signal from *inside* a peer, without tailing logs. The `album` content-type case is filed as a finding against the seed pipeline, not absorbed into this arc.

### 3.3 Cut 2 — the two-peer diversity harness

**Topology.** `MESH_PEERS=matthew,jessica`. Two peers is *sufficient*: at replication factor 2 each peer is the other's complete recoverable state, so the survivor-recovers-the-loser scenario proves both the features and the performance of the architecture — which is exactly what the resiliency saga demands (chapters 1–2: a peer awakens and the household forms; 5–7: co-stewardship, heads converge, custody witnessed; 11: the pull queue can finish). A third peer would add ~2.2 GB (one conductor) and no new claim. Doorway A stays primary→peer 0 with peer 1 as extra; doorway B stays primary→peer 1 (the existing apex stand-in shape) — so doorway-projection diversity is exercised without a new doorway, and doorway failover (the top red) is the same two-doorway pair.

**Per-peer transport.** `MESH_PEER_TRANSPORTS=matthew=libp2p,jessica=iroh` (falls back to `MESH_TRANSPORT_BACKEND` for any peer not named). `storage-restart <peer>` applies that peer's declared transport through the existing env-capture + overlay path; `assert_storage_transport_capability` checks the *per-peer* mode, not the global one; `mesh_transport_backend_from_status` reports the actual per-peer set (e.g. `matthew=dual,jessica=iroh`) instead of `unknown` on any mix.

**The churn primitive is recovery, not a config swap.** Every measured run is one of two recovery shapes, timed from the moment the recovering peer is up to the moment it holds everything the survivor holds:

| shape | what happens | what it proves |
|---|---|---|
| **warm return** | stop peer B's storage; wipe its DocStore + blob store (conductor, keys, chain untouched); restart B in the scenario's transport | the survivor A holds B's full state and B re-acquires it — saga 6/7/11 on a known identity |
| **cold join** | stop peer B entirely; regenerate B as a *new* identity (new sandbox, new agent key); start it in the scenario's transport | a new peer joining recovers the household's whole state from one survivor — saga 1/2 then 6/7/11 |

**Recovered** is defined by the saga's own predicates, not a new one: blob inventory parity with the survivor (chapter 6, `heads converge`), custody manifests witnessed for every blob (chapter 7), `pull.caughtUp === true` with `pull.failed == 0` (chapter 11), and the same content ids served 200 through *both* doorways (chapter 4). `time_to_recover` is the wall-clock from B's `/health` 200 to the last of those four turning true.

**The six transport pairs** are the axis each recovery shape is run along (`hc-mesh-transport-matrix.sh` grows from three homogeneous modes to these):

| scenario | matthew (survivor) | jessica (recovering) | expected |
|---|---|---|---|
| `homo-libp2p` | libp2p | libp2p | recovers |
| `homo-iroh` | iroh | iroh | recovers (pure-iroh bootstrap via the doorway manifest board — dormant on the fleet, enabled locally) |
| `homo-dual` | dual | dual | recovers; races active — the pair the self-awareness claim is measured on |
| `mixed-dual-libp2p` | dual | libp2p | recovers over libp2p; iroh `Unknown` on the libp2p peer, never selected |
| `mixed-dual-iroh` | dual | iroh | recovers over iroh |
| `split-libp2p-iroh` | libp2p | iroh | **must red with the reason** `NoSharedTransport` (canon rule 5, verbatim) — a true finding about bootstrap, not a harness bug |

Survivor and recovering roles swap on alternate runs so neither slot is always the one measured. Each run is a restart of one slot, not a fresh mesh; conductors are untouched on warm return.

**Footprint line.** `hc-mesh.sh status` prints RSS and CPU per conductor/storage/doorway and a total, so the cost of any configuration is read from the same command that reports its health.

### 3.4 Cut 3 — the churn-injected quiescence measure

**One reader, two emitters.** `run-mesh-quiesce-stage.sh` (local) emits the same `fleet-quiesce[<ts>]: PASS|FAIL … — sustained Ns …` line grammar the fleet gate emits, so `genesis/scripts/quiesce-timeline.py` parses both into one series with a `source: local|fleet` field. No second parser.

**The proof shape.** For each diversity scenario × recovery shape, N runs (default 3) of: put the pair into the scenario → inflict the loss → start recovery → record `time_to_recover` (the four saga predicates above), then the local gate's `time_to_quiesce` and `best_window`, plus `sync_changes_applied{transport}` and `transport_route_total` deltas so the transport that actually carried the recovery is named, not assumed. Then the same runs with racing/selection **disabled** (`ELOHIM_TRANSPORT_SELECTION=off`, the flag Cut 1 ships behind, default *on* locally, default *off* on the fleet until the local proof lands). The claim "transport self-awareness improves recovery" is the before/after difference in `time_to_recover` (and `time_to_quiesce`) on the local mesh, per scenario × shape — and it is falsifiable: if the difference is inside run-to-run noise, the flag stays off on the fleet and the arc reports that honestly.

**Fleet confirmation, no redeploy.** Once the local proof exists: flip the flag in the alpha manifests, one deploy, then `[build:edge] [edge:validate-only]` measures without restarting the pods it measures. The fleet series and the local series are compared by the same reader.

## 4. Data flow

```
small op (head / list / small blob)          bulk op (large blob / full sync)
        │                                              │
        ▼                                              ▼
 select_path(peer, eligible, Small, obs)     select_path(peer, eligible, Bulk, obs)
        │  Race(both)                                  │  Single(best) or explore
        ▼                                              ▼
 FuturesUnordered{libp2p, iroh} ── first verified ──▶ result
        │ both outcomes                                │ one outcome
        ▼                                              ▼
 PathObservation.record(peer, transport, rtt, ok, hint)
        │
        ▼
 /p2p/status.transportPaths · elohim_transport_path_rtt_ms · elohim_transport_route_total
```

Existing consumers (`race_fetch`, the iroh sync driver, heal-on-read) keep their verification-before-finalize semantics; the observation is recorded *after* verification, so a fast wrong answer never scores as fast.

## 5. P2P design gate (condensed)

**Entity `PathObservation`** — Ephemeral (C). Local, asymmetric measurement (my RTT to you ≠ yours to me), fully reconstructable by re-observing, unverifiable if self-asserted, unbounded if notarized. Source of truth: in-memory ring. No table, no migration, no `dht_anchor_hash`. Keyed by `agent_cid` via the canonical resolver. Head-plane cost: **zero** — the design's main defense.

**Entity peer transport capability** — already exists (`elohim/transport/manifest` gossip → `IrohPeerBook` → `peer_transport_manifest`); extended, not minted.

**Decision point `select_path`** — composes inside canon's `peer_map::select_transport` (eligible-set refinement; Track3/NoShared untouched) — `pure-decision-predicate`, registered in `elohim/elohim-storage/seam-registry.yaml` with the C-answers in §3.1 and `contractTests` naming the C4/C3 tests (explicit `null` + `gapNote` for C7 until the drift cure lands).

**Network stakes** — behaves under all four stages; nothing here is floor-protected (no constitutional, local-relationship, or counter-evidence surface). The exploration fraction and race threshold are stage-priceable knobs.

**HTTP** — read-only projection on the existing `/p2p/status`; no new route. Anti-patterns checked: no UUID, no DHT entry, no per-host authored field, no k8s-plane modeling, no cross-namespace string compare.

## 6. Verification

- **Cut 0:** regression test on the status projection count; `just gate elohim-storage` green.
- **Cut 1:** contract tests for C4 and C3 named in the registry row; `placement-audit.py --epr-meta` census passes; `@concern:transport-diversity` scenarios in `genesis/a2o/features/dataplane/` assert `transportPaths` populated for every live pair and `Unknown` for the missing one.
- **Cut 2:** both recovery shapes run across all six scenarios on the two-peer mesh; five recover to all four saga predicates, `split-libp2p-iroh` reds with `NoSharedTransport`; footprint line present.
- **Cut 3:** the local series holds N × 6 scenarios × 2 shapes × 2 (flag on/off) records; the before/after table of `time_to_recover` is the arc's deliverable, one line per scenario × shape.
- **Habit delta:** one line in `genesis/manifests/habits.yaml` under `dataplane-convergence` naming the local before/after result and, when it exists, the fleet confirmation build.

## 7. Sequence, risks, decomposition rows

| # | row | depends on | risk named |
|---|---|---|---|
| 0 | `syncDocuments` → `count_all()` + test | — | none |
| 1 | `PathObservation` ring + metrics + `/p2p/status.transportPaths` | 0 | ring key cardinality — bounded by peer book |
| 2 | `select_path` predicate + registry row + C4/C3 contract tests | 1 | C4 regression is the one to guard |
| 3 | wire `Small` racing into head-record / list / small-blob paths (extending `race_fetch`) | 2 | verification-before-record must hold on both planes |
| 4 | wire `Bulk` selection + exploration into large-blob / full-sync paths | 2 | exploration must never pick a `Degraded` plane for a large transfer twice in a row |
| 5 | `syncVerdicts` structured reasons in `/p2p/status` + metric | — | keep bounded (top-N, counts) |
| 6 | `MESH_PEER_TRANSPORTS` + per-peer restart/assert/status in `hc-mesh.sh` | — | `assert_storage_transport_capability` reads the global mode today |
| 7 | recovery primitive (warm return / cold join) + six pair scenarios in `hc-mesh-transport-matrix.sh` + footprint line | 6 | cold join must mint a genuinely new identity (regenerate the sandbox), never reuse B's key |
| 8 | local emitter → shared `quiesce-timeline.py` reader (`source` field) | — | line grammar drift between emitters |
| 9 | before/after `time_to_recover` runs, N=3, per scenario × shape, flag on/off | 3,4,7,8 | noise floor unknown until measured; roles swap on alternate runs |
| 10 | flag flip in alpha manifests + `[edge:validate-only]` fleet confirmation | 9 | only if 9 shows a difference outside noise |

**The risk worth seeing now:** row 9 may show no measurable difference on a two-peer loopback mesh, where both transports are sub-millisecond. If so, the local harness still proves *correctness* of self-awareness (right routes, honest `Unknown`, honest red on the split pair) and the *magnitude* question moves to the fleet, where relay vs direct paths differ by orders of magnitude. The arc reports which of the two it proved.
