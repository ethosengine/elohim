---
id: "backlog-doorway-utility-plane-and-borrowed-infra-exit-doctrine"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The doorway utility plane (shared STUN/signal/CDN/LB, anycast horizon, pkarr naming) + the borrowed-infrastructure exit doctrine — no borrow without a filed exit"
slug: "doorway-utility-plane-and-borrowed-infra-exit-doctrine"
written: "2026-07-11"
author: "operator vision notes (2026-07-11 late, ×3) + dht-unity cure arc"
status: "open"
priority: "high"
area: "architecture/transport-commons"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "memory:project_cross_pollination_surveys"
  - "memory:project_hub_optional_floor"
  - "memory:feedback_p2p_vs_federation_layer_vocabulary"
cites:
  - genesis-pair-dht-unity-plan | Genesis-Pair DHT Unity | path: genesis/docs/superpowers/plans/2026-07-11-genesis-pair-dht-unity-plan.md
  - genesis/data/timeline/backlog/sovereign-turn-relay-transport-commons.md
  - genesis/data/timeline/backlog/dht-scale-envelope-and-web2-projection-at-planetary-scale.md
tags: [doorway, transport-commons, stun, turn, anycast, pkarr, dns, anti-capture, contingency, tier-a, brainstorm-needed]
---

# The doorway utility plane + the borrowed-infrastructure exit doctrine

## The operator's thought (2026-07-11, three connected notes, verbatim intent)

1. Anything with a "global DNS sort of responsibility" is **shared among
   doorways** — doorways might one day share an IPv4 address doing anycast:
   distributed STUN, signal services, proxy protection, CDN, load
   balancing, etc.
2. **pkarr, or some other "blockchain-type" inclusion** — something no one
   needs to think about or govern the rules-about-the-rules of — is a valid
   adopted solution for offloading responsibility WITHOUT creating a
   capture/filter point.
3. The general form: anything that is a potential hazard as a doorway
   concern gets treated as *"we rely on this — AND here is the roadmap for
   how a rolling update to doorways creates a fully-distributed drop-in
   replacement for this in-kind piece of infrastructure if it comes under
   duress."* Contingency as a standing artifact.

## The read (composition, not a new subsystem)

**The utility plane is already half-real.** Doorways today carry the shared
bootstrap store (MongoK2Store), the SBD signal relays + cross-relay bus,
projection caching (CDN-shaped), TLS termination, and SSR. STUN / TURN /
LB / proxy-protection are the same Track-4 class: services needing public
addresses + bandwidth + operational standing — which is what a doorway IS.
This item extends fractal-federation's Tier-A *transport* commons into the
full *utility* commons, hosted by the doorway federation.

**Anycast has two horizons.** (a) Near: "logical anycast" — pkarr-resolved
(or multi-A/AAAA) doorway sets with client-side failover; no BGP needed,
ships as a doorway rolling update. (b) Far: true BGP anycast — an ASN +
address block as a COUNCIL-HELD COMMONS RESOURCE (the earned-reach
governance machinery holding a real-world artifact; the PR-ceremony vision
applied to network resources). Design both; ship (a).

**pkarr fits the no-governance niche exactly — and beats a blockchain.**
Signed DNS records over the BitTorrent mainline DHT: self-certifying ed25519
keys, no token, no fees, no consensus body to pressure, running on the most
ubiquitous capture-resistant p2p substrate in existence. The
cross-pollination surveys already rejected chain+token while flagging
n0/pkarr as the anti-capture pick; the doorway already reserves a `/pkarr/`
route prefix. Purpose now named: doorway discovery/naming that survives
registrar/ICANN duress — DNS becomes a borrowed thing with a filed exit.

**The exit doctrine (the gospel-worthy general form).** Every external
in-kind dependency a doorway relies on carries a standing ledger entry:
- what we borrow,
- its duress modes (outage, rate-limit, censorship, acquisition, ToS flip),
- the roadmap by which a rolling doorway update ships a fully-distributed
  drop-in replacement.
NOT "self-host everything now" (purity theater) — borrow freely, but **no
borrow without a filed exit**. The 2026-07-11 TURN arc is the
proof-of-pattern: borrowed openrelay as a diagnostic and filed the
sovereign coturn replacement the same hour.

## Opening ledger (seed — extend in the design session)

| Borrowed | Duress modes | Exit shape | Status |
|---|---|---|---|
| STUN (Cloudflare/Google) | outage, rate-limit | doorway-hosted STUN (trivial protocol, ~rolling update) | filed here |
| TURN (openrelay, diagnostic) | all + metadata observer | sovereign coturn, Tier-A commons | filed: sovereign-turn-relay-transport-commons |
| DNS names (registrar/ICANN) | seizure, censorship | pkarr-resolved doorway sets | filed here |
| CA/TLS (Let's Encrypt) | policy flip, outage | design question (DANE-over-pkarr? raw-key pinning for peers?) | open |
| shem (rented compute) | provider action | household/hub redistribution — hub-optional floor already doctrine | partially covered |
| GitHub/Jenkins/Harbor (dev plane) | account/org action | self-hosted already (Jenkins/Harbor/Nexus); GitHub exit = git's native distribution | mostly real |

## Deliverable

Fold into the dht-unity T5 design session (federation-level transport
commons) as its widened scope: the utility plane + the exit doctrine +
pkarr adoption design (p2p-design-gate for any new entities — doorway-set
records, utility-role attestations). The doctrine itself is a candidate
gospel paragraph for the seam-map atlas (concern-routing: "borrowed
infrastructure" as a named seam with a standing exit ledger).
