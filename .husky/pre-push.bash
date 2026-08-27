#!/usr/bin/env bash
# Pre-push hook: Runs project-specific checks before allowing push.
# Bypass ALL gates:        HUSKY=0 git push   (or: git push --no-verify)
# Skip ONLY sweettest:     SKIP_SWEETTEST=1 git push      (on dev pushes)
# Force sweettest:         RUN_SWEETTEST=1 git push       (on non-dev pushes)
# Force heavy gates:       FORCE_HEAVY_GATES=1 git push   (under PVC pressure)
#
# TIERED TESTING (2026-06-04): sweettest-check is an INTEGRATION-layer gate —
# it compiles the full Holochain conductor, which is high-value at the
# integration boundary and a velocity-killer on every feature push. Default:
# it runs only when the push target is dev/main (where the orchestrator also
# builds); on feature/sprint/shift pushes it is deferred with a notice. CI's
# DNA pipeline (--run-ignored all) remains the backstop either way.
# Unit-layer gates (fmt/clippy/unit tests/lint) always run.
#
# PVC-PRESSURE STRATEGY (2026-06-04): before running gates the hook reads
# genesis/agentic/pool-policy.json + df. At the soft watermark it first runs
# `cargo-pool enforce --yes` (guarded reclaim — never touches this push's
# family or any flock'd slot); at the hard ceiling it DEFERS heavy Rust gates
# with a loud DEFERRED-BY-PVC banner naming each one, instead of cold-building
# tens of GB mid-push and starving the volume. FORCE_HEAVY_GATES=1 overrides.
#
# Honor HUSKY=0 — git config core.hooksPath points at .husky/ directly,
# so the husky shim (which normally handles this) is not in the call path.
[ "${HUSKY-}" = "0" ] && { echo "pre-push: HUSKY=0 — skipping all gates"; exit 0; }

# Treat pipe failures as the pipeline's exit code. -e is intentionally NOT
# set because many existing gate paths use `command || handle_failure`
# patterns; -u is unsafe given the optional toolchain vars (NVM_DIR etc.).
set -o pipefail

# Detects projects and executes typed gate recipes from build-manifest.json.
# The shared runner supplies cargo-pool/RUSTFLAGS context for native workspaces.
# Representative projects:
#   app/elohim-app/          → eslint + build + unit tests
#   doorway/                 → cargo fmt + clippy + tests
#   doorway/doorway-app/     → eslint + build
#   sophia/              → pnpm lint + typecheck + test
#   elohim/elohim-storage/   → cargo fmt + clippy + tests
#   elohim/elohim-compute/   → cargo fmt + clippy + tests
#   steward/node/            → cargo fmt + clippy + tests
#   app/elohim-library/      → tsc type-check + vitest tests
#   elohim-storybook         → Storybook 10 static build (when storybook sources or build-manifest change)
#   genesis/                 → schema validation + tests
#   schema-validate          → protocol schema seed validation (genesis/ or schema changes)
#   schema-dna               → DNA conformance check (holochain/ or schema changes)
#   schema-codegen           → verify codegen is fresh (schema changes)
#   rakia-codegen            → verify rakia generated_types.rs is fresh (rakia schema or rakia-core changes)
#   rakia-validate           → validate build-manifest.json files against rakia schema (any manifest or rakia schema change)
#   cargo-coverage           → verify every declared [[bin/bench/example]] target is covered by some manifest source glob (any Cargo.toml or build-manifest.json change)
#   manifest-hygiene         → DNA manifest hygiene schema contract (any dna.yaml / happ.yaml / manifest-hygiene test change)
#   sweettest-check          → Sweettest compile-check (any zome source or sweettest source change)
#   genesis/a2o/             → E2E lint + typecheck
#   gherkin-prepush-lint     → Parse every executable A2O feature before E2E can abort blank
#   genesis/orchestrator/    → Jenkinsfile linting (any *Jenkinsfile* change)
#
# There is no project-name shell switch or grep fallback: adding a local gate is
# one manifest edit, and both `just gate` and pre-push consume the same registry.

# ── Load toolchains (git hooks run in minimal shell) ────────────
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm use default --silent

# pnpm (standalone install to PNPM_HOME)
PNPM_HOME="${PNPM_HOME:-/nix/xdg/cache/pnpm}"
[ -d "$PNPM_HOME" ] && export PATH="$PNPM_HOME:$PATH"

# Rustup (cargo fmt, clippy, tests for doorway)
[ -d "$HOME/.cargo/bin" ] && export PATH="$HOME/.cargo/bin:$PATH"


ZERO_SHA="0000000000000000000000000000000000000000"

# ── Lockfile Consistency Check ────────────────────────────────────
# Catch uncommitted lockfile changes that would break CI's --frozen-lockfile
if { git diff --name-only HEAD 2>/dev/null || true; } | grep -q 'pnpm-lock.yaml'; then
  echo "WARNING: pnpm-lock.yaml has uncommitted changes"
  echo "  CI uses --frozen-lockfile and will fail if lockfile is stale."
  echo "  Run: git add pnpm-lock.yaml && git commit --amend"
fi

# ── Change Detection ──────────────────────────────────────────────

CHANGED=""
PUSH_TARGETS=""
while read -r LOCAL_REF LOCAL_SHA REMOTE_REF REMOTE_SHA; do
  # Skip delete pushes
  if [ "$LOCAL_SHA" = "$ZERO_SHA" ]; then
    continue
  fi

  # Track push targets — sweettest tiering keys off "is this the dev push?"
  PUSH_TARGETS="$PUSH_TARGETS $REMOTE_REF"

  # Determine what changed
  if [ "$REMOTE_SHA" = "$ZERO_SHA" ]; then
    # New branch - check recent commits against remote
    NEW_CHANGED=$(git diff --name-only HEAD~10 HEAD 2>/dev/null || echo "")
  else
    NEW_CHANGED=$(git diff --name-only "$REMOTE_SHA" "$LOCAL_SHA" 2>/dev/null || echo "")
  fi

  # Accumulate changed files across all refs
  if [ -n "$NEW_CHANGED" ]; then
    CHANGED=$(printf '%s\n%s' "$CHANGED" "$NEW_CHANGED")
  fi
done

# Nothing changed — let it through
if [ -z "$CHANGED" ]; then
  exit 0
fi

# ── Elohim-agent package projection freshness ────────────────────
#
# `.epr-meta/elohim/packages/**` is the canonical authoring surface for
# Elohim-native skills/agents (see the elohim-package-authoring skill);
# `.claude/skills/**`, `.claude/agents/**`, and `.codex/**` are GENERATED
# projections of those packages, never hand-edited independently. When any
# of these paths change in this push range, verify the projections are
# still fresh relative to their package source — the same class of
# generated-artifact freshness check as .ci-ignore/schema-codegen below,
# just for the agent/skill capability surface instead of code/schema.
#
# MUST run on the raw (pre-.ci-ignore-filter) $CHANGED and before the filter's
# early exit: .ci-ignore lists a broad `.claude/` prefix (CI doesn't need to
# build on agent/skill-only edits), which would otherwise silently drop
# `.claude/skills/**` / `.claude/agents/**` out of $CHANGED — and, if those
# were the ONLY paths touched, exit the whole hook at "All changes are
# .ci-ignore'd" before this check ever ran. Local governance still cares even
# when CI doesn't build.
#
# Pure node (package-projections.mjs verify), no cargo — PVC-pressure-neutral,
# never part of HEAVY_GATES. Fail-open if node is absent
# (degraded shell) — same posture as the node-guarded blocks below.
if echo "$CHANGED" | grep -qE "^\.epr-meta/elohim/|^\.claude/skills/|^\.claude/agents/|^\.codex/"; then
  if command -v node >/dev/null 2>&1; then
    echo "[pre-push] Verifying elohim-agent package projections (Claude/Codex) are fresh..."
    if ! node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify; then
      echo ""
      echo "!!============================================================!!"
      echo "!! ELOHIM-AGENT PACKAGE PROJECTION DRIFT DETECTED"
      echo "!! .claude/skills, .claude/agents, or .codex is stale relative to"
      echo "!! its .epr-meta/elohim/packages source (or vice versa)."
      echo "!! Fix — regenerate the stale side, then stage it, then re-push:"
      echo "!!   pnpm run elohim-agent:packages:project   (package -> runtime projection)"
      echo "!!   pnpm run elohim-agent:packages:import    (runtime -> package import)"
      echo "!!============================================================!!"
      echo ""
      exit 1
    fi
    echo "[pre-push] elohim-agent package projections ✓"
  fi
fi

# ── Claude hook boundary tests ─────────────────────────────
#
# .claude/hooks/** are PreToolUse gates that decide when the pilot gets interrupted —
# sensitive-file-protection.py owns the confidentiality boundary. Its committed test
# suite already pinned that boundary in both directions, but nothing executed it, so a
# 2026-08-19 "cure" for a Rust-expression false positive silently disabled secret
# detection for every UNQUOTED assignment (.env / YAML / shell — where live credentials
# actually get pasted) and shipped with its own regression test red. A verification that
# no lane runs is not a verification.
#
# MUST run on the raw (pre-.ci-ignore-filter) $CHANGED and before that filter's early
# exit, for the same reason as the projection check above: .ci-ignore drops the broad
# `.claude/` prefix, and a hooks-only push would otherwise exit at "All changes are
# .ci-ignore'd" before this ever ran.
#
# Pure python (~ms), no cargo — PVC-EXEMPT BY OMISSION: never in HEAVY_GATES, never
# deferrable. Fail-open if python3 is absent (degraded shell), same posture as above.
if echo "$CHANGED" | grep -qE "^\.claude/hooks/"; then
  if command -v python3 >/dev/null 2>&1; then
    hook_tests_ran=0
    for hook_test in .claude/hooks/tests/test_*.py; do
      [ -f "$hook_test" ] || continue
      hook_tests_ran=1
      echo "[pre-push] Running Claude hook boundary tests: $hook_test"
      if ! python3 "$hook_test"; then
        echo ""
        echo "!!============================================================!!"
        echo "!! CLAUDE HOOK BOUNDARY TEST FAILED: $hook_test"
        echo "!! A PreToolUse hook changed its interruption boundary. If the new"
        echo "!! behavior is intended, update the assertion IN THE SAME COMMIT so the"
        echo "!! boundary stays declared; do not push a red boundary test."
        echo "!!============================================================!!"
        echo ""
        exit 1
      fi
    done
    # A glob that matches nothing must not print a success line — a renamed or deleted
    # suite would otherwise read as "gate passed" forever, which is the same
    # verification-that-never-runs failure this gate exists to close.
    if [ "$hook_tests_ran" -eq 0 ]; then
      echo "[pre-push] ERROR: .claude/hooks changed but no boundary tests were found."
      echo "  Expected at least one .claude/hooks/tests/test_*.py"
      exit 1
    fi
    echo "[pre-push] Claude hook boundary tests ✓"
  fi
fi

# ── .ci-ignore filter ─────────────────────────────────────────────
#
# Drop files that are listed in repo-root .ci-ignore (CLAUDE.md, .claude/,
# .github/, .husky/, etc.) BEFORE any project-detection runs. Same parser
# the Jenkinsfile and graph-walker use, so the local manifest selector and CI
# agree on what counts as a "source" change.
#
# If node isn't on PATH (degraded shell), skip filtering — fail-open is
# safer than refusing to push.
if command -v node >/dev/null 2>&1 && [ -f genesis/orchestrator/ci-ignore.mjs ]; then
  FILTERED=$(printf '%s\n' "$CHANGED" | node genesis/orchestrator/ci-ignore.mjs 2>/dev/null)
  if [ $? -eq 0 ]; then
    CHANGED="$FILTERED"
  fi
  if [ -z "$CHANGED" ]; then
    echo "[pre-push] All changes are .ci-ignore'd (docs/agents/CI-only) — skipping gates."
    exit 0
  fi
fi

# Agentic capability manifests are SDK vocabulary artifacts, not app/storage
# runtime code. The build graph currently treats broad elohim/sdk/** globs as
# app/storybook/storage inputs, which over-gates this manifest-only slice.
# Keep the local push gate scoped to the focused schema validation until
# generated/runtime consumers exist.
#
# NOTE: root package.json (and any other file that can affect builds outside
# elohim/sdk/domains/elohim-agent/) must NOT be excluded here — excluding it
# would make the remainder empty for a push that also touches the workspace
# root, letting it skip every downstream gate (app/storybook/storage,
# .ci-ignore freshness, the .epr-meta compose-gate backstop, sweettest-check).
# Only exclude paths that are genuinely inert outside the agent scope: the
# domains-level README (doc-only) and the agent's own dedicated test script.
AGENT_MANIFEST_SCOPE_REMAINDER=$(
  printf '%s\n' "$CHANGED" | grep -Ev '^(elohim/sdk/domains/README\.md|elohim/sdk/domains/elohim-agent/.*|elohim/sdk/schemas/scripts/test-elohim-agent-manifest\.mjs)$' || true
)
if [ -z "$AGENT_MANIFEST_SCOPE_REMAINDER" ] &&
   printf '%s\n' "$CHANGED" | grep -qE '^elohim/sdk/domains/elohim-agent/|^elohim/sdk/schemas/scripts/test-elohim-agent-manifest\.mjs$'; then
  echo "[pre-push] elohim-agent manifest/schema-only changes — running focused gate."
  pnpm run elohim-agent:test || exit 1
  exit 0
fi

# ── Humans/Presences Schema Validation + Freshness ───────────────
#
# When genesis/data/humans/ or genesis/data/presences/ markdown changes,
# validate schemas and ensure the generated humans.json / presences.json
# artifacts are fresh relative to their markdown sources.
if echo "$CHANGED" | grep -qE "genesis/data/humans/|genesis/data/presences/"; then
  echo "[pre-push] Validating humans and presences schemas..."
  (cd genesis/seeder && pnpm run validate:humans) || exit 1
  (cd genesis/seeder && pnpm run validate:presences) || exit 1

  echo "[pre-push] Regenerating humans.json / presences.json from markdown sources..."
  (cd genesis/seeder && pnpm run build:data) || exit 1

  if ! git diff --quiet -- genesis/data/humans/humans.json genesis/data/presences/presences.json 2>/dev/null; then
    echo "[pre-push] ERROR: humans.json or presences.json is stale relative to markdown sources."
    echo "  Re-run 'pnpm --filter genesis-seeder run build:data' and stage the changes before pushing."
    exit 1
  fi
  echo "[pre-push] humans/presences validation ✓"
fi

# ── Device Archetype Validation ──────────────────────────────────
#
# When device archetype markdown files change, validate frontmatter
# against devices.schema.json and regenerate devices.json.
if echo "$CHANGED" | grep -qE "genesis/data/devices/"; then
  echo "[pre-push] Validating device archetypes..."
  (cd genesis/seeder && pnpm run validate:devices) || exit 1

  echo "[pre-push] Regenerating devices.json from markdown sources..."
  (cd genesis/seeder && pnpm run generate:devices) || exit 1

  if ! git diff --quiet -- genesis/data/devices/devices.json 2>/dev/null; then
    echo "[pre-push] ERROR: devices.json is stale relative to markdown sources."
    echo "  Re-run 'pnpm --filter holochain-seeder run generate:devices' and stage the changes before pushing."
    exit 1
  fi
  echo "[pre-push] device archetype validation ✓"
fi

# ── Deployment ↔ Archetype Resource-Conformance Validation ───────
#
# When deployments.json, a per-human manifest, the archetype budget floor, or
# the validator itself changes, run validate:deployments — cross-refs (humanId,
# deviceArchetype) AND resource conformance: every consolidated human's effective
# conductor resources meet its deviceArchetype floor (archetype-resource-budgets.json),
# and explicit-manifest humans (adam) match their declared edgenode* budget.
# Closes the drift that silently under-provisioned adam vs its family-node-base
# archetype-mate (backlog archetype-resource-conformance-validation-gap) — the
# validator existed but was never gated (seeder-validate-deployments-stale-validator).
if echo "$CHANGED" | grep -qE "genesis/orchestrator/data/deployments\.json|genesis/orchestrator/manifests/humans/|genesis/data/devices/archetype-resource-budgets\.json|genesis/seeder/src/validate-deployments\.ts"; then
  echo "[pre-push] Validating deployments ↔ archetype resource conformance..."
  (cd genesis/seeder && pnpm run validate:deployments) || exit 1
  echo "[pre-push] deployments resource-conformance ✓"
fi

# ── Account Package Schema Validation ────────────────────────────
#
# When account packages or their source data change, validate the
# generated JSON against account-package.schema.json. Catches field
# drift (e.g. missing relationshipType) before it reaches CI/deploy.
if echo "$CHANGED" | grep -qE "genesis/data/account-packages/|genesis/data/humans/|genesis/seeder/src/account-package\.ts"; then
  echo "[pre-push] Validating account packages against schema..."
  (cd genesis/seeder && pnpm run validate:account-packages) || exit 1
  echo "[pre-push] account-packages validation ✓"
fi

# ── .ci-ignore freshness (projected from the .epr-meta manifest ci-trigger cascade) ──
#
# When any .epr-meta manifest or the generated .ci-ignore changes, ensure .ci-ignore is
# fresh relative to the root .epr-meta manifest ci-trigger leg. Fail-open if python3 is
# absent (degraded shell) — same posture as the node-guarded blocks above.
if echo "$CHANGED" | grep -qE "(^|/)\.epr-meta(/|$)|^\.ci-ignore$"; then
  if command -v python3 >/dev/null 2>&1; then
    echo "[pre-push] Verifying .ci-ignore is fresh relative to the .epr-meta manifest ci-trigger leg..."
    if ! python3 .claude/scripts/ci-ignore-projector.py --verify; then
      echo "[pre-push] ERROR: .ci-ignore is stale relative to the root .epr-meta manifest."
      echo "  Run: python3 .claude/scripts/ci-ignore-projector.py && git add .ci-ignore"
      exit 1
    fi
    echo "[pre-push] .ci-ignore freshness ✓"
  fi
fi

# ── habit register freshness (projected from the .epr-meta habit atoms) ──
#
# A habit is declared in the .epr-meta governance package of the directory whose behaviour it
# describes; genesis/manifests/habits.yaml is the GENERATED projection of that walk. This leg is
# why the projection can never become a second hand-written home — the failure mode this repo has
# already paid for twice (cluster-state.yaml vs ELOHIM_REMOTE_COMPUTE_STATUS; the deployments.json
# suspended flags that drifted until they were made derived). --check also refuses an INVALID
# census: a duplicate id, a concern claimed by two habits, or the max-2-active WIP fence breached.
# Pure-python (~ms) so it is PVC-EXEMPT BY OMISSION. Fail-open if python3 or the projector is
# absent (a branch predating the register must never be blocked).
if echo "$CHANGED" | grep -qE "\.habit\.md$|(^|/)\.epr-meta(/|$)|^genesis/manifests/habits\.yaml$"; then
  if command -v python3 >/dev/null 2>&1 && [ -f .claude/scripts/habits-project.py ]; then
    echo "[pre-push] Verifying the habit register projection is fresh..."
    if ! python3 .claude/scripts/habits-project.py --check; then
      echo "[pre-push] ERROR: genesis/manifests/habits.yaml is stale, or the habit census is invalid."
      echo "  Run: python3 .claude/scripts/habits-project.py && git add genesis/manifests/habits.yaml"
      exit 1
    fi
    echo "[pre-push] habit register freshness ✓"
  fi
fi

# ── .epr-meta compose-gate (author-time rule evaluation over the push range) ──
#
# The commit-time gate (.husky/pre-commit) is the primary; this is the backstop for commits made
# with --no-verify at commit time but pushed normally. Same pure engine (_lib/epr_meta.py) over the
# push range instead of the staged set. Pure-python (~ms), so it is PVC-EXEMPT BY OMISSION: never in
# HEAVY_GATES and never deferrable. Fail-open if python3 OR
# the gate script is absent (a branch predating the gate must never be blocked).
if command -v python3 >/dev/null 2>&1 && [ -f .claude/scripts/epr-meta-git-gate.py ]; then
  RANGE_BASE=$(git merge-base origin/dev HEAD 2>/dev/null || echo "HEAD~1")
  if ! python3 .claude/scripts/epr-meta-git-gate.py --range "${RANGE_BASE}..HEAD"; then
    echo "[pre-push] ERROR: .epr-meta compose-gate rejected a change in this push range."
    echo "  Acknowledge (ask-class): EPR_META_ACK=1 git push   |   bypass all: git push --no-verify"
    exit 1
  fi
  echo "[pre-push] .epr-meta compose-gate ✓"
fi

# ── Project Detection (manifest-driven) ──────────────────────────
#
# Try graph walker first — reads build-manifest.json files and matches
# changed files against source globs. Falls back to grep patterns if
# node is unavailable or no manifests exist.

PROJECTS=""

if command -v node >/dev/null 2>&1; then
  # One manifest registry owns both detection and execution. New gate-only
  # checks declare `gate.projects[*].inputs`; there is no second grep map.
  # stderr is NOT suppressed: selection is now fail-CLOSED (a malformed manifest
  # or an unregistered gate project blocks every push), so the reason must be
  # visible. Swallowing it would trade the old "Unknown project" abort for a
  # silent one.
  PROJECTS=$(echo "$CHANGED" | node genesis/orchestrator/gate-runner.mjs --changed-file-list --names | tr '\n' ' ')
  [ $? -eq 0 ] || { echo "[pre-push] manifest gate selection failed (see error above)"; exit 1; }
else
  echo "[pre-push] node unavailable — manifest gate selection skipped (CI backstop)."
fi

# No grep fallback: build manifests are the sole positive gate detector.
# Trim leading space
PROJECTS=$(echo "$PROJECTS" | sed 's/^ //')

# ── Project filter ────────────────────────────────────────────────
# Projects are stable manifest keys; the shared runner resolves directories.
drop_project() {
  DROP_NAME="$1"
  NEW_P=""
  for p in $PROJECTS; do
    [ "$p" = "$DROP_NAME" ] && continue
    NEW_P="$NEW_P $p"
  done
  PROJECTS=$(echo "$NEW_P" | sed 's/^ //')
}

# ── Selective gate skip (SKIP_SWEETTEST) ──────────────────────────
# Drop ONLY the sweettest compile-check while keeping every other gate.
# CI still runs sweettest as a backstop, so this trades a local pre-CI
# catch for push speed — narrower than the all-or-nothing HUSKY=0.
if [ "${SKIP_SWEETTEST-}" = "1" ] && echo "$PROJECTS" | grep -qw "sweettest-check"; then
  drop_project "sweettest-check"
  echo "[pre-push] SKIP_SWEETTEST=1 — skipping sweettest compile-check (CI still runs it)."
fi

# ── Sweettest tiering (integration layer) ─────────────────────────
# sweettest-check compiles the full Holochain conductor — keep it at the
# integration boundary (push to dev/main), not on every feature push.
# RUN_SWEETTEST=1 forces it anywhere; CI's DNA pipeline is the backstop.
if echo "$PROJECTS" | grep -qw "sweettest-check" && [ "${RUN_SWEETTEST-}" != "1" ]; then
  IS_INTEGRATION_PUSH=0
  for ref in $PUSH_TARGETS; do
    case "$ref" in
      refs/heads/dev|refs/heads/main) IS_INTEGRATION_PUSH=1 ;;
    esac
  done
  if [ "$IS_INTEGRATION_PUSH" = "0" ]; then
    drop_project "sweettest-check"
    echo "[pre-push] sweettest-check: integration-layer gate — deferred (push target is not dev/main)."
    echo "           It runs on the dev push; CI DNA pipeline is the backstop. Force now: RUN_SWEETTEST=1 git push"
  fi
fi

# No project source changes — let it through
if [ -z "$PROJECTS" ]; then
  exit 0
fi

# ── PVC-pressure strategy (policy-driven) ─────────────────────────
# Budget from genesis/agentic/pool-policy.json (fail-soft defaults). At the
# soft watermark: reclaim first via the guarded enforce ladder (it never
# touches this push's family or any slot a live cargo holds). At the hard
# ceiling: defer heavy Rust gates with a loud banner instead of cold-building
# tens of GB mid-push. FORCE_HEAVY_GATES=1 overrides the deferral.
REPO_TOP="$(git rev-parse --show-toplevel 2>/dev/null || true)"
POOL_POLICY="$REPO_TOP/genesis/agentic/pool-policy.json"
SOFT_PCT=88; HARD_PCT=92
if command -v jq >/dev/null 2>&1 && [ -f "$POOL_POLICY" ]; then
  SOFT_PCT=$(jq -r '.volume_soft_pct // 88' "$POOL_POLICY" 2>/dev/null || echo 88)
  HARD_PCT=$(jq -r '.volume_hard_pct // 92' "$POOL_POLICY" 2>/dev/null || echo 92)
fi
# df wrapped in `|| true` INSIDE the substitution: under `sh -e` + pipefail a
# failing df would otherwise abort the entire hook (review C3) — a trailing
# `|| echo` after the pipe can't help, pipefail already failed the assignment.
DISK_PCT=$({ df -P /projects 2>/dev/null || true; } | awk 'NR==2 {gsub("%","",$5); print $5}')
HEAVY_GATES="elohim-storage epr-storage doorway steward-node elohim-sdk elohim-compute elohim-epr eprfs seam-contracts sweettest-check domain-types"

if [ -n "${DISK_PCT:-}" ] && [ "$DISK_PCT" -ge "$SOFT_PCT" ] && [ "${CARGO_TARGET_POOL_NO_ENFORCE:-0}" != "1" ]; then
  CARGO_POOL_BIN="$REPO_TOP/genesis/agentic/bin/cargo-pool"
  if [ -x "$CARGO_POOL_BIN" ]; then
    echo "[pre-push] disk at ${DISK_PCT}% >= soft ${SOFT_PCT}% — reclaiming via pool policy before gates..."
    # Export the pushing family explicitly (review D4): if this push runs
    # from a checkout outside .claude/worktrees, protected_families would
    # otherwise not associate the pushing branch's family with a live
    # session — the env override makes the protection authoritative.
    PUSH_FAMILY=""
    if [ -f "$REPO_TOP/genesis/agentic/bin/pool-lib.sh" ]; then
      # Use the pool's family authority here too; do not maintain a second
      # branch-name parser in the hook.
      source "$REPO_TOP/genesis/agentic/bin/pool-lib.sh"
      PUSH_FAMILY="$(detect_family "$REPO_TOP" 2>/dev/null || true)"
    fi
    if command -v timeout >/dev/null 2>&1; then
      CARGO_TARGET_POOL_FAMILY="${PUSH_FAMILY:-}" timeout 300 bash "$CARGO_POOL_BIN" enforce --yes --quiet 2>/dev/null || true
    else
      CARGO_TARGET_POOL_FAMILY="${PUSH_FAMILY:-}" bash "$CARGO_POOL_BIN" enforce --yes --quiet 2>/dev/null || true
    fi
    DISK_PCT=$({ df -P /projects 2>/dev/null || true; } | awk 'NR==2 {gsub("%","",$5); print $5}')
    echo "[pre-push] disk now at ${DISK_PCT:-?}%"
  fi
fi

if [ -n "${DISK_PCT:-}" ] && [ "$DISK_PCT" -ge "$HARD_PCT" ] && [ "${FORCE_HEAVY_GATES-}" != "1" ]; then
  DEFERRED=""
  for h in $HEAVY_GATES; do
    if echo "$PROJECTS" | grep -qw "$h"; then
      DEFERRED="$DEFERRED $h"
      drop_project "$h"
    fi
  done
  if [ -n "$DEFERRED" ]; then
    echo ""
    echo "!!============================================================!!"
    echo "!! DEFERRED-BY-PVC: disk at ${DISK_PCT}% >= hard ${HARD_PCT}% ceiling"
    echo "!! Heavy Rust gates NOT run:$DEFERRED"
    echo "!! CI remains the backstop on dev. To run them now:"
    echo "!!   bash genesis/agentic/bin/cargo-pool enforce --yes   # reclaim"
    echo "!!   FORCE_HEAVY_GATES=1 git push                        # override"
    echo "!!============================================================!!"
    echo ""
  fi
fi

# Re-check: pressure deferral may have emptied the project list.
if [ -z "$PROJECTS" ]; then
  echo "[pre-push] all detected gates deferred — push allowed (CI backstop)."
  exit 0
fi

# Resource Guard — pause rust-analyzer during manifest-declared gates.
#
# Pause rust-analyzer during gate runs to free ~8.8GB of active RAM.
# SIGSTOP freezes the process (no new allocations, OS can reclaim pages).
# SIGCONT resumes on any exit — normal, error, or signal.

RA_PIDS=""
if command -v pgrep >/dev/null 2>&1; then
  RA_PIDS=$(pgrep -f rust-analyzer 2>/dev/null || true)
fi

resume_rust_analyzer() {
  if [ -n "$RA_PIDS" ]; then
    for pid in $RA_PIDS; do
      kill -CONT "$pid" 2>/dev/null || true
    done
  fi
}

# ── Brit advisory (Stage 1a) ─────────────────────────────────────
# Runs brit verify + brit plan as advisory checks. Warn-only — never fails push.
# Stage 2: becomes load-bearing (failure blocks push). Stage 1a: gather signal.
#
# IMPORTANT: REPO_ROOT is NOT set by git when invoking pre-push hooks in
# production. We derive the helper path via git rev-parse — git hooks run with
# CWD set to the repo root, but using rev-parse is more robust to future CWD
# changes. The helper itself defaults its own ${REPO_ROOT:-/projects/elohim}
# fallback when invoked without it being exported.
brit_advisory() {
  REPO_TOPLEVEL="$(git rev-parse --show-toplevel 2>/dev/null)"
  if [ -z "$REPO_TOPLEVEL" ]; then
    return 0  # not in a git repo (shouldn't happen for pre-push); silent skip
  fi
  HELPER="$REPO_TOPLEVEL/genesis/orchestrator/scripts/brit-helper.sh"
  if [ ! -x "$HELPER" ]; then
    return 0  # helper not present (e.g., older branch); silently skip
  fi
  echo ""
  echo "── brit advisory (Stage 1a — warn-only) ─────────────────────"
  REPO_ROOT="$REPO_TOPLEVEL" "$HELPER" verify
  # plan against origin/dev (the merge target); if origin/dev is unreachable, skip.
  if git rev-parse --verify origin/dev >/dev/null 2>&1; then
    REPO_ROOT="$REPO_TOPLEVEL" "$HELPER" plan --target origin/dev 2>/dev/null || true
  fi
}

if [ -n "$RA_PIDS" ]; then
  RA_COUNT=$(echo "$RA_PIDS" | wc -l | tr -d ' ')
  echo "  Pausing $RA_COUNT rust-analyzer process(es) to free RAM..."
  for pid in $RA_PIDS; do
    kill -STOP "$pid" 2>/dev/null || true
  done
  trap resume_rust_analyzer EXIT INT TERM HUP
fi

# ── Run Gates ─────────────────────────────────────────────────────

PROJECT_COUNT=$(echo "$PROJECTS" | wc -w | tr -d ' ')
DISPLAY_LIST=$(echo "$PROJECTS" | tr ' ' ', ')

echo ""
echo "============================================"
echo "  PRE-PUSH GATE: Changes detected"
echo "============================================"
echo "  Projects: $DISPLAY_LIST"
echo ""

TOTAL_START=$(date +%s)
FAILED=""
RESULTS=""
INDEX=0

for PROJECT in $PROJECTS; do
  INDEX=$((INDEX + 1))
  PROJECT_START=$(date +%s)

  node genesis/orchestrator/gate-runner.mjs --target "$PROJECT"
  GATE_EXIT=$?

  PROJECT_END=$(date +%s)
  PROJECT_ELAPSED=$((PROJECT_END - PROJECT_START))

  if [ $GATE_EXIT -ne 0 ]; then
    echo "$PROJECT: FAILED (${PROJECT_ELAPSED}s)"
    FAILED="$FAILED $PROJECT"
  else
    echo "$PROJECT: PASSED (${PROJECT_ELAPSED}s)"
  fi
  echo ""

  RESULTS="$RESULTS  $PROJECT: $([ $GATE_EXIT -eq 0 ] && echo PASSED || echo FAILED) (${PROJECT_ELAPSED}s)\n"
done

TOTAL_END=$(date +%s)
TOTAL_ELAPSED=$((TOTAL_END - TOTAL_START))

# Trim leading space from FAILED
FAILED=$(echo "$FAILED" | sed 's/^ //')

if [ -n "$FAILED" ]; then
  echo "============================================"
  echo "  PRE-PUSH GATE: FAILED (${TOTAL_ELAPSED}s)"
  echo "============================================"
  echo "  Failed: $FAILED"
  echo ""
  echo "  Fix errors before pushing."
  echo "  Bypass: HUSKY=0 git push"
  echo "============================================"
  exit 1
fi

echo "============================================"
echo "  PRE-PUSH GATE: ALL CLEAR (${TOTAL_ELAPSED}s)"
echo "============================================"
echo ""

# ── cargo-deny bans check ────────────────────────────────────────
# Enforces the elohim-views/elohim-sdk boundary — see deny.toml for rules.
if command -v cargo-deny >/dev/null 2>&1; then
  echo ""
  echo "── cargo deny check bans (elohim workspace) ───────────────────"
  if ! (cd "$(git rev-parse --show-toplevel)" && cargo deny --manifest-path elohim/Cargo.toml check bans 2>&1 | tail -20); then
    echo "✗ cargo-deny failed — boundary violation. See deny.toml for rules."
    exit 1
  fi
  echo "── end cargo deny check ───────────────────────────────────────"
fi

# Stage 1a: advisory brit checks (never fail the push).
brit_advisory

exit 0
