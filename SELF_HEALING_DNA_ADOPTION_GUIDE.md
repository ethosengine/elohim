# Self-Healing DNA Pattern: Adoption Guide

## Overview

The self-healing DNA pattern enables any Holochain app to **survive schema evolution without data loss**. Instead of external migration tools, the DNA itself handles healing continuously.

## Key Concepts

### Three Entry States

Every entry has a `ValidationStatus`:

- **Valid**: Current schema, all references good
- **Migrated**: Came from v1, has been validated
- **Degraded**: Has issues (missing refs, validation failed) but still accessible
- **Healing**: Currently being repaired

### The Pattern

```
App v1 (running)
    ↓
    ├─ Build v2 DNA
    ├─ Provision v2 in same hApp
    ├─ App connects to v2
    ├─ v2.init() checks v1 bridge availability
    ├─ First query detects v1 has data
    ├─ Query path: try v2 → v1 (heal) → return
    ├─ Emit signals about healing progress
    └─ App shows healing status in UI
```

### No External Orchestration Required

Unlike traditional migration:
- No CLI tools
- No manual migration step
- No coordination between processes
- No network resets

Just: **Provision → Run → Heal automatically**

---

## Implementation Steps

### Step 1: Implement `SelfHealingEntry` Trait

For each entry type that needs to survive schema evolution:

```rust
use hc_rna::{SelfHealingEntry, ValidationStatus};

#[hdk_entry(type = "content")]
#[derive(Clone)]
pub struct Content {
    pub id: String,
    pub title: String,
    pub schema_version: u32,
    pub validation_status: ValidationStatus,
    // ... your fields
}

impl SelfHealingEntry for Content {
    fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn validation_status(&self) -> ValidationStatus {
        self.validation_status
    }

    fn set_validation_status(&mut self, status: ValidationStatus) {
        self.validation_status = status;
    }

    fn entry_id(&self) -> String {
        self.id.clone()
    }

    fn validate(&self) -> Result<(), String> {
        // Check required fields
        if self.title.is_empty() {
            return Err("Title required".to_string());
        }

        // Check reference integrity
        // This is critical - if references are broken, return Err

        Ok(())
    }
}
```

### Step 2: Add Schema Versioning Fields

Every entry needs two fields:

```rust
pub schema_version: u32,           // Increment when schema changes
pub validation_status: ValidationStatus,  // Tracks health
```

### Step 3: Define Validation Rules

What makes an entry valid?

```rust
impl SelfHealingEntry for Content {
    fn validate(&self) -> Result<(), String> {
        // Required fields
        if self.title.is_empty() {
            return Err("Title is required".to_string());
        }

        // Reference integrity
        if let Some(parent_id) = &self.parent_id {
            match get_content_by_id(parent_id) {
                Ok(Some(_)) => {},
                Ok(None) => return Err(format!("Parent {} not found", parent_id)),
                Err(e) => return Err(format!("Error checking parent: {:?}", e)),
            }
        }

        // Field constraints
        if self.title.len() > 1000 {
            return Err("Title too long".to_string());
        }

        Ok(())
    }
}
```

### Step 4: Implement V1 Fallback

Create a transformation function:

```rust
// V1 export format
#[derive(Serialize, Deserialize)]
pub struct ContentV1 {
    pub id: String,
    pub title: String,
    pub parent_id: Option<String>,
}

// Transform v1 to v2
pub fn transform_v1_to_v2(v1: ContentV1) -> Content {
    Content {
        id: v1.id,
        title: v1.title,
        parent_id: v1.parent_id,
        schema_version: 2,  // Current version
        validation_status: ValidationStatus::Migrated,
    }
}

// V1 DNA provides export function
pub fn export_content_by_id(id: String) -> ExternResult<ContentV1> {
    // Query and return v1 format
}
```

### Step 5: Hook into Initialization

In your DNA's init:

```rust
use hc_rna::HealingOrchestrator;

pub fn init(_: InitPayload) -> InitResult {
    let orchestrator = HealingOrchestrator::new(
        "my-dna-v1",      // Previous DNA role
        "my-dna-v2",      // Current DNA role
    );

    // Check if v1 has data to heal
    match orchestrator.check_v1_on_startup()? {
        Some(has_data) => {
            if has_data {
                // v1 has data, will heal on first query
                debug_log("V1 data found, will heal on demand")?;
            }
        }
        None => {
            // No v1 bridge, fresh start
            debug_log("Fresh start, no v1 bridge")?;
        }
    }

    Ok(InitResult::Pass)  // Always Pass!
}
```

### Step 6: Hook into Read Paths

Replace direct queries with healing-aware paths:

```rust
pub fn get_content(id: String) -> ExternResult<Content> {
    // Try v2 first
    match get_content_by_id(&id)? {
        Some(entry) => {
            // Validate on read
            match entry.validate() {
                Ok(_) => return Ok(entry),
                Err(_) => {
                    // Validation failed, try healing
                }
            }
        }
        None => {
            // Not in v2, will try v1
        }
    }

    // Try to heal from v1
    let orchestrator = HealingOrchestrator::new("my-dna-v1", "my-dna-v2");

    let v1_entry: ContentV1 = hc_rna::bridge_call(
        "my-dna-v1",
        "content_coordinator",
        "export_content_by_id",
        serde_json::json!({ "id": id }),
    )?;

    let mut healed = transform_v1_to_v2(v1_entry);
    healed.validate()?;

    // Optionally cache in v2
    create_entry(&healed)?;

    emit_healing_signal(HealingSignal::HealingSucceeded {
        entry_id: id,
        entry_type: "Content".to_string(),
        was_migrated_from_v1: true,
    })?;

    Ok(healed)
}
```

### Step 7: Hook into Write Paths

Validate and update schema version on every write:

```rust
pub fn create_content(input: ContentInput) -> ExternResult<ActionHash> {
    let mut entry = Content {
        id: input.id,
        title: input.title,
        schema_version: 2,  // Always current version
        validation_status: ValidationStatus::Valid,
        parent_id: input.parent_id,
    };

    // Validate before creating
    entry.validate()?;

    create_entry(&entry)
}
```

### Step 8: Configure hApp

Update your `happ.yaml` to provision both roles:

```yaml
manifest_version: "1"
name: my-app

roles:
  - name: "my-dna-v1"
    dna:
      modifiers:
        network_seed: "my-dna-v1"
      path: "./my-dna-v1.dna"

  - name: "my-dna-v2"
    dna:
      modifiers:
        network_seed: "my-dna-v2"
      path: "./my-dna-v2.dna"
```

### Step 9: Optional - Background Healing

For large datasets, schedule background healing:

```rust
pub fn heal_all_degraded() -> ExternResult<u32> {
    let filter = ChainQueryFilter::new()
        .entry_type(EntryTypes::Content.try_into()?);
    let records = query(filter)?;

    let mut healed_count = 0;
    for record in records {
        if let Entry::App(app_entry) = record.entry() {
            if let Ok(mut entry) = app_entry.deserialize::<Content>() {
                if entry.validation_status == ValidationStatus::Degraded {
                    // Try to heal
                    match entry.validate() {
                        Ok(_) => {
                            entry.validation_status = ValidationStatus::Valid;
                            healed_count += 1;
                        }
                        Err(_) => {
                            // Still broken, try v1
                            match heal_from_v1(&entry.id) {
                                Ok(_) => healed_count += 1,
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(healed_count)
}
```

### Step 10: UI Awareness (TypeScript)

Monitor healing progress in your app:

```typescript
import {
  HealingMonitor,
  HealingSignalType,
  formatHealingSignal,
} from '@holochain/rna';

// Create monitor
const monitor = new HealingMonitor();

// Subscribe to signals
monitor.onSignal((signal) => {
  console.log(formatHealingSignal(signal));

  // Update UI based on signal type
  switch (signal.type) {
    case HealingSignalType.DegradedEntryFound:
      showNotification(`Entry ${signal.entry_id} needs healing`);
      break;
    case HealingSignalType.HealingSucceeded:
      showSuccess(`Healed ${signal.entry_id}`);
      break;
    case HealingSignalType.SystemFullyHealed:
      showSuccess(`All entries healed!`);
      break;
  }
});

// Get current status
const status = monitor.getStatus();
console.log(`System is ${status.is_healthy ? 'healthy' : 'degraded'}`);
console.log(`Degraded entries: ${status.degraded_count}`);
console.log(`Being healed: ${status.healing_in_progress_count}`);

// Show healed entries
const degraded = monitor.getDegradedEntries();
degraded.forEach(entry => {
  console.log(`${entry.entry_type} ${entry.entry_id}: ${entry.last_error}`);
});
```

---

## Migration Workflow

### From Old Approach (External Orchestration)

Before:
```bash
# Stop app
# Export from v1
# Transform data
# Import to v2
# Restart app
# Hope nothing is broken
```

After (Self-Healing):
```bash
# Build v2 DNA
# Update .deb with both DNAs
# Redeploy
# App starts, heals on first query
# Done
```

### Schema Evolution

**Old**: Every schema change breaks the app
```
v1.0 (data)
  ↓
v1.1 (data) - migration needed, manual step
  ↓
v1.2 (data) - migration needed again
```

**New**: Schema evolves transparently
```
v1 (data)
  ↓
v2 (heals from v1 on demand)
  ↓
v3 (heals from v1 via v2 chain)
  ↓
vN (complete history available)
```

---

## Testing

### Unit Tests

```rust
#[test]
fn test_entry_validation() {
    let entry = Content {
        id: "test".to_string(),
        title: "Valid".to_string(),
        schema_version: 2,
        validation_status: ValidationStatus::Valid,
        parent_id: None,
    };

    assert!(entry.validate().is_ok());
}

#[test]
fn test_v1_transformation() {
    let v1 = ContentV1 {
        id: "test".to_string(),
        title: "Old".to_string(),
        parent_id: None,
    };

    let v2 = transform_v1_to_v2(v1);
    assert_eq!(v2.schema_version, 2);
    assert_eq!(v2.validation_status, ValidationStatus::Migrated);
}
```

### Integration Tests

1. Seed v1 with test data
2. Provision v2
3. Query v2 (triggers healing)
4. Verify all data accessible
5. Run app tests

---

## Best Practices

1. **Always Call validate()**: On every read and write
2. **Never Panic**: Return Err, let the app handle degradation
3. **Emit Signals**: So UI can show healing progress
4. **Version Everything**: When schema changes, increment schema_version
5. **Test Transformation**: Unit test v1→v2 transformations
6. **Reference Integrity**: Most important validation rule
7. **Graceful Degradation**: Return data with Degraded status, don't error
8. **Init Never Fails**: Always return InitResult::Pass
9. **Bridge Always Available**: Design assuming v1 may be needed
10. **Signal Driven**: Let UI respond to healing, not app logic

---

## Troubleshooting

### Entry is always Degraded

- Check validation rules, may be too strict
- Verify references actually exist
- Try heal_from_v1() manually to debug

### Healing never completes

- Check v1 role name in happ.yaml matches code
- Verify bridge capability grants are correct
- Check zome function names match

### App slow after update

- Too many degraded entries
- Validation rules are expensive
- Schedule background healing to clear degraded queue

### Lost data after update

- Validation rule rejected good data
- Transform function lost fields
- Reference check too strict
- Check error logs

---

## Example: Lamad

See `/projects/elohim/holochain/dna/lamad-spike/zomes/content_store/src/lib.rs` for complete working example with:

- Content, Path, Mastery, Progress entries
- V1→V2 transformations for each
- Reference integrity validation
- Background healing task
- Jenkins integration

---

## Summary

Self-healing DNA means:

1. **Never lose data** on schema changes
2. **No external tools** needed
3. **No coordination** between processes
4. **Transparent healing** as the app runs
5. **Rapid iteration** on schema
6. **Observable progress** via signals
7. **Always operational** - init never fails
8. **Graceful degradation** - broken data still accessible
9. **Living system** - constantly self-repairs
10. **Generic pattern** - works for any Holochain app

The pattern is intentionally simple so you can adopt it incrementally:

- Start with one entry type
- Expand to more entry types
- Add background healing
- Add UI awareness
- Then iterate schema rapidly

