# Gate Feedback Modal — Design

**Date:** 2026-03-15
**Status:** Approved
**Epic:** Elohim — Governance Surface (intersection of Sprint 7 comment wiring + Sprint 3 governance gateway)

## Vision

The "vertical dots" context menu that the governance design says is always available at every mechanism level. The open-feedback channel — flag, challenge, or provide reasoned feedback on any content. Uses the same `GateArtifactCard` → `GateInteractionService` pipeline that comments use, but renders as a modal overlay triggered from a context menu.

"The door to governance is never locked."

## Architecture

Three new files, no new services:

```
ContentViewerComponent (existing)
  └── GateFeedbackTriggerComponent (new)
        ├── ⋮ button → context menu (Flag | Challenge | Feedback)
        └── GateFeedbackModalComponent (new, rendered via overlay)
              └── GateArtifactCardComponent (existing)
```

- **`GateFeedbackTriggerComponent`** — the `⋮` button + dropdown menu. Input: `contentId`. Clicking a menu item opens the modal with the selected feedback type.
- **`GateFeedbackModalComponent`** — modal overlay with backdrop. Contains a `GateArtifactCardComponent` pre-configured with the right `mutationType` and `contextMetadata`. Emits `posted` and `settled`, then auto-closes.
- **No new service** — `GateInteractionService` already handles the state machine. The feedback type flows through `mutationType` and `contextMetadata`.

Both components live in `app/elohim-app/src/app/elohim/components/gate-feedback/` — governance infrastructure in the elohim pillar.

Modal is a simple CSS overlay (position: fixed, backdrop), not Angular CDK or dialog library. Minimal styles, themed later.

## Data Flow

```
User clicks ⋮ → menu shows → user picks "Challenge"
  → GateFeedbackTriggerComponent opens modal with feedbackType='challenge'
  → GateFeedbackModalComponent renders GateArtifactCardComponent
      with mutationType='challenge', contextMetadata={ contentId, category: 'challenge' }
  → User types reasoning, clicks Submit
  → GateInteractionService.submit() → gate evaluation → affirm/dialogue/settled
  → On 'posted': modal emits feedbackPosted, closes after ~800ms delay
  → On 'settled': modal emits feedbackSettled, stays open showing settlement info
```

### Category Passthrough

The feedback type is passed as `category` in `contextMetadata`, not as a schema change. When Sprint 7 lands the `gateApiCall` input on the card, the category rides along naturally to the backend. No migration needed from this work.

`mutationType` values: `'flag'` | `'challenge'` | `'feedback'`

## Component API

```typescript
// GateFeedbackTriggerComponent
@Input() contentId: string;
@Output() feedbackPosted = new EventEmitter<{ feedbackType: string; reachTier: ReachTier }>();
@Output() feedbackSettled = new EventEmitter<{ boundary: string; appealPath: string | null }>();

// GateFeedbackModalComponent
@Input() feedbackType: 'flag' | 'challenge' | 'feedback';
@Input() contentId: string;
@Output() posted = new EventEmitter<{ reachTier: ReachTier }>();
@Output() settled = new EventEmitter<{ boundary: string; appealPath: string | null }>();
@Output() closed = new EventEmitter<void>();
```

## Behavior

- Trigger manages menu open/close state and modal visibility
- Modal manages its own backdrop and close-on-escape/backdrop-click
- On `posted`: modal closes after ~800ms (enough to see the reach badge)
- On `settled`: stays open — settlement info matters
- Placeholder text varies by type:
  - Flag: `"Describe the issue..."`
  - Challenge: `"State your case..."`
  - Feedback: `"Share your thoughts..."`

## Extensibility

Once the modal exists, mounting it from a comment's `⋮` menu (Option 2) is just another trigger point — same modal, different caller. Trivial to add later.

## Files

1. `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.ts`
2. `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.ts`
3. `app/elohim-app/src/app/elohim/components/gate-feedback/index.ts`
