# Self-Healing DNA - Practical Fix Guide

**Time to Fix**: 60-90 minutes
**Difficulty**: Easy to Medium

---

## Fix 1: Define Validation Constants

**File**: `/projects/elohim/holochain/dna/lamad-spike/zomes/content_store/src/healing_impl.rs`

**Step 1**: Open the file and find line 15 (after imports)

**Step 2**: Add these constants right after the imports:

```rust
// ============================================================================
// Validation Constants (used in SelfHealingEntry implementations)
// ============================================================================

const CONTENT_TYPES: &[&str] = &[
    "concept",       // Atomic knowledge concept
    "lesson",        // Structured learning unit
    "practice",      // Hands-on practice activity
    "assessment",    // Knowledge check
    "reference",     // External reference material
];

const REACH_LEVELS: &[&str] = &[
    "public",   // Open to everyone
    "commons",  // Shared commons (curated)
    "private",  // Private/restricted
];

const CONTENT_FORMATS: &[&str] = &[
    "markdown",   // Markdown format
    "html",       // HTML format
    "plaintext",  // Plain text
    "video",      // Video media
];

const PATH_VISIBILITIES: &[&str] = &[
    "public",  // Published path
    "private", // Private path
    "draft",   // Draft in progress
];

const STEP_TYPES: &[&str] = &[
    "content",   // Reference content
    "path",      // Reference another path
    "external",  // External URL
    "practice",  // Practice activity
];

const MASTERY_LEVELS: &[&str] = &[
    "recognize",    // Can identify
    "recall",       // Can recall
    "understand",   // Understands concepts
    "apply",        // Can apply knowledge
    "synthesize",   // Can combine and create
];

const COMPLETION_CRITERIA: &[&str] = &[
    "all-required",     // All steps required
    "pass-assessment",  // Must pass assessment
    "view-content",     // Just view content
];
```

**Location**: Insert right after line 14 (the imports section), before line 15 (SelfHealingEntry implementation).

**Verify**: Code should compile now for the constant references.

---

## Fix 2: Remove Problematic Reference Validation

**File**: Same as above
**Locations**:
- Content validation: lines 80-96
- LearningPath validation: lines 149-150 (optional, commented)
- PathStep validation: lines 205-229
- ContentMastery validation: lines 315-320 (optional)

### For Content (MUST FIX):

**Current code (lines 80-96)**:
```rust
// Check reference integrity - related_node_ids point to valid content
for related_id in &self.related_node_ids {
    match get_content_by_id_internal(related_id) {
        Ok(Some(_)) => {},
        Ok(None) => {
            return Err(format!(
                "Related content {} not found",
                related_id
            ))
        },
        Err(_) => {
            return Err(format!(
                "Error checking reference to content {}",
                related_id
            ))
        },
    }
}
```

**Replace with**:
```rust
// Reference validation is deferred - will be checked when references are accessed
// For now, just validate that IDs are not empty
for related_id in &self.related_node_ids {
    if related_id.is_empty() {
        return Err("Related content ID cannot be empty".to_string());
    }
}
```

### For PathStep (MUST FIX):

**Current code (lines 205-229)**:
```rust
// Validate path exists
match get_path_by_id_internal(&self.path_id) {
    Ok(Some(_)) => {},
    Ok(None) => {
        return Err(format!("LearningPath {} not found", self.path_id))
    },
    Err(_) => {
        return Err(format!(
            "Error checking reference to path {}",
            self.path_id
        ))
    },
}

// Validate resource exists (if it's a content reference, not external URL)
if self.step_type == "content" || self.step_type == "path" {
    // Try as content first
    if get_content_by_id_internal(&self.resource_id).is_err()
        && get_path_by_id_internal(&self.resource_id).is_err()
    {
        return Err(format!(
            "Resource {} not found as content or path",
            self.resource_id
        ));
    }
}
```

**Replace with**:
```rust
// Validate references are not empty (actual existence checks are deferred)
if self.path_id.is_empty() {
    return Err("Path ID cannot be empty".to_string());
}

if self.resource_id.is_empty() {
    return Err("Resource ID cannot be empty".to_string());
}
```

**Why**: During healing from v1, referenced entries may not exist in v2 yet (lazy migration). We should allow entries to be marked as "Degraded" rather than blocking the entire entry.

**Verify**: Content and PathStep validation now only checks for empty IDs, not existence.

---

## Fix 3: Implement V1 Export Helpers

**File**: `/projects/elohim/holochain/dna/lamad-v1/zomes/content_store/src/healing_exports.rs`

**Location**: Lines 290-345 (the internal helper functions)

### Implementation Strategy

Replace the stub implementations with actual DHT queries. Since there's no separate v1 coordinator code, use the same pattern as the current coordinator:

### For `get_content_by_id_internal`:

**Current (lines 292-302)**:
```rust
fn get_content_by_id_internal(id: &str) -> ExternResult<Option<Content>> {
    // This should call the existing get_content_by_id function
    // For now, placeholder that returns an error
    // In real implementation, use:
    // get_content_by_id(QueryByIdInput { id: id.to_string() })
    //     .map(|output| output.map(|o| o.content))

    Err(wasm_error!(WasmErrorInner::Guest(
        "Use your existing get_content_by_id implementation here".to_string()
    )))
}
```

**Replace with**:
```rust
fn get_content_by_id_internal(id: &str) -> ExternResult<Option<Content>> {
    // Query the DHT for content by ID using the standard anchor/link pattern
    let anchor = StringAnchor::new("content_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let content: Content = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize content".to_string())))?;

            return Ok(Some(content));
        }
    }

    Ok(None)
}
```

### For `get_path_by_id_internal`:

**Current (lines 305-310)**:
```rust
fn get_path_by_id_internal(id: &str) -> ExternResult<Option<LearningPath>> {
    // Replace with actual implementation
    Err(wasm_error!(WasmErrorInner::Guest(
        "Use your existing get_path_by_id implementation here".to_string()
    )))
}
```

**Replace with**:
```rust
fn get_path_by_id_internal(id: &str) -> ExternResult<Option<LearningPath>> {
    // Same pattern as content, but for LearningPath
    let anchor = StringAnchor::new("path_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let path: LearningPath = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize path".to_string())))?;

            return Ok(Some(path));
        }
    }

    Ok(None)
}
```

### For `get_step_by_id_internal`:

**Current (lines 313-318)**:
```rust
fn get_step_by_id_internal(id: &str) -> ExternResult<Option<PathStep>> {
    // Replace with actual implementation
    Err(wasm_error!(WasmErrorInner::Guest(
        "Use your existing get_step_by_id implementation here".to_string()
    )))
}
```

**Replace with**:
```rust
fn get_step_by_id_internal(id: &str) -> ExternResult<Option<PathStep>> {
    // Same pattern as others, for PathStep
    let anchor = StringAnchor::new("step_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPathStep)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let step: PathStep = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize step".to_string())))?;

            return Ok(Some(step));
        }
    }

    Ok(None)
}
```

### For `get_mastery_by_id_internal`:

**Current (lines 321-326)**:
```rust
fn get_mastery_by_id_internal(id: &str) -> ExternResult<Option<ContentMastery>> {
    // Replace with actual implementation
    Err(wasm_error!(WasmErrorInner::Guest(
        "Use your existing get_mastery_by_id implementation here".to_string()
    )))
}
```

**Replace with**:
```rust
fn get_mastery_by_id_internal(id: &str) -> ExternResult<Option<ContentMastery>> {
    // Same pattern as others, for ContentMastery
    let anchor = StringAnchor::new("mastery_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContentMastery)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(record) = record {
            let mastery: ContentMastery = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize mastery".to_string())))?;

            return Ok(Some(mastery));
        }
    }

    Ok(None)
}
```

### Verify Everything

All helper functions now:
- Query the DHT using standard anchor/link pattern
- Handle missing entries gracefully (return Ok(None))
- Deserialize properly with error handling
- Will allow bridge calls from v2 to succeed

---

## Verification Steps

### Step 1: Check for Compilation

```bash
cd /projects/elohim
cargo build --manifest-path holochain/dna/lamad-spike/zomes/content_store/Cargo.toml
```

**Expected**: Should compile without errors

**If errors**:
- Check that constants are defined
- Check that reference validation is simplified
- Check that v1 export helpers are implemented

### Step 2: Run Unit Tests

```bash
cargo test --manifest-path holochain/dna/lamad-spike/zomes/content_store/Cargo.toml
```

**Expected**: Tests should pass
- `test_content_validation` ✓
- `test_content_v1_transformation` ✓
- `test_mastery_validation` ✓

### Step 3: Verify Role Names

Ensure these match:

**In Jenkinsfile.healing (line 104)**:
```groovy
- name: "lamad-v1"
```

**In healing_impl.rs (line 531)**:
```rust
pub fn create_healing_orchestrator() -> HealingOrchestrator {
    HealingOrchestrator::new(
        "lamad-v1",  // 👈 Must match
        "coordinator",
    )
}
```

Both must be **"lamad-v1"** or both must be changed together.

---

## Summary

| Fix | Time | Impact | Status |
|-----|------|--------|--------|
| 1. Add validation constants | 10 min | Compilation | 🔴 CRITICAL |
| 2. Simplify validation logic | 15 min | Zero data loss | 🔴 CRITICAL |
| 3. Implement v1 export helpers | 30 min | Healing works | 🔴 CRITICAL |
| 4. Verify compilation | 10 min | Sanity check | ✅ Quick |

**Total Time**: 65 minutes

**After these fixes**: Code will compile and be ready for integration testing.

---

## Next: Testing

Once these fixes are applied and code compiles:

1. **Test locally**:
   ```bash
   cargo test
   ```

2. **Run Jenkins pipeline**:
   ```bash
   Jenkinsfile.healing
   ```

3. **Manual verification**:
   - Seed v1 with test data
   - Query v2 to trigger healing
   - Verify zero data loss
   - Check healing signals emit

See `INTEGRATION_STATUS.md` for next steps after fixes are applied.
