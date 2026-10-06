---
id: "backlog-storage-sqlite-locked-surfaces-as-500-despite-busy-timeout"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Storage answers 500 'database is locked' on a head-moving PATCH during boot contention — the 30 s busy_timeout does not cover it"
slug: "storage-sqlite-locked-surfaces-as-500-despite-busy-timeout"
written: "2026-09-18"
author: "doorway-overnight-20260914 shift (doorway-failover pickup)"
status: "backlog"
priority: "medium"
tags: [elohim-storage, sqlite, backpressure, deterministic-floor, flake, household-mesh, performance, perf-io, trustful-self, friction-mechanical, plane-projection, fused-planes, unit-call, phase-transition, lane-interactive]
cites:
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/final2/laneD-deliverability-receipt.log
  - genesis/data/timeline/backlog/household-mesh-harness-honest-readiness.md
---

# Storage answers 500 "database is locked" during boot contention

## What was measured (2026-09-18, storage source 0ff361417)

Serving T2 (`epr-app-deliverability.feature`) passed 5/5 at 22:51Z and failed 4/5 at 23:33Z on the
same binaries and household. The failing step, `the incoherent bundle is declared the new version
anyway` (`epr-app-deliverability.steps.ts:1368`), sends one `PATCH /db/content/<slug>` and got

`500 {"error":"Internal error: Update anchor failed: database is locked"}`

Matthew's storage logged 28 `database is locked` lines between 23:29Z and 23:32Z — the minutes
after a storage restart (the lane's own restart scenario plus this run's boot), while the
InfrastructureSignal subscriber, custody backfill, shard-manifest backfill and REA projection were
all writing. The other failures in that window were internal (`apply_delta (held page) failed`,
`ReaProjectionSignal projection failed`); this one reached a client.

## Why the existing guard does not cover it

`db/mod.rs:275-291` sets `PRAGMA busy_timeout = 30000` first on every pooled connection, and the
comment states the intent: absorb the overlap window "without surfacing SQLITE_BUSY to clients".
The error arrived in well under 30 s. The consistent reading — not yet confirmed by a trace — is a
write attempted inside a transaction that began as a read: SQLite returns BUSY immediately when a
deferred transaction tries to upgrade and another writer has moved the WAL, and busy_timeout does
not apply to that case. The anchor update at `db/content_diesel.rs:~1184` runs after earlier reads
in the same handler.

## Missing node

chain / between "storage restarted" → "storage answers a head-moving write" / missing node "a
contended write waits or sheds honestly": assertion — a PATCH that meets lock contention either
succeeds within the busy budget or answers the retryable shed the rest of the dataplane uses
(503 + `Retry-After`), never a 500; probe — drive PATCHes during a storage boot and count 500s.
State: **unmet**.

Candidate cures, to be chosen by whoever takes it: begin head-moving writes with an immediate
(write-intent) transaction so busy_timeout applies; or map `database is locked` to the existing
shed answer at the HTTP boundary. The first is the floor fix; the second is honest labelling.

## Current decision

**Captured, not started.** Waiting for the boot sweeps to go quiet before running the lane is the
measurement condition the household receipts use; it is not a cure.

**RECONCILED 2026-10-02** (shem, fork 2b334df7973d, superproject 4a80267f3, code read only, nothing measured): PRESENT — both head-moving writes run in a deferred transaction that reads before it writes, and nothing maps the error to a shed; the upgrade theory stays unconfirmed by trace. elohim-storage/src/db/content_diesel.rs:1416-1509,2197-2418; db/mod.rs:285. Same mechanism as: the locked head stamp in conductor-cap-grant-scan-per-zome-call; the locked page in inventory-refresh-pages-dropped-as-gaps. Confirming measurement: SQLite extended error code 517 at the two sites, or time-to-500 well under 30 s.

**Design note 2026-10-02** (shem, superproject 4a80267f3, code read only, nothing built or measured): on the author path both head-moving writes run nested inside an outer deferred transaction (`elohim-storage/src/services/content_service.rs:869-875`), so `BEGIN IMMEDIATE` would have to go on that outer transaction; switching the inner calls fails with `AlreadyInTransaction`. Storage has one shared pool (default 10) for reads and writes, WAL, busy_timeout 30 s (`db/mod.rs:285-286`). Diesel reports SQLITE_BUSY and BUSY_SNAPSHOT alike as `DatabaseErrorKind::Unknown` with only the message, and storage maps it to 500 (`services/response.rs:262`). The REA `Update anchor failed` / `Insert failed` lines run autocommit with no transaction, so the upgrade theory does not explain them and IMMEDIATE would not cure them; they imply a full 30 s wait. Proposal (outer IMMEDIATE on the author path, read-then-IMMEDIATE-then-recheck for standalone stamps, a 503 shed for the busy class, a two-connection WAL test that needs no mesh): `genesis/local-dev/perf-deep-dive/storage-begin-immediate-design.md` (gitignored, on shem). Not yet routed or decided.

**DELTA 2026-10-06** (integration worktree, storage c34863dc6, measured on a fresh three-peer mesh): the upgrade theory is confirmed for the stamp, and that one site is cured. `features/dataplane/epr-app-deliverability.feature` failed the same step on two consecutive runs: `POST /db/content/{slug}/canonical-head` answered `500 Stamp declared head failed: database is locked` about 100 ms after the PATCH that precedes it, far inside the 30 s busy timeout, while the projection of that PATCH was writing the row. `stamp_declared_head_witnessed` now opens an immediate transaction when it is the outermost one and a savepoint when a caller already holds one (so the author path's outer deferred transaction is unchanged, and still exposed). After the change: 5 of 5 stations, 102 of 102 steps; storage gate 5,319 passed. Still open: the same shape on the background apply paths (`apply_snapshot`, `apply_delta`, feedback projector, ReaProjectionSignal), which logged `database is locked` on all three peers right after seeding and retried; the outer author transaction; and mapping the error to a shed instead of a 500.

**DELTA 2026-10-06 (second)** (integration worktree, uncommitted at writing): the stamp's helper is now ONE shared helper, `db::read_then_write_transaction` (`elohim/elohim-storage/src/db/mod.rs`; generic over the error type, `BEGIN IMMEDIATE` when outermost, savepoint when nested), and every read-then-write transaction this entry names takes the write lock up front: `peer_blob_inventory::{apply_snapshot, apply_delta, record_fetch_success}`, the feedback projector's `commit_group` and `publish_generation`, `content_diesel::{upsert_with_anchor, bulk_create_content, heal_election_columns}` (and the head stamp), the author path's outer transaction `content_service::project_own_commit`, `peer_statuses::upsert_if_newer`, `shard_locations::apply_peer_announced`, `peer_map::{record_libp2p_observation, record_iroh_observation}` and `relationships_diesel::bulk_create_relationships`. All are synchronous diesel closures, so none can hold the lock across a conductor `.await`; called inside a caller's transaction they become a savepoint and change nothing. Write-first transactions cannot lose an upgrade and were left deferred: `begin_rebuild`, `rebuild::start`, the evaluator re-generation, `rea_commitments::{mark_superseded, create_with_supersession}`, and `economic_events::bulk_record_events` (its `record_event` inserts before it reads the row back). A lock loss that still reaches the HTTP boundary is now classified by marker (`db::is_sqlite_lock_loss`: "database is locked" / "database table is locked") in both `services::response::error_response` and `HttpServer::escaped_error_response` and answers the existing catching-up shed (503 + `Retry-After: 2`, `too_many_requests_with_retry`) instead of 500. Regression tests, each on a temp WAL file database with two connections: `db::lock_upgrade_tests::deferred_read_then_write_loses_the_lock_upgrade_at_once` pins the failure (a deferred transaction that read before another connection committed is refused at once, inside the busy timeout, and only the other write lands); `db::lock_upgrade_tests::read_then_write_transaction_makes_the_other_writer_wait` runs the same interleaving through the helper (the other writer waits, both writes land); `db::lock_upgrade_tests::nested_call_is_a_savepoint_not_a_second_begin`; `db::peer_blob_inventory::tests::snapshot_apply_waits_for_a_concurrent_writer_instead_of_losing_the_lock` drives the real `apply_snapshot` against a connection holding the write lock; `services::response::tests::sqlite_lock_loss_is_shed_with_retry_after` pins the 503 + `Retry-After` mapping. The storage gate is green with these (and gained a `clippy-iroh` leg). Still open: the cure is unmeasured on the mesh — the post-seed `database is locked` log lines on the three peers, and the deliverability scenario's canonical-head step, are the probe. Status stays `backlog` until that receipt exists.
