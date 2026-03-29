# Qahal Domain

This directory is the **qahal protocol domain** — the social + governance pillar's vocabulary, metadata schemas, and coupling contracts. Qahal is the social layer: governance today, community networking tomorrow.

## Vision

OneBody (church CMS) meets P2P topology. Qahal subsumes the social functions of Meta/Facebook/LinkedIn but grounded in governance-first design. Every social interaction produces signals that flow through the protocol's three-leg coupling.

The protocol makes governance an everyday experience — not something that happens in special rooms, but woven into the social fabric. Ranked-choice, consent, proportional representation become habitual through use, not taught in civics class.

## Two Vocabularies

### Governance (implemented)

| Type | Category | Purpose |
|------|----------|---------|
| `collective` | A (DHT-notarized) | Community with governance structure and constitutional layer |
| `proposal` | A (DHT-notarized) | Formal decision with voting mechanism (consent, ranked-choice, score, etc.) |
| `challenge` | A (DHT-notarized) | Constitutional challenge to a decision, manifest, or content |
| `appeal` | A (DHT-notarized) | Appeal of governance decision to higher constitutional layer |
| `statement` | B2 (agent-scoped + attestation) | Polis-style sensemaking statement for deliberation |

### Social (declared, future)

| Type | Category | Purpose |
|------|----------|---------|
| `post` | B (agent-scoped) | Social post or status update |
| `event` | B (agent-scoped) | Community gathering, meetup, service |
| `group` | A2 (derived via link) | Named group within a collective |
| `message` | B (agent-scoped) | Direct or group message |
| `thread` | B (agent-scoped) | Discussion thread |

Future types are declared in the manifest with `"status": "planned"`. The vocabulary is the contract; implementation follows.

## Governance Mechanism Ladder

Graduated feedback flows upward through formality:

1. **Levels 0-2** (casual): Reactions, informal polling, temperature checks — Angular components
2. **Level 3** (formal proposal): Structured voting with mechanism selection — Angular + Psephos
3. **Levels 4-5** (challenge/appeal): Constitutional review — Psephos web component
4. **Levels 6-7** (constitutional): Cross-collective deliberation, sortition — Psephos formal ballots

Psephos (third Sophia pillar) renders formal ballots with election hygiene. Casual governance stays as Angular components.

## Directory Structure

```
elohim/sdk/domains/qahal/
├── manifest.json               # Vocabulary: 10 content types (5 governance, 5 social)
│                                 Each declares three-leg coupling + claims + observations
├── schemas/
│   ├── collective-metadata.schema.json   # { memberCount, governanceModel, constitutionalLayer, geoBoundary }
│   ├── proposal-metadata.schema.json     # { mechanism, quorum, deadline, collectiveId, state }
│   ├── challenge-metadata.schema.json    # { targetEprId, targetType, escalationPath, state }
│   └── statement-metadata.schema.json    # { polarity, bridgingScore, clusterAffinity, isDivisive }
└── scripts/
    └── codegen.mjs             # Reads manifest + schemas → generates TypeScript
```

## Generated Output

`codegen.mjs` produces files to one location:

| Location | Consumer |
|----------|----------|
| `app/elohim-app/src/app/qahal/generated/` | Angular app (import via `@app/qahal/generated/`) |

Generated files:

| File | Contents |
|------|----------|
| `metadata-types.ts` | `CollectiveMetadata`, `ProposalMetadata`, `ChallengeMetadata`, `StatementMetadata` |
| `content-node-types.ts` | `QahalTypedContentNode` discriminated union, `isCollectiveNode()` / `isProposalNode()` / etc. type guards |
| `coupling-map.ts` | `QAHAL_COUPLING_MAP` — value flows and governance signals per content type |
| `manifest-types.ts` | Content type lists, relationship types, signal map |

## Commands

```bash
pnpm run qahal:codegen           # Generate qahal domain types
pnpm run qahal:codegen:verify    # Check if generated files are stale
```

## Signals

| Signal | Substrate | Action | Emitted When |
|--------|-----------|--------|--------------|
| `governance-decision` | compute | produce | Proposal reaches decision |
| `community-report` | attention | produce | Member flags content/behavior |
| `challenge-filed` | compute | produce | Constitutional challenge filed |
| `appeal-filed` | compute | produce | Appeal filed against decision |
| `consensus-reached` | compute | produce | Deliberation converges |
| `social-engagement` | attention | use | Member engages with social content |
| `relationship-formed` | attention | produce | New relationship established |

## Observations (Feedback)

Every content type declares claims with positive + negative observations:

| Positive | Negative | Instrument |
|----------|----------|------------|
| `governance-health` | `governance-outcome-divergence` | outcome-correlation |
| `social-health` | `social-isolation` | distribution-health |
| `participation-breadth` | `participation-concentrated` | distribution-health |
| `community-growth` | `community-attrition` | retention-check |
| `decision-legitimacy` | `decision-challenged` | outcome-correlation |

## Key Services (Angular)

| Service | Purpose |
|---------|---------|
| `CollectiveService` | Community CRUD, membership management |
| `MechanismSelectionService` | Voting mechanism selection based on proposal type |
| `SignalAccumulationService` | Graduated feedback → formal proposal escalation |
| `BracketSynthesisService` | Polis bracket synthesis for sensemaking |

## Cross-Pillar Coupling

Qahal governance gates are informed by other pillars:

- **lamad**: Mastery at `apply` level gates governance participation weight
- **imagodei**: Identity attestations establish who can participate in which collective
- **shefa**: Governance decisions produce economic events; stewardship standing informs voting weight

## Related Files

| Purpose | Path |
|---------|------|
| Angular pillar | `app/elohim-app/src/app/qahal/` |
| Governance sprint plans | `genesis/plans/2026-03-15-governance-gateway-sprint{3-9}-plan.md` |
| Protocol schemas | `elohim/sdk/schemas/v1/` |
| Psephos design | See Sophia architecture notes |
