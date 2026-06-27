---
id: "backlog-admin-users-a2o-domain-scope-mismatch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "a2o user-management asserts @test.elohim.host but doorway gateway-scopes identifiers to @alpha.elohim.host — test-vs-product mismatch (product correct; needs operator judge call)"
slug: "admin-users-a2o-domain-scope-mismatch"
written: "2026-06-27"
author: "overnight doorway-deploy + genesis fan-out shift (2026-06-27T03, wave-2)"
status: "backlog"
priority: "medium"
jobs: [elohim-genesis]
---

## The mismatch (NOT applied this shift — it edits the a2o judge)

a2o `features/auth/user-management.feature` scenarios 7/8/9/10 fail with "X not found in
users list". Wave-2 fixer verdict: **(A) genuine bug, but in the a2o TEST layer, not the
doorway product** — so per agentic-developer principle 6 (test fixtures are off-limits; bail
with a proposal) it was NOT auto-applied. This entry is the proposal for the judge-owner.

**Root cause.** The doorway gateway-scopes every identifier:
`auth_routes.rs::normalize_identifier` re-qualifies the local-part with the doorway's OWN
gateway domain (from `DOORWAY_URL`: `doorway-alpha.elohim.host` → strip `doorway-` →
`alpha.elohim.host`). Both `handle_register` (auth_routes.rs:767-770) and `handle_login`
(:1564-1566) normalize before store/lookup, so the persisted `UserDoc.identifier` is
`localpart@alpha.elohim.host`. `GET /admin/users` correctly returns that canonical stored
identifier.

The a2o fixtures build identifiers under a DIFFERENT domain:
- `fixtures/humans.ts`: Susan → `susan@test.elohim.host` (Matthew is the exception —
  already `matthew.dowell@alpha.elohim.host`, which is why his scenario passes).
- `auth-lifecycle.steps.ts`: Troublemaker → `e2e-troublemaker-<run>@test.elohim.host`.

The step `the users list should include {word}'s entry` (`user-management.steps.ts:46`)
compares `u.identifier === human.identifier` (the `@test.elohim.host` form) → never matches
the gateway-scoped `@alpha.elohim.host` stored form → "not found". Matthew passes only
because his fixture already uses the alpha domain.

## Two resolutions — operator/judge call (do NOT auto-fix)

1. **Test-side (fixer's recommendation):** the a2o fixtures/steps should expect the
   gateway-scoped identifier — either build fixtures under the doorway's resolved domain, or
   compare on the local-part / on the doorway-normalized form rather than the raw fixture
   identifier. This treats gateway-scoping as correct-by-design.
2. **Product-side (only if gateway-scoping is NOT intended for admin-list identity):** make
   the admin-list / lookup identity-stable across the registration domain. Higher risk —
   changes identity semantics; should be a deliberate design decision, not a test fix.

The fixer assessed gateway-scoping as deliberate (sibling to the identity-sovereignty /
doorway-projection model), favoring resolution 1 — but this is an a2o-judge edit, owned by
the operator. A proposed test-layer patch was drafted this shift but intentionally left
unapplied.

## Evidence / refs

- Wave-2 fixer (t6-admin-users-list-scope) STEP-0 verdict A (test-layer), patch drafted-not-applied.
- `doorway/doorway-service/src/routes/auth_routes.rs` `normalize_identifier`, `handle_register`:767, `handle_login`:1564.
- Memory: `feedback-identity-sovereignty-ontology-guard` (doorway-projection identity model).
- Shift journal: `.claude/shifts/2026-06-27T03-overnight-doorway-deploy-genesis-fanout.journal.md` (iter-5).
