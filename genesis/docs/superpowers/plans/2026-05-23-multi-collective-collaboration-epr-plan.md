---
id: multi-collective-collaboration-epr-plan
status: Draft
cites:
  - ../../content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md   # the design spec this plan implements
---

# Multi-Collective Collaboration EPR — M1 Implementation Plan (T0 Collab End-to-End)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land M1 of the multi-collective collaboration spec — a working T0 Collab end-to-end. Pre-declared root only: a steward authors an Agreement-EPR naming multiple Collectives as participants; each Collective's stewards counter-attest; the substrate atomically instantiates a Collab-Qahal (recursive Collective entry) with polymorphic Memberships pointing to the participating Collectives; Form A share allocation evaluates on EconomicEvents emitted under the Collab's scope; commons-pool tribute accumulates; clean exit works; VF projection surfaces the Collab as an Organization with Organization-typed member relationships. Test classes 1, 2, 4 green.

**Architecture:**
- **DNA layer (imagodei zome)** — adds three new entry types: `Collective`, `Membership` (with polymorphic `member_kind`), `CollabAgreement`. Integrity validation is pure-data + `must_get_*` only (no link traversal per `project_hdi_no_get_links_in_validators`). Coordinator functions own atomic multi-step flows (Collab-Qahal instantiation, counter-attestation, withdrawal).
- **Schema layer** — JSON Schema sources of truth in `elohim/sdk/schemas/v1/`; Rust types in `elohim/elohim-views` with `#[derive(TS)]` for codegen; TypeScript types regenerated via `cargo test export_bindings`.
- **Storage layer** — HTTP routes in elohim-storage that project DNA state to clients; services in `elohim/elohim-storage/src/services/` handle multi-step orchestration (cross-zome calls, share-routing evaluation, projection caching).
- **Tests** — sweettest cross-DNA flows in `elohim/holochain/tests/sweettest/`; unit tests for share-routing in `elohim/elohim-storage/src/services/share_routing.rs`; schema contract tests; substrate-deterministic capture-attempt scenarios.

**Tech Stack:** Rust (HDK 0.6 for DNAs; tokio/hyper for storage HTTP; diesel for storage projection cache); JSON Schema + ts-rs codegen for the Rust→TS boundary; sweettest for cross-DNA integration; vitest for any TS-side tests.

**Substrate-currency references (read before starting):**
- `genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md` — this plan implements §2, §5 (pre-declared root only), §6.1 Form A, §6.3 commons-pool tribute, §6.4 clean-exit, §7 VF projection at T0 of that spec
- `genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md` — Collective + Membership shapes; this plan generalizes them to polymorphic membership on first landing (no separate migration; Collective/Membership don't yet exist as code)
- `genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md` — VF projection patterns and extension-field conventions
- `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:887-960` — current `EntryTypes` enum and `LinkTypes` enum (where additions land)
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:512` — example coordinator function (`create_human_relationship`) to mirror style/error handling

**Out of scope for M1 (named, not implemented here):**
- Emergent formation root (collab-candidate Observation-EPR) — **M2**
- Form B affinity-derived share allocation — **M2**
- T0→T1 graduation with commons-elohim counter-attestation — **M3**
- T2+ chain-anchoring (contract-stub for tier-graduation seam tests is M4)
- Repair-exit flow — **M3** (clean-exit only for M1)
- hREA bridge full integration — depends on Wave 3 M3 landing; this plan adds the Collective→Organization mapping shape but full bridge tests live in Wave 3
- Cross-Collab membership — **M5**
- Care-history-baseline pattern recognition — gated on value-scanner online
- Angular UI surfaces — handled in a downstream UI plan once substrate is green

---

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs` | CREATE | Collective + Membership + CollabAgreement entry structs; MemberKind, MembershipRole enums; pure-data validation helpers |
| `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` | MODIFY | Add 3 entries to `EntryTypes` enum (line 887); add link types `MemberOf`, `HasMember`, `HasMembership`, `StewardOf`, `CharterAnchor`, `AgreementOnCollab`; wire `validate_create_*` dispatch for new entries |
| `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs` | CREATE | Coordinator functions: `create_collective`, `create_collab_agreement`, `attest_collab_agreement`, `instantiate_collab_qahal`, `withdraw_membership_clean`; atomic multi-step orchestration |
| `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` | MODIFY | Re-export `qahal_coordinator` module's `#[hdk_extern]` functions |
| `elohim/sdk/schemas/v1/inputs/create-collective-input.schema.json` | CREATE | **Cat C — HTTP wire-shape input** projecting onto the Category-A `Collective` DHT entry; not a source of truth. Input schema for `POST /api/v1/collective`. |
| `elohim/sdk/schemas/v1/inputs/create-collab-agreement-input.schema.json` | CREATE | **Cat C — HTTP wire-shape input** projecting onto the Category-A `CollabAgreement` DHT entry; not a source of truth. Input schema for `POST /api/v1/collab/agreement`. |
| `elohim/sdk/schemas/v1/inputs/attest-collab-agreement-input.schema.json` | CREATE | **Cat C — HTTP wire-shape input** projecting onto the Category-A2 `AgreementOnCollab` link (counter-attestation); not a source of truth. Input schema for `POST /api/v1/collab/agreement/{cid}/attest`. |
| `elohim/sdk/schemas/v1/inputs/withdraw-membership-input.schema.json` | CREATE | **Cat C — HTTP wire-shape input** projecting onto an update of the Category-A `Membership` DHT entry (sets `withdrawn_at_block_height`); not a source of truth. Input schema for `POST /api/v1/collab/{cid}/withdraw`. |
| `elohim/sdk/schemas/v1/objects/share-allocation.schema.json` | CREATE | **Cat C — embedded wire-shape sub-object** of the Category-A `CollabAgreement` DHT entry's `share_allocation_json` field; not a source of truth. Form A only for M1; reserves `form: "affinity_derived"` discriminator for M2. |
| `elohim/sdk/schemas/v1/views/collective-view.schema.json` | CREATE | **Cat C — HTTP wire-shape projection** of the Category-A `Collective` DHT entry; reconstructible at any time; not a source of truth. Wire shape for `GET /api/v1/collective/:cid`. |
| `elohim/sdk/schemas/v1/views/collab-qahal-view.schema.json` | CREATE | **Cat C — HTTP wire-shape projection** of the Category-A `Collective` (when anchor_agreement_cid is set) + traversal over Category-A `Membership` entries; reconstructible; not a source of truth. Wire shape for `GET /api/v1/collab/:cid`. |
| `elohim/sdk/schemas/v1/views/membership-view.schema.json` | CREATE | **Cat C — HTTP wire-shape projection** of the Category-A `Membership` DHT entry (polymorphic via `memberKind`); not a source of truth. Wire shape for membership list endpoints. |
| `elohim/sdk/schemas/v1/views/collab-agreement-view.schema.json` | CREATE | **Cat C — HTTP wire-shape projection** of the Category-A `CollabAgreement` DHT entry plus derived counter-attestation status (computed from Category-A2 `AgreementOnCollab` links); not a source of truth. Wire shape including `shareAllocation`, `commonsPoolTribute`, `participants[]`, counter-attestation status. |
| `elohim/sdk/schemas/v1/enums/member-kind.schema.json` | CREATE | **Cat C — protocol vocabulary** enum mirrored from the Category-A `MemberKind` field on the `Membership` DHT entry; not a source of truth. `Person` / `Collective` / `ElohimAgent`. |
| `elohim/sdk/schemas/v1/enums/elohim-tier.schema.json` | CREATE | **Cat C — protocol vocabulary** enum mirrored from the Category-A `initial_tier` field on `CollabAgreement` + derived tier-evaluation function output; not a source of truth. `T0` / `T1` / `T2` / `T3` (only T0 reachable in M1). |
| `elohim/sdk/schemas/v1/enums/share-allocation-form.schema.json` | CREATE | **Cat C — protocol vocabulary** enum mirrored from the Category-A `share_allocation_json.form` field on `CollabAgreement`; not a source of truth. `Declared` / `AffinityDerived` (M1 only emits `Declared`). |
| `elohim/sdk/schemas/scripts/codegen-ts.mjs` | MODIFY | Add new view schemas to `INTERFACE_FILES` |
| `elohim/elohim-views/src/qahal.rs` | CREATE | Rust structs with `#[derive(TS, Serialize, Deserialize)]`: `CollectiveView`, `CollabQahalView`, `MembershipView`, `CollabAgreementView`, `ShareAllocation`, `MemberKind`, `ElohimTier`, `ShareAllocationForm` |
| `elohim/elohim-views/src/lib.rs` | MODIFY | `pub mod qahal; pub use qahal::*;` |
| `elohim/sdk/storage-client-ts/src/generated/` | REGENERATE | Via `cargo test export_bindings` — do NOT hand-edit |
| `elohim/elohim-storage/src/services/qahal_service.rs` | CREATE | Service layer: `create_collective`, `create_collab_agreement`, `attest_agreement`, `instantiate_qahal_atomically`, `withdraw_membership_clean`, `fetch_collab_view` |
| `elohim/elohim-storage/src/services/share_routing.rs` | CREATE | Pure-function share-routing evaluator (Form A only); hook into EconomicEvent emission path |
| `elohim/elohim-storage/src/services/mod.rs` | MODIFY | `pub mod qahal_service; pub mod share_routing;` |
| `elohim/elohim-storage/src/http.rs` | MODIFY | Register routes: `POST /api/v1/collective`, `POST /api/v1/collab/agreement`, `POST /api/v1/collab/agreement/:cid/attest`, `GET /api/v1/collective/:cid`, `GET /api/v1/collab/:cid`, `POST /api/v1/collab/:cid/withdraw`; each route declares its auth requirement |
| `elohim/elohim-storage/tests/schema_contract.rs` | MODIFY | Add contract tests for the 4 new view types and 4 new input types |
| `elohim/elohim-storage/tests/qahal_http_contract.rs` | CREATE | HTTP contract tests for the new routes (auth + happy path + refusal cases) |
| `elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs` | CREATE | Sweettest cross-DNA flows: pre-declared root creation, counter-attestation, clean withdrawal, refusal cases (zero tribute, sock-puppet, reach-inflation) |
| `elohim/elohim-storage/src/services/share_routing_tests.rs` | CREATE | Unit tests for share-routing function (Form A): proportional distribution, sum-to-one validation, tribute-zero refusal, exit-block-height honoring |
| `elohim/elohim-storage/src/views.rs` | MODIFY | Re-export new view types from `elohim_views::qahal::*` |
| `elohim/sdk/storage-client-ts/src/api/qahal.ts` | CREATE | SDK methods: `createCollective()`, `createCollabAgreement()`, `attestCollabAgreement()`, `getCollab()`, `withdrawFromCollab()` |

---

## Source-of-Truth (P2P Design Gate compliance)

This plan adds storage-layer artifacts (HTTP routes, views, services) that **project DNA-notarized state**. None of the new files introduce a new source of truth — all storage-layer additions are projections of DHT entries that the integrity zome notarizes. Per the spec's design-gate answers (see `2026-05-23-multi-collective-collaboration-epr-design.md` §2):

| New substrate entity | Category | Source of truth | Notarization mechanism |
|---|---|---|---|
| `Collective` (DHT entry, recursive — works as both first-order Collective AND as Collab-Qahal) | **A — notarized** | imagodei DNA, `Collective` entry type | DHT validation (integrity zome pure-data checks + must_get_action on founder signature); content-addressed CID derived from `{founder_agent_cid, charter, created_at_block_height, salt}` |
| `Membership` (polymorphic via `member_kind`) | **A — notarized** | imagodei DNA, `Membership` entry type | DHT validation (integrity zome pure-data checks); CID derived from `{member_cid, collective_cid, role, joined_at_block_height, sponsor_cid?}` |
| `CollabAgreement` | **A — notarized** | imagodei DNA, `CollabAgreement` entry type | DHT validation (integrity zome pure-data checks + must_get on participant Collective entries via coordinator); CID derived from `{authored_by_agent_cid, participants, scope, share_allocation_json, created_at_block_height, salt}` |
| Counter-attestation links (`AgreementOnCollab` link kind) | **A2 — derived via link** | imagodei DNA, link table | Link validation (integrity zome verifies caller is current Steward of attesting Collective via coordinator-side require_caller_is_steward_of) |
| `CollectiveView`, `MembershipView`, `CollabAgreementView`, `CollabQahalView` | **C — operational projection** | elohim-storage projection over DHT entries | Reconstructible from DHT state at any time; not authoritative; HTTP response shapes only |
| `RoutedAmount` (share-routing evaluator output) | **C — operational derivation** | Pure function `evaluate_share_routing(allocation, value, active_set)` | Reconstructible from CollabAgreement + active membership set; emitted as Settlement EconomicEvents which are themselves Category A (notarized via the existing EconomicEvent EPR substrate) |
| Commons-pool balance | **C — operational aggregation** | Sum over Settlement EconomicEvents with beneficiary `commons-pool` scoped to a Collab-Qahal | Derived view; the underlying EconomicEvents are notarized |

**Identity scheme — CID-derived, no slugs.** All new entities use content-address CIDs with the existing `collective:`, `agreement:`, `membership:` prefix conventions (mirrors the 2026-05-19 spec). No human-readable slugs are introduced.

**Coordinator function map** (load-bearing authority gates that require link traversal — must run in coordinator, not integrity):

| Operation | Authority gate | Coordinator function |
|---|---|---|
| Create Collective | None (anyone may found) | `qahal_coordinator::create_collective` (Task 4) |
| Create CollabAgreement | None at creation; counter-attestation required for instantiation | `qahal_coordinator::create_collab_agreement` (Task 5) |
| Counter-attest Agreement | Caller must be current Steward of attesting Collective | `qahal_coordinator::attest_collab_agreement` (Task 5) — calls `require_caller_is_steward_of` which traverses HasMembership links |
| Withdraw Person Membership | Caller must equal `member_cid` | `qahal_coordinator::withdraw_membership_clean` (Task 6) |
| Withdraw Collective Membership | Caller must be current Steward of withdrawing Collective | Same — branches on `member_kind` |

**Integrity validators are pure-data + must_get_* only** (per `project_hdi_no_get_links_in_validators` memory). Link traversal stays coordinator-side. The pure-data validators in `qahal.rs` (Task 1) enforce: charter size, display_name length, salt format, non-zero tribute, T0-only initial tier.

**No new EPR kinds introduced.** The plan reuses `EprKind::Commitment` (for Agreements when projected through Wave 3 hREA bridge), `EprKind::Attestation` (for counter-attestations), `EprKind::EconomicEvent` (for share-routing Settlement events). The DHT vocabulary at `elohim/sdk/schemas/v1/enums/epr-kind.schema.json` is unchanged.

---

## Phase A — DNA substrate (entries + integrity validation)

### Task 1: Add MemberKind + MembershipRole enums

**Files:**
- Create: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs`

- [ ] **Step 1: Create `qahal.rs` with enums and stub entry structs**

Write to `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs`:

```rust
//! Qahal substrate entries: Collective, Membership (polymorphic), CollabAgreement.
//!
//! Per spec: genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md
//! Pure-data validation only — link traversal happens in the coordinator zome.

use hdi::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    Person,
    Collective,
    ElohimAgent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipRole {
    Steward,
    Contributor,
    Observer,
}

#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collective {
    pub founder_agent_cid: String,
    pub charter: String,
    pub display_name: String,
    pub created_at_block_height: u64,
    pub salt: String,
    /// When this Collective is a Collab-Qahal instantiated from a CollabAgreement,
    /// references the agreement's ActionHash. None for first-order Collectives.
    pub anchor_agreement_cid: Option<String>,
}

#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    pub member_cid: String,
    pub member_kind: MemberKind,
    pub collective_cid: String,
    pub role: MembershipRole,
    /// Set when role == Steward and pending counter-attestation. Cleared once attested.
    pub sponsor_cid: Option<String>,
    pub joined_at_block_height: u64,
    /// Set when the Membership has been cleanly withdrawn. Future EconomicEvents
    /// emitted at block heights >= this value do not accrue to this member.
    pub withdrawn_at_block_height: Option<u64>,
}

#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollabAgreement {
    pub authored_by_agent_cid: String,
    pub participants: Vec<String>, // Collective CIDs
    pub scope: String,
    pub share_allocation_json: String, // serialized ShareAllocation; see Task 7
    pub commons_pool_tribute: f64,     // 0.0 < value <= 1.0
    pub governance_terms_json: String, // serialized GovernanceTerms; see Task 7
    pub anchor_collective_cid: Option<String>, // populated once Collab-Qahal instantiated
    pub initial_tier: String, // "T0" only for M1; "T1" rejected (M3) until commons-elohim path lands
    pub created_at_block_height: u64,
    pub salt: String,
}

/// Pure-data validation for Collective.
pub fn validate_collective_pure(c: &Collective) -> ExternResult<ValidateCallbackResult> {
    if c.charter.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Collective.charter must be non-empty".into()));
    }
    if c.charter.len() > 16 * 1024 {
        return Ok(ValidateCallbackResult::Invalid("Collective.charter exceeds 16 KiB".into()));
    }
    if c.display_name.is_empty() || c.display_name.len() > 256 {
        return Ok(ValidateCallbackResult::Invalid("Collective.display_name must be 1..=256 chars".into()));
    }
    if c.salt.len() != 32 || !c.salt.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(ValidateCallbackResult::Invalid("Collective.salt must be 32 hex chars".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Pure-data validation for Membership.
pub fn validate_membership_pure(m: &Membership) -> ExternResult<ValidateCallbackResult> {
    if m.member_cid.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Membership.member_cid must be non-empty".into()));
    }
    if m.collective_cid.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Membership.collective_cid must be non-empty".into()));
    }
    if matches!(m.role, MembershipRole::Steward) && m.sponsor_cid.is_none() {
        // Founder bypass: at Collective creation the founder's Steward Membership is created
        // by the coordinator atomically with no sponsor. The coordinator sets a synthetic
        // sponsor_cid = "founder" to satisfy this gate. See coordinator (Task 4).
        return Ok(ValidateCallbackResult::Invalid("Steward role requires sponsor_cid".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Pure-data validation for CollabAgreement.
pub fn validate_collab_agreement_pure(a: &CollabAgreement) -> ExternResult<ValidateCallbackResult> {
    if a.participants.len() < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "CollabAgreement requires >= 2 participating Collectives".into(),
        ));
    }
    if a.commons_pool_tribute <= 0.0 || a.commons_pool_tribute > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "CollabAgreement.commons_pool_tribute must be in (0.0, 1.0]".into(),
        ));
    }
    if a.scope.is_empty() || a.scope.len() > 16 * 1024 {
        return Ok(ValidateCallbackResult::Invalid("CollabAgreement.scope must be 1..=16 KiB".into()));
    }
    if a.initial_tier != "T0" {
        return Ok(ValidateCallbackResult::Invalid(
            "M1 only supports initial_tier=\"T0\"; T1+ requires commons-elohim path (M3)".into(),
        ));
    }
    if a.salt.len() != 32 || !a.salt.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(ValidateCallbackResult::Invalid("CollabAgreement.salt must be 32 hex chars".into()));
    }
    // share_allocation_json + governance_terms_json structural validation lives in the
    // coordinator (parsing arbitrary JSON inside the integrity validator is avoided —
    // pure-data field shape checks only here). The coordinator performs structural
    // checks before commit_entry. The DHT entry remains the source of truth either way.
    Ok(ValidateCallbackResult::Valid)
}
```

- [ ] **Step 2: Run cargo check**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei/zomes/imagodei_integrity
cargo check --target wasm32-unknown-unknown
```

Expected: PASS with no errors (warnings about unused functions are fine — they're called from `lib.rs` after Task 2).

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs
git commit -m "feat(qahal-integrity): add Collective, Membership, CollabAgreement entry structs

Per multi-collective-collaboration-epr spec §2 + §5.1. Pure-data validation
only; link traversal lives in the coordinator. M1 restricts initial_tier
to T0; T1+ rejected pending commons-elohim path (M3).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Wire entries + link types into integrity zome

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` (lines 887-960 region)

- [ ] **Step 1: Add `mod qahal;` declaration near the top of lib.rs**

Open `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs`. Near the existing top-of-file `mod` declarations, add:

```rust
pub mod qahal;
pub use qahal::{Collective, Membership, MembershipRole, MemberKind, CollabAgreement};
```

- [ ] **Step 2: Extend `EntryTypes` enum (line ~887)**

Find the `#[hdk_entry_defs]` `EntryTypes` enum and add three variants after the existing `HumanRelationship` variant:

```rust
#[hdk_entry_defs]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Human(Human),
    Agent(Agent),
    HumanRelationship(HumanRelationship),
    // ... existing variants stay ...
    Collective(Collective),
    Membership(Membership),
    CollabAgreement(CollabAgreement),
}
```

- [ ] **Step 3: Extend `LinkTypes` enum**

Find the `LinkTypes` enum (~line 960) and add the new link kinds:

```rust
#[hdk_link_types]
pub enum LinkTypes {
    // ... existing variants stay ...

    // Qahal substrate links
    MemberOf,              // Person/Collective/ElohimAgent CID -> Collective
    HasMember,             // Collective -> Member CID (any kind)
    HasMembership,         // Collective -> Membership entry (full metadata)
    StewardOf,             // Person/Collective CID -> Collective (role-filtered)
    CharterAnchor,         // Anchor("collective:<cid>") -> Collective
    AgreementOnCollab,     // CollabAgreement -> Collective (the instantiated Collab-Qahal)
    MembershipForAgreement,// CollabAgreement -> Membership entries
}
```

- [ ] **Step 4: Add validation dispatch (around line 1085)**

In the `validate(op: Op)` function's main `match`, add three new arms inside the `EntryTypes::*` match:

```rust
EntryTypes::Collective(c) => qahal::validate_collective_pure(&c),
EntryTypes::Membership(m) => qahal::validate_membership_pure(&m),
EntryTypes::CollabAgreement(a) => qahal::validate_collab_agreement_pure(&a),
```

- [ ] **Step 5: Run cargo check**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei/zomes/imagodei_integrity
cargo check --target wasm32-unknown-unknown
```

Expected: PASS.

- [ ] **Step 6: Verify the DNA still packs**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei
hc dna pack . 2>&1 | tail -10
```

Expected: `Wrote bundle ./imagodei.dna` (or similar success).

- [ ] **Step 7: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
git commit -m "feat(qahal-integrity): wire Collective/Membership/CollabAgreement entries

Adds 3 entry variants + 7 link types to imagodei integrity zome. Validation
dispatch routes through qahal module's pure-data helpers. DNA continues to
pack cleanly. Headroom remains comfortable (~85 slots available).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Add coordinator-side type re-exports

**Files:**
- Create: `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs` (skeleton)
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`

- [ ] **Step 1: Create the coordinator module skeleton**

Write to `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs`:

```rust
//! Qahal coordinator: atomic multi-step orchestration for Collective + Collab flows.
//!
//! Per spec §2 + §5.1. Coordinator-only authority gates (link traversal happens here,
//! integrity-zome validators remain pure-data).

use hdk::prelude::*;
use imagodei_integrity::qahal::{
    Collective, Membership, MembershipRole, MemberKind, CollabAgreement,
};
use imagodei_integrity::{EntryTypes, LinkTypes};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCollectiveInput {
    pub charter: String,
    pub display_name: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCollabAgreementInput {
    pub participants: Vec<String>,
    pub scope: String,
    pub share_allocation_json: String,
    pub commons_pool_tribute: f64,
    pub governance_terms_json: String,
    pub initial_tier: String,
    pub display_name_for_qahal: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttestCollabAgreementInput {
    pub agreement_action_hash: ActionHash,
    pub attesting_collective_cid: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawMembershipInput {
    pub membership_action_hash: ActionHash,
    pub collab_qahal_cid: String,
}

// Coordinator functions land in Tasks 4–6.
```

- [ ] **Step 2: Re-export coordinator module from lib.rs**

In `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`, near the top, add:

```rust
pub mod qahal_coordinator;
```

- [ ] **Step 3: Run cargo check**

```bash
cd /projects/elohim/elohim/holochain/dna/imagodei/zomes/imagodei
cargo check --target wasm32-unknown-unknown
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs \
        elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
git commit -m "feat(qahal-coordinator): scaffold qahal_coordinator module + input types

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Implement `create_collective` coordinator function

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs`
- Test: `elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs` (test file created in Task 19; the test for THIS task lives in that file but is added now as a failing case)

- [ ] **Step 1: Write the failing sweettest**

Create `elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs` if it doesn't yet exist; add the first test:

```rust
//! Sweettest cross-DNA flows for T0 Collab end-to-end.
//! Per plan: 2026-05-23-multi-collective-collaboration-epr-plan.md

use hdk::prelude::*;
use holochain::sweettest::*;
use imagodei::qahal_coordinator::{CreateCollectiveInput, CreateCollabAgreementInput};

const IMAGODEI_DNA: &str = "../../holochain/dna/imagodei/imagodei.dna";

#[tokio::test(flavor = "multi_thread")]
async fn create_collective_atomic_founder_membership() {
    let mut conductor = SweetConductor::from_standard_config().await;
    let dna = SweetDnaFile::from_bundle(IMAGODEI_DNA.as_ref()).await.unwrap();
    let app = conductor.setup_app("imagodei", &[dna]).await.unwrap();
    let cell = app.cells()[0].clone();

    let result: ActionHash = conductor
        .call(&cell.zome("imagodei"), "create_collective", CreateCollectiveInput {
            charter: "We steward the watershed.".into(),
            display_name: "Watershed Stewards".into(),
            salt: "0123456789abcdef0123456789abcdef".into(),
        })
        .await;

    // Read back: there should be a Collective entry AND a founder Steward Membership.
    let collective: Option<Record> = conductor
        .call(&cell.zome("imagodei"), "get_collective_by_action", result.clone())
        .await;
    assert!(collective.is_some(), "Collective record exists");

    let memberships: Vec<Record> = conductor
        .call(&cell.zome("imagodei"), "list_memberships_for_collective", result)
        .await;
    assert_eq!(memberships.len(), 1, "founder Steward Membership created atomically");
}
```

- [ ] **Step 2: Run the test — confirm failure**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest
RUST_LOG=warn cargo test --test qahal_collab_t0_test create_collective_atomic_founder_membership -- --nocapture 2>&1 | tail -20
```

Expected: FAIL with `create_collective` not defined or similar.

- [ ] **Step 3: Implement `create_collective`**

Append to `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs`:

```rust
#[hdk_extern]
pub fn create_collective(input: CreateCollectiveInput) -> ExternResult<ActionHash> {
    let block_height = current_block_height()?;
    let founder_agent_pubkey = agent_info()?.agent_initial_pubkey;
    let founder_cid = encode_agent_cid(&founder_agent_pubkey)?;

    let collective = Collective {
        founder_agent_cid: founder_cid.clone(),
        charter: input.charter,
        display_name: input.display_name,
        created_at_block_height: block_height,
        salt: input.salt,
        anchor_agreement_cid: None,
    };

    let collective_hash = create_entry(EntryTypes::Collective(collective.clone()))?;
    let collective_cid = action_hash_to_cid(&collective_hash);

    // Atomically create founder Steward Membership.
    let founder_membership = Membership {
        member_cid: founder_cid,
        member_kind: MemberKind::Person,
        collective_cid: collective_cid.clone(),
        role: MembershipRole::Steward,
        sponsor_cid: Some("founder".into()), // synthetic to satisfy integrity gate
        joined_at_block_height: block_height,
        withdrawn_at_block_height: None,
    };
    let membership_hash = create_entry(EntryTypes::Membership(founder_membership))?;

    // Anchor + bidirectional discovery links.
    let anchor = anchor("collective", &collective_cid)?;
    create_link(anchor, collective_hash.clone(), LinkTypes::CharterAnchor, ())?;
    create_link(collective_hash.clone(), membership_hash, LinkTypes::HasMembership, ())?;

    Ok(collective_hash)
}

#[hdk_extern]
pub fn get_collective_by_action(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

#[hdk_extern]
pub fn list_memberships_for_collective(collective_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(collective_hash, LinkTypes::HasMembership, None)?;
    let mut out = Vec::new();
    for link in links {
        if let Some(record) = get(ActionHash::from(link.target), GetOptions::default())? {
            out.push(record);
        }
    }
    Ok(out)
}

// Helper: synthetic CID encoding for agent pubkeys. Mirrors existing imagodei pattern.
fn encode_agent_cid(pubkey: &AgentPubKey) -> ExternResult<String> {
    Ok(format!("agent:{}", pubkey))
}

// Helper: action hash -> CID string for storage interchange.
fn action_hash_to_cid(hash: &ActionHash) -> String {
    format!("collective:{}", hash)
}

fn current_block_height() -> ExternResult<u64> {
    Ok(sys_time()?.as_micros() as u64)
}
```

- [ ] **Step 4: Run the test — confirm pass**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest
cargo test --test qahal_collab_t0_test create_collective_atomic_founder_membership -- --nocapture 2>&1 | tail -20
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs \
        elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs
git commit -m "feat(qahal-coordinator): create_collective atomically creates founder Steward

Pre-declared Collective creation per spec §2. Founder agent's Steward
Membership is created in the same coordinator call to avoid the chicken-
and-egg of \"who attests the first steward?\" Sweettest verifies the
atomic invariant.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Implement `create_collab_agreement` and `attest_collab_agreement`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs`
- Test: extend `elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs`

- [ ] **Step 1: Write failing tests**

Append to `qahal_collab_t0_test.rs`:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn create_collab_agreement_requires_pending_attestations() {
    let mut conductor = SweetConductor::from_standard_config().await;
    let dna = SweetDnaFile::from_bundle(IMAGODEI_DNA.as_ref()).await.unwrap();
    let app = conductor.setup_app("imagodei", &[dna]).await.unwrap();
    let cell = app.cells()[0].clone();

    // Create two collectives first (founder = same agent for now; multi-conductor in Task 6).
    let coll_a: ActionHash = conductor.call(&cell.zome("imagodei"), "create_collective", CreateCollectiveInput {
        charter: "A".into(), display_name: "Coll A".into(),
        salt: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    }).await;
    let coll_b: ActionHash = conductor.call(&cell.zome("imagodei"), "create_collective", CreateCollectiveInput {
        charter: "B".into(), display_name: "Coll B".into(),
        salt: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
    }).await;

    let agreement_hash: ActionHash = conductor.call(&cell.zome("imagodei"), "create_collab_agreement", CreateCollabAgreementInput {
        participants: vec![format!("collective:{}", coll_a), format!("collective:{}", coll_b)],
        scope: "Joint stewardship of riparian restoration".into(),
        share_allocation_json: r#"{"form":"declared","shares":[{"collective_cid":"<A>","share":0.5},{"collective_cid":"<B>","share":0.45}],"commons_pool_tribute":0.05}"#.replace("<A>", &format!("collective:{}", coll_a)).replace("<B>", &format!("collective:{}", coll_b)),
        commons_pool_tribute: 0.05,
        governance_terms_json: r#"{"exit_terms":"clean"}"#.into(),
        initial_tier: "T0".into(),
        display_name_for_qahal: "Riparian Stewards Collab".into(),
        salt: "ccccccccccccccccccccccccccccccc1".into(),
    }).await;

    // At this point the agreement is committed but the Collab-Qahal is NOT instantiated yet
    // because no participating Collective has counter-attested.
    let collab_status: String = conductor.call(&cell.zome("imagodei"), "get_collab_status", agreement_hash.clone()).await;
    assert_eq!(collab_status, "PendingAttestations");

    // Counter-attest from Collective A's founder Steward.
    let _: () = conductor.call(&cell.zome("imagodei"), "attest_collab_agreement", AttestCollabAgreementInput {
        agreement_action_hash: agreement_hash.clone(),
        attesting_collective_cid: format!("collective:{}", coll_a),
    }).await;

    let collab_status_after_a: String = conductor.call(&cell.zome("imagodei"), "get_collab_status", agreement_hash.clone()).await;
    assert_eq!(collab_status_after_a, "PendingAttestations"); // still pending B

    // Counter-attest from Collective B's founder Steward.
    let _: () = conductor.call(&cell.zome("imagodei"), "attest_collab_agreement", AttestCollabAgreementInput {
        agreement_action_hash: agreement_hash.clone(),
        attesting_collective_cid: format!("collective:{}", coll_b),
    }).await;

    let collab_status_final: String = conductor.call(&cell.zome("imagodei"), "get_collab_status", agreement_hash.clone()).await;
    assert_eq!(collab_status_final, "Instantiated");

    // Verify Collab-Qahal exists with two Collective-typed Memberships.
    let collab_qahal_cid: String = conductor.call(&cell.zome("imagodei"), "get_collab_qahal_cid_for_agreement", agreement_hash).await;
    assert!(collab_qahal_cid.starts_with("collective:"));

    let memberships: Vec<Record> = conductor.call(&cell.zome("imagodei"), "list_memberships_for_collective_cid", collab_qahal_cid).await;
    assert_eq!(memberships.len(), 2, "Both participating Collectives are members");
}

#[tokio::test(flavor = "multi_thread")]
async fn create_collab_agreement_refuses_zero_tribute() {
    let mut conductor = SweetConductor::from_standard_config().await;
    let dna = SweetDnaFile::from_bundle(IMAGODEI_DNA.as_ref()).await.unwrap();
    let app = conductor.setup_app("imagodei", &[dna]).await.unwrap();
    let cell = app.cells()[0].clone();

    let coll_a: ActionHash = conductor.call(&cell.zome("imagodei"), "create_collective", CreateCollectiveInput {
        charter: "A".into(), display_name: "Coll A".into(),
        salt: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    }).await;
    let coll_b: ActionHash = conductor.call(&cell.zome("imagodei"), "create_collective", CreateCollectiveInput {
        charter: "B".into(), display_name: "Coll B".into(),
        salt: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
    }).await;

    let result: Result<ActionHash, _> = conductor.call_fallible(&cell.zome("imagodei"), "create_collab_agreement", CreateCollabAgreementInput {
        participants: vec![format!("collective:{}", coll_a), format!("collective:{}", coll_b)],
        scope: "zero-tribute attempt".into(),
        share_allocation_json: r#"{"form":"declared","shares":[{"collective_cid":"<A>","share":0.5},{"collective_cid":"<B>","share":0.5}],"commons_pool_tribute":0.0}"#.replace("<A>", &format!("collective:{}", coll_a)).replace("<B>", &format!("collective:{}", coll_b)),
        commons_pool_tribute: 0.0,
        governance_terms_json: r#"{"exit_terms":"clean"}"#.into(),
        initial_tier: "T0".into(),
        display_name_for_qahal: "Zero-tribute".into(),
        salt: "ddddddddddddddddddddddddddddddd1".into(),
    }).await;

    assert!(result.is_err(), "zero-tribute agreement must be refused");
}
```

- [ ] **Step 2: Run the tests — confirm failure**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest
cargo test --test qahal_collab_t0_test create_collab_agreement -- --nocapture 2>&1 | tail -30
```

Expected: both tests FAIL with missing coordinator functions.

- [ ] **Step 3: Implement the coordinator functions**

Append to `qahal_coordinator.rs`:

```rust
#[hdk_extern]
pub fn create_collab_agreement(input: CreateCollabAgreementInput) -> ExternResult<ActionHash> {
    // Pre-validate the share-allocation JSON parses cleanly + sums to 1.0 with tribute.
    // The integrity validator rejects zero tribute via pure-data check; this coordinator
    // adds the structural JSON shape check.
    validate_share_allocation_json(&input.share_allocation_json, input.commons_pool_tribute)?;

    let agent_cid = encode_agent_cid(&agent_info()?.agent_initial_pubkey)?;
    let agreement = CollabAgreement {
        authored_by_agent_cid: agent_cid,
        participants: input.participants.clone(),
        scope: input.scope,
        share_allocation_json: input.share_allocation_json,
        commons_pool_tribute: input.commons_pool_tribute,
        governance_terms_json: input.governance_terms_json,
        anchor_collective_cid: None,
        initial_tier: input.initial_tier,
        created_at_block_height: current_block_height()?,
        salt: input.salt,
    };
    create_entry(EntryTypes::CollabAgreement(agreement))
}

#[hdk_extern]
pub fn attest_collab_agreement(input: AttestCollabAgreementInput) -> ExternResult<()> {
    // 1. Verify caller is a current Steward of the attesting Collective.
    let caller_cid = encode_agent_cid(&agent_info()?.agent_initial_pubkey)?;
    require_caller_is_steward_of(&caller_cid, &input.attesting_collective_cid)?;

    // 2. Verify the agreement exists and the attesting Collective is a participant.
    let agreement_record = get(input.agreement_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!("CollabAgreement not found"))?;
    let agreement: CollabAgreement = agreement_record.entry().to_app_option()?
        .ok_or_else(|| wasm_error!("CollabAgreement decode failure"))?;
    if !agreement.participants.contains(&input.attesting_collective_cid) {
        return Err(wasm_error!("attesting Collective is not a participant"));
    }

    // 3. Record the counter-attestation as a link from agreement -> attesting collective.
    //    Tag carries the Collective CID for cheap iteration.
    let attesting_collective_action = decode_collective_cid_to_action(&input.attesting_collective_cid)?;
    create_link(
        input.agreement_action_hash.clone(),
        attesting_collective_action,
        LinkTypes::AgreementOnCollab,
        input.attesting_collective_cid.clone().into_bytes(),
    )?;

    // 4. Check whether ALL participants have attested. If so, instantiate the Collab-Qahal.
    let attestation_links = get_links(
        input.agreement_action_hash.clone(),
        LinkTypes::AgreementOnCollab,
        None,
    )?;
    let attested_cids: std::collections::HashSet<String> = attestation_links
        .iter()
        .filter_map(|l| String::from_utf8(l.tag.clone().into_inner()).ok())
        .collect();
    if agreement.participants.iter().all(|p| attested_cids.contains(p)) {
        instantiate_collab_qahal(&input.agreement_action_hash, &agreement)?;
    }
    Ok(())
}

/// Atomic Collab-Qahal instantiation: creates the recursive Collective entry +
/// Collective-typed Memberships for each participant + bidirectional links.
fn instantiate_collab_qahal(
    agreement_hash: &ActionHash,
    agreement: &CollabAgreement,
) -> ExternResult<()> {
    let block_height = current_block_height()?;
    let collab_qahal = Collective {
        founder_agent_cid: agreement.authored_by_agent_cid.clone(),
        charter: format!("Collab-Qahal anchored on CollabAgreement {}", agreement_hash),
        display_name: format!("Collab: {}", agreement.scope.chars().take(64).collect::<String>()),
        created_at_block_height: block_height,
        salt: agreement.salt.clone(),
        anchor_agreement_cid: Some(format!("agreement:{}", agreement_hash)),
    };
    let qahal_hash = create_entry(EntryTypes::Collective(collab_qahal))?;
    let qahal_cid = format!("collective:{}", qahal_hash);

    // Link agreement -> instantiated Qahal (single, for cheap lookup).
    create_link(
        agreement_hash.clone(),
        qahal_hash.clone(),
        LinkTypes::AgreementOnCollab,
        b"INSTANTIATED".to_vec(),
    )?;

    // Create a Collective-typed Membership for each participating Collective.
    // Role = Steward (each participating Collective steward-stewards the Collab).
    // No sponsor needed — the Agreement itself IS the sponsorship act, recorded
    // as agreement_action_hash via MembershipForAgreement link.
    for participant_cid in &agreement.participants {
        let membership = Membership {
            member_cid: participant_cid.clone(),
            member_kind: MemberKind::Collective,
            collective_cid: qahal_cid.clone(),
            role: MembershipRole::Steward,
            sponsor_cid: Some(format!("agreement:{}", agreement_hash)),
            joined_at_block_height: block_height,
            withdrawn_at_block_height: None,
        };
        let m_hash = create_entry(EntryTypes::Membership(membership))?;
        create_link(qahal_hash.clone(), m_hash.clone(), LinkTypes::HasMembership, ())?;
        create_link(agreement_hash.clone(), m_hash, LinkTypes::MembershipForAgreement, ())?;
    }
    Ok(())
}

#[hdk_extern]
pub fn get_collab_status(agreement_hash: ActionHash) -> ExternResult<String> {
    let links = get_links(agreement_hash.clone(), LinkTypes::AgreementOnCollab, None)?;
    let has_instantiation_marker = links.iter().any(|l| l.tag.clone().into_inner() == b"INSTANTIATED");
    if has_instantiation_marker {
        return Ok("Instantiated".into());
    }
    Ok("PendingAttestations".into())
}

#[hdk_extern]
pub fn get_collab_qahal_cid_for_agreement(agreement_hash: ActionHash) -> ExternResult<String> {
    let links = get_links(agreement_hash, LinkTypes::AgreementOnCollab, None)?;
    let instantiation = links.iter().find(|l| l.tag.clone().into_inner() == b"INSTANTIATED")
        .ok_or_else(|| wasm_error!("Collab not yet instantiated"))?;
    Ok(format!("collective:{}", ActionHash::from(instantiation.target.clone())))
}

#[hdk_extern]
pub fn list_memberships_for_collective_cid(collective_cid: String) -> ExternResult<Vec<Record>> {
    let collective_hash = decode_collective_cid_to_action(&collective_cid)?;
    list_memberships_for_collective(collective_hash)
}

fn require_caller_is_steward_of(agent_cid: &str, collective_cid: &str) -> ExternResult<()> {
    let collective_hash = decode_collective_cid_to_action(collective_cid)?;
    let memberships_records = list_memberships_for_collective(collective_hash)?;
    for record in memberships_records {
        if let Some(m) = record.entry().to_app_option::<Membership>()? {
            if m.member_cid == agent_cid
                && matches!(m.role, MembershipRole::Steward)
                && m.withdrawn_at_block_height.is_none()
            {
                return Ok(());
            }
        }
    }
    Err(wasm_error!("caller is not a current Steward of {}", collective_cid))
}

fn decode_collective_cid_to_action(cid: &str) -> ExternResult<ActionHash> {
    let raw = cid.strip_prefix("collective:")
        .ok_or_else(|| wasm_error!("collective CID must start with 'collective:'"))?;
    ActionHash::try_from(raw.to_string())
        .map_err(|_| wasm_error!("invalid ActionHash in collective CID"))
}

fn validate_share_allocation_json(json: &str, claimed_tribute: f64) -> ExternResult<()> {
    let parsed: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| wasm_error!("share_allocation JSON parse: {}", e))?;
    let form = parsed.get("form").and_then(|v| v.as_str())
        .ok_or_else(|| wasm_error!("share_allocation.form missing"))?;
    if form != "declared" {
        return Err(wasm_error!("M1 only supports share_allocation.form=\"declared\""));
    }
    let tribute = parsed.get("commons_pool_tribute").and_then(|v| v.as_f64())
        .ok_or_else(|| wasm_error!("share_allocation.commons_pool_tribute missing"))?;
    if (tribute - claimed_tribute).abs() > 1e-9 {
        return Err(wasm_error!("share_allocation tribute mismatch with field"));
    }
    if tribute <= 0.0 {
        return Err(wasm_error!("commons_pool_tribute must be > 0"));
    }
    let shares = parsed.get("shares").and_then(|v| v.as_array())
        .ok_or_else(|| wasm_error!("share_allocation.shares missing"))?;
    let share_sum: f64 = shares.iter()
        .filter_map(|s| s.get("share").and_then(|v| v.as_f64()))
        .sum();
    if (share_sum + tribute - 1.0).abs() > 1e-6 {
        return Err(wasm_error!(
            "shares ({}) + tribute ({}) must sum to 1.0",
            share_sum, tribute
        ));
    }
    Ok(())
}
```

- [ ] **Step 4: Run the tests — confirm pass**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest
cargo test --test qahal_collab_t0_test create_collab_agreement -- --nocapture 2>&1 | tail -30
```

Expected: both tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs \
        elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs
git commit -m "feat(qahal-coordinator): create_collab_agreement + attest + atomic instantiation

Pre-declared root of a T0 Collab per spec §5.1. CollabAgreement requires
counter-attestation from a Steward of each participating Collective; on
final attestation the Collab-Qahal is instantiated atomically as a
recursive Collective with Collective-typed Memberships for each
participant. Zero-tribute and non-Steward callers are refused.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Implement `withdraw_membership_clean`

**Files:**
- Modify: `elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs`
- Test: extend `elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs`

- [ ] **Step 1: Write failing test**

Append to `qahal_collab_t0_test.rs`:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn withdraw_membership_clean_exit() {
    // Build a 2-collective Collab-Qahal (reuse the helper from previous test).
    let mut conductor = SweetConductor::from_standard_config().await;
    let dna = SweetDnaFile::from_bundle(IMAGODEI_DNA.as_ref()).await.unwrap();
    let app = conductor.setup_app("imagodei", &[dna]).await.unwrap();
    let cell = app.cells()[0].clone();

    let (collab_qahal_cid, membership_for_a) = build_two_collective_collab_qahal(&conductor, &cell).await;

    let _: () = conductor.call(&cell.zome("imagodei"), "withdraw_membership_clean", WithdrawMembershipInput {
        membership_action_hash: membership_for_a.clone(),
        collab_qahal_cid: collab_qahal_cid.clone(),
    }).await;

    // The Membership entry should now reflect withdrawn_at_block_height set.
    let updated: Record = conductor.call(&cell.zome("imagodei"), "get_membership_by_action", membership_for_a).await;
    let m: Membership = updated.entry().to_app_option().unwrap().unwrap();
    assert!(m.withdrawn_at_block_height.is_some(), "withdraw_at_block_height set");
}

async fn build_two_collective_collab_qahal(
    conductor: &SweetConductor,
    cell: &SweetCell,
) -> (String, ActionHash) {
    // (test helper: creates 2 Collectives, an Agreement, two attestations; returns the
    // Collab-Qahal CID and the ActionHash of Collective-A's Membership in the Qahal)
    todo!("inline using the patterns from previous tests")
}
```

(Note: the `todo!()` should be expanded inline by the implementer — copy the setup pattern from the previous test.)

- [ ] **Step 2: Run test — confirm failure**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest
cargo test --test qahal_collab_t0_test withdraw_membership -- --nocapture 2>&1 | tail -15
```

Expected: FAIL.

- [ ] **Step 3: Implement `withdraw_membership_clean`**

Append to `qahal_coordinator.rs`:

```rust
#[hdk_extern]
pub fn withdraw_membership_clean(input: WithdrawMembershipInput) -> ExternResult<()> {
    let existing = get(input.membership_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!("Membership not found"))?;
    let m: Membership = existing.entry().to_app_option()?
        .ok_or_else(|| wasm_error!("Membership decode failure"))?;

    // Authority check: caller must be a current Steward of the WITHDRAWING
    // member (when member_kind == Collective) OR the member themselves (Person).
    let caller_cid = encode_agent_cid(&agent_info()?.agent_initial_pubkey)?;
    match m.member_kind {
        MemberKind::Person => {
            if m.member_cid != caller_cid {
                return Err(wasm_error!("only the member may withdraw their own Person Membership"));
            }
        }
        MemberKind::Collective => {
            require_caller_is_steward_of(&caller_cid, &m.member_cid)?;
        }
        MemberKind::ElohimAgent => {
            // ElohimAgent membership withdrawal is governed differently; deferred.
            return Err(wasm_error!("ElohimAgent Membership withdrawal not supported in M1"));
        }
    }

    let block_height = current_block_height()?;
    let updated = Membership {
        withdrawn_at_block_height: Some(block_height),
        ..m
    };
    update_entry(input.membership_action_hash, EntryTypes::Membership(updated))?;
    Ok(())
}

#[hdk_extern]
pub fn get_membership_by_action(action_hash: ActionHash) -> ExternResult<Record> {
    get(action_hash, GetOptions::default())?
        .ok_or_else(|| wasm_error!("Membership not found"))
}
```

- [ ] **Step 4: Run test — confirm pass**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest
cargo test --test qahal_collab_t0_test withdraw_membership -- --nocapture 2>&1 | tail -15
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs \
        elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs
git commit -m "feat(qahal-coordinator): withdraw_membership_clean

Clean-exit per spec §6.4. Sets withdrawn_at_block_height; future REA flow
allocation calculations honor this boundary. Person members withdraw
themselves; Collective members are withdrawn by their Stewards. Repair-exit
deferred to M3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase B — Schema + InputViews + Views (Rust + JSON + TS codegen)

### Task 7: Author JSON schemas

**Source-of-truth note (P2P design gate):** All files in this task are **Category C — wire-shape projections** of the Cat-A DHT entries created in Phase A (`Collective`, `Membership`, `CollabAgreement`) and the Cat-A2 link (`AgreementOnCollab`). None of these files introduce a new source of truth; each is reconstructible from DHT state. See the "Source-of-Truth" table at the top of this plan for the full mapping.

**Files (all Cat C — wire-shape projection of Cat-A/Cat-A2 DHT entries from Phase A):**
- Create: `elohim/sdk/schemas/v1/enums/member-kind.schema.json` — Cat C vocabulary mirroring Cat-A `Membership.member_kind`
- Create: `elohim/sdk/schemas/v1/enums/elohim-tier.schema.json` — Cat C vocabulary mirroring Cat-A `CollabAgreement.initial_tier`
- Create: `elohim/sdk/schemas/v1/enums/share-allocation-form.schema.json` — Cat C vocabulary mirroring Cat-A `CollabAgreement.share_allocation_json.form`
- Create: `elohim/sdk/schemas/v1/objects/share-allocation.schema.json` — Cat C embedded wire-shape projection of Cat-A `CollabAgreement.share_allocation_json`
- Create: `elohim/sdk/schemas/v1/inputs/create-collective-input.schema.json` — Cat C wire-shape input projecting onto Cat-A `Collective` create flow
- Create: `elohim/sdk/schemas/v1/inputs/create-collab-agreement-input.schema.json` — Cat C wire-shape input projecting onto Cat-A `CollabAgreement` create flow
- Create: `elohim/sdk/schemas/v1/inputs/attest-collab-agreement-input.schema.json` — Cat C wire-shape input projecting onto Cat-A2 `AgreementOnCollab` link create flow
- Create: `elohim/sdk/schemas/v1/inputs/withdraw-membership-input.schema.json` — Cat C wire-shape input projecting onto Cat-A `Membership` update flow (sets `withdrawn_at_block_height`)
- Create: `elohim/sdk/schemas/v1/views/collective-view.schema.json` — Cat C HTTP wire-shape projection of Cat-A `Collective`
- Create: `elohim/sdk/schemas/v1/views/collab-qahal-view.schema.json` — Cat C HTTP wire-shape projection of Cat-A `Collective` (Collab-Qahal) + Cat-A `Membership` traversal
- Create: `elohim/sdk/schemas/v1/views/membership-view.schema.json` — Cat C HTTP wire-shape projection of Cat-A `Membership`
- Create: `elohim/sdk/schemas/v1/views/collab-agreement-view.schema.json` — Cat C HTTP wire-shape projection of Cat-A `CollabAgreement` + derived Cat-A2 counter-attestation status

- [ ] **Step 1: Author all schema files**

`elohim/sdk/schemas/v1/enums/member-kind.schema.json`:

```json
{
  "$id": "epr:schema:enum:member-kind",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MemberKind",
  "description": "Source of truth: Cat-A Membership DHT entry's member_kind field in imagodei DNA (DNA-notarized). Category C — wire-shape projection of that enum field. Polymorphic membership subject type for Collective/Qahal members. Per spec §2.1.",
  "type": "string",
  "enum": ["Person", "Collective", "ElohimAgent"]
}
```

`elohim/sdk/schemas/v1/enums/elohim-tier.schema.json`:

```json
{
  "$id": "epr:schema:enum:elohim-tier",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ElohimTier",
  "description": "Source of truth: Cat-A CollabAgreement DHT entry's initial_tier field in imagodei DNA, plus the deferred friction-gradient evaluator's derived tier output (DNA-notarized at creation; derived thereafter). Category C — wire-shape vocabulary. Coordination scale tier for a Collab-Qahal. Per spec §3.1. M1 only reaches T0; T1+ requires deferred specs.",
  "type": "string",
  "enum": ["T0", "T1", "T2", "T3"]
}
```

`elohim/sdk/schemas/v1/enums/share-allocation-form.schema.json`:

```json
{
  "$id": "epr:schema:enum:share-allocation-form",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ShareAllocationForm",
  "description": "Source of truth: Cat-A CollabAgreement DHT entry's share_allocation_json.form field in imagodei DNA (DNA-notarized). Category C — wire-shape vocabulary. Form of share-routing function. Per spec §6.1. M1 only supports Declared.",
  "type": "string",
  "enum": ["Declared", "AffinityDerived"]
}
```

`elohim/sdk/schemas/v1/objects/share-allocation.schema.json`:

```json
{
  "$id": "epr:schema:object:share-allocation",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ShareAllocation",
  "description": "Source of truth: Cat-A CollabAgreement DHT entry's share_allocation_json field in imagodei DNA (DNA-notarized as canonical CBOR within the entry). Category C — wire-shape projection. Share-routing function declared on a CollabAgreement. Form A = Declared shares; Form B = AffinityDerived (M2).",
  "type": "object",
  "required": ["form", "commonsPoolTribute"],
  "additionalProperties": false,
  "properties": {
    "form": { "$ref": "../enums/share-allocation-form.schema.json" },
    "shares": {
      "type": "array",
      "description": "Required when form == Declared. Each share is fractional (0,1); sum + commonsPoolTribute must equal 1.0.",
      "items": {
        "type": "object",
        "required": ["collectiveCid", "share"],
        "additionalProperties": false,
        "properties": {
          "collectiveCid": { "type": "string" },
          "share": { "type": "number", "exclusiveMinimum": 0, "maximum": 1 }
        }
      }
    },
    "affinityWindowBlocks": {
      "type": "integer",
      "description": "Required when form == AffinityDerived (M2)."
    },
    "rebalanceCadenceBlocks": {
      "type": "integer",
      "description": "Required when form == AffinityDerived (M2)."
    },
    "commonsPoolTribute": {
      "type": "number",
      "exclusiveMinimum": 0,
      "maximum": 1,
      "description": "Substrate-validated > 0. Zero tribute is refused per spec §6.3."
    }
  }
}
```

`elohim/sdk/schemas/v1/inputs/create-collective-input.schema.json`:

```json
{
  "$id": "epr:schema:input:create-collective",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CreateCollectiveInput",
  "description": "Source of truth: Cat-A Collective DHT entry in imagodei DNA (DNA-notarized at commit via integrity validator). Category C — HTTP wire-shape input projecting onto the create_collective coordinator flow. Body for POST /api/v1/collective.",
  "type": "object",
  "required": ["charter", "displayName", "salt"],
  "additionalProperties": false,
  "properties": {
    "charter": { "type": "string", "minLength": 1, "maxLength": 16384 },
    "displayName": { "type": "string", "minLength": 1, "maxLength": 256 },
    "salt": { "type": "string", "pattern": "^[0-9a-f]{32}$" }
  }
}
```

`elohim/sdk/schemas/v1/inputs/create-collab-agreement-input.schema.json`:

```json
{
  "$id": "epr:schema:input:create-collab-agreement",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CreateCollabAgreementInput",
  "description": "Source of truth: Cat-A CollabAgreement DHT entry in imagodei DNA (DNA-notarized at commit via integrity validator + coordinator JSON structural checks). Category C — HTTP wire-shape input projecting onto the create_collab_agreement coordinator flow. Body for POST /api/v1/collab/agreement.",
  "type": "object",
  "required": ["participants", "scope", "shareAllocation", "initialTier", "displayNameForQahal", "salt"],
  "additionalProperties": false,
  "properties": {
    "participants": {
      "type": "array",
      "minItems": 2,
      "items": { "type": "string", "pattern": "^collective:" }
    },
    "scope": { "type": "string", "minLength": 1, "maxLength": 16384 },
    "shareAllocation": { "$ref": "../objects/share-allocation.schema.json" },
    "governanceTerms": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "exitTerms": { "type": "string", "enum": ["clean", "repair"] }
      }
    },
    "initialTier": { "type": "string", "const": "T0" },
    "displayNameForQahal": { "type": "string", "minLength": 1, "maxLength": 256 },
    "salt": { "type": "string", "pattern": "^[0-9a-f]{32}$" }
  }
}
```

`elohim/sdk/schemas/v1/inputs/attest-collab-agreement-input.schema.json`:

```json
{
  "$id": "epr:schema:input:attest-collab-agreement",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AttestCollabAgreementInput",
  "description": "Source of truth: Cat-A2 AgreementOnCollab link in imagodei DNA (DNA-notarized link with attesting collective CID in the tag bytes). Category C — HTTP wire-shape input projecting onto the attest_collab_agreement coordinator flow. Body for POST /api/v1/collab/agreement/{cid}/attest.",
  "type": "object",
  "required": ["agreementCid", "attestingCollectiveCid"],
  "additionalProperties": false,
  "properties": {
    "agreementCid": { "type": "string", "pattern": "^agreement:" },
    "attestingCollectiveCid": { "type": "string", "pattern": "^collective:" }
  }
}
```

`elohim/sdk/schemas/v1/inputs/withdraw-membership-input.schema.json`:

```json
{
  "$id": "epr:schema:input:withdraw-membership",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "WithdrawMembershipInput",
  "description": "Source of truth: Cat-A Membership DHT entry in imagodei DNA (DNA-notarized update setting withdrawn_at_block_height). Category C — HTTP wire-shape input projecting onto the withdraw_membership_clean coordinator flow. Body for POST /api/v1/collab/{cid}/withdraw.",
  "type": "object",
  "required": ["membershipCid", "collabQahalCid"],
  "additionalProperties": false,
  "properties": {
    "membershipCid": { "type": "string", "pattern": "^membership:" },
    "collabQahalCid": { "type": "string", "pattern": "^collective:" }
  }
}
```

`elohim/sdk/schemas/v1/views/collective-view.schema.json`:

```json
{
  "$id": "epr:schema:view:collective",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CollectiveView",
  "description": "Source of truth: Cat-A Collective DHT entry in imagodei DNA (DNA-notarized). Category C — HTTP wire-shape projection; reconstructible at any time from DHT state. Wire shape for GET /api/v1/collective/:cid.",
  "type": "object",
  "required": ["cid", "founderAgentCid", "charter", "displayName", "createdAtBlockHeight", "elohimTier"],
  "additionalProperties": false,
  "properties": {
    "cid": { "type": "string", "pattern": "^collective:" },
    "founderAgentCid": { "type": "string" },
    "charter": { "type": "string" },
    "displayName": { "type": "string" },
    "createdAtBlockHeight": { "type": "integer" },
    "anchorAgreementCid": { "type": ["string", "null"], "pattern": "^agreement:" },
    "elohimTier": { "$ref": "../enums/elohim-tier.schema.json" }
  }
}
```

`elohim/sdk/schemas/v1/views/membership-view.schema.json`:

```json
{
  "$id": "epr:schema:view:membership",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MembershipView",
  "description": "Source of truth: Cat-A Membership DHT entry in imagodei DNA (DNA-notarized). Category C — HTTP wire-shape projection; reconstructible at any time from DHT state. Wire shape for Membership entries with memberKind discriminating polymorphic subject.",
  "type": "object",
  "required": ["cid", "memberCid", "memberKind", "collectiveCid", "role", "joinedAtBlockHeight"],
  "additionalProperties": false,
  "properties": {
    "cid": { "type": "string", "pattern": "^membership:" },
    "memberCid": { "type": "string" },
    "memberKind": { "$ref": "../enums/member-kind.schema.json" },
    "collectiveCid": { "type": "string", "pattern": "^collective:" },
    "role": { "type": "string", "enum": ["Steward", "Contributor", "Observer"] },
    "sponsorCid": { "type": ["string", "null"] },
    "joinedAtBlockHeight": { "type": "integer" },
    "withdrawnAtBlockHeight": { "type": ["integer", "null"] }
  }
}
```

`elohim/sdk/schemas/v1/views/collab-agreement-view.schema.json`:

```json
{
  "$id": "epr:schema:view:collab-agreement",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CollabAgreementView",
  "description": "Source of truth: Cat-A CollabAgreement DHT entry in imagodei DNA (DNA-notarized) plus derived counter-attestation status computed from Cat-A2 AgreementOnCollab links (DNA-notarized link metadata). Category C — HTTP wire-shape projection; reconstructible at any time from DHT state. Includes counter-attestation status + the Collab-Qahal CID once instantiated.",
  "type": "object",
  "required": ["cid", "authoredByAgentCid", "participants", "scope", "shareAllocation", "commonsPoolTribute", "initialTier", "createdAtBlockHeight", "status"],
  "additionalProperties": false,
  "properties": {
    "cid": { "type": "string", "pattern": "^agreement:" },
    "authoredByAgentCid": { "type": "string" },
    "participants": {
      "type": "array",
      "items": { "type": "string", "pattern": "^collective:" }
    },
    "scope": { "type": "string" },
    "shareAllocation": { "$ref": "../objects/share-allocation.schema.json" },
    "commonsPoolTribute": { "type": "number" },
    "governanceTerms": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "exitTerms": { "type": "string", "enum": ["clean", "repair"] }
      }
    },
    "initialTier": { "$ref": "../enums/elohim-tier.schema.json" },
    "createdAtBlockHeight": { "type": "integer" },
    "status": { "type": "string", "enum": ["PendingAttestations", "Instantiated"] },
    "attestedBy": {
      "type": "array",
      "items": { "type": "string", "pattern": "^collective:" }
    },
    "collabQahalCid": { "type": ["string", "null"], "pattern": "^collective:" }
  }
}
```

`elohim/sdk/schemas/v1/views/collab-qahal-view.schema.json`:

```json
{
  "$id": "epr:schema:view:collab-qahal",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CollabQahalView",
  "description": "Source of truth: Cat-A Collective DHT entry in imagodei DNA (the Collab-Qahal is itself a recursive Collective; anchor_agreement_cid distinguishes it) plus traversal over Cat-A Membership entries via Cat-A2 HasMembership links (all DNA-notarized). Category C — HTTP wire-shape projection; reconstructible at any time. Wire shape returned by GET /api/v1/collab/:cid.",
  "type": "object",
  "required": ["cid", "anchorAgreementCid", "displayName", "createdAtBlockHeight", "elohimTier", "memberCollectives", "memberPersons"],
  "additionalProperties": false,
  "properties": {
    "cid": { "type": "string", "pattern": "^collective:" },
    "anchorAgreementCid": { "type": "string", "pattern": "^agreement:" },
    "displayName": { "type": "string" },
    "createdAtBlockHeight": { "type": "integer" },
    "elohimTier": { "$ref": "../enums/elohim-tier.schema.json" },
    "memberCollectives": {
      "type": "array",
      "items": { "$ref": "./collective-view.schema.json" }
    },
    "memberPersons": {
      "type": "array",
      "items": { "type": "string" }
    },
    "commonsPoolBalance": { "type": "number" }
  }
}
```

- [ ] **Step 2: Run schema validation**

```bash
cd /projects/elohim
pnpm run schema:test 2>&1 | tail -10
```

Expected: all schemas validate as JSON Schema 2020-12.

- [ ] **Step 3: Add new view schemas to codegen INTERFACE_FILES**

Open `elohim/sdk/schemas/scripts/codegen-ts.mjs` and add to the `INTERFACE_FILES` array:

```javascript
"objects/share-allocation.schema.json",
"views/collective-view.schema.json",
"views/membership-view.schema.json",
"views/collab-agreement-view.schema.json",
"views/collab-qahal-view.schema.json",
```

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add elohim/sdk/schemas/v1/enums/member-kind.schema.json \
        elohim/sdk/schemas/v1/enums/elohim-tier.schema.json \
        elohim/sdk/schemas/v1/enums/share-allocation-form.schema.json \
        elohim/sdk/schemas/v1/objects/share-allocation.schema.json \
        elohim/sdk/schemas/v1/inputs/create-collective-input.schema.json \
        elohim/sdk/schemas/v1/inputs/create-collab-agreement-input.schema.json \
        elohim/sdk/schemas/v1/inputs/attest-collab-agreement-input.schema.json \
        elohim/sdk/schemas/v1/inputs/withdraw-membership-input.schema.json \
        elohim/sdk/schemas/v1/views/collective-view.schema.json \
        elohim/sdk/schemas/v1/views/collab-qahal-view.schema.json \
        elohim/sdk/schemas/v1/views/membership-view.schema.json \
        elohim/sdk/schemas/v1/views/collab-agreement-view.schema.json \
        elohim/sdk/schemas/scripts/codegen-ts.mjs
git commit -m "feat(schemas): qahal substrate JSON schemas (enums + objects + inputs + views)

3 enums (MemberKind, ElohimTier, ShareAllocationForm), 1 object (ShareAllocation),
4 inputs (create-collective, create-collab-agreement, attest, withdraw),
4 views (collective, collab-qahal, membership, collab-agreement). Added to
codegen INTERFACE_FILES for TS regeneration in next task.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: Author Rust view structs in elohim-views

**Files:**
- Create: `elohim/elohim-views/src/qahal.rs`
- Modify: `elohim/elohim-views/src/lib.rs`

- [ ] **Step 1: Create the Rust qahal module**

Write to `elohim/elohim-views/src/qahal.rs`:

```rust
//! Qahal view types — Rust side of the boundary. Source of truth = JSON schemas in
//! `elohim/sdk/schemas/v1/`. ts-rs derives TypeScript types via `cargo test
//! export_bindings` (see CLAUDE.md for the pipeline).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub enum MemberKind {
    Person,
    Collective,
    ElohimAgent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub enum ElohimTier {
    T0,
    T1,
    T2,
    T3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub enum ShareAllocationForm {
    Declared,
    AffinityDerived,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DeclaredShare {
    pub collective_cid: String,
    pub share: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ShareAllocation {
    pub form: ShareAllocationForm,
    pub shares: Option<Vec<DeclaredShare>>,
    pub affinity_window_blocks: Option<u64>,
    pub rebalance_cadence_blocks: Option<u64>,
    pub commons_pool_tribute: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CollectiveView {
    pub cid: String,
    pub founder_agent_cid: String,
    pub charter: String,
    pub display_name: String,
    pub created_at_block_height: u64,
    pub anchor_agreement_cid: Option<String>,
    pub elohim_tier: ElohimTier,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub enum MembershipRole {
    Steward,
    Contributor,
    Observer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct MembershipView {
    pub cid: String,
    pub member_cid: String,
    pub member_kind: MemberKind,
    pub collective_cid: String,
    pub role: MembershipRole,
    pub sponsor_cid: Option<String>,
    pub joined_at_block_height: u64,
    pub withdrawn_at_block_height: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub enum CollabAgreementStatus {
    PendingAttestations,
    Instantiated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct GovernanceTerms {
    pub exit_terms: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CollabAgreementView {
    pub cid: String,
    pub authored_by_agent_cid: String,
    pub participants: Vec<String>,
    pub scope: String,
    pub share_allocation: ShareAllocation,
    pub commons_pool_tribute: f64,
    pub governance_terms: GovernanceTerms,
    pub initial_tier: ElohimTier,
    pub created_at_block_height: u64,
    pub status: CollabAgreementStatus,
    pub attested_by: Vec<String>,
    pub collab_qahal_cid: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CollabQahalView {
    pub cid: String,
    pub anchor_agreement_cid: String,
    pub display_name: String,
    pub created_at_block_height: u64,
    pub elohim_tier: ElohimTier,
    pub member_collectives: Vec<CollectiveView>,
    pub member_persons: Vec<String>,
    pub commons_pool_balance: f64,
}

// Input types — body shapes for POST routes.

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectiveInputView {
    pub charter: String,
    pub display_name: String,
    pub salt: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct CreateCollabAgreementInputView {
    pub participants: Vec<String>,
    pub scope: String,
    pub share_allocation: ShareAllocation,
    pub governance_terms: GovernanceTerms,
    pub initial_tier: ElohimTier,
    pub display_name_for_qahal: String,
    pub salt: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct AttestCollabAgreementInputView {
    pub agreement_cid: String,
    pub attesting_collective_cid: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct WithdrawMembershipInputView {
    pub membership_cid: String,
    pub collab_qahal_cid: String,
}
```

- [ ] **Step 2: Re-export from lib.rs**

In `elohim/elohim-views/src/lib.rs`, append (or insert in the appropriate location):

```rust
pub mod qahal;
pub use qahal::*;
```

- [ ] **Step 3: Build and regenerate TS bindings**

```bash
cd /projects/elohim/elohim/elohim-views
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test export_bindings 2>&1 | tail -10
```

Expected: PASS; new files appear in `elohim/sdk/storage-client-ts/src/generated/` (CollectiveView.ts, CollabQahalView.ts, etc.).

- [ ] **Step 4: Verify codegen-ts.mjs produces matching interfaces**

```bash
cd /projects/elohim
pnpm run schema:codegen:ts 2>&1 | tail -10
```

Expected: PASS. The schema-codegen produces JSON-schema-aligned types; ts-rs produces Rust-struct-aligned types. They should agree on field shape (both use camelCase per `serde(rename_all = "camelCase")`).

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-views/src/qahal.rs elohim/elohim-views/src/lib.rs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(views): qahal Rust view structs + TS regeneration

Adds CollectiveView, MembershipView, CollabAgreementView, CollabQahalView,
ShareAllocation, and the 4 InputViews. ts-rs export_bindings regenerates
matching TypeScript interfaces. Schema contract tests land in next task.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: Add schema contract tests

**Files:**
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Add contract tests for each new view**

In `elohim/elohim-storage/tests/schema_contract.rs`, append:

```rust
#[test]
fn collective_view_round_trips_through_schema() {
    let v = CollectiveView {
        cid: "collective:abc123".into(),
        founder_agent_cid: "agent:xyz".into(),
        charter: "Test charter".into(),
        display_name: "Test Collective".into(),
        created_at_block_height: 12345,
        anchor_agreement_cid: None,
        elohim_tier: ElohimTier::T0,
    };
    let json = serde_json::to_value(&v).unwrap();
    assert_against_schema(&json, "views/collective-view.schema.json");
    let round: CollectiveView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
fn collab_qahal_view_round_trips() {
    let v = CollabQahalView {
        cid: "collective:qahal-1".into(),
        anchor_agreement_cid: "agreement:abc".into(),
        display_name: "Test Collab".into(),
        created_at_block_height: 22222,
        elohim_tier: ElohimTier::T0,
        member_collectives: vec![],
        member_persons: vec!["agent:p1".into()],
        commons_pool_balance: 0.0,
    };
    let json = serde_json::to_value(&v).unwrap();
    assert_against_schema(&json, "views/collab-qahal-view.schema.json");
    let round: CollabQahalView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
fn membership_view_round_trips() {
    let v = MembershipView {
        cid: "membership:m1".into(),
        member_cid: "collective:a".into(),
        member_kind: MemberKind::Collective,
        collective_cid: "collective:qahal-1".into(),
        role: MembershipRole::Steward,
        sponsor_cid: Some("agreement:abc".into()),
        joined_at_block_height: 33333,
        withdrawn_at_block_height: None,
    };
    let json = serde_json::to_value(&v).unwrap();
    assert_against_schema(&json, "views/membership-view.schema.json");
    let round: MembershipView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
fn collab_agreement_view_round_trips() {
    let v = CollabAgreementView {
        cid: "agreement:abc".into(),
        authored_by_agent_cid: "agent:author".into(),
        participants: vec!["collective:a".into(), "collective:b".into()],
        scope: "test scope".into(),
        share_allocation: ShareAllocation {
            form: ShareAllocationForm::Declared,
            shares: Some(vec![
                DeclaredShare { collective_cid: "collective:a".into(), share: 0.5 },
                DeclaredShare { collective_cid: "collective:b".into(), share: 0.45 },
            ]),
            affinity_window_blocks: None,
            rebalance_cadence_blocks: None,
            commons_pool_tribute: 0.05,
        },
        commons_pool_tribute: 0.05,
        governance_terms: GovernanceTerms { exit_terms: "clean".into() },
        initial_tier: ElohimTier::T0,
        created_at_block_height: 44444,
        status: CollabAgreementStatus::PendingAttestations,
        attested_by: vec![],
        collab_qahal_cid: None,
    };
    let json = serde_json::to_value(&v).unwrap();
    assert_against_schema(&json, "views/collab-agreement-view.schema.json");
    let round: CollabAgreementView = serde_json::from_value(json).unwrap();
    assert_eq!(v, round);
}

#[test]
fn share_allocation_refuses_zero_tribute_via_schema() {
    let bad = serde_json::json!({
        "form": "Declared",
        "shares": [
            {"collectiveCid": "collective:a", "share": 0.5},
            {"collectiveCid": "collective:b", "share": 0.5}
        ],
        "commonsPoolTribute": 0.0
    });
    let res = validate_against_schema(&bad, "objects/share-allocation.schema.json");
    assert!(res.is_err(), "JSON schema must refuse zero tribute via exclusiveMinimum");
}
```

(Use the existing test helpers `assert_against_schema` and `validate_against_schema` in `schema_contract.rs`.)

- [ ] **Step 2: Run the tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test schema_contract qahal 2>&1 | tail -15
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/tests/schema_contract.rs
git commit -m "test(schemas): contract round-trip for qahal view + input types

Each new view round-trips Rust→JSON→schema→Rust. Zero-tribute is refused
at the JSON Schema layer via exclusiveMinimum on commonsPoolTribute.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase C — Storage HTTP surface

### Task 10: Implement `qahal_service` (zome-call wrappers + projection)

**Files:**
- Create: `elohim/elohim-storage/src/services/qahal_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Create service module**

Write to `elohim/elohim-storage/src/services/qahal_service.rs`:

```rust
//! Qahal service — orchestrates zome calls + projection caching for Collective +
//! Collab flows. The HTTP routes in http.rs are thin shells over this service.
//!
//! Per spec: 2026-05-23-multi-collective-collaboration-epr-design.md §2 + §5.1.

use crate::error::StorageError;
use crate::zome_client::ZomeClient;
use elohim_views::qahal::*;
use std::sync::Arc;

pub struct QahalService {
    zome: Arc<ZomeClient>,
}

impl QahalService {
    pub fn new(zome: Arc<ZomeClient>) -> Self {
        Self { zome }
    }

    pub async fn create_collective(
        &self,
        input: CreateCollectiveInputView,
    ) -> Result<CollectiveView, StorageError> {
        let action_hash = self
            .zome
            .call("imagodei", "create_collective", &input)
            .await?;
        self.fetch_collective_by_action(action_hash).await
    }

    pub async fn create_collab_agreement(
        &self,
        input: CreateCollabAgreementInputView,
    ) -> Result<CollabAgreementView, StorageError> {
        let action_hash = self
            .zome
            .call("imagodei", "create_collab_agreement", &input)
            .await?;
        self.fetch_agreement_by_action(action_hash).await
    }

    pub async fn attest_collab_agreement(
        &self,
        input: AttestCollabAgreementInputView,
    ) -> Result<CollabAgreementView, StorageError> {
        let agreement_hash = decode_action_hash_from_cid(&input.agreement_cid, "agreement:")?;
        self.zome
            .call("imagodei", "attest_collab_agreement", &serde_json::json!({
                "agreement_action_hash": agreement_hash,
                "attesting_collective_cid": input.attesting_collective_cid,
            }))
            .await?;
        self.fetch_agreement_by_action(agreement_hash).await
    }

    pub async fn withdraw_membership(
        &self,
        input: WithdrawMembershipInputView,
    ) -> Result<MembershipView, StorageError> {
        let membership_hash = decode_action_hash_from_cid(&input.membership_cid, "membership:")?;
        self.zome
            .call("imagodei", "withdraw_membership_clean", &serde_json::json!({
                "membership_action_hash": membership_hash,
                "collab_qahal_cid": input.collab_qahal_cid,
            }))
            .await?;
        self.fetch_membership_by_action(membership_hash).await
    }

    pub async fn fetch_collective(&self, cid: &str) -> Result<CollectiveView, StorageError> {
        let action_hash = decode_action_hash_from_cid(cid, "collective:")?;
        self.fetch_collective_by_action(action_hash).await
    }

    pub async fn fetch_collab_qahal(&self, cid: &str) -> Result<CollabQahalView, StorageError> {
        // 1. Fetch the Collective entry (the Collab-Qahal IS a Collective record).
        let collective = self.fetch_collective(cid).await?;

        // 2. Fetch its memberships; partition by member_kind.
        let memberships: Vec<MembershipView> = self
            .zome
            .call("imagodei", "list_memberships_for_collective_cid", &cid.to_string())
            .await?;

        let mut member_collectives = Vec::new();
        let mut member_persons = Vec::new();
        for m in &memberships {
            if m.withdrawn_at_block_height.is_some() {
                continue; // skip cleanly-withdrawn members
            }
            match m.member_kind {
                MemberKind::Collective => {
                    let sub_collective = self.fetch_collective(&m.member_cid).await?;
                    member_collectives.push(sub_collective);
                }
                MemberKind::Person => member_persons.push(m.member_cid.clone()),
                MemberKind::ElohimAgent => { /* M1 ignores in projection */ }
            }
        }

        Ok(CollabQahalView {
            cid: collective.cid,
            anchor_agreement_cid: collective.anchor_agreement_cid.unwrap_or_default(),
            display_name: collective.display_name,
            created_at_block_height: collective.created_at_block_height,
            elohim_tier: ElohimTier::T0, // hard-coded for M1; lifts when friction-gradient evaluator lands
            member_collectives,
            member_persons,
            commons_pool_balance: 0.0, // populated by share-routing accumulator in Task 13
        })
    }

    async fn fetch_collective_by_action(
        &self,
        action_hash: holochain_types::dna::ActionHash,
    ) -> Result<CollectiveView, StorageError> {
        let record: holochain_types::record::Record = self
            .zome
            .call("imagodei", "get_collective_by_action", &action_hash)
            .await?;
        self.collective_record_to_view(&record)
    }

    async fn fetch_agreement_by_action(
        &self,
        _action_hash: holochain_types::dna::ActionHash,
    ) -> Result<CollabAgreementView, StorageError> {
        // Implementation note: a companion zome function get_agreement_by_action +
        // get_collab_status + get_collab_qahal_cid_for_agreement compose into this
        // view. See coordinator additions in Task 5 for the underlying functions.
        // The body collects: agreement entry; status string; attested_by tag list
        // from AgreementOnCollab links; collab_qahal_cid if instantiated.
        unimplemented!("expand inline using coordinator helpers from Task 5")
    }

    async fn fetch_membership_by_action(
        &self,
        _action_hash: holochain_types::dna::ActionHash,
    ) -> Result<MembershipView, StorageError> {
        // Mirrors fetch_collective_by_action but for Membership entries.
        unimplemented!("expand inline using get_membership_by_action from Task 6")
    }

    fn collective_record_to_view(
        &self,
        _record: &holochain_types::record::Record,
    ) -> Result<CollectiveView, StorageError> {
        // Decode entry, populate view, default elohim_tier = T0.
        unimplemented!("decode via record.entry().to_app_option::<Collective>()?")
    }
}

fn decode_action_hash_from_cid(
    cid: &str,
    expected_prefix: &str,
) -> Result<holochain_types::dna::ActionHash, StorageError> {
    let raw = cid
        .strip_prefix(expected_prefix)
        .ok_or_else(|| StorageError::BadRequest(format!("expected prefix '{}'", expected_prefix)))?;
    holochain_types::dna::ActionHash::try_from(raw.to_string())
        .map_err(|_| StorageError::BadRequest(format!("invalid ActionHash in CID: {}", cid)))
}
```

(Note: the `unimplemented!()` calls flag implementer work; expand inline using the underlying zome functions from Tasks 4–6. The skeleton above gives the orchestration shape.)

- [ ] **Step 2: Register service in mod.rs**

In `elohim/elohim-storage/src/services/mod.rs`, add:

```rust
pub mod qahal_service;
```

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo build 2>&1 | tail -15
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/services/qahal_service.rs \
        elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(qahal-service): orchestrate Collective+Agreement zome calls

Service-layer wraps zome calls + projection caching for Collective creation,
Agreement creation, attestation, withdrawal, and Collab-Qahal projection.
The HTTP routes in next task are thin shells over this service.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: Register HTTP routes

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Add route handlers**

In `elohim/elohim-storage/src/http.rs`, find the route-registration block (around line 9280). Add new routes alongside `/db/content/{id}`:

```rust
// Qahal routes — per spec 2026-05-23-multi-collective-collaboration-epr-design
Route::new("/api/v1/collective", Method::POST)
    .auth_required()
    .handler(Box::new(|s, req| Box::pin(s.handle_create_collective(req)))),
Route::new("/api/v1/collective/:cid", Method::GET)
    .handler(Box::new(|s, req| Box::pin(s.handle_get_collective(req)))),
Route::new("/api/v1/collab/agreement", Method::POST)
    .auth_required()
    .handler(Box::new(|s, req| Box::pin(s.handle_create_collab_agreement(req)))),
Route::new("/api/v1/collab/agreement/:cid/attest", Method::POST)
    .auth_required()
    .handler(Box::new(|s, req| Box::pin(s.handle_attest_collab_agreement(req)))),
Route::new("/api/v1/collab/:cid", Method::GET)
    .handler(Box::new(|s, req| Box::pin(s.handle_get_collab(req)))),
Route::new("/api/v1/collab/:cid/withdraw", Method::POST)
    .auth_required()
    .handler(Box::new(|s, req| Box::pin(s.handle_withdraw_membership(req)))),
```

(Note: the exact `Route::new` API may differ — mirror the pattern of existing routes in the same file. The route registry pattern at line 9280 is the canonical example.)

- [ ] **Step 2: Add handler methods**

Add the corresponding `async fn handle_*` methods following the existing patterns (e.g., `handle_db_content_by_id` at line 3554 for shape reference):

```rust
async fn handle_create_collective(
    &self,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body = read_body(req).await?;
    let input: CreateCollectiveInputView = serde_json::from_slice(&body)?;
    let view = self.services.qahal.create_collective(input).await?;
    json_response(StatusCode::CREATED, &view)
}

async fn handle_get_collective(
    &self,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let cid = extract_path_param(&req, ":cid")?;
    let view = self.services.qahal.fetch_collective(&cid).await?;
    json_response(StatusCode::OK, &view)
}

async fn handle_create_collab_agreement(
    &self,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body = read_body(req).await?;
    let input: CreateCollabAgreementInputView = serde_json::from_slice(&body)?;
    let view = self.services.qahal.create_collab_agreement(input).await?;
    json_response(StatusCode::CREATED, &view)
}

async fn handle_attest_collab_agreement(
    &self,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body = read_body(req).await?;
    let input: AttestCollabAgreementInputView = serde_json::from_slice(&body)?;
    let view = self.services.qahal.attest_collab_agreement(input).await?;
    json_response(StatusCode::OK, &view)
}

async fn handle_get_collab(
    &self,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let cid = extract_path_param(&req, ":cid")?;
    let view = self.services.qahal.fetch_collab_qahal(&cid).await?;
    json_response(StatusCode::OK, &view)
}

async fn handle_withdraw_membership(
    &self,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body = read_body(req).await?;
    let input: WithdrawMembershipInputView = serde_json::from_slice(&body)?;
    let view = self.services.qahal.withdraw_membership(input).await?;
    json_response(StatusCode::OK, &view)
}
```

- [ ] **Step 3: Wire QahalService into services struct**

Find where `self.services` is constructed (likely in the storage initializer); add a `qahal: Arc<QahalService>` field and initialize it alongside the existing services.

- [ ] **Step 4: Build**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo build 2>&1 | tail -15
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): qahal HTTP routes (collective, agreement, attest, collab, withdraw)

Six routes added per spec §7.4 (read paths) + spec §2/§5.1 (write paths).
Writes require auth; reads are public per existing pattern. Routes are
thin shells over QahalService.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 12: HTTP contract tests

**Files:**
- Create: `elohim/elohim-storage/tests/qahal_http_contract.rs`

- [ ] **Step 1: Write contract tests**

Write to `elohim/elohim-storage/tests/qahal_http_contract.rs`:

```rust
//! HTTP-level contract tests for qahal routes.
//! Validates auth gating, happy paths, and the substrate-deterministic refusal cases.

use crate::test_support::*;

#[tokio::test]
async fn create_collective_requires_auth() {
    let server = start_test_storage_server().await;
    let body = serde_json::json!({
        "charter": "Test",
        "displayName": "Test Collective",
        "salt": "00112233445566778899aabbccddeeff"
    });
    let resp = server.post_unauthenticated("/api/v1/collective", &body).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn create_collective_happy_path() {
    let server = start_test_storage_server().await;
    let body = serde_json::json!({
        "charter": "Test",
        "displayName": "Test Collective",
        "salt": "00112233445566778899aabbccddeeff"
    });
    let resp = server.post_with_admin_key("/api/v1/collective", &body).await;
    assert_eq!(resp.status(), 201);
    let view: CollectiveView = resp.json().await;
    assert_eq!(view.display_name, "Test Collective");
    assert_eq!(view.elohim_tier, ElohimTier::T0);
}

#[tokio::test]
async fn create_collab_agreement_refuses_zero_tribute() {
    let server = start_test_storage_server().await;
    let coll_a = server.create_collective("A", "00112233445566778899aabbccddeeff").await;
    let coll_b = server.create_collective("B", "11223344556677889900aabbccddeeff").await;

    let body = serde_json::json!({
        "participants": [coll_a.cid, coll_b.cid],
        "scope": "zero-tribute",
        "shareAllocation": {
            "form": "Declared",
            "shares": [
                {"collectiveCid": coll_a.cid, "share": 0.5},
                {"collectiveCid": coll_b.cid, "share": 0.5}
            ],
            "commonsPoolTribute": 0.0
        },
        "governanceTerms": {"exitTerms": "clean"},
        "initialTier": "T0",
        "displayNameForQahal": "ZeroTribute",
        "salt": "22334455667788990011aabbccddeeff"
    });
    let resp = server.post_with_admin_key("/api/v1/collab/agreement", &body).await;
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn create_collab_agreement_refuses_t1_initial() {
    let server = start_test_storage_server().await;
    let coll_a = server.create_collective("A", "00112233445566778899aabbccddeeff").await;
    let coll_b = server.create_collective("B", "11223344556677889900aabbccddeeff").await;

    let body = serde_json::json!({
        "participants": [coll_a.cid, coll_b.cid],
        "scope": "T1-attempt",
        "shareAllocation": {
            "form": "Declared",
            "shares": [
                {"collectiveCid": coll_a.cid, "share": 0.475},
                {"collectiveCid": coll_b.cid, "share": 0.475}
            ],
            "commonsPoolTribute": 0.05
        },
        "governanceTerms": {"exitTerms": "clean"},
        "initialTier": "T1",
        "displayNameForQahal": "T1Attempt",
        "salt": "33445566778899001122aabbccddeeff"
    });
    let resp = server.post_with_admin_key("/api/v1/collab/agreement", &body).await;
    // M1 substrate refuses T1+ initial tier (commons-elohim path lands in M3).
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn get_collab_returns_holonic_structure() {
    let server = start_test_storage_server().await;
    let qahal_cid = server.build_t0_collab_qahal_two_collectives().await;
    let resp = server.get(&format!("/api/v1/collab/{}", qahal_cid)).await;
    assert_eq!(resp.status(), 200);
    let view: CollabQahalView = resp.json().await;
    assert_eq!(view.elohim_tier, ElohimTier::T0);
    assert_eq!(view.member_collectives.len(), 2);
    assert_eq!(view.commons_pool_balance, 0.0); // no EconomicEvents yet
}

#[tokio::test]
async fn reach_inflation_via_collab_is_refused() {
    let server = start_test_storage_server().await;
    let qahal_cid = server.build_t0_collab_qahal_two_collectives().await;
    // Attempt to publish a Content-EPR at public reach under a T0 Collab.
    // Substrate refuses via reach-tier mismatch — T0 ceiling is `familiar`.
    let publish_body = serde_json::json!({
        "scope_collab_cid": qahal_cid,
        "content_body": "test",
        "requested_reach": "public"
    });
    let resp = server.post_with_admin_key("/api/v1/epr/content", &publish_body).await;
    assert_eq!(resp.status(), 403);
    let denial: serde_json::Value = resp.json().await;
    assert_eq!(
        denial["extensions"]["elohimAuthorityDenial"]["reason"],
        "TierMismatch"
    );
}
```

(Note: `test_support` module — `start_test_storage_server`, `create_collective`, `build_t0_collab_qahal_two_collectives` — should mirror existing test helpers in `elohim/elohim-storage/tests/`. Inline the helpers if no shared module exists yet.)

- [ ] **Step 2: Run the tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test qahal_http_contract 2>&1 | tail -20
```

Expected: PASS (or the reach-inflation test may require Task 14 — see plan progression).

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/tests/qahal_http_contract.rs
git commit -m "test(qahal-http): contract tests for routes + auth + substrate refusals

Five contract tests: auth required on writes, happy create, zero-tribute
refusal, T1-initial refusal, holonic GET structure, reach-inflation refusal.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase D — REA share-routing

### Task 13: Implement Form A share-routing evaluator

**Files:**
- Create: `elohim/elohim-storage/src/services/share_routing.rs`
- Create: `elohim/elohim-storage/src/services/share_routing_tests.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Write failing tests first**

Write to `elohim/elohim-storage/src/services/share_routing_tests.rs`:

```rust
//! Unit tests for share-routing function (Form A: Declared).
//! Per spec §6.1.

use crate::services::share_routing::*;
use elohim_views::qahal::*;

#[test]
fn declared_shares_distribute_proportionally() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.4 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.4 },
            DeclaredShare { collective_cid: "collective:c".into(), share: 0.15 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.05,
    };
    let event_value = 1000.0;
    let routed = evaluate_share_routing(&allocation, event_value, 0).unwrap();
    let lookup: std::collections::HashMap<_, _> =
        routed.iter().map(|r| (r.collective_cid.clone(), r.amount)).collect();
    assert!((lookup["collective:a"] - 400.0).abs() < 0.01);
    assert!((lookup["collective:b"] - 400.0).abs() < 0.01);
    assert!((lookup["collective:c"] - 150.0).abs() < 0.01);
    assert!((lookup["commons-pool"] - 50.0).abs() < 0.01);
    let total: f64 = routed.iter().map(|r| r.amount).sum();
    assert!((total - 1000.0).abs() < 0.01);
}

#[test]
fn form_b_not_yet_supported() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::AffinityDerived,
        shares: None,
        affinity_window_blocks: Some(1000),
        rebalance_cadence_blocks: Some(100),
        commons_pool_tribute: 0.05,
    };
    let result = evaluate_share_routing(&allocation, 1000.0, 0);
    assert!(result.is_err(), "M1 only supports Declared");
}

#[test]
fn zero_tribute_refused() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.5 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.5 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.0,
    };
    let result = evaluate_share_routing(&allocation, 1000.0, 0);
    assert!(result.is_err(), "zero tribute refused");
}

#[test]
fn shares_must_sum_to_one_with_tribute() {
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.3 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.3 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.05, // sum = 0.65, not 1.0
    };
    let result = evaluate_share_routing(&allocation, 1000.0, 0);
    assert!(result.is_err(), "shares + tribute must sum to 1.0");
}

#[test]
fn withdrawn_member_does_not_accrue() {
    // After clean exit at block 100, EconomicEvents at block > 100 don't accrue to A.
    // For M1 the share-routing function takes an EXPLICIT list of active members from
    // the service layer, so this is tested at the service level (Task 14). Here we
    // just confirm that the function ignores entries marked inactive.
    let allocation = ShareAllocation {
        form: ShareAllocationForm::Declared,
        shares: Some(vec![
            DeclaredShare { collective_cid: "collective:a".into(), share: 0.5 },
            DeclaredShare { collective_cid: "collective:b".into(), share: 0.45 },
        ]),
        affinity_window_blocks: None,
        rebalance_cadence_blocks: None,
        commons_pool_tribute: 0.05,
    };
    let active_set: std::collections::HashSet<String> = vec!["collective:b".into()].into_iter().collect();
    let routed = evaluate_share_routing_active_only(&allocation, 1000.0, 0, &active_set).unwrap();
    let lookup: std::collections::HashMap<_, _> =
        routed.iter().map(|r| (r.collective_cid.clone(), r.amount)).collect();
    assert!(lookup.get("collective:a").is_none(), "A withdrew; no accrual");
    // B's relative share + commons-pool re-normalized over the remaining 0.45 + 0.05 fraction.
    // For M1 we DO NOT re-normalize after withdrawal — the unspent share flows entirely
    // to the commons pool of the Collab. This makes the substrate behavior predictable
    // and prevents oscillation around withdrawal events.
    let expected_b = 1000.0 * 0.45;
    let expected_commons = 1000.0 * (0.50 + 0.05); // A's 0.50 + base tribute 0.05
    assert!((lookup["collective:b"] - expected_b).abs() < 0.01);
    assert!((lookup["commons-pool"] - expected_commons).abs() < 0.01);
}
```

- [ ] **Step 2: Write the share-routing module**

Write to `elohim/elohim-storage/src/services/share_routing.rs`:

```rust
//! Share-routing evaluator. Pure function over ShareAllocation + event value.
//!
//! Per spec §6.1 (Form A only for M1). Form B affinity-derived deferred to M2.

use crate::error::StorageError;
use elohim_views::qahal::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct RoutedAmount {
    pub collective_cid: String,
    pub amount: f64,
}

pub fn evaluate_share_routing(
    allocation: &ShareAllocation,
    event_value: f64,
    _at_block_height: u64,
) -> Result<Vec<RoutedAmount>, StorageError> {
    validate_allocation_invariants(allocation)?;
    let active_set: HashSet<String> = allocation
        .shares
        .as_ref()
        .ok_or_else(|| StorageError::BadRequest("Declared form requires shares[]".into()))?
        .iter()
        .map(|s| s.collective_cid.clone())
        .collect();
    evaluate_share_routing_active_only(allocation, event_value, _at_block_height, &active_set)
}

pub fn evaluate_share_routing_active_only(
    allocation: &ShareAllocation,
    event_value: f64,
    _at_block_height: u64,
    active_set: &HashSet<String>,
) -> Result<Vec<RoutedAmount>, StorageError> {
    validate_allocation_invariants(allocation)?;
    let shares = allocation
        .shares
        .as_ref()
        .ok_or_else(|| StorageError::BadRequest("Declared form requires shares[]".into()))?;

    let mut routed = Vec::new();
    let mut commons_amount = event_value * allocation.commons_pool_tribute;
    for share in shares {
        if active_set.contains(&share.collective_cid) {
            routed.push(RoutedAmount {
                collective_cid: share.collective_cid.clone(),
                amount: event_value * share.share,
            });
        } else {
            // Withdrawn member's share flows entirely to the commons pool (per design note in Task 13 test).
            commons_amount += event_value * share.share;
        }
    }
    routed.push(RoutedAmount {
        collective_cid: "commons-pool".into(),
        amount: commons_amount,
    });
    Ok(routed)
}

fn validate_allocation_invariants(allocation: &ShareAllocation) -> Result<(), StorageError> {
    if !matches!(allocation.form, ShareAllocationForm::Declared) {
        return Err(StorageError::BadRequest(
            "M1 only supports ShareAllocationForm::Declared".into(),
        ));
    }
    if allocation.commons_pool_tribute <= 0.0 {
        return Err(StorageError::BadRequest(
            "commons_pool_tribute must be > 0 (substrate refuses zero tribute)".into(),
        ));
    }
    if allocation.commons_pool_tribute > 1.0 {
        return Err(StorageError::BadRequest(
            "commons_pool_tribute must be <= 1.0".into(),
        ));
    }
    let shares = allocation
        .shares
        .as_ref()
        .ok_or_else(|| StorageError::BadRequest("Declared form requires shares[]".into()))?;
    let share_sum: f64 = shares.iter().map(|s| s.share).sum();
    if (share_sum + allocation.commons_pool_tribute - 1.0).abs() > 1e-6 {
        return Err(StorageError::BadRequest(format!(
            "shares ({}) + commons_pool_tribute ({}) must sum to 1.0",
            share_sum, allocation.commons_pool_tribute
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("share_routing_tests.rs");
}
```

- [ ] **Step 3: Register module**

In `elohim/elohim-storage/src/services/mod.rs`, add:

```rust
pub mod share_routing;
```

- [ ] **Step 4: Run the tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test share_routing 2>&1 | tail -20
```

Expected: 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/services/share_routing.rs \
        elohim/elohim-storage/src/services/share_routing_tests.rs \
        elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(qahal-share-routing): Form A declared share-routing evaluator + 5 tests

Pure-function evaluator: proportional shares + non-zero commons-pool tribute.
Withdrawn members' shares flow to commons-pool (no re-normalization — prevents
oscillation around withdrawal events). Form B (AffinityDerived) refused with
\"M1 only supports Declared\".

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 14: Hook share-routing into EconomicEvent emission

**Files:**
- Modify: `elohim/elohim-storage/src/services/economic_event_service.rs`

- [ ] **Step 1: Inspect existing EconomicEvent service**

```bash
grep -n "pub async fn\|emit_event\|EconomicEvent" /projects/elohim/elohim/elohim-storage/src/services/economic_event_service.rs | head -20
```

Read the existing service to understand the EconomicEvent emission shape.

- [ ] **Step 2: Add collab-scope detection + share-routing application**

Modify `emit_economic_event` (or equivalent emission function) to detect when a Content-EPR has `scope_collab_cid` set, fetch the Collab's CollabAgreement, evaluate share-routing on the event value, and emit one Settlement EconomicEvent per routed amount:

```rust
// Inside emit_economic_event (or equivalent):
if let Some(collab_cid) = source_content.scope_collab_cid {
    let collab = self.qahal.fetch_collab_qahal(&collab_cid).await?;
    let agreement = self.qahal.fetch_agreement_for_collab(&collab_cid).await?;
    let active_collectives: std::collections::HashSet<String> = collab
        .member_collectives
        .iter()
        .map(|c| c.cid.clone())
        .collect();
    let routed = crate::services::share_routing::evaluate_share_routing_active_only(
        &agreement.share_allocation,
        event.value,
        event.at_block_height,
        &active_collectives,
    )?;
    // Emit one Settlement EconomicEvent per routed amount, with extensions.elohim
    // annotations: { allocatingAgreement: agreement.cid, commonsPoolTribute: routed.amount }.
    for amount in routed {
        self.emit_settlement_event(amount, &agreement).await?;
    }
}
```

(Note: the precise integration depends on the existing emission patterns. Mirror them.)

- [ ] **Step 3: Add integration test**

In `elohim/elohim-storage/tests/qahal_http_contract.rs`, add:

```rust
#[tokio::test]
async fn economic_event_under_collab_routes_through_share_allocation() {
    let server = start_test_storage_server().await;
    let qahal_cid = server.build_t0_collab_qahal_two_collectives().await;

    // Emit an EconomicEvent with value 1000 under the Collab's scope.
    let body = serde_json::json!({
        "scope_collab_cid": qahal_cid,
        "value": 1000.0,
        "kind": "Settlement",
        "source_content_cid": "content:fake"
    });
    let resp = server.post_with_admin_key("/api/v1/economic-event", &body).await;
    assert_eq!(resp.status(), 201);

    // Verify the emitted Settlement events.
    let events = server.list_settlements_for_collab(&qahal_cid).await;
    let by_collective: std::collections::HashMap<_, _> =
        events.iter().map(|e| (e.beneficiary_cid.clone(), e.value)).collect();
    let total: f64 = events.iter().map(|e| e.value).sum();
    assert!((total - 1000.0).abs() < 0.01);
    assert!(by_collective.get("commons-pool").is_some(), "commons-pool tribute emitted");
}
```

- [ ] **Step 4: Run tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test --test qahal_http_contract economic_event 2>&1 | tail -15
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/services/economic_event_service.rs \
        elohim/elohim-storage/tests/qahal_http_contract.rs
git commit -m "feat(qahal-rea): hook share-routing into EconomicEvent emission

EconomicEvents under a Collab's scope route through the Agreement's share
allocation, emitting one Settlement event per routed amount including the
commons-pool tribute. End-to-end T0 REA flow lights up.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase E — VF projection at T0

### Task 15: Extend hREA bridge mapping for Collective→Organization with Collective-typed members

**Files:**
- Modify: `bridges/valueflows/valueflows-bridge/src/translate/agent.rs` (or equivalent — confirm path; create if Wave 3 hasn't landed the file yet)

- [ ] **Step 1: Add Collective→Organization mapping with member_kind extension**

Add to the agent translation module:

```rust
/// Map a substrate Collective to an hREA Organization, with elohim extension fields.
pub fn collective_to_organization(c: &CollectiveView, extensions_opted_in: bool) -> serde_json::Value {
    let mut org = serde_json::json!({
        "id": c.cid,
        "name": c.display_name,
        "note": c.charter, // see Wave 3 mapping rule for charter on note
    });
    if extensions_opted_in {
        org["extensions"] = serde_json::json!({
            "elohim": {
                "tier": c.elohim_tier,
                "anchorAgreementCid": c.anchor_agreement_cid,
            }
        });
    }
    org
}

/// Map a substrate Membership to an hREA AgentRelationship with member_kind extension.
pub fn membership_to_agent_relationship(
    m: &MembershipView,
    extensions_opted_in: bool,
) -> serde_json::Value {
    let mut rel = serde_json::json!({
        "id": m.cid,
        "object": m.collective_cid,
        "subject": m.member_cid,
        "relationship": match m.role {
            MembershipRole::Steward => "steward",
            MembershipRole::Contributor => "contributor",
            MembershipRole::Observer => "observer",
        },
    });
    if extensions_opted_in {
        rel["extensions"] = serde_json::json!({
            "elohim": {
                "memberKind": m.member_kind,
                "joinedAtBlockHeight": m.joined_at_block_height,
                "withdrawnAtBlockHeight": m.withdrawn_at_block_height,
            }
        });
    }
    rel
}
```

- [ ] **Step 2: Add learning-ledger TranslationPoint entries**

Per Wave 3 §4.2, emit a TranslationPoint for each translation:

```rust
ledger.record(TranslationPoint {
    at_block_height: c.created_at_block_height,
    direction: Direction::Read,
    vf_type: "Organization".into(),
    elohim_source: "elohim::Collective".into(),
    translation_kind: TranslationKind::SemanticBridge,
    semantic_cost: SemanticCost::JustifiedDistinct,
    ontological_commitment: Some(OntologicalCommitment::SovereigntyToStewardship),
    client_capability: if extensions_opted_in { ClientCapability::ElohimAware } else { ClientCapability::StockVf },
    code_location: file!(),
    notes: None,
});
```

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/bridges/valueflows
cargo build 2>&1 | tail -10
```

Expected: PASS (or guidance that Wave 3 prerequisite work is incomplete — defer this task's commit until then).

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add bridges/valueflows/valueflows-bridge/src/translate/agent.rs
git commit -m "feat(vf-bridge): Collective→Organization projection with member_kind extension

Surfaces substrate Collective/Membership via hREA Organization/
AgentRelationship. Stock VF clients see clean VF shape; elohim-aware
clients see extensions.elohim.{tier,memberKind,anchorAgreementCid}.
Holonic recursion (Organization-to-Organization AgentRelationship) lands
without new VF types.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase F — Storage-client SDK methods

### Task 16: Add SDK convenience methods

**Files:**
- Create: `elohim/sdk/storage-client-ts/src/api/qahal.ts`

- [ ] **Step 1: Write SDK methods**

Write to `elohim/sdk/storage-client-ts/src/api/qahal.ts`:

```typescript
import { StorageClient } from "../client";
import type {
  CollectiveView,
  CollabAgreementView,
  CollabQahalView,
  MembershipView,
  CreateCollectiveInputView,
  CreateCollabAgreementInputView,
  AttestCollabAgreementInputView,
  WithdrawMembershipInputView,
} from "../generated";

export class QahalApi {
  constructor(private client: StorageClient) {}

  async createCollective(input: CreateCollectiveInputView): Promise<CollectiveView> {
    return this.client.postJson("/api/v1/collective", input);
  }

  async getCollective(cid: string): Promise<CollectiveView> {
    return this.client.getJson(`/api/v1/collective/${encodeURIComponent(cid)}`);
  }

  async createCollabAgreement(
    input: CreateCollabAgreementInputView,
  ): Promise<CollabAgreementView> {
    return this.client.postJson("/api/v1/collab/agreement", input);
  }

  async attestCollabAgreement(
    input: AttestCollabAgreementInputView,
  ): Promise<CollabAgreementView> {
    return this.client.postJson(
      `/api/v1/collab/agreement/${encodeURIComponent(input.agreementCid)}/attest`,
      input,
    );
  }

  async getCollab(cid: string): Promise<CollabQahalView> {
    return this.client.getJson(`/api/v1/collab/${encodeURIComponent(cid)}`);
  }

  async withdrawFromCollab(input: WithdrawMembershipInputView): Promise<MembershipView> {
    return this.client.postJson(
      `/api/v1/collab/${encodeURIComponent(input.collabQahalCid)}/withdraw`,
      input,
    );
  }
}
```

- [ ] **Step 2: Wire into the main client**

In `elohim/sdk/storage-client-ts/src/client.ts` (or `index.ts`), add a `qahal` accessor:

```typescript
import { QahalApi } from "./api/qahal";

export class StorageClient {
  // ... existing fields ...
  readonly qahal = new QahalApi(this);
}
```

- [ ] **Step 3: Build the SDK**

```bash
cd /projects/elohim
pnpm --filter @elohim/storage-client build 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add elohim/sdk/storage-client-ts/src/api/qahal.ts \
        elohim/sdk/storage-client-ts/src/client.ts
git commit -m "feat(storage-client): QahalApi SDK methods for Collective + Collab flows

Five methods covering the M1 route set: create/get Collective, create/attest
Agreement, get Collab, withdraw from Collab. Types regenerate from
elohim-views via cargo test export_bindings.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase G — End-to-end sweettest closing the loop

### Task 17: Multi-conductor sweettest for cross-Collective flow

**Files:**
- Modify: `elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs`

- [ ] **Step 1: Add a two-conductor sweettest**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn two_conductor_t0_collab_end_to_end() {
    let (mut conductors, dna) = setup_two_conductor_imagodei().await;

    // Conductor 0 founder creates Collective A.
    let coll_a: ActionHash = conductors[0].call(
        &conductors[0].cells()[0].zome("imagodei"),
        "create_collective",
        CreateCollectiveInput {
            charter: "Coll A".into(),
            display_name: "Coll A".into(),
            salt: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        },
    ).await;

    // Conductor 1 founder creates Collective B.
    let coll_b: ActionHash = conductors[1].call(
        &conductors[1].cells()[0].zome("imagodei"),
        "create_collective",
        CreateCollectiveInput {
            charter: "Coll B".into(),
            display_name: "Coll B".into(),
            salt: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        },
    ).await;

    // Exchange peer info + await consistency (per memory feedback_sweettest_cross_agent_consistency).
    SweetConductor::exchange_peer_info([&conductors[0], &conductors[1]]).await;
    await_consistency(60, [&conductors[0].cells()[0], &conductors[1].cells()[0]]).await.unwrap();

    // Conductor 0 authors the CollabAgreement naming both Collectives.
    let agreement: ActionHash = conductors[0].call(
        &conductors[0].cells()[0].zome("imagodei"),
        "create_collab_agreement",
        CreateCollabAgreementInput {
            participants: vec![
                format!("collective:{}", coll_a),
                format!("collective:{}", coll_b),
            ],
            scope: "Joint riparian stewardship".into(),
            share_allocation_json: serde_json::to_string(&serde_json::json!({
                "form": "declared",
                "shares": [
                    {"collective_cid": format!("collective:{}", coll_a), "share": 0.475},
                    {"collective_cid": format!("collective:{}", coll_b), "share": 0.475}
                ],
                "commons_pool_tribute": 0.05
            })).unwrap(),
            commons_pool_tribute: 0.05,
            governance_terms_json: r#"{"exit_terms":"clean"}"#.into(),
            initial_tier: "T0".into(),
            display_name_for_qahal: "Riparian Stewards".into(),
            salt: "ccccccccccccccccccccccccccccccc1".into(),
        },
    ).await;
    await_consistency(60, [&conductors[0].cells()[0], &conductors[1].cells()[0]]).await.unwrap();

    // Conductor 0's founder attests on behalf of Collective A.
    let _: () = conductors[0].call(
        &conductors[0].cells()[0].zome("imagodei"),
        "attest_collab_agreement",
        AttestCollabAgreementInput {
            agreement_action_hash: agreement.clone(),
            attesting_collective_cid: format!("collective:{}", coll_a),
        },
    ).await;

    // Conductor 1's founder attests on behalf of Collective B.
    let _: () = conductors[1].call(
        &conductors[1].cells()[0].zome("imagodei"),
        "attest_collab_agreement",
        AttestCollabAgreementInput {
            agreement_action_hash: agreement.clone(),
            attesting_collective_cid: format!("collective:{}", coll_b),
        },
    ).await;
    await_consistency(60, [&conductors[0].cells()[0], &conductors[1].cells()[0]]).await.unwrap();

    // Both conductors should now see the Collab-Qahal instantiated.
    for c in &conductors {
        let status: String = c.call(
            &c.cells()[0].zome("imagodei"),
            "get_collab_status",
            agreement.clone(),
        ).await;
        assert_eq!(status, "Instantiated", "all conductors see instantiation");
    }
}

async fn setup_two_conductor_imagodei() -> (Vec<SweetConductor>, SweetDnaFile) {
    let dna = SweetDnaFile::from_bundle(IMAGODEI_DNA.as_ref()).await.unwrap();
    let mut conductors = SweetConductorBatch::from_standard_config(2).await;
    for c in conductors.iter_mut() {
        c.setup_app("imagodei", &[dna.clone()]).await.unwrap();
    }
    (conductors.into_inner(), dna)
}
```

- [ ] **Step 2: Run the test**

```bash
cd /projects/elohim/elohim/holochain/tests/sweettest
cargo test --test qahal_collab_t0_test two_conductor 2>&1 | tail -20
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/tests/sweettest/tests/qahal_collab_t0_test.rs
git commit -m "test(qahal-sweettest): two-conductor end-to-end T0 Collab flow

Founder on conductor-0 creates Coll A; founder on conductor-1 creates Coll B;
agreement authored on conductor-0; both founders counter-attest from their
respective conductors; both conductors see the Collab-Qahal instantiated
after DHT consistency. Closes the M1 sweettest loop.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase H — Quality gates + CI

### Task 18: Run full quality gates locally

- [ ] **Step 1: Run all unit + integration tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo test 2>&1 | tail -30

cd /projects/elohim/elohim/holochain/tests/sweettest
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev \
  cargo test --test qahal_collab_t0_test 2>&1 | tail -20
```

Expected: all tests PASS.

- [ ] **Step 2: Run clippy**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo clippy --all-targets -- -D warnings 2>&1 | tail -15

cd /projects/elohim/elohim/holochain/dna/imagodei
cargo clippy --target wasm32-unknown-unknown -- -D warnings 2>&1 | tail -15
```

Expected: zero warnings.

- [ ] **Step 3: Run fmt check**

```bash
cd /projects/elohim
cargo fmt --check 2>&1 | tail -5
```

Expected: no diff.

- [ ] **Step 4: Run schema pre-push validation**

```bash
cd /projects/elohim
pnpm run schema:codegen:ts 2>&1 | tail -5
git status # should show no diff in elohim/sdk/storage-client-ts/src/generated/
```

Expected: codegen idempotent.

- [ ] **Step 5: Push to remote**

```bash
cd /projects/elohim
git push origin dev
```

The pre-push husky hook will run quality gates again at the level of the changed projects. Watch the output; if anything fails, investigate before retrying.

---

## Self-Review Checklist (for the agentic worker after completion)

After all tasks complete, verify against the spec:

| Spec section | Implemented in tasks | Verified by |
|---|---|---|
| §2.1 Polymorphic Membership | Task 1, 2 | Sweettest member_kind=Collective accepted |
| §2.2 Graph shape | Task 5 (atomic instantiation) | Sweettest verifies HasMembership links land |
| §2.3 What's invariant (T0) | All tasks | Implicit — no graduation logic in M1 |
| §3.1 T0 tier ceiling | Tasks 5 (refuse T1+ initial), 12 (refuse reach inflation) | qahal_http_contract.rs assertions |
| §5.1 Pre-declared root | Tasks 4–5 | create_collab_agreement + attest tests |
| §5.2 Emergent root | NOT IMPLEMENTED — M2 | Spec §11 sequencing |
| §6.1 Form A share allocation | Task 13 | share_routing_tests.rs |
| §6.3 Commons-pool tribute (non-zero) | Tasks 5, 12, 13 | Multiple refusal tests |
| §6.4 Clean exit | Task 6 | withdraw_membership_clean test |
| §6.4 Repair exit | NOT IMPLEMENTED — M3 | Spec §11 sequencing |
| §7.1 Mapping table | Task 15 | VF bridge tests |
| §7.2 Extension fields | Task 15 | extensions.elohim.* in responses |
| §7.3 Holonic queries | Task 15 (Collective→Organization recursion) | VF bridge tests |
| §8.1 Test class 1 (unit) | Tasks 1, 13 | All green |
| §8.1 Test class 2 (sweettest) | Tasks 4, 5, 6, 17 | All green |
| §8.1 Test class 4 (VF bridge) | Task 15 | If Wave 3 base lands |
| §8.1 Test class 6 (capture-attempts, substrate-deterministic) | Task 12 | zero-tribute + T1-initial + reach-inflation refusals |

**Acceptance for M1:** Tasks 1–18 complete, all tests green, no clippy warnings, sweettest two-conductor end-to-end passing, and the storage-client SDK can drive a full T0 Collab lifecycle (create Collectives → author Agreement → attest from each side → query Collab-Qahal → emit EconomicEvent → verify share routing → clean withdrawal).

---

## After M1: hand-off to M2

Once M1 lands and is green, the next plan picks up at:
- **M2 — Emergent root + Form B share allocation** (spec §5.2, §6.1 Form B)
- Requires: friction-gradient parameter substrate (spec §9.2) prerequisite

The M2 plan will be authored after M1 evidence accumulates (real Collab usage in dev/staging produces signal for which threshold defaults are honest).
