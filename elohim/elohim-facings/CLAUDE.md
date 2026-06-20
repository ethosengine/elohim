# elohim-facings — the pure lens crate

The **select→fold→aggregate** substrate for *facings*. A facing materializes a relation ONCE
and folds it into the lenses it needs. The **resiliency facing** (`folds/resiliency.rs`) is the
reference implementation; the four child lens charters (`reach-projection`, `rea-economic`,
`operational-weave`, `epr-content-perspective`) each add a sibling `folds/<facing>.rs` over the
same combinators.

Design: `genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md`
(§3 the holder-relation, §11 the Lens Framework).

## The boundary is the dependency graph, not a lint

This crate depends on **`elohim-views` + `std` ONLY**. It has **no `diesel`, no `elohim-storage`**.
A fold that reaches for a `&mut SqliteConnection` (or writes `use diesel;`) **fails to compile** —
the unresolved import IS the DB boundary (verified: `use diesel::prelude::*;` → `error[E0433]`).
Do **not** add `diesel`, `elohim-storage`, or a DB-access dependency here. The impure loaders
(`load_holder_relation`, manifest/region queries) live in
`elohim-storage::services::household_resilience`, which calls these folds as a thin adapter.

Add `serde`/`chrono` only when a *moved* fold's own type genuinely needs it — not preemptively.

## Module map

| File | Holds |
|------|-------|
| `relation.rs` | `HolderRow` — the materialized `(hub, agent, region)` holder tuple the folds consume. |
| `fold.rs` | Generic combinators: `bucket_by` (groups by key, drops `None`), `distinct_count_by`. |
| `folds/resiliency.rs` | The resiliency facing's folds: `stewarding_hubs`, `intra_hub_peers`, `regional_distribution`, `floor_for_tier`, `build_felt_status` (+ the 10 `felt_status_tests`). |

## Adding a lens (the §11 recipe)

1. Add `folds/<facing>.rs` — pure fns over a `&[Row]` slice. **Define your `Row` INSIDE this fold
   file** (not in `relation.rs` — that is the shared `HolderRow` home; see *Parallel lens sessions*
   below). Reuse `fold::bucket_by` / `distinct_count_by`; do not hand-roll iteration when a
   combinator composes (the `intra_hub_peers` rewrite is the proof: `bucket_by` ∘ `distinct_count_by`).
2. Register it in `folds/mod.rs` (`pub mod <facing>;`).
3. Storage side: add a `load_<facing>_relation` loader (impure, `&mut conn`) in a NEW
   `elohim-storage/src/services/<facing>_facing.rs` (not in `household_resilience.rs`), plus the
   route, following the charter's proof-fold-first slice.
4. Unit-test the fold DB-free in this crate. The folds return `elohim-views` types — no `ts-rs`
   move, no codegen, no schema change (the cross-crate ts-rs trap cannot bite here).

**The not-selected field contract (select→fold→aggregate).** A facing carries ONLY the lenses it
computed; a lens it didn't select is **absent**, not zero. So a per-entry lens field on a View type
(in `elohim-views`) MUST be:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
#[ts(optional)]
pub <lens>: Option<T>,
```

- `skip_serializing_if` → the wire **omits** the key when the lens wasn't selected.
- `#[ts(optional)]` → the generated TS reads `<lens>?: T` (NOT `T | null`), so ts-rs agrees with
  schema-codegen and the wire. **Missing ≡ not-selected**, never a present `null`. (Omitting
  `#[ts(optional)]` reintroduces the ts-rs `T | null` drift — the `intra_hub_peers` MED finding.)
- Combinators return `BTreeMap` (deterministic) so a lens may iterate a fold into a serialized
  `Vec` without leaking iteration order onto the wire.

## Parallel lens sessions — isolated vs shared surfaces

The four lens charters (reach-projection · rea-economic · operational-weave · epr-content-perspective)
can be built in parallel, but **only the fold layer is conflict-free**. The integration surface is
SHARED — respect this split or the sessions collide.

**Isolated (parallelize freely — one file per lens):**
- `elohim-facings/src/folds/<facing>.rs` — your folds + your fold unit tests (new file).
- Your `Row` struct — inside that fold file (NOT `relation.rs`, which is the shared `HolderRow` home).
- Your loader — a NEW `elohim-storage/src/services/<facing>_facing.rs` (NOT `household_resilience.rs`).

**Shared (must SERIALIZE — land one lens's integration before the next starts its):**
- `elohim-views/src/infrastructure.rs` — View types (ts-rs anchored): additive, but a shared file feeding global codegen.
- `elohim-storage/src/http.rs` — your route's match arm (+ the route-shadow guard below).
- `elohim/sdk/schemas/v1/views/*.schema.json` + `tests/schema_contract.rs` + `codegen-ts.mjs` `INTERFACE_FILES`.
- The generated TS (`export_bindings` → sdk; `schema:codegen:ts` → 5 app dirs) — **last-writer-wins**.

**Codegen is global and races.** `cargo test export_bindings` (ts-rs) and `pnpm run schema:codegen:ts`
regenerate SHARED files from the WHOLE crate / schema set. Two sessions running codegen concurrently
clobber each other. Rule: **run codegen only when integrating your lens, and commit the regenerated
files in the SAME commit as your View type.** Never run codegen while another session's View edit is in flight.

**Route-shadow guard.** A new `GET` route can be silently shadowed by a catch-all (the `/auth/portal`
incident: a `starts_with` arm ate the EPR router). Add a routing unit test asserting your path dispatches
to your handler, not the SPA/EPR fallthrough. (Doorway's main listener additionally needs an
`is_service_path` arm — memory `project_doorway_main_route_needs_is_service_path`.)

**Operationally:** one worktree per session; **serialize pushes** (concurrent pushes mutually abort the
orchestrator). Order: land **epr-content-perspective first as the reference** (loaders exist, no
cross-cutting block), then reach / rea / operational copy the pattern as their loaders + data land.

## Testing

`RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/<slot> cargo test --manifest-path elohim/elohim-facings/Cargo.toml`
(native build env — the ambient WASM getrandom flag breaks linking; see root CLAUDE.md Gotchas).
clippy gate: `cargo clippy --manifest-path elohim/elohim-facings/Cargo.toml --all-targets -- -D warnings`.
