# Holochain Content Import Pipeline

This skill provides tools and knowledge for importing content to the Holochain DHT, bypassing Kuzu WASM string size limitations.

## Architecture

```
                    ┌─→ Kuzu DB (lamad.kuzu)         [existing: import command]
Source Files ───────┼
(.md, .feature)     └─→ Holochain Conductor         [NEW: holo-import.ts]
                            │
                            └── CLI via DevContainer proxy (wss://holochain-dev.elohim.host)
```

**Shared:** Parsers, transformers, ContentNode model
**Different:** Storage layer only

## CLI Commands

All commands run from `elohim-library/projects/elohim-service/`:

### holo:import - Import Content to Holochain

```bash
# Full import
npx ts-node src/cli/holo-import.ts holo:import \
  --source ./docs/content \
  --admin-url wss://holochain-dev.elohim.host \
  --app-id lamad-spike \
  --batch-size 50

# Dry run (parse and transform only)
npx ts-node src/cli/holo-import.ts holo:import --dry-run

# Force full reimport
npx ts-node src/cli/holo-import.ts holo:import --full

# Skip relationship extraction
npx ts-node src/cli/holo-import.ts holo:import --skip-relationships
```

**Options:**
- `-s, --source <dir>` - Source content directory (default: `./docs/content`)
- `--admin-url <url>` - Holochain admin WebSocket URL (default: `wss://holochain-dev.elohim.host`)
- `--app-id <id>` - Holochain app ID (default: `lamad-spike`)
- `--happ-path <path>` - Path to .happ file for installation
- `--batch-size <n>` - Entries per bulk call (default: `50`)
- `-f, --full` - Force full reimport
- `-v, --verbose` - Verbose output
- `--dry-run` - Parse and transform but do not write to Holochain
- `--skip-relationships` - Skip relationship extraction

### holo:stats - Show Statistics

```bash
npx ts-node src/cli/holo-import.ts holo:stats
```

Output shows total content count and breakdown by content type.

### holo:verify - Verify Content Exists

```bash
# By comma-separated IDs
npx ts-node src/cli/holo-import.ts holo:verify --ids "manifesto,governance-epic"

# By file (one ID per line)
npx ts-node src/cli/holo-import.ts holo:verify --file ./content-ids.txt
```

### holo:list - List Content by Type

```bash
npx ts-node src/cli/holo-import.ts holo:list --type scenario --limit 20
```

### holo:get - Get Single Content

```bash
# Human-readable
npx ts-node src/cli/holo-import.ts holo:get manifesto

# JSON output
npx ts-node src/cli/holo-import.ts holo:get manifesto --json
```

### holo:test - Test Connection

```bash
npx ts-node src/cli/holo-import.ts holo:test
```

## DNA Schema

### Entry Types

**Content** (extended from original 5-field spike):
```rust
pub struct Content {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    pub content: String,               // Full markdown body
    pub content_format: String,
    pub tags: Vec<String>,
    pub source_path: Option<String>,
    pub related_node_ids: Vec<String>,
    pub author_id: Option<String>,
    pub reach: String,
    pub trust_score: f64,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
```

**LearningPath:**
```rust
pub struct LearningPath {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub purpose: Option<String>,
    pub created_by: String,
    pub difficulty: String,
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

**PathStep:**
```rust
pub struct PathStep {
    pub id: String,
    pub path_id: String,
    pub order_index: u32,
    pub step_type: String,
    pub resource_id: String,
    pub step_title: Option<String>,
    pub step_narrative: Option<String>,
    pub is_optional: bool,
}
```

**ContentRelationship:**
```rust
pub struct ContentRelationship {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub metadata_json: Option<String>,
}
```

### Link Types

```rust
pub enum LinkTypes {
    AuthorToContent,       // AgentPubKey → Content
    IdToContent,           // Hash(id) → Content (lookup by string ID)
    TypeToContent,         // Hash(content_type) → Content
    TagToContent,          // Hash(tag) → Content
    ImportBatchToContent,  // Hash(import_id) → Content (bulk tracking)
    PathToStep,            // LearningPath → PathStep
    StepToContent,         // PathStep → Content
    IdToPath,              // Hash(id) → LearningPath
}
```

### Zome Functions

| Function | Input | Output | Purpose |
|----------|-------|--------|---------|
| `create_content` | CreateContentInput | ContentOutput | Create single entry + links |
| `bulk_create_content` | BulkCreateInput | BulkCreateOutput | Batch create for imports |
| `get_content_by_id` | QueryByIdInput | Option<ContentOutput> | Lookup by string ID |
| `get_content_by_type` | QueryByTypeInput | Vec<ContentOutput> | Filter by content_type |
| `get_content_stats` | () | ContentStats | Aggregate counts by type |
| `create_path` | CreatePathInput | ActionHash | Create learning path |
| `add_path_step` | AddPathStepInput | ActionHash | Add step to path |
| `get_path_with_steps` | String | PathWithSteps | Get path + all steps |

## File Locations

### CLI & Node.js Services
- `elohim-library/projects/elohim-service/src/cli/holo-import.ts` - CLI entry point
- `elohim-library/projects/elohim-service/src/services/holochain-client.service.ts` - WebSocket client
- `elohim-library/projects/elohim-service/src/services/holochain-import.service.ts` - Import adapter
- `elohim-library/projects/elohim-service/src/models/holochain.model.ts` - TypeScript types

### Angular Services
- `elohim-app/src/app/elohim/services/holochain-client.service.ts` - Browser WebSocket client
- `elohim-app/src/app/elohim/services/holochain-content.service.ts` - Content retrieval service
- `elohim-app/src/app/elohim/models/holochain-connection.model.ts` - Shared types

### DNA Source
- `holochain/dna/lamad-spike/zomes/content_store_integrity/src/lib.rs` - Entry types + validation
- `holochain/dna/lamad-spike/zomes/content_store/src/lib.rs` - Coordinator functions

## Validation Rules

### Content Types
`source`, `epic`, `feature`, `scenario`, `concept`, `role`, `video`, `organization`, `book-chapter`, `tool`, `path`, `assessment`, `reference`, `example`

### Content Formats
`markdown`, `gherkin`, `html`, `plaintext`, `video-embed`, `external-link`, `quiz-json`, `assessment-json`

### Reach Levels
`private`, `invited`, `local`, `community`, `federated`, `commons`

### Relationship Types
`CONTAINS`, `BELONGS_TO`, `DESCRIBES`, `IMPLEMENTS`, `VALIDATES`, `RELATES_TO`, `REFERENCES`, `DEPENDS_ON`, `REQUIRES`, `FOLLOWS`, `DERIVED_FROM`, `SOURCE_OF`

## Troubleshooting

### Connection Failed

1. **Check proxy is running:**
   ```bash
   curl -I https://holochain-dev.elohim.host
   ```

2. **Verify conductor is accessible:**
   ```bash
   npx ts-node src/cli/holo-import.ts holo:test
   ```

3. **Check app is installed:**
   - The CLI will attempt to install the app if not found
   - Requires `--happ-path` option pointing to the `.happ` file

### Import Errors

1. **Batch failures:** Check error messages - usually validation errors
2. **String too long:** This shouldn't happen with Holochain (unlike Kuzu WASM)
3. **Connection dropped:** Reduce batch size with `--batch-size 25`

### Browser Connection (Phase 2)

The Angular `HolochainContentService` is prepared but currently unavailable because:
- Admin interface returns localhost ports for app interface
- App interface proxy (Phase 2) needed for browser zome calls

**Current workaround:** Use CLI for imports, JSON files for browser display.

## Comparison: Kuzu vs Holochain

| Feature | Kuzu (WASM) | Holochain |
|---------|-------------|-----------|
| String size limit | ~1KB | Unlimited (MessagePack) |
| Storage | Browser IndexedDB | DHT (distributed) |
| Query | Cypher SQL | Zome functions |
| Real-time sync | No | Yes (gossip protocol) |
| Offline support | Yes | Yes (local source chain) |
| Trust model | None | Entry validation + attestations |

## Example Usage

### Full Import Workflow

```bash
cd elohim-library/projects/elohim-service

# 1. Test connection
npx ts-node src/cli/holo-import.ts holo:test

# 2. Dry run to check parsing
npx ts-node src/cli/holo-import.ts holo:import --dry-run --source ../../data/content

# 3. Full import
npx ts-node src/cli/holo-import.ts holo:import --source ../../data/content

# 4. Verify key content exists
npx ts-node src/cli/holo-import.ts holo:verify --ids "manifesto,governance-epic,policy-maker-readme"

# 5. Check statistics
npx ts-node src/cli/holo-import.ts holo:stats
```

### Programmatic Usage (Node.js)

```typescript
import { HolochainImportService, createLamadImportService } from './services/holochain-import.service';

// Using factory function
const service = createLamadImportService('wss://holochain-dev.elohim.host');

// Or with custom config
const service = new HolochainImportService({
  adminUrl: 'wss://holochain-dev.elohim.host',
  appId: 'lamad-spike',
  batchSize: 50,
});

// Import nodes
const result = await service.importNodes(contentNodes);
console.log(`Imported ${result.createdNodes}/${result.totalNodes} nodes`);

// Query stats
const stats = await service.getStats();
console.log(`Total: ${stats.total_count}`);
```
