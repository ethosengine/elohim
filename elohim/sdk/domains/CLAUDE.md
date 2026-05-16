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

Flow: **DNA coordinator zome ↔ shared types crate (`sdk/domains/{domain}/types/`, zero HDK deps) ↔ consumer (doorway, storage)**. The same crate runs `cargo test --features ts` to produce TypeScript bindings in `types/bindings/*.ts`, which `pnpm run wire-types:generate` copies into `@elohim/storage-client/wire-types/{domain}/` for elohim-app and the seeder. Two compilers, one source of truth.

The types crate:
- Defines input/output structs with `serde::{Serialize, Deserialize}`
- Uses `holo_hash::ActionHash` for action references (pinned to match DNA's hdk version)
- No dependency on HDK, HDI, or any WASM-specific crate
- MessagePack roundtrip tests (`rmp-serde`)
- Optional `ts` feature for TypeScript generation via `ts-rs`

The zome `pub use`s shared types and keeps integrity entry types local (they need `#[hdk_entry_helper]`), converting integrity → wire at construction sites. The consumer `pub use`s the same crate — no hand-copied structs.

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

`Cargo.toml`: `holo_hash = { version = "=0.6.0", features = ["encoding"] }`, `serde = { version = "1", features = ["derive"] }`, dev-dep `rmp-serde = "1"`, optional `ts-rs = "10"` behind a `ts` feature.

The crate's library root (each domain crate's own src/lib.rs): per-type input/output structs decorated with `#[derive(Debug, Clone, Serialize, Deserialize)] #[cfg_attr(feature = "ts", derive(ts_rs::TS))]`, plus a `{type}_msgpack_roundtrip` test per type that round-trips via `rmp_serde::{to_vec, from_slice}`.

## Rules

- **`holo_hash` version pinned to the DNA workspace.** Pin `holo_hash = "=0.6.0"` because `hdk = "=0.6.0"` resolves to `holo_hash 0.6.0`; if these diverge `ActionHash` becomes two different Rust types and the zome won't compile. Consumers on a different version (doorway uses `0.7.0-dev.3`) end up with two versions in their dep tree — fine, Cargo handles it.
- **No HDK/HDI deps.** The crate must compile for `wasm32-unknown-unknown` (zome) and native targets (doorway, storage). If a zome needs an HDK type it converts at the construction site.
- **Optional fields pair `#[serde(default)]` with `skip_serializing_if`** — MessagePack map serialization skips None on write, and without `default` the deserializer fails on the missing key.
- **Wire types mirror integrity entry types field-for-field.** The zome converts at construction sites (`imagodei_types::Human { id: entry.id, display_name: entry.display_name, ... }`). Intentional boilerplate — the price of keeping HDK out, and field mismatches stay compile errors.
- **One MessagePack roundtrip test per type** catches serde-attribute issues before they hit the conductor.

## Existing Implementations

| Domain | Crate | Types | Tests | Zome Wired | Doorway Wired | TS Generated |
|--------|-------|-------|-------|------------|---------------|--------------|
| imagodei | `imagodei/types/` | 3 | 2 | imagodei | zome_helpers.rs | 3 files |
| infrastructure | `infrastructure/types/` | 16 | 4 | infrastructure | federation.rs | 15 files |
| qahal | `qahal/types/` | 40 | 10 | mishpat | N/A | 43 files |
| lamad | `lamad/types/` | 54 | 13 | content_store | N/A | 56 files |
| shefa | `shefa/types/` | 30+ | 9 | content_store | N/A | 35 files |
| avodah | `avodah/types/` | 30+ | 9 | content_store | N/A | 40 files |

## Build Commands

```bash
# Build a domain types crate
cd elohim/sdk/domains/{domain}/types && cargo check

# Test MessagePack roundtrips
cd elohim/sdk/domains/{domain}/types && cargo test

# Generate TypeScript from a single domain
cd elohim/sdk/domains/{domain}/types && RUSTFLAGS="" cargo test --features ts

# Generate TypeScript for ALL domains and copy into @elohim/storage-client
pnpm run wire-types:generate

# Verify zome still compiles with shared types
cd elohim/holochain/dna/{dna}
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown

# Verify doorway still compiles
cd doorway/doorway-service && RUSTFLAGS="" cargo check
```

## TypeScript Integration

Wire types are distributed via the `@elohim/storage-client` npm package:

```typescript
// Per-domain import (coordinator I/O types, snake_case)
import { CreateHumanInput, Human, HumanOutput } from '@elohim/storage-client/wire-types/imagodei';
import { CreateChallengeInput, Challenge } from '@elohim/storage-client/wire-types/qahal';
import { CreateContentInput, Content } from '@elohim/storage-client/wire-types/lamad';

// Namespace import (all domains)
import { lamad, shefa, qahal } from '@elohim/storage-client/wire-types';
```

These sit alongside the existing HTTP API types:

```typescript
// HTTP API types (camelCase, from views.rs)
import { ContentView, HumanView } from '@elohim/storage-client/generated';

// Coordinator wire types (snake_case, from domain types crates)
import { Content, Human } from '@elohim/storage-client/wire-types/lamad';
```

The HTTP types represent what elohim-storage serves over JSON. The wire types
represent what the conductor exchanges over MessagePack. Both derive from the
same domain — the compiler enforces both.
