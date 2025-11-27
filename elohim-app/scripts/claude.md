# Scripts Directory

Python utility scripts for the Elohim Protocol project. Run all scripts from the `elohim-app` root directory.

## Primary Script

### `generate_lamad_data.py`
Generates mock data for the Lamad learning system by parsing `/src/assets/docs/` content.

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

**Key Features:**
- Parses YAML frontmatter from markdown files
- Extracts features and scenarios from Gherkin files
- Generates deterministic IDs from file paths
- Creates the "Who Are You?" quiz assessment
- Builds the main "Elohim Protocol" learning path

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
