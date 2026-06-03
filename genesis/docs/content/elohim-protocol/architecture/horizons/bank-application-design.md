---
title: Bank-as-Collective — financial institution on the substrate, REA-native banking
id: bank-application-design
tier: architecture
status: Horizon (coherent pattern, not on active subsumption path)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
realizes:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (banking as REA dashboard + token minting, not as institutional intermediary)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md (the lifecycle that handles transactions, balances, loans, KYC)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md (REA semantics for financial flows)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md (multi-party regulatory + audit coordination)
defers:
  - Active implementation — regulatory complexity makes this the latest move; bridges (Plaid / Stripe / banking-API) provide parallel operation through the early decades, with banks-as-collectives emerging once substrate proves itself at scale
---

## Why this is a horizon, not active

A bank-as-collective is architecturally the **same primitives as Monarch personal finance** at higher reach. The substrate already knows how to do this. What makes it a late move is **regulatory complexity**: KYC, AML, BSA, multi-jurisdictional licensing, deposit insurance, central-bank settlement. The substrate's commitment is *parallel operation + subsumption-by-merit* — legacy banks operate via bridges (Plaid / Stripe / banking API / KYC vendors) until cooperative bank-as-collective surfaces are mature enough for regulators to recognize. That's a decade-shape move, not a quarter-shape move.

In the meantime: Mint/Monarch B.1 already gives households REA-native visibility into their legacy bank accounts via bridges. Banking-as-substrate is the inversion — when the substrate-native version is so much better that even traditional banks want to participate as collective EPRs in the substrate's reach.

## Primitive composition (bank as a Collective EPR)

| What you see | Primitive | Notes |
|---|---|---|
| Bank / credit union | Collective EPR (`content_type: "financial-institution"`) | participates natively |
| Account at bank | EPR (`content_type: "account"`) | child of Institution; reach=`agent-private` for individual accounts |
| Currency held | Resource (`resource_classified_as: "currency-USD"`) | same primitive as Monarch B.1; balance derived from Event history |
| Transaction | Event (`action: "transfer"`) | same primitive as Monarch B.1, just at higher reach for institutional parties |
| Loan | Commitment chain | each scheduled payment is a planned Event; fulfillment generates actual Events |
| Mortgage | Long-lived Commitment with collateral Resource reference | property as Resource |
| Token minting (community currency) | Event (`action: "mint"`) | the bank-as-collective has minting authority within its reach |
| KYC verification | Attestation (`content_type: "attestation:kyc"`) | issued by the bank or by an external KYC bridge |
| Credit rating | Attestation (`content_type: "attestation:credit-rating"`) | reach-gated; multi-issuer competition possible |
| AML / fraud alert | FeedbackSignal (`signal_kind: "fraud-report"`) | gated by reach + governance |
| Regulatory audit | Mishpat-DNA governance Event flow | regulator gets commons-attested rollups, not per-account access |

## Stress points the substrate handles

- **Compliance-grade provenance**: every Event is DHT-notarized + signed by the institution; cryptographic finality stronger than wire-transfer SWIFT message-of-record
- **Transaction finality**: DHT validator quorum + bank's signature provides two-of-two notarization; settlement is the substrate's projection-controller pattern
- **Multi-jurisdictional regulation**: regulators participate as Collective EPRs themselves; jurisdictional reach scoping means regulators see what they're entitled to see and nothing more
- **Parallel operation with legacy**: bridges to Plaid / Stripe / banking-APIs / SWIFT / Fed-wire let households and merchants connect their legacy bank accounts; the substrate-native version emerges alongside as cooperative banks adopt it
- **Token-minting authority**: a community-bank-collective can mint a community currency under its reach; the substrate doesn't dictate monetary policy, it just records the flows

## Why this is deferred

Three reasons sequence the move late:

1. **Regulatory recognition takes time** — bank-as-substrate-collective isn't a thing regulators will license until cooperative financial primitives have proven themselves at multiple scales (community credit unions, mutual-aid funds, cooperative lending pools)
2. **Mint / Monarch (active B.1) achieves the household-visibility goal NOW** via bridges to legacy banks — the value-prop reaches users without waiting for substrate-native banking
3. **R&O cooperative commerce (active B.7) builds the value-flow patterns** that natural-fit community-bank-collectives; once cooperatives are routinely managing pooled funds + lending on the substrate, banking-as-substrate is the natural next step

The horizon is preserved because the architecture is coherent and the day will come.
