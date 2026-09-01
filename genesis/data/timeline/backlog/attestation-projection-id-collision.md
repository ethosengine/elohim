---
id: "backlog-attestation-projection-id-collision"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Attestation projection PK collision — Content.id = attest-{kind}-{issuer} caps one row per issuer per kind across ALL subjects; plus the evidence_json key mismatch that empties the column"
slug: "attestation-projection-id-collision"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design (discovered by T5 soak-attestation rail)"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-release-soak-attestation-rail"
  - "backlog-security-attestation-issuer-relaundering"
tags: [attestation, projection, correctness, coordinator-hotswap, delegable]
---

**Claimable by any implementation agent. Two bounded projection-correctness
defects in the attestation substrate, discovered and measured 2026-09-01 by
the T5 release-attestation work (atom `task-release-soak-attestation-rail`,
implementation notes). Both pre-date rung 5 and affect EVERY attestation
authored through `content_store::issue_attestation`.**

## Defect 1 — non-unique projection id (the cap)

`elohim/holochain/dna/elohim/zomes/content_store/src/attestation.rs:89`
stamps `Content.id = "attest-{kind}-{issuer}"` — no subject discriminator.
That string IS the projection's primary key, so one issuer can hold at most
ONE projected attestation row per kind across ALL subjects, forever: a
second attestation from the same issuer for a different subject silently
displaces the first in every peer's projection. The promotion-threshold
reader (T5) fails closed against this (link-walk + entry must agree), so
counts deflate rather than corrupt — but the projection is lossy for every
consumer.

**Fix class: coordinator-only (DNA-hash-NEUTRAL, rung-1 hot-swappable).**
Make the id subject-bearing (e.g. `attest-{kind}-{issuer}-{subject_cid}` or
derive from the attestation entry hash). Check every consumer keying on the
old shape before changing it (grep `attest-` across elohim-storage and the
zomes); the device-health bridge is the known live consumer.

## Defect 2 — evidence key mismatch (the empty column)

`elohim/elohim-storage/src/attestation_projector.rs:198` reads
`metadata["evidence"]` but the coordinator writes `metadata["evidence_json"]`
— the `evidence_json` column is `{}` on every attestation from this rail.
One-line class, but verify which side is canonical against the schema
contract before choosing which name moves.

## DoD

- Two attestations from one issuer for two subjects both project, on the
  authoring peer, with distinct ids; `evidence_json` populated. Unit or
  contract test pins each.
- T5's probe (`genesis/a2o/scripts/release-attestation-probe.ts`) rerun on a
  mesh: the third peer's `count_qualifying_attestations` reports 2 qualifying
  with `is_degraded() == false` on the id axis (the issuer-relaundering
  sibling atom still degrades the cross-peer axis until IT lands).
- MUST NOT touch integrity zomes, `generated_attestation_kinds.rs`, or the
  replication path (`reanchor_backfill.rs` / `projection_reconcile.rs` — the
  sibling security atom owns those).
