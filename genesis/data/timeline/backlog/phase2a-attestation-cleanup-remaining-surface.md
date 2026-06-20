---
id: "backlog-phase2a-attestation-cleanup-remaining-surface"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Phase-2a attestation cleanup — remaining surface (2 more dropped-table bugs + orphaned views + stale doc/code refs) for a focused cleanup sprint"
slug: "phase2a-attestation-cleanup-remaining-surface"
written: "2026-06-20"
author: "deprecated-attestation-path sweep (after the content_attestations migration + gate rebuild); cataloged for a deliberate cleanup sprint"
status: "open"
priority: "medium"
tags: [attestation-consolidation, phase-2a, cleanup, dropped-table, diesel-migration, deprecated, ts-rs]
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - genesis/data/timeline/backlog/content-attestations-table-dropped-but-still-consumed.md
  - elohim/elohim-storage/migrations/2026-05-12-100300_drop_legacy_attestation_tables/up.sql
---

# Phase-2a attestation cleanup — remaining surface

The Phase-2a consolidation (`34fcf1070`) dropped 23 legacy attestation tables but left consumers +
schema + docs referencing them. The `content_attestations` instance was migrated (gate rebuilt on
PREREQUISITE edges, `e413523ff`). A 2026-06-20 sweep found the rest of the surface — each item is its
own careful task; do them as a focused cleanup sprint (NOT a blind bulk delete — `content_attestations`
showed live consumers hide in unexpected paths).

## A. More dropped-table-still-consumed bugs (same pattern, real migrations)
Both DROPPED in migration `2026-05-12-100300` (applied via `embed_migrations`), still consumed:
1. **`gate_decision_attestations`** — 8 files: `signals.rs`, `http.rs`, `db/gate_decision_attestations.rs`,
   `db/mod.rs`, `db/governance.rs`, `db/models.rs`, `views_convert/qahal.rs`, `db/elohim_reputation.rs`.
   Migrate consumers to the unified `attestations` / `governance_actions` projection (`GateDecisionAttestation`
   → `attestation:gate-decision` per the design §6.2/§6.3), then delete the module/schema/models.
2. **`statement_votes`** — 2 files. Migrate to the unified `governance_actions` child-vote model
   (`attestation:statement-vote`), delete the dropped-table refs.
   ⚠ Each: confirm live consumers (the `content_attestations` STOP-guard lesson) before deleting.

## B. Orphaned ts-rs view types (frontend blast radius)
`ContentAttestation*View` types in `elohim/elohim-views/src/lamad.rs` — superseded by the unified
attestation views but kept (ts-rs + Angular consumers). Retire: replace consumers with the unified
`AttestationView`, regen ts (`cargo test export_bindings` + `schema:codegen:ts`), update the Angular
imports. Deliberately deferred from the gate rebuild (out of scope).

## C. Removed-entry-type references to triage (~10 files)
Storage files still referencing consolidated-away entry types (`HumanityWitness`, `KeyRevocation`,
`RevocationVote`, `IdentityFreeze`, etc.): `epr_atom_protocol.rs`, `epr_atom_service.rs`,
`reconcile/holochain_app_signal.rs`, `p2p/revocation_attestation_message.rs`, `lib.rs`,
`sensemaking/clustering.rs`, `p2p/recovery_revocation.rs`, `write_through.rs`, `reconcile/sweep.rs`,
`reconcile/signal_stream.rs`. MIXED: some are genuinely stale; some are LEGIT recovery-in-progress
(owned by `2026-05-15-recovery-m4-completion-shamir-optional-plan.md` — do NOT remove those). Triage
per-file: stale → remove; recovery-owned → leave for the recovery plan.

## D. Stale specs/plans to assess (per-doc)
12 specs/plans mention old per-type attestation types (`grep ContentAttestation|HealthAttestation|
GateDecisionAttestation`). MOST are generic/historical mentions (fine) or pre-consolidation specs
(historical). Assess each for whether it presents the OLD per-type model as the CURRENT design (stale,
needs a clarification banner like the 3 already fixed: the consolidation design, wave-0 plan, residual-tails
plan). Do NOT mass-edit — only banner the ones that would mislead a future session into re-planning.

**TRIAGED 2026-06-20:** the one current-design-claim — `2026-04-19-gate-challenge-and-indemnification-design.md`
(status Draft; describes a `GateDecisionChallenge` entry type targeting `GateDecisionAttestation` with
entry-type-level validation) — was bannered (GateDecisionAttestation → `attestation:gate-decision`; don't
re-introduce the entry type). The rest are GENERIC/ILLUSTRATIVE vocabulary mentions, NOT re-plannable
current-design-claims, left as-is: `recursive-architecture`/`escalated-architecture`/`elohim-sdk` (use
`GateDecisionAttestation` as an example of a witnessed/governance act), the `2026-06-14-substrate-passes`
vision batch, and `provenance-manifest-ingestion` (no real match). ONE PENDING: `elohim-peer-fabric-spine-plan`
references `HealthAttestation.response_time_ms` — but HealthAttestation's OWN consolidation fate (observation
layer vs `attestation:*`) is unresolved (design §6 open question), so leave until that's decided. §D DONE.

## Already done (2026-06-20, this session)
- `content_attestations` migrated + gate rebuilt (`e413523ff`); codegen `$ref` fix (`b78908924`).
- The 3 load-bearing planning docs banner-corrected (`9f84c0003`): consolidation design → Implemented,
  wave-0 → Stage-A-landed, residual-tails → Task-2-superseded.
