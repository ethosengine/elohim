# Content Import Command

Import content from source files into lamad ContentNodes.

## Usage

```
/import-content [action] [options]
```

## Actions

- `import` - Run incremental import (default)
- `full` - Run full reimport
- `preview` - Preview what would change
- `stats` - Show import statistics
- `validate` - Validate manifest integrity

## What This Does

This command triggers the elohim-service content import pipeline:

1. **Scans** `/data/content/` for markdown (.md) and Gherkin (.feature) files
2. **Parses** each file extracting:
   - Path metadata (domain, epic, user type)
   - YAML frontmatter
   - Content sections
   - Gherkin scenarios
3. **Transforms** content into ContentNodes:
   - Source nodes (raw content for provenance)
   - Epic nodes
   - Role/Archetype nodes
   - Scenario nodes
   - Resource nodes
4. **Extracts relationships** between nodes:
   - Explicit references
   - Path-based (same epic/user type)
   - Tag-based similarity
   - Content references
5. **Writes output** to `/output/lamad/`:
   - `nodes.json` - All ContentNodes
   - `relationships.json` - Node relationships
   - `content-manifest.json` - For incremental updates
   - `import-summary.json` - Statistics

## Example Prompts

- "Import all content to lamad format"
- "Run a full content reimport"
- "Preview what files have changed since last import"
- "Show me the import statistics"
- "Validate the import manifest"

## Implementation

Located at: `elohim-library/projects/elohim-service/`

To run manually:
```bash
cd elohim-library/projects/elohim-service
npx ts-node src/cli/import.ts import --source ../../data/content --output ../../output/lamad
```
