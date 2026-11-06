---
title: "Re-genesis 1 T9 report: a2o steps, scripts and fixtures on the byte profile"
id: task-rg1-a2o-report
status: "landed"
landed_commit: "regenesis/a2o (local, unpushed); hash in the hand-back"
landed_on_dev: "dev e85eeeee5..6a31ed270 (L2c+L3, 2026-11-06 ~12:45Z); household deliverability 7/7 on the recast ethosengine household"
verified_by: "orchestrator 184beb01 (claude-fable-5-1), post-L2 verification pass 2026-11-06 on the integrated tree 29a62b5c0/428752cb7 — genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/push-L2c-1.log genesis-a2o GREEN 96 s + gherkin-prepush-lint GREEN (every EXIT=0; summary verify-pass-summary.log). Hook gates on the L2c push attempt (push-L2c-1.log): gherkin-prepush-lint, genesis, elohim-ark, genesis-a2o, elohim-storybook, elohim-app, rust-style all GREEN."
actor: agent:a2o-implementer@claude-sonnet-5-5
cites: []
concerns:
  - "epr-release-package.ts (T5) has 3 import/order eslint errors (lines 38-39) that fail `pnpm run lint`, hence `just gate genesis-a2o`; untouched by instruction"
  - "scripts/__tests__/epr-release-package.spec.ts and release-manifest schema suites fail here only because the elohim/rakia submodule is not checked out in this worktree (ENOENT elohim/rakia/schemas/v1/release-manifest.schema.json); not run green"
  - "`just gate genesis-a2o` not run as a unit; its three parts (lint, format:check, typecheck) were run directly"
  - "no blind-reader pass: no scenario was authored; .feature edits are literal value swaps"
  - "commit attribution UNMEASURED (no epr session in a worktree)"
---

# T9 a2o report

## Mechanical wire rename (a2o-wide)
`blobHash`/`serverBlobHash`/`blob_hash`/`shard_hash` -> `blobCid`/`serverBlobCid`/`blob_cid`/`shard_cid` in the 53 files the lint named (storage/doorway wire, ruling 18:22Z). Duplicate-key fallout (`a ?? a`, doubled object keys) removed by hand; tsc caught the rest.

## New shared admission: `genesis/a2o/src/framework/byte-address.ts`
`mintByteCid` (blake3 + multiformats), `parseByteCid`/`isByteCid`/`assertByteCid` by FULL decode (v1, raw 0x55, multihash 0x1e, 32-byte digest, canonical text, 59 chars). Used instead of `@elohim/epr-ts` because a2o does not depend on it and a new workspace dependency would move the frozen lockfile; it is the same check as `verifyAddress`'s `bytes` form. An eslint allowlist entry covers its devDependency imports.

## Steps and scripts now on the byte profile
- `delivery.steps.ts`: `^bafkr4i` regex removed; `assertByteCid` on captured/served `X-Content-Address`.
- `household-chaos`: hand-rolled base32 decode + regex replaced by `assertByteCid`.
- `name-routing`, `served-under-standing`, `epr-app-deliverability.helpers`: `X-Blob-Hash`/`X-Blob-Cid`/legacy hash/hand-rolled raw+sha2 CID gone; seed PUT sends `X-Content-Address` only.
- `epr-app-deliverability.{steps,helpers}`: parses `blob CID:` (what `scripts/ci/stage-spa-blob.sh:929` prints), admits the upload reply by full decode; helper test mock serves `PUT /blob/<cid>`.
- `happ-lineage-migration`, `runtime-upgrade-propagation`: `putBlobRaw` PUTs `/blob/<byte cid>`.
- `app-bundle-elected-delivery`: zip hashes -> byte CIDs; store path `blobs/blobs/<cid[7..11]>/<cid>`.
- `oversized-blob-ingest` (+ `rs-source-loss-reconstruction`): `sha256Address` -> `byteAddress`.
- `lineage-commitments.ts`: signing-payload CID minted under the byte profile (was raw+sha2).
- `workspace-to-fleet-release`: asserts the manifest artifact's `blobCid` is a byte CID (the manifest carries no `sha256` after T5).
- `failover`, `federation-failover`, `epr-follows-its-head`, `device-declared-target`, `landing-page-dogfood` (step now "the blobCid is a byte CID", feature line updated), `doorway-sibling-reader`, `resilience`, `epr-link-hypercard`, `native-epr-projection` (`x-blob-hash` fallback removed), `manifesto-native-declare`, `surface-census`: legacy spellings out; fixtures minted from real bytes.
- `scripts/lib/fingerprint.ts`: normaliser strips `bafkr4i...` CIDs (`<cid>`), with a test.
- Tests: `compute-runner`, `lineage-fixture`, `release-ceremony`, `served-shell`, `fingerprint` use minted byte CIDs.
- Features: observed `sha256-...` values in `blob-replication`, `federation-deploy`, `served-shell-boots` replaced with byte CIDs minted from labelled bytes; `content-addressing` @wip sentinel `bafkr4inewplaceholder`.

## Design ruling (2): `steps/mesh/death-witness.steps.ts`
The step no longer compares digests across profiles. It reads the custody address from the field the ark witness row carries for it (`blob_cid`, the byte CID of the witness's canonical bytes, exactly what `spool_ingest` names the blob by) and compares that to custody-blob commitments, the on-disk store path, the byte route (`/blob/<byte cid>`, not the record CID) and serve-blob events, each by full decode. The record CID (`bafyrei...`) is used only for `/db/content/<id>` and `ark witness show`.

`elohim/ark` change (the record had no such field): `WitnessSummary.blob_cid` in `supervisor/src/spool.rs` (`elohim_epr::address::compute_bytes_cid(witness.canonical_bytes())`), emitted by `ark witness ls` (`cli/src/main.rs`); assertion added to `spool_writes_witness_row_before_blob_and_lists_newest_first`.

T11 follow-up: `ark witness ls` JSON gained `blob_cid`; tell any other consumer of that shape. The witness record itself is unchanged (a record cannot carry the address of its own serialisation).

## Evidence (logs under genesis/local-dev/rg1-a2o-logs/)
| check | result | log |
|---|---|---|
| `pnpm exec tsc --noEmit` (genesis/a2o) | EXIT=0 | tsc-final.log |
| `lint:gherkin` (303 files) | EXIT=0 | gherkin-final.log |
| `substrate-verify:selftest` | EXIT=0 (223 checks) | selftest-final.log |
| `format:check` | EXIT=0 | not saved |
| `eslint .` | EXIT=1, only the 3 T5 errors in `epr-release-package.ts` | eslint-final.log |
| `test:unit` | EXIT=1: rakia-submodule-absent suites and 2 orchestrator tests; the deliverability helper suite (touched) passes when re-run alone | unit.log |
| `cargo test -p elohim-ark-supervisor --lib spool` (RUSTFLAGS="", 1.96.1) | EXIT=0, 9 passed | ark-test2.log |
| address-profile-lint | genesis/a2o hits 367 -> 0 (repo total 906 -> 539, remainder other tracks) | lint2.txt |

## Classified census of what remains under genesis/a2o (lint is clean for a2o)
- record: `household-attestation.test.ts:118` `^bafkrei...$` on brit's SUT-fingerprint artifact CID (`address-lint: allow`; brit-minted vector, T4 owns); `head-election-witness` dag-cbor+sha2 record CID.
- oci/other (not addresses): `sha256:<16hex>` SUT fingerprints, CI runtime image digest, pinned-executable/binary digests, perf/canary/heap digests, coordinator wasm sha256s, PKCE/derived-password/id hashing.
- legacy fixture left deliberately: `epr-follows-its-head` ("legacy sha256 store" launcher) is a device-store compatibility scenario for T3/T11, not a byte surface; flag for the clean-break decision.
- prose: comments saying "blob hash" name the concept; the wire is `blobCid`.

## Deviations / unresolved
- `epr-release-package.ts` untouched (T5): 3 eslint errors there.
- Blind-reader not dispatched (no scenario authored).
- `just test mesh` not run (brief). The `/shard/<cid>` PUT in `app-bundle-elected-delivery` assumes the route accepts the byte CID; unverified against a live peer.
- Worktree: branch `regenesis/a2o` was found at an older head and reset to `regenesis/integrate` (e85eeeee5); git used because brit panics in worktrees (too few index slots).
