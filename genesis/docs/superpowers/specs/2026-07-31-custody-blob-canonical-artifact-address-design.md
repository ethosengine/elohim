---
title: Custody Blob Canonical Artifact Address
id: custody-blob-canonical-artifact-address-design
status: implemented
class: substrate
context-tier: disclosed
steward: storage
graduation-trigger: implementation-verified
cites:
  - elohim/elohim-storage/src/db/models.rs
  - elohim/elohim-storage/src/http.rs
  - genesis/seeder/src/seed-commitments.ts
  - genesis/data/lamad/content/elohim-host-landing.json
---

# Custody Blob Canonical Artifact Address

## Decision

A custody-facing join keys on the exact address in the shard manifest:
`shard_manifests.blob_hash`. A content row is only a resolver for that address;
it is not itself the custody identity.

`blobHash` and `serverBlobHash` are not aliases and do not represent client and
server computations of one artifact:

- `blobHash` names the content row's primary/browser artifact.
- `serverBlobHash` names its optional Angular SSR server bundle.

They differ because they address different bytes. Upload verification already
rejects a caller-supplied hash that differs from the bytes received, so there is
no upload normalization or re-encoding stage that can explain this pair as two
hashes of one blob.

The temporary bare `sha256-…` address remains the exact join key until the named
CID-first blob-plane migration replaces it with the raw CID (`bafkrei…`). The
rule survives that migration unchanged: custody names the manifest's artifact
address, not a preferred column on a content row.

## Resolver contract

A custody producer has two valid inputs:

1. An exact `blobHash`, already obtained from blob ingestion or a shard
   manifest. This is authoritative for the pledge.
2. A `contentId` plus an explicit artifact role:
   - `content` resolves only `blobHash`.
   - `ssr-server` resolves only `serverBlobHash`.

The resolver never falls back between fields. Missing selected data is a
fail-fast authoring error, because falling back would pledge custody of different
bytes. The `elohim-host-landing` seed pair explicitly selects `ssr-server`
because the saga's observed shard manifest is for the SSR server bundle; this is
not a global preference for `serverBlobHash`.

Runtime distribution already holds the generated `ShardManifest`, so runtime
custody producers use `manifest.blob_hash` directly and do not inspect a content
row.

## Surface inventory

| Surface | Address carried | Role |
|---|---|---|
| content `blobHash` / Diesel `blob_hash` | primary/browser artifact | serving and notarized content reference |
| content `serverBlobHash` / Diesel `server_blob_hash` | SSR server bundle | deploy-time operational projection |
| `PUT /blob/{hash}` result and `ShardManifest.blob_hash` | exact stored bytes | canonical custody join |
| `shard_manifests.blob_hash` | exact stored bytes | fold-side custody join |
| custody commitment `resourceClassifiedAs` | exact pledged artifact | must equal the manifest address |

## P2P design classification

No new entity is introduced. The blob is existing content-derived substrate
content; its current legacy address is the manifest's bare SHA-256 marker and
its target address is CIDv1 raw. A custody commitment remains the existing
notarized REA Commitment entry and projects with `dht_anchor_hash`.

## Supersession

A re-upload creates a new artifact address and therefore a new immutable
commitment identity. Automatic producers author and activate the new commitment
idempotently, then cancel older live runtime pledges for the same
provider/content identity through the existing notarized state-transition path.
Changing a content row never mutates an existing custody commitment in place.
