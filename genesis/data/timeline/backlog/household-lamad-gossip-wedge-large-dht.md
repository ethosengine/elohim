---
id: "backlog-household-lamad-gossip-wedge-large-dht"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Household lamad DHT gossip wedge at ~120k ops — full-arc gossip rounds don't complete inside roundTimeoutMs; fleet risk unconfirmed"
slug: "household-lamad-gossip-wedge-large-dht"
written: "2026-09-18"
author: "doorway-overnight-20260914 shift (doorway-failover pickup)"
status: "backlog"
priority: "high"
tags: [risk, conductor, holochain, kitsune2, gossip, household-mesh, scale, prologue]
cites:
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/final/household-gossip-wedge-notes.md
  - genesis/local-dev/preserved/pre-fresh-household-20260918T2235Z-lamad-gossip-wedge/
  - genesis/data/timeline/backlog/conductor-publish-livelock-fk787.md
---

# Household lamad DHT gossip wedge at ~120k ops — large-arc gossip cannot complete

## 1. The defect — measured 2026-09-18, household mesh (final binaries)

Full evidence: `genesis/a2o/reports/recovery/doorway-pickup-20260917/final/household-gossip-wedge-notes.md`.

- `dump-network-metrics` on the lamad DNA (`uhC0keuLMYBe…`, matthew/jessica/james): local ops
  matthew 119,914 / jessica 115,436 / james 115,009. Completed gossip rounds in 2.5h:
  matthew **None**, jessica **1**, james **2**, with **63–102 `peer_timeouts` per peer**.
  `new_ops_bookmark` frozen at 20:00–20:02Z.
- The other four DNAs on the same mesh (imagodei, infrastructure, node-registry, mishpat):
  **~1,500 completed rounds each, zero timeouts** in the same window — the wedge is
  lamad-DNA-specific, not a mesh-wide transport failure.
- `roundTimeoutMs` is already 60000 in the mesh conductor config — raising the ceiling isn't a
  live lever; the round doesn't finish, it doesn't merely run slow.
- `NoPeersForLocation("get")` ≈1,000 per 10 minutes, **constant since the very first 2026-09-18
  mesh start (17:31Z)**, vs ~20/hour on 2026-09-17 — a step-change, not a gradual decay.
- Sys-validation: "0 fetched of N missing dependencies" flat for 100+ minutes — not draining.
- The three conductors together burn ~5 cores (1.3–2.3 each), almost all in `sqlx-sqlite` worker threads; slow statements: scheduled-
  function delete (228), ops-to-publish select (38), publish count (33); `database is locked` on
  `integrate_dht_ops` (105 occurrences). cgroup `cpu.pressure` `some` ≈70%.
- No FK-787 (grepped, count 0) — this is a **distinct** defect from
  `genesis/data/timeline/backlog/conductor-publish-livelock-fk787.md`, not a recurrence of it.

## 2. Cause read

Four prologue runs (plus sweeps) on one household inflated matthew's lamad source chain to
`action_seq 26,821` and the lamad DHT to ~120k local ops. At that size, a full-arc gossip round
cannot complete inside 60s under the observed SQL saturation, so matthew's newest ~4.5k ops
(including the re-staged landing head) never propagate to jessica/james, and the conductor
cascade's `get` finds no selectable peer holding the head. **Prologue is not idempotent in DHT
cost**: each rerun re-authors the corpus rather than skipping content already anchored.

## 3. Fleet relevance — stated as unconfirmed risk

The alpha fleet carries the same lamad corpus at full arc. Whether the fleet's gossip completes
rounds under its current corpus size is **not measured here** — state as risk, not as a fleet
finding. Probe: run `dump-network-metrics` per DNA on an alpha conductor and compare
`completed_rounds` vs `peer_timeouts` for lamad against the other four DNAs, the same
discriminator used above.

## 4. Missing nodes (mintable)

- chain / between "mesh resumed" → "prologue" / missing node "lamad gossip completes rounds":
  assertion — a resumed household's lamad DNA advances `completed_rounds` with flat
  `peer_timeouts` before prologue re-seeds; probe — `dump-network-metrics` per DNA, pre-prologue.
  State: **unmet** (2026-09-18 run wedged at None/1/2 rounds).
- chain / between "prologue EXIT=0" → "lane preflight" / missing node "every peer's conductor can
  retrieve the staged landing head-record": assertion — all three storage peers (not only both
  doorways' happy-path instant) return 200 on `/db/content/elohim-host-landing/head-record`;
  probe — three-peer poll, not a two-doorway spot check. State: **unmet** (jessica 404s for 75s).
- chain / between "household resumed" → "prologue reruns" / missing node "prologue skips content
  already anchored": assertion — a resumed household's prologue does not re-author content whose
  head is already canonical; probe — action_seq delta across consecutive prologue runs on one
  household. State: **not built** (four runs → 26,821 action_seq).

## 5. Preserved state

Non-destructively preserved at
`genesis/local-dev/preserved/pre-fresh-household-20260918T2235Z-lamad-gossip-wedge/` for
comparison against a fresh household's lamad gossip behavior at low ops count.

## 6. Current decision

**Captured, not started.** Owner: next conductor/gossip shift. Cross-cite
`conductor-publish-livelock-fk787.md` (same household, same night, different mechanism — ruled
out as the cause here by a zero FK-787 grep). A fresh household sidesteps the amplitude (low ops
count) without curing the scale ceiling; the fleet-risk probe in §3 is the smallest next step.
