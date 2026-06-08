---
id: "backlog-single-node-content-provenance-visibility"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Single-node/Tauri: API-created content 404s on read-back (no peers to drain provenance) — a peerless laptop must read its own content"
slug: "single-node-content-provenance-visibility"
written: "2026-06-08"
author: "cartographer"
status: "refined"
priority: "medium"
relatedNodeIds:
  - "backlog-seed-provenance-anchor-gap"
  - "memory:project_local_stack_dht_anchor_gap"
  - "memory:project_hub_optional_floor"
tags: [single-node, tauri, provenance, dht-anchor, require-provenance, hub-optional-floor, p2p-drain, product-gap, code-domain]
shift_objective: |
  On a single-node / Tauri node with no peers, API-created content is written diesel-only
  (POST /db/content, http.rs:3416 TODO(p2p-coherence)) with NULL provenance, and the
  publish drain (p2p/mod.rs:3156-3162) bails at 0 connected peers — so it never stamps
  p2p_published_at. The require_provenance read gate (content_diesel.rs:161-167/279-285)
  then 404s the creator's own content on read-back: a write-then-read-back failure on a
  peerless node. This violates the hub-optional floor (project_hub_optional_floor: a
  laptop with no hub is a full participant — its own content must be readable). Fix,
  flag-gated so prod still requires real DHT publish: (option 2) synchronous mark_published
  on create when the node is single-node/dev — content_service.create stamps
  p2p_published_at locally at write time; OR (option 3) the p2p drain marks-without-gossip
  in single-node mode — it stamps p2p_published_at without requiring connected peers,
  preserving the drain path but dropping the peer precondition. Either is gated by a
  single-node/dev flag so a peered prod node still gates visibility on real DHT publish.
  Done when a peerless node can write content via the API and immediately read it back,
  with the same write on a prod-mode peerless node still correctly 404ing (flag respected).
---

# Single-node/Tauri write-then-read-back 404 — the hub-optional floor demands self-visibility

## Why this is more central than "a Tauri edge case" (sharpened 2026-06-08)

This is **the own-content-read decoupling**, and it lands on the most-stable layer. The same fix
**clears the genesis "Read own content" household red** — that scenario fails on the live household
triad (not blocked: `household-nodes` is `available`, mesh peerCount 2), because own-content read is
coupled to a DHT-publish round-trip. Reading content you just wrote to YOUR node should resolve at the
local-write layer, never wait on multi-peer gossip. So this isn't a 1-node corner — it's the principle
"go deep on the most stable architecture; minimize what needs greater complication to prove" applied to
content visibility: own-reads prove at the floor (1 node, local write), peer-publish/discovery is the
only thing that should need ≥2 peers. Fixing it removes peer-dependence from a whole class of the test
surface AND upholds the hub-optional floor. Consider this **high-leverage**, not merely medium — it
discharges a design-floor obligation and clears a standing household red with one change.

## The product gap

This is the **same provenance gate** as `seed-provenance-anchor-gap.md`, surfaced from a different
angle: not CI peer-starvation, but the **designed steady state of a single-node deployment**.

On a single-node / Tauri node (one device, no peers — the explicit design floor):

1. `POST /db/content` writes **diesel-only** with NULL `dht_anchor_hash` / `p2p_published_at`
   (`http.rs:3416 TODO(p2p-coherence)`).
2. The publish drain (`p2p/mod.rs:3156-3162`, `DRAIN_INTERVAL_SECS=15`) — the only thing that stamps
   `p2p_published_at` — **bails with 0 connected peers**. A peerless node has 0 peers permanently.
3. The `require_provenance:true` read gate (`content_diesel.rs:161-167`/`279-285`) 404s any content
   with both provenance fields NULL.

Net: **a peerless node cannot read back the content it just wrote via the API.** This is the identical
write-then-read-back 404 that `seed-provenance-anchor-gap.md` investigation 2 found for e2e-created
content on peer-starved alpha — but here it is **not** an environmental degradation, it is the
*intended* topology of a Tauri/laptop node.

## Why this is its own capture (design floor, not a CI red)

It connects directly to the **hub-optional floor** (`project_hub_optional_floor`): the design floor is
*one device, no hub, full participant — hubs are convenience, never gate participation*. A laptop with
no hub that cannot read its own content is a floor violation, not a CI flake. The seed-provenance
master fixes the *bulk-seed and CI-stack* populations; this item owns the **product** guarantee that a
peerless node's own writes are immediately visible to its own reads. They share the gate and one
candidate fix (option 3 below), but the *requirement* is distinct and worth tracking on its own.

## Fix options (both flag-gated — prod still requires real DHT publish)

The non-negotiable invariant: **a peered production node must still gate visibility on real DHT
publish** (storage-as-projection, refusing un-notarized content is correct there). So any fix is gated
by a single-node/dev flag.

- **Option 2 — synchronous `mark_published` on create (single-node mode).** At
  `content_service.create`, when the node is in single-node/dev mode, stamp `p2p_published_at`
  locally at write time (a self-attested local publish — the node IS the authority for its own DHT in
  a single-node topology). Cleanest semantically: a single-node DHT's "published" is the local write.
- **Option 3 — drain marks-without-gossip in single-node mode.** Keep the drain path; drop the
  *peer precondition* when single-node — the drain stamps `p2p_published_at` without requiring
  connected peers (no gossip target exists, but the local stamp is still valid). This overlaps the
  master's option-2 ("p2p drain stamps without requiring peers in seed/single-node mode") and would
  close both the CI-e2e population and this single-node population with one change.

Settle alongside the master's **p2p-design-gate question**: *should `require_provenance` exempt
creator-scoped reads?* If yes, a creator-scoped read bypass closes this without touching the publish
path at all (the laptop reading its own content is the canonical creator-scoped read). Coordinate the
decision once across both items so they don't diverge.

## Relationship to siblings

- **`seed-provenance-anchor-gap.md`** — the master; same gate, CI/bulk-seed populations. Option 3 here
  is the master's option 2. Cross-linked both ways.
- **`project_local_stack_dht_anchor_gap`** (memory) — the local-stack analog: bulk import never
  anchors, dev repair is a SQLite `p2p_published_at` backfill (which is exactly the manual form of
  what option 2/3 would do automatically and safely under a flag). The action-keyed refinement there
  (project-epr and identity anchoring already work rung-3) means single-node mode could reuse the
  working anchor path rather than inventing one.
- **`project_hub_optional_floor`** (memory) — the design principle this item upholds.

In-tree implementation; **does NOT need a live alpha cluster** to build/test (a single-node test fixture
is the whole point — spin a peerless storage node, POST content, assert read-back). Household-testable.
