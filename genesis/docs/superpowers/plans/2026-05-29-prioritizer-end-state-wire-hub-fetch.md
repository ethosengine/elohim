# Replication-Prioritizer End-State (Wire Hints · Hub Derivation · Fetch Seam) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Primary implementer: **rust-architect** (read its "Canonical Implementation Patterns" section first — wire-format evolution, read-side inline diesel, correct-but-dormant projection, target-pool + shared-tree git discipline all apply here).

**Goal:** Make `replication_prioritizer::score_advertised_blob` actually shape what peers cache — by carrying per-blob hints over inventory gossip, deriving a blob's owning hub from the canonical Collective CID, and wiring HIGH-priority advertised blobs into a real fetch path.

**Architecture:** Three waves on the deterministic substrate floor, **zero new DHT entry types** (per the P2P Design Gate below — everything is Category C operational or Category A2 derived-via-link). Wave 1 evolves the gossip wire additively (backward-compatible). Wave 2 projects the existing `Collective`/`Membership` DHT entries into a `peer/agent → collective_cid` resolver (also closing the `hub_capacity_service` aggregation stub — the device→hub / DHT→projection scalability win). Wave 3 populates the hints and wires the prioritizer into the inventory receive arm + a blob-level fetch seam.

**Tech Stack:** Rust (elohim-storage), Diesel/SQLite projections, libp2p gossipsub + `rmp_serde` (MessagePack) wire, Holochain `Collective`/`Membership` entries (imagodei DNA, existing).

---

## Provenance & Decision Record

- **Source design gate:** P2P Design Gate run inline 2026-05-29 (see below). Predecessor specs: `2026-05-28-mutual-storage-replication-dwelling-hub-design.md` §8.2/§8.4 (prioritizer + "commitments shape cache"), `2026-05-01-light-up-the-topology-design.md` (Category-C precedent), `2026-05-11-tiered-quilt-stewardship-design.md` (the storage-tier vocabulary).
- **Operator decision (2026-05-29):** canonical hub-id = **Collective CID (`collective:{action_hash}`)**. Three live conventions (env slug `household-matthew`; prefix `dwelling:`/`collective:`; Collective CID) converge on the Collective CID as canonical; legacy slugs become an alias to migrate.
- **Operator direction:** extend the gossip wire format now (end-state), build the agent→hub derivation (also unblocks hub aggregation scalability), and thread storage-tier awareness (cache→available→storage→archive ≈ tiered-quilt classes drawn/stocked-warm/stocked/shelved) without baking in a single availability class. See memory `project_storage_tiering_placement_intelligence`.
- **Predecessor that landed first:** Epic B committed-accounting readers (commit `e6300665c`) — `peer_capacity_service` reads real total/pledges/held. This plan is the prioritizer half of Epic B.

### P2P Design Gate output (entities)

| Entity | Class | Address | Source of truth | New DHT type? |
|---|---|---|---|---|
| Inventory-gossip `hints` field | C (operational) | rides `BlobAddress` (sha256/CID) | the notarized sources it mirrors | No |
| peer/agent → hub derivation | **A2 (derived-via-link)** | Collective CID `collective:{hash}` | `Collective`+`Membership` DHT entries (exist) | No |
| Storage-tier dimension (`tier` hint) | C (operational) | n/a (a class label) | tiered-quilt `quilt_tier_state` (future) | No |
| HIGH-priority blob-fetch queue | C (operational) | blob-hash | reconstructed from gossip+reconcile | No |

**Critical design constraint:** `recipient_hub_id` has no per-blob source today, so the prioritizer cannot match until Wave 2 lands. The wire field (Wave 1) is therefore landed *but left dormant* until its producer (Wave 2 derivation, consumed in Wave 3) exists — per the **correct-but-dormant** discipline. Do NOT wire the prioritizer into the receive arm before Wave 3, or it is a guaranteed no-op.

---

## File Structure

**Wave 1 — wire shape (additive, reversible):**
- Modify: `elohim/elohim-storage/src/p2p/inventory_gossip.rs` — add `BlobHint` struct + `#[serde(default)] hints: Vec<BlobHint>` on `BlobInventorySnapshot` and `BlobInventoryDelta`; tests.
- (No broadcaster/receiver behavior change in Wave 1 — emit empty `hints`, ignore inbound `hints`.)

**Wave 2 — hub derivation (new Category-C projection of A2-derived data):**
- Create: `elohim/elohim-storage/migrations/2026-05-29-NNNNNN_collective_membership_projection/{up,down}.sql` — `collective_memberships` projection table (or extend `collective_participations` if it already fits).
- Create/Modify: `elohim/elohim-storage/src/db/collective_memberships.rs` — CRUD + `resolve_collective_for_agent(agent_cid) -> Option<String>` and `list_members_for_collective(collective_cid) -> Vec<String>`.
- Modify: `elohim/elohim-storage/src/services/hub_capacity_service.rs` — replace `resolve_hub_members` stub with the real resolver; `classify_hub` keyed on Collective-CID + slug-alias.
- Modify: the post-commit signal/projector path that lands `Membership` (reconcile dispatcher) — project Membership entries into the new table.
- Modify: `elohim/elohim-storage/src/services/<hub_resolver>.rs` (new helper) — `peer_id → agent_cid` (via `peer_identity_bindings`) → `collective_cid` chain.

**Wave 3 — prioritizer wiring (consume hints, fetch):**
- Modify: `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs` — `build_snapshot`/`build_delta` accept a `&[BlobHint]`; new `gather_hints(conn, hashes)` joins `content` (epr_kind/size) + content→author→Collective (recipient_hub_id) + tier.
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — broadcast path builds hints; receive arm (~5211/5262) builds `AdvertisedBlob` from `hints`, calls `score_advertised_blob`, enqueues `FetchPriority::High` into a new blob-fetch queue.
- Modify: `elohim/elohim-storage/src/services/replication_prioritizer.rs` — add a `from_db_commitments` loader that maps stored `replicates-dwelling` commitments → `ActiveCommitment` (parse `metadata_json`).
- Create: a blob-priority fetch queue mirroring `RaceFetchKicker` (semaphore-bounded `race_fetch` + `finalize_fetch_success`).

---

## Wave 1 — Inventory wire-format end-state

**Outcome:** the gossip wire carries optional per-blob hints, backward-compatible (old↔new peers interoperate), with the wire shape locked so later waves never re-touch the protocol. No behavior change yet.

**Dispatch note (rust-architect):** apply *Wire-format evolution* + *Verification gate*. Keep `hashes`/`added`/`removed` REQUIRED (they disambiguate snapshot vs delta). Set `CARGO_TARGET_DIR` to the family slot; `RUSTFLAGS='--cfg getrandom_backend="custom"'`. Stage only `inventory_gossip.rs`.

### Task 1: `BlobHint` struct + `hints` fields

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/inventory_gossip.rs`
- Test: same file `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn snapshot_round_trips_with_hints() {
    let snap = BlobInventorySnapshot {
        peer_id: "peer:A".into(),
        hashes: vec![BlobAddress::try_from("sha256-aa".to_string()).unwrap()],
        hints: vec![BlobHint {
            address: BlobAddress::try_from("sha256-aa".to_string()).unwrap(),
            recipient_hub_id: Some("collective:abc".into()),
            epr_kind: Some("content".into()),
            size_bytes: Some(4096),
            tier: Some("stocked".into()),
        }],
        snapshot_at: 1,
        sequence: 1,
        signature: vec![0x00],
    };
    let bytes = snap.to_bytes().unwrap();
    let back = BlobInventorySnapshot::from_bytes(&bytes).unwrap();
    assert_eq!(back, snap);
}

#[test]
fn snapshot_without_hints_key_decodes_to_empty() {
    // Simulate an OLD-format snapshot: a struct/value carrying no `hints` key.
    // Encode a legacy-shaped map (peer_id, hashes, snapshot_at, sequence, signature)
    // via a serde_json::json! round-tripped through rmp, OR encode a twin struct
    // lacking `hints`. Assert it decodes with hints == empty Vec.
    let legacy = LegacySnapshot {
        peer_id: "peer:A".into(),
        hashes: vec![BlobAddress::try_from("sha256-aa".to_string()).unwrap()],
        snapshot_at: 1,
        sequence: 1,
        signature: vec![0x00],
    };
    let bytes = rmp_serde::to_vec_named(&legacy).unwrap();
    let back = BlobInventorySnapshot::from_bytes(&bytes).unwrap();
    assert!(back.hints.is_empty());
    assert_eq!(back.hashes.len(), 1);
}

#[test]
fn hints_do_not_break_snapshot_delta_disambiguation() {
    // A delta carrying hints must still NOT decode as a snapshot (no `hashes`).
    let delta = BlobInventoryDelta {
        peer_id: "peer:A".into(),
        added: vec![BlobAddress::try_from("sha256-bb".to_string()).unwrap()],
        removed: vec![],
        hints: vec![],
        emitted_at: 1,
        sequence: 2,
        signature: vec![0x01],
    };
    let bytes = delta.to_bytes().unwrap();
    assert!(BlobInventorySnapshot::from_bytes(&bytes).is_err()); // missing required `hashes`
    assert!(BlobInventoryDelta::from_bytes(&bytes).is_ok());
}
```

`LegacySnapshot` is a test-only twin struct with the pre-hints field set (define it in the test module).

- [ ] **Step 2: Run to verify they fail** — `cargo test --lib inventory_gossip` → FAIL (`BlobHint` / `hints` not defined).

- [ ] **Step 3: Implement**

```rust
/// Optional per-blob enrichment carried alongside `hashes`/`added`. Advisory
/// transport metadata (Category C) — never authority. Sparse: a hint is present
/// only when the broadcaster can populate at least one field. Keyed by `address`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobHint {
    pub address: BlobAddress,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_hub_id: Option<String>, // Collective CID of the blob's owning hub
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epr_kind: Option<String>,         // content_format
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,             // tiered-quilt class: drawn|stocked-warm|stocked|shelved
}
```

Add to both messages (do NOT add `#[serde(default)]` to `hashes`/`added`/`removed`):
```rust
pub struct BlobInventorySnapshot {
    pub peer_id: String,
    pub hashes: Vec<BlobAddress>,
    #[serde(default)]
    pub hints: Vec<BlobHint>,
    pub snapshot_at: i64,
    pub sequence: u64,
    pub signature: Vec<u8>,
}
// BlobInventoryDelta: same additive `#[serde(default)] pub hints: Vec<BlobHint>`.
```

Update the existing construction sites (broadcaster `build_snapshot`/`build_delta`, any test helpers) to set `hints: Vec::new()`.

- [ ] **Step 4: Run to verify pass** — `cargo test --lib inventory_gossip` → PASS. Then full crate: `cargo test --lib` → all green (the broadcaster + receive-arm sites compile with empty hints).

- [ ] **Step 5: fmt + clippy** — `cargo fmt -p elohim-storage` (stage only `inventory_gossip.rs` + any construction site you touched), `cargo clippy --lib -- -D warnings` (isolate your warnings from pre-existing).

- [ ] **Step 6: Commit** — `git add <only the files you changed>` then commit `feat(storage): add optional BlobHint to inventory gossip wire (additive, backward-compatible)`.

---

## Wave 2 — Hub derivation (Collective/Membership projection)

**Outcome:** storage can resolve `agent_cid → collective_cid` and `collective_cid → members`, keyed on the canonical Collective CID; `hub_capacity_service::resolve_hub_members` stops being a single-device stub (the aggregation-scalability win).

**Design (authored to bite-sized TDD at execution start — depends on confirming the Membership signal path):**
- **Projection table** `collective_memberships` (Category C; `-- Source of truth: imagodei DHT Collective+Membership entries`): `collective_cid TEXT, member_cid TEXT, member_kind TEXT, role TEXT, dht_anchor_hash TEXT NOT NULL, valid_from/valid_until, PRIMARY KEY (collective_cid, member_cid)`. Confirm whether existing `collective_participations` already covers this before creating a new table.
- **Projector:** the post-commit signal for `Membership` lands via the reconcile dispatcher (per the `ReconcileController` discipline — projection WRITES go through the controller, not a service). Verify the Membership signal exists or add it.
- **Resolver helper** (read-side, inline diesel): `resolve_collective_for_agent(conn, agent_cid) -> Result<Option<String>>` and `list_members_for_collective(conn, collective_cid) -> Result<Vec<String>>`. Chain `peer_id → agent_cid` via `db::peer_identity_bindings::lookup_active`, then `agent_cid → collective_cid`.
- **hub_capacity_service:** `resolve_hub_members` calls `list_members_for_collective` when `hub_id` is a `collective:` CID; falls back to the existing `stewarded_nodes.household_id` slug join via a **slug→CID alias** during transition. `classify_hub` derives kind from the resolved Collective's `kind`/`governance_layer`, not a string prefix.

**Acceptance criteria:**
- `resolve_collective_for_agent` returns the Collective CID for an agent with a projected Membership; `None` otherwise.
- `hub_capacity_service` aggregates over real members for a `collective:` hub (regression test seeding memberships + peer_capacity rows).
- Legacy `household-matthew` slug still resolves via the alias (no regression to `GET /api/v1/hub/{id}/capacity`).
- No new DHT entry type; the projection table declares its DHT source-of-truth.

**Open sub-questions to resolve at execution:** does a `Membership` post-commit signal/projector already exist? what is `content → author agent` (needed in Wave 3 to derive a blob's owning hub)? does `collective_participations` already serve as the projection?

**Dispatch note (rust-architect):** *Read-side inline diesel* for the resolver; *ReconcileController* for the projection write; A2 classification (derived-via-link on existing Collective entry — no new entry type). Diesel migration timestamp discipline (bump seconds). Sweettest if the Membership signal path changes.

---

## Wave 3 — Prioritizer wiring (populate hints + consume + fetch)

**Outcome:** a peer with an active `replicates-dwelling` commitment for hub H proactively fetches advertised blobs owned by H. End-to-end "commitments shape what peers cache."

**Design (authored to bite-sized TDD at execution start — depends on Waves 1+2):**
- **Broadcaster `gather_hints(conn, &hashes) -> Vec<BlobHint>`:** for each held blob_hash, join `content.blob_hash` → `content_format` (epr_kind) + `content_size_bytes` (size); derive `recipient_hub_id` via content→author_agent→`resolve_collective_for_agent` (Wave 2); `tier` left `None` until the tiered-quilt TierController lands. `build_snapshot`/`build_delta` take `&[BlobHint]`.
- **`ActiveCommitment` loader** in `replication_prioritizer.rs`: `active_commitments_for_provider(conn, self_cid) -> Vec<ActiveCommitment>` — query `rea_commitments` (`provider==self_cid`, `action='replicates-dwelling'`, state not cancelled/terminated), parse `metadata_json` (`ReplicatesDwellingPayload`) into `ActiveCommitment{commitment_cid, action, recipient_hub_id=recipient_dwelling_hub_id, scope_epr_kinds=scope_filter.epr_kinds, bytes_per_blob_max=scope_filter.bytes_per_blob_max}`.
- **Receive arm (`p2p/mod.rs` ~5211/5262):** build a `HashMap<String, &BlobHint>` from `hints`; for each received address, construct `AdvertisedBlob{blob_cid, source_peer_cid, recipient_hub_id_hint, epr_kind_hint, blob_size_bytes}`; call `score_advertised_blob` against the cached `active_commitments`; on `High`, enqueue the blob hash. Query/cache `active_commitments` per the `inventory_freshness_seconds` TTL pattern, not per-message.
- **Blob-fetch seam:** mirror `RaceFetchKicker` — a semaphore-bounded spawn that resolves candidates via `db::peer_blob_inventory::lookup_hosts`, calls `race_fetch` then `finalize_fetch_success`. Category C; reconstructed from gossip+reconcile (no persistence).

**Acceptance criteria:**
- Unit: `active_commitments_for_provider` parses a seeded `replicates-dwelling` row into a matching `ActiveCommitment`.
- Unit: receive-arm builds an `AdvertisedBlob` whose hints come from the gossip `hints` map; a `High` score enqueues exactly the matching blob.
- Integration: a peer with a `collective:H` commitment, receiving a snapshot advertising a blob hinted `recipient_hub_id=collective:H`, issues a fetch; a blob hinted for a different hub does not.
- No regression to the content-id `gap_queue`/`drain_gap_queue` path (separate seam).

**Dispatch note (rust-architect):** *Correct-but-dormant* no longer applies — recipient_hub_id is now populated, so wiring the consumer is correct. *Read-side inline diesel* for both the hint join and the commitment loader. *Verification gate* — full lib suite + clippy; the receive arm is async-with-sync-diesel (no `spawn_blocking`, matching the existing arm).

---

## Tier dimension (threaded, not built)

The `tier` hint (Wave 1) is optional and aligned to the tiered-quilt classes `drawn | stocked-warm | stocked | shelved` (`2026-05-11-tiered-quilt-stewardship-design.md`). This plan only carries it on the wire and leaves room for it in `score_advertised_blob` (a later tier-aware ranking). The full `quilt_tier_state` projection + `custody-quilt` commitment + `TierController` is the separate 8-wave tiered-quilt epic (`project_tiered_quilt_spec`), out of scope here. **Do not** implement TierController in this plan; only ensure nothing here forecloses it (the wire slot + the `tier` field on `BlobHint` are the hooks).

---

## Test & Verification Strategy

- Per wave: TDD red→green; `cargo fmt` + `clippy -D warnings` + **full `cargo test --lib`** (readers/projections feed aggregators + routes).
- Wire (Wave 1): round-trip + old↔new compat + disambiguation-preserved (the three Task-1 tests).
- Cross-crate: if any ts-rs-anchored view changes, `cargo build --workspace` + before/after `rg '^impl From<'`.
- Each wave commits independently and leaves the crate green. Stage only changed files (operator fmt sweep is uncommitted).

## Self-Review

- **Spec coverage:** wire hints (W1) ✓ · hub derivation + canonical Collective CID (W2) ✓ · prioritizer populate+consume+fetch (W3) ✓ · tier hint threaded (W1, full tiering deferred) ✓ · zero new DHT types (gate) ✓.
- **Placeholder scan:** Wave 1 is fully coded; Waves 2–3 are deliberately design+acceptance (their code depends on landed types — detailed TDD authored at each wave's start, per the per-subsystem-plan discipline). This is intentional, not a placeholder gap.
- **Type consistency:** `BlobHint` fields (`address`, `recipient_hub_id`, `epr_kind`, `size_bytes`, `tier`) map to `AdvertisedBlob` (`recipient_hub_id_hint`, `epr_kind_hint`, `blob_size_bytes`) — the rename at the receive-arm construction site is explicit in Wave 3.
