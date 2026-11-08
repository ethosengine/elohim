---
id: task-as-h-report
status: claimed-not-verified
cites: []
landed_commit:
verified_by:
concerns:
  - "R2 adds three witness values beyond the brief's three (recon, fetch-verified, self): those writers exist and needed a class; all three bind the row's peer id to the speaker"
  - "R2 (pass 1) joined the salvage floor through peer_transport_manifest — red-team BLOCK, superseded in pass 2 (beb0888b): every binding and floor read now goes through p2p::transport_authority (cross-signed chain only)"
  - "pass 2 timing: a fresh household's iroh-signed custody/salvage claims bind only after the peer's AgentPeerBinding is minted (binding_mint retries 5 s..600 s after sign_for_agent answers) and its manifest link has arrived (30 s cadence); claims published before that are refused as unbound and custody announcements are not re-sent — the chaos-peer-churn/durability acceptance lanes are the measure"
  - "R7 the blob pledge now rests only on the whole payload read back verified; a verified unit earns its own row and announcement, never the blob's pledge"
  - "R3 exists() now hashes the whole payload on every call (HEAD /shard, claim_held_shard, backfill re-claim): correct, and costlier for large payloads"
  - "filename-set custody acts outside R7 remain: services::custody_rotation (main.rs rotation tick) authors custody from BlobStoreSnapshot (finding 5, Worker B)"
  - "low 7 not taken: flush_held_pages still records a reorder-window page as end-to-end (the stash keeps no arrival plane); low 8/9 filed as follow-ups"
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

### Pass 2 — red-team BLOCK fixes (commit `beb0888b`)

`beb0888b` fix(storage): binding authority only from a cross-signed chain; probes do not hash (H red-team pass 2). It also carries Worker B's then-uncommitted hunks in the four shared files (`blob_store.rs`, `reconcile/custody.rs`, `p2p/mod.rs`, `main.rs`), as the orchestrator directed; both trailers are on it.

**The production binding producer.** `p2p::binding_mint` (`spawn_binding_mint_task`, scheduled in `main.rs` once the swarm identity is loaded and the imagodei bridge answers `sign_for_agent`) commits a cross-signed `AgentPeerBinding` for **agent ↔ libp2p PeerId** (transport half signed with `identity.key`, agent half by the conductor); it reaches peers by DHT arrival (`ReconcileController::on_agent_peer_binding`) and identity-binding gossip, each classifying it with `classify_binding_signature`. Read-only check of the shem household DBs: matthew, jessica and james each hold `cross_signed` rows for all three of their PeerIds (valid to 2026-12-07); their `peer_transport_manifest` rows carry only iroh NodeIds. No binding ever names an iroh NodeId (`transport_id_derives_from` admits libp2p only), so the iroh half is new here: **`p2p::transport_link`** — the libp2p identity key and the iroh key each sign `(iroh_node_id, libp2p_peer_id, issued_at_ms)` over the elohim-epr transcript (domain `elohim:transport-link:v1:`), carried as an additive field on the transport manifest (outside its v1 signing bytes, like `user_agent`), signed once at boot in `main.rs` beside the announcer, verified on receipt and recorded in `transport_links` (migration `2026-11-07-130000_transport_links`). This is the ruling's "acceptable alternative" (the libp2p key attests the iroh id), made two-sided so neither key can claim the other unilaterally; `binding_mint` and the DNA are unchanged.

**Authority.** `p2p::transport_authority::authenticated_agent_for_transport` is the single read: PeerId → active cross-signed binding → agent (two agents for one PeerId → none); iroh NodeId → verified link → PeerId → the same. It backs `gossip_dispatch::bound_agent_for_author` (salvage admission, custody-announcement holder binding, inventory `BoundHolderResolver`), `relay_agent` (the refuse-only relay check) and, through `authenticated_transports_for_agent`, the salvage custody floor's identity join. `peer_transport_manifest` is no longer read by any of them; `peer_map::record_manifest_hint` keeps manifests as dial hints that never replace an agent's recorded iroh NodeId unless the new NodeId is itself authenticated to that agent.

**Mixed version.** A peer whose manifest carries no link (an older build) has its iroh-signed custody announcements, salvage ads and inventory **refused as unbound** — never projected, never counted (`refused_binding`); not held. Its libp2p-signed inventory still binds through its cross-signed PeerId.

**Also fixed.** Finding 3: a salvage ad whose `seq` epoch is later than its own `issued_at` is refused. Finding 4: `exists` quarantines only on a missing part, a wrong length or a hash mismatch (other read errors: not verified, not quarantined); `ShardRequest::Have` and `HEAD /shard` answer from the new structural `BlobStore::present` (no hashing, no side effects); `store`'s idempotence keeps the hashing path. Low 6: `held_envelopes` caps per topic and holds an identical envelope once. Low 10: `VerifiedRead::has` vouches only for the unit it read. Not taken: low 7 (reorder-window pages still recorded `end-to-end`); 8 and 9 left as follow-ups.

**Tests (focused, this device, all EXIT=0).** Default features, lib filters `salvage` 35, `peer_blob_inventory` 7, `blob_store::` 30, `gossip_dispatch` 20, `transport_link` 3, `held_envelopes` 2, `shard_service` 23, `custody_announce` 10, `inventory_gossip` 17, `transport_manifest` 9 passed. In `reconcile::custody` 40 passed and B's own in-progress `striped_upload_rows_cover_every_served_unit` failed on that pre-commit build; B reports it green since. `--features "p2p p2p-iroh" --lib` (dispatch, gossip_receive, announcer, peer_map, transport, salvage): 139 passed; `--test iroh_planes` (inventory fetch parity, peer map): 13 passed; `--test blob`: 11 passed, 1 ignored as before.

| Test | Result |
|---|---|
| `forged_manifest_cannot_bind_a_victim_agent` (drives the real manifest gossip path) | ok |
| `iroh_author_resolves_only_through_the_cross_signed_iroh_half` | ok |
| `forged_salvage_ad_via_manifest_hijack_is_refused` | ok |
| `custody_announcement_via_manifest_hijack_is_refused` | ok |
| `custody_floor_ignores_rows_from_a_hijacked_transport` | ok |
| `a_forged_manifest_does_not_replace_a_recorded_node_id` (iroh feature) | ok |
| `seq_ahead_of_issue_time_is_refused` | ok |
| `transient_read_error_never_quarantines` | ok |
| `have_does_not_hash_the_payload` (shard_service) and `present_is_structural_and_moves_nothing` (blob_store) | ok |
| `a_flood_on_one_topic_never_evicts_another_and_duplicates_are_held_once` | ok |
| honest paths moved onto cross-signed fixtures: every custody / salvage / inventory dispatch test, `forged_iroh_inventory_holder_is_refused` (B's R10 test), `iroh_and_libp2p_dispatch_queue_identical_inventory_work`, `bound_fresh_row_counts_and_expired_one_does_not` | ok |

**Gate.** Not run by Worker A in pass 2, by instruction: the orchestrator runs one combined `just gate elohim-storage` + rust-style for the branch after Worker B commits. `cargo fmt --check` was clean on every file in `beb0888b`; the remaining diffs (`metrics.rs`, `p2p/blob_fetch.rs`, `services/custody_rotation.rs`) are in Worker B's own files.

### Pass 3 — unbound claims held, not lost (commit `0ceb2fd1`)

`0ceb2fd1` fix(storage): hold well-signed unbound claims and replay them when the author binds (H pass 3). This replaces the pass-2 "refused as unbound" rule for mixed versions and fresh households: a custody announcement, salvage ad or inventory delta whose signature verified under its author's own key, but whose author has no cross-signed chain yet, is **held** in `held_unbound_claims` (migration `2026-11-07-140000`). Held claims are capped at 256 per topic, stored once per distinct bytes, expire after 15 min, and are never projected or counted. They are **replayed through full admission** (original plane and gossipsub source) when a transport link is recorded, a cross-signed binding arrives by gossip, or one is projected from the DHT. A claim that still fails on replay is dropped with its reason. Forged or badly signed claims stay refused and are never held. Tests (all ok, focused runs EXIT=0; default lib 179 passed, iroh lib 28, iroh_planes 2): `an_unbound_custody_announcement_is_held_and_admitted_when_its_link_arrives`, `a_held_claim_that_still_fails_on_replay_is_dropped`, `held_unbound_claims_expire`, `held_once_bounded_per_topic_and_taken_only_when_resolvable`. Residual: the per-topic cap means a sender minting many keys can push honest held claims out before their binding arrives. That costs only the recovery of those claims, never integrity.

## Worker B (Sonnet 5.5 — R4, R5, R6, R8, R9, R10, and red-team finding 5)

Status: code complete in the worktree, focused tests green, **not yet committed and storage gate not yet run** (see Deviations 1). Ownership claims: `evidence/H-owner.md`.

### What changed

- **R4** `blob_store::write_atomic` calls new `ensure_bucket`: when a fan-out bucket (`blobs/<b>/`, `chunks/<b>/`, `records/<b>/`) is created, its parent is fsynced before the rename into it. Residual: two writers creating one bucket concurrently, where the second sees the directory before the first has synced it, rely on the first's sync (microsecond window).
- **R5** fault seam gains `CrashAfter` (write durable, process dies before returning) and an `Event` log (`BucketSynced`, `Rename`). `BlobStore::get` was lock-free, so a delete racing a read surfaced as a quarantine plus `HashMismatch`: it now splits into `get_once`, which reports a structural miss, and `settle`, which looks again under the payload's stripe. Gone gives `BlobNotFound`; whole means retry (up to 4 times); still broken is quarantined. `size` and `refuse` share `settle`.
- **R6** `elohim_blob_fetch_wasted_bytes_total{reason}` (`losing_racer`, `oversized_reply`, `hash_failure`, `unpersisted_shard`), pre-touched at zero. Counted in `blob_fetch` (complete replies already in hand when a race is won, oversized replies, hash failures), in `blob_swarm` (iroh hash failure, a verified swarm shard that failed to persist).
- **R8** `BlobStore::list_units()` (verified chunk files and inline payloads, hashed via `get_chunk`; a failing unit is quarantined, never listed) and `unit_names()` (the filename candidate set). `manifest_backfill_pass` ends with `claim_held_units`: every unit not yet rowed for the agent is read back verified, then rowed (counter `units_recorded`, cap = batch cap on rows recorded). `distribute_shards` uploader arm also rows the payload's chunks for a striped manifest. Worker A's per-chunk rows and announcements for unstriped payloads are unchanged.
- **R9** `SelfHeldOutcome::Unchanged`: an identical self-held row is not rewritten. `distribute_shards` announces only on `Recorded`; `Unchanged` still counts as held (`distributed`, `self_recorded_units`) so a redistribution does not manufacture a placement gap. No restart republishing.
- **R10** `forged_iroh_inventory_holder_is_refused` drives `handle_gossip` with `GossipSource::Iroh`.
- **Finding 5 (a)** `main.rs` rotation tick now passes `reconcile::custody::VerifiedPantry` (new: `has` = `BlobStore::holds_verified`, a hashed read) to `run_rotation_pass`, not a filename snapshot. **(b)** `run_custody_reconcile` passes the same `VerifiedPantry` to `reconcile_pass`, so a corrupt local copy is re-fetched. **(c)** `answer_recon` answers from new `BlobStore::list_servable()` (verified units plus verified chunked payloads), not `list_cids`.
- `services::custody_rotation` did author a pledge from the filename list; that is fixed by (a) (it was a call-site fix, not a function rewrite).

### Tests (focused runs, `RUSTUP_TOOLCHAIN=1.96.1 CARGO_BUILD_JOBS=1`, all EXIT=0)

| Item | Test | Result |
|---|---|---|
| R4 | `blob_store::tests::new_bucket_parent_is_synced_before_rename` | ok |
| R5 | `power_loss_after_marker_reopens_verified` | ok |
| R5 | `delete_racing_store_never_serves_partial` | ok |
| R5 | `delete_racing_read_returns_whole_or_typed_absent` | ok |
| R5 | `quarantine_leaves_unrelated_objects_readable` | ok |
| R6 | `p2p::blob_fetch::tests::wasted_bytes_counts_a_losing_racers_partial_body` | ok |
| R6 | `stall_deadline_abandons_a_silent_holder_before_total_deadline` | ok |
| R8 | `reconcile::custody::tests::striped_upload_rows_cover_every_served_unit` | ok (`--features p2p-iroh` build) |
| R8 | `multi_chunk_payload_rows_every_chunk` | ok |
| R8 | `unit_sweep_rows_nothing_it_cannot_read_back_verified`, `blob_store::tests::list_units_names_only_verified_chunks_and_inline_payloads` | ok |
| R9 | `tests/blob distribute_shards_diversity::redistribution_emits_no_unchanged_announcement` | ok (9 passed, 1 ignored as before) |
| R9 | `self_stewardship::tests::self_held_recording_is_idempotent` (now Recorded, Unchanged, Unchanged) | ok |
| R10 | `p2p_iroh::gossip_receive::tests::forged_iroh_inventory_holder_is_refused` | ok (`--features p2p-iroh`) |
| 5a | `services::custody_rotation::tests::rotation_never_pledges_a_payload_with_a_corrupt_chunk` | ok |
| 5b | `reconcile::custody::tests::reconcile_refetches_a_corrupt_local_copy` | ok |
| 5c | `blob_store::tests::recon_set_excludes_unverified_units` | ok |

All 30 `blob_store::` tests pass together, including Worker A's.

### Deviations

1. **Not committed, gate not run.** Worker A resumed in the same worktree with uncommitted hunks interleaved in four files I also edited (`blob_store.rs`: `Exists`/`exists_locked`; `reconcile/custody.rs`: identity join; `p2p/mod.rs`: module declarations; `main.rs`: transport-link announce). `brit add` stages whole files only, so a pathspec commit of mine would carry A's unfinished work, and A's hunks reference untracked files. The files that are purely mine are `metrics.rs`, `p2p/blob_fetch.rs`, `p2p/blob_swarm.rs`, `services/self_stewardship.rs`, `services/custody_rotation.rs`, `p2p_iroh/gossip_receive.rs`, `tests/blob/distribute_shards_diversity.rs`. Intended sequence: A commits, then B commits the remainder by pathspec, then one `just gate elohim-storage` and `just gate rust-style`.
2. **`list_units` semantics.** A chunked RS shard pushed to this node is a marker payload under `blobs/`, not a unit; its chunks are the units and the shard's own row comes from the existing `claim_held_shard` path. A striped upload's derived shards are not held by the uploader, so its rows are the payload's chunks.
3. **R6 cancelled work.** Bytes of a racer aborted before its reply completed are unknowable at this layer and are not counted.
4. A `get` that settles `Healthy` retries up to 4 times; after that it returns `Internal`.
