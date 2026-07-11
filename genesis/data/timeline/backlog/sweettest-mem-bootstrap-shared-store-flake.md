---
id: "backlog-sweettest-mem-bootstrap-shared-store-flake"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sweettest mem-bootstrap store is process-global (thread-id keyed) — every two_agent_conductors() cross-agent test is latently flaky; promote disable_bootstrap into the shared helper"
slug: "sweettest-mem-bootstrap-shared-store-flake"
written: "2026-07-11"
author: "substrate-cure sprint (DNA #1354 triage)"
status: "open"
priority: "medium"
area: "test-infra/sweettest"
domain: "developer"
jobs: [elohim-holochain]
cites:
  - elohim/holochain/tests/sweettest/src/common/conductors.rs
tags: [sweettest, kitsune2, mem-bootstrap, partition, flake, test-isolation]
---

# Sweettest conductors share the kitsune2 mem-bootstrap store — partition assumptions are false

Kitsune2's in-memory bootstrap (`kitsune2_core mem_bootstrap.rs`) is a
process-global `OnceLock<Mutex<HashMap<TestId, HashMap<SpaceId, Store>>>>`
keyed by `std::thread::current().id()` AT CONDUCTOR CONSTRUCTION and
space (DNA hash). Under `#[tokio::test(flavor = "multi_thread")]` two
`two_agent_conductors()` conductors frequently land on the same worker
thread → shared store → the second conductor's FIRST bootstrap poll
discovers the first's agent-info and gossip begins immediately. Any test
assuming "no connectivity until exchange_peer_info" is wrong — this
broke `earned_beats_newer_staging_at_resolve` deterministically in CI
(DNA #1354) and is a latent flake for every cross-agent sweettest whose
assertions are sensitive to early gossip.

Fix landed for the one failing test: `two_agent_conductors_isolated()`
(`SweetConductorConfig::standard().tune_network_config(|nc|
nc.disable_bootstrap = true)`) — the only inter-peer path is then the
explicit `exchange_peer_info`, the canonical partition-then-heal idiom.

## Follow-up (this item)

Audit the ~15 other `two_agent_conductors()` tests: which ones encode a
pre-exchange-isolation assumption? Consider promoting
`disable_bootstrap = true` into the SHARED helper (making explicit
`exchange_peer_info` the only wiring everywhere) — hardens the whole
class; needs a full sweettest suite run to confirm no test relies on
implicit bootstrap discovery.
