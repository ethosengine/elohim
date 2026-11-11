---
id: artifact-store-sprint-2026-11-07-design
status: active
---

# Design record — tracks E3, E4, F2 (artifact-store sprint)

Produced by a read-only Opus Plan agent on 2026-11-07 against dev `6d994a931`; every anchor below was read, not recalled. It governs over the draft rows in `plan.md` §2.1 wherever they differ (`plan.md` §2.4 lists those differences). Reviewers of E3, E4 and F2 check the code against this record.

## 0. What the landed code already settles

| # | Claim in §7.3 / §10 / the design gate | Landed state | Ruling for this sprint |
|---|---|---|---|
| F0.1 | E3: `gate.rs:14,64` ignores root and object | True: `eprfs-peer/src/gate.rs:64-71` takes `_root`/`_cid`. Worse: `eprfs-serve` never calls `admit_object`; `TierEndpoint` calls only `admit_root` (`eprfs-serve/src/lib.rs:232,358,378,402,428`). | Fixing the gate alone enforces nothing on objects. The blob answer path calls `admit_object` itself (§1.3). |
| F0.2 | E3: "mounts the blob store beside its channel store" | The daemon passes one `Arc<ChannelStore>` to `TierProtocol::new` (`daemon.rs:315-326`); `tests/daemon.rs:968 a_started_daemon_serves_only_its_channel_tree` pins "only the channel tree". | E3 amends that assertion to "channel roots plus granted artifact roots, nothing else". |
| F0.3 | §10.11 "chunks of at most 1 MiB" | `TIER_BLOB_PROFILE = BYTE_PROFILE_0` minus erasure (`eprfs-storage/src/tier_blob.rs:36-39`): fastcdc min 256 KiB, avg 1 MiB, max 4 MiB (`elohim/epr/src/profile.rs:212-217`). | §10.11 is superseded by the design-gate CDC rulings. A chunk may be 4 MiB; the 16 MiB answer cap (`tier.rs:49`) fits it, but the 10 s server frame deadline (`tier.rs:65`) sets a ~420 KB/s per-stream floor (§3.d). |
| F0.4 | §7.4 R1 prose: "raw + sha2-256 for bytes" | Superseded by the amended R1; `BlobCid::compute_bytes` is raw+BLAKE3 (`eprfs-core/src/address.rs:71`). | Prose is stale; no code change. |
| F0.5 | §10.12a daemon-down put | Landed: `put_local_blob`/`stream_local_blob` and `tests/local_blob.rs` (5 tests, lines 64-247). | Do not redo; E3 reuses its event shape. |
| F0.6 | a `serve-blob` `Use` writer | Not landed; only `copy:self` Use events exist (`local_blob.rs:95-125`). | E3 §1.5. |
| F0.7 | what is the grant today? | None for blobs. `Admission` (roster → node id → did, `admission.rs`); `CrossingAllowList` (which events cross, `allow.rs:33`); `StewardScope` (which resources are steward-scoped, `steward_scope.rs:46`). `Rules::digest` covers crosses and steward only (`rules.rs:76-105`). | E3 mints the grant as a third rule set (§1.4) inside the digest. |
| F0.8 | how does a record cross devices today? | brit notes are device-local. `gate-verdict@1` crosses as a flow event: `emitGateVerdict` → `epr flow note --measure gate-verdict@1 --reason verdict --env k=v` (`genesis/orchestrator/gate-evidence.mjs:667-711`) → `CrossingAllowList` from `pool-policy.json:165-169` → channel tree (`channel.rs:118`) → sibling folds via `fetch_channel` (`fold.rs:139`) → `epr peer verdicts` reads `via:` events (`epr-cli/src/peer.rs:1150-1195`). | F2 records cross the same way (§3.b.1); bytes cross on the tier ALPN. |
| F0.9 | which client fetches a root from an admitted device? | `tier_call`/`tier_request` (`tier.rs:383-424`) with `Read/List/Diff` only; `verify_spine`/`verified_listing` (`proof.rs:48,98`); `fetch_channel` (`fold.rs:139`). No fetch by CID. | E3 adds two request variants on the same ALPN (§1.2). |
| F0.10 | E4: `BlobStoreTier` "composed under the tier" | `BlobStoreTier` exists but is not wired (`elohim-storage/src/eprfs_tier/mod.rs:14-17`); `BlobStore::new` composed once in storage `main.rs:1497`. eprfs-peer may not link elohim-storage, and epr-cli avoids it (`epr-cli/Cargo.toml:51-54`). | E4 extracts the store into `crates/elohim-blob-store` (§2.1). |
| F0.11 | `brit build put --output <bafkr4i…>` | `BritCid::from_str` admits any CID (`elohim/brit/brit-epr/src/engine/cid.rs:64-69`); `brit_index.rs:548-549` round-trips it. brit's CLAUDE.md "BLAKE3 never as a content address" is stale against the design gate. | Works; no brit change for F2. The gix-slots precondition is re-probed in Phase 0 against pin `10d18eacc`; the sibling path (§3.b) does not need `build find`. |
| F0.12 | iroh FsStore | `p2p_iroh/blob_store.rs` says "Not a payload store", answers `NoBytePath`, keys by `Hash::from_bytes(digest)`. | E4 adds only the never-copies test (§2.4). |

## 1. E3 — `serve-blob` mount with scope enforcement (eprfs-peer)

**1.1 Composition.** `PeerConfig` (`daemon.rs:245-277`) gains `pub payload_store: Option<Arc<dyn EprfsStorage>>` (None = a channel-only daemon, today's behaviour). `PeerDaemon::start` (`daemon.rs:315-326`) builds a `BlobMount` and passes a `ServeScope { channel, mount, serves }` to `TierProtocol::new`. The channel store stays the only tree store; the mount is a second answer path, not a second `EprfsStorage` behind `TierEndpoint`. `epr peer serve` (`peer.rs:667-684`) passes `Some(Arc::new(OffloadedDirectory::new(DirectoryStorage::create(path)?)))` where `path` = `--payload-store` / `EPR_PAYLOAD_STORE` / `$XDG_DATA_HOME/epr/payload`. E4 later swaps the implementation behind the same trait object and touches no eprfs-peer file.

**1.2 Wire (same ALPN, same framing).** `TierRequest` (`tier.rs:116-129`) adds `Manifest { artifact: BlobCid }` and `Blob { artifact: BlobCid, cid: BlobCid }`; `TierAnswer` adds `Blob { cid: BlobCid, bytes: Bytes }`. Refusals reuse `TierAnswer::Unanswered { cid: artifact, path: TreePath::root(), why }`. `check_answer` (`tier.rs:427-452`) requires `cid.verifies(&bytes)`, and `cid == asked` for `Blob`; anything else is `PeerError::TierLeafIntegrity`. Manifest decode and `manifest.record.output == artifact` are checked by the fetch layer (F2).

**1.3 Gate.** `gate.rs` gains a `scope: Arc<dyn ServeScope>`:
```rust
pub trait ServeScope: Send + Sync {
    fn classify(&self, root: &BlobCid) -> Served;                 // Channel | Artifact { method } | Unknown
    fn object_of(&self, artifact: &BlobCid, cid: &BlobCid) -> bool; // cid ∈ {manifest} ∪ chunks(artifact)
    fn grants(&self, device: &str, method_name: &str) -> bool;
}
```
`admit_root`: admit(who)?; Channel → Ok; Artifact{m} granted → Ok, else `Err(OutsideStanding, Some("serve-blob:<m>"))`; Unknown → `Err(OutsideStanding, None)`. `admit_object`: admit(who)?; indexed under some granted artifact → Ok, else `Err(NotUnderRoot, None)`. `Inner::answer` (`tier.rs:357-376`) routes `Manifest`/`Blob` to `mount.answer`, which calls `admit_root`, then `admit_object`, then `object_of` before `store.fetch_blob(cid, LocalOnly)`. A requester with no agent gets the uniform form, byte-identical to `TierEndpoint::root_refusal` (`eprfs-serve/src/lib.rs:321-326`) for held-granted, held-ungranted and unknown alike. An admitted device gets the typed refusal. Labels are shown only to the person's own admitted devices; this deliberately narrows `reach.rs:58-61`'s normalization rule and is written into the gate's doc comment.

**1.4 The grant.** New `eprfs-peer/src/serves.rs`: `ArtifactServeList { by_method: BTreeMap<String, BTreeSet<String>> }`, `POLICY_KEYS = ["berth","artifacts","serves"]`, `from_policy`, `grants(device, method, &Admission)`. Audience vocabulary: `"steward"` = every device `Admission::admits_device`; `"carrier:<did:key>"` reserved for D9. `Rules` (`rules.rs:17-20`) gains `serves` and `Rules::digest` (`rules.rs:76-105`) includes it, so skew is visible in `epr peer status`; relays stay outside the digest (`relay_fields_stay_outside_the_rules_digest_regression_guard` stays green). `pool-policy.json` gains `"artifacts": { "serves": { "mesh-build": ["steward"] } }` beside `berth.channel`.

**1.5 Mount and `Use` writer.** New `eprfs-peer/src/blob_mount.rs`: `BlobMount { store, index: RwLock<ArtifactIndex>, served: mpsc::UnboundedSender<ServedDraw>, sessions }`; `ArtifactIndex { by_output, by_object }`; `Held { manifest, method, method_name, chunks, byte_len }`; `ServedDraw { requester, artifact, manifest, bytes, chunks, of, outcome: Completed | Partial | Refused{reason} }`. `index_manifest` reads the tier manifest and the method descriptor record and fails closed (`MethodUnknown`, `ManifestInvalid`). `rebuild_from_flow` scans Produce events tagged `custody-blob` with `manifest:<cid>`, `copy:self` (`local_blob.rs:58-63`); the index's source of truth is the device's own flow store (daemon-down puts land there too), re-scanned in `Run::tick` when `flow_stamp` moves (as `reproject`, `daemon.rs:1104-1126`). A manifest whose method record is absent is `Unindexed { MethodUnknown }` and never served.
`Run` drains `served_rx` each tick and appends one `FlowEvent { action: Use, provider: me, receiver: requester did, resource: artifact, quantity: bytes, classified_as: ["custody-blob", <artifact>, "serve-blob", "copy:<requester did>", "manifest:<cid>", "draw:<completed|partial|refused>", "reason:<…>"?] }` per (requester, artifact) session, closed on the last distinct chunk, on connection close, or after `SERVE_IDLE = 60 s`. Refusals of admitted devices are one `draw:refused` event with quantity 0; strangers are only counted (`TierCounters` gains `served_manifests`, `served_chunks`, `refused_scope`). No `measure:` slot, so it never crosses (`allow.rs:135-147`).
`local_blob.rs`: factor `pub(crate) fn custody_use_event(ctx, resource, bytes, slots) -> FlowEvent` out of `stream_local_blob` (`:95-125`); `LocalBlobContext` gains `pub crossing: Option<(String, String)>` (measure, kind) so F2's Produce can cross. `eprfs-storage/src/tier_blob.rs`: add `pub async fn read_tier_manifest(store, cid) -> Result<TierBlobManifest, TierBlobUnavailable>` wrapping the private `decode_manifest` (E2 edits that body; disjoint hunks). `error.rs`: `PeerError::ArtifactUnindexed { manifest, reason }`, `PeerError::ServeUnrecorded { artifact, source }`.

**1.6 Tests** (`eprfs-peer/tests/serve_blob.rs`, harness from `tests/support/mod.rs` and `tests/tier_alpn.rs:199-313`):
- `an_admitted_sibling_draws_a_granted_artifact_and_the_holder_records_one_serve_blob_use`
- `an_admitted_sibling_asking_outside_its_grant_is_refused_with_a_typed_reason` (method `not-shared` → `serve-blob:not-shared`; a granted chunk asked under another artifact → `NotUnderRoot`; holder records `draw:refused`)
- `a_stranger_gets_the_uniform_refusal_for_granted_ungranted_and_unknown_artifacts` (three encoded answer frames byte-equal)
- `a_device_dropped_by_a_reload_is_refused_on_its_next_chunk_on_an_open_connection`
- `a_daemon_down_put_is_indexed_and_served_after_start`
- `a_manifest_without_its_method_record_is_unindexed_and_never_served`
- `a_swapped_manifest_cid_for_output_cid_is_refused_both_ways` (the §7.3 swap test)
- amended `tests/daemon.rs:968`; `tests/rules.rs::serves_enters_the_rules_digest_and_relays_still_do_not`

**1.7 Gates.** `eprfs` (`CARGO_BUILD_JOBS=1`; cold ≈ 35 min, evidence hit 22 s) and `orchestrator` (pool-policy.json is in its sources). Expect `siblingGovernanceChange` to veto sibling reuse on this branch (`gate-evidence.mjs:400-406` lists `eprfs-peer/**` and `pool-policy.json`): correct. **Integrity class:** `gate.rs`, `serves.rs`, the `blob_mount.rs` answer path, pool-policy `serves`.

## 2. E4 — one authoritative payload store per device

**2.1 Composition.** Forced by F0.10: (i) H2 lands in `elohim-storage/src/blob_store.rs`; (ii) E4 relocates `blob_store.rs` and `eprfs_tier/blob_store_tier.rs` into a new workspace crate `crates/elohim-blob-store` (deps `elohim-address`, `elohim-epr` profile, `fastcdc`, `blake3`, `serde_ipld_dagcbor`, tokio fs, its own `BlobStoreError`; `eprfs-core` for the tier adapter); the elohim-storage files become `pub use` shims with `StorageError: From<BlobStoreError>`; (iii) epr-cli gains feature `blob-store` and `src/payload.rs`:
```rust
pub enum PayloadFormat { BlobStoreV1, DirectoryV1 }
pub struct PayloadStore { pub format: PayloadFormat, pub root: PathBuf, pub store: Arc<dyn EprfsStorage> }
pub fn open_payload_store(root: &Path) -> Result<PayloadStore, PayloadStoreError>
// root/STORE names the format; first creation writes BlobStoreV1 when the feature is on, else DirectoryV1;
// marker ≠ linked support → PayloadStoreError::Format { found, supported }, never a second store.
```
On a device running elohim-storage, `hc-mesh.sh` exports `EPR_PAYLOAD_STORE` into the storage daemon's env and `main.rs:1497`'s tier becomes `BlobStoreTier::at(EPR_PAYLOAD_STORE)` (replacing `dedicated()`'s `<blobs_dir>/eprfs-tier`), so the storage process's tier is the one store. The `/blob/{cid}` route's store stays separate.

**2.2 Cross-process rule.** `BlobStore` serialises mutations with in-process stripes and a `chunk_gc` RwLock; writes are tmp + fsync + rename and idempotent on a verified existing payload, so two processes storing one CID converge. Deletion, GC and quarantine moves are owner-only: `BlobStore::open_owner(root)` takes `root/owner.lock` (`epr peer serve` holds it); `BlobStore::open_shared(root)` refuses `delete`/`quarantine`, and a failed read returns `Corrupt` and moves nothing. The daemon-down library put uses `open_shared`. Tallies are per instance (diagnostics only).

**2.3 What blocks on H2 (R3).** (a) `BlobStore` composed under the tier at all; (b) `Exists::Verified` is structural-only for chunked payloads; (c) the owner/shared split and a cross-process convergence test are H2 additions (today's `concurrent_stores_of_one_blob_count_it_once` is in-process). Until the H2 receipts, E4 ships the relocation, selector and marker with `directory-v1` as the written default and the `blob-store` feature off; the default flips after H2.

**2.4 Tests.**
- `epr-cli/tests/payload.rs::the_same_cid_is_never_held_twice_on_one_device_by_the_two_paths`
- `…::a_payload_store_of_another_format_is_refused_not_duplicated`
- `crates/elohim-blob-store/tests/multi_process.rs::two_instances_storing_one_chunk_converge_to_one_verified_copy`, `…::a_shared_instance_never_moves_a_file` (after H2)
- `elohim-storage/src/p2p_iroh/blob_store.rs`: `an_iroh_index_references_a_tier_payload_and_never_copies_it` (after a tier put, `IrohBlobStore::has(hash_for(chunk_cid)) == false`; pure-iroh answers `NoBytePath`)

**Gates.** `elohim-storage` (jobs=1, 5.4 GB, tens of minutes), `eprfs`, `orchestrator` (`build-inputs-closure.test.mjs` learns the new crate's path-dep closure). One storage-class gate at a time. **Write-set:** `crates/elohim-blob-store/**`, `elohim/Cargo.toml`, `elohim-storage/{Cargo.toml, src/blob_store.rs, src/eprfs_tier/blob_store_tier.rs, src/main.rs:1497, src/p2p_iroh/blob_store.rs}`, `epr-cli/{Cargo.toml, src/payload.rs}`, `genesis/orchestrator/build-inputs-closure.test.mjs`, the `hc-mesh.sh` storage-launch env block only. **Integrity class:** the owner/shared rule and the never-twice test.

## 3. F2 — the sibling fetch

**3.a Producer.**
1. `MethodDescriptor` (`eprfs-core/src/derive.rs:36-59`) for a mesh build: `name: "mesh-build"`, `step: "mesh-build:<elohim-storage|doorway>"`; `tools: { rustc: rustc -vV host + commit-hash under RUSTUP_TOOLCHAIN, cargo: cargo -V, toolchain: "1.96.1" }`; `flags: ["--bin=<bin>", "--features=p2p p2p-iroh" (storage only), "--profile=dev", "--target=<host triple>"]`; `view`: the gate project's `inputs.sources` for the crate (`elohim/holochain/build-manifest.json:404-425`) plus `elohim/Cargo.lock`, `elohim/.cargo/config.toml`, `rust-toolchain*`; `non_path_inputs`: `RUSTFLAGS` (storage `--cfg getrandom_backend="custom"`, doorway empty) and every `mesh_build_env` line except `CARGO_BUILD_JOBS`/`RUST_TEST_THREADS`; `deterministic: false` (dev-profile output embeds paths).
2. Key: `DerivationKey { method: descriptor.cid(), input: KeyInput::Projected(project_view(spine(HEAD tree, view))) }` (Track A's producer, as F1r). Interim, labelled: `KeyInput::Raw(<epr bin source-state closureCid>)` (`device-runtime/src/bin_store/source.rs:241-285`) with slot `basis-key:git-tree`; an interim receipt does not close F2.
3. CLI: new `epr-cli/src/artifact.rs`, `epr artifact put --method <descriptor.json> --input <cid> --bytes <path> --project <name> --commit <sha> [--cross]`: store the descriptor under its CID; `brit-build-ref --repo <root> build put --step mesh-build:<project> --output <bafkr4i…> --inputs-hash <DerivationKey CID> --commit <sha> --success true --duration-ms <n>` (flags at `brit-build-ref/src/main.rs:150-182`); `DerivationRecord { key, output, computed_by: brit signer agentId, basis: Computed, deterministic: false, witness: Attestation{ epr: "brit-build-ref:mesh-build:<project>@<sha>" } }` (Track C's D7 signature replaces the witness when it lands); `put_local_blob` with `crossing: Some(("artifact-produce@1","produce"))` and env slots `project, method, input, key, output, manifest, bytes, commit, agentId` → one Produce event that is both the local custody ledger and the crossing record. `pool-policy.json` `crosses` gains `"artifact-produce@1": ["produce"]`.
4. `hc-mesh.sh` `_mesh_build_one` (`:1996-2032`): after `install -D` + `cmp`, run `epr artifact put … --bytes "$dest"`; a failure prints `[mesh build] unrecorded: <reason>` and writes `<slot>/mesh-bin/.unrecorded` (explicit, never silent; the build still succeeds).

**3.b Consumer.**
1. Resolution: new verb `epr peer artifacts --key <cid> --json` beside `verdicts` (`peer.rs:1197`), with the same `via:` + `admits_device` filter as `verdict_from_event` (`:1150-1195`) → `[{device, output, manifest, bytes, commit, agentId, at, headSeq}]`. Two holders with two outputs for one key → `Mismatch{outputs}` → build.
2. Who dials: the running daemon holds the device key; a second endpoint on it is not viable and an ephemeral key is not admitted. The daemon fetches: `epr artifact fetch` writes `<peer_dir>/fetch-queue/<key>.json` and waits; `Run::tick` picks it up like the roster/policy stamps (`daemon.rs:851`), runs `fetch::fetch_artifact` and writes `<key>.done.json`. No daemon (`ServeLock` free) → `FetchGap::DaemonDown` with its cure.
3. New `eprfs-peer/src/fetch.rs`:
```rust
pub struct FetchBudget { pub stall: Duration /*30 s per request*/, pub total: Duration /*1800 s*/, pub manifest_cap: u32 /*1 MiB*/, pub in_flight_per_holder: usize /*8*/ }
pub struct Holder { pub device: String, pub addr: NodeAddr, pub manifest: BlobCid }
pub async fn fetch_artifact(endpoint, store, key: &DerivationKey, output: &BlobCid, holders: &[Holder], budget: &FetchBudget) -> Result<Fetched, FetchGap>
pub enum FetchGap { NoRecord, Mismatch{outputs}, NoRoute{device}, Refused{device, refusal}, ManifestTooLarge{len,cap}, ManifestRejected{reason}, DeadlineExceeded{fetched,total}, IntegrityFailed{cid}, DaemonDown, TransportCannotDeliver{mode} }
```
Order: `Manifest{artifact}` from the first holder (capped by `tier_request_capped(.., manifest_cap)`); require `manifest.record.key == key && manifest.record.output == output`; for each chunk `store.has_blob` first (resume = only the missing); race missing chunks across holders round-robin, ≤ 8 in flight per connection, each under `timeout(stall)`; a stalled or failed chunk is re-queued to the next holder; verify `cid.verifies && len == byte_len` → `put_blob_verified`; meter wasted bytes; store the manifest last; then `stream_local_blob(.., FileSink(dest.tmp))` — the whole-blob BLAKE3 verify on read is the install read and writes the `Use` (`draw:hit, copy:self, basis:sibling, fetched-from:<did>, wasted:<n>`); rename to dest. The total deadline abandons a trickling holder and re-races within budget.
4. `hc-mesh.sh`: `mesh_fetch()`; `just mesh build` tries `epr artifact fetch` first (`MESH_BUILD_FETCH=0` opts out); a `FetchGap` prints its typed reason and the arm falls through to `cargo build` (Produce).
5. Transport modes: direct address expected to deliver; relay delivers iff per-stream throughput ≥ 4 MiB per 10 s (`FRAME_DEADLINE`, `tier.rs:65`), else `TransportCannotDeliver{"relay"}`; pure iroh-blobs is `NoBytePath` by design this sprint → `TransportCannotDeliver{"iroh-blobs"}`.

**3.c `epr bin`.** Not the path this sprint: its `Release` is keyed by `sourceCommit/treeClean/gateRun` (`bin_store/mod.rs:200-245`), its `sums.cbor` is "a checksum transport, never an identity" (`ingest.rs:115-116`), and ingest copies bytes into its own staging (`ingest.rs:79-105`), a second copy per device. One path: tier + `epr artifact`. A convergence row goes under F5: `epr bin` resolves a release by `DerivationRecord` and launches from the payload store.

**3.d The 454 MB case.** ≈ 433 chunks at the 1 MiB average (bounds 109–1,733); manifest ≈ 35 KB; a 1 MiB cap admits ≈ 13 GB artifacts. ≈ 433 `Blob` requests at 8 in flight per holder; LAN direct ≈ 5–15 s; relay ≈ 90–450 s at 1–5 MB/s; memory ≤ 8 × 4 MiB + manifest per holder. Receipt rows: fetch `Use` against build `Produce` on each device's flow store, with seconds and bytes.

**3.e Tests.**
- `eprfs-peer/tests/fetch.rs`: `a_sibling_fetches_the_closure_from_one_holder_and_records_use_against_the_holders_produce`; `an_interrupted_fetch_resumes_only_missing_chunks_from_the_remaining_holder`; `a_trickling_holder_is_abandoned_on_the_total_deadline_and_the_chunk_re_raced`; `an_oversized_manifest_is_refused_before_any_chunk_moves`; `two_holders_with_different_outputs_for_one_key_is_a_mismatch_not_a_fetch`; `a_manifest_keyed_for_another_closure_is_rejected`; `a_chunk_that_fails_its_cid_is_metered_as_wasted_and_re_fetched`; `pure_iroh_mode_reports_a_typed_transport_gap`.
- `epr-cli/tests/artifact.rs`: `put_records_one_produce_that_both_stays_local_custody_and_crosses`; `peer_artifacts_lists_only_admitted_siblings_records_for_the_key`; `fetch_without_a_daemon_is_daemon_down_with_a_cure`.
- a2o: `genesis/a2o/features/devflow/sibling-fetches-the-build.feature` (`@concern:sprint-lands-each-closure-once`, the 454 MB scenario per transport mode) and `genesis/a2o/features/storage/artifact-custody.feature` (`@wip @concern:artifact-custody`, the R2/R4 stories).

**Gates.** `eprfs`, `orchestrator`, `genesis-a2o`. **Integrity class:** key computation, manifest/record acceptance, whole-blob verify and refusals (`fetch.rs`, `artifact.rs`); `hc-mesh.sh` plumbing is not.

## 4. Write-sets

| Track | Files | Overlap |
|---|---|---|
| E3 | `eprfs-peer/src/{gate.rs, tier.rs, daemon.rs, rules.rs, lib.rs, error.rs, local_blob.rs, serves.rs (new), blob_mount.rs (new)}`, `eprfs-peer/tests/{serve_blob.rs (new), daemon.rs:968, rules.rs}`, `eprfs-storage/src/tier_blob.rs` (+1 pub fn), `epr-cli/src/peer.rs:649-684`, `genesis/agentic/pool-policy.json` (`berth.artifacts`) | `tier_blob.rs` with E2 (disjoint hunks); one `peer.rs` call site with E4 (E4 lands after) |
| E4 | `crates/elohim-blob-store/**`, `elohim/Cargo.toml`, `elohim-storage/{Cargo.toml, src/blob_store.rs, src/eprfs_tier/blob_store_tier.rs, src/main.rs:1497, src/p2p_iroh/blob_store.rs}`, `epr-cli/{Cargo.toml, src/payload.rs}`, `genesis/orchestrator/build-inputs-closure.test.mjs`, the `hc-mesh.sh` storage env block | after H2; no eprfs-peer file |
| F2 | `epr-cli/src/{artifact.rs (new), peer.rs (artifacts verb, fetch queue), main.rs}`, `eprfs-peer/src/{fetch.rs (new), daemon.rs tick}`, `hc-mesh.sh:1996-2032` + `mesh_fetch`, `justfile:420-429`, `pool-policy.json` `crosses`, two a2o features | after E3 (`daemon.rs`, `local_blob.rs`) |
