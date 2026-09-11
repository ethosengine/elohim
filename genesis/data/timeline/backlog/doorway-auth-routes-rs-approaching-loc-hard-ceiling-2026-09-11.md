---
id: "backlog-doorway-auth-routes-rs-approaching-loc-hard-ceiling"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway-service routes/auth_routes.rs is at ~5980 lines against the rs-loc-ceiling hard limit of 7000 — split by concern before the rail starts refusing edits"
slug: "doorway-auth-routes-rs-approaching-loc-hard-ceiling-2026-09-11"
written: "2026-09-11"
author: "doorway-federation sprint — S2 Tasks 7, 9, 13 (rs-loc-ceiling advisory fired three times)"
status: "open"
priority: "medium"
tags: [doorway-service, code-health, rs-loc-ceiling, auth, modularization, D8]
relatedNodeIds: []
cites:
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/routes/hosted_cell.rs
  - doorway/doorway-service/src/.epr-meta
---

# `auth_routes.rs` is one edit away from the hard ceiling

## Measured (2026-09-11)

- 5238 lines at the start of the sprint; 5714 after Task 7; **5978** after Tasks 9 + 13. Soft ceiling 3000, hard 7000
  (`doorway/doorway-service/src/.epr-meta`, rule `rs-loc-ceiling`). The advisory fired on every commit that touched it.
- The sprint honoured the nudge for NEW logic — `routes/hosted_cell.rs` (grant issue/revoke, `humans_served`) was born as a
  sibling module — but did not refactor mid-edit, so the file still carries register, login, refresh, me, account,
  close-account, session-transfer, OAuth code exchange, provisioning, the synthetic-identity fallback, revocation
  helpers, and ~40 unit tests in one translation unit.

## Why it matters

At 7000 the rail refuses edits, and this is the file every hosted-human story lands in (07-hosted-by-a-household,
05-leaving, the two-portals SSO consolidation). The next story that needs a register/close change will be blocked by a
line count, not by design.

## Shape of the cut (a bounded task, not a redesign)

Split by concern, no behaviour change, tests move with their code:
`routes/auth/{register.rs, login.rs, session.rs, account.rs, close_account.rs, provisioning.rs, revocation.rs}` with
`auth_routes.rs` reduced to the dispatch `match` + shared types. Seam-registry `sourceLocation` rows for
`should_provision`, `synthetic_identity_fallback_allowed`, `close_account_verdict`, `CloseVerdict`, `active_user_filter`,
`close_account_row_update` must be updated in the same commit (the census reads `file` + `line`). Gate: `just gate
doorway`; the 1230-test suite is the regression net. Do it BEFORE the next auth story, not during one.
