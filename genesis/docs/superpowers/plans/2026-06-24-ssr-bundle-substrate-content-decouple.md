---
title: SSR Bundle as Substrate Content — Implementation Plan
id: ssr-bundle-substrate-content-decouple-plan
status: Draft
class: protocol-canonical
domain: D8
topic: [ssr, doorway, projection, content-addressing, app-bundle, build-decouple, render, implementation-plan]
informed-by:
  - genesis/docs/superpowers/specs/2026-06-24-ssr-bundle-substrate-content-decouple-design.md
cites:
  - ssr-bundle-substrate-content-decouple-design | The design spec this plan implements task-by-task; carries the gate answers, rollout phases, and decisions | sha256:78ab3f3b8646d0e7 | path: genesis/docs/superpowers/specs/2026-06-24-ssr-bundle-substrate-content-decouple-design.md
requires_env: [household-nodes]
---

# SSR Bundle as Substrate Content — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **P2P design gate (passed at spec time — no new entity):** This plan adds **no new DHT entry type, no new storage table, and no new HTTP route.** The `elohim-host-landing-ssr` content row reuses the existing content-blob class (same as the `elohim-host-landing` browser EPR); identity is the content-addressed blob CID; the `/db/content/{slug}`, `/admin/seed/blob`, and `/blob/{hash}` routes already exist. Full gate answers: spec §5. (The P2P auditor's route/schema flags are this reuse, not new design.)

**Goal:** Distribute the Angular SSR *server* bundle as content-addressed substrate content (like its browser sibling) and materialize it at boot in every rendering runtime, retiring the doorway image bake and the storage sed-strip.

**Architecture:** App pipeline zips `dist/elohim-app/server`, PUTs it as a blob, PATCHes content row `elohim-host-landing-ssr`. At boot, doorway (HTTP→`STORAGE_URL`) and SSR-enabled storage (local) resolve the slug → blobHash → fetch → verify → unzip into `SSR_BUNDLE_PATH`. Shared logic lives in a new `elohim-render::bootstrap` module. Fetch-at-boot + lazy-on-miss; graceful fall-through preserved.

**Tech Stack:** Rust (elohim-render/doorway/elohim-storage), bash CI (`scripts/ci/stage-spa-blob.sh`), Groovy (`Jenkinsfile`), k8s YAML manifests, `zip`+`sha2` crates.

**Spec:** `genesis/docs/superpowers/specs/2026-06-24-ssr-bundle-substrate-content-decouple-design.md`

## Global Constraints

- **Native Rust builds require `RUSTFLAGS=""`** (the env sets `--cfg getrandom_backend="custom"` for Holochain WASM, which breaks native builds). Applies to doorway, elohim-render.
- **Use an isolated target dir** to dodge the pool fingerprint-ENOENT trap: prefix cargo with `CARGO_TARGET_DIR=/tmp/ssr-sprint-target`.
- **Blob hash wire format is `sha256-<hex>`** (legacy marker, hyphen — see `scripts/ci/stage-spa-blob.sh`). The `materialize_bundle` integrity check MUST format/compare in this exact form.
- **Upload path:** `PUT /admin/seed/blob` with header `X-Blob-Hash` (write-through), NOT `PUT /blob/{hash}` (read-through/GET-only). Link via `PATCH /db/content/{slug}` `{"blobHash":"..."}`, verify via `GET /db/content/{slug}` `.blobHash`.
- **Graceful degradation is a hard invariant:** `SSR_BUNDLE_SLUG` unset OR any fetch/verify/unzip failure ⇒ renderer `None` ⇒ existing fall-through. Never crash the pod, never render from unverified bytes.
- **Branch:** `feat/frontend-eyes-sprint`. Commit per task; do NOT push (integrator owns merge). Stage only files this plan touches (shared worktree).
- **New slug:** `elohim-host-landing-ssr`. New env var: `SSR_BUNDLE_SLUG`.

---

### Task 1: `elohim-render::bootstrap` — fetch + verify + unzip (keystone)

**Files:**
- Create: `elohim/elohim-render/src/bootstrap.rs`
- Modify: `elohim/elohim-render/src/lib.rs` (add `pub mod bootstrap;` + re-exports)
- Modify: `elohim/elohim-render/Cargo.toml` (add `zip`, `sha2` deps if absent)
- Test: inline `#[cfg(test)]` in `bootstrap.rs`

**Interfaces:**
- Produces:
  - `pub trait BundleSource { fn resolve_blob_hash(&self, slug: &str) -> Result<String>; fn fetch_blob(&self, hash: &str) -> Result<Vec<u8>>; }`
  - `pub fn materialize_bundle<S: BundleSource>(src: &S, slug: &str, target_dir: &std::path::Path) -> Result<std::path::PathBuf>` — resolves slug→hash, fetches, verifies `format!("sha256-{:x}", Sha256::digest(&bytes)) == hash`, unzips into `target_dir`, returns `target_dir.join("main.server.mjs")`. Errors (not panics) on mismatch/unzip failure.
- Consumes: `crate::error::{Result, RenderError}` (extend `RenderError` with a `Bootstrap(String)` variant if no general variant fits).

- [ ] **Step 1: Write failing tests** in `bootstrap.rs` `#[cfg(test)]`:
  - `materialize_rejects_hash_mismatch`: a `BundleSource` returning a known hash but bytes whose sha differs ⇒ `materialize_bundle` returns `Err`.
  - `materialize_unzips_and_returns_entry`: build an in-memory zip (via the `zip` crate `ZipWriter`) containing `main.server.mjs` with body `"X"`; source returns the correct `sha256-<hex>` of those zip bytes; assert the returned path exists and reads back `"X"`.
  - `materialize_propagates_resolve_error`: source `resolve_blob_hash` returns `Err` ⇒ `Err`.
- [ ] **Step 2: Run, verify fail.** `cd elohim && CARGO_TARGET_DIR=/tmp/ssr-sprint-target RUSTFLAGS="" cargo test -p elohim-render bootstrap 2>&1 | tail -20` — expect compile error / FAIL (module not defined).
- [ ] **Step 3: Implement** `BundleSource` + `materialize_bundle` (zip extraction via `zip::ZipArchive`, write each entry under `target_dir`, `fs::create_dir_all` for parents). Add deps to Cargo.toml. Wire `pub mod bootstrap;` + `pub use bootstrap::{BundleSource, materialize_bundle};` in lib.rs.
- [ ] **Step 4: Run, verify pass.** Same command — expect PASS. Then `RUSTFLAGS="" cargo clippy -p elohim-render -- -D warnings` (same CARGO_TARGET_DIR) clean; `cargo fmt -p elohim-render`.
- [ ] **Step 5: Commit.** `git add elohim/elohim-render/ && git commit -m "feat(elohim-render): bootstrap module — fetch+verify+unzip SSR bundle from substrate"`

---

### Task 2: Doorway consumes the bundle at boot

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (`init_renderer`, ~`353-410`)
- Create: a `DoorwayBundleSource` (impl `BundleSource`) — co-locate in `doorway/doorway-service/src/ssr.rs` or a small new module.
- Test: unit test in the same module.

**Interfaces:**
- Consumes: `elohim_render::{BundleSource, materialize_bundle}` (Task 1).
- `DoorwayBundleSource` over `STORAGE_URL`: `resolve_blob_hash` = blocking `reqwest` `GET {STORAGE_URL}/db/content/{slug}` → parse `.blobHash`; `fetch_blob` = `GET {STORAGE_URL}/blob/{hash}` → bytes.

- [ ] **Step 1: Write failing test** — `DoorwayBundleSource` parses `blobHash` from a JSON body `{"blobHash":"sha256-abc"}` (factor the parse into a pure fn `parse_blob_hash(&str) -> Result<String>` and test that). Assert error on missing field.
- [ ] **Step 2: Verify fail.** `cd doorway/doorway-service && CARGO_TARGET_DIR=/tmp/ssr-sprint-target RUSTFLAGS="" cargo test parse_blob_hash 2>&1 | tail -20`.
- [ ] **Step 3: Implement.** In `init_renderer`: after reading `SSR_BUNDLE_PATH`, if `SSR_BUNDLE_SLUG` is set, call `materialize_bundle(&DoorwayBundleSource::new(storage_url), &slug, parent_dir_of(bundle_path))`; on `Err`, `warn!` and return `None` (preserve graceful fall-through). Keep the existing `AngularRenderer::with_soft_budget` path. Lazy-on-miss is out of v1 scope here (boot-only) — note it as a follow-up `// TODO(phase-2)`.
- [ ] **Step 4: Verify pass + build.** `RUSTFLAGS="" cargo test parse_blob_hash` PASS; `RUSTFLAGS="" cargo build -p doorway` (same CARGO_TARGET_DIR) clean; `cargo clippy -p doorway -- -D warnings`; `cargo fmt`.
- [ ] **Step 5: Commit.** `git add doorway/doorway-service/ && git commit -m "feat(doorway): materialize SSR bundle from substrate at boot (SSR_BUNDLE_SLUG)"`

---

### Task 3: Storage consumes the bundle — `--features ssr` (RISK-SURFACING)

> **This task may reveal a hard build incompatibility:** elohim-storage builds with `--cfg getrandom_backend="custom"` (Holochain); `elohim-render` is native V8 (deno_core). The sed-strip may be load-bearing. **Attempt the build; if it fails, STOP and report the exact error — do NOT force it.** Doorway-only (Tasks 1,2,4,5,6-doorway,7-doorway) already fixes the live bug; storage SSR becomes a documented follow-on if incompatible.

**Files:**
- Modify: `elohim/elohim-storage/src/ssr.rs` (`SsrState::from_env`, ~`48-65`)
- Test: unit test for the local `BundleSource` resolve.

**Interfaces:**
- Consumes: `elohim_render::{BundleSource, materialize_bundle}`.
- Local `BundleSource`: `resolve_blob_hash` via the content service / db; `fetch_blob` via the local blob store. (Subagent: find the in-process content + blob accessors; if none are ergonomic, a localhost HTTP `GET` to `:8090` is an acceptable v1.)

- [ ] **Step 1: Probe the build FIRST** (before code): `cd elohim/elohim-storage && CARGO_TARGET_DIR=/tmp/ssr-sprint-target-storage cargo build -p elohim-storage --features ssr 2>&1 | tail -40`. (Try first with the storage default RUSTFLAGS, then with `RUSTFLAGS=""`.) Record which (if either) compiles `elohim-render` in. If BOTH fail with a V8/getrandom/deno conflict → **report and stop this task.**
- [ ] **Step 2 (only if build viable): Write failing test** for the local `BundleSource` resolve path.
- [ ] **Step 3: Implement** the materialize-first branch in `SsrState::from_env` (gate on `SSR_BUNDLE_SLUG`; `warn!`+`None` on failure).
- [ ] **Step 4: Verify** `--features ssr` test + clippy + fmt pass.
- [ ] **Step 5: Commit** `git add elohim/elohim-storage/ && git commit -m "feat(storage): materialize SSR bundle at boot under --features ssr"` — OR commit a short findings note to the plan if blocked.

---

### Task 4: Generalize `stage-spa-blob.sh` to publish the server bundle

**Files:**
- Modify: `scripts/ci/stage-spa-blob.sh` (add a `KIND` arg)
- Modify: root `Jenkinsfile` (`stageSpaBlobs` — add the server `{distDir, slug, kind}` entry, ~`371-381`)

- [ ] **Step 1:** Add a 4th positional arg `KIND` (`browser`|`server`, default `browser`). Guard the `index.csr.html → index.html` materialization block behind `[ "$KIND" = browser ]` (server bundles have no index). Everything else (zip → PUT `/admin/seed/blob` → PATCH `/db/content/$SLUG` → verify) is shared.
- [ ] **Step 2: Verify in isolation.** `shellcheck scripts/ci/stage-spa-blob.sh` clean. Dry-logic check: run with a temp dir containing a fake `main.server.mjs` + `KIND=server` against a non-existent URL and confirm it reaches the curl step (zips, computes `sha256-…`) before failing on the network — i.e., the server branch skips index materialization.
- [ ] **Step 3:** In `Jenkinsfile` `stageSpaBlobs`, add `[distDir: "${env.WORKSPACE}/app/elohim-app/dist/elohim-app/server", slug: "elohim-host-landing-ssr", kind: "server"]` to the bundles list and thread `kind` into the `bash stage-spa-blob.sh` call as the 4th arg. Keep it in the SAME invocation as the browser bundle (atomic per build).
- [ ] **Step 4: Commit.** `git add scripts/ci/stage-spa-blob.sh Jenkinsfile && git commit -m "feat(ci): publish SSR server bundle as substrate content (elohim-host-landing-ssr)"`

---

### Task 5: Seed the `elohim-host-landing-ssr` content row

**Files:**
- Modify: the seed source that defines `elohim-host-landing` (subagent: `grep -rl 'elohim-host-landing' genesis/ app/ | grep -iE 'seed|content'` to locate; likely `genesis/data/` or `genesis/seeder/`).

- [ ] **Step 1:** Add a sibling content row `elohim-host-landing-ssr` mirroring `elohim-host-landing` (same shape, `blobHash` intentionally omitted/empty — the deploy PATCH fills it; do not seed a stale blobHash). It exists so the deploy-time `PATCH /db/content/elohim-host-landing-ssr` has a row to update (PATCH 404s otherwise).
- [ ] **Step 2: Verify** with the repo's seed validation (`grep` the new slug; run `pnpm run schema:validate` or the seed validator if one applies to this file).
- [ ] **Step 3: Commit.** `git add <seed file> && git commit -m "feat(seed): add elohim-host-landing-ssr content row for SSR bundle pointer"`

---

### Task 6: Dockerfiles — delete the bake, ship the storage variant

**Files:**
- Modify: `doorway/doorway-service/Dockerfile` (delete `ssr-bundle` stage ~`131-225` + `COPY --from=ssr-bundle … /opt/elohim-render` ~`250`)
- Modify: `elohim/elohim-storage/Dockerfile` (remove the `elohim-render`/`ssr` sed-strip ~`113-119`; add a build path that produces an `--features ssr` variant — **only if Task 3 proved the build viable**)

- [ ] **Step 1 (doorway):** Delete the `ssr-bundle` stage and its `COPY --from`. The runtime now materializes into `/opt/elohim-render` at boot — ensure that dir is writable (created by the materialize step / emptyDir in Task 7).
- [ ] **Step 2 (doorway) verify:** `hadolint doorway/doorway-service/Dockerfile` (warnings ok); confirm no remaining `ssr-bundle`/`node:22-alpine` references via `grep -n 'ssr-bundle\|node:22' doorway/doorway-service/Dockerfile`.
- [ ] **Step 3 (storage):** Only if Task 3 viable — replace the sed with a clean `--features ssr` build arg (default lean, ssr variant for app-serving nodes). If Task 3 was blocked, leave the storage Dockerfile unchanged and note it.
- [ ] **Step 4:** A full image build is heavy (~5-10 min, disk-pressured) and edge/deploy is operator-owned (sprint/feat is not orchestrator-indexed) — do NOT run the full image build in-sprint. Verify by inspection + grep. Note in the commit that image-build verification is deferred to CI/operator.
- [ ] **Step 5: Commit.** `git add doorway/doorway-service/Dockerfile elohim/elohim-storage/Dockerfile && git commit -m "chore(docker): retire SSR bake stage (doorway) + sed-strip (storage); bundle is substrate content"`

---

### Task 7: Manifests — writable bundle dir + slug

**Files:**
- Modify: `genesis/orchestrator/manifests/doorway/{alpha,alpha-b,prod}.yaml`
- Modify: storage manifests serving the app (`genesis/orchestrator/manifests/humans/*` + `_edgenode-consolidated.template.yaml`) — only if Task 3 viable.

- [ ] **Step 1 (doorway):** For each doorway manifest: add an `emptyDir` volume + `volumeMount` at `/opt/elohim-render` (now runtime-populated, was baked); add env `SSR_BUNDLE_SLUG=elohim-host-landing-ssr` (keep `SSR_BUNDLE_PATH=/opt/elohim-render/main.server.mjs`).
- [ ] **Step 2 (storage):** Only if Task 3 viable — app-serving storage nodes get the ssr image variant + `SSR_BUNDLE_SLUG` + the writable dir; lean nodes leave `SSR_BUNDLE_SLUG` unset.
- [ ] **Step 3: Verify** YAML parses: `python3 -c "import yaml,sys; [list(yaml.safe_load_all(open(f))) for f in sys.argv[1:]]" genesis/orchestrator/manifests/doorway/*.yaml`. Deploy verification is operator-owned (note it).
- [ ] **Step 4: Commit.** `git add genesis/orchestrator/manifests/ && git commit -m "chore(manifests): SSR_BUNDLE_SLUG + writable bundle emptyDir for runtime materialization"`

---

## Execution notes

- **Dependency order:** T1 → (T2, T3). T4, T5 independent. T6, T7 after T2/T3 (and gated on T3's viability for the storage halves).
- **The cure ships at Task 2+4+5+6(doorway)+7(doorway)** even if Task 3 (storage SSR) proves incompatible — that path alone fixes the `elohim.host` staleness.
- **No deploy / no push.** Local verification is cargo build/test/clippy + shellcheck + yaml-parse. Image build + edge deploy + live SSR fetch are operator/CI-owned (sprint/feat is not orchestrator-indexed).
- **a2o regression seatbelt** (spec §7) — add as a follow-up once the path is live on alpha; it asserts an app-only change reaches the doorway-projected surface with no doorway image rebuild.
