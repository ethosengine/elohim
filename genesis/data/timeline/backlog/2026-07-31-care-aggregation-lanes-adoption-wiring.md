---
id: "backlog-care-aggregation-lanes-adoption-wiring"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Care-aggregation lanes — schema-first slice landed; wire the ceremony through mishpat + aggregator"
slug: "care-aggregation-lanes-adoption-wiring"
written: "2026-07-31"
author: "agentic-developer"
status: "open"
priority: "medium"
area: "shefa"
source: "care-aggregation floor exploration (resiliency card → omni claims → coupling → care economy, 2026-07-31)"
---

**Spec:** `genesis/docs/superpowers/specs/2026-07-31-care-aggregation-adoption-policy-floor-design.md`
(11-seam decomposition ledger in §11 — this entry tracks the wiring campaign that follows the
schema-first slice).

**Landed 2026-07-31 (slice 0, schema layer only — no enforcement change):**
- `elohim/sdk/schemas/v1/enums/signal-variety.schema.json` — the disclosure taxonomy (6 varieties,
  closed set) that had "no home at all" per the grounded floor audit.
- `elohim/sdk/schemas/v1/enums/aggregation-lane.schema.json` — the 4 lane modes; `suppressed` is the
  bare-adoption default for every variety.
- `elohim/sdk/schemas/v1/commitments/adopts-content.schema.json` — the adoption ceremony's wire
  contract: consent (`policy_ref`, absent = bare adoption), epistemic timestamp
  (`epistemic_digest` = claim-state visible at the adopting node), per-variety `bounds.lanes`.
- `scripts/test-adopts-content-schema.mjs`, chained into root `schema:test`.

**Next slices (each independently shippable, spec §3-5):**
1. Mishpat DNA: `adopts-content` arm in `commitment_action_requirements`
   (`mishpat_integrity/src/lib.rs`) + coordinator typed checks — schema above is the contract.
2. `bounds_validator.rs` check 8: lane enforcement at the aggregation boundary; the aggregator
   (`services/aggregator.rs`, currently zero non-test callers) consults the adopting context's
   lanes before emitting.
3. Care observation kind declaration + first care EconomicEvents (`appreciate` is the live bridge).
4. `CoverageRollup` chaining (multi-level composition is already unit-proven) + `witness_quorum`
   transport + aggregates-as-atoms federation + participation-rate metadata in the rollup shape.
5. Identity-coherence preconditions: `humans.household_id` NULL class; hub-id-in-provider
   namespace divergence; `in_scope_of` single-value truncation.

**Why it matters (one line):** private practice must aggregate to commons attribution without
disclosure — the darkweb pattern (millions of private views, zero commons signal) starves both
creator recognition and the substrate's own resiliency machinery.
