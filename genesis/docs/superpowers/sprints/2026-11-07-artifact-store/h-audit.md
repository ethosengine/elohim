---
id: artifact-store-sprint-2026-11-07-h-audit
status: verified
---

# H0 — hardening audit (what regenesis 1 already landed)

Source: a read-only Codex `gpt-5.6-sol` leg (prompt `prompts/h0-audit.md`, receipt `evidence/h0-audit-receipt.md`, static inspection only, no tests run). The orchestrator re-read the four OPEN claims in code on 2026-11-07 and confirmed each:
- `p2p/salvage_gossip.rs:19-24,43-46`: "a single null byte is sufficient at Stage 1".
- `blob_store.rs:874-901`: chunked `exists` checks chunk file length only, then returns `Verified`.
- `blob_store.rs:395-409`: `list_cids` walks `blobs/` only.
- No `wasted` meter in `p2p/blob_fetch.rs`, `blob_swarm.rs` or `metrics.rs`.
- No power-loss-after-marker or deletion-race test in `blob_store.rs`.

## Landed (no work this sprint)
H1a libp2p holder binding · H1b iroh end-to-end admission (shared path) · H1c signed custody announcements on both planes · H1e replay cannot rebase a cursor · H2a inline temp+fsync+rename · H2c shards verified against their own CID · H2e verify-on-read · H2f quarantine without disabling the store · H2g staging/quarantine excluded · tests for crash mid-write, competing writers, ENOSPC, cleanup failure · H3a racer cancel · H3b stall + total budgets · H3c manifest cap before allocation · H4a full `ByteCid` decode on every byte surface · H4b legacy spelling refused on storage and doorway.

## Residual (the H track, in priority order)

| # | Item | Where | Class | Gates |
|---|---|---|---|---|
| R1 | **Salvage ads are unsigned** (H1d): replace the placeholder with a signed envelope (elohim-epr transcript, scheme + key + sig, epoch), bind transport source to agent on libp2p and end-to-end on iroh, freshness + replay admission; an unsigned or unbound ad changes no projection. | `p2p/salvage_gossip.rs`, `p2p/gossip_dispatch.rs:649-671`, the iroh receive path | integrity | custody evidence at every reach (R3) |
| R2 | **Custody floors cannot tell bound from legacy rows** (H1f): persist provenance (`bound: bool` or the admitting witness) on `peer_blob_inventory` rows; `reconcile/custody.rs:491-508` counts only bound, unexpired rows; legacy/raw rows are fetch hints. | `db/diesel_schema.rs:1267-1274` + a new migration, `db/peer_blob_inventory.rs`, `reconcile/custody.rs` | integrity | custody evidence (R3) |
| R3 | **Chunked `exists` says `Verified` on a structural check** (H2d): either hash every chunk (BLAKE3 is cheap) or return `Exists::Present` for structural-only and keep `Verified` for a hashed check; `store`'s idempotence path then re-verifies or re-writes. | `blob_store.rs:593-597, 874-901` | integrity | **E4** (R3.3: "already exists means exists and verifies") |
| R4 | **Fan-out directory creation is not fsynced** (H2b): fsync `blobs/` / `chunks/` after creating a new bucket. | `blob_store.rs:544` | durability | E4 |
| R5 | **Missing durability tests**: power loss after the marker (fault point after marker fsync, reopen, verify); delete versus concurrent store/read race; an unrelated good object stays readable after a quarantine. | `blob_store.rs` tests | durability | E4 |
| R6 | **No wasted-bytes meter** (H3d): count bytes from losing racers, oversized replies, hash failures, cancelled swarm work; a dedicated stall-deadline test. | `p2p/blob_fetch.rs`, `blob_swarm.rs`, `metrics.rs` | observability | F2 receipt figures |
| R7 | **Selected placement records custody from a filename set** (follow-up C2): gate the selected path on a full verified read (or verified custody-unit read) before any row, pledge or announcement; the comment calling the snapshot a "verify-before-record gate" is corrected. | `reconcile/custody.rs:48-66`, `p2p/mod.rs:2519-2549` | integrity | custody evidence |
| R8 | **Custody units beyond `blobs/` are invisible** (follow-up C3): enumerate verified local chunks and RS shards; row every unit during distribution and restart backfill. | `blob_store.rs` (a `list_units` beside `list_cids`), `reconcile/custody.rs:619-664`, `p2p/mod.rs:2298-2325` | custody | custody evidence |
| R9 | **Unchanged custody is re-announced on a second distribution** (follow-up C1): change detection keeps one row and the active pledge and emits no fresh announcement when nothing changed (`services/self_stewardship.rs:84-121` returns `Recorded` on an upsert of an identical row). **Ruling (orchestrator, 2026-11-07):** the follow-up's text governs; the audit's suggestion to add restart republishing is declined. Liveness after restart belongs to inventory freshness and recon, not to re-announcing unchanged custody. | `services/self_stewardship.rs`, `p2p/mod.rs:2525-2547` | custody | none |
| R10 | an iroh-labelled forged-holder inventory test (H1b's shared admission already refuses it) | `p2p_iroh/gossip_receive.rs` tests | test only | none |

R1, R2 and R7 need a Claude-family adversarial review (H1 is integrity class; plan §7.3).
