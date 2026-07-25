---
id: "backlog-eprfs-regression-recommitment"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A commitment stays 'discharged' forever after its first Produce — a regress-then-recover cycle after that never mints a new fulfilling event"
slug: "eprfs-regression-recommitment"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "envisioned"
priority: "medium"
relatedNodeIds:
  - "elohim/eprfs/epr-cli/src/flow/fulfill.rs"
  - "elohim/eprfs/epr-cli/src/flow/walk.rs"
  - "genesis/a2o/features/dataplane/resiliency-saga/"
tags: [eprfs, epr-rea, fulfill, regression, discharged-set]
---

Traced `fulfill.rs`'s `discharged` derivation while building `saga-status.py`'s combine logic
(T6): `discharged` is a `HashSet<Cid>` built from every event's `fulfills` field — and only
`Produce` events ever carry a non-empty `fulfills` (`Dismiss` events carry `fulfills: []` by
construction, verified against a real fulfill run). That means once a commitment is fulfilled
ONCE, `discharged.contains(commit_cid)` stays `true` forever, regardless of any later `Dismiss`.
Concretely, for a saga chapter that goes green → regresses (red, correctly dismissed) → is fixed
and goes green again: the THIRD run's `all_green` branch sees `discharged.contains(commit_cid) ==
true` and takes the `already_fulfilled` no-op path — **no new Produce event is ever appended for
the recovery**, so there is no evidence artifact distinguishing "green because it was always fine"
from "green again, second time, after a real regression." `saga-status.py` works around exactly
this by deriving its `regressed` state from **event ORDERING** (is the latest event — by
`occurredAt`, across both Produce and Dismiss — a Dismiss?) rather than trusting `discharged`
alone; that's the "walk discharged-set Dismiss awareness" this backlog title refers to, and it's
already correct in `saga-status.py` (verified against a synthetic fulfilled→dismissed fixture).

What's NOT yet fixed is the *evidence gap* in `fulfill.rs`/`walk.rs` itself: `epr flow walk`'s own
`discharged` set (`walk.rs:172`) has the identical blind spot, so the walk's LINEAGE section for a
regressed-then-recovered artifact would still just say "fulfilled" with no visibility into the
regression in between. Candidate fix direction (the "salted re-commitment" idea): after a
`Dismiss`, mint a fresh Commitment salted by the dismiss count or run id (`a2o:scenario-green#2`,
`#3`, ...) so a subsequent green run has a genuinely OPEN commitment to fulfill again, and `walk`'s
discharged-set becomes Dismiss-aware (a commitment whose latest event is a Dismiss is NOT
discharged, full stop) rather than permanently "discharged since first Produce."
