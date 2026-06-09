# substrate_signal Migration (Cluster #3, Slice 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.
> **Execute SEQUENTIALLY — this is a dependency chain, not parallel work** (the wall lands before the
> field; the field threads through one chokepoint before the writers). Do NOT fan out.

**Goal:** Land a first-class, validator-gated `substrate_signal` field on the `EconomicEvent` notarized
entry and thread it end-to-end (DHT entry → validator → SQL projection → wire View), single-value
`"attention"`, on household-nodes — the hard prerequisite for the Cluster #1 limitarian governor.

**Architecture:** `substrate_signal` is a typed `Option<String>` on the integrity struct, gated by a
direct whitelist check (the DNA "wall," reject-at-write). It threads through the **one chokepoint**
`CreateEconomicEventInput` into the `NewEconomicEvent` Insertable; because the Insertable is a typed
struct literal, **adding the field makes every construction site a compile error** — so both production
writers (`record_event` for the HTTP/service path; `upsert_with_anchor` for the DHT-projection path) are
forced to be updated. The generic `project()`/columnMapping path is test-only (`with_shefa_*` is under
`#[cfg(test)]`) and is updated last for fixture parity, never as the landing.

**Tech Stack:** Rust (Holochain HDI integrity zome, `wasm32-unknown-unknown`); Diesel/SQLite (elohim-storage);
ts-rs (elohim-views → generated TS); a2o/Cucumber (already covers the experience).

**Environment (per root + crate CLAUDE.md + memory):**
- DNA zome (`content_store_integrity`): build/pack with `just check` / `just pack` (RUSTFLAGS is in the
  justfile — **do not override**). `just pack` (NOT `just build`) refreshes the `.dna` bundle
  (`project_sweettest_native_build_env`).
- elohim-storage: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test …`. **No `nextest` in this
  container** (`project_container_cargo_environment_quirks`) — plain `cargo test`. Set `CARGO_TARGET_DIR`
  to the pool slot for this worktree (SessionStart preflight prints it).
- ts-rs: `cargo test export_bindings` runs **in `elohim/elohim-views`**, NOT elohim-storage.

**⚠ DNA-hash change (Task 3):** adding the integrity-struct field changes the DNA hash. v1 is
household-nodes only (matthew/jessica/james — the operator controls all peers). The alpha genesis pair
(adam+matthew) reinstall under `ALLOW_DNA_REINSTALL` is an **operator-gated step, NOT in this plan**
(`project_alpha_topology_bootstrap_pair`; mint-new-key/lineage cost). Do not deploy this to alpha as part
of the slice.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` | integrity struct + validator + const re-export | re-export `SUBSTRATE_SIGNALS`; add field; add whitelist check |
| `…/content_store_integrity/src/generated_enums.rs` | codegen'd const | (read-only — `CORE_SUBSTRATE_SIGNALS` already exists at :372) |
| `elohim/elohim-storage/migrations/<ts>_add_substrate_signal/{up,down}.sql` | SQL column | new migration |
| `elohim/elohim-storage/src/db/models.rs` | `NewEconomicEvent` Insertable | add `substrate_signal` (forces writer updates) |
| `elohim/elohim-storage/src/db/economic_events.rs` | `CreateEconomicEventInput` (chokepoint) + `record_event` + `upsert_with_anchor` | add field; thread into both writers |
| `elohim/elohim-storage/src/rea_projection.rs` | DHT-event → `CreateEconomicEventInput` builder (:463) | set `substrate_signal` from the event |
| `elohim/elohim-views/src/shefa.rs` | `EconomicEventView` (read) + `CreateEconomicEventInputView` (write) | add `substrate_signal` |
| `elohim/elohim-storage/src/views_convert/inputs.rs:134` | `From<CreateEconomicEventInputView>` | thread the field |
| `elohim/sdk/schemas/v1/views/economic-event-view.schema.json` (+ input schema) | wire-shape contract | add optional `substrateSignal` |
| `elohim/elohim-storage/src/projector/mapping.rs:176` | fixture column-mapping (test-only) | add line LAST, for parity |

---

## Task 0: Story-first anchor (no new gherkin)

**Files:** read-only — `genesis/a2o/features/lamad/attention-analytics.feature`

- [ ] **Step 1: Confirm the experience is already covered.** `attention-analytics.feature:13-19` asserts
  *content-view → economic event, action "use", resource type "attention"* — the human-facing anchor this
  migration serves. Per `genesis/a2o/CLAUDE.md` ("what belongs in feature files"), `substrate_signal` is a
  **layer-internal contract** (a notarized field + validator + projection), NOT a new lived experience —
  so it is verified by unit / schema-contract / projection tests below, **not** a new scenario. Do NOT
  author a new gherkin scenario for the field (that doc warns against scenario-shaped objects that carry
  no story). No code change in this task — it is the story-first gate, satisfied by the existing feature.

---

## Task 1: Wire the `SUBSTRATE_SIGNALS` const (the dead-const's first consumer)

**Files:** Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:66-71`

- [ ] **Step 1: Add the re-export alias.** In the `pub use generated_enums::{…}` block (lib.rs:66-71),
  add `CORE_SUBSTRATE_SIGNALS as SUBSTRATE_SIGNALS,` alongside the others. This gives the schema's
  `_dna.constant: "SUBSTRATE_SIGNALS"` (`substrate-signal.schema.json`) its first live binding — exactly
  parallel to `CORE_ENGAGEMENT_TYPES as ENGAGEMENT_TYPES` on the same lines.

```rust
pub use generated_enums::{
    CORE_COMPLETION_CRITERIA as COMPLETION_CRITERIA, CORE_CONTENT_FORMATS as CONTENT_FORMATS,
    CORE_CONTENT_TYPES as CONTENT_TYPES, CORE_ENGAGEMENT_TYPES as ENGAGEMENT_TYPES,
    CORE_MASTERY_LEVELS as MASTERY_LEVELS, CORE_PATH_VISIBILITIES as PATH_VISIBILITIES,
    CORE_REACH_LEVELS as REACH_LEVELS, CORE_STEP_TYPES as STEP_TYPES,
    CORE_SUBSTRATE_SIGNALS as SUBSTRATE_SIGNALS,
};
```

- [ ] **Step 2: Type-check.** Run: `cd elohim/holochain/dna && just check`
  Expected: compiles; `SUBSTRATE_SIGNALS` now resolves (was an unbound name before).

- [ ] **Step 3: Commit.**
```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
git commit -m "feat(dna): wire SUBSTRATE_SIGNALS const re-export (first consumer for the dead const)"
```

---

## Task 2: The validator wall (reject-at-write) — TDD

**Files:** Modify: `…/content_store_integrity/src/lib.rs:4392` (`validate_economic_event`) and the test
module `economic_event_validation_tests` at `:4438`.

- [ ] **Step 1: Write the failing tests** in `economic_event_validation_tests` (lib.rs:4438). Use the
  module's existing `EconomicEvent` constructor/helper; set `substrate_signal` to each case. (If the test
  module builds events via a local helper, add a `substrate_signal` arg defaulting to `None`.)

```rust
#[test]
fn accepts_absent_substrate_signal() {
    let mut e = valid_event();        // existing helper
    e.substrate_signal = None;        // old-chain entries deserialize None
    assert!(matches!(validate_economic_event(&e).unwrap(), ValidateCallbackResult::Valid));
}

#[test]
fn accepts_whitelisted_substrate_signal() {
    let mut e = valid_event();
    e.substrate_signal = Some("attention".to_string());
    assert!(matches!(validate_economic_event(&e).unwrap(), ValidateCallbackResult::Valid));
}

#[test]
fn rejects_unknown_substrate_signal() {
    let mut e = valid_event();
    e.substrate_signal = Some("garbage".to_string());
    assert!(matches!(validate_economic_event(&e).unwrap(), ValidateCallbackResult::Invalid(_)));
}
```

- [ ] **Step 2: Run — expect FAIL to compile** (the field does not exist yet). Run:
  `cd elohim/holochain/dna && just check`
  Expected: compile error `no field substrate_signal on EconomicEvent`. This proves the test is real and
  forces Task 3 (the field) before the wall can compile — the correct ordering.

- [ ] **Step 3: Add the validator check** to `validate_economic_event` (lib.rs:4392), BEFORE the final
  `Ok(Valid)` at :4435. A typed field gets a **direct** whitelist check (no substring idiom — that's only
  for `metadata_json`):

```rust
    if let Some(sig) = event.substrate_signal.as_deref() {
        if !SUBSTRATE_SIGNALS.contains(&sig) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "EconomicEvent.substrate_signal '{sig}' is not a recognized substrate signal"
            )));
        }
    }
```

(This compiles only after Task 3 adds the field. Land Tasks 2-step-3, 3, and 4 together as one
"wall+field" unit — the wall is written first conceptually, but the field must exist for the crate to
build. Commit them as one logical change in Task 3.)

- [ ] **Step 4: deferred to Task 3's build** (the field must exist). See Task 3.

---

## Task 3: Add the field to the integrity `EconomicEvent` struct (DNA-hash change)

**Files:** Modify: `…/content_store_integrity/src/lib.rs:1115-1150` (`struct EconomicEvent`).

- [ ] **Step 1: Add the field.** Append to the struct (after `metadata_json`, before `created_at`):
```rust
    /// Which protocol substrate dimension this event consumed (attention/compute/storage/…).
    /// Validated against SUBSTRATE_SIGNALS; None = unspecified (old-chain compatible).
    pub substrate_signal: Option<String>,
```
(`Option<String>` — NOT `serde_json::Value`; per dna/CLAUDE.md, `Value` on a WASM-boundary struct fails at
runtime. A plain `Option<String>` is safe.)

- [ ] **Step 2: Run the validator tests (now compiles) + pack.** Run:
  `cd elohim/holochain/dna && just check`
  Expected: compiles; `cargo test -p content_store_integrity economic_event_validation` (or via the
  zome's test runner) → the three Task-2 tests PASS (`accepts_absent`, `accepts_whitelisted`,
  `rejects_unknown`).
- [ ] **Step 3: Pack the DNA.** Run: `cd elohim/holochain/dna && just pack`
  Expected: `.dna` bundle refreshed; **DNA hash changes** (expected — household-only; do NOT deploy alpha).

- [ ] **Step 4: Commit (wall + field together).**
```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
git commit -m "feat(dna): add validated substrate_signal field to EconomicEvent (reject-at-write wall)"
```

---

## Task 4: SQL column (Diesel migration)

**Files:** Create: `elohim/elohim-storage/migrations/<unique-ts>_add_economic_event_substrate_signal/{up,down}.sql`

- [ ] **Step 1: Create the migration** with a **unique** `YYYY-MM-DD-HHMMSS` timestamp (collision →
  `embed_migrations!` silently keeps one — `feedback_diesel_migration_timestamp_collision`).
  `up.sql`:
```sql
-- Source of truth: DHT (EconomicEvent notarized entry). This column is a write-through projection.
ALTER TABLE economic_events ADD COLUMN substrate_signal TEXT NULL;
```
  `down.sql`:
```sql
ALTER TABLE economic_events DROP COLUMN substrate_signal;
```

- [ ] **Step 2: Add the column to the diesel schema.** In the `economic_events` `table!` block
  (`elohim/elohim-storage/src/db/diesel_schema.rs`), add `substrate_signal -> Nullable<Text>,`.

- [ ] **Step 3: Build to verify schema parity.** Run:
  `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`
  Expected: compiles (the new column is unused until Task 5 — that's fine).

- [ ] **Step 4: Commit.**
```bash
git add elohim/elohim-storage/migrations/ elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add economic_events.substrate_signal column (write-through projection)"
```

---

## Task 5: Thread through the write path (the compiler enforces every writer) — TDD

**Files:** Modify: `db/models.rs:388` (`NewEconomicEvent`), `db/economic_events.rs` (`CreateEconomicEventInput:21`,
`record_event:272`, `upsert_with_anchor:607`), `rea_projection.rs:463`.

- [ ] **Step 1: Write the failing projection round-trip test** in `db/economic_events.rs`'s test module
  (use the existing test-DB harness in that file). This is the **headline test** — it drives a real
  `CreateEconomicEventInput` through `record_event` and asserts the column persists:
```rust
#[test]
fn substrate_signal_persists_through_record_event() {
    let mut conn = test_conn();                       // existing harness
    let ctx = test_ctx();
    let input = CreateEconomicEventInput {
        substrate_signal: Some("attention".to_string()),
        ..minimal_use_event_input()                   // existing helper; action="use"
    };
    let row = record_event(&mut conn, &ctx, input).expect("record");
    let got: Option<String> = economic_events::table
        .find(&row.id).select(economic_events::substrate_signal)
        .first(&mut conn).expect("select");
    assert_eq!(got.as_deref(), Some("attention"));
}

#[test]
fn absent_substrate_signal_persists_as_null() {
    let mut conn = test_conn();
    let ctx = test_ctx();
    let input = CreateEconomicEventInput { substrate_signal: None, ..minimal_use_event_input() };
    let row = record_event(&mut conn, &ctx, input).expect("record");
    let got: Option<String> = economic_events::table
        .find(&row.id).select(economic_events::substrate_signal)
        .first(&mut conn).expect("select");
    assert_eq!(got, None);   // absent = NULL, no error (intended; distinct from dead-seam misuse)
}
```

- [ ] **Step 2: Run — expect FAIL to compile** (`CreateEconomicEventInput` has no `substrate_signal`).
  Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test substrate_signal_persists -- --nocapture`
  Expected: compile error — proves the test bites.

- [ ] **Step 3: Add the field to `CreateEconomicEventInput`** (economic_events.rs:21, after `metadata_json`):
```rust
    #[serde(default)]
    pub substrate_signal: Option<String>,
```

- [ ] **Step 4: Add the field to `NewEconomicEvent`** (models.rs:388, after `metadata_json`):
```rust
    pub substrate_signal: Option<&'a str>,
```
  This **forces a compile error at every `NewEconomicEvent { … }` literal** — that is the safety net: the
  compiler enumerates every writer so none is silently skipped.

- [ ] **Step 5: Thread the field into BOTH writers** (the compile errors point you at the exact lines).
  In the `NewEconomicEvent { … }` literal inside `record_event` (economic_events.rs:~290) AND inside
  `upsert_with_anchor` (economic_events.rs:~620), add:
```rust
        substrate_signal: input.substrate_signal.as_deref(),
```

- [ ] **Step 6: Set it in the DHT-projection builder.** In `rea_projection.rs:463` where the
  `CreateEconomicEventInput { … }` is built from the committed event, add
  `substrate_signal: event.substrate_signal.clone(),` (reading the field off the notarized
  `EconomicEvent`). If the projection's source event type doesn't yet carry it, thread it from the entry
  shape — it must come from the DHT entry, not be invented.

- [ ] **Step 7: Run the tests — expect PASS.** Run:
  `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test substrate_signal_persists absent_substrate_signal -- --nocapture`
  Expected: both PASS. The field now flows `CreateEconomicEventInput → NewEconomicEvent → SQL` on the real
  writer.

- [ ] **Step 8: Commit.**
```bash
git add elohim/elohim-storage/src/db/economic_events.rs elohim/elohim-storage/src/db/models.rs elohim/elohim-storage/src/rea_projection.rs
git commit -m "feat(storage): thread substrate_signal through CreateEconomicEventInput into both writers"
```

---

## Task 6: The wire boundary (View + InputView + schema + ts-rs)

**Files:** Modify: `elohim/elohim-views/src/shefa.rs:18` (`EconomicEventView`) + the `CreateEconomicEventInputView`
in the same crate; `elohim/elohim-storage/src/views_convert/inputs.rs:134` (`From` impl); the view + input
JSON schemas in `elohim/sdk/schemas/v1/views/`.

- [ ] **Step 1: Schema first** (per elohim-storage/CLAUDE.md "Schema Contract"). In
  `elohim/sdk/schemas/v1/views/economic-event-view.schema.json`, add to `properties` (NOT to `required`):
```json
    "substrateSignal": { "type": ["string", "null"] }
```
  Do the same in the `CreateEconomicEventInputView` schema.

- [ ] **Step 2: Add to `EconomicEventView`** (shefa.rs:18, after `scope_collab_cid`):
```rust
    /// Which protocol substrate dimension this event consumed (camelCase: substrateSignal).
    pub substrate_signal: Option<String>,
```
  And add the same field to `CreateEconomicEventInputView`.

- [ ] **Step 3: Thread the `From` impl** (`views_convert/inputs.rs:134`,
  `From<CreateEconomicEventInputView> for CreateEconomicEventInput`): add
  `substrate_signal: v.substrate_signal,`. And in the `From<EconomicEvent(row)> for EconomicEventView`
  impl, add `substrate_signal: row.substrate_signal,`.

- [ ] **Step 4: Schema-contract test.** Run:
  `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract`
  Expected: PASS (View matches the updated schema).

- [ ] **Step 5: Regenerate TS + sha256-verify the diff.** Run:
  `cd elohim/elohim-views && cargo test export_bindings` then `cd ../.. && pnpm run schema:codegen:ts`
  Expected: `EconomicEventView.ts` + `CreateEconomicEventInputView.ts` gain exactly one optional
  `substrateSignal` field. **Verify**: `git diff --stat` shows only the generated EconomicEvent TS files
  changed; if any *sibling* view's TS changed, a cross-crate move sneaked in — STOP and investigate
  (ts-rs cross-crate trap). This is an in-crate field add, so no `../../../../` import breakage.

- [ ] **Step 6: Commit.**
```bash
git add elohim/elohim-views/src/shefa.rs elohim/elohim-storage/src/views_convert/inputs.rs elohim/sdk/schemas/v1/views/ elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(views): expose substrate_signal on EconomicEvent View + InputView (camelCase, optional)"
```

---

## Task 7: End-to-end DHT→SQL proof + fixture parity + read-side COALESCE note

**Files:** Modify: `rea_projection.rs` test module (the existing `ReaEconomicEventCommitted` test at
`:730`); `projector/mapping.rs:176` (fixture parity).

- [ ] **Step 1: Write the failing end-to-end test** in `rea_projection.rs`'s test module. This drives the
  **production DHT-projection path** (`ReaEconomicEventCommitted → CreateEconomicEventInput → upsert_with_anchor`),
  NOT `project()`:
```rust
#[test]
fn rea_committed_event_projects_substrate_signal_to_sql() {
    let (mut conn, ctx) = projection_test_setup();         // existing harness near :730
    let event = sample_economic_event_with(|e| {
        e.substrate_signal = Some("attention".to_string());
        e.action = "use".to_string();
    });
    handle_signal(&mut conn, &ctx, ReaProjectionSignal::ReaEconomicEventCommitted {
        event, action_hash: sample_action_hash(),
    }).expect("project");
    let got: Option<String> = economic_events::table
        .select(economic_events::substrate_signal).first(&mut conn).expect("row");
    assert_eq!(got.as_deref(), Some("attention"),
        "substrate_signal must reach SQL on the PRODUCTION DHT path, not just the fixture projector");
}
```

- [ ] **Step 2: Run — expect FAIL then PASS** after confirming Task-5 step-6 set the field in the
  builder. Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test rea_committed_event_projects_substrate_signal`
  Expected: PASS. This is the test that proves the field is NOT green-in-CI-absent-in-production.

- [ ] **Step 3: Fixture parity (LAST, test-only path).** Add to `shefa_economic_event_column_mapping`
  (`projector/mapping.rs:176`): `m.insert("substrate_signal".to_string(), "payload.substrateSignal".to_string());`
  so the `#[cfg(test)]` `project()` fixtures match production. This is parity, not the landing.

- [ ] **Step 4: Read-side COALESCE note (no code change here — documents the Cluster #1 contract).** When
  Cluster #1's `ConcentrationSnapshot` aggregates `GROUP BY substrate_signal`, it MUST treat pre-migration
  NULL rows as `attention` via `COALESCE(substrate_signal, 'attention')` (NULLs bucket separately from
  `'attention'` otherwise — the convention cannot live in app prose). Record this in the Cluster #1 plan;
  no backfill in this slice.

- [ ] **Step 5: Full storage test run + commit.** Run:
  `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test`
  Expected: green.
```bash
git add elohim/elohim-storage/src/rea_projection.rs elohim/elohim-storage/src/projector/mapping.rs
git commit -m "test(storage): prove substrate_signal projects on the production DHT path + fixture parity"
```

---

## Self-Review

**Spec coverage** (against `2026-06-09-cluster3-…-design.md` §2): const-wiring ✓ (T1); validator-first
reject-at-write ✓ (T2); field-on-integrity-struct + `just pack` + DNA-hash flag ✓ (T3); column ✓ (T4);
**production-path threading via the chokepoint, both writers compiler-forced** ✓ (T5 — the corrected
does-not-hold fix); wire View + schema + ts-rs sha256 ✓ (T6); **end-to-end DHT-path proof** ✓ (T7);
COALESCE-not-NULL-convention ✓ (T7-s4); single-value `attention` ✓; household-only / alpha deferred ✓.
Story-first ✓ (T0, existing feature is the anchor per a2o/CLAUDE.md).

**Placeholder scan:** every code step shows real field additions / test bodies / exact commands. The two
writer-threading sites (T5-s5) are one-line additions the compiler points at; the `From`-impl/builder
additions are one-liners at named lines.

**Type consistency:** `substrate_signal: Option<String>` (owned) on `CreateEconomicEventInput`, the
integrity struct, and `EconomicEventView`; `Option<&'a str>` on the borrowed `NewEconomicEvent`; threaded
with `.as_deref()`. camelCase `substrateSignal` at the wire. `SUBSTRATE_SIGNALS` (aliased) is the validator
whitelist.

**Out of scope (named, not faked):** the alpha-pair `ALLOW_DNA_REINSTALL` reinstall + lineage decision;
multi-dim `place = energy + attention` (single-value v1); any real concentration backfill; the Cluster #1
governor itself.

---

## Execution Handoff

**Plan complete and saved to `genesis/docs/superpowers/plans/2026-06-09-cluster3-substrate-signal-migration-plan.md`.**

Because the DNA-hash change (Task 3) and the cross-zome/Diesel/ts-rs spread make this **sequential and
build-heavy**, the execution recommendation is **subagent-driven-development run one task at a time with a
review checkpoint between tasks** (NOT parallel fan-out, NOT a workflow): dispatch a fresh subagent per
task, review the diff + the green test before the next. Commit-only (integrator pushes,
`feedback_commit_only_integrator_pushes`); the alpha deploy is a separate operator step after the slice is
green on household-nodes.
