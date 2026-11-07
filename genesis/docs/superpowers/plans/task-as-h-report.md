---
id: task-as-h-report
status: claimed-not-verified
cites: []
landed_commit:
verified_by:
concerns:
  - "R2 adds three witness values beyond the brief's three (recon, fetch-verified, self): those writers exist and needed a class; all three bind the row's peer id to the speaker"
  - "R2 the salvage custody floor now joins a provider to its rows through peer_transport_manifest (inventory rows are keyed by transport id, providers by agent_cid); the raw-equality join counted almost nothing for agent_cid providers"
  - "R7 the blob pledge now rests only on the whole payload read back verified; a verified unit earns its own row and announcement, never the blob's pledge"
  - "R3 exists() now hashes the whole payload on every call (HEAD /shard, claim_held_shard, backfill re-claim): correct, and costlier for large payloads"
  - "filename-set custody acts outside R7 remain: services::custody_rotation (main.rs rotation tick) authors custody from BlobStoreSnapshot"
  - "custody-adjacent readers that ignore the witness remain: reconcile_pass receiver-side placement-gap last_seen, distribution_view replica_count, peer_topology_view, services::resilience"
actor: agent:H-A@claude-opus-5-5
---

# Task as-h report

## Worker A (Opus 5.5 — integrity: R1, R2, R3, R7)

Worktree `.worktrees/as-h`, branch `artifact-store/h`. Commits:

- `08981e12` fix(storage): chunked exists hashes every chunk before it says Verified (H R3)
- `ce82e5d7` fix(storage): signed salvage ads, witnessed inventory rows, verified self-placement (H R1 R2 R7)

Function ownership claims: `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/H-owner.md` (main-checkout hub).

### What changed

**R1 — salvage ads are signed, bound, fresh and replay-ordered.** `p2p/salvage_gossip.rs` replaces the Stage-1 placeholder with `SignedSalvageAd`: a `SignatureEnvelope` over the elohim-epr transcript (domain `elohim:salvage-ad:v2:`, epoch 0) whose dag-cbor signed body holds `seq`, `agent_cid`, `archetype`, `author_peer`, `spare_bytes` and `issued_at_ms`. `admit` follows `custody_announce::admit` step for step: oversize refused; no envelope header (an unsigned ad, including the retired Stage-1 shape) refused on both planes; unknown epoch or scheme **Held**; full decode and structural check; signature under the envelope key; `author_peer` must be a spelling of that key (`transport_ids_of_key`); the advertised `agent_cid` must be the agent `peer_transport_manifest` binds to the signer (an unbound signer is refused); a gossipsub `message.source` bound to a different agent refuses. Freshness: `issued_at_ms` no older than 1 h and no more than 5 min ahead. Replay: `db::salvage_capacity::apply_capacity` now writes only a strictly higher per-agent `seq` (one write transaction) and returns `Applied | Replay`. `gossip_dispatch::handle_salvage` binds admission to the DB, retains held ads in the bounded `held_envelopes`, and stamps `last_seen_at` with the signed issue time (never later than now). Both planes enter through `handle_gossip`, so the iroh receive path is the same code with `AuthorWitness::EndToEnd`. Publisher (`P2PNode::broadcast_salvage_capacity`): signs with the custody signer, advertises the resolved self `agent_cid` (active session, else the boot agent key) instead of the transport `Config::self_cid` (which the binding would refuse), and `salvage_seq` is seeded with the boot epoch like `inventory_seq` so a restart stays ahead.

**R2 — inventory rows record their witness.** Migration `2026-11-07-120000_peer_blob_inventory_witness` adds `witness TEXT NOT NULL DEFAULT 'legacy-hint'` (existing rows become `legacy-hint`: fail closed), with a `-- Source of truth:` comment; `diesel_schema.rs` and `PeerBlobInventoryRow` carry the column. `db::peer_blob_inventory::InventoryWitness`: `signed-author` (libp2p, signed `message.source` = author), `end-to-end` (iroh, or an admitted page flushed later from the reorder window), `recon` (negentropy session over the peer's own connection), `fetch-verified`, `self`, `legacy-hint`. Every writer in the module records its witness; `NewPeerBlobInventoryRow` (used by fixtures) carries none and so writes `legacy-hint`. `holds_bound_fresh` is the custody-floor read. `reconcile/custody.rs::salvage_pass` counts a provider honored only on a bound, unexpired row keyed by the provider or by a transport id the manifest binds to it. `lookup_hosts` is unchanged: legacy rows stay fetch hints.

**R3 — chunked `exists` hashes.** `BlobStore::exists_locked` hashes every chunk against its CID and the whole (BLAKE3 streamed, one chunk in memory) against the payload CID before `Verified`; a chunk failing its CID is quarantined with the payload. `store`'s idempotence branch therefore only skips a write over verified bytes; otherwise `put_chunk` and `write_layout` rewrite it.

**R7 — selected self-placement verifies before it records.** `distribute_shards` no longer builds a `BlobStoreSnapshot` (`list_cids` filenames). The self-selected arm and the uploader arm record a self-held row and announce only after `verified_unit_read` (`BlobStore::get_chunk`, which hashes to the unit CID); the custody pledge requires `self_full_copy_verified` (the whole payload read back through `BlobStore::get`). The "verify-before-record gate" comment is replaced; `BlobStoreSnapshot` documents that it is a filename set that must never gate a custody act.

**RS-GOV-04:** `reconcile_pass`'s `too_many_arguments` allow now has a reason; `just gate rust-style` PASS, elohim-storage debt 112 → 111.

### Tests (by name, result)

Focused runs on this device (lib binary filters `salvage`, `peer_blob_inventory`, `reconcile::custody`, `blob_store::`, `gossip_dispatch`: 33, 7, 36, 21, 16 passed, all EXIT=0; `--test blob distribute_shards_diversity inventory_writer_smoke`: 10 passed, 1 ignored as before, EXIT=0):

| Item | Test | Result |
|---|---|---|
| R1 | `unsigned_salvage_ad_is_refused_and_projects_nothing` | ok |
| R1 | `salvage_ad_from_unbound_source_is_refused` | ok |
| R1 | `replayed_salvage_ad_does_not_reproject` | ok |
| R1 | `forged_iroh_salvage_ad_naming_another_holder_is_refused` | ok |
| R1 | `unknown_scheme_salvage_ad_is_held_not_projected` | ok |
| R2 | `custody_count_ignores_unbound_legacy_rows` | ok |
| R2 | `bound_fresh_row_counts_and_expired_one_does_not` | ok |
| R2 | `salvage_already_replicated_no_author` (updated: hosting = bound rows) | ok |
| R3 | `same_length_corrupt_chunk_is_never_reported_verified` | ok |
| R3 | `store_over_a_structurally_present_corrupt_object_repairs_it` | ok |
| R7 | `selected_path_refuses_corrupt_payload` | ok |
| R7 | `selected_path_refuses_missing_chunk_payload` | ok |

Supporting tests added: `salvage_gossip::tests::*` (roundtrip/admit, every signed field covered, Stage-1 shape refused, unbound and mismatched agent, freshness both ends, held before any other check, oversize), `salvage_capacity::tests::an_older_or_equal_seq_never_reprojects`, `peer_blob_inventory::tests::every_writer_records_its_witness_and_an_unwitnessed_insert_is_a_hint`. Changed: `blob_store::tests::corrupt_chunk_is_quarantined_not_served` asserted the R3 bug (`exists == Verified` over a same-length corrupt chunk) and now asserts `Quarantined`.

### Gate

`just gate elohim-storage` (migration-hygiene, image-includes, fmt-check, clippy, clippy-iroh, test): log `genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/gate-H-A-elohim-storage.log`, run at HEAD `ce82e5d7`, 2026-11-07T20:39:27Z → 22:04:38Z (85 min), **`EXIT=0`**: 19 test binaries, 5,900 passed, 0 failed; evidence recorded `bafyreicran2sdzt…`.

### Deviations from the brief, and why

1. **Witness vocabulary (R2).** The brief names three values; the crate has six writers. `recon`, `fetch-verified` and `self` were added rather than folding them into `legacy-hint` (which would have uncounted the main convergence path) or into `signed-author` (which would misstate the witness). All three key the row to a peer id authenticated by the act itself.
2. **Floor join (R2).** Counting "bound" rows by raw `peer_id == provider` would keep counting nothing for `agent_cid` providers, because inventory rows are keyed by the author's transport id. The floor now reads the provider's rows under every transport id `peer_transport_manifest` binds to it (no raw cross-namespace match).
3. **Freshness (R1).** Beyond per-peer `seq`, the signed body carries `issued_at_ms`, bounded at admission and used as `last_seen_at`, so a captured ad replayed to a receiver that never held its agent cannot enter the pool as fresh.
4. **Publisher identity (R1).** The ad advertised `Config::self_cid`, a transport id; it now advertises the resolved self `agent_cid`, without which every honest ad would be refused by the binding.
5. **R7 scope.** The uploader (not-selected) arm used the same filename snapshot and was moved onto the same verified unit read; the pledge condition changed from `self_placement_recorded || self_full_copy_verified` to `self_full_copy_verified`. A verified chunk of an unstriped payload is now recorded as its own unit (before, only an inline payload's single unit ever matched the filename set).
6. **Files outside the ownership list,** all recorded in `H-owner.md`: `db/salvage_capacity.rs` (R1 replay), `p2p/mod.rs` `broadcast_salvage_capacity` + `salvage_seq` init (R1), and mechanical witness arguments in test call sites (`services/spool_custody_author.rs`, `services/salvage_commitment_author.rs`, `p2p/inventory_broadcaster.rs`, `tests/blob/inventory_writer_smoke.rs`). One assertion in `blob_store::tests` (Worker B's module) was flipped because it pinned the R3 defect; the two R3 tests are appended at the module end.

### What the audit got wrong or left out

- R1 freshness by per-peer `seq` alone would have refused every honest ad after a restart: `salvage_seq` started at 0 each boot. Seeded with the boot epoch here.
- The salvage ad's identity was a transport id (`self_cid`), not an `agent_cid`, contrary to the module's own claim; binding exposed it.
- R2's "count only bound rows" at `custody.rs:491-508` was joining `agent_cid` providers to transport-id-keyed rows by raw equality, so the floor was near zero for agent providers before any provenance question.
- R7's `list_cids` snapshot could never match a chunked payload's units (chunks are not under `blobs/`), so the filename gate failed open only for inline payloads; but it also fed the pledge through `self_placement_recorded`.
- Not in the audit: `services::custody_rotation` authors custody commitments from the same filename snapshot (`main.rs` rotation tick), and several custody-reading folds ignore the new witness (see `concerns`).
