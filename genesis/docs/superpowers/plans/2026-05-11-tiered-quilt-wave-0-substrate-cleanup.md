# Tiered Quilt — Wave 0: Substrate cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land two substrate corrections that the tiered-quilt waves depend on, with **zero new feature code**: (1) resolve the duplicate `Attestation` entry type across elohim + imagodei DNAs, (2) rename the field `lamad_event_type` → `elohim_event_type` across the entire stack (83 files affected).

**Architecture:** Single sequenced PR. Stage A handles the Attestation dedupe; Stage B handles the rename. Stages share a feature branch and a single integration test pass before merge.

**Tech Stack:** Rust 2021 (elohim DNA + imagodei DNA + elohim-storage), Diesel migrations, JSON Schema codegen, TypeScript codegen, Angular 19 consumers.

**Source-of-truth spec:** `genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md` (§1 "Wave 0 substrate cleanup")

**Delivery master:** `genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-delivery-master.md` (§2 Wave 0)

---

## ⚠️ Pre-flight correction — operator decision required

The spec asserted that Wave 0 would "remove `Attestation` from imagodei DNA; single source of truth becomes the elohim DNA copy." **Deeper pre-flight against the actual codebase (2026-05-11 evening) found this direction is opposite to existing reality:**

- **imagodei DNA is currently the OWNER** of the `Attestation` entry type. `imagodei/src/lib.rs:581` `issue_attestation` actually calls `create_entry(&EntryTypes::Attestation(...))`. Real attestation entries on the DHT are issued there.
- **elohim DNA's `Attestation` enum variant is VESTIGIAL.** It is declared in the EntryTypes enum at `content_store_integrity:1052` but **no coordinator function ever instantiates it.** Verified via `grep -rn "EntryTypes::Attestation\b" elohim/holochain/dna/elohim/zomes/` returning zero hits.
- **A cross-DNA bridge already exists:** elohim DNA's `content_store/src/lib.rs:1004` defines `issue_attestation_via_imagodei` which uses `CallTargetCell::OtherRole(IMAGODEI_ROLE)` to call into imagodei DNA's `issue_attestation`. This is the canonical attestation issuance pattern today.

**Two resolutions are available. Operator must choose before Stage A dispatches.**

### Option A — Honor the user's brainstorm preference (more invasive)

Move ownership FROM imagodei TO elohim DNA. The user said during brainstorming
(2026-05-11): "let's see if we can move that to elohim core."

**What this means:**
1. Migrate `issue_attestation` coordinator (and `get_agent_attestations`, `get_my_attestations`) from imagodei DNA to elohim DNA's content_store coordinator zome.
2. Remove `Attestation` entry type from imagodei DNA integrity zome (struct + enum variant + validate handler).
3. Remove the cross-DNA bridge `issue_attestation_via_imagodei` and inline its body — the call becomes local.
4. Migrate any existing Attestation entries from imagodei DHT to elohim DHT (a one-time migration tool — minor concern at pre-launch; nontrivial post-launch).
5. Tiered-quilt waves 4–6 use elohim DNA's local `create_attestation` directly. Cleaner downstream.

**Pros:** Storage-stewardship attestations live in the same DNA as `Commitment`. No cross-DNA hops for tier-breach / tier-restitution / tier-holdings / tier-accounting. Stronger coherence: identity attestations (imagodei) stay separable but operate via the elohim DNA Attestation type.

**Cons:** Higher blast radius. Touches imagodei coordinator API surface. Any imagodei zome consumer that calls `issue_attestation` directly must move to the elohim DNA call. Pre-launch makes this tractable; post-launch this is non-trivial.

### Option B — Honor the existing implementation (less invasive)

Keep imagodei DNA as the Attestation owner. Remove only the vestigial elohim
DNA declaration.

**What this means:**
1. Remove the unused `Attestation(Attestation)` enum variant and `pub struct Attestation { ... }` from `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (lines 1052+ for the struct, ~1130 for the EntryTypes variant — need to verify exact lines).
2. Verify nothing references it: `grep -rn "EntryTypes::Attestation\b" elohim/holochain/dna/elohim/zomes/`.
3. The cross-DNA bridge `issue_attestation_via_imagodei` stays as the canonical pattern.
4. Tiered-quilt waves 4–6 issue storage-stewardship attestations VIA the bridge — same shape as today, just five new `attestation_type` discriminator strings flowing through the existing path.

**Pros:** Far smaller blast radius. Honors existing working code. The cross-DNA bridge is already wired and tested.

**Cons:** Storage-stewardship attestations cross a DNA boundary on every breach/restitution/holdings/accounting write. Cross-DNA calls have measurable latency vs local writes. The bridge marshalling adds a serialization round-trip per attestation. Conceptually muddier — "imagodei stores tier-breach attestations" reads weird if you don't know the history.

### Recommendation (this plan author)

**Option B for Wave 0; revisit Option A as a separate sprint after tiered-quilt
stabilizes.** Reasoning: Wave 0's scope is already large (the rename touches 83
files). Compounding it with Attestation ownership migration risks Wave 0
becoming a rollback nightmare. The cross-DNA latency of Option B is real but
amortizes over the per-attestation cadence (one tier-accounting per agent per
day; breach attestations are rare; holdings are periodic but throttled). The
conceptual muddiness is documentable.

**Operator: please mark which option in this plan before Stage A starts.**

> Operator choice: ☐ Option A  ☐ Option B  ☐ Other (annotate below)

---

## ⚠️ Pacing constraints (2026-05-11)

**Read these before dispatching any task in this plan.**

1. **EPR Phase 4 agent has uncommitted work** across
   `elohim-storage/{rea_projection,services,api,p2p,db}`. Stage B's rename
   touches `elohim-storage/src/db/models.rs`, `economic_events.rs`,
   `rea_projection.rs`, and views.rs — direct file overlap. **Wait for EPR
   Phase 4 to merge to `dev` before starting Stage B.** Stage A (Attestation
   dedupe) touches only DNA-side files and has no overlap; safe to start
   immediately.
2. **PVC budget is tight (118G total, 73G used at plan-author time).** Stage B
   compilation across DNA + storage + Angular will push toward the 95% scream
   threshold. **Pace cargo builds: one at a time, no concurrent crates.** Use
   `cargo-pool prune --stale-incrementals --yes` between stages.
3. **Do not run sweettest scenarios in parallel.** Existing sweettest tests
   serialize (`--test-threads=1`); pre-existing pattern; honor it.

---

## File-structure map

Wave 0 modifies the following file groups. Stages A and B are separable; tasks
within a stage may run in parallel where indicated.

### Stage A — Attestation dedupe (Option B path; Option A path differs — see §⚠️)

**Modify:**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`
  - Remove `pub struct Attestation { ... }` (~lines 1043–1064; verify exact lines pre-edit)
  - Remove `Attestation(Attestation)` variant from `EntryTypes` enum (~line 1130; verify)
  - Remove validate-handler arm (if present)

**Verify (no edits):**
- `grep -rn "EntryTypes::Attestation\b" elohim/holochain/dna/elohim/zomes/` should be empty
- `grep -rn "&Attestation\b" elohim/holochain/dna/elohim/zomes/` should not match the bare `Attestation` type (ContentAttestation and RenewalAttestation are different types and stay)

### Stage B — `lamad_event_type` → `elohim_event_type` rename

**Critical sites by group:**

**DNA (atomic group — must land together):**
- Create: `elohim/elohim-storage/migrations/2026-05-12-000000_rename_lamad_event_type/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-12-000000_rename_lamad_event_type/down.sql`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:1124` (struct field rename)
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:3960` (LinkTypes::EventByLamadType → EventByElohimType)
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:476`, `:11799` (coordinator handler refs)

**Schemas (drive codegen):**
- Modify: `elohim/sdk/schemas/v1/inputs/create-economic-event-input.schema.json:27`
- Modify: `elohim/sdk/schemas/v1/views/economic-event-view.schema.json:31`

**Rust storage (the bulk):**
- Modify: `elohim/elohim-storage/src/db/models.rs:349,390,745` (struct field + module name `lamad_event_types` → `elohim_event_types`)
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs:180` (column rename)
- Modify: `elohim/elohim-storage/src/db/economic_events.rs` (lines 12, 50, 79, 159, 160, 252, 278–284, 323, 372, 417, 456, 495, 535, 648, 704, 705)
- Modify: `elohim/elohim-storage/src/rea_projection.rs:107,219`
- Modify: `elohim/elohim-storage/src/views.rs:490,524,1291,1328`
- Modify: `elohim/elohim-storage/src/http.rs:3771`
- Modify: `elohim/elohim-storage/src/p2p/blob_fetch.rs:236`
- Modify: `elohim/elohim-storage/src/reconcile/custody.rs:204`
- Modify: `elohim/elohim-storage/src/services/economic_event_service.rs:219`
- Modify: `elohim/elohim-storage/src/services/exchange_service.rs:201,219,252,456,487,522,607,763,794,829,906`
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs:452`
- Modify: `elohim/elohim-storage/src/services/resource_service.rs:773`
- Modify: `elohim/elohim-storage/tests/reciprocity_view.rs:82` (⚠️ may conflict with EPR agent's modifications — re-verify after their merge)

**Shefa types crate:**
- Modify: `elohim/sdk/domains/shefa/types/src/lib.rs:181,233,790`

**TypeScript wire types (hand-written, drives codegen consumers):**
- Modify: `elohim/sdk/src/types.ts:1481,1500`
- Modify: `elohim/sdk/storage-client-ts/src/wire-types/shefa/CreateReaEconomicEventInput.ts:6` (regenerated from Rust via export_bindings — actually edit Rust, then regenerate)
- Modify: `elohim/sdk/storage-client-ts/src/wire-types/shefa/EconomicEvent.ts:6` (same)
- Modify: `elohim/sdk/src/client/zome-client.ts:946,950`

**Generated TS (must be regenerated, NOT hand-edited):**
- Regenerate via `pnpm run schema:codegen:ts`:
  - `elohim/sdk/schemas/generated-ts/inputs/create-economic-event-input.ts`
  - `elohim/sdk/schemas/generated-ts/views/economic-event-view.ts`
  - `app/elohim-app/src/app/generated/create-economic-event-input.ts`
  - `app/elohim-app/src/app/generated/economic-event-view.ts`
  - `app/elohim-library/projects/elohim-service/src/generated/create-economic-event-input.ts`
  - `app/elohim-library/projects/elohim-service/src/generated/economic-event-view.ts`
- Regenerate via `cargo test export_bindings` (in `elohim-storage`):
  - `elohim/sdk/storage-client-ts/src/generated/EconomicEvent.ts`
  - `elohim/sdk/storage-client-ts/src/generated/EconomicEventView.ts`
  - `elohim/sdk/storage-client-ts/src/generated/CreateEconomicEventInputView.ts`

**Angular consumers:**
- Modify: `app/elohim-app/src/app/elohim/interfaces/storage-writer.interface.ts:87`
- Modify: `app/elohim-app/src/app/elohim/models/economic-event.model.ts:521`
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.ts:462,665`
- Modify: `app/elohim-app/src/app/lamad/components/attention-flow/attention-flow.component.ts:48,57`
- Modify: `app/elohim-app/src/app/lamad/components/attention-flow/attention-flow.component.spec.ts:17,23,29,69,73`
- Modify: `app/elohim-app/src/app/lamad/services/aggregation-instruments.scaffold.ts:29`
- Modify: `app/elohim-app/src/app/lamad/services/signal-harness.service.ts:66`
- Modify: `app/elohim-app/src/app/shefa/interfaces/economic-event-factory.interface.ts:95` (`LamadEventType` type → `ElohimEventType`)
- Modify: `app/elohim-app/src/app/shefa/services/compute-event-api.service.ts:368,373,377,396,407` (local variable + type refs)
- Modify: `app/elohim-app/src/app/shefa/services/event.service.ts:98,137,151,172,191,211,234,269,270,287,291` (and `LamadEventTypes.X` constants — rename constant container too)
- Modify: `app/elohim-app/src/app/shefa/services/event.service.spec.ts:372,387`
- Modify: `app/elohim-app/src/app/shefa/services/insurance-mutual.service.ts` (lines 189, 285, 294, 300, 308, 316, 373, 532, 889, 995)
- Modify: `app/elohim-app/src/app/testing/mocks/test-data-factories.ts:364`

**Seed data + a2o framework:**
- Modify: `genesis/a2o/src/framework/storage-client.ts:57`
- Modify: `genesis/a2o/src/framework/testnet-manager.ts:312,334`

**Doc + skill references (lower priority — can land in a follow-up commit):**
- Modify: `.claude/skills/rea-economics/SKILL.md:140,194`
- Modify: `.claude/skills/rea-economics/references/generated-types.md:23`
- Modify: `app/elohim-app/src/app/shefa/README-EXCHANGE.md:365,374`
- Modify: `app/elohim-app/src/app/shefa/README-INSURANCE-MUTUAL.md:342,350,358`
- Modify: `elohim/holochain/dna/LINK_ARCHITECTURE.md:175`
- Modify: `genesis/a2o/research/economics.md:41,65,93,131`
- Modify: `genesis/a2o/research/execution-model.md:169`
- Modify: `genesis/a2o/research/vision.md:102`
- Modify: `genesis/docs/DEV-QUICK-START.md:146`
- Modify: `genesis/docs/PHASE-1-BUILD-SUMMARY.md:231`
- Modify: `genesis/docs/integration/exchange-integration.md:136,350,468,481`
- Modify: `genesis/docs/integration/insurance-mutual-integration-guide.md:62,132,204,337,397,459`
- Modify: `genesis/docs/superpowers/plans/2026-05-02-blob-custody-reconciliation-plan.md:1897,2470` (historical plan — DO NOT edit, leave as-is; "do not rewrite history" per vocabulary.md convention)
- Modify: `genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-delivery-master.md:102,109` (this plan's parent — update to reflect actual landed wave-0 work)

**Tests + drift-detection hook:**
- Modify: `.husky/pre-push` or equivalent hook — add a drift test: `git grep -E "lamad_event_type|lamadEventType" -- ':!*.lock' ':!**/target/**' ':!**/node_modules/**' ':!genesis/docs/superpowers/plans/2026-05-02-*'` must return empty.

---

## Stage A — Attestation dedupe (Option B path)

> If operator chose Option A, replace Stage A entirely with the Stage A-alt
> section appended below.

### Task A1 — Verify the vestigial-only claim

**Files:**
- Read only: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`
- Read only: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

- [ ] **Step 1: Confirm zero instantiation of `EntryTypes::Attestation` (the bare variant) in elohim DNA.**

Run: `grep -rn "EntryTypes::Attestation\b" /projects/elohim/elohim/holochain/dna/elohim/zomes/ | grep -v ContentAttestation`

Expected: empty result.

- [ ] **Step 2: Confirm zero call to `create_entry(&Attestation` (the bare struct) in elohim DNA.**

Run: `grep -rn "create_entry.*&Attestation\b" /projects/elohim/elohim/holochain/dna/elohim/zomes/ | grep -v ContentAttestation | grep -v RenewalAttestation`

Expected: empty result.

- [ ] **Step 3: Confirm cross-DNA bridge is the actual issuance path.**

Run: `grep -n "issue_attestation_via_imagodei" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

Expected: at least one match at line 1004 (the bridge definition) and possibly callers.

- [ ] **Step 4: Stop and abort if any of steps 1–3 contradict the vestigial claim.**

If grep returns hits, the dedupe direction must be reconsidered — surface to operator. Do NOT proceed to Task A2.

### Task A2 — Remove vestigial `Attestation` from elohim DNA integrity zome

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

- [ ] **Step 1: Find the exact line numbers.**

Run: `grep -n "^pub struct Attestation\b\|^    Attestation(Attestation)" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

Expected: two matches.

- [ ] **Step 2: Remove the `pub struct Attestation { ... }` block.**

The block spans from `pub struct Attestation {` line through the closing `}` of its impl/derive. Use `Read` to confirm boundaries, then `Edit` to remove the entire block (including the preceding `#[hdk_entry_helper]` line and any doc comments).

- [ ] **Step 3: Remove the `Attestation(Attestation)` variant from the EntryTypes enum.**

Use `Edit` to remove the single line `    Attestation(Attestation),` from the EntryTypes enum block.

- [ ] **Step 4: Remove any validate-handler arm for `EntryTypes::Attestation(...)`.**

Run: `grep -n "EntryTypes::Attestation(" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

If a `validate(...)` function matches against it, remove that arm. If no arm exists, skip.

- [ ] **Step 5: Compile-check (when EPR-Phase-4 has merged + PVC budget permits).**

Run: `cd /projects/elohim/elohim/holochain/dna/elohim && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__holochain__dna__elohim/dev cargo check --release -p content_store_integrity`

Expected: PASS. If it fails with a missing-Attestation-type error, there is a hidden reference; investigate.

- [ ] **Step 6: Commit Stage A.**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
git commit -m "$(cat <<'EOF'
refactor(elohim-dna): remove vestigial Attestation entry type

elohim DNA's content_store_integrity zome had a duplicate Attestation entry
type declaration that was never instantiated (zero grep hits for
EntryTypes::Attestation in elohim coordinator zomes). The canonical
Attestation lives in imagodei DNA; the existing cross-DNA bridge
issue_attestation_via_imagodei is the issuance path.

Tiered-quilt wave-0 substrate cleanup; spec at
genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md.
EOF
)"
```

---

## Stage A-alt — Attestation OWNERSHIP MIGRATION (Option A path)

> Only execute this if operator chose Option A. Skip if Option B was chosen.

(Plan body deferred until operator commits to Option A. Migration is
substantially larger: requires moving `issue_attestation`,
`get_agent_attestations`, `get_my_attestations`, `AttestationOutput`,
`IssueAttestationInput` from imagodei to elohim DNA; rewriting the cross-DNA
bridge into a local call; and a DHT migration tool for any existing imagodei
Attestation entries. Author this section when needed.)

---

## Stage B — `lamad_event_type` → `elohim_event_type` rename

> **No new source-of-truth schemas are introduced in this stage.** Every
> reference to a schema file below is an *existing* JSON Schema source-of-truth
> in `elohim/sdk/schemas/v1/` being mechanically renamed — the schema's
> identity, location, and ownership are unchanged; only one field name inside
> it changes. Wave 1 of the delivery master is where new schemas land
> (`QuiltCustodyClassification`, etc.); Wave 0 only renames an existing field.

**This stage waits for EPR Phase 4 to merge to `dev`.** Cannot execute against
uncommitted overlapping changes. Verify before starting:

```bash
git log origin/dev --oneline -10 | grep -i "phase 4\|epr"
git status   # must be clean
```

### Task B1 — Schema rename (drives codegen)

**Files:**
- Modify: `elohim/sdk/schemas/v1/inputs/create-economic-event-input.schema.json`
- Modify: `elohim/sdk/schemas/v1/views/economic-event-view.schema.json`

- [ ] **Step 1: Read the two schema files to confirm field shape.**

```bash
grep -n "lamadEventType" /projects/elohim/elohim/sdk/schemas/v1/inputs/create-economic-event-input.schema.json /projects/elohim/elohim/sdk/schemas/v1/views/economic-event-view.schema.json
```

Expected: 1 hit per file.

- [ ] **Step 2: Edit both schemas to rename `lamadEventType` → `elohimEventType`.**

In each file, change:
```json
"lamadEventType": { "type": "string", ... }
```
to:
```json
"elohimEventType": { "type": "string", ... }
```

- [ ] **Step 3: Commit (intermediate, codegen will follow).**

```bash
git add elohim/sdk/schemas/v1/inputs/create-economic-event-input.schema.json elohim/sdk/schemas/v1/views/economic-event-view.schema.json
git commit -m "refactor(schemas): rename lamadEventType -> elohimEventType in v1 schemas"
```

### Task B2 — Rust DNA struct field rename

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:1124`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:3960` (LinkTypes variant)
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:476`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:11799`

- [ ] **Step 1: Rename the EconomicEvent struct field.**

In `content_store_integrity/src/lib.rs`, change:
```rust
pub lamad_event_type: Option<String>, // LamadEventType for domain-specific tracking
```
to:
```rust
pub elohim_event_type: Option<String>, // ElohimEventType for domain-specific tracking
```

- [ ] **Step 2: Rename the LinkTypes variant.**

In `content_store_integrity/src/lib.rs` near line 3960, change:
```rust
EventByLamadType,        // Anchor(lamad_event_type) -> EconomicEvent
```
to:
```rust
EventByElohimType,        // Anchor(elohim_event_type) -> EconomicEvent
```

- [ ] **Step 3: Update coordinator zome handler references.**

In `content_store/src/lib.rs`, change every `lamad_event_type` field access to `elohim_event_type`. Verify with:

```bash
grep -n "lamad_event_type" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
```

Expected after edits: empty.

- [ ] **Step 4: Update any LinkTypes::EventByLamadType references in the same file.**

```bash
grep -n "EventByLamadType" /projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
```

Rename each to `EventByElohimType`.

- [ ] **Step 5: Compile-check (WAIT for PVC + EPR).**

```bash
cd /projects/elohim/elohim/holochain/dna/elohim
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__holochain__dna__elohim/dev cargo check -p content_store_integrity -p content_store
```

Expected: PASS.

### Task B3 — Diesel migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-12-000000_rename_lamad_event_type/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-12-000000_rename_lamad_event_type/down.sql`

- [ ] **Step 1: Verify timestamp non-collision.**

```bash
ls /projects/elohim/elohim/elohim-storage/migrations/ | grep "2026-05-12"
```

Expected: empty. If hit, increment to `2026-05-12-000001` to avoid the timestamp-collision class noted in `feedback_diesel_migration_timestamp_collision`.

- [ ] **Step 2: Write up.sql.**

```sql
-- Rename lamad_event_type to elohim_event_type (Wave 0 substrate cleanup;
-- lamad is the LMS pillar, elohim is the protocol core).
ALTER TABLE economic_events RENAME COLUMN lamad_event_type TO elohim_event_type;

-- Update index name accordingly.
DROP INDEX IF EXISTS idx_event_lamad_type;
CREATE INDEX idx_event_elohim_type ON economic_events(elohim_event_type);
```

- [ ] **Step 3: Write down.sql.**

```sql
-- Reverse of up.sql for migration rollback.
DROP INDEX IF EXISTS idx_event_elohim_type;
ALTER TABLE economic_events RENAME COLUMN elohim_event_type TO lamad_event_type;
CREATE INDEX idx_event_lamad_type ON economic_events(lamad_event_type);
```

- [ ] **Step 4: Verify migrations directory layout.**

```bash
ls /projects/elohim/elohim/elohim-storage/migrations/2026-05-12-000000_rename_lamad_event_type/
```

Expected: `up.sql`, `down.sql`.

### Task B4 — Rust storage models + schema module

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs:180` (column rename in table! macro)
- Modify: `elohim/elohim-storage/src/db/models.rs:349,390,745`

- [ ] **Step 1: Rename the column reference in `diesel_schema.rs`.**

Change:
```rust
lamad_event_type -> Nullable<Text>,
```
to:
```rust
elohim_event_type -> Nullable<Text>,
```

- [ ] **Step 2: Rename the struct fields in `models.rs`.**

Lines 349 and 390 both contain `lamad_event_type:` fields on the `EconomicEvent` and `NewEconomicEvent` structs. Rename to `elohim_event_type:`.

- [ ] **Step 3: Rename the `lamad_event_types` module.**

`models.rs:745` defines `pub mod lamad_event_types { ... }`. Rename the module to `elohim_event_types`. Inside the module, any `LAMAD_EVENT_TYPE_*` constants stay (string values unchanged). The module is the namespace; the constants are values.

- [ ] **Step 4: Update all `lamad_event_types::X` references to `elohim_event_types::X`.**

```bash
grep -rn "lamad_event_types::" /projects/elohim/elohim/elohim-storage/src/ | head -20
```

Each match in src/ — apply `Edit` with `replace_all=true` per file.

### Task B5 — Update storage queries + projection + views

**Files:**
- Modify: `elohim/elohim-storage/src/db/economic_events.rs` (multiple sites — see file-structure map above)
- Modify: `elohim/elohim-storage/src/rea_projection.rs:107,219`
- Modify: `elohim/elohim-storage/src/views.rs:490,524,1291,1328`
- Modify: `elohim/elohim-storage/src/http.rs:3771`
- Modify: `elohim/elohim-storage/src/p2p/blob_fetch.rs:236`
- Modify: `elohim/elohim-storage/src/reconcile/custody.rs:204`
- Modify: `elohim/elohim-storage/src/services/economic_event_service.rs:219`
- Modify: `elohim/elohim-storage/src/services/exchange_service.rs` (lines per map)
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs:452`
- Modify: `elohim/elohim-storage/src/services/resource_service.rs:773`
- Modify: `elohim/elohim-storage/tests/reciprocity_view.rs:82`

- [ ] **Step 1: Apply mechanical rename in each file.**

For each file in the list, use `Edit` with `replace_all=true` to replace `lamad_event_type` with `elohim_event_type`. The replace_all is safe here because the field name is not used in any unrelated context (no `lamad_event_type` in strings, comments are also-targetable and intentional).

- [ ] **Step 2: Verify replacement covered all sites.**

```bash
grep -rn "lamad_event_type" /projects/elohim/elohim/elohim-storage/src/ /projects/elohim/elohim/elohim-storage/tests/
```

Expected: empty (except possibly comment-only references that need separate review).

- [ ] **Step 3: Compile-check storage (WAIT for PVC budget).**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo check
```

Expected: PASS.

### Task B6 — Update shefa types crate

**Files:**
- Modify: `elohim/sdk/domains/shefa/types/src/lib.rs:181,233,790`

- [ ] **Step 1: Apply rename in shefa/types/src/lib.rs.**

Use `Edit` with `replace_all=true` to replace `lamad_event_type` with `elohim_event_type`. Same string-replacement pattern.

- [ ] **Step 2: Compile-check shefa-types.**

```bash
cd /projects/elohim/elohim/sdk/domains/shefa/types
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/crates/dev cargo check
```

Expected: PASS.

### Task B7 — Regenerate TypeScript artifacts

**Files (all regenerated, NOT hand-edited):**
- Generated: `elohim/sdk/storage-client-ts/src/generated/{EconomicEvent.ts, EconomicEventView.ts, CreateEconomicEventInputView.ts}`
- Generated: `elohim/sdk/schemas/generated-ts/inputs/create-economic-event-input.ts`
- Generated: `elohim/sdk/schemas/generated-ts/views/economic-event-view.ts`
- Generated: `app/elohim-app/src/app/generated/create-economic-event-input.ts`
- Generated: `app/elohim-app/src/app/generated/economic-event-view.ts`
- Generated: `app/elohim-library/projects/elohim-service/src/generated/create-economic-event-input.ts`
- Generated: `app/elohim-library/projects/elohim-service/src/generated/economic-event-view.ts`

- [ ] **Step 1: Run Rust-to-TS codegen.**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo test export_bindings
```

This regenerates `storage-client-ts/src/generated/*.ts`.

- [ ] **Step 2: Run JSON-Schema-to-TS codegen.**

```bash
cd /projects/elohim
pnpm run schema:codegen:ts
```

Expected: completes without errors; regenerates schema-derived TS files across the three target directories.

- [ ] **Step 3: Verify field rename propagated.**

```bash
grep -rn "lamadEventType" /projects/elohim/elohim/sdk/storage-client-ts/src/generated/ /projects/elohim/elohim/sdk/schemas/generated-ts/ /projects/elohim/app/elohim-app/src/app/generated/ /projects/elohim/app/elohim-library/projects/elohim-service/src/generated/
```

Expected: empty.

### Task B8 — Update hand-written TypeScript wire-types + SDK client

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/wire-types/shefa/CreateReaEconomicEventInput.ts`
- Modify: `elohim/sdk/storage-client-ts/src/wire-types/shefa/EconomicEvent.ts`
- Modify: `elohim/sdk/src/types.ts:1481,1500`
- Modify: `elohim/sdk/src/client/zome-client.ts:946,950`

Note: `storage-client-ts/src/wire-types/shefa/` are technically also generated from Rust via `export_bindings`. If task B7 fully covered them, this task is a verification step only. If not, hand-edit them as below.

- [ ] **Step 1: Verify wire-types content.**

```bash
grep -n "lamad_event_type" /projects/elohim/elohim/sdk/storage-client-ts/src/wire-types/shefa/CreateReaEconomicEventInput.ts /projects/elohim/elohim/sdk/storage-client-ts/src/wire-types/shefa/EconomicEvent.ts
```

If empty, skip ahead to Step 3.

- [ ] **Step 2: Apply rename to wire-types files (if Step 1 found hits).**

Use `Edit` with `replace_all=true` per file.

- [ ] **Step 3: Update SDK client (`elohim/sdk/src/`).**

In `src/types.ts:1481,1500` and `src/client/zome-client.ts:946,950` apply the rename. The zome-client has both a method name `getEventsByLamadType` and an internal `lamadEventType` argument — rename both:
- Method: `getEventsByLamadType` → `getEventsByElohimType`
- Arg: `lamadEventType` → `elohimEventType`

### Task B9 — Update Angular consumers

**Files (large list):** see file-structure map for full path-by-path.

- [ ] **Step 1: Rename `LamadEventType` TS type alias to `ElohimEventType`.**

```bash
grep -rn "LamadEventType\b" /projects/elohim/app/elohim-app/src/ /projects/elohim/app/elohim-library/projects/
```

For each hit, replace `LamadEventType` with `ElohimEventType` via `Edit` with `replace_all=true` per file.

- [ ] **Step 2: Rename `LamadEventTypes` const container.**

```bash
grep -rn "LamadEventTypes\b" /projects/elohim/app/elohim-app/src/ /projects/elohim/app/elohim-library/projects/
```

Replace `LamadEventTypes` with `ElohimEventTypes`. The string constants inside (e.g. `LamadEventTypes.PATH_STEP_COMPLETE`) keep their values; only the container name changes.

- [ ] **Step 3: Rename `lamadEventType` field references.**

```bash
grep -rln "lamadEventType\b" /projects/elohim/app/elohim-app/src/ /projects/elohim/app/elohim-library/projects/
```

For each file in the result, use `Edit` with `replace_all=true` to replace `lamadEventType` with `elohimEventType`.

- [ ] **Step 4: Run Angular typecheck (WAIT for PVC + don't compete with EPR).**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec tsc --noEmit
```

Expected: PASS.

### Task B10 — Update seed data + a2o framework

**Files:**
- Modify: `genesis/a2o/src/framework/storage-client.ts:57`
- Modify: `genesis/a2o/src/framework/testnet-manager.ts:312,334`

- [ ] **Step 1: Rename in each file via `Edit` with `replace_all=true`.**

```bash
grep -rln "lamadEventType\|lamad_event_type" /projects/elohim/genesis/a2o/src/
```

Apply rename per file.

### Task B11 — Update doc references (lowest priority — can land in follow-up commit)

**Files:** see file-structure map "Doc + skill references" section.

- [ ] **Step 1: Apply rename across all doc files EXCEPT historical plans.**

```bash
git grep -lE "lamadEventType|lamad_event_type" -- ':!genesis/docs/superpowers/plans/2026-05-02-*' ':!*.lock' ':!**/target/**' ':!**/node_modules/**' \
  | grep -E '\.md$'
```

For each markdown file in the result, use `Edit` with `replace_all=true` for each of the two patterns.

- [ ] **Step 2: Manually verify the historical-plan exemption.**

The `2026-05-02-blob-custody-reconciliation-plan.md` references stay (historical record per vocabulary.md convention: "do not rewrite history"). Confirm:

```bash
grep -n "lamad_event_type" /projects/elohim/genesis/docs/superpowers/plans/2026-05-02-blob-custody-reconciliation-plan.md
```

Expected: 2 hits (lines 1897 and 2470). Leave untouched.

- [ ] **Step 3: Update the parent delivery master.**

In `genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-delivery-master.md:102,109` the master mentions the rename plan abstractly. Update the language from future-tense ("will rename") to past-tense ("renamed in wave 0; see wave-0 plan").

### Task B12 — Add drift-detection pre-push hook

**Files:**
- Modify or create: `.husky/pre-push` (or appropriate hook location)

- [ ] **Step 1: Find the existing pre-push hook.**

```bash
ls -la /projects/elohim/.husky/ 2>/dev/null
```

If `pre-push` exists, append the drift check. If not, create one.

- [ ] **Step 2: Add the drift check.**

Append to `.husky/pre-push`:

```bash
# Wave 0 drift check — lamad_event_type rename guard
DRIFT=$(git grep -E "lamad_event_type|lamadEventType" -- \
  ':!*.lock' ':!**/target/**' ':!**/node_modules/**' \
  ':!genesis/docs/superpowers/plans/2026-05-02-*' 2>/dev/null || true)

if [ -n "$DRIFT" ]; then
  echo "❌ lamad_event_type drift detected (Wave 0 substrate cleanup guard):"
  echo "$DRIFT"
  echo ""
  echo "These references should use elohim_event_type / elohimEventType."
  echo "If a reference is intentional (historical record), add the file to the"
  echo "exemption list in the drift check above."
  exit 1
fi
```

- [ ] **Step 3: Verify hook runs.**

```bash
cd /projects/elohim
chmod +x .husky/pre-push
bash .husky/pre-push
```

Expected: exits 0 (no drift detected, post-rename).

### Task B13 — Final integration verification

- [ ] **Step 1: Confirm `git grep` is clean.**

```bash
git grep -E "lamad_event_type|lamadEventType" -- \
  ':!*.lock' ':!**/target/**' ':!**/node_modules/**' \
  ':!genesis/docs/superpowers/plans/2026-05-02-*'
```

Expected: empty.

- [ ] **Step 2: Run Bands 1+2 of existing test suite serially (NOT in parallel with EPR agent).**

PVC-sensitive. Coordinate with operator (the user) before running. Suggested run order, one at a time:

```bash
# Storage unit tests
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo test --lib --bins

# Angular tests
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts

# Sweettest (slowest — last)
cd /projects/elohim/elohim/holochain/dna
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__holochain__tests__sweettest/dev cargo test --test-threads=1
```

Expected: all pass.

- [ ] **Step 3: Commit the full Stage B as one PR-grade integrated commit.**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor(elohim): rename lamad_event_type -> elohim_event_type across stack

lamad_event_type was legacy naming drift from when "lamad" still meant
protocol-core. lamad is the LMS pillar; elohim is the protocol core. The
field is renamed everywhere it appears:

- JSON schemas (elohim/sdk/schemas/v1/EconomicEvent.schema.json + input)
- Rust DNA: elohim/holochain/dna/elohim/zomes/content_store_integrity/
  + content_store/ (struct field + LinkTypes::EventByLamadType variant)
- Rust storage: elohim-storage/src/db/{diesel_schema.rs, models.rs,
  economic_events.rs}, rea_projection.rs, views.rs, http.rs, services/*,
  reconcile/custody.rs, p2p/blob_fetch.rs
- Diesel migration: 2026-05-12-000000_rename_lamad_event_type/{up,down}.sql
- TypeScript: generated via schema:codegen:ts + export_bindings;
  hand-written wire-types in elohim/sdk/storage-client-ts/src/wire-types/;
  elohim/sdk/src/{types,client/zome-client}.ts
- Angular: app/elohim-app/src/app/{elohim,lamad,shefa,testing}/* + the
  LamadEventType -> ElohimEventType type alias and LamadEventTypes ->
  ElohimEventTypes container rename
- a2o framework + research docs + integration guides
- Historical plans intentionally NOT modified (2026-05-02-blob-custody-
  reconciliation-plan.md) per vocabulary.md "do not rewrite history" rule

Pre-push hook drift check added in .husky/pre-push to prevent reintroduction.

Tiered-quilt Wave 0 substrate cleanup; spec at
genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Acceptance gate (Wave 0 → Wave 1)

Before dispatching the Wave 1 plan-authoring session:

- [ ] `git grep -E "lamad_event_type|lamadEventType" -- ':!*.lock' ':!**/target/**' ':!**/node_modules/**' ':!genesis/docs/superpowers/plans/2026-05-02-*'` returns empty
- [ ] imagodei DNA's `Attestation` entry type is still authoritative (Option B path) OR has been migrated to elohim DNA (Option A path)
- [ ] All Bands 1+2 tests pass on the renamed substrate
- [ ] Pre-push hook drift check is wired and verified
- [ ] No regression in EPR Phase 4 work (verify their tests still pass after rebase onto Wave 0)
- [ ] Memory anchor `project_elohim_event_type_field_rename.md` created
- [ ] Memory anchor `project_attestation_dedupe_elohim_dna_canonical.md` created (with reality-corrected direction)

When the gate is green, dispatch Wave 1 plan-authoring with the locked
decisions from delivery master §1.

---

## Risk + rollback

**Wave 0 has HIGH risk** per delivery master §6:

- Wide blast radius (83 files)
- Touches schemas / generated artifacts / Rust / Angular
- DNA migration is field rename only (no entry shape change, no historic-entry migration), but DNA recompile + redeploy required
- Coordinated with EPR Phase 4 work — timing-sensitive

**Rollback procedure** if Wave 0 must be reverted:

1. `git revert <wave-0-commits>` — clean revert; the rename is mechanical
2. `cd elohim/elohim-storage && diesel migration revert` — undoes the column rename
3. Regenerate TS codegen: `pnpm run schema:codegen:ts` + `cargo test export_bindings`
4. Confirm via `git grep -E "elohim_event_type" -- ':!*.lock' ':!**/target/**'` returns only the rollback-safe historical references

Rollback timing: Same-day if pre-push drift hook catches issues; next-day if
sweettest reveals deeper coupling. Wave 1 cannot proceed until rollback or
forward-fix lands.

---

## Cross-references

- Spec: `genesis/docs/superpowers/specs/2026-05-11-tiered-quilt-stewardship-design.md`
- Delivery master: `genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-delivery-master.md`
- Diesel migration timestamp-collision memory: `feedback_diesel_migration_timestamp_collision`
- Multi-agent PVC pacing memory: `feedback_multi_agent_pvc_pacing`
- "Do not rewrite history" rule: `genesis/graphos/vocabulary.md`
