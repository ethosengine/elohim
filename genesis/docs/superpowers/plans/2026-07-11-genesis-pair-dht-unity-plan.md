---
id: genesis-pair-dht-unity-plan
title: Genesis-Pair DHT Unity — finish the substrate item behind notary scenario 2
status: active
class: plan
created: 2026-07-11
steward: rust-architect
cites:
  - genesis/data/timeline/backlog/genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md
  - peer-discovery-fractal-federation | Peer Discovery as Fractal Federation | sha256:42ae0e67f9e9d4bc | path: genesis/docs/superpowers/specs/2026-07-09-peer-discovery-fractal-federation-design.md
  - genesis/docs/superpowers/plans/2026-06-14-federation-bootstrap-plan.md
  - genesis/a2o/features/dataplane/notary-authority.feature
  - genesis/data/timeline/backlog/view-federation-request-flakiness-mesh-wide.md
---

# Genesis-Pair DHT Unity — finish the substrate item behind notary scenario 2

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
> Handoff authored 2026-07-11 at the close of shift `notary-scenario2-green` (sprint result: `.claude/shifts/2026-07-11T03-20-notary-scenario2-green.journal.md`). Operator direction folded in verbatim: *"if we need to add a simple upgrade signal scenario which drives a primitive holochain DHT mediated upgrade to bring the doorway CRDT sync plane together, we should do that, or develop a deeper understanding of where the holochain DHT network seams are"* — both legs are in this plan (T4 and T3), sequenced behind the cheap discriminators.

## 1. Mission + the question this answers

**Mission:** make the two doorways' conductor sets behave as the ONE Holochain DHT they already logically are, so a notarized canonical head declared anywhere resolves everywhere — finishing the cross-conductor-fetch backlog item and flipping notary-authority scenario 2 green.

**The operator's framing, answered precisely.** *"The holochain DHTs are treating the doorways still like two separate networks right? Doorways are creating an unexpected constraint."* — Almost exactly. It is **one DHT space** (both sides run the identical DNA hash; there is no second DHT) but **two transport clouds**: each conductor's `conductor-config.yaml` points at *its own doorway's* bootstrap and signal endpoints (`isRemote ? doorwayB : doorwayA` routing in `elohim/holochain/Jenkinsfile`). Doorways are Track-4 projection — they were never supposed to define network topology — but bundling per-doorway bootstrap+signal infra made each doorway an accidental **transport membrane**. Two membranes have been dissolved already (bootstrap: `MongoK2Store` shared table; signal: `MongoSignalBus` shared relay backing — both verified configured live, `/health dhtBacking.signalShared: true` on A and B). Cross-conductor **fetch still fails**, so at least one seam below the relay layer remains. This plan finds and closes it, then hardens the convergence loop so the CRDT sync plane and the DHT notary plane pull together instead of waiting on each other.

**The trust rule that bounds every fix here (do not violate):** a doorway/storage NEVER adopts a head from gossip or HTTP announcement. The DHT notary is the only authority (REQ-N5 / REQ-F4 anti-laundering, live in the serve path). Any "upgrade signal" is a **doorbell — "go verify against the DHT now"** — never a payload carrying authority. This is exactly how doorways trust notarized versions of the same EPR at near-global scale: announcements travel on any convenient plane; *verification always terminates in the receiving peer's own conductor.*

## 2. Verified ground state (2026-07-11, all live-probed)

WORKS (proven this shift, do not re-litigate):
- Election machinery live on BOTH genesis conductors (tier-aware cross-root selector; functional proof ×2: scenario 3 green = new guard refusing unauthorized moves; elohim.host's conductor answers the new fn's own error text from `content_store/src/lib.rs:3207`).
- Declaration act wired + firing EVERY app deploy: `authorHeadOnce` → `POST /db/content/{id}/canonical-head` → propagated to every doorway (`DECLARE_ONLY` leg, `scripts/ci/stage-spa-blob.sh`). Declaring side converges in-run (row+resolve = declared hash).
- Byte replication of the bundles: the redesigned blobs are ON elohim.host (`GET /blob/... 200`) — only the head POINTER is stale there.
- libp2p storage plane between the pair: HEALTHY (content-sync green, `caughtUp: true`, drain 4194/4194).
- Signal bus (MongoSignalBus) + shared bootstrap (MongoK2Store): deployed, configured, `signalShared: true` both doorways.

FAILS (the one blocker):
- elohim.host-side conductor(s) cannot RETRIEVE matthew-authored actions: every declare propagation returns `Guest("declare_canonical_head: target action ... is not retrievable")`. Watched divergent 48+ min across multiple freshly-authored heads. The old anchor (`f41d…`) still serves the OLD landing design on https://elohim.host while doorway-alpha serves the redesign — the user-visible face.
- **Free standing diagnostic:** every app deploy prints `✓ canonical head propagated to <doorway>` (healed) or the `⚠ not retrievable` warning (still down) in the `Upload SPA Blob` stage console. No manual repro needed, ever.

## 3. The seam map so far (extend in T3)

| Seam | State | Evidence |
|---|---|---|
| DHT space identity (DNA hash) | ONE space | identical DNA both sides (fractal-federation §2) |
| Bootstrap (kitsune2 discovery) | SHARED ✅ end-to-end | MongoK2Store, one `elohim-bootstrap` table; BOTH doorways' `/admin/bootstrap-coherence` read the identical store: 5 spaces × 35 agents = all 7 conductors publishing (probed 2026-07-11) |
| Signal relay (SBD, WebRTC handshake path) | DELIVERING CROSS-POD ✅ (T1 verdict 2026-07-11) | outside-in probe `doorway/doorway-service/tools/sbd-cross-relay-probe.py`: authenticated SBD clients on the public relays, all 4 legs DELIVERED (A→A, B→B, A→B, B→A) — the mongo bus bridges frames both directions at runtime |
| tx5/WebRTC session establishment conductor↔conductor | PRIME SUSPECT, sharpened: ICE is STUN-only, NO TURN anywhere | both conductor configs carry `ice_servers: [stun.cloudflare, stun.l.google]` and nothing else; adam (shem, cloud NAT) ↔ matthew (on-prem, home-router NAT) is a WAN NAT pair — srflx↔srflx with no TURN fallback is the classic silent-fail; live evidence readable via `GET /db/p2p/conductor-diagnostics?include=metrics` once deployed (T3 surface, built 2026-07-11) |
| Conductor pool fan-out behind elohim.host | AUDITED ✅ — deterministic, not intermittent (T2) | CONDUCTOR_URLS fans to ALL 7 humans' conductors on BOTH doorways (`computeConductorUrls`, edge Jenkinsfile); the primary write target differs per doorway: B→adam, A→matthew (CONDUCTOR_ADMIN_URL/STORAGE_URL). Declare/resolve/stamp ride the primary — so "elohim.host can't retrieve" = ADAM's conductor specifically |
| kitsune2 op-gossip vs explicit `get` | READ VIA T3 SURFACE | `?include=metrics` exposes per-DNA fetch queue (`peers_on_backoff`!) + gossip round summaries |
| ICE path between pods | NOT same-cluster — adam is shem-pinned (remote), matthew on-prem household (2026-05-27 placement directive) | WAN NAT traversal, not host-candidate ICE; the plan's earlier "both namespaces in one cluster" assumption was WRONG |

## 4. Tasks — cheapest discriminator first

### T1 — Runtime-prove the signal bus cross-pod path — ✅ DONE 2026-07-11: BUS DELIVERS, seam is below
- [x] ~~mongo `#[ignore]` test~~ superseded by a stronger proof: mongo is cluster-internal (unreachable from dev), so an OUTSIDE-IN probe was built instead — `doorway/doorway-service/tools/sbd-cross-relay-probe.py` speaks the SBD protocol against the two PUBLIC relays with real ed25519 auth. All 4 legs DELIVERED (controls A→A/B→B; cross A→B and B→A). This is end-to-end through ingress+relay+bus+relay — strictly stronger than the mongo-only unit proof.
- [ ] (hardening, non-blocking) Cat-C bus counters on `/health dhtBacking` — frames published / drained / delivered-to-local, per origin_relay.
- [x] Bus IS delivering → the seam is below. T2/T3 executed.

### T2 — Pool-topology audit behind elohim.host — ✅ DONE 2026-07-11 (read-only)
- [x] Routing is deterministic, not intermittent: `computeConductorUrls`/`computeStorageUrls` (edge Jenkinsfile) fan ALL 7 active humans' conductors/storages to BOTH doorways; each doorway's declare/resolve/stamp path rides its PRIMARY (CONDUCTOR_ADMIN_URL/STORAGE_URL): doorway-B→adam, doorway-A→matthew. The failing fetch is adam's conductor, full stop.
- [x] Network config per member: every conductor gets its own doorway-affinity bootstrap/signal endpoints (`isRemote` split), all STUN-only (`stun.cloudflare` + `stun.l.google`), NO TURN deployed anywhere in the manifests. adam(shem/cloud NAT) ↔ matthew(on-prem/home NAT) therefore has no relay fallback if srflx pairing fails — the sharpest remaining suspect, to be confirmed/refuted by the T3 diagnostics read.
- Note (T2 side-finding): doorway-B's routed storage answers `connectedPeerCount: null` on `/api/v1/federation/p2p-peers` while doorway-A reports a 6-peer libp2p mesh — the adam-side storage's libp2p visibility needs the same T3 read once the diagnostics route deploys (per-host deploy lag vs genuinely down: distinguish there, not by assumption).

### T3 — DHT seam map: settle "one network or two" with conductor-level evidence (Loki legs @requires:observability)
- [x] Diagnostic surface BUILT (2026-07-11): `GET /db/p2p/conductor-diagnostics` in elohim-storage — conductor peer store (`agent_info` projection: agent/space/url/timestamps/tombstone/arc) + `dump_network_stats` (live transport connections, blocked counts); `?include=metrics` adds per-DNA `dump_network_metrics` (fetch queue incl. `peers_on_backoff`, gossip round summaries). Manifest-declared → doorway auto-forwards; Cat-C, no auth (agent infos already public via shared bootstrap). Deploys with the next edge build.
- [ ] Per-conductor kitsune2 peer visibility: once deployed, read BOTH doorways' `/db/p2p/conductor-diagnostics` — does matthew's agent url appear in adam's peer store and vice versa? (Route rides the primary conductor per T2, which is exactly the failing member.)
- [ ] tx5 session evidence: WebRTC offer/answer attempts + outcomes in conductor logs during a declare-propagation window (correlate with the free per-deploy probe).
- [ ] Distinguish op-gossip from explicit `get`: does ANY matthew-authored op arrive on the elohim.host side (new anchors appearing over hours = gossip alive, get broken) or NOTHING (transport fully down)? The 2081-divergent-anchors history says gossip once worked — establish when it stopped (correlate with F-T19 onset).
- [ ] Write the findings INTO the fractal-federation spec §2 (update the live diagnosis) — the seam map is the deliverable the operator asked for, not a side note.

### T4 — The upgrade-signal scenario (operator direction; build AFTER T1-T3 name the seam, alongside the fix)
The primitive: **a DHT-mediated upgrade doorbell.** When a canonical head is declared, receiving peers should not wait for passive op-gossip + a heal-tick coincidence; they should be *told to go look*. Two candidate carriers, both trust-safe (doorbell only, verification stays conductor-local):
- (a) **DHT-native:** the canonical-head link already IS the DHT signal — add a conductor `remote_signal` fan-out to known peer agents on declare, so remote conductors trigger their own `resolve → stamp` immediately. Purest, but rides the exact transport being repaired.
- (b) **Working-plane doorbell:** announce `canonical-head-changed {content_id}` over the HEALTHY libp2p storage gossip plane; the receiving storage triggers its own conductor resolve + heal-stamp (never adopting the announced value — REQ-N5). Converges the CRDT sync plane and the notary plane the moment T1-T3's fix lands, and degrades harmlessly when transport is down (resolve just fails until it heals).
- [ ] Write the a2o RED first: `genesis/a2o/features/dataplane/notary-authority.feature` gains scenario 5 — *"A declared canonical head rings the upgrade doorbell — every federation peer re-verifies and adopts within one sync window (no deploy, no heal-tick coincidence)"* (@requires:multi-node). Story-first: the scenario is the spec.
- [ ] Implement carrier (b) (compose-don't-fork: reuse the transport-neutral `gossip_dispatch` handler + shared DedupLru from the Dual-transport work; new topic, one handler body that calls the existing heal entrypoint for the named id).
- [ ] Keep carrier (a) as the follow-on once T1-T3 heal transport (it then supersedes (b)'s doorbell for DHT-native purity, or they coexist — decide in review).

### T5 — Settle the architecture: doorways must stop being accidental membranes (design session, p2p-design-gate)
- [ ] Brainstorm+design: conductor network config should reference a FEDERATION-level bootstrap+signal set (the Tier-A domain backing from the fractal-federation design §6), not per-doorway endpoints — so adding a doorway never partitions transport again. Includes the near-global trust question: Tier-A (shared backing within a domain) vs Tier-B (earned/governed cross-domain federation) is the scaling seam; the notary-terminated verification rule (§1 above) is what makes near-global trust of "the same EPR" safe at any scale.
- [ ] Route through `p2p-design-gate` (entity classification for any new registration/announce entities) and update the fractal-federation spec rather than forking a new one.

## 5. Definition of done
- [ ] Notary-authority scenario 2 green ×2 consecutive fresh-trigger edge builds (the standing shift measure: `sprint-report-dataplane.json byConcern.notary-authority.failed == 0`).
- [ ] https://elohim.host serves the redesigned landing (visual: `pnpm look` both doorways, same hero) — the user-visible proof.
- [ ] The per-deploy probe prints `✓ canonical head propagated to https://elohim.host` in the app pipeline console.
- [ ] Seam map written into fractal-federation §2; T5 design decision recorded (or explicitly deferred with its own backlog item — no dumps).

## 6. Watch-outs (from the shift's burned ground)
- Trigger-pipeline mismatch: the actuator (declare) rides the APP pipeline; the measure rides EDGE dataplane validation. Sequence actuator→measure; one tag does not cover both.
- Every app deploy re-authors the head (new action hash per run) — never chase a specific hash; the propagation is idempotent-by-content.
- Measurement-during-churn: validation runs right after deploy restarts; peer-mesh `caughtUp=false` reds during that window are churn artifacts.
- Jenkins MCP is anonymous (no triggerBuild); fresh triggers via `git push` `[build:app]`/`[build:edge]`. Cluster ops are operator-owned — no kubectl from dev; Loki legs are `@requires:observability` (and note: the observability MCP was absent even from ci-investigator's runtime toolset this session — registration drift worth an operator look before T3 relies on it).
- The retired amber write and any per-host head write remain FORBIDDEN cures (divergent un-witnessed heads — the disease itself).
