---
id: "backlog-sovereign-turn-relay-transport-commons"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sovereign TURN relay as Tier-A transport commons — replace the diagnostic third-party relay wired 2026-07-11"
slug: "sovereign-turn-relay-transport-commons"
written: "2026-07-11"
author: "dht-unity cure leg"
status: "open"
priority: "high"
area: "substrate/transport-commons"
domain: "operator"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:project_alpha_topology_bootstrap_pair"
cites:
  - genesis-pair-dht-unity-plan | Genesis-Pair DHT Unity | path: genesis/docs/superpowers/plans/2026-07-11-genesis-pair-dht-unity-plan.md
  - genesis/data/timeline/backlog/genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md
tags: [turn, ice, webrtc, tx5, transport-commons, fractal-federation, sovereignty]
---

# Sovereign TURN relay — the Tier-A transport commons leg

## Why this exists

The genesis-pair cross-conductor fetch seam was isolated (2026-07-11) to the
WebRTC data channel: app #1604 proved the DHT `get` fails even with fully
CONVERGED peer stores (0/7 URL-mismatched at fire time), healthy shared
bootstrap, and a proven cross-pod signal bus. The pair straddles shem (cloud
NAT) ↔ on-prem (home NAT) with STUN-only ICE — no relay fallback. The cure
wired that day adds a **third-party diagnostic TURN**
(openrelay.metered.ca, TCP:80/443 forms, public shared credentials) to
`webrtc_config.ice_servers` in the edgenode template + adam manifest.

## The debt

A third-party TURN sees only encrypted DTLS bytes, but it IS a traffic-metadata
observer and an availability dependency on the exact plane the protocol calls
a domain-scoped commons (fractal-federation Tier-A: bootstrap/signal/relay).
The bootstrap and signal legs are already sovereign (MongoK2Store, doorway SBD
relays + mongo bus); TURN is the one leg outsourced.

## The work

- Deploy coturn (or equivalent) as domain infra: manifests under
  `genesis/orchestrator/manifests/infra/`, exposed on the shem side (public
  IP), TCP:443 (`turns:`) mandatory so household egress that blocks UDP still
  traverses; long-term-credential or REST-auth secrets via the pipeline
  secret path (never argv).
- Swap the openrelay entries in `_edgenode-consolidated.template.yaml` +
  `adam-firstman.yaml` for the sovereign URLs.
- Design tie-in: this is dht-unity T5's first concrete artifact — the
  federation-level transport commons (bootstrap+signal+relay as ONE set
  conductors reference independent of doorway affinity). Fold into that
  design session rather than treating as an isolated ops chore.
- Measure: the per-deploy declare-propagation probe stays green across ×2
  fresh edge builds after the swap.

## Escalation 2026-07-12 (evening): the third-party relay is DEAD — this item
## is now the SOLE blocker for notary-authority scenario 2

Loki evidence from tonight's declare window: `openrelay.metered.ca` →
216.39.253.123 is unreachable from BOTH networks — `no route to host` (shem)
and `connection timed out` (matthew home). The 2026-07-11 iceServers cure is
therefore STUN-only in practice; adam↔matthew pairing is an ICE lottery (won
once at edge #1187 ~05:40Z, lost every attempt all evening — app #1615's
36-minute declare ladder exhausted on `not retrievable`, and matthew's
kitsune2 logged `could not send publish ops: tx5 send error … timed out`).
Everything above the relay is DONE: monotonic-heal guard (no more backward
moves — adam held stable-stale all day), resilient 503-retrying declare
ladder (DECLARE_MAX_ATTEMPTS=24 on the propagation leg), and one landed
declare permanently unlocks heal-driven self-convergence. Priority: this is
the next operator move on the scenario-2 arc.
