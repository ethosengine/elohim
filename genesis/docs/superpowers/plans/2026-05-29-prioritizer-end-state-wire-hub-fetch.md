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

## Wave 2 — Hub identity (CID canonical + slug alias) + Membership projection

> **Operator decision (2026-05-29): hybrid + correct-now.** Collective CID `collective:{action_hash}` is the **canonical** identity; **slug** (`family-dowell`) is a **first-class steward-configurable alias** (human names + SEO in the elohim network, not just the seeder), resolving to the CID. Do the full path: reconcile the slug-keyed SQL onto the canonical CID AND add the DNA `MembershipCommitted` signal + storage projector. See memory `project_hub_identity_cid_canonical_slug_alias`.

**Outcome:** storage resolves `agent_cid → owning hub (canonical Collective CID)` and `hub → members`, with slug↔CID aliasing; `hub_capacity_service::resolve_hub_members` stops being a single-device stub (the device→hub aggregation-scalability win); live DHT `Membership` entries project into `collective_participations`.

**Resolved facts (Wave-2 scout, 2026-05-29):**
- `collective_participations` ALREADY exists, is annotated `-- Source of truth: DHT ... Classification: A2`, resolves both directions (`collectives.rs::get_participations_for_human` / `get_participants_of_collective`), seed/HTTP-populated only. **Extend it; do NOT create a new table.**
- Local SQL is **slug-keyed**: `collectives.id` = slug (`family-dowell`); `humans.household_id` + `stewarded_nodes.household_id` point at slugs; `collective_participations.collective_id` = slug, member = `human_id` (NOT `agent_cid`). `collectives` has **no** `action_hash`/`dht_anchor_hash` column.
- imagodei DNA emits **no** Collective/Membership post-commit signal (`imagodei/src/lib.rs:198`); storage `translate_imagodei` (`reconcile/holochain_app_signal.rs:234`) has no arm. CID helpers exist: `encode_agent_cid → agent:{pubkey}`, `action_hash_to_cid → collective:{hash}` (`qahal_coordinator.rs:377-385`).
- Wave-3 chain is mostly built: `content.blob_hash` (hop1) → `content.created_by` IS the agent_cid (hop2, no join) → hub (hop3 = this wave). `peer_topology_view::compute_resilience_cliffs:275` is the prototype that stops at agent_cid for exactly this missing hop. `HubKind` doc (`elohim-views/infrastructure.rs:1804-1825`) already specifies Dwelling=`humans.household_id` binding / Collective=`collective_participations` binding / Computed=none.
- `peer_identity_bindings` is bidirectional (`lookup_active` peer→agent, `list_active_for_agent` agent→peers).

**Task order — land read-side value first; isolate the DNA deploy risk (T5–T6) last:**

- **T1 (migration + models):** `collectives` gains `collective_cid TEXT` (canonical `collective:{action_hash}`, nullable pre-coherence) + keep `id` as the slug + a `slug` accessor; `collective_participations` gains `member_cid TEXT` (canonical `agent:{pubkey}`/`collective:{hash}`), `member_kind TEXT DEFAULT 'person'`, `dht_anchor_hash TEXT`. Migration comment: source-of-truth DHT, A2. Diesel models + `NewX` updated. Backfill note: existing slug rows keep working (resolver falls back to slug when `collective_cid` is NULL). Bump migration seconds (collision discipline).
- **T2 (resolver, read-side inline diesel, NO DNA):** `db`/service helpers `resolve_owning_hub(conn, agent_cid) -> Option<HubId>` (prefer `humans.household_id → collectives`; emit `collective_cid` if set else slug), `resolve_member_collectives(conn, agent_cid) -> Vec<HubId>` (via `collective_participations`), `members_of_hub(conn, hub_id) -> Vec<...>` (slug- or CID-keyed), and `slug_to_cid`/`cid_to_slug` alias lookups. **Verify at execution:** does `content.created_by` (`agent:{pubkey}`) join to `humans.id` or `humans.agent_pub_key`? (codebase uses both — `stewardship_allocations.rs:216` = `humans.id`; `household_resilience.rs:271` = `humans.agent_pub_key`). Retrofit the `peer_topology_view:275` gap to use the resolver.
- **T3 (hub_capacity_service rewrite):** `resolve_hub_members` = CID-first, slug-alias fallback, over the existing slug joins (`list_by_household_with_peer_status`) + `collective_participations`; `classify_hub` from `collectives.governance_layer` (`family`→Dwelling) / membership presence, not a string prefix; `display_label` from the Collective name. **Identity caveat:** `compute_peer_capacity` keys pledges on `rea_commitments.provider` (agent CID) but raw-capacity on libp2p peer id — map node→agent via `humans.agent_pub_key` (mirror `household_resilience.rs:271`) or pledges read 0.
- **T4 (DNA signal — isolated, carries deploy risk):** add `ImagodeiSignal::MembershipCommitted { action_hash, membership, author }` (+ `CollectiveCommitted` if needed) to `imagodei/src/lib.rs:149` enum + emit in `post_commit` (and/or in `create_collective`/membership coordinator fns). Mirror the variant in storage `signals.rs:822`. Sweettest the emit. **DNA-redeploy gotcha applies** (memory `project_dna_changes_dont_redeploy_without_forced_reinstall`) — new DNA hash needs forced reinstall on alpha.
- **T5 (storage projector):** `translate_imagodei` arm: `MembershipCommitted → upsert collective_participations` keyed by `dht_anchor_hash`, writing `member_cid`, `member_kind`, `collective_id` (resolve to the collectives row; project the Collective entry first if absent), `role_context`, `departed_at` from `withdrawn_at_block_height`. Reuse `create_participation`'s upsert. Idempotency key → `(h_app_id, collective_id, member_cid)` or `dht_anchor_hash`.
- **T6 (reconciliation):** align `humans.household_id` / `stewarded_nodes.household_id` reads to resolve through the slug↔CID alias so both representations work; converge seed commitments' `recipient_dwelling_hub_id` onto the canonical CID where a DHT anchor exists (so Wave-3 hint↔commitment match is representation-consistent).

**Acceptance:** `resolve_owning_hub(agent)` returns the canonical CID (or slug pre-coherence); `hub_capacity_service` aggregates real members for both a `collective:`-CID and a legacy slug (no regression to `GET /api/v1/hub/{id}/capacity`); a projected `MembershipCommitted` lands a `collective_participations` row keyed by `dht_anchor_hash`; `slug_to_cid`/`cid_to_slug` round-trip; zero new DHT entry types. Each task commits independently, crate green.

**Gate note:** covered by the prioritizer-epic p2p-design-gate — hub derivation = A2 (derived-via-link on the existing `Collective`/`Membership` entries, no new entry type); `slug` = Category-C operational alias; the `MembershipCommitted` signal is A2-projection wiring (the entry already exists), not a new type.

**Dispatch note (rust-architect):** *Read-side inline diesel* for T2/T3 resolvers; *ReconcileController* discipline for T5 (projection writes go through the signal→translate→projector path, not a service); migration-timestamp + sweettest discipline for T1/T4. Land T1–T3 (read-side, seed-data-functional) before T4–T5 (DNA + deploy risk).

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
