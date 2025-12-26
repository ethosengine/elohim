# Genesis

The **Genesis** project is the meta-infrastructure layer for the Elohim Protocol. It handles the complete cycle from raw source content through seeding and validation.

## The Genesis Pipeline

Genesis is the permanent interface between raw human inspiration and the Elohim network's intelligent synthesis. The `genesis/docs` content represents source material that any participant could import - the network then creates meaning, context, learning paths, contributor presences, attributions, and value flows from that raw content according to constitutional principles.

This synthesis is **non-deterministic by nature**, but bounded:
- Falls within layers of consensus negotiated by participants
- Attributions flow according to constitutional principles
- Outcomes are contextually bounded to human flourishing

The pipeline persists. What changes is *who performs the synthesis*.

### Current State: Embryonic Intelligence

Today, the intelligent synthesis is centralized in the development workflow:

- **Claude** serves as the embryonic form of the Elohim intelligence - synthesizing raw inspiration into structured content, adapting to human flourishing through iterative refinement
- **[Matthew Dowell](https://ethosengine.com)** (principal developer) acts as the constitutional hierarchy enforcer, stewarding the coupling of responsibility, power, design, and value during the protocol's genesis

This arrangement exists because no distributed infrastructure yet hosts the intelligence. The synthesis loop runs through personal usage of Claude, bounded by human judgment.

### Success Criteria

Genesis succeeds when synthesis migrates from Claude+developer to the deployed network:

1. **Any user imports** raw content through the genesis pipeline
2. **Elohim network synthesizes** meaning, context, learning paths, and attributions
3. **Constitutional negotiation** determines value flows and contributor presence
4. **Consensus layers** bound the non-deterministic outcomes to human flourishing

At that point, the same genesis pipeline operates - but the intelligence performing synthesis is embedded in the network itself, subject to hierarchical constitutional governance of the Elohim participants, hosted on distributed infrastructure they collectively steward.

## Purpose

Genesis manages the lifecycle: **source → seed → validate → feedback**

- **Source**: Raw documentation, Gherkin specs, and content (`docs/`)
- **Seed**: Transform and load content into Holochain (`seeder/`, `data/`)
- **Validate**: BDD tests proving the system works (`Jenkinsfile`)
- **Feedback**: Performance metrics and validation reports

## Structure

```
genesis/
├── Jenkinsfile          # Pipeline: seed + validate in one flow
├── docs/                # Raw source documentation
│   └── content/         # Markdown, Gherkin source files
├── data/                # Structured seed data
│   └── lamad/           # Learning content JSON
│       ├── content/     # Concept nodes
│       ├── paths/       # Learning paths
│       └── assessments/ # Assessment definitions
└── seeder/              # Holochain seeding tools
    └── src/             # TypeScript seeder scripts
```

## Pipeline

```
genesis/docs/content/      [Raw markdown, Gherkin - human authored]
      ↓
   Claude + MCP tools      [Non-deterministic, creative transformation]
      ↓
genesis/data/lamad/        [Structured JSON - schema-aligned seed data]
      ↓
   genesis/seeder          [Deterministic script - loads JSON to DHT]
      ↓
Holochain DHT              [Production data]
      ↓
   genesis/Jenkinsfile     [BDD validation - confirms it works]
```

## Jenkinsfile Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `TARGET_HOST` | Host to test against | `https://staging.elohim.host` |
| `DOORWAY_HOST` | Doorway API (auto-detected) | - |
| `SEED_DATA` | Run seeding before tests | `true` |
| `SEED_IDS` | Specific IDs to seed (comma-separated) | (all) |
| `FEATURE_AREAS` | Feature areas to test (multiselect) | (none = skip tests) |
| `TEST_PATH_ID` | BDD test path ID | `bdd-smoke-tests` |
| `TEST_TAGS` | Filter tests by tag | (all) |
| `SKIP_TESTS` | Seed only, no tests | `false` |
| `ANALYZE_DRIFT` | Analyze schema drift | `true` |

### Feature Areas

The `FEATURE_AREAS` parameter controls which parts of the application are tested:

- **all** - Run all tests
- **doorway** - Visitor flow, onboarding, hosting
- **imagodei** - Identity, profile, presence
- **lamad** - Learning content, paths, assessments
- **shefa** - Dashboard, metrics, economics
- **qahal** - Community, governance, deliberation

Selecting nothing skips tests entirely (useful for seed-only runs).

## Usage

### Seed + Validate (Full Pipeline)

```bash
# Trigger via Jenkins
jenkins job trigger genesis/Jenkinsfile \
  -p TARGET_HOST=https://staging.elohim.host
```

### Seed Only

```bash
# Via Jenkins (skip tests)
jenkins job trigger genesis/Jenkinsfile \
  -p TARGET_HOST=https://staging.elohim.host \
  -p SKIP_TESTS=true

# Directly
cd genesis/seeder
npm install
HOLOCHAIN_ADMIN_URL="wss://doorway-dev.elohim.host?apiKey=..." npm run seed
```

### Validate Only

```bash
# Via Jenkins (skip seeding)
jenkins job trigger genesis/Jenkinsfile \
  -p TARGET_HOST=https://staging.elohim.host \
  -p SEED_DATA=false
```

### Local Development

```bash
# From elohim-app
npm run hc:seed              # Full seed to local conductor
npm run hc:seed:sample       # Sample (10 items)

# From genesis/seeder
cd genesis/seeder
npm run seed                 # Full seed
npm run seed:sample          # Limited sample
```

## Dogfooding

The Genesis pipeline practices what it preaches:
- **Product tests are stored as ContentNodes** with `contentFormat: 'gherkin'`
- **The pipeline fetches test specs from the running app** it's testing
- **The app becomes self-documenting** through executable specifications

This means the same content graph that stores learning content also stores the tests that validate the platform works correctly.
