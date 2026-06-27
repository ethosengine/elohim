---
title: "Plural Mishpat Lenses over an EPR — Wave-1 implementation plan (hash-neutral, household-provable)"
id: plural-mishpat-lenses-wave1-plan
status: Draft
class: protocol-canonical
domain: D7
sprint: vision-deferred   # D7 collective-governance ranks below the household seed; Wave-1 is a thin household-provable proof-of-loop, not a scheduled sprint
topic: [governance, mishpat, lens-market, facings, fold, commitment-action, affinity, contention, regime-drift, election, bounty, hash-neutral, participation-tracks, deterministic-contract, household-nodes]
refines:
  - genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
cites:
  - plural-mishpat-lenses-over-epr-design | the spec this plan implements (Wave-1 = gap-items #1-#9); plan refines it and corrects the binding-key to slug-id (A3) | sha256:ab0055896398ef95 | path: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
  - resilience-facings-select-fold-aggregate-design | the select-fold-aggregate fold idiom (free-fn static dispatch, no trait, no-diesel firewall) the affinity/contention/selector folds follow | sha256:8f2136ecd8678e6c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - per-substrate-limitarian-governor-design | the concentration_snapshot C-class migration + measure.rs fold are the template for the affinity/contention C-class projections | sha256:5d10a556e2ec7a14 | path: genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md
  - elohim/elohim-facings/src/fold.rs
  - elohim/elohim-facings/src/folds/rea.rs
  - elohim/elohim-facings/src/folds/operational_weave.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
  - elohim/elohim-storage/src/signals.rs
  - elohim/elohim-storage/src/mishpat_projection.rs
  - elohim/elohim-storage/src/db/mishpat_commitments.rs
  - elohim/elohim-storage/src/db/rea_commitments.rs
  - elohim/elohim-storage/src/services/mutuality_audit_service.rs
  - elohim/elohim-storage/src/epr_codec.rs
  - elohim/elohim-storage/src/services/bounds_validator.rs
  - elohim/sdk/domains/elohim/manifest.json
# Mixed-env plan (CLAUDE.md scope convention): NO doc-level requires_env. Every Wave-1 task is testable on
# the household-nodes class (local hc:start:seed conductor+storage+doorway, or the live matthew pod).
# Waves 2-4 are out of scope here; Wave-3/4 carry @requires:alpha-cluster-6peer in the spec gap-items.
---

# Plural Mishpat Lenses over an EPR — Wave-1 plan

Implements **Wave-1** of [`…plural-mishpat-lenses-over-epr-design.md`](../specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md)
(gap-items #1–#9). Wave-1 proves the *whole loop at human scale* on `household-nodes` with **no DNA
reinstall**. It is written **architecture-first**: Part A fixes the seams every wave composes against;
Part B implements Wave-1 behind those seams so Waves 2–4 slot in without rework.

---

# PART A — Architecture (the wave-spanning seams; read before any task)

## A1. Crate placement — FOLD into three existing crates; do NOT create `elohim-lens-market`

The lens-market is **three concerns with three already-enforced homes**; a new crate would straddle all
three dependency walls and have no clean seam (it can't hold the WASM zome, can't hold the folds without
re-deriving the no-diesel firewall, can't hold projections without pulling diesel/axum — at which point
it *is* a slice of elohim-storage).

| Piece | Home module (new files marked +) |
|-------|----------------------------------|
| Lens / affinity / contention / selector **folds** | `elohim/elohim-facings/src/folds/lens_affinity.rs` +, `lens_contention.rs` +, `lens_selector.rs` + (register via `pub mod` in `folds/mod.rs`) |
| **Commitment-action** validators (teeth) | `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (new `validate_*` fns + arms in `validate_commitment_payload`) |
| **Projection parse** | `elohim/elohim-storage/src/mishpat_projection.rs` (new `parse_author_lens` etc. in `parse_commitment_payload`) |
| **Signal handler** | `elohim/elohim-storage/src/signals.rs` (extend the existing `CommitmentCommitted` arm — **no new signal type**) |
| **Tables + CRUD** | `elohim/elohim-storage/src/db/lenses.rs` +, `lens_bindings.rs` +, `lens_affinity.rs` +, `lens_contention.rs` + + migrations — **source-of-truth per I5: A-class (lenses, epr_lens_bindings, ballot_tallies, elections, lens_bounties) → Holochain DHT; C-class (lens_affinity, epr_contention) → operational (reconstructable, no `dht_anchor_hash`)** |
| **ts-rs views** | `elohim/elohim-views/src/lens.rs` + + `views_convert/` From-impls |
| **Facing adapters** (DB→fold→view) | `elohim/elohim-storage/src/services/lens_*_facing.rs` + (mirror `mishpat_commitment_facing.rs`) |
| **`contention-breach` signal** | `elohim/sdk/domains/elohim/manifest.json` `signalKinds` (**data, not code**) |

## A2. The contracts that span all four waves (the interfaces)

**Decision: no unifying `trait Lens` / `trait Fold`.** The facings crate has *no* `Fold` trait, no
registry, no `Box<dyn>` — it is free-fn static dispatch + `pub mod` registration, kept DB-free by a
transitive-diesel guard test (`elohim-facings/src/lib.rs`). Forcing dyn-dispatch would fight the crate's
purity boundary. So the **interface IS the data contract**, not a Rust trait: the lens payload schema +
the fold Row types + the fold fn signatures + the projection-class rules. This is what templates clean
code without inventing an abstraction the substrate resists. (A unifying trait stays an explicit, *later*
option if the lens count ever demands it — net-new scope, not Wave-1.)

**THE CONTRACT IS THE SCHEMA + MANIFEST (backend-authoritative).** The front↔back contract that makes
the code compile consistently end-to-end is NOT ad-hoc ts-rs structs — it is the **view schema**
(`elohim/sdk/schemas/v1/views/lens-market-view.schema.json`, SoT for field names, validated by
`elohim-storage/tests/schema_contract.rs::lens_market_view_matches_schema`) plus the **app-manifest
vocabulary** (`elohim/sdk/domains/elohim/manifest.json` → `signalKinds."contention-breach"`). The Rust
view (`elohim/elohim-views/src/lens.rs`) CONFORMS to the schema; ts-rs `export_bindings` +
`schema:codegen:ts` are the mechanical projections to TS. The front end SENSES and INSPIRES these
shapes; the backend DECIDES them. Process = the "Adding a new view" 6-step: schema → conforming Rust
struct → schema_contract test → `INTERFACE_FILES` → codegen → pre-push freshness gate. **[Wave-1
scaffolding LANDED 2026-06-27: `lens-market-view.schema.json` + `LensMarketView`/`LensBindingView` +
`contention-breach` signalKind + schema_contract test + INTERFACE_FILES entry; codegen runs at build
(freshness-gated).]**

**(I1) The Lens payload schema — THE central durable contract** (a Lens *is* a `Commitment` with
`action = "author-lens"`; the whole concept lives in `payload_json`, zero struct change). Author it as
`elohim/sdk/schemas/v1/commitments/author-lens.schema.json` (mirrors `delegates-compute.schema.json`):

```jsonc
{
  "action": "author-lens",
  "governs_epr": "epr:lamad-spa",      // EPR SLUG-ID (the scope key) — see A3, NOT the dag-cbor CID
  "school": "georgist",                 // the collective / school-of-thought label
  "rule": { /* deterministic predicate over the EPR signal — the teeth */ },
  "telos": { /* what this lens steers toward (viability and/or justice target) */ },
  "role": "lens",                       // lens | floor | ceiling
  "version_parent": null                // CID of the superseded lens (immutable → new-create chain)
}
```

**(I2) The fold Row types + signatures** (free-fn idiom; `BTreeMap` returns for wire determinism, per
`fold.rs`):

```rust
// elohim/elohim-facings/src/folds/lens_affinity.rs
pub struct LensSelectionRow { pub lens_cid: String, pub selector_agent: String,
                              pub epr_scope: String, pub selected_at: String }
pub fn affinity_by_lens(rows: &[LensSelectionRow]) -> BTreeMap<String, usize>;          // distinct selectors / lens
pub fn affinity_for(rows: &[LensSelectionRow], lens_cid: &str, scope: &str) -> usize;

// elohim/elohim-facings/src/folds/lens_contention.rs
pub struct LensVerdictRow { pub epr_cid: String, pub lens_cid: String,
                            pub verdict: String, pub agent: String }
pub fn contention_index(rows: &[LensVerdictRow], epr_scope: &str) -> f64;               // agree/disagree dispersion
pub fn verdicts_by_lens(rows: &[LensVerdictRow]) -> BTreeMap<String, BTreeMap<String, usize>>;

// elohim/elohim-facings/src/folds/lens_selector.rs
pub struct LensRow { pub lens_cid: String, pub epr_scope: String,
                     pub affinity: usize, pub contention: f64, pub valid: bool }
pub fn select_lenses(rows: &[LensRow]) -> Vec<&LensRow>;   // skip !valid (per-lens degrade), rank by affinity
```

**(I3) The regime-drift contract** (the genuinely new primitive — the joint predicate; pure fn, lives in
`lens_selector.rs` or a sibling `regime.rs`):

```rust
pub enum RegimeStatus { Stable, Drifting, Breached }
// Breached iff affinity_now < affinity_prev (justice surface decays) AND
//             contention_now > contention_prev (viability/dissent surface rises) — the FUSION (spec §8).
pub fn classify_regime(affinity_now: usize, affinity_prev: usize,
                       contention_now: f64, contention_prev: f64) -> RegimeStatus;
```

**(I4) The `ContentionBreach` signal** = a **manifest-declared string-named extension signal**
(`"contention-breach"` in `sdk/domains/elohim/manifest.json` `signalKinds`), projected via the existing
`project_extension_signal` path — **NOT** a new `SignalKind` enum variant (which would force schema enum +
exhaustive policy match + `parse_signal_kind` + a ts-rs fan-out). Emit is templated on the *built*
`MutualityAuditService::run_sweep` (classify → breach → idempotent audit-log row); verifiable cause rides
the governance coupling leg + `evidence_cid`.

**(I5) The projection-class contract:**
- **A-class** (lenses, epr_lens_bindings, ballot_tallies, elections, lens_bounties): `cid TEXT PRIMARY
  KEY` (= **entry_hash**), nullable `dht_anchor_hash` (NULL ⇒ un-notarized, fail-closed), source-of-truth
  = DHT. Template: `mishpat_commitments` migration + the anchor-preserving upsert.
- **C-class** (lens_affinity, epr_contention): **no `dht_anchor_hash`**, slug PK (`{lens}:{scope}:{at}`),
  `Queryable + Serialize` only, recompute path = a facing fold. Template: `concentration_snapshot`
  migration.

**(I6) The Rust→TS view contract:** `#[derive(… TS)] + #[serde(rename_all="camelCase")] + #[ts(export,
export_to="../../sdk/storage-client-ts/src/generated/")]` in `elohim-views/src/lens.rs`; `cargo test
export_bindings` from `elohim/elohim-views/`. Not-selected lens fields: `#[serde(default,
skip_serializing_if="Option::is_none")] #[ts(optional)]`.

## A3. The binding-key correction (load-bearing — supersedes spec §5/§13 detail)

The spec pinned the Lens↔EPR binding on the dag-cbor `EprHead` CID (`bafyrei…`). **The live scope/bounds
machinery keys on the EPR *slug-id* string** (`epr:lamad-spa`) everywhere — `bounds.epr_scope`,
`in_scope_of`, `find_active_delegates_compute`. A forward index on the dag-cbor entry_hash would line up
with **no existing scope row**. **Wave-1/2 bind on the slug-id** so the forward index reuses
`find_active_delegates_compute` (a SQL scope projection, zero new DHT anything). This collapses the spec's
"lone DNA-move risk" (the forward-index LinkType): per the DNA CLAUDE.md ("a link that exists only to
serve a query belongs in the SQL projection"), the LinkType is only forced if the *index itself must be
notarized* — Wave-2 does not require that, so **Waves 1 and 2 are both hash-neutral.** → spec follow-up
captured (A6).

## A4. Hash-neutrality ledger (the deployment contract)

| New thing | Class | Verdict |
|-----------|-------|---------|
| `author-lens`/`ratifies-election`/`bounty-fresh-lens` arms + `validate_*` fns | coordinator action | **HASH-NEUTRAL** (`update_coordinators` hot-swap) |
| AffinityFold / ContentionFold / LensSelector / classify_regime | facing folds | **HASH-NEUTRAL** |
| lens / binding / affinity / contention / election / bounty tables + CRUD + views + signal-arm | storage | **HASH-NEUTRAL** |
| `contention-breach` signal | manifest data | **HASH-NEUTRAL** |
| Forward index (EPR slug-id → lenses) via SQL scope projection | storage | **HASH-NEUTRAL** (A3) |
| `commitment_action_requirements` integrity arm / first-class `Lens` entry type / `EprToLens` LinkType | integrity bytecode | **HASH-MOVING — out of scope (Wave-2+ only if forced)** |

**Wave-1 is fully hash-neutral** → deployable via coordinator hot-swap, provable on `household-nodes`, no
agent re-key.

## A5. Gospel guardrails baked into the interfaces (non-negotiable)

- **`cid == entry_hash`** (read key, scope key); `action_hash` is *only* `dht_anchor_hash`. Returning
  action_hash as CID passes per-task tests but silently breaks every bounds-gate.
- **Commitments are immutable** → lens versioning = new `create` (new CID) + `version_parent`; reuse
  `CommitmentByState` links for lifecycle/outcome, never mint a new link type.
- **Closed coordinator default** → a new action MUST get a `validate_commitment_payload` arm or
  `create_commitment` rejects it; integrity defaults to pass (`_ => None`) — do **not** add a
  `commitment_action_requirements` arm (that is the hash-moving line).
- **Ballots are B2** → `cast_ballot` reuses the **imagodei Attestation** path (private ballot + notarized
  tally), NOT a Commitment; only `ratifies-election` is the notarized Commitment.
- **Signal = manifest string, not enum variant** (I4).
- **Cross-namespace `agent_cid` / h_app_id consistency** → lens selection rows that join household/agent
  data use one consistent `h_app_id` (the resilience-card projection-dormancy precedent); never raw-compare
  agent_cid against a transport id.
- **Fail-closed per row** → unparseable/invalid lens degrades *its own row* (`filter_map` + `warn!`,
  template `db/rea_commitments.rs`), never empties the lens set (the EprRouter lesson).

## A6. Complementary work captured (kept out of this plan's scope)

- Spec follow-up: amend `…plural-mishpat-lenses…design.md` §5/§13 to record the **slug-id binding key**
  (A3) and that Wave-2's forward index is hash-neutral. → backlog item.
- `emit_reciprocity_imbalance` is a logging stub (`mutuality_audit_service.rs`); wiring the conductor
  signing path (`signal_emit.rs`) is shared infra the ContentionBreach emit needs (T6) — note as a shared
  dependency, not new scope.

---

# PART B — Wave-1 implementation (TDD; each task: failing test → implement → verify)

Discipline: **TDD** (write the failing test first). Per-touched-tree pre-push gates run per task
(`sprint-DoD-includes-prepush-gates`): facings/storage → `RUSTFLAGS="" cargo test/clippy/fmt` with
`CARGO_TARGET_DIR` set; zome → sweettest; views → `cargo test export_bindings`. CI-green ≠ binding-correct
→ Wave-1 closes only after a live `household-nodes` render of the loop.

### T1 — Lens payload schema + `author-lens` validator (teeth) · gap #2
> **Source of truth: Holochain DHT.** A Lens *is* a notarized `Mishpat::Commitment` (action
> `author-lens`); `author-lens.schema.json` is the commitment *payload* contract, **not** a storage
> table. Its storage projection is the `lenses` A-class table (T3); `cid == entry_hash` (I5/A5).
- **Test first:** a Mishpat unit/sweettest that `create_commitment{action:"author-lens", payload:…}`
  succeeds for a valid lens payload and is **rejected** for a malformed one (closed-coordinator default).
- **Implement:** `author-lens.schema.json` (I1); `validate_author_lens(&payload)` + arm at
  `commitments.rs` `validate_commitment_payload` (template `sets-authority-arc`). Assert `cid ==
  entry_hash` in the output. No integrity arm (A4/A5).
- **Verify:** sweettest green on a 1-conductor `household-nodes` stack.

### T2 — ≥2 facing-lenses as folds (the sensemaking half, plural) · gap #1
- **Test first:** `lens_affinity.rs` unit tests over a fixed `LensSelectionRow` slice — two lenses
  (`georgist`, `beerian`) yield two independent readings; **no collapse**; `BTreeMap` order deterministic.
- **Implement:** `affinity_by_lens` / `affinity_for` (I2) + `pub mod lens_affinity;`; seed two concrete
  lens payloads as fixtures.
- **Verify:** facings crate tests + the diesel-guard test still passes (purity preserved).

### T3 — Lens A-class projection (lenses table + parse + signal-arm + CRUD) · gap #2 (storage leg)
- **Test first:** a storage test: a `CommitmentCommitted{action:"author-lens"}` signal upserts a `lenses`
  row with `cid` PK = entry_hash and `dht_anchor_hash = Some(action_hash)`; re-delivery is
  anchor-preserving.
- **Implement:** `lenses` migration (A-class template, `-- Source of truth: Holochain DHT … Classification
  A`); `parse_author_lens` in `mishpat_projection.rs`; extend the `signals.rs` `CommitmentCommitted` arm;
  `db/lenses.rs` `upsert_with_anchor` (template `mishpat_commitments.rs`). Per-row `Err => warn!` skip.
- **Verify:** storage tests green.

### T4 — AffinityFold C-class table + recompute path · gap #3
- **Test first:** `lens_affinity` C-class table round-trips; the recompute path (re-fold over selection
  rows) reproduces the stored aggregate byte-for-byte (the integrity-derivation contract).
- **Implement:** `lens_affinity` migration (C-class template `concentration_snapshot`, **no
  dht_anchor**, slug PK); `services/lens_affinity_facing.rs` (DB select → `affinity_by_lens` fold → view).
- **Verify:** storage tests; recompute == stored.

### T5 — ContentionFold (controversy spread) C-class + table · gap #4 (score)
- **Test first:** `contention_index` over a `LensVerdictRow` slice: 500↑/500↓ ⇒ high; 1000↑/0↓ ⇒ low
  (the spec §8 controversy semantics, not net score).
- **Implement:** `lens_contention.rs` folds (I2); `epr_contention` C-class table + `lens_contention_facing.rs`.
- **Verify:** facings + storage tests.

### T6 — RegimeDriftTrigger → `contention-breach` signal · gap #4 (breach, the net-new primitive)
- **Test first:** `classify_regime` returns `Breached` **only** on the joint `affinity-decay ∧
  contention-rise` (I3); each single-surface case returns `Stable`/`Drifting`.
- **Implement:** `classify_regime`; `"contention-breach"` in `manifest.json` `signalKinds`; a sweep
  (template `MutualityAuditService::run_sweep`) that emits the extension signal via
  `project_extension_signal` + an idempotent audit-log row; wire the conductor-signing emit
  (`signal_emit.rs`, the shared stub from A6).
- **Verify:** storage test asserts the breach fires on the joint condition and the signal carries an
  `evidence_cid`.

### T7 — Ballot B2 (private + attested tally) · gap #5
- **Test first:** a raw ballot stays a private source-chain entry (no DHT gossip); `certify_tally` issues a
  notarized imagodei Attestation; affinity/contention read the **tally**, never raw counts (gaming-resistance).
- **Implement:** `cast_ballot` via the imagodei Attestation path; `certify_tally`; `ballot_tallies`
  A-class table.
- **Verify:** sweettest (private ballot not on DHT; tally attestation present).

### T8 — Manual election (`ratifies-election`) · gap #6
- **Test first:** `create_commitment{action:"ratifies-election"}` records a certified outcome; rejected if
  malformed; authority is the collective franchise (not the EPR-holder).
- **Implement:** `validate_ratifies_election` + arm; `elections` A-class table + projection; reuse
  `CommitmentByState` for the outcome link.
- **Verify:** sweettest.

### T9 — Bounty stub (`bounty-fresh-lens`) · gap #7
- **Test first:** `create_commitment{action:"bounty-fresh-lens", bounded_by:<breach>, in_scope_of:<epr
  slug-id>}` records a bounty; an REA `EconomicEvent` can reference it as fulfillment.
- **Implement:** `validate_open_bounty` + arm (reuse `Mishpat::Commitment`); `lens_bounties` A-class table.
- **Verify:** sweettest.

### T10 — LensSelector + forward index + per-lens fail-closed degrade · gaps #8, #11(partial)
- **Test first:** `find_lenses_governing_epr(epr_slug_id)` returns all governing lenses (the forward index,
  A3); `select_lenses` skips an invalid lens row (degrades that row only, never empties); ranks by affinity.
- **Implement:** `db/lens_bindings.rs::find_lenses_governing_epr` (SQL scope projection, template
  `find_active_delegates_compute`, slug-id keyed); `select_lenses` (I2); the storage-side `filter_map`-skip
  loader.
- **Verify:** storage test incl. a poisoned-row case proving fail-closed-per-row.

### T11 — ts-rs views + export_bindings · type-boundary
- **Test first:** `cargo test export_bindings` from `elohim/elohim-views/` regenerates lens binding /
  affinity / contention TS with no drift (sha256-stable).
- **Implement:** `elohim-views/src/lens.rs` (I6) + `views_convert/` From-impls.
- **Verify:** generated TS lands in `sdk/storage-client-ts/src/generated/`; schema-contract test green.

### T12 — a2o qahal scenarios + Mishpat sweettest · gap #9
- **Test first (these ARE the spec):** `genesis/a2o/features/qahal/` — (a) plural-observation: two lenses,
  two valid readings, no collapse; (b) election-on-conflict; (c) contention→bounty. Plus a Mishpat
  sweettest: the deterministic `author-lens` contract fires on an EPR signal.
- **Implement:** wire steps to the routes/projections from T1–T10.
- **Verify:** a2o green on `household-nodes`; **then render the loop live on a household node** (CI-green ≠
  binding-correct) — Wave-1 DoD.

---

## Definition of Done (Wave-1)
1. T1–T12 green; gap-items #1–#9 flip OPEN→verified (a checked box is a claim — verify, don't assert).
2. Per-touched-tree gates pass (clippy `-D warnings`, fmt, `export_bindings`, sweettest, a2o).
3. **No DNA hash move** — confirm the mishpat DNA hash is unchanged vs `origin/dev` (hash-neutral proof).
4. The loop renders live on a `household-nodes` stack (author 2 lenses → plural read → ballot/tally →
   manual election → contention → bounty).
5. Spec §5/§13 slug-id follow-up (A6) filed to backlog.

## Wave-1 task checklist (decomposition surface — one line-item per task)
- [ ] T1 — Lens payload schema + `author-lens` validator (teeth); sweettest; `cid==entry_hash` (gap #2) @requires:household-nodes
- [ ] T2 — ≥2 facing-lenses as folds (plural sensemaking, no collapse) (gap #1) @requires:household-nodes
- [ ] T3 — `lenses` A-class projection: migration + parse + signal-arm + anchor-preserving CRUD (gap #2 storage) @requires:household-nodes
- [ ] T4 — AffinityFold C-class table + integrity-recompute path (gap #3) @requires:household-nodes
- [ ] T5 — ContentionFold (controversy spread) C-class table (gap #4 score) @requires:household-nodes
- [ ] T6 — RegimeDriftTrigger `classify_regime` (joint predicate) + `contention-breach` manifest signal + wired emit (gap #4 breach) @requires:household-nodes
- [ ] T7 — Ballot B2 (private source-chain + attested tally); gaming-resistant inputs (gap #5) @requires:household-nodes
- [ ] T8 — Manual election (`ratifies-election` validator + `elections` A-class table) (gap #6) @requires:household-nodes
- [ ] T9 — Bounty stub (`bounty-fresh-lens` reusing Commitment + `lens_bounties` table) (gap #7) @requires:household-nodes
- [ ] T10 — LensSelector + forward index (slug-id SQL projection) + per-lens fail-closed degrade (gaps #8,#11) @requires:household-nodes
- [ ] T11 — ts-rs views (lens binding/affinity/contention) + `export_bindings` drift-free @requires:household-nodes
- [ ] T12 — a2o qahal scenarios (plural/election/contention→bounty) + Mishpat sweettest; live household render (gap #9) @requires:household-nodes

## Waves 2–4 forward-compatibility (why these seams hold)
- **Wave 2** (telos-fitness selector + forward index at scale): extends `select_lenses` + the SQL scope
  projection — **no rework**, hash-neutral (A3).
- **Wave 3** (automated breach→bounty→shadow-evaluate→promote across collectives): adds a shadow-eval
  consumer of the *same* folds + the *same* `contention-breach` signal; `@requires:alpha-cluster-6peer`.
- **Wave 4** (recursive method-lens selection + constitutional-floor hardening): the method-lens is just
  another `author-lens` Commitment + fold; only floor-hardening in the integrity zome is hash-moving
  (operator-gated).
