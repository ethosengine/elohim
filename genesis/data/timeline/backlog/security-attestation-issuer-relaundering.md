---
id: "backlog-security-attestation-issuer-relaundering"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "SECURITY: replicated attestations are re-authored under the LOCAL agent's key — issuer provenance laundered on every peer-discovered attestation"
slug: "security-attestation-issuer-relaundering"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design (discovered + measured by T5 soak-attestation rail)"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-release-soak-attestation-rail"
  - "backlog-attestation-projection-id-collision"
  - "habit:dataplane-convergence"
tags: [security, attestation, provenance, replication, red-team, dataplane]
---

**Trust-plane defect, measured live 2026-09-01: attestation provenance does
not survive replication.** Discovered by the T5 release-attestation work;
affects EVERY attestation authored through `content_store::issue_attestation`
— reach verification badges, device-health, and (once live) release
promotion evidence. **Requests a red-team pass beyond the bounded fix.**

## Mechanism (root cause located exactly)

`elohim/elohim-storage/src/reanchor_backfill.rs:51` —
`is_canonical_content_type` returns TRUE for `attestation:` content types,
so `p2p/projection_reconcile.rs` treats a peer-discovered attestation as
canonical content to re-anchor and **re-authors it through
`call_create_content` under the LOCAL agent's key**. Measured on the 3-peer
mesh: three probe attestations authored on matthew/james projected on
jessica with `issuer_cid = jessica` at distinct local ActionHashes, while
the row `id` still named the real issuer (the id/issuer disagreement is
itself the tell).

## Why it matters

An attestation's entire value is WHO attests. Re-authoring on replication
means any consumer reading the projection's `issuer_cid` sees the local
agent as issuer for every remote attestation — provenance laundering by
plumbing, not malice. C1-class machinery downstream (builder-exclusion,
earned promotion, badge rendering) silently degrades; T5's reader fails
closed against it (conductor link-walk + entry agreement required), which
is the interim mitigation, not the cure.

## Fix direction (design-touched — not a blind patch)

Attestations must replicate as **carried records** — the original signed
action verified in the receiving peer's own conductor (the
declare-carries-Record / `validate_carried_head_record` precedent from the
election plane) — or be excluded from re-anchoring entirely and left to DHT
gossip. Re-authoring under a different key is never correct for any
agent-signed kind; audit `is_canonical_content_type` for OTHER content
classes with the same property (anything whose author identity is
load-bearing).

## DoD

- A peer-discovered attestation projects with the ORIGINAL `issuer_cid` and
  verifiable provenance, or does not project at all (typed reason) — never
  a re-keyed row. Contract test pins it.
- T5's probe cross-peer leg: third peer counts 2 qualifying with
  provenance intact (`provenance_mismatched == 0`).
- Red-team review of the attestation replication surface recorded (scope:
  who can cause a foreign attestation to appear locally-authored, and what
  consumes `issuer_cid` today).
- The interim fail-closed posture in `release_attestation.rs` stays until
  this lands; remove the degradation flags only with the contract test
  green.
