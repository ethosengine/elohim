---
id: "backlog-limitarian-governor-v1-followons"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Limitarian governor v1 follow-ons — the spec §11 explicit deferrals + arc discoveries"
slug: "limitarian-governor-v1-followons"
written: "2026-06-10"
author: "claude-fable-overnight-build-leg-1"
status: "documented"
priority: "medium"
ci_status: documented
jobs: []
relatedNodeIds: []
tags: [governor, limitarian, shefa, mishpat, attention-substrate, follow-on, overnight-arc]
cites:
  - genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md
  - genesis/docs/superpowers/plans/2026-06-10-limitarian-governor-v1-plan.md
  - genesis/a2o/features/shefa/limitarian-governor.feature
---

# Limitarian governor v1 — follow-ons

The v1 slice landed 2026-06-10 (overnight build leg 1): measure module,
continuous decay rate, concentration_snapshots + Phase-A aggregator,
LimitGradientRegistry + CID fix, DNA wall validator, ratify-limit-gradient
governance kind + dead-seam writeback, convergence/anti-capture/firewall
tests, story-first a2o pin. These are the explicit deferrals (spec §11) plus
arc discoveries — each queued, none orphaned.

## Spec §11 explicit deferrals
1. **Aggregate-tick scheduler + HTTP poke route** — v1 is reflexive-sensing,
   harness-driven. The poke route also un-@wips the a2o scenarios.
2. **`limit-gradient` Manifest home** (Phase-3.5; needs create_manifest
   authority gating) — v1 governs via the Commitment carrier only.
3. **Reach gradient** — needs a new erosion path; out of scope by design.
4. **Multi-dim substrate_signal** + cross-collective GE aggregation.
5. **Structural apex non-accumulability** (spec §Decision 1 — operator).
6. **Step definitions for the three @wip a2o scenarios** (zome-call probe
   steps + the governance-action proposal step for the new kind).

## Arc discoveries (new work, owned here)
7. **Spec §11 convergence-test erratum**: the spec's verbatim test asserts
   `top < 1e9` under inflow c=0.20 > k_max — impossible in the saturated
   regime by the spec's OWN §4.2 correction (absolute balances diverge;
   closure lives in the scale-invariant C-series). The landed test asserts
   share-convergence + C-descent-to-target (main run) and absolute
   boundedness under c=0 (secondary run). Backfill an erratum note to §11.
8. **Ratified-gradient params beyond dignity_floor are not yet consumed**:
   apply_ratification stamps the dead columns + dignity_floor; the full
   gradient params (c_target, k_max, …) from a ratified payload still need a
   read path (query_effective_limit_gradient over the ratified row) replacing
   LimitGradientRegistry::core_default at the decay call sites — the spec's
   "ratified row if ratified_by set, else core default" lookup, v1.5-sized.
9. **substrate_signal-keyed distributions**: the aggregator computes over
   token balances (spec §11.3); switch the Phase-A input to per-substrate
   EconomicEvent.substrate_signal distributions once HTTP-path origination
   lands (cluster-3 open decision, operator ceiling).

## Status
`documented` — items 1-6 deliberate deferrals; 7-9 discovered during the arc.
