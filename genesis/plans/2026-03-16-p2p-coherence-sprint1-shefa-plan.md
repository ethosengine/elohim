# P2P Coherence Sprint 1: Shefa (Economics) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire `dht_anchor_hash` provenance links into the remaining 4 unanchored shefa tables, add source-of-truth comments to all 9 shefa tables, reclassify steward_affinity as operational, and verify end-to-end provenance on the 4 already-anchored tables.

**Architecture:** Each table gets a diesel migration adding `dht_anchor_hash TEXT`, the model struct gains the field, the View struct exposes it (camelCase via serde), TypeScript types are regenerated, and source-of-truth comments document every table's classification. steward_affinity is confirmed operational with a documented reconstruction strategy.

**Tech Stack:** Diesel migrations (SQLite), Rust (models.rs, views.rs), ts-rs type generation, TypeScript (storage-client-ts)

**P2P Design Gate Classification (completed):**
- stewardship_allocations → A2 (Derived, anchored via parent Agreement)
- steward_credentials → A (Notarized, maps to Attestation in imagodei)
- access_grants → A (Notarized, maps to Attestation in imagodei)
- premium_gates → A (Notarized, linked from Content)
- steward_affinity → C (Operational, reconstructable from curation events)

---

### Task 1: Migration — Add dht_anchor_hash to stewardship_allocations

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-16-000000_shefa_provenance/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-16-000000_shefa_provenance/down.sql`

**Step 1: Create the migration directory**

```bash
mkdir -p elohim/elohim-storage/migrations/2026-03-16-000000_shefa_provenance
```

**Step 2: Write the up.sql migration**

This single migration handles all 4 tables plus source-of-truth comments. SQLite doesn't support `ALTER TABLE ADD COMMENT`, so comments go as SQL line comments before each ALTER.

```sql
-- P2P Coherence Sprint 1: Shefa Provenance
-- Adds dht_anchor_hash to unanchored shefa tables.
-- Each table's source of truth is documented inline.

-- stewardship_allocations: Source of truth: DHT (derived from Agreement via Link)
-- Classification: A2 (Derived) — anchored via parent Agreement's ActionHash
ALTER TABLE stewardship_allocations ADD COLUMN dht_anchor_hash TEXT;

-- steward_credentials: Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — maps to Attestation with type=credential
ALTER TABLE steward_credentials ADD COLUMN dht_anchor_hash TEXT;

-- premium_gates: Source of truth: DHT (Link on Content entry in lamad DNA)
-- Classification: A (Notarized) — anchored via parent Content's ActionHash
ALTER TABLE premium_gates ADD COLUMN dht_anchor_hash TEXT;

-- access_grants: Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — maps to Attestation with type=access
ALTER TABLE access_grants ADD COLUMN dht_anchor_hash TEXT;

-- steward_affinity: Source of truth: SQLite (operational)
-- Classification: C (Operational) — reconstructable from economic_events curation acts
-- Reconstruction: re-derive from economic_events WHERE action='curate' grouped by steward+content
-- No dht_anchor_hash needed.

-- Source-of-truth comments for already-anchored tables:
-- economic_events: Source of truth: DHT (EconomicEvent in lamad DNA). dht_anchor_hash added in migration 2026-03-10-100000.
-- rea_commitments: Source of truth: DHT (Commitment in lamad DNA). dht_anchor_hash added in migration 2026-03-10-100000.
-- agreements: Source of truth: DHT (Agreement in lamad DNA). dht_anchor_hash present since table creation.
-- stewarded_nodes: Source of truth: DHT (StewardedResource in lamad DNA). dht_anchor_hash present since table creation.
-- node_stewardship: Source of truth: SQLite (operational). Derived from stewarded_nodes relationships. No dht_anchor_hash needed.
```

**Step 3: Write the down.sql migration**

```sql
-- SQLite doesn't support DROP COLUMN before 3.35.0, so we use the recreate pattern.
-- For dev environments, this is acceptable. Production would need a more careful approach.

-- Note: SQLite 3.35.0+ supports ALTER TABLE DROP COLUMN
ALTER TABLE stewardship_allocations DROP COLUMN dht_anchor_hash;
ALTER TABLE steward_credentials DROP COLUMN dht_anchor_hash;
ALTER TABLE premium_gates DROP COLUMN dht_anchor_hash;
ALTER TABLE access_grants DROP COLUMN dht_anchor_hash;
```

**Step 4: Run the migration**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo run -- migrate
```

Expected: Migration applies successfully. 4 columns added.

**Step 5: Verify schema updated**

```bash
cd elohim/elohim-storage
sqlite3 data/elohim-storage.db ".schema stewardship_allocations" | grep dht_anchor_hash
sqlite3 data/elohim-storage.db ".schema steward_credentials" | grep dht_anchor_hash
sqlite3 data/elohim-storage.db ".schema premium_gates" | grep dht_anchor_hash
sqlite3 data/elohim-storage.db ".schema access_grants" | grep dht_anchor_hash
```

Expected: Each shows `dht_anchor_hash TEXT` in the column list.

**Step 6: Update diesel_schema.rs**

After migration, regenerate the diesel schema:

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' diesel print-schema > src/db/diesel_schema.rs
```

Or manually add `dht_anchor_hash -> Nullable<Text>,` to the 4 table macros in `src/db/diesel_schema.rs`:
- `stewardship_allocations` table (around line 298-325)
- `steward_credentials` table (around line 605-615)
- `premium_gates` table (around line 618-628)
- `access_grants` table (around line 631-640)

**Step 7: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-16-000000_shefa_provenance/
git add elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(shefa): add dht_anchor_hash to stewardship_allocations, steward_credentials, premium_gates, access_grants

Source-of-truth comments document all 9 shefa tables.
steward_affinity confirmed as operational (C) with reconstruction strategy."
```

---

### Task 2: Model Structs — Add dht_anchor_hash field

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`

**Step 1: Add dht_anchor_hash to StewardshipAllocation struct**

Find the `StewardshipAllocation` struct (around line 990-1016). Add after the `updated_at` field:

```rust
    pub dht_anchor_hash: Option<String>,
```

**Step 2: Add dht_anchor_hash to steward_credentials model**

Find the struct for steward_credentials (search for `StewardCredential`). Add:

```rust
    pub dht_anchor_hash: Option<String>,
```

**Step 3: Add dht_anchor_hash to premium_gates model**

Find the struct for premium_gates (search for `PremiumGate`). Add:

```rust
    pub dht_anchor_hash: Option<String>,
```

**Step 4: Add dht_anchor_hash to access_grants model**

Find the struct for access_grants (search for `AccessGrant`). Add:

```rust
    pub dht_anchor_hash: Option<String>,
```

**Step 5: Add reconstruction comment to StewardAffinity**

Find `StewardAffinity` struct (around line 958-967). Add a doc comment:

```rust
/// Operational (Category C): reconstructable from economic_events WHERE action='curate'
/// grouped by (steward_id, content_id). No dht_anchor_hash needed.
pub struct StewardAffinity {
```

**Step 6: Verify compilation**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: Compiles successfully. May show warnings about unused fields — that's fine, the views will use them next.

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/db/models.rs
git commit -m "feat(shefa): add dht_anchor_hash field to 4 model structs, document steward_affinity as operational"
```

---

### Task 3: View Structs — Expose dht_anchor_hash to API boundary

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

**Step 1: Add dht_anchor_hash to StewardshipAllocationView**

Find `StewardshipAllocationView` (around line 847-877). Add after `updated_at`:

```rust
    pub dht_anchor_hash: Option<String>,
```

Update the corresponding `From<StewardshipAllocation>` impl to map the field:

```rust
    dht_anchor_hash: a.dht_anchor_hash,
```

**Step 2: Add dht_anchor_hash to StewardCredentialView**

Find the View struct for steward_credentials. Add:

```rust
    pub dht_anchor_hash: Option<String>,
```

Update the `From` impl.

**Step 3: Add dht_anchor_hash to PremiumGateView**

Find the View struct for premium_gates. Add:

```rust
    pub dht_anchor_hash: Option<String>,
```

Update the `From` impl.

**Step 4: Add dht_anchor_hash to AccessGrantView**

Find the View struct for access_grants. Add:

```rust
    pub dht_anchor_hash: Option<String>,
```

Update the `From` impl.

**Step 5: Verify StewardedNodeView already exposes dht_anchor_hash**

Check `StewardedNodeView` (around line 4219-4234). It currently does NOT expose `dht_anchor_hash` despite the model having it. Add the field:

```rust
    pub dht_anchor_hash: Option<String>,
```

Update its `From` impl to map from the model.

**Step 6: Verify EconomicEventView, ReaCommitmentView, AgreementView already expose it**

These 3 views already have `dht_anchor_hash: Option<String>`. Confirm they're correctly mapped in their `From` impls.

**Step 7: Verify compilation**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: Clean compilation.

**Step 8: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(shefa): expose dht_anchor_hash in 5 View structs (4 new + StewardedNodeView fix)"
```

---

### Task 4: Regenerate TypeScript Types

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/generated/*.ts` (auto-generated)

**Step 1: Run the type export**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings -- --nocapture
```

Expected: Test passes. Generated TypeScript files updated in `elohim/sdk/storage-client-ts/src/generated/`.

**Step 2: Verify the new fields appear**

```bash
grep -l "dhtAnchorHash" elohim/sdk/storage-client-ts/src/generated/*.ts
```

Expected: Should include at minimum:
- `StewardshipAllocationView.ts`
- `StewardCredentialView.ts` (or similar name)
- `PremiumGateView.ts` (or similar name)
- `AccessGrantView.ts` (or similar name)
- `StewardedNodeView.ts`
- `EconomicEventView.ts` (already had it)
- `ReaCommitmentView.ts` (already had it)
- `AgreementView.ts` (already had it)

**Step 3: Build the TypeScript package**

```bash
cd elohim/sdk/storage-client-ts
pnpm build
```

Expected: Builds successfully with the new type fields.

**Step 4: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(shefa): regenerate TypeScript types with dhtAnchorHash on shefa View types"
```

---

### Task 5: Add source-of-truth comments to existing migration files

**Files:**
- Modify: `elohim/elohim-storage/migrations/2026-01-10-300000_economic_events/up.sql`
- Modify: `elohim/elohim-storage/migrations/2026-01-10-500000_stewardship_allocations/up.sql`
- Modify: `elohim/elohim-storage/migrations/2026-03-10-000000_rea_commitments/up.sql`
- Modify: `elohim/elohim-storage/migrations/2026-03-10-200000_agreements/up.sql`
- Modify: `elohim/elohim-storage/migrations/2026-03-12-000000_stewarded_nodes/up.sql`
- Modify: `elohim/elohim-storage/migrations/2026-03-14-000000_steward_affinity/up.sql`

**Step 1: Add source-of-truth header to each migration**

Prepend a comment block to each existing migration's `up.sql`:

For `economic_events/up.sql`:
```sql
-- Source of truth: Holochain DHT (EconomicEvent entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to EconomicEvent ActionHash
```

For `stewardship_allocations/up.sql`:
```sql
-- Source of truth: Holochain DHT (derived from Agreement via Link)
-- Classification: A2 (Derived) — dht_anchor_hash links to parent Agreement ActionHash
```

For `rea_commitments/up.sql`:
```sql
-- Source of truth: Holochain DHT (Commitment entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Commitment ActionHash
```

For `agreements/up.sql`:
```sql
-- Source of truth: Holochain DHT (Agreement entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Agreement ActionHash
```

For `stewarded_nodes/up.sql`:
```sql
-- Source of truth: Holochain DHT (StewardedResource entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to StewardedResource ActionHash
-- node_stewardship: Source of truth: SQLite (operational). Classification: C.
```

For `steward_affinity/up.sql`:
```sql
-- Source of truth: SQLite (operational)
-- Classification: C (Operational) — reconstructable from economic_events curation acts
-- Reconstruction: SELECT steward_id, content_id, SUM(value) FROM economic_events WHERE action='curate' GROUP BY steward_id, content_id
```

**Step 2: Commit**

```bash
git add elohim/elohim-storage/migrations/
git commit -m "docs(shefa): add source-of-truth and P2P classification comments to all shefa migrations"
```

---

### Task 6: Verify end-to-end provenance on already-anchored tables

**Files:**
- Read: `elohim/elohim-storage/src/api/economic_events.rs`
- Read: `elohim/elohim-storage/src/api/rea_commitments.rs`
- Read: `elohim/elohim-storage/src/api/agreements.rs`

**Step 1: Verify economic_events create handler populates dht_anchor_hash**

Read `src/api/economic_events.rs` and find the create handler. Verify that when an economic event is created, the `dht_anchor_hash` field is populated. If creation goes through a zome call → post-commit signal → storage projection, the anchor hash should come from the signal. If creation goes directly to storage (bulk import), the anchor hash may be null — document this as a known gap for backfill.

**Step 2: Verify rea_commitments create handler populates dht_anchor_hash**

Same check for commitments.

**Step 3: Verify agreements create handler populates dht_anchor_hash**

Same check for agreements.

**Step 4: Verify API responses include dht_anchor_hash**

```bash
# If dev environment is running:
curl -s http://localhost:8090/api/v1/economic-events?limit=1 | python3 -m json.tool | grep dhtAnchorHash
curl -s http://localhost:8090/api/v1/commitments?limit=1 | python3 -m json.tool | grep dhtAnchorHash
curl -s http://localhost:8090/api/v1/agreements?limit=1 | python3 -m json.tool | grep dhtAnchorHash
```

Expected: `dhtAnchorHash` field present in responses (may be null for pre-coherence data).

**Step 5: Document findings**

If any create handler does NOT populate dht_anchor_hash, file a TODO comment in the handler:

```rust
// TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
// Currently null for direct storage writes. Backfill needed for pre-coherence data.
```

**Step 6: Commit if any changes made**

```bash
git add elohim/elohim-storage/src/api/
git commit -m "audit(shefa): document provenance gaps in existing anchored table handlers"
```

---

### Task 7: Run full quality gate

**Step 1: Rust compilation check**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: Clean compilation.

**Step 2: Rust tests**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

Expected: All existing tests pass. New fields are Optional so no existing code breaks.

**Step 3: Clippy**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
```

Expected: No warnings.

**Step 4: Format check**

```bash
cd elohim/elohim-storage
cargo fmt --check
```

Expected: Clean formatting.

**Step 5: TypeScript build**

```bash
cd elohim/sdk/storage-client-ts
pnpm build
```

Expected: Clean build with updated types.

**Step 6: Verify p2p-schema-audit hook is clean on modified files**

```bash
echo '{"tool_input":{"file_path":"/projects/elohim/elohim/elohim-storage/src/views.rs"}}' | \
  CLAUDE_PROJECT_DIR=/projects/elohim python3 .claude/hooks/p2p-schema-audit.py
```

Expected: Warnings should be reduced compared to before (the new View structs now have dht_anchor_hash). Pre-existing views from other pillars will still warn — that's expected and will be fixed in Sprints 2-4.

**Step 7: Final commit if any formatting fixes needed**

```bash
git add -A
git commit -m "chore(shefa): formatting and lint fixes for Sprint 1 coherence changes"
```

(Only commit if changes exist. Skip if clean.)
