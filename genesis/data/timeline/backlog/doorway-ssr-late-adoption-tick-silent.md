---
id: "backlog-doorway-ssr-late-adoption-tick-silent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A doorway that boots before its SSR heads are declared never adopts them — the registry reconcile tick goes silent after its first pass, and warmup reports complete with servedBundleHeads []"
slug: "doorway-ssr-late-adoption-tick-silent"
written: "2026-10-19"
author: "2026-10-18 integration pass (wave 6 landing)"
status: "backlog"
priority: "medium"
jobs: [elohim-edge]
relatedNodeIds:
  - "habit:dataplane-convergence"
cites:
  - doorway/doorway-service/src/main.rs
  - doorway/doorway-service/src/render/registry.rs
  - genesis/a2o/features/dataplane/doorway-failover.feature
tags: [doorway, ssr, render, reconcile, household, cold-start, lesson]
---

**What is measured (household, 2026-10-18/19).** doorway-failover's step "a completed warmup on doorway
X has a servable head to show for it" failed on all three runs of the integration landing's binaries
(2355e99d8). Each time, the household had just cold-started, so every doorway booted before storage
answered and before the prologue declared the landing's `serverBlobHash`.
- Boot logged `SSR server bundle materialization failed … head not yet declared or storage unreachable;
  deferring this app to the reconcile adoption`.
- Then `SSR bundle-head reconcile tick started: every 300s`.
- Then not one `SSR bundle reconcile:` line over the next 35 minutes, although `main.rs` logs every
  completed tick at INFO and the adoption poll is 30 s.
- Both upstream circuits were closed (the tick's only skip path). Storage served the declared server
  blob (200, 9 ms) and the landing index (200).
- `/health/startup` kept `warmup.completed: true, servedBundleHeads: []`.

**Discriminator.** `just mesh doorway-restart a`, then `b`, on the SAME binary, with heads now declared.
Each doorway materialized `elohim-host-landing` at boot (status `current`, within 2–15 s), and
doorway-failover then passed 41/41 (`sprint-report-household-20261019T004309Z-b1e42ad8`). The scenario
had passed on 10-16 (11a0ce69) and 10-17 (a8d3e71a). On those runs a head was already materialized, and
the 10-17 doorway logs show a reconcile line every 30 s ("1 current (3 slugs checked)").
`render/registry.rs` and the `main.rs` tick are unchanged between a8d3e71a and 2355e99d8. So this is an
existing late-adoption defect that a cold start exposes, not that landing.

**Assertion (missing node).** Chain: dataplane-convergence. Between "doorway boots" and "doorway
serves its SSR head". Missing node: "a slug deferred at boot is adopted within one adoption poll
(30 s) of its head being declared". Probe: `servedBundleHeads` on `/health/startup`, plus one
`SSR bundle reconcile:` line per tick while `awaiting_adoption()`. Current state: red on every cold
household start.

**Next read.**
- Where `reconcile()` (registry.rs ~774) or its caller blocks once storage answers. Only one tick
  completed, while storage was still unreachable.
- Suspects: the adoption branch past the Stale arm (materialize + isolate build on `spawn_blocking`),
  and `declared_server_head` → `resolve_declared` with no timeout.
- A tokio-console or `RUST_LOG=doorway::ssr=debug` run on a cold household start answers which.

**Workaround until cured.** After a cold household start, run `just mesh prologue`, then
`just mesh doorway-restart a` and `b`, before any lane that reads `servedBundleHeads`.
