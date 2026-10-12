---
id: "backlog-consent-speaks-refresh-is-a-timer-not-a-demand"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every node reads its own consent standing every 30 s whether or not anyone is asking: the device carrier's speaks refresher is a timer keeping a 120 s freshness bound warm"
slug: "consent-speaks-refresh-is-a-timer-not-a-demand"
written: "2026-10-12"
author: "claude-opus-5-5 (sprint 2026-10-12-idle-call-floor-sprint)"
status: "open"
priority: "low"
severity: low
nodes: [alpha, household]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [device-consent, carrier, mishpat, performance, perf-cpu, idle-is-free, trustful-self, friction-verify, plane-notary, unit-peer, phase-steady, lane-interactive]
cites:
  - genesis/docs/superpowers/sprints/2026-10-12-idle-call-floor-sprint.md
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

**What.** `services/device_carrier.rs` keeps "whom this node speaks for" in memory with a 120 s maximum age
(`SPEAKS_MAX_AGE_MICROS`); a refresher spawned in `HttpServer::serve` re-reads it with
`mishpat::my_consent_standing` every 30 s (`SPEAKS_REFRESH_SECS`), plus `my_human` when the node is a controller.
Measured on the household at rest 2026-10-12: 2.0 to 2.2 calls a minute per peer, on the interactive lane
(`call_zome_mishpat` has no classed variant), all day, on every node. That is a third of the whole `idle-is-free`
call budget spent holding one value fresh for an ask that may never come.

**What was tried and withdrawn.** The 2026-10-12 sprint built a slower idle cadence (first 80 s, then 60 s, then
45 s with a 15 s retry, short cadence kept while the carrier is in use) and withdrew it after three independent
review rounds. Each version was shown a schedule in which a node's evidence expires where the 30 s cadence's does
not: a later first read leaves less-fresh evidence when an outage begins, whatever the retry does afterwards. The
saving was under one call a minute and was paid for in the freshness the read exists to hold. The withdrawn diff
is kept at `genesis/a2o/reports/recovery/idle-call-floor-20261012/consent-pacing-withdrawn.diff` with the review
receipts beside it.

**The cure that does not trade freshness.** Read on demand. `Carrier::on_ask` is synchronous (it runs in the
transport's event loop) and reads the cached value; that is why the value must be kept warm. If the ask path can
await one single-flighted standing read, rate-limited to one per `SPEAKS_REFRESH_SECS` however many asks arrive
(the existing "never per incoming ask" bound, kept as a rate), the timer can stop and a resting node makes no
call. The 120 s bound then limits how stale an answer may be, as now, without a poll. The refresher and the
affirmation pass should read on the background lane either way, which needs a classed mishpat call.

**Constraints.** The freshness bound is a consent decision and is not this entry's to lengthen. A node whose
joining record no longer stands must still stop listing within it. `ensure_listed_proof` rides the refresher loop
and needs a home.

**Done when.** `household-at-rest.feature` shows no `my_consent_standing` call on a resting peer, and a device
ceremony scenario shows the first ask after an idle hour listed without a retry.
