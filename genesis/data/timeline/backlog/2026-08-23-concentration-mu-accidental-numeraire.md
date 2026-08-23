---
id: "backlog-concentration-mu-accidental-numeraire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The concentration service's mu (mean balance) has no locality column — two communities on one storage node share one relational scale nobody set"
slug: "concentration-mu-accidental-numeraire"
written: "2026-08-23"
author: "monetary-posture research pass (mint)"
status: "backlog"
priority: "medium"
tags: [economics, numeraire, concentration, elohim-storage, research-mint]
cites:
  - elohim/elohim-storage/src/services/concentration_service.rs
---
# The accidental numéraire

`mu` (mean balance) at `elohim/elohim-storage/src/services/concentration_service.rs:47` carries **no
locality column**. Two communities co-resident on one storage node therefore share a single relational
scale that neither of them set, and that nobody authored.

## Why it matters more than its size suggests

The standing operator decision is that the substrate stays **agnostic to the measure applied** — we
adopt no numéraire. But the mirror failure is equally live, and this is it: *refusing to author a rate
while shipping a default one.* A fallback nobody chose is harder to contest than one somebody did.

This is a **defect repair, not a feature**.

## Fix shape (from the 2026-08-07 audit, unchanged)

1. Add `collective_cid` to the scope key of `concentration_snapshots`, `responsibility_demand_configs`
   and the corresponding manifests.
2. Give `ExchangeRateView` the `CollabAgreement` shape — bilateral, steward-counter-attested,
   negotiated.
3. **Delete the `"algorithm"` source variant.** An algorithmically-derived exchange rate is precisely
   the authored-by-nobody default the corpus refuses.

Note the countervailing find: `share_routing.rs:65-69` already hard-refuses to derive an allocation.
The refusal is correct and in place; this is the seam where a default slipped past it anyway.

**Standalone, not folded into a cluster**: operationally-atomic per `CLUSTERS.md`.

Minted from [the monetary posture](epr:monetary-posture-internal-currencies-external-fiat-2026-08-23) §2.7 and
[the succession evidence bridge](epr:succession-without-conquest-mutualist-lineage-2026-08-23) §2.6.3.
