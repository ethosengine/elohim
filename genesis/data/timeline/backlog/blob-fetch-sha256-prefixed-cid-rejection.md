---
id: "backlog-blob-fetch-sha256-prefixed-cid-rejection"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Blob pull requests double-wrap CID-form blob hashes as sha256-<cid>, so peers reject them (T21) and CID-addressed bytes never replicate"
slug: "blob-fetch-sha256-prefixed-cid-rejection"
written: "2026-08-22"
author: "orchestrator"
status: "backlog"
priority: "high"
severity: high
---

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
