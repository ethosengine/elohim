---
id: rg1-eprfs-report
title: "Re-genesis 1, T3 — eprfs: addresses through the one kernel, epoch-2 tier blobs, the byte profile's TypeScript twin"
status: claimed-not-verified
landed_commit: a3315d6da
verified_by:
concerns: "short_fingerprint -> Result forced a one-call adaptation at cites.rs:760 (fingerprint bytes unchanged, pinned); bin-* view schemas in elohim/sdk/schemas (T5's tree) re-patterned because the device-runtime contract test validates them; tier blob manifests record erasure `none`, not BYTE_PROFILE_0's rs{4,3}; habit declared red, not unwired (the census refuses unwired beside checks); governance_parity_vectors.rs needed no change (it never reads tree-vectors.json); ~140 compute_raw callers outside cites still mint raw+sha2 for non-cite bytes — successor work; device bin-store custody minted before this change is no longer `verified` and needs a re-ingest; commit attribution UNMEASURED"
class: protocol-canonical
gap: regenesis-1#T3
actor: agent:eprfs-implementer@claude-opus-5-5
habits: [addresses-are-declared-profiles]
commits: [0fe60b253, a71e7b02b, 08e7b1831, 20a2ccbb1, 1d2916dca, 101bc17ef, a3315d6da]
cites:
  - "native-dev-tooling-over-peers-plan | §7 rulings R1–R4 that re-genesis 1 amends; T3 carries the amended R1 byte profile into eprfs | sha256:a894a6f45fc92215 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
---

Branch `regenesis/eprfs`, worktree `/projects/elohim/.worktrees/rg1-eprfs`, rebased onto the
`regenesis/kernel` ref at `9f9fef877` (kernel + kernel-fix, itself rebased onto dev). Brief:
`genesis/local-dev/regenesis-1/briefs/T3-eprfs.md`. `ChannelHead` (`eprfs-peer/src/{head,announce,head_table}.rs`,
`heads.json`) untouched.

## What landed

1. **`eprfs-core/src/address.rs`** delegates to `elohim-address` (path + registry dep; `multihash-codetable`
   leaves eprfs-core; rail R-E1's frozen set now names `elohim-address`). `compute_bytes` (byte profile),
   `compute` (record), `compute_raw` (citation; output bytes unchanged, pinned by
   `raw_codec_matches_brit_vector` and `short_fingerprint_equals_python_cite_fingerprint`). `verifies` =
   `verify_cid_any == Verified`; `recompute_for_codec` → `recompute_for_profile` (codec + multihash + length);
   `short_fingerprint` → `Result<String, AddressError>` refusing non-sha2; new `byte_cid()` narrows a parsed
   CID to the byte surface; `From<ByteCid>`. The local `RAW_CODEC`/`DAG_CBOR_CODEC` constructors are gone.
   Tests: `compute_bytes_matches_kernel_vector`, `verifies_rejects_sha2_cid_over_blake3_bytes`,
   `short_fingerprint_refuses_blake3`, `byte_surface_refuses_the_negative_header_vectors`.
2. **`eprfs-storage/src/tier_blob.rs`**: `TIER_BLOB_CHUNK_BYTES` deleted; chunks cut by
   `elohim_epr::profile::try_chunk_bytes(&TIER_BLOB_PROFILE, …)` (fastcdc-2020, profile validated whole) and
   addressed by `compute_bytes`; `TIER_BLOB_MANIFEST_EPOCH = 2`; `profile: ByteProfile` recorded on every
   manifest; `decode_manifest` reads `{epoch}` first, so an epoch-1 object answers
   `UnsupportedEpoch { epoch: 1 }` (held, never a hit, never a parse error — enforced: `eprfs-storage/tests/local_tier_blob.rs` `epoch_1_manifest_is_held_not_read_and_not_a_parse_error`); `validate_manifest` validates the
   recorded profile, bounds chunk lengths by its `max` and refuses chunks not addressed under it; the
   whole-artifact digest is incremental BLAKE3 for a `(0x55,0x1e)` output, sha2 for `(0x55|0x71,0x12)`, else
   `UnsupportedProfile`; `read_tier_blob` returns bytes only after the whole digest verifies. eprfs-storage
   0.3.6, eprfs-local 0.3.5 (publish precheck). Durability suites re-pinned to byte CIDs (`eprfs-storage` 9,
   `eprfs-local` 5). New tests: `epoch_1_manifest_is_held_not_read_and_not_a_parse_error`,
   `profile_is_persisted_with_every_layout`, `a_sha2_output_still_verifies_through_byte_profile_chunks`,
   `reordered_or_substituted_chunks_never_return_bytes`, `an_unsupported_output_profile_is_unavailable_never_a_hit`;
   eprfs-peer `local_blob` re-pinned to 4 MiB max-cut runs.
3. **Vectors**: `eprfs-core/tests/vectors.rs` adds the `address` section and `wire.byteProfile`, re-derives the
   `chunker` section from its own header (xorshift64* generator, table digest, `ValidatedProfile::EPOCH_0`),
   and `EPRFS_WRITE_VECTORS=1` now only ADDS (`merge_additive`; refusal test
   `write_vectors_refuses_to_change_a_pinned_entry`). eprfs-ts: blake3 via `@noble/hashes/blake3` +
   multiformats `hasher.from`; `src/chunker.ts` BigInt twin reproduces all 13 chunker fixtures exactly.
4. **device-runtime bin store**: `ingest.rs` admits sums by `ByteCid::parse`; `files.rs` `require_cid`
   admits a byte CID or a dag-cbor+sha2 record CID, `file_cid` mints `compute_bytes`, `sums_manifest`
   requires byte CIDs; `migrate.rs` keeps legacy `sha256:` sums as sha2 evidence beside the byte identity;
   bin-* view schemas re-patterned. `cites.rs:886` unchanged; `:760` adapts to the `Result` only (fingerprint
   pinned by `fingerprint_of_is_the_frozen_citation_profile` and the end-to-end
   `cite_fingerprint_stays_the_citation_profile_after_the_address_crossing`);
   `research_corpus_has_no_dead_cite_after_the_address_crossing` runs `stamp` + `verify` over `genesis/research`.
   `import.rs:834` carries the citation-profile comment.
5. **Habit** `elohim/eprfs/.epr-meta/addresses-are-declared-profiles.habit.md` (projected into
   `genesis/manifests/habits.yaml`).

## Interfaces (downstream tracks read this block)

```rust
// eprfs_core (eprfs-core 0.3.4, unpublished) — crate root re-exports AddressError, BlobCid, ByteCid
pub use elohim_address::{AddressError, ByteCid};
impl BlobCid {
    pub fn compute_bytes(bytes: &[u8]) -> Self;   // CIDv1 raw 0x55 + BLAKE3-256, bafkr4i…
    pub fn compute(bytes: &[u8]) -> Self;         // dag-cbor 0x71 + sha2-256, bafyrei…
    pub fn compute_raw(bytes: &[u8]) -> Self;     // raw 0x55 + sha2-256, bafkrei… (citations; bytes frozen)
    pub fn short_fingerprint(&self) -> Result<String, AddressError>; // "sha256:<16 hex>", NotSha2 otherwise
    pub fn encode_manifest<T: Serialize>(v: &T) -> Result<(Self, Vec<u8>), EncodeError<TryReserveError>>;
    pub fn parse(s: &str) -> Result<Self, cid::Error>;  // any CID; byte surfaces parse TEXT with ByteCid::parse
    pub fn byte_cid(&self) -> Result<ByteCid, AddressError>; // profile check of a parsed CID
    pub fn verifies(&self, bytes: &[u8]) -> bool;      // verify_cid_any == Verified; unknown profile = false
    pub fn recompute_for_profile(&self, bytes: &[u8]) -> Option<Self>; // was recompute_for_codec
    pub fn as_cid(&self) -> &Cid;
}
impl From<ByteCid> for BlobCid;
// removed: BlobCid::recompute_for_codec; infallible short_fingerprint

// eprfs_storage (0.3.6)
pub const TIER_BLOB_MANIFEST_EPOCH: u32 = 2;
pub const TIER_BLOB_PROFILE: ByteProfile = ByteProfile { erasure: Erasure::None, ..BYTE_PROFILE_0 };
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct TierBlobChunk { pub cid: BlobCid, pub byte_len: u32 }          // cid: byte CID
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct TierBlobManifest {                                             // canonical DAG-CBOR, bafyrei…
    pub epoch: u32,                 // 2; read FIRST, before this schema
    pub profile: ByteProfile,       // the profile the chunks were cut and addressed under
    pub record: DerivationRecord,   // record.output: byte CID, or a sha2 record/citation CID
    pub byte_len: u64,
    pub chunks: Vec<TierBlobChunk>,
}
pub enum TierBlobUnavailable { ManifestCorrupt, ManifestInvalid { reason: String },
    UnsupportedEpoch { epoch: u32 }, UnsupportedProfile { cid: BlobCid } /* new */,
    ChunkMissing { cid: BlobCid }, ChunkCorrupt { cid: BlobCid }, BlobCorrupt { cid: BlobCid },
    Sink { reason: String }, Store { reason: String } }
// removed: TIER_BLOB_CHUNK_BYTES
```

```ts
// @elohim/eprfs (eprfs-ts)
export const SHA2_256 = 0x12;
export const BLAKE3_256 = 0x1e;
export const blake3Hasher; // multiformats hasher.from({ name: 'blake3', code: 0x1e, encode: noble blake3 })
export function byteCidOf(bytes: Uint8Array): Promise<CID>;                 // raw 0x55 + BLAKE3-256
export function verifiesCid(cid: CID, bytes: Uint8Array): Promise<boolean>; // dispatch codec+multihash+len
export function parseByteCid(text: string): CID | { refused: string };       // full decode + canonical re-encode
// src/chunker.ts
export const CHUNKER_FASTCDC_2020 = 'fastcdc-2020';
export const FASTCDC_2020_TABLE_DIGEST = '3bd9b87949825cd9b2a592d199dc15d485812848535c9b1d2ff6979baf7a8001';
export interface Fastcdc2020 { id: 'fastcdc-2020'; min: number; avg: number; max: number; level: number }
export const FASTCDC_2020_EPOCH_0: Fastcdc2020; // {262144, 1048576, 4194304, 1}
export const FASTCDC_2020_GEAR: readonly bigint[]; export const FASTCDC_2020_GEAR_LS: readonly bigint[];
export const FASTCDC_2020_MASKS: readonly bigint[];
export function fastcdc2020TableDigest(): string;
export class ChunkerError extends Error {}
export function validateFastcdc2020(params: Fastcdc2020): void;
export function chunkBytes(bytes: Uint8Array, params?: Fastcdc2020): Array<[number, number]>; // [offset, len]
// src/wire.ts
export type Chunker = { id: 'fastcdc-2020'; min: number; avg: number; max: number; level: number };
export type Erasure = { id: 'none' } | { id: 'rs-gf8-0x11d-v1'; data: number; parity: number };
export interface ByteProfile { epoch: number; codec: number; multihash: number; chunker: Chunker; erasure: Erasure }
export function isByteProfile(value: unknown): value is ByteProfile;
```

`elohim/eprfs/fixtures/tree-vectors.json` additions (compare-only; additive writer):
- `address`: `{ about, positive: [{ input, byteCid, citeCid, recordCid, shortFingerprint }], negative: [{ name, input, check }] }` —
  positives are the kernel's inputs plus `"ok target body"` (`sha256:4e72cded3d6affd3`), each field equal to the
  kernel's row; negatives are `crates/elohim-address/vectors/address-vectors.json`'s rows, each refused by
  `ByteCid::parse` with the named check.
- `wire.byteProfile`: `BYTE_PROFILE_0` as serde writes it (`{chunker:{id,min,avg,max,level}, codec:85, epoch:0, erasure:{id,data,parity}, multihash:30}`).
- `chunker` (written by `elohim/epr/tests/chunker_vectors.rs`) is re-derived here, never rewritten (enforced: `eprfs-core/tests/vectors.rs` `write_vectors_refuses_to_change_a_pinned_entry`).

Habit: `elohim/eprfs/.epr-meta/addresses-are-declared-profiles.habit.md`.

## Gate evidence

Logs are in the sprint hub `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/` (the main checkout's `genesis/local-dev/regenesis-1/logs` link); each ends in an `EXIT=` line. All runs are on `a3315d6da` (this report was the only dirty path).

| Check | Result | Log |
|---|---|---|
| `just gate eprfs` (`CARGO_BUILD_JOBS=1`: publish precheck, fmt, clippy `-D warnings`, rust-style RS-GOV-04 PASS, workspace tests) | EXIT=0 | `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/gate-T3-eprfs.log` |
| `cargo test --workspace --no-fail-fast` (pool slot `sprint/elohim__eprfs/dev`, `RUSTFLAGS=""`, jobs 1) | EXIT=0 — 2059 passed, 0 failed | `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/cargo-test-T3-eprfs.log` |
| eprfs-ts `pnpm build && typecheck && lint && test` (the `eprfs-ts` gate's steps) | EXIT=0 — 34 passed | `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/gate-T3-eprfs-ts-direct.log` |
| `just gate eprfs-ts` | not run (host build slot held by other sessions' cargo; the hook refuses `just gate` while any build runs) | — |
| `just gate rust-style` (standalone) | not run separately (host slot held); RS-GOV-04 ran inside `just gate eprfs`: `[rust-style] PASS`, every touched eprfs crate debt 0 → 0 | `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/gate-T3-eprfs.log` |
| `EPRFS_WRITE_VECTORS=1 cargo test -p eprfs-core --test vectors`, then without | EXIT=0 / EXIT=0; diff additive only (+122 then +15 lines, 0 removed); second run byte-stable | in-session |

Earlier red runs on this branch, kept as evidence of what the gate caught: the publish precheck (`eprfs-storage 0.3.5 packaged source differs` → bumped), `cargo fmt`, the bin-store contract test against the bin-* schemas and the legacy migration test (fixed in `20a2ccbb1`), and the `bin ingest` identity refusal (`reports bafkrei…, bytes are bafkr4i…`, fixed in `a3315d6da`).

## Concerns and deviations

1. **`cites.rs:760` changed by one adaptation.** `short_fingerprint` returning `Result` (brief item 1) cannot
   coexist with a byte-identical `Some(…short_fingerprint())`; the line now ends `.short_fingerprint().ok()`.
   The construction (`compute_raw` over `canonical_body`) and `:886-892` are untouched; the fingerprint is
   pinned by a unit test on `fingerprint_of` and an end-to-end `stamp` test (ok at `sha256:4e72cded3d6affd3`,
   stale one digit off).
2. **bin-* view schemas (`elohim/sdk/schemas/v1/views/bin-{resolution,gc,install-launcher,migrate,pin,release}.schema.json`)**
   are T5's tree, but `device-runtime`'s `f2_machine_output_contracts…` validates real outputs against them,
   so the eprfs gate could not pass without them: release/head CIDs `^(bafkr4i|bafyrei)[a-z2-7]{52}$`, sums
   values `^bafkr4i[a-z2-7]{52}$`. Generated TS carries no patterns (no codegen drift). `source-state`'s
   `closureCid` (raw+sha2, `bafkrei`) is unchanged — the closure CID is still `compute_raw`.
3. **Tier manifests record `erasure: none`** (`TIER_BLOB_PROFILE`), not `BYTE_PROFILE_0`'s `rs-gf8-0x11d-v1 {4,3}`:
   the local tier store holds plain chunks, and design gate byte-profile 6 says a manifest records the
   representation it is. Chunker and address triple are `BYTE_PROFILE_0`'s.
4. **Habit declared `red`, not `unwired`.** `habits-project.py` refuses `status: unwired` beside a non-empty
   `checks:` list. The born line is kept verbatim; a second DELTA records why it is red (no a2o scenario
   carries `@concern:addresses-are-declared-profiles`; `scripts/ci/address-profile-lint.sh` does not exist yet, T0b).
5. **`governance_parity_vectors.rs` needed no change.** It reads `elohim/sdk/schemas/v1/registries/governance-parity-vectors.json`,
   not `tree-vectors.json` (the census attribution in plan §1.2 is wrong); it runs in the eprfs gate.
6. **`compute_raw` still mints non-citation bytes in ~140 call sites** (tree file blobs, memory/flow,
   eprfs-serve testing, device-runtime source closures; `put_blob` mints dag-cbor). Moving them changes pinned
   epoch-0 tree vectors, which this leg's compare-only rule forbids; they are successor work, and the habit's
   retire-when names the condition.
7. **Device bin-store custody minted before this change** (sums `bafkrei…`, single-binary releases named by
   them) is no longer `verified`/admissible; the legacy `sha256:` adoption path still migrates. A device running
   the new `epr` must re-ingest its release (no dual-read, C9).
8. **`parseByteCid` in eprfs-ts** is a full decode (multiformats parse, `01 55 1e 20`, 36 bytes, canonical
   re-encoding) — not a prefix test — so the TS suite can read the negative rows. Its refusal names are
   coarser than the kernel's (`decode`, `non-canonical`); the test asserts refusal, not the check name.
   `@elohim/epr`'s `verifyAddress` (T5) remains the app-facing twin.
9. **Kernel-fix absorbed** (kernel ref at `9f9fef877`): `chunk_bytes` takes `&ValidatedProfile` — tier blobs use
   `try_chunk_bytes(&TIER_BLOB_PROFILE, …)` (validated whole, error mapped to `EprfsError::Storage`), vectors and
   tests use `ValidatedProfile::EPOCH_0`. eprfs-ts mirrors no signing transcript, so transcript v1 needs nothing
   here. The kernel's added negative rows enter `address.negative` through the additive writer.
10. **Attribution UNMEASURED.** `epr session open --model claude-opus-5-5` refused `session-conflict`; `berth moor
    --model claude-opus-5-5 --lab anthropic --runtime claude-code --task regenesis-1-T3` succeeded; every commit
    hook printed `[commit-attribution] UNMEASURED`. Commits carry `Co-Authored-By: Claude Opus 5.5`.
11. **Brit gap.** `brit status --no-write` fails in this worktree ("too few index slots", then "could not peel
    HEAD" after the 1edce85de pin); every repository operation used a recorded Git fallback.
12. **`epr --version` now names the executable by its byte CID** (`epr-cli/src/govern.rs`
    `evaluator_identity`). Not in the brief, but required: `bin ingest` compares the binary's self-reported CID
    with the bytes' CID, and refused every fixture binary (`session_envelope` suite) while the two profiles
    differed. Recall's `executor_digest` (via `bin_store::file_cid`) follows the same profile.
13. **`genesis/research` cite health.** `verify` (the DEAD-CITE reader) reports none over all 56 documents;
    `stamp` reports 3 `dead` and 9 `stale` verdicts and 11 documents with no frontmatter. Those are
    pre-existing (cite construction is unchanged by this leg) and are not repaired here.
14. **`cargo test` needs `RUSTFLAGS=""`** for the eprfs workspace (as its manifest declares); with the shell's
    WASM `getrandom_backend="custom"` flag the `eprfs-local` test link fails on `__getrandom_v03_custom`.
15. **Worktree `node_modules` vanished mid-leg** (reinstalled with `pnpm install --offline`); the hook-written
    `.claude/data/governance-findings.jsonl` lines were set aside to
    `genesis/local-dev/regenesis-1/T3-hook-ledger.diff` for the rebase and re-applied uncommitted.
