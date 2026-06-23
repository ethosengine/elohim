# Diagnostic Read-Model End-to-End Wiring — Implementation Plan (P-DIAGNOSTIC)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.
> Working draft — NOT cite-sealed. Authored against the P2P-Dataplane Contract Ledger (`/projects/elohim/P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md`).

**Goal:** Make the system's own anchor-gap self-diagnosable. The provide-loop is gated by `!self_cid.is_empty()` (`main.rs:965`), and the steward CID anchor presence (`self_cid`) is the precondition for live authoring — yet NEITHER fact reaches any operator surface. Add `selfCidPresent`/`provideLoopEnabled` from `P2PStatusInfo` through doorway's composed `/admin/self-healing` read model to the typed wire schema, so the incident the system can already *observe* (anchor missing → provide-loop never spawned → nothing published) becomes *reportable*.

**Architecture:** This is the terminal consumer track of the diagnostic spine. The rich self-healing observability is already 90% built (Plan B/C, on this branch): `/admin/self-healing` composes a `SelfHealingView` from doorway runtime state + a 30s storage `/p2p/status` poll; `caughtUp`/`divergentAnchor`/`lagSeconds` already plumb end-to-end; `Retry-After`/`catching-up` already exist (`storage_proxy.rs:71`). The ONE confirmed remaining gap is the two anchor booleans — they were never added to `P2PStatusInfo`. This plan adds them at the source (`refresh_status()`), threads them through the doorway poll → `P2PHealth` → `SelfHealingInputs` → `compose_self_healing` into a NEW `anchor` block on `SelfHealingView`, and bumps the additive schema + regenerates TS. No new aggregation logic, no new routes — doorway-local Operational state surfaced through an existing composed view (swap-test clean: any doorway projecting the same storage gets the same anchor booleans).

**Tech Stack:** Rust (elohim-storage WASM-flagged build; doorway-service native build), serde_json, ts-rs codegen, JSON Schema. No new deps. TDD: schema_contract test + doorway compose unit tests + storage status unit assert.

**p2p-class:** No new entity. The two booleans are projections of existing Operational Cat-C node-local state (`self_cid` config + provide-loop spawn decision). No DHT entry, no table, no coordinator fn, no new route. Per the ledger §p2p-design-gate: "New runtime entities are Cat-C node-local read-models … Do not re-litigate; cite the class."

---

## Findings this plan CLOSES (with verdicts from the track review)

| Finding (#2 family) | Verdict | Action here |
|---|---|---|
| `selfCidPresent`/`provideLoopEnabled` absent from `P2PStatusInfo` | **CONFIRMED — the one real gap** | Add both fields + populate; thread to doorway anchor block + schema. |
| "parsed-then-DISCARDED by doorway" | REFUTED (`main.rs:483-494` already sets caughtUp/divergentAnchor) | No-op; correct stale opportunity-map row. |
| `/api/v1/status/projector` is 404 / absent from manifest | REFUTED (`http.rs:9776` declared, `:939` handler, `:12505` manifest test) | No-op; correct stale row. |
| "unmeasured vs measured-zero honesty bug" | REFUTED (already fixed: `distributionState` in schema/view/compute/backfill) | No-op; correct stale row. |
| consumers route at raw `/p2p/status` not safe transforms | REFINED (raw only in direct/tauri mode where it's fine; doorway mode uses composed `/admin/self-healing`) | No-op; correct stale row. The "use `/api/v1/federation/p2p-peers` at http.rs:2194" pointer is STALE — that route does not exist (`http.rs:2194` is blob-manifest healing). |
| `HealthIndicatorComponent` mounted nowhere / 2 dead recovery routes | OUT-OF-TRACK (Angular/imagodei rows of §4) | NOT assessed; left to the named Angular sibling follow-on. |

**Net new work = the one CONFIRMED gap + correcting the stale opportunity-map rows so no sibling track re-plans done work.**

---

## OWNED FILES (verbatim from ledger §2 file-ownership map)

**MUTATE (M):**
- `elohim/elohim-storage/src/p2p/mod.rs` — add 2 `bool` fields to `P2PStatusInfo` (after `:757`) + populate in `refresh_status()` (`:7050`). **SEQUENCED behind P-RECONCILE** (ledger RESOLUTION-B: P-RECONCILE is PRIMARY structural owner of mod.rs; rebase these 2 additive fields onto its `run()`/snapshot landing). Also touch the two stub/test `P2PStatusInfo` literals (`:1152`, `:1692`, `:7050` plus `for_testing` paths) for compile.
- `doorway/doorway-service/src/main.rs:483-494` — read 2 booleans from the polled `/p2p/status` JSON into `P2PHealth`. **SOLE owner** of this block (ledger RESOLUTION-G; P-DEFENSE touches `worker/conductor.rs`, a different file).
- `doorway/doorway-service/src/routes/health.rs:77` — add 2 fields to `P2PHealth`. **SOLE owner** (RESOLUTION-G).
- `doorway/doorway-service/src/routes/self_healing.rs` — add `anchor` block to `SelfHealingView` + `ProjectorView` siblings; extend `SelfHealingInputs` + `compose_self_healing`. **SOLE owner** (ledger: "new `anchor` block").
- `elohim/sdk/schemas/v1/views/stability-status-view.schema.json` — additive `anchor` object (+ optional `syncPaused`). **SOLE owner**.
- `app/elohim-app/src/app/generated/{p2p-status-view,stability-status-view}.ts` — regen only. **SOLE owner**.
- `elohim/elohim-storage/tests/schema_contract.rs` — add/extend the p2p-status assertion for the 2 fields. (In-track; P-PROOFS owns the `tests/` *new files* but this is an existing contract test for an existing view this plan mutates — hand-off note below.)

**CREATE (C):** none.

**Collision statement:** Every file above is either SOLE-owned by P-DIAGNOSTIC or is the single SEQUENCED mod.rs hand-off behind P-RECONCILE (RESOLUTION-B). This plan touches **no file owned by another plan** except `p2p/mod.rs`, where it makes ONLY the 2 additive `bool` fields + populate, strictly after P-RECONCILE's structural change lands — and `schema_contract.rs`, where it extends (not creates) the existing p2p-status assertion (hand-off note to P-PROOFS below). No type collisions: `SelfHealingView` family is THIS track's sole-owned surface; `P2PStatusInfo` single-owner = elohim-storage `p2p/mod.rs`.

---

## NEW PRIMITIVES THIS PLAN OWNS

| Primitive | Home | Shape |
|---|---|---|
| `P2PStatusInfo.self_cid_present` | `elohim-storage::p2p::mod` (extends ledger S9) | `pub self_cid_present: bool` |
| `P2PStatusInfo.provide_loop_enabled` | `elohim-storage::p2p::mod` (extends ledger S9) | `pub provide_loop_enabled: bool` |
| `SelfHealingView.anchor` | `doorway::routes::self_healing` (ledger §3 row) | `{ self_cid_present: bool, provide_loop_enabled: bool }` (camelCase) |
| `AnchorView` struct | `doorway::routes::self_healing` | `#[serde(rename_all="camelCase")] { pub self_cid_present: bool, pub provide_loop_enabled: bool }` |

These are NOT shared types — they are extensions of the single-owner `P2PStatusInfo` (ledger S9, owner = elohim-storage; this plan is the named consumer that adds the 2 fields per RESOLUTION-B) and a new sub-struct on this track's own `SelfHealingView`.

## CONSUMED PRIMITIVES (skip-if-present clause)

- **`SweepRegistrySnapshot` (ledger S5, owner P-RECONCILE)** — `DEPENDS-ON: HARD`. This plan does NOT define it. **Skip-if-present:** before reading any sweep snapshot, verify `elohim_compute::sweep::SweepRegistrySnapshot` is exposed and that `P2PStatusInfo` already embeds it (P-RECONCILE's "embed snapshot" item). If present, VERIFY-ONLY (the anchor work sits ALONGSIDE the snapshot field — no edit to it). If absent at integration time, P-DIAGNOSTIC's anchor additions still compile independently (they do not reference the snapshot); proceed with anchor work and flag the missing snapshot in hand-off notes. **The anchor fields and the sweep snapshot are orthogonal additions to the same struct — neither blocks the other's compilation; the sequencing is purely to avoid a merge conflict on the struct literal.**
- `P2PStatusInfo` (ledger S9) — owner is elohim-storage p2p/mod.rs (this plan IS the named field-adder; not a foreign type).

---

## DEPENDENCY EDGES (from ledger §4 DAG)

| Edge | Type | Reason |
|---|---|---|
| P-DIAGNOSTIC → P-RECONCILE | **HARD (file-sequencing)** | The 2-field `P2PStatusInfo` edit + any snapshot embed is sequenced behind P-RECONCILE's structural `run()` refactor + snapshot-embed on the same struct (RESOLUTION-B). Non-mod.rs work (doorway, schema, TS) is fully independent and may begin in Wave 2. |
| P-DIAGNOSTIC → (Track-7/P-PROOFS) | **SOFT** | A placement-diversity invariant could read `distributionState` (read-only, different file) — no conflict. |

**Dispatch wave:** WAVE 3 terminal consumer for the mod.rs touch; doorway/schema/TS sub-tasks may run in WAVE 2 in an isolated worktree and rebase the mod.rs touch last.

---

## Build / test commands (per-crate RUSTFLAGS + /tmp target + plain cargo)

elohim-storage (Tasks 1, 6 — WASM getrandom flag REQUIRED for this crate):
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --test schema_contract p2p_status 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib p2p::tests 2>&1 | tail -40
```

ts-rs export (regenerate generated TS for P2PStatusInfo — run from elohim-views, the ts-rs anchor crate per storage CLAUDE.md):
```
cd /projects/elohim/elohim/elohim-views && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ev-test RUSTC_WRAPPER="" cargo test export_bindings 2>&1 | tail -40
```

doorway-service (Tasks 2–5 — native; RUSTFLAGS MUST be empty):
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib self_healing 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib health 2>&1 | tail -40
```

Schema codegen TS (Task 5):
```
cd /projects/elohim && pnpm run schema:codegen:ts 2>&1 | tail -40
```

Final gates:
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && cargo fmt --check
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
```

Rules (memory): `RUSTFLAGS=""` for doorway/elohim-views native crates; `--cfg getrandom_backend="custom"` for elohim-storage (WASM); `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target dirs (fingerprint-ENOENT on pool slot); **plain `cargo test`, NEVER nextest**; never `&&`-pipe a gate exit code (use `2>&1 | tail -N`).

---

## A NOTE ON `provide_loop_enabled` (the genuine plumbing wrinkle)

`self_cid_present` is trivial inside `refresh_status()` — `P2PNode` owns `self.config.self_cid`. But the provide-loop is spawned OUTSIDE `P2PNode`, in the composition scope (`main.rs:959-1069`), gated by `(Some(lamad_hc), Some(provide_pool), Some(self_cid)) if !self_cid.is_empty()`. `P2PNode` has no field tracking whether that loop was actually spawned. **Resolution (chosen for minimal blast radius):** populate `provide_loop_enabled` in `refresh_status()` from the SAME predicate the spawn site uses, but evaluated from `P2PNode`-visible state — i.e. `self.config.self_cid.as_deref().map(|c| !c.is_empty()).unwrap_or(false) && self.db_pool.is_some()`. This is the storage-visible necessary condition; it CANNOT see the `lamad_hc` registry (composition-only). **Therefore `provide_loop_enabled` here means "storage-side preconditions for the provide-loop are met" — NOT a confirmation the loop task is alive.** The lamad-HcClient leg is a FOLLOW-ON seam (below). Tasks assert exactly this semantics; the doc-comment on the field states it verbatim so no consumer over-reads it.

---

## TASK 1 — Add `self_cid_present` + `provide_loop_enabled` to `P2PStatusInfo` + populate

> **SEQUENCING GATE:** Do NOT start this task until P-RECONCILE's `p2p/mod.rs` `run()` refactor + snapshot-embed have landed on the integration branch (ledger RESOLUTION-B). Rebase onto it first. Tasks 2–5 (doorway/schema/TS) have NO such gate and may proceed in parallel in the worktree.

Files:
- `elohim/elohim-storage/src/p2p/mod.rs` — struct fields after `:757` (`placement_gaps_emitted_total`); populate in `refresh_status()` `:7050`; fix the stub/test literals at `:1152`, `:1692`, `:7050`, and any `P2PStatusInfo {` in `for_testing`/`from_parts_for_testing` paths.

- [ ] Write the failing test — append to `p2p/mod.rs` `#[cfg(test)] mod tests` (or the nearest existing status test module). Use the `for_testing` handle to assert defaults serialize, and a direct literal to assert camelCase:
```rust
    #[test]
    fn p2p_status_info_carries_anchor_booleans_camel_case() {
        let s = P2PStatusInfo {
            self_cid_present: true,
            provide_loop_enabled: false,
            ..p2p_status_info_test_default() // existing helper, or build a full literal
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"selfCidPresent\":true"), "{json}");
        assert!(json.contains("\"provideLoopEnabled\":false"), "{json}");
    }
```
  (If no `p2p_status_info_test_default` helper exists, build a full literal — the struct has ~22 fields; copy the `:1152` stub literal and add the two new ones.)
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib p2p::tests 2>&1 | tail -40` — expect `missing field self_cid_present` / `no field selfCidPresent`.
- [ ] Write minimal implementation — add to `P2PStatusInfo` after `:757` (`placement_gaps_emitted_total`):
```rust
    /// True when this node has a steward CID anchor configured (`Config::self_cid`
    /// non-empty). The provide-loop and custody sweeps NO-OP silently without it
    /// (`main.rs:965`), so an absent anchor is the silent root cause of "node
    /// published nothing." Surfaced so the anchor gap is self-diagnosable.
    pub self_cid_present: bool,
    /// True when the STORAGE-SIDE preconditions for the Slice-2b provide-loop are
    /// met (non-empty `self_cid` AND a db pool). This is the necessary condition
    /// visible to `P2PNode`; it does NOT confirm the loop TASK is alive (the
    /// lamad-HcClient leg is composition-scope only — see plan FOLLOW-ON seam).
    pub provide_loop_enabled: bool,
```
  Add a matching `#[ts(...)]` is NOT needed (bool maps directly). In `refresh_status()` (`:7050` literal) add:
```rust
            self_cid_present: self
                .config
                .self_cid
                .as_deref()
                .map(|c| !c.is_empty())
                .unwrap_or(false),
            provide_loop_enabled: self
                .config
                .self_cid
                .as_deref()
                .map(|c| !c.is_empty())
                .unwrap_or(false)
                && self.db_pool.is_some(),
```
  Add `self_cid_present: false, provide_loop_enabled: false,` to the stub literals at `:1152` and `:1692` (and any `for_testing` literal) so the crate compiles.
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib p2p::tests 2>&1 | tail -40`.
- [ ] Regenerate TS bindings: `cd /projects/elohim/elohim/elohim-views && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ev-test RUSTC_WRAPPER="" cargo test export_bindings 2>&1 | tail -40` — confirm `app/elohim-app/src/app/generated/p2p-status-view.ts` (or the `sdk/storage-client-ts/src/generated/` path the `#[ts(export_to)]` names) gains `selfCidPresent`/`provideLoopEnabled`. **Verify byte-stable diff** — only the 2 new fields added (memory: codegen oscillation is cosmetic; do not let it churn other types).
- [ ] Commit (selective-stage):
```
git add elohim/elohim-storage/src/p2p/mod.rs elohim/sdk/storage-client-ts/src/generated/P2PStatusInfo.ts app/elohim-app/src/app/generated/p2p-status-view.ts
git commit -m "feat(elohim-storage): surface selfCidPresent/provideLoopEnabled on P2PStatusInfo

The anchor gap (no self_cid -> provide-loop never spawned -> nothing
published) was observable but unreportable. provide_loop_enabled means
storage-side preconditions met, NOT loop-task-alive (lamad-HcClient leg
is composition-scope; named follow-on).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 2 — Add anchor fields to doorway `P2PHealth`

Files:
- `doorway/doorway-service/src/routes/health.rs:77` (`P2PHealth` struct).

- [ ] Write the failing test — extend the existing `p2p_health_carries_reconcile_caught_up_and_divergent_anchor` test (`:343`) or add a sibling asserting the new fields default + serialize:
```rust
    #[test]
    fn p2p_health_carries_anchor_booleans() {
        let h = P2PHealth {
            enabled: true,
            peer_count: 2,
            peer_id: Some("p".into()),
            caught_up: Some(true),
            divergent_anchor: Some(0),
            self_cid_present: Some(true),
            provide_loop_enabled: Some(false),
        };
        assert_eq!(h.self_cid_present, Some(true));
        assert_eq!(h.provide_loop_enabled, Some(false));
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib health 2>&1 | tail -30` — expect `missing field self_cid_present`.
- [ ] Write minimal implementation — add to `P2PHealth` (after `divergent_anchor` at `:90`):
```rust
    /// Anchor presence from storage /p2p/status. None when storage unreachable.
    pub self_cid_present: Option<bool>,
    /// Provide-loop storage-side precondition flag from /p2p/status. None when
    /// storage unreachable. (Necessary condition, not loop-task-alive — see
    /// elohim-storage P2PStatusInfo doc-comment.)
    pub provide_loop_enabled: Option<bool>,
```
  Update any OTHER `P2PHealth { ... }` literal in this file's tests (`:344`, `:358`) to add `self_cid_present: None, provide_loop_enabled: None,`.
- [ ] Run, expect PASS: same command.
- [ ] Commit: `git add doorway/doorway-service/src/routes/health.rs` + message `feat(doorway): P2PHealth carries anchor booleans`.

## TASK 3 — Read the two booleans in the doorway `/p2p/status` poll

Files:
- `doorway/doorway-service/src/main.rs:483-494` (the `P2PHealth { ... }` construction inside the 30s poll).

This block has no pure unit test (it's an inline `tokio::spawn` in `main`); it is verified by compilation + the whole-crate gate. The JSON-extraction logic mirrors the existing `caught_up: recon["caughtUp"].as_bool()` idiom — these two live at the TOP level of the status JSON, not under `projectionReconcile`.

- [ ] Write minimal implementation — extend the `P2PHealth { ... }` literal at `:485` (after `divergent_anchor` at `:492`):
```rust
                                self_cid_present: status["selfCidPresent"].as_bool(),
                                provide_loop_enabled: status["provideLoopEnabled"].as_bool(),
```
- [ ] Run, expect PASS (compile): `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40` — confirm the bin compiles (the literal now matches the struct from Task 2).
- [ ] Commit: `git add doorway/doorway-service/src/main.rs` + message `feat(doorway): read selfCidPresent/provideLoopEnabled from storage p2p/status poll`.

## TASK 4 — `AnchorView` + thread through `SelfHealingInputs` / `compose_self_healing`

Files:
- `doorway/doorway-service/src/routes/self_healing.rs` — add `AnchorView` struct; add `anchor: AnchorView` field to `SelfHealingView` (`:32`); extend `SelfHealingInputs` (`:124`) with the 2 inputs; populate in `compose_self_healing` (`:139`); read from `state.p2p_health` in the handler (`:224`).

- [ ] Write the failing test — append to `self_healing.rs` `mod tests`:
```rust
    #[test]
    fn compose_surfaces_anchor_block() {
        let view = compose_self_healing(SelfHealingInputs {
            self_cid_present: Some(false),
            provide_loop_enabled: Some(false),
            ..sample_inputs()
        });
        assert_eq!(view.anchor.self_cid_present, false);
        assert_eq!(view.anchor.provide_loop_enabled, false);
    }

    #[test]
    fn anchor_serializes_camel_case() {
        let view = compose_self_healing(SelfHealingInputs {
            self_cid_present: Some(true),
            provide_loop_enabled: Some(true),
            ..sample_inputs()
        });
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("\"anchor\""));
        assert!(json.contains("\"selfCidPresent\":true"), "{json}");
        assert!(json.contains("\"provideLoopEnabled\":true"), "{json}");
    }
```
  (Add the 2 new fields to the `sample_inputs()` helper at `:289` as `self_cid_present: None, provide_loop_enabled: None,`.)
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib self_healing 2>&1 | tail -40` — expect `no field anchor` / `missing field self_cid_present`.
- [ ] Write minimal implementation:
  1. New struct (near `ProjectorView` at `:77`):
```rust
/// Steward-CID anchor presence + provide-loop precondition. A node with NO
/// anchor publishes nothing (provide-loop never spawns); this block makes that
/// silent failure self-diagnosable. None inputs (storage unreachable) render
/// `false` — honest default: "we cannot confirm the anchor is present."
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorView {
    pub self_cid_present: bool,
    pub provide_loop_enabled: bool,
}
```
  2. Add `pub anchor: AnchorView,` to `SelfHealingView` (after `projector` at `:46`).
  3. Add to `SelfHealingInputs` (after `p2p_divergent_anchor` at `:127`):
```rust
    pub self_cid_present: Option<bool>,
    pub provide_loop_enabled: Option<bool>,
```
  4. In `compose_self_healing` (`:174` region) add:
```rust
        anchor: AnchorView {
            self_cid_present: inputs.self_cid_present.unwrap_or(false),
            provide_loop_enabled: inputs.provide_loop_enabled.unwrap_or(false),
        },
```
  5. In the handler (`:224-259`), extract from `state.p2p_health` alongside `caught_up`:
```rust
    let (p2p_caught_up, p2p_divergent_anchor, self_cid_present, provide_loop_enabled) =
        match state.p2p_health.try_read() {
            Ok(g) => match &*g {
                Some(h) => (h.caught_up, h.divergent_anchor, h.self_cid_present, h.provide_loop_enabled),
                None => (None, None, None, None),
            },
            Err(_) => (None, None, None, None),
        };
```
  and pass `self_cid_present, provide_loop_enabled,` into the `SelfHealingInputs { ... }` literal (`:256`).
- [ ] Run, expect PASS: same `self_healing` command.
- [ ] Commit: `git add doorway/doorway-service/src/routes/self_healing.rs` + message `feat(doorway): anchor block on SelfHealingView (selfCidPresent/provideLoopEnabled)`.

## TASK 5 — Additive schema bump + TS regen + schema_contract

Files:
- `elohim/sdk/schemas/v1/views/stability-status-view.schema.json` — add `anchor` object to `properties` + `required`.
- `app/elohim-app/src/app/generated/stability-status-view.ts` — regen via `pnpm run schema:codegen:ts` (do NOT hand-edit).

- [ ] Write the failing test FIRST (schema-side) — add `anchor` to the schema, then run codegen; the failing condition is the codegen freshness / contract. Add to `stability-status-view.schema.json` `properties` (after `projector`):
```json
    "anchor": {
      "description": "Steward-CID anchor presence + provide-loop storage-side precondition. A node with no anchor publishes nothing.",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "selfCidPresent": { "type": "boolean" },
        "provideLoopEnabled": { "type": "boolean" }
      },
      "required": ["selfCidPresent", "provideLoopEnabled"]
    },
```
  and add `"anchor"` to the top-level `"required"` array (`:9`).
- [ ] Run codegen, expect a diff in the generated TS: `cd /projects/elohim && pnpm run schema:codegen:ts 2>&1 | tail -40` — confirm `stability-status-view.ts` gains an `anchor` interface field. (Memory: codegen is non-idempotent on Reach/ContentFormat — run twice, commit only the stable result; verify ONLY `anchor` changed.)
- [ ] Confirm the doorway `SelfHealingView` JSON matches the schema — there is no Rust-side schema_contract for the doorway view (the schema NOTE says it is "kept in sync with doorway's Rust SelfHealingView"), so verify by hand: serialize `compose_self_healing(sample_inputs())` in a doorway test and assert the JSON validates against the schema's required keys (`upstreams,projector,peers,render,warmup,conductor,anchor`). Add this assertion to the `self_healing.rs` test from Task 4:
```rust
    #[test]
    fn view_has_all_schema_required_top_level_keys() {
        let v = serde_json::to_value(compose_self_healing(sample_inputs())).unwrap();
        for k in ["upstreams","projector","peers","render","warmup","conductor","anchor"] {
            assert!(v.get(k).is_some(), "missing required key {k}");
        }
    }
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib self_healing 2>&1 | tail -40`.
- [ ] Commit: `git add elohim/sdk/schemas/v1/views/stability-status-view.schema.json app/elohim-app/src/app/generated/stability-status-view.ts doorway/doorway-service/src/routes/self_healing.rs` + message `feat(schema): additive anchor block on stability-status-view + TS regen`.

## TASK 6 — Extend storage `schema_contract.rs` p2p-status assertion (HAND-OFF to P-PROOFS)

Files:
- `elohim/elohim-storage/tests/schema_contract.rs` — the existing `p2p_status_view_matches_schema` test (referenced in the struct doc-comment at `mod.rs:703`).

> **HAND-OFF NOTE:** P-PROOFS owns NEW files under `tests/` (ledger §2). This is an EXISTING contract test for an EXISTING view this plan mutates — extending it is in-track maintenance, not a new test file. If P-PROOFS has an in-flight edit to `schema_contract.rs`, sequence after it (this is a 2-key additive assertion, mechanical merge).

- [ ] Update the p2p-status-view schema (`elohim/sdk/schemas/v1/views/p2p-status-view.schema.json`) to add `selfCidPresent`/`provideLoopEnabled` booleans to `properties` + `required` (the schema_contract test validates `P2PStatusInfo` against THIS schema, distinct from stability-status-view).
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --test schema_contract p2p_status 2>&1 | tail -40` — the contract test confirms struct↔schema parity for the 2 new fields.
- [ ] Commit: `git add elohim/sdk/schemas/v1/views/p2p-status-view.schema.json` + message `test(elohim-storage): p2p-status-view schema contract covers anchor booleans`.

## TASK 7 — Correct stale opportunity-map rows (no code; prevents sibling re-planning)

Files:
- The opportunity-map / roadmap doc(s) under `genesis/docs/superpowers/` that carry the §4/§6 stale rows. (Locate via grep; do NOT cite-seal — working-draft edits only.)

- [ ] `grep -rn "status/projector\|parsed-then-discard\|federation/p2p-peers\|caughtUp" genesis/docs/superpowers/` to find the stale rows.
- [ ] Mark RESOLVED inline (do not delete history): the projector route (`http.rs:9776`), the caughtUp/divergentAnchor plumb (`main.rs:489`), the `distributionState` honesty fix, and the nonexistent `/api/v1/federation/p2p-peers` pointer (`http.rs:2194` is blob-manifest healing) are DONE/STALE. Leave a one-line "RESOLVED 2026-06-14 (P-DIAGNOSTIC) — see <commit>" note per row.
- [ ] Commit: `git add <opportunity-map files>` + message `docs(p2p-diagnostic): mark resolved/stale diagnostic rows in opportunity map`.

---

## // FOLLOW-ON seams (deliberately left for the integration pass / named siblings)

1. **`provide_loop_enabled` lamad-HcClient confirmation.** This plan reports the STORAGE-VISIBLE precondition (`self_cid` non-empty + db pool), NOT loop-task-liveness. A true "loop is alive" signal needs the composition scope (`main.rs:959`) to thread the spawn decision (or an `AtomicBool` set when the `tokio::spawn` fires) back into `P2PNode`/`Config`. **SEAM-DELTA — not in ledger.** Integration pass should decide: (a) add `Config::provide_loop_spawned: Arc<AtomicBool>` set by main.rs and read in `refresh_status()`, or (b) accept the precondition semantics permanently. Doc-comment already states the weaker semantics so no consumer over-reads.
2. **Angular stability-lens consumer of the `anchor` field.** The page (`app/elohim-app/.../debug/lenses/stability-lens.component.ts`) already exists and consumes `/admin/self-healing`; rendering the new `anchor` block is a NAMED frontend sibling follow-on (eyes-first; `pnpm look` on the stability lens). Schema field lands verbatim for it.
3. **`syncPaused` / `degenerateRate` surfacing in the C view.** `P2PStatusInfo.sync_paused` and the render `degenerateRate` are present at source; surfacing `syncPaused` in `SelfHealingView` is a one-line follow-on (recommended) but OUT of this plan's confirmed-gap scope. Left for integration to fold if desired.
4. **`SweepRegistrySnapshot` embed alongside `anchor`.** P-RECONCILE embeds the sweep snapshot in `P2PStatusInfo`; if this plan's anchor fields and that snapshot land in the same wave, the integrator merges both additively into the struct literal (orthogonal; RESOLUTION-B sequencing).

---

## Dispatch note

- **Isolated-worktree, subagent-driven, commit-only.** Run from a dedicated worktree off the integration branch. The integrator pushes/merges (memory: commit-only; never `git push`).
- **Sequencing:** Tasks 2–5, 7 (doorway/schema/TS/docs) are independent and may run FIRST in the worktree (Wave 2). Task 1 + Task 6 (the `p2p/mod.rs` + storage schema_contract touches) are SEQUENCED behind P-RECONCILE's mod.rs structural landing (Wave 3) — rebase onto updated `mod.rs` before applying the 2 additive fields.
- **Selective-stage** each commit (concurrent sessions share the worktree per memory) — the per-task `git add` lists name exact files only; never bulk-revert ambient mods.
- **Per-crate RUSTFLAGS discipline is load-bearing** (elohim-storage = WASM custom getrandom; doorway/elohim-views = empty). Mixing them mid-build link-fails with `undefined __getrandom_v03_custom`.
