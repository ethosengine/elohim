# Per-Domain Wire Types — imagodei Proof of Concept

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `imagodei-types` crate at `elohim/sdk/domains/imagodei/types/` that owns wire types (coordinator function inputs/outputs), consumed by both the imagodei zome and doorway — so the compiler catches type mismatches instead of runtime deserialization failures.

**Architecture:** SDK domain directories (`elohim/sdk/domains/{domain}/`) own all IoC exportables: schemas, manifests, and now wire types. The `types/` crate is the next logical artifact alongside the existing `schemas/` and `manifest.json`. DNAs and doorway both consume these types. The compiler enforces agreement.

**Tech Stack:** Rust, serde, holo_hash (0.7.0-dev.3), ts-rs (optional feature)

---

## Directory Structure

The SDK domain directories already exist with schemas and manifests. Types completes the picture:

```
elohim/sdk/domains/
  imagodei/
    CLAUDE.md                      ← (exists)
    manifest.json                  ← domain vocabulary (exists)
    schemas/                       ← JSON schemas (exists)
      human-metadata.schema.json
      presence-metadata.schema.json
    scripts/                       ← codegen scripts (exists)
    types/                         ← Rust wire types (NEW)
      Cargo.toml
      src/lib.rs
  lamad/
    manifest.json                  ← (exists)
    schemas/                       ← (exists)
    types/                         ← (future)
  qahal/                           ← (exists, future types/)
  shefa/                           ← (exists, future types/)
  avodah/                          ← (exists, future types/)
```

Consumers:
- `elohim/holochain/dna/imagodei/zomes/imagodei/` depends on `imagodei-types`
- `doorway/doorway-service/` depends on `imagodei-types`
- Future: codegen reads `types/` + `schemas/` to produce TypeScript

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `elohim/sdk/domains/imagodei/types/Cargo.toml` | Create | Crate manifest — minimal deps |
| `elohim/sdk/domains/imagodei/types/src/lib.rs` | Create | Wire type definitions |
| `elohim/holochain/dna/imagodei/zomes/imagodei/Cargo.toml` | Modify | Depend on imagodei-types |
| `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` | Modify | Replace local types with re-exports |
| `doorway/doorway-service/Cargo.toml` | Modify | Depend on imagodei-types |
| `doorway/doorway-service/src/routes/zome_helpers.rs` | Modify | Replace hand-copied types with imports |

---

### Task 1: Create imagodei-types crate

**Files:**
- Create: `elohim/sdk/domains/imagodei/types/Cargo.toml`
- Create: `elohim/sdk/domains/imagodei/types/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

Create `elohim/sdk/domains/imagodei/types/Cargo.toml`:

```toml
[package]
name = "imagodei-types"
version = "0.1.0"
edition = "2021"
description = "Wire types for imagodei domain coordinator functions"

[dependencies]
holo_hash = { version = "0.7.0-dev.3", features = ["encoding"] }
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
rmp-serde = "1"

[features]
default = []
ts = ["dep:ts-rs"]

[dependencies.ts-rs]
version = "10"
optional = true
```

- [ ] **Step 2: Create src/lib.rs with wire types**

Create `elohim/sdk/domains/imagodei/types/src/lib.rs`:

```rust
//! Wire types for imagodei domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! imagodei zome calls. They are consumed by:
//! - The imagodei coordinator zome (WASM target)
//! - Doorway gateway service (native target)
//! - Any future client that calls imagodei functions
//!
//! This crate is an IoC artifact in `sdk/domains/imagodei/`, alongside
//! the domain's schemas and manifest. It must NOT depend on HDK, HDI,
//! or any WASM-specific crates.

use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};

// =============================================================================
// Human Profile Types
// =============================================================================

/// Input for imagodei::create_human coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateHumanInput {
    pub id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Human profile fields.
///
/// Matches the integrity zome's Human entry type field-for-field.
/// The integrity zome wraps this with `#[hdk_entry_helper]` for DHT storage;
/// this version uses plain serde for wire format compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Human {
    pub id: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    pub location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from imagodei::create_human coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HumanOutput {
    pub action_hash: ActionHash,
    pub human: Human,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_human_input_msgpack_roundtrip() {
        let input = CreateHumanInput {
            id: "test-123".to_string(),
            display_name: "Test User".to_string(),
            bio: Some("A test user".to_string()),
            affinities: vec!["testing".to_string()],
            profile_reach: "public".to_string(),
            location: None,
        };

        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: CreateHumanInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-123");
        assert_eq!(decoded.display_name, "Test User");
    }

    #[test]
    fn human_msgpack_roundtrip() {
        let human = Human {
            id: "test-456".to_string(),
            display_name: "Another User".to_string(),
            bio: None,
            affinities: vec![],
            profile_reach: "community".to_string(),
            location: Some("Earth".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let bytes = rmp_serde::to_vec(&human).unwrap();
        let decoded: Human = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test-456");
        assert_eq!(decoded.location, Some("Earth".to_string()));
    }
}
```

- [ ] **Step 3: Verify the crate builds and tests pass**

Run: `cd elohim/sdk/domains/imagodei/types && cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd elohim/sdk/domains/imagodei/types && cargo test 2>&1 | tail -5`
Expected: 2 tests pass

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/domains/imagodei/types/
git commit -m "feat(imagodei): create wire types crate in sdk/domains/imagodei/types/

Wire types (CreateHumanInput, Human, HumanOutput) in a thin crate with
zero HDK deps alongside existing schemas and manifest. Consumable by
both WASM zomes and native doorway."
```

---

### Task 2: Wire imagodei coordinator zome to use imagodei-types

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/Cargo.toml`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`

- [ ] **Step 1: Add imagodei-types dependency to coordinator zome**

In `elohim/holochain/dna/imagodei/zomes/imagodei/Cargo.toml`, add under `[dependencies]`:

```toml
imagodei-types = { path = "../../../../sdk/domains/imagodei/types" }
```

Path: `zomes/imagodei/` → `dna/imagodei/` → `holochain/` → `elohim/` → `sdk/domains/imagodei/types/`

- [ ] **Step 2: Replace local type definitions with re-exports**

In `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`:

Replace `CreateHumanInput` definition (lines 24-33):
```rust
/// Input for creating/updating a Human profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHumanInput {
    pub id: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    pub location: Option<String>,
}
```

With:
```rust
pub use imagodei_types::CreateHumanInput;
```

Replace `HumanOutput` definition (lines 50-55):
```rust
/// Output from profile operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanOutput {
    pub action_hash: ActionHash,
    pub human: Human,
}
```

With:
```rust
pub use imagodei_types::HumanOutput;
```

- [ ] **Step 3: Fix HumanOutput construction sites**

`HumanOutput.human` now expects `imagodei_types::Human`, but the zome constructs it from `imagodei_integrity::Human`. Convert field-by-field at each construction site:

```bash
grep -n 'HumanOutput {' elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
```

At each site:
```rust
// Before:
Ok(HumanOutput { action_hash, human: human_entry })

// After:
Ok(HumanOutput {
    action_hash,
    human: imagodei_types::Human {
        id: human_entry.id,
        display_name: human_entry.display_name,
        bio: human_entry.bio,
        affinities: human_entry.affinities,
        profile_reach: human_entry.profile_reach,
        location: human_entry.location,
        created_at: human_entry.created_at,
        updated_at: human_entry.updated_at,
    },
})
```

- [ ] **Step 4: Verify zome builds for WASM target**

```bash
cd elohim/holochain/dna/imagodei
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown -p imagodei 2>&1 | tail -10
```

Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei/
git commit -m "refactor(imagodei): use sdk/domains/imagodei/types for wire types in zome

CreateHumanInput and HumanOutput re-exported from imagodei-types.
Zome functions convert between integrity Human and wire Human."
```

---

### Task 3: Wire doorway to use imagodei-types

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml`
- Modify: `doorway/doorway-service/src/routes/zome_helpers.rs`

- [ ] **Step 1: Add imagodei-types dependency to doorway**

In `doorway/doorway-service/Cargo.toml`, add:

```toml
imagodei-types = { path = "../../elohim/sdk/domains/imagodei/types" }
```

- [ ] **Step 2: Replace hand-copied types with imports**

In `doorway/doorway-service/src/routes/zome_helpers.rs`, replace the imports and type definitions:

Replace:
```rust
use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::server::AppState;
use crate::types::{DoorwayError, Result};
use crate::worker::ZomeCallConfig;

// =============================================================================
// Imagodei Zome Types
// =============================================================================

/// Input for imagodei::create_human zome call
...entire CreateHumanInput struct...

/// Output from imagodei::create_human
...entire HumanOutput struct...

/// Human entry from the zome
...entire Human struct...
```

With:
```rust
use tracing::{debug, warn};

use crate::server::AppState;
use crate::types::{DoorwayError, Result};
use crate::worker::ZomeCallConfig;

// Wire types from SDK domain crate — compiler enforces zome/doorway agreement
pub use imagodei_types::{CreateHumanInput, Human, HumanOutput};
```

- [ ] **Step 3: Verify doorway builds and tests pass**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo check 2>&1 | tail -5
RUSTFLAGS="" cargo test --lib 2>&1 | tail -5
```

Expected: compiles, all tests pass

- [ ] **Step 4: Prove compiler catches mismatches**

Temporarily rename `display_name` to `BROKEN` in `elohim/sdk/domains/imagodei/types/src/lib.rs`. Verify both doorway and zome fail to compile. Revert.

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/Cargo.toml doorway/doorway-service/src/routes/zome_helpers.rs
git commit -m "refactor(doorway): use sdk/domains/imagodei/types for wire types

Compiler enforces type agreement between zome and doorway. No more
hand-copied types or runtime deserialization surprises."
```

---

### Task 4: Final verification

- [ ] **Step 1: Build all three consumers**

```bash
cd elohim/sdk/domains/imagodei/types && cargo test
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib
cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings
```

- [ ] **Step 2: cargo fmt all modified crates**

```bash
cd elohim/sdk/domains/imagodei/types && cargo fmt
cd elohim/holochain/dna/imagodei/zomes/imagodei && cargo fmt
cd doorway/doorway-service && cargo fmt
```

- [ ] **Step 3: Commit if needed**

---

## Pattern Established

Each SDK domain directory now has a complete IoC surface:

```
elohim/sdk/domains/{domain}/
  manifest.json     ← domain vocabulary
  schemas/          ← JSON schemas
  types/            ← Rust wire types (Cargo crate)
```

DNAs consume types. Doorway consumes types. Codegen reads types. The compiler enforces it all.
