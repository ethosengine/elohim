---
title: "Operator Surface — Observe & Act"
id: operator-surface-observe-act
status: Draft
class: feature
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete OR superseded-by-implementation
created: 2026-07-05
maintainers: Matthew Dowell + Opus 4.8
cites:
  - actuatable-self-healing-control-plane-design | Actuation is never an admin API key — a bounded, audited, revocable REA action; reset_connection = bounded subsystem restart | path: genesis/docs/superpowers/specs/2026-06-13-actuatable-self-healing-control-plane-design.md
---

# Operator Surface — Observe & Act

An operator (a person running their own node) needs to (a) **see** what their runtime
is doing, and (b) **act** on it — bounce a runtime, push an update payload to one peer —
without `kubectl`. This spec is driven by a real incident: the shem-side anchor `adam`
drowned its conductor read-pool, the mesh could not converge, and the lamad SPA sat on an
opaque "syncing" spinner with no sense of *how much* was left. The surface turns that class
of incident into something an operator can diagnose and heal themselves.

**Principle.** The observe side is a *sensing* surface over backend-authoritative state — it
renders, it never dictates (backend truth-layer owns the numbers). The act side is *not* an
admin API key; it is a bounded, audited, revocable REA actuation (see v2).

Delivered in two phases so the visibility lands first: **you see before you act.**

## v1 — Visibility layer (SHIPPED)

No new backend. Two thin renderers over the existing `P2PStatusView` (`/p2p/status`) and
`SelfHealingView` (`/admin/self-healing`) contracts — separate bundles (`app/lamad` vs
`app/elohim-app`) so they share the *data contract*, not code.

- **Learner-facing sync strip** (`app/lamad`): replaces the boolean "Loading content…"
  spinner with an honest `fetched/total · N peers` bar over `pull` (fallback `replication`).
  Four honest phases — `waiting · syncing · ready · unreachable` — derived by a pure,
  unit-tested function. Non-blocking (content is never trapped behind it), self-hiding once
  the driving stream reports `caughtUp`, and — per the wire contract — a `null` `pull` reads
  as "keep waiting," **never** as done. Files: `services/sync-status.service.ts`,
  `components/sync-progress/*`, wired into `lamad-layout`.
- **Operator health view** (`app/elohim-app` `/debug` stability lens): the same contract, plus
  `provideLoop.{reanchorPending, reanchorFailed, reanchorCaughtUp}` — the exact metric that
  lights up when a conductor is thrashing already-anchored rows (adam's failure signature) and
  flips green when it heals. Extends the existing `StabilityLensComponent` (reuse, don't rebuild).

## v2 — Action layer (DESIGN DIRECTION — full pass runs the p2p-design-gate)

Two operator actions: **bounce** a runtime, and **upload an update payload** to a single peer.

- **Authority (the crux).** No new DHT entity is invented. Authorization reuses the existing
  REA compute-commitment primitive — a `Mishpat::Commitment` with the `delegates-compute`
  action — exactly as the self-healing control-plane spec mandates ("an actuation is never an
  admin API key"). The `bounce`/`reset_connection` precedent already exists in code:
  `ConductorManager::restart()` is wired behind a commitment-gated actuation endpoint; v2
  generalizes that gate to new capability names rather than adding an ungated `/admin/*` route.
- **Reachability & security constraints.** A new node-local route needs BOTH a match arm and an
  `is_service_path` entry, or the projection router shadows it to the SPA. And note: `DEV_MODE`
  is true on hosted alpha today, which leaves the seed/admin gate open and several `/admin/*`
  routes ungated — v2 must land its own commitment gate and must **not** lean on that dev gate.
- **Payload upload** is the real greenfield: today the hApp/config lifecycle is 100% pull-based
  (k8s initContainer `oras pull` + boot-time reconcile via `update_coordinators`/`install_app`).
  A single-peer push needs an authenticated upload → local write → in-process install path.

The full v2 design (entity classification, the commitment-gated route, the upload mechanism)
is its own gated pass; this section records the settled direction only.

## Out of scope

Substrate healing of the driving incident (the reanchor-backfill idempotency fix, corpus
purge, primary re-point) is tracked separately — it is what makes the *observe* numbers move,
not part of this surface.
