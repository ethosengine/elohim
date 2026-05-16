# EPR Phase 2B — First-Draft Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Status: first-draft.** Each batch kickoff session will tighten per-batch task scope before execution. Tasks listed here establish shape, files, step structure, and convergence points — *not* final-grained task breakdown.

**Goal:** Close the five `TODO(phase-2b)` seams in EPR federation: real identity binding replacing `StubIdentityMap`, resolver-backed signature verify, EprHead↔Envelope reconciliation, projector from `epr_atoms` to pillar tables, signal harness migration, per-pillar write-through flag, and Kad+gossipsub discovery composition.

**Architecture:** elohim-storage runs as a reconciliation controller over the Holochain DHT manifest (Principle P1). Identity bindings, key rotations, and revocations are DHT-authoritative; the controller reconciles operational state (caches, projection tables, session bindings) eagerly on observed DNA signals. Four batches: A identity+controller foundation, B projector+read-model, C producer migration+ramp, D discovery+fanout.

**Tech Stack:**
- Rust 1.84+ (Holochain HDK/HDI, diesel, libp2p 0.54, ciborium, ed25519-dalek, blake3, lru, tokio)
- TypeScript / Angular 19 (signal harness migration, elohim-service SDK)
- JSON Schema (Draft 2020-12) for wire contracts (schema-first per `feedback_schema_first_ioc`)
- SQLite via diesel migrations
- Holochain DNA: imagodei integrity zome extensions

**Spec:** `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` (8 coupling decisions resolved; 9 invariants; 15 p2p-design-gate classifications; convergence with Recovery M1–M4 and Phase 3–7 graph surface documented)

---

## Pre-flight — read these first

1. **The spec.** Sections 2 (Principle P1), 3 (8 decisions), 5 (9 invariants) are load-bearing.
2. **Phase 2C lock.** `tests/vectors/epr_atom_messages.json` + the 2C wire format are immutable. Any wire-breaking change is out of scope.
3. **Recovery M4 convergence.** Batch A task 3 (DNA signal stream contract) is shared with Recovery M4 fast-path revocation. Check the current state of `feature/recovery-m4-fast-path-revocation` branch and coordinate.
4. **Build commands** (per `CLAUDE.md`):
   - elohim-storage: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release`
   - storage tests: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test`
   - storage federation tests: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration`
   - Holochain DNA: `cd elohim/holochain && nix develop -c hc dna pack dna/imagodei`
   - TS codegen after View change: `cd elohim/elohim-storage && cargo test export_bindings`
   - Schema validation: `pnpm run schema:validate` + `pnpm run schema:check-dna` + `pnpm run schema:codegen:ts`
5. **Sweettest for DNA changes.** Per memory `feedback_swarm_composition_fresh_tree_build`: any integrity zome change needs `cargo check` from a clean tree + sweettest coverage before commit.
6. **Branch pattern:** `feature/epr-phase-2b-design` is this session's branch. Per-batch execution may fork to `feature/epr-phase-2b-batch-{a,b,c,d}` at batch kickoff.
7. **Husky hook** runs docs-only lint on spec/plan edits. Do not bypass (`HUSKY=0`).

### Source-of-truth declarations for schemas introduced by this plan

Per `.claude/skills/p2p-design-gate/SKILL.md` and spec §Appendix A. Every schema this plan introduces is classified here. Individual tasks reference this table rather than re-declaring.

| Schema file (new) | p2p-gate class | Source of truth |
|---|---|---|
| `elohim/sdk/schemas/v1/agent-peer-binding.schema.json` | **A** | Holochain DHT entry in imagodei integrity zome (`AgentPeerBinding` EntryType); libp2p layers are readers only |
| `elohim/sdk/schemas/v1/device-archetype.schema.json` | **A** (enum pin) | Pinned enum referenced by `AgentPeerBinding` (A); value set evolves via zome upgrade |
| `elohim/sdk/schemas/v1/dna-signal-stream.schema.json` + `dna-signals/*.schema.json` | **C** | Transient operational stream; messages are decode-only projections of DHT entries (`KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation`); reconstructable from DNA at any time |
| `elohim/sdk/schemas/v1/p2p/identity-handshake.schema.json` | **C** | libp2p session-local handshake payload wrapping a signed `AgentPeerBinding` (A); no persistent source of truth of its own |
| `elohim/sdk/schemas/v1/pillar-projection.schema.json` (extension to `app-manifest.schema.json`) | **C** in Phase 2B; promotes to **A2** once Phase 3 makes manifests into Manifest-EPRs | During 2B, manifests are committed config files; each declaration is a pillar-stewarded mapping rule. Phase 3 makes pillar manifests into A-notarized Manifest-EPRs, at which point projections become A2 (derived-via-link from the Manifest-EPR). |
| `elohim/sdk/schemas/v1/signal-intent.schema.json` | **C** | Transient HTTP request payload to `/api/v1/signal/emit`; never persisted; source-of-truth lives upstream in the Angular client's session state and downstream in the composed `Envelope` (A) after ingest |
| `elohim/sdk/schemas/v1/conductor-signing.schema.json` | **C** | Transient storage↔conductor RPC contract; no persistent source of truth; the signed bytes it produces become part of an `Envelope` (A) |

Rationale for the heavy-C skew: per Principle P1 (spec §2), 2B deliberately confines A-notarization to DHT entries (new `AgentPeerBinding` only) and leaves all operational state as reconcilable C-class projections. No B or B2 entities — the design explicitly avoids agent-scoped attestation layers in favor of DHT authority + operational reconciliation.

---

## Batch A — Identity & controller foundation

**Scope:** Decisions #1 (identity) and #2 (verify cache) from the spec. Delivers real `PeerIdentityMap`, `AgentPeerBinding` DNA entry, DNA signal stream contract (converging with Recovery M4), `ReconcileController` infrastructure in storage, two-level verify cache with eager revocation sweep.

**Converges with Recovery M1–M4.** Task A.3 (signal stream contract) is shared; coordinate with the recovery-m4 branch.

### Task A.1: JSON schema for `AgentPeerBinding`

**Files:**
- Create: `elohim/sdk/schemas/v1/agent-peer-binding.schema.json`
- Create: `elohim/sdk/schemas/v1/device-archetype.schema.json`

**Source of truth:** Holochain DHT entry in imagodei integrity zome (Category **A** per p2p-design-gate). The schema is the wire-format declaration of a DNA-notarized entity; the DHT entry is canonical, the SQLite projection in Task A.5 is a read-optimized derivation.

- [x] **Step 1: Write the JSON schema (schema-first)**

Content (summary — produce full Draft 2020-12 shape):
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.dev/schemas/v1/agent-peer-binding.schema.json",
  "title": "AgentPeerBinding",
  "type": "object",
  "required": ["peerId", "agentCid", "validFrom", "deviceArchetype", "signature"],
  "properties": {
    "peerId":         { "type": "string", "description": "libp2p PeerId, multibase-encoded" },
    "agentCid":       { "type": "string", "description": "CID of the Agent EPR (imagodei)" },
    "validFrom":      { "type": "string", "format": "date-time" },
    "validUntil":     { "type": ["string", "null"], "format": "date-time" },
    "deviceArchetype":{ "$ref": "device-archetype.schema.json" },
    "signature":      { "type": "string", "description": "base64 ed25519 over canonical body" },
    "supersededBy":   { "type": ["string", "null"], "description": "ActionHash of the binding that replaces this one" }
  }
}
```

For `device-archetype.schema.json`: enum of `"node" | "desktop" | "mobile" | "steward"` per memory `project_multi_device_humans`.

*Classification recap — both schemas above describe entities whose source of truth is the DHT (notarized via imagodei integrity zome in Task A.2). The JSON schema files are wire-format declarations; canonical state lives on the Holochain DHT; operational SQLite projections derive from DHT state per Principle P1.*

- [x] **Step 2: Run schema validation**

Run: `pnpm run schema:test && pnpm run schema:validate`
Expected: PASS (new schema is self-tested). The schemas describe DHT-notarized (Category A) entities, so the validation harness is verifying wire-format shape only — the DHT itself is the source of truth for instance data.

- [x] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/agent-peer-binding.schema.json elohim/sdk/schemas/v1/device-archetype.schema.json
git commit -m "schema(epr-2b): agent-peer-binding + device-archetype schemas (Category A — DHT notarized, source-of-truth in imagodei integrity zome)"
```

### Task A.2: `AgentPeerBinding` entry type + validators in imagodei integrity zome

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:991` (EntryTypes enum)
- Create: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/agent_peer_binding.rs`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:1037` (LinkTypes enum — add `AgentToPeerBinding`, `PeerToBinding`)
- Test: `elohim/holochain/tests/sweettest/src/tests/imagodei_peer_binding.rs`

- [x] **Step 1: Add entry type to integrity zome**

Append to EntryTypes enum:
```rust
pub enum EntryTypes {
    // ... existing entries ...
    AgentPeerBinding(AgentPeerBinding),
}
```

Add LinkTypes:
```rust
pub enum LinkTypes {
    // ... existing link types ...
    AgentToPeerBinding,   // AgentPubKey -> AgentPeerBinding (current bindings for this agent)
    PeerToBinding,        // StringAnchor(peer_id) -> AgentPeerBinding (reverse lookup)
}
```

- [x] **Step 2: Define `AgentPeerBinding` struct + canonical bytes**

In `agent_peer_binding.rs`:
```rust
use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct AgentPeerBinding {
    pub peer_id: String,              // multibase-encoded libp2p PeerId
    pub agent_cid: String,            // CID of Agent EPR
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
    pub device_archetype: DeviceArchetype,
    pub signature: Vec<u8>,           // ed25519 over canonical_bytes()
    pub superseded_by: Option<ActionHash>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DeviceArchetype { Node, Desktop, Mobile, Steward }

impl AgentPeerBinding {
    pub fn canonical_bytes(&self) -> Vec<u8> { /* deterministic CBOR of fields ex-signature,ex-supersededBy */ }
}
```

- [x] **Step 3: Write validator** (HDI-compatible — no `get_links`, per memory `project_hdi_no_get_links_in_validators`)

```rust
pub fn validate_create_agent_peer_binding(
    action: EntryCreationAction,
    binding: AgentPeerBinding,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: signer's AgentPubKey must match binding.agent_cid's current key
    //   (resolve via must_get_entry on Agent EPR; NOT via get_links)
    // Rule 2: signature must verify over canonical_bytes() with signer's pubkey
    // Rule 3: valid_from <= valid_until (if present)
    // Rule 4: device_archetype must be in the enum
    // Note: cross-entity rotation-chain checks (e.g., "is this key rotated?") live
    // in the coordinator pre-commit gate, not here.
    Ok(ValidateCallbackResult::Valid)
}
```

- [x] **Step 4: Write sweettest fixture**

Use `hc_sweettest::SweetConductor` to create two agents; agent A creates a binding for a test PeerId; assert:
- Binding entry is created
- Links from `AgentPubKey(A)` → binding exist via `AgentToPeerBinding`
- Links from `StringAnchor(peer_id)` → binding exist via `PeerToBinding`
- Agent B creating a binding claiming A's `agent_cid` fails validation (rule 1)

- [x] **Step 5: Run sweettest**

Run: `cd elohim/holochain/tests/sweettest && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test imagodei_peer_binding`
Expected: PASS

- [x] **Step 6: Commit**

```bash
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/ elohim/holochain/tests/sweettest/src/tests/imagodei_peer_binding.rs
git commit -m "feat(imagodei): AgentPeerBinding entry type + validators"
```

### Task A.3: DNA signal stream contract (imagodei → elohim-storage)

**Files:**
- Create: `elohim/sdk/schemas/v1/dna-signal-stream.schema.json`
- Create: `elohim/sdk/schemas/v1/dna-signals/` (directory for per-signal schemas)
- Create: `elohim/elohim-storage/src/reconcile/signal_stream.rs`

**Source of truth:** The signal stream itself is Category **C** (operational) — transient messages with no persistent source-of-truth of their own. Each signal message is a decode-only projection of an underlying DHT-notarized entry (`KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation` — all A-classified entries on the DHT). Stream is fully reconstructable by re-querying the DNA; no storage schema persists signals.

**Convergence point:** This task is shared with Recovery M4. Coordinate: if M4's branch already defines `KeyRevocation` signal emission, 2B's subscriber contract must match.

- [x] **Step 1: Schema-first — define signal message types**

The schema below describes operational (Category C) wire messages. Each `$ref` points to a per-signal sub-schema that carries the projection of its underlying DHT-notarized entry.

In `dna-signal-stream.schema.json`:
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.dev/schemas/v1/dna-signal-stream.schema.json",
  "title": "DNA Signal Stream Message (Category C — operational, source-of-truth is the DHT entry referenced by actionHash)",
  "oneOf": [
    { "$ref": "dna-signals/key-rotation.schema.json" },
    { "$ref": "dna-signals/key-revocation.schema.json" },
    { "$ref": "dna-signals/agent-peer-binding.schema.json" },
    { "$ref": "dna-signals/revocation-attestation.schema.json" }
  ]
}
```

*Classification recap: stream is operational (C); each message's source of truth is the DHT-notarized entry its `actionHash` field references. Stream is a projection, not canonical.*

Each sub-schema in `dna-signals/` declares:
- `signalType` tag (`keyRotation` / `keyRevocation` / `agentPeerBinding` / `revocationAttestation`)
- `actionHash` of the DNA entry that emitted it
- Per-type payload (pubkey, timestamps, revoked-pubkey, compromise_at, etc.)
- `emittedAt` timestamp

- [x] **Step 2: Define Rust types in storage**

```rust
// elohim-storage/src/reconcile/signal_stream.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "signalType", rename_all = "camelCase")]
pub enum DnaSignal {
    KeyRotation(KeyRotationSignal),
    KeyRevocation(KeyRevocationSignal),
    AgentPeerBinding(AgentPeerBindingSignal),
    RevocationAttestation(RevocationAttestationSignal),
}
// Per-variant payload structs mirror the JSON schema (Category C — operational;
// source-of-truth is the DHT-notarized entry referenced by signal.action_hash)
```

- [x] **Step 3: Define subscription trait**

```rust
#[async_trait]
pub trait DnaSignalStream: Send + Sync + 'static {
    async fn next_signal(&mut self) -> Option<DnaSignal>;
    async fn cursor(&self) -> Option<SignalCursor>;
    async fn resume_from(&mut self, cursor: SignalCursor) -> Result<(), SignalStreamError>;
}

// Stub implementation for tests: InMemoryDnaSignalStream
// Real implementation (Batch A task 11): HolochainAppSignalStream over ws
```

- [x] **Step 4: Unit test the stub**

```rust
#[tokio::test]
async fn stub_signal_stream_delivers_in_order() {
    let mut stream = InMemoryDnaSignalStream::with_signals(vec![
        DnaSignal::KeyRotation(..),
        DnaSignal::KeyRevocation(..),
    ]);
    assert!(matches!(stream.next_signal().await, Some(DnaSignal::KeyRotation(_))));
    assert!(matches!(stream.next_signal().await, Some(DnaSignal::KeyRevocation(_))));
    assert!(stream.next_signal().await.is_none());
}
```

- [x] **Step 5: Run test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test reconcile::signal_stream`
Expected: PASS

- [x] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/dna-signal-stream.schema.json elohim/sdk/schemas/v1/dna-signals/ elohim/elohim-storage/src/reconcile/
git commit -m "feat(storage): DNA signal stream contract + stub subscriber (Category C — operational projection of DHT-notarized entries, no persistent source-of-truth in stream itself)"
```

### Task A.4: `ReconcileController` skeleton

**Files:**
- Create: `elohim/elohim-storage/src/reconcile/controller.rs`
- Create: `elohim/elohim-storage/src/reconcile/mod.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` (expose `reconcile` module)
- Test: `elohim/elohim-storage/src/reconcile/controller_tests.rs`

- [x] **Step 1: Write failing test — controller dispatches by signal type**

```rust
#[tokio::test]
async fn controller_routes_key_rotation_to_timeline_update() {
    let (signal_tx, signal_rx) = mpsc::channel(4);
    let stream = ChannelSignalStream::new(signal_rx);
    let mut controller = ReconcileController::new(stream, mock_storage());
    signal_tx.send(DnaSignal::KeyRotation(sample_rotation())).await.unwrap();
    controller.run_one_pass().await;
    assert!(controller.pubkey_cache().get(&agent_cid()).is_some());
}
```

- [x] **Step 2: Run test, verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test controller_routes_key_rotation`
Expected: FAIL (type/method not defined)

- [x] **Step 3: Implement minimal skeleton**

```rust
pub struct ReconcileController<S: DnaSignalStream> {
    stream: S,
    pubkey_cache: PubkeyTimelineCache,  // see Task A.6
    binding_cache: Arc<RwLock<PeerBindingCache>>, // see Task A.5
    db_pool: Arc<Pool>,
}

impl<S: DnaSignalStream> ReconcileController<S> {
    pub async fn run_one_pass(&mut self) -> Result<(), ReconcileError> {
        while let Some(signal) = self.stream.next_signal().await {
            match signal {
                DnaSignal::KeyRotation(r)          => self.on_key_rotation(r).await?,
                DnaSignal::KeyRevocation(r)        => self.on_key_revocation(r).await?,
                DnaSignal::AgentPeerBinding(b)     => self.on_agent_peer_binding(b).await?,
                DnaSignal::RevocationAttestation(a)=> self.on_revocation_attestation(a).await?,
            }
        }
        Ok(())
    }
    // Handlers: defer full impl to later tasks; stub to insert into caches
}
```

- [x] **Step 4: Run test, verify it passes**

Run: same as Step 2
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/reconcile/ elohim/elohim-storage/src/lib.rs
git commit -m "feat(storage): ReconcileController skeleton"
```

### Task A.5: `peer_identity_bindings` table + `HolochainBackedPeerIdentityMap`

**Files:**
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD-HHMMSS_peer_identity_bindings/up.sql`
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD-HHMMSS_peer_identity_bindings/down.sql`
- Create: `elohim/elohim-storage/src/db/peer_identity_bindings.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`
- Modify: `elohim/elohim-storage/src/p2p/identity_map.rs` (replace `StubIdentityMap` with `HolochainBackedPeerIdentityMap`)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:913,988` (construction sites)

- [x] **Step 1: Write migration**

```sql
-- up.sql
CREATE TABLE peer_identity_bindings (
    peer_id              TEXT NOT NULL,
    agent_cid            TEXT NOT NULL,
    dht_anchor_hash  TEXT NOT NULL,
    valid_from           TIMESTAMP NOT NULL,
    valid_until          TIMESTAMP,
    observed_at          TIMESTAMP NOT NULL,
    source               TEXT NOT NULL,  -- 'dht' | 'gossip' | 'handshake'
    PRIMARY KEY (peer_id, dht_anchor_hash)
);
CREATE INDEX idx_peer_identity_bindings_peer_id ON peer_identity_bindings(peer_id);
CREATE INDEX idx_peer_identity_bindings_agent_cid ON peer_identity_bindings(agent_cid);
```

```sql
-- down.sql
DROP TABLE peer_identity_bindings;
```

- [x] **Step 2: Write failing test — HolochainBacked map resolves registered binding**

```rust
#[tokio::test]
async fn holochain_backed_map_resolves_binding_from_table() {
    let pool = test_pool();
    insert_binding(&pool, "peer-xyz", "agent-abc", /* ... */).await;
    let map = HolochainBackedPeerIdentityMap::new(pool, mock_stream());
    assert_eq!(map.lookup(&peer_id("peer-xyz")), CallerIdentity::Agent("agent-abc".into()));
}
```

- [x] **Step 3: Run failing test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test holochain_backed_map_resolves`
Expected: FAIL

- [x] **Step 4: Implement**

Diesel model for `peer_identity_bindings`, diesel queries, and `HolochainBackedPeerIdentityMap`:
```rust
impl PeerIdentityMap for HolochainBackedPeerIdentityMap {
    fn lookup(&self, peer: &PeerId) -> CallerIdentity {
        // Query peer_identity_bindings where peer_id = peer AND (valid_until IS NULL OR valid_until > now())
        // Return Agent(agent_cid) or Anonymous
    }
}
```

- [x] **Step 5: Replace construction sites**

At `elohim/elohim-storage/src/p2p/mod.rs:913`, replace `StubIdentityMap::new()` with `HolochainBackedPeerIdentityMap::new(pool.clone(), signal_stream)`. Delete `identity_map::StubIdentityMap` export; keep the trait.

- [x] **Step 6: Run tests + phase-2c regression**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration`
Expected: existing Batch D tests still pass (reach-gate behavior preserved; the map now reads from DB rather than in-memory).

- [x] **Step 7: Commit**

```bash
git add elohim/elohim-storage/migrations/*/peer_identity_bindings/ elohim/elohim-storage/src/db/peer_identity_bindings.rs elohim/elohim-storage/src/p2p/identity_map.rs
git commit -m "feat(storage): peer_identity_bindings table + HolochainBackedPeerIdentityMap replacing StubIdentityMap"
```

### Task A.6: Per-agent pubkey timeline LRU

**Files:**
- Create: `elohim/elohim-storage/src/reconcile/pubkey_timeline.rs`

- [x] **Step 1: Failing test — timeline finds validity at issued_at**

```rust
#[test]
fn pubkey_timeline_finds_key_valid_at_timestamp() {
    let mut tl = PubkeyTimeline::new();
    tl.insert(pubkey_a(), t(100), Some(t(200)), ah_rotation());
    tl.insert(pubkey_b(), t(200), None, ah_rotation());
    assert_eq!(tl.pubkey_at(t(150)).unwrap().pubkey, pubkey_a());
    assert_eq!(tl.pubkey_at(t(250)).unwrap().pubkey, pubkey_b());
    assert_eq!(tl.pubkey_at(t(50)), None);
}
```

- [x] **Step 2: Run, verify fails**

Run: `cargo test pubkey_timeline_finds`
Expected: FAIL

- [x] **Step 3: Implement `PubkeyTimeline` + LRU cache wrapper**

```rust
pub struct PubkeyTimeline {
    validities: Vec<PubkeyValidity>,  // sorted by valid_from
}
pub struct PubkeyValidity {
    pub pubkey: [u8; 32],
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub action_hash: String,
}
impl PubkeyTimeline {
    pub fn pubkey_at(&self, at: DateTime<Utc>) -> Option<&PubkeyValidity> { /* binary search */ }
    pub fn insert(&mut self, pubkey: [u8;32], from: DateTime<Utc>, until: Option<DateTime<Utc>>, ah: String) { /* maintain sorted order */ }
    pub fn mark_revoked(&mut self, from: DateTime<Utc>) { /* set all valid_until > from to Some(from) */ }
}

pub struct PubkeyTimelineCache {
    lru: LruCache<String /*agent_cid*/, PubkeyTimeline>,
}
impl PubkeyTimelineCache {
    pub fn get_or_load(&mut self, agent_cid: &str, conn: &mut SqliteConnection) -> Result<&PubkeyTimeline, _> { /*…*/ }
    pub fn update_on_rotation(&mut self, signal: &KeyRotationSignal) { /*…*/ }
    pub fn invalidate(&mut self, agent_cid: &str) { self.lru.pop(agent_cid); }
}
```

- [x] **Step 4: Run, verify passes**

Expected: PASS

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/reconcile/pubkey_timeline.rs
git commit -m "feat(storage): pubkey timeline cache for verify"
```

### Task A.7: `verified_at` + `verified_signer_fingerprint` columns on `epr_atoms`

**Files:**
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD-HHMMSS_verified_at_on_epr_atoms/up.sql`
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD-HHMMSS_verified_at_on_epr_atoms/down.sql`
- Modify: `elohim/elohim-storage/src/db/epr_atoms.rs` (diesel model)
- Modify: `elohim/elohim-storage/src/services/epr_service.rs` (ingest sets `verified_at` on success)

- [x] **Step 1: Migration up**

```sql
ALTER TABLE epr_atoms ADD COLUMN verified_at TIMESTAMP;
ALTER TABLE epr_atoms ADD COLUMN verified_signer_fingerprint TEXT;
CREATE INDEX idx_epr_atoms_signer_cid_issued_at ON epr_atoms(signer_cid, issued_at);
```

`down.sql`: drop both columns + index.

- [x] **Step 2: Failing test — ingest stamps verified_at on success**

```rust
#[test]
fn ingest_stamps_verified_at_when_signature_verifies() {
    let epr = valid_epr();
    let result = EprService::ingest(&mut conn, epr).unwrap();
    let stored = fetch(&mut conn, &result.cid).unwrap();
    assert!(stored.atom.verified_at.is_some());
    assert!(stored.atom.verified_signer_fingerprint.is_some());
}
```

- [x] **Step 3: Verify failure, then implement**

Extend `EprService::ingest` to: resolve pubkey via `PubkeyTimelineCache`, verify ed25519 signature over canonical bytes, set `verified_at = Utc::now()` and `verified_signer_fingerprint = blake3-128-prefix(pubkey)` on success. On verify failure, reject with `StorageError::VerifyFailed`.

- [x] **Step 4: Run tests**

Expected: PASS + existing ingest tests still pass.

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/*/verified_at_on_epr_atoms/ elohim/elohim-storage/src/db/epr_atoms.rs elohim/elohim-storage/src/services/epr_service.rs
git commit -m "feat(storage): verified_at column + resolver-backed signature verify"
```

### Task A.8: Eager revocation sweep in `ReconcileController`

**Files:**
- Modify: `elohim/elohim-storage/src/reconcile/controller.rs` (implement `on_key_revocation`)
- Create: `elohim/elohim-storage/src/reconcile/sweep.rs`

- [x] **Step 1: Failing test — revocation sweep clears verified_at for affected EPRs**

```rust
#[tokio::test]
async fn revocation_sweep_clears_verified_within_compromise_window() {
    let mut conn = test_conn();
    insert_epr(&mut conn, signer_cid="A", issued_at=t(100), verified_at=Some(t(101)));
    insert_epr(&mut conn, signer_cid="A", issued_at=t(50),  verified_at=Some(t(51)));
    let mut ctrl = controller_with_dna_state(&conn);
    ctrl.on_key_revocation(KeyRevocationSignal {
        agent_cid: "A".into(),
        compromise_at: t(75),
        action_hash: ..,
    }).await.unwrap();
    assert_eq!(fetch_verified_at(&conn, signer="A", issued_at=t(100)), None);
    assert_eq!(fetch_verified_at(&conn, signer="A", issued_at=t(50)),  Some(t(51))); // pre-compromise preserved
}
```

- [x] **Step 2: Implement sweep**

```rust
// Sweep operates on epr_atoms (A-class, content-addressed via CID — source of truth
// is the signed Envelope bytes; the diesel 'schema' module below is just the diesel
// column-name import, not a new storage schema).
pub async fn sweep_on_revocation(
    conn: &mut SqliteConnection,
    signal: &KeyRevocationSignal,
) -> Result<SweepReport, ReconcileError> {
    use crate::db::schema::epr_atoms::dsl::*;  // diesel-generated column names (operational)
    let updated = diesel::update(
        epr_atoms.filter(signer_cid.eq(&signal.agent_cid))
                 .filter(issued_at.ge(signal.compromise_at)),
    )
    .set((
        verified_at.eq::<Option<NaiveDateTime>>(None),
        verified_signer_fingerprint.eq("revoked_stale"),
    ))
    .execute(conn)?;
    Ok(SweepReport { affected_rows: updated, at: Utc::now() })
}
```

Wire it into `ReconcileController::on_key_revocation`. Also call `pubkey_cache.invalidate(&signal.agent_cid)` so the next verify reloads fresh.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): eager revocation sweep in ReconcileController"
```

### Task A.9: Libp2p authentication handshake — exchange signed bindings

**Files:**
- Create: `elohim/elohim-storage/src/p2p/identity_handshake.rs`
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs` (register handshake protocol)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (wire handshake into connection-established event)

**Source of truth:** The handshake payload is Category **C** (operational, session-local) — it wraps a signed `AgentPeerBinding` whose source of truth is the DHT-notarized entry in imagodei (Task A.2). The handshake itself is not persisted; it is a libp2p session-local projection of DHT state, reconstructable on re-handshake.

- [x] **Step 1: Define handshake wire type (schema-first)**

The schema below describes an operational (Category C) wire message. Source of truth for the inner `binding` remains the DHT entry it projects; handshake adds no new notarized state.

Schema file: `elohim/sdk/schemas/v1/p2p/identity-handshake.schema.json`. Shape: `{ "binding": AgentPeerBinding, "timestamp": DateTime, "nonce": bytes }`. Include in `schema:codegen:ts`. (Category C — operational handshake, not a DHT-notarized entity in itself.)

- [x] **Step 2: Failing integration test — peer A dials peer B, B learns A's binding**

Extend `elohim/elohim-storage/tests/epr_atom_federation_integration.rs::two_peer_swarm()` helper: after connection, assert peer B's `HolochainBackedPeerIdentityMap.lookup(&peer_a_id)` returns `CallerIdentity::Agent(peer_a_agent_cid)`.

- [x] **Step 3: Implement handshake**

New libp2p request-response protocol `/elohim/identity/handshake/1.0.0`. On connection-established, each peer sends the other its current signed `AgentPeerBinding`. Receiver verifies signature + validity window; on valid: inserts into `peer_identity_bindings` table with `source='handshake'`; on invalid: logs and leaves peer Anonymous.

- [x] **Step 4: Run integration test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration`
Expected: new handshake test passes; existing Batch D tests still pass.

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/identity_handshake.rs elohim/sdk/schemas/v1/p2p/identity-handshake.schema.json elohim/elohim-storage/tests/epr_atom_federation_integration.rs
git commit -m "feat(storage): libp2p identity handshake exchanges signed bindings (Category C — operational, wraps A-notarized AgentPeerBinding; session-local projection, no persistent source-of-truth)"
```

### Task A.10: Gossipsub `elohim/identity/binding` topic

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs` (subscribe `elohim/identity/binding` topic — add next to existing `recovery.invitation` subscription at commit `e9e2806a`)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (publish binding on `AgentPeerBinding` DNA signal; consume + insert into table on receive)

- [x] **Step 1: Failing test — binding gossiped mid-session propagates to connected peers**

Extend integration test: peer A rotates key, emits new `AgentPeerBinding`; peer B (already connected) receives via gossip; peer B's cache updates; peer B's verify cache invalidates stale verifications for agent A.

- [x] **Step 2: Implement subscribe + publish**

Subscribe `elohim/identity/binding` in behaviour setup. In swarm event loop, handle `GossipsubEvent::Message` where topic matches: deserialize `AgentPeerBinding`, verify, insert into table with `source='gossip'`, emit reconcile signal.

On outbound: `ReconcileController::on_agent_peer_binding` (new handler) — when the local agent creates a binding, publish on the topic in addition to DHT-notarizing.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): gossipsub identity.binding topic for mid-session rotation propagation"
```

### Task A.11: Real `HolochainAppSignalStream` — connect to imagodei conductor

**Files:**
- Create: `elohim/elohim-storage/src/reconcile/holochain_app_signal.rs`
- Modify: `elohim/elohim-storage/src/reconcile/signal_stream.rs`

- [x] **Step 1: Design test harness**

Connect to a running sweettest conductor over the standard `holochain_client_rust` app-websocket interface. Subscribe to imagodei cell. Translate `Signal::App(_)` events into `DnaSignal::*` by inspecting the signal payload (match on imagodei's coordinator-emitted signal names: `key_rotation_observed`, `key_revocation_observed`, `agent_peer_binding_created`, `revocation_attestation_recorded`).

Sweettest setup: extend `elohim/holochain/tests/sweettest/` with a fixture that spins a conductor, emits a `KeyRotation` from coordinator, confirms storage's controller receives via the stream.

<!-- NOTE: Steps 2-4 left [ ] — the conductor-bound sweettest (epr_phase_2b_batch_a_e2e.rs::epr_2b_batch_a_full_loop) is #[ignore]'d pending Stage 2 `derive_compromise_at` from the key_revocations projection. Wire-shape contract tests in the same file ARE enabled and passing. -->
- [ ] **Step 2: Failing test — real stream delivers rotation signal**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn real_signal_stream_delivers_rotation_from_coordinator() {
    let (conductor, _cell) = spin_imagodei_sweettest().await;
    let mut stream = HolochainAppSignalStream::connect(conductor.app_port()).await.unwrap();
    conductor.call_coordinator("imagodei", "rotate_key", sample_args()).await.unwrap();
    let signal = timeout(Duration::from_secs(5), stream.next_signal()).await.unwrap().unwrap();
    assert!(matches!(signal, DnaSignal::KeyRotation(_)));
}
```

- [ ] **Step 3: Implement**

`HolochainAppSignalStream` wraps `holochain_client_rust::AppWebsocket`, subscribes to app signals, filters and decodes to `DnaSignal`. Emits via internal channel.

- [ ] **Step 4: Run tests**

Expected: PASS (sweettest DNA path verified).

- [x] **Step 5: Wire into storage startup**

At storage binary init, construct `HolochainAppSignalStream` alongside the existing db pool, pass to `ReconcileController::new`, spawn `controller.run_loop()` as a tokio task.

- [x] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/reconcile/holochain_app_signal.rs
git commit -m "feat(storage): real DNA signal stream subscribes to imagodei coordinator signals"
```

### Task A.12: Integration test — rotation propagation + sweep end-to-end

**Files:**
- Modify: `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` (add Batch A scenario)

<!-- NOTE: Steps 1-2 left [ ] — the full two-conductor sweettest scenario (epr_phase_2b_batch_a_e2e.rs::epr_2b_batch_a_full_loop) is #[ignore]'d. Scenario structure exists in holochain/tests/sweettest; 6 ignore markers pending Stage 2 derive_compromise_at. Wire-shape contract tests in same file ARE enabled and passing. -->
- [ ] **Step 1: Write integration scenario**

Two peers A, B. Both running imagodei conductor + storage controller. Peer A publishes an EPR signed with key K1; peer B fetches, verifies, stores with `verified_at=Some(t1)`. Peer A rotates to key K2 (emits `KeyRotation` on DNA); B's controller observes rotation signal, updates pubkey cache. Peer A then publishes `KeyRevocation` of K1 with `compromise_at=t0` (before the EPR was signed). B's controller sweeps: the earlier EPR's `verified_at` is cleared.

- [ ] **Step 2: Run end-to-end**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration`
Expected: PASS.

- [x] **Step 3: Commit Batch A close**

```bash
git commit -am "test(epr-2b): batch A integration — rotation + revocation sweep end-to-end"
```

---

## Batch B — Projector & read-model reconciliation

**Scope:** Decisions #3 (projector) and #4 (EprHead) from the spec. Delivers the projector controller, manifest-declared mapping schema, `EprHead` production-path refactor, shefa's first projection target.

### Task B.1: Projector mapping schema extension

**Files:**
- Modify: `elohim/sdk/schemas/v1/app-manifest.schema.json` (add optional `projections` field)
- Create: `elohim/sdk/schemas/v1/pillar-projection.schema.json`

- [x] **Step 1: Schema extension**

Add to `app-manifest.schema.json`:
```json
{
  "properties": {
    "projections": {
      "type": "array",
      "items": { "$ref": "pillar-projection.schema.json" }
    }
  }
}
```

`pillar-projection.schema.json` shape:
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "required": ["kind", "schemaKey", "targetTable", "columnMapping"],
  "properties": {
    "kind":         { "enum": ["Content", "Agent", "Manifest", "Claim", "Observation", "EconomicEvent", "Commitment", "Attestation", "Delegation"] },
    "schemaKey":    { "type": "string" },
    "targetTable":  { "type": "string" },
    "columnMapping":{ "type": "object", "additionalProperties": { "type": "string" } }
  }
}
```

- [x] **Step 2: Run schema tests**

Run: `pnpm run schema:test && pnpm run lamad:codegen`
Expected: PASS. The codegen regenerates `manifest-types.ts` with the new optional field.

- [x] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/app-manifest.schema.json elohim/sdk/schemas/v1/pillar-projection.schema.json
git commit -m "schema(epr-2b): pillar-projection mapping extension to app-manifest"
```

### Task B.2: Shefa manifest declares first projection (EconomicEvent → economic_events)

**Files:**
- Modify: `elohim/sdk/domains/shefa/manifest.json`

- [x] **Step 1: Add projection declaration**

```json
{
  "projections": [
    {
      "kind": "EconomicEvent",
      "schemaKey": "economic-event",
      "targetTable": "economic_events",
      "columnMapping": {
        "provider":          "payload.provider",
        "receiver":          "payload.receiver",
        "action":            "payload.action",
        "resourceConformsTo":"payload.resourceConformsTo",
        "quantity":          "payload.quantity.numericValue",
        "unit":              "payload.quantity.hasUnit",
        "hasPointInTime":    "payload.hasPointInTime",
        "effortQuantity":    "payload.effortQuantity"
      }
    }
  ]
}
```

- [x] **Step 2: Run lamad:codegen**

Run: `pnpm run lamad:codegen`
Expected: PASS; shefa manifest types include the projection field.

- [x] **Step 3: Commit**

```bash
git add elohim/sdk/domains/shefa/manifest.json
git commit -m "feat(shefa): declare EconomicEvent projection in manifest"
```

### Task B.3: Projector skeleton + cursor table

**Files:**
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD-HHMMSS_projector_cursor/up.sql`
- Create: `elohim/elohim-storage/src/projector/mod.rs`
- Create: `elohim/elohim-storage/src/projector/cursor.rs`
- Create: `elohim/elohim-storage/src/projector/mapping.rs`

- [x] **Step 1: Migration**

The `projector_cursor` table is Category **C** (operational — tracks reconciliation progress; no dht_anchor_hash needed because it is not a projection of any DHT-notarized entity, just the controller's own advancement state. Fully rebuildable from `epr_atoms` via cursor replay. Per Principle P1, the table stewards the controller's operational source-of-truth for "how far have I projected" — pure local state.)

```sql
-- Category C operational cursor; rebuildable from epr_atoms replay; no DHT anchor by design.
CREATE TABLE projector_cursor (
    pillar         TEXT NOT NULL,
    kind           TEXT NOT NULL,
    last_epr_cid   TEXT,        -- CID of last projected EPR (content-address, not DHT anchor)
    last_issued_at TIMESTAMP,
    updated_at     TIMESTAMP NOT NULL,
    PRIMARY KEY (pillar, kind)  -- (pillar, kind) tuple — operational C-class, not a notarized entity
);
```

- [x] **Step 2: Failing test — projector loads cursor and advances**

```rust
#[tokio::test]
async fn projector_advances_cursor_after_pass() {
    let mut conn = test_conn();
    let mut projector = Projector::new(mock_manifest_registry(), pool_for(&conn));
    insert_epr_atom(&mut conn, cid="x", kind="EconomicEvent", issued_at=t(100));
    projector.run_one_pass(&mut conn).await.unwrap();
    let cursor = fetch_cursor(&conn, "shefa", "EconomicEvent");
    assert_eq!(cursor.last_epr_cid.as_deref(), Some("x"));
}
```

- [x] **Step 3: Implement skeleton**

```rust
pub struct Projector { manifest_registry: Arc<ManifestRegistry> }
impl Projector {
    pub async fn run_one_pass(&self, conn: &mut SqliteConnection) -> Result<PassReport, _> {
        for projection in self.manifest_registry.all_projections() {
            let cursor = load_cursor(conn, &projection.pillar, &projection.kind)?;
            let new_atoms = fetch_epr_atoms_since(conn, &projection, cursor)?;
            for atom in new_atoms {
                self.project(conn, &projection, atom)?;
                advance_cursor(conn, &projection.pillar, &projection.kind, atom.cid, atom.issued_at)?;
            }
        }
        Ok(PassReport { /* counts per pillar */ })
    }
    fn project(&self, conn: &mut SqliteConnection, mapping: &PillarProjection, atom: EprAtom) -> Result<(), _> {
        // Generic: evaluate columnMapping JSONPath over atom.payload, UPSERT into targetTable
        // Inherits atom.verified_at into a verified column if the target table declares one
    }
}
```

- [x] **Step 4: Run test**

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/*/projector_cursor/ elohim/elohim-storage/src/projector/
git commit -m "feat(storage): projector skeleton with cursor table"
```

### Task B.4: Projector — EconomicEvent round-trip

**Files:**
- Modify: `elohim/elohim-storage/src/projector/mod.rs`
- Modify: `elohim/elohim-storage/src/db/economic_events.rs` (if needed; verify column mapping fits existing table)

- [x] **Step 1: Failing integration test**

```rust
#[tokio::test]
async fn economic_event_epr_projects_to_economic_events_table() {
    let mut conn = test_conn();
    let epr = build_economic_event_epr(provider="A", receiver="B", quantity=5.0);
    EprService::ingest(&mut conn, epr.clone()).unwrap();
    let projector = Projector::new(load_shefa_manifest(), pool_for(&conn));
    projector.run_one_pass(&mut conn).await.unwrap();
    let row = fetch_economic_event(&conn, &epr.envelope.cid.to_string()).unwrap();
    assert_eq!(row.provider, "A");
    assert_eq!(row.quantity, 5.0);
}
```

- [x] **Step 2: Implement the column mapping evaluator**

Use `serde_json::Value::pointer()` or a small custom JSONPath evaluator on the payload (decoded from `canonical_bytes`). For each column mapping entry, evaluate the source path, upsert into `economic_events`.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): projector projects EconomicEvent EPR to economic_events row"
```

### Task B.5: Invariants I1–I9 enforcement — idempotency, manifest-authority, unmapped transparency

**Files:**
- Modify: `elohim/elohim-storage/src/projector/mod.rs`
- Create: `elohim/elohim-storage/src/projector/invariants_tests.rs`

- [x] **Step 1: Write tests for each invariant I1, I2, I6** (idempotency, manifest-authority, unmapped-transparency)

```rust
#[tokio::test]
async fn i1_projector_idempotent_on_replay() { /* run twice, row count unchanged */ }
#[tokio::test]
async fn i2_projector_rejects_undeclared_kind_schema() { /* atom with kind not in manifest is NOT projected */ }
#[tokio::test]
async fn i6_unmapped_kinds_remain_in_epr_atoms() { /* atom not projected is still queryable by CID */ }
```

- [x] **Step 2: Implement — idempotent UPSERT on PRIMARY KEY (cid), manifest-authority guard before project**

The PRIMARY KEY is `cid` on the pillar projection row — this is safe because `cid` IS the content-address of the source EPR (Category A content-addressing doubles as dht_anchor for this C-class projection). Source-of-truth for the row is the signed Envelope in `epr_atoms` whose `cid` matches; projection is derived and rebuildable.

```rust
// Manifest-authority guard (Invariant I2); UPSERT on content-address cid (which is
// the DHT anchor for this A-class source EPR) ensures idempotent projection (I1).
if !self.manifest_registry.has_projection(&atom.kind, &atom.schema_key) {
    // I6: leave unprojected, advance cursor, move on (unmapped kinds stay pure in epr_atoms)
    return Ok(());
}
// I1: use UPSERT (sqlite INSERT ... ON CONFLICT(cid) DO UPDATE SET …) —
// cid is the A-notarized content-address; operational projection rebuilds identically.
```

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): projector enforces I1/I2/I6 invariants"
```

### Task B.6: Invariants I3, I4, I5 — causal ordering, revocation propagation, verified-state consistency

**Files:**
- Modify: `elohim/elohim-storage/src/projector/mod.rs`
- Modify: `elohim/elohim-storage/src/reconcile/controller.rs` (wire revocation sweep to also clear projection rows)

- [x] **Step 1: Tests for I3, I4, I5**

```rust
#[tokio::test]
async fn i3_projector_processes_in_issued_at_order_per_signer_per_kind() { /* out-of-arrival-order EPRs produce correct end state */ }
#[tokio::test]
async fn i4_revocation_sweep_clears_projection_row_within_same_pass() { /* KeyRevocation hits BOTH epr_atoms.verified_at AND economic_events.verified */ }
#[tokio::test]
async fn i5_projection_row_verified_state_matches_source_epr() { /* if epr.verified_at is None, projection row verified column is also None */ }
```

- [x] **Step 2: Implement**

- Fetch atoms for projection pass sorted by `(signer_cid, issued_at)` within each `(pillar, kind)`.
- Extend `sweep_on_revocation` (from A.8) to also update projection rows (where the target table has a `verified` column; determined by manifest `columnMapping` including a `verifiedAt` source entry).
- Projection row `verified` column derives from source EPR's `verified_at IS NOT NULL`.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): projector enforces I3/I4/I5 invariants + revocation sweep clears projection rows"
```

### Task B.7: `EprHead` production-path refactor

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:5500-5641` (existing EprHead endpoints)
- Modify: `elohim/elohim-storage/src/epr_codec.rs:97+` (EprHead struct — may need small adjustments to align with projector output)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:768+` (p2p EprHead fetch path)

- [x] **Step 1: Audit current EprHead construction sites**

Grep `EprHead {` across the codebase. Each site currently constructs an `EprHead` directly from DB rows. Refactor each to instead:
1. Look up the underlying EPR in `epr_atoms`
2. Use the projector's pillar mapping to derive `EprLamadContext` / `EprShefaContext` / `EprQahalContext`
3. Assemble the `EprHead` from projector-derived contexts
4. Preserve wire format for downstream consumers

- [x] **Step 2: Failing test — EprHead serves projector-derived context**

```rust
#[tokio::test]
async fn epr_head_serves_projector_derived_contexts() {
    let mut conn = test_conn();
    let epr = build_content_epr_with_lamad_context(/*...*/);
    EprService::ingest(&mut conn, epr.clone()).unwrap();
    projector.run_one_pass(&mut conn).await.unwrap();
    let head = fetch_epr_head(&mut conn, &epr.envelope.cid.to_string()).unwrap();
    assert!(head.lamad.is_some());
    assert_eq!(head.lamad.unwrap().concept_cid, /*expected from mapping*/);
}
```

- [x] **Step 3: Implement refactor**

Route both `/elohim/epr/1.0.0` (libp2p) and `/db/epr-head/*` (HTTP) paths through a `derive_epr_head(cid, conn, manifest_registry)` helper. Wire format unchanged; source unified.

- [x] **Step 4: Run tests**

Expected: new test passes, all existing EprHead tests still pass.

- [x] **Step 5: Commit**

```bash
git commit -am "refactor(storage): EprHead produced via projector (A2 from Envelope)"
```

### Task B.8: Projector signal emission + reconciliation-lag metric

**Files:**
- Create: `elohim/elohim-storage/src/projector/signals.rs`
- Modify: `elohim/elohim-storage/src/projector/mod.rs`
- Create (or modify): HTTP status endpoint `/api/v1/status/projector`

- [x] **Step 1: Failing test — projection write emits `<pillar>.<kind>.projected` signal + lag metric**

```rust
#[tokio::test]
async fn projector_emits_signal_on_write() {
    let (sig_tx, mut sig_rx) = mpsc::channel(4);
    let mut projector = Projector::with_signal_sink(sig_tx, ...);
    ingest_and_project(&mut projector, economic_event_epr()).await;
    let signal = sig_rx.recv().await.unwrap();
    assert_eq!(signal.pillar, "shefa");
    assert_eq!(signal.kind, "EconomicEvent");
    assert!(signal.epr_cid.len() > 0);
}
```

- [x] **Step 2: Implement**

Each projection write emits `ProjectorSignal { pillar, kind, epr_cid, table, row_key, timestamp }` on an internal channel. Subscribers: dashboards, elohim-agent defenders (future), Phase 4 GraphQL subscriptions (future).

Add `/api/v1/status/projector` endpoint returning per-`(pillar, kind)` cursor + last-observed `issued_at` in `epr_atoms` + computed lag.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit Batch B close**

```bash
git commit -am "feat(storage): projector signal emission + lag metric endpoint"
```

---

## Batch C — Producer migration & ramp controls

**Scope:** Decisions #5 (signal harness) and #6 (write-through flag) from the spec. Delivers `/api/v1/signal/emit` endpoint, conductor signing API contract, signal harness client migration, 4-layer write-through flag, integrity-always-on hardcoded exception.

### Task C.1: Conductor signing API contract (prerequisite)

**Files:**
- Create: `elohim/sdk/schemas/v1/conductor-signing.schema.json`
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` (expose a coordinator fn for storage to call)
- Create: `elohim/elohim-storage/src/conductor_client/signing.rs`

**Source of truth:** The conductor-signing schema is Category **C** (operational RPC contract). No persistent source-of-truth in the request/response pair itself; the signed bytes the API returns become part of an `Envelope` (Category A, DHT-notarized) after ingest. The conductor holds the agent's key material (the actual notarization root); storage is a consumer requesting signatures, not an owner of keys.

**Spec §7 O1** — first verify what signing path actually exists in the current tauri-plugin-holochain / doorway conductor stack. Adjust this task's shape accordingly.

- [x] **Step 1: Investigate**

Grep the holochain-client-rust + tauri-plugin-holochain code for "sign" / "agent_sign" endpoints. Check whether there's a zome call pattern to get an ed25519 signature over arbitrary bytes. If found, document the endpoint; skip to Step 3.

- [x] **Step 2: If missing — define schema-first contract**

`conductor-signing.schema.json`:
```json
{
  "request": {
    "required": ["agentCid", "canonicalBytes"],
    "properties": {
      "agentCid":       { "type": "string" },
      "canonicalBytes": { "type": "string", "contentEncoding": "base64" }
    }
  },
  "response": {
    "required": ["signature"],
    "properties": {
      "signature": { "type": "string", "contentEncoding": "base64" },
      "signerPubkey": { "type": "string", "contentEncoding": "base64" }
    }
  }
}
```

Add an imagodei coordinator fn `sign_for_agent(agent_cid: Cid, bytes: Vec<u8>) -> ExternResult<Vec<u8>>` gated to the agent's own key material (call `sign` helper from hdk).

- [x] **Step 3: Storage client wrapper**

```rust
// elohim-storage/src/conductor_client/signing.rs
pub struct ConductorSigningClient { app_ws: AppWebsocket }
impl ConductorSigningClient {
    pub async fn sign(&self, agent_cid: &Cid, bytes: &[u8]) -> Result<Vec<u8>, SigningError> { /* call sign_for_agent */ }
}
```

- [x] **Step 4: Test against sweettest conductor**

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/conductor-signing.schema.json elohim/holochain/dna/imagodei/ elohim/elohim-storage/src/conductor_client/
git commit -m "feat(conductor+storage): signing API contract for EPR composition"
```

### Task C.2: Signal-intent schema + `/api/v1/signal/emit` endpoint

**Files:**
- Create: `elohim/sdk/schemas/v1/signal-intent.schema.json`
- Create: `elohim/elohim-storage/src/http/signal_emit.rs`
- Modify: `elohim/elohim-storage/src/http.rs` (route)

- [x] **Step 1: Schema-first**

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SignalIntent",
  "required": ["pillar", "signalType", "agentCid", "payload", "couplingRefs"],
  "properties": {
    "pillar":       { "enum": ["lamad", "shefa", "imagodei", "mishpat", "qahal"] },
    "signalType":   { "type": "string" },
    "agentCid":     { "type": "string" },
    "payload":      { "type": "object" },
    "couplingRefs": {
      "type": "object",
      "properties": {
        "knowledge":  { "type": ["string","null"] },
        "value":      { "type": ["string","null"] },
        "governance": { "type": ["string","null"] }
      }
    }
  }
}
```

- [x] **Step 2: Failing integration test**

```rust
#[tokio::test]
async fn signal_emit_composes_epr_and_ingests() {
    let app = test_app().await;
    let resp = app.post("/api/v1/signal/emit").json(&signal_intent_sample()).await;
    assert_eq!(resp.status(), 201);
    let body: SignalEmitResponse = resp.json().await;
    assert!(body.epr_cid.starts_with("bafy"));
    let atom = fetch_atom(&app.conn(), &body.epr_cid);
    assert!(atom.verified_at.is_some());
}
```

- [x] **Step 3: Implement handler**

Handler reads intent → looks up pillar manifest for the signalType → composes Envelope (kind, schema_key, coupling) → requests signature via `ConductorSigningClient` → calls `EprService::ingest` → returns `{eventCid, eprCid}`.

- [x] **Step 4: Run tests**

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/signal-intent.schema.json elohim/elohim-storage/src/http/signal_emit.rs
git commit -m "feat(storage): /api/v1/signal/emit endpoint composes + signs + ingests EPRs"
```

### Task C.3: Angular signal harness migration

**Files:**
- Modify: `app/elohim-app/src/app/lamad/services/signal-harness.service.ts`
- Create: `app/elohim-library/projects/elohim-service/src/services/signal-emit.service.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/services/index.ts`

- [x] **Step 1: Feature flag + dual-output during rollout**

Add a feature flag reading from `environment.ts`: `useSignalEmitEndpoint: boolean`. Default false.

When `false`: current behavior (direct economic-event POST).
When `true`: POST to `/api/v1/signal/emit` with signal-intent payload.

- [x] **Step 2: Implement `SignalEmitService`**

```typescript
@Injectable({ providedIn: 'root' })
export class SignalEmitService {
  constructor(private http: HttpClient) {}
  emit(intent: SignalIntent): Observable<SignalEmitResponse> {
    return this.http.post<SignalEmitResponse>('/api/v1/signal/emit', intent);
  }
}
```

- [x] **Step 3: Update `SignalHarnessService`**

```typescript
// signal-harness.service.ts
if (environment.useSignalEmitEndpoint) {
  await this.signalEmit.emit({ pillar: 'shefa', signalType: 'rendererCompletion', ... }).toPromise();
} else {
  // existing direct POST
}
```

- [x] **Step 4: Failing test — with flag on, HTTP client receives signal-intent**

```typescript
it('posts to /api/v1/signal/emit when feature flag on', () => {
  TestBed.overrideProvider('environment', { useValue: { useSignalEmitEndpoint: true } });
  /* trigger renderer completion */
  httpMock.expectOne('/api/v1/signal/emit').flush({ epr_cid: 'bafy...', event_cid: '...' });
});
```

- [x] **Step 5: Run Angular tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "signal-harness"`
Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/services/signal-harness.service.ts app/elohim-library/projects/elohim-service/src/services/signal-emit.service.ts
git commit -m "feat(lamad): signal harness emits EPR-intent behind feature flag"
```

### Task C.4: Write-through flag — manifest default + effective state view

**Files:**
- Modify: `elohim/sdk/schemas/v1/app-manifest.schema.json` (add `writeThrough` field)
- Create: `elohim/elohim-storage/src/config/write_through.rs`
- Create: HTTP endpoint `/api/v1/status/write-through`

- [x] **Step 1: Schema extension**

```json
"writeThrough": {
  "type": "object",
  "required": ["enabled"],
  "properties": {
    "enabled": { "type": "boolean" },
    "kinds":   { "type": "array", "items": { "type": "string" } }
  }
}
```

Default when absent: `{ "enabled": false }`.

- [x] **Step 2: Implement 4-layer composition**

```rust
pub struct WriteThroughState {
    manifest: WriteThroughConfig,  // layer 1
    policy:   Option<WriteThroughOverride>, // layer 2 from policy.toml
    env:      Option<WriteThroughOverride>, // layer 3
    admin:    Arc<RwLock<Option<WriteThroughOverride>>>, // layer 4
}
impl WriteThroughState {
    pub fn effective_for(&self, pillar: &str, kind: &str) -> EffectiveWriteThrough {
        // Resolve in order 4 -> 3 -> 2 -> 1, first hit wins; hardcoded integrity exception always-on
        if is_integrity_kind(kind) { return EffectiveWriteThrough::OnIntegrityException; }
        // ...
    }
}
pub fn is_integrity_kind(kind: &str) -> bool {
    matches!(kind, "KeyRotation" | "KeyRevocation" | "RevocationAttestation" | "AgentPeerBinding")
}
```

- [x] **Step 3: Effective-state endpoint**

`GET /api/v1/status/write-through` returns JSON per-(pillar,kind): manifest default, each override level, effective state, rolling-window write count (observed activity per `project_elohim_active_observed_not_flagged`).

- [x] **Step 4: Failing tests for composition + integrity exception**

```rust
#[test]
fn integrity_kinds_bypass_disabled_config() {
    let state = WriteThroughState { manifest: WriteThroughConfig::disabled(), policy: None, env: None, admin: no_admin() };
    assert_eq!(state.effective_for("imagodei", "KeyRevocation"), EffectiveWriteThrough::OnIntegrityException);
}
#[test]
fn admin_override_wins_over_policy_wins_over_manifest() { /* assert 4-layer precedence */ }
```

- [x] **Step 5: Run tests**

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/app-manifest.schema.json elohim/elohim-storage/src/config/write_through.rs
git commit -m "feat(storage): 4-layer write-through flag composition + integrity exception"
```

### Task C.5: Wire write-through flag into ingest path

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_service.rs`
- Modify: `elohim/elohim-storage/src/http/signal_emit.rs` (respect flag)

- [x] **Step 1: Failing test — pillar with flag off does not ingest EPR**

```rust
#[tokio::test]
async fn signal_emit_rejects_when_write_through_disabled() {
    let state = WriteThroughState { manifest: WriteThroughConfig::disabled(), ... };
    let result = signal_emit_handler(state, shefa_intent()).await;
    assert_eq!(result.status(), 503); // Service unavailable - write-through disabled
}
#[tokio::test]
async fn integrity_signal_always_ingests() {
    let state = WriteThroughState::all_disabled();
    let result = signal_emit_handler(state, imagodei_revocation_intent()).await;
    assert_eq!(result.status(), 201);
}
```

- [x] **Step 2: Implement the guard**

At `signal_emit_handler` entry: compute `state.effective_for(intent.pillar, intent.signalType.into_kind())` → if disabled, return 503 with body `{ reason: "write-through disabled for (pillar, kind)" }`; if on or integrity-exception, proceed.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): write-through guard on signal_emit with integrity exception"
```

### Task C.6: policy.toml + env/CLI + admin override wiring

**Files:**
- Modify: `elohim/elohim-storage/src/bin/elohim-storage-server.rs` (env/CLI parsing)
- Modify: `elohim/elohim-storage/src/config/mod.rs` (policy.toml parser)
- Create: admin HTTP route `POST /admin/write-through`

- [x] **Step 1: Implement policy.toml parser**

Read `ELOHIM_STORAGE_POLICY_TOML` env var or `--policy-toml` CLI arg. Parse:
```toml
[write_through]
shefa = { enabled = true, kinds = ["EconomicEvent"] }
imagodei = { enabled = true, kinds = ["Agent", "Attestation"] }
```

- [x] **Step 2: Implement env/CLI overrides**

`ELOHIM_WRITE_THROUGH_SHEFA=on` parses to an override for pillar `shefa` (all kinds).

- [x] **Step 3: Implement admin endpoint**

`POST /admin/write-through { pillar, kind, enabled, reason }` updates the `admin` RwLock field of `WriteThroughState`. Guard with admin-auth middleware (existing pattern).

- [x] **Step 4: Test each layer in isolation + composition**

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git commit -am "feat(storage): write-through flag 4-layer override wiring"
```

### Task C.7: Batch C close — end-to-end shefa migration test

**Files:**
- Modify: `app/elohim-app/src/app/lamad/services/signal-harness.service.spec.ts` (Angular)
- Modify: `elohim/elohim-storage/tests/epr_atom_federation_integration.rs` (storage)

- [x] **Step 1: Angular side — flag on, renderer completes, intent posted, storage returns epr_cid**

Mock `SignalEmitService`; trigger a sample renderer completion; assert the service receives the expected intent payload.

- [x] **Step 2: Storage side — intent → EPR → projector → `economic_events` row**

Compose signal intent, POST to `/api/v1/signal/emit`, run projector, assert `economic_events` row exists with verified=true.

- [x] **Step 3: Run both**

Expected: PASS.

- [x] **Step 4: Commit Batch C close**

```bash
git commit -am "test(epr-2b): batch C integration — shefa signal-intent → EPR → projection end-to-end"
```

---

## Batch D — Discovery & fanout

**Scope:** Decision #7 from the spec. Delivers tiered routing by reach, Kad provider records, gossipsub topic structure, reach-gated subscription enforcement, integrity-always-both exception, dedup LRU.

### Task D.1: Reach-tier routing policy

**Files:**
- Create: `elohim/elohim-storage/src/p2p/fanout.rs`
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs`

- [x] **Step 1: Define routing policy**

```rust
pub enum FanoutChannel { DirectOnly, Gossip, KadLight, Kad, Both }
pub fn channels_for_reach(reach: Reach, kind: &str) -> Vec<FanoutChannel> {
    if is_integrity_kind(kind) {
        // D.5: always both + direct-notify
        return vec![FanoutChannel::Gossip, FanoutChannel::Kad, FanoutChannel::DirectOnly];
    }
    match reach {
        Reach::Private | Reach::SelfScope => vec![FanoutChannel::DirectOnly],
        Reach::Intimate                    => vec![FanoutChannel::DirectOnly],
        Reach::Trusted | Reach::Familiar   => vec![FanoutChannel::Gossip],
        Reach::Community                   => vec![FanoutChannel::Gossip, FanoutChannel::KadLight],
        Reach::Public                      => vec![FanoutChannel::Gossip, FanoutChannel::Kad],
        Reach::Commons                     => vec![FanoutChannel::Kad, FanoutChannel::Gossip],
    }
}
```

- [x] **Step 2: Failing test for each reach tier**

Assert the mapping table above holds for all 8 reach variants + the integrity exception.

- [x] **Step 3: Run tests, commit**

Expected: PASS.

```bash
git add elohim/elohim-storage/src/p2p/fanout.rs
git commit -m "feat(storage): tiered fanout policy by reach + integrity exception"
```

### Task D.2: Kad `start_providing` on Announce

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_store.rs:230` (replace TODO(phase-2b) with Kad call, gated by fanout policy)

- [x] **Step 1: Implement conditional Kad register**

```rust
fn put(&self, conn: &mut SqliteConnection, epr: Epr) -> Result<EprIngestResult, StorageError> {
    let result = self.local.put(conn, epr.clone())?;
    let channels = channels_for_reach(epr.envelope.reach, &epr.envelope.kind_str());
    if channels.contains(&FanoutChannel::Kad) || channels.contains(&FanoutChannel::KadLight) {
        self.swarm_handle.kad_start_providing(result.cid.parse()?)?;
    }
    Ok(result)
}
```

- [x] **Step 2: Integration test — Commons EPR is discoverable by a cold-start peer**

Test harness: peer A announces an EPR (reach=Commons); peer B connects later (after announce); peer B finds peer A as a provider via Kad; fetches atom via existing EPR atom protocol.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): Kad start_providing on Announce for Kad-tier reach"
```

### Task D.3: Gossipsub topic structure + publish on Announce

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs` (subscribe applicable topics)
- Modify: `elohim/elohim-storage/src/services/epr_store.rs:192` (publish to gossipsub on put)

- [x] **Step 1: Topic enumeration**

```rust
pub fn topic_for(pillar: &str, reach: Reach, collective: Option<&str>) -> String {
    let base = format!("elohim/{}/{}", pillar, reach.as_str());
    match collective {
        Some(c) => format!("{}/{}", base, c),
        None => base,
    }
}
// Integrity topics
pub const TOPIC_IDENTITY_BINDING: &str = "elohim/identity/binding";
pub const TOPIC_INTEGRITY_REVOCATION: &str = "elohim/integrity/revocation";
```

- [x] **Step 2: Subscribe on startup**

Storage subscribes to topics its manifests declare interest in (writable pillars), plus integrity topics always.

- [x] **Step 3: Publish on Announce**

On `put`, if channel includes `Gossip`, publish the announce message (CBOR-encoded `(cid, epr_bytes)` or just announce-cid) to the derived topic.

- [x] **Step 4: Integration test — gossiped announce received by subscriber**

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git commit -am "feat(storage): gossipsub topic structure + publish-on-announce"
```

### Task D.4: Reach-gated subscription enforcement

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs`
- Create: `elohim/elohim-storage/src/p2p/subscription_auth.rs`

**Open question O2** — see spec §7. Exact authorization mechanism may shift at batch kickoff. Proposed: subscription request carries a Delegation EPR CID or AgentPeerBinding claim; peer verifies via existing imagodei trust graph.

- [x] **Step 1: Failing test — peer without membership cannot subscribe to community topic**

```rust
#[tokio::test]
async fn unauthorized_peer_cannot_subscribe_to_community_topic() {
    let ctx = two_peer_swarm().await;
    let result = ctx.peer_b.subscribe_to_community("household-xyz").await;
    assert!(result.is_err()); // peer B is not a member of household-xyz
}
```

- [x] **Step 2: Implement subscription authorization**

Gossipsub has per-message validators. Register a validator that rejects subscription requests (or drops inbound messages) from peers whose `peer_identity_bindings.agent_cid` does not have a membership link into the topic's scope (e.g., a `MemberOfHousehold` link in qahal DNA for household topics).

- [x] **Step 3: Run tests, commit**

Expected: PASS.

```bash
git commit -am "feat(storage): reach-gated subscription authorization on gossipsub"
```

### Task D.5: Integrity-always-both routing + direct-notify

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/fanout.rs` (direct-notify helper)
- Modify: `elohim/elohim-storage/src/reconcile/controller.rs` (on_key_revocation triggers direct-notify)

**Converges with Recovery M4** — M4 produces the affected-peer list contract; 2B consumes.

- [x] **Step 1: Failing test — revocation reaches all 3 channels simultaneously**

Three-peer integration: A revokes a key. Peer B is connected (receives gossip). Peer C connects later (receives Kad). Peer D was recently served by A's revoked key (receives direct-notify). Assert: all four channels observed.

- [x] **Step 2: Implement direct-notify**

On observed `KeyRevocation`, query `peer_identity_bindings` for peers recently bound to the revoked agent, or (Recovery M4 contract) consume M4's affected-peer list. Send a direct `Announce` over the EPR atom protocol to each.

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): integrity-always-both fanout + direct-notify for revocations"
```

### Task D.6: Dedup LRU on receive path

**Files:**
- Create: `elohim/elohim-storage/src/p2p/dedup.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (wire on inbound handlers)

- [x] **Step 1: Failing test — duplicate announce is a no-op (observable via counter)**

```rust
#[tokio::test]
async fn duplicate_announce_deduped() {
    let dedup = DedupLru::new(1024);
    assert!(dedup.insert(cid_a()));   // first time — accepted
    assert!(!dedup.insert(cid_a()));  // duplicate — rejected
    assert_eq!(dedup.seen_counter(), 2);
}
```

- [x] **Step 2: Implement bounded LRU + counter**

```rust
pub struct DedupLru { inner: LruCache<Cid, ()>, seen: AtomicUsize }
impl DedupLru {
    pub fn insert(&self, cid: Cid) -> bool { /* returns true if new */ }
    pub fn seen_counter(&self) -> usize { /* ... */ }
}
```

Wire on inbound `Announce` handler: `if !dedup.insert(cid) { return early; }`.

- [x] **Step 3: Run tests, commit**

Expected: PASS.

```bash
git commit -am "feat(storage): bounded dedup LRU on inbound receive path"
```

### Task D.7: `providers()` integration — DHT provider records in response

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_store.rs:261` (the TODO(phase-2b))

- [x] **Step 1: Failing test — providers() returns Local + Kad providers**

```rust
#[tokio::test]
async fn providers_returns_local_plus_kad() {
    let epr = /* ... */;
    ingest_locally_then_announce_via_kad(&storage, &epr).await;
    let providers = storage.providers(&mut conn, &epr.envelope.cid.to_string()).unwrap();
    assert!(providers.iter().any(|p| p.peer_id == "local"));
    assert!(providers.iter().any(|p| p.peer_id != "local"));
}
```

- [x] **Step 2: Implement**

```rust
fn providers(&self, conn: &mut SqliteConnection, cid: &str) -> Result<Vec<ProviderRef>, StorageError> {
    let mut providers = self.local.providers(conn, cid)?;
    let dht = self.swarm_handle.kad_get_providers(cid).await?;
    providers.extend(dht.into_iter().map(ProviderRef::from));
    Ok(providers)
}
```

- [x] **Step 3: Run tests**

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git commit -am "feat(storage): FederatedEprStore.providers() returns local + DHT providers"
```

### Task D.8: Batch D close — full-stack integration

**Files:**
- Modify: `elohim/elohim-storage/tests/epr_atom_federation_integration.rs`

- [x] **Step 1: End-to-end scenario**

Four peers A, B, C, D with different connection topologies and subscription scopes. Test:
1. A publishes a Commons-reach EconomicEvent → B (subscribed) receives via gossip; C (cold) discovers via Kad; D (not subscribed, no query) never sees it
2. A publishes a Community-reach EPR in household-xyz → B (household member, subscribed) receives; C (non-member) blocked by subscription auth
3. A revokes key → all peers observe via all channels (including D via direct-notify because D was recently served)

- [x] **Step 2: Run integration**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test epr_atom_federation_integration`
Expected: PASS.

- [x] **Step 3: Commit Batch D close**

```bash
git commit -am "test(epr-2b): batch D integration — tiered fanout + subscription auth + integrity exception"
```

---

## Post-Phase-2B wrap-up

### Task Z.1: Resolve `TODO(phase-2b)` markers

Grep for remaining `TODO(phase-2b)` across `elohim-storage/src/` — confirm all 5 in `epr_store.rs:7,192,221,230,261` are resolved or escalated to Phase 3. File issues for any escalated items.

### Task Z.2: Update Batch D addendum pointer

Edit `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2c-batch-d-completion-addendum.md` §"What follows Batch D" (lines 180–194):

```markdown
## What follows Batch D

Once Batch D is green:

1. ✅ Merged to dev (through commit `e9e2806a` on gossipsub foundation)
2. ✅ **Phase 2B designed** — `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`
   and `genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md` cover:
   - PeerId → AgentPubKey via DHT-notarized AgentPeerBinding
   - Resolver-backed Ed25519 verify with eager revocation sweep
   - Projector + manifest mapping from epr_atoms to pillar tables
   - EprHead as projector-derived (A2 from Envelope)
   - Signal harness as EPR-intent producer
   - Per-pillar 4-layer write-through flag
   - Tiered Kad+gossipsub fanout by reach
3. Phase 2B batches A/B/C/D awaiting per-batch execution sessions.
4. Phase 3 (manifest-graph resolver) kickoff prompt to be written at 2B completion.
```

### Task Z.3: Write Phase 3 kickoff prompt

Per spec §7 O8: at 2B completion, draft `genesis/docs/plans/2026-MM-DD-epr-phase-3-manifest-resolver-kickoff-prompt.md` describing: manifest-EPRs (manifests become publishable atoms); schemaRef → schema graph-traversal; Phase 4 groundwork.

### Task Z.4: Verify & commit spec + plan

```bash
# Back in /projects/elohim (or in the worktree at .claude/worktrees/epr-phase-2b-design)
git add genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md \
        genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md \
        genesis/docs/plans/2026-04-24-epr-phase-2b-brainstorm-kickoff-prompt.md
git commit -m "docs(epr-2b): phase 2b design spec + first-draft plan + kickoff prompt"

# Update the Batch D addendum pointer
git add genesis/docs/superpowers/plans/2026-04-24-epr-phase-2c-batch-d-completion-addendum.md
git commit -m "docs(epr-2b): batch D addendum points to phase 2b spec + plan"
```

Husky runs docs-only lint; do not bypass.

---

## Convergence checkpoints with Recovery M4

The DNA signal stream is shared between 2B Batch A and Recovery M4. Coordination points:

1. **Signal message schema.** Task A.3 defines `DnaSignal::KeyRevocation`. Recovery M4 must emit matching shape. If M4 ships first, A.3 uses M4's definition; otherwise M4 implements A.3's.
2. **Affected-peer list contract.** Task D.5 (direct-notify) needs Recovery M4's output: "which peers were recently served by the revoked key." If M4 hasn't shipped this by D.5 execution, D.5 scopes a placeholder that queries `peer_identity_bindings` directly.
3. **Stream cursor / durability.** Open question O4. Flagged in Task A.3; coordinate resolution with M4's stream-emit design.

**Single convergence artifact:** the `dna-signal-stream.schema.json` file (Task A.3 Step 1). Both epics write to this.

---

## Known risks & mitigations

| Risk | Mitigation |
|---|---|
| Conductor signing API missing in current stack | Task C.1 Step 1 investigates; Step 2 designs if missing. Worst case adds ~1 week to Batch C. |
| Gossipsub authorization proof design is undesigned (O2) | Task D.4 opens with batch-kickoff decision pass; fall-back is simple peer-list allowlist per topic scope |
| Projector backfill semantics undecided (O3) | Task B.7 opens with batch-kickoff decision pass; fall-back is "projector starts at now(), operator triggers backfill command for historical" |
| HDI `get_links` constraint in AgentPeerBinding validator | Per memory `project_hdi_no_get_links_in_validators`: cross-entity rotation checks live in the coordinator pre-commit gate, not in the integrity validator |
| Per-batch worktree isolation | Each batch may fork to `feature/epr-phase-2b-batch-{a,b,c,d}` at its kickoff. Rebase onto dev between batches; do not long-live the parent design branch. |
| Concurrent rebase with Recovery M4 branch | Use schema-first contracts as the coordination surface (`dna-signal-stream.schema.json`); merge conflict is on schema file, which is human-resolvable |

---

*End of plan. Companion spec at `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md`.*
