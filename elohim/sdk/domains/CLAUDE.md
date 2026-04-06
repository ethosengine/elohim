# SDK Domain Wire Types — Pattern Guide

## What This Is

Each domain directory can contain a `types/` Rust crate that defines the
MessagePack-serialized inputs and outputs for coordinator zome functions.
Both the DNA coordinator zome and its consumers (doorway, storage, clients)
depend on this crate. The compiler catches type mismatches at build time
instead of runtime deserialization failures.

## Why This Exists

Before shared types, each consumer hand-copied structs from the zome source.
Fields drifted silently — `Vec<u8>` vs `ActionHash`, missing fields, renamed
fields — producing runtime errors like `missing field 'action_hash'` that
cost hours to debug. Shared types make drift impossible.

## The Pattern

```
DNA coordinator zome                    Consumer (doorway, storage, etc.)
        │                                        │
        └──── both depend on ─────┐──────────────┘
                                  │
                    sdk/domains/{domain}/types/
                    (Cargo crate, zero HDK deps)
```

The types crate:
- Defines input/output structs with `serde::{Serialize, Deserialize}`
- Uses `holo_hash::ActionHash` for action references (pinned to match DNA's hdk version)
- Has NO dependency on HDK, HDI, or any WASM-specific crate
- Includes MessagePack roundtrip tests (`rmp-serde`)
- Optional `ts` feature for TypeScript generation via `ts-rs`

The zome:
- `pub use {domain}_types::CreateFooInput;` (re-exports shared types)
- Keeps integrity entry types local (they need `#[hdk_entry_helper]`)
- Converts integrity types → wire types at construction sites (field-by-field)

The consumer:
- `pub use {domain}_types::{CreateFooInput, FooOutput};`
- No hand-copied structs, no comment saying "must match zome"

## Domain-to-DNA Mapping

Not every domain maps 1:1 to a DNA. Multiple domains can source types from
the same DNA coordinator:

| Domain | DNA | Coordinator Zome | Types Scope |
|--------|-----|------------------|-------------|
| imagodei | imagodei | imagodei | Identity: Human, Attestation, Relationship, Mastery, Presence |
| lamad | elohim | content_store | Content: Content, Path, Step, Chapter, Relationship, Progress |
| shefa | elohim | content_store | Economics: Agreement, Commitment, EconomicEvent, PremiumGate |
| qahal | mishpat | mishpat | Governance: Challenge, Proposal, Precedent, Discussion, Voting |
| avodah | elohim | content_store | Work: ServiceRequest, ServiceOffer, FlowPlan, Insurance |

When multiple domain types crates source from the same DNA, each crate
contains only the types relevant to its domain. The zome re-exports from
whichever domain crate owns each type.

## Template

### Cargo.toml

```toml
[package]
name = "{domain}-types"
version = "0.1.0"
edition = "2021"
description = "Wire types for {domain} domain coordinator functions"

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

### src/lib.rs

```rust
//! Wire types for {domain} domain coordinator functions.

use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};

/// Input for {domain}::create_foo coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateFooInput {
    pub id: String,
    // ... fields matching coordinator function input
}

/// Output from {domain}::create_foo coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FooOutput {
    pub action_hash: ActionHash,
    pub foo: Foo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo_msgpack_roundtrip() {
        let input = CreateFooInput { id: "test".into() };
        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: CreateFooInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "test");
    }
}
```

## Rules

### holo_hash version MUST match the DNA workspace

The types crate pins `holo_hash = "=0.6.0"` because the DNA workspace uses
`hdk = "=0.6.0"` which resolves to `holo_hash 0.6.0`. If these versions
diverge, `ActionHash` becomes two different Rust types and the zome won't
compile.

Consumers using a different holo_hash version (e.g., doorway uses 0.7.0-dev.3)
will have two versions in their dep tree. This is fine — Cargo handles it,
and serde deserializes ActionHash identically across versions.

### No HDK/HDI dependencies

The types crate must compile for both `wasm32-unknown-unknown` (zome) and
native targets (doorway, storage). HDK/HDI are WASM-only. If you need an
HDK type, the zome converts at the construction site.

### Optional fields need `#[serde(default)]` with `skip_serializing_if`

When using `#[serde(skip_serializing_if = "Option::is_none")]`, always pair
it with `#[serde(default)]`. MessagePack map serialization skips None fields
on write; without `default`, the deserializer fails on the missing key.

### Wire types mirror integrity entry types field-for-field

The wire `Human` has the same fields as the integrity `Human`. The zome
converts between them at each construction site:

```rust
Ok(HumanOutput {
    action_hash,
    human: imagodei_types::Human {
        id: entry.id,
        display_name: entry.display_name,
        // ... all fields
    },
})
```

This is intentional boilerplate. It's the price of keeping HDK out of the
types crate, and it makes field mismatches a compile error.

### One test per type: MessagePack roundtrip

Every input and output type gets a `{type}_msgpack_roundtrip` test that
serializes to bytes and deserializes back. This catches serde attribute
issues (missing `default`, wrong `skip_serializing_if`, etc.) before they
hit the conductor.

## Existing Implementations

| Domain | Status | Crate Path |
|--------|--------|------------|
| imagodei | Done | `elohim/sdk/domains/imagodei/types/` |
| lamad | Planned | `elohim/sdk/domains/lamad/types/` |
| shefa | Planned | `elohim/sdk/domains/shefa/types/` |
| qahal | Planned | `elohim/sdk/domains/qahal/types/` |
| avodah | Planned | `elohim/sdk/domains/avodah/types/` |
| infrastructure | Planned | `elohim/sdk/domains/infrastructure/types/` (doorway federation) |

## Build Commands

```bash
# Build a domain types crate
cd elohim/sdk/domains/{domain}/types && cargo check

# Test MessagePack roundtrips
cd elohim/sdk/domains/{domain}/types && cargo test

# Verify zome still compiles with shared types
cd elohim/holochain/dna/{dna}
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown

# Verify doorway still compiles
cd doorway/doorway-service && RUSTFLAGS="" cargo check
```
