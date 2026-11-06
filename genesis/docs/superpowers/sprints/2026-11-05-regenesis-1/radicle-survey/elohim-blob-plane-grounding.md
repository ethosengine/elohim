GROUNDING REPORT: artifact storage on the p2p blob plane (read-only; paths are under /projects/elohim). Not checked: I did not open the iroh blob wire code (p2p_iroh/blob_store.rs, inventory_fetch.rs), the habit files tied to this sprint beyond blob-durability, or the rakia-executor crate's use of `output_cid`. Treat those as unverified.

## 1. The blob stores

**elohim-storage BlobStore.** State: wired.
- `elohim/elohim-storage/src/blob_store.rs:84` defines `BlobStore`. `CHUNK_SIZE` is 1 MiB (`:52`). `MAX_INLINE_SIZE` is 16 MiB (`:55`).
- Addresses are CIDv1 raw (0x55) with a sha2-256 multihash (`:204-261`). The legacy `sha256-<hex>` path form is still accepted (`:221-240`).
- `store()` at `:282` computes the addresses itself. `store_chunked` (`:433`) writes `<hash>.d/NNNNNNNN` chunk files plus an index JSON (`chunk_hashes`, `chunk_size`) and a `CHUNKED` marker at the blob path.
- There is also `store_dag_cbor` (`:332`) and `store_and_announce` (`:402`).
- Known wart: the already-exists branch returns `chunk_count: 1 // TODO` (`:292`).
- Wrapper: the `shard_*` and `sharding` paths (ShardManifest, RS-quilt) cover blobs over 16 MiB. That wrapper is a separate mechanism from the 1 MiB chunk index.

**eprfs tier blob store.** State: wired, local only, not on the network.
- `elohim/eprfs/eprfs-storage/src/tier_blob.rs:1-60`. `TIER_BLOB_CHUNK_BYTES` is 1 MiB. A `TierBlobManifest {epoch, record: DerivationRecord, byte_len, chunks[BlobCid, byte_len]}` is stored as DAG-CBOR under `manifest_cid`.
- `record.output` is the raw CID of the whole artifact. The reader verifies every chunk, then the whole CID, before returning bytes.
- The module header says it creates "no network head, publication, or daemon dependency."
- `put_local_blob` (`eprfs-peer/src/local_blob.rs`) puts the artifact and appends a `Produce` FlowEvent. If custody succeeds but the append fails it returns `LocalBlobUnrecorded` carrying the manifest CID. That is the commit "box the manifest CID in the unrecorded-blob errors."
- Origin: backlog row `native-dev-track-e-tier-blob-store-slice1-local-chunked-put`, from the spec `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md`.

**How the two relate.** Same chunk size and same raw-sha256 CID codec. They are not the same store. eprfs-storage is the `EprfsStorage` trait with a memory adapter and a private directory store (`eprfs-storage/src/lib.rs:1-5`). `StorageEndpoint` is documented there as an adapter config "for a future elohim-storage HTTP-backed client." No code binds the tier blob store to elohim-storage's BlobStore. The network dataplane the artifact sprint builds on is elohim-storage plus its p2p modules. The eprfs tier blob manifest is the candidate shape for the artifact entry. The two need joining.

## 2. Transfer

- **libp2p:** `/elohim/blob/1.0.0` request-response (`p2p/blob_protocol.rs`, `p2p/blob_fetch.rs`).
  - `race_fetch` takes candidates from `peer_blob_inventory`, strongest evidence first, filtered to connected peers. It races batches of `fetch_blob_parallelism` (default 3).
  - The first reply that returns bytes and verifies the sha256 wins (`blob_fetch.rs:1-15`).
  - On success it persists locally, records fetch-success in the inventory, and emits a `serve-blob` REA event.
  - State: wired, and verify-then-store.
- **Multi-source chunk fetch.** State: partially wired in elohim-storage. `p2p/blob_swarm.rs:1-50` is a "torrent-style shard-swarm". Each shard races independently across the known holder set, with bounded total in-flight. Landed shards persist via `finalize_fetch_success`, so each peer that lands a shard becomes a source. This works on shard manifests (the over-16 MiB path) and per-shard host lookups. A plain 1 MiB-chunked blob with a chunk index does not appear to use it. I did not verify that.
- **iroh:** `p2p_iroh/{blob_store,inventory_fetch,shard,pull_core}.rs`, `iroh-blobs =0.94` behind the `p2p-iroh` feature (`Cargo.toml:414,429`).
- **R12(d) as written** ("`materialize` took one base"): the only `materialize` in eprfs is `eprfs-local/src/lib.rs:43`, which materializes a projection manifest to disk. That is not a network multi-source fetch. In eprfs, `FetchPolicy` has only `LocalOnly` and `FetchIfMissing` (`eprfs-core/src/storage.rs:41-45`). The fetch trait checks `cid.verifies(bytes)` on receipt (`storage.rs:47-49`). So multi-source is wired on the elohim-storage side for shards and absent on the eprfs side.

## 3. Inventory and discovery

- `peer_blob_inventory` is a diesel table (migrations 2026-05-02, 05-08 blake3 hash, 05-10 self_custody, 06-15 transport_affinity). `db::peer_blob_inventory::lookup_hosts` is used by `blob_swarm.rs`. State: wired.
- Gossip: topic `elohim/inventory/blob` over gossipsub, MessagePack, with `BlobInventorySnapshot` and `BlobInventoryDelta` (`p2p/inventory_gossip.rs:1-30`), plus `inventory_broadcaster.rs` and `inventory_reorder.rs`. iroh has `p2p_iroh/gossip.rs`.
- **Gap that matters for Radicle:** inventory signatures are a structural placeholder. The file says non-empty, with a single null byte sufficient, "Ed25519 in Stage 2" (`inventory_gossip.rs:16-17,121,143,175`). I did not check whether Stage 2 has since landed elsewhere. Inventory is advisory. The fetch hash check is what makes it safe, as the trust contract says.
- Discovery caveat: `custody.rs:15-21` notes that gossip inventory publish dies on every pod restart (`InsufficientPeers`). So an empty inventory falls back to racing connected peers (cap `FALLBACK_CANDIDATE_CAP`).
- eprfs side: `eprfs-peer/src/announce.rs`, `gossip.rs`, `head_table.rs`, `head.rs` carry signed channel heads, not blob inventory.

## 4. Custody and replication

- `reconcile/custody.rs:1-40` is the controller. For each `custody-blob` commitment in `rea_commitments`, if this peer is the provider and the blob is missing, it kicks `race_fetch`. If this peer is the receiver and the provider has been silent past `placement_grace_seconds`, it emits a `placement-gap` event. `custody_sweep.rs` is the timer and `ConnectionEstablished` driver. `reconcile/placement.rs` handles distance-based placement. The file says distance never enters an eviction decision (`:33`).
- Replication: salvage (`custody.rs:398-406`). `SalvageConfig {enabled, target_replicas, inventory_freshness_seconds}`. A node is never conscripted (opt-in, default false). `config.salvage_target_replicas` is set at `main.rs:440`. Replica count is a per-config target of distinct holders. It is not a per-blob policy.
- Eviction/GC: `services/holds.rs` implements the rule from the footprint spec `genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md` §4. Bytes a peer brought here by its own act, and that nothing names any more, are its own to let go. Bytes another peer placed here are that peer's to withdraw. Bytes with no arrival record are reported and never released. "Named" means a kept release, a content row, a live `custody-blob` commitment, or a shard of a held blob. Arrivals are read from `db::blob_arrivals`.
- So an unpinned blob that this peer fetched itself is released on the retention tick, bounded per pass. Another peer's placed blob is held until that peer withdraws.
- Habit `elohim/elohim-storage/.epr-meta/blob-durability.habit.md` (status green; flipped 2026-10-18 on household evidence; fleet half unmeasured). Its check is `@concern:blob-durability`. Six of its eight listed feature files are wholly `@wip`, so the runnable set is 14 scenarios in 4 files.
- R12(b), "persistent custody for build outputs (not task-scoped leases)": no code minting a `custody-blob` commitment for a build output was found. The related backlog item is `2026-07-30-custody-blob-first-commitment-auto-producer.md`. State: absent for build outputs.

## 5. rakia (`elohim/rakia`, a submodule per `.gitmodules`)

- Members: rakia-core, rakia-brit, rakia-executor, plus `schemas/v1/`. `docs/composition.md`, `docs/specs/2026-04-12-rakia-design.md` and `docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md` exist.
- **Release manifest.** `schemas/v1/release-manifest.schema.json` already exists. It has `channelId` (`runtime:<class>:<network>:<channel>`), `artifactClass` and `artifacts[]`. Each artifact carries `blobCid` (CIDv1 raw sha2-256), `sha256` and an `artifact` $def. The schema says the channel is a content id and "the channel's canonical head is the current release." The previous-release CID is a hint only. Soak attestations exclude the builder's own, and there are rollout waves and an `adoptionDiscipline`. It is open by design (unknown properties tolerated). It rides the existing `Content` entry type through the `metadata_json` valve, so it is DNA-hash-neutral. Governing spec: `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md` §5/§8. State: wired for runtime releases (coordinator bundles, config, storage binaries, happ bundles, app bundles). Relationships such as depends-on, supersedes and constellation graph are not in this schema. That is why R12(a) remains.
- **Artifact manifest entry as R12(a) describes it** (a generic artifact entry naming a blob CID with depends-on and supersedes): not found. The memory note names it as the missing node `artifact-manifest-entry`. State: absent.
- **Channel-head resolution (R12 e).** The runtime-release path exists. Heads are elected and are not recency. Generic build-artifact heads do not exist.

## 6. brit

- `elohim/brit/brit-build-ref/src/build_cmd.rs:72-93`. The `build put` path takes `--output <CID>` or `--output-json`. With `--output-json` it stores a `GateVerdictContentNode` and uses its CID as `output_cid`. `BuildAttestationContentNode {manifest_cid, step_name, inputs_hash, output_cid, agent_id, hardware_profile, build_duration_ms, built_at, success, signature}` is signed with ed25519. So R12(c) is mechanically possible today: `--output <artifact CID>`. What is missing is the artifact being a CID-stored blob on the plane. State: partial.
- Evidence read: the ngit survey (2026-10-29) reports `build find/get` matched raw JSON with no signature check, but a test file `brit-build-ref/tests/verified_reads.rs` and `tests/cas_writes.rs` now exist. I did not verify whether those gaps are closed.
- Key custody: `brit-epr/src/engine/signing.rs`. One ed25519 `AgentKey` per clone at `.git/brit/agent-key`. Decision D5 (steward-space spec, 2026-10-27) keeps it as its own per-device key domain. No threshold-of-signers primitive for a ref was found in signing.rs.
- Signed refs: `BranchContentNode`/`RefUpdateContentNode` are designed in `elohim/brit/docs/specs/2026-04-12-brit-design.md:66-81` and unbuilt as of the ngit survey. eprfs-peer's `ChannelHead {device, root, seq, at, folded, signature}` (`head.rs:3-40`) is built. It carries a per-device strictly increasing `seq`, and `at` is display only.

## 7. The sprint's own plan

- Governing: `genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md` (A5/A7/A8/A10, S2/S4 rakia node) and `2026-10-29-closure-keyed-landing-plane-design.md` (the "Artifact custody" row at `:61`, closure-tagged artifacts `:93`, skip predicate `has_artifact ∧ brit note` `:289`). The runtime-release spec above covers heads. `2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md` §11 is the sequencing hub (Tracks A-E).
- R12 itself is recorded in `.claude/memory/project_rakia_as_artifact_store_custody_registry.md`, citing plan `/projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md`. That plan file is outside the repo and I did not read it.
- The ngit survey's H8 (read-only OCI gateway) is tied to "plan row 5 (artifact custody), closure-keyed."
- Cluster file: `genesis/data/timeline/backlog/CLUSTERS.md` (read). A Radicle take would fold into:
  - `arch-native-dev-plane-backlog`, which says "Fold new native-dev-plane concerns here" (artifact custody, signed refs and the tier blob store).
  - `arch-dataplane-borrows-backlog`, for externally sourced dataplane mechanisms such as gossip inventory and seeding policy. Each row is p2p-design-gated.
  - `brit-daily-driver-handoff-2026-10-27`, for signed refs and COBs on the brit side.
  - `commons-holonic-stewardship-backlog`, for delegate-threshold or maintainer-set standing.
- Habits touching this: `elohim/elohim-storage/.epr-meta/blob-durability.habit.md`, `dataplane-convergence.habit.md` and `elohim/brit/.epr-meta/brit-native-daily-driver.habit.md`. No habit is named for artifact storage specifically. Per CLAUDE.md, the new work needs one bound to a runnable check, with a one-line DELTA in the habit's atom.

## 8. Prior verdicts (ngit survey, `genesis/research/ngit-nostr-git-forge-cross-pollination-2026-10-29.md`)

The core cut: state (signed refs, replicated as events) is separate from data (git objects on interchangeable servers).
- **T0 Take:** keep stock git porcelain and add evidence verbs that fail closed and work offline. Authority rides beside git, not instead of it.
- **T1 Take:** use the NIP-34 lifecycle as a checklist. Status is a derived fold over immutable atoms, never an authored last-writer-wins record.
- **T2 Take:** a listing grants nothing until the listed person acts. An act is judged by the role its signer held at the time.
- **T4 Take:** the fast-forward half of the release plane is built. The missing node is admission without GitHub, via a brit ValidationAttestation.
- **T5 Take:** three CI invariants: unknown policy is refused, the provider asserts only execution, and a result binds the workflow bytes.
- **T7 Take:** brit names and publishes its signer safely (a verb, atomic generation). Custody itself is settled by D5.
- **T10 Take:** serve exactly what the signatures justify. Absence never proves disposability.
- **L1 Leave:** the keypair as repository root, fusing provenance and standing. A signature is provenance only. Standing is held by people and collectives, and the network holds the root.
- **L2 Leave:** signed state with no expected prior, ordered by `created_at`. Our heads move by per-device `seq`, by election, and by an expected prior.
- **L3 Leave:** authority rules interpreted by several parties. The lesson to keep: one verified read path for brit evidence.
- **L4 Leave:** git SHA-1 and event ids as signed identity. Our address is CIDv1.
- **L5 Leave:** a service-wide whitelist as "private."
- **L6 Leave:** patches as event bodies.
- **H1-H9 Hold:**
  - H1: CI compute marketplace.
  - H2: server-side moderation.
  - H3: NIP-34 bridge.
  - H4: GRASP-06 fork-free PRs.
  - H5: Nix runner and Concord.
  - H6: CI secret delivery.
  - H7: fork grouping by `genesisCommit` CID.
  - H8: read-only OCI gateway, with ncontainer's publishing order kept (every reachable blob has a confirmed copy before the signed tag map moves).
  - H9: git smart-HTTP. The serving tier speaks `bundle:<manifest-cid>` over the admitted-devices tier.
- The survey took no GRASP, Blossom or NIP-82 mirror. It records that the rakia release manifest is "more complete than NIP-82."
- **Maintainer sets:** the survey says there is no repo maintainer set and no role history. Standing, delegation and revocation are "station 6, open." Roster home is an open question (participants roster, Mishpat `CouncilRoster`, or a brit stewardship record).
- **Radicle mentions:** `genesis/research/README.md` matches a case-insensitive "radicle" grep, so check that file's exact line before writing the synthesis. The only substantive prior is `playnet-free-association-cross-pollination-2026-08-05.md`. It used Radicle as a code host (`threshold: 1` delegate, seed.radicle.garden ~9 weeks stale). It is not a Radicle architecture survey. No prior Radicle architecture verdict exists, so there is nothing to duplicate.

## The five facts any Radicle verdict must be adjudicated against

1. **No second store.** The plane is elohim-storage `BlobStore` plus the p2p modules. Both blob stores use 1 MiB chunks and raw sha2-256 CIDv1, and rakia names blobs by CID. A Radicle take that implies git-object storage (a repo-as-store) contradicts the standing direction, as the ngit survey's L4 and H9 already say. The eprfs tier blob store is local-only and unjoined to the network plane.
2. **Verification is receipt-side and hash-based.** `race_fetch` verifies sha256 before it stores. The eprfs trait checks `cid.verifies`. Inventory gossip signatures are placeholders (Stage 1). So Radicle's signed inventory announcements and signed ref namespaces would be an addition, not a replacement.
3. **Heads are elected, signed per device with `seq`, never recency.** `ChannelHead` is built (`eprfs-peer/src/head.rs`). The rakia release channel's canonical head is declared. brit `RefUpdateContentNode` is designed and unbuilt. Radicle's delegate-threshold canonical ref would land on the open "station 6" standing and maintainer-set question, which has no roster home yet. L1 means a key never holds standing.
4. **Custody is consent-based and commitment-driven.** `custody-blob` commitments, opt-in salvage with a per-config `target_replicas`, and holds that release only what a peer fetched itself. Radicle's seeding policies map onto this. Nothing is conscripted, and there is no per-blob policy.
5. **R12 is still mostly unbuilt.** (a) the generic artifact entry with relationships: absent. (b) persistent build-output custody: absent. (c) `output_cid` as an artifact CID: mechanically possible, not wired to a stored blob. (d) multi-source fetch: partial (shards only). (e) generic channel heads: absent. The runtime-release manifest is the closest existing precedent.
