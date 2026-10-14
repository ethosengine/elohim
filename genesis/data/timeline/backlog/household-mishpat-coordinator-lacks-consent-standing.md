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

## 2026-10-13 — the storage half is built; the fixture half is not

`consent-speaks-refresh-is-a-timer-not-a-demand` now reads on demand by default, so a resting peer's carrier makes no
`my_consent_standing` call whatever its coordinator holds (the affirmation worker still reads standing once a minute
after start and then every 30 minutes). On demand this answer is asked for again at most every 30 s while something
asks. Under `ELOHIM_CONSENT_SPEAKS_REFRESH=timer` the refresher asks every 30 s as it always did; it says so once at
warn and then at debug. The household's installed mishpat coordinator is unchanged: until it is brought
up to the built one, no household peer lists an ask over the carrier in either mode. Not measured on the household.
