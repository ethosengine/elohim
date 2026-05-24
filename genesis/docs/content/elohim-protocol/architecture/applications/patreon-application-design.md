---
title: Patreon — substrate-native creator monetization
tier: architecture
status: Composition draft (primitives mapped; full walkthrough pending)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: shefa (primary — patronage flows), lamad (exclusive content), imagodei (creator + patron identity), qahal (patron community)
realizes:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (value flowing directly creator ↔ patron with no extraction)
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (creator economy without platform tax)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md (REA for recurring patronage flows)
informs:
  - app/elohim-app/src/app/shefa/ + lamad pillar for creator surfaces
  - bridges/stripe/ (for legacy payment cash-out)
  - elohim/sdk/domains/shefa/manifest.json (action verbs: subscribe, patronize, gift)
defers:
  - Tax / 1099 reporting (derived view; legal-side compliance is a separate concern)
  - Multi-currency creator-fund management (cross-currency Resource flow)
---

## The grandma test

A creator opens the app. They see: their patron count, monthly recurring patronage (the "MRR" chart they used to see in Patreon), recent patron-tier changes, the exclusive content they've published this month, comments and reactions from patrons. A patron sees: who they support, what tiers they're on, what exclusive content they have access to, their giving history. Patreon-shape — but no platform takes a cut, the patron-creator relationship is direct, exclusive content is reach-gated by the substrate.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Creator profile | imagodei Human + Creator-EPR (`content_type: "creator"`) | reach=`commons` for public-facing |
| Tier (e.g., "$5/month") | Commitment (`action: "subscribe"`) from patron to creator | recurring; child of Creator-EPR |
| Patronage payment | Event (`action: "transfer"`) | fulfills the tier-Commitment monthly; observation_refs to bank-import Observations |
| Exclusive content | EPR (`content_type: "post"` or `"video"`) with reach gated by tier-Attestation | only patrons with matching Attestation see it |
| Patron | Membership of creator's collective with `tier_metadata` | tier-attestation issued on first payment, renewed monthly |
| Patron-only community | Collective EPR (`content_type: "qahal"`) with reach=patron-tier | patron-tier Attestation required for entry |
| Creator-fund | Resource (`resource_classified_as: "currency-USD"` or equivalent) | balance derived from incoming Events |
| Tier change | Event (`action: "modify-subscription"`) | new tier-Commitment + Attestation; cancels old |
| Cancel | Event (`action: "cancel-subscription"`) | Commitment closes; tier-Attestation expires; reach to exclusive content revokes |

## Stress points the substrate handles

- **Subscription billing reliability**: Commitments fire scheduled Events on cadence (Spring-Batch shape); failure handling = retry-Events + grace-period Attestations + auto-downgrade on persistent failure
- **Tier-gated content access**: substrate validates the patron's current tier-Attestation against the content's reach-requirement at projection time; no platform-side ACL needed
- **Payout flows**: many patrons → one creator = many incoming Events under creator-account EPR; balance accumulates as Resource derived view; creator can request cash-out via Stripe bridge or convert to community currency
- **Cash-out (both sides)**:
  - Patron disconnects: cancel-subscription Event; existing patronage history preserved; future Events cease
  - Creator wants to leave platform-mediated payments: export Event history + patron list + tier definitions; portable
- **Recurring failure recovery**: bank-import Observations fail to graduate → creator's elohim-node detects via cursor-stuck signal → notification to patron to update payment method

## Scale answer

- Per-creator: ~1k–100k patrons (high-end); ~10k Events/month (patronage cycles) × 500 B ≈ ~5 MB SQL/month
- Per-patron: ~50 creators supported (high-end); ~100 Events/month
- Exclusive content: subset of creator's posts; reach-gated; storage shape same as Meta archetype
- Globally: bounded per-peer because each peer only sees creators they patronize + creators their reach scope includes

## Bridges to legacy

- **bridges/stripe/** — patron's bank account → Stripe → Observation → graduated patronage Event (parallel operation with credit-card flow during transition)
- **bridges/patreon/** (import) — Patreon export → batch-graduated Patronage Events under stewardship-elohim signature (creator brings their existing patron base)
- **bridges/paypal/** — same pattern for PayPal recurring billing
- **Cash-out**: creator's accumulated balance can transfer to legacy bank via Stripe bridge or convert to community-currency Resource

## Where agentic intelligence carries the load

- **Without shefa-elohim**: every monthly Patronage payment requires manual confirmation — patrons forget, creators chase. Patreon needs a centralized service to do this.
- **With shefa-elohim**: scheduled Commitment-fulfillment Events fire automatically; failures are detected and surfaced; patron and creator are notified of state changes
- **Without curation-elohim**: creator must manually decide what to publish to which tier; substrate provides reach-gating but content decisions are still manual
- **With curation-elohim**: elohim can suggest tier-targeting based on patron-engagement patterns (still creator-approved); cross-creator collab orchestration

## Code anchors

| Surface | Path |
|---|---|
| Patreon-shape surfaces | `app/elohim-app/src/app/shefa/` (recurring) + `app/elohim-app/src/app/lamad/` (content) |
| Subscription Commitments | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (Commitment with action="subscribe") |
| Stripe bridge | `bridges/stripe/` (planned) |
| Recurring scheduler | `elohim/elohim-storage/src/services/` (planned commitment-fulfillment task) |

*Full draft pending.*
