# Sprint 5: Constitutional Immune System — Challenges & Appeals (v2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the write path for the governance immune system — filing challenges, responding to challenges (elohim/admin), filing appeals, SLA tracking, and precedent management. This makes governance actionable, not just observable.

**Architecture:** Challenge → Response (SLA-bound) → Appeal → Precedent chain. Every entity can be challenged. Every challenge gets guaranteed engagement. Blocks from consent rounds (Sprint 4) feed into this same escalation pathway.

**What exists:**
- Rich TypeScript models in `governance-feedback.model.ts`: Challenge, ChallengeStanding, ChallengeGrounds, ChallengeResponse, ChallengeState, Appeal, Precedent
- GovernanceApiService pattern (firstValueFrom, catchError) — 7 methods added in Sprint 4
- ContextMenuOnlyComponent emits `challenge` event — needs to be wired to FileChallengeComponent
- No Rust backend for challenges yet — needs migration, models, views, CRUD, routes
- `ChallengeView` exists in views.rs as a From<Challenge> impl but Challenge model doesn't exist in models.rs (view was likely scaffolded but never backed by a real table)

**Tech Stack:** Rust (Diesel ORM, SQLite, ts-rs), Angular 19, TypeScript

**Design doc:** `genesis/plans/2026-03-15-governance-feedback-mechanism-gateway-design.md`
**Depends on:** Sprint 4 (gateway renders mechanisms, context menu emits challenge event)

---

### Task 1: Migration — challenges and appeals tables

**Files:**
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD_add_challenges_and_appeals/up.sql`
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD_add_challenges_and_appeals/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`

```sql
CREATE TABLE IF NOT EXISTS challenges (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    challenger_id TEXT NOT NULL,
    standing_basis TEXT NOT NULL,
    grounds_primary TEXT NOT NULL,
    grounds_secondary TEXT,
    evidence TEXT NOT NULL,
    requested_outcome TEXT,
    state TEXT NOT NULL DEFAULT 'pending',
    response_outcome TEXT,
    response_reasoning TEXT,
    response_actions TEXT,
    response_by TEXT,
    sets_precedent INTEGER NOT NULL DEFAULT 0,
    filed_at TEXT NOT NULL,
    acknowledged_at TEXT,
    response_deadline TEXT NOT NULL,
    responded_at TEXT,
    resolved_at TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS appeals (
    id TEXT PRIMARY KEY NOT NULL,
    challenge_id TEXT NOT NULL,
    appellant_id TEXT NOT NULL,
    grounds TEXT NOT NULL,
    additional_evidence TEXT,
    state TEXT NOT NULL DEFAULT 'pending',
    escalation_level TEXT,
    decision TEXT,
    decision_reasoning TEXT,
    decided_by TEXT,
    filed_at TEXT NOT NULL,
    decided_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_challenges_entity ON challenges(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_challenges_state ON challenges(state);
CREATE INDEX IF NOT EXISTS idx_appeals_challenge ON appeals(challenge_id);
```

SLA defaults (response_deadline = filed_at + 3 days) computed at insert time.

Manually update diesel_schema.rs. Add both tables to allow_tables_to_appear_in_same_query!.

**Commit:** `feat(storage): add challenges and appeals tables`

---

### Task 2: Diesel models — Challenge, NewChallenge, Appeal, NewAppeal

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`

Add Queryable + Insertable structs for both tables. Follow existing ProposalOption/RankedVote patterns from Sprint 3.

Challenge has SLA-related timestamp fields: `filed_at`, `acknowledged_at`, `response_deadline`, `responded_at`, `resolved_at`.

**Commit:** `feat(storage): add Challenge and Appeal Diesel models`

---

### Task 3: View types — ChallengeView, AppealView, input views

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

Check if ChallengeView already exists (it was scaffolded earlier). If so, update it to match the new model. If not, create it.

Add:
- `ChallengeView` with ts-rs export, From<Challenge> impl
- `AppealView` with ts-rs export, From<Appeal> impl
- `FileChallengeInputView` (Deserialize) — entity_type, entity_id, challenger_id, standing_basis, grounds_primary, grounds_secondary, evidence, requested_outcome
- `RespondToChallengeInputView` (Deserialize) — outcome, reasoning, actions, sets_precedent
- `FileAppealInputView` (Deserialize) — grounds, additional_evidence

Add `SlaStatus` computed field to ChallengeView:
```rust
pub sla_status: String, // "on_time", "warning", "overdue"
```

Compute based on: if responded_at exists → "resolved", else if now > response_deadline → "overdue", else if now > (response_deadline - 1 day) → "warning", else → "on_time"

**Commit:** `feat(storage): add Challenge and Appeal view types with SLA status`

---

### Task 4: CRUD functions and HTTP routes

**Files:**
- Modify: `elohim/elohim-storage/src/db/governance.rs`
- Modify: `elohim/elohim-storage/src/api/governance.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

CRUD:
- `create_challenge(conn, new: &NewChallenge) -> Result<Challenge>`
- `get_challenge(conn, id: &str) -> Result<Option<Challenge>>`
- `query_challenges(conn, entity_type: &str, entity_id: &str) -> Result<Vec<Challenge>>`
- `respond_to_challenge(conn, id: &str, outcome: &str, reasoning: &str, actions: &str, responder: &str, sets_precedent: bool) -> Result<Challenge>`
- `create_appeal(conn, new: &NewAppeal) -> Result<Appeal>`
- `get_appeal(conn, id: &str) -> Result<Option<Appeal>>`
- `query_appeals_for_challenge(conn, challenge_id: &str) -> Result<Vec<Appeal>>`

HTTP routes:
- `POST /challenges` — file challenge, auto-compute response_deadline (filed_at + 3 days)
- `GET /challenges?entityType=X&entityId=Y` — list challenges for entity
- `GET /challenges/{id}` — get challenge detail
- `POST /challenges/{id}/respond` — respond to challenge (updates state, sets responded_at)
- `POST /challenges/{id}/appeal` — file appeal (creates appeal record)
- `GET /appeals/{challengeId}` — list appeals for challenge

Register routes in http.rs.

**Commit:** `feat(storage): add challenge/appeal CRUD and HTTP routes`

---

### Task 5: Generate TypeScript types

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/generated/index.ts`

Run `cargo test export_bindings`. Add new exports for ChallengeView, AppealView, FileChallengeInputView, RespondToChallengeInputView, FileAppealInputView.

**Commit:** `chore: regenerate TypeScript types with challenge/appeal types`

---

### Task 6: GovernanceApiService — challenge/appeal methods

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`
- Modify: `app/elohim-app/src/app/elohim/interfaces/governance.interface.ts`

Add methods following the established pattern (firstValueFrom, encodeURIComponent):
- `fileChallenge(input: FileChallengeInputView): Promise<ChallengeView>`
- `respondToChallenge(id: string, input: RespondToChallengeInputView): Promise<ChallengeView>`
- `fileAppeal(challengeId: string, input: FileAppealInputView): Promise<AppealView>`
- `getChallengesForEntity(entityType: string, entityId: string): Promise<ChallengeView[]>`
- `getChallenge(id: string): Promise<ChallengeView | null>`
- `getAppeals(challengeId: string): Promise<AppealView[]>`

**Commit:** `feat(qahal): add challenge/appeal API methods`

---

### Task 7: FileChallengeComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/file-challenge/file-challenge.component.ts`

Standalone component with inline template. Form fields:
- Challenge grounds — select from ChallengeGroundType enum values in governance-feedback.model.ts: factual-error, misapplication, bias, inconsistency, outdated, harmful, procedural
- Evidence — textarea (required, min 100 chars)
- Standing basis — auto-detected or selected: content-owner, affected-party, community-member, public-interest
- Requested outcome — optional text

Inputs: `entityType`, `entityId`. Output: `challengeFiled` event.

On submit → calls GovernanceApiService.fileChallenge(). Show success/error state.

**Commit:** `feat(qahal): add file challenge component`

---

### Task 8: ChallengeListComponent + ChallengeDetailComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/challenge-list/challenge-list.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/challenge-detail/challenge-detail.component.ts`

**ChallengeList:** Standalone, inline template. Shows challenges for an entity.
- Status badges with color: pending (gray), acknowledged (blue), responded (green), overdue (red), appealed (amber)
- SLA countdown: "Response due in 2d 4h" or "OVERDUE by 1d"
- Click navigates to detail

**ChallengeDetail:** Shows full challenge info.
- Challenge grounds + evidence
- SLA status indicator (green/yellow/red based on sla_status field)
- Response section (if responded): outcome, reasoning, actions
- "Appeal this response" button (if responded and not already appealed)
- "Respond" button (if pending — for elohim/admin, rendered via RespondToChallengeComponent)

**Commit:** `feat(qahal): add challenge list and detail components`

---

### Task 9: RespondToChallengeComponent + FileAppealComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/respond-to-challenge/respond-to-challenge.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/file-appeal/file-appeal.component.ts`

**RespondToChallenge:** Form with:
- Outcome: upheld / partially-upheld / rejected (radio buttons)
- Reasoning (required textarea, min 200 chars)
- Actions taken (textarea, required if upheld)
- Sets precedent? (checkbox)
- Submit → calls respondToChallenge()

This is the seam where elohim engagement happens. In Sprint 9, the elohim agent will auto-generate responses. For now, it's manual.

**FileAppeal:** Form with:
- Grounds: new-evidence, procedural-error, disproportionate-response (select)
- Additional evidence (textarea)
- Submit → calls fileAppeal()

**Commit:** `feat(qahal): add respond-to-challenge and file-appeal components`

---

### Task 10: Routes + context menu wiring

**Files:**
- Modify: qahal routing (find the routing config)
- Modify: `app/elohim-app/src/app/qahal/components/context-menu-only/context-menu-only.component.ts`
- Modify: `app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts`

Wire the ContextMenuOnly "Challenge" action to navigate to the FileChallengeComponent. Options:
1. Route-based: navigate to `/governance/challenges/new?entityType=X&entityId=Y`
2. Modal-based: open FileChallengeComponent in a dialog/overlay

Choose whichever matches existing patterns (check if there's a modal/dialog service).

Add routes if route-based:
- `governance/challenges` → ChallengeListComponent
- `governance/challenges/new` → FileChallengeComponent
- `governance/challenges/:id` → ChallengeDetailComponent

**Commit:** `feat(qahal): wire challenge routes and context menu integration`

---

### Task 11: Vitest tests

Focus on testable services:
- GovernanceApiService challenge methods (HTTP endpoint verification)
- SLA status computation (pure logic — on_time/warning/overdue based on dates)
- FileChallengeComponent form validation (grounds required, evidence min length)

Don't test components with external templates.

**Commit:** `test(qahal): add challenge/appeal workflow tests`

---

### Task 12: A2O scenarios

**Files:**
- Modify: `genesis/a2o/features/qahal/collective-governance.feature`

Scenarios:
- "Learner challenges inaccurate content" — files challenge with factual-error grounds
- "Challenge shows SLA countdown in challenge list" — response due in X days
- "Elohim responds to challenge" — response with reasoning, SLA met
- "Learner appeals rejected challenge" — appeal escalates
- "SLA overdue triggers visual warning" — red indicator on overdue challenge
- "Challenge response sets precedent" — precedent flag visible

**Commit:** `feat(a2o): add challenge and appeal workflow scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Migration — challenges + appeals tables | Rust |
| 2 | Diesel models | Rust |
| 3 | View types with SLA status | Rust |
| 4 | CRUD + HTTP routes | Rust |
| 5 | Generate TypeScript types | Codegen |
| 6 | GovernanceApiService methods | Angular service |
| 7 | FileChallengeComponent | Angular component |
| 8 | ChallengeList + ChallengeDetail | Angular component |
| 9 | RespondToChallenge + FileAppeal | Angular component |
| 10 | Routes + context menu wiring | Integration |
| 11 | Vitest tests | Testing |
| 12 | A2O scenarios | Scenarios |
