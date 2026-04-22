# Sweettest DNA-Level Integration Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire sweettest into the noisy-husky enforcement chain (sync-rule nudge + compile-check gate) and fold outstanding Wave 1 sweettest work (DnaBundle fix, Jenkins stage, README note, Cargo.lock) into a coherent commit set, ready for a single verified push.

**Architecture:** Four integration points — sync-rule registry (`file-relationships.json`), graph-walker project (`build-manifest.json`), husky pre-push script, and the sweettest crate itself. On zome-code pushes, husky detects the change via graph-walker, emits a sync-rule nudge, and runs `cargo check -p elohim_sweettest` with Che env vars so compile-time drift (extern signature changes) fails at push, not at Jenkins.

**Tech Stack:** Husky v9, bash pre-push hook, Node-based graph-walker, Cargo (Rust edition 2021), Jenkins declarative pipeline, json-schema-keyed sync registry.

---

## Spec

Reference: `genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md`

Key decisions captured there:
- Coverage model = per-extern happy path + per-`must_*` rejection floor (mechanical), plus per-Wave named focal-point scenarios (narrative).
- Enforcement = noisy husky — sync-rule nudge AND compile-check, both at push time.
- Jenkins = full `cargo test --release` with `--test-threads=1`, `CARGO_BUILD_JOBS=2`, failure-only logging, archived log.

## File map

**New files:**
- None. All integration is into existing files.

**Modified files:**
- `.claude/file-relationships.json` — add `zome-sweettest-sync` entry
- `elohim/holochain/dna/build-manifest.json` — add `sweettest-check` step + `gate.projects` entry
- `.husky/pre-push` — add grep-fallback detection + two switch-block handlers + PROJECT_DIR mapping
- `elohim/holochain/tests/sweettest/README.md` — add Che/contributor env setup section; supersede the "Che compile blocker" section which is now resolved
- `elohim/holochain/tests/sweettest/src/common/conductors.rs` — **already fixed in working tree** (uncommitted): `DnaBundle::read_from_file` → `DnaBundle::unpack`
- `elohim/holochain/dna/Jenkinsfile` — **already edited in working tree** (uncommitted): `DNA Integration (bootstrap-steward)` stage

**Untracked files to include:**
- `elohim/holochain/tests/sweettest/Cargo.lock` — sweettest is a standalone workspace, its lock belongs under version control

---

## Task 1 — Register `zome-sweettest-sync` in file-relationships.json

**Files:**
- Modify: `.claude/file-relationships.json` — append new relationship entry after `humans-presences-sync` (line ~143)

- [ ] **Step 1.1: Read the existing `testid-sync` and `a2o-sync` entries**

Read `.claude/file-relationships.json` and locate the `testid-sync` entry (~line 96) and `a2o-sync` entry (~line 118). These are the structural models — one-to-one sync_rules with `trigger_pattern` + `notify` + `message`.

- [ ] **Step 1.2: Insert `zome-sweettest-sync` entry**

Find the exact closing `},` of `humans-presences-sync` (the entry immediately before `model-sync`). Insert the following block immediately before `model-sync`:

```json
    "zome-sweettest-sync": {
      "description": "Zome source changes should have matching sweettest updates — see genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md",
      "sync_rules": [
        {
          "trigger_pattern": "elohim/holochain/dna/imagodei/zomes/**/*.rs",
          "notify": [],
          "message": "imagodei zome changed. Update tests/sweettest/src/tests/imagodei.rs — per-extern happy-path test, per-must_* rejection test. See R&O parity bar in the design spec."
        },
        {
          "trigger_pattern": "elohim/holochain/dna/mishpat/zomes/**/*.rs",
          "notify": [],
          "message": "mishpat zome changed. Update tests/sweettest/src/tests/mishpat.rs — per-extern happy-path test, per-must_* rejection test."
        },
        {
          "trigger_pattern": "elohim/holochain/dna/lamad/zomes/**/*.rs",
          "notify": [],
          "message": "lamad zome changed. Update tests/sweettest/src/tests/lamad.rs — per-extern happy-path test, per-must_* rejection test."
        },
        {
          "trigger_pattern": "elohim/holochain/dna/node-registry/zomes/**/*.rs",
          "notify": [],
          "message": "node-registry zome changed. Update tests/sweettest/src/tests/node_registry.rs — per-extern happy-path test, per-must_* rejection test."
        },
        {
          "trigger_pattern": "elohim/holochain/dna/infrastructure/zomes/**/*.rs",
          "notify": [],
          "message": "infrastructure zome changed. Update tests/sweettest/src/tests/infrastructure.rs — infrastructure is federation-native (no bootstrap steward), exercise doorway self-registration flow."
        }
      ]
    },
```

- [ ] **Step 1.3: Validate JSON parses**

Run: `node -e "JSON.parse(require('fs').readFileSync('.claude/file-relationships.json', 'utf8'))"`
Expected: exits 0 with no output. If it errors, fix the trailing comma or brace.

- [ ] **Step 1.4: Commit**

```bash
git add .claude/file-relationships.json
git commit -m "$(cat <<'EOF'
chore(sync): add zome-sweettest-sync relationship

Nudges authors toward updating tests/sweettest/src/tests/<dna>.rs
whenever a zome file in dna/<dna>/zomes/**/*.rs changes. Matches the
existing model-sync / a2o-sync / testid-sync pattern.

Ref: genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md §3

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2 — Add `sweettest-check` project to build-manifest.json

**Files:**
- Modify: `elohim/holochain/dna/build-manifest.json` — insert new step under `"steps"` and new entry under `gate.projects`

- [ ] **Step 2.1: Read the current build-manifest.json**

Read `elohim/holochain/dna/build-manifest.json` in full. Note how `manifest-hygiene` is structured as both a `steps` entry and a `gate.projects` entry.

- [ ] **Step 2.2: Add the `sweettest-check` step**

Insert this step after the `manifest-hygiene` step in the `steps` object (match its indentation):

```json
    "sweettest-check": {
      "description": "Sweettest compile-check — catches extern-signature drift before Jenkins-time integration run. See spec §3 (noisy-husky Gate 2).",
      "inputs": {
        "sources": [
          "elohim/holochain/dna/*/zomes/**/src/*.rs",
          "elohim/holochain/dna/*/zomes/**/Cargo.toml",
          "elohim/holochain/tests/sweettest/src/**",
          "elohim/holochain/tests/sweettest/Cargo.toml"
        ],
        "buildProcess": []
      },
      "outputs": {
        "artifacts": [],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Sweettest Compile Check",
        "function": null
      }
    }
```

- [ ] **Step 2.3: Add the `sweettest-check` gate.projects entry**

In `gate.projects`, add this entry after `manifest-hygiene`:

```json
      "sweettest-check": { "dir": ".", "steps": ["sweettest-check"] }
```

Note: `dir: "."` because the husky handler runs `cargo check` with an explicit `--manifest-path`, not by `cd`-ing.

- [ ] **Step 2.4: Validate JSON**

Run: `node -e "JSON.parse(require('fs').readFileSync('elohim/holochain/dna/build-manifest.json', 'utf8'))"`
Expected: exits 0 with no output.

- [ ] **Step 2.5: Verify graph-walker picks up the new project**

Write a probe diff and pipe it to the walker:

```bash
echo "elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs" | node genesis/orchestrator/graph-walker.mjs
```

Expected output contains `sweettest-check` in the emitted project list.

- [ ] **Step 2.6: Commit**

```bash
git add elohim/holochain/dna/build-manifest.json
git commit -m "$(cat <<'EOF'
chore(build-manifest): register sweettest-check project

Graph-walker project that fires when zome sources or sweettest sources
change. Husky dispatches it at push time to run cargo check -p
elohim_sweettest — catches extern-signature drift without waiting for
Jenkins.

Ref: genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md §3 Gate 2

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3 — Wire `sweettest-check` into husky pre-push

**Files:**
- Modify: `.husky/pre-push` — four edits: header doc comment, grep-fallback detection, two switch-block handlers, PROJECT_DIR map

- [ ] **Step 3.1: Add header doc comment**

Find line 21 (the existing `manifest-hygiene` comment). Insert after it:

```bash
#   sweettest-check          → Sweettest compile-check (any zome source or sweettest source change)
```

- [ ] **Step 3.2: Add grep-fallback detection**

Find line ~203 (the existing `manifest-hygiene` grep block). Insert **after** the manifest-hygiene block:

```bash
  if echo "$CHANGED" | grep -qE "^elohim/holochain/dna/[^/]+/zomes/.*\.rs$|^elohim/holochain/tests/sweettest/"; then
    PROJECTS="$PROJECTS sweettest-check"
  fi
```

- [ ] **Step 3.3: Add to the manifest-driven "projects that need cargo/direct invocation" list**

Find line ~253 — the long `if [ "$PROJECT_NAME" = "schema-validate" ] || ...` check. Append `sweettest-check` to that OR chain:

Change:
```bash
  if [ "$PROJECT_NAME" = "schema-validate" ] || [ "$PROJECT_NAME" = "schema-dna" ] || [ "$PROJECT_NAME" = "schema-codegen" ] || [ "$PROJECT_NAME" = "constants-sync" ] || [ "$PROJECT_NAME" = "domain-types" ] || [ "$PROJECT_NAME" = "rakia-codegen" ] || [ "$PROJECT_NAME" = "rakia-validate" ] || [ "$PROJECT_NAME" = "manifest-hygiene" ]; then
```

To:
```bash
  if [ "$PROJECT_NAME" = "schema-validate" ] || [ "$PROJECT_NAME" = "schema-dna" ] || [ "$PROJECT_NAME" = "schema-codegen" ] || [ "$PROJECT_NAME" = "constants-sync" ] || [ "$PROJECT_NAME" = "domain-types" ] || [ "$PROJECT_NAME" = "rakia-codegen" ] || [ "$PROJECT_NAME" = "rakia-validate" ] || [ "$PROJECT_NAME" = "manifest-hygiene" ] || [ "$PROJECT_NAME" = "sweettest-check" ]; then
```

- [ ] **Step 3.4: Add switch-block handler #1 (manifest-driven path)**

Find line ~304 — the `manifest-hygiene)` case. Append **after** it (before the closing `esac`):

```bash
      sweettest-check)
        echo "[$PROJECT_NAME] Running sweettest compile check..."
        # Che devcontainer needs BINDGEN_EXTRA_CLANG_ARGS to point at clang resource dir;
        # Nix/Jenkins provide it automatically. Setting it unconditionally is harmless.
        BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS:--I/usr/lib/clang/20/include}" \
        RUSTFLAGS="" \
          cargo check --manifest-path elohim/holochain/tests/sweettest/Cargo.toml --tests 2>&1
        rc=$?
        ;;
```

- [ ] **Step 3.5: Add switch-block handler #2 (grep-fallback path)**

Find line ~427 — the OTHER `manifest-hygiene)` case in the grep-fallback switch. Append **after** it (before its closing `esac`):

```bash
      sweettest-check)
        echo "[$PROJECT_NAME] Running sweettest compile check..."
        BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS:--I/usr/lib/clang/20/include}" \
        RUSTFLAGS="" \
          cargo check --manifest-path elohim/holochain/tests/sweettest/Cargo.toml --tests 2>&1
        rc=$?
        ;;
```

- [ ] **Step 3.6: Add PROJECT_DIR mapping**

Find line ~524 — the `manifest-hygiene) PROJECT_DIR="." ;;` line. Append **after** it:

```bash
      sweettest-check) PROJECT_DIR="." ;;
```

- [ ] **Step 3.7: Verify husky parses**

Run: `bash -n .husky/pre-push`
Expected: exits 0 with no output. If it errors, check for missing `;;` or braces.

- [ ] **Step 3.8: Dry-run the gate**

Touch a zome source to simulate a change, without staging:

```bash
echo "# dry-run trigger" >> elohim/holochain/dna/imagodei/zomes/imagodei/src/bootstrap_steward.rs
git diff --name-only | node genesis/orchestrator/graph-walker.mjs
```

Expected: output includes `sweettest-check`. Revert the probe change:

```bash
git checkout elohim/holochain/dna/imagodei/zomes/imagodei/src/bootstrap_steward.rs
```

- [ ] **Step 3.9: Commit**

```bash
git add .husky/pre-push
git commit -m "$(cat <<'EOF'
chore(husky): add sweettest-check pre-push gate

When zome sources or sweettest sources change, run cargo check against
the sweettest workspace before push. Sets BINDGEN_EXTRA_CLANG_ARGS for
Che devcontainer (harmless on Nix/Jenkins). Handles both graph-walker
and grep-fallback dispatch paths.

Ref: genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md §3 Gate 2

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4 — Update sweettest README with devcontainer info

**Files:**
- Modify: `elohim/holochain/tests/sweettest/README.md` — replace the stale "Che compile blocker" section with a "Build environment" section that points at the devcontainer and covers the three surfaces (Che, Nix/Jenkins, bare laptop)

- [ ] **Step 4.1: Read the current README**

Read `elohim/holochain/tests/sweettest/README.md` in full. The "Che compile blocker (2026-04-21)" section (lines 79-117) is now stale — the blockers are resolved in the `ethosengine/che-devworkspaces` devcontainer.

- [ ] **Step 4.2: Replace the stale section**

Replace lines 79-117 (the entire "Che compile blocker" section through the final paragraph) with:

```markdown
## Build environment

Sweettest pulls in `holochain` (native) and builds `libdatachannel` from
source via `datachannel-sys`. That build chain needs `cmake`, `clang-libs`,
`zlib-devel`, and a one-line patch to libdatachannel's CMakeLists injecting
`find_package(ZLIB REQUIRED)` before `find_package(OpenSSL)`. Three surfaces
to be aware of:

### Eclipse Che (recommended for contributors)

The `ethosengine/che-devworkspaces` image is preconfigured. See
https://github.com/ethosengine/che-devworkspaces/blob/main/containers/rust-dev/claude.md
for the container spec. Includes the libdatachannel CMakeLists patch and
sets `BINDGEN_EXTRA_CLANG_ARGS` so bindgen finds its clang resource dir
without a `clang` driver binary installed.

### Jenkins Nix build

The `holochain/dna/*.nix` dev shell provides `cmake`, `clang` (full driver),
`libsodium`, `openssl`, and the holochain toolchain. No workarounds needed —
`nix develop --command cargo test ...` just works. See the Jenkins stage
`DNA Integration (bootstrap-steward)` in `elohim/holochain/dna/Jenkinsfile`.

### Bare laptop (not recommended; contributors should use Che)

If you must build outside Che and outside Nix:

```bash
# RHEL/Fedora
sudo dnf install -y cmake clang-libs zlib-devel

# Debian/Ubuntu
sudo apt-get install -y cmake libclang-dev zlib1g-dev

# Apply the libdatachannel CMakeLists patch in the cargo registry:
CML=$(find ~/.cargo/registry/src -path '*datachannel-sys-0.23.0+0.23.2/libdatachannel/CMakeLists.txt' -print -quit)
grep -q 'find_package(ZLIB REQUIRED)' "$CML" || \
  sed -i 's|^\tfind_package(OpenSSL REQUIRED)$|\tfind_package(ZLIB REQUIRED)\n\tfind_package(OpenSSL REQUIRED)|' "$CML"

# Set bindgen's clang resource path (Linux location may vary):
export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/20/include"
```

The devcontainer applies the patch and env var automatically. Contributors
on bare systems maintain the patch themselves.

## Husky compile-check gate

Any push that touches `elohim/holochain/dna/*/zomes/**/*.rs` or
`elohim/holochain/tests/sweettest/**` triggers a push-time
`cargo check -p elohim_sweettest`. This catches extern-signature drift
before Jenkins runs. See `.husky/pre-push` and
`elohim/holochain/dna/build-manifest.json` for the wiring.

## Jenkins stage

The Jenkins stage `DNA Integration (bootstrap-steward)` runs
`cargo test --release -- --test-threads=1 --include-ignored` after the
`Build DNA` stage (packed `.dna` artifacts are a prerequisite). Output
is filtered to summary-on-pass, full panic context on fail. See
`elohim/holochain/dna/Jenkinsfile`.
```

- [ ] **Step 4.3: Verify README renders**

Run: `head -c 1 elohim/holochain/tests/sweettest/README.md`
Expected: `#` (confirms file still starts as a markdown heading; didn't accidentally prepend junk).

Optionally: `markdownlint elohim/holochain/tests/sweettest/README.md` if available.

- [ ] **Step 4.4: Commit**

```bash
git add elohim/holochain/tests/sweettest/README.md
git commit -m "$(cat <<'EOF'
docs(sweettest): document Che devcontainer + husky + Jenkins wiring

Supersedes the stale "Che compile blocker" section. Points at the
ethosengine/che-devworkspaces container spec, documents the Nix/Jenkins
path (no workarounds), and covers the bare-laptop fallback for
contributors not using Che. Adds forward references to the husky
compile-check gate and the Jenkins integration stage.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5 — Fold outstanding working-tree changes into coherent commits

This task commits work that was done in-session but not yet committed: the `DnaBundle` API fix, the Jenkinsfile stage, and the generated `Cargo.lock`.

**Files:**
- Commit: `elohim/holochain/tests/sweettest/src/common/conductors.rs` (modified)
- Commit: `elohim/holochain/dna/Jenkinsfile` (modified — new stage added)
- Commit: `elohim/holochain/tests/sweettest/Cargo.lock` (untracked — standalone workspace lockfile)

- [ ] **Step 5.1: Verify working-tree state**

Run: `git status --short elohim/holochain/`
Expected output includes:
```
 M elohim/holochain/dna/Jenkinsfile
 M elohim/holochain/tests/sweettest/src/common/conductors.rs
?? elohim/holochain/tests/sweettest/Cargo.lock
```

If any of those are missing, the changes from earlier in the session have been lost — stop and investigate before proceeding.

- [ ] **Step 5.2: Commit the DnaBundle fix**

```bash
git add elohim/holochain/tests/sweettest/src/common/conductors.rs
git commit -m "$(cat <<'EOF'
fix(sweettest): replace DnaBundle::read_from_file with unpack

DnaBundle::read_from_file doesn't exist in holochain 0.6.0. Correct API
is DnaBundle::unpack(bytes) — read bytes explicitly, pass as a slice.
Caught during the sweettest-in-Che spike (compile failure, pre-Jenkins).
Reference for the compile-check gate this is now guarded by:
.husky/pre-push / sweettest-check project.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5.3: Commit the Jenkinsfile DNA Integration stage**

```bash
git add elohim/holochain/dna/Jenkinsfile
git commit -m "$(cat <<'EOF'
ci(holochain): add DNA Integration (bootstrap-steward) stage

Runs sweettest against packed DNAs after Build DNA. Resource-guarded:
CARGO_BUILD_JOBS=2 (caps link-phase RAM spike), --test-threads=1
(serializes conductor spin-up), --release (~30% less runtime RAM),
30-minute stage timeout. Output is summary-only on pass, full panic
context on fail, log archived unconditionally. Post-step cleans leaked
sweet_conductor_* temp dirs.

Ref: genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md §4

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 5.4: Commit the Cargo.lock**

```bash
git add elohim/holochain/tests/sweettest/Cargo.lock
git commit -m "$(cat <<'EOF'
chore(sweettest): commit Cargo.lock for standalone workspace

Sweettest is a standalone workspace (not a member of any DNA workspace),
so its Cargo.lock is authoritative for its own dep resolution. Committing
it pins the holochain 0.6.0 deploy chain and makes builds reproducible.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6 — End-to-end verification

Exercise every gate the plan added, before pushing. If any step fails, stop and triage — do not push.

**Files:**
- None modified; read-only verification.

- [ ] **Step 6.1: Confirm `cargo check` against sweettest works from a clean-ish state**

Run:
```bash
cd elohim/holochain/tests/sweettest
BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/20/include" RUSTFLAGS="" cargo check --tests 2>&1 | tail -20
```
Expected: final line includes `Finished` (may have warnings; no errors).

- [ ] **Step 6.2: Dry-run graph-walker on a simulated zome change**

From repo root:
```bash
echo "elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs" | node genesis/orchestrator/graph-walker.mjs
```
Expected output contains both `sweettest-check` AND `build-dna-wasm` (the existing DNA build trigger).

- [ ] **Step 6.3: Dry-run graph-walker on a sweettest-only change**

```bash
echo "elohim/holochain/tests/sweettest/src/tests/imagodei.rs" | node genesis/orchestrator/graph-walker.mjs
```
Expected output contains `sweettest-check` and NOT `build-dna-wasm`.

- [ ] **Step 6.4: Dry-run husky pre-push end-to-end (without actually pushing)**

```bash
# Simulate the pre-push environment. This runs the same script husky would.
CHANGED=$(git diff --name-only origin/dev HEAD)
echo "$CHANGED" | grep -qE "^elohim/holochain/dna/[^/]+/zomes/.*\.rs$|^elohim/holochain/tests/sweettest/" && echo "sweettest-check would fire"
```

Since this plan's commits include the Cargo.lock and conductors.rs, the expected output is `sweettest-check would fire`.

- [ ] **Step 6.5: Verify sync-rule emits the expected nudge**

If the repo has a sync-rule evaluator (check `.claude/scripts/` or `.claude/hooks/`), run it against a simulated zome change. Otherwise: read `.claude/file-relationships.json` and confirm the `zome-sweettest-sync.sync_rules[0].trigger_pattern` matches a real zome path (e.g., `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` against `elohim/holochain/dna/imagodei/zomes/**/*.rs` = match).

- [ ] **Step 6.6: Final `git status` — expect clean tree**

Run: `git status --short`
Expected: empty output (all changes committed).

- [ ] **Step 6.7: Final `git log` — confirm commit order**

Run: `git log --oneline origin/dev..HEAD`
Expected output (order top-to-bottom, newest first):
```
<sha> chore(sweettest): commit Cargo.lock for standalone workspace
<sha> ci(holochain): add DNA Integration (bootstrap-steward) stage
<sha> fix(sweettest): replace DnaBundle::read_from_file with unpack
<sha> docs(sweettest): document Che devcontainer + husky + Jenkins wiring
<sha> chore(husky): add sweettest-check pre-push gate
<sha> chore(build-manifest): register sweettest-check project
<sha> chore(sync): add zome-sweettest-sync relationship
<sha> docs(sweettest): spec for DNA-level integration test layer
```

Eight commits. The spec commit is already on HEAD before this plan starts.

- [ ] **Step 6.8: Push with husky engaged**

```bash
git push origin dev
```

Expected: husky runs, triggers `sweettest-check` (because the push includes sweettest source changes), the `cargo check` succeeds (because we just verified in Step 6.1), push completes.

If `sweettest-check` takes too long (>10 min warm), consider whether the target/ cache is populated. If husky fails, DO NOT use `HUSKY=0` — fix the underlying issue and re-push.

---

## Self-review checklist

**Spec coverage:**
- §1 (test-layer positioning) — informational; no task needed.
- §2 (coverage model — hybrid) — §3 enforcement task exists (Task 1 sync rule drives authoring; Task 3 compile-check drives discovery). Focal-point authoring is per-Wave, out of plan scope.
- §3 (enforcement — sync rule + compile check) — Task 1 (sync rule), Task 2 (graph-walker project), Task 3 (husky wiring). Covered.
- §4 (Jenkins stage) — Task 5.3 commits the already-written stage. Covered.
- §5 (Wave-by-Wave rollout) — per-Wave work, not in this plan.
- §6 (resource envelope) — Jenkins resource settings in Task 5.3 commit; Che documented in Task 4. Covered.
- §7 (failure modes protected) — informational; no new tasks needed.

**No placeholders:** scanned; every code/sed step has concrete content.

**Type consistency:** project name `sweettest-check` is used identically in build-manifest.json (Task 2), husky (Task 3), graph-walker probes (Task 6). PROJECT_DIR and command invocation are consistent across both switch blocks in Task 3.

**Out-of-scope surfaces:** per-Wave test authoring (not this plan — this plan makes it easy; authoring is the next Wave's work). Coverage reporting/dashboard (spec explicitly out of scope).
