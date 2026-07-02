---
id: "backlog-peer-hoster-async-sync-readiness-assessment"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Always-On Peer-Hoster Readiness Assessment — Async Store-and-Forward CRDT Sync"
slug: "peer-hoster-async-sync-readiness-assessment"
written: "2026-06-21"
author: "workflow:peer-hoster-async-sync-readiness"
status: "backlog"
priority: "medium"
tags: [peer-hoster, always-on, crdt, automerge, store-and-forward, peerid-addressing, durability, readiness-assessment, seam-3-10]
# OPEN concern: point-in-time readiness assessment (verdict NO — async store-and-forward CRDT sync
# does not work end-to-end peer-natively yet). The gap list is the backlog. Routed out of
# .claude/data 2026-07-02 (machine-ledger law); referenced by the seam map (seam 3.10). NOTE:
# partially advanced by the 2026-06-27/07-01 Automerge content-sync plane landings — re-verify
# section-level verdicts against sync/projector.rs + p2p/mod.rs before acting.
---

# Always-On Peer-Hoster Readiness Assessment

**Role under assessment:** an always-on node that supports the network by HOSTING PEER-NATIVE STATE (async store-and-forward CRDT sync, peer-native SSR, durable availability) for intermittently-connected spokes — addressed by **peerId, NO DNS, NOT a public doorway**.

**Synthesized from 5 verified, adversarially-checked briefs:** async store-and-forward CRDT sync · peer-native SSR · peerId addressing/discovery · durability/availability · today's actual app dataplane.

---

## 1. THE DECISIVE VERDICT

**Does "Person A edits offline → syncs to the always-on node → Person B (never concurrent with A) later syncs A's changes from the node" work END TO END today, peer-natively?**

### NO.

The peer-native loop does **not** complete by any path today. The CRDT machinery is durable and the node↔node sync loop genuinely runs, but **no deployed client ever feeds a CRDT store**, and there is **no DNS-free way for a spoke to reach its household node** in the first place.

**Single biggest blocker:** *No deployed client writes to any peer-native CRDT store.* The Angular app has zero Automerge client code (`sync.ts:24` self-attests "no browser caller today"), and the doorway **deliberately excludes** `/sync` from its route manifest (`http.rs:1652-1653`: "Infrastructure endpoints (/health, /shard/\*, /sync/\*, /import/\*) are intentionally omitted"). The CRDT serve/apply/durable-sled/node↔node-loop substrate is fully built and running — but nothing puts a document into it, so A→node→B never even begins via CRDT.

**Second, vision-specific co-blocker:** *No DNS-free reach to the household node exists.* mDNS is disabled in production AND dial-less (discovery never becomes a connection); there is no peerId→"my household relay" binding; the node listens on `0.0.0.0/tcp/0` with no `announce_addresses` and its p2p service is cluster-internal (headless `clusterIP: None`, no LoadBalancer/NodePort/Ingress). Every running path — both the conductor (kitsune) plane and the libp2p content plane — rendezvous through a **public DNS-named doorway**. (Removing the writer/route gap alone gets a working async loop *centrally via doorway*; removing the reach gap alone still delivers nothing because no client feeds the store — which is why the writer/route gap is the *tightest* blocker and the reach gap is the *vision-defining* one.)

### The honest nuance the reader must keep

The **same user-observable outcome** (A-offline → B-later, non-concurrent) **IS achieved today** — but via the centralized **conductor-DHT → content.db** path through a public DNS doorway in front of one SQLite-backed node. That centralized analog is *exactly the thing the peer-hoster role is meant to replace.* So: **NO on the peer-native axis**, even though a centralized substitute delivers the outcome.

**Concurrency is NOT the blocker.** A and B never need to overlap — the node's durable sled store intermediates, surviving disconnect and restart. The precise model:
- The **store is async-capable** (durable sled on PVC; serve-half replays a doc to a later reader even across a restart).
- The **exchange legs are connection-based**: node↔node CRDT delta exchange is libp2p request-response (early-returns on `peers.is_empty()`, `p2p/mod.rs:6813`), and blob byte transfer is request-response (gossip carries inventory *metadata only*). Both endpoints must be online *at exchange time* — but A and B themselves never need to be online together.

---

## 2. READINESS TABLE

Status legend: **LIVE-WIRED** (running + reachable end-to-end) · **BUILT-UNCONSUMED** (code exists, no driver/route/client) · **SPEC-ONLY** (designed, no implementing code) · **ABSENT** (not built).

| Peer-Hoster Capability | Status | Evidence (file:line / doc) | Gap |
|---|---|---|---|
| **Node-side durable, replayable Automerge op-log store** | LIVE-WIRED | `sync/doc_store.rs:126-128` (`doc.save()` embeds full history); persisted `/data/sync.sled` on PVC (`main.rs:1927-1936`) | None — clean win. But nothing writes into it in production. |
| **Node↔node CRDT sync loop** | LIVE-WIRED | 60s `sync_interval.tick()` → `initiate_sync_round` (`p2p/mod.rs:2375-2382,6808`); serve `handle_sync_request`/`get_changes_since` (6129+), apply `apply_changes` (6238,6392) | Runs but exchanges empty doc lists — no doc ever seeded. Delta exchange needs both nodes connected (`peers.is_empty()` early-return 6813). |
| **Spoke(browser)→node CRDT sync (client + route)** | BUILT-UNCONSUMED | HTTP `/sync/v1/...` real (`http.rs:980-981,3630`, `with_sync_manager` main.rs:2426); TS `AutomergeSync`/`createSync` (`sdk/.../sync.ts`); but zero app callers, doorway excludes `/sync` (`http.rs:1652-1653`), absent from `is_service_path` + proxy contexts | **No frontend Automerge client; doorway deliberately drops `/sync`.** This is the #1 blocker. |
| **Blob persists on disk + survives restart + serves back via `/blob`** | LIVE-WIRED | `blob_store.rs:179-219` (content-addressed fs write/read); `STORAGE_DIR=/data` real `volumeClaimTemplate` 5Gi RWO (`matthew-manager.yaml:335`); `/blob/` proxied (`is_service_path`, server/http.rs:1573) | None — the one durable, doorway-reachable win. Single on-node copy only. |
| **Single-shard push to one peer (`/db/content` write)** | LIVE-WIRED (degenerate) | `distribute_shards` (`p2p/mod.rs:1469`) → `push_shard` (1435/2992); receiver `ShardService::handle_push`→`blob_store.store` (shard_service.rs:88-96); fires from `/db/content` (http.rs:4288+) | Single-copy, authoring-path only (not `/blob` upload), often **zero** copies on thin alpha mesh (writes `placement_gaps`). |
| **RS(N,K) erasure coding of everyday content** | ABSENT | `determine_encoding` returns `"none"` for ≤16MB, `"chunked"` ≤64MB, only `"rs-4-7"` >64MB (`sharding.rs:125-133`) | No redundancy for the content people actually store. |
| **RS encode for >64MB content** | BUILT-UNCONSUMED | encoder real (`sharding.rs:174,207,301`); `reconstruct` reader has ZERO non-test callers (all refs `#[cfg(test)]`) | Read/reassemble path unconsumed even for large content. |
| **Shard manifest in DHT (cross-node reconstruction recipe)** | ABSENT | manifests written only to local sqlite (`db/shard_manifests.rs:5`: "per-peer local state (Category C), not DHT-notarized"); `sharding.rs:9` "designed to" = comment | Node loss = recipe loss = scattered shards un-reassemblable. |
| **Re-quilt / re-replicate on shard loss** | SPEC-ONLY | detection live (`run_custody_reconcile` p2p/mod.rs:2031; `verify_shard_locations` marks `"lost"`); repair emits `placement-gap` REA event only — no consumer; salvage is "future" (`2026-05-02-blob-custody-reconciliation-design.md:91`); tiered-quilt `status: Draft` | Redundancy bleeds away monotonically; no repair actor. |
| **Node-resident SSR (`/spa/*`, `/render`)** | BUILT-UNCONSUMED | route arms exist (`http.rs:1455-1530`); `with_ssr_state` defined (`http.rs:606`) but **never called** (grep-confirmed) → `ssr_state=None` → 503; `ssr` feature `sed`-stripped from Dockerfile (`Dockerfile:99-110`); `LocalFetcher` = 404 stub (`ssr.rs:81-92`) | Dead on every axis; V8 floor (1Gi/1000m) most peer HW can't sustain. |
| **peerId-addressed (libp2p-routed) render** | ABSENT | `ssr.rs:5` "libp2p-routed" comment aspirational; `/spa/*` on HTTP listener; grep of `p2p/`+`p2p_iroh/` for render-forwarding = empty | No render-over-peerId protocol exists. |
| **mDNS LAN discovery (libp2p plane)** | BUILT-UNCONSUMED | handler `Mdns::Discovered` adds to Kademlia + `delivery_peers` (`p2p/mod.rs:4538-4583`) but **never dials**; only bootstrap dials (2226,2477); `DISABLE_MDNS:"true"` in every node (matthew-manager.yaml:265) | Disabled in prod AND dial-less — discovery never becomes reach. |
| **peerId / household-node identity binding** | ABSENT | `DeliveryPeer.household_id = None` at discovery (`p2p/mod.rs:4578-4580`); no "reach my household relay" addressing | Can't say "the peerId that is my household node." |
| **DNS-free reach: stable announce addr + externally-exposed p2p** | ABSENT | default listen `/ip4/0.0.0.0/tcp/0`, no `announce_addresses` in any manifest (mod.rs:435); `listen_addresses` from `swarm.listeners()` (7149) = cluster-internal; p2p svc headless/ClusterIP, no LB/NodePort/Ingress | External spoke literally cannot dial tcp/9876; returned addrs non-routable. |
| **Bootstrap/signal WITHOUT a public doorway** | ABSENT | spoke libp2p bootstrap from `GET {doorway}/api/v1/federation/p2p-peers` (federation.rs:295-321); conductor bootstrap/signal from doorway `native-handoff` (doorway.rs:43-58); doorway URL is public DNS | No household-local bootstrap/signal facility. |
| **DHT-first write-through for content/mastery** | BUILT-UNCONSUMED | content/mastery/relationships diesel-primary (`http.rs:4240,~6233,4971`); `WriteThroughState` exists but gates content Off; `is_integrity_kind` notarizes only 4 key/agent kinds (write_through.rs:209-212) | App truth lives in SQLite, not DHT, for the writes that matter. |
| **Signed-EPR emit path (`/api/v1/signal/emit`)** | BUILT-UNCONSUMED | `signing_client: None` default (http.rs:406); `with_signing_client` (487) **never called** in main.rs → 503 | The real browser→conductor-signed→DHT bridge is dark. **Cheap to wire.** |
| **iroh DNS-free alternative (local discovery)** | BUILT (n0-only) / local ABSENT | iroh stack built+parity-tested; defaults `use_n0_discovery:true` (config.rs:92); `add_node_addr` "what tests do" (72); no LocalSwarmDiscovery/mDNS (README:314 "Phases 4+"); prod backend `Libp2p` (main.rs:392) | Discovery leans on external n0 infra, not household-local. |

---

## 3. THE DELTA: today's dataplane → the peer-hoster vision

**Today's dataplane is a centralized REST app:** `Angular service → HTTP (camelCase) → doorway (single-target proxy) → elohim-storage → diesel → SQLite`. SQLite is de-facto source of truth for content/mastery/relationships; exactly one write class (economic events via `/api/v1/lamad/events`) is conductor-first/DHT-truth. Cross-person reach works on alpha only because the doorway funnels everyone to **MATTHEW's single node** — A reaches B because both browsers hit the same node behind the same doorway, not because data propagated between independent peers.

The concrete missing pieces between that and the peer-hoster vision:

1. **A CRDT writer in the app + a route to carry it.** The entire Automerge plane (TS `AutomergeSync` client, HTTP `/sync/v1`, node↔node loop, durable sled) is built but moves zero app data: no frontend client, and the doorway deliberately drops `/sync`. *Without a writer, the durable store stays empty and the A→node→B loop never starts.* **(The decisive delta.)**

2. **DHT-first (or signed-EPR) write-through for content/mastery.** For writes to be peer-native rather than SQLite-primary, they must notarize first. The write-through subsystem exists but gates content Off; the signed-EPR emit path is built but `signing_client` is never constructed (returns 503).

3. **A DNS-free path for a spoke to find and reach its household node.** Requires: mDNS that *auto-dials* (today dial-less), a household-node *identity binding* (none — `household_id=None` at discovery), *stable announce addresses* + an *externally-exposed p2p endpoint* (today `0.0.0.0/tcp/0`, cluster-internal only), and a *household-local bootstrap/signal* facility (today both planes rendezvous through a public DNS doorway).

4. **Record-level propagation between INDEPENDENT peers.** Today's "cross-person" reach is single-node centralization. The vision's "A on hoster-1 → B on hoster-2" needs the self-healing P2P dataplane to actually carry app *records* (not just blobs/heads — inventory gossip is metadata-only) between distinct nodes.

5. **An always-on PEER-hoster ≠ today's doorway.** The always-available target today is a web2 doorway (a CDN-edge projection in front of one node), explicitly "views served THROUGH a doorway, never owned BY one." The peer-hoster (an always-on *peer* holding A's data and serving B over P2P) is SPEC-ONLY (resilience patron-CDN / household-hub specs).

6. **Cross-node redundancy + repair so node-loss ≠ data-loss.** Everyday content is `encoding="none"` (single copy), the reconstruction manifest lives only in the origin's sqlite (not the DHT), and loss is detected-but-not-repaired. The node is currently a single point of failure, not a redundant fabric.

7. **Peer-native SSR + a peerId render transport** (if SSR is in scope). Node-resident render is BUILT-UNCONSUMED on every axis; peerId-addressed render is ABSENT; even the live LAN/direct transport carries *data only* (the app shell is doorway/Tauri-supplied, not node-served).

---

## 4. PRIORITIZED GAP WORK-LIST (leverage-ordered)

Tags: **[code-now]** buildable on household-nodes today · **[needs-substrate]** needs a capability not currently available · **[design-first]** needs design before code.

1. **Wire `with_signing_client` in `main.rs`** — **[code-now]** · *CHEAP HIGH-LEVERAGE FINISH.* A few lines (plus conductor bridge) flips `/api/v1/signal/emit` from 503 to a live browser→conductor-signed→DHT write path. The single cheapest unblock; `signing_client` is already fully built and only the constructor call is missing. (`http.rs:487`, `main.rs`)

2. **Ship a frontend Automerge client + carry `/sync` to the node** — **[code-now] for the client, [design-first] for the route.** *HIGHEST-LEVERAGE, but NOT cheap.* The server substrate is DONE (serve/apply/durable-sled/node↔node-loop all LIVE) — the missing half is the browser client (real work; `AutomergeSync` exists in the SDK but has no app consumer) and reversing the *deliberate* doorway `/sync` exclusion (a policy reversal + the is_service_path/manifest gates, the `/auth/portal` shadow shape). Closing this makes the A→node→B async loop work centrally via the doorway — the largest single step toward the user-visible outcome.

3. **Route content/mastery writes through DHT-first write-through** — **[code-now] / [design-first].** The `WriteThroughState` subsystem exists but gates app content Off; turning it on (and deciding the notarization contract for content vs the current 4 key/agent kinds) moves app truth from SQLite-primary toward DHT-truth-with-projection. (`write_through.rs:209-212`)

4. **mDNS auto-dial + household-node identity binding** — **[code-now] for dial, [design-first] for binding.** Make `Mdns::Discovered` *dial* the discovered peer (today it only updates the routing table, so LAN discovery never becomes a gossip-mesh/fetch path), and add a binding that marks a discovered peerId as "my household relay" (`household_id` is `None` at discovery). Re-enable mDNS off the cluster (`DISABLE_MDNS`). First real step to DNS-free household reach. (`p2p/mod.rs:4538-4583,2226`)

5. **Stable `announce_addresses` + externally-exposed p2p endpoint** — **[needs-substrate].** A node must advertise a dialable LAN/IP+peerId address (today `0.0.0.0/tcp/0`, unannounced) and the p2p service must be reachable from outside the cluster (today headless/ClusterIP, no LB/NodePort/Ingress) for any non-co-located spoke to reach it without DNS. Cluster-ops-owned exposure + manifest changes.

6. **Household-local bootstrap/signal facility** — **[design-first] → [needs-substrate].** So neither plane rendezvous through a public DNS doorway. Either a household-resident libp2p/kitsune bootstrap, or the iroh local resolver below.

7. **Shard manifest → DHT + repair-on-loss actor** — **[design-first].** Publish the reconstruction recipe to the DHT (today local-sqlite only, the fact that makes node-loss = data-loss) and add a consumer for `placement-gap` events that recruits a replacement holder (tiered-quilt re-quilt is Draft). Required before the node is a trustworthy durable target. (`db/shard_manifests.rs:5`, `2026-05-11-tiered-quilt-stewardship-design.md`)

8. **Enable RS(N,K) for everyday content** — **[design-first].** The encoding band gates RS out below 64MB; the `reconstruct` reader has no live caller. Both need a deliberate redundancy contract for small content (the bulk of what's stored). (`sharding.rs:125-133,301`)

9. **iroh `LocalSwarmDiscovery` / manifest resolver** — **[code-now] but scoped.** Replace n0-only discovery (`use_n0_discovery:true`) with a household-local resolver for a genuinely DNS-free, external-infra-free reach path. The transport-selection seam (`select_transport`) is already live. (`p2p_iroh/config.rs:68,92`)

10. **Node-resident SSR wiring** — **[code-now] to wire, [needs-substrate] to be useful.** `with_ssr_state` is never called, the `ssr` feature is `sed`-stripped from the Dockerfile, and `LocalFetcher` is a 404 stub; wiring is small but the V8 resource floor (1Gi/1000m) is beyond most peer hardware, and a peerId render transport is ABSENT. Lowest priority for the async-sync core. (`http.rs:606`, `Dockerfile:99-110`, `ssr.rs:81-92`)

**Built-unconsumed cheap-finish flags:** #1 (signing client — genuinely cheap) and the *server half* of #2 (CRDT serve/loop/store all done — only the client+route remain). The steward/node Automerge engine and `crates/elohim-sdk` sync are also built-unconsumed but are NOT cheap finishes: steward/node is off-cluster with broken ingest (`DocRequest` never sent, `Announce` unhandled, save paths test-only), and the sdk `sync` feature is uncompilable (`automerge_sync.rs` is absent) — do not treat either as a near-term lever.

---

## 5. WHERE THE BRIEFS DISAGREED / LEFT UNCERTAINTY

These were resolved in-flight during adversarial verification — surfacing them because each corrected an over-optimistic first read:

1. **"LAN serves CSR + data today" (SSR brief) → CORRECTED to "data only."** The storage socket binds `0.0.0.0:{http_port}`, but there is **no `GET /` route arm** — bare root 404s (`http.rs:1536-1539`). The node serves the DATA API and (for packaged HTML5 bundles) an `index.html` SPA-fallback under `/apps/*` only. The **Elohim Angular app shell is doorway-served (browser) or Tauri-bundled (desktop), never node-served at root.** So even the non-SSR LAN baseline is narrower than first claimed.

2. **`/api/v1/federation/p2p-peers` content (discovery brief) → CORRECTED.** It does **not** hand back the doorway storage's `connectedPeers` list. `project_p2p_peers` (`federation.rs:295-321`) builds exactly ONE row: the doorway storage node's OWN peerId + listenAddresses; `connectedPeers` is read only as an integer count. Net: a spoke is handed the *doorway's own node identity* to dial — which makes the "rendezvous through public DNS doorway" conclusion **stronger**, not weaker.

3. **Spoke libp2p bootstrap path "LIVE-WIRED" (discovery brief) → DOWNGRADED to reachability-broken-from-outside.** The data flow runs, but the returned `listen_addresses` come from `swarm.listeners()` over `0.0.0.0/tcp/0` with no announce addresses, and the p2p service is cluster-internal (headless/ClusterIP, no LB/NodePort/Ingress). An external Tauri spoke cannot dial tcp/9876 on a k8s storage pod, and the address handed back is non-routable. Reach does not complete end-to-end from outside the cluster.

4. **Citation correction (durability brief, label unaffected):** evidence that shard manifests are not DHT-durable should cite `db/shard_manifests.rs:5` ("per-peer local state (Category C), not DHT-notarized"), not `services/bootstrap_manifests.rs:8` (which concerns a *different* policy-manifests table). Conclusion (manifest local-only) stands.

5. **Three distinct CRDT stacks, only one deployed (sync brief).** Worth flagging for any reader who greps "automerge" and finds multiple homes: (a) **elohim-storage** stack — deployed, durable, node↔node loop runs, but unfed; (b) **steward/node** engine — durable serve-half but ingest never wired AND not on the cluster (the deployed `elohim-node` is the storage crate, a naming collision); (c) **crates/elohim-sdk** sync — uncompilable (`automerge_sync.rs` absent), default-off, no consumer. Only (a) is a near-term lever.

6. **Residual uncertainty:** "MATTHEW-only" single-node routing is a *deployment-topology fact* (memory-pinned alpha-substrate-probe), not a hardcoded code constant — the doorway is single-target by its manifest-forward model. Cited as topology, not file:line. The centralization conclusion does not depend on a specific routing constant.
