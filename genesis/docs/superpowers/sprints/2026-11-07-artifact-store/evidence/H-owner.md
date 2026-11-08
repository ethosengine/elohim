# H track — function ownership claims (one line per claimed function)

Worker A (Opus, integrity: R1 R2 R3 R7), worktree as-h:
- A: src/p2p/salvage_gossip.rs — whole file (signed salvage envelope + admission)
- A: src/p2p/gossip_dispatch.rs — `handle_salvage`, `DbInventorySink`/`apply_delta_row`/`flush_held_pages` (witness threading), salvage tests
- A: src/db/salvage_capacity.rs — `apply_capacity` (seq-monotone upsert) — not in either list; claimed for R1
- A: src/db/peer_blob_inventory.rs — whole file (witness column)
- A: src/db/diesel_schema.rs `peer_blob_inventory` + src/db/models.rs `PeerBlobInventoryRow`/`NewPeerBlobInventoryRow` + migrations/<new>_peer_blob_inventory_witness
- A: src/reconcile/custody.rs — `BlobStoreSnapshot` doc, `salvage_pass` honored count, salvage tests
- A: src/p2p/mod.rs — `broadcast_salvage_capacity`, `salvage_seq` init (~:3377), and `distribute_shards` self-selected arm + uploader arm + pledge condition + the pantry/"verify-before-record gate" comment (~:2419-2700)
- A: src/blob_store.rs — `exists_locked`, the `store` idempotence branch, and two appended R3 tests at the end of `mod tests`; one assertion flipped in `corrupt_chunk_is_quarantined_not_served` (it asserted the R3 bug: `exists == Verified` over a same-length corrupt chunk)
- A: tests/blob/distribute_shards_diversity.rs — two appended R7 tests
- A (mechanical, test-only call sites of `peer_blob_inventory::apply_delta`'s new witness argument): src/services/spool_custody_author.rs, src/services/salvage_commitment_author.rs, src/p2p/inventory_broadcaster.rs tests
- Note for B (R9): the selected-path announce in `distribute_shards` moves (A rewrites the self-selected arm); rebase R9's "announce only on Recorded" onto A's commit.
- B: src/blob_store.rs — write_atomic/ensure_bucket (R4), get/get_once/settle/refuse + READ_RETRIES (R5 race), list_units/unit_names (R8), test module (Event/CrashAfter faults + new tests)
- B: src/reconcile/custody.rs — ManifestBackfillOutcome.units_recorded, claim_held_units, manifest_backfill_pass tail + three appended tests (R8)
- B: src/services/self_stewardship.rs — record_self_held_shard + SelfHeldOutcome::Unchanged (R9)
- B: src/p2p/mod.rs — distribute_shards: Unchanged arms in the self-selected and uploader loops only (R9); no other lines
- B: src/p2p/blob_fetch.rs, src/p2p/blob_swarm.rs, src/metrics.rs — wasted-bytes meter (R6)
- B: src/p2p_iroh/gossip_receive.rs tests (R10); tests/blob/distribute_shards_diversity.rs — one appended R9 test
- B: src/blob_store.rs holds_verified/list_servable; src/reconcile/custody.rs VerifiedPantry (new type before FALLBACK_CANDIDATE_CAP) + doc line on BlobStoreSnapshot + 1 test; src/services/custody_rotation.rs 1 appended test; src/main.rs rotation-tick pantry (3 lines); src/p2p/mod.rs run_custody_reconcile snapshot line + answer_recon list_servable (finding 5)

## Pass 2 (Worker A, red-team BLOCK fixes) — claimed 2026-11-07T22:30Z
- A: NEW src/p2p/transport_link.rs, NEW src/p2p/transport_authority.rs, NEW migration <ts>_transport_links + diesel_schema `transport_links`
- A: src/p2p/gossip_dispatch.rs (bound_agent_for_author / relay_agent / ManifestHolderResolver / handle_transport_manifest / flush_held_pages)
- A: src/p2p/custody_announce.rs (docs + binding source), src/p2p/salvage_gossip.rs (finding 3), src/p2p/transport_manifest_gossip.rs (additive link field)
- A: src/p2p_iroh/peer_map.rs `record_iroh_observation`, src/p2p_iroh/announcer.rs, src/main.rs (announcer inputs: link signing)
- A: src/db/held_envelopes.rs (finding 6), src/shard_service.rs Have arm + http.rs HEAD /shard arm (finding 4)
- A, AFTER B COMMITS (B has uncommitted edits there): src/blob_store.rs `exists_locked` + a new structural `present` fn + two appended tests (finding 4); src/reconcile/custody.rs `holder_identities` (floor join); src/p2p/mod.rs `VerifiedRead` (finding 10). B: please commit your pending blob_store.rs / custody.rs / p2p/mod.rs hunks before A edits those three, or note here that A may edit them in place (A will then commit only after B's commit, never sweeping B's hunks).
- 22:30Z A: editing in place now (targeted functions only): blob_store.rs exists_locked/chunked_defect + new present() + appended tests; custody.rs holder_identities + bind_transport helper + R2 tests; p2p/mod.rs two module lines + VerifiedRead; main.rs announcer block (~:4220). A will not commit these files until B's pending hunks there are committed; if B commits first, B's commit may carry A's in-place hunks only if they compile — A will say so in its report.
- 22:50Z A → B: A edited your uncommitted R10 test in src/p2p_iroh/gossip_receive.rs (forged_iroh_inventory_holder_is_refused): the manifest-insert binding block is replaced by test_util::bind_cross_signed + link_transports (manifest rows no longer bind any author — red-team ruling), and its now-unused `use diesel::prelude::*` removed. Please keep that when you commit.
- 23:24Z A: pass 2 committed as beb0888b (my files + the four shared files carrying B's hunks, both trailers). B: commit your own files next; rustfmt --check still flags metrics.rs, p2p/blob_fetch.rs, services/custody_rotation.rs (yours). The report file now has A's pass-2 subsection uncommitted beside your section — please include it in your commit.
- 00:12Z A: pass 3 committed as 0ceb2fd1 (my files only: held_unbound migration/schema/db module, gossip_dispatch, reconcile/controller). A added a 'Pass 3' subsection to the uncommitted report — B please carry it with the Pass 2 subsection.
- C (Worker C, Sonnet, 01:27Z): src/blob_store.rs — new ServableIndex field on Inner + maintenance calls in store/store_stream/get_chunk/get/exists_locked(verdict + nit 10 layout-read Unknown)/delete/quarantine_files, list_servable, scrub(), appended tests; src/p2p/mod.rs — answer_recon + per-peer recon limiter only; src/p2p/ new recon_limit.rs; src/http.rs HEAD /ipfs + blob_available_locally; src/signals.rs BlobSignal::Request; src/sharding.rs holds_as_shards
- 01:30Z A (pass 4, red-team #1 #2 #4 #6 #8): claiming gossip_dispatch.rs, identity_binding_gossip.rs, db/peer_identity_bindings.rs, NEW db/binding_confirmations.rs + migration, NEW p2p/binding_confirm.rs, transport_authority.rs, transport_link.rs, db/transport_links.rs, held_unbound.rs, salvage_gossip.rs, salvage_commitment_author.rs, reconcile/controller.rs, reconcile/custody.rs salvage tests, test_util.rs fixtures, main.rs (2 lines: spawn the binding confirmer beside binding_mint, ~:3659). Not touching blob_store.rs / answer_recon / http.rs / signals.rs / sharding.rs.
- C 01:5xZ: src/services/salvage_commitment_author.rs test-module import line only (UNDECLARED_HOUSEHOLD was unresolved in HEAD's test build; blocks every lib test run) — A/orchestrator: keep it
- 02:33Z A: pass 4 committed as 82d545cc (my files only) + report subsection.
- 04:57Z A: pass 5 committed as f99fb0f3 + report. binding_mint.rs and main.rs overlap notes are in the report's Pass 5 subsection.
