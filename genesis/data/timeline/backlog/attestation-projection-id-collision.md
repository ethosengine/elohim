---
id: "backlog-attestation-projection-id-collision"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Attestation projection PK collision — Content.id = attest-{kind}-{issuer} caps one row per issuer per kind across ALL subjects; plus the evidence_json key mismatch that empties the column"
slug: "attestation-projection-id-collision"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design (discovered by T5 soak-attestation rail)"
status: "complete"
priority: "high"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-release-soak-attestation-rail"
  - "backlog-security-attestation-issuer-relaundering"
tags: [attestation, projection, correctness, coordinator-hotswap, delegable]
claimedBy: "codex"
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

## Completion evidence — 2026-09-01

- `Content.id` is now the deterministic subject-bearing projection key
  `attest-{kind}-{issuer}-{subject}`; the canonical attestation identity remains
  the returned `EntryHash`. Issuance also creates the existing `IdToContent`
  index immediately, so a context-bearing read no longer depends on a later
  generic re-author pass.
- Both attestation projection readers now consume the coordinator's canonical
  `metadata.evidence_json` field. The storage regression suite pins populated
  evidence and two distinct rows for one issuer/kind across two subjects.
- DNA gates passed: schema parity, manifest hygiene (9/9), sweettest compile
  check, and the focused coordinator unit test
  `projection_id_distinguishes_subjects_for_one_issuer_and_kind`.
- The elohim-storage format and clippy legs passed; its 3,153 library tests
  passed with 0 failures, as did the subsequent integration inventory. The
  final doc-test compilation was blocked outside this task by the concurrent
  `release_adoption/mod.rs` declaration of a not-yet-present `apply.rs`.
- Household mesh receipt `release-soak-probe-1788294014323` passed in libp2p
  mode. One issuer retained two same-kind attestations for distinct subjects,
  both independently readable and SQL-projected with evidence. The third-peer
  read then saw 3/3 links, counted 2 qualifying independent issuers, excluded
  the builder, and reported `mismatched=0`, `unresolved=0`.
- Story-graph interstitial captured in the receipt: between “attestation
  issued” and “threshold reader resolves context,” issuance must create the
  subject-bearing `IdToContent` index. The live conductor read is the probe;
  no new entity, route, integrity type, or head-plane item was introduced.
