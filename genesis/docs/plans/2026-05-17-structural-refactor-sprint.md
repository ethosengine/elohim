# Structural Refactor Sprint — Compilation Boundaries + SDK Cleanup

> ## 📦 SUPERSEDED 2026-05-18
>
> This skeleton plan has been replaced by three executable plans, orchestrated by `genesis/docs/specs/2026-05-18-refactor-orchestration.md`:
>
> - **Plan 1** — `genesis/docs/plans/2026-05-18-app-manifest-modularization.md` (new emphasis: manifest splits per concern)
> - **Plan 2** — `genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md` (Phase D of this old plan, fleshed out)
> - **Plan 3** — `genesis/docs/plans/2026-05-18-monolithic-code-decomposition.md` (Phases A/B/C of this old plan + new p2p/mod.rs target)
>
> Phases E (elohim-provenance), F (elohim-schema-tools), G (elohim-test-fixtures), H (feature-gating), and I (multi-workspace unification) from this old plan are explicitly deferred until after Plans 1-3 land — see the orchestration spec's Non-Goals section.
>
> **Reading this file** remains useful for the architectural intent of the deferred phases, but the executable work has moved.
>
> ---
>
> ## 🚦 [Historical] GATE — REVISIT BEFORE EXECUTING
>
> **This plan is INTENTIONALLY SKELETON-FORM.** It captures the deferred refactor work in dependency order while the design context is fresh, but each phase needs operator review and detail fill-in before any task is executed.
>
> **Before executing any phase:**
> 1. The companion plan (`2026-05-17-cargo-registry-and-compilation-load-reduction.md`) must be complete and its measurements doc landed — that's the baseline this work is measured against.
> 2. Re-validate the phase's file paths against the current codebase state (files may have moved/changed/grown since this plan was written).
> 3. Fill in the marked `_FILL IN_` slots with exact file paths, code blocks, expected outputs. The bite-sized-task discipline from `superpowers:writing-plans` is REQUIRED before subagent dispatch.
> 4. Decide phase ordering: phases are independent enough that some can run in parallel; some have soft dependencies noted in each phase's "Depends on" line.
> 5. Operator-explicit go-ahead required — none of these phases should be subagent-dispatched without a human signing off on the touched-up plan.
>
> **Why gate it this way:** the conversation that produced this plan covered design intent rather than implementation detail. The discipline is captured (sibling-module pattern, SDK boundary, utility crate extraction strategy, feature-gating, multi-workspace unification), but the specifics will change between now and execution. A flush-from-context dispatch would produce sloppy work.
>
> ---

**Goal:** Land the structural refactor work that the cargo-registry plan deferred: decompose the three monolithic files (http.rs / content_store/lib.rs / views.rs) into sibling modules; extract the SDK boundary cleanly; pull out the three highest-value utility crates (provenance, schema-tools, test-fixtures); audit feature-gating; and optionally unify the multi-workspace structure.

**Architecture:**
- **Sibling-module discipline within crates** is the working decomposition exemplar (demonstrated by last night's `graph_views/lamad/` + `graph_views/shefa/` landing). Module boundaries scope reasoning without creating new compile-artifact boundaries. This pattern is applied to the three giants: http.rs (10k LOC), content_store/lib.rs (12k LOC), views.rs (8k LOC).
- **SDK boundary extraction** separates implementation (elohim-storage internals) from public surface (elohim-sdk + the ts-rs-anchored types). Once separated, internals can refactor freely without touching consumers. Enforces [[project_elohim_dna_as_sdk_boundary]] structurally.
- **Utility crate extraction** for crates that are stable, reused across 2+ workspaces, and benefit from independent build cache. Three candidates: elohim-provenance (CID/signatures/EPR header parsing), elohim-schema-tools (JSON schema validator + codegen helpers), elohim-test-fixtures (sweettest harness + persona seeders).
- **Feature-gating audit** for heavy optional subsystems: libp2p, iroh, holochain-conductor, cozo. The graph-native sprint already feature-gated `graph-native`; same shape applies to others.
- **Multi-workspace unification** (hardest, last) brings the currently-excluded crates (`elohim-storage`, `elohim-cache-core`, `elohim/holochain`, `rust-ipfs`, `sdk`) into the elohim/ workspace. Collapses dep-version drift surface but exposes whatever latent version pinning issues exist today.

**Tech Stack:** Same as plan 1.

---

## Phase Dependency Map

```
                    [Plan 1: cargo-registry + compilation load reduction]
                                          |
                                          v
                       ┌──────────────────┴──────────────────┐
                       v                                     v
              ┌────────────────────┐              ┌────────────────────┐
              │ Phase A:           │              │ Phase B:           │
              │ http.rs decomp     │  (parallel)  │ content_store/     │
              │ (sibling modules)  │              │ lib.rs decomp      │
              └────────────────────┘              └────────────────────┘
                       |                                     |
                       └──────────────────┬──────────────────┘
                                          v
                       ┌────────────────────────────────────┐
                       │ Phase C: views.rs sibling-module   │
                       │ migration (incremental)            │
                       └────────────────────────────────────┘
                                          v
                       ┌────────────────────────────────────┐
                       │ Phase D: SDK boundary extraction   │
                       │ (elohim-sdk separation from        │
                       │  elohim-storage internals)         │
                       └────────────────────────────────────┘
                                          v
                       ┌──────────────────┴──────────────────┐
                       v                                     v
              ┌────────────────────┐              ┌────────────────────┐
              │ Phase E:           │              │ Phase F:           │
              │ elohim-provenance  │ (parallel)   │ elohim-schema-     │
              │ extraction         │              │ tools extraction   │
              └────────────────────┘              └────────────────────┘
                       |                                     |
                       └──────────────────┬──────────────────┘
                                          v
                       ┌────────────────────────────────────┐
                       │ Phase G: elohim-test-fixtures      │
                       │ extraction                         │
                       └────────────────────────────────────┘
                                          v
                       ┌────────────────────────────────────┐
                       │ Phase H: feature-gating audit      │
                       │ (libp2p/iroh/holochain/cozo        │
                       │  behind features)                  │
                       └────────────────────────────────────┘
                                          v
                       ┌────────────────────────────────────┐
                       │ Phase I (OPTIONAL, HARDEST):       │
                       │ multi-workspace unification        │
                       └────────────────────────────────────┘
```

Phases A and B can run in parallel after Plan 1 completes. Phase C should follow them (views.rs is the most cross-cutting). D depends on the giants being broken up. E and F can run in parallel after D. G and H follow. I is optional and last.

---

## Phase A — http.rs Sibling-Module Decomposition

**Goal:** Break `elohim/elohim-storage/src/http.rs` (10,199 LOC) into focused sibling modules. Same crate, same compile boundary, but module-level reasoning scope.

**Depends on:** Plan 1 complete (sccache repaved, dep tree clean).

**Estimated effort:** 1 week.

**Pre-execution gate:**
- [ ] Confirm http.rs is still ~10k LOC (re-run `wc -l elohim/elohim-storage/src/http.rs`)
- [ ] Identify the natural decomposition seams (read the file; look for `// ============ Auth ============` style separators)
- [ ] Decide on the sibling-module structure: typically `http/{mod,routes,handlers,middleware,error,extractors}.rs` or by-domain (`http/{lamad,shefa,qahal,imagodei,doorway_proxy}.rs`)
- [ ] Verify the existing test suite covers the route surfaces being moved
- [ ] **Operator sign-off on the chosen seam structure before subagent dispatch**

**Skeleton tasks (FILL IN BEFORE EXECUTING):**

### Task A.1: Survey the existing structure
- Read http.rs in full; identify section markers, route grouping, shared helpers
- Document the proposed module split in a short design note before any code moves
- Files: `_FILL IN once survey is done_`

### Task A.2: Create the mod.rs skeleton
- Move `pub mod ...` declarations and re-exports into `http/mod.rs`
- Keep http.rs as a temporary shim that re-exports from the new modules during transition
- Files: `elohim/elohim-storage/src/http/mod.rs` (create), `elohim/elohim-storage/src/http.rs` (modify → shim)

### Task A.3-A.N: Migrate one module per task
- For each identified seam (per-domain or per-concern), move the relevant code into a sibling module
- Each task = move + verify build + commit
- _FILL IN specific module names + line ranges once survey complete_

### Task A.final: Remove the shim
- Once all routes are in sibling modules, delete the http.rs shim
- Update any consumers that imported from `crate::http::*` to use the new module paths
- Run the full test suite + a2o pre-push gate

**Success criteria:** http.rs reduced from 10,199 LOC to <500 LOC of mod re-exports, OR removed entirely. Every test still green. No change in external API surface.

---

## Phase B — content_store/lib.rs Sibling-Module Decomposition

**Goal:** Break `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (12,197 LOC — the largest single file in the codebase) into focused sibling modules.

**Depends on:** Plan 1 complete.

**Estimated effort:** 1 week.

**Pre-execution gate:**
- [ ] Confirm lib.rs is still ~12k LOC
- [ ] Identify the natural decomposition seams (HDK functions for content CRUD vs validators vs queries vs index entries)
- [ ] Decide module structure: probably by entry-type (`{content,relationship,attestation,reach,validators}.rs`)
- [ ] Verify the integrity zome (separate crate) doesn't import from content_store internals in ways the split would break
- [ ] Confirm sweettest coverage is solid before moving things
- [ ] **Operator sign-off on the chosen seam structure**

**Skeleton tasks (FILL IN BEFORE EXECUTING):**

### Task B.1: Survey + design note
### Task B.2: Create lib.rs mod skeleton
### Task B.3-B.N: Migrate one entry-type or concern per task
### Task B.final: Remove shim if applicable

**Success criteria:** lib.rs reduced to <500 LOC. All zome tests + sweettest passes. Validators still pass. DNA hash unchanged (the migration is internal-module-only; HDK signatures are unchanged).

**Risk:** changing module layout inside a zome can affect DNA hash if `pub use` re-exports drift. Verify hash stability after each task: `hc dna hash <path>`.

---

## Phase C — views.rs Sibling-Module Migration (Incremental)

**Goal:** Reduce the ts-rs anchor pressure on `elohim/elohim-storage/src/views.rs` (8,208 LOC) by migrating new view types into per-domain sibling modules following the `graph_views/lamad/` and `graph_views/shefa/` exemplar from the 2026-05-16 graph-native landing.

**Depends on:** A, B (so monolithic patterns are addressed broadly first).

**Estimated effort:** 2 weeks (incremental; views.rs continues to function during migration).

**Pre-execution gate:**
- [ ] Read the existing `graph_views/lamad/` and `graph_views/shefa/` modules — they ARE the pattern this phase generalizes
- [ ] Identify which view types in views.rs are candidates for migration (probably all of them eventually; do it in domain-batches)
- [ ] Decide the target module structure: probably `views/{shared,lamad,shefa,qahal,imagodei,infrastructure}.rs` mirroring the existing `graph_views/`
- [ ] Verify the ts-rs codegen pipeline still produces correct TypeScript output after each migration batch — this is the critical contract surface
- [ ] **Operator sign-off**

**Skeleton tasks (FILL IN BEFORE EXECUTING):**

### Task C.1: Survey + module structure design
### Task C.2: Migrate lamad views (per-type, each commit verifies ts-rs export)
### Task C.3: Migrate shefa views
### Task C.4: Migrate qahal views
### Task C.5: Migrate imagodei views
### Task C.6: Migrate infrastructure views
### Task C.7: Shared primitives in views/shared.rs (anything used by 2+ domains)
### Task C.final: views.rs becomes a thin re-export shim or is removed

**Success criteria:** views.rs reduced to <500 LOC. Each domain has its own ts-rs-anchored module. The TypeScript exports (`elohim/sdk/storage-client-ts/src/generated/`) are byte-identical before and after each task. No change in external API surface.

**Risk:** ts-rs export ordering is path-dependent. The codegen output file list could change unexpectedly. Verify with `git diff` after each migration that the .ts output is identical OR that any diff is purely cosmetic.

---

## Phase D — SDK Boundary Extraction

**Goal:** Cleanly separate elohim-storage (the implementation) from elohim-sdk (the public consumed surface). Today, elohim-storage ships types that are *both* implementation internals AND consumed by external consumers via ts-rs. After this phase, elohim-storage depends on elohim-sdk; consumers depend on elohim-sdk only.

**Depends on:** A, B, C (the giants need to be decomposed first so the SDK extraction has clean cuts to work with).

**Estimated effort:** 2-3 weeks.

**Pre-execution gate:**
- [ ] Read the existing `crates/elohim-sdk/` to see what's already extracted
- [ ] Identify the types in views.rs that belong in elohim-sdk (anything ts-rs-exported and consumed by an external crate)
- [ ] Identify the storage-internal types that should NOT be in the SDK (e.g. Diesel models, internal error types, P2P transport details)
- [ ] Design the dependency: `elohim-sdk` declares the wire contract; `elohim-storage` depends on `elohim-sdk` and implements it; consumers (`doorway-service`, `steward/node`, `crates/elohim-storage-client`) depend on `elohim-sdk` only
- [ ] Verify [[project_elohim_dna_as_sdk_boundary]] memory entry stays valid (boundary should fail at compile time if leaks occur)
- [ ] **Operator sign-off on the boundary cuts**

**Skeleton tasks (FILL IN BEFORE EXECUTING):**

### Task D.1: Audit current elohim-sdk contents
### Task D.2: Inventory SDK-surface types in elohim-storage (the ts-rs-exported types)
### Task D.3: Inventory implementation-internal types (must NOT be in SDK)
### Task D.4: Move SDK-surface types to elohim-sdk crate
### Task D.5: Update elohim-storage to depend on elohim-sdk (path-dep within elohim/ workspace, OR registry-dep if you've published elohim-sdk)
### Task D.6: Switch each consumer to depend on elohim-sdk only
### Task D.7: Publish elohim-sdk 0.1.0 to cargo-internal registry
### Task D.8: Add a compile-time boundary check (e.g. dependency-cruiser-style or cargo-deny rule) that prevents elohim-storage from re-exposing internal types through the SDK

**Success criteria:** Consumers can compile against elohim-sdk alone without pulling elohim-storage. The set of types in elohim-sdk is finite and reviewable. elohim-storage internals can refactor freely without touching elohim-sdk.

---

## Phase E — elohim-provenance Crate Extraction

**Goal:** Extract CID computation, signature verification, EPR header parsing, and related provenance primitives into their own crate. Currently scattered across `epr`, `elohim-storage`, `doorway-service`.

**Depends on:** D (SDK boundary done; provenance types need a clean home).

**Estimated effort:** 1 week.

**Pre-execution gate:**
- [ ] Grep for `compute_cid`, `verify_signature`, `parse_epr_head`, `Cid::from_*` across the workspace
- [ ] Identify the canonical implementation (probably in `elohim/epr/`) and the duplicate/divergent implementations
- [ ] Design the crate's public API
- [ ] **Operator sign-off**

**Skeleton tasks (FILL IN BEFORE EXECUTING):**

### Task E.1: Inventory provenance functions across consumers
### Task E.2: Create `elohim/elohim-provenance/` crate skeleton
### Task E.3: Move canonical implementations
### Task E.4: Update consumers to depend on elohim-provenance
### Task E.5: Delete duplicate/divergent implementations
### Task E.6: Publish elohim-provenance 0.1.0

**Success criteria:** Single source of truth for CID/sig/EPR-header logic. Consumers depend on the crate. No duplicate definitions remain.

---

## Phase F — elohim-schema-tools Crate Extraction

**Goal:** Extract the JSON-schema validation harness, schema codegen helpers, and manifest validator into their own crate. Currently scattered across `elohim/sdk/schemas/scripts/`, `elohim/rakia/schemas/scripts/`, `genesis/orchestrator/validate-manifests.mjs`-equivalent Rust code.

**Depends on:** D.

**Estimated effort:** 1 week.

**Pre-execution gate:**
- [ ] Audit which schema-validation code lives where today
- [ ] Decide what's Rust vs what's Node/TS (Node-side stays as-is; this crate is Rust)
- [ ] Design API surface
- [ ] **Operator sign-off**

**Note on the naming:** despite the "schema" in the crate name, this is JSON-schema-validation tooling, NOT a DHT storage schema. The crate is utility/tooling, not protocol primitives. (Flagging because the P2P design audit hook keeps catching the word "schema" here.)

**Skeleton tasks:** mirrors Phase E. _FILL IN_.

**Success criteria:** Single canonical JSON-schema validation crate. Consumers (genesis/orchestrator, elohim-storage manifest validation, rakia validators) depend on it.

---

## Phase G — elohim-test-fixtures Crate Extraction

**Goal:** Extract the sweettest two-conductor harness, persona seeders, and test fixture utilities into a `[dev-dependencies]`-style utility crate. Currently in `elohim/holochain/tests/sweettest/`.

**Depends on:** E, F (provenance and schema-tools may be transitively pulled into fixtures).

**Estimated effort:** 1 week.

**Pre-execution gate:**
- [ ] Inventory the sweettest helpers + persona seeders
- [ ] Decide whether the crate should be its own workspace member OR a `[dev-dependencies]` crate in a subdirectory of an existing crate
- [ ] **Operator sign-off**

**Skeleton tasks:** mirrors Phase E. _FILL IN_.

**Success criteria:** Tests across multiple zomes can share fixture setup. No code duplication.

---

## Phase H — Feature-Gating Audit

**Goal:** Identify subsystems in elohim-storage (and other crates) that can be feature-gated so default builds don't pay their compile cost when not needed. The 2026-05-16 graph-native sprint already feature-gated `graph-native`; this phase generalizes the pattern.

**Depends on:** D (SDK boundary done; feature gates should not affect SDK contracts).

**Estimated effort:** 1-2 weeks.

**Pre-execution gate:**
- [ ] Identify candidate subsystems:
  - `libp2p` (transport for one of two P2P paths)
  - `iroh` (transport for the other)
  - `holochain-conductor` (embedded conductor; not needed for many builds)
  - `cozo` (graph engine; already gated as `graph-native`)
  - `rendering` (V8/deno_core; already partially optional)
- [ ] Decide default features for each consumer (steward-node, doorway-service, elohim-node, etc.)
- [ ] Verify each subsystem's API can be gated cleanly without breaking consumers
- [ ] **Operator sign-off on the default-feature set**

**Skeleton tasks (FILL IN BEFORE EXECUTING):**

### Task H.1: Audit current features
### Task H.2: Add `libp2p` feature gate
### Task H.3: Add `iroh` feature gate
### Task H.4: Add `holochain-conductor` feature gate
### Task H.5: Add additional feature gates as identified
### Task H.6: Update CI matrix to test all feature combinations
### Task H.7: Update build-manifest source globs to reflect feature scoping

**Success criteria:** Default builds compile noticeably faster. Consumers can opt-out of subsystems they don't need. CI tests `--no-default-features --features <X>` combinations.

**Risk:** Feature unification can produce surprising rebuilds. Audit `cargo tree --features` outputs to verify the right thing happens for each consumer.

---

## Phase I — Multi-Workspace Unification (OPTIONAL, HARDEST)

**Goal:** Bring the currently-excluded crates (`elohim-storage`, `elohim-cache-core`, `elohim/holochain`, `rust-ipfs`, `sdk`) into the elohim/ workspace. Collapses dep-version drift surface but exposes any latent version pinning issues.

**Depends on:** A through H (do not attempt this until the structural refactor is solid).

**Estimated effort:** 3-4 weeks. **HIGH RISK.**

**Pre-execution gate:**
- [ ] Plan 1's measurements show clear remaining compile-pressure value from this unification (don't do it if the previous phases already brought PVC pressure under control)
- [ ] Cross-workspace Cargo.lock diff analysis: how far do versions diverge today?
- [ ] Identify Holochain pinned-version requirements that may not be compatible with the elohim/ workspace's deps
- [ ] Decide which currently-excluded crates are worth unifying — probably elohim-cache-core (small, clean) first; elohim-storage second; holochain last (or never, if its pinning constraints are fundamentally incompatible)
- [ ] **Operator sign-off REQUIRED — this is the highest-risk phase**

**Skeleton tasks (FILL IN BEFORE EXECUTING):**

### Task I.1: Cross-workspace Cargo.lock diff analysis
### Task I.2: Unify elohim-cache-core first (smallest blast radius)
### Task I.3: Verify holochain pinning compat
### Task I.4: Unify elohim-storage (largest gain, largest risk)
### Task I.5: Decide on holochain — unify or leave excluded

**Success criteria:** Compile-pressure drops measurably (compare against Plan 1's baseline). No regressions. Cargo.lock is single source of truth for the merged set.

**Failure mode:** if unification reveals incompatible version pins that can't be resolved, the phase is REVERTED and the excluded-workspaces structure stays. The plan must include a clean revert path at every step.

---

## Cross-Cutting Concerns

**During every phase:**
- Run `cargo test --workspace --lib` after every meaningful change
- Run `cargo tree -d --workspace` to ensure dedupe progress doesn't regress
- Run pre-push gates to ensure CI stays green
- After each task, capture a short note in the relevant phase's "lessons learned" section (to be added during fill-in)
- If a phase reveals architectural issues that aren't covered here, STOP and add a memory entry capturing the issue before continuing

**Sequencing decision points:**
- After Phase C: revisit whether the SDK extraction (Phase D) is still the right next move or whether one of E/F/G provides more value first
- After Phase H: revisit whether multi-workspace unification (Phase I) is still worth the risk
- Phases are independent enough that the operator can re-sequence based on what the cargo-registry plan's measurements reveal

---

## Self-Review Notes

**Coverage:**
- Sibling-module decomposition of all three giants (Phases A, B, C)
- SDK boundary extraction (Phase D)
- Three utility crate extractions (Phases E, F, G)
- Feature-gating audit (Phase H)
- Multi-workspace unification (Phase I)

**Risk acknowledgment:**
- This plan is skeleton-form. Every phase requires fill-in before execution.
- The `[[memory-entry]]` references throughout assume the memory state at 2026-05-17. Re-verify before each phase.
- Module-layout changes in zomes (Phase B) can affect DNA hash. Verify per-task.
- ts-rs export ordering is path-dependent (Phase C). Diff TypeScript output per-task.
- Phase I is high-risk and optional; do not attempt without phases A-H being solid.

**Sequencing notes:**
- The phase-dependency graph above is the recommended ordering. Phases A and B can run in parallel; E and F can run in parallel.
- Phase I is optional. The structural improvements through H may bring PVC pressure under control sufficient that I isn't worth the risk.

---

## 🚦 Re-stating the Gate

**No phase in this plan should be subagent-dispatched in its current form.**

Before executing any phase:
1. Re-read the pre-execution gate checklist for that phase
2. Fill in the `_FILL IN_` slots with exact paths, code blocks, expected outputs (the bite-sized-task discipline from `superpowers:writing-plans`)
3. Re-verify against the current codebase state
4. Operator explicit go-ahead

**Plan complete and saved to `genesis/docs/plans/2026-05-17-structural-refactor-sprint.md`.**

This plan is companion to `2026-05-17-cargo-registry-and-compilation-load-reduction.md`. Execute Plan 1 first; touch up each phase of this plan before executing.
