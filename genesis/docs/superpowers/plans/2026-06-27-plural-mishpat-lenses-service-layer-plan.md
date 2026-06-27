---
title: "Plural Mishpat Lenses over an EPR — service-layer vertical slice (DNA teeth → DAO → service → route)"
id: plural-mishpat-lenses-service-layer-plan
status: Draft
class: protocol-canonical
domain: D7
sprint: vision-deferred   # D7 collective-governance ranks below the household seed; this is the read+write vertical proof, not a scheduled sprint
topic: [governance, mishpat, lens-market, service-layer, facing-adapter, dao, diesel, projection, author-lens, affinity, contention, regime-drift, hash-neutral, vertical-slice, household-nodes]
refines:
  - genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-wave1-plan.md
  - genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
cites:
  - plural-mishpat-lenses-wave1-plan | the parent Wave-1 plan; THIS plan sharpens its storage legs (T1/T3/T4/T5/T10/T11) into one bottom-up testable vertical slice and DEFERS its T6-emit/T7/T8/T9 (breach-emit, ballots, elections, bounty) | path: genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-wave1-plan.md
  - plural-mishpat-lenses-over-epr-design | the charter spec; the entity model (I5 classes), the binding-key (A3 slug-id), and the §8 regime-drift fusion | path: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
  - resilience-facings-select-fold-aggregate-design | the select-fold-aggregate facing idiom (free-fn static dispatch, no-diesel firewall) the lens facing-adapter follows | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - elohim/elohim-storage/src/db/mishpat_commitments.rs
  - elohim/elohim-storage/src/db/models.rs
  - elohim/elohim-storage/src/services/mishpat_commitment_facing.rs
  - elohim/elohim-storage/src/mishpat_projection.rs
  - elohim/elohim-storage/src/signals.rs
  - elohim/elohim-storage/src/api/mod.rs
  - elohim/elohim-storage/src/api/resilience.rs
  - elohim/elohim-storage/src/views_convert/lamad.rs
  - elohim/elohim-views/src/lens.rs
  - elohim/elohim-facings/src/folds/lens_affinity.rs
  - elohim/elohim-facings/src/folds/lens_contention.rs
  - elohim/elohim-facings/src/folds/lens_selector.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
  - elohim/sdk/schemas/v1/views/lens-market-view.schema.json
# Mixed-env plan (CLAUDE.md scope convention): NO doc-level requires_env. Every task validates on the
# household-nodes class (local hc:start:seed conductor+storage+doorway, or the live matthew pod).
---

# Plural Mishpat Lenses — service-layer vertical slice

**Compose, don't fork.** This plan does **not** re-spec anything. It takes the
[Wave-1 plan](2026-06-27-plural-mishpat-lenses-wave1-plan.md)'s storage legs and re-orders them as **one
bottom-up, end-to-end testable vertical slice**: the *service layer the operator asked for*, plus everything
underneath it that does not yet exist. On-disk audit (2026-06-27) confirms the scaffold commit `b8e902e6d`
landed only the **pure ends** — folds (`elohim-facings`), the contract (schema + `LensMarketView`/
`LensBindingView` + `contention-breach` signal + generated TS), and the docs. **The DAO does not exist**: no
tables, no migrations, no `db/*.rs`, no `find_lenses_governing_epr`, no projection, no facing service, no
route. So "the service layer above the DAO" requires building the DAO underneath it.

**Slice boundary (operator decision 2026-06-27 — "include the DNA teeth"):** the full **author → project →
DAO → service → serve** read+write loop. **IN:** the `author-lens` coordinator arm (teeth), the projection
write-arm, the DAO read+write, the facing service, the composite assembler, the route, the ts-rs leg.
**OUT (deferred to the Wave-1 remainder — captured §A6):** elections (T8), ballots (T7), bounty write-path
(T9), and the active `contention-breach` *emit* sweep (T6-emit). Consequence: `regime_status` is rendered
**read-only** (via `classify_regime`), `open_bounty_cid` is always `None` until the bounty leg lands, and
affinity/contention render over whatever selection/verdict rows exist (zero ⇒ a still-valid plural market:
*two lenses surfaced side-by-side, no collapse* — which is the spec's headline demo).

**Why this slice is safe to build now:** it is **fully hash-neutral** (the `author-lens` arm is a coordinator
hot-swap; everything else is storage + manifest data — see §A3), so it deploys without a DNA reinstall and is
provable on `household-nodes`.

---

# PART A — Architecture (seams; inherited from Wave-1, restated for this slice)

## A1. Crate placement (unchanged from Wave-1 A1 — fold into existing homes, no new crate)

| Piece | Home (new files marked +) |
|-------|---------------------------|
| `author-lens` validator (teeth) | `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs` (new `validate_author_lens` + arm) |
| Projection parse + signal-arm | `elohim/elohim-storage/src/mishpat_projection.rs` (`parse_author_lens`) + `signals.rs` (extend the `CommitmentCommitted` arm — **no new signal type**) |
| DAO: Queryable/Insertable structs | `elohim/elohim-storage/src/db/models.rs` (mirror `MishpatCommitment` @ ~L590) |
| DAO: tables + CRUD + scope query | `db/lenses.rs` +, `db/lens_bindings.rs` +, `db/lens_affinity.rs` +, `db/epr_contention.rs` + + migrations |
| **Service layer** (the ask) | `elohim/elohim-storage/src/services/lens_facing.rs` + (mirror `mishpat_commitment_facing.rs` @ L63–82) |
| Composite assembler | `elohim/elohim-storage/src/views_convert/lens.rs` + (`build_lens_market_view`; register in `views_convert/mod.rs`) |
| HTTP route | `elohim/elohim-storage/src/api/lens.rs` + + dispatch arm in `api/mod.rs` (~L170) |
| ts-rs views / signal | already landed (`elohim-views/src/lens.rs`, `manifest.json`) — codegen only |

## A2. The contract is the schema + manifest (backend-authoritative — already landed)

The front↔back contract is the **view schema** (`sdk/schemas/v1/views/lens-market-view.schema.json`, SoT for
field names, validated by `schema_contract.rs::lens_market_view_matches_schema`) plus the **manifest signal
vocabulary** (`sdk/domains/elohim/manifest.json → contention-breach`). Both shipped in `b8e902e6d`. The Rust
view (`elohim-views/src/lens.rs`) CONFORMS; ts-rs `export_bindings` + `schema:codegen:ts` are the mechanical
TS projections. **This plan authors NO new wire shape** — it fills the body underneath an already-fixed
contract. The front end senses/inspires; the backend decides — the seam is locked, so the service body falls
out against it.

## A3. Projection-class + binding-key contract (the two load-bearing rules; spec I5 / A3)

- **A-class** (`lenses`): `cid TEXT PRIMARY KEY` (= **entry_hash**, the read/scope key — *never* action_hash),
  nullable `dht_anchor_hash` (NULL ⇒ un-notarized, fail-closed), source-of-truth = DHT. Migration template:
  `2026-06-09-000000_mishpat_commitments/up.sql` (has `dht_anchor_hash TEXT`). Upsert template:
  `db/mishpat_commitments.rs::upsert_with_anchor` (L35–94, conditional anchor rewrite).
- **C-class** (`lens_affinity`, `epr_contention`): **no `dht_anchor_hash` column**, slug PK, `Queryable +
  Serialize` only, recompute path = a facing fold. Migration template:
  `2026-06-10-020000_concentration_snapshot/up.sql` ("DELIBERATELY NOT DHT-ANCHORED").
- **Binding key = EPR slug-id** (`epr:lamad-spa`), NOT the dag-cbor CID. `find_lenses_governing_epr` mirrors
  `db/mishpat_commitments.rs::find_active_delegates_compute` (L172–187: filter action + scope slug + state +
  revocation + anchor-not-null, order desc) — a pure SQL scope projection, zero new DHT anything.

## A4. Hash-neutrality ledger (the deployment contract for THIS slice)

| New thing | Class | Verdict |
|-----------|-------|---------|
| `author-lens` arm + `validate_author_lens` | coordinator action | **HASH-NEUTRAL** (`update_coordinators` hot-swap) |
| `parse_author_lens` + `CommitmentCommitted` signal-arm | storage projection | **HASH-NEUTRAL** |
| `lenses`/`lens_affinity`/`epr_contention` tables + CRUD + `find_lenses_governing_epr` | storage | **HASH-NEUTRAL** |
| `lens_facing.rs` + `build_lens_market_view` + `api/lens.rs` route | storage | **HASH-NEUTRAL** |
| ts-rs codegen of the landed views | codegen | **HASH-NEUTRAL** |

**No integrity bytecode is touched** ⇒ no DNA hash move ⇒ deploy by coordinator hot-swap, no agent re-key.

## A5. Gospel guardrails (non-negotiable — carried from Wave-1 A5)

- **`cid == entry_hash`** (read/scope key); `action_hash` is *only* `dht_anchor_hash`. Returning action_hash
  as CID passes per-task tests but silently breaks every scope projection.
- **Closed coordinator default** — `author-lens` MUST get a `validate_commitment_payload` arm or
  `create_commitment` rejects it; do **NOT** add a `commitment_action_requirements` integrity arm (that is
  the hash-moving line). Commitments are immutable → lens versioning = new `create` + `version_parent`.
- **Fail-closed per row** — an unparseable/invalid lens degrades *its own row* (`filter_map` + `warn!`,
  template `db/rea_commitments.rs`), never empties the lens set (the EprRouter lesson:
  `[[project_epr_router_empties_on_poisoned_scope]]`).
- **Cross-namespace `h_app_id` consistency** — selection/affinity rows that join household/agent data use one
  consistent `h_app_id`; never raw-compare `agent_cid` against a transport id (the resilience-card dormancy
  precedent: `[[project_dataplane_next_lens_diversity_placement]]`).
- **NO `is_service_path` gate in elohim-storage** — that shadow gate is doorway-only
  (`[[project_doorway_main_route_needs_is_service_path]]`); storage routes match inline via the `api/mod.rs`
  prefix dispatcher. (If/when doorway proxies `/api/v1/epr/.../lens-market`, the doorway gate applies there.)

## A6. Complementary work captured (kept OUT of this slice's scope — backlog)

- **Ballots (T7), elections (T8), bounty write-path (T9), contention-breach emit-sweep (T6-emit)** — the
  Wave-1 remainder. This slice renders `regime_status` read-only and `open_bounty_cid: None`; those legs make
  affinity/contention non-trivial (real intensity input) and light the active breach→bounty renewal loop.
  → already enumerated as Wave-1 plan tasks; no new backlog row needed, but note the dependency: the bounty
  field on `LensMarketView` stays `None` until T9.
  **Frontend sensing (operator note 2026-06-27):** the human voting/intensity *input* surface is **already
  rendered by `psephos`** in the sophia package (`sophia/packages/psephos` — widgets: `dot-vote`,
  `score-vote`, `approval`, `consent`, `ranked-choice`; `psephos-element` web-component bundle;
  `psephos-ballot` skill + `psephos-ballot.schema.json`). Completeness unverified — *sense before building*
  (`[[feedback_frontend_review_eyes_first]]`: render it first). When T7/T8 land, the ballot/election input is
  a **binding to psephos** (its score/RCV/quadratic widgets carry the intensity signal `contention_index`
  reads — spec §8), NOT a new input control. Backend stays authoritative
  (`[[feedback-backend-authoritative-frontend-senses]]`): the ballot **tally** is the B2 attestation the
  folds read; psephos is the sensing/capture layer that inspires the payload shape, never dictates it.
- **Selection write-path** — affinity folds over `LensSelectionRow` (who exercised which lens). This slice
  seeds selection rows directly in the integration test (S5) to prove the fold renders ranked affinity; the
  *production* selection endpoint (a B2 attestation or C-class operational record) is its own slice. → backlog
  one-liner: `lens-selection-write-path-slice`.

---

# PART B — Implementation (bottom-up; TDD: failing test → implement → verify)

Discipline: **TDD**. Per-touched-tree pre-push gates per task (`[[feedback_sprint_dod_includes_prepush_gates]]`):
storage → `RUSTFLAGS="" cargo nextest run`/`clippy -D warnings`/`fmt` with `CARGO_TARGET_DIR` set
(`[[feedback_cargo_target_dir_for_native_builds]]`); zome → sweettest (`RUSTFLAGS=""`, `just pack`); views →
`cargo test export_bindings`. **CI-green ≠ binding-correct** — the slice closes only on a live `household-nodes`
render (S9).

### S1 — `author-lens` validator (the teeth) · Wave-1 T1
- **Test first:** a Mishpat sweettest: `create_commitment{action:"author-lens", payload:<valid>}` succeeds and
  `<malformed>` is **rejected** (closed-coordinator default); assert the returned `cid == entry_hash`.
- **Implement:** `validate_author_lens(&payload)` + arm in `commitments.rs::validate_commitment_payload`
  (template the existing `delegates-compute`/`sets-authority-arc` arms). Author the payload contract as
  `sdk/schemas/v1/commitments/author-lens.schema.json` (mirror `delegates-compute.schema.json`). **No
  integrity arm** (A4/A5).
- **Verify:** sweettest green on a 1-conductor stack; mishpat DNA hash unchanged vs `origin/dev`.

### S2 — `lenses` A-class DAO (table + structs + anchor upsert) · Wave-1 T3 (DAO leg)
- **Test first:** `db/lenses.rs` test — `upsert_with_anchor` inserts a row with `cid` PK = entry_hash and
  `dht_anchor_hash = Some(action_hash)`; re-delivery with a later anchor is anchor-preserving (no clobber to
  NULL); `find_by_cid` round-trips.
- **Implement:** migration `…_lenses/up.sql` (A-class template, header `-- Source of truth: Holochain DHT …
  Classification A`); `Lens`/`NewLens` structs in `db/models.rs` (mirror `MishpatCommitment` @ ~L590);
  `db/lenses.rs::{upsert_with_anchor, find_by_cid, list_in_scope}` (template `db/mishpat_commitments.rs`).
- **Verify:** storage tests green.

### S3 — Projection write-arm (parse + signal-arm) · Wave-1 T3 (write leg)
- **Test first:** a storage test feeds a **synthetic** `CommitmentCommitted{action:"author-lens", payload,
  action_hash, entry_hash}` and asserts a `lenses` row is upserted (PK=entry_hash, anchor=Some). A malformed
  payload `warn!`-skips that row only (never panics, never empties).
- **Implement:** `parse_author_lens` in `mishpat_projection.rs::parse_commitment_payload`; extend the existing
  `CommitmentCommitted` arm in `signals.rs` to route `author-lens` → `db::lenses::upsert_with_anchor`.
  Per-row `Err => warn!` skip (template `db/rea_commitments.rs`).
- **Verify:** storage tests green; the projection is exercisable **without the DNA** (synthetic signal), so
  S3 stands alone.

### S4 — Forward index + C-class tables (the rest of the DAO) · Wave-1 T4/T5/T10 (DAO legs)
- **Test first:** `find_lenses_governing_epr(conn, epr_slug_id)` returns exactly the lenses bound to that
  slug-id (ignores other scopes, revoked, anchor-NULL); the `lens_affinity` + `epr_contention` C-class tables
  round-trip and have **no `dht_anchor_hash` column** (assert via schema introspection or a compile-time
  Selectable that omits it).
- **Implement:** `db/lens_bindings.rs::find_lenses_governing_epr` (slug-id SQL scope projection, template
  `find_active_delegates_compute` @ L172–187); C-class migrations for `lens_affinity` + `epr_contention`
  (template `concentration_snapshot`); their `Queryable` structs + read fns (`affinity_rows_in_scope`,
  `contention_rows_in_scope`).
- **Verify:** storage tests; a poisoned/extra-scope row proves the query is correctly scoped.

### S5 — Service layer: `lens_facing.rs` (DB → fold → `LensBindingView`) · the operator's ask (a)
- **Test first:** seed (directly) `lenses` + `lens_affinity` selection rows for scope `epr:lamad-spa` with
  two lenses (`georgist`, `beerian`); `build_lens_bindings(conn, scope)` returns **two** `LensBindingView`s,
  affinity-ranked (`lens_affinity::affinity_by_lens` fold), `BTreeMap`-deterministic, **no collapse**; a
  poisoned lens row degrades to `valid:false` (surfaced, excluded from rank) — never empties the vec.
- **Implement:** `services/lens_facing.rs` (mirror `mishpat_commitment_facing.rs` L63–82): `load_*`
  (impure, `&mut conn`, `Err => warn! + Vec::new()`), `to_view` per-row (→ `LensBindingView`), `build_*`
  orchestrator. Imports `elohim_facings::folds::{lens_affinity, lens_contention}` + `elohim_views::*`.
- **Verify:** storage tests; fail-closed-per-row case green.

### S6 — Service layer: composite assembler (`build_lens_market_view`) · the operator's ask (b)
- **Test first:** `build_lens_market_view(conn, scope)` assembles a full `LensMarketView`: `lenses` (S5),
  `contention_index` (`lens_contention::contention_index` fold), `regime_status` (`lens_selector::
  classify_regime` over current vs prior C-class snapshot → `stable|drifting|breached`), `open_bounty_cid:
  None`, `computed_at` (RFC3339, injected — not `Date::now()` in a fold). For an unknown scope it returns an
  **empty-but-valid** market (lenses `[]`, contention `0.0`, regime `stable`), never an error.
- **Implement:** `views_convert/lens.rs::build_lens_market_view` (register `pub mod lens;` in
  `views_convert/mod.rs`). Time injected by the caller (handler) for testability.
- **Verify:** storage tests + `schema_contract.rs::lens_market_view_matches_schema` green (the assembled
  shape validates against the **already-landed SoT contract** — `lens-market-view.schema.json` from
  `b8e902e6d`; this task authors NO new schema, source-of-truth is unchanged).

### S7 — HTTP route `GET /api/v1/epr/{scope}/lens-market` · new (read-projection; **entry type = the S1 `author-lens` DHT commitment**, route follows it)
- **Test first:** a handler/integration test: `GET /api/v1/epr/epr:lamad-spa/lens-market` → `200` + body that
  jsonschema-validates against the landed SoT contract `lens-market-view.schema.json`; an unknown scope →
  `200` empty-valid market (not `404`). This route is a pure read-projection of the A-class `lenses` table
  (notarized in the DHT via S1) — it adds NO entry type and NO new source-of-truth.
- **Implement:** `api/lens.rs::handle(req, method, resource_path, pool, ctx, …)` (signature template
  `api/resilience.rs` L25–60); parse `{scope}` from the path, `get_conn(pool)?`, call
  `build_lens_market_view`, `response::ok(&view)`. Add the dispatch arm in `api/mod.rs` (~L170) routing the
  `epr/.../lens-market` suffix to `lens::handle` (NOT a doorway `is_service_path` change — storage is inline).
- **Verify:** route test green; manual `curl` against a local storage stack returns the shape.

### S8 — ts-rs codegen + From-impls drift-free · Wave-1 T11
- **Test first:** `cargo test export_bindings` from `elohim/elohim-views/` regenerates `LensMarketView.ts` /
  `LensBindingView.ts` with **no drift** (sha256-stable vs the committed generated files); `pnpm run
  schema:codegen:ts` clean.
- **Implement:** any `views_convert/lens.rs` From-impls needed; ensure `INTERFACE_FILES` (already has the
  entry) + the pre-push freshness gate pass.
- **Verify:** generated TS byte-identical to committed; `schema_contract` green.

### S9 — Live proof on `household-nodes` (the real DoD) · Wave-1 T12 (subset)
- **Test first (this IS the spec):** `genesis/a2o/features/qahal/` scenario — *two lenses authored over one
  EPR both surface in the market, side by side, no collapse* (the plural-observation headline). `@requires:
  household-nodes`.
- **Implement:** wire the a2o steps to the S7 route; author two real lenses through the S1 `author-lens` path
  (real signal → S3 projection → S2 table), then GET the market.
- **Verify:** a2o green, **then render the loop live** on a `household-nodes` stack (`pnpm hc:start:seed` or
  the matthew pod): author 2 lenses → `GET …/lens-market` returns both, plural. CI-green ≠ binding-correct.

---

## Definition of Done (this slice)
1. S1–S9 green; the author→project→DAO→service→serve loop renders two co-valid lenses over one EPR.
2. Per-touched-tree gates pass (sweettest, `clippy -D warnings`, `fmt`, `export_bindings`, `schema_contract`,
   a2o).
3. **No DNA hash move** — mishpat DNA hash unchanged vs `origin/dev` (the hash-neutral proof; A4).
4. The market renders **live on `household-nodes`** (not just CI).
5. Deferred legs (ballots/elections/bounty/breach-emit, selection write-path) remain captured (§A6), so
   `regime_status` read-only + `open_bounty_cid: None` are *intended*, not gaps.

## Task checklist (decomposition surface — one line-item per task)
- [ ] S1 — `author-lens` validator arm + payload schema (teeth); sweettest; `cid==entry_hash`; no integrity arm @requires:household-nodes
- [ ] S2 — `lenses` A-class DAO (**source of truth: Holochain DHT**, projected from the S1 `author-lens` entry; A3): migration + `db/models.rs` structs + `db/lenses.rs` anchor-preserving upsert @requires:household-nodes
- [ ] S3 — projection write-arm: `parse_author_lens` + `CommitmentCommitted` signal-arm → upsert; per-row warn-skip @requires:household-nodes
- [ ] S4 — forward index `find_lenses_governing_epr` (slug-id SQL projection) + `lens_affinity`/`epr_contention` C-class tables (**source of truth: operational/reconstructable, NOT notarized**; A3) @requires:household-nodes
- [ ] S5 — service layer `services/lens_facing.rs`: DB→fold→`LensBindingView`, affinity-ranked, fail-closed per row @requires:household-nodes
- [ ] S6 — composite assembler `build_lens_market_view` → `LensMarketView` (contention + classify_regime + empty-valid); schema_contract green @requires:household-nodes
- [ ] S7 — HTTP route `GET /api/v1/epr/{scope}/lens-market` (**route FOLLOWS the S1 `author-lens` DHT entry type**, not vice-versa; read-projection of the A-class `lenses` table): `api/lens.rs` handler + `api/mod.rs` dispatch arm; unknown scope → empty-valid @requires:household-nodes
- [ ] S8 — ts-rs `export_bindings` + codegen drift-free (sha256-stable); pre-push freshness gate @requires:household-nodes
- [ ] S9 — a2o qahal plural-observation scenario + live `household-nodes` render of the loop @requires:household-nodes
