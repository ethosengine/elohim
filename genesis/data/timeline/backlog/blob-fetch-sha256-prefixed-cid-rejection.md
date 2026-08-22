---
id: "backlog-blob-fetch-sha256-prefixed-cid-rejection"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Blob pull requests double-wrap CID-form blob hashes as sha256-<cid>, so peers reject them (T21) and CID-addressed bytes never replicate"
slug: "blob-fetch-sha256-prefixed-cid-rejection"
written: "2026-08-22"
author: "orchestrator"
status: "fixed-pending-runtime-proof"
priority: "high"
severity: high
---

## Fix (2026-08-22, branch fix/doorway-breaker-trial-theft-and-apps-extraction-herd)

Root cause found: NOT a Rust fetch-path wrap. The double-wrapped marker was **minted at seed
time** by `genesis/seeder/src/seed-commitments.ts` `normalizeBlobHash` (prefixed `sha256-`
onto anything not already prefixed — including CID-form blob hashes), reached the drill
custody pairs via `seed-drill-fixtures.ts` `resolveExistingSubject` (manifesto's
`content.blob_hash` is the bare CID), and was notarized into `rea_commitments`
`custody-blob` rows on all three household peers (`resource_classified_as =
["sha256-bafkrei…"]`, verified in james's content.db). The T23 custody reconcile sweep then
kicked that literal onto the wire every tick — the ~2min T21 rejection drumbeat.

Landed:

- **Seeder (constructor)**: `normalizeBlobHash` in `seed-commitments.ts` (+ the same inline
  pattern in `blob-manager.ts`, `seed-sqlite.ts` ×2) now prefixes ONLY bare 64-hex; CID-form
  and already-marked addresses pass through untouched.
- **Rust requester hygiene** (`elohim-storage`): new `blob_fetch::normalize_fetch_address`
  (bare hex → prefixed; `sha256-<hex>`/CID untouched; double-wrapped `sha256-<cid>`
  REPAIRED to the inner CID; anything else refused) enforced at the `race_fetch` choke
  point — a malformed address never reaches the wire (`FetchOutcome::InvalidAddress`,
  terminal, WARN). `verify_blob_hash`/`manifest_hash_matches` are now CID-digest-aware.
- **Retry hygiene (sweep driver)**: `reconcile_pass` normalizes markers before any fetch
  decision — CID/double-wrapped markers are re-keyed to the canonical on-disk
  `sha256-<hex>` key (so the presence check stops re-kicking once bytes land, and the
  inventory join finds real holders); unrecognizable markers are counted
  (`ReconcileOutcome::invalid_markers`, logged in the T23 pass line) and skipped — never
  kicked. Responder stays strict (T21 unchanged).
- Regression tests: `normalize_fetch_address_*` (all three forms + repair + garbage),
  `verify_blob_hash_accepts_cid_form`,
  `race_fetch_gives_up_on_invalid_address_without_wire_request`,
  `double_wrapped_cid_marker_is_repaired_before_kick`,
  `double_wrapped_cid_marker_stops_kicking_once_bytes_land`,
  `invalid_marker_is_counted_and_never_kicked`.

**Honest status**: code + tests landed on the branch; the live mesh still runs the old
binary, so the drumbeat continues until the next binary roll. Runtime proof (the "done
when" below — bytes replicate, T21 stops) waits for that roll. The three existing
notarized `sha256-bafkrei…` rows stay in the DHT; the repaired requester now reads them
as their canonical `sha256-<hex>` key, so no data healing is required for the fetch path.

## What was observed (live, local mesh 2026-08-22 ~02:30)

james (:8092) repeatedly logs, on a ~2min cadence, against BOTH peers:

```
WARN T21: rejected blob request with invalid content address
     peer=12D3KooWSN43… hash="sha256-bafkreigvnhemxjinifgz7zri4kdsiu4z45ervy2z4m7qozf5yle4vwtali"
```

The requested address is a CIDv1 (`bafkrei…`, raw-codec) wrapped in the LEGACY `sha256-` marker — a
double-wrapped address no peer accepts. The blob is `evolution-of-trust`'s bundle
(content.blob_hash = `bafkreihokma4tfmwp7y6bj5qpj7v4lpe6x2upozxdbkh2mzeqrj3o7ftb4` per matthew's
shard_manifest_backfill log, which also records "bytes absent locally" for it on matthew). Net effect:
CID-addressed bytes can never replicate over the fetch path — the requester keeps retrying forever
(also a retry-hygiene smell: no backoff visible at this cadence, no give-up).

## Suspected shape

Some fetch-path caller builds the wire address as `format!("sha256-{}", blob_hash)` assuming
`blob_hash` is bare hex, but newer content rows store CID-form (`bafkrei…`) blob hashes. This is the
bare-sha→CID migration seam named in the p2p-design-gate ("legacy `sha256-<hex>` marker; canonical
target is the wrapping CID"). Grep candidates: `elohim/elohim-storage/src/blob_fetch.rs` (T21 emitter),
the pull-queue / acquisition fetch dispatch, and any `sha256-` format sites in p2p paths.

## Fix shape

- The requester must pass the stored address through untouched when it is already a CID (or already
  carries the legacy marker) — prefix only bare hex.
- Responder side stays strict (rejecting malformed addresses is correct — keep T21 honest).
- Add the missing retry hygiene while there: bounded backoff / give-up for a persistently-rejected
  address, so a malformed row cannot generate an infinite 2-minute drumbeat.
- Regression test: fetch-address construction for (bare-hex, `sha256-hex`, `bafkrei…`) inputs.

## Done when

evolution-of-trust bytes replicate to a peer that lacks them (james), T21 rejections stop, and the
fetch-address unit test pins all three input forms.
