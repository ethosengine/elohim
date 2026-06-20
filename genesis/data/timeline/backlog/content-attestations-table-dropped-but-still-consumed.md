---
id: "backlog-content-attestations-table-dropped-but-still-consumed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Phase-2a incompleteness: content_attestations table is DROPPED but still defined in diesel_schema + queried by live consumers (EPR head attestation reads)"
slug: "content-attestations-table-dropped-but-still-consumed"
written: "2026-06-20"
author: "surfaced during the attestation-consolidation residual-tails plan (Task 2 STOP-guard); the dead-code premise was wrong — it's a live-consumer incoherence"
status: "open"
priority: "high"
tags: [attestation-consolidation, phase-2a, content_attestations, diesel-migration, epr-head, runtime-incoherence, tiered-quilt]
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - genesis/docs/superpowers/plans/2026-06-20-attestation-consolidation-residual-tails-plan.md
  - elohim/elohim-storage/migrations/2026-05-12-100300_drop_legacy_attestation_tables/up.sql
  - elohim/elohim-storage/src/db/diesel_schema.rs
  - elohim/elohim-storage/src/epr_head.rs
  - elohim/elohim-storage/src/epr_service.rs
  - elohim/elohim-storage/src/api/attestations.rs
---

# content_attestations: dropped table still consumed (Phase-2a incompleteness)

The Phase-2a attestation consolidation (commit `34fcf1070`) dropped the legacy `content_attestations`
table — `migrations/2026-05-12-100300_drop_legacy_attestation_tables/up.sql:18` `DROP TABLE IF EXISTS
content_attestations;` — and consolidated content attestations into the unified `attestations` table
(`migrations/2026-05-12-100000_attestations`). The table is CREATEd only in `2026-01-08-000000_initial`
and never recreated after the drop, and `db/mod.rs:193` `embed_migrations!` runs pending migrations on
startup — so **at runtime the table is gone.**

**But the consuming code was never migrated.** 8 files still reference `content_attestations`:
- `db/diesel_schema.rs:742` — still declares the table (compile-time schema).
- `db/content_attestations.rs`, `db/models.rs` — the diesel module + `NewContentAttestation`/query fns.
- `epr_head.rs:127`, `epr_service.rs:136`, `http.rs:4686` — **LIVE consumers**: the EPR head response's
  `qahal.attestation_requirements` is read via `content_attestations::query_attestations_for_content`.
- `api/attestations.rs:105-186` — legacy POST/GET/revoke arms (the residual-tails plan's Slice 2 target).

**Runtime effect:** any query to `content_attestations` hits a dropped table → diesel error. Either the
EPR-head attestation reads are silently degrading (caught → empty `attestation_requirements`) or erroring.
Either way the EPR-head attestation surface is incoherent post-Phase-2a.

## Why this is NOT the residual-tails plan's Slice 2

That plan assumed the `api/attestations.rs` arms were isolated dead code. They are not — the SAME
dropped-table query exists on the EPR head read path. Deleting only the api arms would leave
`epr_head`/`epr_service` still querying the dropped table. This needs a coherent migration of the whole
consumer surface, not a deletion.

## Fix direction (its own task)

Migrate all `content_attestations` consumers to the unified `attestations` projection (the Phase-2a
replacement): repoint `query_attestations_for_content` at `attestations` (joined on `content_id`/subject),
update `epr_head`/`epr_service`/`http.rs` reads, delete the legacy `api/attestations.rs` arms + the
`db/content_attestations.rs` module + the `diesel_schema.rs:742` table decl + `models.rs` structs. Verify
the EPR-head `attestation_requirements` still populates from the unified table. Native storage work
(household-nodes testable); no DNA change. Scope: ~8 files. This completes the Phase-2a content-attestation
migration the consolidation left half-done.
