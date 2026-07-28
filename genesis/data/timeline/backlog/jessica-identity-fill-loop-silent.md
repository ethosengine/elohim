---
id: "backlog-jessica-identity-fill-loop-silent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "jessica emits no identity_fill WARN loop while all other pods poll at ~5min cadence — loop dead or mis-scheduled?"
slug: "jessica-identity-fill-loop-silent"
written: "2026-07-28"
author: "heads-converge-truthful-resilience shift"
status: "open"
priority: "low"
tags: [identity-fill, imagodei, alpha, observability]
cites: []
---

# jessica identity_fill loop silent — anomaly, unexplained

Observed 2026-07-28 (2h Loki window, quoted-evidence sweep): every alpha pod
except jessica emits the periodic WARN
`identity_fill: discovery found zero household cids (no memberships on DHT or
projection) — nothing to fill` at ~5min cadence (adam≈23, eve≈21, gertrude≈20,
james≈24, matthew≈24, susan≈17 in 2h). jessica: 0 WARN hits (one unrelated
INFO substring match only).

Either jessica's identity_fill loop is not running (scheduler/boot wiring), or
it is succeeding silently (unlikely — there is nothing to find fleet-wide), or
its log target/level differs. Untriaged; needs jessica's raw log tail scoped to
`elohim_storage::services::identity_fill` to confirm the loop is scheduled.

Status: OPEN (investigate). Small; good first probe for any runtime-triage
pass touching identity coherence.
