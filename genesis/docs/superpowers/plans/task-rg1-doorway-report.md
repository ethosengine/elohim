---
title: "Re-genesis 1 T6 report: doorway on the byte profile"
id: task-rg1-doorway-report
status: claimed-not-verified
landed_commit: "regenesis/doorway on regenesis/storage bc0edbd64: f02fce8d5 (doorway-service + doorway-client), 3ed8d4984 (report + live /blob tests), 6a6679917 (clippy fix)"
verified_by:
actor:
  - "agent:doorway-implementer@claude-opus-5-5 (brief named Sonnet 5.5; run on Opus 5.5)"
cites: []
concerns:
  - "ContentServer field rename follows plan T1.2 ahead of T1: the subscriber's infrastructure-signal mirror (ContentServerData.content_cid), services/storage_registration.rs (wildcard content_cid \"*\") and doorway-client ContentServer / PublishSignal / FindPublishers* now say content_cid. regenesis/dna has not landed T1.2; if it keeps content_hash the signal parse breaks, and T1's byte-CID validator must admit the \"*\" wildcard storage registration sends"
  - "ImportQueueRequest and the queue payload sent to storage now carry blob_cid (plan T1.4 renames ImportBatch.blob_hash); unverified against storage's import route"
  - "POST /api/blob/verify wire changed to expected_cid in, and is_valid + outcome {kind: verified | mismatch | unsupported-profile | error} + computed_cid + expected_cid out; lamad blob-verification.service.ts on regenesis/app still sends and reads expected_hash / computed_hash (T7 must follow)"
  - "elohim-render's BundleSource trait keeps resolve_blob_hash / resolve_server_blob_hash (outside T6's write set); the doorway impls return byte CIDs. The rename belongs to whoever owns elohim/elohim-render"
  - "new decision type VerifyOutcome (services/verification.rs) and the upstream-cid-mismatch refusal are not registered in doorway/doorway-service/seam-registry.yaml (the seam-birth advisory fired on commit); census backstop not run"
  - "cache/keys.rs holds zome-call cache keys only (args digest), no blob address: nothing to recast there"
  - "TieredBlobCache's BlobMetadata.hash field keeps its name (it now holds a byte CID; compute_hash became compute_cid)"
  - "clippy under the host default toolchain (rustc 1.99.1) reports pre-existing findings in untouched files (bootstrap/k2.rs, routes/admin_users.rs, routes/human_binding.rs, signal/bus.rs); the gate's pinned 1.96.1 does not (gate EXIT=0); they will surface on the next toolchain bump"
  - "no epr session or actor claim from the worktree; the commit carries UNMEASURED attribution (hook output recorded)"
---

# T6 report (doorway)

Every byte address the doorway admits is a byte CID (`bafkr4i…`, CIDv1 raw 0x55 + BLAKE3-256
0x1e, 59 chars), admitted by full decode through one parser. There is no second spelling:
`sha256-<hex>`, `blake3-<hex>`, bare hex, `bafkrei…`, CIDv0 and upper-case text are refused;
`X-Blob-Hash` is gone; `parse_content_address` and both `compute_sha256`s are deleted.

## Interfaces

### Parser and headers (crate `doorway`, module `routes`)

```rust
pub struct BlobAddress;
impl BlobAddress {
    /// IS elohim_address::ByteCid::parse — full decode; no prefix/length check exists.
    pub fn parse(text: &str) -> Result<ByteCid, AddressError>;
}
pub fn invalid_byte_cid_response(error: &AddressError) -> Response<Full<Bytes>>;
pub const CONTENT_ADDRESS_HEADER: &str = "X-Content-Address";   // the only content header
```

Typed refusal, the same shape as elohim-storage's:
`400 {"error":"invalid-byte-cid","check":<AddressError::check()>,"detail":<Display>}`.

### Routes

| Route | Contract |
|---|---|
| `GET/HEAD /blob/{cid}` (registry-routed to storage via `forward_blob_to_storage`) | Segment admitted by `BlobAddress::parse` **before** the serving fold or storage sees it → typed 400. Pantry hit/miss as before; a storage 200 is stocked and served only if `compute_bytes_cid(body) == cid`, else `502 {"error":"upstream-cid-mismatch","expected","actual"}` (never cached; enforced: doorway/doorway-service/src/routes/storage_proxy.rs `blob_200_that_misses_its_address_is_refused_and_not_stocked`, `blob_legacy_spelling_is_the_typed_400`). A 200 carries `X-Content-Address: <cid>`. Range/206/non-200 pass through uncached. |
| `/blob/{cid}` custom handlers (`routes::blob`, not dispatched by default) | Same parser and 400; `ETag`, `X-Content-Address`, immutable cache headers; storage-proxy fallback verifies fetched bytes; shard fallback below. |
| shard fallback (`ShardResolver`) | Reads the projected DNA `ShardManifest {blob_cid, byte_len, mime_type, chunker:{id,min,avg,max,level}, chunks:[{cid,len}], erasure:{id,data,parity}, stripes:[{index,byte_len,shards:[{cid,len,index}]}], reach, author_id, created_at, verified_at}` (snake_case; every CID through full decode; unknown chunker/erasure ids fail closed). Fetches units by `GET {storage}/shard/{cid}`; each unit is verified against its CID (a bad unit counts as absent); erasure `none` → chunks concatenated in manifest order; `rs-gf8-0x11d-v1` → per stripe at least `data` good shards, missing data shards rebuilt, stripe truncated to `byte_len`. The **whole** payload must hash to `blob_cid` before it is cached or served. |
| `/shard/{cid}`, `GET /manifest/{cid}` | Not doorway logic: registry-routed to storage per its manifest (T2 contract). |
| `GET /apps/{identifier}/{file}` | `identifier` is a content address iff `ByteCid::parse` admits it (`cache::app_file_cache::is_content_address`); anything else is a slug. Projected heads (`data.blobCid`, `data.serverBlobCid`) are admitted only as byte CIDs. `/_capability` answers `x-content-address: <cid>` (one value, replacing storage's). |
| `app_file_cache` (Mongo) | `_id = "{slug}:{file_path}:{blob_cid}"` (slug is the CID itself on a CID path); field `blob_cid`; index `slug_blob_cid_index` on `{slug, blob_cid}`. |
| `PUT /admin/seed/blob` and `PUT /admin/seed/blob/{cid}` | Claimed CID from the path segment and/or `X-Content-Address` (T8's seeder sends the header); each admitted by full decode and, when both are present, they must agree (else 400). Body hashed with `compute_bytes_cid`; mismatch → `409 {success:false, cid, …, error:"CID mismatch: expected …, got …"}`, nothing cached. Success → `200 {success, cid, already_cached, forwarded_to_storage, size, error?}`; the forward is `PUT {storage}/blob/{cid}` plus read-back. Response field `hash` became `cid`. |
| `HEAD /admin/seed/blob/{cid}` | Typed 400 for a non-byte-CID; 200/404 on pantry presence. |
| `POST /api/blob/verify` | `{expected_cid, data_base64 \| fetch_url}` → `verify_cid_any` under the claimed CID's own profile; outcome `verified` / `mismatch` / `unsupported-profile{codec,multihash,len}` / `error`; `is_valid` only for `verified`; non-CID text is `error`, never a mismatch (enforced: doorway/doorway-service/src/services/verification.rs `legacy_spellings_are_errors_not_mismatches`, `unsupported_profile_is_typed_not_valid`). |
| SSR (`ssr.rs`) | `parse_blob_cid` / `parse_server_blob_cid` read wire `blobCid` / `serverBlobCid` and admit only byte CIDs (else a clean `Err` → CSR fallback). |
| freshness amber | `x-elohim-served-head` is the body's byte CID (was raw + sha2). |

## Evidence

- `cargo test --offline --lib --bins` (doorway-service, `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/doorway__doorway-service/dev`, `RUSTFLAGS=""`, host rustc 1.99.1) on the committed tree: **EXIT=0**, `2000 passed; 0 failed; 7 ignored` — `genesis/local-dev/regenesis-1/test-T6-doorway-3.log` (worktree-local; `genesis/local-dev` is gitignored). Runs 1–2 are the red runs fixed before commit.
- `cargo test --offline --tests`: **INTEG_EXIT=0** (every integration target ok) — `test-T6-doorway-clippy-integ.log`. Same log: `cargo clippy --all-targets -- -D warnings` **CLIPPY_EXIT=101** on 1.99.1, findings only in untouched files (see concerns).
- `cargo test --offline -p doorway-client` (through doorway's lockfile): **EXIT=0**, 14 + 2 passed.
- Named tests: `routes::blob::proxy_cache_tests::blob_route_refuses_legacy_spellings_400`, `routes::storage_proxy::…::blob_200_that_misses_its_address_is_refused_and_not_stocked` and `…::blob_legacy_spelling_is_the_typed_400` (added after run 3; exercised by the gate run), `services::shard_resolver::tests::shard_fallback_reassembles_from_stripes_and_verifies_whole`, `…::reordered_chunks_are_refused`, plus `a_33_byte_digest_header_is_refused_by_its_header`, `blob_proxy_refuses_bytes_that_miss_their_address`, `too_few_good_shards_is_refused`, `manifest_with_unknown_chunker_or_erasure_fails_closed`, `addressed_blob_cid_admits_only_byte_cids`, `seed_claims_are_admitted_by_full_decode`, `unsupported_profile_is_typed_not_valid`, `parse_blob_cids_refuse_legacy_spellings`.
- `just gate doorway` at 3ed8d4984 (f02fce8d5 code): **EXIT=1** — clippy (pinned 1.96.1) refused three large `Err` variants (seed.rs `claimed_seed_cid`, verification.rs `parse_expected`) and one index loop in a shard test — `genesis/local-dev/regenesis-1/gate-T6-doorway.log`. Fixed in 6a6679917.
- `just gate doorway` at 6a6679917: **EXIT=0** — rust-style PASS (`doorway/doorway-service: debt 38 -> 37`), clippy and fmt clean, `cargo test` lib `2002 passed; 0 failed; 7 ignored` plus integration targets ok; evidence recorded `bafyreihiwi2slnp…` — `genesis/local-dev/regenesis-1/gate-T6-doorway-2.log`. The pre-existing clippy findings seen under rustc 1.99.1 do not appear under the gate's pinned toolchain.

## Deviations (with reasons)

- **Test name.** The brief's `blob_route_404s_legacy_spellings` is `blob_route_refuses_legacy_spellings_400`: T2's contract answers the typed 400 `invalid-byte-cid`, and the doorway mirrors it so a refusal reads the same at either edge. It covers `sha256-<hex>`, `blake3-<hex>`, bare hex, `bafkrei…`, the 33-byte-digest header (59 chars, refused by `digest-length`), upper case, upper-case body and truncation; storage is never called (enforced: doorway/doorway-service/src/routes/blob.rs `blob_route_refuses_legacy_spellings_400` asserts zero upstream calls).
- **Seed address carrier.** The brief said "compared to the path CID"; T8 (regenesis/seeder e489b0150) already sends `X-Content-Address` on `PUT /admin/seed/blob`. Both forms are admitted and must agree when both are given.
- **Verify-before-serve on the live `/blob` proxy** (`routes/storage_proxy.rs`), though the brief did not list the file: it is the path `/blob/{cid}` actually takes, and stocking unverified bytes under a CID key would poison the pantry. Eight pantry tests now mint their CID from the body they serve.
- **Renames beyond the listed files** (`blob_hash → blob_cid`, `blobHash → blobCid`, `BlobHash → BlobCid` across doorway-service; `content_hash → content_cid` on the ContentServer mirror and doorway-client): mechanical, so no `*_hash` identifier remains on the blob plane. elohim-render's trait method names are kept (concerns).
- **Fixtures** re-minted as byte CIDs of real bytes (the placeholder label's bytes, or a sha256-prefix label when over 64 bytes) by `genesis/local-dev/regenesis-1/tools/doorway_fixture_sweep.py` with a pure-Python BLAKE3 (`mint_byte_cid.py`) pinned at import by the three kernel vectors; prose/history comments naming old `sha256-…` heads are left as history. New tests use the kernel vectors or bytes hashed in the test.
- **Tier.** Executed by Opus 5.5, not Sonnet.

## Unresolved

- Merged-tree behaviour with T1 (ContentServer / ImportBatch field names, the `"*"` wildcard) and T7 (verify wire) — not verified.
- seam-registry rows for `VerifyOutcome` and the 502 upstream-cid-mismatch refusal.
- `doorway/CLAUDE.md` "Addressing canon" still describes `bafkrei…` blobs and `sha256-<hex>` as legacy; a managed-surface edit left for the integrator.
