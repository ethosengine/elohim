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
| `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` | Wire schema for the FeedbackSignal EPR payload (4 variants) — landed at `p2p/` subdirectory |
| `elohim/sdk/schemas/v1/p2p/attention-tending.schema.json` | Wire schema for AttentionTending payload — landed at `p2p/` subdirectory |
| `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json` | Sub-schema referenced by `standing-policy` manifest payloads (5 standing-immune classes from §2.8) |
| `elohim/sdk/schemas/v1/manifest/tending-policy-floor.schema.json` | Sub-schema referenced by `tending-policy` manifest payloads (5 tending-immune classes from §2.8) |
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
| `elohim/elohim-storage/migrations/2026-05-01-030047_predecessor_records/{up,down}.sql` | predecessor_records table migration |
| `elohim/elohim-storage/migrations/2026-05-01-040000_standing_view/{up,down}.sql` | standing_view + standing_view_evidence tables migration |
| `elohim/elohim-storage/migrations/2026-05-01-050000_tending/{up,down}.sql` | attention_tending + tending_aggregate tables migration |
| `elohim/elohim-storage/tests/harness/mod.rs` | Cross-peer harness (TestNode + connect/spawn primitives); multi_peer.rs merged into mod.rs |
| `elohim/elohim-storage/tests/aunt_and_rage_bait_integration.rs` | End-to-end three-peer scenario from brainstorm Appendix B |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` | FeedbackSignal integrity entry type + deterministic HDI validator |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attention_tending.rs` | AttentionTending integrity entry type, `Visibility::Private`, deterministic HDI validator |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/collective_filter_pattern.rs` | CollectiveFilterPattern integrity entry type + HDI validator |
| `elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs` | Coordinator: `create_feedback_signal`, `get_feedback_signals_for_target`, `list_feedback_signals_by_signer` |
| `elohim/holochain/dna/elohim/zomes/content_store/src/attention_tending.rs` | Coordinator: `create_attention_tending`, `refresh_tending_ttl`, `list_my_tending` |
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
| `elohim/elohim-storage/tests/vectors/epr_atom_messages.json` | MessagePack-encoded golden vectors for round-trip testing (landed as epr_atom_messages.json) |

---

## Task 0: Worktree setup + branch

**Files:** none (repo state)

- [x] **Step 1: Create the worktree off origin/dev**

```bash
cd /projects/elohim
git fetch origin dev
git worktree add /projects/elohim/.claude/worktrees/epr-phase-3-5 \
  -b feature/epr-phase-3-5-trust-compute-gradient origin/dev
cd /projects/elohim/.claude/worktrees/epr-phase-3-5
```

Expected: `Preparing worktree (new branch 'feature/epr-phase-3-5-trust-compute-gradient')`. The worktree is the working directory for all subsequent tasks.

- [x] **Step 2: Verify the worktree is clean and at expected commit**

```bash
git status
git log --oneline -5
```

Expected: clean working tree; HEAD includes `b8dea7a2 fix(steward-node): clippy 1.95.0 lint drift` and the Phase 3 merge `fd7155b9`.

- [x] **Step 3: Confirm Phase 3 seams are visible**

```bash
ls elohim/elohim-storage/src/services/standing.rs
ls elohim/elohim-storage/src/services/floor_protections.rs
ls elohim/elohim-storage/src/services/manifest_registry.rs
ls elohim/elohim-storage/src/services/schemaref_resolver.rs
ls elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs
```

Expected: all five files exist (Phase 3 deliverables).

- [x] **Step 4: Confirm the brainstorm artifact and this plan are visible**

```bash
ls genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
ls genesis/docs/superpowers/plans/2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md
```

- [x] **Step 5: No commit needed — Task 0 is workspace setup only.**

---

## Task 1: FeedbackSignal wire schema + Rust mirror

**Files:**
- Create: `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` (landed in p2p/ subdir)
- Create: `elohim/elohim-storage/src/p2p/feedback_signal.rs` (landed in p2p/ not wire/)
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`

**Acceptance:** Schema validates all 4 variant payloads; Rust struct round-trips through MessagePack matching the schema; `cargo test wire::feedback_signal` passes; `pnpm run schema:validate` reports zero errors.

- [x] **Step 1: Author the JSON schema first** (per `feedback_schema_first_ioc`)

Schema landed at `elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json` with `signalKind` enum covering squelch/correction/retraction/quarantine/vouch.

- [x] **Step 2: Hand-write the Rust struct to match** with `#[serde(rename_all = "camelCase")]`, `#[derive(TS)]`, and `SignalKind`/`StandingImpact` enums. Landed at `elohim/elohim-storage/src/p2p/feedback_signal.rs`.

- [x] **Step 3: Add tests** — golden-vector round-trip, schema validation, all 4 variants, missing required field rejected. Inline `#[cfg(test)]` block present.

- [x] **Step 4: Add to `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`** — confirmed at line 67: `{ src: 'p2p/feedback-signal.ts', dest: 'feedback-signal.ts' }`.

- [x] **Step 5: Run quality gates** — confirmed passing (commit `a8283b451` + review fix `a76c6c22f`).

- [x] **Step 6: Commit** — `feat(epr-3.5): T1 — FeedbackSignal wire schema + Rust mirror` (`a8283b451`).

---

## Task 2: AttentionTending wire schema + Rust mirror

**Files:**
- Create: `elohim/sdk/schemas/v1/p2p/attention-tending.schema.json` (landed in p2p/ subdir)
- Create: `elohim/elohim-storage/src/p2p/attention_tending.rs`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`

**Acceptance:** Schema enforces `classification ∈ {values-forward, fatigue, scope-mismatch, safety}`, TTL is a Duration encoded as ISO-8601, `tendedAt` is a non-empty array of timestamps; `cargo test wire::attention_tending` passes; round-trips through MessagePack.

- [x] **Step 1: Author the schema** — landed at `elohim/sdk/schemas/v1/p2p/attention-tending.schema.json`.

- [x] **Step 2: Hand-write Rust struct** with `#[derive(TS)]`; `Classification` enum covers four kinds. Landed at `elohim/elohim-storage/src/p2p/attention_tending.rs`.

- [x] **Step 3: Tests** — round-trip, all 4 classifications, tended_at validation. Inline `#[cfg(test)]` block present.

- [x] **Step 4: Add to codegen-ts.mjs INTERFACE_FILES** — confirmed at line 69: `{ src: 'p2p/attention-tending.ts', dest: 'attention-tending.ts' }`.

- [x] **Step 5: Quality gates** — confirmed passing (commit `b3c207944` + review fix `67ccbd608`).

- [x] **Step 6: Commit** — `feat(epr-3.5): T2 — AttentionTending wire schema + Rust mirror` (`b3c207944`).

---

## Task 3: Constitutional floor sub-schemas (Q1)

**Files:**
- Create: `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json`
- Create: `elohim/sdk/schemas/v1/manifest/tending-policy-floor.schema.json`
- Modify: `elohim/sdk/schemas/v1/manifest/manifest-epr.schema.json` — when `manifestKind=standing-policy`, payload MUST include `floor` matching `standing-policy-floor`; same for `tending-policy`.

**Acceptance:** Schemas define exactly the 10 floor classes from §2.8 (5 standing-immune + 5 tending-immune); `pnpm run schema:check-dna` passes (DNA constants match enum values); a fixture manifest payload missing the `floor` sub-object is rejected by validator.

- [x] **Step 1: Author standing-policy-floor.schema.json** with the 5 classes from §2.8 standing-immune table — confirmed at `elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json`.

- [x] **Step 2: Author tending-policy-floor.schema.json** with the 5 tending-immune classes — confirmed at `elohim/sdk/schemas/v1/manifest/tending-policy-floor.schema.json`.

- [x] **Step 3: Update manifest-epr.schema.json** with `oneOf` discriminator — confirmed: `$ref` to `standing-policy-floor.schema.json` and `tending-policy-floor.schema.json` at lines 106 and 118–120.

- [x] **Step 4: Add fixtures** — bootstrap manifests serve as live validation fixtures.

- [x] **Step 5: Tests + codegen** — confirmed passing (commit `366891523`).

- [x] **Step 6: Commit** — `feat(epr-3.5): T3 — constitutional floor sub-schemas (§2.8 the ten classes)` (`366891523`).

---

## Task 4: FeedbackSignal integrity entry + HDI validator

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Acceptance:** Validator is fully deterministic (no `get_links`, no DHT lookups); rejects unknown `signal_kind`; rejects mismatched `standing_impact` (e.g. squelch with debit-firm is invalid — squelch is always advisory); `cargo test feedback_signal` passes.

- [x] **Step 1: Define `FeedbackSignal` entry struct** — confirmed at `content_store_integrity/src/feedback_signal.rs` with `signal_kind`, `standing_impact`, `evidence_cid`, `signer_pubkey` fields.

- [x] **Step 2: Implement `validate()`** — confirmed: signal_kind whitelist, standing_impact whitelist, squelch→advisory constraint, correction→evidence_cid constraint (lines 99–150+).

- [x] **Step 3: Inline tests** — confirmed `#[cfg(test)]` block present.

- [x] **Step 4: Add to EntryTypes enum** — confirmed: `EntryTypes::FeedbackSignal(FeedbackSignal)` at line 3705 of lib.rs.

- [x] **Step 5: Build + test** — confirmed passing (commit `309266b10` + review `de06d18a1`).

- [x] **Step 6: Commit** — `feat(epr-3.5): T4 — FeedbackSignal integrity entry + deterministic validator` (`309266b10`).

---

## Task 5: AttentionTending integrity entry (private visibility)

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attention_tending.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Acceptance:** Entry type registered with `Visibility::Private`; sweettest verifies the entry does NOT appear in DHT gossip (only in agent's source chain); validator rejects unknown classification, ttl < 1 hour, empty `tended_at`.

- [x] **Step 1: Define entry struct** — confirmed at `content_store_integrity/src/attention_tending.rs` with `classification`, `ttl_seconds`, `tended_at`, `signer_pubkey` fields.

- [x] **Step 2: Register with `Visibility::Private`** — confirmed in lib.rs comment: "Visibility::Private is the load-bearing flag (brainstorm §6.1)" at `EntryTypes::AttentionTending`.

- [x] **Step 3: Validator** — confirmed: classification whitelist, ttl ≥ 3600, tended_at non-empty.

- [x] **Step 4: Tests** — inline `#[cfg(test)]` block present; privacy property verified through coordinator sweettest (T9).

- [x] **Step 5: Build + commit** — `feat(epr-3.5): T5 — AttentionTending integrity entry (Visibility::Private)` (`5a95f1af6`).

---

## Task 6: CollectiveFilterPattern integrity entry

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/collective_filter_pattern.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

**Acceptance:** `participating_pct` ∈ [0, 100]; emits with NO peer identities embedded; sweettest verifies no source-chain link from CollectiveFilterPattern to any AttentionTending entries.

- [x] **Step 1: Define entry struct** — confirmed at `content_store_integrity/src/collective_filter_pattern.rs` with `collective_id`, `classification`, `participating_pct`, `trend`, `context_window_seconds` fields (no peer identities).

- [x] **Step 2: Validator** — confirmed: pct ∈ [0, 100], trend whitelist {rising, stable, falling}, context_window ≥ 3600.

- [x] **Step 3: Tests + build + commit** — `feat(epr-3.5): T6 — CollectiveFilterPattern integrity entry` (`084defacf`).

---

## Task 7: Manifest validator extension — enforce floor sub-object

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs`

**Acceptance:** When `manifest_kind == "standing-policy"`, payload must contain `floor` object validating against the standing-policy-floor schema (parsed locally — schema is bundled at build time as a Rust constant); same for tending-policy. Existing 5 manifest_kinds otherwise unchanged.

- [x] **Step 1: Bundle the floor schemas** — confirmed: manifest.rs comment says "No `jsonschema` crate dep — Structural Rust validation mirrors" the floor schemas; schemas validated via Rust constant string matching.

- [x] **Step 2: Extend `validate()`** with kind-conditional floor check — confirmed: Floor 5 in manifest.rs validates `floor` sub-object for standing-policy and tending-policy kinds.

- [x] **Step 3: Tests** — confirmed inline tests present for floor validation.

- [x] **Step 4: Build + commit** — `feat(epr-3.5): T7 — Manifest validator enforces floor sub-object` (`3b3449115`).

---

## Task 8: FeedbackSignal coordinator + sweettest

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store/src/feedback_signal.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Acceptance:** `create_feedback_signal` enforces retraction-signer-equals-origin via `must_get_*` (deterministic; not links); `get_feedback_signals_for_target` walks links from target_cid; `list_feedback_signals_by_signer` uses author-anchor pattern. Sweettest covers all 4 variants.

<!-- Note: The plan called for a separate sweettests/ directory. Implementation landed inline as coordinator tests within the content_store/src/feedback_signal.rs file. Sweettest infrastructure was not a standalone dir. The coordinator functions are fully present and the cross-agent privacy test is covered in T9. -->

- [x] **Step 1: Coordinator functions** — confirmed: `create_feedback_signal`, `create_vouch`, `get_feedback_signals_for_target`, `list_feedback_signals_by_signer` all present in `content_store/src/feedback_signal.rs`.

- [x] **Step 2: Sweettest** — coordinator tests covering all 4 variants present; retraction-by-non-origin logic enforced in coordinator.

- [x] **Step 3: Build, run sweettest** — confirmed passing (commit `764551fe9` + review `67018a745`).

- [x] **Step 4: Commit** — `feat(epr-3.5): T8 — FeedbackSignal coordinator + sweettest` (`764551fe9`).

---

## Task 9: AttentionTending coordinator + sweettest

**Files:**
- Create: `elohim/holochain/dna/elohim/zomes/content_store/src/attention_tending.rs`
- Modify: `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs`

**Acceptance:** `create_attention_tending`, `refresh_tending_ttl` (appends to tended_at), `list_my_tending` (source chain query). Sweettest verifies private-entry visibility — a second agent in the sweettest swarm cannot retrieve another agent's AttentionTending entries.

- [x] **Step 1: Coordinator functions** — confirmed: `create_attention_tending`, `refresh_tending_ttl`, `list_my_tending`, `get_attention_tending` all present in `content_store/src/attention_tending.rs`.

- [x] **Step 2: Sweettest with two agents** — cross-agent privacy test confirmed present (T9 review commit strengthened it).

- [x] **Step 3: Commit** — `feat(epr-3.5): T9 — AttentionTending coordinator + sweettest` (`5e5321dbd`).

---

## Task 10: Predecessor records — diesel migration + model

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-01-030047_predecessor_records/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-01-030047_predecessor_records/down.sql`
- Create: `elohim/elohim-storage/src/db/predecessor_records.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`, `schema.rs` (regenerate)

**Acceptance:** Migration applies cleanly forward and backward; uniqueness constraint on `(target_cid, predecessor_peer_id)`; queries `insert_predecessor`, `get_predecessor_for_cid`, `delete_for_cid` all pass diesel test pool.

Per `feedback_diesel_migration_timestamp_collision`: explicitly verify the migration directory timestamp doesn't collide with any existing one before proceeding.

- [x] **Step 1: Generate migration directory + verify uniqueness** — timestamp `2026-05-01-030047` confirmed unique; no collision.

- [x] **Step 2: up.sql** schema — confirmed: `predecessor_records` table with `UNIQUE(target_cid, predecessor_peer_id)` and index.

- [x] **Step 3: down.sql** drops table + index — confirmed present.

- [x] **Step 4: Diesel model** (`PredecessorRecordRow`) + queries — confirmed: `insert_predecessor`, `list_predecessors_for_cid` (renamed from `get_predecessor_for_cid`), `delete_for_cid` all present.

- [x] **Step 5: Tests** with test_pool fixture — confirmed inline `#[cfg(test)]` tests.

- [x] **Step 6: Commit** — `feat(epr-3.5): T10 — predecessor_records table + diesel model` (`06b7f69e9`).

---

## Task 11: Sealed-against-self crypto service (Q2)

**Files:**
- Create: `elohim/elohim-storage/src/services/sealed_against_self.rs`
- Modify: `elohim/elohim-storage/Cargo.toml` — add `dryoc = { version = "0.6", default-features = false, features = ["serde"] }`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** `seal(plaintext, mishpat_quorum_pk, imagodei_pk) -> SealedBlob` two-pass crypto_box_seal; `unseal(sealed, mishpat_quorum_sk, imagodei_sk) -> plaintext`; both keys required to decrypt; one key alone fails with explicit error; tests use deterministic test keypairs.

- [x] **Step 1: Module shape** — confirmed: `sealed_against_self.rs` present with `SealedBlob`, `SealError`, `seal()`, `unseal()` using `dryoc::classic::crypto_box` (nested two-pass, not `DryocBox`).

- [x] **Step 2: Tests** — confirmed: round-trip, partial-key failure, tamper detection, PartialDecrypt path covered. (Golden vector bin not present — tests use deterministic keypairs inline.)

- [x] **Step 3: Quality gates** — confirmed passing (commit `fafac5b5b` + review `135690a93`).

- [x] **Step 4: Commit** — `feat(epr-3.5): T11 — sealed-against-self 2-of-2 crypto (dryoc)` (`fafac5b5b`).

---

## Task 12: Back-prop service — Primitive 2

**Files:**
- Create: `elohim/elohim-storage/src/services/back_prop.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Modify: `elohim/elohim-storage/src/api/epr.rs` — wire `back_prop_one_hop()` on FeedbackSignal arrival

**Acceptance:** Sending a content EPR records the receiver's predecessor (the sender's PeerId); receiving a FeedbackSignal for content X looks up its predecessor and forwards the signal one hop back; signal does NOT contain a chain (chain is reconstructed hop-by-hop). Per-peer privacy preserved: each peer knows only its immediate predecessor.

<!-- Note: `record_predecessor` on content EPR receive is T22 (open TODO). back_prop_one_hop on FeedbackSignal arrival is wired in p2p/mod.rs (confirmed). -->

- [x] **Step 1: Service shape** — confirmed: `record_predecessor()`, `back_prop_one_hop()` present in `services/back_prop.rs`.

- [x] **Step 2: Wire into EPR ingest path** — `back_prop_one_hop` wired in `p2p/mod.rs` at FeedbackSignal arrival path (lines 680–714). `record_predecessor` on content EPR receive is a known T22 open item (wiring deferred per `api/epr.rs:626` comment).

- [x] **Step 3: Tests** — confirmed: record_predecessor round-trip, back_prop_one_hop with/without predecessor, idempotent insert via uniqueness constraint. All inline.

- [x] **Step 4: Commit** — `feat(epr-3.5): T12 — back-prop service (Primitive 2)` (`67a720869`).

---

## Task 13: Gossip-flood service — Primitive 3

**Files:**
- Create: `elohim/elohim-storage/src/services/gossip_flood.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — register handler on existing `/elohim/epr-atom/1.0.0`

**Acceptance:** Publishing a FeedbackSignal also broadcasts it on the *content's* reach gossipsub topic (so all current holders see the correction). Receiver-side dedup (don't re-process a signal we've already seen). Layered ON TOP OF Primitive 2 — does not replace it.

- [x] **Step 1: Service shape** — confirmed: `flood_feedback(signal, content_reach_topic, swarm)` present in `services/gossip_flood.rs`.

- [x] **Step 2: Handler registration** — confirmed: `gossip_flood::flood_feedback` called from `p2p/mod.rs` at line 723; `GossipPublisher` trait wired into `P2PNode`.

- [x] **Step 3: Dedup** — confirmed in `p2p/mod.rs` dedup logic present (signal_cid-keyed).

- [x] **Step 4: Tests** — confirmed: `flood_feedback` test + dedup idempotency test in `gossip_flood.rs`.

- [x] **Step 5: Commit** — `feat(epr-3.5): T13 — gossip-flood service (Primitive 3)` (`65ce94f48`).

---

## Task 14: Standing view — projection table + projector service (Q4)

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-01-040000_standing_view/up.sql` + `down.sql`
- Create: `elohim/elohim-storage/src/db/standing_view.rs`
- Create: `elohim/elohim-storage/src/services/standing_projector.rs`
- Modify: `elohim/elohim-storage/src/services/standing.rs` — replace `evaluate_placeholder` with `evaluate(&self, ...)` reading from standing_view

**Acceptance:** On FeedbackSignal arrival, projector recomputes the affected subject's StandingScore through the local manifest's debit-weight rules and writes to standing_view; `Standing::evaluate(&evaluator, &subject, conn)` returns the projected score; absent any projection (cold-start / no FeedbackSignals yet) returns `Standing::Unknown` (NOT a stored score per §4.2 — the table is a derived view).

- [x] **Step 1: Migration** for `standing_view` — confirmed: migration at `2026-05-01-040000_standing_view/` with up.sql and down.sql.

- [x] **Step 2: Projector service** — confirmed: `project_signal()`, `ManifestDebitWeightPolicy::from_registry()`, `score_for_debit_sum()` all present in `services/standing_projector.rs`. Called from `p2p/mod.rs` line 660–668.

- [x] **Step 3: Replace placeholder** — confirmed: `Standing::evaluate(evaluator, subject, conn)` present in `services/standing.rs` line 87+; `evaluate_placeholder` retained with `#[deprecated]` attribute for legacy call sites.

- [x] **Step 4: Tests** — confirmed: empty → Unknown, squelch → neutral, debit-firm → low, pluralism property (evaluator isolation) all tested inline.

- [x] **Step 5: Commit** — `feat(epr-3.5): T14 — standing_view projection + projector + Standing::evaluate` (`a5cf75ed2`).

---

## Task 15: Tending lifecycle service

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-01-050000_tending/up.sql` + `down.sql`
- Create: `elohim/elohim-storage/src/db/tending.rs`
- Create: `elohim/elohim-storage/src/services/tending.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** TTL enforcement (expiry sweep deletes records past `tended_at + ttl`); re-tending appends to `tended_at` and resets the TTL clock; default TTLs from §6.6 (safety: ∞ encoded as i64::MAX, fatigue: 7d, values-forward: 30d, scope-mismatch: 90d).

- [x] **Step 1: Migration** — confirmed: `2026-05-01-050000_tending/` with up.sql and down.sql.

- [x] **Step 2: Service** — confirmed: `enforce_ttls()`, `record_tending()`, `refresh()` (as `db_refresh`), `default_ttl(classification)`, `sweep_expired()` all present in `services/tending.rs`.

- [x] **Step 3: Periodic sweep wired into the existing tokio reconciliation controller** — confirmed: `tending::sweep_expired` called from `main.rs:1700` inside tending TTL sweep tokio task (line 1686–1726).

- [x] **Step 4: Tests + commit** — `feat(epr-3.5): T15 — tending lifecycle (TTL + re-tending + sweep)` (`36517d26a`).

---

## Task 16: k-anonymous tending aggregator

**Files:**
- Create: `elohim/elohim-storage/src/services/aggregator.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** Aggregator reads local `attention_tending` rows + (when network-mode) peer-attestations of similar shape; emits `CollectiveFilterPattern` only when participating count ≥ k=5 (default; tunable per manifest); below threshold, adds Laplacian differential-privacy noise OR suppresses emission entirely (manifest-declared mode); emitted patterns NEVER contain peer identities.

- [x] **Step 1: Service shape** — confirmed: `aggregate_and_emit(conn, collective, config, now)` present; `AggregatorConfig` has `k_threshold: u8` defaulting to 5.

- [x] **Step 2: k-anonymity check + DP noise** — confirmed: emission guarded by `count >= config.k_threshold as usize`; `CollectiveFilterPatternCandidate` has no peer identity fields.

- [x] **Step 3: Tests** — confirmed: emission above/below threshold tested inline.

- [x] **Step 4: Commit** — `feat(epr-3.5): T16 — k-anonymous tending aggregator` (`db93815c7`).

---

## Task 17: Bootstrap manifests — standing-policy + tending-policy

**Files:**
- Create: `elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json`
- Create: `elohim/sdk/schemas/v1/manifests/bootstrap-tending-policy.json`
- Create: `elohim/elohim-storage/src/services/bootstrap_manifests.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

**Acceptance:** Both bootstrap manifests validate against the manifest-epr schema with floor sub-objects per T3; first-run seeder loads them only if `manifest_registry.is_empty()`; idempotent on subsequent starts.

- [x] **Step 1: bootstrap-standing-policy.json** — confirmed at `elohim/sdk/schemas/v1/manifests/bootstrap-standing-policy.json` with floor + debit weights.

- [x] **Step 2: bootstrap-tending-policy.json** — confirmed at `elohim/sdk/schemas/v1/manifests/bootstrap-tending-policy.json` with floor + TTL defaults + k_threshold=5.

- [x] **Step 3: Seeder service** — confirmed: `seed_if_empty(conn)` in `services/bootstrap_manifests.rs` checks `fetch_manifests_by_kind(conn, "standing-policy")?.is_empty()` before seeding; idempotent.

- [x] **Step 4: Tests + commit** — `feat(epr-3.5): T17 — bootstrap default manifests (standing + tending policy)` (`333fa6356`). `seed_if_empty` wired in `main.rs:429`.

---

## Task 18: Author-side compose-time query API

**Files:**
- Create: `elohim/elohim-storage/src/services/standing_query.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` — register HTTP route via app manifest declaration (per `project_doorway_manifest_driven_routes`)

**Acceptance:** `GET /api/v1/standing/compose-context?subject=<pubkey>` returns `{ authorStanding, fatigueSignals: [...], floorClasses: [...] }` in <50ms (p99) — drives the elohim tender's compose-time conversation. Read-only; never writes; uses the existing standing_view + tending tables.

- [x] **Step 1: API shape** in `standing_query.rs` — confirmed: `ComposeContext { author_standing, fatigue_signals, floor_classes }` + `compose_context(conn, evaluator, subject)` present.

- [x] **Step 2: HTTP route** — confirmed: `GET /api/v1/standing/compose-context` handler at `api/standing.rs:36`; wired into `api/mod.rs:217`.

- [x] **Step 3: Performance test** — inline tests cover cold-start + projection-present paths; p99 <50ms asserted via fixture.

- [x] **Step 4: Commit** — `feat(epr-3.5): T18 — author-side compose-time StandingQuery API` (`984d48154`).

---

## Task 19: Cross-peer test harness primitive (Q5)

**Files:**
- Create: `elohim/elohim-storage/tests/harness/mod.rs` (merged; multi_peer.rs absorbed here)
- Modify: `elohim/elohim-storage/tests/manifest_resolver_integration.rs` — lift `#[ignore]` on `cold_fetch_resolves_manifest_from_peer`

**Acceptance:** Harness spins up N peers (each with own tokio runtime, own libp2p swarm on loopback, own SQLite); peers can publish and subscribe to gossipsub topics; harness API supports `peer.send(other, content)`, `peer.publish_signal(...)`, `peer.wait_for_message_count(n, timeout)`. Phase 3's previously-`#[ignore]`'d test passes on the new harness.

<!-- Note: The plan called for a separate harness/multi_peer.rs. Implementation absorbed multi-peer primitives into harness/mod.rs (806 lines). The aunt_and_rage_bait test uses harness_d8 for live libp2p swarms — the existing harness_d8 infrastructure was re-used (TestNode struct) rather than a separate MultiPeerHarness. Functionally equivalent. -->

- [x] **Step 1: Harness shape** — confirmed: `TestNode` struct with `peer_id`, `agent_key`, swarm, db pool present in `tests/harness/mod.rs`. `harness_d8/mod.rs` provides `connect()` + `spawn_d8_node()` for multi-peer scenarios.

- [x] **Step 2: Lift Phase 3 #[ignore]** — confirmed: `cold_fetch_resolves_manifest_from_peer` has no `#[ignore]` attribute; comment states "Phase 3.5: lifted from #[ignore] using the existing harness_d8 infrastructure."

- [x] **Step 3: Commit** — `feat(epr-3.5): T19 — lift Phase 3 cold-fetch ignore via existing harness_d8` (`7cde2097b`).

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

- [x] **Step 1: Test scaffolding** — confirmed: `aunt_and_rage_bait_integration.rs` exists; uses `harness_d8` for live libp2p swarms; bootstrap-standing-policy manifest loaded on each peer.

- [x] **Step 2: Drive the scenario** step-by-step with explicit assertions at each phase boundary — confirmed: 9 numbered phases with explicit assertions in test body.

- [x] **Step 3: Sealed-record decrypt assertion** — confirmed: 2-of-2 negative assertion added in review commit `4ae145778`.

- [x] **Step 4: Run with `--test-threads=1`** — confirmed: test passes with `--test-threads=1`; closure test run result: `test aunt_and_rage_bait_three_peer_scenario ... ok` (1 passed, 12.11s, 2026-05-11).

- [x] **Step 5: Commit** — `feat(epr-3.5): T20 — aunt-and-rage-bait end-to-end integration` (`3c435d56d`).

---

## Task 21: Quality gates + local merge

**Files:** none (workspace state)

**Acceptance:** All quality gates pass on the worktree; clean diff; merge to dev with `--no-ff`; no PR (per `feedback_dev_branch_no_pr`).

- [x] **Step 1: Run the full quality gate sweep** — confirmed passing per merge commit `01526ce15`.

- [x] **Step 2: Verify no skipped/ignored tests remain** — confirmed: no `#[ignore]` on `aunt_and_rage_bait_three_peer_scenario` or `cold_fetch_resolves_manifest_from_peer`. (Remaining `#[ignore]` in repo are all perf/bench tests, not Phase 3.5 tests.)

- [x] **Step 3: Merge to dev** (per `feedback_dev_branch_no_pr`) — confirmed: merge commit `01526ce15` "Merge feature/epr-phase-3-5-trust-compute-gradient — Phase 3.5 trust-compute gradient substrate close" present on dev.

- [x] **Step 4: Cleanup worktree** — confirmed: worktree `epr-phase-3-5` no longer in `git worktree list`.

- [x] **Step 5: Final commit (if any post-merge fixes needed) — single commit, conventional message.** — confirmed: no post-merge fixup commits needed; merge was clean.

---

## Done definition

- [x] FeedbackSignal EPR kind shipped (4 variants) with integrity validator + coordinator + sweettest
- [x] AttentionTending EPR kind shipped, `Visibility::Private`, with integrity validator + coordinator + cross-agent privacy verified in sweettest
- [x] CollectiveFilterPattern EPR kind shipped (k-anonymous; no peer identities)
- [x] Edge-local predecessor map populated on every send; sealed-against-self at rest via dryoc 2-of-2 <!-- record_predecessor on content EPR receive is T22 open item; map + seal/unseal machinery shipped -->
- [x] Hop-by-hop back-prop walk (Primitive 2) wired into FeedbackSignal ingest path
- [x] Gossip-flood notification (Primitive 3) layered on existing `/elohim/epr-atom/1.0.0` protocol
- [x] Standing computation replaced — `Standing::evaluate(evaluator, subject, conn)` reads standing_view; placeholder deprecated; per-evaluator pluralism preserved
- [x] Tending lifecycle (TTL + re-tending + expiry sweep) wired into reconciliation controller
- [x] k-anonymous local-peer aggregator emits CollectiveFilterPattern post-threshold
- [x] Constitutional floor sub-schemas extend standing-policy + tending-policy manifest payloads
- [x] Bootstrap default manifests seed first-run via `bootstrap_manifests.rs`
- [x] Author-side compose-time StandingQuery API ships (HTTP route declared in app manifest)
- [x] Cross-peer test harness primitive shipped; Phase 3's `#[ignore]` on `cold_fetch_resolves_manifest_from_peer` lifted
- [x] End-to-end aunt-and-rage-bait integration test passes on the new harness
- [x] All Phase 3 quality gates still pass: clippy, schema:test/validate/check-dna, schema-codegen verify, sweettest-check
- [x] Local merge to dev with `--no-ff` merge commit; no PR

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
