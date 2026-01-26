# Schema Generator Quickstart Guide

## 5-Minute Setup

### Step 1: Build the Generator Tool

```bash
cd holochain/rna/rust
cargo build --bin hc-rna-generate --features cli
```

### Step 2: Generate Providers for Your DNA

```bash
./target/debug/hc-rna-generate \
  --integrity ../dna/your-dna/zomes/your_integrity/src/lib.rs \
  --output ../dna/your-dna/zomes/your_coordinator/src/providers_generated.rs \
  --verbose
```

### Step 3: Review Generated Code

```bash
# Preview without writing
./target/debug/hc-rna-generate \
  --integrity ../dna/your-dna/zomes/your_integrity/src/lib.rs \
  --output /tmp/preview.rs \
  --dry-run \
  --verbose
```

### Step 4: Customize Validators

Open `providers_generated.rs` and add validation rules:

```rust
pub struct ContentValidator;

impl Validator for ContentValidator {
    fn validate_json(&self, data: &Value) -> Result<(), String> {
        // ✓ Generated field checks
        let id = data["id"].as_str().ok_or("id required")?;

        // TODO: Add enum validation
        let content_type = data["content_type"].as_str().ok_or("content_type required")?;
        const VALID_TYPES: &[&str] = &["concept", "lesson", "assessment"];
        if !VALID_TYPES.contains(&content_type) {
            return Err(format!("Invalid content_type: {}", content_type));
        }

        Ok(())
    }
}
```

### Step 5: Register Providers

Update your DNA's `lib.rs`:

```rust
use hc_rna::{EntryTypeRegistry, FlexibleOrchestrator, FlexibleOrchestratorConfig, BridgeFirstStrategy};
use std::sync::Arc;

pub mod providers;  // Add this

#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    init_flexible_orchestrator()?;
    Ok(InitCallbackResult::Pass)
}

fn init_flexible_orchestrator() -> ExternResult<()> {
    let mut registry = EntryTypeRegistry::new();

    // Register all providers
    registry.register(Arc::new(providers::ContentProvider))?;
    registry.register(Arc::new(providers::LearningPathProvider))?;
    // ... register all providers

    let config = FlexibleOrchestratorConfig {
        v1_role_name: Some("dna-v1".to_string()),
        v2_role_name: Some("dna-v2".to_string()),
        healing_strategy: Arc::new(BridgeFirstStrategy),
        allow_degradation: true,
        max_attempts: 3,
        emit_signals: true,
    };

    let _orchestrator = FlexibleOrchestrator::new(config, registry);
    Ok(())
}
```

## Common Workflows

### Generate with All Helper Features

```bash
hc-rna-generate \
  -i integrity/src/lib.rs \
  -o coordinator/src/providers.rs \
  --with-registration \      # Generate registration code
  --with-tests \             # Generate test templates
  --with-docs \              # Generate documentation
  -vvv                       # Maximum verbosity
```

**Output:**
1. `providers.rs` - All validators, transformers, resolvers, handlers, providers
2. Registration code snippet ready to copy-paste
3. Test file with starter test cases
4. Documentation for each provider

### Generate Only Specific Entry Types

```bash
# Only generate for Content, LearningPath, PathStep
hc-rna-generate \
  -i integrity/src/lib.rs \
  -o providers.rs \
  --only "Content, LearningPath, PathStep"
```

### Skip Certain Entry Types

```bash
# Generate for everything except governance types
hc-rna-generate \
  -i integrity/src/lib.rs \
  -o providers.rs \
  --skip "Proposal, Challenge, Discussion, GovernanceState"
```

### Merge with Existing Providers

```bash
# Keep existing customizations, add new entry types
hc-rna-generate \
  -i integrity/src/lib.rs \
  -o coordinator/src/providers.rs \
  --merge \
  -v
```

**What happens:**
- Existing customizations are preserved
- New entry types are added
- Duplicate providers skip existing ones
- TODO markers guide what to update

### Get Statistics (JSON Format)

```bash
hc-rna-generate \
  -i integrity/src/lib.rs \
  -o /tmp/providers.rs \
  --dry-run \
  --format json
```

**Output:**
```json
{
  "entry_types": 57,
  "fields": 1024,
  "references": 234,
  "enums": 12,
  "output": {
    "lines": 12334,
    "bytes": 467456
  },
  "generation_time_ms": 245
}
```

### Export Entry Type Inventory (CSV)

```bash
hc-rna-generate \
  -i integrity/src/lib.rs \
  -o /tmp/providers.rs \
  --dry-run \
  --format csv
```

**Output:**
```csv
entry_type,fields,references,required_fields
content,20,4,8
learning_path,15,3,7
path_step,25,5,12
```

## Verbosity Levels

### `-v` (Standard)
```
✓ Found 57 entry types
✓ Found 12 enums
🔨 Generating provider implementations...
✅ Successfully generated providers!
```

### `-vv` (Detailed)
```
✓ Found 57 entry types:
   - Content (20 fields, 4 references)
   - LearningPath (15 fields, 3 references)
   ...
✓ Found 12 enums:
   - ValidationStatus (4 variants)
   - ContentType (10 variants)
   ...
🔨 Generating provider implementations...
✓ Generated code written to: providers.rs

📊 Statistics:
   Entry types: 57
   Fields: 1024
   References: 234
   Output lines: 12334
   Generation time: 245ms
```

### `-vvv` (Debug)
```
[Shows everything from -vv plus:]
🔍 Analyzing DNA schema...
   Integrity file: zomes/content_store_integrity/src/lib.rs
   Regex match found: pub struct Content
   Field parsed: id (type: String, required: true, reference: false)
   Field parsed: content_type (type: String, required: true, reference: false)
   ...
```

## CLI Reference

```bash
hc-rna-generate [OPTIONS] --integrity <PATH> --output <PATH>

OPTIONS:
  -i, --integrity <PATH>
      Path to the integrity zome lib.rs file

  -o, --output <PATH>
      Path to output providers.rs file

  -d, --dry-run
      Preview output without writing to file

  --diff
      Show diff between existing and generated code

  -m, --merge
      Merge with existing providers (keep custom changes)

  -v, --verbose
      Increase verbosity level (can be used multiple times: -v, -vv, -vvv)

  --with-tests
      Generate test file alongside providers

  --with-registration
      Generate registration code snippet

  --with-docs
      Generate documentation for providers

  --only <TYPES>
      Only generate providers for specific entry types (comma-separated)
      Example: --only "Content, LearningPath, PathStep"

  --skip <TYPES>
      Skip generating providers for specific entry types (comma-separated)
      Example: --skip "Proposal, Challenge"

  --validate
      Validate generated code can compile

  --format <FORMAT>
      Output format: human (default), json, csv

  -h, --help
      Print help
```

## Workflow Examples

### New DNA Project

```bash
# 1. Create basic structures in integrity zome
# (in content_store_integrity/src/lib.rs)
pub struct Content { ... }
pub struct LearningPath { ... }

# 2. Generate initial providers
./hc-rna-generate \
  -i zomes/content_store_integrity/src/lib.rs \
  -o zomes/content_store/src/providers.rs \
  --with-registration \
  -v

# 3. Customize validators in providers.rs
# Add enum validation, constraints, business rules

# 4. Copy registration code into lib.rs init()

# 5. Test
cargo test
cargo build
```

### Existing DNA with New Entry Types

```bash
# 1. Add new struct to integrity zome
pub struct Assessment { ... }

# 2. Re-generate with merge
./hc-rna-generate \
  -i zomes/content_store_integrity/src/lib.rs \
  -o zomes/content_store/src/providers.rs \
  --merge \
  -v

# 3. Review new Assessment provider
# Customize if needed

# 4. Register in init_flexible_orchestrator()
// Add: registry.register(Arc::new(providers::AssessmentProvider))?;

# 5. Test
cargo test
cargo build
```

### Schema Evolution (v1 to v2)

```bash
# 1. Update structs in integrity zome
// Old: pub struct Content { pub old_field: String }
// New: pub struct Content { pub new_field: String }

# 2. Re-generate
./hc-rna-generate \
  -i zomes/content_store_integrity/src/lib.rs \
  -o zomes/content_store/src/providers.rs \
  -vv

# 3. Update transformer for field migration
pub struct ContentTransformer;
impl Transformer for ContentTransformer {
    fn transform_v1_to_v2(&self, v1_data: &Value) -> Result<Value, String> {
        // Generated: extracts v1_data["old_field"]
        // TODO: Map old_field → new_field
        let old_field = v1_data["old_field"].as_str().unwrap_or("");
        let new_field = convert_old_to_new(old_field);
        // Generated: builds v2 JSON with new_field
        Ok(serde_json::json!({
            "new_field": new_field,
            ...
        }))
    }
}

# 4. Test healing workflow
cargo test
```

## Troubleshooting

### "No entry types found"
**Problem:** Generator found 0 entry types in integrity zome

**Solution:**
1. Verify file path is correct
2. Check that structs have `pub` visibility: `pub struct MyEntry { ... }`
3. Check that structs have an `id` field
4. Run with `-vvv` to see parsing details

```bash
./hc-rna-generate -i zomes/integrity/src/lib.rs -o /tmp/debug.rs -vvv
```

### "Failed to read file"
**Problem:** Can't read integrity zome file

**Solution:**
1. Check file exists: `ls -la zomes/integrity/src/lib.rs`
2. Check permissions: `chmod +r zomes/integrity/src/lib.rs`
3. Use absolute path if relative path doesn't work

### Generated code has parse errors
**Problem:** Generated code won't compile

**Solution:**
1. Check integrity zome has valid Rust syntax
2. Run analyzer in debug mode: `-vvv`
3. Review TODO comments in generated code
4. Missing types often need to be imported

## Tips & Tricks

### Dry-run before generating

```bash
# Always preview first
./hc-rna-generate \
  -i integrity/src/lib.rs \
  -o providers.rs \
  --dry-run \
  -vv

# Review output, then run without --dry-run
./hc-rna-generate \
  -i integrity/src/lib.rs \
  -o providers.rs \
  -v
```

### Keep generated and custom code separate

```bash
# Generate to separate file
./hc-rna-generate \
  -i integrity/src/lib.rs \
  -o src/providers_generated.rs \
  -v

# Create wrapper file that includes and customizes
// src/providers.rs
pub mod generated {
    include!("providers_generated.rs");
}

// Custom implementations that override generated ones
pub use generated::*;

pub struct ContentValidator;  // Custom implementation
impl Validator for ContentValidator { /* your code */ }
```

### Generate registration code separately

```bash
# Generate just the registration code
./hc-rna-generate \
  -i integrity/src/lib.rs \
  -o /tmp/providers.rs \
  --dry-run \
  --with-registration

# Copy the registration code output into your lib.rs
```

### Export statistics for reporting

```bash
# Get JSON output for dashboards/reports
./hc-rna-generate \
  -i integrity/src/lib.rs \
  -o /tmp/providers.rs \
  --dry-run \
  --format json > stats.json

# Get CSV for spreadsheets
./hc-rna-generate \
  -i integrity/src/lib.rs \
  -o /tmp/providers.rs \
  --dry-run \
  --format csv > entry_types.csv
```

## Next Steps

1. **Review Generated Code**: Check validators, transformers, resolvers
2. **Customize Validators**: Add enum validation and business rules
3. **Verify Transformers**: Check field mappings match your v1→v2 schema
4. **Implement Resolvers**: Add DHT lookups for reference checking
5. **Set Degradation Policies**: Decide which entry types fail vs. degrade
6. **Run Tests**: Test each provider independently
7. **Integration Testing**: Test healing workflow end-to-end
8. **Deploy**: Update init() to register all providers

## Resources

- **SCHEMA_ANALYZER_AND_GENERATOR.md** - Full technical documentation
- **ARCHITECTURE.md** - Design pattern explanation
- **IMPLEMENTATION_SUMMARY.md** - Implementation overview
- **FLEXIBLE_HEALING_INTEGRATION.md** - Integration guide

## Getting Help

```bash
# View all options
./hc-rna-generate --help

# Run with maximum verbosity
./hc-rna-generate -i integrity/src/lib.rs -o providers.rs -vvv
```
