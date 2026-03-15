# Psephos — Governance Ballot Rendering for Sophia

**Date:** 2026-03-15
**Status:** Design
**Epic:** Qahal — The Governance Immune System
**Workspace:** `sophia/packages/psephos/`

---

## What Is Psephos?

A **psephos** (ψῆφος) is the pebble dropped into an urn to cast a vote in Athenian democracy. It's the root of "psephology" — the study of elections. The name captures exactly what this package does: it renders the artifact through which a choice is expressed.

Psephos is the third pillar of the Sophia rendering ecosystem:

| Package | Domain | Renders | Output |
|---------|--------|---------|--------|
| **Perseus** | Learning | Exercise widgets (radio, input, expression, etc.) | Recognition (correct/incorrect + score) |
| **Psyche** | Assessment | Psychometric instruments (mastery, discovery, reflection) | Recognition (ability estimate + confidence) |
| **Psephos** | Governance | Ballot widgets (ranked-choice, approval, score, dot, consent) | BallotRecognition (preferences + reasoning) |

Each names the *object*, not the institution. Perseus is the exercise, not the classroom. Psyche is the measure, not the test. Psephos is the ballot, not the election.

---

## Why a Separate Rendering Package?

**Election hygiene matters.** How you present choices shapes outcomes. This is McLuhan's "the medium is the message" applied to ballot design:

- Position bias: options listed first get disproportionate support
- Visual weight bias: larger/bolder options attract more clicks
- Anchoring bias: showing current results before voting creates bandwagon effects
- Friction asymmetry: if blocking is easier than consenting (or vice versa), outcomes skew

Psephos builds election hygiene into every widget, the same way Perseus builds pedagogical soundness into every exercise. These aren't optional features — they're structural properties of the rendering.

**Casual governance stays in Angular.** Not every governance interaction needs ballot-grade rendering. The mechanism ladder has two zones:

| Levels | Mechanism | Renderer | Why |
|--------|-----------|----------|-----|
| 0 | Reasoned dissent only (context menu) | Angular | UI chrome, not a ballot |
| 1 | Emotional reactions | Angular | Lightweight inline signals |
| 2 | Graduated feedback scales | Angular | Simple structured input |
| 3 | Approval vote | **Psephos** | Multiple options, position bias matters |
| 4 | Ranked-choice bracket | **Psephos** | Complex interaction, IRV visualization |
| 5 | Score vote | **Psephos** | Range inputs need equal visual weight |
| 6 | Consent round | **Psephos** | Block/consent requires careful framing |
| 7 | Full deliberation | **Psephos** | Highest stakes, maximum hygiene |

The boundary is clear: when the elohim selects a formal voting mechanism (levels 3-7), it loads a Psephos ballot. When it selects a casual signal (levels 0-2), it renders Angular components inline.

---

## Architecture

### Package Structure

```
sophia/packages/psephos/
├── src/
│   ├── index.ts                    # Public API
│   ├── types.ts                    # PsephosBallotProps, BallotRecognition, ElectionHygiene
│   ├── psephos-renderer.tsx        # Main renderer (React, like Perseus)
│   ├── widgets/
│   │   ├── ranked-choice.tsx       # Drag-to-rank or click-to-assign
│   │   ├── approval.tsx            # Checkbox per option
│   │   ├── score-vote.tsx          # Slider/number input per option
│   │   ├── dot-vote.tsx            # Budget allocation with constraint
│   │   └── consent.tsx             # Consent/Block with reasoning prompt
│   ├── hygiene/
│   │   ├── randomize-options.ts    # Fisher-Yates shuffle, seeded for reproducibility
│   │   ├── equal-weight.ts         # CSS constraints for visual parity
│   │   └── confirmation-step.tsx   # "You chose X. Submit?" interstitial
│   └── __tests__/
│       ├── ranked-choice.test.tsx
│       ├── approval.test.tsx
│       ├── score-vote.test.tsx
│       ├── dot-vote.test.tsx
│       ├── consent.test.tsx
│       └── hygiene.test.ts
├── package.json                    # @ethosengine/psephos
└── tsconfig.json
```

### Web Component: `psephos-element`

```
sophia/packages/psephos-element/
├── src/
│   ├── index.ts                    # Defines <psephos-ballot> custom element
│   └── psephos-ballot.ts           # React → Web Component bridge
├── package.json                    # @ethosengine/psephos-element
└── rollup.config.js                # UMD bundle (like sophia-element)
```

`<psephos-ballot>` is the web component that wraps the React rendering. It follows the same pattern as `<sophia-question>`:
- Receives props as attributes/properties
- Emits `recognition` custom events
- UMD bundle loaded by the Angular app

### Angular Wrapper: `psephos-plugin`

```
app/elohim-library/projects/psephos-plugin/
├── src/
│   ├── lib/
│   │   ├── psephos-ballot.component.ts   # Angular wrapper for <psephos-ballot>
│   │   └── psephos-ballot.module.ts
│   └── public-api.ts
└── package.json                          # @elohim/psephos-plugin
```

Follows the same pattern as `sophia-plugin` in `elohim-library`. Wraps the web component with Angular inputs/outputs.

---

## Widget Specifications

### Ranked-Choice (`ranked-choice`)

**Interaction:** Drag-to-rank or click-to-assign rank number per option.

**Hygiene:**
- Options rendered in random order (seeded by proposal ID + human ID for consistency across revisits)
- Equal-height option cards
- Rank numbers displayed clearly alongside option labels
- Unranked options shown in a separate "not ranked" zone
- Partial ranking allowed (rank your top N)

**Validation:**
- No duplicate ranks
- At least 1 option ranked

**Display after vote:**
- Round-by-round IRV elimination visualization
- Winner highlighted with percentage
- "Your ranking was: 1. X, 2. Y, 3. Z"

### Approval (`approval`)

**Interaction:** Checkbox per option. Check = approve. Multiple selections allowed.

**Hygiene:**
- Random option order
- Equal visual weight (no option pre-checked)
- Clear "you may select multiple" instruction

**Validation:**
- At least 1 option approved

**Display after vote:**
- Bar chart of approval counts
- Threshold line if passage_threshold set

### Score Vote (`score-vote`)

**Interaction:** Slider or number input per option within `[scoreMin, scoreMax]` range.

**Hygiene:**
- Random option order
- All sliders start at midpoint (not min — prevents anchoring at zero)
- Equal-width slider tracks
- Score labels at endpoints ("1 = Strongly Oppose", "10 = Strongly Support")
- Must explicitly set each score (no defaults counted as votes)

**Validation:**
- All options must be scored (no unscored options)
- Scores within `[scoreMin, scoreMax]`

**Display after vote:**
- Mean score per option with distribution spread
- Total score ranking

### Dot Vote (`dot-vote`)

**Interaction:** Increment/decrement buttons per option. Budget display: "N dots remaining."

**Hygiene:**
- Random option order
- Budget constraint enforced visually (can't exceed `dotsPerVoter`)
- Zero dots is valid (intentional non-allocation)
- Equal visual weight per option row

**Validation:**
- Total dots ≤ `dotsPerVoter`
- No negative dot counts

**Display after vote:**
- Dot distribution visualization (stacked dots or bar chart)
- Total dots per option ranked

### Consent (`consent`)

**Interaction:** Two primary buttons: **Consent** (green) and **Block** (amber, not red — blocks are escalation triggers, not rejection).

**Hygiene:**
- Consent and Block buttons equal size and visual weight
- Block requires reasoning text (minimum 50 characters)
- Consent optionally allows reasoning ("Why I support this")
- Clear explanation: "Blocking does not veto — it triggers a facilitated conversation"
- No peer pressure: don't show current vote counts before submission

**Validation:**
- Must choose Consent or Block
- Block requires reasoning

**Display after vote:**
- Consent count vs Block count
- If blocked: "Escalation in progress — the elohim will engage with concerns"
- Block reasoning visible to all (transparency)

---

## Election Hygiene System

Every Psephos widget receives an `ElectionHygiene` configuration:

```typescript
interface ElectionHygiene {
  randomizeOrder: boolean;        // Shuffle option display order
  randomSeed?: string;            // Seed for reproducible shuffle (proposalId + humanId)
  equalVisualWeight: boolean;     // CSS constraints for visual parity
  requireReasoning: boolean;      // Require text justification for votes
  reasoningMinLength?: number;    // Minimum characters for reasoning (default: 50 for blocks)
  showResultsAfterVote: boolean;  // Only show tally after submission
  confirmBeforeSubmit: boolean;   // Show confirmation interstitial
  hideVoterCount: boolean;        // Don't show "N people have voted" before voting
}
```

**Default hygiene per mechanism:**

| Mechanism | randomize | equalWeight | requireReasoning | showAfter | confirm | hideCount |
|-----------|-----------|-------------|------------------|-----------|---------|-----------|
| ranked-choice | ✓ | ✓ | ✗ | ✓ | ✓ | ✓ |
| approval | ✓ | ✓ | ✗ | ✓ | ✗ | ✓ |
| score-vote | ✓ | ✓ | ✗ | ✓ | ✓ | ✗ |
| dot-vote | ✓ | ✓ | ✗ | ✓ | ✓ | ✗ |
| consent | ✗ (only 2 options) | ✓ | blocks only | ✓ | ✓ | ✓ |

The elohim can override defaults based on context. For example, a constitutional amendment might set `requireReasoning: true` for ALL votes, not just blocks.

---

## Type Definitions

### Input: What Psephos Receives

```typescript
/** Props passed to <psephos-ballot> */
interface PsephosBallotProps {
  /** Which voting mechanism to render */
  mechanism: 'ranked-choice' | 'approval' | 'score-vote' | 'dot-vote' | 'consent';

  /** The options to vote on */
  options: PsephosOption[];

  /** Mechanism-specific configuration */
  config: PsephosConfig;

  /** Election hygiene settings */
  hygiene: ElectionHygiene;

  /** Proposal context (for display) */
  context: {
    proposalId: string;
    title: string;
    description: string;
  };
}

/** A single voting option */
interface PsephosOption {
  id: string;
  label: string;
  description: string;
  position: number;         // Original position (before randomization)
  source?: string;          // Who proposed this option
  sourceJustification?: string;
}

/** Mechanism-specific config */
interface PsephosConfig {
  scoreMin?: number;        // score-vote: minimum score
  scoreMax?: number;        // score-vote: maximum score
  dotsPerVoter?: number;    // dot-vote: budget per voter
}
```

### Output: What Psephos Produces

```typescript
/** Emitted when voter submits their ballot */
interface BallotRecognition {
  /** Which mechanism was used */
  mechanism: string;

  /** The voter's choices */
  ballots: BallotEntry[];

  /** Optional reasoning text */
  reasoning?: string;

  /** When the ballot was cast */
  timestamp: string;

  /** Proposal context for downstream processing */
  proposalId: string;
}

/** A single entry in the ballot (one per option) */
interface BallotEntry {
  optionId: string;
  rank?: number;            // ranked-choice: preference order
  score?: number;           // score-vote: assigned score
  dots?: number;            // dot-vote: dots allocated
  approved?: boolean;       // approval/consent: yes/no
}
```

These types align exactly with `@elohim/storage-client`'s generated `BallotEntry` and `CastRankedVoteInputView`. The Angular wrapper converts `BallotRecognition` → `CastRankedVoteInputView` and calls the governance API.

---

## Integration with Elohim-App

### Data Flow

```
ContentView
  └─ FeedbackMechanismGateway (Angular)
       ├─ MechanismSelectionService → {level, renderTarget}
       │
       ├─ renderTarget: 'angular' (levels 0-2)
       │   ├─ ContextMenuOnly
       │   ├─ ReactionBar
       │   └─ GraduatedFeedback
       │
       └─ renderTarget: 'psephos' (levels 3-7)
           └─ PsephosBallotWrapper (Angular)
               ├─ Loads options via GovernanceApiService
               ├─ Builds PsephosBallotProps
               ├─ Mounts <psephos-ballot> web component
               ├─ Listens for BallotRecognition event
               ├─ Converts to CastRankedVoteInputView
               ├─ Calls GovernanceApiService.castRankedVotes()
               └─ Displays tally result via GovernanceApiService.getTally()
```

### Build Pipeline

1. `@ethosengine/psephos` built as part of Sophia workspace (`pnpm build`)
2. `@ethosengine/psephos-element` bundled as UMD (`pnpm build:umd`)
3. UMD bundle copied to `app/elohim-app/src/assets/` (like `sophia-element`)
4. `@elohim/psephos-plugin` wraps the web component for Angular
5. `FeedbackMechanismGateway` imports `PsephosBallotComponent` from plugin

Same pipeline as Sophia. `prebuild` script checks for psephos-element UMD before elohim-app builds.

---

## Accessibility

Every Psephos widget must be:
- **Keyboard navigable:** Tab through options, Enter/Space to select, arrow keys to rank/adjust
- **Screen reader friendly:** ARIA labels on all interactive elements, live regions for budget updates (dot-vote)
- **Color-independent:** Don't rely solely on color to distinguish consent/block states
- **Mobile-responsive:** Touch-friendly targets (min 44px), swipe-to-rank for ranked-choice

---

## Relationship to Existing Models

The qahal pillar already has comprehensive governance models in:
- `governance-deliberation.model.ts` — VotingMechanism enum, ProposalType, GraduatedFeedbackScale
- `governance-feedback.model.ts` — GovernanceState, ChallengeGrounds, SLA tracking

Psephos does NOT depend on these Angular models. It receives `PsephosBallotProps` (plain TypeScript interfaces) and emits `BallotRecognition`. The Angular wrapper in `psephos-plugin` bridges between the qahal models and Psephos's props.

---

## Testing Strategy

- **Unit tests:** Each widget renders correctly, validates input, emits correct BallotRecognition
- **Hygiene tests:** Randomization produces uniform distribution over many runs, visual weight CSS verified
- **Integration tests:** Web component mounts, receives props, emits events
- **Accessibility tests:** Keyboard navigation, ARIA roles, screen reader output
- **Election hygiene regression:** Specific tests that position bias isn't introduced by future changes

Use `@testing-library/react` (same as Perseus/Psyche).

---

## Implementation Sprints (Suggested)

**Psephos Sprint 1:** Package setup + approval widget (simplest) + web component + Angular wrapper
**Psephos Sprint 2:** Ranked-choice widget with drag-to-rank + IRV result visualization
**Psephos Sprint 3:** Score-vote + dot-vote widgets
**Psephos Sprint 4:** Consent widget with block escalation messaging
**Psephos Sprint 5:** Election hygiene system (randomization, confirmation, result hiding)

Each sprint produces a working UMD bundle that the elohim-app can consume.
