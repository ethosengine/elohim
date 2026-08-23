---
id: "backlog-beacon-health-aware-shared-lane-membership"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "relay-addr-beacon: a leg contributes its A record to a shared lane only while its doorway is SERVING (/health/serving), leaving on shed with hysteresis — the first health-aware doorway-set DNS"
slug: "beacon-health-aware-shared-lane-membership"
written: "2026-08-23"
author: "fable-5 session 2026-08-23 (operator-requested Codex queue, doorway-federated continuity roadmap)"
status: "refined"
priority: "high"
area: "doorway/utility-plane"
domain: "protocol"
jobs: [relay-addr-beacon]
relatedNodeIds:
  - "habit:doorway-failover"
cites:
  - genesis/docs/superpowers/specs/2026-07-16-dual-wan-utility-plane-failover-design.md
  - genesis/a2o/features/dataplane/doorway-failover.feature
tags: [doorway, dns, logical-anycast, beacon, health, bounded-feature, codex-claimable, agent-agnostic]
---

# Health-aware shared-lane membership

**Why this exists.** Today failover for reads is purely client-side
(`doorwayFallbacks` in the browser interceptor); the beacon is DDNS for its
own leg and never removes a *shedding-but-alive* doorway from
`doorways.elohim.host`. The doorway already publishes the readiness contract
the a2o `doorway-failover` scenarios route on: `/health/serving` (status code
carries serving|shedding; `/health` on :8080 must never flip because it is the
k8s probe). This item makes the shared lane consume that contract.

## Scope (beacon crate only)

1. New flags: `--serving-probe-url <url>` (default unset = today's behavior),
   `--serving-probe-interval-secs` (default 15), `--serving-leave-after <n>`
   consecutive non-serving probes before withdrawing (default 3),
   `--serving-join-after <n>` consecutive serving probes before re-adding
   (default 2). Hysteresis both ways; never flap on one probe.
2. Membership state machine in `state.rs` (Serving → Withdrawn → Serving);
   when Withdrawn, the shared-lane PATCH removes/omits ONLY this owner's record
   (the ownership stamp in `sinks/cloudflare.rs` already scopes writes to the
   owner) and the exclusive `--record-name` lane is NOT touched (a withdrawn
   doorway must stay reachable by its own name for operators and for the
   sibling's client fallback).
3. Probe failure (connection refused / timeout) counts as non-serving; an
   unparseable 200 counts as serving (fail-open on ambiguity, fail-closed on
   silence — document the choice).
4. One structured log line per transition; a `/metrics`-style counter if the
   beacon has one, else skip.

## DoD / verification

- Unit tests with a mock probe (wiremock or a tiny hyper server): join/leave
  hysteresis counts; withdrawn state omits the owner record; exclusive lane
  untouched; probe timeout = non-serving.
- `cd relay-addr-beacon && RUSTFLAGS="" cargo test` → `EXIT=0` on its own line;
  clippy `-D warnings`; fmt.
- Proof on the fleet is an operator measurement (not this item): after the
  manifests gain `--serving-probe-url http://<doorway>:8080/health/serving`,
  `dig doorways.elohim.host` drops a shedding doorway within
  interval×leave-after seconds.

## Disjointness

Own crate. Sibling: `beacon-repeatable-shared-record-lanes`.
