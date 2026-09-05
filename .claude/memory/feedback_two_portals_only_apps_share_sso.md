---
name: feedback_two_portals_only_apps_share_sso
title: Two portals only — native + doorway; apps share them like SSO
description: "Operator decision 2026-09-05 — the ONLY sign-in portals are the p2p-native portal (stewards) and the doorway portal (hosted humans); every app is an OAuth relying party of them, never its own portal — bites on any in-app login/register form"
metadata: 
  node_type: memory
  title: Two portals only — native + doorway; apps share them like SSO
  type: feedback
  originSessionId: e0407b6c-164b-489a-aa86-da20d9fa1b8b
  modified: 2026-09-05T00:27:39.084Z
---

The operator ruled (2026-09-05, during the auth-portal review): there are exactly two portals — the **p2p-native portal** (the steward's own runtime: `app/imagodei-portal/` bundle over elohim-storage `/auth/me`, `/session`, `/session/exchange`) and the **doorway portal** for hosted humans (`doorway-app` under `/threshold/*`). Apps (elohim-app, lamad, doorway-app, any EPR app) **share those portals like SSO**: they discover where to sign in from `/.well-known/elohim-auth`, redirect there with OAuth params, and consume the code on callback. An app never owns a password field or a registration form.

**Why:** the tree already declared "ZERO third portal" (login.component.ts, auth_routes.rs handle_login comment) but elohim-app still carried its own password sign-in card and a registration page posting straight to `/auth/register` — a de facto third portal that duplicated the doorway's, drifted (snake_case keys, conductor-socket gating), and split the human's story across surfaces.

**How to apply:** retire in-app password/register paths (PasswordAuthProvider.register, register.component, the `allow-password` login card) in favour of redirect-to-portal + OAuth callback; keep only the "which doorway" resolver step in the app; profile enrichment (bio, affinities, reach) happens in the app AFTER SSO, not at registration. Any new app is a registered OAuth client with an allowed callback (keeps the hostile-callback proof in oauth-authorization-code.feature). Related: [[project_alpha_auth_portal_baseline_2026_09_04]], [[feedback_p2p_vs_federation_layer_vocabulary]].
