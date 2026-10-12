---
title: "Idle call floor sprint 2026-10-12 — the storage peer's own conductor calls at rest, caller by caller"
id: idle-call-floor-sprint-20261012
date: 2026-10-12
status: in-flight
author: "claude-opus-5-5 (owner); reader Explore; at the operator's direction"
habits: [idle-is-free]
cites:
  - "idle-cpu-residual-sprint-20261011 | the sprint that took the conductor half of idle-is-free and left the storage call budget, which this one takes | sha256:d079e41a35954c3f | path: genesis/docs/superpowers/sprints/2026-10-11-idle-cpu-residual-sprint.md"
  - genesis/data/timeline/backlog/performance-concern-index.md
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

# Idle call floor sprint 2026-10-12

This is a plan. Results go to the habit's own atom as a DELTA line, never here.

## Why this item

`idle-is-free` budgets a storage peer at 6 conductor calls a minute with nobody acting. The 2026-10-11 sprint
took the conductor side (scheduler latch, rolled) and named the gossip-round cost as the CPU residual, a fork
change with a fleet roll. The call budget is the half that is storage's alone: no fork, no roll, closable in a
day. The check has printed the same five callers since 2026-09-20b.

## The callers, as read from code (2026-10-12)

| Caller | Rate at rest | Driver | Settled-state memory before | Cure |
|---|---|---|---|---|
| `app_info` (ungated) | 13/min | four bridge supervisors pinging every 20 s, plus the heartbeat every 60 s | none: a socket that just answered a zome call is asked again | ask only when there is something to learn: any doubt (a wake, a closed streak, a role not `Live`, the shared admin read failing) pings at once; a healthy role rests on its last answer for 20 s doubling to 300 s; the heartbeat rests on its own write |
| `get_feedback_signal_refs_for_target` (interactive) | 8/min for 3N/8 minutes after every restart (N = 379 on matthew: 142 min), then 0 | 60 s sweep, 8 members a tick | heat ladder, in memory only: a restart makes every member Hot | retirement is recorded durably when a member goes Cold, per generation, and withdrawn by any re-arm; a restart seeds Cold from that record only; one Cold member is re-audited every five sweeps (the backstop a restart used to be); the sweep reads on the Background lane |
| `resolve_content_head` (background) | pointer audit: every declared row every lap (106 rows at 25 per 300 s, about 5/min, same in production) | 300 s tick | none: a row found in step is re-asked every lap | remember an in-step verdict against (declared head, blob pointer); the record at an action hash cannot change |
| `my_consent_standing` (interactive) | 2/min (4 when the node is a controller) | 30 s refresher keeping a 120 s freshness bound warm | none | NOT TAKEN. A slower refresh was built and withdrawn after three review rounds: any later first read leaves less-fresh evidence under an outage, so the saving (under 1 call/min) is paid for in the consent freshness it exists to hold. The real cure is a demand-driven read, which needs the carrier's ask path to await; the withdrawn diff is `consent-pacing-withdrawn.diff` in the evidence directory |
| `resolve_content_heads_local` (background) | 1 per reconcile sweep (2/min household, 0.2/min production) | anchor-verify at the tail of the heal leg | confirmed rows leave the set; a row the conductor cannot confirm is re-asked every sweep | remember a definite non-confirming answer against the anchor, on the miss ledger's dormancy schedule |

## Done-when

`household-at-rest.feature` reads at most 6 calls a minute on each peer with the household at production gossip
cadence, settled; refused permits 0. The CPU budget is not this sprint's (gossip-round cost, entry
`conductor-gossip-round-op-store-cost`), so the habit stays red and says why. Evidence under
`genesis/a2o/reports/recovery/idle-call-floor-20261012/`.

## Boundaries

- No budget is raised and no freshness bound is lengthened.
- A cure that trades detection latency says so where the constant is declared.
- Lands on `dev` validate-only with the serving receipt; no fleet roll is asked for.
