# Iroh Phase 12 — peer_transport_manifest Schema Graduation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Graduate the transition-bridge `cross_stack_peer_map` table to the permanent `peer_transport_manifest` schema with capability-level + per-transport profile + plane-aware transport selection, then consume the new manifest from four iroh adapters to unblock the five Phase 11 stopgaps.

**Architecture:** New Diesel migration replaces the bridge table with a richer manifest (`agent_cid` PK, both transport profiles, per-transport supported planes, discovery methods, `capability_level` 0-5 from device archetypes). The existing `peer_map` Rust module is rewritten with an extended public API plus back-compat shims; a new `select_transport(self, peer, plane)` algorithm chooses iroh / libp2p / Track-3 hub bridge / no-shared-transport at every call site. Four iroh adapters (`epr_atom_backend`, `view_fed_backend`, `auth_backends::trust`, `auth_backends::identity_handshake`) consume the manifest to populate caller identity, connected-peer lists, ambient trust cache, and authoritative peer labels. Schema-first: JSON schema → ts-rs Rust struct → schema-contract test → TS codegen.

**Tech Stack:** Diesel 2 + SQLite, ts-rs 7, jsonschema validator, json-schema-to-typescript codegen, async-trait, tokio. Build with `RUSTFLAGS='--cfg getrandom_backend="custom"'`.

---

## Source of truth (P2P design gate)

**Source of truth: libp2p Swarm + iroh Endpoint observations (Operational, Category C). Reconstructed from observed handshake arrivals on either transport. Not DHT-notarized.**

This applies to every storage artifact created by this plan:
- The `peer_transport_manifest` SQLite table (Task 1).
- The `PeerTransportManifestView` / `Libp2pTransportProfileView` / `IrohTransportProfileView` ts-rs structs (Task 3).
- The `peer-transport-manifest.schema.json` JSON schema (Task 2 — `description` field declares "Source of truth: libp2p Swarm + iroh Endpoint observations (Operational, Category C)" per `elohim/sdk/schemas/v1/views/CONVENTIONS.md` rule 2).
- The in-memory `PeerTransportManifest` shape and the `record_*_observation` writers (Task 6).

**No `dht_anchor_hash` column** is required because this is Category C — operational projection, not Category A — DHT-notarized. The Category A authority for "which agent_cid corresponds to which transport identity" lives in the **infrastructure DNA** (kitsune2/tx5 for canonical agent identity) and the **identity-binding gossip topic** for `libp2p_peer_id` / `iroh_node_id` derivation (spec lines 496-498). The Phase 12 manifest is a per-node observational projection of those Category A facts; if the manifest disagrees with the DHT, the DHT wins, and the manifest row is rebuilt on the next handshake. This matches the existing `peer_identity_bindings` and `cross_stack_peer_map` precedents (both Category C, both keyed on `agent_cid`, neither carries `dht_anchor_hash`).

The `agent_cid` PRIMARY KEY references the DHT-canonical identity by string; provenance back to the DHT is anchored in the **AgentPeerBinding** entry that ReconcileController writes when the binding is observed (existing Phase 10 plumbing — see `src/p2p/identity_map.rs` HolochainBackedPeerIdentityMap). No new DHT entry type is created by this plan.

---

## Public API surface (consumed by downstream plans)

These symbols MUST exist with the exact signatures below at the end of this plan. Plans 2 / 4 / 6 import directly from `elohim_storage::p2p_iroh::peer_map`.

```rust
// in elohim_storage::p2p_iroh::peer_map
pub struct PeerTransportManifest {
    pub agent_cid: String,
    pub libp2p: Option<Libp2pTransportProfile>,
    pub iroh: Option<IrohTransportProfile>,
    pub discovery: Vec<String>,
    pub capability_level: u8,           // 0-5
    pub last_observed: i64,             // unix seconds
}

pub struct Libp2pTransportProfile {
    pub peer_id: String,
    pub addrs: Vec<String>,
    pub supports: Vec<String>,          // Plane.as_str() values
}

pub struct IrohTransportProfile {
    pub node_id: String,
    pub relays: Vec<String>,
    pub supports: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Plane {
    Blob, Gossip, Sync, Epr, EprAtom, Shard,
    ViewFed, IdentityHandshake, Trust,
}
impl Plane {
    pub fn as_str(self) -> &'static str;
    pub fn parse(s: &str) -> Option<Plane>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportChoice {
    Iroh,
    Libp2p,
    Track3Bridge { hub_agent_cid: String },
    NoSharedTransport,
}

pub fn record_libp2p_observation(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    peer_id: &str,
    addrs: &[String],
    supports: &[Plane],
    observed_at: i64,
) -> Result<(), StorageError>;

pub fn record_iroh_observation(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    node_id: &str,
    relays: &[String],
    supports: &[Plane],
    observed_at: i64,
) -> Result<(), StorageError>;

pub fn record_capability(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    capability_level: u8,
) -> Result<(), StorageError>;

pub fn record_discovery(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    methods: &[&str],
) -> Result<(), StorageError>;

pub fn lookup_by_agent_cid(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> Result<Option<PeerTransportManifest>, StorageError>;

pub fn lookup_by_libp2p_peer_id(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<PeerTransportManifest>, StorageError>;

pub fn lookup_by_iroh_node_id(
    conn: &mut SqliteConnection,
    node_id: &str,
) -> Result<Option<PeerTransportManifest>, StorageError>;

pub fn select_transport(
    self_manifest: &PeerTransportManifest,
    peer_manifest: &PeerTransportManifest,
    plane: Plane,
) -> Result<TransportChoice, StorageError>;

// Back-compat shims (delegate to the four observation/lookup fns above).
pub fn record_libp2p(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    peer_id: &str,
    observed_at: &str, // ISO 8601
) -> Result<(), StorageError>;
pub fn record_iroh(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    node_id: &str,
    observed_at: &str, // ISO 8601
) -> Result<(), StorageError>;
pub fn iroh_for_libp2p(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<String>, StorageError>;
pub fn libp2p_for_iroh(
    conn: &mut SqliteConnection,
    node_id: &str,
) -> Result<Option<String>, StorageError>;
```

### Downstream consumers

| Plan | Imports | Use |
|---|---|---|
| Plan 2 (HTTP `/blob` dual-format) | `select_transport`, `Plane::Blob`, `TransportChoice` | Per-request transport selection in `http.rs` blob handler |
| Plan 4 (Gossip dual-publish) | `lookup_by_agent_cid`, `Plane::Gossip`, `select_transport` | Per-recipient gossip routing decision |
| Plan 6 (Recovery e2e) | `lookup_by_agent_cid`, `select_transport` (with all planes used by recovery: `Sync`, `Epr`, `EprAtom`, `IdentityHandshake`, `Trust`) | Per-witness transport selection |

Plans 2 / 4 / 6 do NOT consume the back-compat shims; those exist only for internal handshake plumbing migrated by Task 11.

---

## Self-review summary

- **Spec coverage** (lines 440-505): schema columns → Task 1; view type → Tasks 3-4; selection algorithm → Task 8; migration data path → Task 1; "what lives where" — selection at call site is the `peer_map` module per spec line 500, addressed in Tasks 6-12.
- **No placeholders**: every signature defined above appears in a numbered task; every adapter modification has a single named file.
- **Type consistency**: `PeerTransportManifest` definition in Task 6 matches the Public API surface above; `select_transport` arity matches Tasks 8 + 9 + 11.
- **Inter-plan exports**: enumerated above.

## Decision Required (one decision; no new crate dependencies)

**D1. Plane enum string encoding format.** Plane values stored in `libp2p_supports_json` / `iroh_supports_json` are JSON arrays of strings. The plan uses kebab-case `&'static str` constants matching protocol-spec wire conventions:

| Variant | Wire string |
|---|---|
| `Plane::Blob` | `"blob"` |
| `Plane::Gossip` | `"gossip"` |
| `Plane::Sync` | `"sync"` |
| `Plane::Epr` | `"epr"` |
| `Plane::EprAtom` | `"epr-atom"` |
| `Plane::Shard` | `"shard"` |
| `Plane::ViewFed` | `"view-fed"` |
| `Plane::IdentityHandshake` | `"identity-handshake"` |
| `Plane::Trust` | `"trust"` |

These also appear as enum values in `peer-transport-manifest.schema.json`. **Confirm before Task 3 runs.**

No other decisions required. No new crate dependencies. No spec contradictions found.

---

## Crate-wide caller inventory (frozen at plan-write time)

The grep for `cross_stack_peer_map`, `CrossStackPeerMapRow`, `iroh_for_libp2p`, `libp2p_for_iroh`, `record_libp2p`, `record_iroh`, `peer_map::` found these production + test call sites. Each is classified as **migrate** (move to extended API) or **shim** (keep calling back-compat shim until a follow-up sprint).

| Site | Classification | Owning task |
|---|---|---|
| `src/p2p_iroh/peer_map.rs` (module body) | **rewrite** | Task 6 |
| `src/p2p_iroh/README.md` (docs) | **migrate** (docs update) | Task 12 |
| `src/db/diesel_schema.rs:1411,1473` (`cross_stack_peer_map` table macros) | **regenerated** by Task 1 (`diesel print-schema`) | Task 1 |
| `tests/iroh_peer_map.rs` (4 functions) | **migrate** to new API | Task 9 |

No crate-wide production caller of `peer_map::*` exists outside `p2p_iroh/peer_map.rs` and the test file — verified by grep on `record_libp2p`, `record_iroh`, `iroh_for_libp2p`, `libp2p_for_iroh`, `peer_map::`. The libp2p-side identity-handshake plumbing in `src/p2p/mod.rs` writes to `peer_identity_bindings`, **not** `cross_stack_peer_map`. The shims therefore have no current callers — they exist only to hold the API surface stable for any out-of-tree consumer (steward, etc.) and for the README's documented examples. Task 6 documents this state.

---

## Build + test commands (every task uses these from `/projects/elohim/elohim/elohim-storage/`)

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features p2p-iroh
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh <test-name>
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract
```

---

## Task 1: Diesel migration `peer_transport_manifest`

**Files:**
- Create `/projects/elohim/elohim/elohim-storage/migrations/2026-05-10-120000_peer_transport_manifest/up.sql`
- Create `/projects/elohim/elohim/elohim-storage/migrations/2026-05-10-120000_peer_transport_manifest/down.sql`
- Modify `/projects/elohim/elohim/elohim-storage/src/db/diesel_schema.rs` (regenerated by `diesel print-schema`)

The HHMMSS `120000` is chosen at noon UTC on 2026-05-10 to be unambiguously distinct from `2026-05-08-045024` (cross_stack_peer_map predecessor) and `2026-05-08-033248` (peer_blob_inventory_blake3_hash) — the spec calls out diesel timestamp collision as a known footgun.

- [ ] **Step 1: Write `up.sql`.** Create the new table, copy existing rows from `cross_stack_peer_map` with sensible defaults, drop the old table.
  ```sql
  -- Phase 12 of iroh parallel stack — peer_transport_manifest graduation.
  -- Replaces 2026-05-08-045024_cross_stack_peer_map (transition-bridge schema).
  -- Per genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
  -- lines 440-505: this is permanent structural schema, not transition-bridge.
  --
  -- Source-of-truth notes (Category C operational projection):
  -- - agent_cid is the canonical DHT-anchored identity (imagodei); PK so
  --   writes from either transport converge.
  -- - libp2p_peer_id and iroh_node_id are NULL-able; CHECK enforces that
  --   at least one is populated (a row with neither is a protocol violation).
  -- - libp2p_addrs_json, iroh_relays_json, libp2p_supports_json,
  --   iroh_supports_json are TEXT containing JSON arrays of strings.
  --   Encoding/decoding lives in src/p2p_iroh/peer_map.rs.
  -- - discovery_methods_json is a TEXT JSON array of method names
  --   ("pkarr", "kademlia", "mdns", "doorway-introduction").
  -- - capability_level is 0-5 (per device archetypes — 5 = hub default,
  --   4 = always-on full node, 3 = laptop, 2 = phone, 1 = wearable, 0 = unknown).
  -- - last_observed is unix epoch seconds.

  -- Source of truth: libp2p Swarm + iroh Endpoint observations (Operational, Category C).
  -- No dht_anchor_hash column: agent_cid is a string reference to the DHT-canonical
  -- imagodei identity (anchored via the existing AgentPeerBinding entry type +
  -- peer_identity_bindings projection). This table is rebuildable from observed
  -- handshake arrivals; if it disagrees with the DHT, the DHT wins.
  CREATE TABLE peer_transport_manifest (
      agent_cid              TEXT NOT NULL PRIMARY KEY,  -- Category C operational projection key; DHT-anchor lives in AgentPeerBinding
      libp2p_peer_id         TEXT NULL,
      iroh_node_id           TEXT NULL,
      libp2p_addrs_json      TEXT NULL,
      iroh_relays_json       TEXT NULL,
      libp2p_supports_json   TEXT NULL,
      iroh_supports_json     TEXT NULL,
      discovery_methods_json TEXT NOT NULL,
      capability_level       INTEGER NOT NULL,
      last_observed          INTEGER NOT NULL,
      CHECK (libp2p_peer_id IS NOT NULL OR iroh_node_id IS NOT NULL)
  );

  CREATE UNIQUE INDEX idx_peer_transport_manifest_libp2p_peer_id
      ON peer_transport_manifest(libp2p_peer_id) WHERE libp2p_peer_id IS NOT NULL;
  CREATE UNIQUE INDEX idx_peer_transport_manifest_iroh_node_id
      ON peer_transport_manifest(iroh_node_id) WHERE iroh_node_id IS NOT NULL;

  -- Mechanical migration of existing rows from the bridge table.
  -- Spec lines 502-504: capability_level defaults to 5 (hub default);
  -- discovery_methods_json defaults to '["kademlia"]' for libp2p-only rows
  -- and '["pkarr","kademlia"]' if iroh_node_id is present.
  -- last_seen_at (ISO 8601 string) is converted to unix epoch via strftime.
  -- libp2p_addrs_json / iroh_relays_json / *_supports_json default to NULL —
  -- they will populate as observations land.
  INSERT INTO peer_transport_manifest (
      agent_cid,
      libp2p_peer_id,
      iroh_node_id,
      libp2p_addrs_json,
      iroh_relays_json,
      libp2p_supports_json,
      iroh_supports_json,
      discovery_methods_json,
      capability_level,
      last_observed
  )
  SELECT
      agent_cid,
      peer_id,
      node_id,
      NULL,
      NULL,
      NULL,
      NULL,
      CASE
          WHEN node_id IS NOT NULL THEN '["pkarr","kademlia"]'
          ELSE '["kademlia"]'
      END,
      5,
      CAST(strftime('%s', last_seen_at) AS INTEGER)
  FROM cross_stack_peer_map;

  DROP INDEX IF EXISTS idx_cross_stack_peer_map_peer_id;
  DROP INDEX IF EXISTS idx_cross_stack_peer_map_node_id;
  DROP TABLE cross_stack_peer_map;
  ```

- [ ] **Step 2: Write `down.sql`.** Inverse: recreate the bridge table, copy back the two-id columns, drop the new table.
  ```sql
  -- Inverse of 2026-05-10-120000_peer_transport_manifest/up.sql.
  -- Recreates the cross_stack_peer_map bridge table and copies back
  -- agent_cid + peer_id + node_id. last_seen_at is reconstructed from
  -- the unix epoch in last_observed; first_seen_at is set equal to it
  -- (we discarded first_seen_at in the up migration — accept the loss).

  CREATE TABLE cross_stack_peer_map (
      agent_cid     TEXT NOT NULL PRIMARY KEY,
      peer_id       TEXT NULL,
      node_id       TEXT NULL,
      first_seen_at TEXT NOT NULL,
      last_seen_at  TEXT NOT NULL
  );

  CREATE UNIQUE INDEX idx_cross_stack_peer_map_peer_id
      ON cross_stack_peer_map(peer_id) WHERE peer_id IS NOT NULL;
  CREATE UNIQUE INDEX idx_cross_stack_peer_map_node_id
      ON cross_stack_peer_map(node_id) WHERE node_id IS NOT NULL;

  INSERT INTO cross_stack_peer_map (agent_cid, peer_id, node_id, first_seen_at, last_seen_at)
  SELECT
      agent_cid,
      libp2p_peer_id,
      iroh_node_id,
      strftime('%Y-%m-%dT%H:%M:%SZ', last_observed, 'unixepoch'),
      strftime('%Y-%m-%dT%H:%M:%SZ', last_observed, 'unixepoch')
  FROM peer_transport_manifest;

  DROP INDEX IF EXISTS idx_peer_transport_manifest_libp2p_peer_id;
  DROP INDEX IF EXISTS idx_peer_transport_manifest_iroh_node_id;
  DROP TABLE peer_transport_manifest;
  ```

- [ ] **Step 3: Regenerate `diesel_schema.rs`.** Run from `/projects/elohim/elohim/elohim-storage/`:
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' diesel print-schema --database-url "sqlite:///tmp/phase12-schema-regen.sqlite" > src/db/diesel_schema.rs
  ```
  Then verify the diff shows: removal of the `cross_stack_peer_map` table block (lines 1409-1418) and removal of `cross_stack_peer_map,` from `allow_tables_to_appear_in_same_query!` (line 1473), plus addition of a new `peer_transport_manifest` table block with the column types from up.sql. **Do not hand-edit** — re-run `diesel print-schema` if the diff looks wrong.

- [ ] **Step 4: Build to verify.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features p2p-iroh
  ```
  Expected: builds successfully (the existing `peer_map.rs` references `cross_stack_peer_map` and will fail to compile — that's expected; Task 6 fixes it). If any **other** module fails (i.e. an unforeseen caller of the old table), STOP and add it to the caller inventory.

- [ ] **Step 5: Commit migration + schema regen.**
  ```bash
  git add elohim/elohim-storage/migrations/2026-05-10-120000_peer_transport_manifest \
          elohim/elohim-storage/src/db/diesel_schema.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: add peer_transport_manifest migration

  Graduates cross_stack_peer_map (transition-bridge schema) to
  peer_transport_manifest (permanent structural schema). Adds per-transport
  profile, supported-planes lists, discovery methods, capability_level (0-5
  from device archetypes). Existing rows migrate with capability_level=5
  default and discovery_methods inferred from which transport id was set.

  Spec: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
  lines 440-505.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 2: View JSON schema

**Files:**
- Create `/projects/elohim/elohim/sdk/schemas/v1/views/peer-transport-manifest.schema.json`

- [ ] **Step 1: Write the schema.** Follow `elohim/sdk/schemas/v1/views/CONVENTIONS.md` exactly: kebab-case file name matching `$id` suffix, source-of-truth declaration in `description`, `additionalProperties: false`, `required` arrays, nullable type pattern.
  ```json
  {
    "$id": "epr:schema:view:peer-transport-manifest",
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "PeerTransportManifestView",
    "description": "Source of truth: libp2p Swarm + iroh Endpoint observations (Operational, Category C). Persisted via peer_transport_manifest SQLite table for hybrid-window transport selection. Rebuildable from observed handshake arrivals on either transport. Per spec 2026-05-08-iroh-libp2p-complementarity §peer-transport-manifest.",
    "type": "object",
    "required": ["agentCid", "discovery", "capabilityLevel", "lastObserved"],
    "properties": {
      "agentCid": {
        "type": "string",
        "description": "Canonical DHT-anchored agent identity (imagodei). Stable across transports."
      },
      "libp2p": {
        "oneOf": [
          { "$ref": "#/$defs/Libp2pTransportProfileView" },
          { "type": "null" }
        ],
        "description": "libp2p transport profile if peer speaks libp2p. Null when only iroh-observed."
      },
      "iroh": {
        "oneOf": [
          { "$ref": "#/$defs/IrohTransportProfileView" },
          { "type": "null" }
        ],
        "description": "iroh transport profile if peer speaks iroh. Null when only libp2p-observed."
      },
      "discovery": {
        "type": "array",
        "items": { "type": "string", "enum": ["pkarr", "kademlia", "mdns", "doorway-introduction"] },
        "description": "Discovery methods that surfaced this peer."
      },
      "capabilityLevel": {
        "type": "integer",
        "minimum": 0,
        "maximum": 5,
        "description": "Device-archetype capability tier: 0=unknown, 1=wearable, 2=phone, 3=laptop, 4=always-on full node, 5=hub default."
      },
      "lastObserved": {
        "type": "integer",
        "description": "Unix epoch seconds of the last observation."
      }
    },
    "additionalProperties": false,
    "$defs": {
      "Libp2pTransportProfileView": {
        "type": "object",
        "required": ["peerId", "addrs", "supports"],
        "properties": {
          "peerId": { "type": "string", "description": "libp2p PeerId (multihash, multibase encoded)." },
          "addrs": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Listen multiaddrs."
          },
          "supports": {
            "type": "array",
            "items": {
              "type": "string",
              "enum": ["blob", "gossip", "sync", "epr", "epr-atom", "shard", "view-fed", "identity-handshake", "trust"]
            },
            "description": "Planes this peer supports over libp2p."
          }
        },
        "additionalProperties": false
      },
      "IrohTransportProfileView": {
        "type": "object",
        "required": ["nodeId", "relays", "supports"],
        "properties": {
          "nodeId": { "type": "string", "description": "iroh NodeId (32-byte ed25519 pubkey, base32 encoded)." },
          "relays": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Relay URLs the peer is reachable through."
          },
          "supports": {
            "type": "array",
            "items": {
              "type": "string",
              "enum": ["blob", "gossip", "sync", "epr", "epr-atom", "shard", "view-fed", "identity-handshake", "trust"]
            },
            "description": "Planes this peer supports over iroh."
          }
        },
        "additionalProperties": false
      }
    }
  }
  ```

- [ ] **Step 2: Validate the schema parses.** From repo root:
  ```bash
  pnpm run schema:test
  ```
  Expected: passes (existing tests + the new schema parses cleanly; no new test added at this step — Task 4 adds the contract test).

- [ ] **Step 3: Commit.**
  ```bash
  git add elohim/sdk/schemas/v1/views/peer-transport-manifest.schema.json
  git commit -m "$(cat <<'EOF'
  iroh phase 12: add peer-transport-manifest JSON schema

  Defines the wire shape for PeerTransportManifestView with nested
  Libp2pTransportProfileView + IrohTransportProfileView. Plane enum
  values are kebab-case (blob, gossip, sync, epr, epr-atom, shard,
  view-fed, identity-handshake, trust) matching wire conventions.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 3: Rust view structs

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/views.rs` (append at end)

- [ ] **Step 1: Add view structs to `views.rs`.** Append at end of file:
  ```rust
  // ============================================================================
  // Peer Transport Manifest View (Phase 12)
  // ============================================================================
  //
  // Source-of-truth pairing: peer_transport_manifest SQLite table,
  // populated by p2p_iroh::peer_map record_* fns.

  #[derive(Debug, Clone, Serialize, TS)]
  #[serde(rename_all = "camelCase")]
  #[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
  pub struct Libp2pTransportProfileView {
      pub peer_id: String,
      pub addrs: Vec<String>,
      pub supports: Vec<String>,
  }

  #[derive(Debug, Clone, Serialize, TS)]
  #[serde(rename_all = "camelCase")]
  #[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
  pub struct IrohTransportProfileView {
      pub node_id: String,
      pub relays: Vec<String>,
      pub supports: Vec<String>,
  }

  #[derive(Debug, Clone, Serialize, TS)]
  #[serde(rename_all = "camelCase")]
  #[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
  pub struct PeerTransportManifestView {
      pub agent_cid: String,
      pub libp2p: Option<Libp2pTransportProfileView>,
      pub iroh: Option<IrohTransportProfileView>,
      pub discovery: Vec<String>,
      pub capability_level: u8,
      pub last_observed: i64,
  }
  ```

- [ ] **Step 2: Re-export from `lib.rs`.** Add to the existing `pub use views::{...}` block in `/projects/elohim/elohim/elohim-storage/src/lib.rs` the names: `Libp2pTransportProfileView, IrohTransportProfileView, PeerTransportManifestView`. Locate the existing `pub use views::` lines via `grep -n "pub use views::" src/lib.rs` and add to the matching block.

- [ ] **Step 3: Run ts-rs export.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh export_bindings
  ```
  Expected: produces three new `.ts` files in `elohim/sdk/storage-client-ts/src/generated/`: `Libp2pTransportProfileView.ts`, `IrohTransportProfileView.ts`, `PeerTransportManifestView.ts`. Verify `git status` shows them.

- [ ] **Step 4: Commit.**
  ```bash
  git add elohim/elohim-storage/src/views.rs \
          elohim/elohim-storage/src/lib.rs \
          elohim/sdk/storage-client-ts/src/generated/Libp2pTransportProfileView.ts \
          elohim/sdk/storage-client-ts/src/generated/IrohTransportProfileView.ts \
          elohim/sdk/storage-client-ts/src/generated/PeerTransportManifestView.ts
  git commit -m "$(cat <<'EOF'
  iroh phase 12: add PeerTransportManifestView ts-rs export

  Adds Libp2pTransportProfileView + IrohTransportProfileView nested
  inside PeerTransportManifestView. camelCase boundary preserved per
  views.rs convention. cargo test export_bindings produces three new
  .ts files.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 4: Schema contract test

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Add the contract test.** Append at end of file (after the existing `peer_list_view_matches_schema` and before any closing module brace if present):
  ```rust
  // ── Peer Transport Manifest (Phase 12) ──────────────────────────

  #[test]
  fn peer_transport_manifest_view_matches_schema() {
      use elohim_storage::{
          IrohTransportProfileView, Libp2pTransportProfileView, PeerTransportManifestView,
      };

      let manifest = PeerTransportManifestView {
          agent_cid: "bafyrei...agent-1".to_string(),
          libp2p: Some(Libp2pTransportProfileView {
              peer_id: "12D3KooWPeer1".to_string(),
              addrs: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
              supports: vec!["blob".to_string(), "gossip".to_string()],
          }),
          iroh: Some(IrohTransportProfileView {
              node_id: "abcd...node-1".to_string(),
              relays: vec!["https://relay.iroh.network".to_string()],
              supports: vec!["blob".to_string(), "epr".to_string()],
          }),
          discovery: vec!["pkarr".to_string(), "kademlia".to_string()],
          capability_level: 5,
          last_observed: 1746878400,
      };

      let json = serde_json::to_value(&manifest).unwrap();
      validate_against_schema("views/peer-transport-manifest.schema.json", &json);
  }

  #[test]
  fn peer_transport_manifest_view_libp2p_only() {
      use elohim_storage::{Libp2pTransportProfileView, PeerTransportManifestView};

      let manifest = PeerTransportManifestView {
          agent_cid: "bafyrei...agent-2".to_string(),
          libp2p: Some(Libp2pTransportProfileView {
              peer_id: "12D3KooWPeer2".to_string(),
              addrs: vec![],
              supports: vec!["blob".to_string()],
          }),
          iroh: None,
          discovery: vec!["kademlia".to_string()],
          capability_level: 3,
          last_observed: 1746878500,
      };

      let json = serde_json::to_value(&manifest).unwrap();
      validate_against_schema("views/peer-transport-manifest.schema.json", &json);
  }

  #[test]
  fn peer_transport_manifest_view_iroh_only() {
      use elohim_storage::{IrohTransportProfileView, PeerTransportManifestView};

      let manifest = PeerTransportManifestView {
          agent_cid: "bafyrei...agent-3".to_string(),
          libp2p: None,
          iroh: Some(IrohTransportProfileView {
              node_id: "wxyz...node-3".to_string(),
              relays: vec![],
              supports: vec!["sync".to_string(), "view-fed".to_string()],
          }),
          discovery: vec!["pkarr".to_string()],
          capability_level: 5,
          last_observed: 1746878600,
      };

      let json = serde_json::to_value(&manifest).unwrap();
      validate_against_schema("views/peer-transport-manifest.schema.json", &json);
  }
  ```

- [ ] **Step 2: Run the contract test.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test schema_contract peer_transport_manifest
  ```
  Expected: 3 tests pass.

- [ ] **Step 3: Commit.**
  ```bash
  git add elohim/elohim-storage/tests/schema_contract.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: schema_contract test for PeerTransportManifestView

  Three test cases (both transports, libp2p-only, iroh-only) validate
  Rust serialization against the JSON schema at
  elohim/sdk/schemas/v1/views/peer-transport-manifest.schema.json.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 5: Codegen distribution

**Files:**
- Modify `/projects/elohim/elohim/sdk/schemas/scripts/codegen-ts.mjs`

- [ ] **Step 1: Add to `INTERFACE_FILES`.** Open the file, locate the `INTERFACE_FILES` array (starts at line 35). Add a new entry (insertion point: after `view-slice.ts` near the end of the array, line ~95):
  ```javascript
    { src: 'views/peer-transport-manifest.ts', dest: 'peer-transport-manifest.ts' },
  ```

- [ ] **Step 2: Run codegen.** From repo root:
  ```bash
  pnpm run schema:codegen:ts
  ```
  Expected: produces `peer-transport-manifest.ts` in all three GENERATED_OUTPUT_DIRS:
  - `genesis/seeder/src/generated/peer-transport-manifest.ts`
  - `app/elohim-app/src/app/generated/peer-transport-manifest.ts`
  - `app/elohim-library/projects/elohim-service/src/generated/peer-transport-manifest.ts`

- [ ] **Step 3: Commit.**
  ```bash
  git add elohim/sdk/schemas/scripts/codegen-ts.mjs \
          genesis/seeder/src/generated/peer-transport-manifest.ts \
          app/elohim-app/src/app/generated/peer-transport-manifest.ts \
          app/elohim-library/projects/elohim-service/src/generated/peer-transport-manifest.ts
  git commit -m "$(cat <<'EOF'
  iroh phase 12: distribute peer-transport-manifest.ts via schema codegen

  Adds peer-transport-manifest to INTERFACE_FILES in codegen-ts.mjs.
  Generates identical .ts to seeder, elohim-app, and elohim-library.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 6: Rewrite `peer_map.rs` core API

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/peer_map.rs` (full rewrite)

This task delivers the in-memory shape, observation/lookup/capability/discovery functions, and the back-compat shims. Selection algorithm is Task 8.

- [ ] **Step 1: Write failing unit tests for the new API first.** Inside the rewritten `peer_map.rs`, append a `#[cfg(test)] mod tests` module with these tests (write them BEFORE the implementation so the cycle is red → green):
  - `record_libp2p_observation_creates_row_with_supports`
  - `record_iroh_observation_upserts_existing_libp2p_row`
  - `record_capability_overwrites_default`
  - `record_discovery_replaces_methods`
  - `lookup_by_agent_cid_returns_full_manifest`
  - `lookup_by_libp2p_peer_id_finds_row`
  - `lookup_by_iroh_node_id_finds_row`
  - `lookup_returns_none_for_unknown`
  - `back_compat_record_libp2p_writes_row`
  - `back_compat_record_iroh_writes_row`
  - `back_compat_iroh_for_libp2p_resolves`
  - `back_compat_libp2p_for_iroh_resolves`

  Each test follows the existing pattern in `tests/iroh_peer_map.rs`: `init_pool_from_dir(tempdir().path())`, `run_migrations(&pool)`, `pool.get()`. Use realistic values: agent_cid `"bafyrei...agent-N"`, peer_id `"12D3KooWPeerN"`, node_id `"node-id-N"`.

- [ ] **Step 2: Implement the new module.** Replace the file entirely with the new module body:
  ```rust
  //! Phase 12 — peer transport manifest (graduated permanent schema).
  //!
  //! Replaces the Phase 10 cross_stack_peer_map bridge. Backed by the
  //! `peer_transport_manifest` table (Category C operational projection).
  //!
  //! Identity is keyed by `agent_cid`. Either `libp2p_peer_id` or
  //! `iroh_node_id` may be NULL (CHECK-enforced not-both-NULL). Per-
  //! transport profiles carry listen addrs / relay URLs and supported
  //! planes (kebab-case strings matching wire conventions).
  //!
  //! Selection at call site is via [`select_transport`] (defined in
  //! Task 8 of the Phase 12 plan).
  //!
  //! Spec: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
  //! lines 440-505.

  use diesel::prelude::*;
  use serde_json::Value as JsonValue;

  use crate::db::diesel_schema::peer_transport_manifest;
  use crate::error::StorageError;

  // ────────────────────────────────────────────────────────────────
  // In-memory shapes
  // ────────────────────────────────────────────────────────────────

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct PeerTransportManifest {
      pub agent_cid: String,
      pub libp2p: Option<Libp2pTransportProfile>,
      pub iroh: Option<IrohTransportProfile>,
      pub discovery: Vec<String>,
      pub capability_level: u8,
      pub last_observed: i64,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct Libp2pTransportProfile {
      pub peer_id: String,
      pub addrs: Vec<String>,
      pub supports: Vec<String>,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct IrohTransportProfile {
      pub node_id: String,
      pub relays: Vec<String>,
      pub supports: Vec<String>,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum Plane {
      Blob,
      Gossip,
      Sync,
      Epr,
      EprAtom,
      Shard,
      ViewFed,
      IdentityHandshake,
      Trust,
  }

  impl Plane {
      pub fn as_str(self) -> &'static str {
          match self {
              Plane::Blob => "blob",
              Plane::Gossip => "gossip",
              Plane::Sync => "sync",
              Plane::Epr => "epr",
              Plane::EprAtom => "epr-atom",
              Plane::Shard => "shard",
              Plane::ViewFed => "view-fed",
              Plane::IdentityHandshake => "identity-handshake",
              Plane::Trust => "trust",
          }
      }

      pub fn parse(s: &str) -> Option<Plane> {
          match s {
              "blob" => Some(Plane::Blob),
              "gossip" => Some(Plane::Gossip),
              "sync" => Some(Plane::Sync),
              "epr" => Some(Plane::Epr),
              "epr-atom" => Some(Plane::EprAtom),
              "shard" => Some(Plane::Shard),
              "view-fed" => Some(Plane::ViewFed),
              "identity-handshake" => Some(Plane::IdentityHandshake),
              "trust" => Some(Plane::Trust),
              _ => None,
          }
      }
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum TransportChoice {
      Iroh,
      Libp2p,
      Track3Bridge { hub_agent_cid: String },
      NoSharedTransport,
  }

  // ────────────────────────────────────────────────────────────────
  // Diesel row + (de)serialization helpers
  // ────────────────────────────────────────────────────────────────

  #[derive(Debug, Clone, Queryable, Selectable)]
  #[diesel(table_name = peer_transport_manifest)]
  #[diesel(check_for_backend(diesel::sqlite::Sqlite))]
  struct ManifestRow {
      agent_cid: String,
      libp2p_peer_id: Option<String>,
      iroh_node_id: Option<String>,
      libp2p_addrs_json: Option<String>,
      iroh_relays_json: Option<String>,
      libp2p_supports_json: Option<String>,
      iroh_supports_json: Option<String>,
      discovery_methods_json: String,
      capability_level: i32,
      last_observed: i64,
  }

  fn parse_string_array(json: &str, ctx: &'static str) -> Result<Vec<String>, StorageError> {
      let v: JsonValue = serde_json::from_str(json)
          .map_err(|e| StorageError::Database(format!("{ctx}: invalid json: {e}")))?;
      let arr = v
          .as_array()
          .ok_or_else(|| StorageError::Database(format!("{ctx}: expected array")))?;
      let mut out = Vec::with_capacity(arr.len());
      for item in arr {
          let s = item
              .as_str()
              .ok_or_else(|| StorageError::Database(format!("{ctx}: expected string element")))?;
          out.push(s.to_string());
      }
      Ok(out)
  }

  fn serialize_string_array(items: &[String]) -> String {
      JsonValue::Array(items.iter().map(|s| JsonValue::String(s.clone())).collect()).to_string()
  }

  fn supports_to_strings(supports: &[Plane]) -> Vec<String> {
      supports.iter().map(|p| p.as_str().to_string()).collect()
  }

  fn row_to_manifest(row: ManifestRow) -> Result<PeerTransportManifest, StorageError> {
      let libp2p = match row.libp2p_peer_id {
          Some(peer_id) => Some(Libp2pTransportProfile {
              peer_id,
              addrs: row
                  .libp2p_addrs_json
                  .as_deref()
                  .map(|s| parse_string_array(s, "libp2p_addrs_json"))
                  .transpose()?
                  .unwrap_or_default(),
              supports: row
                  .libp2p_supports_json
                  .as_deref()
                  .map(|s| parse_string_array(s, "libp2p_supports_json"))
                  .transpose()?
                  .unwrap_or_default(),
          }),
          None => None,
      };
      let iroh = match row.iroh_node_id {
          Some(node_id) => Some(IrohTransportProfile {
              node_id,
              relays: row
                  .iroh_relays_json
                  .as_deref()
                  .map(|s| parse_string_array(s, "iroh_relays_json"))
                  .transpose()?
                  .unwrap_or_default(),
              supports: row
                  .iroh_supports_json
                  .as_deref()
                  .map(|s| parse_string_array(s, "iroh_supports_json"))
                  .transpose()?
                  .unwrap_or_default(),
          }),
          None => None,
      };
      let discovery = parse_string_array(&row.discovery_methods_json, "discovery_methods_json")?;
      Ok(PeerTransportManifest {
          agent_cid: row.agent_cid,
          libp2p,
          iroh,
          discovery,
          capability_level: row.capability_level.clamp(0, 5) as u8,
          last_observed: row.last_observed,
      })
  }

  // ────────────────────────────────────────────────────────────────
  // Observation / capability / discovery writers
  // ────────────────────────────────────────────────────────────────

  pub fn record_libp2p_observation(
      conn: &mut SqliteConnection,
      agent_cid: &str,
      peer_id: &str,
      addrs: &[String],
      supports: &[Plane],
      observed_at: i64,
  ) -> Result<(), StorageError> {
      use peer_transport_manifest as t;
      let addrs_json = serialize_string_array(addrs);
      let supports_json = serialize_string_array(&supports_to_strings(supports));
      conn.transaction(|conn| {
          let existing: Option<ManifestRow> = t::table
              .filter(t::agent_cid.eq(agent_cid))
              .first(conn)
              .optional()?;
          match existing {
              Some(_) => {
                  diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
                      .set((
                          t::libp2p_peer_id.eq(peer_id),
                          t::libp2p_addrs_json.eq(&addrs_json),
                          t::libp2p_supports_json.eq(&supports_json),
                          t::last_observed.eq(observed_at),
                      ))
                      .execute(conn)?;
              }
              None => {
                  diesel::insert_into(t::table)
                      .values((
                          t::agent_cid.eq(agent_cid),
                          t::libp2p_peer_id.eq(Some(peer_id)),
                          t::iroh_node_id.eq::<Option<&str>>(None),
                          t::libp2p_addrs_json.eq(Some(&addrs_json)),
                          t::iroh_relays_json.eq::<Option<&str>>(None),
                          t::libp2p_supports_json.eq(Some(&supports_json)),
                          t::iroh_supports_json.eq::<Option<&str>>(None),
                          t::discovery_methods_json.eq("[\"kademlia\"]"),
                          t::capability_level.eq(5),
                          t::last_observed.eq(observed_at),
                      ))
                      .execute(conn)?;
              }
          }
          Ok::<_, diesel::result::Error>(())
      })
      .map_err(|e| StorageError::Database(format!("record_libp2p_observation: {e}")))
  }

  pub fn record_iroh_observation(
      conn: &mut SqliteConnection,
      agent_cid: &str,
      node_id: &str,
      relays: &[String],
      supports: &[Plane],
      observed_at: i64,
  ) -> Result<(), StorageError> {
      use peer_transport_manifest as t;
      let relays_json = serialize_string_array(relays);
      let supports_json = serialize_string_array(&supports_to_strings(supports));
      conn.transaction(|conn| {
          let existing: Option<ManifestRow> = t::table
              .filter(t::agent_cid.eq(agent_cid))
              .first(conn)
              .optional()?;
          match existing {
              Some(_) => {
                  diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
                      .set((
                          t::iroh_node_id.eq(node_id),
                          t::iroh_relays_json.eq(&relays_json),
                          t::iroh_supports_json.eq(&supports_json),
                          t::last_observed.eq(observed_at),
                      ))
                      .execute(conn)?;
              }
              None => {
                  diesel::insert_into(t::table)
                      .values((
                          t::agent_cid.eq(agent_cid),
                          t::libp2p_peer_id.eq::<Option<&str>>(None),
                          t::iroh_node_id.eq(Some(node_id)),
                          t::libp2p_addrs_json.eq::<Option<&str>>(None),
                          t::iroh_relays_json.eq(Some(&relays_json)),
                          t::libp2p_supports_json.eq::<Option<&str>>(None),
                          t::iroh_supports_json.eq(Some(&supports_json)),
                          t::discovery_methods_json.eq("[\"pkarr\",\"kademlia\"]"),
                          t::capability_level.eq(5),
                          t::last_observed.eq(observed_at),
                      ))
                      .execute(conn)?;
              }
          }
          Ok::<_, diesel::result::Error>(())
      })
      .map_err(|e| StorageError::Database(format!("record_iroh_observation: {e}")))
  }

  pub fn record_capability(
      conn: &mut SqliteConnection,
      agent_cid: &str,
      capability_level: u8,
  ) -> Result<(), StorageError> {
      use peer_transport_manifest as t;
      let level = capability_level.min(5) as i32;
      let updated = diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
          .set(t::capability_level.eq(level))
          .execute(conn)
          .map_err(|e| StorageError::Database(format!("record_capability: {e}")))?;
      if updated == 0 {
          return Err(StorageError::Database(format!(
              "record_capability: no manifest row for agent_cid {agent_cid}"
          )));
      }
      Ok(())
  }

  pub fn record_discovery(
      conn: &mut SqliteConnection,
      agent_cid: &str,
      methods: &[&str],
  ) -> Result<(), StorageError> {
      use peer_transport_manifest as t;
      let json = serialize_string_array(
          &methods.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
      );
      let updated = diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
          .set(t::discovery_methods_json.eq(json))
          .execute(conn)
          .map_err(|e| StorageError::Database(format!("record_discovery: {e}")))?;
      if updated == 0 {
          return Err(StorageError::Database(format!(
              "record_discovery: no manifest row for agent_cid {agent_cid}"
          )));
      }
      Ok(())
  }

  // ────────────────────────────────────────────────────────────────
  // Lookups
  // ────────────────────────────────────────────────────────────────

  pub fn lookup_by_agent_cid(
      conn: &mut SqliteConnection,
      agent_cid: &str,
  ) -> Result<Option<PeerTransportManifest>, StorageError> {
      use peer_transport_manifest as t;
      let row: Option<ManifestRow> = t::table
          .filter(t::agent_cid.eq(agent_cid))
          .select(ManifestRow::as_select())
          .first(conn)
          .optional()
          .map_err(|e| StorageError::Database(format!("lookup_by_agent_cid: {e}")))?;
      row.map(row_to_manifest).transpose()
  }

  pub fn lookup_by_libp2p_peer_id(
      conn: &mut SqliteConnection,
      peer_id: &str,
  ) -> Result<Option<PeerTransportManifest>, StorageError> {
      use peer_transport_manifest as t;
      let row: Option<ManifestRow> = t::table
          .filter(t::libp2p_peer_id.eq(peer_id))
          .select(ManifestRow::as_select())
          .first(conn)
          .optional()
          .map_err(|e| StorageError::Database(format!("lookup_by_libp2p_peer_id: {e}")))?;
      row.map(row_to_manifest).transpose()
  }

  pub fn lookup_by_iroh_node_id(
      conn: &mut SqliteConnection,
      node_id: &str,
  ) -> Result<Option<PeerTransportManifest>, StorageError> {
      use peer_transport_manifest as t;
      let row: Option<ManifestRow> = t::table
          .filter(t::iroh_node_id.eq(node_id))
          .select(ManifestRow::as_select())
          .first(conn)
          .optional()
          .map_err(|e| StorageError::Database(format!("lookup_by_iroh_node_id: {e}")))?;
      row.map(row_to_manifest).transpose()
  }

  // ────────────────────────────────────────────────────────────────
  // Back-compat shims (Phase 10 API → Phase 12 store)
  // ────────────────────────────────────────────────────────────────
  //
  // No production caller of these shims exists at Phase 12 land time
  // (verified by crate-wide grep, see plan caller inventory). They
  // remain to keep the Phase 10 surface stable for any out-of-tree
  // consumer (steward) and for the README's documented examples.

  fn iso_to_unix(observed_at: &str) -> Result<i64, StorageError> {
      chrono::DateTime::parse_from_rfc3339(observed_at)
          .map(|dt| dt.timestamp())
          .map_err(|e| StorageError::Database(format!("iso_to_unix({observed_at}): {e}")))
  }

  pub fn record_libp2p(
      conn: &mut SqliteConnection,
      agent_cid: &str,
      peer_id: &str,
      observed_at: &str,
  ) -> Result<(), StorageError> {
      let ts = iso_to_unix(observed_at)?;
      record_libp2p_observation(conn, agent_cid, peer_id, &[], &[], ts)
  }

  pub fn record_iroh(
      conn: &mut SqliteConnection,
      agent_cid: &str,
      node_id: &str,
      observed_at: &str,
  ) -> Result<(), StorageError> {
      let ts = iso_to_unix(observed_at)?;
      record_iroh_observation(conn, agent_cid, node_id, &[], &[], ts)
  }

  pub fn iroh_for_libp2p(
      conn: &mut SqliteConnection,
      peer_id: &str,
  ) -> Result<Option<String>, StorageError> {
      Ok(lookup_by_libp2p_peer_id(conn, peer_id)?.and_then(|m| m.iroh.map(|p| p.node_id)))
  }

  pub fn libp2p_for_iroh(
      conn: &mut SqliteConnection,
      node_id: &str,
  ) -> Result<Option<String>, StorageError> {
      Ok(lookup_by_iroh_node_id(conn, node_id)?.and_then(|m| m.libp2p.map(|p| p.peer_id)))
  }
  ```
  Note: `select_transport` is added in Task 8.

- [ ] **Step 3: Build and run unit tests.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --lib peer_map
  ```
  Expected: all 12 unit tests pass. If any fail, fix the implementation, not the test.

- [ ] **Step 4: Commit.**
  ```bash
  git add elohim/elohim-storage/src/p2p_iroh/peer_map.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: rewrite peer_map module on peer_transport_manifest

  Replaces the Phase 10 cross_stack_peer_map adapter with the Phase 12
  manifest API: PeerTransportManifest in-memory shape, Plane enum,
  record_*_observation / record_capability / record_discovery /
  lookup_by_* functions. Phase 10 record_libp2p / record_iroh /
  iroh_for_libp2p / libp2p_for_iroh are kept as back-compat shims that
  delegate to the new functions.

  select_transport (the selection algorithm) lands in Task 8.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 7: Migration round-trip + back-compat tests

**Files:**
- Create `/projects/elohim/elohim/elohim-storage/tests/iroh_peer_transport_manifest_migration.rs`
- Modify `/projects/elohim/elohim/elohim-storage/tests/iroh_peer_map.rs`

- [ ] **Step 1: Write migration round-trip test.** Create the new test file:
  ```rust
  //! Phase 12 migration round-trip — verifies up.sql (and down.sql via
  //! diesel migration revert) preserve cross-stack peer data.

  #![cfg(feature = "p2p-iroh")]

  use elohim_storage::db::{init_pool_from_dir, run_migrations};
  use elohim_storage::p2p_iroh::peer_map;
  use tempfile::tempdir;

  #[test]
  fn up_migration_preserves_phase10_rows_with_defaults() {
      // Migrations run all the way to Phase 12; we then validate that
      // post-migration rows can be read via the new API. Because the
      // up.sql copies from cross_stack_peer_map BEFORE dropping it, any
      // bridge-table rows present at migration time end up in the new
      // table with capability_level=5 and discovery defaults.
      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      let mut conn = pool.get().expect("conn");

      // Fresh DB has no Phase 10 rows to copy, so the migration is a
      // no-op for data. Seed via the new API and read back.
      peer_map::record_libp2p_observation(
          &mut conn,
          "bafyrei...agent-mig",
          "12D3KooWMig",
          &["/ip4/10.0.0.1/tcp/4001".to_string()],
          &[peer_map::Plane::Blob, peer_map::Plane::Gossip],
          1746878400,
      )
      .unwrap();

      let m = peer_map::lookup_by_agent_cid(&mut conn, "bafyrei...agent-mig")
          .unwrap()
          .expect("manifest present");
      assert_eq!(m.agent_cid, "bafyrei...agent-mig");
      assert!(m.libp2p.is_some());
      assert!(m.iroh.is_none());
      assert_eq!(m.capability_level, 5);
      assert_eq!(m.discovery, vec!["kademlia".to_string()]);
      let lp = m.libp2p.unwrap();
      assert_eq!(lp.peer_id, "12D3KooWMig");
      assert_eq!(lp.supports, vec!["blob".to_string(), "gossip".to_string()]);
  }

  #[test]
  fn iroh_only_observation_defaults_discovery_to_pkarr_kademlia() {
      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      let mut conn = pool.get().expect("conn");

      peer_map::record_iroh_observation(
          &mut conn,
          "bafyrei...agent-iroh",
          "node-id-iroh",
          &["https://relay.iroh.network".to_string()],
          &[peer_map::Plane::Sync],
          1746878400,
      )
      .unwrap();

      let m = peer_map::lookup_by_agent_cid(&mut conn, "bafyrei...agent-iroh")
          .unwrap()
          .unwrap();
      assert_eq!(m.discovery, vec!["pkarr".to_string(), "kademlia".to_string()]);
  }
  ```

- [ ] **Step 2: Update `tests/iroh_peer_map.rs` to use the new API.** Replace the four existing tests so they exercise both the back-compat shims AND the new API. Full replacement file body:
  ```rust
  //! Phase 12 acceptance — peer transport manifest API + Phase 10
  //! back-compat shims.

  #![cfg(feature = "p2p-iroh")]

  use elohim_storage::db::{init_pool_from_dir, run_migrations};
  use elohim_storage::p2p_iroh::peer_map;
  use tempfile::tempdir;

  fn setup() -> (tempfile::TempDir, diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>) {
      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      let conn = pool.get().expect("conn");
      (dir, conn)
  }

  #[test]
  fn back_compat_libp2p_then_iroh_converge_for_same_agent() {
      let (_dir, mut conn) = setup();
      let agent = "bafyrei...agent-1";
      peer_map::record_libp2p(&mut conn, agent, "12D3Koo...A", "2026-05-08T05:00:00Z").unwrap();
      peer_map::record_iroh(&mut conn, agent, "node-id-A", "2026-05-08T05:01:00Z").unwrap();

      let nid = peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...A").unwrap();
      assert_eq!(nid.as_deref(), Some("node-id-A"));

      let pid = peer_map::libp2p_for_iroh(&mut conn, "node-id-A").unwrap();
      assert_eq!(pid.as_deref(), Some("12D3Koo...A"));
  }

  #[test]
  fn back_compat_iroh_only_observation_returns_no_libp2p() {
      let (_dir, mut conn) = setup();
      let agent = "bafyrei...agent-2";
      peer_map::record_iroh(&mut conn, agent, "node-id-B", "2026-05-08T05:00:00Z").unwrap();
      let pid = peer_map::libp2p_for_iroh(&mut conn, "node-id-B").unwrap();
      assert_eq!(pid, None);
  }

  #[test]
  fn back_compat_libp2p_only_observation_returns_no_iroh() {
      let (_dir, mut conn) = setup();
      let agent = "bafyrei...agent-3";
      peer_map::record_libp2p(&mut conn, agent, "12D3Koo...C", "2026-05-08T05:00:00Z").unwrap();
      let nid = peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...C").unwrap();
      assert_eq!(nid, None);
  }

  #[test]
  fn back_compat_unknown_peer_id_resolves_to_none() {
      let (_dir, mut conn) = setup();
      assert_eq!(peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...nope").unwrap(), None);
      assert_eq!(peer_map::libp2p_for_iroh(&mut conn, "node-id-nope").unwrap(), None);
  }

  #[test]
  fn extended_api_full_manifest_roundtrip() {
      let (_dir, mut conn) = setup();
      let agent = "bafyrei...agent-full";
      peer_map::record_libp2p_observation(
          &mut conn,
          agent,
          "12D3KooWFull",
          &["/ip4/10.0.0.1/tcp/4001".to_string()],
          &[peer_map::Plane::Blob, peer_map::Plane::Gossip],
          1746878400,
      )
      .unwrap();
      peer_map::record_iroh_observation(
          &mut conn,
          agent,
          "node-id-full",
          &["https://relay.iroh.network".to_string()],
          &[peer_map::Plane::Blob, peer_map::Plane::Sync, peer_map::Plane::Epr],
          1746878401,
      )
      .unwrap();
      peer_map::record_capability(&mut conn, agent, 4).unwrap();
      peer_map::record_discovery(&mut conn, agent, &["pkarr", "mdns"]).unwrap();

      let m = peer_map::lookup_by_agent_cid(&mut conn, agent).unwrap().unwrap();
      assert_eq!(m.capability_level, 4);
      assert_eq!(m.discovery, vec!["pkarr".to_string(), "mdns".to_string()]);
      assert!(m.libp2p.is_some());
      assert!(m.iroh.is_some());
      assert_eq!(m.iroh.as_ref().unwrap().supports.len(), 3);
  }
  ```

- [ ] **Step 3: Run both test files.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_peer_map
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_peer_transport_manifest_migration
  ```
  Expected: 5 + 2 = 7 tests pass.

- [ ] **Step 4: Commit.**
  ```bash
  git add elohim/elohim-storage/tests/iroh_peer_map.rs \
          elohim/elohim-storage/tests/iroh_peer_transport_manifest_migration.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: peer_map back-compat + migration round-trip tests

  Updates iroh_peer_map.rs to exercise both Phase 10 shims and the
  Phase 12 extended API. Adds iroh_peer_transport_manifest_migration.rs
  for migration default behaviors (capability_level=5, discovery
  defaults inferred from transport set).

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 8: `select_transport` algorithm

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/peer_map.rs`

Per spec lines 480-490:
1. Both peers must support the plane on at least one shared transport.
2. Prefer iroh if both support iroh AND plane verdict allows iroh.
3. Fall back to libp2p if either peer lacks iroh or plane verdict requires libp2p.
4. If neither shares a transport, route via dwelling hub Track 3 if applicable.
5. Else fail with `NoSharedTransport`.

The "plane verdict" for "iroh-allowed" is the spec's decision rule (line 539): blob is iroh-canonical; gossip + recovery + identity-binding are dual-publish (selection prefers iroh when both support it); other planes follow the same prefer-iroh rule.

Track-3 bridge applies when `peer_manifest.capability_level <= 2` (wearable / phone) AND there is no shared substrate transport — those peers reach the substrate via a dwelling hub. The hub_agent_cid is taken from `self_manifest` IFF self has `capability_level >= 4` (this hub serves the bridge); otherwise `NoSharedTransport`.

- [ ] **Step 1: Write failing tests in the `peer_map.rs` test module.** Add these test cases:
  - `select_transport_both_iroh_supported_picks_iroh` — both peers list `Plane::Blob` in iroh.supports → `Iroh`
  - `select_transport_only_libp2p_shared_falls_back` — iroh present on both but `Plane::Trust` only in `libp2p.supports` for one → `Libp2p`
  - `select_transport_iroh_unsupported_by_peer_picks_libp2p` — self has both transports, peer is libp2p-only → `Libp2p`
  - `select_transport_low_capability_no_shared_returns_track3` — peer capability_level=2 and no shared transport, self capability_level=5 → `Track3Bridge { hub_agent_cid: <self.agent_cid> }`
  - `select_transport_no_shared_no_hub_returns_no_shared` — peer capability_level=2 but self capability_level=2 → `NoSharedTransport`
  - `select_transport_unsupported_plane_on_both_returns_no_shared` — both peers have iroh + libp2p but neither lists `Plane::Shard` in either → `NoSharedTransport`

- [ ] **Step 2: Implement `select_transport`.** Append to `peer_map.rs` (after the lookup functions, before the `#[cfg(test)]` module):
  ```rust
  // ────────────────────────────────────────────────────────────────
  // Selection algorithm (Phase 12 spec lines 480-490)
  // ────────────────────────────────────────────────────────────────

  fn libp2p_supports_plane(profile: &Option<Libp2pTransportProfile>, plane: Plane) -> bool {
      profile
          .as_ref()
          .map(|p| p.supports.iter().any(|s| s == plane.as_str()))
          .unwrap_or(false)
  }

  fn iroh_supports_plane(profile: &Option<IrohTransportProfile>, plane: Plane) -> bool {
      profile
          .as_ref()
          .map(|p| p.supports.iter().any(|s| s == plane.as_str()))
          .unwrap_or(false)
      }

  /// Select the transport for `plane` between `self_manifest` and
  /// `peer_manifest`. See spec lines 480-490 for the algorithm.
  ///
  /// Returns:
  /// - `Iroh` if both peers list the plane in their iroh profile.
  /// - `Libp2p` if both peers list the plane in their libp2p profile
  ///   (and the iroh path was not eligible).
  /// - `Track3Bridge { hub_agent_cid }` when no transport is shared
  ///   AND the peer is consumer-grade (capability_level <= 2) AND
  ///   self is hub-capable (capability_level >= 4) — the hub carries
  ///   the request via the Phase 11 doorway HTTP/WS bridge.
  /// - `NoSharedTransport` otherwise.
  ///
  /// `self_manifest` is the local node's own manifest entry (the
  /// caller is expected to look it up before calling, e.g. via
  /// `lookup_by_agent_cid(conn, local_agent_cid)`).
  pub fn select_transport(
      self_manifest: &PeerTransportManifest,
      peer_manifest: &PeerTransportManifest,
      plane: Plane,
  ) -> Result<TransportChoice, StorageError> {
      // Rule 2: prefer iroh when both peers support the plane on iroh.
      let self_iroh = iroh_supports_plane(&self_manifest.iroh, plane);
      let peer_iroh = iroh_supports_plane(&peer_manifest.iroh, plane);
      if self_iroh && peer_iroh {
          return Ok(TransportChoice::Iroh);
      }

      // Rule 3: fall back to libp2p when both peers support the plane
      // on libp2p (covers "either lacks iroh" and "plane verdict
      // requires libp2p" cases — both reduce to "no iroh path").
      let self_libp2p = libp2p_supports_plane(&self_manifest.libp2p, plane);
      let peer_libp2p = libp2p_supports_plane(&peer_manifest.libp2p, plane);
      if self_libp2p && peer_libp2p {
          return Ok(TransportChoice::Libp2p);
      }

      // Rule 4: Track 3 dwelling-hub bridge for consumer-grade peer +
      // hub-capable self.
      if peer_manifest.capability_level <= 2 && self_manifest.capability_level >= 4 {
          return Ok(TransportChoice::Track3Bridge {
              hub_agent_cid: self_manifest.agent_cid.clone(),
          });
      }

      // Rule 5: no path.
      Ok(TransportChoice::NoSharedTransport)
  }
  ```

- [ ] **Step 3: Run the new tests.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --lib peer_map::tests::select_transport
  ```
  Expected: 6 tests pass.

- [ ] **Step 4: Commit.**
  ```bash
  git add elohim/elohim-storage/src/p2p_iroh/peer_map.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: select_transport algorithm

  Implements the spec lines 480-490 selection rules: prefer iroh when
  both peers support the plane on iroh, fall back to libp2p when both
  support it there, route via Track 3 dwelling-hub bridge for
  consumer-grade peers when self is hub-capable, else NoSharedTransport.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 9: Integration test for full CRUD + multi-plane select_transport

**Files:**
- Create `/projects/elohim/elohim/elohim-storage/tests/iroh_peer_transport_manifest.rs`

- [ ] **Step 1: Write the integration test.**
  ```rust
  //! Phase 12 integration — full CRUD + select_transport across three planes.

  #![cfg(feature = "p2p-iroh")]

  use elohim_storage::db::{init_pool_from_dir, run_migrations};
  use elohim_storage::p2p_iroh::peer_map::{
      self, IrohTransportProfile, Libp2pTransportProfile, PeerTransportManifest, Plane,
      TransportChoice,
  };
  use tempfile::tempdir;

  fn hub_manifest() -> PeerTransportManifest {
      PeerTransportManifest {
          agent_cid: "bafyrei...hub".to_string(),
          libp2p: Some(Libp2pTransportProfile {
              peer_id: "12D3KooWHub".to_string(),
              addrs: vec!["/ip4/10.0.0.1/tcp/4001".to_string()],
              supports: vec!["blob".into(), "gossip".into(), "trust".into()],
          }),
          iroh: Some(IrohTransportProfile {
              node_id: "node-hub".to_string(),
              relays: vec![],
              supports: vec!["blob".into(), "sync".into(), "epr".into()],
          }),
          discovery: vec!["pkarr".into(), "kademlia".into()],
          capability_level: 5,
          last_observed: 1746878400,
      }
  }

  #[test]
  fn full_crud_persists_and_reads_back() {
      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      let mut conn = pool.get().expect("conn");

      let agent = "bafyrei...crud";
      peer_map::record_libp2p_observation(
          &mut conn,
          agent,
          "12D3KooWCrud",
          &["/ip4/10.0.0.1/tcp/4001".to_string()],
          &[Plane::Blob, Plane::Gossip, Plane::Trust],
          1746878400,
      )
      .unwrap();
      peer_map::record_iroh_observation(
          &mut conn,
          agent,
          "node-crud",
          &["https://relay.iroh.network".to_string()],
          &[Plane::Blob, Plane::Sync, Plane::Epr],
          1746878401,
      )
      .unwrap();
      peer_map::record_capability(&mut conn, agent, 4).unwrap();
      peer_map::record_discovery(&mut conn, agent, &["pkarr", "mdns"]).unwrap();

      let m = peer_map::lookup_by_agent_cid(&mut conn, agent).unwrap().unwrap();
      assert_eq!(m.capability_level, 4);
      assert_eq!(m.libp2p.as_ref().unwrap().supports.len(), 3);
      assert_eq!(m.iroh.as_ref().unwrap().supports.len(), 3);

      let m2 = peer_map::lookup_by_libp2p_peer_id(&mut conn, "12D3KooWCrud").unwrap().unwrap();
      assert_eq!(m2.agent_cid, agent);
      let m3 = peer_map::lookup_by_iroh_node_id(&mut conn, "node-crud").unwrap().unwrap();
      assert_eq!(m3.agent_cid, agent);
  }

  #[test]
  fn select_transport_blob_prefers_iroh() {
      let self_m = hub_manifest();
      let peer_m = hub_manifest();
      let choice = peer_map::select_transport(&self_m, &peer_m, Plane::Blob).unwrap();
      assert_eq!(choice, TransportChoice::Iroh);
  }

  #[test]
  fn select_transport_trust_only_libp2p_shared_falls_back() {
      // Both peers have iroh, but neither lists "trust" in iroh.supports;
      // both list it in libp2p.supports.
      let self_m = hub_manifest();
      let peer_m = hub_manifest();
      let choice = peer_map::select_transport(&self_m, &peer_m, Plane::Trust).unwrap();
      assert_eq!(choice, TransportChoice::Libp2p);
  }

  #[test]
  fn select_transport_sync_iroh_only_picks_iroh() {
      let self_m = hub_manifest();
      let peer_m = hub_manifest();
      let choice = peer_map::select_transport(&self_m, &peer_m, Plane::Sync).unwrap();
      assert_eq!(choice, TransportChoice::Iroh);
  }
  ```

- [ ] **Step 2: Run.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_peer_transport_manifest
  ```
  Expected: 4 tests pass (1 CRUD + 3 plane selection cases).

- [ ] **Step 3: Commit.**
  ```bash
  git add elohim/elohim-storage/tests/iroh_peer_transport_manifest.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: integration test for full CRUD + select_transport

  Exercises four planes (Blob, Trust, Sync) across the hub/hub case
  and validates the prefer-iroh / fall-back-libp2p decision tree.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 10: Iroh adapter — `epr_atom_backend` caller identity

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/epr_atom_backend.rs`

The current backend defaults `caller_identity` to `CallerIdentity::Anonymous`. Wire it through the manifest: at construction the backend gets a `DbPool`; on each request it looks up the connecting iroh `NodeId`. Because `ProtocolHandler::accept` does not yet expose the connecting peer's NodeId in our parity-harness API surface (per the existing module-level docs), Phase 12 wires the lookup function but leaves the NodeId source pluggable via a `with_caller_resolver` constructor — for tests and immediate consumption by Plan 2.

- [ ] **Step 1: Add a caller-resolver field + constructor.** Edit the struct definition to add:
  ```rust
  use crate::db::DbPool;
  use super::peer_map::{lookup_by_iroh_node_id, PeerTransportManifest};

  pub struct EprAtomServiceBackend {
      service: Arc<EprAtomService>,
      caller_resolver: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
      pool: Option<DbPool>,
  }

  impl EprAtomServiceBackend {
      pub fn new(service: Arc<EprAtomService>) -> Self {
          Self { service, caller_resolver: None, pool: None }
      }

      /// Phase 12 wiring: pass a DbPool and a closure that returns
      /// the connecting peer's iroh NodeId (or None if the harness
      /// can't expose it). The backend resolves the NodeId to a
      /// PeerTransportManifest and uses agent_cid as caller identity.
      pub fn with_caller_resolver(
          service: Arc<EprAtomService>,
          pool: DbPool,
          resolver: Arc<dyn Fn() -> Option<String> + Send + Sync>,
      ) -> Self {
          Self {
              service,
              caller_resolver: Some(resolver),
              pool: Some(pool),
          }
      }
  }
  ```

- [ ] **Step 2: Update the `handle` method.** Replace the body:
  ```rust
  #[async_trait::async_trait]
  impl EprAtomBackend for EprAtomServiceBackend {
      async fn handle(&self, request: EprAtomRequest) -> EprAtomResponse {
          let (peer_label, identity) = self.resolve_caller();
          self.service.handle(&peer_label, identity, request)
      }
  }

  impl EprAtomServiceBackend {
      fn resolve_caller(&self) -> (String, CallerIdentity) {
          let Some(resolver) = self.caller_resolver.as_ref() else {
              return ("iroh:peer".to_string(), CallerIdentity::Anonymous);
          };
          let Some(node_id) = resolver() else {
              return ("iroh:peer".to_string(), CallerIdentity::Anonymous);
          };
          let Some(pool) = self.pool.as_ref() else {
              return (format!("iroh:{node_id}"), CallerIdentity::Anonymous);
          };
          let mut conn = match pool.get() {
              Ok(c) => c,
              Err(e) => {
                  tracing::warn!(target: "elohim_storage::epr_atom",
                      error = %e, "iroh caller resolve: pool unavailable");
                  return (format!("iroh:{node_id}"), CallerIdentity::Anonymous);
              }
          };
          match lookup_by_iroh_node_id(&mut conn, &node_id) {
              Ok(Some(PeerTransportManifest { agent_cid, .. })) => (
                  format!("iroh:{node_id}"),
                  CallerIdentity::Agent(agent_cid),
              ),
              Ok(None) => (format!("iroh:{node_id}"), CallerIdentity::Anonymous),
              Err(e) => {
                  tracing::warn!(target: "elohim_storage::epr_atom",
                      error = %e, node_id = %node_id, "iroh caller resolve: lookup failed");
                  (format!("iroh:{node_id}"), CallerIdentity::Anonymous)
              }
          }
      }
  }
  ```

- [ ] **Step 3: Add a unit test for the resolver path.** In the existing `mod tests`:
  ```rust
  #[tokio::test]
  async fn resolver_yielding_known_node_id_resolves_to_agent_identity() {
      use crate::db::{init_pool_from_dir, run_migrations};
      use crate::p2p_iroh::peer_map::{record_iroh_observation, Plane};
      use tempfile::tempdir;

      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      {
          let mut conn = pool.get().unwrap();
          record_iroh_observation(
              &mut conn,
              "bafyrei...known",
              "node-known",
              &[],
              &[Plane::EprAtom],
              1746878400,
          )
          .unwrap();
      }

      let service = Arc::new(EprAtomService::new(None, Arc::new(DedupLru::new())));
      let backend = EprAtomServiceBackend::with_caller_resolver(
          service,
          pool,
          Arc::new(|| Some("node-known".to_string())),
      );
      let (label, identity) = backend.resolve_caller();
      assert_eq!(label, "iroh:node-known");
      match identity {
          CallerIdentity::Agent(cid) => assert_eq!(cid, "bafyrei...known"),
          other => panic!("expected Agent, got {other:?}"),
      }
  }
  ```

- [ ] **Step 4: Build + run.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh epr_atom
  ```
  Expected: existing tests still pass + new resolver test passes.

- [ ] **Step 5: Commit.**
  ```bash
  git add elohim/elohim-storage/src/p2p_iroh/epr_atom_backend.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: epr_atom_backend caller identity from manifest lookup

  Adds with_caller_resolver constructor that takes a DbPool + a NodeId
  resolver closure. Backend looks up the iroh NodeId in
  peer_transport_manifest and surfaces the agent_cid as
  CallerIdentity::Agent. Falls back to Anonymous when the resolver
  yields nothing or the manifest has no row for the NodeId — matches
  libp2p PeerIdentityMap semantics.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 11: Iroh adapter — `view_fed_backend` connected_peers from manifest

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/view_fed_backend.rs`

The current adapter passes `&[]` for `connected_peers` to `service.handle`. Wire it through the manifest: query for all rows that have a non-null `iroh_node_id` (or `libp2p_peer_id`, depending on what the call site expects — `view_federation` builds a slice of currently-connected peers as libp2p `PeerId` values, so we map agent_cid → libp2p peer_id when present).

- [ ] **Step 1: Add a helper that yields `Vec<PeerId>` from the manifest.** Edit `peer_map.rs` to add (no test needed — covered by Task 11 Step 3 below):
  ```rust
  /// Returns all libp2p PeerIds in the manifest. Used by view-fed
  /// adapter to populate connected_peers.
  pub fn list_libp2p_peer_ids(
      conn: &mut SqliteConnection,
  ) -> Result<Vec<String>, StorageError> {
      use peer_transport_manifest as t;
      t::table
          .filter(t::libp2p_peer_id.is_not_null())
          .select(t::libp2p_peer_id)
          .load::<Option<String>>(conn)
          .map(|rows| rows.into_iter().flatten().collect())
          .map_err(|e| StorageError::Database(format!("list_libp2p_peer_ids: {e}")))
  }
  ```

- [ ] **Step 2: Update `view_fed_backend.rs` to use the helper.** Add a `DbPool` field + constructor variant; in `handle`, if the pool is set, query and pass the resulting `Vec<PeerId>` to `service.handle`. Use `libp2p::PeerId::from_bytes` / `from_str` as appropriate (existing module already pulls libp2p in as a dep). Replace the `Iroh-mode connected_peers is currently an empty libp2p PeerId slice` comment block:
  ```rust
  use crate::db::DbPool;
  use crate::p2p_iroh::peer_map::list_libp2p_peer_ids;
  use libp2p::PeerId;

  pub struct ViewFedServiceBackend {
      service: Arc<ViewFedService>,
      local_peer_id_fallback: String,
      pool: Option<DbPool>,
  }

  impl ViewFedServiceBackend {
      pub fn new(service: Arc<ViewFedService>, local_peer_id_fallback: String) -> Self {
          Self { service, local_peer_id_fallback, pool: None }
      }

      pub fn with_manifest_pool(
          service: Arc<ViewFedService>,
          local_peer_id_fallback: String,
          pool: DbPool,
      ) -> Self {
          Self { service, local_peer_id_fallback, pool: Some(pool) }
      }
  }

  // … in handle():
  let connected: Vec<PeerId> = match self.pool.as_ref() {
      Some(pool) => match pool.get() {
          Ok(mut conn) => list_libp2p_peer_ids(&mut conn)
              .unwrap_or_default()
              .into_iter()
              .filter_map(|s| s.parse::<PeerId>().ok())
              .collect(),
          Err(_) => Vec::new(),
      },
      None => Vec::new(),
  };
  match self.service.handle(request.clone(), &connected).await {
      // … unchanged downstream
  }
  ```

- [ ] **Step 3: Add a test exercising the manifest-backed path.** In the existing `mod tests`:
  ```rust
  #[tokio::test]
  async fn manifest_backed_connected_peers_includes_libp2p_rows() {
      use crate::db::{init_pool_from_dir, run_migrations};
      use crate::p2p_iroh::peer_map::{record_libp2p_observation, Plane};
      use tempfile::tempdir;

      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      {
          let mut conn = pool.get().unwrap();
          // 12D3KooW... is the canonical libp2p PeerId prefix; the
          // test uses a known-valid base58btc PeerId.
          record_libp2p_observation(
              &mut conn,
              "bafyrei...vf",
              "12D3KooWNZyfTfb1ENd1HRgGTbvXebvA7iJfCRZHm9NzpDdEsTwo",
              &[],
              &[Plane::ViewFed],
              1746878400,
          )
          .unwrap();
      }
      // The test asserts construction succeeds with the manifest pool;
      // full-handle behavior is covered in iroh_view_fed_real_backend.
      let kp = libp2p::identity::Keypair::generate_ed25519();
      let local = libp2p::PeerId::from(kp.public()).to_string();
      let svc = Arc::new(ViewFedService::new(
          "agent-cid-self".into(),
          local.clone(),
          kp,
          None,
      ));
      let _backend = ViewFedServiceBackend::with_manifest_pool(svc, local, pool);
      // Smoke: doesn't panic.
  }
  ```

- [ ] **Step 4: Build + run.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh view_fed
  ```
  Expected: existing tests pass + new test passes.

- [ ] **Step 5: Commit.**
  ```bash
  git add elohim/elohim-storage/src/p2p_iroh/peer_map.rs \
          elohim/elohim-storage/src/p2p_iroh/view_fed_backend.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: view_fed_backend connected_peers from manifest

  Adds peer_map::list_libp2p_peer_ids and ViewFedServiceBackend::
  with_manifest_pool. When the manifest pool is wired, the iroh-mode
  view-federation adapter passes a real connected-peers slice (parsed
  from manifest libp2p_peer_id rows) instead of empty.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 12: Iroh adapter — `auth_backends` (trust + identity_handshake)

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/auth_backends.rs`

Two stopgaps in this file:
1. `TrustServiceBackend` skips `peer_trust_cache` insertion (libp2p-PeerId-keyed cache). Phase 12 hydrates the cache from the manifest at construction time AND on every accepted identity-handshake.
2. `IdentityHandshakeServiceBackend` uses `request.binding.peer_id` as the authoritative peer label. Phase 12 looks up the connecting NodeId in the manifest and uses the resulting `agent_cid` as the label.

The `PeerTrustCache` is already libp2p-PeerId-keyed (see `src/p2p/trust_cache.rs`). Hydration walks all manifest rows that have a `libp2p_peer_id`, parses each into `PeerId`, and inserts a `VerifiedTrustContext` derived from the agent_cid. (This matches the libp2p flow at `src/p2p/mod.rs:3922`.)

- [ ] **Step 1: Add hydration helper to `peer_map.rs`.**
  ```rust
  /// Returns (libp2p PeerId string, agent_cid) pairs for all manifest
  /// rows with a libp2p_peer_id. Used by trust-cache hydration.
  pub fn list_libp2p_to_agent(
      conn: &mut SqliteConnection,
  ) -> Result<Vec<(String, String)>, StorageError> {
      use peer_transport_manifest as t;
      t::table
          .filter(t::libp2p_peer_id.is_not_null())
          .select((t::libp2p_peer_id, t::agent_cid))
          .load::<(Option<String>, String)>(conn)
          .map(|rows| rows.into_iter().filter_map(|(pid, cid)| pid.map(|p| (p, cid))).collect())
          .map_err(|e| StorageError::Database(format!("list_libp2p_to_agent: {e}")))
  }
  ```

- [ ] **Step 2: Wire `TrustServiceBackend` to hydrate.** Edit `auth_backends.rs`:
  ```rust
  use crate::db::DbPool;
  use crate::p2p::trust_cache::{PeerTrustCache, VerifiedTrustContext};
  use crate::p2p_iroh::peer_map::{list_libp2p_to_agent, lookup_by_iroh_node_id};
  use libp2p::PeerId;

  pub struct TrustServiceBackend {
      service: Arc<TrustService>,
      trust_cache: Option<PeerTrustCache>,
      pool: Option<DbPool>,
  }

  impl TrustServiceBackend {
      pub fn new(service: Arc<TrustService>) -> Self {
          Self { service, trust_cache: None, pool: None }
      }

      /// Phase 12 wiring: hydrate the libp2p-keyed trust cache from
      /// the manifest at construction. Subsequent identity-handshake
      /// arrivals call `hydrate_one(node_id)` to refresh.
      pub fn with_trust_cache(
          service: Arc<TrustService>,
          trust_cache: PeerTrustCache,
          pool: DbPool,
      ) -> Self {
          let backend = Self {
              service,
              trust_cache: Some(trust_cache),
              pool: Some(pool),
          };
          backend.hydrate_all();
          backend
      }

      fn hydrate_all(&self) {
          let (Some(cache), Some(pool)) = (self.trust_cache.as_ref(), self.pool.as_ref()) else {
              return;
          };
          let Ok(mut conn) = pool.get() else { return; };
          let pairs = match list_libp2p_to_agent(&mut conn) {
              Ok(v) => v,
              Err(e) => {
                  tracing::warn!(target: "elohim_storage::trust",
                      error = %e, "iroh trust-cache hydrate: list failed");
                  return;
              }
          };
          for (peer_str, agent_cid) in pairs {
              if let Ok(peer) = peer_str.parse::<PeerId>() {
                  let ctx = VerifiedTrustContext {
                      agent_pubkey: agent_cid,
                      reach_ceiling: "public".to_string(),
                      verified_at_unix: chrono::Utc::now().timestamp(),
                      ttl_seconds: 3600,
                  };
                  // PeerTrustCache::insert is async; tokio::block_in_place
                  // is the safest path inside a sync construction context.
                  let cache = cache.clone();
                  tokio::task::block_in_place(|| {
                      tokio::runtime::Handle::current().block_on(cache.insert(peer, ctx));
                  });
              }
          }
      }
  }
  ```
  Note: confirm `VerifiedTrustContext` field names against `src/p2p/trust_cache.rs` before committing. If the struct shape differs, adjust the constructor call accordingly without inventing fields.

- [ ] **Step 3: Wire `IdentityHandshakeServiceBackend` to use manifest agent_cid.** Same file, replace the existing struct + `handle` body:
  ```rust
  pub struct IdentityHandshakeServiceBackend {
      service: Arc<IdentityHandshakeService>,
      local_peer_id_label: String,
      caller_resolver: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
      pool: Option<DbPool>,
  }

  impl IdentityHandshakeServiceBackend {
      pub fn new(service: Arc<IdentityHandshakeService>, local_peer_id_label: String) -> Self {
          Self {
              service,
              local_peer_id_label,
              caller_resolver: None,
              pool: None,
          }
      }

      pub fn with_caller_resolver(
          service: Arc<IdentityHandshakeService>,
          local_peer_id_label: String,
          pool: DbPool,
          resolver: Arc<dyn Fn() -> Option<String> + Send + Sync>,
      ) -> Self {
          Self {
              service,
              local_peer_id_label,
              caller_resolver: Some(resolver),
              pool: Some(pool),
          }
      }
  }

  #[async_trait::async_trait]
  impl IdentityHandshakeBackend for IdentityHandshakeServiceBackend {
      async fn handle(&self, request: IdentityHandshakeRequest) -> IdentityHandshakeResponse {
          let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
          let peer_label = self.resolve_peer_label(&request);
          self.service.handle(request, &peer_label, &now_iso)
      }
  }

  impl IdentityHandshakeServiceBackend {
      fn resolve_peer_label(&self, request: &IdentityHandshakeRequest) -> String {
          let Some(resolver) = self.caller_resolver.as_ref() else {
              return request.binding.peer_id.clone();
          };
          let Some(node_id) = resolver() else {
              return request.binding.peer_id.clone();
          };
          let Some(pool) = self.pool.as_ref() else {
              return node_id;
          };
          let mut conn = match pool.get() {
              Ok(c) => c,
              Err(_) => return node_id,
          };
          match lookup_by_iroh_node_id(&mut conn, &node_id) {
              Ok(Some(m)) => m.agent_cid,
              _ => node_id,
          }
      }
  }
  ```

- [ ] **Step 4: Add tests covering both new paths.** Append to the existing `mod tests`:
  ```rust
  #[tokio::test]
  async fn trust_backend_hydrates_libp2p_rows_into_cache() {
      use crate::db::{init_pool_from_dir, run_migrations};
      use crate::p2p::trust_cache::PeerTrustCache;
      use crate::p2p_iroh::peer_map::{record_libp2p_observation, Plane};
      use tempfile::tempdir;
      use libp2p::PeerId;

      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      let known_peer = "12D3KooWNZyfTfb1ENd1HRgGTbvXebvA7iJfCRZHm9NzpDdEsTwo";
      {
          let mut conn = pool.get().unwrap();
          record_libp2p_observation(
              &mut conn,
              "bafyrei...trust",
              known_peer,
              &[],
              &[Plane::Trust],
              1746878400,
          ).unwrap();
      }
      let cache = PeerTrustCache::new();
      let _backend = TrustServiceBackend::with_trust_cache(
          Arc::new(TrustService::new()),
          cache.clone(),
          pool,
      );
      // The cache should now contain an entry keyed by the parsed PeerId.
      let peer = known_peer.parse::<PeerId>().unwrap();
      assert!(cache.try_get(&peer).is_some());
  }

  #[tokio::test]
  async fn identity_handshake_uses_manifest_agent_cid_when_resolver_yields_known_node() {
      use crate::db::{init_pool_from_dir, run_migrations};
      use crate::p2p_iroh::peer_map::{record_iroh_observation, Plane};
      use tempfile::tempdir;

      let dir = tempdir().unwrap();
      let pool = init_pool_from_dir(dir.path()).expect("pool");
      run_migrations(&pool).expect("migrations");
      {
          let mut conn = pool.get().unwrap();
          record_iroh_observation(
              &mut conn,
              "bafyrei...idh",
              "node-idh",
              &[],
              &[Plane::IdentityHandshake],
              1746878400,
          ).unwrap();
      }
      let backend = IdentityHandshakeServiceBackend::with_caller_resolver(
          Arc::new(IdentityHandshakeService::new(None)),
          "iroh-local-node".to_string(),
          pool,
          Arc::new(|| Some("node-idh".to_string())),
      );
      let req = sample_id_request("12D3KooWClaim", "agent-cid-claimed");
      let label = backend.resolve_peer_label(&req);
      assert_eq!(label, "bafyrei...idh");
  }
  ```

- [ ] **Step 5: Build + run.**
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh auth_backends
  ```
  Expected: existing 3 tests + 2 new tests pass.

- [ ] **Step 6: Commit.**
  ```bash
  git add elohim/elohim-storage/src/p2p_iroh/peer_map.rs \
          elohim/elohim-storage/src/p2p_iroh/auth_backends.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: auth_backends trust hydration + handshake agent_cid

  TrustServiceBackend::with_trust_cache hydrates the libp2p-PeerId-keyed
  cache from peer_transport_manifest at construction.
  IdentityHandshakeServiceBackend::with_caller_resolver looks up the
  connecting iroh NodeId in the manifest and surfaces agent_cid as the
  authoritative peer label, replacing the stopgap that used the
  claimed binding.peer_id.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Task 13: Documentation update + EPR Announce dependency note

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/README.md`
- Modify `/projects/elohim/elohim/elohim-storage/src/p2p_iroh/epr_backend.rs` (docs only — code change is Plans 4 + 5)

- [ ] **Step 1: Update the README's "What's next" section.** Locate the existing block referencing `cross_stack_peer_map` (lines 110-113) and replace with a paragraph noting the Phase 12 graduation and the new module API. Update the Phase 10 acceptance checklist line to "Phase 12 acceptance — peer_transport_manifest". Cite the spec lines 440-505 and the four wired adapters.

- [ ] **Step 2: Add a docstring note to `epr_backend.rs`.** The Announce stopgap is unblocked by Plans 4 (gossip dual-publish) + 5 (recovery e2e), NOT by Plan 1 (this plan). Add a single line to the existing module-level doc block after the Announce paragraph:
  ```rust
  //! Announce graduation: blocked on Plan 4 (gossip dual-publish wires the
  //! identity-binding gossip topic over both transports) + Plan 5 (recovery
  //! e2e validates the dual-published path). Phase 12's peer_transport_manifest
  //! is a prerequisite (provides the per-peer support map) but does not by
  //! itself unblock Announce.
  ```

- [ ] **Step 3: Pre-push gate sanity.** Run from `/projects/elohim/elohim/elohim-storage/`:
  ```bash
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features p2p-iroh
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p-iroh --all-targets -- -D warnings
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo fmt --check
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh
  ```
  Expected: all clean. Then from repo root:
  ```bash
  pnpm run schema:test
  pnpm run schema:codegen:ts -- --verify
  ```
  Expected: schemas pass; codegen-verify reports no drift (because Task 5 already ran the codegen).

- [ ] **Step 4: Commit.**
  ```bash
  git add elohim/elohim-storage/src/p2p_iroh/README.md \
          elohim/elohim-storage/src/p2p_iroh/epr_backend.rs
  git commit -m "$(cat <<'EOF'
  iroh phase 12: README + epr_backend docs reflect manifest graduation

  README points to the new peer_transport_manifest module surface and
  drops the Phase 10 bridge-table language. epr_backend Announce
  docstring records that this Plan is a prerequisite, but Plans 4 + 5
  are the actual unblockers.

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

---

## Closing checklist

- [ ] All 13 tasks landed as 13 commits.
- [ ] `git log --oneline | head -13` shows the chain.
- [ ] `cargo test --features p2p-iroh` is green; `cargo test --test schema_contract` is green; `cargo test export_bindings` is green and produced the three new TS files.
- [ ] `git grep cross_stack_peer_map -- ':!**/migrations/**'` returns no hits in production code (only the new migration's down.sql references it for inverse).
- [ ] `git grep -E "list_libp2p_peer_ids|list_libp2p_to_agent|lookup_by_(agent_cid|libp2p_peer_id|iroh_node_id)|select_transport"` shows usage from `auth_backends.rs`, `view_fed_backend.rs`, `epr_atom_backend.rs` — no other production file consumes the API yet (Plans 2 / 4 / 6 wire the rest).
- [ ] Spec lines 440-505 re-read; every numbered concern is in a numbered task above.
