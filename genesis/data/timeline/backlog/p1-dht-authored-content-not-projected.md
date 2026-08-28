---
id: "backlog-p1-dht-authored-content-not-projected"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A content node authored on the DHT by a peer with no storage sidecar is never served by the fleet — storage discovers ids from storage-to-storage inventory, never from the DHT manifest (P1 gap, measured)"
slug: "p1-dht-authored-content-not-projected"
written: "2026-08-28"
author: "shift 2026-08-28T03-25-shakeout-landing-perf-trust-hybrid"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
---

## Measured (sovereign-peer spike, 2026-08-28T03:27–03:35Z)

A workspace conductor joined alpha's DHT as a NEW agent (`hc-start.sh --conductor`, join-alpha
profile, deployed bundle: all 5 cell DNA hashes ∈ doorway-alpha's diagnostics spaces; the agent was
listed live by `/db/p2p/conductor-diagnostics` within ~4 min). It then authored
`spike-sovereign-peer-1787887906169` via `content_store.create_content` (48 ms, action
`uhCkkb97Vh6z…`), and `get_content_by_id` read it back locally. doorway-alpha
`/db/content/<id>` answered **404 for 4 minutes** and would have kept doing so:

- `elohim-storage` projects content from (a) its OWN conductor's post-commit signals and
  (b) ids advertised by OTHER STORAGES over inventory gossip (libp2p / iroh manifest board).
  `services/conductor_writes.rs::get_content_by_id` (a DHT read by id) is called only by the
  re-anchor backfill (`content_service.rs:566`), never on an HTTP read miss.
- So an agent that participates only at the DHT layer — the hub-optional floor the seam map
  promises — writes entries the fleet's conductors will hold but no web2 surface will serve.
  P1 ("the DHT is the manifest; storage is a controller that eagerly reconciles") is not yet true
  for ids that no storage has ever advertised.

## Why it matters

This is the sovereign-peer / hybrid-dev mode's blocking gap: a developer's own conductor can join
and author, but nothing they author is visible to anyone through a doorway until a storage
adopts it. It is also the same shape a future device peer (Tauri, no sidecar) would hit.

## Candidate cures (design decision — not a shift edit)

1. Storage discovers new ids from the DHT: a bounded periodic walk of the `content_store` id
   index links (the zome already keeps `IdToContent` / all-content anchors) on the storage's own
   conductor view → candidate adopt through the existing reach-authorized adoption path.
   Cost: one link scan per sweep; bounded by the index's size, not the corpus.
2. Doorway read-miss fallback: `/db/content/{id}` 404 → one `get_content_by_id` zome call on the
   pool conductor → serve + let storage project (`project_existing_anchor` already exists for
   the anchor path). Cheaper to ship, but a serve-path DHT read on every 404 is an amplification
   vector without a reach check first.
3. The a2o story carrying the promise: `features/deployment/sovereign-peer-join.feature`
   scenario 3 "(RED — the gap)"; the spike script is `genesis/seeder/scripts/spikes/sovereign-peer-author.ts`.

## Correction (2026-08-28, M0 shift — the spike's conductor was never connected)

The 2026-08-28T03:27Z spike ran the **stock holochain 0.6.0 (tx5)** conductor. Alpha's conductors run
the ethosengine fork on the **iroh** transport (agent URLs `https://relay.alpha.elohim.host/…`,
`relay_url` in conductor-config). A tx5 conductor publishes itself to alpha's bootstrap and IS listed by
doorway-alpha's `/db/p2p/conductor-diagnostics` — and then holds `dumpNetworkStats.connections: []`
for ever (measured 35 min): tx5 and iroh never dial each other. So "listed live within ~4 min" was
true and "joined" was not; the authored node could not have been gossiped to any fleet conductor,
which means the 404 measured above is over-determined — the storage-discovery gap is still real
(storage never DHT-walks ids), but the spike could not have exercised it. Re-measured on the fork
pair (`/projects/.cargo-target-pool/family/dev/crates/dev/release`, holochain 0.6.3, iroh): 4–5
connections within a minute (one direct), peer store 30 entries, gossip rounds initiated.
`hc-start.sh` now uses the fork pair when present and refuses a stock join-alpha unless
`ALLOW_STOCK_JOIN=1`; `features/deployment/sovereign-peer-join.feature` scenario 1 is wired and
green on the fork. Scenario 3 (this gap) is still RED and should be re-run on the fork before the
cure decision — the read-miss fallback (candidate 2) only helps if the fleet conductors can fetch
the workspace's entry, which now they can.
