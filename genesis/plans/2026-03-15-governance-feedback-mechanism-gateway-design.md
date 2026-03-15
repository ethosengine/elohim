# Governance Feedback Mechanism Gateway — Design

**Date:** 2026-03-15
**Status:** Approved
**Epic:** Qahal — The Governance Immune System

## Vision

Marshall McLuhan: "The medium is the message." FPTP governance produces polarization as the game plays out. Algorithmic amplification of like buttons and viral competition creates a medium that rewards bullies and fascist tendencies without participants understanding the game.

The Elohim Protocol's governance medium must structurally incentivize collaboration, win-win outcomes, and experienced (not academic) democratic participation. Ranked-choice voting, consent-based decision-making, and proportional representation should be part of everyday content interaction — not relegated to a separate governance dashboard.

Nicky Case's "Evolution of Trust" demonstrates that the structure of interaction determines emergent behavior. The protocol must make cooperative strategies the natural, lived experience.

### Lineage

This design realizes the vision originally seeded in [Bright](https://github.com/Mbd06b/bright) — a suggestion box app inspired by Forby (approval + ranked-choice), Loomio (collaborative decisions), Polis (AI-powered sentiment clustering), Kolibri (universal education), and Holochain (P2P infrastructure). All of these are now woven into the Elohim Protocol.

## Architecture: Three Layers (A → C → B)

The design builds in three layers, each enabling the next:

- **Layer A (this sprint):** Feedback Mechanism Gateway — elohim selects the right governance mechanism for content context, renders it inline, generates REA events
- **Layer C (next):** Polis Sensemaking — accumulated signals cluster into opinion groups, bridging statements surface, elohim synthesizes brackets
- **Layer B (future):** Elohim Deliberation — elohim carry human governance profiles into peer deliberation, traverse hierarchy for settlements, humans opt-in to override

Each layer feeds the next. Nothing is thrown away.

---

## Layer A: Feedback Mechanism Gateway

### Core Concept

Every piece of content has a governance surface. When a user views content, the `FeedbackMechanismGateway` determines which feedback mechanism to render. This is the elohim's first governance act — "election hygiene."

The gateway sits between content rendering and governance UI. It consults the elohim (via ElohimGate trust signals) and returns what to render.

### Mechanism Ladder

| Level | Mechanism | When | Notes |
|-------|-----------|------|-------|
| 0 | **Reasoned dissent only** | Constitutional/settled content | Context menu (flag, challenge, open feedback via GateModalInteraction) always available. No low-friction signals. Nothing is above challenge. |
| 1 | **Emotional reaction** | Low-stakes, personal response | "This resonated with me" |
| 2 | **Graduated scale** | Quality signal needed | Accuracy/usefulness scales (already built in governance-deliberation.model.ts) |
| 3 | **Approval vote** | Multiple options, pick favorites | "Which explanations are clearest?" |
| 4 | **Ranked-choice bracket** | Competing ideas need ordering | "Rank these proposed revisions" — instant-runoff tallying |
| 5 | **Score vote** | Ideas need independent evaluation | "Score each approach 1-10" |
| 6 | **Consent round** | Collective decision with stakes | "Any objections to this change?" — blocks trigger escalation, not veto |
| 7 | **Full deliberation** | High-stakes, affects hierarchy | Constitutional amendments, precedent-setting |

**Key principle:** Even Level 0 has the vertical-dots context menu. The door to governance is never locked.

### Selection Logic

The elohim considers:

- Content governance state (unreviewed → constitutional)
- Active deliberations (is there already a proposal?)
- Trust context from ElohimGate (mastery depth, steward standing, relationship density, governance health, behavioral trust, intent divergence)
- Content type (concept vs. assessment vs. path)
- Viewer's relationship to content (steward, learner, newcomer)

The gateway doesn't just pick — it can explain why this mechanism was chosen: "This content has competing revision proposals from two stewards, so I'm showing you a ranked-choice bracket rather than a simple reaction."

### REA Integration

Every feedback submission generates an economic event:

- **Resource:** the signal (vote, reaction, ranking)
- **Event:** the act of providing governance input
- **Agent:** the human (or their elohim acting as proxy)

These flow into the existing recognition pipeline — governance participation IS stewardship, and it earns recognition accordingly. Curation acts already build steward affinity (wired in steward economy Sprint 2).

### Service Contract

```typescript
interface FeedbackMechanismGateway {
  getMechanism(
    entityType: string,
    entityId: string
  ): Observable<MechanismSelection>;

  submitFeedback(
    entityType: string,
    entityId: string,
    feedback: FeedbackSubmission
  ): Observable<EconomicEventView>;  // returns the REA event created
}

interface MechanismSelection {
  mechanism: MechanismLevel;
  justification: string;           // elohim's reasoning
  options?: ProposalOption[];       // for bracket/ranked/score mechanisms
  config?: VotingConfig;            // score range, dot budget, etc.
  activeDeliberation?: string;      // link to existing proposal if one exists
  contextActions: ContextAction[];  // always present: flag, challenge, open-feedback
}

type MechanismLevel =
  | 'reasoned-dissent-only'
  | 'emotional-reaction'
  | 'graduated-scale'
  | 'approval-vote'
  | 'ranked-choice'
  | 'score-vote'
  | 'consent-round'
  | 'full-deliberation';
```

### Where It Renders

Not a separate page. The gateway produces a `MechanismSelection` that a `GovernanceSurfaceComponent` renders inline at the content. A small, contextual UI element — expands when engaged, collapses to a subtle indicator when not. The vertical dots context menu is always there.

---

## Data Model

### Proposal Options Table

```sql
CREATE TABLE proposal_options (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    position INTEGER NOT NULL,          -- display order
    source TEXT,                         -- 'steward-nominated', 'polis-bridging', 'elohim-synthesized'
    source_justification TEXT,           -- elohim's reasoning for including this option
    created_at TEXT NOT NULL
);
```

### Ranked Votes Table

```sql
CREATE TABLE ranked_votes (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    option_id TEXT NOT NULL,
    rank INTEGER,                       -- for ranked-choice (1 = first preference)
    score INTEGER,                      -- for score-vote (within configured range)
    dots INTEGER,                       -- for dot-vote (allocation from budget)
    approved INTEGER,                   -- for approval vote (1/0)
    reasoning TEXT,
    proxy_elohim_id TEXT,               -- non-null if elohim voted on behalf
    proxy_justification TEXT,           -- elohim's reasoning (audit trail for human override)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(proposal_id, human_id, option_id)
);
```

### Governance Signals Table (Layer C seam)

```sql
CREATE TABLE governance_signals (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    signal_type TEXT NOT NULL,           -- 'reaction', 'scale', 'rank', 'score', 'consent', 'dissent'
    signal_value TEXT NOT NULL,          -- JSON: the actual feedback payload
    mechanism_level INTEGER NOT NULL,    -- 0-7, which mechanism produced this
    proxy_elohim_id TEXT,
    created_at TEXT NOT NULL
);
```

Every mechanism feeds into governance_signals. When Polis sensemaking arrives, it reads from this table with no retrofitting.

### Tally Strategy Pattern (Rust)

```rust
trait TallyStrategy {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig
    ) -> TallyResult;

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        config: &VotingConfig
    ) -> Result<(), BallotError>;
}
```

Each mechanism is a struct implementing the trait, registered in a map. Adding new mechanisms (quadratic, cumulative, whatever emerges) is one file and one registration — loosely coupled, extensible.

Built-in strategies:

| Mechanism | Tally Logic |
|-----------|-------------|
| `ranked-choice` | Instant-runoff: eliminate lowest, redistribute preferences, repeat |
| `approval` | Count approvals per option, highest wins |
| `score-vote` | Sum scores per option, highest wins |
| `dot-vote` | Sum dots per option, highest wins |
| `consent` | Pass unless block exists; blocks trigger escalation with guaranteed engagement |
| `conviction` | Time-weighted accumulation — votes gain weight the longer they're held |

---

## Blocks Are Not Vetoes

In sociocratic consent, a single block stops the process. This doesn't scale — it becomes self-sabotage when anyone can hold a process hostage.

In the Elohim Protocol, a block is an **escalation trigger with guaranteed engagement**:

1. Block is filed with required justification
2. The elohim acknowledges and engages in genuine conversation with the blocker
3. The elohim can escalate up the governance hierarchy as far as necessary
4. At every level, the person gets a real sit-down — the elohim is infinitely patient
5. The decision stands if the hierarchy has justified it well across the deliberation levels consulted
6. The person's dissent is permanently recorded in the settlement log
7. The person can appeal through constitutional channels (existing SLA guarantees apply)

The protocol promises: you will always get a real conversation, not a form letter. Your dignity is never sacrificed for efficiency.

---

## Quorum: Solved by Elohim Proxy

Traditional democratic governance fails because participation is a civic duty most people don't meet. The Elohim Protocol makes participation irrelevant to legitimacy.

Each person's elohim carries their values, mastery profile, steward affinities, and governance disposition. When a governance question arises, the elohim represents their human's interests — faithfully, based on accumulated context.

- Every affected person is always "present" through their elohim
- Humans opt-in to engage directly, not opt-in to participate
- If a human shows up and disagrees with their elohim's position, the elohim explains its reasoning
- The human's final word carries proportional weight to the collective/hierarchy level they represent
- The elohim updates its model of the person for future representation

### Governance Disposition (Layer B seam)

```typescript
interface GovernanceDisposition {
  participationRate: number;
  valueSignals: ValueSignal[];          // "cares about accuracy", "prioritizes accessibility"
  domainAffinities: string[];           // content domains where they have standing
  escalationHistory: number;            // engagement depth signal, not penalty
}
```

Built from the governance_signals table. When elohim compute arrives, this disposition feeds the elohim's deliberation stance.

---

## Settlement Type (Layer B seam)

```typescript
interface Settlement {
  proposalId: string;
  outcome: TallyResult;
  hierarchyLevelsConsulted: string[];
  constraints: Constraint[];
  justification: string;
  dissentLog: DissentEntry[];
  humanOverrides: Override[];
}
```

Exists as a type now, unused. When elohim-to-elohim deliberation arrives, settlements flow through the existing gateway and governance infrastructure.

---

## Integration with Existing Infrastructure

| Existing Piece | How Gateway Uses It |
|----------------|---------------------|
| **ElohimGate** (344 tests, 6 trust signals) | Provides trust context for mechanism selection |
| **Graduated feedback scales** (governance-deliberation.model.ts) | Level 2 mechanisms, already defined |
| **ProposalVoteComponent** (99% coverage) | Renders consent rounds (Level 6) |
| **GateModalInteraction** | Open-feedback channel, always available as context action |
| **Recognition pipeline** (Rust, 5-stage) | Receives REA events from feedback submissions |
| **Steward affinity** (mastery gate + curation deltas) | Governance participation builds affinity |
| **GovernanceService** (Sprint 2, API-backed) | Proposals, votes, discussions persistence |

---

## Pipeline Visualization

```
Layer A (now):     Human ←→ Content ←→ Gateway ←→ Mechanism ←→ REA Event
                                                      ↓
Layer C (next):    Signals accumulate → Polis clusters → Elohim synthesizes brackets
                                                      ↓
Layer B (future):  Elohim carries disposition → Deliberates with peer elohim →
                   Traverses hierarchy → Produces Settlement → Human can override
```

---

## What This Design Does NOT Include

- Polis sensemaking implementation (Layer C — future)
- Elohim-to-elohim deliberation (Layer B — future, requires agent compute)
- Constitutional council sortition (companion spec)
- Specific UI component designs (deferred to implementation planning)

These are all enabled by the seams left in this design. Preparing a place.
