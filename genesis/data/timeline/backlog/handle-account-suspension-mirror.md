---
title: handle_account performs no is_active check — suspended user with valid JWT sees Active until token expiry
created: 2026-06-11
domain: D8 (doorway auth surface)
source: auth-wire plan Task 4 review (commit f5a8d86a7)
severity: low
---

`/auth/me` rejects suspended users (`session_revoked_by_user_doc`,
auth_routes.rs:1813-1893, 401 ACCOUNT_SUSPENDED) and login requires
`is_active: true` (:1572) — but `GET /auth/account` (`handle_account`,
:1939-2110) performs NO is_active check, so a user suspended mid-session with
a still-valid JWT loads the account page (and its now-constant "Active" badge)
until expiry. One-conditional fix: mirror `session_revoked_by_user_doc` in
handle_account — which also makes doorway-account.component's comment ("a
successful bearer-authenticated account probe implies active") true instead of
aspirational. Pre-existing server behavior, surfaced (not introduced) by the
T4 wire-truth remap. Sibling nits from same review: dev-mode synthesis returns
conductor_id None (cosmetic "runs own conductor" for everyone in dev);
doorway-admin.service.ts:425 inline session-token shape could reuse generated
SessionTokenResponse.
