---
title: "Doorway-federated continuity — one doorway-set name, CDN-shaped projections, blobs from any holder, and a steward's household surviving its own node — on either transport"
id: doorway-federated-continuity-roadmap
status: Draft
class: substrate
domain: doorway projection (T4) × peer-hoster dataplane (T2) × delegates-compute (3.4) × confidentiality/recovery (3.13)
sprint: proposed (Lane 0 in flight 2026-08-23; Lanes A/B/T are schedulable; Lane C opens with a design-gate session)
habits: [doorway-failover, blob-durability, dataplane-convergence]
cites:
  - "doorway-federation-failover-sprint-plan | the 2026-07-31 sprint this roadmap extends — its WS2 (JWKS, HostedAgentBinding, fork guard) is landed and its WS3 read-path anycast tasks are this roadmap Lane A | sha256:c66fd04c3b4f16e2 | path: genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md"
  - "dual-wan-utility-plane-failover | the three anycast horizons (multi-A+client retry / Cloudflare LB / BGP) this roadmap grades Lane A against; only 3a is built | sha256:86f425b0045ce6d0 | path: genesis/docs/superpowers/specs/2026-07-16-dual-wan-utility-plane-failover-design.md"
  - "elohim-seam-map-concern-routing | the atlas the subject was routed through — T4 doorway, T2 peer-hoster, 3.4 delegates-compute, 3.13 confidentiality; exclusions named there | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "holochain-iroh-convergence-upgrade-campaign | why the storage iroh pin stays at 0.92 until Wave 3 — Lane T builds on the pinned stack, not the 1.0 lift | sha256:381e7dad57e8cd23 | path: genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md"
  - "rea-compute-substrate-native-roadmap | names the unbuilt serve-url-projection scope that Lane C4 turns into the third-party delegation instance | sha256:64e5ffe3b8756e6e | path: genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md"
  - "private-layer-blind-custody-resiliency-floor | the held private-replica design; Lane C6 stays held until KeyEnvelope lands, so read continuity is public-footprint only | sha256:1dd9950a41c2ff73 | path: genesis/docs/superpowers/plans/2026-08-09-private-layer-blind-custody-resiliency-floor.md"
  - "2026-06-23-recovery-floor-witness-reconnection-ioc-design | documents the cross-DNA witness severance Lane C5 repairs (DNA-hash-moving) | sha256:c76fa4470a91c78d | path: genesis/docs/superpowers/specs/2026-06-23-recovery-floor-witness-reconnection-ioc-design.md"
  - "iroh-pkarr-resolver | gate #10 — the doorway pkarr relay Lane A5 lights by enabling it in a reconciled manifest | sha256:589150a22f167bad | path: genesis/docs/superpowers/plans/2026-05-10-iroh-pkarr-resolver.md"
  - "doorway-catching-up-page | the shed contract (503 catching-up body, /health/serving) that health-aware doorway-set membership (A1) consumes | sha256:2dbde4d56b074a5e | path: genesis/docs/superpowers/specs/2026-07-19-doorway-catching-up-page-design.md"
memory_anchors:
  - feedback_local_mesh_first_cadence
  - feedback_delegate_narrow_tasks_to_cheaper_tiers
  - feedback_subagent_disjointness_read_write
  - agent-agnostic-backlog-delegation
  - project_content_sync_plane
  - project_two_premises_dns_beacon_owned
  - feedback_p2p_vs_federation_layer_vocabulary
---

# Doorway-federated continuity — roadmap

> **For agentic workers:** this is a multi-lane roadmap, not one sprint. Each lane's tasks carry a tier
> (Opus / Sonnet / Codex-or-Gemini) and a tree; claim a task only if its tree is disjoint from every task
> currently `wip`. Prove on the local mesh first; the fleet CONFIRMS (one `[build:edge] [edge:validate-only]`
> per batch of ~10 commits), it never discovers. Every lane serves an existing habit — no new habit is
> minted (the register is 12/12 with the max-2 `active` fence already spent on `doorway-failover` +
> `operator-runtime-surface`).

**Goal.** A person reaching the commons through *any* doorway — including a library terminal while their
household node is dark — gets: (1) a name that always resolves to a serving doorway, (2) projections that
are the same content-addressed bytes whichever doorway serves them, (3) blobs delivered from whichever peer
holds them, with the p2p dataplane as the fallthrough, and (4) read continuity for their own household
footprint from the peers they hold custody/compute commitments with — with write continuity designed
through a bounded, revocable delegation rather than key copying. All of it measured on **both** transports
(libp2p and iroh) because alpha runs `dual` and the mesh has never run iroh.

**The thesis in one line.** Selection-time, never per-request: a doorway chooses ONE upstream per read,
but chooses it from a set it can see the health of — steward peers for blobs today, a household's replica
set tomorrow; the same predicate (`select_route`) generalises from "pool peer whose breaker is closed" to
"custody peer the commitments name". DNS does the same one level up: a doorway-set name that a shedding
doorway leaves and a serving one joins. Nothing iterates; everything selects.

## Where we are (grounded 2026-08-23, wired-vs-spec — evidence is file:line; do not re-derive)

**Doorway seam (T4).** Apex `elohim.host` is single-owner (shem beacon leg `--record-name`); the
doorway-set `doorways.elohim.host` shared lane is WIRED on both legs; apex multi-A was REVERTED on the
beacon's single-slot flag — **cleared 2026-08-23** by Codex (`relay-addr-beacon` repeatable
`--shared-record NAME=OWNER`, 43 tests), the apex flip itself stays operator-owned. Browser failover is
WIRED and client-side (`environment.prod.ts:29` / `environment.alpha.ts:29` `doorwayFallbacks`,
`api-base-url.interceptor.ts:233`, sticky-preferred); **no health-aware DNS exists** (the beacon is DDNS
for its own leg). `bootstrap_url` / `signal_url` are single `Option<String>` (`config.rs:274,279`) — the
conductor plane has no fallback list. Cross-doorway trust is WIRED: JWKS dual-verify (`auth/jwt.rs`
~500-565, `e3399c0ae`), `HostedAgentBinding` A2 link + honest `307`/`409` landing (`271b41a65`;
`chaperone.rs:564-569, 682-720, 650-667, 727-739`; storage `GET /api/v1/federation/hosted-binding/…`
at `elohim-storage/src/http.rs:1461`), silent-fork guard (`73347ba90`). The EPR projection path already
selects across a pool (`projection/epr_router.rs:177-238 fetch_projections_with_fallback`).
`fetch_from_remote_doorway` (`services/federation.rs:374`) is DEAD with a lying doc comment.
Ed25519 node key is regenerated each boot → the EdDSA minting flip is blocked (operator item 3).

**Blob delivery.** Doorway `/blob` forward was pinned to the declared primary — **cured 2026-08-23**
(`select_route`, `server/http.rs:115`; counter `doorway_blob_target_failover_total`; a2o "A blob rides
through its primary's bad hour"). Storage serve-from-any-holder is WIRED: `get_blob_or_heal`
(`http.rs:3414`) → `race_fetch_with_swarm` (`:3664`, `p2p/blob_swarm.rs:333`, libp2p `BlobCodec`) →
named 404s (`:3767-3801`); the serve-blob REA row is booked atomically (`p2p/blob_fetch.rs:365,409`).
**Found 2026-08-23:** any blob over `RS_THRESHOLD` (64 MiB) panicked `PUT /blob` (`http.rs:2617` — the
`rs-4-7` manifest was sliced as raw chunks); the landing SSR bundle crossed the line with source maps.
Cure in flight (Lane 0).

**Custody / replica set.** Custody = `rea_commitments action='custody-blob'` (`reconcile/custody.rs:1-22`);
salvage self-authors successors through `PlacementStrategy` (`reconcile/placement.rs:67`; diversity-aware
default `config.rs:722-727`) — deterministic, **not socially scored**; initial custodians are operator seed
(`genesis/seeder/src/seed-commitments.ts`). `custody_rotation.rs` re-pledges existing custodians only.
The household footprint: source chain is node-local and never gossiped (`api/source_chain.rs:8-17`) —
NOT replicable by design; public projections and Automerge docs are replica-servable; **private data is
SPEC-ONLY** (`services/private_replica.rs` is a single-host proof; `KeyEnvelope` entry HELD).

**Delegated authority.** `delegates-compute` is a WIRED Mishpat commitment kind with bounds + revocation
(`mishpat/src/commitments.rs:183-204`, schema `schemas/v1/commitments/delegates-compute.schema.json`);
enforced scopes are `republish-epr` and the `orchestrate-node` op-gate, which **requires performer ==
recipient** (`services/operation_authorization.rs`). The `serve-url-projection` scope — the one that would
let a peer or doorway-hosted conductor serve on a household's behalf — is SPEC-ONLY
(`2026-05-28-rea-compute-substrate-native-roadmap.md:665-693`).

**Social recovery.** Zome machinery exists (`create_recovery_request` `imagodei/src/lib.rs:2719`,
`submit_intimate_witness` `:3902`, entry types `imagodei_integrity/src/lib.rs:550-746`, gossip topics
`p2p/mod.rs:2383,2442`), but the path is SEVERED: the coordinator writes the witness cross-DNA with a
zero-sentinel ActionHash and the validator (`recovery_v2.rs:363`) needs an in-DNA `HumanityWitness`, so an
`IntimateQuorum` KeyRotation can never validate; doorway recovery routes are `501`
(`auth_routes.rs:2970,2984,3030`). `steward/node` and Tauri have no "home node offline → use replica" path.

**Dual-plane transport.** `ELOHIM_TRANSPORT_BACKEND` = Libp2p | Iroh | Dual (`config.rs:36-41`); alpha runs
`dual` (`alpha.yaml:290`); the household mesh sets neither the env nor the `p2p-iroh` feature — **it has
never run iroh**. Sync, inventory gossip and view federation are genuinely DUAL (shared `SyncManager`,
`DualGossipPublisher`, `gossip_receive.rs`, `view_fed.rs`). **Blob heal-on-read and custody push are
libp2p-only**: `IrohNode::fetch_blob_from` (`p2p_iroh/node.rs:146`) has zero production callers;
`IrohBlobStore::get_bytes` is a local read; `transport_resolve.rs:47-67` extracts only the libp2p id. In
`Iroh` mode there is no peer blob fetch at all. Loopback benches say iroh wins p50 45-541× on small frames
and 1.2-128× on blobs (`p2p_iroh/README.md`); no WAN measure exists.

**pkarr.** Four stations built, none lit: doorway `/pkarr/{z32}` relay behind
`DOORWAY_PKARR_RESOLVER_ENABLED` (`services/pkarr_resolver.rs`, gate #10 plan), beacon pkarr sink
(`relay-addr-beacon/src/sinks/pkarr.rs`, unreferenced by manifests), iroh discovery hook
(`p2p_iroh/config.rs:15-43`), signed doorway-endpoint record in the infrastructure zome
(`bridges/pkarr`, `09d7a026e`, DNA-hash-moving, no reader). Nothing resolves a pkarr name into a
doorway-set address list.

## Design gate (p2p-design-gate — pre-answered; re-confirm in the Lane C session)

- **Doorway-set membership record** (who serves a name right now) — **C / ephemeral** today: DNS records
  written by the beacon from `/health/serving`; no DHT entry. The durable exit (pkarr-native doorway set)
  would be **A2** — a link off the existing signed `DoorwayRegistration`/endpoint record, not a new entry.
- **Household → replica set** — **A2 derived**: the set IS the active `custody-blob` commitments naming the
  household's artifacts; no new entry. The doorway learns it by reading commitments through storage.
- **Act-for-household delegation** — **A**, but as an *instance* of the existing `delegates-compute`
  commitment (new `scope`, e.g. `serve-household-projection` / `author-for-household`), bounds
  `{epr_scope, reach_ceiling, rate_per_hour, rotation_ttl_days}`, on-chain revocation. Recipient is the
  delegate (a replica peer or the doorway's chaperone conductor), performer is the delegate — which means
  lifting the `performer == recipient` self-grant rule into a *third-party* grant: this is the one real
  design decision and it is gate-mandatory. The steward's own key is never copied.
- **Private replica** — stays HELD (blind-custody plan); read continuity for private data is out of scope
  until `KeyEnvelope` lands.

## Lanes

### Lane 0 — this session (prove, then push)
| # | Task | Tier | Tree | State |
|---|---|---|---|---|
| 0.1 | Selection-time blob failover (`select_route`) + a2o story | Opus | doorway-service, a2o | landed `1bd802b37` |
| 0.2 | RS-band `PUT /blob` panic → shard through the encoder; reconstruct through parity; a2o ">64 MiB artifact accepted whole" | Opus | elohim-storage, a2o | in flight |
| 0.3 | Doorway gate under rustc 1.98 (`result_large_err` ×3) + seed-forward budget scales with size; pantry hit re-attempts the forward | Sonnet | doorway-service | in flight |
| 0.4 | Beacon repeatable shared lanes | Codex | relay-addr-beacon | landed `906d7b159` |
| 0.5 | Local proof: scoped household lane on the two new stories, then the full lane; no new red vs run `20260823T000551Z` | orchestrator | mesh | next |
| 0.6 | Push + `[build:edge] [edge:validate-only]`; status flips are the operator's | operator | fleet | after 0.5 |

### Lane A — one name, many doorways (logical anycast, spec §3a; BGP §3c stays vision)
| # | Task | Tier | Tree |
|---|---|---|---|
| A1 | Health-aware shared-lane membership (join/leave on `/health/serving`, hysteresis) — backlog `beacon-health-aware-shared-lane-membership` | Codex | relay-addr-beacon |
| A2 | Apex multi-A manifest diff (both legs contribute `elohim.host`; ingress host + TLS SAN on A) — committed, **operator applies** (menu item 2) | Sonnet → operator | orchestrator manifests |
| A3 | Conductor-plane multi-URL: `bootstrap_url`/`signal_url` become ordered lists; the conductor client retries the next on connect failure (tx5-fork path per the 2026-07-16 spec §7) | Opus design, Sonnet impl | doorway-service config + conductor client |
| A4 | Doctrine call (made here): **delete** `fetch_from_remote_doorway` and its FEDERATION.md claims — storage's race-fetch IS the p2p fallthrough; a doorway→doorway blob hop is a third tier at the wrong layer | Codex | doorway-service (after Lane 0 lands) |
| A5 | pkarr doorway-set: one design-gate session joins the four stations (signed endpoint record → resolver → client address list); then enable `DOORWAY_PKARR_RESOLVER_ENABLED` in a *reconciled* orchestrator manifest to start gate #10's one-week clock | Opus (design) → Sonnet | doorway-service, manifests, app client |
| A6 | a2o: generalise "same declared truth" from the pair to the set (N doorways, one head) | Sonnet | a2o |

### Lane B — CDN-shaped projections
| # | Task | Tier | Tree |
|---|---|---|---|
| B1 | `X-Content-Address` (CIDv1) + `x-elohim-served-head` on every projection and blob response (today: apps + storage proxy); doorway response cache keyed by content address so any doorway serves identical bytes | Sonnet | doorway-service |
| B2 | Cross-doorway cache-fill is *selection* too: on a projection miss, pick a sibling doorway whose `/health/serving` is green (pool = `DoorwayRegistration`), never iterate | Opus design, Sonnet impl | doorway-service |
| B3 | Cloudflare LB (§3b) — **operator ceiling**, default no (new borrowed dependency needs its own exit row) | operator | — |

### Lane C — the steward's household survives its own node
| # | Task | Tier | Tree |
|---|---|---|---|
| C1 | **Read continuity now:** a doorway's steward-peer pool for a household's artifacts = the custody peers its commitments name; `select_route` gains a household-scoped candidate source (derived A2, no new entry); a2o: "my household node is dark; my public footprint still reads through a foreign doorway" | Opus | doorway-service + elohim-storage read route |
| C2 | Custody coverage for drill content (seed) — backlog `seed-custody-coverage-for-drill-content` | Codex | seeder + prologue |
| C3 | Socially-informed placement: `PlacementStrategy` input from affinity/standing (P3-8), diversity stays the floor | Opus design, Sonnet impl | elohim-storage |
| C4 | **Write continuity (design gate session):** third-party `delegates-compute` scope (`author-for-household` / `serve-household-projection`), bounds + revocation, chaperone conductor as the delegate; answer the gate's 5 questions; spec before any route | Opus | mishpat zome (coordinator-only if the scope is data, integrity if validation changes → DNA hash moves) |
| C5 | Recovery severance: in-DNA `HumanityWitness` path so `IntimateQuorum` rotation validates; doorway recovery routes off `501` | Opus | imagodei integrity+coordinator (**DNA-hash-moving**: alpha genesis pair both need `ALLOW_DNA_REINSTALL`) |
| C6 | **Design pass done 2026-08-23 → spec `swarm-curve-and-blind-custody-design` §4-§6 (rows C6-a reader key via `attestation:key-stewardship` reuse + conductor-signed record · C6-b bound signed ring with mandatory floor envelope, `StorableBytes` at the store · C6-c fail-closed custody gate + commitment-gated push · C6-d quarantine/refuse actuator across eight planes). Red-teamed 2026-08-23, 36 findings folded — the first draft's Shamir-in-ring and `attestation:reader-key` are withdrawn.** **Blind custody graduation** (operator vision 2026-08-23: Adam replicates Matthew's whole household — love map included — and Matthew Adam's, neither able to read the other's private content): graduate `services/private_replica.rs` (encrypt → RS-4-7 → sealed-DEK envelope, single-host proof) onto the existing `custody-blob` action; ONE design-gate decision — `KeyEnvelope` as a per-reader sealed DEK (A2 link off the manifest, or a new entry if the gate says so); add a `blind` marker so the substrate can tell holds-and-serves from holds-and-reads. Inherits the swarm for free (it is encryption-agnostic). Recovery half depends on C5 | Opus (design gate) → Opus impl | elohim-storage + mishpat (+ imagodei if the reader-key resolver needs it) |

### Lane S — the swarm curve (operator vision 2026-08-23: shards sync faster as more shards are replicated)
The mechanism exists: `p2p/blob_swarm.rs` races each shard independently across a rotated holder list with
per-shard `serve-blob` credit; a manifest-only holder answers `FetchOutcome::Manifest` so the requester
pivots to the swarm. What flattens the curve today, and the rows that un-flatten it:
| # | Task | Tier | Tree |
|---|---|---|---|
| S1 | **Design pass done 2026-08-23 → `swarm-curve-and-blind-custody-design` spec.** Grounding correction: shard inventory IS gossiped (every shard is its own blob; the swarm already does per-shard `lookup_hosts`) — the gap is shape. Row S1′ = shard **bitfield** hint on the composite address (`BlobHint.shards_held`), `peer_blob_inventory.shard_bitfield`, broadcaster fold; backlog `2026-08-23-shard-level-inventory-gossip` re-pointed | Sonnet (wire frozen in spec §3.1) | elohim-storage (inventory + blob_swarm) |
| S0 | **Prerequisites surfaced by the 2026-08-23 red-team (spec §3 F0/F0′/F0″, §9):** S0-a re-hash the reassembled composite against `blob_hash` (a peer-supplied manifest is trusted today); S0-b push wire carries `manifest_cid` + `shard_index` and the receiver persists membership (a pushed shard has no manifest association); S0-c custody presence manifest-aware (every RS commitment re-kicks forever); S0-d doorway blob pantry gating (backlog `security-doorway-blob-pantry-ungated`) | Sonnet / Codex | elohim-storage `blob_swarm.rs`, `shard_service.rs`, `reconcile/custody.rs`; doorway `storage_proxy.rs` |
| S4 | Per-shard `BlobInventoryDelta` with the bit set in the `FetchOutcome::Hit` arm — today the delta fires once per completed composite, so the curve is cadence-bound (~60 s snapshot), not bandwidth-bound; **this is the superlinear enabler** | Codex | elohim-storage `blob_swarm.rs`, `p2p/mod.rs` |
| S5 | Band collapse: retire `chunked` for new writes (`> SINGLE_SHARD_MAX → rs-4-7`, `> 256 MiB → rs-8-12`); reads keep `chunked`. The 16–64 MiB band is where most content lands and has no fastest-k property | Sonnet | elohim-storage `sharding.rs` |
| S6 | Scarce-first shard ordering in `plan_shard_holders` (ascending holder count; pure fn) | Codex | elohim-storage `blob_swarm.rs` |
| S2 | Data-vs-parity role derived from the manifest (`ShardRole`, `plan_shard_placement_slots` — **in flight, uncommitted in the shared worktree 2026-08-23**); no registry/DNA field needed | — | elohim-storage `sharding.rs`, `p2p/mod.rs` |
| S3 | Measure the curve: a2o scenario that fetches one RS blob with 1, 2, 3 holders and asserts wall-clock falls (parameter-bearing; household lane) | Sonnet | a2o |
| S4 | iroh leg = T2 — on iroh use iroh-blobs' native multi-provider range streaming rather than re-implementing the race | Opus | elohim-storage |

### Lane T — rock-solid on either transport
| # | Task | Tier | Tree |
|---|---|---|---|
| T1 | Mesh transport knob (`MESH_TRANSPORT_BACKEND`, `p2p-iroh` mesh binary, mode in the report) — backlog `mesh-transport-backend-knob` | Codex | scripts + justfile |
| T2 | iroh peer blob-fetch in heal-on-read: candidates carry iroh NodeIds from `peer_transport_manifest`; `race_fetch_with_swarm` races both planes in `dual`, iroh alone in `iroh` | Opus | elohim-storage |
| T3 | Custody push over iroh (`transport_resolve` returns the iroh id; `push_shard` via the mounted `IrohShardProtocol`) | Opus | elohim-storage |
| T4 | `inventory_fetch` on the iroh receive path (`gossip_receive.rs:19-26`) | Sonnet / Codex | elohim-storage |
| T5 | Habit twin: `cargo test --test sync_iroh_convergence` beside the libp2p check in `dataplane-convergence`; a2o `@transport:` tag so the household lane reports per mode; CI mesh stage runs `libp2p` and `dual` | Sonnet | elohim-storage tests, a2o, CI |
| T6 | WAN direct-connection ratio meter (existing backlog) | Codex | elohim-storage |

## Sequencing

```
Lane 0: 0.1 ✓ → 0.2, 0.3 (parallel, disjoint crates) → 0.5 local proof → 0.6 push/measure
Lane A: A1 (now) · A4 (after Lane 0) · A2 (operator) · A3 → A5 (design gate) · A6 anytime
Lane B: B1 → B2 (after A-set health is observable)
Lane C: C2 (now) · C1 (after 0.2) → C3 · C4 design session (before any write-continuity code) · C5 own deploy ceremony
Lane T: T1 (now) → T2 → T3 · T4/T5 anytime after T1 · T6 anytime
```
WIP fence: the two `active` habits are unchanged; every task above reports its delta under
`doorway-failover`, `blob-durability`, or `dataplane-convergence`.

## Delegation matrix (who gets what, and why)

- **Opus (rust-architect):** anything cross-module or truth-layer — T2/T3, C1/C4/C5, A3/A5/B2 design. High-risk
  areas (auth, shared state, DNA) are Opus-implemented and reviewed by a different agent, never self-certified.
- **Sonnet:** scoped implementation against a decided design — B1, A6, T4/T5, A2's manifest diff, 0.3.
- **Codex / Gemini (agent-agnostic backlog, `codex-claimable`):** crate-local work with a test oracle and a
  disjoint tree — A1, A4, C2, T1, T6, plus the "after Lane 0" queue. Each is a conformant backlog entry with
  scope + DoD + the exact verification command; claim by flipping `status: wip`.
- **Haiku:** evidence only — lane-report diffs, gate summaries, checklist verification.

## Regression guard (the DoD every task inherits)

1. The touched tree's `gate` exits 0 (`GATE_EXIT=0` echoed, never judged from piped output).
2. Scoped household lane on the task's story is green; the **full** lane shows no new red versus the last
   recorded run (baseline `20260823T000551Z-32aff87a`: 186/25/4/29). Reds are classified in the habit delta.
3. For Lane T: the story runs in `libp2p` AND `dual` (and `iroh` once T2 lands); a mode-specific red is
   recorded, not hidden.
4. One push per batch; the fleet run is `[build:edge] [edge:validate-only]` unless a deploy is intended.
5. A one-line delta in `habits.yaml`. Status flips are the operator's on fleet evidence.

## Operator menu (ceiling — decisions, not agent work)

1. Apex multi-A flip timing (A2) — mechanism ceiling cleared; DNS semantics change.
2. Persisted doorway Ed25519 node key (k8s secret / PVC) → then the EdDSA minting window.
3. Cloudflare LB — adopt or not (default: not).
4. Lane C5's DNA-hash-moving deploy ceremony (both alpha genesis peers).
5. Lane C4: whether the chaperone conductor lives on the doorway host (shem-class) or on a household replica
   peer — the design session proposes; you decide placement.
