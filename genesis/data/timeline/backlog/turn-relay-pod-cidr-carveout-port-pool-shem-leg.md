---
id: "backlog-turn-relay-pod-cidr-carveout-port-pool-shem-leg"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "TURN relay pair: SSRF guard denied the cluster's own pod CIDR (fixed in-repo), 41-port relay pool exhausting, shem leg has never carried a session — two operator router actions remain"
slug: "turn-relay-pod-cidr-carveout-port-pool-shem-leg"
written: "2026-07-27"
author: "claude (networking check follow-up — coturn triage)"
status: "open"
priority: "high"
ci_status: open
jobs: [elohim-edge]
tags: [coturn, turn-relay, tx5, webrtc, ssrf-guard, allowed-peer-ip, port-pool, shem, port-forward, dual-wan, divergent-anchor]
cites:
  - genesis/orchestrator/manifests/infra/alpha-coturn-operations.yaml
  - genesis/orchestrator/manifests/infra/alpha-coturn-shem.yaml
  - self-heal-adam-projection-catchup-exhaustion-full-arc | adam post-restart catch-up exhaustion | path: genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
---

# TURN relay pair: three stacked faults; one fixed in-repo, two operator-owned

## The finding (2026-07-27 read-only networking check)

Cluster fabric ruled clean (WireGuard 0% loss, MTU coherent, DNS/Endpoints
populated, doorway→adam:8090 5–15ms, no NetworkPolicy hit, shem 45% CPU / 9%
mem). The P2P transport breaks at the coturn pair:

1. **SSRF guard blocked conductor↔conductor relay** — 226×
   `403: Forbidden IP` on CreatePermission for the cluster's own pod IPs
   (eve 50×, gertrude 44×, susan 39×, jessica 26×, james 19× — Calico
   `10.1.0.0/16`), because `denied-peer-ip=10.0.0.0-10.255.255.255` swallows
   the pod CIDR. Surfaces in adam as `Fail to refresh permissions:
   CreatePermission error response` → `tx5 send error (src: timed out)` →
   `Initiated round timed out`.
2. **Relay port pool exhausting** — `min-port=49160/max-port=49200` is 41
   ports; 11 of 17 recent ALLOCATEs returned `508: Cannot create socket`
   (65% failure), cascading into 134× `437 Invalid allocation` and 155×
   `438 Wrong nonce`. (Exhaustion is a strong inference, not proven — the
   alternative is a bind failure against the beacon-rendered external-ip.)
3. **shem leg has never carried a session** — coturn-shem up 4 days with 0
   session log lines vs 35,058 on coturn-ethosengine. DNS is correct
   (turn-shem.elohim.host → the beacon-detected WAN IP), so nothing on the
   internet reaches shem UDP/3478 — almost certainly no port-forward on
   shem's WAN router. The dual-WAN pair runs single-legged, concentrating
   all load onto fault 2's 41-port pool.

## Fixed in-repo (this commit)

Fault 1: `allowed-peer-ip=10.1.0.0-10.1.255.255` carved out of the denied
ranges in BOTH base ConfigMaps, plus a pod-template `conf-revision`
annotation so the pipeline apply actually rolls the pods (the beacon only
re-renders turnserver.conf on WAN-IP change).

Semantics verified against coturn 4.6.2 source (`good_peer_addr`,
ns_turn_server.c): whitelist is checked FIRST, unlisted peers are allowed by
default — `allowed-peer-ip` is an exception carve-out, never a global
whitelist. The `59fd85582` removal of `allowed-peer-ip` as a
"deployed-but-dead whitelist bug" assumed the opposite model; that original
`allowed-peer-ip=0.0.0.0-9.255.255.255` line was harmless (allowed a range
nothing denied), and removing it was not what fixed anything.

The household LAN (`192.168.86.0/24`, 23× denials incl. the router at .1)
stays denied deliberately: the TURN credential is static-in-repo, so
relaying to the LAN would expose the router admin surface to any TURN
client.

## Operator-owned remainder

1. **shem WAN router port-forward** (fault 3): forward `3478/udp+tcp` and
   `49160-49200/udp` to the shem node's LAN IP (DHCP-reserved), per the
   manifest header's deploy-time actions. Verify: coturn-shem session lines
   appear; ICE pairs form via turn-shem.elohim.host.
2. **Widen the relay port pool** (fault 2): pick a larger range (e.g.
   49152–49500), then change min-port/max-port in BOTH ConfigMaps AND the
   port-forwards on BOTH WAN routers in the same maneuver — the conf range
   MUST match the router forwards, so this cannot land repo-side alone.
   Re-measure ALLOCATE 508 rate afterward; if fault 1's fix eliminated the
   permission-failure retry churn, pressure may already be much lower.

## Open question for the dataplane conversation

adam's reconcile counters read `content_divergent_anchor: 3585` with
`content_missing: 0` — nothing is absent; 3,585 anchors disagree. Restoring
relay transport un-blocks gossip/declare traffic, but divergent-anchor
reconciliation may be a separate defect that transport alone won't clear.
Watch the counter after the relay fix lands; if it doesn't drain, route it
as its own finding (relates to the catch-up exhaustion in the cited adam
backlog item, which is a conductor-latency ceiling, not a transport fault).
