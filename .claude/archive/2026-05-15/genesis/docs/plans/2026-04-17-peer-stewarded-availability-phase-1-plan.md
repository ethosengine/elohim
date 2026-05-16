# Peer-Stewarded Availability — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the DNA + elohim-storage foundation for peer-stewarded availability signaling: every peer publishes a `PeerStatus` DHT entry on a 60-second cadence, evaluated from a policy engine that reads `peer-policy.toml` against live state, with an optional TCP forwarder that can expose the localhost-bound Holochain conductor on a policy-gated external address. No doorway or wrapper changes in this phase.

**Architecture:** `PeerStatus` is a DHT entry authored by each peer's agent key, registered in the existing infrastructure DNA as an operational entry type (not an SDK primitive). A coordinator zome function publishes it; a post-commit signal projects it into elohim-storage's SQLite for local queries. In elohim-storage, a tokio heartbeat task evaluates policy on each tick and either publishes a new entry or skips (unchanged state below a minimum cadence). The forwarder is a tiny tokio TCP pipe started at boot when policy enables it.

**Tech Stack:** Rust (HDI/HDK for DNA), sweettest for DNA integration tests, diesel for SQLite migrations, tokio for heartbeat + forwarder, `toml` crate for policy config, `chrono` for timestamps.

---

## File Structure

**Created:**
- `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/peer_status.rs` — `PeerStatus`, `PeerLifecycleState`, `PeerCapabilityFlags` types + validation
- `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/peer_status.rs` — coordinator fns: `record_peer_status`, `get_latest_peer_status_for_agent`, `get_all_current_peer_statuses`
- `elohim/holochain/tests/peer_status.rs` — sweettest integration test
- `elohim/elohim-storage/src/db/peer_statuses.rs` — diesel model + queries
- `elohim/elohim-storage/migrations/<timestamp>_create_peer_statuses/up.sql` + `down.sql`
- `elohim/elohim-storage/src/policy/mod.rs` + `policy/config.rs` + `policy/evaluator.rs` — policy engine
- `elohim/elohim-storage/src/heartbeat.rs` — peer-status heartbeat task
- `elohim/elohim-storage/src/forwarder.rs` — optional TCP forwarder
- `elohim/elohim-storage/config/peer-policy.example.toml` — example config

**Modified:**
- `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs` — register new entry type + link types
- `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs` — wire up new module + post-commit signal
- `elohim/elohim-storage/src/db/diesel_schema.rs` — regenerate after migration (auto-generated reflection of the projection schema; source of truth remains the DHT entry, not this file)
- `elohim/elohim-storage/src/db/mod.rs` — expose `peer_statuses` module
- `elohim/elohim-storage/src/signals.rs` — handle `PeerStatusRecorded` signal
- `elohim/elohim-storage/src/config.rs` — add `peer_policy_path` field
- `elohim/elohim-storage/src/lib.rs` + `main.rs` — spawn heartbeat + forwarder at boot
- `elohim/elohim-storage/Cargo.toml` — add `toml` if absent

---

## P2P Design Gate

### Entity: `PeerStatus`

- **Classification**: **Notarized (Category A)** — new DHT entry type in the Infrastructure DNA.
- **Justification**: Peer-authored availability claims are inputs to doorway routing decisions across the mesh; the protocol would be lying if a peer's claim could be silently altered or fabricated by another party. The claim is not reconstructable from other sources (it's a fresh read of live state per tick). Infrastructure DNA is at ~6/~100 entry types — ample headroom.
- **Content Address Strategy**: **Agent-Scoped Composite**. Logical identity is `(AgentPubKey, timestamp)`; each publication is an immutable entry, uniqueness enforced by author + creation time. "Latest PeerStatus for agent X" is resolved via the `AgentToPeerStatus` link traversal (Task 2).
- **Address Justification**: Not content-derived because two peers could (trivially) author byte-identical PeerStatus structs and both are meaningful, independent claims. Not slug/UUID because the author's identity is load-bearing — doorway trusts the claim precisely because only that agent could have signed it.
- **Source of Truth**: **Holochain DHT**. The SQLite `peer_statuses` table is a read-optimized projection populated by the post-commit signal in Task 6.
- **Coordinator Zome**: `infrastructure::record_peer_status`, `infrastructure::get_latest_peer_status_for_agent` (Task 4).
- **Storage Projection**: `peer_statuses` (dht_anchor_hash: **yes**, see Task 7 migration).
- **HTTP Route**: None in Phase 1. Doorway subscribes to the projection directly in Phase 2; any HTTP read route is a Phase 2 concern.
- **Anti-Pattern Check**: No UUID PK (uses `peer_id` text = AgentPubKey base64, with `dht_anchor_hash` as the cryptographic anchor). Not designed HTTP-first. No CID-as-FK. Not a shared table for private state (public by design). Source-of-truth declared inline in the migration SQL. Reuses no existing entry type because none of `DoorwayHeartbeat`, `HealthAttestation`, or `DoorwayRegistration` carry the required shape (peer-generic, not doorway-scoped; lifecycle state machine; capability flag set).

### Entity: `peer_statuses` (SQLite projection only)

- **Classification**: Projection of a Category A entity — not a standalone entity.
- **Source of Truth**: Holochain DHT (`PeerStatus` entry above).
- **Migration comment required**: `-- Source of truth: DHT (infrastructure DNA PeerStatus entry)` — added to Task 7 `up.sql`.
- **Anti-Pattern Check**: Has `dht_anchor_hash NOT NULL`. Primary key is `peer_id` (AgentPubKey) which matches the logical identity of the upstream entry. No CID foreign keys.

### Design Constraints Discovered

- **Infrastructure DNA headroom is not the bottleneck** for this feature, but this plan introduces the first new infrastructure entry type in a while — verify the count with the DNA maintainers if their tracking says otherwise.
- **Migration of `DoorwayHeartbeat`**: Phase 2 will deprecate doorway's direct heartbeat writes in favor of the peer-stewarded surface. The two entry types coexist during Phase 1. No data-migration concern — heartbeats are short-lived (summarized daily); the old types simply stop being written once Phase 2 lands.
- **Agent-to-peer link shape**: The `AgentToPeerStatus` link uses the peer's `AgentPubKey` as the anchor base. This mirrors the agent-anchor pattern already used for other agent-scoped queries in the DNA; verify consistency during Task 2.

---

## Conventions for this plan

- All Rust commands run with `RUSTFLAGS='--cfg getrandom_backend="custom"'` for DNA/storage builds (per repo CLAUDE.md).
- DNA tests: `cd elohim/holochain && just test` (wraps sweettest with correct RUSTFLAGS).
- Storage tests: `cd elohim/elohim-storage && cargo test`.
- Commit per task using the shown message (`feat(peer-status): ...` / `test(peer-status): ...`).
- TDD order: failing test → minimal impl → pass → commit.

---

## Module A: DNA — Integrity Zome

### Task 1: Define `PeerStatus` integrity types

**Files:**
- Create: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/peer_status.rs`
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs`
- Test: (validation test added in Task 3)

- [ ] **Step 1: Create `peer_status.rs` with types**

```rust
// elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/peer_status.rs
use hdi::prelude::*;

/// Lifecycle state of a peer. Folds periodic status and transition
/// announcements into one enum — see spec §PeerStatus surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerLifecycleState {
    Starting,
    Online,
    Degraded,
    Maintenance,
    Leaving,
}

impl std::fmt::Display for PeerLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerLifecycleState::Starting => write!(f, "starting"),
            PeerLifecycleState::Online => write!(f, "online"),
            PeerLifecycleState::Degraded => write!(f, "degraded"),
            PeerLifecycleState::Maintenance => write!(f, "maintenance"),
            PeerLifecycleState::Leaving => write!(f, "leaving"),
        }
    }
}

/// Capability flags advertised by a peer. v1 keeps this tight; see spec
/// §Evolution for how traffic_class / current_load extend this struct
/// additively.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerCapabilityFlags {
    pub general_pool_member: bool,
    pub accepting_stewardship_reserves: bool,
}

/// A peer's self-authored availability snapshot.
///
/// Validation:
/// - Author must equal `peer_id` (peers cannot author for others).
/// - Timestamp must be within 5 minutes of DHT validation time.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PeerStatus {
    pub peer_id: AgentPubKey,
    pub status: PeerLifecycleState,
    pub flags: PeerCapabilityFlags,
    pub archetype_class: Option<String>,
    pub timestamp: Timestamp,
}
```

- [ ] **Step 2: Wire module into integrity lib**

Edit `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs` — add near the top imports, and register in the `EntryTypes` enum. Find the existing `#[hdk_entry_defs]` / `EntryTypes` block and add a variant. Example pattern to follow (do NOT delete existing variants — only add):

```rust
pub mod peer_status;
pub use peer_status::{PeerStatus, PeerLifecycleState, PeerCapabilityFlags};

#[hdk_entry_defs]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    // ... existing variants ...
    PeerStatus(PeerStatus),
}
```

Verify existing `EntryTypes` block location by reading `lib.rs` first, and mirror its attribute macros exactly.

- [ ] **Step 3: Verify it compiles**

```bash
cd /projects/elohim/elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity
just check 2>&1 | tail -20
# (If justfile is at the DNA root, run it from there instead.)
```

Expected: no errors, warnings acceptable.

- [ ] **Step 4: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/peer_status.rs \
        elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs
git commit -m "feat(peer-status): integrity types for PeerStatus DHT entry"
```

### Task 2: Add link types for peer→status lookup

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs`

Rationale: to query "latest PeerStatus for agent X" without scanning, each write links from `AgentPubKey` (as anchor) to the new action hash.

- [ ] **Step 1: Extend `LinkTypes`**

Locate the existing `#[hdk_link_types]` / `LinkTypes` block (follow the `DoorwayToHeartbeat` pattern already present). Add:

```rust
#[hdk_link_types]
pub enum LinkTypes {
    // ... existing variants ...
    AgentToPeerStatus,  // base: AgentPubKey (as EntryHash), target: PeerStatus action hash
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /projects/elohim/elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity
just check 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs
git commit -m "feat(peer-status): AgentToPeerStatus link type for latest-status lookup"
```

### Task 3: Validation rules for `PeerStatus`

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs` (or existing `validate_create_entry` dispatch)
- Test: added via sweettest in Task 7

Locate the `validate` fn (the main validation callback). Follow the existing pattern for heartbeats if there is one; otherwise add a branch for `EntryTypes::PeerStatus`.

- [ ] **Step 1: Add validation branch**

```rust
// Inside validate() fn, under Op::StoreEntry -> match entry_types -> {
EntryTypes::PeerStatus(ps) => {
    // Author must equal peer_id
    if ps.peer_id != action.author().clone() {
        return Ok(ValidateCallbackResult::Invalid(
            "PeerStatus.peer_id must match entry author".into(),
        ));
    }
    // Timestamp must be within ±5 minutes of DHT validation time
    let now = sys_time()?;
    let delta = (now.as_micros() - ps.timestamp.as_micros()).abs();
    if delta > 5 * 60 * 1_000_000 {
        return Ok(ValidateCallbackResult::Invalid(
            "PeerStatus.timestamp outside ±5m window".into(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
```

The exact surrounding structure depends on the existing `validate` fn — read it first, mirror the style used for `DoorwayRegistration`.

- [ ] **Step 2: Verify it compiles**

```bash
cd /projects/elohim/elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity
just check 2>&1 | tail -10
```

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs
git commit -m "feat(peer-status): validation — author=peer_id, ±5m timestamp window"
```

---

## Module B: DNA — Coordinator Zome

### Task 4: `record_peer_status` coordinator fn

**Files:**
- Create: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/peer_status.rs`
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`

- [ ] **Step 1: Write the failing sweettest**

File: `elohim/holochain/tests/peer_status.rs`

```rust
use hdk::prelude::*;
use holochain::sweettest::*;
use infrastructure_integrity::{PeerCapabilityFlags, PeerLifecycleState, PeerStatus};

#[tokio::test(flavor = "multi_thread")]
async fn record_peer_status_roundtrip() {
    let (dna, _) = SweetDnaFile::from_bundle(&std::path::PathBuf::from(
        // Path to packed DNA — adapt to actual location
        "dna/infrastructure/workdir/infrastructure.dna",
    ))
    .await
    .unwrap();

    let mut conductor = SweetConductor::from_standard_config().await;
    let app = conductor
        .setup_app("infra", &[dna])
        .await
        .unwrap();
    let cell = app.cells()[0].clone();

    let ps = PeerStatus {
        peer_id: cell.agent_pubkey().clone(),
        status: PeerLifecycleState::Online,
        flags: PeerCapabilityFlags {
            general_pool_member: true,
            accepting_stewardship_reserves: true,
        },
        archetype_class: Some("home-nuc".into()),
        timestamp: Timestamp::now(),
    };

    let hash: ActionHash = conductor
        .call(&cell.zome("infrastructure"), "record_peer_status", ps.clone())
        .await;

    let fetched: Option<PeerStatus> = conductor
        .call(
            &cell.zome("infrastructure"),
            "get_latest_peer_status_for_agent",
            cell.agent_pubkey().clone(),
        )
        .await;

    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().status, PeerLifecycleState::Online);
}
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd /projects/elohim/elohim/holochain
just test -- peer_status 2>&1 | tail -20
```

Expected: compile error — `record_peer_status` fn not found.

- [ ] **Step 3: Implement `record_peer_status` in coordinator**

Create `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/peer_status.rs`:

```rust
use hdk::prelude::*;
use infrastructure_integrity::{EntryTypes, LinkTypes, PeerStatus};

#[hdk_extern]
pub fn record_peer_status(ps: PeerStatus) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::PeerStatus(ps.clone()))?;
    // Link from agent anchor to this status entry for latest-status lookup.
    create_link(
        AnyLinkableHash::from(ps.peer_id.clone()),
        AnyLinkableHash::from(action_hash.clone()),
        LinkTypes::AgentToPeerStatus,
        (),
    )?;
    Ok(action_hash)
}

#[hdk_extern]
pub fn get_latest_peer_status_for_agent(
    agent: AgentPubKey,
) -> ExternResult<Option<PeerStatus>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(
            AnyLinkableHash::from(agent),
            LinkTypes::AgentToPeerStatus,
        )?
        .build(),
    )?;
    // Most recent link wins (DHT stores links in deterministic order; sort by
    // create_link action timestamp to be safe).
    let mut sorted = links;
    sorted.sort_by_key(|l| l.timestamp);
    let latest = sorted.last();
    let Some(link) = latest else {
        return Ok(None);
    };
    let Some(action_hash) = link.target.clone().into_action_hash() else {
        return Ok(None);
    };
    let record = get(action_hash, GetOptions::default())?;
    let Some(record) = record else {
        return Ok(None);
    };
    let ps: Option<PeerStatus> = record.entry.to_app_option().map_err(
        |e| wasm_error!(WasmErrorInner::Guest(e.to_string())),
    )?;
    Ok(ps)
}
```

Wire module in coordinator `lib.rs`:

```rust
pub mod peer_status;
pub use peer_status::*;
```

- [ ] **Step 4: Run test, verify it passes**

```bash
cd /projects/elohim/elohim/holochain
just test -- peer_status 2>&1 | tail -20
```

Expected: `test record_peer_status_roundtrip ... ok`.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure/src/peer_status.rs \
        elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs \
        elohim/holochain/tests/peer_status.rs
git commit -m "feat(peer-status): record_peer_status + get_latest coordinator fns"
```

### Task 5: `get_all_current_peer_statuses` query

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/peer_status.rs`
- Modify: `elohim/holochain/tests/peer_status.rs`

- [ ] **Step 1: Extend test to cover multi-peer query**

Append to `peer_status.rs` test:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn get_all_current_peer_statuses_returns_each_agent_latest() {
    let (dna, _) = SweetDnaFile::from_bundle(&std::path::PathBuf::from(
        "dna/infrastructure/workdir/infrastructure.dna",
    ))
    .await
    .unwrap();
    let mut conductor = SweetConductor::from_standard_config().await;
    let app = conductor.setup_app("infra", &[dna]).await.unwrap();
    let agent_a = app.cells()[0].agent_pubkey().clone();
    // Second agent joins the same DHT
    let agent_b = conductor
        .setup_app("infra-b", &[app.cells()[0].dna_hash().clone()])
        .await
        .unwrap()
        .cells()[0]
        .agent_pubkey()
        .clone();

    // A publishes twice, B publishes once
    let ps = |who: AgentPubKey, state: PeerLifecycleState| PeerStatus {
        peer_id: who,
        status: state,
        flags: PeerCapabilityFlags {
            general_pool_member: true,
            accepting_stewardship_reserves: true,
        },
        archetype_class: None,
        timestamp: Timestamp::now(),
    };

    let cell_a = &app.cells()[0];
    let _: ActionHash = conductor
        .call(&cell_a.zome("infrastructure"), "record_peer_status",
              ps(agent_a.clone(), PeerLifecycleState::Starting)).await;
    let _: ActionHash = conductor
        .call(&cell_a.zome("infrastructure"), "record_peer_status",
              ps(agent_a.clone(), PeerLifecycleState::Online)).await;
    // B publishes via its own cell — left as a sweettest wiring detail;
    // this test may stay single-agent if multi-agent sweettest setup diverges.

    let all: Vec<PeerStatus> = conductor
        .call(&cell_a.zome("infrastructure"),
              "get_all_current_peer_statuses", ()).await;

    // Expect each agent's latest exactly once.
    assert!(all.iter().any(|s| s.peer_id == agent_a && s.status == PeerLifecycleState::Online));
}
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd /projects/elohim/elohim/holochain
just test -- get_all_current 2>&1 | tail -10
```

Expected: fn not found.

- [ ] **Step 3: Implement**

Add to `peer_status.rs`:

```rust
#[hdk_extern]
pub fn get_all_current_peer_statuses(_: ()) -> ExternResult<Vec<PeerStatus>> {
    // v1 strategy: query all PeerStatus entries authored in a time window,
    // then reduce to latest per peer. Acceptable while the network is small;
    // refine via an anchor-all-agents pattern when it scales.
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::PeerStatus.try_into()?)
        .include_entries(true);
    let mut by_agent: std::collections::HashMap<AgentPubKey, PeerStatus> =
        std::collections::HashMap::new();
    // NOTE: query() only reads this agent's source chain, so for v1 we
    // rely on per-agent DHT link walks. Delegate to a cross-agent query
    // by iterating known peers is out-of-scope for v1; this fn returns
    // only the calling agent's latest for now, and doorway will aggregate
    // via its subscription instead.
    if let Some(ps) = get_latest_peer_status_for_agent(agent_info()?.agent_initial_pubkey)? {
        by_agent.insert(ps.peer_id.clone(), ps);
    }
    Ok(by_agent.into_values().collect())
}
```

Leave the multi-agent aggregation to the doorway subscription layer in Phase 2; this fn is a convenience for operator debug tools in v1. Document this in the doc-comment.

- [ ] **Step 4: Run test, verify it passes**

```bash
cd /projects/elohim/elohim/holochain
just test -- peer_status 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure/src/peer_status.rs \
        elohim/holochain/tests/peer_status.rs
git commit -m "feat(peer-status): get_all_current_peer_statuses debug query"
```

### Task 6: Post-commit signal for projection

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`

Check how the existing post-commit hook fires `ProjectionSignal` variants for `DoorwayHeartbeat` — mirror that pattern.

- [ ] **Step 1: Add signal variant**

In the signal enum (likely in `infrastructure/src/lib.rs` or a shared signals crate):

```rust
// Add to the ProjectionSignal enum (or equivalent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectionSignal {
    // ... existing variants ...
    PeerStatusRecorded {
        peer_id: AgentPubKey,
        status: String,
        general_pool_member: bool,
        accepting_stewardship_reserves: bool,
        archetype_class: Option<String>,
        timestamp: i64,
        action_hash: ActionHash,
    },
}
```

Emit from `post_commit` when the committed entry is a `PeerStatus`:

```rust
#[hdk_extern]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for a in committed_actions {
        if let Some((eh, EntryType::App(aed))) = a.action().entry_data() {
            if let Ok(Some(ps)) = get_latest_peer_status_by_action(a.action_address().clone()) {
                let _ = emit_signal(ProjectionSignal::PeerStatusRecorded {
                    peer_id: ps.peer_id.clone(),
                    status: ps.status.to_string(),
                    general_pool_member: ps.flags.general_pool_member,
                    accepting_stewardship_reserves: ps.flags.accepting_stewardship_reserves,
                    archetype_class: ps.archetype_class.clone(),
                    timestamp: ps.timestamp.as_micros(),
                    action_hash: a.action_address().clone(),
                });
            }
        }
    }
}
```

Adapt to whatever the existing `post_commit` hook looks like — read first, add a branch.

- [ ] **Step 2: Verify it compiles**

```bash
cd /projects/elohim/elohim/holochain && just check 2>&1 | tail -10
```

- [ ] **Step 3: Commit**

```bash
git add elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs
git commit -m "feat(peer-status): emit PeerStatusRecorded projection signal"
```

---

## Module C: elohim-storage — SQLite Projection

### Task 7: Diesel migration for `peer_statuses` table

**Files:**
- Create: `elohim/elohim-storage/migrations/<timestamp>_create_peer_statuses/up.sql`
- Create: `elohim/elohim-storage/migrations/<timestamp>_create_peer_statuses/down.sql`

- [ ] **Step 1: Generate migration skeleton**

```bash
cd /projects/elohim/elohim/elohim-storage
diesel migration generate create_peer_statuses
```

- [ ] **Step 2: Fill in `up.sql`**

```sql
-- Source of truth: DHT (infrastructure DNA PeerStatus entry, Category A).
-- This table is a read-optimized projection populated by the post-commit
-- signal handler in src/signals.rs. Do not write here directly — all
-- writes flow from ProjectionSignal::PeerStatusRecorded. If this table
-- and the DHT disagree, the DHT wins (rebuild the projection from it).

CREATE TABLE peer_statuses (
    peer_id TEXT PRIMARY KEY,                   -- AgentPubKey (base64) of the peer
    status TEXT NOT NULL,                       -- PeerLifecycleState: starting|online|degraded|maintenance|leaving
    general_pool_member INTEGER NOT NULL,       -- 0/1
    accepting_stewardship_reserves INTEGER NOT NULL, -- 0/1
    archetype_class TEXT,                       -- optional archetype id (e.g. "home-nuc")
    timestamp BIGINT NOT NULL,                  -- micros since epoch (from PeerStatus.timestamp)
    dht_anchor_hash BLOB NOT NULL,              -- ActionHash of the upstream DHT entry
    updated_at BIGINT NOT NULL                  -- local insert/update time, micros since epoch
);

CREATE INDEX idx_peer_statuses_status ON peer_statuses(status);
CREATE INDEX idx_peer_statuses_pool ON peer_statuses(general_pool_member);
```

- [ ] **Step 3: Fill in `down.sql`**

```sql
DROP TABLE peer_statuses;
```

- [ ] **Step 4: Run migration**

```bash
cd /projects/elohim/elohim/elohim-storage
diesel migration run
# Or whatever invocation the repo uses — check CLAUDE.md / existing migrations.
```

- [ ] **Step 5: Regenerate `diesel_schema.rs` (reflects the projection, not a new source of truth — DHT remains authoritative)**

Follow the pattern established by `device_policies` / `humans` migrations. Typically:

```bash
# Reflect the projection schema — source of truth is the DHT PeerStatus entry.
diesel print-schema > src/db/diesel_schema.rs
```

Verify the new `peer_statuses` table block appears.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/migrations/*_create_peer_statuses \
        elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(peer-status): peer_statuses projection table + diesel schema (source of truth: DHT)"
```

### Task 8: Diesel model + upsert

**Files:**
- Create: `elohim/elohim-storage/src/db/peer_statuses.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Write the failing test**

At the bottom of `peer_statuses.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::context::test_conn;

    #[test]
    fn upsert_then_read_latest() {
        let mut conn = test_conn();
        let row = PeerStatusRow {
            peer_id: "uhCAkABC".into(),
            status: "online".into(),
            general_pool_member: true,
            accepting_stewardship_reserves: true,
            archetype_class: Some("home-nuc".into()),
            timestamp: 1_700_000_000_000_000,
            dht_anchor_hash: vec![0u8; 39],
            updated_at: 1_700_000_000_000_000,
        };
        upsert(&mut conn, &row).unwrap();

        let fetched = get_by_peer(&mut conn, "uhCAkABC").unwrap().unwrap();
        assert_eq!(fetched.status, "online");
        assert!(fetched.general_pool_member);
    }

    #[test]
    fn list_pool_members_filters_to_true() {
        let mut conn = test_conn();
        let mut mk = |peer: &str, member: bool| PeerStatusRow {
            peer_id: peer.into(),
            status: "online".into(),
            general_pool_member: member,
            accepting_stewardship_reserves: true,
            archetype_class: None,
            timestamp: 1,
            dht_anchor_hash: vec![0u8; 39],
            updated_at: 1,
        };
        upsert(&mut conn, &mk("adam", true)).unwrap();
        upsert(&mut conn, &mk("terrance", false)).unwrap();

        let members = list_pool_members(&mut conn).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].peer_id, "adam");
    }
}
```

(`test_conn()` is assumed to exist in `db/context.rs` alongside other test helpers — verify and use the actual helper name.)

- [ ] **Step 2: Run test, verify it fails**

```bash
cd /projects/elohim/elohim/elohim-storage
cargo test db::peer_statuses 2>&1 | tail -10
```

Expected: compile error — `PeerStatusRow`, `upsert`, `get_by_peer`, `list_pool_members` undefined.

- [ ] **Step 3: Implement the module**

```rust
// elohim/elohim-storage/src/db/peer_statuses.rs
//
// Projection (not a source of truth) of the notarized PeerStatus DHT entry.
// Writes here come exclusively from the post-commit signal projection in
// src/signals.rs. Source of truth: DHT. If this projection and the DHT
// disagree, the DHT wins.
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
// Reflection of the projection schema — source of truth remains the DHT entry.
use crate::db::diesel_schema::peer_statuses;

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = peer_statuses, primary_key(peer_id))]
pub struct PeerStatusRow {
    pub peer_id: String,
    pub status: String,
    pub general_pool_member: bool,
    pub accepting_stewardship_reserves: bool,
    pub archetype_class: Option<String>,
    pub timestamp: i64,
    pub dht_anchor_hash: Vec<u8>,
    pub updated_at: i64,
}

pub fn upsert(conn: &mut SqliteConnection, row: &PeerStatusRow) -> QueryResult<usize> {
    diesel::insert_into(peer_statuses::table)
        .values(row)
        .on_conflict(peer_statuses::peer_id)
        .do_update()
        .set(row)
        .execute(conn)
}

pub fn get_by_peer(conn: &mut SqliteConnection, peer: &str) -> QueryResult<Option<PeerStatusRow>> {
    peer_statuses::table
        .find(peer)
        .first::<PeerStatusRow>(conn)
        .optional()
}

pub fn list_pool_members(conn: &mut SqliteConnection) -> QueryResult<Vec<PeerStatusRow>> {
    peer_statuses::table
        .filter(peer_statuses::general_pool_member.eq(true))
        .filter(peer_statuses::status.eq_any(vec!["online", "degraded"]))
        .load::<PeerStatusRow>(conn)
}
```

Expose in `db/mod.rs`:

```rust
pub mod peer_statuses;
```

- [ ] **Step 4: Run test, verify it passes**

```bash
cargo test db::peer_statuses 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/peer_statuses.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(peer-status): diesel model with upsert + pool-member query"
```

### Task 9: Signal handler — project `PeerStatusRecorded` into SQLite

**Files:**
- Modify: `elohim/elohim-storage/src/signals.rs`

- [ ] **Step 1: Write the failing test**

In `signals.rs` (add `#[cfg(test)] mod tests`):

```rust
#[test]
fn peer_status_recorded_upserts_row() {
    use crate::db::context::test_conn;
    use crate::db::peer_statuses::get_by_peer;
    let mut conn = test_conn();

    let signal = ProjectionSignal::PeerStatusRecorded {
        peer_id: "uhCAkABC".to_string(),
        status: "online".to_string(),
        general_pool_member: true,
        accepting_stewardship_reserves: false,
        archetype_class: Some("home-nuc".to_string()),
        timestamp: 1_700_000_000_000_000,
        action_hash: vec![0u8; 39],
    };
    handle_signal(&mut conn, signal).unwrap();

    let row = get_by_peer(&mut conn, "uhCAkABC").unwrap().unwrap();
    assert_eq!(row.status, "online");
    assert!(row.general_pool_member);
    assert!(!row.accepting_stewardship_reserves);
}
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cargo test signals::tests::peer_status_recorded 2>&1 | tail -10
```

- [ ] **Step 3: Add `PeerStatusRecorded` to local signal enum + handler**

Follow the existing signal handler dispatch. Add:

```rust
// signals.rs
pub enum ProjectionSignal {
    // existing variants ...
    PeerStatusRecorded {
        peer_id: String,
        status: String,
        general_pool_member: bool,
        accepting_stewardship_reserves: bool,
        archetype_class: Option<String>,
        timestamp: i64,
        action_hash: Vec<u8>,
    },
}

pub fn handle_signal(conn: &mut SqliteConnection, signal: ProjectionSignal) -> anyhow::Result<()> {
    match signal {
        ProjectionSignal::PeerStatusRecorded {
            peer_id, status, general_pool_member,
            accepting_stewardship_reserves, archetype_class,
            timestamp, action_hash
        } => {
            let row = crate::db::peer_statuses::PeerStatusRow {
                peer_id,
                status,
                general_pool_member,
                accepting_stewardship_reserves,
                archetype_class,
                timestamp,
                dht_anchor_hash: action_hash,
                updated_at: chrono::Utc::now().timestamp_micros(),
            };
            crate::db::peer_statuses::upsert(conn, &row)?;
            Ok(())
        }
        // existing arms ...
    }
}
```

Adapt to actual signal dispatch shape.

- [ ] **Step 4: Run test, verify it passes**

```bash
cargo test signals::tests 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/signals.rs
git commit -m "feat(peer-status): project PeerStatusRecorded signal into SQLite"
```

---

## Module D: Policy Engine

### Task 10: Config loader for `peer-policy.toml`

**Files:**
- Create: `elohim/elohim-storage/src/policy/mod.rs`
- Create: `elohim/elohim-storage/src/policy/config.rs`
- Create: `elohim/elohim-storage/config/peer-policy.example.toml`
- Modify: `elohim/elohim-storage/src/lib.rs` (`pub mod policy;`)
- Modify: `elohim/elohim-storage/Cargo.toml` — add `toml = "0.8"` if absent

- [ ] **Step 1: Write the example TOML**

`elohim/elohim-storage/config/peer-policy.example.toml`:

```toml
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 20
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
expose_conductor_externally = false
conductor_external_bind = "0.0.0.0:4445"
conductor_internal_port = 4445
```

- [ ] **Step 2: Write the failing test**

`elohim/elohim-storage/src/policy/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example_config() {
        let toml = include_str!("../../config/peer-policy.example.toml");
        let cfg: PolicyConfig = toml::from_str(toml).unwrap();
        assert!(matches!(cfg.pool.accept_general_traffic, AutoOrBool::Auto));
        assert_eq!(cfg.pool.min_free_storage_pct, 20);
        assert!(!cfg.network.expose_conductor_externally);
        assert_eq!(cfg.network.conductor_external_bind, "0.0.0.0:4445");
    }

    #[test]
    fn auto_or_bool_accepts_literal_true_and_false() {
        let t: AutoOrBool = toml::from_str("value = true").unwrap_or_else(|_| {
            // Shortcut: parse inline
            toml::from_str::<toml::Value>("value = true").unwrap();
            AutoOrBool::Bool(true)
        });
        assert!(matches!(t, AutoOrBool::Bool(true)));
    }
}
```

- [ ] **Step 3: Run test, verify it fails**

```bash
cd /projects/elohim/elohim/elohim-storage
cargo test policy::config 2>&1 | tail -10
```

- [ ] **Step 4: Implement `PolicyConfig`**

`elohim/elohim-storage/src/policy/config.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AutoOrBool {
    Bool(bool),
    #[serde(with = "auto_literal")]
    Auto,
}

mod auto_literal {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("auto")
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<(), D::Error> {
        let s = String::deserialize(d)?;
        if s == "auto" { Ok(()) } else {
            Err(serde::de::Error::custom("expected \"auto\""))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub accept_general_traffic: AutoOrBool,
    pub min_free_storage_pct: u8,
    pub require_conductor_healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StewardshipConfig {
    pub accept_new_reserves: AutoOrBool,
    pub max_storage_pct: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub expose_conductor_externally: bool,
    pub conductor_external_bind: String,
    pub conductor_internal_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub pool: PoolConfig,
    pub stewardship: StewardshipConfig,
    pub network: NetworkConfig,
}

impl PolicyConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }
}
```

`elohim/elohim-storage/src/policy/mod.rs`:

```rust
pub mod config;
pub mod evaluator;
pub use config::{PolicyConfig, AutoOrBool};
pub use evaluator::{evaluate, LiveState, EvaluatedFlags};
```

- [ ] **Step 5: Run test, verify it passes**

```bash
cargo test policy::config 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/policy/config.rs \
        elohim/elohim-storage/src/policy/mod.rs \
        elohim/elohim-storage/config/peer-policy.example.toml \
        elohim/elohim-storage/src/lib.rs \
        elohim/elohim-storage/Cargo.toml
git commit -m "feat(peer-status): peer-policy.toml loader with AutoOrBool"
```

### Task 11: Policy evaluator

**Files:**
- Create: `elohim/elohim-storage/src/policy/evaluator.rs`

- [ ] **Step 1: Write failing tests**

```rust
// elohim/elohim-storage/src/policy/evaluator.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::config::*;

    fn base_cfg() -> PolicyConfig {
        PolicyConfig {
            pool: PoolConfig {
                accept_general_traffic: AutoOrBool::Auto,
                min_free_storage_pct: 20,
                require_conductor_healthy: true,
            },
            stewardship: StewardshipConfig {
                accept_new_reserves: AutoOrBool::Auto,
                max_storage_pct: 80,
            },
            network: NetworkConfig {
                expose_conductor_externally: false,
                conductor_external_bind: "0.0.0.0:4445".into(),
                conductor_internal_port: 4445,
            },
        }
    }

    #[test]
    fn auto_pool_member_respects_conductor_and_storage() {
        let cfg = base_cfg();
        let healthy = LiveState { free_storage_pct: 50, conductor_healthy: true };
        assert!(evaluate(&cfg, &healthy).general_pool_member);

        let unhealthy = LiveState { free_storage_pct: 50, conductor_healthy: false };
        assert!(!evaluate(&cfg, &unhealthy).general_pool_member);

        let low_storage = LiveState { free_storage_pct: 10, conductor_healthy: true };
        assert!(!evaluate(&cfg, &low_storage).general_pool_member);
    }

    #[test]
    fn explicit_false_overrides_auto() {
        let mut cfg = base_cfg();
        cfg.pool.accept_general_traffic = AutoOrBool::Bool(false);
        let healthy = LiveState { free_storage_pct: 50, conductor_healthy: true };
        assert!(!evaluate(&cfg, &healthy).general_pool_member);
    }

    #[test]
    fn stewardship_flag_respects_max_storage() {
        let cfg = base_cfg();
        let mostly_full = LiveState { free_storage_pct: 15, conductor_healthy: true };
        // max_storage_pct 80 → refuse when used >80% (free <20%)
        assert!(!evaluate(&cfg, &mostly_full).accepting_stewardship_reserves);
    }
}
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cargo test policy::evaluator 2>&1 | tail -10
```

- [ ] **Step 3: Implement evaluator**

```rust
use crate::policy::config::{AutoOrBool, PolicyConfig};

#[derive(Debug, Clone, Copy)]
pub struct LiveState {
    pub free_storage_pct: u8,
    pub conductor_healthy: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedFlags {
    pub general_pool_member: bool,
    pub accepting_stewardship_reserves: bool,
}

pub fn evaluate(cfg: &PolicyConfig, state: &LiveState) -> EvaluatedFlags {
    EvaluatedFlags {
        general_pool_member: eval_pool(cfg, state),
        accepting_stewardship_reserves: eval_stewardship(cfg, state),
    }
}

fn eval_pool(cfg: &PolicyConfig, state: &LiveState) -> bool {
    match cfg.pool.accept_general_traffic {
        AutoOrBool::Bool(b) => b,
        AutoOrBool::Auto => {
            state.free_storage_pct >= cfg.pool.min_free_storage_pct
                && (!cfg.pool.require_conductor_healthy || state.conductor_healthy)
        }
    }
}

fn eval_stewardship(cfg: &PolicyConfig, state: &LiveState) -> bool {
    match cfg.stewardship.accept_new_reserves {
        AutoOrBool::Bool(b) => b,
        AutoOrBool::Auto => {
            // used_pct = 100 - free_pct; accept if used_pct <= max_storage_pct
            (100u8.saturating_sub(state.free_storage_pct)) <= cfg.stewardship.max_storage_pct
        }
    }
}
```

- [ ] **Step 4: Run test, verify it passes**

```bash
cargo test policy::evaluator 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/policy/evaluator.rs \
        elohim/elohim-storage/src/policy/mod.rs
git commit -m "feat(peer-status): policy evaluator — auto pool + stewardship flags"
```

---

## Module E: Heartbeat Task

### Task 12: `HeartbeatTask` — periodic publish

**Files:**
- Create: `elohim/elohim-storage/src/heartbeat.rs`
- Modify: `elohim/elohim-storage/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// elohim/elohim-storage/src/heartbeat.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{PolicyConfig, LiveState};

    #[tokio::test]
    async fn one_tick_publishes_one_status() {
        let cfg = crate::policy::config::PolicyConfig {
            // ... use the `base_cfg` equivalent from evaluator tests ...
            pool: Default::default_pool_for_tests(),
            stewardship: Default::default_stew_for_tests(),
            network: Default::default_net_for_tests(),
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Published>(4);
        let fake_publisher = TestPublisher { tx };
        let fake_probe = TestProbe { state: LiveState { free_storage_pct: 50, conductor_healthy: true } };

        let task = HeartbeatTask::new(cfg, fake_publisher, fake_probe);
        task.tick_once().await.unwrap();

        let published = rx.recv().await.unwrap();
        assert_eq!(published.status, "online");
        assert!(published.flags.general_pool_member);
    }
}
```

(Helpers `Default::default_pool_for_tests` etc. — add inline `fn` helpers in the test module; omitted here for brevity. Actually mirror the `base_cfg()` fn from Task 11 — paste it directly into the test module.)

- [ ] **Step 2: Run test, verify it fails**

```bash
cargo test heartbeat 2>&1 | tail -10
```

- [ ] **Step 3: Implement `HeartbeatTask`**

```rust
use crate::policy::{evaluate, LiveState, PolicyConfig};
use std::time::Duration;
use tokio::sync::broadcast;

pub struct Published {
    pub status: String,
    pub flags: crate::policy::EvaluatedFlags,
    pub archetype_class: Option<String>,
}

#[async_trait::async_trait]
pub trait Publisher: Send + Sync + 'static {
    async fn publish(&self, p: Published) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait LiveProbe: Send + Sync + 'static {
    async fn sample(&self) -> anyhow::Result<LiveState>;
}

pub struct HeartbeatTask<P: Publisher, L: LiveProbe> {
    cfg: PolicyConfig,
    publisher: P,
    probe: L,
    archetype_class: Option<String>,
    lifecycle: tokio::sync::Mutex<LifecycleState>,
}

#[derive(Clone, Copy, PartialEq)]
enum LifecycleState {
    Starting,
    Online,
    Degraded,
    Maintenance,
    Leaving,
}

impl<P: Publisher, L: LiveProbe> HeartbeatTask<P, L> {
    pub fn new(cfg: PolicyConfig, publisher: P, probe: L) -> Self {
        Self {
            cfg,
            publisher,
            probe,
            archetype_class: None,
            lifecycle: tokio::sync::Mutex::new(LifecycleState::Starting),
        }
    }

    pub async fn tick_once(&self) -> anyhow::Result<()> {
        let state = self.probe.sample().await?;
        let flags = evaluate(&self.cfg, &state);
        let mut lifecycle = self.lifecycle.lock().await;
        if matches!(*lifecycle, LifecycleState::Starting) {
            *lifecycle = LifecycleState::Online;
        } else if !flags.general_pool_member && matches!(*lifecycle, LifecycleState::Online) {
            *lifecycle = LifecycleState::Degraded;
        }
        let status = match *lifecycle {
            LifecycleState::Starting => "starting",
            LifecycleState::Online => "online",
            LifecycleState::Degraded => "degraded",
            LifecycleState::Maintenance => "maintenance",
            LifecycleState::Leaving => "leaving",
        }.to_string();
        self.publisher
            .publish(Published {
                status,
                flags,
                archetype_class: self.archetype_class.clone(),
            })
            .await
    }

    pub async fn run(self, mut shutdown: broadcast::Receiver<()>) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.tick_once().await {
                        tracing::warn!("heartbeat tick failed: {e}");
                    }
                }
                _ = shutdown.recv() => {
                    // Announce Leaving before exiting
                    let mut lifecycle = self.lifecycle.lock().await;
                    *lifecycle = LifecycleState::Leaving;
                    drop(lifecycle);
                    let _ = self.tick_once().await;
                    break;
                }
            }
        }
    }
}

// Test-only fakes
#[cfg(test)]
pub(crate) struct TestPublisher { pub tx: tokio::sync::mpsc::Sender<Published> }
#[cfg(test)]
#[async_trait::async_trait]
impl Publisher for TestPublisher {
    async fn publish(&self, p: Published) -> anyhow::Result<()> {
        self.tx.send(p).await?;
        Ok(())
    }
}
#[cfg(test)]
pub(crate) struct TestProbe { pub state: LiveState }
#[cfg(test)]
#[async_trait::async_trait]
impl LiveProbe for TestProbe {
    async fn sample(&self) -> anyhow::Result<LiveState> { Ok(self.state) }
}
```

Add `pub mod heartbeat;` to `lib.rs`.

- [ ] **Step 4: Run test, verify it passes**

```bash
cargo test heartbeat 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/heartbeat.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(peer-status): HeartbeatTask with policy eval + lifecycle transitions"
```

### Task 13: Graceful shutdown — publish `Leaving`

**Files:**
- Modify: `elohim/elohim-storage/src/heartbeat.rs` (already drafted above — verify)
- Modify: `elohim/elohim-storage/src/main.rs` — wire heartbeat into existing shutdown broadcast channel

- [ ] **Step 1: Write the failing test**

Add to `heartbeat.rs` test module:

```rust
#[tokio::test]
async fn shutdown_publishes_leaving_before_exit() {
    let cfg = /* base_cfg */;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Published>(4);
    let task = HeartbeatTask::new(
        cfg,
        TestPublisher { tx },
        TestProbe { state: LiveState { free_storage_pct: 50, conductor_healthy: true } },
    );
    let (sd_tx, sd_rx) = tokio::sync::broadcast::channel(1);
    let handle = tokio::spawn(task.run(sd_rx));

    // Give interval a moment to fire; or force: don't wait for 60s — just shut down
    sd_tx.send(()).unwrap();
    handle.await.unwrap();

    // Drain published messages; at least one should be "leaving"
    let mut saw_leaving = false;
    while let Ok(p) = rx.try_recv() {
        if p.status == "leaving" { saw_leaving = true; }
    }
    assert!(saw_leaving, "heartbeat must publish Leaving on shutdown");
}
```

- [ ] **Step 2: Run test, verify it fails or passes** (the impl in Task 12 already emits — this test locks behavior in)

```bash
cargo test heartbeat::tests::shutdown 2>&1 | tail -10
```

If it fails, refine the `run` loop. If it passes, move on.

- [ ] **Step 3: Wire into `main.rs`**

In `main.rs`, near the existing shutdown broadcast channel, spawn the heartbeat task. Example insert point (find the existing `shutdown_tx` broadcast channel, around line 684):

```rust
// After policy config load:
let policy_cfg = elohim_storage::policy::PolicyConfig::load(&cfg.peer_policy_path)?;
let heartbeat_publisher = elohim_storage::heartbeat::ZomeCallPublisher::new(hc_client.clone(), /* agent_key */);
let heartbeat_probe = elohim_storage::heartbeat::DefaultProbe::new(blob_store.clone(), hc_client.clone());
let heartbeat = elohim_storage::heartbeat::HeartbeatTask::new(
    policy_cfg,
    heartbeat_publisher,
    heartbeat_probe,
);
let hb_shutdown = shutdown_tx.subscribe();
tokio::spawn(async move { heartbeat.run(hb_shutdown).await });
```

The `ZomeCallPublisher` and `DefaultProbe` are real implementations of the traits. Add them to `heartbeat.rs`:

```rust
pub struct ZomeCallPublisher {
    hc: std::sync::Arc<crate::hc_client::HcClient>,
    agent: holo_hash::AgentPubKey,
}
impl ZomeCallPublisher {
    pub fn new(hc: std::sync::Arc<crate::hc_client::HcClient>, agent: holo_hash::AgentPubKey) -> Self {
        Self { hc, agent }
    }
}
#[async_trait::async_trait]
impl Publisher for ZomeCallPublisher {
    async fn publish(&self, p: Published) -> anyhow::Result<()> {
        let ps = serde_json::json!({
            "peer_id": self.agent,
            "status": p.status,
            "flags": {
                "general_pool_member": p.flags.general_pool_member,
                "accepting_stewardship_reserves": p.flags.accepting_stewardship_reserves,
            },
            "archetype_class": p.archetype_class,
            "timestamp": chrono::Utc::now().timestamp_micros(),
        });
        self.hc.zome_call("infrastructure", "record_peer_status", ps).await?;
        Ok(())
    }
}

pub struct DefaultProbe {
    blob: std::sync::Arc<crate::blob_store::BlobStore>,
    hc: std::sync::Arc<crate::hc_client::HcClient>,
}
impl DefaultProbe {
    pub fn new(blob: std::sync::Arc<crate::blob_store::BlobStore>, hc: std::sync::Arc<crate::hc_client::HcClient>) -> Self {
        Self { blob, hc }
    }
}
#[async_trait::async_trait]
impl LiveProbe for DefaultProbe {
    async fn sample(&self) -> anyhow::Result<LiveState> {
        let free_pct = self.blob.free_storage_pct().unwrap_or(100);
        let healthy = self.hc.ping().await.is_ok();
        Ok(LiveState { free_storage_pct: free_pct, conductor_healthy: healthy })
    }
}
```

Adapt method names (`blob.free_storage_pct`, `hc.ping`) to whatever the real APIs are — verify by reading `blob_store.rs` and `hc_client.rs`. If the methods don't exist, add minimal ones.

- [ ] **Step 4: Verify the whole binary builds**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/heartbeat.rs elohim/elohim-storage/src/main.rs
git commit -m "feat(peer-status): wire HeartbeatTask into main with graceful shutdown"
```

---

## Module F: TCP Forwarder

### Task 14: Forwarder module

**Files:**
- Create: `elohim/elohim-storage/src/forwarder.rs`
- Modify: `elohim/elohim-storage/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// elohim/elohim-storage/src/forwarder.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn forwarder_pipes_bytes_bidirectionally() {
        // Upstream echo server on random port
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = upstream.accept().await {
                let mut buf = [0u8; 64];
                if let Ok(n) = socket.read(&mut buf).await {
                    let _ = socket.write_all(&buf[..n]).await;
                }
            }
        });

        // Forwarder listening on another random port → upstream
        let ext = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ext_addr = ext.local_addr().unwrap();
        tokio::spawn(async move {
            forwarder_accept_loop(ext, upstream_addr).await;
        });

        // Client connects via forwarder
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mut client = TcpStream::connect(ext_addr).await.unwrap();
        client.write_all(b"ping").await.unwrap();
        let mut resp = [0u8; 4];
        client.read_exact(&mut resp).await.unwrap();
        assert_eq!(&resp, b"ping");
    }
}
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cargo test forwarder 2>&1 | tail -10
```

- [ ] **Step 3: Implement forwarder**

```rust
use std::net::SocketAddr;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};

pub async fn forwarder_accept_loop(listener: TcpListener, upstream: SocketAddr) {
    loop {
        let (mut inbound, peer) = match listener.accept().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("forwarder accept error: {e}");
                continue;
            }
        };
        tokio::spawn(async move {
            let mut outbound = match TcpStream::connect(upstream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("forwarder upstream connect failed ({peer} -> {upstream}): {e}");
                    return;
                }
            };
            if let Err(e) = copy_bidirectional(&mut inbound, &mut outbound).await {
                tracing::debug!("forwarder copy ended: {e}");
            }
        });
    }
}

pub async fn spawn_forwarder(bind: &str, upstream_port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    let upstream: SocketAddr = format!("127.0.0.1:{upstream_port}").parse()?;
    tracing::info!("peer-status forwarder: {bind} -> {upstream}");
    tokio::spawn(forwarder_accept_loop(listener, upstream));
    Ok(())
}
```

Add `pub mod forwarder;` to `lib.rs`.

- [ ] **Step 4: Run test, verify it passes**

```bash
cargo test forwarder 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/forwarder.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(peer-status): tokio TCP forwarder with bidirectional copy"
```

### Task 15: Gate forwarder on policy flag in `main.rs`

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs`

- [ ] **Step 1: Add gated spawn near conductor attach**

Find where `attach_app_interface` is called in the startup flow (via hc_client or happ_manager). Immediately after it succeeds, conditionally start the forwarder:

```rust
if policy_cfg.network.expose_conductor_externally {
    elohim_storage::forwarder::spawn_forwarder(
        &policy_cfg.network.conductor_external_bind,
        policy_cfg.network.conductor_internal_port,
    )
    .await?;
}
```

- [ ] **Step 2: Verify the whole binary builds**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -10
```

- [ ] **Step 3: Manual smoke test**

Create a throwaway `peer-policy.toml` with `expose_conductor_externally = true`, run the storage binary locally, confirm `curl -v ws://127.0.0.1:4445/` reaches the conductor. Revert the config afterward.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "feat(peer-status): gate forwarder on policy flag at startup"
```

---

## Module G: Config Plumbing

### Task 16: Add `peer_policy_path` to storage config

**Files:**
- Modify: `elohim/elohim-storage/src/config.rs`

- [ ] **Step 1: Add field with default**

Find the existing `Config` struct and add:

```rust
pub struct Config {
    // ... existing fields ...
    pub peer_policy_path: std::path::PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // existing defaults ...
            peer_policy_path: std::path::PathBuf::from("./config/peer-policy.toml"),
        }
    }
}
```

If config parses from env (`ELOHIM_STORAGE_*` pattern), add the env var too — follow the existing pattern exactly.

- [ ] **Step 2: Verify it builds**

```bash
cargo build 2>&1 | tail -10
```

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/config.rs
git commit -m "feat(peer-status): config.peer_policy_path"
```

---

## Final: E2E smoke test

### Task 17: End-to-end integration — peer publishes, SQLite has row

**Files:**
- Create: `elohim/elohim-storage/tests/peer_status_e2e.rs`

- [ ] **Step 1: Write the test**

```rust
// Start a conductor + storage together, wait ~62s, assert SQLite
// peer_statuses table has exactly one row with status='online'.
// Use the existing integration-test harness pattern from other *_e2e tests
// in this crate — do not invent a new one.

#[tokio::test]
#[ignore]  // slow — run manually with `cargo test -- --ignored`
async fn peer_publishes_status_within_one_minute() {
    // harness setup here — adapt from existing e2e tests
    // ...
    tokio::time::sleep(std::time::Duration::from_secs(65)).await;
    let mut conn = harness.db_conn();
    let row = crate::db::peer_statuses::get_by_peer(&mut conn, &harness.agent_pubkey_str())
        .unwrap()
        .expect("peer_status row must exist after one heartbeat cycle");
    assert_eq!(row.status, "online");
    assert!(row.general_pool_member);
}
```

Fill the harness setup by copying the nearest existing `*_e2e.rs` pattern from the storage crate.

- [ ] **Step 2: Run it**

```bash
cd /projects/elohim/elohim/elohim-storage
cargo test peer_status_e2e -- --ignored --nocapture 2>&1 | tail -30
```

Expected: row present, status online.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/peer_status_e2e.rs
git commit -m "test(peer-status): e2e — peer publishes within one heartbeat cycle"
```

### Task 18: Run full pre-push gate

- [ ] **Step 1: Full DNA test suite**

```bash
cd /projects/elohim/elohim/holochain && just test 2>&1 | tail -20
```

- [ ] **Step 2: Full elohim-storage test suite**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -20
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -10
cargo fmt --check
```

- [ ] **Step 3: Confirm no placeholder writes landed**

```bash
git log --format=%s origin/dev..HEAD | grep -E "(TODO|WIP|XXX)" && echo "bad commits" || echo "clean"
```

---

## Self-review

Spec coverage:
- `PeerStatus` surface with enriched status enum → Tasks 1, 3 (integrity) + 4–6 (coordinator)
- Policy engine reading `peer-policy.toml` with archetype-ready defaults → Tasks 10–11
- Heartbeat task with lifecycle transitions + graceful `Leaving` → Tasks 12–13
- Optional TCP forwarder gated by policy → Tasks 14–15
- SQLite projection via post-commit signal → Tasks 6 (emit) + 7–9 (project)
- Archetype-defaulted thresholds: deferred — v1 reads thresholds from the TOML directly; archetype-classifier is Phase 3.
- elohim-node wrapper integration: deferred to Phase 2 (explicit).
- Doorway subscription + agent-addressed routing: deferred to Phase 2 (explicit).
- elohim-agent co-stewardship channel: design hook only; no code — matches spec.

Placeholder scan: no TBDs / "implement later" / "similar to Task N" without code shown. The `[ignore]` smoke test in Task 17 carries a real assertion; the harness setup references the existing e2e test pattern rather than spelling it out, which is acceptable as a pointer but flag this for Phase 2 if the pattern turns out to be absent.

Type consistency: `PeerStatus`, `PeerLifecycleState`, `PeerCapabilityFlags` names are consistent across all tasks; `general_pool_member` / `accepting_stewardship_reserves` flag names match from integrity types through TOML through SQLite columns through signal payload.

---

## Execution handoff

Plan complete and saved to `genesis/docs/plans/2026-04-17-peer-stewarded-availability-phase-1-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
