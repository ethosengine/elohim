---
title: "Landing SPA lost its protocol affordance when the per-surface badge was replaced by the shell omni"
created: 2026-06-04
domain: "product"
tags: [landing, protocol-omni, dogfood, a2o, elohim, product-domain]
shift_objective: |
  ProtocolSignalBadgeComponent was deleted 2026-05-20 (980ea505d) in favor of the
  shell-mounted ProtocolOmniComponent — correct for elohim-app surfaces (omni scenarios
  pass), but the STANDALONE landing SPA blob is a separate artifact that never mounts the
  shell, so the landing page now has no protocol affordance. The dogfood scenario "The
  protocol-signal badge renders on the landing page" (now @wip) correctly caught the
  regression. Decide: (a) embed ProtocolOmniComponent (or a lightweight variant) in the
  landing SPA build, (b) re-add a minimal badge to the landing surface only, or (c) drop
  the requirement and rewrite the dogfood scenario. Then update the scenario to the chosen
  surface's testid and un-wip. Done when landing-page-dogfood passes on a fresh blob.
---

Verified during the 2026-06-04 local shakeout (git history + zero-grep for the testid +
console artifacts showing the blob page loads but the selector cannot exist in ANY fresh
build). Distinct from the stale-blob operator item — a blob refresh alone cannot fix this.
