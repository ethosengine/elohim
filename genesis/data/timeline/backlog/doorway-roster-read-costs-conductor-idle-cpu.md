---
id: "backlog-doorway-roster-read-costs-conductor-idle-cpu"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Each doorway reads the whole doorway roster from its conductor about six times a minute; every read fetches 78 registration records for three live doorways and takes about 98 ms"
slug: "doorway-roster-read-costs-conductor-idle-cpu"
written: "2026-10-13"
author: "claude (overnight shift 2026-10-13T04-00, idle call floor and gossip round)"
status: "open"
priority: "medium"
severity: low
nodes: [household, alpha]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [doorway, federation, doorway-registry, infrastructure-zome, performance, perf-cpu, idle-is-free, phase-steady, lane-background]
cites:
  - doorway/doorway-service/src/services/federation.rs
  - doorway/doorway-service/src/main.rs
  - elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
  - genesis/data/timeline/backlog/doorway-registry-ttl-unenforced-no-heartbeat.md
---

**What.** With nobody using the household, each conductor answers `infrastructure::get_all_doorways` 30 to 34
times in a 310 s window. The caller is the doorway, not the storage peer, so these calls are outside the
`idle-is-free` call budget (which counts the storage peer's calls) and inside its CPU budget.

**Measured (household, 2026-10-13, production gossip cadence, the conductors' own `hc_*` series;
`genesis/a2o/reports/recovery/idle-call-floor-20261013/diff-fork3-prod-t0-fork3-prod-t1.md`).** Per conductor
over 310 s: 30 zome calls at a mean of 98 ms, 2,340 `get` host calls inside them (78 a call) and 30 `get_links`.
It is the largest wasm consumer on an idle conductor by a wide margin (166 M metering units, against 70 M for
the next function). Thirty calls at 98 ms is about 2.9 s of call time per conductor per window, roughly 0.57 s a
minute each and 1.7 s a minute across the household, where the at-rest lane reads about 21 s a minute against a
budget of 10. Call time is wall time; how much of it is CPU was not separated.

**Why 78 fetches for three doorways (from the code, not measured per link).** A doorway registers on every
boot, and each registration adds an entry and a link on the `__all__` anchor. `get_all_doorways`
(`infrastructure/src/lib.rs:619`) fetches the record behind every link and then keeps the newest per doorway
id, so its cost grows with the number of restarts the network has ever seen. The household's anchor carries 78
links.

**Why six a minute (from the code).** At least three doorway loops each read the roster on their own 60 s
timer: peer discovery (`main.rs:1690`), the peer-JWKS refresh (`main.rs:1767`, `federation.rs:1282`) and the
heartbeat's peer probe (`federation.rs:758`), plus the federation route. Which doorway is paired with which
conductor, and so the exact share per loop, was not traced.

**Candidates, none built.**

- One roster read per doorway per interval, shared by the loops that need it.
- Read less per call: the id is not in the link today, so the function cannot skip superseded registrations
  without fetching them. Carrying the doorway id in the link tag would let it fetch one record per id; that is a
  coordinator change (hot-swappable, no DNA hash move) provided the link type and tag validation allow it, which
  was not checked.
- Remove superseded links when a doorway re-registers.

**Relation to the existing entry.** `doorway-registry-ttl-unenforced-no-heartbeat` is about correctness of the
same read (no staleness filter). This entry is its cost. A redesign of the registration record should settle
both.

**Done when.** An idle household conductor shows `get_all_doorways` at no more than one call a minute per
paired doorway, each fetching no more records than there are live doorways, read from the conductor's `hc_*`
series.
