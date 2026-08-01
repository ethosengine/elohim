---
id: "backlog-sync-surfaces-epr-and-doorway"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Two sync surfaces — per-EPR sync page and doorway-wide sync view, linked content→node"
slug: "sync-surfaces-epr-and-doorway"
written: "2026-08-01"
author: "saga limit-cycle shift (operator design note)"
status: "backlog"
priority: "medium"
jobs: [elohim, elohim-edge]
---

# Two sync surfaces — per-EPR and doorway-wide, linked

Operator design note (2026-08-01), captured verbatim in intent: there are at
least two distinct sync surfaces, and they should be linked.

## 1. Per-EPR sync surface (content-scoped)

What a visitor sees at `elohim.host/<epr>` when THIS epr isn't yet
synced/set-up for the doorway they arrived through. Today that moment is a
bare shed (`x-ssr-skipped` / catching-up 503) or an empty state. It should be
an honest progress surface: "this doorway is still acquiring/projecting this
content — x of y".

Existing primitives (mapped 2026-08-01, sync-status surface exploration):
- `pull` status on the wire is x-of-y ready: `{total, fetched, pending,
  failed, caughtUp}` (`PullStatusInfo`), nullable with null = "resolving",
  never "done".
- `PinProgressComponent` (app/elohim-app/src/app/elohim/components/
  pin-progress/) renders exactly this contract (bar, x/y, N failed + retry,
  ✓ serving, Resolving… for null total). Currently mounted only inside
  `epr-link` for an in-flight peer pin.

## 2. Doorway sync surface (node-scoped)

The doorway generally: what it is projecting and how far along, plus the
doorway's own dataplane drain. Doesn't exist as a page today — split across
the debug stability lens (booleans only) and Grafana (operator-only).

**MVP scope (operator, 2026-08-01): aggregate x-of-y ONLY — a count where x
visibly ticks up. No per-EPR item breakdown; finer resolution is a LATER
consideration.** This makes the MVP zero-new-wire: the aggregate numbers
already exist on `/p2p/status` (`drain` = `{total, published, pending}`,
`pull` = `{total, fetched, …}`, reconcile carries cumulative `healedTotal`).

Prereq (independently useful, smallest backend change): doorway `/health`
`P2PHealth` currently discards every numeric progress field at
doorway-service/src/main.rs:~551-560 (keeps only caughtUp/converged/
divergentAnchor). Additive serde pass-through of `pending/completed/exhausted`
+ `pull`/`drain` blocks unblocks every hosted-mode surface.

LATER (post-MVP): a "list projected EPRs with per-EPR sync state" wire
surface would be NEW — that design pass must run the p2p-design-gate (entity
class, DHT-vs-projection source of truth, identity) before any HTTP route is
proposed.

## 3. The link (content → node)

When viewing an EPR through a doorway, the per-EPR sync surface links to that
doorway's node-scoped sync view: "this page isn't ready → here's how this
whole doorway is doing." Navigation goes content-scoped → node-scoped;
no reverse dependency.

## The acceptance walk (operator, 2026-08-01 — the a2o story seed)

The whole MVP is one watchable journey:

1. Visit `elohim.host` → the per-EPR sync surface shows (this EPR's own
   acquisition state through this doorway).
2. It carries a doorway link → choose it.
3. Route to the doorway sync page → an aggregate progress bar where x of y
   visibly ticks up as the doorway drains.

That walk is the scenario the eventual `.feature` file should assert
(story-first: implementation is done when this journey passes; author the
scenario at design time, Opus-authored per a2o conventions, blind-reader
reviewed).

## Honest-denominator rule (from the 2026-08-01 limit-cycle RCA)

For the projection-reconcile stream, a naive `gaps`-based fraction oscillates
by construction (rotating inventory page sample — exactly how the limit cycle
stayed invisible). An honest reconcile bar is per-sweep progress + cumulative
`healedTotal`/adopted counters with `converged` (NOT `caughtUp`) as the finish
line. `pull`/`drain` blocks are stable-denominator and safe to render
directly. See backlog/content-gap-limit-cycle-blocks-convergence.md.
