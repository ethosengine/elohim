---
id: rg1-seeder-report
title: "Task report: re-genesis 1 T8 (seeder + CI scripts + recast script + durable lint)"
status: claimed-not-verified
landed_commit: e489b0150
verified_by:
actor: agent:seeder-implementer@claude-sonnet-5-5
class: protocol-canonical
cites: []
concerns:
  - "genesis/local-dev fixtures NOT moved: the tree is gitignored and exists only in the main checkout, which this track may not touch."
  - "just gate genesis exits 1 on preflight only (submodules elohim/brit elohim/rakia sophia and node_modules not checked out in this worktree); its embedded tsc and vitest legs are green."
  - "Wire assumptions below are unconfirmed against T2/T5/T6."
---

# T8 report

## Wire assumptions (for T2/T5/T6 to confirm)

- Header: `X-Content-Address: <bafkr4i...>` on `PUT /admin/seed/blob` and `PUT /blob/<cid>`; `X-Blob-Hash` and `X-Blob-Cid` are no longer sent. `X-Blob-Size`, `X-Entry-Point`, `X-Fallback-Url` unchanged.
- Content-row PATCH/POST fields: `blobCid`, `serverBlobCid` (responses: `stagingCandidateBlobCid`, `servedBundleHeads[].serverBlobCid`).
- `PUT /shard/<cid>` response carries `cid` (was `hash`). `PUT /blob` response may carry `blobCid`/`cid`; no `blake3Hash`.
- `PUT /admin/seed/shard-manifest` body field `blobCid` (was `blobHash`). Zome `get_blob` payload `{cid}`.
- Release-manifest artifact entries identify by `blobCid` (publish-app-release idempotency compares `artifacts[].blobCid`); no `sha256`.
- Seeder `ShardManifest` (storage-client.ts) keeps snake_case `blob_cid`/`shard_cids`; the T1.4/T5.3 shape (`chunks`, `shards`) is not adopted.
- Env var names `CONTENT_BLOB_HASH` / `PROBE_BLOB_HASH` kept (coupled to Jenkinsfile and a2o); values are byte CIDs.

## Recast script

`genesis/scripts/ci/recast-storage-pod.sh <service> <namespace> <humanId> [recast|verify]`, container default `elohim-node` (checked against the pod's containers). Removal list (relative to storage dir, default `$STORAGE_DIR` else `/data`): content.db(+-wal,-shm), graph.db(+-wal,-shm), sync.sled, blobs, blobs_iroh, chunks, index, quarantine, tmp. Kept: identity.key, iroh.key, runtime-config.toml, release-adoption, credentials, ark, compute-payloads. `verify` refuses (exit 3) a retired `sha256-`/`blake3-` entry, missing content.db, unhealthy `/health`, or blobs above RECAST_FRESH_BLOB_MAX. Wired in genesis/Jenkinsfile (recast, then verify after the existing Ready wait). Old script deleted. Never run here (operator-owned).

## genesis/blobs/manifest.json

Regenerated with the seeder's own `genesis-pack` (rewritten to `byteCid`; blobs now named by CID, entries `{cid,size_bytes,content_format}`, `hash` field gone). The bytes were in the tree; old hex-named blob files removed. 3559 entries (the January manifest had 3526 and was stale against current data).

## Deviations

- Lint wired as its own gate project `address-profile-lint` (root-just recipe, warn-class) following how shell gates are declared here, not as a step on the `genesis` project. Phase 3 flip: make the `_gate-address-profile-lint` justfile body just `bash scripts/ci/address-profile-lint.sh`.
- `bundle-zip.sh` mints via `scripts/ci/lib/byte-cid.mjs` (noble blake3 + hand base32), not multiformats (ESM-only, cannot be required by a shell-launched script); vectors pinned by `byte-cid.test.sh`.
- `@noble/hashes` added to genesis/seeder (3-line pnpm-lock edit, no relock).
- Codemod `apply --track seeder+CI` hit collisions (blobHash/blobCid dual fields, duplicate `deriveBlobCid`, `Cides` plurals); all hand-resolved. Receipt: genesis/local-dev/regenesis-1/codemod-receipt.jsonl.
- Hand-edited genesis/data/lamad/content/{manifesto,fct-course,evolution-of-trust,letter-to-dtpr,imagodei-portal,elohim-host-landing}.json (blobHash -> blobCid byte CID; zip sha256s verified before rewrite).
- `genesis/seeder/src/generated/*` still carries `blobHash`/`serverBlobHash` (T5 regenerates; the lint flags them).
- substrate-verify.sh state keys renamed (`contentBlobCid`, `content_blob_cid`); a2o's TS port/selftest (T9) must follow.
- `epr actor claim` refused (envelope-checkout-mismatch: session bound to the main checkout); `berth moor` succeeded. brit cannot open this worktree (gix index-slot error); git used for stage/commit.

## Evidence

genesis/local-dev/regenesis-1/gate-T8-genesis.log (EXIT=1, preflight-only), gate-T8-shell.log (all shell tests, seeder vitest, seeder tsc EXIT=0).
