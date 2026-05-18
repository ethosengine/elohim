# Structural Refactor Orchestration — 2026-05-18

**Replaces:** `genesis/docs/plans/2026-05-17-structural-refactor-sprint.md` (skeleton, gated)

**Status:** Active. Three executable plans referenced below.

---

## Problem

Three load-bearing surfaces have grown beyond comfortable reasoning scope:

1. **App manifests** — `elohim/sdk/domains/lamad/manifest.json` is **1,923 LOC** mixing contentTypes, couplings, rendering, projections, signalKinds, and graph rules in a single file. The schema already supports `$ref` for sub-documents (used today for metadataSchemas), but contentTypes and signals are still inlined. Editing one concern means scrolling through unrelated concerns.

2. **SDK boundary** — `crates/elohim-sdk/` is a 50-line skeleton with `client`/`cache`/`sync`/`traits`/`reach` directories but almost no content. Meanwhile, `elohim-storage/src/views.rs` (8,208 LOC) holds the ts-rs-anchored View types that consumers actually depend on. The boundary is logically there but structurally absent — there is no compile-time wall preventing storage internals from leaking into the consumed surface.

3. **Monolithic code** — Four files dominate reasoning load:
   - `holochain/dna/elohim/zomes/content_store/src/lib.rs` — **12,197 LOC** (largest single file; Holochain zome; DNA-hash-sensitive)
   - `elohim-storage/src/http.rs` — **10,199 LOC** (single `impl HttpServer` block from line 280, route registrations from line 8145)
   - `elohim-storage/src/views.rs` — **8,208 LOC** (ts-rs anchor; per-view schemas at `sdk/schemas/v1/views/*.schema.json` already exist as exemplar)
   - `elohim-storage/src/p2p/mod.rs` — **6,279 LOC** (NEW culprit; 31 sub-modules already declared but the giant `impl P2PNode` at line 1501 is the body)

The `graph_views/{lamad,shefa}/` sibling-module exemplar (2026-05-16 graph-native landing) is the proven pattern to generalize.

## Goals

- **Plan 1 (Manifests)**: Split `lamad/manifest.json` into per-concern files referenced via `$ref` from a thin shell. Generated TypeScript output stays byte-identical (verified per task). Pattern documented for the other 7 domains as follow-on.
- **Plan 2 (SDK Boundary)** [REVISED 2026-05-18 post-PILOT]: Create lightweight `crates/elohim-views` crate holding ALL ts-rs-anchored View types in one atomic migration. `elohim-storage` depends on `elohim-views`; `elohim-sdk` re-exports `elohim-views` as a consumer-friendly facade with client helpers; lightweight consumers (`elohim-storage-client`, future third-party SDKs) depend on `elohim-views` alone — no transitive elohim-storage. The original incremental per-domain plan was retired after a T4 PILOT discovered ts-rs's cross-crate import-path mechanic breaks partial moves; see `[[feedback_ts_rs_cross_crate_import_paths]]`.
- **Plan 3 (Monoliths)**: Sibling-module decomposition of the four giants in dependency-respecting order. Each file reduced to <500 LOC of re-exports OR removed entirely. DNA hash stability verified per task on content_store. ts-rs output byte-identical per task on views.rs.

## Non-Goals (explicit)

- **Phases E/F/G from the old gated plan** (elohim-provenance, elohim-schema-tools, elohim-test-fixtures utility-crate extractions) are deferred until after Plan 3 lands. Premature without the boundary work.
- **Phase H (feature-gating audit)** deferred. The 2026-05-16 graph-native sprint already proved the pattern; generalizing it is downstream of the decompositions.
- **Phase I (multi-workspace unification)** deferred. High-risk; do not attempt until Plans 1-3 are solid and PVC pressure is measured against Plan 1-of-2026-05-17's baseline.
- **Other domain manifest splits** (shefa, qahal, imagodei, etc.) — Plan 1 documents the pattern via lamad; the others follow the same recipe in a follow-on plan if value emerges.

## Plan Sequencing

```
Plan 1 (manifests)       Plan 2 (SDK boundary)         Plan 3 (monoliths)
     |                          |                              |
     |                          |                              |
     v                          v                              v
small blast radius        medium blast radius          large blast radius
~1 week                   ~1-2 weeks                   ~3-4 weeks
independent               independent of P1            independent of P1+P2
                          some Plan 3 sub-phases       (Plan 3.Phase A "views.rs"
                          depend on P2 published       benefits from P2 landed
                                                       first to avoid double-move)
```

**Recommended execution order:** Plan 1 → Plan 2 → Plan 3.Phase A (views.rs) → Plan 3.Phase B (http.rs) → Plan 3.Phase C (p2p) → Plan 3.Phase D (content_store).

**Parallelism opportunities:** Plan 1 and Plan 2 are fully independent and CAN run in parallel by different subagent streams. Plan 3 phases are mostly sequential within themselves but Phase B and Phase C can overlap (no shared files).

## Plans

| Plan | Path | Blast radius | Approx tasks |
|---|---|---|---|
| 1. App-Manifest Modularization | `genesis/docs/plans/2026-05-18-app-manifest-modularization.md` | low | ~12 |
| 2. SDK Boundary Clarification | `genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md` | medium | ~14 |
| 3. Monolithic Code Decomposition | `genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md` | large | ~32 |

## Cross-cutting constraints

These apply to every task in every plan:

- `RUSTFLAGS=""` required for native cargo invocations (system default sets `--cfg getrandom_backend="custom"` which breaks native builds)
- `RUSTC_WRAPPER=""` for now (sccache poisoning unresolved; see `.claude/memory/feedback_sccache_cache_corruption_recovery.md`)
- `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev` per the cargo-pool preflight
- Subagent-driven-development discipline executes (fresh subagent per task, two-stage review: spec compliance then code quality)
- DNA-hash stability is load-bearing for content_store. HC 0.6 unstable-migration is the only path to changing it cleanly; we want to avoid that. Verify hash per task in Plan 3.Phase D.
- ts-rs export ordering is path-dependent. views.rs decomp (Plan 3.Phase A) and SDK type-moves (Plan 2) must verify byte-identical TypeScript output per task with `git diff --no-color elohim/sdk/storage-client-ts/src/generated/` after each migration.

## Success Criteria (sprint-level)

- `wc -l` of each named monolith drops by ≥80% (final size <500 LOC of re-exports OR removed)
- `lamad/manifest.json` shell file <300 LOC (was 1,923); per-concern files each <500 LOC
- `cargo build --workspace --release` time does not regress vs the 2026-05-17 baseline (25.6s cold)
- `cargo tree -d --workspace` count unchanged or lower (was 19 unique post-T12)
- No DNA-hash change on the elohim DNA (`hc dna hash elohim/holochain/dna/elohim/elohim.dna` byte-identical before and after Plan 3.Phase D)
- TypeScript generated outputs (`elohim/sdk/storage-client-ts/src/generated/`, `app/elohim-app/src/app/lamad/generated/`, `genesis/seeder/src/generated/`) byte-identical (or only cosmetically different — whitespace/ordering) before and after every type-moving task
- `cargo-deny check bans` passes with the new boundary rule that prevents `elohim-storage` from being a direct dep of `crates/elohim-storage-client` consumers

## What changes after these plans land

- Editing one concern of one app manifest is a single-file edit
- Adding a new view type means writing one schema, one Rust struct in one domain folder, and one re-export line — no scrolling through 8,000 lines
- A consumer of the SDK depends on a small, reviewable type surface; the implementation can refactor freely behind the boundary
- The Holochain zomes are reviewable per-concern, not per-file
- The P2P node's `impl P2PNode` is split by protocol concern, matching the 31 already-declared submodules
- New developers reading any of these surfaces hold one concern in head at a time

## Open follow-ups (post-sprint)

- Apply manifest-modularization pattern to shefa/qahal/imagodei/avodah/mishpat/infrastructure (smaller wins, same recipe)
- Resume elohim-provenance, elohim-schema-tools, elohim-test-fixtures extractions
- Feature-gating audit
- Multi-workspace unification (high-risk; only if measurements justify)
- Publish `elohim-sdk` 0.1.0 to Nexus (blocked behind the cargo-registry T9 Basic-auth issue — see `.claude/memory/feedback_nexus_cargo_publish_basic_auth.md`)
