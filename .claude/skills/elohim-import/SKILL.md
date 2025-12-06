---
name: elohim-import
description: Knowledge and workflows for importing Elohim Protocol content into the lamad learning platform
---

# Elohim Import Skill

This skill provides domain expertise for working with the Elohim Protocol content import pipeline. It transforms raw markdown and Gherkin files into structured ContentNode JSON for the lamad learning system.

## When to Use This Skill

Reference this skill when the user:
- Asks about importing content from `/data/content/`
- Wants to understand the ContentNode schema
- Needs to explore epics, user types, or scenarios
- Is debugging import pipeline issues
- Wants to validate standards compliance
- Needs to manage human network data

## Content Structure

### Source Content Locations

| Directory | Description |
|-----------|-------------|
| `/data/content/elohim-protocol/` | Main protocol content (governance, value_scanner, etc.) |
| `/data/content/fct/` | Foundations for Christian Technology course |
| `/data/content/ethosengine/` | Ethos Engine project content |

### Epic Structure

Each epic follows this directory pattern:

```
[epic_name]/
├── epic.md              # Domain narrative with YAML frontmatter
├── [user_type]/
│   ├── README.md        # Archetype definition
│   └── scenarios/       # Gherkin feature files (.feature)
└── resources/
    ├── books/           # Reference books
    ├── videos/          # Video content
    └── organizations/   # Related organizations
```

### Epics Available

- `governance` - AI constitutional oversight and appeals
- `value_scanner` - Care economy and value recognition
- `public_observer` - Civic participation and oversight
- `autonomous_entity` - Workplace transformation
- `social_medium` - Relationship-centered digital communication
- `economic_coordination` - REA-based value flows

## Architecture

The import pipeline uses **Kuzu embedded graph database** as the source of truth:

```
/data/content/*.md, *.feature
        ↓
   import command → Kuzu database (schema enforced)
        ↓
   db:dump → lamad-seed.cypher (git-committed)
        ↓
   Angular WASM loads Kuzu directly
```

This architecture:
- Enforces schema with primary key uniqueness
- Uses Holochain-style hash IDs for future compatibility
- Enables Cypher graph queries
- Produces git-friendly seed files

## CLI Commands

Run from `elohim-library/projects/elohim-service/`:

### Import Commands

```bash
# Full import to Kuzu database (recommended)
npx ts-node src/cli/import.ts import \
  --source /projects/elohim/data/content \
  --db ./output/lamad.kuzu \
  --full

# Incremental import (only changed files)
npx ts-node src/cli/import.ts import --db ./output/lamad.kuzu

# Skip relationships (faster, uses less memory)
npx ts-node src/cli/import.ts import --db ./output/lamad.kuzu --skip-relationships

# Dry run (don't write to database)
npx ts-node src/cli/import.ts import --db ./output/lamad.kuzu --dry-run
```

### Exploration Commands

```bash
# List all epics with node counts
npx ts-node src/cli/import.ts list-epics

# List user types/archetypes
npx ts-node src/cli/import.ts list-user-types
npx ts-node src/cli/import.ts list-user-types --epic governance

# Explore content with filters
npx ts-node src/cli/import.ts explore --epic governance
npx ts-node src/cli/import.ts explore --user-type policy_maker
npx ts-node src/cli/import.ts explore --type scenario --limit 20
npx ts-node src/cli/import.ts explore --node specific-node-id
```

### Validation Commands

```bash
# Check manifest and import stats
npx ts-node src/cli/import.ts stats
npx ts-node src/cli/import.ts validate

# Validate standards compliance (DID, JSON-LD, Open Graph)
npx ts-node src/cli/import.ts validate-standards
```

### Trust & Human Commands

```bash
# Enrich content with trust scores from attestations
npx ts-node src/cli/import.ts enrich-trust

# Scaffold templates for user types
npx ts-node src/cli/import.ts scaffold --list
npx ts-node src/cli/import.ts scaffold --epic governance
npx ts-node src/cli/import.ts scaffold --epic governance --user policy_maker

# Manage human network
npx ts-node src/cli/import.ts add-human --name "Alice" --id alice --bio "Activist" --category community
npx ts-node src/cli/import.ts add-relationship --from alice --to bob --type neighbor
npx ts-node src/cli/import.ts import-humans
```

### Database Commands (Kuzu Graph DB)

Kuzu is the **source of truth** for all content data.

```bash
# Show database statistics
npx ts-node src/cli/import.ts db:stats --db ./output/lamad.kuzu

# Export to Cypher seed file (git-friendly, for version control)
npx ts-node src/cli/import.ts db:dump \
  --db ./output/lamad.kuzu \
  --output ./output/lamad-seed.cypher

# Export to JSON (for Angular or other consumers)
npx ts-node src/cli/import.ts db:export \
  --db ./output/lamad.kuzu \
  --output ./output/lamad-export

# Execute raw Cypher queries
npx ts-node src/cli/import.ts query --db ./output/lamad.kuzu \
  -q "MATCH (p:LearningPath) RETURN p.id, p.title"

npx ts-node src/cli/import.ts query --db ./output/lamad.kuzu \
  -q "MATCH (c:ContentNode) WHERE c.contentType = 'scenario' RETURN c.id, c.title LIMIT 10"
```

### Path & Content CRUD

```bash
# List all learning paths
npx ts-node src/cli/import.ts path:list --db ./output/lamad.kuzu

# Show path details
npx ts-node src/cli/import.ts path:show --db ./output/lamad.kuzu --id elohim-protocol

# Create new path
npx ts-node src/cli/import.ts path:create --db ./output/lamad.kuzu \
  --id my-journey \
  --title "My Learning Journey" \
  --difficulty beginner

# Add step to path
npx ts-node src/cli/import.ts path:add-step --db ./output/lamad.kuzu \
  --path my-journey \
  --content manifesto \
  --position 0 \
  --title "The Vision"

# Show content node
npx ts-node src/cli/import.ts content:show --db ./output/lamad.kuzu --id manifesto

# Create new content node
npx ts-node src/cli/import.ts content:create --db ./output/lamad.kuzu \
  --id my-concept \
  --type concept \
  --title "My Concept" \
  --content "Description here"
```

#### Cypher Query Examples

```cypher
-- Find all paths with their step counts
MATCH (p:LearningPath)-[:PATH_HAS_STEP]->(s:PathStep)
RETURN p.id, p.title, count(s) as stepCount

-- Find content by type
MATCH (c:ContentNode)
WHERE c.contentType = 'scenario'
RETURN c.id, c.title

-- Find related content (graph traversal)
MATCH (a:ContentNode)-[:RELATES_TO]->(b:ContentNode)
WHERE a.id = 'manifesto'
RETURN b.id, b.title

-- Find path steps with content
MATCH (p:LearningPath)-[:PATH_HAS_STEP]->(s:PathStep)-[:STEP_USES_CONTENT]->(c:ContentNode)
WHERE p.id = 'elohim-protocol'
RETURN s.orderIndex, s.stepTitle, c.title
ORDER BY s.orderIndex
```

### Learning Path Generation

```bash
# Generate a learning path from imported content
npx ts-node src/cli/import.ts generate-path \
  --id governance-intro \
  --title "Introduction to AI Governance" \
  --epic governance \
  --user-type policy_maker \
  --max-steps 10

# Generate with chapters (grouped by content type)
npx ts-node src/cli/import.ts generate-path \
  --id value-scanner-journey \
  --title "Value Scanner Deep Dive" \
  --epic value_scanner \
  --chapters \
  --max-steps 15

# Preview without writing (dry run)
npx ts-node src/cli/import.ts generate-path \
  --id test-path \
  --title "Test Path" \
  --epic governance \
  --dry-run

# Full options
npx ts-node src/cli/import.ts generate-path \
  --id <id>                    # Required: kebab-case path ID
  --title <title>              # Required: display title
  --description <desc>         # Optional: path description
  --purpose <purpose>          # Optional: why follow this path
  --epic <name>                # Filter to specific epic
  --user-type <type>           # Filter to specific user type
  --type <types>               # Content types (default: scenario,role,epic)
  --difficulty <level>         # beginner, intermediate, advanced
  --max-steps <n>              # Maximum steps (default: 10)
  --chapters                   # Organize into chapters by type
  --dry-run                    # Preview without writing
  --output <dir>               # Output directory (default: ./output/lamad)
```

## ContentNode Schema

### Core Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (kebab-case) |
| `contentType` | ContentType | source, epic, scenario, role, reference, example |
| `title` | string | Display title |
| `description` | string | Brief summary |
| `content` | string | Full content body |
| `contentFormat` | ContentFormat | markdown, gherkin, plaintext, html |
| `tags` | string[] | Classification tags |
| `relatedNodeIds` | string[] | Links to related content |
| `metadata` | object | Additional structured data |

### Metadata Fields

| Field | Description |
|-------|-------------|
| `epic` | Parent epic name |
| `userType` | Associated user archetype |
| `derivedFrom` | Source node ID (provenance) |
| `sourceType` | Type of source file |
| `importedAt` | Import timestamp |

## Relationship Types

| Type | Description |
|------|-------------|
| `CONTAINS` / `BELONGS_TO` | Hierarchical parent-child |
| `DERIVED_FROM` / `SOURCE_OF` | Provenance chain |
| `RELATES_TO` | General conceptual connection |
| `REFERENCES` | Content mentions another node |

## Output Structure

```
output/lamad/
├── nodes.json           # All ContentNodes
├── relationships.json   # All relationships
├── import-summary.json  # Import statistics
├── manifest.json        # Incremental tracking
└── paths/               # Generated learning paths
    ├── index.json       # Path catalog/index
    └── [path-id].json   # Individual path definitions
```

## Models & Schemas

### Service Models (elohim-service/src/models/)

| Model | File | Purpose |
|-------|------|---------|
| `ContentNode` | content-node.model.ts | Core content unit with id, title, content, metadata |
| `ContentRelationship` | content-node.model.ts | Graph edges between nodes |
| `PathMetadata` | path-metadata.model.ts | Metadata extracted from file paths |
| `ContentManifest` | manifest.model.ts | Tracks imports for incremental updates |
| `ImportOptions` | import-context.model.ts | Pipeline configuration options |
| `ImportResult` | import-context.model.ts | Pipeline execution results |

### App Models (elohim-app/src/app/lamad/models/)

| Model | File | Purpose |
|-------|------|---------|
| `ContentNode` | content-node.model.ts | App-side content (mirrors service) |
| `LearningPath` | learning-path.model.ts | Structured learning sequences |
| `Exploration` | exploration.model.ts | User exploration state |
| `HumanNode` | human-node.model.ts | Human personas in network |
| `ContentAttestation` | content-attestation.model.ts | Trust attestations |
| `TrustBadge` | trust-badge.model.ts | Visual trust indicators |
| `ContentMastery` | content-mastery.model.ts | Learning progress tracking |
| `KnowledgeMap` | knowledge-map.model.ts | Knowledge graph visualization |

### Type Definitions

**ContentType** (content-node.model.ts):
- `source` - Raw source file (provenance layer)
- `epic` - Domain narrative
- `feature` - Feature specification
- `scenario` - Behavioral specification (gherkin)
- `concept` - Abstract concept
- `role` - Archetype/persona definition
- `video` - Video content
- `organization` - Organization profile
- `book-chapter` - Reference material
- `path` - Learning path
- `assessment` - Assessment instrument
- `reference` - External reference
- `example` - Code or usage example

**ContentFormat**:
- `markdown` - Markdown text
- `gherkin` - Gherkin feature files
- `html` - HTML content
- `plaintext` - Plain text
- `video-embed` - Embedded video
- `external-link` - External URL
- `quiz-json` - Quiz data
- `assessment-json` - Assessment data

**ImportMode** (import-context.model.ts):
- `full` - Import everything from scratch
- `incremental` - Only import changed files
- `schema-migrate` - Update existing to new schema

**EpicCategory** (path-metadata.model.ts):
- `governance`, `autonomous_entity`, `public_observer`
- `social_medium`, `value_scanner`, `economic_coordination`
- `lamad`, `other`

**HumanCategory** (human.service.ts):
- `core-family`, `workplace`, `community`, `affinity`
- `local-economy`, `newcomer`, `visitor`, `red-team`, `edge-case`

## Services

| Service | File | Purpose |
|---------|------|---------|
| `import-pipeline` | import-pipeline.service.ts | Main import orchestration |
| `manifest` | manifest.service.ts | Manifest loading/saving |
| `relationship-extractor` | relationship-extractor.service.ts | Graph relationship inference |
| `standards` | standards.service.ts | DID, JSON-LD, Open Graph generation |
| `trust` | trust.service.ts | Attestation-based trust scoring |
| `human` | human.service.ts | Human network management |
| `scaffold` | scaffold.service.ts | Template generation |

## Database (Kuzu)

### Schema Overview

The Kuzu graph database schema is designed for Holochain compatibility:

| Node Table | Purpose | Maps To |
|------------|---------|---------|
| `ContentNode` | Core content unit | Holochain content entry |
| `LearningPath` | Curated learning journey | Holochain path entry |
| `PathStep` | Step in a learning path | Embedded in path |
| `PathChapter` | Chapter grouping steps | Embedded in path |
| `Agent` | Human, AI, or org | Holochain AgentPubKey |
| `AgentProgress` | User progress on path | Private Holochain entry |
| `ContentAttestation` | Trust endorsement | Holochain link + entry |

| Relationship Table | From → To | Purpose |
|--------------------|-----------|---------|
| `CONTAINS` | ContentNode → ContentNode | Hierarchical |
| `RELATES_TO` | ContentNode → ContentNode | Semantic link |
| `DEPENDS_ON` | ContentNode → ContentNode | Prerequisite |
| `PATH_HAS_STEP` | LearningPath → PathStep | Path structure |
| `PATH_HAS_CHAPTER` | LearningPath → PathChapter | Chapter structure |
| `CHAPTER_HAS_STEP` | PathChapter → PathStep | Chapter steps |
| `STEP_USES_CONTENT` | PathStep → ContentNode | Step content |
| `AUTHORED` | Agent → ContentNode | Authorship |

### Database Files

| File | Purpose |
|------|---------|
| `db/kuzu-schema.ts` | Schema DDL definitions |
| `db/kuzu-client.ts` | KuzuClient class with CRUD operations |
| `db/index.ts` | Module exports |

## Troubleshooting

### Out of Memory

For large imports, increase Node.js memory:
```bash
NODE_OPTIONS="--max-old-space-size=4096" npx ts-node src/cli/import.ts import
```

Or skip relationship extraction:
```bash
npx ts-node src/cli/import.ts import --skip-relationships
```

### Missing Relationships

Relationships are skipped by default for performance. Run with relationships:
```bash
npx ts-node src/cli/import.ts import --full  # includes relationships
```

### Schema Validation Errors

Check that source files have valid YAML frontmatter:
```yaml
---
epic: governance
user_type: policy_maker
---
```

## File Sync

The following files should stay in sync:
- **This skill** documents CLI commands and schemas
- **CLI** (import.ts) implements commands using services
- **Models** define data structures used throughout
- **Services** implement business logic

When modifying any of these, hooks will remind you to update related files.
