# Scripts Directory

Python utility scripts for the Elohim Protocol project. Run all scripts from the `elohim-app` root directory.

## Primary Scripts

### `generate_lamad_data.py`
Generates core mock data for the Lamad learning system by parsing `/src/assets/docs/` content.

```bash
python scripts/generate_lamad_data.py
```

**Input Sources:**
- `/src/assets/docs/manifest.json` - Content registry
- `/src/assets/docs/**/*.md` - Markdown content with YAML frontmatter
- `/src/assets/docs/**/*.feature` - Gherkin scenario files

**Output:**
- `/src/assets/lamad-data/paths/index.json` - PathIndex catalog
- `/src/assets/lamad-data/paths/elohim-protocol.json` - Main LearningPath
- `/src/assets/lamad-data/content/index.json` - ContentIndex catalog
- `/src/assets/lamad-data/content/*.json` - Individual ContentNode files
- `/src/assets/lamad-data/graph/` - Graph visualization data

**Key Features:**
- Parses YAML frontmatter from markdown files
- Extracts features and scenarios from Gherkin files
- Generates deterministic IDs from file paths
- Creates the "Who Are You?" quiz assessment
- Builds the main "Elohim Protocol" learning path
- Generates graph relationships and overview data

---

### `generate_lamad_mock_data.py`
Generates comprehensive mock data for UI development including paths, assessments, quizzes, and governance data.

```bash
python scripts/generate_lamad_mock_data.py
```

**Output:**

| Directory | Content |
|-----------|---------|
| `paths/` | 6 epic-specific learning paths |
| `assessments/` | 5 psychometric instruments |
| `governance/` | Challenges, proposals, precedents, discussions |
| `knowledge-maps/` | Self-knowledge and person map templates |
| `content/` | Quizzes and supporting content nodes |

**Generated Learning Paths:**
1. `governance-deep-dive` - Constitutional AI oversight (5 steps)
2. `value-scanner-journey` - Care work economics (3 steps)
3. `public-observer-path` - Civic participation (3 steps)
4. `autonomous-entity-path` - Workplace ownership (3 steps)
5. `social-medium-path` - Digital dignity (2 steps)
6. `know-thyself-path` - Self-discovery with gated assessments (4 steps)

**Generated Assessments:**
1. `assessment-values-hierarchy` - Likert-7 + ranking (inspired by Schwartz Values)
2. `assessment-attachment-style` - Likert-7 quadrant model (inspired by ECR-R)
3. `assessment-strengths-finder` - Likert-5 strength ranking (inspired by VIA)
4. `assessment-constitutional-reasoning` - Competency quiz with short-answer
5. `assessment-personal-values` - Sliders + multi-select reflection

**Generated Quizzes:**
- `quiz-governance-foundations` - Test governance understanding
- `quiz-care-economics` - Care work economics concepts
- `quiz-civic-engagement` - Public participation readiness
- `quiz-distributed-ownership` - Worker ownership models
- `quiz-digital-dignity` - Dignity-preserving design

**Generated Governance Data:**
- `challenges.json` - 3 example challenges (acknowledged, under-review, resolved)
- `proposals.json` - 3 proposals (sense-check, consent, consensus types)
- `precedents.json` - 3 constitutional precedents
- `discussions.json` - Deliberation thread examples
- `state-*.json` - Governance states for content entities

**Generated Knowledge Maps:**
- `map-self-demo-learner` - Self-knowledge map with Imago Dei dimensions
- `map-person-template` - Gottman-inspired person map template

**Key Features:**
- Idempotent - updates indexes without duplicating existing data
- Generates diverse question types (Likert, ranking, slider, multiple-select)
- Creates interconnected data (paths reference quizzes and assessments)
- Supports attestation granting on completion
- Includes gated content examples (prerequisite attestations)

---

## Legacy Scripts

These scripts were used for earlier content organization and may still be useful:

| Script | Purpose |
|--------|---------|
| `generate_lamad_manifest.py` | Generates manifest.json from docs directory |
| `generate_user_templates.py` | Creates README/TODO templates for user types |
| `update_node_types.py` | Updates node type definitions |
| `reorganize_content_types.py` | Reorganizes content by type |
| `verify_all_gems.py` | Verifies gem/content integrity |
| `split_keen_collections.py` | Splits Keen collections into separate files |

## Data Models Reference

When modifying scripts, reference these TypeScript interfaces:
- `src/app/lamad/models/content-node.model.ts` - ContentNode, ContentType, ContentFormat
- `src/app/lamad/models/learning-path.model.ts` - LearningPath, PathStep, PathIndex

## Content Type Mapping

| Manifest Type | Lamad ContentType |
|--------------|-------------------|
| epic | epic |
| feature | feature |
| scenario | scenario |
| user_type | concept |
| book | book-chapter |
| organization | organization |
| video, audio | video |
| document, article | concept |
