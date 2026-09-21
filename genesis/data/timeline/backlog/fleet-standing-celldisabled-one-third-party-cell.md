---
id: "backlog-fleet-standing-celldisabled-one-third-party-cell"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "One third-party cell on the alpha fleet has answered CellDisabled continuously since 2026-09-18 — background sweeps on four pods retry against it every minute, and it is NOT the class that broke app delivery"
slug: "fleet-standing-celldisabled-one-third-party-cell"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 pre-push fleet read"
status: "open"
priority: "medium"
jobs: [elohim-edge]
tags: [fleet, conductor, cell-disabled, projection-reconcile, feedback-projector, alpha]
---

**The fact.** Two different things on the alpha fleet answer `CellDisabled`, and a raw grep conflates them.

1. *A holder's OWN cell during its own restart.* Bounded and self-healing: over three restarts in 72 h, matthew's
   and adam's own-cell `CellDisabled` lines stop within 0–4 minutes of that conductor's `Conductor ready.` line
   (matthew ready 2026-09-20T12:49:32Z, adam 13:11:40Z; one later matthew reconnect took ~35 min), and neither has
   logged one in 13–19 h since. This is the class that failed app builds #1709, #1712 and #1714 — they start
   seconds after an edge roll and author through that window. Cured on the CI side by the readiness wait in
   `scripts/ci/stage-spa-blob.sh`; not this item.
2. *One third-party cell, standing.* `CellDisabled(CellId(DnaHash(uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt),
   AgentPubKey(uhCAkhsVVjkuw1D8zGPU0uRV3zuPIR-HIUX5J2CubFyAQikOt--Ks)))` — the lamad DNA, an agent that is neither
   matthew (`uhCAkJH75…`) nor adam (`uhCAkCUzKl3…`). Present across the whole 72 h scan, identical before and after
   every pod restart, from at least 2026-09-18T13:57Z through 2026-09-21T07:54Z. **This item.**

**Evidence.** Loki, namespace `elohim-alpha`, 2026-09-21T07:54Z, last 60 min: `elohim-eve-alpha-0` 133 lines
(`feedback_projector`: "discovery enumeration failed — the member yields its turn and is retried"),
`elohim-gertrude-alpha-0` 60 lines (~1/min, heartbeat cadence), `elohim-doorway-alpha-…` 89 and
`elohim-doorway-alpha-b-…` 99 lines; zero on matthew and adam over 6 h (772 453 lines scanned). On the storage
side the caller is `elohim_storage::p2p::projection_reconcile` ("conductor get failed; retry next sweep") while
reconciling `provide-<matthew|adam>-content:commons` commitments — the sweeps depend on the disabled cell, they are
not the disabled cell. Fleet serving is unaffected: both doorways' `/health/serving` are 200 with
`storageServing: serving`, and matthew and adam publish ops normally (23 637/23 637 and 46 625/46 625 at 07:51Z).

**Why it matters.** It is the "fleet CellDisabled since 2026-09-18" that earlier notes recorded as one standing
red, and reading it that way sent the app-delivery diagnosis the wrong way for three days (be16ffbc1 declared
every CellDisabled structural). It also costs four pods a failed conductor round-trip per sweep, forever, and it
will keep every CellDisabled alert ambiguous until it is named.

**Not determined.** Whose cell it is and on which conductor it is installed (Loki cannot say; the conductor's
admin app listing can), and whether its app is genuinely disabled with a reason or is a hosted cell whose
conductor never finished initialising it. Storage's per-role health probe (e7858cc02) observes a holder's own
roles, not a third party's cell, so it will not surface this one.

**Smallest next step.** Read the owning conductor's app listing for that agent key (operator-owned: admin
interface, not kubectl from the dev environment) — `disabled` with a reason names the cure; `enabled` with a dead
cell is a conductor defect worth a specimen. Independently, `projection_reconcile` and `feedback_projector` should
stop paying a conductor round-trip per sweep for a dependency that has answered the same way for days: back off
per cell, and count it in a metric so the standing case is visible without a log grep.

**Links.** `scripts/ci/stage-spa-blob.sh` (the bounded class's CI cure). `elohim/elohim-storage/src/p2p/projection_reconcile.rs`
and the feedback projector (the retrying callers). Evidence run: `genesis/a2o/reports/recovery/serving-edge-20260921/`.
Sibling: `doorway-registry-ttl-unenforced-no-heartbeat.md`.
