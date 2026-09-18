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
tags: [elohim-storage, sqlite, backpressure, deterministic-floor, flake, household-mesh]
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
