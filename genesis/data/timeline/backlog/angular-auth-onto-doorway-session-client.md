---
title: Migrate Angular auth.service.ts + doorway-app auth-state.service.ts onto @elohim/identity DoorwaySessionClient
created: 2026-06-10
domain: imagodei (app auth surface; SDK consolidation follow-up)
source: arc plan Task 1.3 capture (2026-06-10)
severity: medium
---

`DoorwaySessionClient` (landed a91dc88/f205533ac, 67 tests) is now the SDK home
for the doorway auth surface; the a2o framework migrated onto it (0026de6b1,
two-stage reviewed). Remaining hand-rolled duplicates: `app/elohim-app/src/app/
imagodei/services/auth.service.ts:46-63` (AuthProvider registry + token
refresh timer + localStorage persistence) and `doorway/doorway-app/src/app/
services/auth-state.service.ts`. Migration = wrap the framework-free client in
an Angular adapter (tokenStore backed by localStorage, fetchImpl from the
platform), keep the AuthProvider abstraction at the edge. Bigger blast radius
than a2o (production login flows, refresh timers, SSR) — own plan, not a task
rider. Connection-matrix rails apply (arc spec §client-connection-matrix):
trustMode stays discovered-from-/auth/me; no strategy logic enters the client.
