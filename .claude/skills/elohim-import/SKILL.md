---
name: elohim-import
description: Knowledge and workflows for importing Elohim Protocol content into the lamad learning platform
---

# Elohim Import Skill

This skill provides domain expertise for transforming raw Elohim Protocol content into structured learning content for the lamad system. It supports both direct import of raw source files and creative transformation into structured concepts and paths.

## Design Principles

1. **Rust DNA is source of truth** - TypeScript/JSON schemas align with Holochain Rust structs
2. **Maximal model complexity** - The graph supports rich relationships; AI derives meaning for learners
3. **No static complexity** - Complexity is relative to the learner, not the content
4. **This skill IS the AI** - During prototyping, Claude (via this skill) fulfills the AI derivation role at import time. In production, distributed AI agents will compute paths dynamically per-learner.

## Content Pipeline Architecture

```
Holochain DNA (Rust)    [Source of truth - entry types, relationships]
      ↓
MCP Schemas (TypeScript) [Aligned with DNA structs]
      ↓
docs/content/           [Raw markdown, Gherkin - human authored]
      ↓
   Claude + MCP tools   [Non-deterministic, creative transformation]
      ↓
data/lamad/             [Structured JSON - schema-aligned seed data]
      ↓
   holochain/seeder     [Deterministic script - loads JSON to DHT]
      ↓
Holochain DHT           [Production data]
```

---

## Pedagogical Seed Generation Pipeline

This section documents the repeatable process for generating meaningful learning path hierarchies. The pipeline transforms raw content into pedagogically-sound curricula with proper scope and sequence.

### Core Terminology

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

**Anti-patterns to AVOID:**
- ❌ Repeating parent title at child level ("Governance" → "Governance Overview" → "Governance Intro")
- ❌ Generic titles ("Introduction", "Overview", "Basics")
- ❌ Purely structural titles ("Part 1", "Section A", "Module 1")

**Good patterns:**
- ✅ Each level adds specificity
- ✅ Titles answer different questions
- ✅ Can reconstruct location from title alone

### Section = Lesson (Critical Constraint)

Each Section represents ONE LESSON with a **maximum duration of ~1 hour**. This constraint is based on human learning capacity limits.

**Content Budget per Section:**
- 2-4 concept items (articles, videos, etc.)
- ~15-45 minutes of content consumption
- ~15-20 minutes for reflection/assessment
- Natural break points for session boundaries

**Section Structure:**
```
Section: [Concept Title]
├── Content 1: Foundation (the "what")
├── Content 2: Depth (the "how" or "why")
├── Content 3: Example/Application (optional)
└── Assessment(s): Verify understanding before proceeding
```

### Assessments as Skills (Khan Academy Model)

Assessments are NOT separate artifacts. They are **smart aggregations of questions generated FROM each piece of content**, scoped to the Lesson (Section) level.

**Content → Skills → Assessments:**
```
Content Item: "The Appeals Process" (article)
   └── Generates skill questions about appeals

Content Item: "Appellant Scenario" (example)
   └── Generates application questions about the same concept

Section Assessment = Aggregation of questions from both content items
```

**Multiple Assessments per Lesson:**
A section can have multiple assessments approaching the same concept from different angles:

```
Section: "Adding Two Numbers" (Lesson)
├── Content: Explanation of addition
├── Content: Visual representations
├── Assessment 1: "Adding Two Numbers" (core - direct practice)
└── Assessment 2: "Adding Two Numbers - Word Problems" (applied)
```

**Assessment Types:**
- **`core`**: Direct application of concepts (knowledge recall, understanding)
- **`applied`**: Scenarios, word problems, real-world application
- **`synthesis`**: Combining multiple concepts, higher-order thinking

### The 6-Phase Pedagogical Pipeline

#### Phase 1: Audience Analysis

**Purpose:** Understand the learner before designing curriculum.

Create an audience archetype document (stored in `data/lamad/audiences/`):

```yaml
archetype:
  name: "Policy-Developer-Blogger"
  description: "Tech-literate advocate interested in systems change"

entry_knowledge:
  - Basic understanding of distributed systems
  - Familiarity with governance concepts
  - Some exposure to blockchain/crypto discourse

motivations:
  - Understand enough to advocate effectively
  - Implement or contribute to the protocol
  - Write/communicate about these ideas

decisions_enabled:
  - "Should I/my organization adopt this approach?"
  - "How do I explain this to stakeholders?"
  - "Where can I contribute technically?"

time_budget: "6-8 hours total, 30-60 min sessions"

resistance_points:
  - Skepticism about "love" in technology
  - Concerns about feasibility at scale
  - Questions about economic sustainability
```

#### Phase 2: Content Inventory & Concept Extraction

**Purpose:** Map all atomic concepts and their relationships.

1. Read all source docs for the target domain
2. Extract atomic concepts (single ideas that can stand alone)
3. Identify relationships (prereq, related, extends, exemplifies)
4. Tag concepts by type (theory, practice, example, assessment)

#### Phase 3: Learning Objective Mapping

**Purpose:** Define what learners should be able to DO at each level.

Using Bloom's Taxonomy progression (Remember → Understand → Apply → Analyze → Evaluate → Create):

- **Chapter (Terminal Objectives):** "Evaluate governance decisions against constitutional principles"
- **Module (Enabling Objectives):** "Apply the appeals process to novel scenarios"
- **Section (Concept Objectives):** "Explain how appeals escalate through constitutional layers"

#### Phase 4: Hierarchical Scope Generation

**Purpose:** Generate titles that provide DISTINCT semantic value at each level.

Apply the Scoping Questions Framework to transform flat content into properly scoped hierarchy.

**Example Transformation:**

BEFORE (flat):
```
Chapter: AI Governance
  Step: Governance Overview
  Step: Quiz
  Step: Policy Maker Perspective
```

AFTER (properly scoped):
```
Chapter: AI Governance (domain)
  Module: Understanding Constitutional Architecture (capability)
    Section: The Layered Governance Model (concept)
      Content: Governance Epic Overview
      Content: Constitutional Layers Diagram
  Module: Navigating the System as a Stakeholder (capability)
    Section: The Policy Maker's Interface (concept)
      Content: Policy Maker README
    Section: The Appellant's Journey (concept)
      Content: Appellant README
```

#### Phase 5: Sequence Optimization

**Purpose:** Order content for optimal learning.

Sequencing Principles:
1. **Prerequisites first:** Concepts that enable understanding come before those that require it
2. **Scaffold complexity:** Simple → Complex within each module
3. **Theory before practice:** Concepts before applications
4. **Assess after clusters:** Check understanding after related concepts
5. **End with synthesis:** Final module should integrate previous learning

#### Phase 6: Narrative Threading

**Purpose:** Each level tells a coherent story.

**Narrative Templates:**

- **Chapter description:** "In [Chapter], you'll explore [domain]. By the end, you'll be able to [terminal objective]."
- **Module description:** "This module builds your ability to [capability]. You'll learn [key concepts] through [content types]."
- **Section description:** "[Concept] is [brief definition]. Understanding this enables you to [application]."

### Path JSON Schema (4-Level with Assessments)

```json
{
  "id": "elohim-protocol",
  "title": "Elohim Protocol: Living Documentation",
  "chapters": [
    {
      "id": "chapter-2-governance",
      "title": "AI Governance",
      "description": "Constitutional oversight, appeals, and democratic AI governance",
      "modules": [
        {
          "id": "mod-constitutional-architecture",
          "title": "Understanding Constitutional Architecture",
          "description": "Learn how the layered governance model enables both local autonomy and global coherence",
          "sections": [
            {
              "id": "sec-layered-model",
              "title": "The Layered Governance Model",
              "estimatedMinutes": 45,
              "conceptIds": [
                "governance-epic",
                "constitutional-layers"
              ],
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

### Directory Structure (Updated)

| Directory | Purpose |
|-----------|---------|
| `/docs/content/` | Raw source content (markdown, gherkin) |
| `/data/lamad/` | Structured JSON seed data |
| `/data/lamad/content/` | Concept JSON files |
| `/data/lamad/paths/` | Learning path JSON files |
| `/data/lamad/assessments/` | Assessment JSON files |
| `/data/lamad/audiences/` | Audience archetype YAML files |

---

## When to Use This Skill

Reference this skill when the user:
- Wants to import raw source files directly as content
- Wants to create learning paths or modules from docs
- Needs to transform markdown into structured concepts
- Is building the content graph (concepts + relationships)
- Wants to create quizzes or assessments
- Needs to seed content to Holochain

## Directory Structure

| Directory | Purpose |
|-----------|---------|
| `/docs/content/` | Raw source content (markdown, gherkin) |
| `/data/lamad/` | Structured JSON seed data |
| `/data/lamad/content/` | Concept JSON files |
| `/data/lamad/paths/` | Learning path JSON files |
| `/data/lamad/assessments/` | Assessment JSON files |

## MCP Server: elohim-content

The `elohim-content` MCP server provides tools for reading, transforming, and writing content.

### Source Reading Tools

| Tool | Purpose | Example |
|------|---------|---------|
| `read_doc` | Read markdown/gherkin from docs/content/ | `{"path": "elohim-protocol/governance/epic.md"}` |
| `list_docs` | List documents by epic or pattern | `{"epic": "governance"}` |
| `search_docs` | Search for concepts/keywords | `{"query": "constitutional oversight"}` |

### Seed Data CRUD

| Tool | Purpose |
|------|---------|
| `list_seeds` | List existing seed files (concepts, paths, assessments) |
| `read_seed` | Read a specific seed JSON file |
| `write_seed` | Write/update structured JSON to data/lamad/ |
| `delete_seed` | Remove a seed file |
| `validate_seed` | Validate against Holochain schemas |

### Content Graph Tools

| Tool | Purpose |
|------|---------|
| `create_concept` | Create atomic concept from docs or raw source |
| `create_relationship` | Link concepts (DEPENDS_ON, RELATES_TO, CONTAINS, etc.) |
| `query_graph` | Find concepts by tags, relationships |
| `get_related` | Get related concepts for a node |
| `update_concept` | Modify concept content/metadata |
| `delete_concept` | Remove concept and relationships |

### Path Authoring Tools

Paths are **views/projections** over the content graph:

| Tool | Purpose |
|------|---------|
| `create_path` | Create ordered traversal through graph |
| `create_chapter` | Group concepts into chapter |
| `create_module` | Group into module |
| `create_section` | Group into section |
| `add_to_path` | Add concept at position |
| `remove_from_path` | Remove concept (keeps in graph) |
| `generate_path` | Auto-generate from graph region |

### Assessment Tools

| Tool | Purpose |
|------|---------|
| `create_quiz` | Generate quiz from concepts |
| `create_assessment` | Build assessment instrument |
| `update_assessment` | Modify questions/scoring |

## Import Modes

### 1. Direct Source Import

Import a raw source file directly as content, preserving the original markdown:

```
1. Read the source file
   → read_doc(path: "elohim-protocol/manifesto.md")

2. Create concept with full content
   → create_concept(
       id: "manifesto",
       title: "Elohim Protocol Manifesto",
       content: <full markdown from file>,
       sourceDoc: "elohim-protocol/manifesto.md",
       tags: ["elohim", "manifesto", "vision"]
     )
```

Use this mode when:
- The source file should be preserved as-is
- Content is already atomic (single document = single concept)
- You want 1:1 mapping from docs to content

### 2. Creative Transformation

Transform source material into multiple atomic concepts with relationships:

```
1. Read the source file(s)
   → read_doc(path: "elohim-protocol/governance/epic.md")

2. Extract multiple atomic concepts
   → create_concept(id: "separation-of-powers", ...)
   → create_concept(id: "appeals-process", ...)
   → create_concept(id: "constitutional-oversight", ...)

3. Create relationships
   → create_relationship(source: "appeals-process", target: "separation-of-powers", type: "DEPENDS_ON")
```

Use this mode when:
- Source content covers multiple distinct concepts
- You want to build a knowledge graph
- Content needs to be chunked for learning purposes

## Content Model

### Content Graph vs Paths

```
                ┌─────────────────────────────────────┐
                │         CONTENT GRAPH               │
                │  (multi-dimensional knowledge graph) │
                │                                     │
                │   Concept ←→ Concept ←→ Concept     │
                │      ↑          ↑          ↑        │
                │ DEPENDS_ON  RELATES_TO  CONTAINS    │
                │      ↓          ↓          ↓        │
                │   Concept ←→ Concept ←→ Concept     │
                └─────────────────────────────────────┘
                                ↓
                ┌─────────────────────────────────────┐
                │      PATH (one view/projection)      │
                │                                     │
                │   Chapter → Module → Section        │
                │      (ordered traversal of graph)   │
                └─────────────────────────────────────┘
```

- **Content Graph**: Underlying knowledge graph with all relationships
- **Path**: One ordered traversal/projection over the graph
- **Exploration**: Users can leave path to explore related graph nodes

### Concept Schema

Aligned with Holochain DNA `Content` struct:

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

#### Content Fields

| Field | Type | Description |
|-------|------|-------------|
| `estimatedMinutes` | number | Reading/viewing time in minutes |
| `contentType` | `"article"` \| `"video"` \| `"interactive"` \| `"assessment"` | Media format |
| `metadata` | object | Extensible JSON for additional data |

**Note:** Complexity is NOT stored on content. It's relative to the learner - a beginner finds content "deep" that an expert finds "light".

**This skill IS the AI:** During prototyping, Claude (via this skill) makes complexity and sequencing judgments when creating relationships and paths. When building a learning path, Claude assesses prerequisite depth, concept ordering, and appropriate chunking based on the target learner profile. In production, distributed AI agents will make these assessments dynamically per-learner.

### Path Schema

Paths use a **4-level hierarchy** aligned between MCP schemas and Angular models:

```
Path
  └── Chapter (PathChapter)
        └── Module (PathModule)
              └── Section (PathSection)
                    └── conceptIds: string[]
```

**Example:**
```json
{
  "id": "governance-intro",
  "title": "Introduction to Constitutional Governance",
  "description": "Learn the foundations of AI constitutional oversight",
  "difficulty": "beginner",
  "chapters": [
    {
      "id": "ch-foundations",
      "title": "Constitutional Foundations",
      "order": 0,
      "modules": [
        {
          "id": "mod-principles",
          "title": "Core Principles",
          "order": 0,
          "sections": [
            {
              "id": "sec-separation",
              "title": "Separation of Powers",
              "order": 0,
              "conceptIds": ["separation-of-powers", "checks-balances"]
            }
          ]
        }
      ]
    }
  ]
}
```

**Angular Model Types:**
- `PathChapter` - contains `modules: PathModule[]` (required)
- `PathModule` - contains `sections: PathSection[]` (required)
- `PathSection` - contains `conceptIds: string[]` (required, links to ContentNode IDs)

### Relationship Types (DNA-aligned)

Aligned with Holochain DNA `Relationship.relationship_type`:

| Type | Description |
|------|-------------|
| `RELATES_TO` | General association between concepts |
| `CONTAINS` | Parent-child hierarchical relationship |
| `DEPENDS_ON` | Prerequisite dependency (must understand first) |
| `IMPLEMENTS` | Implementation of a concept |
| `REFERENCES` | Citation or reference to another concept |
| `DERIVED_FROM` | This content was derived from source content |

#### The `DERIVED_FROM` Relationship

Use `DERIVED_FROM` to link derived/transformed content back to its source:

```
Source Document (raw)          Derived Content (atomic lesson)
┌─────────────────────┐       ┌─────────────────────────────┐
│ governance-epic.md  │ ←──── │ separation-of-powers.json   │
│ (full epic document)│       │ relationships: [            │
└─────────────────────┘       │   {target: "governance-epic",│
                              │    type: "DERIVED_FROM"}    │
                              │ ]                           │
                              └─────────────────────────────┘
```

This enables:
- **Provenance tracking**: Know where content came from
- **Discovery**: "View source document" links in UI
- **Hierarchy derivation**: AI can traverse `DERIVED_FROM` to build scope/sequence

## Example Workflows

### Enrich Content with Attention Metadata

When importing or updating content, add attention metadata for digestible learning sessions:

```
1. Read existing concept
   → read_seed(path: "content/separation-of-powers.json")

2. Estimate reading time (word count / 200 wpm)
   → ~1500 words = 8 minutes

3. Update with metadata
   → write_seed(path: "content/separation-of-powers.json", {
       ...existingContent,
       estimatedMinutes: 8,
       contentType: "article"
     })
```

**Note:** Do NOT add a static complexity field. Complexity is relative to the learner. Instead, **this skill (Claude) embeds complexity judgments into the graph structure** - through relationship types (`DEPENDS_ON` for prerequisites), ordering in paths, and chunking decisions made during import.

### Import Raw Source Files

Import a collection of markdown files directly:

```
1. List available documents
   → list_docs(epic: "governance")

2. For each document, read and create concept
   → doc = read_doc(path: "elohim-protocol/governance/epic.md")
   → create_concept(
       id: "governance-epic",
       title: doc.frontmatter.title || "Governance Epic",
       content: doc.content,
       sourceDoc: doc.path,
       tags: doc.frontmatter.tags || ["governance"]
     )
```

### Transform into Learning Module

```
1. Read the governance epic
   → read_doc(path: "elohim-protocol/governance/epic.md")

2. Extract atomic concepts
   → create_concept(id: "separation-of-powers", title: "...", content: "...")
   → create_concept(id: "appeals-process", ...)
   → create_concept(id: "constitutional-oversight", ...)

3. Create relationships
   → create_relationship(source: "appeals-process", target: "separation-of-powers", type: "DEPENDS_ON")

4. Build learning path
   → create_path(id: "governance-intro", title: "...")
   → create_chapter(id: "ch-foundations", pathId: "governance-intro", ...)
   → create_module(id: "mod-principles", pathId: "governance-intro", chapterId: "ch-foundations", ...)
   → create_section(id: "sec-overview", conceptIds: ["separation-of-powers", "appeals-process"], ...)

5. Create quiz
   → create_quiz(id: "gov-quiz-1", title: "Governance Basics", conceptIds: [...])
```

### Seed to Holochain

After creating content in data/lamad/, run the seeder:

```bash
cd /projects/elohim/holochain/seeder
npm run seed
```

The seeder reads JSON from data/lamad/ and loads it to Holochain DHT.

## Epics Available

Source content organized by epic:

| Epic | Description |
|------|-------------|
| `governance` | AI constitutional oversight and appeals |
| `value_scanner` | Care economy and value recognition |
| `public_observer` | Civic participation and oversight |
| `autonomous_entity` | Workplace transformation |
| `social_medium` | Relationship-centered digital communication |
| `economic_coordination` | REA-based value flows |

## Schema Definitions

Schemas are defined in `/projects/elohim/mcp-servers/elohim-content/src/schemas/index.ts`:

- `conceptSchema` - Atomic knowledge unit
- `pathSchema` - Learning path with chapters/modules/sections
- `assessmentSchema` - Quiz and assessment instruments
- `questionSchema` - Individual assessment questions

## Legacy CLI Commands

The CLI in `elohim-library/projects/elohim-service/` provides exploration commands:

```bash
# List epics
npx ts-node src/cli/import.ts list-epics

# Explore content
npx ts-node src/cli/import.ts explore --epic governance

# List user types
npx ts-node src/cli/import.ts list-user-types
```

## Troubleshooting

### No content in data/lamad/

Use the MCP tools to:
1. **Direct import**: Read source files and create concepts with full content
2. **Transform**: Extract atomic concepts from source material

### Seeder can't find content

Ensure JSON files exist in:
- `data/lamad/content/*.json` - Concepts
- `data/lamad/paths/*.json` - Learning paths
- `data/lamad/assessments/*.json` - Assessments

### MCP server not responding

Build and restart the MCP server:
```bash
cd /projects/elohim/mcp-servers/elohim-content
npm install
npm run build
```
