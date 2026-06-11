---
title: Auth Wire-Contract Completion — schema-contract the doorway auth shapes, migrate the Angular consumers
id: auth-wire-contract-completion-plan
status: Draft
class: protocol-canonical
domain: D8
sprint: unranked — born 2026-06-10; sequenced AFTER sdk-core-entrypoints-plan (identity package churn). Operational wire-contract only — no storage, no DHT.
cites:
  - che-network-agency-arc-design | parent arc — Stage A created DoorwaySessionClient and the connection-matrix rails this plan completes the consumer migration for | sha256:ede3841e83bc2b65 | path: genesis/docs/superpowers/specs/2026-06-10-che-network-agency-arc-design.md
  - genesis/data/timeline/backlog/doorway-auth-view-schema-contract.md
  - genesis/data/timeline/backlog/angular-auth-onto-doorway-session-client.md
  - sdk-core-entrypoints-plan | the packaging plan this one is sequenced behind — identity exports must settle before Angular consumers migrate | sha256:2c8f646aec32b42a | path: genesis/docs/superpowers/plans/2026-06-10-sdk-core-entrypoints-plan.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
---

# Auth Wire-Contract Completion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or
> superpowers:executing-plans, task-by-task, two-stage review per task.

**Goal:** Finish what the arc's Stage A started. (1) Put the doorway auth wire shapes under the
**view-schema contract system** so hand-rolled drift is structurally impossible (arc Task 1.1
found 5 drift classes, including an `AccountResponse` whose `usage`/`quota` nesting never existed
on the wire). (2) Migrate the two remaining hand-rolled Angular consumers
(`app/elohim-app/.../auth.service.ts`, `doorway/doorway-app/.../auth-state.service.ts`) onto
`DoorwaySessionClient` — the pattern a2o proved (arc 1.2, two-stage reviewed).

**Source-of-truth declaration (P2P gate):** auth sessions are **operational (Category C)** —
doorway-local, reconstructable by re-auth; NO DHT entry, NO storage table. This plan is wire-
contract + client-consolidation discipline only; the schemas describe HTTP shapes, not storage.

**Sequencing:** AFTER `sdk-core-entrypoints-plan` lands (it moves the identity package's exports;
migrating Angular consumers onto a package mid-churn invites rework). Binding rails from the arc
spec §client-connection-matrix: `trustMode` stays **discovered from `/auth/me`, never config**;
no connection-strategy logic enters any client; elohim-app is SSR-rendered, so the Angular
tokenStore adapter must be platform-guarded (no bare `localStorage` on the server path).

---

## Task 1: Auth view schemas + contract tests (drift becomes impossible)

Source of truth: **operational session state (Category C)** — HTTP wire shapes only; no storage
table, no DHT projection anywhere in this task.

**Files:** `elohim/sdk/schemas/v1/views/auth-response.schema.json`, `me-response.schema.json`,
`exchange-session-response.schema.json`, `session-token-response.schema.json`,
`account-response.schema.json`; `elohim/elohim-storage/tests/schema_contract.rs` pattern applied
where the auth structs live (doorway-service — check whether the harness extends there or a
sibling test lands in doorway: **investigate first**, the existing contract harness targets
elohim-storage; doorway structs need an equivalent — follow the established pattern, do not
invent a parallel mechanism); `elohim/sdk/schemas/scripts/codegen-ts.mjs` `INTERFACE_FILES`.

- [x] Write the five schemas per `views/CONVENTIONS.md` (10 rules), field-for-field with
      `doorway/doorway-service/src/routes/auth_routes.rs` (AuthResponse :164 incl. the
      absent-when-false `isSteward`; MeResponse :248-264 incl. `trustMode`/`authority`;
      ExchangeSessionResponse :88; SessionTokenResponse :78; AccountResponse :283-311 flat shape)
- [x] Contract tests pinning the Rust structs to the schemas (investigate-first note above;
      record the mechanism decision in one journal line before coding)
- [x] Add to `INTERFACE_FILES`; `pnpm run schema:codegen:ts`; pre-push freshness gate passes
      (codegen of operational wire shapes — storage untouched)
- [x] Reconcile `@elohim/identity`'s hand-matched types to the generated interfaces — ONE source:
      identity re-exports/extends the generated shapes (no second hand-written copy survives)
- [x] a2o + identity suites green (the consumers of those types). Commit (schema + tests + codegen
      together).

## Task 2: elohim-app `auth.service.ts` onto `DoorwaySessionClient`

**Files:** `app/elohim-app/src/app/imagodei/services/auth.service.ts` (+ its spec), a new
platform tokenStore adapter in the imagodei pillar.

- [x] Angular adapter: `SessionTokenStore` backed by `localStorage` with SSR platform guard
      (`isPlatformBrowser` — the server path uses the in-memory default); `fetchImpl` from the
      platform; the `AuthProvider` registry and refresh-timer semantics stay at the service edge —
      the client replaces only the hand-rolled HTTP walking + token bookkeeping
- [x] Preserve observable behavior: same `AuthState` signal shape, same localStorage keys
      (`AUTH_TOKEN_KEY` etc. — migrate-on-read if the stored shape changes), same refresh timing;
      `isRefreshing` guard maps onto `refresh()`
- [x] Existing auth.service spec green + new tests for the adapter (SSR path included);
      `pnpm test` (app) for the imagodei subset; lint clean
- [ ] Verify in the dev loop: `pnpm look http://localhost:4200/<auth-surface>` renders login OK
      (eyes check — capture.json clean of new console errors). Commit.

## Task 3: doorway-app `auth-state.service.ts` onto the client

**Files:** `doorway/doorway-app/src/app/services/auth-state.service.ts` (+ spec).

- [ ] Same adapter pattern (doorway-app is browser-only — no SSR guard needed; verify);
      trustMode handling stays exactly as-is (discovered from `/auth/me` at portal render —
      rail check: grep proves no config-pinned trustMode appears)
- [ ] doorway-app lint + test green (`pnpm exec eslint src --ext .ts,.html`). Commit.

## Task 4: Retire the duplication evidence

- [ ] Grep proof: no hand-written `AuthResponse`/`MeResponse`/`AccountResponse` interface
      declarations remain anywhere outside generated dirs + the identity re-export
- [ ] Update the two backlog entries (this plan's cites) to `resolved-by:` lines; re-run
      `decompose.py` on this plan; `placement-audit.py --ledger` reflects the drain. Commit.


## Execution log (2026-06-10)

- **T1 LANDED, review interrupted:** 09becf281 (7 schemas — 5 + $ref'd authority-ref/human-profile;
  sibling doorway harness, 14/14 contract tests, clippy/fmt clean; codegen idempotent) + 3b3a89d4d
  (identity types → generated re-exports; identity became the SIXTH codegen distribution location —
  implementer correctly rejected the storage-client import premise as wrong-source + bad dependency
  direction for /core; suites 70 + 108 green, a2o tsc clean). Independent two-lens review was
  dispatched but **terminated by the account spend limit after 31 tool calls — re-review queued as
  the first action when work resumes.** Boxes below are CLAIMS with implementer evidence only.
- **T2 LANDED + reviewed ✅/APPROVED:** df41f874d — full app suite 214 files / 4612 tests; SSR
  spy-proof real; storage contract preserved (expiry canonicalized w/ migrate-on-read); seconds/ms
  timer math traced both directions; consolidation bypassed the OAuth provider's drifted snake_case
  refresh parse. T3 (doorway-app) dispatched; T4 pending.

## Out of scope

The Tauri identity-handoff path (custody canon); any change to auth ROUTES or session semantics
in Rust (wire shapes are documented as-is; behavior changes are not this plan); peer-conductor
trustMode flows beyond preserving discovery semantics.

## Self-Review

Composes from the arc spec + proven a2o pattern; sequenced behind the packaging plan; Category-C
source-of-truth declared (no storage, no DHT); connection-matrix rails restated as in-plan checks
(trustMode discovery grep, SSR tokenStore guard); drift fix is structural (schema contract), not
another reviewed-away hand-match.
