# Sprint 4: Angular Feedback Mechanism Gateway — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the Angular frontend that consumes Sprint 3's multi-mechanism voting backend — the `FeedbackMechanismGateway` component, mechanism selection service, and integration into content views. This is Layer A's user-facing surface.

**Architecture:** The gateway sits between content rendering and governance UI. It consults governance state to determine which mechanism to render, then renders the appropriate voting/feedback UI inline at the content.

**Tech Stack:** Angular 19, TypeScript, RxJS, `@elohim/storage-client` generated types

**Design doc:** `genesis/plans/2026-03-15-governance-feedback-mechanism-gateway-design.md`
**Backend:** Sprint 3 (complete) — routes at `/api/v1/governance/proposals/{id}/options|ranked-votes|tally` and `/api/v1/governance/signals`

---

### Task 1: GovernanceApiService — new HTTP client methods

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`

Add methods that call Sprint 3's new backend routes:
- `getProposalOptions(proposalId: string): Observable<ProposalOptionView[]>`
- `createProposalOptions(proposalId: string, options: CreateProposalOptionInputView[]): Observable<ProposalOptionView[]>`
- `castRankedVotes(proposalId: string, ballot: CastRankedVoteInputView): Observable<RankedVoteView[]>`
- `getRankedVotes(proposalId: string): Observable<RankedVoteView[]>`
- `getTally(proposalId: string): Observable<TallyResult>`
- `recordSignal(signal: RecordSignalInputView): Observable<GovernanceSignalView>`
- `getSignals(entityType: string, entityId: string): Observable<GovernanceSignalView[]>`

Import types from `@elohim/storage-client`. Follow existing patterns in the service (HttpClient, error handling).

**Commit:** `feat(qahal): add governance API methods for multi-mechanism voting`

---

### Task 2: MechanismSelectionService — determine which mechanism to render

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/mechanism-selection.service.ts`

This service implements the mechanism ladder from the design doc. Given an entity's governance state, it returns which feedback mechanism to render.

```typescript
export interface MechanismSelection {
  level: number;                    // 0-7 from ladder
  mechanism: VotingMechanism;       // from governance-deliberation.model.ts
  contextMenuOnly: boolean;         // level 0: only context menu available
  allowReactions: boolean;          // levels 1+
  allowGraduatedFeedback: boolean;  // levels 2+
  activeProposal?: ProposalView;    // levels 3-7: the active proposal to vote on
}

@Injectable({ providedIn: 'root' })
export class MechanismSelectionService {
  selectMechanism(governanceState: GovernanceStateView, contentType: string): MechanismSelection
}
```

Selection logic:
- Constitutional/settled content → level 0 (reasoned dissent only)
- Content with no active proposal → level 1-2 (reactions + graduated feedback)
- Content with active proposal → level matches proposal's voting_mechanism
- Use `governanceState.governanceStatus` and `contentType` as inputs

**Commit:** `feat(qahal): add MechanismSelectionService for feedback gateway`

---

### Task 3: FeedbackMechanismGatewayComponent — the gateway shell

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts`

This is the core orchestrating component. It:
1. Takes `@Input() entityType: string` and `@Input() entityId: string`
2. Loads governance state via GovernanceApiService
3. Calls MechanismSelectionService to determine what to render
4. Renders the appropriate sub-component via `@switch` on `selection.level`

Template structure:
```html
@switch (selection()?.level) {
  @case (0) { <qahal-context-menu-only /> }
  @case (1) { <qahal-reaction-bar /> }
  @case (2) { <qahal-graduated-feedback /> }
  @case (3) { <qahal-approval-vote [proposal]="selection()!.activeProposal!" /> }
  @case (4) { <qahal-ranked-choice-vote [proposal]="selection()!.activeProposal!" /> }
  @case (5) { <qahal-score-vote [proposal]="selection()!.activeProposal!" /> }
  @case (6) { <qahal-consent-round [proposal]="selection()!.activeProposal!" /> }
  @case (7) { <qahal-full-deliberation [proposal]="selection()!.activeProposal!" /> }
}
```

For Sprint 4, sub-components at levels 0-2 and 7 can be stubs. Levels 3-6 are the voting components built in Tasks 4-7.

**Commit:** `feat(qahal): add FeedbackMechanismGateway orchestrating component`

---

### Task 4: RankedChoiceVoteComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/ranked-choice-vote/ranked-choice-vote.component.ts`

Takes `@Input() proposal: ProposalView`. Renders:
1. List of options (loaded via GovernanceApiService.getProposalOptions)
2. Drag-to-rank or click-to-rank UI for ordering preferences
3. Submit button that calls GovernanceApiService.castRankedVotes
4. After submit, shows tally results (round-by-round IRV visualization)

Uses Angular CDK drag-drop for ranking if available, otherwise simple numbered inputs.

**Commit:** `feat(qahal): add ranked-choice voting component with IRV visualization`

---

### Task 5: ApprovalVoteComponent and ScoreVoteComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/approval-vote/approval-vote.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/score-vote/score-vote.component.ts`

**ApprovalVote:** Checkbox per option (approve/don't approve). Submit calls castRankedVotes with `approved: true/false`.

**ScoreVote:** Slider or number input per option within [score_min, score_max]. Submit calls castRankedVotes with `score` values.

Both show tally results after submission.

**Commit:** `feat(qahal): add approval and score voting components`

---

### Task 6: DotVoteComponent and ConsentRoundComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/dot-vote/dot-vote.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/consent-round/consent-round.component.ts`

**DotVote:** Budget display (e.g. "10 dots remaining"), increment/decrement per option. Validates total ≤ budget. Submit calls castRankedVotes with `dots` values.

**ConsentRound:** Two buttons per option: "Consent" / "Block". Block requires reasoning text. Shows tally with "blocked" recommendation triggering escalation messaging. The escalation message says the elohim will engage — this is the seam for Layer B.

**Commit:** `feat(qahal): add dot-vote and consent round components`

---

### Task 7: Integrate gateway into content views

**Files:**
- Modify: content viewer component(s) — find the component that renders ContentNode detail
- Add `<qahal-feedback-mechanism-gateway [entityType]="'content'" [entityId]="node.id" />` at the bottom of content views

Also add the gateway to:
- Path detail views (if they exist)
- Collective proposal views

This is the "governance at the place you're experiencing the content" principle.

**Commit:** `feat(qahal): integrate feedback mechanism gateway into content views`

---

### Task 8: REA economic event generation on feedback submission

**Files:**
- Modify: `app/elohim-app/src/app/qahal/components/` (voting components from Tasks 4-6)
- Modify or create: recognition event helper

After each vote/signal submission, generate an REA economic event via the recognition pipeline:
- Resource: the governance signal/vote
- Event type: "governance-participation"
- Agent: the voter
- Flow: EconomicEvent → recognition pipeline → steward affinity delta

This connects governance participation to stewardship — curation acts build affinity.

**Commit:** `feat(qahal): generate REA events on governance participation`

---

### Task 9: Vitest tests for services and components

**Files:**
- Create tests for MechanismSelectionService
- Create tests for GovernanceApiService new methods
- Create tests for FeedbackMechanismGatewayComponent (renders correct sub-component per level)

**Commit:** `test(qahal): add tests for governance gateway services and components`

---

### Task 10: A2O scenario updates

**Files:**
- Modify: `genesis/a2o/features/qahal/collective-governance.feature`

Add scenarios:
- "Learner sees feedback gateway on content" — constitutional content shows only context menu
- "Learner votes in ranked-choice bracket at content" — gateway renders ranked-choice for active proposal
- "Governance participation generates REA event" — voting creates economic event
- "Gateway renders consent round for collective decision" — block triggers escalation message

**Commit:** `feat(a2o): add feedback mechanism gateway integration scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | GovernanceApiService HTTP methods | Service |
| 2 | MechanismSelectionService | Service |
| 3 | FeedbackMechanismGateway component | Component |
| 4 | RankedChoiceVoteComponent | Component |
| 5 | ApprovalVote + ScoreVote | Component |
| 6 | DotVote + ConsentRound | Component |
| 7 | Integrate into content views | Integration |
| 8 | REA event generation | Integration |
| 9 | Vitest tests | Testing |
| 10 | A2O scenarios | Scenarios |
