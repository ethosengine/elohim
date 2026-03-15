# Sprint 4: Angular Feedback Mechanism Gateway — Implementation Plan (v2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the Angular frontend that consumes Sprint 3's multi-mechanism voting backend — the `FeedbackMechanismGateway` component, mechanism selection service, casual governance components (levels 0-2), and the Psephos integration seam for formal voting (levels 3-7). Integrate into content views.

**Architecture:** The gateway sits between content rendering and governance UI. It consults governance state to determine which mechanism to render. Two rendering paths:

- **Casual governance (levels 0-2):** Angular components — context menu, emotional reactions, graduated feedback. These are lightweight, inline, always-available. Built in this sprint.
- **Formal governance (levels 3-7):** Psephos web component (`<psephos-ballot>`) — renders structured ballots with election hygiene (randomized ordering, equal visual weight, reasoning prompts). Psephos is a new `@ethosengine/psephos` package in the Sophia workspace, sibling to Perseus and Psyche. This sprint builds the **integration seam** (Angular wrapper + data flow). The Psephos package itself is a separate sprint in the Sophia workspace.

**Psephos boundary:** Perseus renders exercises, Psyche measures understanding, Psephos renders ballots. The Recognition callback pattern maps directly — a ballot submission IS a recognition. `<sophia-question>` wraps Perseus; `<psephos-ballot>` wraps Psephos. The Angular `psephos-plugin` wraps `<psephos-ballot>` the same way `sophia-plugin` wraps `<sophia-question>`.

**Tech Stack:** Angular 19, TypeScript, RxJS, `@elohim/storage-client` generated types

**Design doc:** `genesis/plans/2026-03-15-governance-feedback-mechanism-gateway-design.md`
**Backend:** Sprint 3 (complete) — routes at `/api/v1/governance/proposals/{id}/options|ranked-votes|tally` and `/api/v1/governance/signals`

---

### Task 1: GovernanceApiService — new HTTP client methods ✅ DONE

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`

Add methods that call Sprint 3's new backend routes:
- `getProposalOptions(proposalId: string): Promise<ProposalOptionView[]>`
- `createProposalOptions(proposalId: string, options: CreateProposalOptionInputView[]): Promise<ProposalOptionView[]>`
- `castRankedVotes(proposalId: string, ballot: CastRankedVoteInputView): Promise<RankedVoteView[]>`
- `getRankedVotes(proposalId: string): Promise<RankedVoteView[]>`
- `getTally(proposalId: string): Promise<TallyResult>`
- `recordSignal(signal: RecordSignalInputView): Promise<GovernanceSignalView>`
- `getSignals(entityType: string, entityId: string): Promise<GovernanceSignalView[]>`

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
  mechanism: string;                // voting mechanism name
  renderTarget: 'angular' | 'psephos'; // which renderer handles this level
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
- Constitutional/settled content → level 0 (reasoned dissent only), renderTarget: 'angular'
- Content with no active proposal → level 1-2 (reactions + graduated feedback), renderTarget: 'angular'
- Content with active proposal → level matches proposal's voting_mechanism, renderTarget: 'psephos'
- Use `governanceState.governanceStatus` and `contentType` as inputs

The `renderTarget` field is the key architectural boundary: it tells the gateway whether to render an Angular component or mount a Psephos web component.

**Commit:** `feat(qahal): add MechanismSelectionService with Psephos routing`

---

### Task 3: FeedbackMechanismGatewayComponent — the gateway shell

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts`

This is the core orchestrating component. It:
1. Takes `@Input() entityType: string` and `@Input() entityId: string`
2. Loads governance state via GovernanceApiService
3. Calls MechanismSelectionService to determine what to render
4. Renders based on `selection.renderTarget`:

```html
@if (selection(); as sel) {
  <!-- Casual governance: Angular-rendered (levels 0-2) -->
  @if (sel.renderTarget === 'angular') {
    @switch (sel.level) {
      @case (0) { <qahal-context-menu-only [entityType]="entityType" [entityId]="entityId" /> }
      @case (1) { <qahal-reaction-bar [entityType]="entityType" [entityId]="entityId" /> }
      @case (2) {
        <qahal-graduated-feedback [entityType]="entityType" [entityId]="entityId" />
        <qahal-reaction-bar [entityType]="entityType" [entityId]="entityId" />
      }
    }
  }

  <!-- Formal governance: Psephos-rendered (levels 3-7) -->
  @if (sel.renderTarget === 'psephos') {
    <qahal-psephos-ballot-wrapper
      [proposal]="sel.activeProposal!"
      [mechanism]="sel.mechanism"
      (ballotSubmitted)="onBallotSubmitted($event)" />
  }
}
```

The `qahal-psephos-ballot-wrapper` is the integration seam built in Task 6.

**Commit:** `feat(qahal): add FeedbackMechanismGateway with angular/psephos routing`

---

### Task 4: ContextMenuOnlyComponent — level 0 governance

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/context-menu-only/context-menu-only.component.ts`

For constitutional/settled content where no low-friction signals are shown. Renders:
- A subtle "..." or kebab menu icon
- Menu options: Flag, Challenge, Open Feedback (via GateModalInteraction)
- Nothing is above challenge — this is always available

Minimal component. The context menu triggers the existing challenge/feedback flows.

**Commit:** `feat(qahal): add context-menu-only component for constitutional content`

---

### Task 5: Enhance existing ReactionBar and GraduatedFeedback components

**Files:**
- Modify: `app/elohim-app/src/app/qahal/components/reaction-bar/reaction-bar.component.ts`
- Modify: `app/elohim-app/src/app/qahal/components/graduated-feedback/graduated-feedback.component.ts`

These components already exist but need to be wired to the governance signals backend:
- **ReactionBar:** On reaction click → call `GovernanceApiService.recordSignal()` with signal_type 'reaction', signal_value '{emoji_type}', mechanism_level 1
- **GraduatedFeedback:** On scale selection → call `GovernanceApiService.recordSignal()` with signal_type 'graduated', signal_value '{scale}:{value}', mechanism_level 2. Require reasoning text for negative feedback (bottom 2 options).

Both should show aggregate signal counts (load via `getSignals()`).

**Commit:** `feat(qahal): wire reaction-bar and graduated-feedback to governance signals API`

---

### Task 6: PsephosBallotWrapperComponent — the Psephos integration seam

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/psephos-ballot-wrapper/psephos-ballot-wrapper.component.ts`

This is the **critical architectural seam**. It wraps what will become `<psephos-ballot>` (the web component from `@ethosengine/psephos`). For now, it renders a **placeholder** that shows:
1. The proposal title and description
2. The voting mechanism type
3. The options (loaded via GovernanceApiService.getProposalOptions)
4. A message: "Psephos ballot rendering coming soon — use fallback"
5. A fallback form that allows basic voting (simple select/input per option + submit)

The fallback form calls `GovernanceApiService.castRankedVotes()` with the appropriate ballot structure.

When `@ethosengine/psephos` is built (separate Sophia sprint), this wrapper replaces the fallback with:
```html
<psephos-ballot
  [mechanism]="mechanism"
  [options]="options"
  [config]="votingConfig"
  (recognition)="onRecognition($event)" />
```

The `(recognition)` output follows the same pattern as `sophia-plugin`'s Recognition callback handling.

**Interface contract for Psephos (what it must provide):**
```typescript
// Input: what to render
interface PsephosBallotProps {
  mechanism: 'ranked-choice' | 'approval' | 'score-vote' | 'dot-vote' | 'consent';
  options: ProposalOptionView[];
  config: {
    scoreMin?: number;
    scoreMax?: number;
    dotsPerVoter?: number;
  };
  electionHygiene: {
    randomizeOrder: boolean;    // prevent position bias
    equalVisualWeight: boolean; // no option visually larger
    requireReasoning: boolean;  // require text justification
    showResultsAfterVote: boolean;
  };
}

// Output: the ballot recognition
interface BallotRecognition {
  mechanism: string;
  ballots: BallotEntry[];       // from @elohim/storage-client
  reasoning?: string;
  timestamp: string;
}
```

**Commit:** `feat(qahal): add Psephos ballot wrapper with fallback and interface contract`

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
- Modify: `app/elohim-app/src/app/qahal/components/` (reaction-bar, graduated-feedback, psephos-ballot-wrapper)

After each vote/signal submission, generate an REA economic event via the recognition pipeline:
- Resource: the governance signal/vote
- Event type: "governance-participation"
- Agent: the voter
- Flow: EconomicEvent → recognition pipeline → steward affinity delta

Map mechanism levels to recognition weight:
- Level 0-1 (reactions): low weight
- Level 2-3 (feedback, approval): medium weight
- Level 4-6 (ranked-choice, score, consent): high weight
- Level 7 (deliberation): highest weight

This connects governance participation to stewardship — curation acts build affinity.

**Commit:** `feat(qahal): generate REA events on governance participation`

---

### Task 9: Vitest tests

**Files:**
- Create tests for MechanismSelectionService (returns correct level and renderTarget per governance state)
- Create tests for GovernanceApiService new methods (HTTP calls correct endpoints)
- Create tests for FeedbackMechanismGatewayComponent (renders angular components for levels 0-2, renders psephos wrapper for levels 3-7)
- Create tests for PsephosBallotWrapperComponent (shows fallback form, submits ballot)

**Commit:** `test(qahal): add tests for governance gateway services and components`

---

### Task 10: A2O scenario updates

**Files:**
- Modify: `genesis/a2o/features/qahal/collective-governance.feature`

Add scenarios:
- "Learner sees context menu only on constitutional content" — level 0, no reactions shown
- "Learner reacts to content with emotional response" — level 1, reaction recorded as signal
- "Learner sees Psephos ballot for ranked-choice proposal" — level 4, formal governance renders ballot
- "Governance participation generates REA event" — voting creates economic event
- "Gateway renders consent round ballot for collective decision" — block triggers escalation message

**Commit:** `feat(a2o): add feedback mechanism gateway integration scenarios`

---

## Psephos: What It Must Provide (Separate Sprint in Sophia Workspace)

This section documents what `@ethosengine/psephos` needs to deliver. It is NOT built in this sprint — this sprint builds the seam.

### Package: `@ethosengine/psephos`

**Location:** `sophia/packages/psephos/` (sibling to `sophia/packages/perseus/`)

**Purpose:** Render formal governance ballots with election hygiene. The voting equivalent of Perseus exercise widgets.

**Widget types (one per mechanism):**
| Widget | Mechanism | Interaction |
|--------|-----------|-------------|
| `ranked-choice` | IRV ranked preferences | Drag-to-rank or click-to-assign rank per option |
| `approval` | Approve multiple options | Checkbox per option |
| `score-vote` | Score each option | Slider or number input per option within [min, max] |
| `dot-vote` | Allocate limited dots | Increment/decrement per option, budget constraint |
| `consent` | Consent or block | Two buttons: Consent / Block. Block requires reasoning. |

**Election hygiene (built into every widget):**
- Randomized option ordering (prevent position bias)
- Equal visual weight per option (no option visually larger/bolder)
- Required reasoning for blocking/negative actions
- Results hidden until after vote (prevent bandwagon)
- Confirmation step before submission ("You ranked X > Y > Z. Submit?")
- Accessible: keyboard navigation, screen reader labels

**Output:** `BallotRecognition` callback (see Task 6 interface contract)

**Web component:** `<psephos-ballot>` — UMD bundle like `sophia-element`

**Angular wrapper:** `psephos-plugin` in `elohim-library` — wraps `<psephos-ballot>` like `sophia-plugin` wraps `<sophia-question>`

**Dependencies:**
- `@elohim/storage-client` generated types (ProposalOptionView, BallotEntry, etc.)
- No dependency on Perseus or Psyche — standalone rendering package

---

## Summary

| Task | What | Renderer | Layer |
|------|------|----------|-------|
| 1 ✅ | GovernanceApiService HTTP methods | — | Service |
| 2 | MechanismSelectionService | — | Service |
| 3 | FeedbackMechanismGateway component | Router | Component |
| 4 | ContextMenuOnlyComponent | Angular | Component (level 0) |
| 5 | Enhance ReactionBar + GraduatedFeedback | Angular | Component (levels 1-2) |
| 6 | PsephosBallotWrapper (seam + fallback) | Psephos seam | Component (levels 3-7) |
| 7 | Integrate into content views | — | Integration |
| 8 | REA event generation | — | Integration |
| 9 | Vitest tests | — | Testing |
| 10 | A2O scenarios | — | Scenarios |
