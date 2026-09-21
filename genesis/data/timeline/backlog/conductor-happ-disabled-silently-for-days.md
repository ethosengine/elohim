---
id: "backlog-conductor-happ-disabled-silently-for-days"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The hApp sat DISABLED on the conductors behind both public doorways for two days and nothing noticed — storage reads CellDisabled as proof the path is live, its liveness probe discards the status it is handed, and nothing ever calls enable_app"
slug: "conductor-happ-disabled-silently-for-days"
written: "2026-09-20"
author: "overnight pipeline shift 2026-09-20"
status: "open"
priority: "high"
jobs: [elohim, elohim-edge, elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
  - "habit:dataplane-convergence"
tags: [conductor, happ, cell-disabled, observability, self-heal, app-delivery]
---

**What is true (Loki, 2026-09-18 → 09-20, namespace `elohim-alpha`).** On matthew's and adam's conductors — the
backends of `alpha.elohim.host` and `elohim.host` — and on eve's, the installed hApp is DISABLED. Every supervised role
answers every zome call with `CellDisabled(CellId(DnaHash(…), AgentPubKey(…)))`: lamad/content_store
`uhC0kZezl4k2…GTOt`, imagodei `uhC0kRGwtzMN…AdFr`, infrastructure `uhC0k5a385O0…DHnFb`, node_registry
`uhC0k51dR_IB…Vqw`. First seen on matthew 2026-09-18T13:54:36Z, thirty-six seconds after the conductor pod booted: the
app came up disabled from the conductor's persistent state. No install / reinstall / disable line appears in storage's
logs in the 38 hours before it, so the trigger is unknown and is NOT a DNA-hash-moving roll by this fleet's own
happ manager. It has not healed in the 72 hours since.

**What it cost.** No head can be authored through either public doorway. App builds #1712 and #1713 failed at "Publish
and Verify App Delivery" — #1712 after 63 minutes, because `scripts/ci/stage-spa-blob.sh` read the 503 as a transient
shed and spent its full 360 s budget on each of eight combinations; neither build ever received a head action hash.
Genesis #1577's Eve failure is the same disablement on the imagodei role, not a separate incident. Each app failure
fails the orchestrator run, which holds the baseline, which makes the next timer run re-dispatch every pipeline —
including a multi-hour edge roll — and fail on app again.

**Why nothing noticed.** Four independent blind spots, each verified in code:
- storage serves content reads from its own SQLite projection, so the node looks alive;
- `conductor_bridge_health.rs` `classify_zome_error` / `is_transport_dead` lists transport failures only, so a
  `CellDisabled` error classifies as `ZomeObservation::PathLive` — evidence the path WORKS;
- the bridge supervisor probes with `HcClient::ping()` = `app_info()`, which succeeds on a disabled app, and the
  status it returns is discarded;
- `happ_manager.rs`'s drift check compares role presence and DNA hash, never the app's status, so a
  structurally-correct-but-disabled app is `DriftAction::NoOp` forever. Nothing in the crate calls `enable_app`.
Callers get a bare 503 (`services/response.rs`), indistinguishable from backpressure.

**Correction to this shift's own reading.** For a day the undeliverable app and the fleet's failing writes were
attributed to conductor saturation (the per-call capability-grant scan) and to a torn landing row. Both of those are
real and separately recorded, but neither is why no head could be authored: the cell was disabled. The lesson is the
one this item's cure encodes — read the error text before the status code.

**Cures.** (1) LANDED 2026-09-20, CORRECTED 2026-09-21: `stage-spa-blob.sh` stopped reading `CellDisabled` as a
transient shed — but calling it *structural* was wrong in the other direction. Measured 2026-09-21: a local-household
conductor accepts interface calls before cell initialisation finishes and answers `CellDisabled` for up to ~11 min
after start, then succeeds untouched (`enable_app` is a no-op for an already-enabled app); and on this fleet, across
three restarts in 72 h of Loki, matthew's and adam's OWN-cell `CellDisabled` lines stop within 0–4 min of that
conductor's "Conductor ready." line and have not recurred in the 13–19 h since. App #1714 started seconds after edge
#1470 finished rolling the fleet and spent its whole budget inside that window, as #1709 and #1712 had. So the script
now WAITS on a separate per-doorway-host readiness budget (`STAGE_CELL_READY_BUDGET_SECS`, default 2700 s = 45 min)
shared across every leg of one pipeline run, and once that budget is spent reports the measurement it actually has —
"still unavailable after N s", pointing at the conductor's own app/cell state — rather than a diagnosis it cannot
see from the far side of a doorway (`CellDisabled` means only that an installed cell is absent from `running_cells`,
conductor.rs:1663). That keeps #1712's 63-minute per-leg re-spend cured without losing the post-roll readiness race. (2) IN FLIGHT: storage sees a disabled app (probe reads the status; `CellDisabled`
classifies as not-live; gauge + reason logged once per transition), heals it (`enable_app` at boot and from the
supervisor when there is no structural drift, backed off 60 s → 1 h, never a reinstall, never across a closed-chain
fence), and says so (503 with a named cause and no `retryAfter`). (3) OPEN: find what disabled the apps on
2026-09-18 — if `enable_app` is refused, the conductor's refusal text is the diagnosis; if it succeeds, the trigger
still needs a look further back than the log window, because it can recur.

**Sharper, same day.** Two corrections from the code and one more log line. (1) `enable_app` does exist in the crate —
`happ_manager.rs:697`, inside `ensure_happ_installed` — but it has one caller, gated on `EMBEDDED_CONDUCTOR`, which on
the fleet is true only in the CONDUCTOR pod and runs once at its boot; storage, which probes every 20 s and sees every
failure, had no path to it. (2) That boot check DID run on 2026-09-18: at 13:54:05Z matthew's conductor pod logged
`App already installed … status Enabled`, thirty-one seconds before the first `CellDisabled`. So the app is ENABLED and
its CELLS are not running — a cell-level condition, which `enable_app` will most likely not lift. Cure (2) landed
2026-09-20: it sees the condition from the error text, says it, and logs the conductor's verbatim answer to the enable
attempt. HYPOTHESIS, not a finding: `authorize_signing_credentials` commits a capability grant, and the closed-chain
fence exists because a write after a chain is sealed is warranted into a cell block that Holochain 0.7 cannot lift;
if these conductors crossed a lineage seal and storage then minted a grant on the old chain, a permanent cell block
would look exactly like this — app enabled, every cell disabled, from boot, indefinitely. One read settles it: the
app's cell list and any warrant or block records on matthew's conductor.

**Done when:** a fleet roll carrying cure (2) shows `elohim_conductor_app_enabled{role}` = 1 on every peer, app +
genesis deliver from one push, and the trigger of the 2026-09-18 disablement is named.
