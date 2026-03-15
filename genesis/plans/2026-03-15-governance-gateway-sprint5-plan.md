# Sprint 5: Constitutional Immune System — Challenges & Appeals

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the write path for the governance immune system — filing challenges, responding to challenges (elohim/admin), filing appeals, SLA tracking, and precedent management. This makes governance actionable, not just observable.

**Architecture:** Challenge → Response (SLA-bound) → Appeal → Precedent chain. Every entity can be challenged. Every challenge gets guaranteed engagement. Blocks from consent rounds (Sprint 4) feed into this same escalation pathway.

**Tech Stack:** Angular 19, TypeScript, elohim-storage Rust backend

**Design doc:** `genesis/plans/2026-03-15-governance-feedback-mechanism-gateway-design.md`
**Depends on:** Sprint 4 (gateway renders mechanisms, user can interact with governance)

---

### Task 1: Backend — challenge/appeal CRUD routes

**Files:**
- Modify: `elohim/elohim-storage/src/api/governance.rs`
- Modify: `elohim/elohim-storage/src/db/governance.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

Add POST routes for:
- `POST /challenges` — file a new challenge (grounds, evidence, standing)
- `POST /challenges/{id}/respond` — respond to challenge (elohim/admin)
- `POST /challenges/{id}/appeal` — file appeal on challenge response
- `GET /challenges?entityType=X&entityId=Y` — list challenges for entity
- `GET /precedents?scope=X` — list precedents

Add View types: `FileChallengeInputView`, `ChallengeResponseInputView`, `FileAppealInputView`

**Commit:** `feat(storage): add challenge and appeal write routes`

---

### Task 2: SLA tracking — deadline calculation and status

**Files:**
- Modify: `elohim/elohim-storage/src/db/governance.rs`
- Possibly create: `elohim/elohim-storage/src/services/sla_service.rs`

SLA defaults from the model:
- Acknowledgment: 1 hour
- Initial response: 3 days
- Resolution: 14 days

Add `sla_status` computation (on_time, warning, overdue) to challenge queries. Track `acknowledged_at`, `responded_at`, `resolved_at` timestamps.

**Commit:** `feat(storage): add SLA tracking for challenges`

---

### Task 3: GovernanceApiService — challenge/appeal methods

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`

Add:
- `fileChallenge(input: FileChallengeInput): Observable<ChallengeView>`
- `respondToChallenge(id: string, response: ChallengeResponseInput): Observable<ChallengeView>`
- `fileAppeal(challengeId: string, appeal: FileAppealInput): Observable<AppealView>`
- `getChallengesForEntity(entityType: string, entityId: string): Observable<ChallengeView[]>`
- `getPrecedents(scope?: string): Observable<PrecedentView[]>`

**Commit:** `feat(qahal): add challenge/appeal API methods`

---

### Task 4: FileChallengeComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/file-challenge/file-challenge.component.ts`

Form fields:
- Challenge grounds (select from: factual-error, misapplication, bias, inconsistency, outdated, harmful, procedural — from governance-feedback.model.ts)
- Evidence (text area)
- Standing (auto-detected: content-owner, affected-party, community-member, public-interest)
- Submit → calls GovernanceApiService.fileChallenge()

Accessible from the context menu (GateModalInteraction) on any content.

**Commit:** `feat(qahal): add file challenge component`

---

### Task 5: ChallengeDetailComponent + ChallengeListComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/challenge-detail/challenge-detail.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/challenge-list/challenge-list.component.ts`

**ChallengeList:** Shows challenges for an entity with status badges (pending, acknowledged, responded, resolved, appealed). SLA countdown display.

**ChallengeDetail:** Shows challenge, response (if any), appeal option (if applicable), SLA status with visual indicators (green/yellow/red).

**Commit:** `feat(qahal): add challenge list and detail components`

---

### Task 6: RespondToChallengeComponent (elohim/admin only)

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/respond-to-challenge/respond-to-challenge.component.ts`

Response form:
- Outcome: upheld, partially-upheld, rejected
- Reasoning (required)
- Actions taken (if upheld)
- Precedent setting? (boolean — if yes, becomes referenceable)

This is the seam where elohim engagement happens. In Layer B, the elohim agent will auto-generate responses. For now, it's manual.

**Commit:** `feat(qahal): add respond-to-challenge component`

---

### Task 7: FileAppealComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/file-appeal/file-appeal.component.ts`

Appeal form:
- Grounds for appeal (new evidence, procedural error, disproportionate response)
- Additional evidence
- Submit → creates appeal, triggers escalation to next governance level

**Commit:** `feat(qahal): add file appeal component`

---

### Task 8: Governance routes + context menu integration

**Files:**
- Modify: qahal routing module
- Modify: content context menu component

Wire routes:
- `governance/challenges` → ChallengeListComponent
- `governance/challenges/new` → FileChallengeComponent
- `governance/challenges/:id` → ChallengeDetailComponent

Add "Challenge" option to the content context menu (GateModalInteraction).

**Commit:** `feat(qahal): wire challenge/appeal routes and context menu`

---

### Task 9: Tests

Tests for:
- FileChallengeComponent (form validation, submit flow)
- ChallengeDetailComponent (renders status, SLA indicators)
- GovernanceApiService challenge methods
- SLA status calculation

**Commit:** `test(qahal): add challenge/appeal workflow tests`

---

### Task 10: A2O scenarios

Scenarios:
- "Learner challenges inaccurate content" — files challenge with factual-error grounds
- "Elohim responds to challenge within SLA" — response arrives, SLA green
- "Learner appeals rejected challenge" — appeal escalates to higher governance level
- "Challenge response sets precedent" — precedent becomes referenceable
- "SLA overdue triggers escalation warning" — visual indicator changes

**Commit:** `feat(a2o): add challenge and appeal workflow scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Backend challenge/appeal CRUD routes | Rust |
| 2 | SLA tracking | Rust |
| 3 | GovernanceApiService methods | Service |
| 4 | FileChallengeComponent | Component |
| 5 | ChallengeList + ChallengeDetail | Component |
| 6 | RespondToChallengeComponent | Component |
| 7 | FileAppealComponent | Component |
| 8 | Routes + context menu | Integration |
| 9 | Tests | Testing |
| 10 | A2O scenarios | Scenarios |
