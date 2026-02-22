# Genesis

Genesis seeds the world and validates that creation is good.

Everything that defines *what should exist* and *how to verify it exists correctly* lives here. The source content, the seed data, the seeding tools, the E2E validation harness, and the CI/CD orchestration that ties it all together.

## Structure

```
genesis/
├── Jenkinsfile          # Genesis pipeline: seed + validate
├── docs/                # Source content (markdown, Gherkin scenarios)
│   └── content/
│       └── elohim-protocol/  # BDD feature files by domain
├── data/                # Structured seed data (JSON)
│   └── lamad/
│       ├── content/     # Concept nodes
│       ├── paths/       # Learning paths
│       └── assessments/ # Assessment definitions
├── assets/              # Static assets (images, fonts)
├── blobs/               # Binary content for seeding
├── seeder/              # Holochain seeding tools (TypeScript)
├── a2o/                 # Alpha-to-omega E2E validation
│   ├── features/        # Executable BDD scenarios
│   ├── steps/           # Step definitions (Cucumber-JS)
│   ├── src/             # Test framework (devices, assertions)
│   └── scripts/         # Coverage scanner, skeleton generator
└── orchestrator/        # CI/CD pipeline controller
    ├── Jenkinsfile      # Central webhook receiver
    ├── environments/    # Per-environment config (.env)
    ├── manifests/       # K8s deployment manifests
    └── scripts/         # Deploy helpers (ingress checks, stale detection)
```

## The Genesis Cycle

```
  docs/content/            Raw inspiration — markdown, Gherkin, human-authored
       ↓
  Claude + MCP tools       Non-deterministic synthesis (bounded by constitution)
       ↓
  data/lamad/              Structured JSON — schema-aligned seed data
       ↓
  seeder/                  Deterministic — loads JSON into Holochain DHT
       ↓
  Holochain DHT            Living data
       ↓
  a2o/                     Alpha-to-omega — validates deployed artifacts
       ↓
  orchestrator/            Triggers the whole cycle on every push
```

## Subprojects

### docs/ — Source Content

Raw human-authored content: markdown documents and Gherkin feature files organized by domain (`value_scanner`, `public_observer`, etc.) and governance layer. These are *conceptual* scenarios — what the protocol should do, written as aspirational BDD specs.

### data/ — Seed Data

Schema-aligned JSON produced by the import pipeline (`elohim-import`). This is what the seeder actually loads into the DHT. Content nodes, learning paths, and assessment definitions.

### seeder/ — Seeding Tools

TypeScript scripts that load seed data into a running Holochain instance via doorway. Includes validation, dry-run mode, and snapshot management.

```bash
cd genesis/seeder
npm run seed              # Full seed
npm run seed:validate     # Validate without seeding
npm run seed:dry-run      # Preview what would be seeded
```

### a2o/ — Alpha-to-Omega Validation

E2E test harness that validates *deployed artifacts* against genesis intent. Named "alpha to omega" because it closes the loop: genesis defines the beginning (alpha), a2o verifies the end state (omega).

No build dependency on implementation code. Tests hit deployed doorway endpoints and verify observable behavior.

```bash
cd genesis/a2o
npm test                          # Run @e2e tagged scenarios
npm run test:federation           # Federation-specific tests
npm run test:genesis:dry          # Dry-run genesis scenarios (find gaps)
npm run scan:coverage             # BDD coverage gap analysis
npm run explore                   # Interactive exploration session
```

### orchestrator/ — CI/CD Controller

The **only pipeline that receives GitHub webhooks**. Analyzes changesets and triggers downstream pipelines in dependency order. Also owns K8s manifests and environment configs.

See [`orchestrator/README.md`](./orchestrator/README.md) for pipeline architecture.

## The Synthesis Model

Genesis is the permanent interface between raw human inspiration and the Elohim network's intelligent synthesis. The `docs/` content represents source material that any participant could import — the network then creates meaning, context, learning paths, contributor presences, attributions, and value flows according to constitutional principles.

This synthesis is **non-deterministic by nature**, but bounded:
- Falls within layers of consensus negotiated by participants
- Attributions flow according to constitutional principles
- Outcomes are contextually bounded to human flourishing

The pipeline persists. What changes is *who performs the synthesis*.

**Today**: Claude serves as the embryonic intelligence, with the principal developer as constitutional enforcer. The synthesis loop runs through personal usage of Claude, bounded by human judgment.

**Tomorrow**: The same genesis pipeline operates, but the intelligence performing synthesis is embedded in the network itself, subject to hierarchical constitutional governance of the Elohim participants, hosted on distributed infrastructure they collectively steward.

## Genesis Jenkinsfile

The genesis pipeline seeds content and runs E2E validation against deployed environments.

| Parameter | Description | Default |
|-----------|-------------|---------|
| `TARGET_HOST` | Host to test against | Auto-detected from branch |
| `DOORWAY_HOST` | Doorway API | Auto-detected |
| `SEED_DATA` | Run seeding before tests | `true` |
| `SEED_IDS` | Specific IDs to seed | (all) |
| `FEATURE_AREAS` | Areas to test (multiselect) | (none = skip) |
| `SKIP_TESTS` | Seed only, no tests | `false` |

### Environment Auto-Detection

When triggered by the orchestrator, target is auto-detected from branch:

| Branch | Target | Doorway |
|--------|--------|---------|
| `dev`, `feat-*`, `claude/*` | alpha.elohim.host | doorway-alpha.elohim.host |
| `staging*` | staging.elohim.host | doorway-staging.elohim.host |
| `main` | elohim.host | doorway.elohim.host |

## Dogfooding

Genesis practices what it preaches:
- **Product tests are stored as ContentNodes** with `contentFormat: 'gherkin'`
- **The a2o scanner can fetch specs from the running app** it's testing
- **The app becomes self-documenting** through executable specifications

The same content graph that stores learning content also stores the tests that validate the platform works correctly.
