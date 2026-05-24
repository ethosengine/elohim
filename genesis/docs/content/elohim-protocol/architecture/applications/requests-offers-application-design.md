---
title: Requests & Offers — substrate-native cooperative commerce (Amazon's cooperative side)
tier: architecture
status: Composition draft (primitives mapped; full walkthrough pending)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: shefa (primary — value flows), qahal (cooperative collectives), imagodei (party identity), elohim (substrate)
realizes:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (REA at marketplace scale; cooperative procurement)
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (household needs visible across life-stage archetypes)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md (VF / hREA bridge — R&O hApp interop)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md (cross-collective cooperation patterns)
informs:
  - app/elohim-app/src/app/shefa/ + qahal pillar for marketplace surfaces
  - bridges/valueflows/ (VF-GraphQL projection — the upstream-compat surface)
  - elohim/sdk/domains/shefa/manifest.json (content_types: offer, request; action verbs: match, fulfill)
defers:
  - Reputation scoring algorithm (substrate provides Attestation streams; ranking is app-layer)
  - Logistics / shipping integration (separate bridges per logistics provider)
  - Smart-contract escrow patterns (Commitment-with-collateral; subsequent spec)
---

## The grandma test

A user opens the app. They see: things they've offered (the kid bikes the kids outgrew; tools they're not using; baked goods from this morning), things they're looking for (school supplies for the new term; help painting the house), matches their elohim-agent has surfaced ("Dawn three doors down has the rake you asked about"), recent fulfillments. A cooperative purchasing group sees: pooled demand for bulk orders, supplier candidates, joint Commitments. Amazon-marketplace-shape on the cooperative side — but no platform takes a cut, every Offer or Request earns reach by the contributor's standing, fulfillment is between parties directly.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Offer | EPR (`content_type: "offer"`) | "I have X available"; reach=`community` by default |
| Request | EPR (`content_type: "request"`) | "I need X"; reach=`community` |
| Match | Event (`action: "matched"`) | links Offer + Request when elohim or human confirms fit |
| Fulfillment | Event (`action: "transfer"`) | implements the match — provider, receiver, resource, quantity, parent_epr_cid → offer or request |
| Cooperative pool | Collective EPR (`content_type: "purchasing-cooperative"`) | members are households; pools demand |
| Joint Commitment | Commitment from cooperative to supplier | multiple households' pooled purchase intent |
| Resource catalog | each Offer references an `EconomicResourceClassification` | manifest-declared taxonomy of stuff people offer / need |
| Reputation | Attestation streams from prior fulfillments | per-party Attestations issued by counterparties on completion |
| Search / browse | local SQL over Offers in reach + federated query through cooperative-hub | |
| Bid / counter-offer | FeedbackSignal (`signal_kind: "bid"`) on an Offer | with proposed-terms in payload |
| Dispute | FeedbackSignal (`signal_kind: "dispute"`) | escalates to qahal-mediation flow |

## Stress points the substrate handles

- **Marketplace-scale matching**: Offers/Requests stay at `community` reach by default (don't melt DHT with global catalogs); cross-community matching = federated query through cooperative-hub at the requester's initiative
- **Trust without central rating**: Attestations from prior fulfillments accumulate per party; reputation is content-addressed and verifiable; no platform-owned star-rating
- **Cooperative purchasing**: multiple households pool demand via joint Commitment; supplier sees aggregate demand; fulfillment splits into per-household receive-Events
- **VF / hREA interop**: bridges/valueflows projects R&O Offers and Requests as VF `Intent`s, fulfillments as VF `EconomicEvent`s — interoperability with the existing R&O hApp and other VF-aware cooperatives
- **Logistics**: shipping / delivery integration via per-provider bridges; Fulfillment Event references the delivery-Attestation when complete

## Scale answer

- Per-household: ~10–100 active Offers + ~10 active Requests at any time × few KB each ≈ ~1 MB SQL
- Per-cooperative hub: aggregates over member households via federated query; doesn't replicate per-household data
- Per-Offer / Request: ~5 KB DHT entry; reach=community keeps gossip footprint local to that community
- Globally: 8B humans × 10s of active needs = bounded local working sets; matching happens within reach scope unless escalated

## Bridges to legacy

- **bridges/valueflows/** — VF-GraphQL bridge already exists; R&O hApp interop is its primary use case
- **bridges/amazon/** (read-only) — Amazon product catalog as read-only Offer-EPRs under stewardship-elohim signature (transition path: substrate-native Offers grow alongside Amazon-bridge Offers)
- **bridges/stripe/** — for the small-percentage of transactions that need payment-rails clearing (cooperative-internal flows use community-currency Resources instead)
- **Cash-out**: every Offer / Request / Fulfillment exports as VF-shaped JSON; portable to any VF-compatible system

## Code anchors

| Surface | Path |
|---|---|
| R&O surfaces | `app/elohim-app/src/app/shefa/` (offers / requests) + `qahal/` (cooperatives) |
| VF bridge | `bridges/valueflows/` |
| Match / fulfill coordinator | `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` |

*Full draft pending.*
