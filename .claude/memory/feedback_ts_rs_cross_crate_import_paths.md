---
name: ts-rs-cross-crate-import-paths
description: "ts-rs 10.x computes cross-crate import paths in generated TypeScript using the Rust source crate's file path, NOT the export_to directory. If type A in crate X references type B that was moved to crate Y, A's generated .ts gets an import like '../../../../crateY/src/...' instead of './B'. Verified by 2026-05-18 PILOT (Plan 2 T4): 71 of 356 .ts files broke when shared primitives moved to elohim-sdk while consumer types stayed in elohim-storage. The fix: move ALL ts-rs-anchored types together to one crate in a single atomic migration — no partial moves, no incremental decomp."
metadata:
  node_type: memory
  type: feedback
---

When refactoring ts-rs-anchored types across crate boundaries, the safe pattern is **atomic single-crate consolidation**, not incremental partial moves. Verified the hard way 2026-05-18.

## What goes wrong

`#[ts(export_to = "../../sdk/storage-client-ts/src/generated/")]` resolves the *output* path relative to the source crate's Cargo.toml directory. But when ts-rs generates a `.ts` file for type A that references type B, the **import path inside that .ts file** is computed differently — it uses the actual Rust source file's location relative to the `export_to` target.

Concretely: if `ContentView` (in `elohim-storage/src/views.rs`) has a field of type `Freshness` (moved to `elohim-sdk/src/views/shared.rs`), ts-rs generates:

```typescript
// In ContentView.ts (generated from elohim-storage):
import type { Freshness } from "../../../../elohim/sdk/storage-client-ts/src/generated/Freshness";
```

instead of the expected:

```typescript
import type { Freshness } from "./Freshness";
```

The path is computed by walking from `elohim-storage/src/views.rs` up to a common ancestor with `elohim-sdk/src/views/shared.rs`, then back down to the target — and the extra `../../` hops break the TypeScript-side relative resolution.

## The verified failure (2026-05-18 PILOT)

Plan 2 T4 attempted to move ViewSlice, Freshness, JsonValue wrapper, DataValue from `elohim-storage/src/views.rs` to `crates/elohim-sdk/src/views/shared.rs` while leaving the ~257 consumer View types in elohim-storage. Result:

- The 4 moved types' own `.ts` files were byte-identical to baseline ✓
- 71 of 356 generated `.ts` files in the storage-client-ts target broke with cross-crate import paths ✗
- Three ts-rs `export_bindings` tests in elohim-sdk ran and passed (the moved types exported correctly)
- Every elohim-storage type that referenced one of the moved primitives produced broken paths

The diagnosis was unambiguous: ts-rs cross-crate import path generation is the blocker, not the `export_to` retargeting itself.

## The verified-correct pattern

**Move ALL ts-rs-anchored types together to one crate in one atomic commit.** No cross-crate references means ts-rs sees a single source-file tree and emits import paths relative to that single tree → all imports become `./TypeName` form, matching the flat output directory.

For elohim, this means:

- `crates/elohim-views` holds **every** type with `#[derive(TS)] + #[ts(export)]` — Views, InputViews, shared primitives, everything
- `elohim-storage`, `elohim-sdk`, and any other consumer depends on `elohim-views`
- `cargo test export_bindings` runs from `elohim-views` (NOT from elohim-storage)
- `export_to` paths in elohim-views point at `../../elohim/sdk/storage-client-ts/src/generated/` (relative to crates/elohim-views/Cargo.toml)

The byte-identical verification reduces to a single `cargo test export_bindings` run from `elohim-views` and a sha256 diff.

## What this enables architecturally

- **Lightweight consumer crates**: `elohim-storage-client` and any future third-party Rust SDK can depend on `elohim-views` without pulling diesel/axum/libp2p/conductor transitively
- **Clean boundary**: `cargo-deny` rules ban `elohim-storage` from non-server consumer dep trees; consumers use `elohim-views` (and the `elohim-sdk` facade that re-exports it)
- **ts-rs sanity**: One crate owns the ts-rs export pass; no duplicate test infrastructure, no path drift between crates

## How to apply

1. **Never move ts-rs-anchored types incrementally across crate boundaries.** If you must split, move ALL or none.
2. **When designing a new boundary that touches ts-rs types, set up a single types crate from the start.** The `elohim-views` shape is the canonical pattern: one crate owns the wire-shape Rust types, others depend on it.
3. **Verify byte-identical generated TS** after the consolidation with a sha256 diff against the pre-move baseline. Anything else (line-count, file-count, eye-inspection) misses subtle import-path drift.
4. **Do NOT try to fix ts-rs's path resolution upstream** as part of a sprint. It's not a bug per se — ts-rs's behavior is consistent — it just doesn't fit the multi-crate-shared-output-directory pattern.

Related: `[[project_elohim_dna_as_sdk_boundary]]` (the SDK is a first-class contract); `[[feedback_design_for_a_generation_no_shortcuts]]` (do the right thing, not the expedient thing). The Plan 2 T4 PILOT failure cost ~5 minutes of cargo compilation; the resulting clarity will save weeks of incremental-decomp pain.
