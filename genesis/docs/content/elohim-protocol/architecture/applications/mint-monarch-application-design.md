---
title: Mint / Monarch.app — personal finance + household stuff on the substrate
tier: architecture
status: Full draft (exemplar — the template-shape for other application archetypes)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: shefa (primary — economic flows + REA), elohim (substrate primitives), imagodei (household membership), lamad (Resource state for tagged stuff)
realizes:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (Beer's Cybersyn on P2P; the Okonkwo family wakes up to economic visibility)
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (household care made legible)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md (the eight foundational primitives this composes)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md (REA / ValueFlows semantics)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md (Observation tier; bank-statement parsing graduates here)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (Plaid / Stripe bridge layer)
informs:
  - app/elohim-app/src/app/shefa/ (where the Monarch-shape dashboard lives)
  - bridges/plaid/ (planned — Plaid bank-import bridge crate; pattern reference)
  - bridges/stripe/ (planned — Stripe commerce bridge crate)
  - elohim/sdk/domains/shefa/manifest.json (action verbs: transfer, classify-spending; observation_kinds: card-swipe, statement-parsed)
defers:
  - Multi-currency conversion semantics (cross-currency Resource flow needs its own design)
  - Tax-report generation (derived view, but the report-generation logic is a separate spec)
  - Investment portfolio Greeks / risk modeling (out of substrate scope; lives in app layer)
---

## The grandma test

Grandma opens the app on her Android. She sees: net worth chart, monthly cash flow, recent transactions, accounts list, budgets, and "my stuff" (couches, cars, tools). She can also tap a tab to see the **three-family view** — net worth aggregated across herself + her two adult kids' households, joint commitments (the shared cabin), and shared assets.

The app feels like Mint. It is not Mint. There is no central server. Her data is hers. She can cash out at any time.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Household | EPR (`content_type: "household"`) | root vessel; reach=`household` or `community` |
| Account at her bank | EPR (`content_type: "account"`) | child of Household; reach=`agent-private` |
| Household-Inventory | EPR (`content_type: "inventory"`) | child of Household; the basin for stuff |
| Couch / Car / Tool | Resource (`resource_classified_as: "furniture"` etc.) | `parent_epr_cid = inventory_cid`; subordinate, not independently gossiped |
| Currency she holds | Resource (`resource_classified_as: "currency-USD"`) | balance derived from Event history |
| Every transaction | Event (`action: "transfer"`) | `parent_epr_cid = account_cid`; mostly elohim-graduated from observations |
| Monthly budget | Commitment | planned future Events; child of Household |
| Spending category | `event_classified_as` discrimination | manifest-declared, not a DHT entry |
| Investment price | Attestation (`content_type: "attestation:price-feed"`) | reach=`commons`; daily-cadence |
| Net worth, cash flow | Derived SQL views | computed locally; zero entries |
| 3-family collective | Collective EPR (`content_type: "household-collective"`) | references three Household EPRs |

Eight primitives, ~6 discriminator values, no special-casing for personal finance.

## How one transaction flows

Grandma swipes her debit card at a coffee shop:

```
1. Bank webhook → bridges/plaid/ crate receives event
2. stewardship-elohim signs an Observation
       observation_kind: "shefa:card-swipe"
       subject_cid: grandma's account EPR
       payload_json: { merchant: "Joe's Coffee", amount: 5.50, ... }
       libp2p only — no DHT write
3. Graduation policy says card-swipes graduate 1:1 (high-signal evidence)
4. shefa-elohim authors an Event:
       action: "transfer"
       provider: grandma's account
       receiver: Joe's Coffee (a Merchant EPR if known, else a stub)
       resource: USD-currency
       quantity: 5.50
       parent_epr_cid: account_cid
       observation_refs: [iroh://... pointing at the source observation]
5. DHT write — single ~2 KB entry; validated by neighborhood
6. libp2p sync plane delta-syncs the Event projection to grandma's other devices
7. Local SQL projection updates account balance derived view
8. Dashboard auto-refresh: SELECT shows the new transaction in <100 ms
```

The Plaid bridge is bidirectional. If grandma initiates a transfer in the app, the bridge can push it back to her bank (where the bank's API supports writes). When she cashes out — disconnects Plaid — the bridge stops authoring new Observations; existing Events stay; she can export the bridge-authored Events to legacy formats.

## Storage footprint per household

| Item | Count | Size | Total |
|---|---|---|---|
| EPRs (household + accounts + inventory + categorizations) | ~1k | 10 KB | 10 MB |
| Events (10 years × 50/day) | ~180k | 500 B | 90 MB |
| Resources (stuff) | ~10k | 1 KB | 10 MB |
| Commitments (budgets, recurring) | ~100 | 1 KB | 100 KB |
| Attestations (price feeds, KYC) | ~50 | 2 KB | 100 KB |
| **Total local SQL projection** | | | **~110 MB** |
| Cold archive (events >2 yr in quilt) | — | — | ~50 MB residue |

**Fits on a phone, fits in memory.** The dashboard loads instantly because it's reading local SQL like Mint reads Postgres.

## Network bandwidth profile

- New transaction: ~2 KB DHT write + neighborhood gossip ≈ ~20 KB total peer load
- Daily inbound: 50 events × 20 KB ≈ 1 MB/day
- libp2p sync plane: cursor-driven, delta-only
- **Per household: <100 MB/month** for full Monarch participation

## DHT entry impact

- 10k Events/year/household × 100M households = 10¹² Events/year IF everything went global
- But: reach is `agent-private` for personal transactions; peers only see Events within their reach scope
- Per-peer DHT entry visibility: only EPRs+Events in the household + opted-in collective scope
- Typical household peer holds ~100k entries — well inside DHT validator budget

## Why the dashboard renders fast

- **Net worth**: `SELECT SUM(quantity)` over Resource derived-views. Local SQL, milliseconds.
- **Recent transactions**: `SELECT * FROM events WHERE parent_epr_cid IN (account_cids) ORDER BY observed_at DESC LIMIT 50`. Indexed, milliseconds.
- **Monthly cash flow**: `GROUP BY month`. Indexed.
- **"My stuff"**: `SELECT * FROM resources WHERE parent_epr_cid = inventory_cid`. Lazy-loaded; couches don't render until you tap.
- **3-family aggregate**: a federated SQL query through the collective-hub elohim-node. Hub queries each family's local SQL projection in parallel via libp2p RPC; aggregates; returns. **No data replication — just query federation.** Per-family privacy preserved by reach.

## Why couches don't melt the network

- Resources are subordinate to Household-Inventory EPR (via `parent_epr_cid`)
- Not independently gossiped — only queryable through Inventory parent's reach scope
- 10k Resources/household × 100M households = 10¹² Resources globally
- But: each Resource's gossip scope = its parent's reach = `household` by default
- DHT only sees Resources that *earn* higher reach (couch listed for sale → reach elevates to `community` via a `grant-reach` Event)

## Dissolution in practice

- Grandma changes banks → `Event(action: "close-account", subject: account_cid)` → account EPR transitions to `closed`
- Closed account's transaction history queryable but excluded from current net worth
- Grandma throws out the couch → `Event(action: "dispose", subject: couch_cid)` → couch Resource transitions to `closed`
- Closed couch doesn't appear in current inventory but appears in disposal history if she taps "show closed"
- Future Events targeting a closed account/Resource fail validation (substrate-floor invariant)

## Where agentic intelligence carries the load

- **Without shefa-elohim**: every grocery trip would be 30 manual entries. Nobody does that. → Mint barely works for non-financial-nerds.
- **With shefa-elohim**: receipt photo → automated Event creation; bridge-imported bank events; `event_classified_as` auto-tagged by elohim cognition.
- **Without inventory-elohim**: keeping inventory of all your stuff is "I'll do it later" — nobody does it.
- **With inventory-elohim**: receipts and purchase confirmations narrate → Resource creation + parent-link to Inventory automatic.
- **Without care-stewardship-elohim**: cooking, cleaning, caregiving are invisible to any financial dashboard.
- **With care-stewardship-elohim**: household activity observed → care-Events with quantities (meals provided, hours of caregiving) → aggregates to a care-account Resource → care work becomes **legible to the household's economic picture**.

This is "an economy that can scale love and care."

## What the 3-family view actually shows

The Collective EPR refs three Household EPRs. Each family opts in by setting reach=`collective` on participating EPRs (their household balance summary, their share of joint Commitments — not their individual transactions unless they explicitly opted in).

Render path: dashboard query → collective-hub elohim-node → parallel libp2p RPC to each family's local SQL → aggregated result. Hub holds metric-projections only, not source data. If a family revokes membership, their data stops flowing immediately because the hub never held it. **Cash-out is structural.**

## Bridges (legacy interop / cash-out)

- **bridges/plaid/** — bidirectional bank-import bridge: webhook → Observation → graduated Event; user-initiated transfer → bridge → bank-API call (where supported). Cash-out: disconnect Plaid → bridge stops authoring → existing Events stay → user exports to QFX / CSV for legacy import elsewhere.
- **bridges/stripe/** — commerce bridge: payment received → Observation → Event under merchant-account EPR.
- **bridges/banking-api/** — direct bank-API bridges where Plaid is undesired or unavailable; same shape.
- **bridges/quickbooks/** (import-only) — historical-data backfill: 10 years of QuickBooks export → batch-graduated Events under stewardship-elohim signature.

The protocol's commitment is bidirectional legibility: anyone can leave with their data; anyone can join with their history.

## Code anchors

| Surface | Path |
|---|---|
| Shefa pillar Angular services | `app/elohim-app/src/app/shefa/` |
| Account / Transaction views | `elohim/elohim-storage/src/views.rs` (`AccountView`, `EconomicEventView`, `EconomicResourceView`) |
| View schemas | `elohim/sdk/schemas/v1/views/account-view.schema.json`, etc. |
| Shefa pillar manifest | `elohim/sdk/domains/shefa/manifest.json` (action verbs, observation_kinds, classifications) |
| EconomicEvent + Resource entry types | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` |
| Coordinator functions | `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` |
| Plaid bridge (planned) | `bridges/plaid/` |
| Stripe bridge (planned) | `bridges/stripe/` |
| Doorway HTTP routes | `doorway/doorway-service/src/handlers/shefa/` |

## What this proves about the substrate

A skeptical systems architect should walk away from this archetype able to say:

- The Postgres-like primary surface (EPR + Event + Resource projected to local SQL) handles a Mint-shape working set in low-MB at 10-year history depth
- The Kafka-like event flow (REA Events with mass-conservation discipline) replaces transactional bookkeeping without losing finality
- The S3-like asset surface (iroh-blob with content-addressing) handles receipts / scanned documents at near-zero gossip cost
- The federated-query pattern (collective-hub coordinating libp2p RPC across member households) gives multi-party aggregation without centralizing data
- The bridge-and-cash-out pattern means no user is captured by the substrate; legacy migration is incremental and reversible

If those five claims hold for Mint/Monarch, the substrate is real. The other seven application archetypes in this directory test the same claims against patterns with different stress profiles (massive blob, social-graph density, real-time collab, marketplace matching, compute economics, learner trajectories, creator monetization).
