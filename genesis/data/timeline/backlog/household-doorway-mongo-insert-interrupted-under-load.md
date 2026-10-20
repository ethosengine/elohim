---
id: "backlog-household-doorway-mongo-insert-interrupted-under-load"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Household doorway A's MongoDB writes fail with 'Connection to 127.0.0.1:27017 interrupted due to server monitor timeout' under conductor load — a registration answers 500 DB_ERROR and is unwound"
slug: "household-doorway-mongo-insert-interrupted-under-load"
written: "2026-10-20"
author: "overnight shift 2026-10-20 (ci-runner-story-green)"
status: "open"
priority: "medium"
jobs: [household-mesh]
tags: [doorway, mongodb, household, flake]
---

**Evidence.** `genesis/local-dev/household-dowell/logs/doorway-restart-a.log` carries the same
driver error at 2026-10-16T20:20, 10-17T22:08 (twice), 10-17T22:21 and 10-20T04:52. On 10-20, three
of five ci-runner-stewarded lanes lost one `POST /auth/register` each to it (`500 {"error":"Failed
to create user: Database error: Insert failed: Kind: Connection to 127.0.0.1:27017 interrupted due
to server monitor timeout"}`), while the household ran five conductors (one at ~370% CPU, matthew's
at 9 GB) and the workspace RAM guard read HIGH. mongod itself was idle (139 MB, 0.8% CPU) and logged
nothing, which points at the doorway's driver monitor missing its heartbeat — a starved or blocked
doorway runtime — rather than at mongod.

**What holds today.** A service-agent registration whose insert fails is unwound
(`unwind_service_agent`, `auth_routes.rs`), so no cell leaks. Open edge: an "interrupted" insert may
have been applied server-side, leaving a row whose cell was just uninstalled.

**To decide.** Whether the doorway's register path should retry once on a pool-interruption error
(standalone mongod has no retryable writes), and whether provisioning work blocks the doorway's
async runtime long enough to starve the monitor. The ci-runner story's steps retry a sponsorship
once on `DB_ERROR` with a fresh identifier and log it, so the story measures the runner's
standing, not this flake.

**Same class, conductor side (2026-10-20 06:5x).** Right after a household conductor recycle,
`POST /auth/register` answered `503 PROVISIONING_FAILED: Failed to enable app on conductor-0 …
StateMutationError(Sqlx(Database(SqliteError { code: 5, message: "database is locked" })))`, and
every `uninstall_app` from doorway A timed out for ~20 min while the uninstalls still landed.
The doorway now settles a timed-out uninstall by re-reading `list_apps` (`provisioner.rs`
`uninstall_settled`) instead of keeping a ghost mapping that pinned the ceiling at 14/14.

