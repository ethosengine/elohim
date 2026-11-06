---
title: "Re-genesis 1 T10+T11 report: steward/node and ark on the byte profile"
id: task-rg1-steward-report
status: claimed-not-verified
landed_commit: "regenesis/steward on regenesis/storage bc0edbd64: b026b4fdf (T11 ark), d46347e28 (T10 steward/node), and the rust-style paydown commit (git log of regenesis/steward)"
verified_by:
actor:
  - agent:steward-ark-implementer@claude-sonnet-5-5
cites: []
concerns:
  - "steward/node has no Reed-Solomon implementation of its own: the storage action handlers are stubs and reed-solomon appears only in Cargo.lock via elohim-storage. The brief's own striped RS keyed by byte CID has nothing to port; shards and stripes come from elohim-storage sharding.rs through the shared ShardCodec. No RS was invented."
  - "ark ArtifactRef::Pinned.sha256 stays a mandatory field (evidence). The supervisor passport (artifact_sha256), native.rs and task_files.rs all compare that digest at spawn; making it optional re-plumbs the passport and was out of scope for working-code-first. The byte CID is the checked identity when present; cid stays an Option."
  - "the byte CID is verified against bytes only at ingestion (ark-cli task_files retain_artifact), not in TaskChild::check, which has no bytes."
  - "the native_task CLI tests and supervisor tests still pin by sha256 only; a cid-bearing end-to-end spawn is not exercised."
  - "no epr session or actor claim in the worktree; every commit carries UNMEASURED attribution."
---

# T10 + T11 report

## Interfaces

### steward/node (T10)

- Shard protocol id `/elohim/shard/2.0.0` (`p2p/protocols.rs` SHARD_PROTOCOL, comment in `p2p/transport.rs`). `ShardCodec`, `ShardProtocol`, `ShardRequest`, `ShardResponse` are re-exported unchanged from `elohim_storage::p2p::shard_protocol` (SHARD_PROTOCOL_ID is `/elohim/shard/2.0.0`, requests carry `cid: ByteCid`). Steward holds no wire types of its own, so storage's `ShardManifest` stripe shape is the only one.
- Action params: `blob_hash` is gone. ReplicateBlob, EvictBlob and RebuildShard take `"blob_cid": "<bafkr4i...>"`, parsed by `ByteCid::parse` in `pod/actions/storage.rs` (`blob_cid_param`). A missing key, the old key, `sha256-` or `sha256:` spellings and record CIDs are refused with `success: false`. `details.blob_cid` is the canonical CID string.
- CLI: `exec replicate-blob` and `exec evict-blob` take `--cid <byte cid>` (was `--hash`), parsed at the boundary (`pod/cli.rs` `parse_blob_cid`).
- `storage::blobs::BlobManager` (bitswap `BitswapStore`) sits on the new `BlobStore`: byte CIDs via get/exists/store, dag-cbor (0x71) via get_record/store_record. `insert` refuses bytes whose recomputed CID differs from the claimed one. Other CID profiles are reported as not held.
- Local blob route `/blob/{cid}`: `ByteCid::parse`, else 400.
- New dependency: `elohim-epr = { path = "../../elohim/epr" }` (re-exports elohim_address).

### ark (T11)

- `ArtifactRef::Pinned { cid: Option<String>, sha256: String, bytes: Option<u64> }`, shape unchanged. `cid`, when present, must satisfy `ByteCid::parse` (`TaskChild::check`, ark-core `task.rs`); a sha2 raw CID or record CID is refused as identity. `sha256` is externally supplied evidence, 64 lowercase hex, still validated.
- Ingestion (`ark-cli` `task_files::retain_artifact`) verifies the bytes against the byte CID when present and against the sha256 evidence; either mismatch refuses with EXIT_ARTIFACT.
- Tests: `task_contract.rs` test `the_child_is_temporary_scrubbed_pinned_and_writes_only_under_output` carries the byte-CID-accepted and sha2-raw-CID-refused cases (labelled task_pin_accepts_byte_cid and task_pin_refuses_sha2_raw_cid_as_identity inside it, because that test already owned the pin cases); `task_files::tests::sha256_evidence_is_verified_when_present` is standalone.

## Evidence (logs under genesis/local-dev/rg1-steward/)

- `ark-test.log`: `cargo test -p elohim-ark-core -p elohim-ark-supervisor -p elohim-ark` in elohim/, RUSTFLAGS="", EXIT=0.
- `steward-test.log` and `steward-test.exit`: `cargo test` in steward/node, RUSTFLAGS="", pool slot steward__node/dev, EXIT=0 (59 lib plus 6, 16, 17 integration tests, including the two new storage-action tests).
- `gate-T10-steward-node.log`: `just gate steward-node`; first run refused by rust-style paydown (113 -> 113), fixed by reasoning two dead_code allows (113 -> 111) and rerun; EXIT=0 on the rerun.
- `gate-T11-ark.log`: `just gate elohim-ark`, EXIT=0.

## Deviations

- Test-name deviations as noted above.
- No striped RS port (nothing exists in steward to port).
- The hub symlink genesis/local-dev/regenesis-1 does not resolve inside the worktree (the sprint dir is untracked in the main checkout); the brief was read from the main checkout, read-only.
