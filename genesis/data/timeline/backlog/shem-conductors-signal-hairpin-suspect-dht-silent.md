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

## Operator sequence (ceiling — no agent lever exists)

1. From any shem-side pod: `curl -sv https://signal.elohim.host/` —
   hang/refused ⇒ hairpin confirmed; clean 400 "requires WebSocket
   upgrade" ⇒ hairpin theory dead, look at the conductor's rendered
   signal_url and tx5 logs instead.
2. If hairpin confirmed: split-horizon the name inside shem (CoreDNS
   rewrite of `signal.elohim.host` → in-cluster ingress service IP, or pod
   hostAliases) — repo manifests are the cleanup surface; the sprint that
   picks this up should land it in the conductor/orchestrator manifests,
   not live-only.
3. Restart the four shem conductors (adam, eve, gertrude, susan), wait the
   ~20min churn window.
4. Pass signal: `GET https://elohim.host/db/p2p/conductor-diagnostics`
   shows `agentCount > 0` with non-empty peer_urls.
5. THEN DoD-1 is one HTTP lever away — but decide head direction first
   (see the declared-vs-declared finding in saga ch06: A's head is OLDER,
   08:56:34Z, than B's, 10:30:38Z, and they point at different SPA blobs;
   "B adopts A" as written would roll B backward).

Status: OPEN (operator). Downstream: saga ch06 both stations, ch10, the
DoD-1 red family in edge Dataplane Validation.
