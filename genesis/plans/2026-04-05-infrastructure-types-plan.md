# Infrastructure Wire Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `infrastructure-types` crate at `elohim/sdk/domains/infrastructure/types/` with wire types for the infrastructure DNA coordinator, then wire it into both the infrastructure zome and doorway's federation service — eliminating hand-copied types.

**Architecture:** Same pattern as imagodei-types (see `elohim/sdk/domains/CLAUDE.md`). The infrastructure DNA handles doorway registration, heartbeats, health attestations, and content server discovery. Doorway's `federation.rs` currently hand-copies 6 input/output structs from this zome — those become imports from the shared crate.

**Tech Stack:** Rust, serde, holo_hash (=0.6.0), rmp-serde

**Parallel Safety:** This plan touches the infrastructure DNA and doorway. It does NOT touch the elohim DNA or any other zome. Safe to run in parallel with lamad-types, shefa-types, avodah-types, and qahal-types plans.

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `elohim/sdk/domains/infrastructure/types/Cargo.toml` | Create | Crate manifest |
| `elohim/sdk/domains/infrastructure/types/src/lib.rs` | Create | Wire type definitions |
| `elohim/holochain/dna/infrastructure/zomes/infrastructure/Cargo.toml` | Modify | Depend on infrastructure-types |
| `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs` | Modify | Replace local types with re-exports |
| `doorway/doorway-service/Cargo.toml` | Modify | Depend on infrastructure-types |
| `doorway/doorway-service/src/services/federation.rs` | Modify | Replace hand-copied types with imports |

---

### Task 1: Create infrastructure domain directory and types crate

**Files:**
- Create: `elohim/sdk/domains/infrastructure/types/Cargo.toml`
- Create: `elohim/sdk/domains/infrastructure/types/src/lib.rs`

**Note:** The `elohim/sdk/domains/infrastructure/` directory does not exist yet. Create it.

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p elohim/sdk/domains/infrastructure/types/src
```

- [ ] **Step 2: Create Cargo.toml**

Create `elohim/sdk/domains/infrastructure/types/Cargo.toml`:

```toml
[package]
name = "infrastructure-types"
version = "0.1.0"
edition = "2021"
description = "Wire types for infrastructure domain coordinator functions"

[dependencies]
holo_hash = { version = "=0.6.0", features = ["encoding"] }
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

- [ ] **Step 3: Create src/lib.rs with wire types**

Create `elohim/sdk/domains/infrastructure/types/src/lib.rs`:

```rust
//! Wire types for infrastructure domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! infrastructure zome calls. Consumed by:
//! - The infrastructure coordinator zome (WASM target)
//! - Doorway gateway service — federation.rs (native target)

use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};

// =============================================================================
// Doorway Registration Types
// =============================================================================

/// Input for infrastructure::register_doorway coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RegisterDoorwayInput {
    pub id: String,
    pub url: String,
    pub capabilities_json: String,
    pub reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
    pub version: String,
}

/// DoorwayRegistration entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoorwayRegistration {
    pub id: String,
    pub url: String,
    pub operator_agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_human: Option<String>,
    pub capabilities_json: String,
    pub reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
    pub version: String,
    pub tier: String,
    pub registered_at: String,
    pub updated_at: String,
}

/// Output from doorway registration operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DoorwayOutput {
    pub action_hash: ActionHash,
    pub doorway: DoorwayRegistration,
}

// =============================================================================
// Heartbeat & Health Types
// =============================================================================

/// Input for infrastructure::record_heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecordHeartbeatInput {
    pub doorway_id: String,
    pub status: String,
    pub uptime_ratio: f32,
    pub active_connections: u32,
    pub content_served: u64,
}

/// Input for infrastructure::record_daily_summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecordSummaryInput {
    pub doorway_id: String,
    pub date: String,
    pub uptime_ratio: f32,
    pub total_content_served: u64,
    pub peak_connections: u32,
    pub heartbeat_count: u32,
}

/// Input for infrastructure::record_health_attestation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecordHealthAttestationInput {
    pub attestor_doorway_id: String,
    pub subject_doorway_id: String,
    pub observed_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductor_healthy: Option<bool>,
}

/// HealthAttestation entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HealthAttestation {
    pub attestor_doorway_id: String,
    pub operator_agent: String,
    pub subject_doorway_id: String,
    pub observed_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductor_healthy: Option<bool>,
    pub timestamp: i64,
}

/// Output from health attestation queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HealthAttestationOutput {
    pub action_hash: String,
    pub entry_hash: String,
    pub attestation: HealthAttestation,
    pub author: String,
}

// =============================================================================
// Content Server Types
// =============================================================================

/// Input for infrastructure::register_content_server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RegisterContentServerInput {
    pub content_hash: String,
    pub capability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<StorageEndpointInput>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
}

/// Storage endpoint input (used in RegisterContentServerInput).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StorageEndpointInput {
    pub url: String,
    pub protocol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
}

/// Storage endpoint entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StorageEndpoint {
    pub url: String,
    pub protocol: String,
    pub priority: u8,
}

/// ContentServer entry fields (mirrors integrity zome).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentServer {
    pub content_hash: String,
    pub capability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_url: Option<String>,
    pub endpoints: Vec<StorageEndpoint>,
    pub online: bool,
    pub priority: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<u32>,
    pub registered_at: u64,
    pub last_heartbeat: u64,
}

/// Output from content server operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentServerOutput {
    pub action_hash: ActionHash,
    pub server: ContentServer,
}

/// Input for infrastructure::find_publishers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FindPublishersInput {
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer_region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub online_only: Option<bool>,
}

/// Output from infrastructure::find_publishers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FindPublishersOutput {
    pub content_hash: String,
    pub publishers: Vec<ContentServerOutput>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_doorway_input_msgpack_roundtrip() {
        let input = RegisterDoorwayInput {
            id: "doorway-1".to_string(),
            url: "https://doorway.example.com".to_string(),
            capabilities_json: r#"{"bootstrap":true}"#.to_string(),
            reach: "commons".to_string(),
            region: Some("us-east".to_string()),
            bandwidth_mbps: Some(100),
            version: "1.0.0".to_string(),
        };
        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: RegisterDoorwayInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "doorway-1");
        assert_eq!(decoded.region, Some("us-east".to_string()));
    }

    #[test]
    fn record_heartbeat_input_msgpack_roundtrip() {
        let input = RecordHeartbeatInput {
            doorway_id: "doorway-1".to_string(),
            status: "healthy".to_string(),
            uptime_ratio: 0.99,
            active_connections: 42,
            content_served: 1024,
        };
        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: RecordHeartbeatInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.doorway_id, "doorway-1");
        assert_eq!(decoded.active_connections, 42);
    }

    #[test]
    fn record_health_attestation_input_msgpack_roundtrip() {
        let input = RecordHealthAttestationInput {
            attestor_doorway_id: "doorway-1".to_string(),
            subject_doorway_id: "doorway-2".to_string(),
            observed_status: "healthy".to_string(),
            response_time_ms: Some(45),
            conductor_healthy: Some(true),
        };
        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: RecordHealthAttestationInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.attestor_doorway_id, "doorway-1");
        assert_eq!(decoded.response_time_ms, Some(45));
    }

    #[test]
    fn find_publishers_input_msgpack_roundtrip() {
        let input = FindPublishersInput {
            content_hash: "bafkrei123".to_string(),
            capability: Some("serve".to_string()),
            prefer_region: None,
            limit: Some(10),
            online_only: Some(true),
        };
        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: FindPublishersInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.content_hash, "bafkrei123");
        assert_eq!(decoded.limit, Some(10));
    }
}
```

- [ ] **Step 4: Verify the crate builds and tests pass**

Run: `cd elohim/sdk/domains/infrastructure/types && cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd elohim/sdk/domains/infrastructure/types && cargo test 2>&1 | tail -5`
Expected: 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/infrastructure/
git commit -m "feat(infrastructure): create wire types crate in sdk/domains/infrastructure/types/

Wire types for doorway registration, heartbeats, health attestations,
and content server discovery. Zero HDK deps, consumable by zome and doorway."
```

---

### Task 2: Wire infrastructure coordinator zome to use infrastructure-types

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/Cargo.toml`
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`

- [ ] **Step 1: Read the zome's current lib.rs**

Read the full file to find all struct definitions and HumanOutput-style construction sites.

- [ ] **Step 2: Add infrastructure-types dependency to coordinator zome**

In `elohim/holochain/dna/infrastructure/zomes/infrastructure/Cargo.toml`, add under `[dependencies]`:

```toml
infrastructure-types = { path = "../../../../../sdk/domains/infrastructure/types" }
```

Path: `zomes/infrastructure/` → `dna/infrastructure/` → `holochain/` → `elohim/` → `sdk/domains/infrastructure/types/`

- [ ] **Step 3: Replace local type definitions with re-exports**

In `lib.rs`, replace each locally-defined input/output struct with a re-export:

```rust
pub use infrastructure_types::{
    RegisterDoorwayInput, DoorwayOutput,
    RecordHeartbeatInput, RecordSummaryInput,
    RecordHealthAttestationInput, HealthAttestationOutput,
    RegisterContentServerInput, StorageEndpointInput, ContentServerOutput,
    FindPublishersInput, FindPublishersOutput,
};
```

Keep integrity entry types (`DoorwayRegistration`, `HealthAttestation`, `ContentServer`, `StorageEndpoint`) from the integrity zome — they use `#[hdk_entry_helper]`.

- [ ] **Step 4: Fix construction sites**

At each site where `DoorwayOutput`, `ContentServerOutput`, or `HealthAttestationOutput` is constructed, convert the integrity entry type to the wire type field-by-field (same pattern as imagodei — see `elohim/sdk/domains/CLAUDE.md`).

- [ ] **Step 5: Verify zome builds for WASM target**

```bash
cd elohim/holochain/dna/infrastructure
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown -p infrastructure 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure/
git commit -m "refactor(infrastructure): use sdk wire types in coordinator zome

Re-export input/output types from infrastructure-types crate.
Convert integrity entry types to wire types at construction sites."
```

---

### Task 3: Wire doorway federation service to use infrastructure-types

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml`
- Modify: `doorway/doorway-service/src/services/federation.rs`

- [ ] **Step 1: Read federation.rs**

Read the full file to find all hand-copied types and their usage sites.

- [ ] **Step 2: Add infrastructure-types dependency to doorway**

In `doorway/doorway-service/Cargo.toml`, add:

```toml
infrastructure-types = { path = "../../elohim/sdk/domains/infrastructure/types" }
```

- [ ] **Step 3: Replace hand-copied types with imports**

In `federation.rs`, replace the hand-copied struct definitions with:

```rust
pub use infrastructure_types::{
    RegisterDoorwayInput, DoorwayOutput, DoorwayRegistration,
    RecordHeartbeatInput, RecordHealthAttestationInput,
    FindPublishersInput,
};
```

**IMPORTANT:** The doorway's `DoorwayOutput` currently has `action_hash: Vec<u8>` while the shared type has `action_hash: ActionHash`. Verify that federation.rs doesn't construct `DoorwayOutput` locally — it should only deserialize it from the conductor. If it does construct it, check whether `Vec<u8>` was a workaround and the ActionHash version is correct.

Also keep `PeerDoorway` and any doorway-only types (these are NOT zome types).

- [ ] **Step 4: Verify doorway builds and tests pass**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo check 2>&1 | tail -5
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/Cargo.toml doorway/doorway-service/src/services/federation.rs
git commit -m "refactor(doorway): use sdk/domains/infrastructure/types for federation

Compiler enforces type agreement between infrastructure zome and
doorway federation service. No more hand-copied types."
```

---

### Task 4: Final verification

- [ ] **Step 1: Build all consumers**

```bash
cd elohim/sdk/domains/infrastructure/types && cargo test
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins
cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings
```

- [ ] **Step 2: cargo fmt all modified crates**

```bash
cd elohim/sdk/domains/infrastructure/types && cargo fmt
cd elohim/holochain/dna/infrastructure/zomes/infrastructure && cargo fmt
cd doorway/doorway-service && cargo fmt
```

- [ ] **Step 3: Commit if needed**
