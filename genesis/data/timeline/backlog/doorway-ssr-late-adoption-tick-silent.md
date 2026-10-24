---
id: "backlog-doorway-ssr-late-adoption-tick-silent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A doorway that boots before its SSR heads are declared never adopts them — the registry reconcile tick goes silent after its first pass, and warmup reports complete with servedBundleHeads []"
slug: "doorway-ssr-late-adoption-tick-silent"
written: "2026-10-19"
author: "2026-10-18 integration pass (wave 6 landing)"
status: "resolved"
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

## Resolution (2026-10-24)

**Root cause.** The reconcile loop in `main.rs` skipped every pass while the warm-stream's private
`WarmStreamHealth` breaker read open for the reconcile upstream. Warmup records that breaker at a fixed
tick 0, so a cold-start trip never cools down; warmup can still complete through another peer, and the
skip logged only at DEBUG. `/health/startup` reports a different map (`state.upstream_breakers`), so
operators saw "closed". The tick was alive and silent, not blocked.

**Cure.** Fix `432049fe8` (branch `fix/ssr-late-adoption-reconcile-gate`): the tick gates on
`upstream_breakers.would_shed` (read-only), with the key trimmed the way the recorders trim it (a trailing
slash had read as an untracked, closed key). It skips at most one consecutive pass, so an unhealed
breaker cannot park adoption either, and it logs the skip at INFO. The startup line names both cadences.
Unit tests in `render::registry::tests`: `warm_stream_breaker_open_does_not_skip_reconcile`,
`authoritative_open_breaker_skips_with_normalized_key`,
`consecutive_skips_are_bounded_so_adoption_cannot_wedge`, `registry_without_reconcile_never_skips`.
Doorway gate: `cargo test --lib --bins` 1843 passed, clippy and fmt clean, rust-style debt 46 → 45.

**Proof on the household (cold start, no doorway restart).** All three doorways booted with storage
unreachable and deferred `elohim-host-landing` (16:51:09 / 16:51:18 / 16:51:26Z). The tick logged every
30 s. matthew's storage HTTP came up at 16:53:30.8Z. Doorway A observed the head at 16:53:39 and served
it at 16:53:43.7 (13 s later); B and C adopted at 16:53:51 and 16:53:58. `/health/startup`
`servedBundleHeads` = `elohim-host-landing` `status: current`. Then
`just test mesh features/dataplane/doorway-failover.feature` passed 10/10 scenarios, 41/41 steps,
`sprint-report-household-20261024T170625Z-f2c9ca61` (the report's SUT id is the main checkout's
commit; the doorway binary under test was built from `432049fe8`). The workaround above is retired.

