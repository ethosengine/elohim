---
id: rg1-dna-report
title: "Re-genesis 1, T1 — the DNA crossing: five-DNA seed ladder, infrastructure and node-registry byte CIDs, content_store striped ShardManifest"
status: claimed-not-verified
landed_commit: d12677652
verified_by:
concerns: "DNA hashes pending the first CI DNA run (local packs use rustc 1.99 + --import-undefined, so they cannot be the baseline; ruled by 184beb01); sweettests not run (working-code-first steer); cross-track wire drift (a)-(f) routed to the L2 integrator; authority row 5 rode on infrastructure ContentServer, row 1 did not; BlobToReplicas deletion shifts later LinkTypes indices; commit attribution UNMEASURED (PATH epr 0.3.2 lacks commit-trailers; trailers set explicitly)"
class: protocol-canonical
gap: regenesis-1#T1
actor: agent:dna-implementer@claude-opus-5-5
habits: []
commits: [e21b23f16, 2520f112f, d12677652]
cites:
  - genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/briefs/T1-dna.md
  - genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/plan.md
---

Branch `regenesis/dna`, worktree `/projects/elohim/.worktrees/rg1-dna`, base
`origin/regenesis/kernel` `e6adb9b43`. Brief:
`genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/briefs/T1-dna.md`, governed by
plan §2.5. Device shem, session `617b9272-adc3-4226-9bd4-cec1ac027767`. The cargo work was
done by two rust-architect subagents (infrastructure + node-registry; content_store +
imagodei); the session owner reviewed each diff and made every commit.

## Commits

| Commit | Deliverable | What moves |
|---|---|---|
| `e21b23f16` | T1.1 seed ladder | `network_seed` `elohim_<dna>_alpha → _alpha2` in all five `dna.yaml` and `workdir/happ.yaml`; `manifest_hygiene.rs` expects `_alpha2` |
| `2520f112f` | T1.2 infrastructure, T1.3 node-registry | `ContentServer.content_cid`; `CustodianAssignment.content_cid`, `ShardAssignment.content_cid`; anchors and links renamed; authority row 5 |
| `d12677652` | T1.4 content_store, imagodei spelling, `SCHEMA_VERSIONS.md` v2 | striped `ShardManifest`; `Content.content_hash` deleted; `byte_cid_of` replaces the SipHash helper |

All three carry `[dna:migrate]`. `dna-hashes.baseline` is not moved yet: a fix-forward
commit carries the five CI `DNA-HASH` lines once the first CI DNA run over the landed
crossing prints them.

## Interfaces

The `ShardManifest` family, as written in
`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`. The nested structs
derive `Serialize, Deserialize, Clone, PartialEq, Eq, Debug`; `ShardManifest` is
`#[hdk_entry_helper] #[derive(Clone, PartialEq)]`. Field names and numeric types match
storage's `sharding.rs` on `regenesis/storage` `bc0edbd64` (confirmed by 184beb01).

```rust
pub struct ChunkerDecl { pub id: String, pub min: u32, pub avg: u32, pub max: u32, pub level: u8 }
pub struct ChunkRef { pub cid: String, pub len: u32 }
pub struct ErasureDecl { pub id: String, pub data: u8, pub parity: u8 }
pub struct Stripe { pub index: u32, pub byte_len: u32, pub shards: Vec<ShardRef> }
pub struct ShardRef { pub cid: String, pub len: u32, pub index: u8 }
pub struct ShardManifest {
    pub blob_cid: String,
    pub byte_len: u64,
    pub mime_type: String,
    pub chunker: ChunkerDecl,
    pub chunks: Vec<ChunkRef>,
    pub erasure: ErasureDecl,
    pub stripes: Vec<Stripe>,
    pub reach: String,
    pub author_id: Option<String>,
    pub created_at: String,
    pub verified_at: Option<String>,
}
```

Validation (`validate_shard_manifest`): `blob_cid`, every chunk `cid` and every shard `cid`
parse with `elohim_address::ByteCid::parse` (full decode, no prefix predicate);
`chunker.id ∈ {"fastcdc-2020"}`; `erasure.id ∈ {"none", "rs-gf8-0x11d-v1"}`; `stripes` is
empty iff `erasure.id == "none"`; each `stripe.byte_len ≤ 16 MiB`; each
`stripe.shards.len() == data + parity`. Not enforced: the sum of chunk lengths equal to
`byte_len` (cheap and deterministic; decide with storage).

Coordinator surface other tracks read: content_store anchors `"blob_cid"` and `"shard_cid"`
(were `"blob_hash"`, `"shard_hash"`); node-registry anchors `"content_cid"` and
`"content_cid:shard_index"`; infrastructure anchor `"content_cid"` and link
`CidToContentServer`; signals `ReplicateContent` and `ShardAssignmentCommitted` carry
`content_cid`, `ProjectionSignal::ImportBatchQueued` carries `blob_cid`. `BlobToReplicas` is
deleted, so every later `LinkTypes` discriminant in content_store shifts down by one.

### DNA hashes

Pending the first CI DNA run (CI builds with `RUSTFLAGS=""` on its own toolchain). Local
packs, for reference only and never the baseline:

| role | local packed hash |
|---|---|
| lamad | `uhC0kb2uit1NuNZep9ph9bWmMZci6pSDYkslhsvBpHKUKGfVplu1U` |
| imagodei | `uhC0kcVJouRpPF0cHcIRiNgrN2yQEGY4GB9o_ZiCyDuJ_xTXRII--` |
| infrastructure | `uhC0kkOHWes7Z4UpBHkkfKKyFpzrMufa3d9Eml9Zt67A7Lgya-NNJ` |
| node_registry | `uhC0kXGYcwGtEckpz2WEtaER8w1bUyqibctTcGOmS-9Rhh3hUvk1E` |
| node_registry_v2 (lineage-witness) | `uhC0k_5zYowafhL4wMt-M4RwH8-_lZEIY4J53kIgunhObwVLveSUH` |
| mishpat | not packed locally (seed change only) |

## Verification

| Check | Result |
|---|---|
| `just gate manifest-hygiene` (after T1.1) | EXIT=0, 46 s, 9 passed; crate debt 2 → 0 |
| wasm32 release build + pack: infrastructure, node-registry, node-registry `lineage-witness`, elohim, imagodei | EXIT=0 each |
| `cargo test` infrastructure_integrity | 4 passed (3 new) |
| `cargo test` node_registry_integrity (default and `lineage-witness`) | 2 passed (2 new) |
| `cargo test` content_store_integrity | 106 passed (8 new) |
| `cargo test` content_store `--lib` | 139 passed (2 new) |
| `just gate rust-style` | PASS; imagodei debt 1 → 0 |
| `just gate schema-dna` | pending (run after the workspace install) |
| sweettests (`ShardManifest` create/get, `just gate sweettest-check`) | not run (working-code-first steer) |

New tests: `content_server_rejects_sha256_spelling_and_bafkrei`,
`content_server_accepts_byte_cid`, `content_server_operator_must_be_author`,
`shard_assignment_requires_byte_cid`, `custodian_assignment_requires_byte_cid`,
`shard_manifest_rejects_unknown_chunker_id`, `shard_manifest_chunks_must_be_byte_cids`,
`shard_manifest_stripe_shard_count_equals_data_plus_parity`,
`shard_manifest_stripe_byte_len_at_most_16mib`, `content_hash_field_is_gone`,
`well_formed_manifests_are_valid`, `shard_manifest_rejects_unknown_erasure_id`,
`blob_entry_and_shard_location_require_byte_cids`, `byte_cid_of_matches_the_kernel_vector`,
`on_chain_shard_cid_is_byte_cid`.

## Beyond the brief

- `ContentSuccession.content_hash → content_cid`, validated when present (the same
  sha256-of-body class the brief forbids keeping).
- `get_storage_dids_for_blob` read the anchor `"blob_manifest"`, which nothing writes, so it
  always returned empty; it now reads the anchor the manifest is registered under.
- The infrastructure `ContentServer` update arm previously fell through to `Valid`; it now
  validates.

## Cross-track drift (routed to the L2 integrator by 184beb01)

- (a) storage `node_registry_api.rs` `ShardAssignment.content_hash` → `content_cid`.
- (b) SDK `elohim/sdk/domains/infrastructure/types` still names `content_hash` on
  `ContentServer`, `RegisterContentServerInput`, `FindPublishersInput` and `FindPublishersOutput`.
- (c) storage `import_handler.rs` decodes `ImportBatchQueued.blob_hash`; the DNA emits
  `blob_cid`.
- (d) SDK lamad-types `CreateContentInput`, `UpdateContentInput` and `Content` still carry
  `content_hash`; `QueueImportInput` carries `blob_hash`.
- (e) seeder `seed-production.ts:554` sends the old `register_shard_manifest` payload.
- (f) anchor strings and `LinkTypes` indices changed.
