---
title: "Re-genesis 1 T5 report: rakia + sdk schemas + elohim-views + generated TS + packager"
id: task-rg1-schemas-report
status: landed
landed_commit: "see git log regenesis/schemas (feat(regenesis-1/T5) plus follow-up); rakia submodule f9787c9e58305e1cf51a220968946fc37e6d4b89 on rakia branch regenesis/schemas (local, unpushed)"
landed_on_dev: "dev 9f9fef877..e85eeeee5 (L2)"
verified_by: "orchestrator 184beb01 (claude-fable-5-1), post-L2 verification pass 2026-11-06 on the integrated tree 29a62b5c0/428752cb7 — genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/verify-gate-app-lifecycle-contracts.log, verify-gate-schema-validate.log, verify-gate-schema-codegen.log, verify-gate-domain-types.log, verify-gate-rakia-codegen.log, verify-gate-rakia-validate.log, verify-gate-storage-client-ts.log (every EXIT=0; summary verify-pass-summary.log). Hook gates on the L2c push attempt (push-L2c-1.log): gherkin-prepush-lint, genesis, elohim-ark, genesis-a2o, elohim-storybook, elohim-app, rust-style all GREEN."
actor: agent:schemas-implementer@claude-sonnet-5-5
cites: []
concerns:
  - rakia commit is local to the submodule checkout; push upstream before the pin lands
  - rakia-executor and its schema mirror stay epoch-1 until elohim-epr 0.2.0 is published (ignored test executor_uses_kernel_bytes_cid)
  - no epr session or actor claim could be opened; commits carry UNMEASURED attribution
---

# T5 report

Rebased onto regenesis/kernel (signature schema, epr.ts, Signature.ts, SignatureEnvelope.ts, interop test taken from the kernel unchanged).

## Interfaces

put-blob-response (camelCase; `PutBlobResponseView`):
`{ blobCid, byteLen, mimeType, profile:{ epoch, chunker:{id,min,avg,max,level}, erasure:{id,data,parity} }, chunks:[{cid,len}], stripes:[{index,byteLen,shards:[{cid,len,index}]}], reach, authorId|null, createdAt, verifiedAt|null }`. `stripes` is empty when `erasure.id=="none"`; every cid matches `^bafkr4i[a-z2-7]{52}$`; no other fields.

Signature object (kernel): `{signer:[cid bytes], envelope:{scheme:u16, key:[bytes], sig:[bytes]}}`. View (`EprSignatureView`, camelCase, hex strings): `{signer, envelope:{scheme:number, key:"ed01...", sig:"..."}}`, with new `EprSignatureEnvelopeView`.

release-manifest 2.0: field `manifestVersion` (kept; not renamed to schemaVersion) const "2.0", required; `artifacts[]: {blobCid, bytes, profile:{epoch}, filename, mimeType?, role?, app?, kind?}`; no sha256.

compute-task / compute-receipt descriptors (sdk copies): `{cid, bytes}` (logs add `name`; chunks `{cid, bytes}`); `runtimeImage` stays OCI `sha256:`.

Content views: `blobHash` gone, `blobCid`; `serverBlobHash` -> `serverBlobCid`; `stagingCandidateBlobHash` -> `stagingCandidateBlobCid`; PlacementGapView and ResilienceSnapshot `shardHash` -> `shardCid`.

Packager: storage `PUT /blob/<bafkr4i..>` must answer `{blobCid}` equal to the minted CID; doorway admin seed route takes request header `X-Blob-Cid: <cid>` instead of `X-Blob-Hash` (T6 must accept it).

epr-ts: `addressForm` returns `'bytes' | 'raw' | 'dag-cbor' | null` (T7: `raw-cid-verify.ts` must require `'bytes'`; `'legacy'` is gone); `verifyBlobHash` is async.

## Gate EXIT lines (logs in genesis/local-dev/regenesis-1/)
- gate-T5-epr-ts.log EXIT=0 (before rebase; after rebase epr-ts `pnpm test` 42/42)
- gate-T5-storage-client-ts.log EXIT=0; gate-T5-schema-validate.log EXIT=0; gate-T5-schema-codegen.log EXIT=0
- gate-T5-app-lifecycle-contracts.log EXIT=0 (after RS-GOV-04 reasons on 25 allows in elohim-views)
- gate-T5-seam-guidance-contracts.log EXIT=0; gate-T5-adapter-test.log EXIT=0 (projection_cids_stay_cite_profile)
- gate-T5-domain-types.log EXIT=0
- gate-T5-export-bindings.log EXIT=0 (after rebase, `cargo test -p elohim-views`, 473 tests)
- gate-T5-epr-schema-contract.log EXIT=0 (after rebase, kernel schema_contract against the recast schemas)
- gate-T5-packager-spec.log EXIT=0 (32 pass; 14 CLI tests need a mock 127.0.0.1:8090 /admin/adoption, a pre-existing environment dependency)
- gate-T5-seam-contracts.log EXIT=1: wasm32-unknown-unknown target not installed here (environment)
- gate-T5-app-tsc.log EXIT=2: TS6059 rootDir noise (pre-existing) plus the three consumer errors below
- `node elohim/sdk/schemas/scripts/test-address-crossing-schemas.mjs`: 46 passed; rakia `node --test schemas/tests/release-manifest.test.mjs`: 8 passed

## schema_contract.rs assertions storage must change (not edited by T5)
- ~204-242 ContentHeadView: `blob_hash` -> `blob_cid`, `staging_candidate_blob_hash` -> `staging_candidate_blob_cid`
- ~1450, ~1486 PlacementGapView `shard_hash` -> `shard_cid`
- ~1662-1740 EprSignatureView `{signer, algorithm, signature}` -> `{signer, envelope:{scheme,key,sig}}`
- ~1783-1861 `sample_content_view`: drop `blob_hash`, `server_blob_hash` -> `server_blob_cid`, the snake_case-leak assert
- ~3529-3573 both PutBlobResponseView samples: struct rewritten (no encoding/data_shards/total_shards/shard_size/shard_hashes/blake3_hash)
- `sha256-` cid fixtures at ~4014-4059 if they validate against schemas touched here
- storage `views_convert/infrastructure.rs`, `lib.rs` re-exports (Blob*View, EprSignatureEnvelopeView); storage-generated `storage-client-ts/src/generated/Content.ts` and `ContentWithTags.ts` still say `blob_hash`

## Consumers broken by design (tsc)
`app/elohim-app/.../raw-cid-verify.ts:65` ('legacy'), `shefa/services/journal-routing.service.ts:115` (blobHash), `elohim-library/.../conductor-normalizer.ts:101` (blobHash). Not touched: a2o steps, seeder, doorway, `crates/doorway-client`, `crates/elohim-storage-client`, storage-client-ts hand-written client (`client.ts`, `doorway.ts` seedBlob/X-Blob-Hash, `BlobResult.hash`).

## Assumptions and deviations
1. `epr session open` (session-conflict) and `epr actor claim` (envelope-checkout-mismatch) refused; only `berth moor` worked. Attribution is UNMEASURED.
2. brit cannot read HEAD in this worktree (index slots); git fallbacks declared per call. brit worked in the rakia submodule except branch creation.
3. Brief said `schemaVersion`; the live field is `manifestVersion` (storage verify.rs:313 reads it), kept, set "2.0", required. T2 must update verify.rs.
4. rakia is a submodule: committed on its branch regenesis/schemas (f9787c9e); gitlink moved. Push needed.
5. rakia-executor: commented dependency + ignored `executor_uses_kernel_bytes_cid` only. `contract.rs`/`runtime.rs` and the executor's own schema mirror stay epoch-1 (kernel unpublished; crate cannot build here: registry credentials). sdk copies are `{cid,bytes}`, so copies diverge until the swap. ark `pinned.sha256` (runtime.rs:184) is T11.
6. Codemod over-reach reverted by hand (verifyBlobHash rename, a sha2 fixture relabelled bafkr4i, an adapter citation assert). It does not walk submodules. Receipt: worktree genesis/local-dev/regenesis-1/codemod-receipt.jsonl (ignored dir).
7. Collision files by hand: `blob_hash` removed beside `blob_cid` in ContentView/ContentHeadView/CreateContentInput/UpdateContentInput; `Content.content_hash` deleted from lamad types; `content_hash` -> `content_cid` in infrastructure types; import batch `blob_hash` -> `blob_cid`.
8. Signature recast taken from the kernel; mine is only EprSignatureView and the epr-envelope-view proof schema.
9. `@noble/hashes ^1.5.0` added to genesis/a2o/package.json (lockfile +3 lines).
10. Left alone, flagged: `blake3:<hex>` observation-log roots (observation-*-view schemas, RecipeRefView), `ShardInfoView.hash`, `PackageManifestView.contentHash`.
11. MethodId (placement-parameter-set) and source-state `closureCid` patterns moved to bafkr4i per brief; their minters must switch from sha2 to blake3.
