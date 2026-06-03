---
title: seed-test-admin.ts sends admin_bootstrap_key (snake) — wire field is adminBootstrapKey (camel)
created: 2026-06-03
domain: D-identity
source: admin-key usage review (shift a2o-e2e-household-greenup)
severity: low
---

`genesis/seeder/src/seed-test-admin.ts:235,246` reads `API_KEY_ADMIN` and sends
`admin_bootstrap_key` (snake_case) in the `/auth/register` body. `RegisterRequest`
is `#[serde(rename_all="camelCase")]` (auth_routes.rs:110) with no field override,
so the wire field is `adminBootstrapKey` — the snake_case key is silently dropped
and the tool's minted "admin" never gets Admin promotion. seed-humans.ts already
sends the correct camelCase form. One-char fix; not on the e2e critical path
(seed-test-admin is a standalone key-minting tool), hence backlog not shift-scope.
