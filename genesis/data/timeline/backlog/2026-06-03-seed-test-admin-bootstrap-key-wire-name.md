---
title: seed-test-admin.ts sends admin_bootstrap_key (snake) — wire field is adminBootstrapKey (camel)
created: 2026-06-03
resolved: 2026-06-11
status: RESOLVED
domain: D-identity
source: admin-key usage review (shift a2o-e2e-household-greenup)
severity: low
---

**RESOLVED 2026-06-11** (jenkins-seed-bearer-gate plan, Task 3): the body field is now
`adminBootstrapKey` (camelCase) in `seed-test-admin.ts:registerAuthCredentials`. This became
load-bearing once the doorway seed routes bearer-gate (Task 1) — the `jenkins-ci` CI account
provisioned by this tool MUST actually receive Admin, or the authenticated seed run 403s. Verified
via `tsc --noEmit` + full vitest suite (302 passing).

---

`genesis/seeder/src/seed-test-admin.ts:235,246` reads `API_KEY_ADMIN` and sends
`admin_bootstrap_key` (snake_case) in the `/auth/register` body. `RegisterRequest`
is `#[serde(rename_all="camelCase")]` (auth_routes.rs:110) with no field override,
so the wire field is `adminBootstrapKey` — the snake_case key is silently dropped
and the tool's minted "admin" never gets Admin promotion. seed-humans.ts already
sends the correct camelCase form. One-char fix; not on the e2e critical path
(seed-test-admin is a standalone key-minting tool), hence backlog not shift-scope.
