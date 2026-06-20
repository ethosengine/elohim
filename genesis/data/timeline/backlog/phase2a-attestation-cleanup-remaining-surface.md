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

## A. More dropped-table-still-consumed bugs — ✅ BOTH DONE (2026-06-20)
Both DROPPED in migration `2026-05-12-100300`, still consumed → migrated onto the unified `attestations` table:
1. **`gate_decision_attestations`** → `attestation:gate-decision` — ✅ DONE, commit `ce3ede44b`. Authoritative
   mapping (design §3/§6.2/§7.4 + the live DNA bridge `create_gate_decision_attestation`); active WRITER
   (`signals.rs`) + readers repointed; round-trip test (14 fields preserved); 39 tests pass; grep→0.
2. **`statement_votes`** → `attestation:statement-vote` — ✅ DONE, commit `b281e0900`. Authoritative mapping
   (live `create_statement_vote` bridge); latest-wins preserved (deterministic id `sv-{statement}-{voter}`);
   the load-bearing recount side-effect kept; 5 round-trip tests + full `--lib` 1695 pass; grep→0.

## B. `ContentAttestationView` retirement — OPEN (angular-architect task, NOT a quick deletion)
`ContentAttestationView` (`elohim/elohim-views/src/lamad.rs:259`) feeds generated TS (`storage-client-ts`)
AND a LIVE Angular surface — **not an orphan** (sized 2026-06-20): `ContentAttestationApiService`
(`app/elohim-app/src/app/elohim/services/content-attestation-api.service.ts`, `implements IContentAttestation`,
calls `/api/v1/attestations/*`) is registered in `app.config.ts`, exported from the `services/index.ts` barrel,
and referenced across interfaces/models. **First determine live-vs-dead:** the gate rebuild deleted the
`api/attestations.rs:105-186` content_attestations arms — are `/api/v1/attestations/*` still served by the
unified attestation routes (→ this is a TYPE migration `ContentAttestationView`→`AttestationView`) or gone
(→ retire the service + its consumers)? Then: regen ts (`cargo test export_bindings` + `schema:codegen:ts`),
update the Angular service/interface/barrel/`app.config.ts` provider. Hand to **angular-architect** — real
frontend blast radius, do NOT rush-delete (would break the app build).

## C. Removed-entry-type references to triage (~10 files)
Storage files still referencing consolidated-away entry types (`HumanityWitness`, `KeyRevocation`,
`RevocationVote`, `IdentityFreeze`, etc.): `epr_atom_protocol.rs`, `epr_atom_service.rs`,
`reconcile/holochain_app_signal.rs`, `p2p/revocation_attestation_message.rs`, `lib.rs`,
`sensemaking/clustering.rs`, `p2p/recovery_revocation.rs`, `write_through.rs`, `reconcile/sweep.rs`,
`reconcile/signal_stream.rs`. MIXED: some are genuinely stale; some are LEGIT recovery-in-progress
(owned by `2026-05-15-recovery-m4-completion-shamir-optional-plan.md` — do NOT remove those). Triage
per-file: stale → remove; recovery-owned → leave for the recovery plan.

**TRIAGED 2026-06-20 → LEAVE (recovery-owned).** Every hit is `KeyRevocation`/`RevocationVote`/recovery-signal
machinery (`epr_atom_service:291` `"KeyRevocation"=>`, `recovery_revocation`, `revocation_attestation_message`,
`sweep`, `signal_stream`, `holochain_app_signal`, `write_through:211`; the non-recovery names land only in
`recovery_witnesses.rs`/`signals.rs`/`models.rs` identity machinery). These are the active recovery/revocation
path the **recovery-m4 plan owns and is itself migrating** (RecoveryRequest→`governance-action:recovery-request`,
RecoveryVote→`attestation:recovery-approval`) — NOT stale dead-code for this sprint. §C = no action here;
folds into recovery-m4. ✅ resolved (leave).

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
