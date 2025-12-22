# Self-Healing DNA Pattern: Implementation Plan

## Overview

Transform the RNA module from **external orchestration** (migrate externally, then start) to **living DNA** (start always, heal continuously). This makes schema evolution transparent and resilient for any Holochain app.

## Architecture Changes

### Current RNA Pattern (External Orchestration)
```
App v1 (running)  →  Export via bridge  →  Transform externally  →  Import via CLI  →  App v2 (start)
```

### New RNA Pattern (Self-Healing)
```
App v1 (running)  →  App v2 provisions  →  v2 init() checks bridge  →  Reads/writes trigger lazy migration  →  App heals continuously
                                              ↓
                                         Mark as needing healing
                                              ↓
                                         Background tasks heal
                                              ↓
                                         Emit healing signals
```

## Core Components to Implement

### 1. Validation Status & Entry Versioning (NEW)

**File**: `holochain/rna/rust/src/healing.rs`

Core types:
- `ValidationStatus` enum: Valid, Migrated, Degraded, Healing
- `VersionedEntry` trait: Any entry can declare its schema version
- `ValidationRule` trait: Custom validation logic per entry type
- `HealingSignal` enum: Signals emitted during healing

### 2. Self-Healing Entry Trait (NEW)

**File**: `holochain/rna/rust/src/self_healing.rs`

```rust
pub trait SelfHealingEntry: Serialize + DeserializeOwned {
    /// Get this entry's schema version
    fn schema_version(&self) -> u32;

    /// Get this entry's validation status
    fn validation_status(&self) -> ValidationStatus;

    /// Set validation status
    fn set_validation_status(&mut self, status: ValidationStatus);

    /// Get unique identifier for this entry
    fn entry_id(&self) -> String;

    /// Validate this entry (check references, required fields, etc)
    fn validate(&self) -> Result<(), String>;

    /// Mark when this entry was healed (if applicable)
    fn set_healed_at(&mut self, timestamp: u64);
}
```

### 3. Healing Task Orchestrator (NEW)

**File**: `holochain/rna/rust/src/healing_orchestrator.rs`

Types:
- `HealingTask`: Represents one degraded entry needing healing
- `HealingReport`: Summary of healing work done
- `HealingOrchestrator`: Manages heal workflow

Capabilities:
- Find degraded entries
- Attempt healing with retry logic
- Track healing progress
- Emit signals
- Never fail (graceful degradation)

### 4. Bridge Heal Pattern (NEW)

**File**: `holochain/rna/rust/src/bridge_heal.rs`

```rust
pub trait BridgeHeal {
    /// Check if v1 bridge is available
    fn check_bridge_availability(&self) -> ExternResult<bool>;

    /// Export from v1 fallback
    fn export_from_v1(&self, id: &str) -> ExternResult<serde_json::Value>;

    /// Transform v1 data to current schema
    fn heal_from_v1(&self, v1_data: serde_json::Value) -> ExternResult<Self>
    where Self: Sized;
}
```

### 5. Enhanced Reporting (UPDATE existing)

**File**: `holochain/rna/rust/src/report.rs` (extend)

Add:
- `HealingReport`: Tracks healing attempt results
- `ValidationReport`: Per-entry validation results
- Signal emission infrastructure

### 6. TypeScript Healing Monitoring (NEW)

**File**: `holochain/rna/typescript/src/healing.ts`

Capabilities:
- Subscribe to healing signals
- Monitor healing progress
- Query healing status
- Detect when system is fully healed

## Templates to Create

### 1. self-healing.rs.template
Shows how to:
- Implement `SelfHealingEntry` for a custom entry
- Define validation rules
- Implement `BridgeHeal` for v1 fallback
- Use healing orchestrator in init()
- Use healing orchestrator in read paths

### 2. self-healing-app.ts.template
Shows how to:
- Set up healing signal listeners
- Monitor healing progress
- Display healing status in UI
- Handle degraded entries gracefully

## Implementation Sequence

### Phase 1: Core Framework (Rust)
1. ✅ Design `healing.rs` with `ValidationStatus` and traits
2. ✅ Implement `self_healing.rs` with `SelfHealingEntry` trait
3. ✅ Build `healing_orchestrator.rs` with task management
4. ✅ Create `bridge_heal.rs` for v1 fallback pattern
5. ✅ Update `report.rs` to track healing
6. ✅ Update `lib.rs` to export new types

### Phase 2: TypeScript Orchestration
7. ✅ Add healing monitoring to TypeScript orchestrator
8. ✅ Implement signal handling
9. ✅ Create debugging utilities

### Phase 3: Templates & Documentation
10. ✅ Create `self-healing.rs.template`
11. ✅ Create `self-healing-app.ts.template`
12. ✅ Document adoption pattern

### Phase 4: Lamad Implementation
13. ✅ Implement `SelfHealingEntry` for Content, Path, Mastery, Progress
14. ✅ Define validation rules for each entry type
15. ✅ Implement BridgeHeal for v1 fallback
16. ✅ Integrate healing into init and read paths

### Phase 5: Jenkins Integration
17. ✅ Add healing step to Jenkinsfile
18. ✅ Create verification that system is fully healed
19. ✅ Package .deb with healing enabled

## Key Design Principles

1. **Always Succeeds on Startup**: `init()` never fails, returns `Pass` always
2. **Graceful Degradation**: Missing/broken data returns degraded entry, not error
3. **Continuous Healing**: Background tasks fix issues over time
4. **Validation on Read**: Every query checks integrity
5. **Signals Emitted**: App knows when healing happens
6. **Idempotent Operations**: Healing can run multiple times safely
7. **Version Embedded**: Every entry knows its schema version
8. **Bridge Always Available**: Always reach back to v1 if needed
9. **Observable State**: App sees validation_status for UI decisions
10. **Never Loses Data**: Worst case is degraded access, never deletion

## Extensibility Points

Any Holochain app can adopt this by:

```rust
// 1. Implement SelfHealingEntry for your entry types
impl SelfHealingEntry for MyEntry {
    // ... define validation, versioning, healing logic
}

// 2. Implement BridgeHeal for fallback to v1
impl BridgeHeal for MyEntry {
    // ... define how to read from v1 and heal
}

// 3. Use in init()
pub fn init(_: InitPayload) -> InitResult {
    let orchestrator = HealingOrchestrator::new(
        "my-dna-v1",
        "my-dna-v2",
    );

    match orchestrator.check_and_heal_on_startup()? {
        HealingResult::NoV1Data => { /* fresh start */ },
        HealingResult::DataHealed { .. } => { /* healing happened */ },
        HealingResult::HealingNeeded => { /* mark for background healing */ },
    }

    Ok(InitResult::Pass)
}

// 4. Use in read paths
pub fn get_my_entry(id: String) -> ExternResult<MyEntry> {
    match get_entry(&id)? {
        Some(entry) => Ok(entry),  // v2 format, all good
        None => {
            // Check v1 bridge, heal if possible
            HealingOrchestrator::try_heal_from_bridge::<MyEntry>(&id)?
        }
    }
}

// 5. Background healing (scheduled or signal-triggered)
pub fn heal_degraded() -> ExternResult<HealingReport> {
    HealingOrchestrator::heal_all_degraded::<MyEntry>()
}
```

## Success Criteria

- [ ] Any entry type can implement `SelfHealingEntry`
- [ ] Init never fails, always returns `Pass`
- [ ] Degraded entries are accessible (with status flag)
- [ ] Background healing works continuously
- [ ] Bridge fallback to v1 works seamlessly
- [ ] Signals enable UI awareness of healing
- [ ] Lamad v1→v2 migration works end-to-end with this pattern
- [ ] Jenkins .deb build includes healing orchestration
- [ ] Can redeploy with schema changes without data loss
- [ ] Framework is generic enough for any Holochain app

## Files to Create/Modify

### New Files
- `holochain/rna/rust/src/healing.rs` - Core validation/signal types
- `holochain/rna/rust/src/self_healing.rs` - SelfHealingEntry trait
- `holochain/rna/rust/src/healing_orchestrator.rs` - Healing task management
- `holochain/rna/rust/src/bridge_heal.rs` - V1 fallback pattern
- `holochain/rna/typescript/src/healing.ts` - Healing monitoring
- `holochain/rna/templates/self-healing.rs.template` - App template
- `holochain/rna/templates/self-healing-app.ts.template` - UI template

### Modified Files
- `holochain/rna/rust/src/lib.rs` - Export new modules
- `holochain/rna/rust/src/report.rs` - Add healing report types
- `holochain/rna/rust/Cargo.toml` - Dependencies if needed
- `holochain/rna/typescript/src/index.ts` - Export healing types

## Testing Strategy

For each component:
1. Unit tests in Rust (traits, transformations)
2. Integration tests with dual-DNA setup
3. Operational tests with elohim-app
4. Jenkins pipeline tests

For Lamad:
1. Seed v1 data
2. Provision v2 DNA
3. Trigger healing via signals
4. Verify all data accessible
5. Run app tests against healed data

