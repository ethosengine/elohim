# Self-Healing DNA Integration - Complete Guide

## Files Delivered (Three Remaining Integrations)

### 1. Read Path Integration ✅
**File**: `/holochain/dna/lamad-spike/zomes/content_store/src/healing_integration.rs` (300+ lines)

**What it provides:**
- `get_content_by_id_with_healing()` - Content with v1 fallback
- `get_path_by_id_with_healing()` - LearningPath with v1 fallback
- `get_step_by_id_with_healing()` - PathStep with v1 fallback
- `get_mastery_by_id_with_healing()` - ContentMastery with v1 fallback

**Pattern for each:**
1. Try to get from v2 (DHT)
2. If not found, try v1 via bridge call
3. Transform v1 data to v2 schema
4. Validate and cache in v2
5. Emit healing signals

**Integration steps:**
1. Add `pub mod healing_integration;` to `lib.rs`
2. Replace direct calls with healing-aware versions in existing functions
3. Update `get_content_by_id()` to call `healing_integration::get_content_by_id_with_healing()`

### 2. Write Path Integration ✅
**File**: Same file - `healing_integration.rs` (bottom section)

**What it provides:**
- `prepare_content_for_storage()` - Sets schema_version, validates
- `prepare_path_for_storage()` - Same for LearningPath
- `prepare_step_for_storage()` - Same for PathStep
- `prepare_mastery_for_storage()` - Same for ContentMastery

**Integration steps:**
In every create/update function, before calling `create_entry()`:

```rust
// OLD:
let content = Content { ... };
let action_hash = create_entry(&EntryTypes::Content(content))?;

// NEW:
let mut content = Content { ... };
let content = healing_integration::prepare_content_for_storage(content)?;
let action_hash = create_entry(&EntryTypes::Content(content))?;
```

### 3. V1 Export Functions ✅
**File**: `/holochain/dna/lamad-v1/zomes/content_store/src/healing_exports.rs` (350+ lines)

**What it provides:**
- `is_data_present()` - Check if v1 has data
- `export_content_by_id()` - Export content for v2
- `export_path_by_id()` - Export path for v2
- `export_step_by_id()` - Export step for v2
- `export_mastery_by_id()` - Export mastery for v2

**Integration steps:**
1. Create v1 coordinator zome if doesn't exist
2. Add `pub mod healing_exports;`
3. Replace placeholder implementations with actual query functions
4. These become bridge-callable from v2 via `hc_rna::bridge_call()`

---

## How to Integrate (Step by Step)

### Step 1: Update Coordinator lib.rs

```rust
// Add at top with other mods
pub mod healing_impl;
pub mod healing_integration;

// Add init() function if not present
#[hdk_extern]
pub fn init(_: InitPayload) -> InitResult {
    // Check v1 availability
    let _ = healing_impl::init_healing();

    Ok(InitResult::Pass)
}
```

### Step 2: Update Read Functions

Find `get_content_by_id()` in lib.rs and update:

```rust
// BEFORE:
#[hdk_extern]
pub fn get_content_by_id(input: QueryByIdInput) -> ExternResult<Option<ContentOutput>> {
    let anchor = StringAnchor::new("content_id", &input.id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())?;
        get_content(action_hash)
    } else {
        Ok(None)
    }
}

// AFTER:
#[hdk_extern]
pub fn get_content_by_id(input: QueryByIdInput) -> ExternResult<Option<ContentOutput>> {
    // Use healing-aware retrieval
    let content = healing_integration::get_content_by_id_with_healing(&input.id)?;

    match content {
        Some(content) => {
            let entry_hash = hash_entry(&EntryTypes::Content(content.clone()))?;

            // Need to get the action_hash - use existing query
            let anchor = StringAnchor::new("content_id", &input.id);
            let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
            let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
            let links = get_links(query, GetStrategy::default())?;

            let action_hash = if let Some(link) = links.first() {
                ActionHash::try_from(link.target.clone())?
            } else {
                // Newly healed entry, create link
                let new_hash = create_entry(&EntryTypes::Content(content.clone()))?;
                create_id_to_content_link(&content.id, &new_hash)?;
                new_hash
            };

            Ok(Some(ContentOutput {
                action_hash,
                entry_hash,
                content,
            }))
        }
        None => Ok(None),
    }
}
```

**Repeat this pattern for:**
- `get_path_with_steps()` → use `get_path_by_id_with_healing()`
- `get_step_by_id()` → use `get_step_by_id_with_healing()`
- Other query functions

### Step 3: Update Write Functions

Find `create_content()` and update:

```rust
// BEFORE:
pub fn create_content(input: CreateContentInput) -> ExternResult<ContentOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let content = Content {
        id: input.id.clone(),
        // ... other fields
        metadata_json: input.metadata_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::Content(content.clone()))?;
    // ...
}

// AFTER:
pub fn create_content(input: CreateContentInput) -> ExternResult<ContentOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let mut content = Content {
        id: input.id.clone(),
        // ... other fields
        metadata_json: input.metadata_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        schema_version: 2,  // NEW: Always current version
        validation_status: String::new(),  // Will be set by prepare_
    };

    // NEW: Prepare and validate
    let content = healing_integration::prepare_content_for_storage(content)?;

    let action_hash = create_entry(&EntryTypes::Content(content.clone()))?;
    // ... rest of function
}
```

**Repeat for:**
- `create_path()` → use `prepare_path_for_storage()`
- `add_path_step()` → use `prepare_step_for_storage()`
- `initialize_mastery()` → use `prepare_mastery_for_storage()`
- Bulk create functions

### Step 4: Add V1 Export Functions

In v1 DNA coordinator zome:

```rust
pub mod healing_exports;

// Add these bridge-callable functions

#[hdk_extern]
pub fn is_data_present(_: ()) -> ExternResult<bool> {
    healing_exports::is_data_present(())
}

#[hdk_extern]
pub fn export_content_by_id(input: serde_json::Value) -> ExternResult<serde_json::Value> {
    let export = healing_exports::export_content_by_id(input)?;
    Ok(serde_json::to_value(export)?)
}

// Same for export_path_by_id, export_step_by_id, export_mastery_by_id
```

Replace the placeholder implementations in `healing_exports.rs`:

```rust
// In healing_exports.rs, replace:
fn get_content_by_id_internal(id: &str) -> ExternResult<Option<Content>> {
    // Replace this with actual call to your v1 get_content_by_id function
    super::get_content_by_id(&id)?.map(|o| o.content).transpose()
}
```

### Step 5: Update Dependency in Cargo.toml

Ensure hc-rna is in dependencies (should already be there):

```toml
[dependencies]
hc-rna = { path = "../../../../rna/rust" }
```

---

## Jenkins Integration

Use the provided `Jenkinsfile.healing`:

```bash
# Copy to root
cp Jenkinsfile.healing Jenkinsfile

# Or create pipeline job pointing to it
```

**The pipeline:**
1. Builds v1 DNA (fallback)
2. Builds v2 DNA (with healing)
3. Creates dual-role hApp
4. Seeds v1 with test data
5. Queries v2 (triggers healing)
6. Verifies data integrity
7. Packages .deb with both DNAs

**Key environment variables:**
- `HOLOCHAIN_ADMIN_URL` - Admin websocket URL
- `HOLOCHAIN_APP_URL` - App websocket URL
- `HC_PORTS_FILE` - Path to .hc_ports file

---

## Testing Checklist

After integration, verify:

- [ ] Read functions use healing (`get_content_by_id()` calls healing version)
- [ ] Write functions set schema_version (`create_content()` sets version=2)
- [ ] Write functions validate (`prepare_*_for_storage()` calls validate())
- [ ] V1 exports are callable (`is_data_present()` returns bool)
- [ ] Bridge calls work (v2 can call v1 functions)
- [ ] Healing signals emit (UI receives "DataMigrated" signals)
- [ ] .deb includes both DNAs (v1 and v2)
- [ ] Jenkins pipeline passes all stages
- [ ] No data loss on schema change (test with real data)
- [ ] Degraded entries still accessible (broken refs don't crash)

---

## Common Issues & Solutions

### Issue: "Bridge call not found"
**Cause**: v1 export functions not registered
**Solution**: Add `#[hdk_extern]` to export functions in v1 coordinator

### Issue: "Entry validation failed"
**Cause**: Reference to non-existent entry
**Solution**: This is correct behavior - mark as Degraded, don't crash
**In code**: Return `ValidationStatus::Degraded` instead of error

### Issue: "Transformation loses fields"
**Cause**: V1→V2 transform missing fields
**Solution**: Ensure all fields copied in transformation function
**In code**: Check `transform_*_v1_to_v2()` implementations

### Issue: ".deb missing v1 DNA"
**Cause**: Build script not copying v1.dna
**Solution**: Update steward build to copy both DNA files
**In code**: Add to build script:
```bash
cp lamad-v1.dna steward/resources/dnas/
cp lamad-v2.dna steward/resources/dnas/
```

---

## Files Summary

| File | Purpose | Status |
|------|---------|--------|
| `healing_integration.rs` | Read/write path glue | ✅ Ready |
| `healing_exports.rs` | V1 bridge exports | ✅ Ready |
| `Jenkinsfile.healing` | CI/CD pipeline | ✅ Ready |
| `lib.rs` | Add healing_impl, healing_integration mods | ⏳ Needs editing |
| `Coordinator read functions` | Update to use healing | ⏳ Needs editing |
| `Coordinator write functions` | Update to set schema_version | ⏳ Needs editing |
| `V1 coordinator` | Add healing_exports module | ⏳ Needs editing |

---

## Expected Result

Once integrated:

1. **Schema changes are safe** - No data loss on DNA updates
2. **Healing is automatic** - v2 queries v1 for missing data
3. **Transparent to users** - Healing happens in background
4. **Observable progress** - Signals show what's happening
5. **Graceful degradation** - Broken refs don't crash app
6. **.deb is self-contained** - Both DNAs included, no external tools

---

## Next Steps

1. **Copy integration files** to your project
2. **Edit coordinator zome** - Add modules, update functions
3. **Edit v1 zome** - Add export functions
4. **Update Jenkinsfile** - Use new healing pipeline
5. **Test locally** - Schema change + data verification
6. **Deploy to testnet** - Verify healing works end-to-end
7. **Deploy to mainnet** - .deb with healing support

**Time estimate**: 2-3 hours of editing, testing per pair

---

## Success Criteria

Your implementation is complete when:

✅ App starts with both v1 and v2 DNAs
✅ Query v2 for content in v1 → returns healed data
✅ New creates set schema_version=2
✅ Validation catches bad references
✅ Signals emit for healing progress
✅ Jenkins pipeline passes all stages
✅ .deb includes both DNA files
✅ Real schema change tested without data loss

