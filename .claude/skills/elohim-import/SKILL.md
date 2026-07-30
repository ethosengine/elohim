---
name: elohim-import
description: Workflows for transforming Elohim Protocol source content (markdown, Gherkin) into seed-data for the lamad learning platform. Use when "import this content to lamad", "transform markdown to content nodes", "run the elohim-import CLI", or working on the genesis → seed-data pipeline. Complements holochain-import (DHT-seed layer) by handling the transformation layer.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/elohim-import"
---

# Elohim Import Skill

Transforms raw Elohim Protocol content (markdown, Gherkin) into structured lamad learning content — either direct 1:1 import or creative transformation into a concept graph plus learning paths.

## Design Principles

1. **Rust DNA is source of truth** — TypeScript/JSON schemas align with the Holochain Rust structs.
2. **The DNA validates metadata, not content.** Required metadata: `id`, `title`. Optional: `format_hint`, `blob_hash`, signed provenance. `contentFormat` accepts ANY string (a client rendering hint, never DNA-validated), so new formats never require a DNA upgrade. Blobs live in the cache layer, not the DHT.
3. **No static complexity** — complexity is relative to the learner and is never stored on content. Encode it structurally: `DEPENDS_ON` relationships, path ordering, chunking decisions.
4. **This skill IS the AI** — during prototyping Claude makes the complexity, prerequisite-depth, ordering, and chunking judgments at import time, based on the target learner profile. In production, distributed AI agents compute paths dynamically per-learner.

## Content Pipeline

1. **Holochain DNA (Rust)** — source of truth: entry types, relationships.
2. **hc-rna (Rust)** — schema analyzer → JSON Schema + validation.
3. **`genesis/docs/content/`** — raw markdown, Gherkin; human authored.
4. **Claude + MCP tools** — non-deterministic, creative transformation.
5. **`genesis/data/lamad/`** — structured JSON, schema-aligned seed data.
6. **`hc-rna-fixtures --analyze`** — validate metadata before seeding.
7. **`genesis/seeder`** — deterministic script, loads JSON to DHT.
8. **Holochain DHT** — production data.

## When to Use This Skill

Importing raw source files directly as content; creating learning paths or modules from docs; transforming markdown into structured concepts; building the content graph (concepts + relationships); creating quizzes or assessments; seeding content to Holochain.

## Directory Structure

| Directory | Purpose |
|-----------|---------|
| `/docs/content/` | Raw source content (markdown, gherkin) |
| `/data/lamad/` | Structured JSON seed data |
| `/data/lamad/content/` | Concept JSON files |
| `/data/lamad/paths/` | Learning path JSON files |
| `/data/lamad/assessments/` | Assessment JSON files |
| `/data/lamad/audiences/` | Audience archetype YAML files |

## MCP Server: elohim-content

**Source reading** — `read_doc` reads markdown/gherkin from `docs/content/`, e.g. `{"path": "elohim-protocol/governance/epic.md"}`; `list_docs` lists by epic or pattern, e.g. `{"epic": "governance"}`; `search_docs` searches for concepts/keywords, e.g. `{"query": "constitutional oversight"}`.

**Seed data CRUD** (over `data/lamad/`) — `list_seeds`, `read_seed`, `write_seed`, `delete_seed`, `validate_seed` (validates against Holochain schemas).

**Content graph** — `create_concept` (atomic concept from docs or raw source), `create_relationship` (link concepts: `DEPENDS_ON`, `RELATES_TO`, `CONTAINS`, …), `query_graph` (by tags/relationships), `get_related`, `update_concept`, `delete_concept`.

**Path authoring** (paths are views/projections over the graph) — `create_path` (ordered traversal), `create_chapter`, `create_module`, `create_section`, `add_to_path` (concept at position), `remove_from_path` (removes from path, keeps in graph), `generate_path` (auto-generate from a graph region).

**Assessments** — `create_quiz` (from concepts), `create_assessment` (build instrument), `update_assessment` (questions/scoring).

## Import Modes

### 1. Direct Source Import

Preserve the original markdown 1:1. Use when the source should be preserved as-is, the content is already atomic (one document = one concept), or you want a 1:1 docs→content mapping.

```
read_doc(path: "elohim-protocol/manifesto.md")
create_concept(
  id: "manifesto",
  title: "Elohim Protocol Manifesto",
  content: <full markdown from file>,
  sourceDoc: "elohim-protocol/manifesto.md",
  tags: ["elohim", "manifesto", "vision"]
)
```

### 2. Creative Transformation

Split source material into multiple atomic concepts with relationships. Use when the source covers multiple distinct concepts, you want a knowledge graph, or content must be chunked for learning.

```
read_doc(path: "elohim-protocol/governance/epic.md")
create_concept(id: "separation-of-powers", ...)
create_concept(id: "appeals-process", ...)
create_concept(id: "constitutional-oversight", ...)
create_relationship(source: "appeals-process", target: "separation-of-powers", type: "DEPENDS_ON")
```

## Content Model

The **content graph** is the multi-dimensional knowledge graph holding all concepts and their relationships. A **path** is one ordered traversal/projection over it (Chapter → Module → Section). Learners can leave a path to explore related graph nodes.

### Concept Schema

Aligned with the Holochain DNA `Content` struct:

```json
{
  "id": "separation-of-powers",
  "title": "Separation of Powers",
  "content": "Markdown content here...",
  "contentFormat": "markdown",
  "contentType": "article",
  "sourceDoc": "elohim-protocol/governance/epic.md",
  "tags": ["governance", "constitutional"],
  "relationships": [
    {"target": "appeals-process", "type": "RELATES_TO"},
    {"target": "ai-oversight", "type": "DEPENDS_ON"},
    {"target": "governance-epic", "type": "DERIVED_FROM"}
  ],
  "estimatedMinutes": 8
}
```

| Field | Type | Description |
|-------|------|-------------|
| `estimatedMinutes` | number | Reading/viewing time in minutes (word count / 200 wpm) |
| `contentType` | `"article"` \| `"video"` \| `"simulation"` \| `"assessment"` \| `"discovery-assessment"` | Semantic type |
| `contentFormat` | ContentFormat | How content is rendered (see table below) |
| `metadata` | object | Extensible JSON for additional data |

### ContentFormat Values

All content must carry a `contentFormat` that maps to a renderer:

| Format | Description | Renderer |
|--------|-------------|----------|
| `markdown` | Rich text with embedded media (default for most content) | MarkdownRenderer |
| `html` | Raw HTML content | MarkdownRenderer |
| `plaintext` | Unformatted text | MarkdownRenderer |
| `gherkin` | Behavior-driven development scenarios | GherkinRenderer |
| `html5-app` | Interactive web apps (e.g., [Evolution of Trust](https://github.com/ncase/trust)). Rendered via sandboxed iframe. Can have attestation quizzes built to verify understanding. | IframeRenderer |
| `video-embed` | Embedded video (YouTube, Vimeo, etc.) | IframeRenderer |
| `video-file` | Direct video file (blob-based streaming) | VideoRenderer |
| `audio-file` | Direct audio file (podcasts, lectures, music) | AudioRenderer |
| `perseus-quiz-json` | Legacy Khan Academy Perseus quiz format | SophiaRenderer |
| `sophia-quiz-json` | Sophia Moment format with purpose-based assessment (mastery/discovery/reflection). Uses psyche-core for psychometric aggregation. | SophiaRenderer |
| `epub` | E-book format | EpubRenderer |
| `external-link` | Link to external resource | ExternalLinkRenderer |

**Content Principle:** Content should translate into consumable multimedia experiences. Raw data formats (like `bible-json`, `contributor-json`) are not valid — transform them into `markdown` or an appropriate media format during import.

Example `html5-app` content, showing the extra fields an embedded app needs:

```json
{
  "id": "simulation-evolution-of-trust",
  "contentType": "simulation",
  "title": "The Evolution of Trust",
  "description": "An interactive guide to the game theory of trust",
  "url": "https://ncase.me/trust/",
  "content": "https://ncase.me/trust/",
  "contentFormat": "html5-app",
  "metadata": {
    "author": "Nicky Case",
    "sourceUrl": "https://github.com/ncase/trust",
    "embedStrategy": "iframe",
    "requiredCapabilities": ["javascript"],
    "securityPolicy": {
      "sandbox": ["allow-scripts", "allow-same-origin"]
    }
  }
}
```

### Relationship Types (DNA-aligned)

Aligned with the Holochain DNA `Relationship.relationship_type`:

| Type | Description |
|------|-------------|
| `RELATES_TO` | General association between concepts |
| `CONTAINS` | Parent-child hierarchical relationship |
| `DEPENDS_ON` | Prerequisite dependency (must understand first) |
| `IMPLEMENTS` | Implementation of a concept |
| `REFERENCES` | Citation or reference to another concept |
| `DERIVED_FROM` | This content was derived from source content |

`DERIVED_FROM` links derived/transformed content back to its source doc — e.g. `separation-of-powers.json`, derived from `governance-epic.md`, carries `{target: "governance-epic", type: "DERIVED_FROM"}`. It carries provenance tracking, "View source document" links in the UI, and lets AI traverse source→derived to build scope/sequence.

### Path Schema

Paths use a **4-level hierarchy** aligned between MCP schemas and Angular models: `Path` → `Chapter` (`PathChapter`, requires `modules: PathModule[]`) → `Module` (`PathModule`, requires `sections: PathSection[]`) → `Section` (`PathSection`, requires `conceptIds: string[]`, linking to ContentNode IDs). Sections also carry `estimatedMinutes` and `assessments[]`.

```json
{
  "id": "elohim-protocol",
  "title": "Elohim Protocol: Living Documentation",
  "difficulty": "beginner",
  "chapters": [
    {
      "id": "chapter-2-governance",
      "title": "AI Governance",
      "description": "Constitutional oversight, appeals, and democratic AI governance",
      "order": 0,
      "modules": [
        {
          "id": "mod-constitutional-architecture",
          "title": "Understanding Constitutional Architecture",
          "description": "Learn how the layered governance model enables both local autonomy and global coherence",
          "order": 0,
          "sections": [
            {
              "id": "sec-layered-model",
              "title": "The Layered Governance Model",
              "order": 0,
              "estimatedMinutes": 45,
              "conceptIds": ["governance-epic", "constitutional-layers"],
              "assessments": [
                {
                  "id": "skill-governance-layers-core",
                  "title": "Governance Layers",
                  "type": "core",
                  "description": "Identify and explain the constitutional layers"
                },
                {
                  "id": "skill-governance-layers-applied",
                  "title": "Governance Layers - Scenarios",
                  "type": "applied",
                  "description": "Apply layer concepts to real-world scenarios"
                }
              ]
            }
          ]
        }
      ]
    }
  ]
}
```

**Step flattening with module metadata:** the seeder flattens chapters → modules → sections → conceptIds into a flat step array stored in Holochain. Each step keeps its ancestry so the UI can show total steps across the whole path AND filter to the current module ("Step 2 of 5") without losing hierarchical context.

```typescript
interface PathStep {
  stepType: string;        // 'learn', 'quiz', 'assessment', etc.
  resourceId: string;      // ContentNode ID
  orderIndex: number;      // Global step order (0-based)
  // Module association metadata for UI filtering
  chapterId?: string;      // Chapter this step belongs to
  moduleId?: string;       // Module this step belongs to (enables "Step 2 of 5" filtering)
  sectionId?: string;      // Section this step belongs to (fine-grained tracking)
}
```

---

## Pedagogical Seed Generation Pipeline

The repeatable process for generating meaningful learning-path hierarchies with proper scope and sequence.

### Terminology

| Term | Definition |
|------|------------|
| **Chapter** | Domain/theme grouping (answers "what world?") |
| **Module** | Capability grouping (answers "what can I do?") |
| **Section** | = **Lesson** (≤1hr, answers "what concept?") |
| **Content** | Individual learning artifact |
| **Skill** | Assessment derived from content |
| **Assessment** | Aggregation of skill questions, scoped to lesson |

### The Scoping Questions Framework

Each hierarchy level must answer a **different question** to provide distinct semantic value:

| Level | Question | Title Pattern | Example |
|-------|----------|---------------|---------|
| **Chapter** | "What domain?" | Domain noun phrase | "AI Governance" |
| **Module** | "What capability?" | Verb + Object | "Navigating Constitutional Architecture" |
| **Section** | "What concept?" | Concept noun phrase | "The Appeals Hierarchy" |
| **Content** | "What artifact?" | Specific title | "The Appellant Journey" |

**AVOID:** repeating the parent title at the child level ("Governance" → "Governance Overview" → "Governance Intro"); generic titles ("Introduction", "Overview", "Basics"); purely structural titles ("Part 1", "Section A", "Module 1").

**Aim for:** each level adds specificity; titles answer different questions; location is reconstructable from the title alone.

### Section = Lesson (Critical Constraint)

Each Section is ONE LESSON with a **maximum duration of ~1 hour** (a human learning-capacity limit). Budget per Section: 2-4 concept items (~15-45 min of consumption) + ~15-20 min reflection/assessment, with natural session break points. Structure: Content 1 = foundation (the "what"), Content 2 = depth (the "how"/"why"), Content 3 = example/application (optional), then assessment(s) to verify understanding before proceeding.

### Assessments as Skills (Khan Academy Model)

Assessments are NOT separate artifacts. They are **smart aggregations of questions generated FROM each piece of content**, scoped to the Lesson (Section) level: each content item generates skill questions, and the Section assessment aggregates across those items. A section may carry multiple assessments approaching the same concept from different angles (e.g. "Adding Two Numbers" core practice plus "Adding Two Numbers - Word Problems" applied).

**Assessment `type` values:**
- **`core`**: Direct application of concepts (knowledge recall, understanding)
- **`applied`**: Scenarios, word problems, real-world application
- **`synthesis`**: Combining multiple concepts, higher-order thinking

### The 6 Phases

**Phase 1 — Audience Analysis.** Create an audience archetype document in `data/lamad/audiences/`:

```yaml
archetype:
  name: "Policy-Developer-Blogger"
  description: "Tech-literate advocate interested in systems change"
entry_knowledge:
  - Basic understanding of distributed systems
  - Familiarity with governance concepts
motivations:
  - Understand enough to advocate effectively
  - Implement or contribute to the protocol
decisions_enabled:
  - "Should I/my organization adopt this approach?"
  - "Where can I contribute technically?"
time_budget: "6-8 hours total, 30-60 min sessions"
resistance_points:
  - Skepticism about "love" in technology
  - Concerns about feasibility at scale
```

**Phase 2 — Content Inventory & Concept Extraction.** Read all source docs for the target domain; extract atomic concepts (single ideas that stand alone); identify relationships (prereq, related, extends, exemplifies); tag concepts by type (theory, practice, example, assessment).

**Phase 3 — Learning Objective Mapping.** Define what learners can DO at each level, using Bloom's progression (Remember → Understand → Apply → Analyze → Evaluate → Create): Chapter = terminal objectives ("Evaluate governance decisions against constitutional principles"); Module = enabling objectives ("Apply the appeals process to novel scenarios"); Section = concept objectives ("Explain how appeals escalate through constitutional layers").

**Phase 4 — Hierarchical Scope Generation.** Apply the Scoping Questions Framework to turn flat content into a properly scoped hierarchy, so each title carries distinct semantic value.

**Phase 5 — Sequence Optimization.** Prerequisites first (enabling concepts before dependent ones); scaffold complexity simple → complex within each module; theory before practice; assess after clusters of related concepts; end with a synthesis module that integrates prior learning.

**Phase 6 — Narrative Threading.** Each level tells a coherent story:
- **Chapter:** "In [Chapter], you'll explore [domain]. By the end, you'll be able to [terminal objective]."
- **Module:** "This module builds your ability to [capability]. You'll learn [key concepts] through [content types]."
- **Section:** "[Concept] is [brief definition]. Understanding this enables you to [application]."

---

## Workflows

### Import Raw Source Files

```
list_docs(epic: "governance")
doc = read_doc(path: "elohim-protocol/governance/epic.md")
create_concept(
  id: "governance-epic",
  title: doc.frontmatter.title || "Governance Epic",
  content: doc.content,
  sourceDoc: doc.path,
  tags: doc.frontmatter.tags || ["governance"]
)
```

### Transform into a Learning Module

```
read_doc(path: "elohim-protocol/governance/epic.md")

create_concept(id: "separation-of-powers", title: "...", content: "...")
create_concept(id: "appeals-process", ...)
create_concept(id: "constitutional-oversight", ...)

create_relationship(source: "appeals-process", target: "separation-of-powers", type: "DEPENDS_ON")

create_path(id: "governance-intro", title: "...")
create_chapter(id: "ch-foundations", pathId: "governance-intro", ...)
create_module(id: "mod-principles", pathId: "governance-intro", chapterId: "ch-foundations", ...)
create_section(id: "sec-overview", conceptIds: ["separation-of-powers", "appeals-process"], ...)

create_quiz(id: "gov-quiz-1", title: "Governance Basics", conceptIds: [...])
```

### Enrich Content with Attention Metadata

```
read_seed(path: "content/separation-of-powers.json")
# estimate reading time: word count / 200 wpm → ~1500 words = 8 minutes
write_seed(path: "content/separation-of-powers.json", {
  ...existingContent,
  estimatedMinutes: 8,
  contentType: "article"
})
```

Never add a static complexity field (see Design Principle 3).

### Seed to Holochain

```bash
cd /projects/elohim/holochain/seeder
npm run seed
```

The seeder reads JSON from `data/lamad/` and loads it to the Holochain DHT.

## Epics Available

| Epic | Description |
|------|-------------|
| `governance` | AI constitutional oversight and appeals |
| `value_scanner` | Care economy and value recognition |
| `public_observer` | Civic participation and oversight |
| `autonomous_entity` | Workplace transformation |
| `social_medium` | Relationship-centered digital communication |
| `economic_coordination` | REA-based value flows |

## Schema Definitions & CLI

Schemas live in `/projects/elohim/mcp-servers/elohim-content/src/schemas/index.ts`: `conceptSchema` (atomic knowledge unit), `pathSchema` (chapters/modules/sections), `assessmentSchema` (quiz and assessment instruments), `questionSchema` (individual questions).

The CLI in `elohim-library/projects/elohim-service/` provides import and exploration commands:

```bash
npx ts-node src/cli/import.ts list-epics
npx ts-node src/cli/import.ts explore --epic governance
npx ts-node src/cli/import.ts list-user-types
```

---

## Seed Data Validation (hc-rna-fixtures)

`hc-rna-fixtures` validates JSON seed data against the DNA schema before seeding, catching errors at build time rather than runtime.

```bash
cd /projects/elohim/holochain/rna/rust

# Build the CLI
RUSTFLAGS="" cargo build --features cli --bin hc-rna-fixtures

# Analyze all seed data (metadata validation)
RUSTFLAGS="" cargo run --features cli --bin hc-rna-fixtures -- \
  -f /projects/elohim/genesis/data/lamad/content --analyze -v
```

### Validation Modes

| Mode | Required Fields | Description |
|------|-----------------|-------------|
| `metadata` (default) | `id`, `title` | Validates only truly required fields. Content format is a hint. |
| `strict` | All DNA fields | Full schema validation against integrity zome |
| `loose` | `id` only | Minimal validation for exploratory work |

```bash
hc-rna-fixtures -f fixtures/ --mode metadata --analyze
hc-rna-fixtures -f fixtures/ --mode strict -i path/to/integrity/src/lib.rs
hc-rna-fixtures -f fixtures/ --mode loose --analyze
```

### Field Categories

| Category | Fields | Validation |
|----------|--------|------------|
| **Required** | `id`, `title` | Must be present, correct type |
| **Client Hints** | `contentFormat`, `contentType` | Any string value accepted |
| **Blob Resolution** | `blob_hash`, `entry_point`, `blob_url` | Indicates cache layer content |
| **Auto-Defaulted** | `reach`, `trust_score`, `schema_version` | Seeder/zome provides defaults |

### Reading the Output

`--analyze` prints a `FIXTURE ANALYSIS (Metadata-Focused)` report with counts for **Total files**, **Valid metadata** (has required `id` + `title`), **With blob refs** (references external blobs needing cache resolution), **Missing required** (must be 0 before seeding), and a **Content Formats (client hints)** distribution of `contentFormat` values (hints, not validated).

### Pre-Seed Validation Workflow

Always validate before running the seeder:

```bash
# 1. Validate content files
cd /projects/elohim/holochain/rna/rust
RUSTFLAGS="" cargo run --features cli --bin hc-rna-fixtures -- \
  -f /projects/elohim/genesis/data/lamad/content --analyze -v

# 2. Check the report for "Missing required: X ❌"

# 3. If all valid, proceed with seeding
cd /projects/elohim/genesis
npm run seed
```

### Common Validation Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| `Missing required field 'id'` | JSON lacks `id` field | Add unique identifier |
| `Missing required field 'title'` | JSON lacks `title` field | Add human-readable title |
| `Invalid JSON` | Malformed JSON syntax | Check for trailing commas, unclosed braces |
| `Fixture must be a JSON object` | Array at root level | Wrap in object or split into files |

### Generating Rust Fixtures

The tool can also emit a Rust module with embedded fixtures (`include_str!` JSON constants, lazy-parsed fixture collections, an idempotent `seed_fixtures()` function) for compile-time validation:

```bash
hc-rna-fixtures -f fixtures/ -o src/fixtures.rs --module-name seed_data
```

---

## Troubleshooting

**No content in `data/lamad/`** — use the MCP tools to either direct-import (read source files, create concepts with full content) or transform (extract atomic concepts from source material).

**Seeder can't find content** — ensure JSON exists in `data/lamad/content/*.json` (concepts), `data/lamad/paths/*.json` (learning paths), `data/lamad/assessments/*.json` (assessments).

**MCP server not responding** — build and restart it:

```bash
cd /projects/elohim/mcp-servers/elohim-content
npm install
npm run build
```
