# Holochain RNA - Migration Toolkit

**RNA transcribes data between DNA versions during Holochain migrations.**

Every integrity zome change creates a new DNA hash - a completely separate network. RNA solves this by providing a universal migration pattern: export from the old DNA, transform the schema, import into the new DNA.

## The Biological Metaphor

In molecular biology:

| Biology | Holochain Analog |
|---------|------------------|
| **DNA** | Integrity zome - immutable validation rules, the "genetic code" of your network |
| **RNA** | This module - transcribes data between DNA versions |
| **Codon** | Transform function - maps old field patterns to new (like nucleotide triplets → amino acids) |
| **Ribosome** | Import function - synthesizes new entries from transformed data |
| **mRNA** | Export data - carries genetic information from source DNA |
| **tRNA** | Bridge call - transfers data between cells |
| **Polymerase** | Orchestrator - the enzyme that catalyzes the transcription process |

This metaphor extends Holochain's biological naming: just as DNA stores genetic information and RNA coordinates its expression, your integrity zome stores validation rules and this module coordinates data migration between versions.

## Quick Start

### Option 1: Use Templates (Recommended)

Copy the templates into your project:

```bash
# Copy Rust template to your coordinator zome
cp templates/migration.rs.template my-dna/zomes/coordinator/src/migration.rs

# Copy TypeScript template to your tools
cp templates/migrate.ts.template tools/migrate.ts

# Customize the placeholders ({{DNA_NAME}}, {{APP_ID}}, etc.)
```

### Option 2: Import as Dependencies

**Rust** (in your coordinator's `Cargo.toml`):
```toml
[dependencies]
hc-rna = { path = "../../rna/rust" }
```

**TypeScript** (in your `package.json`):
```json
{
  "dependencies": {
    "@holochain/rna": "file:../rna/typescript"
  }
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       CONDUCTOR                                  │
│                                                                  │
│  ┌───────────────────┐         ┌───────────────────┐           │
│  │   DNA v1          │         │   DNA v2          │           │
│  │   (source)        │         │   (target)        │           │
│  │                   │  RNA    │                   │           │
│  │ export_for_       │ ─────►  │ import_migrated   │           │
│  │   migration()     │ bridge  │                   │           │
│  │                   │  calls  │ verify_migration  │           │
│  └───────────────────┘         └───────────────────┘           │
│           │                             │                       │
│           │         TypeScript          │                       │
│           │        Orchestrator         │                       │
│           │    (Polymerase enzyme)      │                       │
│           └─────────────┬───────────────┘                       │
│                         │                                        │
│                         ▼                                        │
│                  MigrationReport                                │
└─────────────────────────────────────────────────────────────────┘
```

The migration happens in four phases:

1. **Export** (mRNA synthesis) - Read all entries from source DNA
2. **Transform** (Codon translation) - Map old schema to new schema
3. **Import** (Ribosome synthesis) - Create entries in target DNA
4. **Verify** (Quality control) - Check counts and reference integrity

## Two Migration Patterns

### 1. External Orchestration (Original)

Use TypeScript CLI or external tooling to drive migration. Good for one-time migrations or controlled deployments.

### 2. Self-Healing DNA (New)

The DNA heals itself continuously - no external tools needed. Perfect for:
- Rapid schema iteration during development
- Zero-downtime deployments
- Graceful degradation when healing fails

## Module Structure

```
holochain/rna/
├── README.md                     # This file
├── rust/                         # Rust crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Main exports
│       ├── bridge.rs             # bridge_call helper (tRNA)
│       ├── report.rs             # MigrationReport structs
│       ├── config.rs             # Configuration types
│       ├── traits.rs             # Exporter, Transformer, Importer
│       │
│       │   # Self-Healing Extension
│       ├── healing.rs            # ValidationStatus, HealingSignal, HealingReport
│       ├── self_healing.rs       # SelfHealingEntry trait
│       ├── healing_orchestrator.rs  # HealingOrchestrator
│       ├── entry_type_provider.rs   # Pluggable provider traits
│       ├── healing_strategy.rs   # BridgeFirst, SelfRepairFirst, etc.
│       └── flexible_orchestrator.rs # FlexibleOrchestrator
├── typescript/                   # TypeScript package
│   ├── package.json
│   └── src/
│       ├── index.ts              # Main exports
│       ├── orchestrator.ts       # MigrationOrchestrator (Polymerase)
│       ├── connection.ts         # Holochain connection helpers
│       ├── config.ts             # Configuration types
│       └── report.ts             # Report types
└── templates/                    # Copy-paste templates
    ├── migration.rs.template     # Rust zome template
    └── migrate.ts.template       # TypeScript CLI template
```

## Rust API

### Bridge Calls (tRNA)

```rust
use hc_rna::bridge_call;

// Call export function on previous DNA version
let data: Vec<Content> = bridge_call(
    "my-dna-v1",      // Role name in happ.yaml
    "coordinator",     // Zome name
    "export_all",      // Function name
    ()                 // Payload
)?;
```

### Traits

```rust
use hc_rna::{Transformer, Importer, MigrationReport};

// Implement Transformer for schema changes (Codon)
struct MyTransformer;
impl Transformer<OldEntry, NewEntry> for MyTransformer {
    fn transform(&self, old: OldEntry) -> NewEntry {
        NewEntry {
            id: old.id,
            title: old.title,
            // New field with default
            created_at: old.metadata.get("created_at")
                .cloned()
                .unwrap_or_default(),
        }
    }
}

// Implement Importer for creating entries (Ribosome)
struct MyImporter;
impl Importer<NewEntry> for MyImporter {
    fn import_one(&self, entry: NewEntry) -> ExternResult<bool> {
        if exists(&entry.id)? {
            return Ok(false); // Skip, already exists
        }
        create_entry(&entry)?;
        Ok(true)
    }

    fn entry_type_name(&self) -> &str {
        "MyEntry"
    }
}
```

### Migration Report

```rust
use hc_rna::MigrationReport;

let mut report = MigrationReport::new("v1".to_string(), "v2".to_string());

report.record_success("Content");
report.record_skip("Content");  // Already exists
report.record_failure("Path", Some("path-123".to_string()), "Invalid ref".to_string());

report.complete();
println!("Success: {}", report.is_success());
```

## TypeScript API

### Connection

```typescript
import { connect, disconnect } from '@holochain/rna';

const conn = await connect({
  adminUrl: 'ws://localhost:4444',
  appId: 'my-app',
  sourceRole: 'my-dna-v1',
  targetRole: 'my-dna-v2',
});

// Use conn.appWs, conn.sourceCellId, conn.targetCellId

await disconnect(conn);
```

### Orchestrator (Polymerase)

```typescript
import { MigrationOrchestrator, formatReport } from '@holochain/rna';

const orchestrator = new MigrationOrchestrator(
  conn.appWs,
  conn.sourceCellId,
  conn.targetCellId,
  {
    sourceZome: 'coordinator',
    targetZome: 'coordinator',
  }
);

// Full migration
const report = await orchestrator.migrate({ dryRun: false });
console.log(formatReport(report));

// Dry run (preview)
const preview = await orchestrator.migrate({ dryRun: true });

// With custom transform
const report = await orchestrator.migrate({}, (data) => {
  return transformData(data);
});
```

## hApp Configuration

During migration, bundle both DNA versions:

```yaml
# happ.yaml
manifest_version: "1"
name: my-app
roles:
  - name: my-dna              # Current version (v2)
    dna:
      bundled: ./my-dna.dna

  - name: my-dna-previous     # Previous version (v1)
    dna:
      bundled: ./archive/my-dna-v1.dna
```

## Migration Workflow

### For Each Release:

1. **Before development**: Archive current DNA
   ```bash
   cp workdir/my-dna.dna archive/my-dna-v1.dna
   ```

2. **During development**: Write transform functions for schema changes

3. **At release**: Bundle both DNAs, deploy, run migration

4. **After verification**: Remove previous DNA role

### For Hosted Users (Transparent)

```
User → Browser → Node
                   │
                   ├─► my-dna-v2 (primary)
                   │      │
                   │      ├─ bridge ─► my-dna-v1
                   │      │               │
                   │      └─ import ◄─────┘
                   │
                   └─► Migration happens server-side
                       User never notices
```

### For Independent Users (CLI)

```bash
# Download migration bundle
curl -O https://releases.example.com/my-app-migration.happ

# Install with both DNAs
hc sandbox call install-app my-app-migration.happ

# Run migration
npx tsx migrate.ts --source my-dna-v1 --target my-dna-v2

# Verify
npx tsx migrate.ts --verify-only
```

## Safe vs Breaking Changes

| Change | New DNA Hash? | Migration Required? |
|--------|--------------|---------------------|
| Add entry field | Yes | Yes |
| Remove entry field | Yes | Yes |
| Add link type | Yes | Yes |
| Change validation | Yes | Yes |
| **Add coordinator function** | **No** | **No** |
| **Change coordinator logic** | **No** | **No** |
| **Add to metadata_json** | **No** | **No** |

**Pro tip**: Use `metadata_json` fields for extensibility. New fields go there first, then promote to schema when stable.

## Error Handling

```rust
// In transform function - handle missing fields
fn transform(old: OldEntry) -> NewEntry {
    NewEntry {
        // Required field with fallback
        title: old.title.unwrap_or_else(|| "Untitled".to_string()),

        // Optional field from metadata
        tags: old.metadata_json
            .and_then(|m| serde_json::from_str::<Value>(&m).ok())
            .and_then(|v| v.get("tags").cloned())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
    }
}
```

```typescript
// In orchestrator - handle partial failures
const report = await orchestrator.migrate({ dryRun: false });

if (!report.verification.passed) {
  console.error('Migration incomplete:');
  for (const note of report.verification.notes) {
    console.error(`  - ${note}`);
  }
  // Decide: retry, manual fix, or rollback
}
```

## Self-Healing DNA Pattern

For DNAs that need continuous healing without external orchestration:

### 1. Implement Entry Type Providers

```rust
use hc_rna::{EntryTypeProvider, Validator, Transformer, ReferenceResolver, DegradationHandler};

pub struct ContentProvider;
impl EntryTypeProvider for ContentProvider {
    fn entry_type(&self) -> &str { "content" }
    fn validator(&self) -> &dyn Validator { &ContentValidator }
    fn transformer(&self) -> &dyn Transformer { &ContentTransformer }
    fn reference_resolver(&self) -> &dyn ReferenceResolver { &ContentReferenceResolver }
    fn degradation_handler(&self) -> &dyn DegradationHandler { &ContentDegradationHandler }
}
```

### 2. Register in DNA init()

```rust
use hc_rna::{EntryTypeRegistry, FlexibleOrchestrator, BridgeFirstStrategy};

fn init_healing() -> ExternResult<()> {
    let mut registry = EntryTypeRegistry::new();
    registry.register(Arc::new(ContentProvider))?;
    registry.register(Arc::new(LearningPathProvider))?;

    let orchestrator = FlexibleOrchestrator::new(
        FlexibleOrchestratorConfig {
            v1_role_name: Some("my-dna-v1".to_string()),
            healing_strategy: Arc::new(BridgeFirstStrategy),
            allow_degradation: true,
            ..Default::default()
        },
        registry,
    );
    Ok(())
}
```

### 3. Use in Read Paths

```rust
// Try v2 first, fall back to healing
if let Some(entry) = get_from_v2(id)? {
    return Ok(Some(entry));
}
if let Some(healed) = orchestrator.heal_by_id("content", id, None)? {
    return Ok(Some(healed.entry));
}
Ok(None)
```

### Healing Strategies

| Strategy | Behavior |
|----------|----------|
| `BridgeFirstStrategy` | Try v1 bridge, fall back to local repair |
| `SelfRepairFirstStrategy` | Try local repair, fall back to v1 bridge |
| `LocalRepairOnlyStrategy` | Never use v1, only local repair |
| `NoHealingStrategy` | Accept entries as-is, no healing |

See `holochain/dna/lamad-spike/` for a complete implementation example.

## Contributing

This module is part of the [Elohim Protocol](https://github.com/ethosengine/elohim). Contributions welcome.

## License

MIT OR Apache-2.0
