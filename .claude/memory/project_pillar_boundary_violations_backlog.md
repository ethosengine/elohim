---
name: 174 pre-existing pillar-boundary violations — backlog cleanup sprint
description: M5 Task 16 audit surfaced 174 cross-pillar import violations in elohim-app; ESLint rule is currently warn-level; they need a dedicated cleanup sprint
type: project
originSessionId: 4d20bf7b-4639-43d8-ad10-fccb514a7f0a
---
**Discovery (2026-04-25, M5 Task 16):** When `eslint-plugin-boundaries` was added to elohim-app to enforce composability per `project_elohim_app_as_composable_view_federation`, the audit found **174 pre-existing cross-pillar import violations** spanning every pillar.

**Breakdown:**
- `elohim` → `{imagodei, lamad, shefa, qahal}`: 97 violations (navigator component, context-assembly service, coordination-envelope model)
- `imagodei` → `lamad`: 49 violations (DiscoveryAttestationService, ContentService, StewardshipAllocationService)
- `lamad` → `{imagodei, shefa, qahal}`: 24 violations
- `shefa` → `{imagodei, lamad}`: 24 violations
- `qahal` → `{imagodei, lamad}`: 4 violations

**Examples requiring architectural moves:**
- `lamad` services using `SessionHumanService`, `IdentityService` from `@app/imagodei` → fix: hoist session identity to `elohim` pillar or `@elohim/storage-client`
- `elohim` components reaching into pillars → fix: move components to consuming pillar or restructure cross-cutting types into `@elohim/storage-client`
- `*.routes.ts` files using `identityGuard` from `@app/imagodei` → fix: move to `elohim/guards/` or a shared module
- `imagodei` services depending on lamad attestation types → fix: attestation types belong in storage-client generated types

**Current state:**
- `eslint.config.js` has boundaries plugin configured at `warn` level (not `error`) so CI doesn't break.
- `scripts/audit-pillar-imports.mjs` runs the same audit standalone.
- Specs are exempt via ESLint global ignores (mocking across pillars in tests is acceptable).

**How to apply:**
- M5's new `account/` pillar (Task 17+) MUST NOT introduce new violations — only imports imagodei/elohim/storage-client/common.
- Future pillar work (graduating profile/learn/wallet to composability per the federation memory) cleans up its slice of the 174.
- Dedicated cleanup sprint: when scheduled, batch by pillar (start with smallest violation count — qahal → 4) and work through.
- The audit script + warn-level rule provides ongoing visibility — violations don't grow silently.
