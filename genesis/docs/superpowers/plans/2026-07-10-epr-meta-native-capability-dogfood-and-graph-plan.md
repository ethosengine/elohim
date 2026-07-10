---
id: "plan-epr-meta-native-capability-dogfood-and-graph"
status: "active"
cites:
  - epr-meta-native-capability-dogfood-and-graph | Dogfood .epr-meta Native Governance + Certify Claude→Elohim-Native Translation (+ eprfs Package Graph) | sha256:33681d28fbbdf425 | path: genesis/docs/superpowers/specs/2026-07-10-epr-meta-native-capability-dogfood-and-graph-design.md
  - epr-meta-eprfs-elohim-native-sotu-2026-07-09 | EPR Meta / EPRFS / Elohim-Native Capability SOTU | sha256:b4c6c115da8d0e24 | path: genesis/docs/analysis/2026-07-09-epr-meta-eprfs-elohim-native-sotu.md
---

# EPR-Meta Native Capability Dogfood + Graph — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the already-green elohim-native package layout into a committed, governed, drift-gated, fidelity-certified system, and add a Rust eprfs adapter that proves the package tree is a portable content-addressed graph.

**Architecture:** Three pillars over the existing `.epr-meta/elohim/packages` tree. (A) Govern capabilities at source via a co-located `.epr-meta`, project an `EprRef` governance backref, verify it, and lodge non-compliance to a findings ledger. (B) A standing round-trip fidelity gate + Python↔Rust resolver parity fixtures + a written directionality contract. (C) A thin Rust domain adapter (`elohim-agent-adapter`) that maps the package tree to an `eprfs-core::ProjectionManifest` and round-trips it through `eprfs-local` byte-identically.

**Tech Stack:** Node ESM (`package-projections.mjs`), Python 3 (`.claude/scripts/_lib/epr_meta.py`), Rust (`eprfs-core`, `eprfs-local`, `eprfs-storage::MemoryStorage`, new `elohim-agent-adapter` crate), husky bash.

## Global Constraints

- **Commit path-scoped to the epr-meta concern only.** This is a shared worktree with ambient unrelated changes — never `git add -A`; add explicit paths. Integrator pushes; do NOT push or merge.
- **`.claude` runtime files stay byte-identical to their human source** (B1 fidelity). The governance backref therefore lives in the package JSON + codex projection + projection fixtures, never mutated into the `.claude` SKILL.md/agent .md. Claude-actor awareness is delivered by the live `epr-meta-resolver.py` compose-gate at edit time.
- **Rust native builds:** set `CARGO_TARGET_DIR` to the pool slot for this worktree and `RUSTFLAGS=""` (non-WASM). Pool slot for `crates`/new crates: `/projects/.cargo-target-pool/family/frontend/crates/dev`. Prefer `cargo` (no nextest in this container).
- **Governance enforcement class is nudge/observe, never block/clobber** (`.claude` is still authored source this sprint).
- **CID has one source of truth:** `eprfs_core::BlobCid::compute` (`CIDv1(dag-cbor, sha2-256)`). Never re-implement CID in JS/Python.
- Package CLI constants (in `elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs`): `REPO_ROOT`, `SKILL_SOURCE_DIR` (`.claude/skills`), `AGENT_SOURCE_DIR` (`.claude/agents`), `PACKAGE_DIR` (`.epr-meta/elohim/packages`), `PROJECTION_DIR` (`.epr-meta/elohim/projections`).

---

### Task 1: Govern the capabilities at source (A2a)

**Files:**
- Create: `.epr-meta/elohim/packages/.epr-meta`
- Test: `.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` (add one assertion)

**Interfaces:**
- Consumes: the live `.claude/scripts/_lib/epr_meta.py` resolver (directory-form + legacy cascade, root-first / nearest-wins).
- Produces: a resolvable `capability-governance` rule bound to the package tree, visible to the cascade.

- [ ] **Step 1:** Read `.claude/epr-meta/policies.yaml` and pick the lightest existing observation/measure policy that fits "this is a governed capability; re-import after editing its projection." If one fits, bind it; only add a new `capability-governance@1` observation policy if none does. Read `genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md` for the rule grammar and enforcement-class ladder.
- [ ] **Step 2:** Write `.epr-meta/elohim/packages/.epr-meta` with frontmatter: `epr-meta-version: 1`, `id: capability-package-governance`, a `purpose:` line, and a `rules:` list binding the chosen policy (observation class — never `deny`). Model the shape on the root `.epr-meta/manifest.md` `rs-loc-ceiling` rule.
- [ ] **Step 3:** Add a Python assertion in `epr_meta_cascade_test.py` proving the cascade collects the `capability-package-governance` rule when resolving a path under `.epr-meta/elohim/packages/skills/`. Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` — Expected: FAIL first (rule absent), then PASS after Step 2.
- [ ] **Step 4:** Confirm the compose-gate does not now DENY normal package edits: `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` still exits 0.
- [ ] **Step 5:** Commit. `git add .epr-meta/elohim/packages/.epr-meta .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py .claude/epr-meta/policies.yaml && git commit -m "feat(epr-meta): govern elohim capability packages at source (A2a)"`

---

### Task 2: Round-trip fidelity gate (B1)

**Files:**
- Modify: `elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs` (add a check in the `verify` path, near the existing `runVerify`/assertion block ~lines 450-610)

**Interfaces:**
- Consumes: `loadSourcePackages()` (returns packages built from `.claude` sources), `projectClaude(pkg)`, `readFile`.
- Produces: a `verify` assertion `fidelity: project(import(source)) === source` per Claude source.

- [ ] **Step 1: Write the failing check.** In the verify path, for every package returned by `loadSourcePackages()` whose `metadata.sourceRuntime === 'claude'`, read the raw on-disk source at `pkg.projections.claude.path` and assert byte-equality with `projectClaude(pkg)`:

```js
// Round-trip fidelity floor: project(import(source)) === source, byte-for-byte.
for (const pkg of sourcePackages) {
  if (pkg.metadata.sourceRuntime !== 'claude') continue;
  const sourcePath = resolve(REPO_ROOT, pkg.projections.claude.path);
  const original = await readFile(sourcePath, 'utf8');
  assert(
    projectClaude(pkg) === original,
    `fidelity: project(import(${pkg.kind}:${pkg.metadata.id})) === source`,
  );
}
```

- [ ] **Step 2: Prove it detects a break.** Temporarily append a space to one `.claude/agents/*.md` body region NOT captured by the package (or momentarily mutate `projectClaude` output), run verify, confirm the new assertion FAILS. Revert the mutation.
- [ ] **Step 3: Run to green.** `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` — Expected: PASS, count risen by (#claude skills + #claude agents).
- [ ] **Step 4:** `pnpm run elohim-agent:packages:test` — Expected: PASS (may need the pnpm store approval path used previously).
- [ ] **Step 5:** Commit. `git add elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs && git commit -m "feat(elohim-agent): standing round-trip fidelity gate (B1)"`

---

### Task 3: Governance backref — EprRef relationship snapshot (A2b + A2c)

**Files:**
- Modify: `elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs`
  - `skillPackageFromClaude` / `agentPackageFromClaude`: add a `governance` block to `metadata`.
  - `codexFrontmatter`: emit a `governance` field.
  - verify path: assert package `governance` present + codex projection backref matches package.

**Interfaces:**
- Produces: `metadata.governance = { eprRef, policy, gates, ledger }` on every package; a `metadata.governance` line in every codex projection frontmatter; a verify assertion `governance backref matches package`.
- Consumes: the package `metadata.id`, `metadata.kind`.

- [ ] **Step 1:** Define a pure helper `governanceFor(kind, id)` returning the relationship snapshot. `eprRef` is a deterministic, offline-valid slug anchored on the package id (NOT a live storage lookup this sprint):

```js
function governanceFor(kind, id) {
  return {
    eprRef: `epr:elohim-agent/${kind}/${id}`,     // offline floor anchor; resolves to earned trust when the substrate is reachable
    policy: 'capability-governance@1',
    gates: ['epr-meta-resolver', 'elohim-agent:packages:verify'],
    ledger: '.claude/data/governance-findings.jsonl',
  };
}
```

- [ ] **Step 2:** In `skillPackageFromClaude` and `agentPackageFromClaude`, set `metadata.governance = governanceFor(kind, name)` (kind = `'skills'`/`'agents'`). In `codexFrontmatter`, add `frontmatter.governance = governance.eprRef` when passed a `governance` arg; thread `governance` through both call sites. Do NOT touch the claude projection (`frontmatterRaw` passthrough → byte-identity preserved).
- [ ] **Step 3:** In the verify path, assert every package has `metadata.governance.eprRef` and that the on-disk codex projection (fixture) contains that `eprRef`. Run verify — Expected: FAIL until fixtures regenerated.
- [ ] **Step 4:** Regenerate: `node …/package-projections.mjs project --write-fixtures` then `… project --write-runtime` (codex may need the escalated write path per SOTU). Re-run verify — Expected: PASS. Confirm `git diff --stat .claude/skills .claude/agents` shows **no** changes (byte-identity held); only `.codex/**` and `.epr-meta/elohim/**` moved.
- [ ] **Step 5:** Commit. `git add elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs .epr-meta/elohim/packages .epr-meta/elohim/projections .codex && git commit -m "feat(elohim-agent): EprRef governance backref on packages + codex projections (A2b/A2c)"`

---

### Task 4: Lodging ledger scaffold (A2d)

**Files:**
- Modify: `elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs` (on verify-red for a governance/backref/fidelity assertion, append a finding)
- Create (at runtime, git-ignored or committed empty): `.claude/data/governance-findings.jsonl`, `.claude/data/governance-cursor.json`

**Interfaces:**
- Consumes: the existing findings-ledger contract — study `.claude/data/ci-findings.jsonl` + `.claude/scripts/_lib/ci_trigger.py` for the fingerprint+cursor shape (deterministic fingerprint so the same drift does not re-fire).
- Produces: a `lodgeGovernanceFinding({fingerprint, kind, id, detail})` helper that appends one JSONL line, dedup-guarded by fingerprint against the cursor.

- [ ] **Step 1:** Read `.claude/scripts/_lib/ci_trigger.py` and one existing `*-findings.jsonl` to copy the record shape (fingerprint, first_seen, detail, status).
- [ ] **Step 2:** Add `lodgeGovernanceFinding(...)` that computes a stable fingerprint (e.g. `sha256(kind:id:assertionClass)` truncated) and appends to `.claude/data/governance-findings.jsonl` only if the fingerprint is not already open in the ledger. Wire it into the verify failure path for `governance backref` / `fidelity` assertions (lodge, then still exit 1).
- [ ] **Step 3:** Test: temporarily forge a stale backref in one codex fixture, run verify, confirm (a) verify exits 1 AND (b) exactly one new line appears in `governance-findings.jsonl`; run verify again, confirm NO duplicate line (dedup). Revert the forgery; re-run verify green.
- [ ] **Step 4:** Commit. `git add elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs .claude/data/governance-findings.jsonl .claude/data/governance-cursor.json && git commit -m "feat(elohim-agent): governance non-compliance lodging ledger scaffold (A2d)"`

---

### Task 5: Wire verify into pre-push (A3)

**Files:**
- Modify: `.husky/pre-push.bash`

**Interfaces:**
- Consumes: `node …/package-projections.mjs verify` (pure node, PVC-neutral, exits 1 on drift).

- [ ] **Step 1:** Read `.husky/pre-push.bash`; find where generated-artifact freshness gates run (e.g. the `.ci-ignore`/schema-codegen freshness checks). Add a step that runs the packages verify when any of `.epr-meta/elohim/**`, `.claude/skills/**`, `.claude/agents/**`, `.codex/**` changed in the push range. Reuse the existing change-detection idiom in the file — do not invent a new one.
- [ ] **Step 2:** Emit a clear failure banner naming the fix command (`pnpm run elohim-agent:packages:project` / `:import`) on non-zero exit. Keep it heredoc-free if the pattern in the file requires (bash bodies may live in `scripts/ci/*.sh` — follow the file's convention).
- [ ] **Step 3:** Dry-run: with a deliberately stale codex fixture, run the relevant hook function locally and confirm it fails; restore fixture and confirm it passes.
- [ ] **Step 4:** Commit. `git add .husky/pre-push.bash && git commit -m "feat(pre-push): gate elohim capability projection drift (A3)"`

---

### Task 6: Python↔Rust `.epr-meta` resolver parity fixtures (B2)

**Files:**
- Create: `.claude/scripts/_lib/__tests__/fixtures/epr_meta_parity/` (shared fixture manifests: `root-directory-form/`, `legacy-nested/`, `cascade-conflict/`)
- Modify: `.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` (assert resolution over the shared fixtures)
- Create: `elohim/eprfs/eprfs-meta/tests/parity.rs` (assert the SAME fixtures resolve identically)

**Interfaces:**
- Consumes: `epr_meta.py` resolver (Python), `eprfs-meta` resolver (Rust) — both must agree on rule ids + nearest-wins order for each fixture.
- Produces: a shared fixture corpus + a Python test + a Rust test asserting identical resolved rule-id sequences.

- [ ] **Step 1:** Author 3 minimal fixture trees under the fixtures dir: directory-form root manifest; legacy nested `.epr-meta` file; a root+nested id-collision (nearest-wins) case. Keep each tiny.
- [ ] **Step 2:** Python test: for each fixture, resolve and assert the ordered resolved rule-ids equal a hard-coded expected list. Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` — Expected: PASS.
- [ ] **Step 3:** Rust test in `eprfs-meta/tests/parity.rs`: read `eprfs-meta`'s public resolver API (read `elohim/eprfs/eprfs-meta/src/lib.rs`), resolve the same fixture paths, assert the SAME ordered rule-ids. Run: `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/crates/dev RUSTFLAGS="" cargo test -p eprfs-meta parity` — Expected: PASS.
- [ ] **Step 4:** Update the standing hazard `genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md` — note the parity suite now exists and what it covers.
- [ ] **Step 5:** Commit. `git add .claude/scripts/_lib/__tests__ elohim/eprfs/eprfs-meta/tests genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md && git commit -m "test(epr-meta): shared Python/Rust resolver parity fixtures (B2)"`

---

### Task 7: Directionality contract (B3)

**Files:**
- Modify: `.claude/skills/elohim-package-authoring/SKILL.md` (or its package source), and `elohim/sdk/domains/elohim-agent/CLAUDE.md`

**Interfaces:** documentation only — no code.

- [ ] **Step 1:** Add a "Directionality contract" section stating: import (Claude→package) is certified byte-lossless (B1 gate); `.claude`/`.codex` are authored source today; packages are a certified mirror + governance home; the package-master flip (generated-and-clobbered runtime) is deferred behind trusted B1 + A2; the governance `EprRef` is the offline floor anchor whose ceiling (deep trust + REA value-flow reconciliation) resolves through the p2p substrate.
- [ ] **Step 2:** If editing `.claude/skills/elohim-package-authoring/SKILL.md` directly (it is `sourceRuntime: elohim-agent`, package-owned), re-import/re-project after: `node …/package-projections.mjs project --write-fixtures`. Confirm verify green.
- [ ] **Step 3:** Commit. `git add .claude/skills/elohim-package-authoring elohim/sdk/domains/elohim-agent/CLAUDE.md .epr-meta/elohim && git commit -m "docs(elohim-agent): directionality contract for Claude→native translation (B3)"`

---

### Task 8: eprfs package graph adapter — build the manifest (C1)

**Files:**
- Create: `elohim/sdk/domains/elohim-agent/adapter/Cargo.toml`
- Create: `elohim/sdk/domains/elohim-agent/adapter/src/lib.rs`
- Modify: root `Cargo.toml` workspace `members` (add the new crate path)

**Interfaces:**
- Consumes: `eprfs_core::{ProjectionManifest, ProjectionEntry, ProjectionRoot, ProjectionId, ProjectionPath, ProjectionSource, ProjectionSourceKind, EntryKind, ProjectionStatus, BlobCid, EprRef}`.
- Produces: `pub fn manifest_from_package_tree(root: &Path) -> Result<ProjectionManifest>` — one `ProjectionEntry` per file under the tree; directories as `EntryKind::Directory`; files as `EntryKind::File` with `blob = BlobCid::compute(bytes)`, `source = ProjectionSource::new("elohim-agent", Content, "<Kind>:<name>")`, `status = Local`. Also `pub fn blobs_for_tree(root) -> Vec<(BlobCid, Vec<u8>)>` for the test's storage seeding.

- [ ] **Step 1:** Cargo.toml: `name = "elohim-agent-adapter"`, `edition = "2021"`, deps `eprfs-core = { path = "../../../../eprfs/eprfs-core" }`, `serde_json` (workspace); dev-deps `eprfs-local`, `eprfs-storage`, `bytes`, `tokio` (workspace, `features=["fs","macros","rt"]`). Verify the relative path depth to `elohim/eprfs/eprfs-core` from `elohim/sdk/domains/elohim-agent/adapter/` before committing the `path =`.
- [ ] **Step 2:** Add the crate to root `Cargo.toml` `[workspace] members`. Run `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/crates/dev RUSTFLAGS="" cargo metadata --no-deps -q >/dev/null` to confirm the workspace resolves.
- [ ] **Step 3: Write the failing test** (in `src/lib.rs` `#[cfg(test)]`): build a tiny temp tree with 1 dir + 2 files, call `manifest_from_package_tree`, assert `manifest.validate().is_ok()`, entry count == 3, and a file entry's `blob == BlobCid::compute(file_bytes)`.
- [ ] **Step 4: Implement** `manifest_from_package_tree`: walk the tree (std::fs, deterministic sorted order), map each entry per the interface. `ProjectionPath::new(rel_path)?`; `ProjectionEntry::file(path, BlobCid::compute(&bytes))` then set `source`, `size_bytes`, `status = ProjectionStatus::Local`. Root: `ProjectionRoot { id: ProjectionId::new("elohim-agent-packages"), root: EprRef::new("epr:elohim-agent/packages") }`.
- [ ] **Step 5: Run** `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/crates/dev RUSTFLAGS="" cargo test -p elohim-agent-adapter` — Expected: PASS. Then `cargo fmt` + `cargo clippy -p elohim-agent-adapter -- -D warnings`.
- [ ] **Step 6:** Commit. `git add elohim/sdk/domains/elohim-agent/adapter Cargo.toml Cargo.lock && git commit -m "feat(elohim-agent-adapter): package tree → eprfs ProjectionManifest (C1)"`

---

### Task 9: Graph round-trip — materialize byte-identical (C2)

**Files:**
- Modify: `elohim/sdk/domains/elohim-agent/adapter/src/lib.rs` (add the round-trip test)

**Interfaces:**
- Consumes: `eprfs_local::LocalMaterializer`, `eprfs_storage::MemoryStorage`, `eprfs_core::MaterializationPolicy::LocalOnly`, `bytes::Bytes`.
- Produces: proof that adapter-manifest + `eprfs-local` materialize the real package tree byte-identically at an arbitrary root.

- [ ] **Step 1: Write the failing test** (async, `#[tokio::test]`):

```rust
#[tokio::test]
async fn materializes_package_tree_byte_identical() {
    let src = /* path to a small real subtree, or a fixture built in-test */;
    let manifest = manifest_from_package_tree(&src).unwrap();

    let storage = eprfs_storage::MemoryStorage::default();
    for (cid, bytes) in blobs_for_tree(&src).unwrap() {
        storage.insert_blob(cid, bytes::Bytes::from(bytes)).await;
    }

    let target = std::env::temp_dir().join(format!("eaa-roundtrip-{}", std::process::id()));
    let _ = tokio::fs::remove_dir_all(&target).await;
    let materializer = eprfs_local::LocalMaterializer::new(storage);
    materializer.materialize(&manifest, &target, eprfs_core::MaterializationPolicy::LocalOnly).await.unwrap();

    // every source file exists at target with identical bytes
    for (rel, bytes) in read_all_files(&src) {
        assert_eq!(tokio::fs::read(target.join(&rel)).await.unwrap(), bytes, "mismatch: {}", rel.display());
    }
    let _ = tokio::fs::remove_dir_all(&target).await;
}
```

- [ ] **Step 2:** Implement `blobs_for_tree` + the `read_all_files` test helper (deterministic walk).
- [ ] **Step 3: Run** `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/crates/dev RUSTFLAGS="" cargo test -p elohim-agent-adapter materializes_package_tree_byte_identical` — Expected: PASS (byte-identical proves the portable content-addressed graph).
- [ ] **Step 4:** `cargo fmt --check` + `cargo clippy -p elohim-agent-adapter -- -D warnings`.
- [ ] **Step 5:** Commit. `git add elohim/sdk/domains/elohim-agent/adapter && git commit -m "test(elohim-agent-adapter): byte-identical package-graph materialization (C2)"`

---

### Task 10: Full-suite verify + finalize

**Files:** none (verification + the SOTU/spec status bump)

- [ ] **Step 1:** Run the whole gate suite and capture output:
  - `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify`
  - `pnpm run elohim-agent:packages:test` · `pnpm run elohim-agent:test`
  - `python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py` · `python3 .claude/scripts/_lib/__tests__/ci_trigger_test.py`
  - `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/crates/dev RUSTFLAGS="" cargo test -p eprfs-meta && cargo test -p elohim-agent-adapter`
  - `cargo fmt --check` (touched crates) · `git diff --check`
- [ ] **Step 2:** Bump the spec `status: Draft → Implemented` and add a one-line "delivered" note to `genesis/docs/analysis/2026-07-09-epr-meta-eprfs-elohim-native-sotu.md` closeout items #1 (parity), #3 (gate wired), #5 (adapter). Re-seal cites: `python3 .claude/scripts/memory-kit/cite-gen.py --seal <file>` for any edited managed surface.
- [ ] **Step 3:** Final path-scoped commit of any residual (spec/SOTU status). Confirm `git log --oneline -10` shows the pillar commits and the branch is `feat/frontend-eyes-sprint`. Do NOT push/merge — hand off to the integrator.

---

## Self-Review

**Spec coverage:** A1 (grounding, done) · A2a→Task1 · A2b/A2c→Task3 · A2d→Task4 · A3→Task5 · B1→Task2 · B2→Task6 · B3→Task7 · C1→Task8 · C2→Task9 · commit discipline→Global Constraints + Task10. The ceiling (behavioral judge), substrate value-plane, and cross-repo CLI config are spec'd out-of-scope — no task, intentionally.

**Type consistency:** `governanceFor(kind,id)` shape (Task3) matches the `metadata.governance` verify assertion (Task3) and the ledger `ledger:` path (Task4). `manifest_from_package_tree` / `blobs_for_tree` names are consistent across Tasks 8 and 9. `ProjectionSource::new(namespace, kind, id)` and `ProjectionEntry::file(path, blob)` match `eprfs-core` signatures read from `projection.rs`.

**Placeholder scan:** the one `/* path to a small real subtree … */` in Task 9 Step 1 is a deliberate implementer choice (real subtree vs in-test fixture) with both options named — resolve to an in-test fixture if the real tree is large.
