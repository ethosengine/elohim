---
id: "backlog-blobhash-serverblobhash-duality-canonical-join-key"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "blobHash/serverBlobHash duality — no canonical join key, custody-facing folds silently miss on client-hash classification"
slug: "blobhash-serverblobhash-duality-canonical-join-key"
written: "2026-07-30"
author: "agentic-developer"
status: "done"
priority: "high"
area: "elohim-storage"
tags: [blob-hash, custody-blob, canonical-join-key, cid-first-migration, derive-class, elohim-storage]
relatedNodeIds:
  - "backlog-custody-blob-first-commitment-auto-producer"
  - "elohim/elohim-storage/src/db/models.rs"
  - "genesis/seeder/src/seed-commitments.ts"
shift_objective: |
  Write a bounded spec resolving the blobHash/serverBlobHash duality into ONE
  canonical join key for custody-facing joins, then implement it.

  Verified live (2026-07-30, cost hours to trace): content rows carry BOTH
  `blobHash` (client-computed) and `serverBlobHash` (server-computed), and these
  are frequently DIFFERENT values for the same row — observed live on
  elohim-host-landing: `blobHash` sha256-a12218a5…, `serverBlobHash`
  sha256-56adaebb…. The blob store, `shard_manifests.blob_hash`, and therefore
  the custody-facing fold (`derive_class`) all key by the SERVER hash. Any
  consumer that classifies or joins by the client `blobHash` silently never
  matches — a custody-blob pledge classified by the client hash stayed
  completely invisible to `derive_class`, which honestly reported class `none`
  with no error, no warning, just silence.

  Open questions for the bounded spec:
  - Why do the two hashes differ? Candidates: different normalization, different
    encoding, or literally hashing different bytes (declared/pre-upload bytes on
    the client vs bytes-as-stored on the server, e.g. after re-encoding,
    chunking, or a content-type transform).
  - Which surfaces expose which hash today (seed data, upload API responses,
    shard_manifests, the fold's classification inputs) — a full inventory, not a
    sample.
  - Canonical rule candidates to choose between:
    (a) `serverBlobHash` is THE join key everywhere until the CID-first blob-plane
        migration lands; `blobHash` is demoted to display/debug only.
    (b) The fold accepts either hash via an alias-resolution step (more
        resilient, more surface area to get wrong).
    (c) Fix the divergence at the upload path so the two values converge and the
        duality stops existing.

  Relates to the CID-first blob-plane migration arc (bare sha256 → bafkrei) named
  in the p2p-design-gate skill — whatever canonical-key rule is chosen here should
  not fight that migration's direction.
---

# blobHash/serverBlobHash duality has no canonical join key

## Evidence (verified live, 2026-07-30)

Content rows in this system carry two independently-computed hash fields:

- `blobHash` — computed client-side.
- `serverBlobHash` — computed server-side.

These are **not guaranteed equal**, and in practice frequently diverge for the
same row. Live example (elohim-host-landing): `blobHash` began
`sha256-a12218a5…`, `serverBlobHash` began `sha256-56adaebb…` — same content row,
two different hash values.

The blob store itself, `shard_manifests.blob_hash`, and therefore every
custody-facing fold that classifies or joins on blob identity, are keyed by the
**SERVER** hash. A commitment or manifest row classified by the **client**
`blobHash` therefore never matches anything in the blob-identity keyspace — the
join silently produces zero rows, and `derive_class` reports `none` with no
error surfaced anywhere. This is the same failure shape as a poisoned-key miss:
it looks like absence of data, not like a bug in the join key.

This cost real debugging hours this sprint precisely because the failure mode
is silent — no exception, no log line, just a fold that correctly reports "no
custody evidence found" against a row that in fact has evidence, keyed under the
other hash.

## Why this is `priority: high`

This is the same failure class already paid for once in this repo — a single
mismatched key silently emptying a fold/join rather than erroring
(`memory:project_epr_router_empties_on_poisoned_scope` is the closest prior
precedent: a poisoned scope row emptied `EprRouter`, cured by fail-closed
per-row handling; here the failure is a systemic key mismatch rather than one
bad row, but the "silently empties" signature is identical). Any future
consumer that classifies by the wrong hash reproduces this exact silent miss.

## Scope note

This entry is a design-readiness backlog item: the spec must be written and a
canonical rule chosen (a/b/c above, or a fourth option surfaced during
investigation) before implementation. The investigation should also produce the
"why do they differ" answer, since options (a)/(b) are workarounds while (c) is
the only option that actually removes the duality rather than routing around
it — the choice should be made with that trade-off explicit, not by default.

## Completion (2026-07-31)

Investigation disproved the entry's premise: the fields name different
artifacts, not two computations of one artifact. `blobHash` is the
primary/browser bundle; `serverBlobHash` is the optional SSR server bundle.
The bounded spec
`genesis/docs/superpowers/specs/2026-07-31-custody-blob-canonical-artifact-address-design.md`
therefore designates the exact `ShardManifest.blob_hash` as the custody join
key. Content lookup is now an explicit artifact-role resolver with no
cross-field fallback. Seeder coverage passes 38/38 focused tests.
