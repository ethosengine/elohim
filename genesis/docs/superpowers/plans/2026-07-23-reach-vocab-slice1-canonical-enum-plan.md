---
title: "Reach Reconciliation Slice 1 — canonical enum consolidation + drift test"
id: reach-vocab-slice1-canonical-enum-plan
status: Ready
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: superseded-by-implementation — graduate once slice 1 lands and the drift test is green in CI
created: 2026-07-23
topic: [reach, vocabulary-drift, epr, elohim-storage, schema-contract, slice-1]
cites:
  - reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:d0303e0209f57b76 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - elohim/epr/src/reach.rs
  - elohim/elohim-storage/src/services/epr_kind.rs
  - elohim/elohim-storage/src/services/reach_earning.rs
---

# Reach Reconciliation Slice 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Kill the two vocabularies the spec marks DIES (Rust services kebab-8, TS `VALID_REACH_LEVELS` 6) by consolidating onto the canonical `elohim_epr::Reach` (schema-8), with a drift-prevention contract test landing FIRST so divergence is caught before it is cured.

**Architecture:** Spec §1 (`2026-07-22-reach-ontology-vocabulary-split-spec.md`): schema-8 (`private/self/intimate/trusted/familiar/community/public/commons`, source-of-record `elohim/sdk/schemas/v1/enums/reach.schema.json`) is the ONLY declared-reach vocabulary. `elohim_epr::Reach` already matches it (guardrail: generated `REACH_OPENNESS`). This slice re-points elohim-storage's divergent local enum at the canonical one via a re-export + extension trait (floor-class semantics preserved), keeps legacy manifest keys parseable (data-aware migration, spec §7.5), and deletes the dead TS constant. **Out of scope (later slices):** geographic-8 rename, custody rename, verdict surface, announcement slot, resilience README gap-matrix rewrite.

**Tech Stack:** Rust (elohim-storage native, elohim-epr crate), TypeScript (elohim-library, elohim-app), serde, cargo test (NO nextest in this container).

## Global Constraints

- Storage builds: `RUSTFLAGS='--cfg getrandom_backend="custom"'` and `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev`. Run cargo from `/projects/elohim/elohim/elohim-storage` (explicit `cd` per gate — shell state doesn't persist).
- epr crate builds: `RUSTFLAGS=""` and `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/crates/dev`, run from `/projects/elohim/elohim/epr`.
- Plain `cargo test` (container has no nextest). `cargo fmt` + `cargo clippy -- -D warnings` before each commit.
- Commit-only discipline: all commits on branch `shift/reach-vocab-slice1`; never push, never merge to dev. Path-limited commits (`git commit -m "…" -- <paths>`).
- Canonical old→new value mapping (pinned here; every task uses it verbatim):

| old (kebab-8) | new (schema-8) | floor class |
|---|---|---|
| `personal` | `self` | any (floor-allowed) |
| `intimate` | `intimate` | any |
| `household` | `trusted` | any |
| `neighborhood` | `familiar` | any |
| `collective` | `community` | standing-required |
| `community` | `community` | standing-required |
| `district` | `public` | standing-required |
| `public` | `commons` | standing-required |

(`private` and `self` are floor-allowed for totality; old had 4 floor + 4 standing, new has 5 floor + 3 standing — `collective`+`community` merge, and old `public` maps to `commons` because old-public was the maximally-open rung.)

---

### Task 0: Branch

- [ ] **Step 1:** `cd /projects/elohim && git checkout -b shift/reach-vocab-slice1` (from current integ/dev-merge HEAD). Expected: `Switched to a new branch`.

### Task 1: Drift-prevention contract test (lands FIRST, passes at birth, fails forever after on any divergence)

**Files:**
- Create: `elohim/elohim-storage/tests/reach_vocabulary_contract.rs`

**Interfaces:**
- Consumes: `elohim_epr::Reach` (already a storage dep — `p2p/reach_authorization.rs` uses it), `elohim/sdk/schemas/v1/enums/reach.schema.json`.
- Produces: the contract test later tasks must keep green.

- [ ] **Step 1: Write the test**

```rust
//! Drift-prevention contract: the canonical Rust Reach enum must serialize to
//! EXACTLY the schema enum values, in order. Source of truth:
//! elohim/sdk/schemas/v1/enums/reach.schema.json (spec: reach-ontology-vocabulary-split-spec §1).

use elohim_epr::Reach;

const ALL: [Reach; 8] = [
    Reach::Private,
    Reach::SelfScope,
    Reach::Intimate,
    Reach::Trusted,
    Reach::Familiar,
    Reach::Community,
    Reach::Public,
    Reach::Commons,
];

#[test]
fn rust_reach_matches_schema_enum_exactly() {
    let schema_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sdk/schemas/v1/enums/reach.schema.json"
    );
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap()).unwrap();
    let schema_values: Vec<String> = schema["enum"]
        .as_array()
        .expect("reach.schema.json must carry an enum array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let rust_values: Vec<String> = ALL
        .iter()
        .map(|r| serde_json::to_value(r).unwrap().as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        rust_values, schema_values,
        "elohim_epr::Reach diverged from reach.schema.json — the schema is the source of record; fix the Rust side (or run the schema-change process, never hand-drift)"
    );
}
```

- [ ] **Step 2: Run it**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo test --test reach_vocabulary_contract
```
Expected: PASS (epr crate is already aligned; this test now guards the pair).

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim && git add elohim/elohim-storage/tests/reach_vocabulary_contract.rs && git commit -m "test(reach): schema-contract drift test pinning elohim_epr::Reach to reach.schema.json (slice 1, spec reach-ontology-vocabulary-split)" -- elohim/elohim-storage/tests/reach_vocabulary_contract.rs
```

### Task 2: Replace `epr_kind::Reach` (divergent kebab-8) with canonical re-export + floor-class extension

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_kind.rs` (enum at ~lines 83–125 + its tests)

**Interfaces:**
- Consumes: `elohim_epr::Reach`.
- Produces (later tasks rely on these exact names): `pub use elohim_epr::Reach;` · trait `ReachStandingExt` with `fn is_floor_allowed(self) -> bool` and `fn as_manifest_key(self) -> &'static str` · `pub fn parse_reach_key(key: &str) -> Option<Reach>` (accepts canonical keys AND legacy aliases per the Global-Constraints mapping).

- [ ] **Step 1: Write failing tests** (append to `epr_kind.rs` tests module; they fail to compile until Step 2)

```rust
    #[test]
    fn canonical_reach_floor_split() {
        use super::ReachStandingExt;
        for r in [Reach::Private, Reach::SelfScope, Reach::Intimate, Reach::Trusted, Reach::Familiar] {
            assert!(r.is_floor_allowed(), "{r:?} must be floor-allowed");
        }
        for r in [Reach::Community, Reach::Public, Reach::Commons] {
            assert!(!r.is_floor_allowed(), "{r:?} must require standing");
        }
    }

    #[test]
    fn legacy_manifest_keys_still_parse() {
        // Data-aware migration (spec §7.5): live standing-policy manifests may
        // still carry the retired kebab-8 keys. They map, never 404.
        for (legacy, canonical) in [
            ("personal", Reach::SelfScope),
            ("household", Reach::Trusted),
            ("neighborhood", Reach::Familiar),
            ("collective", Reach::Community),
            ("district", Reach::Public),
        ] {
            assert_eq!(parse_reach_key(legacy), Some(canonical));
        }
        // Legacy "public" meant the top rung → commons; canonical "public" stays public.
        assert_eq!(parse_reach_key("commons"), Some(Reach::Commons));
        assert_eq!(parse_reach_key("self"), Some(Reach::SelfScope));
        assert_eq!(parse_reach_key("nonsense"), None);
    }
```

- [ ] **Step 2: Replace the enum block** (delete local `pub enum Reach {…}` + its `impl` at ~83–125; insert)

```rust
/// Canonical declared-reach vocabulary — re-exported from the protocol crate.
/// The schema (elohim/sdk/schemas/v1/enums/reach.schema.json) is the source of
/// record; the local kebab-8 (personal…district/public) is RETIRED
/// (spec: reach-ontology-vocabulary-split §1). Legacy manifest keys parse via
/// `parse_reach_key` and map per the spec's pinned table.
pub use elohim_epr::Reach;

/// Standing/floor semantics for the standing-policy manifest's `reachThresholds`.
pub trait ReachStandingExt {
    /// `true` ⇒ manifest floor class "any": bypasses the standing check
    /// (CID-targeted lookup + local-relationship floor classes).
    fn is_floor_allowed(self) -> bool;
    /// Canonical manifest key (the serde/schema string).
    fn as_manifest_key(self) -> &'static str;
}

impl ReachStandingExt for Reach {
    fn is_floor_allowed(self) -> bool {
        matches!(
            self,
            Reach::Private | Reach::SelfScope | Reach::Intimate | Reach::Trusted | Reach::Familiar
        )
    }
    fn as_manifest_key(self) -> &'static str {
        match self {
            Reach::Private => "private",
            Reach::SelfScope => "self",
            Reach::Intimate => "intimate",
            Reach::Trusted => "trusted",
            Reach::Familiar => "familiar",
            Reach::Community => "community",
            Reach::Public => "public",
            Reach::Commons => "commons",
        }
    }
}

/// Parse a manifest/config reach key: canonical schema-8 keys plus the retired
/// kebab-8 legacy aliases (data-aware migration — spec §7.5: no value removed
/// while live rows/manifests still carry it).
pub fn parse_reach_key(key: &str) -> Option<Reach> {
    Some(match key {
        "private" => Reach::Private,
        "self" | "personal" => Reach::SelfScope,
        "intimate" => Reach::Intimate,
        "trusted" | "household" => Reach::Trusted,
        "familiar" | "neighborhood" => Reach::Familiar,
        "community" | "collective" => Reach::Community,
        // legacy "district" sat below legacy "public"(top). district→public.
        "public" | "district" => Reach::Public,
        "commons" => Reach::Commons,
        _ => return None,
    })
}
```

Note: legacy `"public"` is ambiguous (old top rung ⇒ semantically `commons`, but canonical `"public"` must parse as `Public`). Canonical reading wins in `parse_reach_key` (`"public"` → `Public`); the old-public→commons mapping applies only when *migrating stored old-vocabulary data*, which Task 3 checks for. This asymmetry is deliberate — record it in the commit message.

- [ ] **Step 3: Delete the old enum's variant tests** in the same tests module (the `is_floor_allowed` assertions over `Personal/Household/…` at ~lines 168–182) — they are superseded by Step 1's tests.

- [ ] **Step 4: Build + run module tests** (expect compile errors ONLY in `epr_compose.rs`/`reach_earning.rs` — that's Task 3; to isolate, run `cargo test --lib services::epr_kind` after Task 3's mechanical rename if the crate doesn't compile in isolation. Otherwise proceed to Task 3 and run the full gate there.)

### Task 3: Migrate the two consumers (`epr_compose.rs`, `reach_earning.rs`)

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_compose.rs` (imports `epr_kind::Reach`)
- Modify: `elohim/elohim-storage/src/services/reach_earning.rs` (imports at :12; fixture JSON at ~:243–244; `floor_reach_household…` test at ~:346)

**Interfaces:**
- Consumes: Task 2's `Reach` re-export, `ReachStandingExt`, `parse_reach_key`.

- [ ] **Step 1: Compiler-driven variant rename** in both files, using EXACTLY the Global-Constraints mapping for variant names: `Personal→SelfScope`, `Household→Trusted`, `Neighborhood→Familiar`, `Collective→Community`, `District→Public`, and old `Public→Commons` **where the old code meant the top rung** (check each site: if the code treats `Public` as "maximally open", it becomes `Commons`; if it's parsing the canonical wire string "public", it stays `Public`). Replace any `.as_kebab()` calls with `.as_manifest_key()`, and any string→Reach parsing with `parse_reach_key`. Where `is_floor_allowed` was inherent, import `ReachStandingExt`.
- [ ] **Step 2: Update fixture JSON** at `reach_earning.rs:~243` to canonical keys:

```json
"private":"any","self":"any","intimate":"any","trusted":"any","familiar":"any",
"community":"neutral","public":"high","commons":"high"
```

- [ ] **Step 3: Rename/adjust the behavior tests** — e.g. `floor_reach_household_returns_local_relationship_reach_class` → `floor_reach_trusted_returns_local_relationship_reach_class` with `Reach::Trusted`. Keep ONE test exercising a legacy-key manifest (fixture JSON with `"household":"any"`) to prove live old-vocabulary manifests still evaluate — via `parse_reach_key`.
- [ ] **Step 4: Data audit (spec §7.5)** — confirm no other live carriers of the kebab-8:

```bash
cd /projects/elohim && grep -rn "reachThresholds" --include="*.json" --include="*.rs" --include="*.yaml" elohim/ genesis/ | grep -v target | head -30
grep -rn '"household"\|"neighborhood"\|"collective"\|"district"' elohim/elohim-storage/src/migrations elohim/sdk 2>/dev/null | head
```
Expected: hits only in `reach_earning.rs`/`epr_kind.rs` (already handled) and possibly seed/manifest JSON. **If any manifest/seed JSON carries old keys, leave the data untouched** (the alias parser covers it) and list the files in the commit message for the future cleanup slice.
- [ ] **Step 5: Full storage gate**

```bash
cd /projects/elohim/elohim/elohim-storage && export RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev && cargo fmt && cargo build && cargo test --lib && cargo test --test reach_vocabulary_contract && cargo clippy -- -D warnings
```
Expected: all green (contract test from Task 1 still passing proves no new drift was introduced).
- [ ] **Step 6: Commit**

```bash
cd /projects/elohim && git add elohim/elohim-storage/src/services/epr_kind.rs elohim/elohim-storage/src/services/epr_compose.rs elohim/elohim-storage/src/services/reach_earning.rs && git commit -m "refactor(reach): retire services kebab-8 — canonical elohim_epr::Reach + ReachStandingExt + legacy-alias parser (slice 1; old-public→commons on stored data only, canonical 'public' wins at parse)" -- elohim/elohim-storage/src/services/
```

### Task 4: Delete TS `VALID_REACH_LEVELS` (the mutually-inconsistent 6)

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/models/holochain.model.ts:319–326,350`
- Modify: `app/elohim-app/src/app/elohim/models/holochain-connection.model.ts:353–360,382`

**Interfaces:** none produced — pure deletion (recon 2026-07-23: zero consumers of the const outside the defining files).

- [ ] **Step 1: Verify zero consumers of the TYPE too**

```bash
cd /projects/elohim && grep -rn "ValidReachLevel" app --include="*.ts" | grep -v dist | grep -v "\.spec\." | grep -v "models/holochain"
```
Expected: no output. If a consumer appears: re-point it to `import type { Reach } from '@elohim/storage-client'` (the ts-rs-generated schema-8 type) instead of deleting blind, and add that file to the commit.
- [ ] **Step 2:** In both files delete the `VALID_REACH_LEVELS` const block AND the `export type ValidReachLevel = …` line. Leave the neighboring `VALID_RELATIONSHIP_TYPES` / `VALID_DIFFICULTY_LEVELS` untouched. Add no replacement — declared reach in TS is the generated `Reach` type (schema codegen), never a hand-typed list (spec §1 drift-prevention law).
- [ ] **Step 3: Gates**

```bash
cd /projects/elohim/app/elohim-library && pnpm exec eslint projects/elohim-service/src --ext .ts 2>&1 | tail -3
cd /projects/elohim/app/elohim-library/projects/elohim-service && pnpm test 2>&1 | tail -5
cd /projects/elohim/app/elohim-app && pnpm run lint 2>&1 | tail -3 && pnpm exec vitest run --config vite.config.ts src/app/elohim 2>&1 | tail -5
```
Expected: green (or pre-existing reds only — verify by `git stash && rerun && git stash pop` if any red appears).
- [ ] **Step 4: Commit**

```bash
cd /projects/elohim && git add app/elohim-library/projects/elohim-service/src/models/holochain.model.ts app/elohim-app/src/app/elohim/models/holochain-connection.model.ts && git commit -m "refactor(reach): delete VALID_REACH_LEVELS 6-value strand — schema-8 generated Reach is the only TS declared-reach vocabulary (slice 1)" -- app/elohim-library/projects/elohim-service/src/models/holochain.model.ts app/elohim-app/src/app/elohim/models/holochain-connection.model.ts
```

### Task 5: Close the loop on the paper trail

**Files:**
- Modify: `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md` (append disposition note)

- [ ] **Step 1:** Append to the strand doc:

```markdown

---

## Slice-1 disposition (2026-07-23, shift/reach-vocab-slice1)

Per `reach-ontology-vocabulary-split-spec` §1: vocabulary **#2 (Rust services kebab-8)** and
**#5 (`VALID_REACH_LEVELS` 6)** are RETIRED — elohim-storage re-exports `elohim_epr::Reach`
(schema-8) with `ReachStandingExt` floor semantics and a legacy-alias parser
(`parse_reach_key`; data-aware migration, old manifests keep evaluating); both TS
`VALID_REACH_LEVELS` definitions deleted (zero consumers). Drift test:
`elohim-storage/tests/reach_vocabulary_contract.rs` pins Rust↔schema.
Remaining strands: geographic-8 rename (locality) and Part-V custody rename — later slices.
```

- [ ] **Step 2:** Commit:

```bash
cd /projects/elohim && git add genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md && git commit -m "docs(reach): record slice-1 disposition in the vocabulary strand" -- genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
```

- [ ] **Step 3 (final verification):** `cd /projects/elohim && git log --oneline shift/reach-vocab-slice1 ^HEAD@{u} 2>/dev/null || git log --oneline -5` — expect the 5 slice commits; leave the branch un-pushed (integrator owns push/merge).

---

## Self-review notes

- **Spec coverage:** DoD item 1 fully (canonical confirmed via contract test; services-8 migrated; `VALID_REACH_LEVELS` removed; drift test added). DoD item 5 partially (the two touched consumers reconciled; doorway/steward/DNA consumers untouched — they already use schema-8 or are dormant, per the strand doc's liveness verdicts). Items 2/3/4/6 are explicitly later slices.
- **Ambiguity flagged, not hidden:** legacy-`public` maps to `Commons` only for stored old-vocabulary data; the canonical string `"public"` always parses to `Public`. One test pins each side.
- **Risk:** `reach_earning.rs` internals were recon'd by grep, not full read — Task 3 Step 1 is compiler-driven for that reason; the pinned mapping table is the invariant, not a line-by-line diff.
- **P2P design gate:** slice 1 creates NO new data entity, table, route, or sync message — it consolidates an existing enum onto its already-notarized source of record (`reach.schema.json` ↔ DNA `CORE_REACH_LEVELS`). The gate ran at spec time (`reach-ontology-vocabulary-split-spec`); audit hits on this file are keyword matches on the word "schema", not new-entity findings.
- **a2o note:** spec DoD item 6 (composition-law scenarios) belongs to the verdict-surface slice, not here — nothing behavioral changed for learners in slice 1 (story-harvest at branch finish should confirm).
