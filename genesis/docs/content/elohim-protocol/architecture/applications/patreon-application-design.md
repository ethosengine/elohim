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

A creator opens the app on her laptop. She sees: her patron count, monthly recurring patronage (the "MRR" chart she used to see in Patreon), the recent tier-changes from the past week, the exclusive video she published Tuesday and which patrons have viewed it, comments and reactions from patrons threaded under each post. Beside that, a "creator-fund" balance showing accumulated patronage this month and a "cash-out" button that drains to her bank via the Stripe bridge or shifts into a community-currency Resource.

A patron opens the app on his phone. He sees: the four creators he supports, the tier he's on with each, the exclusive posts he has access to this month, his giving history with a year-over-year chart. When he taps a creator's tier-up button, his next monthly fulfillment Event reflects the new amount; the substrate re-issues his tier-Attestation and the higher-tier reach unlocks immediately.

The app feels like Patreon. It is not Patreon. There is no platform between the patron and the creator. The patron's bank knows nothing beyond the routing line. The creator-patron relationship is direct and the substrate ratchets a manifest-declared sliver into Bridge Commons and Global Commons; nobody else takes a cut.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Creator profile | imagodei Human + Creator-EPR (`content_type: "creator"`) | reach=`commons` for public-facing |
| Tier (e.g., "$5/month") | Commitment (`action: "subscribe"`) from patron to creator | recurring; child of Creator-EPR; D.17 stagger applies |
| Patronage payment | Event (`action: "transfer"`) | fulfills the tier-Commitment monthly; observation_refs to bank-import Observations |
| Exclusive content | EPR (`content_type: "post"` or `"video"`) with reach gated by tier-Attestation | only patrons with matching Attestation see it |
| Patron (as Membership) | Membership EPR of creator's Collective with `tier_metadata` | tier-Attestation issued on first payment, renewed monthly per D.1 subordination |
| Patron-only community | Collective EPR (`content_type: "qahal"`) with reach=patron-tier | patron-tier Attestation required for entry |
| Creator-fund | EconomicResource (`resource_classified_as: "currency-USD"` or `"currency-community"`) | balance derived from incoming Events per D.4; `parent_epr_cid = creator_epr_cid` |
| Tier change | Event (`action: "modify-subscription"`) | cancels old Commitment, authors new one + new tier-Attestation |
| Cancel | Event (`action: "cancel-subscription"`) | Commitment closes; tier-Attestation lapses; reach to exclusive content revokes per D.9 |
| Bridge Commons + Global Commons accumulation | EconomicResource on Commons EPRs | manifest-declared fee_splits per Event; mass-conservation enforced per D.20 |

Eight primitives, ~8 discriminator values across `action` and `content_type`, no special-casing for creator monetization. Tier-Membership is just D.1 subordination of a Membership EPR under the creator's Collective; exclusive content is just D.9 reach-mutation gated by an Attestation.

## How one patronage fulfillment flows

Patron Lin signed up for Mira (a documentary filmmaker) at the $10/month "behind-the-scenes" tier on the 6th of last month. It's the 6th again today:

```
1. shefa-elohim scheduler on Lin's node detects:
       Commitment(action="subscribe", provider=lin, receiver=mira,
                  state="accepted", due=2026-05-06T00:00:00Z)
   in `rea_commitments`. D.17 stagger discipline applies:
       blake3(commitment_cid || lin_agent_id || billing_epoch=202605) % 3600s
       = 1742-second delay (offset within the 1-hour stagger window)
2. At 00:29:02 UTC, the scheduler fires:
       Event(action="transfer", provider=lin's currency-USD Resource,
             receiver=mira's creator-fund Resource, quantity=10.00,
             parent_epr_cid=lin's account_cid,
             fee_splits=[{bridge_commons: 0.05}, {global_commons: 0.05}],
             observation_refs=[<Stripe bridge confirmation observation>])
3. Substrate-floor mass-conservation validator runs on the integrity zome:
       sum(fee_splits) + receiver_amount == total_authored_amount
       (10.00 = 9.90 + 0.05 + 0.05 — green per D.20)
4. DHT write — ~2 KB Event entry; validated by neighborhood quorum;
   EventFulfillsCommitment link from Event → Commitment authored
5. ReconcileController on Mira's node consumes the libp2p delta:
       UPDATE mira's creator-fund Resource derived view: +$9.90
       UPDATE Commitment state to 'fulfilled' (cycle complete)
       INSERT new Commitment for next cycle (state='accepted', due=2026-06-06)
       UPDATE epr_event_edges adjacency (parent=lin's account_cid → Event)
6. shefa-elohim on Lin's node authors a fresh Attestation:
       Attestation(content_type="attestation:patron-tier",
                   subject_cid=mira_collective_cid,
                   metadata_json={tier:"behind-the-scenes", expires:2026-06-06})
   reach=`agent-private` (only Lin and Mira's reach-gate evaluator see it)
7. Mira's exclusive-content reach-gate evaluator detects the renewed Attestation;
   patron-tier content remains visible to Lin for another cycle
8. Within the 1-hour stagger window, ~278 patrons/sec land in Mira's projection
   — manageable inbound; her dashboard's "new payment" toast queue is paced,
   not flooded
```

Cash-out: when Lin disconnects, an `Event(action="cancel-subscription", ...)` closes the Commitment; future Events cease; the historical Events stay in the substrate as Lin's giving record. When Mira wants to convert her creator-fund balance to legacy money, she authors `Event(action="cash-out", provider=mira's creator-fund, receiver=mira's legacy-USD-bank-account, observation_refs=[<Stripe payout confirmation>])` via the Stripe bridge — itself a D.8 bridge-Collective whose Commons takes its declared share and ratchets through D.20.

## Storage footprint per creator (1k–100k patrons)

| Item | Count | Size | Total |
|---|---|---|---|
| Creator-EPR + Collective-EPR + tier Commitments authored | ~10 | 2 KB | 20 KB |
| Patron Membership EPRs (one per active patron) | ~10k | 1 KB | 10 MB |
| Patron-side subscribe Commitments (visible via reach scope) | ~10k | 1.5 KB | 15 MB |
| Monthly patronage Events (10k patrons × 12 months × 3 yr) | ~360k | 500 B | 180 MB |
| Tier-Attestations (10k patrons × 36 monthly renewals) | ~360k | 800 B | 288 MB |
| Exclusive content EPRs authored | ~500 | 5 KB | 2.5 MB |
| Creator-fund EconomicResource | 1 | 1 KB | 1 KB |
| **Total local SQL projection on creator's node** | | | **~495 MB** |
| Cold archive (Events + Attestations >2 yr in quilt) | — | — | ~200 MB residue |

**Fits on a laptop with room to spare.** A patron's projection is ~50× smaller (~10 MB) — they hold their own giving history plus the creators they support, not the creator's full patron base.

## Network bandwidth profile

- New patronage Event (one patron, one cycle): ~2 KB DHT write + neighborhood gossip ≈ ~20 KB total peer load
- Creator-side monthly inbound across 10k patrons: 10k × 20 KB ≈ 200 MB/month
- libp2p sync plane is cursor-driven: a creator's node delta-syncs ~278 Events/sec during the 1-hour stagger window; sustained sub-MB/sec
- A creator with 1M patrons handles ~278/sec inbound at distribution peak — well under what a single household-class node sustains
- **Per-patron bandwidth: <100 MB/month** (the patron's own subscriptions + the exclusive content they consume)
- **Per-creator bandwidth: 200 MB/month per 10k patrons** (linear in patron count, but bounded by D.17 stagger so peak ≈ average)

## DHT entry impact

- 10k Events/year/creator × 10k creators × 100k patrons = 10¹⁰ Events/year IF every creator went global; but reach for a patronage Event is *bilateral* — patron + creator + their immediate elohim agents
- Per-peer DHT entry visibility: a patron sees their own ~50 subscriptions × 12 = 600 patronage Events/year + the creators' public-facing entries; a creator sees their own ~10k patrons' bilateral entries
- 1M-patron creator: bilateral Events distribute across the patron graph; the creator's node does not hold all 1M as DHT-validated copies — it holds projections via reach-scoped libp2p sync. The DHT entries themselves are sharded across each patron's neighborhood
- Typical creator-node DHT footprint: ~30k entries (own Commitments + Attestations + exclusive content + Collective metadata) — well inside the ~3k-per-cell-per-neighborhood validator budget after sharding

## Why the dashboard renders fast

- **MRR chart**: `SELECT SUM(c.resource_quantity_value) FROM rea_commitments c WHERE c.receiver = :creator_agent_id AND c.action = 'subscribe' AND c.state NOT IN ('cancelled', 'breached')`. Indexed on `receiver + action + state`. Sub-millisecond.
- **Recent patron-tier changes**: `SELECT * FROM economic_events e JOIN epr_event_edges ed ON ed.child_event_cid = e.cid WHERE ed.parent_epr_cid = :creator_collective_cid AND e.action IN ('modify-subscription', 'cancel-subscription') ORDER BY e.observed_at DESC LIMIT 50`. D.1 adjacency index makes this an indexed read.
- **"Who has access to this post"**: `SELECT m.patron_cid FROM memberships m JOIN attestations a ON a.subject_cid = m.collective_cid WHERE a.content_type = 'attestation:patron-tier' AND a.metadata_json->>'tier' >= :required_tier AND a.expires_at > now()`. Reach-gate is local SQL, not a network call.
- **Creator-fund balance**: `SELECT SUM(e.resource_quantity_value) FROM economic_events e WHERE e.receiver = :creator_fund_resource_cid` — derived per D.4. Zero entries beyond the existing Events.
- **MRR forecast (next 30 days)**: `SELECT SUM(c.resource_quantity_value) FROM rea_commitments c WHERE c.receiver = :creator_agent AND c.state = 'accepted' AND c.due BETWEEN now() AND now() + interval '30 days'`. Forecasting is the Commitment table; no separate forecasting service needed.

## Why 1M patrons don't melt the creator's node

The thundering-herd problem is the obvious failure mode. Without stagger discipline, 1M patrons all billed on the first-of-month would land 1M Events on the creator's projection in seconds — a denial-of-service against the creator's own node. D.17's BLAKE3-hash stagger distributes those 1M Events uniformly over a 1-hour window (manifest-declared per `action: "subscribe"`):

```
fulfillment_delay = blake3(commitment_cid || agent_id || billing_epoch) % 3600s
```

- 1M patrons ÷ 3600s = ~278 Events/sec peak; sustained for an hour
- Each Event is ~2 KB; ~556 KB/sec inbound; trivial for a household-class node
- Creator's "new patronage" toast feed displays at human-readable pace
- ReconcileController's projection update is batched (every ~5s); SQL writes happen in bulk inserts
- Even a creator with 10M patrons (top-percentile celebrity scale) sees ~2780 Events/sec — still tractable; the stagger window can be widened in manifest to 6 hours for that scale

The patrons themselves don't see latency because their Commitment fires asynchronously; the patron's UX is "your $10 went out on the 6th" — not "watched the spinner for 30 seconds at midnight."

## Dissolution in practice

- Patron cancels → `Event(action="cancel-subscription", subject=commitment_cid)` → Commitment state transitions to `cancelled` per A.5; tier-Attestation expires at month-end without renewal; reach to exclusive content revokes per D.9 reach-mutation Event
- Recurring billing failure (patron's bank declined) → ReconcileController detects cursor-stuck on Stripe-bridge observation graduation; shefa-elohim authors a `notify-payment-update-needed` Attestation surfaced to patron; after manifest-declared retry window (3 cycles default), Commitment transitions to `breached`; tier-Attestation lapses; auto-downgrade to free-tier reach
- Creator retires from the platform → `Event(action="dispose", subject=creator_collective_cid)` → existing patron Memberships transition to historical view; queued exclusive-content remains accessible to patrons-of-record for a manifest-declared sunset period (default 30 days), then transitions to `closed`
- Closed Commitments + their fulfillment Events stay queryable as a patron's giving history forever; the substrate-floor invariant prevents new Events targeting a `closed` Commitment

## Where agentic intelligence carries the load

- **Without shefa-elohim**: monthly patronage requires manual patron-side confirmation — chase emails, lapsed cards, abandoned subscriptions. Patreon needs a centralized service to keep this together. **With shefa-elohim**: scheduled Commitment-fulfillment Events fire automatically per D.17 stagger; observation-graduation failures (Stripe webhook stuck) detected via cursor-stuck signal; patron and creator notified with actionable context — "Lin's last payment didn't go through; here's the update-payment link."
- **Without curation-elohim**: creator manually decides what tier sees what post — laborious; high-tier patrons feel undervalued when the creator forgets. **With curation-elohim**: elohim suggests tier-targeting based on patron-engagement Events (which posts got reactions, which trigger churn vs. tier-ups); cross-creator collaboration orchestration ("you and Mira's tiers overlap 40% — collaborate?").
- **Without commons-stewardship-elohim**: Bridge Commons and Global Commons accumulation is opaque to the creator; mass-conservation invariant is just a validator rule. **With commons-stewardship-elohim**: creator sees a small "where your fees go" surface — public-good allocation Events from Global Commons; tangible accountability for the 0.5% × 2 that the substrate ratchets.
- **Care-class isolation invariant** (per D.18): patron-engagement signals (`endorse`, `comment`, `react` on the creator's content) are `signal_class: "care"` — they affect care_standing only. A patron's reliability as a creditor (`signal_class: "compute"` debit on a bad-compute provider, separately) never mixes with their care-economy reputation. A creator who breached a custody Commitment elsewhere does not get downgraded as a patron-care signal-receiver.

This is the value-prop unlock: the recurring-promise economy is tractable only because elohim agents fire, monitor, and surface state transitions at machine-speed. Human-only operational overhead consumes the value before it reaches the creator.

## What the cross-creator / collective view shows

Patrons commonly support multiple creators in an aesthetic neighborhood (independent journalism, niche music, regional documentaries). A Collective EPR can reference multiple Creator-EPRs as Memberships — a "creator collective" pattern (think a regional journalism cooperative).

Render path: a patron's "creators I support" view federates across the Creator-EPRs via reach-scoped libp2p RPC; aggregates their monthly billing schedule; surfaces a unified "next-30-days outflow" chart. **No data replication — federated query through the patron's own node.** If a creator leaves the collective, their data stops appearing immediately because the hub never held it.

For a creator-cooperative pooling-fund pattern (per the bridges/valueflows/ reference), patron contributions can flow into a Collective Commons (per D.20) governed by the participating creators' Memberships. Allocation Events distribute the pool by manifest-declared formula. **The Cybersyn move**: each creator sees the cooperative's full economic picture without anyone holding sole authority over the data.

## Bridges (legacy interop / cash-out)

- **bridges/stripe/** — patron's credit card → Stripe → webhook → Observation → graduated patronage Event under stripe-bridge-stewardship-elohim signature (per D.8). Stripe Inc itself is a substrate-native Collective EPR; its commercial revenue flows into its Bridge Commons per D.20. Parallel operation during transition: patrons can use Stripe-card or direct-substrate-currency; the Event shape is identical.
- **bridges/patreon/** (import-only) — Patreon Takeout JSON export → batch-graduated patronage Events under stewardship-elohim signature. A creator migrating brings their 10-year patron base; substrate-native Events reflect the historical relationship; patrons re-authorize via OAuth and the Stripe bridge picks up the monthly recurring without break.
- **bridges/paypal/** — same pattern; PayPal Inc is its own substrate-native Collective EPR with its own Bridge Commons.
- **Cash-out (creator side)**: creator's accumulated creator-fund balance transfers to legacy bank via `Event(action="cash-out", provider=creator_fund, receiver=legacy-USD-bank, observation_refs=[Stripe payout confirmation])`. The cash-out Event itself ratchets through D.20 (Bridge Commons + Global Commons take their declared share); the creator receives the post-fee amount in their legacy bank.
- **Cash-out (patron side)**: patron disconnects bridge → `Event(action="revoke-authorization", subject=stripe-bridge-stewardship-commitment)` → Stripe bridge stops authoring new Observations; existing Events stay in the substrate as the patron's giving history; future Commitments transition to `cancelled` per D.7 disposal.

The protocol's commitment is bidirectional legibility: anyone can leave with their data; anyone can join with their history.

## Code anchors

| Surface | Path |
|---|---|
| Patreon-shape recurring + creator-fund surfaces | `app/elohim-app/src/app/shefa/` (`services/economic-events-api.service.ts`, `services/flow-planning-api.service.ts`, `services/budget-reconciliation.service.ts`) |
| Exclusive-content rendering | `app/elohim-app/src/app/lamad/` (`services/content.service.ts`, `services/blob-cache-tiers.service.ts`) |
| Subscription Commitments + tier-Attestations | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (Commitment with `action: "subscribe"`; Attestation with `content_type: "attestation:patron-tier"`) |
| Recurring scheduler with D.17 stagger | `elohim/elohim-storage/src/services/commitment_scheduler.rs` (planned per D.17) |
| Patron-membership-as-subordinate | `elohim/elohim-storage/src/services/reconcile_controller.rs` (D.1 adjacency projection) |
| Reach-gate evaluator | `elohim/elohim-storage/src/services/reach_gate_service.rs` (planned per D.9) |
| Stripe bridge | `bridges/stripe/` (planned per D.8) |
| Patreon import bridge | `bridges/patreon/` (planned per D.8) |
| Shefa manifest declarations | `elohim/sdk/domains/shefa/manifest.json` (action verbs: `subscribe`, `transfer`, `modify-subscription`, `cancel-subscription`, `cash-out`; bridge_kinds: stripe, patreon, paypal; friction_gradient tiers) |
| Bridge Commons + Global Commons EPRs | `elohim/sdk/domains/elohim/manifest.json` (Commons EPR declarations per D.20) |
| Doorway HTTP routes | `doorway/doorway-service/src/handlers/shefa/` (patronage flows + reach-gated content delivery) |

## What this proves about the substrate

A skeptical systems architect should walk away from this archetype able to say:

- The Stripe-Subscription primary surface (Commitment + fulfilling Event with D.17 stagger) handles Patreon-shape recurring billing at 1M-patron scale without thundering-herd, without a centralized scheduler, without a platform-mediated trust anchor
- The reach-gating mechanism (D.9 reach-mutation gated by D.6 elohim-authored Attestation) replaces platform-side ACLs with substrate-native access control that the patron's node enforces locally — no platform server to capture
- The cash-out structural property (D.8 bridge pattern + D.20 Layered Commons ratchet) means both sides can leave the substrate at any time with their accumulated value and full event-history portable; legacy migration is incremental and reversible
- The Commons fee mechanism (D.20) replaces "platform takes 10–15%" with manifest-declared, anti-concentration-ratcheted, elohim-mediated public-good flows — value flows directly creator-patron with a substrate-native sliver into public goods, governed by elohim-councils whose authority is structurally non-extractive
- The agentic-intelligence layer (shefa-elohim scheduler + reach-gate evaluator + commons-stewardship-elohim) is what makes the promise-economy tractable at human scale — recurring patronage is fire-and-forget for the patron, sense-and-respond for the creator, machine-witnessed for the substrate

If those five claims hold for Patreon-shape creator monetization, the substrate's commitment to "value flows like water, no extraction" is real at the surface where the prior incumbents (Patreon, Substack, Ko-fi, OnlyFans) most visibly took a cut.
