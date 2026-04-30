# EPR Phase 3.5 — Trust-Compute Gradient Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Light up the gradient signal Phase 3 wired with placeholders. Ship `FeedbackSignal` (4 variants) + `AttentionTending` + `CollectiveFilterPattern` EPR kinds; introduce edge-local sealed-against-self predecessor records; implement Primitive 2 (hop-by-hop back-prop) and Primitive 3 (gossip-flood); replace `Standing::evaluate_placeholder` with a cached projection table maintained on FeedbackSignal arrival; extend `standing-policy` and `tending-policy` manifest payloads with mandatory `floor` sub-objects encoding the §2.8 constitutional floors; ship the bootstrap default manifests; introduce a reusable cross-peer test harness and pass an end-to-end aunt-and-rage-bait integration test (also lifting Phase 3's `#[ignore]` on `cold_fetch_resolves_manifest_from_peer`).

**Architecture:** Schema-first IoC throughout. HDI validators are deterministic (no `get_links`); cross-entity authority gating lives in coordinator pre-commit gates. Three-layer truth model: DHT notarizes signals (FeedbackSignal, CollectiveFilterPattern, manifest payloads), libp2p carries them as data-ops, doorway projects them as web2 view. AttentionTending uses HDI `Visibility::Private` so it stays on the agent's source chain, signed and immutable, never gossiped. Standing remains a *derived view* per brainstorm §4.2 — the projection table is a local cache, recomputable from the FeedbackSignal subgraph at any time, and per-evaluator (each peer projects through *their* manifest subscriptions).

**Tech Stack:** Rust 1.x, Holochain HDK 0.5, libp2p 0.53, diesel 2.x with SQLite, tokio, dryoc 0.6 (pure-Rust libsodium for sealed-against-self crypto). Schemas authored as JSON Schema → Rust structs hand-written to match → `ts-rs` for TypeScript codegen.

**Source-of-truth references:**
- Architectural foundation: `genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md`
- Phase 3 plan (pattern + seams): `genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-manifest-resolver-plan.md`
- Phase 2B design (extended): `genesis/docs/superpowers/specs/2026-04-24-epr-phase-2b-design.md` §6.4

---

## P2P Design Gate Classifications

| Entity | Category | DHT? | Why |
|--------|----------|------|-----|
| `FeedbackSignal` (squelch / correction / retraction / quarantine) | A — notarized | yes (all 4 variants) | Squelch's "privacy" is in propagation behavior (no gossip-flood, only local effect + chain-walked back-prop); the atom itself is signed and DHT-published for accountability |
| `AttentionTending` | B — agent-scoped | no — `Visibility::Private` source chain only | Peer-private discernment per brainstorm §6.1; signed for non-repudiation; aggregator emits public summaries separately |
| `CollectiveFilterPattern` | A — notarized | yes | Public, k-anonymous summary published by aggregator after threshold met |
| Predecessor record | B — agent-scoped | no — local SQLite | Sealed-against-self at rest (2-of-2 mishpat-quorum + imagodei); local memory of "who I received from" |
| Standing view | C — operational | no — local projection | Per-evaluator projection through local manifest's debit weights; recomputable from FeedbackSignal subgraph |
| Tending aggregate (working set) | C — operational | no — local SQLite | Pre-aggregation buffer; emits CollectiveFilterPattern (Category A) once k-anonymity threshold met |

DNA entry-type budget: +3 in elohim DNA (`FeedbackSignal`, `AttentionTending`, `CollectiveFilterPattern`); ~73 → ~76. No new mishpat entry types — the constitutional floor lives in extended payloads of existing `Manifest` (manifest_kind=standing-policy or tending-policy).

---

## File Structure

### New files

| Path | Responsibility |
|------|----------------|
| `elohim/sdk/schemas/v1/feedback-signal.schema.json` | Wire schema for the FeedbackSignal EPR payload (4 variants) |
| `elohim/sdk/schemas/v1/attention-tending.schema.json` | Wire schema for AttentionTending payload |
| `elohim/sdk/schemas/v1/collective-filter-pattern.schema.json` | Wire schema for k-anonymous CollectiveFilterPattern emission |
| `elohim/sdk/schemas/v1/predecessor-record.schema.json` | Wire schema for sealed-against-self predecessor record |
| `elohim/sdk/schemas/v1/standing-policy-floor.schema.json` | Sub-schema referenced by `standing-policy` manifest payloads (5 standing-immune classes from §2.8) |
| `elohim/sdk/schemas/v1/tending-policy-floor.schema.json` | Sub-schema referenced by `tending-policy` manifest payloads (5 tending-immune classes from §2.8) |
| `elohim/elohim-storage/src/services/sealed_against_self.rs` | dryoc-based 2-of-2 sealed-box encrypt/decrypt (mishpat-quorum + imagodei) |
| `elohim/elohim-storage/src/services/back_prop.rs` | Primitive 2 — record predecessor on send, walk one hop on FeedbackSignal arrival |
| `elohim/elohim-storage/src/services/gossip_flood.rs` | Primitive 3 — broadcast FeedbackSignal on content's reach gossipsub topic |
| `elohim/elohim-storage/src/services/standing_projector.rs` | Per-evaluator projection: recompute on FeedbackSignal arrival; reads manifest debit weights |
| `elohim/elohim-storage/src/services/tending.rs` | TTL enforcement, re-tending events, expiry sweep |
| `elohim/elohim-storage/src/services/aggregator.rs` | k-anonymous tending aggregator with differential-privacy noise below threshold |
| `elohim/elohim-storage/src/services/standing_query.rs` | Author-side compose-time query API (cheap synchronous read for elohim tender) |
| `elohim/elohim-storage/src/db/predecessor_records.rs` | Diesel model + queries for predecessor_records table |
| `elohim/elohim-storage/src/db/standing_view.rs` | Diesel model + queries for standing_view table |
| `elohim/elohim-storage/src/db/tending.rs` | Diesel model + queries for attention_tending + tending_aggregate tables |
| `elohim/elohim-storage/migrations/<ts>_predecessor_records/{up,down}.sql` | predecessor_records table migration |
| `elohim/elohim-storage/migrations/<ts>_standing_view/{up,down}.sql` | standing_view + standing_view_evidence tables migration |
| `elohim/elohim-storage/migrations/<ts>_tending/{up,down}.sql` | attention_tending + tending_aggregate tables migration |
| `elohim/elohim-storage/tests/harness/multi_peer.rs` | Reusable cross-peer harness — multi-tokio runtime, real loopback swarms, EPR propagation primitives |
| `elohim/elohim-storage/tests/harness/mod.rs` | Module root for test harness |
| `elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs` | End-to-end three-peer scenario from brainstorm Appendix B |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` | FeedbackSignal integrity entry type + deterministic HDI validator |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attention_tending.rs` | AttentionTending integrity entry type, `Visibility::Private`, deterministic HDI validator |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/collective_filter_pattern.rs` | CollectiveFilterPattern integrity entry type + HDI validator |
| `elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs` | Coordinator: `create_feedback_signal`, `get_feedback_signals_for_target`, `list_feedback_signals_by_signer` |
| `elohim/holochain/dna/elohim/zomes/content_store/src/attention_tending.rs` | Coordinator: `create_attention_tending`, `refresh_tending_ttl`, `list_my_tending` |
| `elohim/holochain/dna/elohim/zomes/content_store/src/collective_filter_pattern.rs` | Coordinator: `publish_collective_pattern` (threshold-gated by aggregator) |
| `elohim/holochain/dna/elohim/sweettests/feedback_signal.rs` | Sweettest coverage for FeedbackSignal (per zome-sweettest-sync) |
| `elohim/holochain/dna/elohim/sweettests/attention_tending.rs` | Sweettest coverage for AttentionTending (private-entry visibility verified) |
| `elohim/holochain/dna/elohim/sweettests/collective_filter_pattern.rs` | Sweettest coverage for CollectiveFilterPattern |
| `elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json` | Bootstrap standing-policy manifest with floor sub-object (5 standing-immune classes) |
| `elohim/sdk/schemas/v1/manifests/bootstrap-tending-policy.json` | Bootstrap tending-policy manifest with floor sub-object (5 tending-immune classes) + TTL defaults |
| `elohim/elohim-storage/src/services/bootstrap_manifests.rs` | First-run seeder that loads bootstrap manifests if registry is empty |

### Modified files

| Path | Change |
|------|--------|
| `elohim/elohim-storage/Cargo.toml` | Add `dryoc = { version = "0.6", default-features = false, features = ["serde"] }` |
| `elohim/elohim-storage/src/services/mod.rs` | Re-export new modules |
| `elohim/elohim-storage/src/services/standing.rs` | Replace `evaluate_placeholder` with `evaluate(&self, evaluator: &AgentKey, subject: &AgentKey, conn: &mut SqliteConnection) -> Standing` reading from standing_view; bootstrap fallback when no manifest yet |
| `elohim/elohim-storage/src/services/manifest_registry.rs` | Add `floor_for_kind` methods extracting floor classes from standing-policy/tending-policy payloads |
| `elohim/elohim-storage/src/services/floor_protections.rs` | Add `is_un_filterable_class(kind: TendingClass) -> bool` and `is_standing_immune(class: StandingFloorClass) -> bool` reading from registry |
| `elohim/elohim-storage/src/api/epr.rs` | After FeedbackSignal ingestion: invoke back_prop walk + standing_projector update; PeerId threading for dedup |
| `elohim/elohim-storage/src/api/mod.rs` | Add new HTTP routes for StandingQuery (declared in app manifest per `project_doorway_manifest_driven_routes`) |
| `elohim/elohim-storage/src/p2p/mod.rs` | Wire gossip_flood handler into existing `/elohim/epr-atom/1.0.0` protocol |
| `elohim/elohim-storage/tests/manifest_resolver_integration.rs` | Lift `#[ignore]` on `cold_fetch_resolves_manifest_from_peer`; rewrite to use new harness |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` | Add three new entry types to `EntryTypes` enum + module declarations |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs` | Extend validator: when `manifest_kind` is `standing-policy` or `tending-policy`, payload must contain `floor` sub-object validating against the corresponding floor schema |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | Add module declarations + re-exports for new coordinator functions |
| `elohim/sdk/schemas/scripts/codegen-ts.mjs` | Add new view schemas to `INTERFACE_FILES` |
| `elohim/sdk/domains/lamad/manifest.json` | Register new EPR kinds in app vocabulary (if applicable) |

### Test fixtures

| Path | Content |
|------|---------|
| `elohim/elohim-storage/tests/vectors/feedback_signal_messages.json` | MessagePack-encoded golden vectors for round-trip testing (one per signal_kind) |
| `elohim/elohim-storage/tests/vectors/attention_tending_messages.json` | Golden vectors for tending kinds |
| `elohim/elohim-storage/tests/vectors/sealed_predecessor_blob.bin` | Reference encrypted predecessor blob (deterministic test keypairs) |

---

## Task 0: Worktree setup + branch

**Files:** none (repo state)

- [ ] **Step 1: Create the worktree off origin/dev**

```bash
cd /projects/elohim
git fetch origin dev
git worktree add /projects/elohim/.claude/worktrees/epr-phase-3-5 \
  -b feature/epr-phase-3-5-trust-compute-gradient origin/dev
cd /projects/elohim/.claude/worktrees/epr-phase-3-5
```

Expected: `Preparing worktree (new branch 'feature/epr-phase-3-5-trust-compute-gradient')`. The worktree is the working directory for all subsequent tasks.

- [ ] **Step 2: Verify the worktree is clean and at expected commit**

```bash
git status
git log --oneline -5
```

Expected: clean working tree; HEAD includes `b8dea7a2 fix(steward-node): clippy 1.95.0 lint drift` and the Phase 3 merge `fd7155b9`.

- [ ] **Step 3: Confirm Phase 3 seams are visible**

```bash
ls elohim/elohim-storage/src/services/standing.rs
ls elohim/elohim-storage/src/services/floor_protections.rs
ls elohim/elohim-storage/src/services/manifest_registry.rs
ls elohim/elohim-storage/src/services/schemaref_resolver.rs
ls elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs
```

Expected: all five files exist (Phase 3 deliverables).

- [ ] **Step 4: Confirm the brainstorm artifact and this plan are visible**

```bash
ls genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
ls genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md
```

- [ ] **Step 5: No commit needed — Task 0 is workspace setup only.**

---

## Task 1: FeedbackSignal wire schema + Rust mirror

**Files:**
- Create: `elohim/sdk/schemas/v1/feedback-signal.schema.json`
- Create: `elohim/elohim-storage/src/wire/feedback_signal.rs`
- Modify: `elohim/elohim-storage/src/wire/mod.rs`

**Acceptance:** Schema validates all 4 variant payloads; Rust struct round-trips through MessagePack matching the schema; `cargo test wire::feedback_signal` passes; `pnpm run schema:validate` reports zero errors.

- [ ] **Step 1: Author the JSON schema first** (per `feedback_schema_first_ioc`)

Schema shape (write to `elohim/sdk/schemas/v1/feedback-signal.schema.json`):

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "feedback-signal.schema.json",
  "title": "FeedbackSignal",
  "type": "object",
  "required": ["targetCid", "signalKind", "standingImpact", "signedBy", "signature"],
  "properties": {
    "targetCid": { "type": "string" },
    "signalKind": { "enum": ["squelch", "correction", "retraction", "quarantine"] },
    "evidenceCid": { "type": "string" },
    "standingImpact": { "enum": ["advisory", "debit-soft", "debit-firm"] },
    "signedBy": { "type": "string" },
    "signature": { "type": "string" }
  }
}
```

- [ ] **Step 2: Hand-write the Rust struct to match** with `#[serde(rename_all = "camelCase")]`, `#[derive(TS)]`, and an `EprKind::FeedbackSignal` variant in `elohim_epr::EprKind` (existing crate).

- [ ] **Step 3: Add tests** — golden-vector round-trip, schema validation, all 4 variants, missing required field rejected.

- [ ] **Step 4: Add to `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`** so TypeScript types regenerate.

- [ ] **Step 5: Run quality gates**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib wire::feedback_signal
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:codegen:ts
```

Expected: all pass.

- [ ] **Step 6: Commit**

```
feat(epr-3.5): T1 — FeedbackSignal wire schema + Rust mirror

Four signal variants: squelch / correction / retraction / quarantine.
Standing impact graduated: advisory / debit-soft / debit-firm.
Schema-first per feedback_schema_first_ioc; ts-rs codegen wired.
```

---

## Task 2: AttentionTending wire schema + Rust mirror

**Files:**
- Create: `elohim/sdk/schemas/v1/attention-tending.schema.json`
- Create: `elohim/elohim-storage/src/wire/attention_tending.rs`
- Modify: `elohim/elohim-storage/src/wire/mod.rs`

**Acceptance:** Schema enforces `classification ∈ {values-forward, fatigue, scope-mismatch, safety}`, TTL is a Duration encoded as ISO-8601, `tendedAt` is a non-empty array of timestamps; `cargo test wire::attention_tending` passes; round-trips through MessagePack.

- [ ] **Step 1: Author the schema** matching brainstorm §6.1 shape; `tendedAt: { type: "array", items: { type: "string", format: "date-time" }, minItems: 1 }`.

- [ ] **Step 2: Hand-write Rust struct** with `#[derive(TS)]`; add `EprKind::AttentionTending`.

- [ ] **Step 3: Tests** — round-trip, all 4 classifications, ttl serializes as ISO-8601 duration string.

- [ ] **Step 4: Add to codegen-ts.mjs INTERFACE_FILES.**

- [ ] **Step 5: Quality gates** (same as T1).

- [ ] **Step 6: Commit**

```
feat(epr-3.5): T2 — AttentionTending wire schema + Rust mirror

Four classifications per brainstorm §6.1; TTL-bounded with re-tending events.
Wire shape only; Visibility::Private integrity entry lands at T6.
```

---

## Task 3: Constitutional floor sub-schemas (Q1)

**Files:**
- Create: `elohim/sdk/schemas/v1/standing-policy-floor.schema.json`
- Create: `elohim/sdk/schemas/v1/tending-policy-floor.schema.json`
- Modify: `elohim/sdk/schemas/v1/manifest/manifest-epr.schema.json` — when `manifestKind=standing-policy`, payload MUST include `floor` matching `standing-policy-floor`; same for `tending-policy`.

**Acceptance:** Schemas define exactly the 10 floor classes from §2.8 (5 standing-immune + 5 tending-immune); `pnpm run schema:check-dna` passes (DNA constants match enum values); a fixture manifest payload missing the `floor` sub-object is rejected by validator.

- [ ] **Step 1: Author standing-policy-floor.schema.json** with the 5 classes from §2.8 standing-immune table:

```
local-relationship-reach | cid-targeted-lookup | constitutional-floor-signatures
| new-voice-baseline | vulnerable-class-elevation
```

Each class entry has `{ class: string, protection: string, applies_when?: string }`.

- [ ] **Step 2: Author tending-policy-floor.schema.json** with the 5 tending-immune classes:

```
accountability-information | community-facts | custodial-communications
| constitutional-updates | elohim-as-counsel-notifications
```

- [ ] **Step 3: Update manifest-epr.schema.json** with `oneOf` discriminator on `manifestKind` requiring the `floor` sub-object for the two policy kinds.

- [ ] **Step 4: Add fixtures** — valid manifest payload with floor; invalid (missing floor) — both must validate as expected.

- [ ] **Step 5: Tests + codegen**

```bash
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:check-dna
pnpm run schema:codegen:ts
pnpm run schema:codegen:rs
```

- [ ] **Step 6: Commit**

```
feat(epr-3.5): T3 — constitutional floor sub-schemas (§2.8 the ten classes)

standing-policy-floor: 5 classes (local-relationship-reach,
cid-targeted-lookup, constitutional-floor-signatures, new-voice-baseline,
vulnerable-class-elevation). tending-policy-floor: 5 classes
(accountability-information, community-facts, custodial-communications,
constitutional-updates, elohim-as-counsel-notifications).

Per Q1 design decision: floors live in extended payloads of existing
standing-policy/tending-policy manifest_kinds rather than introducing new
DHT entry types.
```

---

## Task 4: FeedbackSignal integrity entry + HDI validator

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Acceptance:** Validator is fully deterministic (no `get_links`, no DHT lookups); rejects unknown `signal_kind`; rejects mismatched `standing_impact` (e.g. squelch with debit-firm is invalid — squelch is always advisory); `cargo test feedback_signal` passes.

- [ ] **Step 1: Define `FeedbackSignal` entry struct** per `feedback_serde_json_value_breaks_zome_boundary` — pre-stringify with `payload_json: String`:

```rust
#[hdk_entry_helper]
#[derive(Clone)]
pub struct FeedbackSignal {
    pub target_cid: String,
    pub signal_kind: String,        // squelch / correction / retraction / quarantine
    pub evidence_cid: Option<String>,
    pub standing_impact: String,    // advisory / debit-soft / debit-firm
    pub signer_pubkey: Vec<u8>,
}
```

- [ ] **Step 2: Implement `validate()`** with deterministic floors:
  - signal_kind ∈ {squelch, correction, retraction, quarantine}
  - standing_impact ∈ {advisory, debit-soft, debit-firm}
  - squelch ⇒ standing_impact == advisory
  - correction ⇒ evidence_cid is Some
  - retraction ⇒ signer_pubkey == content's original signer (cross-entity rule **lives in coordinator**, not here per `project_hdi_no_get_links_in_validators`)

- [ ] **Step 3: Inline tests** for each accepted/rejected shape.

- [ ] **Step 4: Add to EntryTypes enum.**

- [ ] **Step 5: Build + test**

```bash
cd elohim/holochain/dna/elohim
just check
just build
```

- [ ] **Step 6: Commit**

```
feat(epr-3.5): T4 — FeedbackSignal integrity entry + deterministic validator

Phase 3.5 P3.5.1. Cross-entity rules (e.g. retraction signer == origin
author) deferred to coordinator pre-commit gate per
project_hdi_no_get_links_in_validators.
```

---

## Task 5: AttentionTending integrity entry (private visibility)

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attention_tending.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Acceptance:** Entry type registered with `Visibility::Private`; sweettest verifies the entry does NOT appear in DHT gossip (only in agent's source chain); validator rejects unknown classification, ttl < 1 hour, empty `tended_at`.

- [ ] **Step 1: Define entry struct** (pre-stringified payload):

```rust
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AttentionTending {
    pub filter_subject_json: String,   // JSON-encoded FilterSubject
    pub classification: String,         // values-forward / fatigue / scope-mismatch / safety
    pub reason: Option<String>,
    pub ttl_seconds: u64,
    pub tended_at: Vec<i64>,            // unix timestamps
    pub context_json: String,           // JSON-encoded ContextScope
    pub signer_pubkey: Vec<u8>,
}
```

- [ ] **Step 2: Register with `Visibility::Private`** in EntryTypes — this is the load-bearing flag.

- [ ] **Step 3: Validator** — classification whitelist; ttl ≥ 3600; tended_at non-empty.

- [ ] **Step 4: Tests** verify private visibility is set.

- [ ] **Step 5: Build + commit**

```
feat(epr-3.5): T5 — AttentionTending integrity entry (Visibility::Private)

Phase 3.5 P3.5.5. Source-chain entry only — never gossiped. Honors
brainstorm §6.1 'peer-private by default' while keeping the signed-atom
provenance for non-repudiation. Aggregator (T15) reads private chains and
emits CollectiveFilterPattern (T6) post-k-anonymity.
```

---

## Task 6: CollectiveFilterPattern integrity entry

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/collective_filter_pattern.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Acceptance:** `participating_pct` ∈ [0, 100]; emits with NO peer identities embedded; sweettest verifies no source-chain link from CollectiveFilterPattern to any AttentionTending entries.

- [ ] **Step 1: Define entry struct** matching brainstorm §6.4 shape (k-anonymous, no peer identities).

- [ ] **Step 2: Validator** — pct ∈ [0, 100]; trend ∈ {rising, stable, falling}; context_window ≥ 1 hour.

- [ ] **Step 3: Tests + build + commit.**

```
feat(epr-3.5): T6 — CollectiveFilterPattern integrity entry

Phase 3.5 P3.5.6 (DHT side). k-anonymous summary published by aggregator
post-threshold. Validator enforces 'no peer identities' shape.
```

---

## Task 7: Manifest validator extension — enforce floor sub-object

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs`

**Acceptance:** When `manifest_kind == "standing-policy"`, payload must contain `floor` object validating against the standing-policy-floor schema (parsed locally — schema is bundled at build time as a Rust constant); same for tending-policy. Existing 5 manifest_kinds otherwise unchanged.

- [ ] **Step 1: Bundle the floor schemas** as Rust string constants at build time (include_str!).

- [ ] **Step 2: Extend `validate()`** with kind-conditional floor check using `jsonschema` crate already in scope.

- [ ] **Step 3: Tests** — valid policy with floor passes; without floor fails; floor with unknown class fails.

- [ ] **Step 4: Build + commit.**

```
feat(epr-3.5): T7 — Manifest validator enforces floor sub-object

Phase 3.5 Q1 design decision lands. standing-policy and tending-policy
manifests now require a `floor` object matching the bundled JSON Schema.
Cross-entity authority (mishpat-DNA-notarization gate) remains in
coordinator pre-commit per project_hdi_no_get_links_in_validators.
```

---

## Task 8: FeedbackSignal coordinator + sweettest

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs`
- Create: `elohim/holochain/dna/elohim/sweettests/feedback_signal.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Acceptance:** `create_feedback_signal` enforces retraction-signer-equals-origin via `must_get_*` (deterministic; not links); `get_feedback_signals_for_target` walks links from target_cid; `list_feedback_signals_by_signer` uses author-anchor pattern. Sweettest covers all 4 variants.

- [ ] **Step 1: Coordinator functions** with cross-entity gates using `must_get_*` only (per HDI validator constraint, get_links is HDK-only — coordinators can use both).

- [ ] **Step 2: Sweettest** — create + read + verify links; all 4 variants; retraction-by-non-origin rejected.

- [ ] **Step 3: Build, run sweettest** (`pnpm run sweettest-check` or equivalent).

- [ ] **Step 4: Commit.**

```
feat(epr-3.5): T8 — FeedbackSignal coordinator + sweettest

Cross-entity gates (retraction signer == origin author) at coordinator
pre-commit. Sweettest covers all 4 variants per zome-sweettest-sync.
```

---

## Task 9: AttentionTending coordinator + sweettest

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store/src/attention_tending.rs`
- Create: `elohim/holochain/dna/elohim/sweettests/attention_tending.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Acceptance:** `create_attention_tending`, `refresh_tending_ttl` (appends to tended_at), `list_my_tending` (source chain query). Sweettest verifies private-entry visibility — a second agent in the sweettest swarm cannot retrieve another agent's AttentionTending entries.

- [ ] **Step 1: Coordinator functions** — refresh_ttl uses `update_entry`; list_my queries source chain only (no DHT).

- [ ] **Step 2: Sweettest with two agents** — agent A creates tending; agent B's `get` returns None (private visibility working).

- [ ] **Step 3: Commit.**

```
feat(epr-3.5): T9 — AttentionTending coordinator + sweettest

refresh_tending_ttl re-tending event; list_my_tending source-chain only.
Sweettest verifies cross-agent privacy (Visibility::Private working).
```

---

## Task 10: Predecessor records — diesel migration + model

**Files:**
- Create: `elohim/elohim-storage/migrations/<ts>_predecessor_records/up.sql`
- Create: `elohim/elohim-storage/migrations/<ts>_predecessor_records/down.sql`
- Create: `elohim/elohim-storage/src/db/predecessor_records.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`, `schema.rs` (regenerate)

**Acceptance:** Migration applies cleanly forward and backward; uniqueness constraint on `(target_cid, predecessor_peer_id)`; queries `insert_predecessor`, `get_predecessor_for_cid`, `delete_for_cid` all pass diesel test pool.

Per `feedback_diesel_migration_timestamp_collision`: explicitly verify the migration directory timestamp doesn't collide with any existing one before proceeding.

- [ ] **Step 1: Generate migration directory + verify uniqueness**

```bash
TIMESTAMP=$(date -u +%Y-%m-%d-%H%M%S)
mkdir "migrations/${TIMESTAMP}_predecessor_records"
ls migrations/ | sort   # Verify no collision
```

- [ ] **Step 2: up.sql** schema:

```sql
CREATE TABLE predecessor_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_cid TEXT NOT NULL,
    predecessor_peer_id TEXT NOT NULL,
    received_at TEXT NOT NULL,
    sealed_blob BLOB NOT NULL,
    UNIQUE(target_cid, predecessor_peer_id)
);
CREATE INDEX idx_predecessor_target ON predecessor_records(target_cid);
```

- [ ] **Step 3: down.sql** drops table + index.

- [ ] **Step 4: Diesel model** (`PredecessorRecordRow`) + queries.

- [ ] **Step 5: Tests** with test_pool fixture.

- [ ] **Step 6: Commit.**

```
feat(epr-3.5): T10 — predecessor_records table + diesel model

Phase 3.5 P3.5.2 storage layer. sealed_blob holds the dryoc-encrypted
2-of-2 record (T11 lights up the crypto). Per Q3 design decision:
durable disk persistence so peer-restart doesn't truncate back-prop chains.
```

---

## Task 11: Sealed-against-self crypto service (Q2)

**Files:**
- Create: `elohim/elohim-storage/src/services/sealed_against_self.rs`
- Modify: `elohim/elohim-storage/Cargo.toml` — add `dryoc = { version = "0.6", default-features = false, features = ["serde"] }`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** `seal(plaintext, mishpat_quorum_pk, imagodei_pk) -> SealedBlob` two-pass crypto_box_seal; `unseal(sealed, mishpat_quorum_sk, imagodei_sk) -> plaintext`; both keys required to decrypt; one key alone fails with explicit error; tests use deterministic test keypairs.

- [ ] **Step 1: Module shape**

```rust
//! Sealed-against-self — interim 2-of-2 encryption per brainstorm §10.1.
//!
//! Phase 3.5 ships a 2-of-2 (mishpat-quorum + subject's imagodei) sealed-box
//! using dryoc's crypto_box_seal (X25519 sealed-box). Phase 5/6 will replace
//! with t-of-n threshold scheme; the property (recovery requires governance
//! cooperation, never unilateral disclosure) is canonical here.

use dryoc::dryocbox::{DryocBox, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedBlob {
    pub mishpat_outer: Vec<u8>,   // ciphertext sealed to mishpat-quorum pk
    pub imagodei_inner: Vec<u8>,  // (encoded inside mishpat_outer's plaintext)
}

#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error("crypto failure: {0}")]
    Crypto(String),
    #[error("decryption requires both keys; got partial decrypt")]
    PartialDecrypt,
}

pub fn seal(
    plaintext: &[u8],
    mishpat_pk: &PublicKey,
    imagodei_pk: &PublicKey,
) -> Result<SealedBlob, SealError> { /* two crypto_box_seal calls, nested */ }

pub fn unseal(
    sealed: &SealedBlob,
    mishpat_sk: &SecretKey,
    imagodei_sk: &SecretKey,
) -> Result<Vec<u8>, SealError> { /* unwrap outer with mishpat_sk, then inner with imagodei_sk */ }
```

The 2-of-2 property is enforced by *nesting*: the outer ciphertext is only decryptable by mishpat-quorum; the inner plaintext (decryptable only by imagodei) lives inside that. Either key alone yields nothing useful.

- [ ] **Step 2: Tests**
  - round-trip with both keys → plaintext returned
  - mishpat_sk only → outer decrypts but inner is opaque ciphertext (test asserts the bytes are NOT the plaintext)
  - imagodei_sk only → outer fails, returns Err
  - tampered outer → fails
  - tampered inner → fails after outer decrypt
  - golden vector → byte-for-byte stability (load `tests/vectors/sealed_predecessor_blob.bin`)

- [ ] **Step 3: Quality gates**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib services::sealed_against_self
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
```

- [ ] **Step 4: Commit.**

```
feat(epr-3.5): T11 — sealed-against-self 2-of-2 crypto (dryoc)

Q2 design decision: pure-Rust libsodium (dryoc) sealed-box, nested. Recovery
requires cooperation of mishpat-quorum + subject's imagodei. Phase 5/6 will
replace with t-of-n threshold; this scheme makes the property testable
end-to-end today.
```

---

## Task 12: Back-prop service — Primitive 2

**Files:**
- Create: `elohim/elohim-storage/src/services/back_prop.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Modify: `elohim/elohim-storage/src/api/epr.rs` — wire `record_predecessor()` on send and `back_prop_one_hop()` on FeedbackSignal arrival

**Acceptance:** Sending a content EPR records the receiver's predecessor (the sender's PeerId); receiving a FeedbackSignal for content X looks up its predecessor and forwards the signal one hop back; signal does NOT contain a chain (chain is reconstructed hop-by-hop). Per-peer privacy preserved: each peer knows only its immediate predecessor.

- [ ] **Step 1: Service shape**

```rust
//! Primitive 2 — hop-by-hop back-prop walk per brainstorm §5.2.
//!
//! Each peer maintains a private predecessor map (sealed at rest). When a
//! FeedbackSignal arrives for content X, we forward the signal one hop back
//! to whoever sent us X. The chain reconstructs hop-by-hop, never on the wire.

pub fn record_predecessor(
    conn: &mut SqliteConnection,
    target_cid: &Cid,
    predecessor_peer_id: &PeerId,
    sealing: &SealingKeys,
) -> Result<(), Error> { /* seal + insert */ }

pub fn back_prop_one_hop(
    conn: &mut SqliteConnection,
    signal: &FeedbackSignal,
    swarm: &SwarmHandle,
) -> Result<Option<PeerId>, Error> {
    /* lookup predecessor for signal.target_cid; if found, send signal to that peer */
}
```

- [ ] **Step 2: Wire into EPR ingest path** in `api/epr.rs`:
  - On content EPR receive (cold-fetch or gossip), call `record_predecessor` with the sender PeerId already threaded through Phase 3
  - On FeedbackSignal receive, call `back_prop_one_hop`

- [ ] **Step 3: Tests** (in-process; cross-peer test in T19)
  - record_predecessor round-trips through seal/unseal
  - back_prop_one_hop with no predecessor → Ok(None)
  - back_prop_one_hop with predecessor → Ok(Some(peer_id)) and swarm send invoked (use mock)
  - duplicate predecessor entry → updated, not duplicated (uniqueness constraint)

- [ ] **Step 4: Commit.**

```
feat(epr-3.5): T12 — back-prop service (Primitive 2)

Phase 3.5 P3.5.3. Hop-by-hop walk via sealed predecessor map; each peer
knows only its immediate predecessor; chain reconstructs through local
memory of every participant in propagation. Trust-bubble bounded — walk
breaks at peer-offline / peer-out-of-relationship boundaries (humane
property per brainstorm §5.2).
```

---

## Task 13: Gossip-flood service — Primitive 3

**Files:**
- Create: `elohim/elohim-storage/src/services/gossip_flood.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — register handler on existing `/elohim/epr-atom/1.0.0`

**Acceptance:** Publishing a FeedbackSignal also broadcasts it on the *content's* reach gossipsub topic (so all current holders see the correction). Receiver-side dedup (don't re-process a signal we've already seen). Layered ON TOP OF Primitive 2 — does not replace it.

- [ ] **Step 1: Service shape** — single function `flood_feedback(signal, content_reach_topic, swarm)`.

- [ ] **Step 2: Handler registration** — gossipsub callback hands FeedbackSignal envelopes to the standing_projector (T14) and to back_prop's one-hop walk (T12).

- [ ] **Step 3: Dedup** — small LRU keyed on (signal_cid) per content; bounded; reset on restart is fine.

- [ ] **Step 4: Tests** — flood publishes; dedup keeps re-receives idempotent.

- [ ] **Step 5: Commit.**

```
feat(epr-3.5): T13 — gossip-flood service (Primitive 3)

Phase 3.5 P3.5.4. Layered on /elohim/epr-atom/1.0.0; reaches all current
holders of the content. Complement to Primitive 2: standing-impact walks
the chain (T12); epistemic notification floods to current holders (T13).
```

---

## Task 14: Standing view — projection table + projector service (Q4)

**Files:**
- Create: `elohim/elohim-storage/migrations/<ts>_standing_view/up.sql` + `down.sql`
- Create: `elohim/elohim-storage/src/db/standing_view.rs`
- Create: `elohim/elohim-storage/src/services/standing_projector.rs`
- Modify: `elohim/elohim-storage/src/services/standing.rs` — replace `evaluate_placeholder` with `evaluate(&self, ...)` reading from standing_view

**Acceptance:** On FeedbackSignal arrival, projector recomputes the affected subject's StandingScore through the local manifest's debit-weight rules and writes to standing_view; `Standing::evaluate(&evaluator, &subject, conn)` returns the projected score; absent any projection (cold-start / no FeedbackSignals yet) returns `Standing::Unknown` (NOT a stored score per §4.2 — the table is a derived view).

- [ ] **Step 1: Migration** for `standing_view`:

```sql
CREATE TABLE standing_view (
    evaluator_pubkey BLOB NOT NULL,
    subject_pubkey BLOB NOT NULL,
    score TEXT NOT NULL,           -- floor / low / neutral / high / trusted
    debit_weight_sum INTEGER NOT NULL DEFAULT 0,
    last_signal_at TEXT NOT NULL,
    manifest_cid TEXT NOT NULL,    -- which standing-policy manifest produced this projection
    PRIMARY KEY (evaluator_pubkey, subject_pubkey)
);
CREATE INDEX idx_standing_view_subject ON standing_view(subject_pubkey);
```

- [ ] **Step 2: Projector service** — on FeedbackSignal arrival:
  1. Look up the local agent's active standing-policy manifest (via ManifestRegistry from Phase 3)
  2. Apply the manifest's debit-weight for this signal_kind+standing_impact tuple
  3. Recompute StandingScore from running debit_weight_sum + new-voice baseline floor
  4. Upsert standing_view row

- [ ] **Step 3: Replace placeholder**

```rust
// in services/standing.rs
impl Standing {
    pub fn evaluate(
        evaluator: &[u8],
        subject: &[u8],
        conn: &mut SqliteConnection,
    ) -> Self {
        match db::standing_view::fetch(conn, evaluator, subject) {
            Ok(Some(row)) => Standing::Computed { score: row.score.into() },
            _ => Standing::Unknown,  // bootstrap: no projection yet
        }
    }
}
```

Phase 3's `evaluate_placeholder(_agent_pubkey)` is deprecated in favor of `evaluate(evaluator, subject, conn)`. Update all call sites that took `_agent_pubkey` to thread the evaluator pubkey + db conn.

- [ ] **Step 4: Tests**
  - Empty standing_view → Unknown
  - One squelch → projection writes neutral (squelch is advisory)
  - One debit-firm correction → projection writes low
  - Restitution correction by subject → projection rises
  - Different evaluators see different scores when subscribed to different manifests (pluralism property per §4.2)

- [ ] **Step 5: Commit.**

```
feat(epr-3.5): T14 — standing_view projection table + projector

Q4 design decision lands: standing is a per-evaluator derived view,
projected on FeedbackSignal arrival through the local manifest's debit
weights. NOT a stored authoritative score — table is recomputable from the
FeedbackSignal subgraph at any time. Different evaluators see different
views (pluralism per §4.2). Standing::evaluate replaces evaluate_placeholder
across all gradient-relevant code paths from Phase 3.
```

---

## Task 15: Tending lifecycle service

**Files:**
- Create: `elohim/elohim-storage/migrations/<ts>_tending/up.sql` + `down.sql`
- Create: `elohim/elohim-storage/src/db/tending.rs`
- Create: `elohim/elohim-storage/src/services/tending.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** TTL enforcement (expiry sweep deletes records past `tended_at + ttl`); re-tending appends to `tended_at` and resets the TTL clock; default TTLs from §6.6 (safety: ∞ encoded as i64::MAX, fatigue: 7d, values-forward: 30d, scope-mismatch: 90d).

- [ ] **Step 1: Migration** — `attention_tending` local table mirroring source-chain entries (cache for fast aggregator reads).

- [ ] **Step 2: Service** — `enforce_ttls(conn)`, `record_tending(conn, ...)`, `refresh(conn, id)`, `default_ttl(classification)`.

- [ ] **Step 3: Periodic sweep wired into the existing tokio reconciliation controller** per `principle_p1_reconciliation_controller`.

- [ ] **Step 4: Tests + commit.**

```
feat(epr-3.5): T15 — tending lifecycle (TTL + re-tending + sweep)

Phase 3.5 P3.5.5 dataplane. TTL defaults from brainstorm §6.6.
Re-tending extends; un-tended expires (mindless filter-everything is
structurally costly per §6.2).
```

---

## Task 16: k-anonymous tending aggregator

**Files:**
- Create: `elohim/elohim-storage/src/services/aggregator.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** Aggregator reads local `attention_tending` rows + (when network-mode) peer-attestations of similar shape; emits `CollectiveFilterPattern` only when participating count ≥ k=5 (default; tunable per manifest); below threshold, adds Laplacian differential-privacy noise OR suppresses emission entirely (manifest-declared mode); emitted patterns NEVER contain peer identities.

- [ ] **Step 1: Service shape**

```rust
pub fn aggregate_and_emit(
    conn: &mut SqliteConnection,
    collective: &CollectiveId,
    coordinator: &impl HcCoordinator,
    bootstrap_k: u8,  // default 5
) -> Result<Vec<CollectiveFilterPattern>, Error> { ... }
```

- [ ] **Step 2: k-anonymity check + DP noise**

  When aggregating across the local view (the local elohim doesn't see other peers' AttentionTending — those are private source-chain entries on other agents). For Phase 3.5 the aggregator emits patterns *for this local peer's collectives*, summarizing the local peer's own tending across categories. Cross-peer aggregation (privacy-preserving union sketch) is deferred to Phase 4+.

- [ ] **Step 3: Tests** — emission only above threshold; below-threshold mode follows manifest declaration.

- [ ] **Step 4: Commit.**

```
feat(epr-3.5): T16 — k-anonymous tending aggregator

Phase 3.5 P3.5.6. Local-peer aggregation across own AttentionTending
records emits CollectiveFilterPattern (T6) when k-threshold met. Below
threshold: DP noise OR suppression per manifest. Cross-peer
privacy-preserving union sketch is Phase 4+ work; this lights up the
single-peer summary primitive.
```

---

## Task 17: Bootstrap manifests — standing-policy + tending-policy

**Files:**
- Create: `elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json`
- Create: `elohim/sdk/schemas/v1/manifests/bootstrap-tending-policy.json`
- Create: `elohim/elohim-storage/src/services/bootstrap_manifests.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** Both bootstrap manifests validate against the manifest-epr schema with floor sub-objects per T3; first-run seeder loads them only if `manifest_registry.is_empty()`; idempotent on subsequent starts.

- [ ] **Step 1: bootstrap-standing-policy.json** — full §2.8 standing-immune floor + debit weights for the 4 FeedbackSignal kinds (squelch=advisory=0, correction=debit-soft=10, retraction=debit-soft=10, quarantine=debit-firm=30) + new-voice baseline (StandingScore::Floor with subject in protected-class lift).

- [ ] **Step 2: bootstrap-tending-policy.json** — §2.8 tending-immune floor + TTL defaults from §6.6 + k-anonymity threshold k=5.

- [ ] **Step 3: Seeder service** — on storage init, if registry empty, parse both JSON files, project them through `manifest_registry::project_manifest`, persist as Manifest EPRs (locally, not DHT-publishing — bootstrap manifests are seed-only; production deployments author their own and publish through normal Manifest EPR flow).

- [ ] **Step 4: Tests + commit.**

```
feat(epr-3.5): T17 — bootstrap default manifests (standing + tending policy)

Phase 3.5 P3.5.8. First-run seeder; idempotent. Communities fork-and-modify
these as the brainstorm §7.2 'starting point, not law' pattern.
```

---

## Task 18: Author-side compose-time query API

**Files:**
- Create: `elohim/elohim-storage/src/services/standing_query.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` — register HTTP route via app manifest declaration (per `project_doorway_manifest_driven_routes`)

**Acceptance:** `GET /api/v1/standing/compose-context?subject=<pubkey>` returns `{ authorStanding, fatigueSignals: [...], floorClasses: [...] }` in <50ms (p99) — drives the elohim tender's compose-time conversation. Read-only; never writes; uses the existing standing_view + tending tables.

- [ ] **Step 1: API shape** in `standing_query.rs`:

```rust
pub struct ComposeContext {
    pub author_standing: Standing,
    pub fatigue_signals: Vec<FatigueSignal>,
    pub floor_classes: Vec<StandingFloorClass>,
}

pub fn compose_context(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
    subject: &[u8],
) -> Result<ComposeContext, Error> { ... }
```

- [ ] **Step 2: HTTP route** declared in app manifest (per project_doorway_manifest_driven_routes); thin adapter calls compose_context.

- [ ] **Step 3: Performance test** — fixture with 1000 standing_view rows + 100 tending → query <50ms.

- [ ] **Step 4: Commit.**

```
feat(epr-3.5): T18 — author-side compose-time StandingQuery API

Phase 3.5 P3.5.9. Cheap synchronous read for elohim tender; <50ms p99.
HTTP route declared in app manifest per project_doorway_manifest_driven_routes;
read-only (no writes from this endpoint).
```

---

## Task 19: Cross-peer test harness primitive (Q5)

**Files:**
- Create: `elohim/elohim-storage/tests/harness/multi_peer.rs`
- Create: `elohim/elohim-storage/tests/harness/mod.rs`
- Modify: `elohim/elohim-storage/tests/manifest_resolver_integration.rs` — lift `#[ignore]` on `cold_fetch_resolves_manifest_from_peer`

**Acceptance:** Harness spins up N peers (each with own tokio runtime, own libp2p swarm on loopback, own SQLite); peers can publish and subscribe to gossipsub topics; harness API supports `peer.send(other, content)`, `peer.publish_signal(...)`, `peer.wait_for_message_count(n, timeout)`. Phase 3's previously-`#[ignore]`'d test passes on the new harness.

- [ ] **Step 1: Harness shape**

```rust
pub struct MultiPeerHarness {
    pub peers: Vec<TestPeer>,
}

pub struct TestPeer {
    pub peer_id: PeerId,
    pub agent_key: AgentKey,
    pub swarm: SwarmHandle,
    pub conn_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub runtime: tokio::runtime::Runtime,
}

impl MultiPeerHarness {
    pub async fn new(peer_count: usize) -> Self { ... }
    pub async fn connect_full_mesh(&mut self) { ... }
    pub async fn drive_until_idle(&mut self, timeout: Duration) { ... }
}
```

- [ ] **Step 2: Lift Phase 3 #[ignore]** — rewrite the test to use the harness. Verify it now passes end-to-end.

- [ ] **Step 3: Commit.**

```
feat(epr-3.5): T19 — cross-peer test harness + lift Phase 3 ignore

Q5 design decision lands. Multi-tokio loopback swarms; reusable for
future P2P tests (recovery M-series, defender, federation). Phase 3's
cold_fetch_resolves_manifest_from_peer #[ignore] lifted; passes on
the new harness.
```

---

## Task 20: Aunt-and-rage-bait integration test

**Files:**
- Create: `elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs`

**Acceptance:** Three-peer scenario from brainstorm Appendix B passes:
1. Bob (peer 0) authors content with reach=district, signed
2. Aunt (peer 1) subscribed to the district topic; receives Bob's content; re-shares to Aunt's family group
3. Sarah (peer 2) receives via Aunt's re-share; her elohim flags it; Sarah authors a Correction EPR + FeedbackSignal{kind=correction, target=Bob's CID, evidence=Sarah's correction}
4. Sarah's FeedbackSignal travels:
   - **Primitive 2 (back-prop)**: Sarah → Aunt (one hop); Aunt's standing_projector debits Aunt; Aunt's predecessor map says Bob → forward to Bob; Bob's standing debited at the constitutional-floor level (debit-firm, racism is in §2.8 un-filterable category)
   - **Primitive 3 (gossip-flood)**: published on the district topic; all current holders see the correction
5. Bob's next compose attempt at reach=district fails the reach-earning gate (his standing_view dropped below threshold)
6. Bob authors a Correction acknowledging Sarah; Sarah signs a Vouch; Bob's standing_view recovers (restitution path)

- [ ] **Step 1: Test scaffolding** — reuse harness from T19; build the manifest registry on each peer with the bootstrap-standing-policy from T17; seed Bob with new-voice baseline.

- [ ] **Step 2: Drive the scenario** step-by-step with explicit assertions at each phase boundary (post-publish, post-fanout, post-correction, post-back-prop, post-gossip-flood, post-restitution).

- [ ] **Step 3: Sealed-record decrypt assertion** — the predecessor record at Aunt's node, when read with both mishpat-quorum + Bob's imagodei test keypairs, decrypts to Bob's PeerId (test-only — production governance flow is brainstorm §B.7).

- [ ] **Step 4: Run with `--test-threads=1`** to avoid the env-var flakiness from `feedback_env_var_test_flakiness`.

- [ ] **Step 5: Commit.**

```
feat(epr-3.5): T20 — aunt-and-rage-bait end-to-end integration

Phase 3.5 P3.5.10 lands. Brainstorm Appendix B scenario passes
end-to-end: 3 peers, FeedbackSignal back-prop + gossip-flood, sealed
predecessor decrypt by 2-of-2 governance test keys, standing-view
debit + recovery via restitution path.
```

---

## Task 21: Quality gates + local merge

**Files:** none (workspace state)

**Acceptance:** All quality gates pass on the worktree; clean diff; merge to dev with `--no-ff`; no PR (per `feedback_dev_branch_no_pr`).

- [ ] **Step 1: Run the full quality gate sweep**

```bash
cd /projects/elohim/.claude/worktrees/epr-phase-3-5

# Schemas
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:check-dna
pnpm run schema:codegen:ts   # verify mode — no diffs
pnpm run schema:codegen:rs   # verify mode — no diffs
pnpm run lamad:codegen       # verify mode — no diffs

# Rust
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Holochain DNA
cd ../holochain/dna/elohim
just check
just build
just pack

# Sweettests
cd ../../..
pnpm run sweettest-check    # or equivalent — see CLAUDE.md
```

- [ ] **Step 2: Verify no skipped/ignored tests remain**

```bash
grep -rn '#\[ignore\]' elohim/elohim-storage/tests/   # Should be empty (Phase 3 ignore was lifted in T19)
```

- [ ] **Step 3: Merge to dev** (per `feedback_dev_branch_no_pr`)

```bash
cd /projects/elohim
git checkout dev
git merge --no-ff feature/epr-phase-3-5-trust-compute-gradient \
  -m "Merge feature/epr-phase-3-5-trust-compute-gradient — Phase 3.5 trust-compute gradient substrate close"
```

- [ ] **Step 4: Cleanup worktree**

```bash
git worktree remove /projects/elohim/.claude/worktrees/epr-phase-3-5
```

- [ ] **Step 5: Final commit (if any post-merge fixes needed) — single commit, conventional message.**

---

## Done definition

- [ ] FeedbackSignal EPR kind shipped (4 variants) with integrity validator + coordinator + sweettest
- [ ] AttentionTending EPR kind shipped, `Visibility::Private`, with integrity validator + coordinator + cross-agent privacy verified in sweettest
- [ ] CollectiveFilterPattern EPR kind shipped (k-anonymous; no peer identities)
- [ ] Edge-local predecessor map populated on every send; sealed-against-self at rest via dryoc 2-of-2
- [ ] Hop-by-hop back-prop walk (Primitive 2) wired into FeedbackSignal ingest path
- [ ] Gossip-flood notification (Primitive 3) layered on existing `/elohim/epr-atom/1.0.0` protocol
- [ ] Standing computation replaced — `Standing::evaluate(evaluator, subject, conn)` reads standing_view; placeholder deleted; per-evaluator pluralism preserved
- [ ] Tending lifecycle (TTL + re-tending + expiry sweep) wired into reconciliation controller
- [ ] k-anonymous local-peer aggregator emits CollectiveFilterPattern post-threshold
- [ ] Constitutional floor sub-schemas extend standing-policy + tending-policy manifest payloads
- [ ] Bootstrap default manifests seed first-run via `bootstrap_manifests.rs`
- [ ] Author-side compose-time StandingQuery API ships (HTTP route declared in app manifest)
- [ ] Cross-peer test harness primitive shipped; Phase 3's `#[ignore]` on `cold_fetch_resolves_manifest_from_peer` lifted
- [ ] End-to-end aunt-and-rage-bait integration test passes on the new harness
- [ ] All Phase 3 quality gates still pass: clippy, schema:test/validate/check-dna, schema-codegen verify, sweettest-check
- [ ] Local merge to dev with `--no-ff` merge commit; no PR

---

## Cross-coordination commitments

- **Recovery M5**: Coordinate any change to `dna-signal-stream.schema.json` with the M5 branch maintainer per `project_epr2b_recovery_m4_convergence`. FeedbackSignal's emission shape may inform M5's revocation UX.
- **Defender stub** (M5): The constitutional-floor manifests landed in T17 are what the defender specialist consults. Lighting them up may unblock part of the defender stub.
- **Phase 4 (VF-GraphQL)** depends on Phase 3.5 standing/tending substrate being live before VF semantics layer adds nuance. T14's standing_view is the projection Phase 4's GraphQL resolvers will read from.

## Subagent dispatch guardrails (verbatim from Phase 3 kickoff)

- Forbid `git revert`, `git reset --hard`, `git checkout --`, `git rm` on pre-existing commits. Out-of-scope file changes → BLOCKED report, not silent work-around.
- No new branches (stay on `feature/epr-phase-3-5-trust-compute-gradient` after worktree setup).
- No `git push` — local dev is integration target.
- Subagents always run `cargo build --tests` from the worktree before committing (per `feedback_swarm_composition_fresh_tree_build`).
- `RUSTFLAGS='--cfg getrandom_backend="custom"'` for elohim-storage; `RUSTFLAGS=""` for doorway/steward.
- `pnpm run sweettest-check` after any zome change (per `zome-sweettest-sync`).
- Run with `--test-threads=1` for any test that touches env vars (per `feedback_env_var_test_flakiness`).
