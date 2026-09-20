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

**Cures.** (1) LANDED 2026-09-20: `stage-spa-blob.sh` treats `CellDisabled` as a structural answer and fails in
seconds with a named cause. (2) IN FLIGHT: storage sees a disabled app (probe reads the status; `CellDisabled`
classifies as not-live; gauge + reason logged once per transition), heals it (`enable_app` at boot and from the
supervisor when there is no structural drift, backed off 60 s → 1 h, never a reinstall, never across a closed-chain
fence), and says so (503 with a named cause and no `retryAfter`). (3) OPEN: find what disabled the apps on
2026-09-18 — if `enable_app` is refused, the conductor's refusal text is the diagnosis; if it succeeds, the trigger
still needs a look further back than the log window, because it can recur.

**Done when:** a fleet roll carrying cure (2) shows `elohim_conductor_app_enabled{role}` = 1 on every peer, app +
genesis deliver from one push, and the trigger of the 2026-09-18 disablement is named.
