# Qahal Governance Write Path Design (Sprint 2)

**Date**: 2026-03-15
**Scope**: Sprint 2 of the Governance Immune System — wire proposals, votes, and discussions to real persistence within collectives

## Problem

Governance UI components exist and are tested (ProposalVoteComponent 99% coverage, ReactionBarComponent 100%, GraduatedFeedbackComponent) but persist to localStorage. Backend CRUD functions exist but aren't exposed via HTTP. No individual vote tracking exists — only aggregate counters on the proposal table.

## Design Decisions

### Proposals, votes, and discussions only

Challenges, appeals, and precedents are deferred to Sprint 3 (the immune system). Sprint 2 gives collectives a voice — "let's decide something together." Sprint 3 adds the immune response — "something's wrong, let's fix it."

### New votes table

The current `proposals` table has `votes_for` / `votes_against` integer counters but no record of who voted. Individual vote records are needed for:
- Attributed voting (who voted what)
- Vote changing (Loomio allows changing your vote as discussion evolves)
- Block justification (blocks require written reason)

```sql
CREATE TABLE votes (
    id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    position TEXT NOT NULL,        -- agree, abstain, disagree, block
    reason TEXT,                   -- required for block, optional otherwise
    anonymous BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(proposal_id, human_id)  -- one vote per person per proposal
);
```

The UNIQUE constraint enforces one vote per person. Changing your vote is an UPDATE, not a new INSERT.

### Configurable vote anonymity

Each proposal has a `voting_anonymous` field (new column on proposals table, default `false`).

- **Attributed** (`false`): `GET /votes` returns full vote records with `humanId`
- **Anonymous** (`true`): `GET /votes` returns votes with `humanId: null` — counts visible, voters aren't

The proposer sets this when creating. Constitutional enforcement of defaults is Sprint 3+ territory.

### Proposals scoped to collectives

A proposal belongs to a collective (via `content_id` which holds the collective ID in governance context). This connects governance to the directory — tap a group, see its proposals.

## Data Model Changes

### New table: votes

See schema above. Positions follow Loomio's 4-position model: `agree`, `abstain`, `disagree`, `block`.

### Modified table: proposals

Add column: `voting_anonymous BOOLEAN NOT NULL DEFAULT FALSE`

### No changes to: discussions, governance_states

These tables already have the right schema.

## HTTP Routes

### New routes (5)

| Route | Method | Purpose |
|-------|--------|---------|
| `/db/governance/proposals` | POST | Create a proposal within a collective |
| `/db/governance/proposals/{id}/votes` | POST | Cast or update a vote |
| `/db/governance/proposals/{id}/votes` | GET | List votes (respects anonymity) |
| `/db/governance/discussions` | POST | Start a discussion thread |
| `/db/governance/discussions/{id}/messages` | POST | Reply to a discussion |

### Existing routes (unchanged)

All existing GET routes for proposals, discussions, governance states, challenges, precedents remain unchanged.

## Frontend Changes

### GovernanceApiService — add POST methods

- `createProposal(input)` → `POST /db/governance/proposals`
- `castVote(proposalId, vote)` → `POST /db/governance/proposals/{id}/votes`
- `getVotes(proposalId)` → `GET /db/governance/proposals/{id}/votes`
- `createDiscussion(input)` → `POST /db/governance/discussions`
- `postMessage(discussionId, message)` → `POST /db/governance/discussions/{id}/messages`

### GovernanceService — replace localStorage

- `submitProposal()` → calls `governanceApi.createProposal()`
- `voteOnProposal()` → calls `governanceApi.castVote()`
- `postMessage()` → calls `governanceApi.postMessage()`
- Remove all `localStorage` read/write for governance data
- Remove `lamad-governance-*` localStorage keys

### New component: CollectiveDetailComponent

When you tap a group in the community directory, you see:
- Group name + description + member faces (reusing FaceCardComponent)
- Proposals tab (active proposals with ProposalVoteComponent)
- Discussions tab (discussion threads)
- "New Proposal" button

### Route addition

`/community/collective/:id` → CollectiveDetailComponent

Linked from the directory's group view (tap a group header or a "View group" action).

## A2O Scenario

```gherkin
Feature: Collective governance
  As a member of a small group
  I want to propose and vote on group decisions
  So that our group self-governs through consent

  Scenario: A small group proposes and votes on their next study
    Given I am a member of the "Valley Bible Study" collective
    When I create a proposal "Study Romans next quarter"
    Then the proposal appears in the collective's governance view
    And other members can vote agree, abstain, disagree, or block
    And votes are attributed by default

  Scenario: Anonymous voting on a sensitive decision
    Given I am a member of the "Valley Bible Study" collective
    When I create a proposal with anonymous voting enabled
    Then vote counts are visible but voters are not identified
    And block votes still require written justification
```

## What This Does NOT Touch

- Challenges, appeals, precedents (Sprint 3)
- Content governance / computed boundaries (emerges from use)
- Elohim resolve function (Sprint 3)
- Constitutional enforcement of voting rules (future)
- Proposal outcome execution (decided → implemented lifecycle)
- Graduated feedback / reaction bar persistence (signal service — separate concern)

## Sprint Roadmap

| Sprint | Scope | Status |
|--------|-------|--------|
| **1** | Directory grid, face cards, household collectives | Done |
| **2 (this design)** | Proposals, votes, discussions — real persistence | Next |
| **3** | Immune system — challenges, appeals, precedents, elohim resolve |  |
