---
id: "backlog-shem-conductors-signal-hairpin-suspect-dht-silent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "All four shem-hosted conductors DHT-silent since the 2026-07-28 DNS flip — signal.elohim.host hairpin suspected; operator probe + restart sequence"
slug: "shem-conductors-signal-hairpin-suspect-dht-silent"
written: "2026-07-29"
author: "resilience-cards-converge sprint"
status: "open"
priority: "high"
tags: [dataplane, kitsune2, signal, dns, shem, operator-ceiling, saga-06]
cites:
  - genesis/data/timeline/backlog/content-divergence-unhealable-without-canonical-heads.md
---

# Shem conductors off the DHT since 21:15Z 2026-07-28 — every B-side convergence path dead-ends

Evidence (probed 2026-07-29 ~01:10-01:30Z):

- `GET https://elohim.host/db/p2p/conductor-diagnostics` → `agentCount:0,
  peer_urls:[], connections:[]` — persistent across 5 probes over 18 min
  (01:08→01:24Z), 4h+ past boot, far outside the ~20min churn window.
  Doorway-A same route: `agentCount:15` throughout. Both doorways read the
  SAME bootstrap store (`bootstrap-coherence: {backend:"mongo", spaces:5,
  agents:15}`) — 15 rows = exactly 3 on-prem agents × 5 spaces. Shem
  publishes nothing.
- adam's conductor booted `21:15:06Z` ("networking started"); its LAST
  kitsune2 line ever is `21:19:15Z WARN kitsune2_gossip::initiate: No local
  agents available, skipping initiate`. Fleet Loki counts (3h15m window):
  kitsune2_gossip james 958 / jessica 960 / matthew 945 — adam/eve/gertrude/
  susan ABSENT (each had 8k-22k lines in the preceding 24h; all four went
  silent 21:00-22:00Z).
- Clincher: B cannot retrieve its own declared head —
  `GET https://elohim.host/db/content/elohim-host-landing/head-record` →
  404 "this peer cannot retrieve the head action; no record to serve",
  while the same route on A returns 200 with the signed action. Full-arc
  fleet: local get miss is terminal, so declare answers "not retrievable",
  heal answers refused, gossip never runs.

**Prime suspect** (needs one operator probe before any restart): commit
0b44ecddc (18:00Z, 3h before the silent boots) renamed the signal plane —
shem-primary conductors render `signalUrl: wss://signal.elohim.host`
(elohim/holochain/Jenkinsfile:388, :758), which now resolves to shem's OWN
WAN IP. From inside shem's cluster that requires router hairpin-NAT — and
the shem router (GFiber) is the one we already know cannot be reconfigured
(see relay-capacity.feature). The failure set (all four shem-primary
humans, zero on-prem humans) matches the doorway-B signal URL exactly.
Both hostnames answer correctly FROM OUTSIDE (WebSocket-upgrade 400s, TLS
verified) — the break, if real, is inside shem.

## CONFIRMED 2026-07-29 (operator k8s-side diagnostic) — corrected mechanism

The hairpin is real but it does NOT hang: the GFiber router INTERCEPTS the
hairpinned 443 and answers with its own web UI (self-signed cert
`O=%Deaf3ef60`) — `curl` exits 60 with **ssl_verify_result=18** (and 403
with -k). Key on verify=18, never on a hang (a hang test false-clears).
Shem's pod egress IP (136.50.16.133) IS what the flipped names resolve to;
control test to example.com shows no blanket 443 interception.

- **Both flipped records are broken from shem, not just signal:**
  `bootstrap_url https://elohim.host/bootstrap` AND
  `signal_url wss://signal.elohim.host` (http=000 verify=18 from shem;
  200 from main). Fixing signal alone leaves bootstrap dead.
- **The conductors are not quiet — they're invisible.** Signature:
  `kitsune2_core::factories::core_space: Not updating agent info because
  we don't have a current url` (30min counts: adam 46, eve 115, gertrude
  150, susan 3; on-prem trio 0). All four still publish ops with huge
  republish backlogs (adam 64859/27943/26481) — "is it publishing?" reads
  healthy while everything cycles into nothing.
- **Validated fix:** `--resolve` to `10.99.0.2` (shem's node running BOTH
  nginx-ingress and coturn-shem) gives bootstrap 200/verify 0 and signal
  WS-upgrade 200/verify 0. CoreDNS rewrite is a TRAP here: node-local-dns
  runs DIRECT mode (169.254.20.10 forwards external names upstream,
  never consults coredns-ha), and it's one cluster-wide DaemonSet — a
  rewrite there would send main-side pods across WireGuard (worse than
  the bug). The scoped fix is **hostAliases on the four shem conductors +
  doorway-B**:
  `hostAliases: [{ip: 10.99.0.2, hostnames: [elohim.host, signal.elohim.host]}]`
  Leave `alpha.elohim.host` ALONE — other WAN (136.51.77.49), not
  hairpinned, verified good from shem (the working TURN leg). Caveat:
  pins shem's node IP into the pod spec — fine while shem-pinned; if a
  pod schedules elsewhere the alias is slow, not broken.

## Sequence

1. Operator: live-patch hostAliases (validation probe; podspec change
   rolls the pods, which they need anyway). Recovery = the
   "Not updating agent info" lines STOP on all four, B's
   conductor-diagnostics `agentCount > 0`, bootstrap store grows past 15.
2. Land the same hostAliases in genesis/orchestrator manifests (repo is
   the durable home; a live patch reverts on the next Jenkins build).
3. THEN DoD-1 is one HTTP lever away — but decide head direction first
   (see the declared-vs-declared finding in saga ch06: A's head is OLDER,
   08:56:34Z, than B's, 10:30:38Z, and they point at different SPA blobs;
   "B adopts A" as written would roll B backward).

Status: OPEN (operator). Downstream: saga ch06 both stations, ch10, the
DoD-1 red family in edge Dataplane Validation.
