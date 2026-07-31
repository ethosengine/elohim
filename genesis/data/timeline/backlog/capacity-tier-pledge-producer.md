---
id: "backlog-capacity-tier-pledge-producer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "capacity-tier pledge producer — no coordinator fn or HTTP lever authors a replicates-dwelling / commons-capacity commitment, so totalPledgedBytes is unreachable"
slug: "capacity-tier-pledge-producer"
written: "2026-07-31"
author: "doorway-federation-failover sprint planning (resiliency-saga ch9 documented residue)"
status: "done"
priority: "medium"
area: "elohim-storage"
tags: [replication-commitment, capacity-tier, totalPledgedBytes, mishpat, p2p-design-gate, resiliency-saga]
relatedNodeIds:
  - "backlog-custody-blob-first-commitment-auto-producer"
  - "backlog-blobhash-serverblobhash-duality-canonical-join-key"
  - "elohim/elohim-facings/src/folds/replication_commitment.rs"
  - "genesis/a2o/features/dataplane/resiliency-saga/README.md"
shift_objective: |
  Build the missing producer for CAPACITY-tier replication pledges, so a steward can
  pledge bytes (a `replicates-dwelling` commitment, or the `capacity` variant of a
  `replicates-commons` commitment) and `totalPledgedBytes` on the served
  `commitmentBackedReplication` object becomes reachable.

  Verified gap (resiliency-saga ch9 "Documented residue", 2026-07-29/30): only a
  capacity-tier pledge contributes bytes to the fold
  (`elohim-facings/src/folds/replication_commitment.rs:36-42, 182-194`); a
  content-tier commitment (`replicates-content` / `replicates-commons` content
  variant — the kind ch5 proves) names an EPR and pledges 0 bytes BY DESIGN
  (counted, not summed). No coordinator function or HTTP lever anywhere in the
  codebase authors a capacity-tier commitment. Ch9's a2o assertion was therefore
  deliberately narrowed to `commonsCommitments >= 1`; when this producer lands,
  widen it back to (or pair it with) `totalPledgedBytes >= 1`.

  Constraints:
  - **p2p-design-gate is MANDATORY in-session** — this mints a notarized Mishpat
    commitment (DHT-shaped write with consent/governance semantics). Follow the
    sibling entry `custody-blob-first-commitment-auto-producer` for the seam walk;
    the two should share the commitment-authoring path, not fork it.
  - Design order: coordinator fn → post-commit signal → storage projection → HTTP
    lever LAST. Commitment CID = entry_hash (never action_hash — the
    project_mishpat_commitment_cid_is_entry_hash class).
  - Classify blobs by the canonical join key the blobHash/serverBlobHash
    resolution designates (serve that entry FIRST if unresolved — it is the
    higher-priority sibling).
  - Acceptance: a2o — un-narrow ch9 (`09-projectors-carry.feature`) to assert
    `totalPledgedBytes >= 1` alongside `commonsCommitments >= 1`, landed in the
    same change as the producer (story-first: the widened assertion IS the spec).

  Disjointness: writes elohim-storage/facings + mishpat zome + one a2o feature;
  no overlap with the doorway-federation sprint write-set (doorway-service, a2o
  dataplane failover feature, doorway/infra manifests, imagodei zome).
---

## Why now

The doorway-federation/failover sprint (2026-07-31 plan) closes the resiliency saga's
doorway arc; this entry is the saga's last named residue on the capacity plane. It is
well-specified, disjoint from the sprint's write-set, and sized for a single focused
session — a canonical side-delegation package (Claude, Codex, or Gemini may claim it).

## Completion (2026-07-31)

The mandatory design gate reused the existing Class-A Mishpat Commitment entry,
generic `mishpat::create_commitment` coordinator, post-commit signal, and
per-commitment REA mirror. The new authenticated
`POST /api/v1/commitments/capacity` lever is an explicit stewardship act: it
validates the canonical typed capacity payload, injects the local steward's
agent CID, notarizes it, and eagerly projects both read models idempotently.
Chapter 9 again asserts `totalPledgedBytes >= 1` alongside
`commonsCommitments >= 1`. Focused Rust builder/validation tests and the storage
route-manifest guard pass; the Gherkin corpus parses cleanly.
