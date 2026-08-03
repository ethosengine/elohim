---
id: "backlog-adopt-before-author-evidence-starvation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Adopt-before-author is live but evidence-starved: e2e-* phantoms are 61% of the refusal population; the real-content remainder is ~100% responder budget_elapsed (C11 conductor saturation)"
slug: "adopt-before-author-evidence-starvation"
written: "2026-08-03"
author: "adopt-before-author flip session (post-flip adjudication)"
status: "backlog"
priority: "high"
tags: [dataplane, adopt-before-author, head-adoption, view-federation, c11-saturation, e2e-phantom-content, convergence, saga]
cites:
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/elohim-storage/src/p2p/view_federation.rs
  - elohim/elohim-storage/src/p2p/head_record_client.rs
  - genesis/data/timeline/backlog/resolve-canonical-election-get-links-deadline.md
  - genesis/data/timeline/backlog/susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors.md
---

# Adopt-before-author live 7/7 but evidence-starved — the exit needs supply, not wiring

## State (2026-08-03, flip enacted d9e8d9e76, edge #1299 deployed 7/7)

`ELOHIM_ADOPT_BEFORE_AUTHOR=true` live fleet-wide (per-pod boot WARN 14:19–14:24Z).
At +100min: `contest_failed{no_local_chain}` 3533 and climbing; `minted{source=
"adopt_before_author"}` NO SERIES; `adopt_refused` 0 (pre-touched); the zome
bypass has never been asked once.

## Root cause (rust-architect read-only trace, all confirmed live)

The arm is SAME-CALL and fires on every refusal — it ran all 3533 times and
exited at its first line (`head_adoption.rs:1613` `let bytes = bytes?;`).
Every view-federation answer for this class is `Present` **hash-only**
(`record: null`): the advertiser serves `head_action_hash` from SQL, then its
own conductor cannot yield the record inside the 5s responder budget
(`view_federation.rs:486`, `HEAD_RECORD_CONDUCTOR_TIMEOUT` — compile-time
const, build-asserted strictly below the requester's 10s; deliberately NOT
env-tunable). Flag plumb clean; zome landed-and-untested-by-traffic, not broken.

Live adjudication of the two sub-causes, both confirmed as a SPLIT:

- **~61% phantom**: 2499/4115 refusal log lines are `e2e-*` content ids —
  the known phantom class (2026-07-26 acquisition-pull diagnosis). No bytes
  exist anywhere; PERMANENTLY unmintable; they cycle contest budget + backoff
  ledger forever.
- **~39% C11-starved**: real-content refusals ≈ 1616 vs
  `head_record_degraded_total{cause="budget_elapsed"}` = 1615 — a near-exact
  match. The bytes EXIST on the advertiser; its saturated conductor times out
  serving them. Same C11 DB-pool-saturation class as
  `resolve-canonical-election-get-links-deadline.md` (4 read permits, 10s
  semaphore; `adopt_sweep{budget_elapsed}` corroborates).
- `"carried":true` count over 2h: **zero** — not one contest ever had bytes
  in hand.

## Levers (in leverage order)

1. **Phantom hygiene**: exclude/tombstone `e2e-*` ids from the contest corpus
   (they are 61% of the refusal load and 0% of the convergence value). Data
   hygiene on content rows, operational class C — biggest single supply win.
2. **C11 relief** (the standing wall): conductor read-permit widening / pod
   CPU / sweep pacing so responders answer inside the budget. Owned by the
   existing C11 items — this entry adds the adopt-evidence path as a fourth
   consumer starved by the same pool.
3. **Instrumentation gap** (mechanical, ride next storage wave): the
   `Ok(Ok(None))` hash-only cause at `view_federation.rs:490` is the ONLY
   uncounted branch — precisely the structural one. Without it, phantom vs
   C11 is only separable via the `carried=` INFO log line, not from meters.

## Addendum 2026-08-03 (same day): levers 1+3 landed as one storage wave

Implemented (Opus rust-architect, adversarially reviewed by Sonnet code-reviewer,
SHIP verdict): `record_absent_reason` threaded through the ContentHeadRecord
answer (additive, mixed-fleet-compat proven — map-keyed rmp_serde, payload
travels as opaque Value, no deny_unknown_fields, byte-identical carried shape,
typed `Unknown` fallback so future vocabulary can't brick decode); responder
counts the formerly-uncounted no-record branch; requester classifies evidence
and records an **evidence-absent long-backoff class** — the content-agnostic
phantom-hygiene mechanism (lever 1 re-shaped: deferral-not-deletion,
self-healing on re-admission) — plus `adopt_evidence{state}` pre-touched
(lever 3 closed).

Reviewer's one MODERATE caveat, accepted and mitigated: a clean conductor
no-record can also mean not-yet-gossiped (full-arc law), so a real,
still-converging id could catch the long deferral. Mitigation: code default
86400s but alpha bakes `ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS=21600` (6h,
genesis-declared in the human manifests); watch
`adopt_evidence{state="no_record"}` vs later `carried` recoveries before
raising; `=0` restores the ordinary 3600s window exactly. Bridge-less
responder arm deliberately answers no reason → ordinary window (never the
long class without an actual conductor ask).

Still open here: lever 2 (C11 conductor saturation — permits/CPU/pacing),
now measurable in isolation once the phantom load drains; and the HTTP twin
(`GET /db/content/{id}/head-record`) not carrying the reason field (flagged,
non-blocking).

## Verdict for the saga

The flip is correct and behaviorally proven; convergence stays blocked on
SUPPLY (phantom noise + C11), not on the exit's design. The recording gate
(converged on A) will not open from the flip alone. Fix disposition:
storage-only + conductor ops — no DNA wave.
