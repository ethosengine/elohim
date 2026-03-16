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

### Content Supply Chain

The protocol supplies the content for each pillar through the EPR pipeline:

| Pillar | Protocol supplies... | Sophia renders... |
|--------|---------------------|-------------------|
| Perseus | The exercise content (via EPR content node) | The interactive widget |
| Psyche | The psychometric instrument (item parameters, calibration) | The assessment experience |
| Psephos | The ballot (proposal + options + mechanism + hygiene config) | The voting interface |

**The ballot is a governed content artifact.** It flows through EPR content addressing with governance validation — the same three-pillar coupling (lamad + shefa + qahal) that governs all content. This means:

- Ballot options are EPR-addressed, not assembled ad hoc by the client
- The governance dimension on the EPR Head validates that this ballot was properly constituted
- Election hygiene rules come from the protocol, not from client-side configuration
- You can't tamper with ballot content because it carries integrity verification
- The `PsephosBallotProps` are resolved from an EPR reference, not constructed from loose API calls

Psephos is a pure renderer — it receives the ballot artifact and renders it faithfully, the same way Perseus receives an exercise and renders it faithfully. The protocol owns the content; Sophia owns the experience.

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

### Integration with sophia-core

Psephos extends sophia-core's type system rather than standing alone. The parallel:

| | Perseus | Psyche | Psephos |
|--|---------|--------|---------|
| **Protocol supplies** | Moment with `content: PerseusRenderer` | Moment with `subscaleContributions` | **PsephosBallot** with options + mechanism + hygiene |
| **Renders** | Exercise widgets | Discovery/reflection UX | Voting widgets |
| **Callback** | Recognition with `mastery` | Recognition with `resonance/reflection` | Recognition with `governance: GovernanceResult` |
| **Aggregation** | perseus-score (client-side) | psyche-survey (client-side) | Server-side TallyStrategy (Sprint 3) |

**sophia-core additions:**
- `'governance'` added to `AssessmentPurpose`
- `governance?: GovernanceResult` added to `Recognition`
- `hasGovernanceResult()` type guard
- `GovernanceScoringStrategy` registered via `registerScoringStrategy()`

**Key difference from Perseus/Psyche:** The input is NOT a `Moment`. A ballot is structurally different from an exercise — it has options, mechanism config, and election hygiene rules instead of widget definitions. `PsephosBallot` is the governance equivalent of `Moment`, but its own type.

### Package Structure

```
sophia/packages/psephos/
├── src/
│   ├── index.ts                    # Public API + auto-registers GovernanceScoringStrategy
│   ├── types.ts                    # PsephosBallot, PsephosOption, ElectionHygiene, GovernanceResult
│   ├── governance-strategy.ts      # ScoringStrategy implementation for ballot validation
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

### sophia-core Extensions

These types are added to `sophia-core/src/types.ts`:

```typescript
/** Extended AssessmentPurpose */
type AssessmentPurpose = 'mastery' | 'discovery' | 'reflection' | 'invitation' | 'governance';

/** Added to Recognition interface */
interface Recognition {
  // ...existing fields (momentId, purpose, mastery?, resonance?, reflection?, userInput, timestamp?)...
  governance?: GovernanceResult;
}

/** The governance result — parallel to MasteryResult, ResonanceResult, ReflectionResult */
interface GovernanceResult {
  mechanism: string;
  ballots: BallotEntry[];
  reasoning?: string;
  timestamp: string;
  proposalId: string;
}

/** Type guard — parallel to hasMasteryResult, hasResonanceResult */
function hasGovernanceResult(rec: Recognition): rec is Recognition & { governance: GovernanceResult }
```

### Input: PsephosBallot (What the Protocol Supplies)

The protocol supplies the ballot content, just as it supplies Moments for Perseus and instruments for Psyche. The Angular wrapper transforms `ProposalView` + `ProposalOptionView[]` → `PsephosBallot`.

```typescript
/** The governance equivalent of a Moment — what the protocol supplies to Psephos */
interface PsephosBallot {
  /** Unique ballot identifier (typically proposalId) */
  id: string;

  /** Always 'governance' — aligns with AssessmentPurpose */
  purpose: 'governance';

  /** The proposal being voted on */
  proposal: {
    id: string;
    title: string;
    description: string;
    proposalType: string;
  };

  /** The options to vote on — supplied by the protocol */
  options: PsephosOption[];

  /** Which voting mechanism to render */
  mechanism: 'ranked-choice' | 'approval' | 'score-vote' | 'dot-vote' | 'consent';

  /** Mechanism-specific config (from ProposalView) */
  config: PsephosConfig;

  /** Election hygiene — the protocol's ballot integrity rules */
  hygiene: ElectionHygiene;

  /** Optional: existing ballot for review/amendment */
  previousBallot?: BallotEntry[];
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
  quorumPercentage?: number;
  passageThreshold?: number;
}
```

### Protocol-to-Ballot Mapping

How the Angular wrapper transforms storage-client types to PsephosBallot:

```
ProposalView (storage-client)     PsephosBallot (psephos)
─────────────────────────         ─────────────────────────
proposal.id                  →    ballot.id / ballot.proposal.id
proposal.title               →    ballot.proposal.title
proposal.body                →    ballot.proposal.description
proposal.proposalType        →    ballot.proposal.proposalType
proposal.votingMechanism     →    ballot.mechanism
proposal.scoreMin/Max        →    ballot.config.scoreMin/Max
proposal.dotsPerVoter        →    ballot.config.dotsPerVoter
proposal.quorumPercentage    →    ballot.config.quorumPercentage
proposal.passageThreshold    →    ballot.config.passageThreshold

ProposalOptionView[]         →    ballot.options[]
option.id                    →    option.id
option.label                 →    option.label
option.description           →    option.description
option.position              →    option.position
option.source                →    option.source
option.sourceJustification   →    option.sourceJustification

GovernanceState + elohim     →    ballot.hygiene (defaults per mechanism,
                                    overridable by constitutional rules)
```

### Output: What Psephos Produces

Psephos emits standard `Recognition` objects (from sophia-core) with the `governance` field populated:

```typescript
// Recognition emitted by <psephos-ballot> via onRecognition callback
{
  momentId: ballot.id,           // The ballot ID
  purpose: 'governance',
  governance: {                  // GovernanceResult
    mechanism: 'ranked-choice',
    ballots: [
      { optionId: 'opt-1', rank: 1 },
      { optionId: 'opt-2', rank: 2 },
      { optionId: 'opt-3' }     // Unranked
    ],
    reasoning: 'Option 1 best addresses...',
    timestamp: '2026-03-15T10:30:00Z',
    proposalId: 'prop-123',
  },
  userInput: { /* raw widget state */ },
  timestamp: Date.now(),
}
```

### BallotEntry (shared with storage-client)

```typescript
/** A single entry in the ballot (one per option) — matches storage-client's BallotEntry */
interface BallotEntry {
  optionId: string;
  rank?: number | null;     // ranked-choice: preference order
  score?: number | null;    // score-vote: assigned score
  dots?: number | null;     // dot-vote: dots allocated
  approved?: boolean | null; // approval/consent: yes/no
}
```

The Angular wrapper converts `Recognition.governance` → `CastRankedVoteInputView` and calls the governance API. Psephos never knows about storage-client types.

### GovernanceScoringStrategy

Registered via sophia-core's `registerScoringStrategy()`, parallel to perseus-score and psyche-survey:

```typescript
const GovernanceScoringStrategy: ScoringStrategy = {
  id: 'governance',
  name: 'Governance Ballot',

  getEmptyWidgetIds(ballot: PsephosBallot, userInput: UserInputMap): string[] {
    // Returns option IDs that haven't been voted on yet
    // Mechanism-specific validation:
    //   ranked-choice: at least 1 option ranked
    //   approval: at least 1 option approved
    //   score-vote: all options must be scored
    //   dot-vote: valid (0 dots is intentional non-allocation)
    //   consent: must choose consent or block
  },

  recognize(ballot: PsephosBallot, userInput: UserInputMap): Recognition {
    return {
      momentId: ballot.id,
      purpose: 'governance',
      governance: {
        mechanism: ballot.mechanism,
        ballots: buildBallotEntries(userInput, ballot),
        reasoning: userInput.reasoning as string | undefined,
        timestamp: new Date().toISOString(),
        proposalId: ballot.proposal.id,
      },
      userInput,
      timestamp: Date.now(),
    };
  },
};
```

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

### sophia-core (dependency)

Psephos extends sophia-core's type system:
- Adds `'governance'` to `AssessmentPurpose`
- Adds `GovernanceResult` to `Recognition`
- Registers `GovernanceScoringStrategy` via `registerScoringStrategy()`

### qahal Angular models (NO dependency)

The qahal pillar has comprehensive governance models in:
- `governance-deliberation.model.ts` — VotingMechanism enum, ProposalType, GraduatedFeedbackScale
- `governance-feedback.model.ts` — GovernanceState, ChallengeGrounds, SLA tracking

Psephos does NOT depend on these Angular models. It receives `PsephosBallot` (plain TypeScript interfaces defined in psephos/types.ts) and emits `Recognition` with `GovernanceResult`. The Angular wrapper transforms between qahal models / storage-client types and Psephos's own types.

### storage-client (NO dependency)

`BallotEntry` is defined in both psephos and storage-client with matching shapes. The Angular wrapper handles the translation. Psephos never imports from `@elohim/storage-client`.

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
