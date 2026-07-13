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

## Update 2026-07-13 — the diagnostic relay is confirmed GLOBALLY DEAD; cure authored, deploy-ready

**New evidence (why this is now urgent, not merely a hardening chore).** The
openrelay diagnostic relay is not slow — it is gone. Identical `i/o timeout`
reaching `staticauth.openrelay.metered.ca:80` AND `:443` from THREE disjoint
networks: adam (shem/cloud), matthew (on-prem/home), and the dev env (a third
ISP). Three networks failing the same way ⇒ the relay endpoints are down, not
any local egress. Conductor evidence (Loki, both edgenodes):
`tx5_go_pion_sys: failed to get server reflexive address ...
openrelay.metered.ca ... i/o timeout` → `Failed to ping without candidate
pairs. Connection is not possible yet.` → kitsune2 gossip rounds initiate/accept
(signaling is fine) but time out (the data channel never forms) → matthew-
authored actions are `not retrievable` on adam → the notary content arm re-flags
the same divergent set every sweep (divergentAnchor pinned ~2080), saturates
adam's conductor (`PTxnGuard` held 1–2s), and holds the doorway-B→adam circuit
breaker OPEN — so https://elohim.host serves `503 catching-up` on every read.
This is the live user-visible outage, not a benign stale head.

**Seam confirmed (dht-unity T3).** Discovery WORKS — matthew's conductor peer
store carries agents on BOTH signal clouds (shared bootstrap gossips agent_infos
across the pair). The failure is purely tx5/WebRTC session establishment across
the WAN NAT pair with no working relay. STUN-only + a dead TURN = no candidate
pair. So the sovereign relay is the whole remaining item behind notary
scenario 2.

**Cure authored (this session), deploy-ready:**
- `genesis/orchestrator/manifests/infra/alpha-coturn.yaml` — coturn 4.6.2,
  hostNetwork-pinned to shem (node-type=remote, the public-IP node; this
  cluster has no LoadBalancer/MetalLB and the nginx ingress can't carry raw
  UDP/TCP), LTC auth, cert-manager TLS for `turn.elohim.host`, relay range
  49160–49200/udp. Passes no gate yet because it is infra (applied directly).
- `_edgenode-consolidated.template.yaml` + `adam-firstman.yaml` iceServers
  swapped openrelay → `turn:turn.elohim.host:3478` (udp+tcp) + `turns:5349`.
  Both PASS `scripts/ci/validate-conductor-config.sh`.

**Irreducible operator residual (cluster-owned; no repo change bypasses it) —**
enumerated in the manifest header, summarized: (1) DNS A record
`turn.elohim.host → shem public IP`; (2) open shem cloud firewall for
3478/udp+tcp, 5349/tcp, 49160–49200/udp; (3) set `external-ip` in the ConfigMap
if shem is a cloud-NAT host; (4) confirm the cert-manager ClusterIssuer name.
The backlog's "turns on :443" ask conflicts with nginx owning shem:443 — the
manifest uses 5349; honoring :443 needs a second IP or ingress TCP passthrough
(operator call). After deploy: the per-deploy `✓ canonical head propagated`
probe + notary scenario 2 green ×2 is the DoD.

Status stays **open** — code is banked, but "done" is gated on the operator
deploy above; a follow-up agent should verify the probe post-deploy, not
re-author the manifest.
