---
id: "backlog-custody-blob-first-commitment-auto-producer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "custody-blob first-commitment auto-producer — no runtime path authors the FIRST custody-blob commitment for a blob"
slug: "custody-blob-first-commitment-auto-producer"
written: "2026-07-30"
author: "agentic-developer"
status: "open"
priority: "medium"
area: "elohim-storage"
tags: [custody-blob, self-stewardship, distribute-shards, commitment, p2p-design-gate, mishpat]
relatedNodeIds:
  - "backlog-blobhash-serverblobhash-duality-canonical-join-key"
  - "elohim/elohim-storage/src/p2p/mod.rs"
  - "elohim/elohim-storage/src/services/self_stewardship.rs"
  - "genesis/seeder/src/seed-commitments.ts"
shift_objective: |
  Design and implement a runtime auto-producer for the FIRST custody-blob commitment
  on a blob, gated by the p2p-design-gate skill (mandatory — this mints a notarized
  Mishpat commitment, a DHT-shaped write with consent/governance semantics).

  Verified gap (this sprint): the custody plane's `stocked` class requires an ACTIVE
  custody-blob commitment naming the holder, but no runtime producer authors the FIRST
  commitment for a blob. `distribute_shards` / `self_stewardship` write only the
  evidence plane (`shard_locations`) plus manifests (`shard_manifests`, since the
  reconcile backfill `manifest_backfill_pass` landed in 37921b931). `salvage_pass`
  only ADDS replicas to blobs that already have custody-blob rows — it never mints
  the first one. The genesis seeder (`seed-commitments.ts`, resolving content's
  `serverBlobHash` at seed time since b495d81e4) covers seeded content only; nothing
  parallel exists for content uploaded at runtime.

  Design question to resolve in the gated session: should `distribute_shards`'
  self-selected placement branch (`elohim/elohim-storage/src/p2p/mod.rs` ~line 1922,
  the self-stewardship seam, alongside the existing call to
  `self_stewardship::record_self_held_shard`) also author a self-custody commitment
  — provider=receiver=self agent_cid, classified by the blob's SERVER hash (see the
  companion entry on blobHash/serverBlobHash duality; classify by whichever this
  sprint's canonical-join-key resolution designates as authoritative)?

  Constraints to weigh in the design session:
  - Consent/governance semantics: a node auto-pledging custody of its own uploads
    reads as natural self-stewardship, but it is still a DHT write minted without an
    explicit human act — the p2p-design-gate's identity/agency framing questions
    apply directly.
  - Idempotency: derive a deterministic commitment id from
    provider|receiver|blob (mirroring the seeder's approach) so re-running
    distribute_shards never double-mints.
  - Revocation/supersede path: when re-uploaded content changes the server hash, the
    old commitment must be revoked or superseded rather than orphaned pointing at a
    stale hash.
  - Answer the p2p-design-gate's four questions explicitly before any HTTP route or
    coordinator function is proposed: notarization class (A/A2/B/B2/C), whether a DHT
    entry type already exists for this shape, identity derivation (CID vs
    agent-composite vs slug), and which coordinator function creates it / which
    signal projects it.

  MUST pass p2p-design-gate before implementation begins.
---

# custody-blob first-commitment auto-producer

## Evidence (verified this sprint)

The custody plane's `stocked` class derivation requires an ACTIVE custody-blob
commitment naming the holder. Tracing every runtime producer that writes
custody-adjacent state for a blob:

- `distribute_shards` / `self_stewardship` (`elohim/elohim-storage/src/p2p/mod.rs`,
  `elohim/elohim-storage/src/services/self_stewardship.rs`) write the **evidence
  plane** (`shard_locations`) and, since the reconcile backfill in `37921b931`
  (`feat(storage): custody manifest + self-held evidence backfill`), the
  **manifest plane** (`shard_manifests`). Neither writes a custody-blob
  **commitment**.
- `salvage_pass` only **adds replicas** to blobs that already have a custody-blob
  commitment row present — it has no code path to mint the first one for a blob
  that has none.
- The genesis seeder (`genesis/seeder/src/seed-commitments.ts`), since `b495d81e4`
  (`feat(seeder): custody pairs resolve content's serverBlobHash at seed time`),
  authors commitments for **seeded content only** — a design-time fixture path,
  not a runtime one.

Net effect: any blob uploaded and distributed at runtime (outside the seeder) can
accumulate shard evidence and a manifest, yet never earn a custody-blob commitment,
so it can never classify as `stocked` no matter how well-replicated it actually is.

## Why this needs the design gate, not a quick patch

Minting a Mishpat commitment is not a side-effect write — it is a notarized DHT
entry with standing/revocation semantics (see
`memory:project_rea_compute_commitment_primitive`). An auto-producer that mints
commitments on every upload without a human-consent frame, or without an
idempotent/revocable shape, risks the same class of accidental-authority bug the
protocol's REA compute-commitment primitive was built to prevent. The
`p2p-design-gate` skill's four questions (notarization class, existing DHT entry
type, identity derivation, coordinator/signal pairing) must be answered before any
implementation, per CLAUDE.md's P2P Design Gate (MANDATORY) section.

## Scope note

This entry is a design-readiness backlog item, not an implementation ticket. The
`shift_objective` above is ready to paste into `/shift`, but the first session must
open with the `p2p-design-gate` skill invocation before any code is proposed.
