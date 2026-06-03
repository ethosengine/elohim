---
title: Requests & Offers — substrate-native cooperative commerce (Amazon's cooperative side)
id: requests-offers-application-design
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

Grandma opens the Requests & Offers app on her phone. She sees: things she's offered (the kid bikes her grandkids outgrew; her sewing machine on idle weekdays; tomatoes from her garden), things she's looking for (a sturdy step-stool; someone to help paint the back porch), and matches her elohim-agent has surfaced ("Dawn three doors down posted a step-stool yesterday; Carlos at the corner shop owes you a favor and offered painting hours"). She taps "join cooperative" and sees the **Okonkwo Family Buying Club** her granddaughter set up — pooled demand for bulk rice, cooking oil, school supplies, joint Commitments to a supplier in Ibadan, current cycle's per-household share.

The app feels like Amazon-on-the-cooperative-side. It is not Amazon. No platform takes a cut. Every Offer or Request earns visibility through her standing in her community, not through ad spend. When the cooperative buys rice in bulk, the supplier sees one Collective Commitment from twelve households — not twelve customers being upsold individually.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Offer | EPR (`content_type: "offer"`) | "I have X available"; reach=`community` by default per D.9 |
| Request | EPR (`content_type: "request"`) | "I need X"; reach=`community` by default |
| Resource on offer | Resource (`resource_classified_as: "tool"|"food"|"labor-hours"|...`) | `parent_epr_cid = offer_cid`; subordinate (D.1) — not independently gossiped |
| Match | Event (`action: "matched"`) | links Offer + Request; provider=offerer, receiver=requester |
| Fulfillment | Event (`action: "transfer"`) | implements the match; `parent_epr_cid` → offer or request |
| Cooperative pool | Collective EPR (`content_type: "purchasing-cooperative"`) | members are households via qahal Memberships |
| Joint Commitment | Commitment from cooperative → supplier | aggregated household pledges; `clause_of` references a cooperative Agreement |
| Per-household share | Commitment from household → cooperative | `clause_of` references the cooperative's parent Agreement |
| Visibility widening | Event (`action: "grant-reach"`, D.9) | "list this offer for community / commons-scope" — explicit, audited |
| Trust signal | confirmation-class Attestation (`attestation:fulfillment-completed`, D.16) | multi-counterparty signs the same fact; NOT a star-rating |
| Bid / counter-offer | FeedbackSignal (`signal_kind: "bid"`) on an Offer | proposed-terms in payload |
| Dispute | FeedbackSignal (`signal_kind: "dispute"`) | escalates to qahal-mediation flow |
| Cooperative fee | manifest-declared fee_split (D.20 Layered Commons) | small slice of each Fulfillment ratchets to the cooperative's Bridge Commons |

Eight primitives, ~10 discriminator values, no special-casing for commerce.

## How one cooperative procurement flows

The Okonkwo Family Buying Club orders 100 kg of rice for twelve households:

```
 1. Each household authors a Commitment to the cooperative
       action: "request"
       provider: <household_cid>
       receiver: <cooperative_collective_cid>
       resource_classified_as: "food:rice-50kg-bag"
       quantity: <1 or 2 bags per household>
       clause_of: <cycle_agreement_cid>      (D.5 Agreement scoping)
       parent_epr_cid: <cooperative_cid>     (D.1 subordination)
 2. Cooperative-elohim aggregates accepted Commitments at the cycle close
       → emits a Collective Commitment from cooperative → supplier
           action: "purchase"
           provider: <cooperative_collective_cid>
           receiver: <supplier_agent_cid>     (a Merchant EPR or stub)
           resource_quantity_value: 100 kg
 3. The Collective Commitment broadcasts at reach=`community` so any
    member supplier in the bridge's reach can counter-offer
 4. Supplier accepts; Agreement transitions to in-progress
 5. Supplier authors fulfillment:
       Event { action: "transfer", provider: supplier, receiver: cooperative,
               resource: 100 kg rice, parent_epr_cid: agreement_cid }
       fee_splits: [{commons_cid: <bridge_commons>, amount: 1.0},
                    {commons_cid: <global_commons>, amount: 0.5}]    (D.20)
 6. Cooperative-elohim splits the receipt into per-household receive-Events
    under each household's Commitment (per-share Fulfillment)
 7. Each household authors a `confirmation`-class fulfillment Attestation
    signed by both household + cooperative + supplier (D.16 chain)
 8. Standing accrues to all parties; supplier's Offer earns more
    cross-community reach on next cycle via reputation projection
```

Bidirectionality is structural: any household can pull its own Commitments + Attestations via VF-shaped JSON export at any time; the cooperative cannot lock anyone in because the substrate stores the truth.

## Storage footprint per household

| Item | Count | Size | Total |
|---|---|---|---|
| EPRs (offers + requests, active + 2 yr history) | ~500 | 5 KB | 2.5 MB |
| Resources (the stuff being offered / requested) | ~500 | 1 KB | 500 KB |
| Events (matches + fulfillments, 5 yr) | ~5k | 500 B | 2.5 MB |
| Commitments (cooperative shares, ongoing + history) | ~200 | 1.5 KB | 300 KB |
| Attestations (`fulfillment-completed`, confirmation-class) | ~2k | 2 KB | 4 MB |
| FeedbackSignals (bids, disputes) | ~1k | 500 B | 500 KB |
| **Total local SQL projection** | | | **~10 MB** |

**Fits on a phone.** A typical household participates in 1-3 cooperatives + a dozen ad-hoc neighbor exchanges per month; the working set is small because reach-scoping keeps gossip local.

## Network bandwidth profile

- New Offer at reach=community: ~5 KB DHT write + neighborhood gossip ≈ ~50 KB total peer load
- Match Event: ~500 B; Fulfillment Event: ~1 KB; confirmation Attestation with 3-signer chain: ~3 KB
- Federated cross-community query (rare; "anyone in the cooperative-hub network have a snowblower?"): one fan-out RPC + parallel SQL responses; ~10-50 KB per round-trip
- **Per active household: <20 MB/month** for full R&O participation including cooperative cycles

## DHT entry impact

- 100M households × ~50 active Offers + ~10 active Requests = ~6 × 10^9 active entries globally
- But: reach=`community` keeps each entry's gossip scope local to the authoring community (~150-500 peers); per-peer DHT visibility ≈ ~30k entries from the local community plus a few hundred at commons reach
- Cooperatives multiply this only slightly: a cooperative's Collective Commitment lives at the cooperative's collective-reach scope (its member households), not globally
- Reach widening to `commons` (the rare publicly-listed offer) requires the D.9 reach-mutation Event with mishpat governance — not a one-click "publish to all" — which structurally prevents global product-catalog explosion

## Why matching renders fast

- **Local browse**: `SELECT * FROM offers WHERE community_reach_includes(:my_community) AND state='active' ORDER BY observed_at DESC LIMIT 100`. Indexed on reach + state. Milliseconds.
- **My matches**: cooperative-elohim runs a continuous query over local SQL classifying recent Offers against open Requests; surfaces as ambient notifications, never as a feed
- **Federated cross-community search**: only on explicit user action ("look broader than my community"); cooperative-hub elohim-node fans out libp2p RPC to peer cooperatives' local SQL; results stream back; hub holds zero replicated data — pure query federation
- **Reputation lookup**: `SELECT * FROM attestations WHERE subject_cid = :counterparty AND attestation_kind = 'fulfillment-completed' AND class = 'confirmation'`. The result is a list of multi-signed transaction confirmations — not a star average. The UI shows "47 confirmed fulfillments across 9 communities" + the actual confirmation chain, never a ranked score.

## Why this doesn't melt the network

- Offers are EPRs whose Resources subordinate via `parent_epr_cid` (D.1) — the 500 kg of rice listed by twelve households doesn't generate twelve global Resource entries; the Resources live under their parent Offer's reach
- Reach defaults to `community`; D.9 reach-mutation Events are the **only** path to broader visibility — every widening is signed, audited, and authority-checked at integrity-zome floor
- Cooperative purchasing collapses N households × 1 supplier into 1 Collective Commitment from the cooperative's perspective; the supplier sees aggregate demand, not a thundering herd
- No global product index exists at the substrate level — there's no `GET /api/products`; "what's available?" is always answered relative to the asker's reach scope

## Dissolution in practice

- Offer fulfilled or withdrawn → `Event(action: "close", subject: offer_cid)` → Offer transitions to `closed`; subordinate Resources move to disposal-history
- Cooperative member leaves → `Event(action: "revoke-membership")` against the Membership EPR; future Commitments from that household to the cooperative reject at validation; existing Commitments remain queryable as history
- Cooperative dissolves → cooperative-elohim authors `close` against the Collective EPR; outstanding Commitments must resolve (fulfilled or cancelled) before final close; per D.7 dissolution semantics, closed Collective EPRs reject all new reach-mutation and subordination Events
- Supplier exits the network → their stub Merchant EPR transitions to `closed`; historical fulfillments remain attestable, but no new Offers can subordinate under them

## Where agentic intelligence carries the load

- **Cooperative-elohim**: aggregates per-household Commitments into Collective Commitments at cycle close; selects supplier candidates from prior-fulfillment Attestation chains; surfaces ambient cycle-status to members; without it, every cooperative buy is a group-chat coordination crisis
- **Stewardship-elohim (per household)**: classifies inventory drift — "the kids outgrew this bike" — into draft Offers the human approves with one tap; narrates household-need patterns into Requests
- **Reputation-elohim (per requester's standing)**: presents counterparty fulfillment-chains in legible form ("9 communities have signed off on 47 of their fulfillments; here are 3 communities where they had disputes") rather than collapsing trust to a number — multi-attestor confirmation is the substrate signal, not a fabricated rating
- **Logistics-elohim (per provider bridge)**: tracks delivery Observations from courier bridges; graduates Fulfillment-Attestations only when the receiver confirms physical receipt (peer-witness or signed scan)
- **Care-class isolation invariant**: gift-economy Offers (`signal_class: "care"`) and market-economy Offers (`signal_class: "commerce"`) flow on the same primitives but never cross-contaminate standing — a household that gives generously doesn't accumulate commerce-class standing that can be liquidated; per D.18 signal_class.

This is "an economy that can scale love and care" applied to commerce: the substrate makes cooperative procurement, gift exchange, and supplier relationships legible to the network without flattening them into a single transactable surface.

## What the cooperative-hub view actually shows

The cooperative Collective EPR references each member Household EPR via Membership entries. Members opt in by setting reach=`collective` on the Commitments they pool. Render path: dashboard query → cooperative-hub elohim-node → parallel libp2p RPC to each member's local SQL → aggregated cycle summary. Hub holds aggregate-only projections (current cycle's pooled quantity per Resource class, list of accepted Collective Commitments, fulfillment status) — not per-household balances. If a household revokes membership, their data stops flowing immediately because the hub never held it. **Cash-out is structural; the cooperative is incapable of capturing its members.**

Cross-cooperative views compose the same way: a cooperative-of-cooperatives (e.g., a regional buying federation) is itself a Collective EPR with member Collective EPRs; federated query at higher tiers; the substrate's reach scoping ensures each layer sees aggregates, never raw membership data.

## How Layered Commons funds the cooperative without extraction (D.20)

Every Fulfillment Event optionally declares a manifest-declared `fee_splits`: a small slice (e.g., 0.5% — manifest-tunable per cooperative) ratchets to the cooperative's Bridge Commons; another small slice (e.g., 0.5%) ratchets to the Global Commons. The cooperative's own membership governs how its Bridge Commons spends (shared logistics, group insurance fund, software-stewardship costs); the Global Commons is governed by apex elohim councils for protocol-wide public goods. **This is where the substrate distributes the platform-value-capture Amazon currently extracts**: the same percentage that would have funded a CEO's bonus instead funds the cooperative's shared infrastructure + the network's anti-concentration redistribution. Mass-conservation discipline (substrate-floor invariant) enforces `receiver_amount + sum(fee_splits) = total_authored_amount` — the integrity zome rejects Events that don't sum.

Friction-gradient limitarianism (per D.20 + `project_friction_gradient_limitarianism`) applies if any single supplier-EPR accumulates outsized commerce-class holdings: their incoming Fulfillment Events ratchet a larger fraction to Global Commons as their balance crosses manifest-declared tiers. The substrate makes accumulation-as-marketplace-platform mechanically expensive without prohibiting any single transaction.

## Bridges (legacy interop / cash-out)

- **bridges/valueflows/** (existing reference) — VF-GraphQL bridge; the upstream-compat surface. R&O Offers project as VF `Intent`s; Requests project as VF `Intent`s with opposite-direction; Fulfillments project as VF `EconomicEvent`s; Commitments project as VF `Commitment`s. The R&O hApp can speak to elohim cooperatives natively via this bridge.
- **bridges/amazon/** (read-only, planned) — Amazon product catalog imported as Offer-EPRs under stewardship-elohim signature. Substrate-native Offers from local contributors grow alongside Amazon-bridge Offers; price + availability deltas surface as the substrate's reputation system accumulates evidence that local alternatives are reliable. Transition path: Amazon Inc. could become a Collective EPR if/when they choose to participate substrate-natively per D.8 reframe — the bridge becomes the rails for that handoff.
- **bridges/stripe/** — for transactions needing legacy-currency clearing; cooperative-internal flows use community-currency Resources where available
- **bridges/logistics/** (planned, per provider) — delivery confirmation Observations feed Fulfillment Attestations; the bridge is a per-provider crate (UPS, DHL, local courier collectives) that translates webhook into substrate-native Observation
- **Cash-out**: any Offer / Request / Fulfillment exports as VF-shaped JSON via the valueflows bridge; portable to any VF-compatible system. A household leaving the network walks away with its complete commerce history; nothing is captured by the substrate.

## Code anchors

| Surface | Path |
|---|---|
| R&O Angular surfaces | `app/elohim-app/src/app/shefa/` (offers / requests / cooperatives) |
| Cooperative Membership surfaces | `app/elohim-app/src/app/qahal/` (collective governance + Memberships) |
| Offer / Request entry types | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (`Content` with `content_type: offer | request`) |
| Match / fulfill coordinator | `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (action verbs: `matched`, `transfer`, `purchase`) |
| View schemas | `elohim/sdk/schemas/v1/views/offer-view.schema.json`, `request-view.schema.json` (planned) |
| Shefa manifest | `elohim/sdk/domains/shefa/manifest.json` (action verbs: `matched`, `purchase`, `bid`, `confirmation_requirements` for fulfillment attestation_kind) |
| VF-GraphQL bridge | `bridges/valueflows/valueflows-bridge/src/translate/proposal.rs` + `commitment.rs` (R&O `Proposal` + `Intent` translation) |
| Amazon read-only bridge | `bridges/amazon/` (planned) |
| Cooperative coordinator service | `elohim/elohim-storage/src/services/cooperative_service.rs` (planned — Commitment aggregation; cycle close; per-share fulfillment splitting) |
| Doorway HTTP routes | `doorway/doorway-service/src/handlers/shefa/offers/` (planned) |

## What this proves about the substrate

A skeptical commerce architect should walk away able to say:

- The Amazon-shape primary surface (Offers + Requests + Resources projected to local SQL with reach-scoped search) handles cooperative-procurement workloads at low-MB working sets — no centralized catalog, no platform-side ranking, no engagement optimization
- The cooperative-procurement flow (N households' Commitments aggregate into 1 Collective Commitment to a supplier; Fulfillment splits per-share) maps cleanly onto Commitment + Agreement + Event without inventing marketplace-specific primitives
- Trust via D.16 multi-attestor confirmation chains is structurally distinct from centralized star-ratings — counterparties sign the same fact, the chain is content-addressed, no platform owns or operates the reputation
- D.9 reach-mutation Events prevent global product-catalog explosion: every widening is an authority-checked Event; the substrate cannot accidentally publish 6 × 10^9 Offers to a global index because there is no global index — there's only federated query at the requester's initiative
- D.20 Layered Commons distributes the platform-value-capture Amazon currently extracts: the cooperative's Bridge Commons funds shared infrastructure; the Global Commons funds protocol-wide public goods; friction-gradient ratcheting makes platform-extraction-shape accumulation mechanically expensive
- The VF-GraphQL bridge proves substrate-native cooperatives can interoperate with the existing R&O hApp + other VF-aware cooperatives without either side surrendering its worldview

If those six claims hold for cooperative-procurement, the substrate is the cooperative-economy proof — structurally egalitarian by composition, not by policy.
