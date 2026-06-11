---
id: "backlog-shefa-sensemaking-surface-session-seed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Shefa own-session seed — the four-surface sensemaking tool over EPRs (Drive × Mint × Analytics × Amazon)"
slug: "shefa-sensemaking-surface-session-seed"
written: "2026-06-11"
author: "operator articulation + session comprehension-test (subject-routing locus arc)"
status: "refined"
priority: "high"
tags: [shefa, lens-model, exchange, storefront, inventories, sensemaking, per-locus-session]
cites:
  - app/elohim-app/src/app/shefa/CLAUDE.md
  - elohim/sdk/domains/shefa/CLAUDE.md
  - genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md
shift_objective: |
  Run the shefa per-locus session (ceremony recipe proven on avodah, 2026-06-11 chronicle): rewrite the
  shefa gospels as the four-surface sensemaking lens, p2p-design-gate the exchange-definition entities
  (storefront EPR, requests-&-offers-admin EPR, wishlist), and reconcile the gaps that gate the Flows
  panel (reach-enum reconciliation → storage-stewardship-summary; commitments 'proposed'→'active'
  junction; conductor-first writes). Verification rules from the avodah pilot apply: substrate citation
  required for category/notarization claims; verifiers use cite tooling, never ad-hoc hashes.
---

# Shefa own-session seed

Operator articulation (2026-06-11, validated against the existing substrate): **shefa is the value/authoring
lens — a sensemaking tool OVER EPRs with FOUR fused surfaces** on one substrate:

1. **Steward (Drive)** — filesystem/namespace + the authoring CMS ("new doc / sheet / car / boat / epr-app").
   **Exchanges are themselves authored EPRs here**: a *storefront EPR* / *requests-&-offers-admin EPR*
   defines an exchange the way a doc defines a document.
2. **Exchange (Amazon/Walmart/real-estate — the "stuff" axis)** — what defined exchanges *offer*; what I
   *offer/want* (requests + **wishlists**). Physical goods, property, services — not just content.
3. **Flows (Mint/Monarch)** — the lens over **my stewarded inventories**: accounts are resource pools
   (pantry/storage, compute, work-capacity, attention, mutual-credit), the register is the economic-event
   ledger, recurring = commitments (hosting, premiums, cadenced chores, compute delegations).
4. **Insights (Analytics)** — value-through-content, reach circulation, standing/demurrage curves, coverage.

**Item detail = the R**: one EPR through its three legs — knowledge (graph edges), value (event history +
gate revenue), governance (reach + claims/observations).

## Hazard rail (already in the pillar gospel)

The inverted CMS: content lives one layer DOWN — shefa authors *into* and projects *from* notarized EPRs,
never owning them. UI consequence: **every number wears provenance** (`dht_anchor_hash` chip; un-anchored
rows read "projection-only"). Cross-lens links on the same primitive (a work-story: "view in avodah" board
card / "view in shefa" contribution event) — one core, two lenses, visible in chrome.

## Existing substrate that lights these surfaces (validated 2026-06-11)

| Exists today | Surface |
|---|---|
| ~3,400 seeded content nodes | Drive (authored/stewarded) |
| `exchange-metadata.schema.json` (offerType, requestType, terms, resourceNature) + `ExchangeApiService` + `README-EXCHANGE` | Exchange — metadata vocabulary already in the domain |
| `ServiceRequest`/`ServiceOffer`/`ServiceMatch` (avodah types — the *process* view of the same exchange) | Exchange (value view) |
| `EconomicEvent` ledger + signal harness; avodah terminal-column events → `work-credit`/`stewardship-standing` | Flows register + contribution |
| `rea_commitments` (incl. `custody-blob`), REA compute-commitment primitive (`delegates-compute`) | Flows recurring |
| imagodei `HostingCostSummary` + `NodeStewardHostingIncome` | Flows bills/income (modeled, parked in imagodei) |
| `peer_blob_inventory` + pantry/stock/draw vocabulary + `NodeCapabilities.cache_budget_bytes` | Flows accounts (storage pool) |
| `PremiumGate`/`GateRevenueSummary`/`RecognitionEventSummary` | Insights creator earnings |
| `InsuranceMutualService` + `BudgetReconciliationService` + `banking-bridge/` | Coverage + settlement |
| `stewardship-context` + demurrage + affinity | Insights standing curves |
| reach (8 values) + earned-reach machinery | Insights circulation |
| resource-nature dimensions (rivalry/excludability/depletability/fungibility/circularity) | "new car / new boat" = EconomicResource + nature + custody context |

## p2p-design-gate items (design in the shefa session, NOT before)

- **Storefront EPR / requests-&-offers-admin EPR** — the exchange-definition entities (category? identity?
  coordinator fn? which DNA?). These make exchanges first-class authored content.
- **Wishlist** — likely agent-scoped (B?) composite of requests; unverified.
- Flows-panel gates: reach-enum reconciliation (gates `storage-stewardship-summary`), commitments seed as
  `proposed` not `active` (resilience-snapshot junction), conductor-first write transition.
