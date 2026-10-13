---
id: "backlog-household-mishpat-coordinator-lacks-consent-standing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The household's mishpat coordinator has no my_consent_standing, so every storage peer's consent read has failed every 30 seconds since at least 2026-10-11, and the coordinator drift check reports no drift"
slug: "household-mishpat-coordinator-lacks-consent-standing"
written: "2026-10-13"
author: "claude (overnight performance shift, operator-directed)"
status: "open"
priority: "medium"
severity: medium
nodes: [household]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [elohim-storage, mishpat, device-consent, coordinator-update, household-fixture, performance, perf-cpu, idle-is-free, friction-verify, plane-governance, unit-peer, phase-steady, lane-background]
cites:
  - elohim/elohim-storage/src/services/device_consent_cell.rs
  - elohim/elohim-storage/src/happ_manager.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/device_enrollment.rs
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

**What (measured, household, 2026-10-13).** Each storage peer's log carries, about 120 times an hour,
`device carrier: whom this node speaks for could not be read`, with the conductor's answer
`Attempted to call a zome function that doesn't exist: Zome: mishpat Fn my_consent_standing`. On james the first
occurrence in the retained log is 2026-10-11T23:23Z, the boot of that log. The ten `my_consent_standing` calls per
300 s that every `idle-is-free` call reading counts are these failures.

**Why it matters to the open consent-refresh question.** The household has never shown what the refresher costs or
does when the read succeeds. Any reading of "the refresher is a third of the idle call floor" is a reading of a
failing call.

**What is known about the cause.** The function exists in source (`device_enrollment.rs`, added 2026-10-05 in
`0284b142d`) and in the built `mishpat.wasm` (2026-10-06). The storage coordinator sweep reports the mishpat role
`drifted: false`, with installed and bundled hashes equal, and `bundle_source: persisted`. So the persisted bundle
the sweep compares against is as old as the installed coordinator. Why the persisted bundle is older than
`elohim/holochain/dna/elohim/workdir/mishpat.dna` was not determined.

**Not known.** Whether the alpha fleet's mishpat coordinator has the function. That needs a fleet log or metric
reading, which this workspace cannot take.

**Two separate cures, neither built.**

- Household fixture: bring the installed mishpat coordinator up to the built one (a coordinator hot-swap, no DNA
  hash move). This changes the standing household, so it was left for the operator.
- Storage: a "function doesn't exist" answer is durable until the coordinator changes. The refresher could rest on
  it and re-ask when a coordinator update lands, instead of every 30 s. This sits inside the consent-refresh
  invariant the operator has not yet ruled on, so it was not built.

**Done when.** A household call reading shows `my_consent_standing` answering, and `idle-is-free`'s ledger carries
what the refresher costs when it does.

## 2026-10-13: cause found, household brought up to date (operator-approved)

**Cause (verified).** A household storage peer runs without `--embedded-conductor`, so it has no boot bundle. Its
coordinator sweep measures against `<peer>/coordinators/last-applied.happ`, written 2026-10-05, before
`my_consent_standing` existed. Rebuilding a DNA repacks `workdir/elohim.happ`, and nothing hands that bundle to a
running household peer. The unpacked `last-applied.happ` and the built bundle carry different `mishpat.dna` bytes.

**Applied.** `POST /admin/coordinators/sync?apply=true&appId=elohim` with the built bundle, on each of the three
peers at 18:24Z. Mishpat hot-swapped on each (`appliedCount: 1`); nothing restarted. Receipts:
`genesis/a2o/reports/recovery/idle-call-floor-20261013/coordinator-sync-{dryrun,apply}-{8090,8091,8092}.json`.

**Result (measured).** `could not be read` failures per peer: 28 or 29 in the 14 minutes before, 0 in the five
minutes after. Matthew's log at 18:24:46Z: `device carrier: whom this node speaks for … An approval here is for
Matthew`, then `this node's key is proven over its transport id`.

**Left as found.**

- Only each peer's own app was swept. The hosted people's apps on the same conductors still run the older mishpat
  coordinator.
- The same dry run refuses the lamad role on every app: the built bundle's lamad DNA hash (`uhC0kT_R8…`) differs
  from the installed one (`uhC0kUQPS…`), an integrity-zome difference. The household's lamad cells are on an older
  DNA than the tree builds. That needs a reinstall or migration, not a hot-swap, and was not touched. Because of
  that refusal the apply did not record a new `last-applied.happ`, so the standing sweep still compares against the
  2026-10-05 bundle.
- Nothing delivers a rebuilt coordinator to a running household peer on its own. `just mesh` has no arm for it.

**Still owed for done-when.** A settled 300 s call reading with the read answering.
