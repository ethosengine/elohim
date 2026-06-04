---
title: "Stewardship-grant surface for fixtures — portal-handoff e2e scenarios design-blocked on it"
created: 2026-06-04
domain: "code"
relatedNodeIds:
  - "memory:project_peer_native_account_canonical_surface"
tags: [doorway, auth, stewardship, a2o, portal-handoff, p2p-design-gate, code-domain]
shift_objective: |
  All three runnable steward-login-portal-handoff scenarios fail/pend on one gap: no
  legitimate surface grants a fixture persona is_steward=true. The only existing path is
  POST /auth/confirm-stewardship (Ed25519 key-possession proof + conductor deprovisioning
  — destructive to every other scenario using the persona). The Given step registers a
  portal host best-effort but UserDoc.is_steward stays false → /auth/login omits isSteward
  (serde skip_serializing_if) → wire assertions get undefined → sc1+sc3 fail; sc2's
  redirect can't fire (threshold-login only redirects when the login response carries
  portalHostUrl, which requires is_steward). DESIGN DECISION needed (p2p-design-gate —
  stewardship standing is substrate truth, Mongo is a projection): candidates (a) admin
  PUT /admin/users/{id}/steward (operator-authed; must document that it bypasses the
  key-proof invariant — footgun for hosted users), (b) account-package/seed flag honored at
  registration for fixture personas only, (c) derive steward standing from substrate
  (node-registry attestation / portal-host registration with key-backed proof) instead of
  the Mongo flag — protocol-correct home. Done when sc1-sc3 of
  steward-login-portal-handoff.feature pass on a fresh genesis run.
---

Discovered during shift 2026-06-04T14-52-post-merge-shakeout-e2e-greenup iteration 3
(genesis#1088, first run of the merged scenarios). Sprint A documented the gap in the
Given step ("Registration is best-effort — until the GAP-1 surface accepts it").

Secondary piece (same feature, smaller): in playwright stage-mode the portal-handoff
scenarios never OPEN a PlaywrightDevice (the Given builds only an HTTP BrowserDevice), so
the three navigation Then-steps return 'pending' even when the stage is browser-mode —
attach a PlaywrightDevice in the Given when world.deviceMode==='playwright' (steps-glue,
in-scope, but pointless until the grant surface exists since the redirect can't fire).
Also note: the example portal host https://matthew.steward.example does not resolve in CI
— the redirect assertion must intercept the attempted navigation, not await a load.
