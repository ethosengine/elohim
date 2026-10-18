---
id: "backlog-conductor-startup-lifecycle-ownership"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor startup needs lifecycle ownership: a pending per-app startup attempt can still publish cells for an app disabled or uninstalled meanwhile, override reservations cover only the startup snapshot, and discarding a failed attempt's cells stops tasks by CellId"
slug: "conductor-startup-lifecycle-ownership"
written: "2026-10-18"
author: "autonomous sprint 2026-10-18 (controller ruling on the Codex review of fork branch fix/startup-per-app-publication)"
status: "open"
priority: "medium"
shift_objective: "push-delivers-within-budget station 2 — the deploy window closes within 120 s of caughtUp on the household"
jobs: [elohim-edgenode, elohim-edge]
tags: [conductor, fork, startup, cell-disabled, concurrency, lifecycle, deploy-window]
---

**Where this comes from.** The fork branch `fix/startup-per-app-publication` (base `96bcd2bee`) replaces
the serial, all-or-nothing enabled-app startup in `Conductor::initialize_conductor` with a bounded per-app
unit (`crates/holochain/src/conductor/conductor/app_startup.rs`: `APP_STARTUP_CONCURRENCY = 4`,
`startup_enabled_apps`, `startup_app`, `disable_app_after_failed_startup`, `P2pOverrideReservations`) and
publishes each app's cells to `running_cells` as soon as that app's joins finish. It exists because the
household's post-restart write-readiness exceeded the 120 s bound twice on 2026-10-17 while every cell
answered `CellDisabled`: apps started one at a time behind each other's kitsune2 `Dht::try_from_store`
walk (`fleet-standing-celldisabled-one-third-party-cell.md` traced that walk on 2026-09-22; the sargable-arc
indexes `e0bfc6c7a` shortened it, the serial loop remained).

**What the independent review found and what was done tonight.** The Codex review (gpt-6-astra, read-only)
returned SEND-BACK on three concurrency findings. The controller ruled them pre-existing race classes made
wider rather than new semantics, closed the common cases, and left the structural cure here:

| # | Residual race | Closed tonight | Structural cure (this row) |
|---|---|---|---|
| 1 | `DisableApp` / uninstall lands while an app's startup attempt is pending; the attempt publishes cells for an app that is no longer Enabled. Admin interfaces now open before ribosome loading, so the window is longer than before. | status re-read immediately before publication; a no-longer-Enabled app's cells are discarded instead of published | an attempt carries a lifecycle generation / ownership token; disable and uninstall invalidate pending attempts and drain their resources before deleting state |
| 2 | `P2pOverrideReservations` covers the startup snapshot only; a concurrent runtime `EnableApp` relies on the `create_cell` runtime check, so two apps can both pass before either publishes | documented at the type; the runtime check stays | one atomic reservation shared by startup and runtime cell creation, held through publication or cleanup |
| 3 | `discard_unpublished_cells` stops tasks by `CellId`; two apps sharing a cell id created concurrently can stop each other's tasks, and skipping an already-running id leaves the discarded instance's duplicate tasks alive. The check-and-stop now runs under one `running_cells` read, but the network `leave` for joined cells runs after the lock is released and can evict a replacement cell with the same id that joined meanwhile | documented at the function; shared cell ids across apps were already the duplicate-creation case | attempt-owned task handles, or exclusive per-cell lifecycle ownership |

Also open from the review: tests gate a real attempt at the attempt boundary, not a real network join, and
assert publication rather than task lifetime; a bound measured on a real conductor and a task-count accessor
on `TaskManagerClient` would close that.

**Done when.** A `holochain` lib test disables an app while its startup attempt is held at a real pending
join and observes no publication and no leaked tasks; startup and runtime `EnableApp` share one reservation
and a gated startup-versus-enable test passes; the three "Known limitations" lines in `app_startup.rs`'s
module doc are removed because they no longer hold.
