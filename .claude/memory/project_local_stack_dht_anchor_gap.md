---
id: project-local-stack-dht-anchor-gap
name: local-stack-dht-anchor-gap
title: Local stack DHT-anchor gap
description: Local bulk seed never DHT-anchors → provenance gate 404s all reads by design; dev repair = p2p_published_at backfill; real fix = import anchor step
metadata: 
  node_type: memory
  type: project
  originSessionId: 52e75242-0c09-464a-9a0d-613667fc14e0
  modified: 2026-07-26T12:55:10.799Z
cites:
  - elohim/elohim-storage/src/db/content_diesel.rs
  - elohim/elohim-storage/src/services/rea_commitment_service.rs
  - elohim/elohim-storage/src/db/rea_commitments.rs
---

On the local dev stack, the seeder's bulk content import writes the storage SQLite but the DHT-anchor step no-ops (post-flight 2026-06-04: 1/600 conductor entries; all rows `dht_anchor_hash=NULL, p2p_published_at=NULL`). External HTTP reads then 404 EVERYTHING via the provenance gate (`db/content_diesel.rs get_content require_provenance=true` filters rows lacking both fields) — storage-as-projection working as designed, refusing unanotarized content.

**Why:** hours were lost chasing "content not found" through doorway/proxy/reach layers when the row was sitting in the DB. The single-entry conductor write path works (seeder pre-flight `write_capability` passes); only the bulk path skips anchoring.

**How to apply:**
- Symptom: `db/stats` shows contentCount>0 but every `GET /db/content/{id}` + `/epr-head/{id}` 404s → check provenance columns first.
- Dev-only unblock: `UPDATE content SET p2p_published_at=<now> WHERE dht_anchor_hash IS NULL AND p2p_published_at IS NULL` on the live DB at `/nix/xdg/data/elohim-storage/content.db` (NEVER commit; it fakes notarization).
- Real fix home: import pipeline anchor step. Related: relationship seeding writes 0 rows to `relationships` (blocks epr-link-navigation fixtures). Also note seeder `--ids a,b,c` flag exists for targeted seeding; `--limit N` takes the first N of ~3455 files (manifesto missed the 200/600 cuts).
- **REFINEMENT (2026-06-04 realism audit): the gap is ACTION-KEYED, not blanket.** `rea_commitment_service.rs:45` routes ONLY `action=="project-epr"` through the conductor (`create_via_conductor` → `upsert_with_anchor(Some(action_hash))` — anchors fine); ALL other actions (custody-blob, operate-doorway, in-kind…) take `create_via_diesel` → `rea_commitments.rs:224` hardcodes `dht_anchor_hash: None`. Module doc defers blanket migration (wire-shape divergence). Meanwhile identity seeding is ALREADY rung-3 real: `seed-conductor-identities.ts`/`seed-agent-bindings.ts` callZome on each persona's OWN conductor (real agent-signed DHT entries). Don't read this memory as "nothing anchors" — the anchoring rung exists and works for project-epr and identity; the gap is the unextended action gate.
- Full shake-out fix chain (seeder path, SSR guards, localhost proxy-origin, lazy automerge, graphql manifest routes, dev /auth/account) is in commits `a91eb4d1d`/`a498026f0`/`4fb288cf1` + sprint result [[concurrent-sessions-shared-worktree]].
- **ALPHA-B FINAL CLASS (2026-07-26, three diagnostic refinements deep): GHOST-WITNESSED.** elohim.host's content row CLAIMS a notarized anchor (`dhtAnchorHash uhCkkl4C9…`) but that anchor outlived the conductor incarnation that authored it (2026-07-24 DNA reinstall re-key wiped conductor state; the SQLite projection persisted on PVC) — the CURRENT conductor holds no chain for the id (`gather_content_chain` → empty → declare refused "no content found"). Ghost rows are invisible to the NULL-anchor witness sweep (non-NULL claim) → blind spot. Cure: ghost-witness classifier (conductor-miss ∩ anchored) + re-author via the stale-anchor heal (projection_reconcile). That fixes declare GATE 1; GATE 2 (B retrieving A's action Record, full-arc local-only) still blocks head adoption — legit paths: gossip arc, or coordinator-only `declare_canonical_head` accepting the serialized Record (hot-swappable). Convergence node: saga ch06 station `A.dhtAnchorHash == B.dhtAnchorHash`.
