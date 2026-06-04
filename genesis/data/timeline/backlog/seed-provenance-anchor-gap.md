---
title: "Seed pipeline provenance-anchor gap — bulk/HTTP-created content invisible behind require_provenance"
created: 2026-06-04
domain: "code"
relatedNodeIds:
  - "memory:project_local_stack_dht_anchor_gap"
tags: [seeder, provenance, dht-anchor, a2o, content, code-domain]
shift_objective: |
  Bulk-seeded and HTTP-created content carries no dht_anchor_hash/p2p_published_at, so the
  require_provenance=true filter on GET /db/content hides it — failing a2o scenarios
  "Value-scanner content has multiple stewards", "Stewardship reflects human affinities"
  (tag queries return empty) and plausibly content-lifecycle "Read own content" (404 on
  read-back of API-created content). Confirmed on alpha CI (genesis#1087), not just local
  stack. Fix home per the memory: an explicit anchor step in the seed/import pipeline
  (seeder writes a synthetic/derived anchor via a storage-accepted input field —
  CreateContentInputView/UpdateContentInputView need dhtAnchorHash), NOT a gate bypass.
  Design question to settle first (p2p-design-gate): should require_provenance exempt
  creator-scoped reads ("Read own content" semantics)? Done when the four scenarios pass
  on a fresh genesis run.
---

Discovered during shift 2026-06-04T14-52-post-merge-shakeout-e2e-greenup (iteration 1-2
investigation of genesis#1087). Out of the shift's scope (genesis/seeder not in
objective.scope.paths). Evidence chain: http.rs:3361 (require_provenance=true) →
content_diesel.rs:270-278 (anchor-or-published filter) → CreateContentInputView
(elohim-views/src/lamad.rs:108) has no anchor field; seeder never anchors.

Related (same seeder home, capture together):
- **Seeder idempotency partial-write**: seed-stewardship.ts skips content with ANY
  existing allocation (per-content guard, not per-steward), so a mid-batch failure leaves
  ratio sums < 1.0 forever ("Allocation ratios sum to ~1.0" fails at 0.800 — eve 0.50 +
  nancy 0.30 present, matthew 0.20 missing). Fix: per-steward completeness check before
  skip, or idempotent upsert on (contentId, stewardPresenceId).
- **"Susan" persona mapping latent failure**: stewardship-allocation.feature line 19 says
  "Susan should be listed as a steward" but no presence is named Susan
  (value-scanner slot maps to jessica-spouse). Persona-rename-cascade class. Will fire as
  soon as the tag query is fixed. Either the feature line should say Jessica (feature edit
  = fixture change, operator-gated) or DISPLAY_NAME_TO_PRESENCE gains Susan→jessica-spouse
  (only if Susan is canon somewhere upstream — verify against humans.json first).
