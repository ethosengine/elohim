---
id: "backlog-security-doorway-logout-token-version-revocation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway logout is a server-side no-op — wire token_version revocation (force-logout already increments it; nothing reads it)"
slug: "security-doorway-logout-token-version-revocation"
written: "2026-06-07"
author: "tackle-top-three investigation (wf_e3cc3753-f1a)"
status: "open"
priority: "medium"
tags: [security, doorway, auth, logout, revocation, token-version, jwt]
cites:
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/routes/admin_users.rs
  - doorway/doorway-service/src/db/schemas/user.rs
---

# Server-side logout/force-logout revocation is unwired

Surfaced while fixing the genesis #1105 suspend-revocation bug (the
`identifier`-keyed lookup fix in `handle_me`/`handle_account`). The
SUSPENSION (`is_active`) half of revocation now targets the right account;
the LOGOUT half remains structurally absent:

- `handle_logout` (auth_routes.rs) is a no-op: "logout is handled
  client-side". A captured/held JWT remains valid until expiry.
- Admin force-logout (`admin_users.rs` ~1201-1209) already `$inc`s
  `UserDoc.token_version` — but **no verification path reads it**, so
  force-logout doesn't actually invalidate anything.
- `UserDoc.token_version` exists (db/schemas/user.rs:158) and
  `UserDoc::new` initializes it to 1.

## The fix shape

1. Issue JWTs carrying the account's current `token_version` (new claim,
   `#[serde(default)]` for legacy tokens).
2. `handle_me` / `require_admin` (the same durable-state check that now does
   the `identifier`-keyed `is_active` lookup) also compare
   `claims.token_version` vs `UserDoc.token_version` — mismatch ⇒ 401.
3. `handle_logout` `$inc`s `token_version` for the authenticated account —
   logout becomes a real server-side invalidation; force-logout starts
   working for free.
4. Keep the degrade-to-JWT-only arm when Mongo is unavailable (existing
   posture), and keep all of it dev_mode-INDEPENDENT.

a2o coverage: the `auth-lifecycle.feature` "identity check should fail with
unauthorized" assertion text already speaks of "after logout" — extend the
scenario (or add a sibling) to exercise real logout invalidation once wired.
