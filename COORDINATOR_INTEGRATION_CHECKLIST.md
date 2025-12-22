# Coordinator Zome Integration Checklist

This is a quick reference for the remaining function updates needed in the coordinator zome.

**Location**: `/projects/elohim/holochain/dna/lamad-spike/zomes/content_store/src/lib.rs`

---

## ✅ Already Updated

- [x] `get_content_by_id()` - Uses healing integration
- [x] `create_content()` - Sets schema_version=2, uses prepare_content_for_storage()

---

## 📋 Read Functions - Needs Update

### 1. `get_path_with_steps()`
- **Line**: Find with `grep -n "pub fn get_path_with_steps"`
- **Pattern**: Replace direct query with `healing_integration::get_path_by_id_with_healing()`
- **Time**: 10 min
- **Status**: ⏳ PENDING

```rust
// OLD:
let path = /* direct DHT query */;

// NEW:
let path = healing_integration::get_path_by_id_with_healing(&path_id)?;
```

### 2. `get_step_by_id()`
- **Line**: Find with `grep -n "pub fn get_step_by_id"`
- **Pattern**: Replace direct query with `healing_integration::get_step_by_id_with_healing()`
- **Time**: 10 min
- **Status**: ⏳ PENDING

### 3. Other Path/Step Queries (if any)
- Check for: `get_path_overview()`, `get_path_full()`, `get_chapters_for_path()`
- If they load individual entries: update to use healing for lookups
- **Time**: 5 min per function

---

## 📝 Write Functions - Needs Update

### 1. `create_path()`
- **Line**: Find with `grep -n "pub fn create_path"`
- **Updates**:
  - Add `schema_version: 2,` to entry
  - Add `validation_status: String::new(),` to entry
  - Call `healing_integration::prepare_path_for_storage(path)?;` before `create_entry()`
- **Time**: 15 min
- **Status**: ⏳ PENDING

```rust
// Add to struct initialization:
schema_version: 2,
validation_status: String::new(),

// Add before create_entry:
let path = healing_integration::prepare_path_for_storage(path)?;
```

### 2. `add_path_step()`
- **Line**: Find with `grep -n "pub fn add_path_step"`
- **Updates**: Same pattern as create_path but for PathStep
- **Time**: 15 min
- **Status**: ⏳ PENDING

### 3. `initialize_mastery()`
- **Line**: Find with `grep -n "pub fn initialize_mastery"`
- **Updates**: Same pattern as above but for ContentMastery
- **Time**: 15 min
- **Status**: ⏳ PENDING

### 4. `bulk_create_content()`
- **Line**: Find with `grep -n "pub fn bulk_create_content"`
- **Updates**: Loop through content, add schema_version/validation_status, call prepare for each
- **Time**: 20 min
- **Status**: ⏳ PENDING

### 5. Any Other Create/Update Functions
- Search for: `create_entry(&EntryTypes::`
- For each: add schema_version, validation_status, call prepare_*_for_storage()
- **Time**: 10 min per function

---

## Quick Find Commands

```bash
# Find all zome functions
grep -n "^pub fn " holochain/dna/lamad-spike/zomes/content_store/src/lib.rs | head -30

# Find all create_entry calls
grep -n "create_entry(" holochain/dna/lamad-spike/zomes/content_store/src/lib.rs | head -20

# Find all Content entries being created
grep -n "EntryTypes::Content" holochain/dna/lamad-spike/zomes/content_store/src/lib.rs | head -20

# Find all LearningPath entries
grep -n "EntryTypes::LearningPath" holochain/dna/lamad-spike/zomes/content_store/src/lib.rs | head -20
```

---

## Priority Order

1. **High Priority** (affects core queries):
   - [ ] `get_path_with_steps()` - Needed for path queries
   - [ ] `create_path()` - Needed for path creation
   - [ ] `add_path_step()` - Needed for step creation

2. **Medium Priority** (affects mastery):
   - [ ] `initialize_mastery()` - Needed for learning tracking
   - [ ] `bulk_create_content()` - Needed for imports

3. **Lower Priority** (nice to have):
   - [ ] Other utility functions

---

## Testing Each Update

After updating each function:

```bash
# Test compilation
cd holochain/dna/lamad-spike/zomes/content_store
cargo build

# If error, check:
# 1. Correct function signature
# 2. Correct healing_integration function name
# 3. Correct error handling
```

---

## Total Remaining Work

- **Read functions**: 3-4 functions × 10 min = 30-40 min
- **Write functions**: 4-5 functions × 15 min = 60-75 min
- **Testing**: 20-30 min
- **Total**: 2-2.5 hours

---

## Sign-Off When Complete

When all functions are updated:

```bash
# Compile to verify
cargo build --manifest-path holochain/dna/lamad-spike/zomes/content_store/Cargo.toml

# Check for errors
echo "✅ Compilation successful"

# Next step: Run Jenkinsfile.healing
```

---

## Example: Complete Update

Here's a complete example for `create_path()`:

```rust
#[hdk_extern]
pub fn create_path(input: CreatePathInput) -> ExternResult<LearningPathOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let mut path = LearningPath {
        id: input.id.clone(),
        version: "1.0".to_string(),
        title: input.title,
        description: input.description,
        purpose: input.purpose,
        created_by: agent_info.agent_initial_pubkey.to_string(),
        difficulty: input.difficulty,
        estimated_duration: input.estimated_duration,
        visibility: input.visibility,
        path_type: input.path_type,
        tags: input.tags.clone(),
        metadata_json: input.metadata_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        schema_version: 2,           // NEW: Always current
        validation_status: String::new(),  // NEW: Will be set by prepare
    };

    // NEW: Prepare and validate
    let path = healing_integration::prepare_path_for_storage(path)?;

    // Create the entry
    let action_hash = create_entry(&EntryTypes::LearningPath(path.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::LearningPath(path.clone()))?;

    // Create index links
    create_id_to_path_link(&input.id, &action_hash)?;
    // ... other links ...

    Ok(LearningPathOutput {
        action_hash,
        entry_hash,
        path,
    })
}
```

That's it! Follow this pattern for each function.
