---
title: SSR Two-Row Collapse — Implementation Plan (Phase 1 of the native-shell arc)
id: ssr-row-collapse-plan
status: Draft
class: protocol-canonical
domain: D8
topic: [ssr, doorway, storage, content-node, server-bundle, collapse, epr, implementation-plan, dataplane-trajectory]
informed-by:
  - genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md
cites:
  - native-rust-epr-shell-ssr-design | The design spec this plan implements (Phase 1: the row collapse only); carries the gate answers, the EPR-nature × peer-capability model, and the trajectory framing | path: genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md
requires_env: [household-nodes]
---

# SSR Two-Row Collapse — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse the server SSR bundle from a sibling `elohim-host-landing-ssr` content row into a `serverBlobHash` field on the **one** `elohim-host-landing` EPR node — eliminating the second-row deploy artifact whose missing seed 404'd and stranded elohim.host, and aligning the model with "one content-addressed EPR carries its full nature."

**Architecture:** The doorway/storage SSR runtime resolves the Angular **server** bundle by `GET /db/content/{slug}` → read a hash field → materialize. Today `SSR_BUNDLE_SLUG=elohim-host-landing-ssr` and the read field is `blobHash` (on the second row). After this plan, `SSR_BUNDLE_SLUG=elohim-host-landing` and the read field is **`serverBlobHash`** (on the one EPR node, alongside the browser `blobHash`). The deploy pipeline PATCHes `serverBlobHash` onto the one node instead of `blobHash` onto a second row.

**Tech Stack:** Rust (doorway-service, elohim-storage, elohim-render), Jenkins declarative pipeline + bash, JSON seed data, JSON-schema/ts-rs content view.

**This is Phase 1 of the native-shell arc.** It is the data-model correction only. Phase 2 (the native Rust chrome renderer + omnibar relocation + theme) is a separate plan that builds on this clean model. The omnibar's immediate theme staleness on elohim.host is already addressed independently by Track A (`2a09234c7`).

## Global Constraints

- **RUSTFLAGS="" for native builds** of doorway-service and elohim-storage (the Holochain WASM `getrandom` flag breaks native builds → undefined-symbol at link). elohim-render builds under the same override when built via these crates.
- **camelCase at the Rust↔TS / HTTP boundary.** The field is `serverBlobHash` on the wire (`#[serde(rename_all = "camelCase")]`); snake_case never leaves Rust.
- **`serverBlobHash` is OPTIONAL.** Absent ⇒ SSR simply isn't materialized (the existing `materialize_bundle` failure path returns `None` → CSR fallback, never a crash). This is the migration safety: the field can be absent on any host mid-transition without breaking serving.
- **Mirror `blobHash` exactly.** Whatever storage mechanism, PATCH path, and wire-surfacing `blobHash` uses, `serverBlobHash` uses the same. Read the storage content model first; if `blobHash` is a nullable column, add a parallel nullable `serverBlobHash` column (a trivial additive migration — **not** a new entity/table; the P2P gate still holds). If `blobHash` rides a JSON content blob, `serverBlobHash` rides it too (no migration).
- **No new DHT entry type, no new content row, no new table-as-entity.** The change *removes* a row and adds one nullable field to an existing node.
- **Jenkinsfile CPS discipline:** helpers stay heredoc-free; bash bodies live in `scripts/ci/*.sh` (the root `Jenkinsfile` sits near the 64KB CPS method limit).
- **Per-host publish is retained** (MVP scaffold — blobs don't auto-replicate). Only the ROW collapses; the pipeline still PATCHes both hosts, Track-A-isolated.
- **Commit-only.** Commit each task on `feat/frontend-eyes-sprint`; the integrator merges to `dev`.

---

### Task 1: Add `serverBlobHash` to the content node (schema + Rust model + wire)

**Files:**
- Modify: `elohim/sdk/schemas/v1/views/content-view.schema.json` (add `serverBlobHash`)
- Modify: the Rust content view struct that owns `blobHash` (find via `grep -rn "blobHash\|blob_hash" elohim/elohim-storage/src` — likely `elohim/elohim-storage/src/views.rs` or an `elohim-views` crate)
- Modify: the storage content model/persistence that stores+returns `blobHash` (mirror it; column or JSON per the Global Constraint)
- Test: `elohim/elohim-storage/tests/schema_contract.rs` (extend the content-view contract)

**Interfaces:**
- Produces: a content node JSON that may carry top-level `serverBlobHash: string | null`, surfaced by `GET /db/content/{slug}` and PATCHable like `blobHash`.

- [ ] **Step 1:** Locate exactly how `blobHash` is declared (schema), typed (`Option<String>` + `#[serde(rename = "blobHash")]` or `rename_all = "camelCase"` with `blob_hash`), persisted (column vs JSON), and PATCHed. Record the mechanism in the task report.
- [ ] **Step 2:** Add `serverBlobHash` to `content-view.schema.json` mirroring `blobHash` (nullable string, not required).
- [ ] **Step 3:** Add the parallel Rust field (`server_blob_hash: Option<String>`, camelCase on the wire) and the parallel persistence (column migration *only if* `blobHash` is columnar; else the JSON path). If a migration is needed, follow the diesel timestamp-collision guard (unique `YYYY-MM-DD-HHMMSS`).
- [ ] **Step 4:** Extend `schema_contract.rs` to assert the Rust struct ↔ schema round-trip includes `serverBlobHash`. Run: `RUSTFLAGS="" cargo test --lib schema_contract` (or the contract test name). Expected: PASS.
- [ ] **Step 5:** Regenerate TS types if applicable (`cargo test export_bindings`); verify byte-identical generated TS except the added field.
- [ ] **Step 6:** Commit: `feat(storage): add optional serverBlobHash to content node (SSR row collapse T1)`.

### Task 2: PATCH `serverBlobHash` onto the EPR node; verify the PATCH+read round-trips

**Files:**
- Modify: the storage PATCH handler for `/db/content/{slug}` (the one `stage-spa-blob.sh` calls) to accept `serverBlobHash` (mirror how it accepts `blobHash`)
- Test: a handler/unit test asserting `PATCH {serverBlobHash}` then `GET` returns it

**Interfaces:**
- Consumes: T1's field. Produces: `PATCH /db/content/elohim-host-landing {"serverBlobHash": "sha256-…"}` persists and is returned by the subsequent `GET`.

- [ ] **Step 1:** Find the PATCH handler accepting `blobHash` (grep `blobHash` in `elohim/elohim-storage/src/**` handlers). Add `serverBlobHash` to the accepted patch body (same partial-update semantics — must NOT clobber `blobHash` or other fields).
- [ ] **Step 2:** Write the failing test: PATCH `{serverBlobHash}`, GET, assert it equals; and PATCH `{blobHash}` then `{serverBlobHash}`, assert both retained (no clobber).
- [ ] **Step 3:** Implement minimal handler change. Run the test. Expected: PASS.
- [ ] **Step 4:** Commit: `feat(storage): accept serverBlobHash in /db/content PATCH (SSR row collapse T2)`.

### Task 3: Doorway resolves the server bundle from the EPR node's `serverBlobHash`

**Files:**
- Modify: `doorway/doorway-service/src/ssr.rs` (`DoorwayBundleSource::resolve_blob_hash` + `parse_blob_hash`)
- Modify: `doorway/doorway-service/src/server/http.rs:363-438` (`init_renderer` — the `SSR_BUNDLE_SLUG` branch)
- Test: `doorway/doorway-service/src/ssr.rs` `#[cfg(test)]` (a `parse_server_blob_hash` unit test)

**Interfaces:**
- Consumes: `GET /db/content/elohim-host-landing` returning `{… "serverBlobHash": "sha256-…"}`.
- Produces: `materialize_bundle` resolves the SERVER bundle from `serverBlobHash` on the EPR slug — no separate `-ssr` slug.

- [ ] **Step 1:** Add `parse_server_blob_hash(body) -> Result<String>` mirroring `parse_blob_hash` (ssr.rs:352) but reading `serverBlobHash`. Write a unit test with a JSON fixture carrying both `blobHash` and `serverBlobHash`, asserting it returns the server one. Run: `RUSTFLAGS="" cargo test --lib parse_server_blob_hash`. Expected: FAIL → PASS.
- [ ] **Step 2:** Add a `BundleSource` path that resolves via `serverBlobHash` (either a `resolve_server_blob_hash` on `DoorwayBundleSource`, or a kind-parameter). `elohim_render::materialize_bundle` currently calls `resolve_blob_hash`; introduce a server-bundle variant (e.g. `materialize_server_bundle` in elohim-render, or a `field` parameter) — see Task 4 for the elohim-render seam. Keep the change minimal and mirror the existing thread-isolation (`std::thread::spawn(...).join()`) for the blocking call.
- [ ] **Step 3:** In `init_renderer`, the `SSR_BUNDLE_SLUG` branch now resolves the server bundle from the EPR slug's `serverBlobHash`. (The env var value becomes `elohim-host-landing`; see Task 6.) On resolve failure, keep the existing `return None` → CSR fallback.
- [ ] **Step 4:** `RUSTFLAGS="" cargo build --release && cargo test --lib --bins ssr && cargo clippy -- -D warnings && cargo fmt --check`. Expected: green.
- [ ] **Step 5:** Commit: `feat(doorway): resolve SSR server bundle from EPR node serverBlobHash (SSR row collapse T3)`.

### Task 4: elohim-render — server-bundle materialize seam

**Files:**
- Modify: `elohim/elohim-render/src/bootstrap.rs` (`materialize_bundle` / `BundleSource`)
- Test: `elohim/elohim-render` unit test for the server-field resolve

**Interfaces:**
- Consumes: a `BundleSource` whose resolve reads `serverBlobHash`. Produces: `materialize_server_bundle(src, slug, dir)` (or `materialize_bundle(src, slug, dir, BundleKind::Server)`), used by both doorway (T3) and storage (T5).

- [ ] **Step 1:** Inspect `bootstrap.rs` `materialize_bundle` + the `BundleSource` trait. Decide the minimal seam: add `resolve_server_blob_hash` to the trait (default-delegating to `resolve_blob_hash` is NOT acceptable — it must read a different field), OR add a `BundleKind` parameter threaded to the source. Prefer a `BundleKind { Browser, Server }` param so one code path serves both.
- [ ] **Step 2:** Write the failing test: a mock `BundleSource` returning distinct hashes for browser vs server; assert `materialize_*` fetches the server hash for the server kind.
- [ ] **Step 3:** Implement. Run the elohim-render tests. Expected: PASS.
- [ ] **Step 4:** Reconcile with T3 (doorway) — the doorway's `DoorwayBundleSource` implements the new trait method. Re-run doorway build/tests.
- [ ] **Step 5:** Commit: `feat(render): server-bundle materialize seam reading serverBlobHash (SSR row collapse T4)`.

### Task 5: Storage SSR path (`LocalBundleSource`) — same resolve

**Files:**
- Modify: `elohim/elohim-storage/src/ssr.rs` (`LocalBundleSource` + `SsrState::from_env`)

**Interfaces:**
- Consumes: T4's seam. Mirrors T3 for the storage-self-SSR variant.

- [ ] **Step 1:** Apply the T3/T4 change to `LocalBundleSource`: resolve the server bundle via `serverBlobHash`. Mirror the thread-isolated blocking call.
- [ ] **Step 2:** `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --features ssr` (storage uses the WASM flag) — confirm it still links with the `ssr` feature. (Lean default build needs no SSR.)
- [ ] **Step 3:** Commit: `feat(storage): resolve SSR server bundle from serverBlobHash (SSR row collapse T5)`.

### Task 6: Pipeline + manifests — PATCH one node; point SSR_BUNDLE_SLUG at the EPR; drop the `-ssr` row

**Files:**
- Modify: `scripts/ci/stage-spa-blob.sh` (the `kind=server` PATCH writes `serverBlobHash` on `elohim-host-landing`)
- Modify: root `Jenkinsfile` `stageAndVerifyAllBundles` (remove the `elohim-host-landing-ssr` bundle entry; the server `distDir` now targets slug `elohim-host-landing` with `kind:server`)
- Modify: `genesis/orchestrator/manifests/doorway/{alpha,alpha-b}.yaml` + storage manifests — `SSR_BUNDLE_SLUG: elohim-host-landing` (was `elohim-host-landing-ssr`)
- Delete: `genesis/data/lamad/content/elohim-host-landing-ssr.json`
- Modify: `genesis/data/lamad/content/elohim-host-landing.json` (document `serverBlobHash` as deploy-populated, absent in source — mirror the `blobHash` `metadata.blobPopulatedBy` note)

**Interfaces:**
- Consumes: T1–T5. Produces: the deploy publishes the server bundle's hash to `serverBlobHash` on the one node, per host, Track-A-isolated.

- [ ] **Step 1:** In `stage-spa-blob.sh`, when `KIND=server`, PATCH `{"serverBlobHash": "${SPA_HASH}"}` to `/db/content/${SLUG}` (and verify-read `serverBlobHash`, not `blobHash`). The browser path is unchanged.
- [ ] **Step 2:** In `Jenkinsfile`, change the server bundle entry to `[distDir: ".../server", slug: "elohim-host-landing", kind: "server"]` and **remove** the `elohim-host-landing-ssr` slug. (The per-`(host,slug)` isolation from `2a09234c7` stays.)
- [ ] **Step 3:** Update `SSR_BUNDLE_SLUG` to `elohim-host-landing` in the doorway + storage manifests.
- [ ] **Step 4:** Delete `elohim-host-landing-ssr.json`; add the `serverBlobHash` deploy-populated note to `elohim-host-landing.json`.
- [ ] **Step 5:** `bash -n scripts/ci/stage-spa-blob.sh` (syntax) + a manual trace of the Jenkinsfile change (no local Jenkins). Confirm no `wc -l Jenkinsfile` regression past the CPS proxy.
- [ ] **Step 6:** Commit: `feat(ci): collapse SSR server bundle onto elohim-host-landing.serverBlobHash; drop -ssr row (T6)`.

### Task 7: Cleanup sweep + done-criteria

**Files:** repo-wide

- [ ] **Step 1:** `grep -rn "elohim-host-landing-ssr\|SSR_BUNDLE_SLUG" --include='*.rs' --include='*.sh' --include='*.yaml' --include='*.json' --include='Jenkinsfile' .` — confirm every reference now points at `elohim-host-landing` or is gone. No dangling `-ssr` slug.
- [ ] **Step 2:** Confirm the optional-field safety: with `serverBlobHash` absent, `init_renderer` returns `None` (CSR fallback), not a crash — covered by a doorway test or a reasoned note.
- [ ] **Step 3:** Update the spec's §8 step 2 status note (the collapse) — leave the spec authored by the integrator's cite tooling; do not hand-edit cites.
- [ ] **Step 4:** Final commit / progress-ledger update. The whole-branch review (subagent-driven Step "final review") runs the touched trees' gates: `RUSTFLAGS="" cargo clippy -- -D warnings && cargo fmt --check` for doorway + storage; `bash -n` for the script.

## Done

- `grep` shows zero live `elohim-host-landing-ssr` row references (slug gone from seed, pipeline, manifests, doorway/storage resolve).
- The doorway/storage SSR path resolves the server bundle from `elohim-host-landing.serverBlobHash`.
- `serverBlobHash` absent ⇒ CSR fallback, no crash.
- Doorway + storage gates green (clippy/fmt/tests).
- Verification that SSR still renders on a host is **post-merge** (integrator merges to `dev` → pipeline PATCHes `serverBlobHash` → doorway materializes). Captured as the post-deploy check, not a local gate.
