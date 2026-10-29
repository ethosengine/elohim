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

# ── Anchor Git context before gates change directory ────────────
# Git exports GIT_DIR to hooks without necessarily exporting GIT_WORK_TREE.
# With an explicit GIT_DIR, Git otherwise treats each gate's cwd as its worktree.
HOOK_WORK_TREE=$(git rev-parse --show-toplevel) || exit 1
HOOK_GIT_DIR=$(git rev-parse --absolute-git-dir) || exit 1
export GIT_WORK_TREE="$HOOK_WORK_TREE"
export GIT_DIR="$HOOK_GIT_DIR"
# ── Git context anchored ────────────────────────────────────────

# ── Attested submodules checked out ─────────────────────────────
# brit, rakia and sophia declare their attested gate projects in manifests that
# live inside each submodule. A tree without them runs every gate, then the
# orchestrator gate's registry and oracle tests fail (2026-10-27: refused at
# 104 s, after eprfs had spent 5231 s). Refuse before any gate starts.
attested_submodules_missing() {
  git submodule status -- elohim/brit elohim/rakia sophia 2>/dev/null | awk '/^-/ { print $2 }'
}
MISSING_SUBMODULES=$(attested_submodules_missing | xargs)
if [ -n "$MISSING_SUBMODULES" ]; then
  echo "[pre-push] REFUSED: not checked out: $MISSING_SUBMODULES. Run: git submodule update --init $MISSING_SUBMODULES" >&2
  exit 1
fi
# ── Attested submodules present ─────────────────────────────────

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
#   cid-artifact-freshness   → generated CID twin matches its source .md (genesis/docs/content/elohim-protocol/ or genesis/data/lamad/content/ change)
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

# The pushed refs, read once: the berth claims below must see them before any check runs.
PUSH_LINES=()
while read -r PUSH_LINE; do
  PUSH_LINES+=("$PUSH_LINE")
done

# ── Berth: integration claims ─────────────────────────────────────
#
# A push to dev or main lands on the branch every device of this person integrates on, so it holds
# the steward-scoped `dev-integration` through berth for the life of the push (class verify, ttl
# 1800 s, renewed every 600 s while the gates run — a steward lease with no ttl would bind until
# released, and an unrenewed one would lapse under a long landing). A pushed commit carrying
# `[build:edge]` (or a tag list naming edge, or `[build:all]`) also needs `alpha-fleet`, since it
# deploys to the test fleet; any `[build:*]` tag also needs `jenkins-dispatch`. Spec
# 2026-10-23-workspaces-cooperate-as-devices-design.md §3.5. Every pushed commit's message is read
# ("a pushed commit"), so this can claim more than the orchestrator dispatches, which reads only the
# tip's message: a tag quoted in an earlier commit's prose still claims. Over-claiming is the safe
# side. The messages are matched with here-strings, never a pipe: under pipefail an early-exiting
# `grep -q` on more than a pipe buffer of messages read as "no tag" (SIGPIPE).
#
#   berth exit 0  held for this push, released on every exit below (pass, failed gate, refusal,
#                 Ctrl-C); a claim that only RENEWED a lease this session already held (a session
#                 watching a deploy) is left to that session, never released here
#   berth exit 3  the push is refused before any check runs; berth's reason line is printed as is
#   berth exit 4  no session resolvable: moor a hook-owned holder and retry; refuse if still unleased
#   berth exit 1  only for berth's own "renewed here, but …" line (a renew of a lease this session
#                 held, whose record did not reach the channel: still held, earlier terms); any
#                 other exit 1 is an error and refuses the push
#
# A workspace that is one of its steward's devices but has no `epr` binary is refused every push
# to dev (berth: "no `epr` binary to read the steward lease view …"): an unreadable view is never a
# free resource. The cure is the one berth names — build `epr` or set EPR_BIN.
#
# The hook never overrides a silent device on its own: BERTH_OFFLINE_OK=1, set by the pushing
# session, makes each claim an explicit `--offline-ok` (it overrides `unconfirmed` only, never
# `held by`, and leaves an override record). A checkout with no berth refuses integration pushes.
# HUSKY=0 still bypasses the whole hook.
#
# core.hooksPath is absolute (.husky/hooks-path.mjs), so a push from a linked worktree runs the MAIN
# checkout's copy of this script against the worktree: until the main checkout carries this block,
# such a push claims nothing. The hook on dev governs pushes to dev.
HOOK_ON_EXIT=""
HOOK_OWNER_PID=$BASHPID
hook_on_exit() {
  # A renewer can exit before resetting its inherited traps; only this shell owns the claims.
  [ "$BASHPID" = "$HOOK_OWNER_PID" ] || return 0
  local handler
  for handler in $HOOK_ON_EXIT; do
    "$handler"
  done
}
trap hook_on_exit EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

BERTH_BIN="$HOOK_WORK_TREE/genesis/agentic/bin/berth"
BERTH_HELD=""
BERTH_KEPT="" # leases this session already held: renewed while the gates run, never released
BERTH_CLAIMING=""
BERTH_HOOK_SESSION=""
berth_moor_hook() {
  local principal
  principal=$(git config user.email || git config user.name || id -un) || return 1
  BERTH_HOOK_SESSION="pre-push-${HOOK_OWNER_PID}-${EPOCHREALTIME//./}-$RANDOM"
  # Declare the same P1 owner to berth and native envelope validation; the parent is unchanged.
  export ELOHIM_SESSION_ID="$BERTH_HOOK_SESSION" ELOHIM_SESSION_OWNER_PID="$HOOK_OWNER_PID"
  "$BERTH_BIN" moor --session "$BERTH_HOOK_SESSION" --pid "$HOOK_OWNER_PID" \
    --runtime git-hook --principal "$principal" --task "pre-push to${BERTH_TARGETS}"
}
berth_release_claims() {
  local res
  # A claim still in flight when the push ends (Ctrl-C mid-settle) may have written its lease.
  for res in $BERTH_HELD $BERTH_CLAIMING; do
    "$BERTH_BIN" release "$res" >/dev/null 2>&1 ||
      echo "[pre-push] berth: $res released here, but did not reach the channel — run: berth release $res" >&2
  done
  BERTH_HELD=""
  if [ -n "$BERTH_HOOK_SESSION" ]; then
    "$BERTH_BIN" unmoor --session "$BERTH_HOOK_SESSION" >/dev/null 2>&1 ||
      echo "[pre-push] berth: hook mooring cleanup failed — run: berth unmoor --session $BERTH_HOOK_SESSION" >&2
  fi
}
HOOK_ON_EXIT="$HOOK_ON_EXIT berth_release_claims"

# While the gates run, every lease this push needs (taken or already held) is renewed every ttl/3 by a background loop, so a
# long landing never outlives its lease (the other devices read a lease past since + ttl as dead).
# A renew whose record did not cross prints berth's line and keeps going; a renew refused outright
# prints berth's line and stops the push (at the end of the gate step then running). The loop is
# stopped with its subprocess group on every exit, before releasing the claims, so nothing
# renews a lease after the push gave it back.
BERTH_TTL_S="${BERTH_PREPUSH_TTL_S:-1800}"
BERTH_HOOK_PID=$$
BERTH_RENEW_PID=""
berth_renew_loop() {
  local interval=$((BERTH_TTL_S / 3)) sleeper="" res out rc
  [ "$interval" -ge 1 ] || interval=1
  set +m # descendants share this renewer's process group
  trap - EXIT INT HUP
  # A sleeper forked but not yet exec'd still runs this trap's handler, swallows the group TERM and
  # then sleeps the full interval under `wait`; KILL cannot be caught in that window.
  trap 'trap "" TERM; [ -z "$sleeper" ] || kill -KILL "$sleeper" 2>/dev/null; kill -TERM -- "-$BASHPID" 2>/dev/null; wait; exit 0' TERM
  while :; do
    sleep "$interval" </dev/null >/dev/null 2>&1 &
    sleeper=$!
    wait "$sleeper"
    sleeper=""
    for res in $BERTH_HELD $BERTH_KEPT; do
      out=$("$BERTH_BIN" claim "$res" "${BERTH_FLAGS[@]}" 2>&1)
      rc=$?
      [ "$rc" -eq 0 ] && continue
      [ -n "$out" ] && printf '%s\n' "$out" >&2
      if [ "$rc" -eq 1 ] && grep -q "^$res: renewed here, but " <<<"$out"; then
        continue
      fi
      echo "[pre-push] REFUSED: renewing $res while the gates ran was refused (above). The push stops." >&2
      kill -TERM "$BERTH_HOOK_PID" 2>/dev/null
      exit 0
    done
  done
}
berth_stop_renewing() {
  [ -n "$BERTH_RENEW_PID" ] || return 0
  kill -TERM -- "-$BERTH_RENEW_PID" 2>/dev/null
  wait "$BERTH_RENEW_PID" 2>/dev/null
  BERTH_RENEW_PID=""
}
HOOK_ON_EXIT="berth_stop_renewing $HOOK_ON_EXIT"

BERTH_TARGETS=""
BERTH_MESSAGES=""
for PUSH_LINE in "${PUSH_LINES[@]}"; do
  read -r _ B_LOCAL B_REMOTE_REF B_REMOTE <<<"$PUSH_LINE"
  case "$B_REMOTE_REF" in
    refs/heads/dev | refs/heads/main) ;;
    *) continue ;;
  esac
  BERTH_TARGETS="$BERTH_TARGETS ${B_REMOTE_REF#refs/heads/}"
  [ "$B_LOCAL" = "$ZERO_SHA" ] && continue
  if [ "$B_REMOTE" != "$ZERO_SHA" ] && git cat-file -e "$B_REMOTE^{commit}" 2>/dev/null; then
    BERTH_MESSAGES="$BERTH_MESSAGES$(git log --format=%B "$B_REMOTE..$B_LOCAL" 2>/dev/null)"$'\n'
  else
    BERTH_MESSAGES="$BERTH_MESSAGES$(git log --format=%B "$B_LOCAL" --not --remotes 2>/dev/null)"$'\n'
  fi
done

if [ -n "$BERTH_TARGETS" ]; then
  if [ ! -x "$BERTH_BIN" ]; then
    echo "[pre-push] REFUSED: this push needs dev-integration; restore executable $BERTH_BIN."
    exit 1
  fi
  # The orchestrator's own tag grammar (genesis/orchestrator/commit-tag-parser.mjs): [build:a,b-c].
  BERTH_NEEDS="dev-integration"
  if grep -qiE '\[build:([a-z,-]*,)?(edge|all)(,[a-z,-]*)?\]' <<<"$BERTH_MESSAGES"; then
    BERTH_NEEDS="$BERTH_NEEDS alpha-fleet"
  fi
  if grep -qiE '\[build:[a-z,-]+\]' <<<"$BERTH_MESSAGES"; then
    BERTH_NEEDS="$BERTH_NEEDS jenkins-dispatch"
  fi
  BERTH_FLAGS=(--class verify --ttl "$BERTH_TTL_S" --note "pre-push to${BERTH_TARGETS}")
  [ "${BERTH_OFFLINE_OK-}" = "1" ] && BERTH_FLAGS+=(--offline-ok)
  for BERTH_RES in $BERTH_NEEDS; do
    BERTH_CLAIMING="$BERTH_RES"
    BERTH_OUT=$("$BERTH_BIN" claim "$BERTH_RES" "${BERTH_FLAGS[@]}" 2>&1)
    BERTH_RC=$?
    if [ "$BERTH_RC" -eq 4 ] && [ "$BERTH_RES" = dev-integration ]; then
      [ -n "$BERTH_OUT" ] && printf '%s\n' "$BERTH_OUT" >&2
      if ! berth_moor_hook; then
        echo "[pre-push] REFUSED: could not moor a hook holder; repair berth's session/store error above, or push from a moored session."
        exit 1
      fi
      BERTH_OUT=$("$BERTH_BIN" claim "$BERTH_RES" "${BERTH_FLAGS[@]}" 2>&1)
      BERTH_RC=$?
    fi
    BERTH_CLAIMING=""
    [ -n "$BERTH_OUT" ] && printf '%s\n' "$BERTH_OUT" >&2
    case "$BERTH_RC" in
      0)
        # A renew is a lease this session already held (it watches that build or deploy, or an
        # earlier push of it was killed): not this push's to give back, but the gates still need it.
        if grep -q "^$BERTH_RES: renewed" <<<"$BERTH_OUT"; then
          BERTH_KEPT="$BERTH_KEPT $BERTH_RES"
        else
          BERTH_HELD="$BERTH_HELD $BERTH_RES"
        fi
        ;;
      1)
        if grep -q "^$BERTH_RES: renewed here, but " <<<"$BERTH_OUT"; then
          # A renew whose record did not cross: the lease is still held, on its earlier terms.
          echo "[pre-push] berth: $BERTH_RES is held, but the other devices see its earlier terms (above)."
        else
          echo "[pre-push] REFUSED: berth claim $BERTH_RES failed (exit 1, above). Bypass all gates: HUSKY=0 git push"
          BERTH_HELD="$BERTH_HELD $BERTH_RES" # it may have written its lease before failing
          exit 1
        fi
        ;;
      3)
        echo "[pre-push] REFUSED: this push needs $BERTH_RES (reason above). Nothing was pushed."
        exit 1
        ;;
      4)
        echo "[pre-push] REFUSED: this push needs $BERTH_RES; repair berth's session/store error above, or push from a moored session."
        exit 1
        ;;
      *)
        echo "[pre-push] REFUSED: berth claim $BERTH_RES exited $BERTH_RC (above). Bypass all gates: HUSKY=0 git push"
        BERTH_HELD="$BERTH_HELD $BERTH_RES" # it may have written its lease before failing
        exit 1
        ;;
    esac
  done
  if [ -n "$BERTH_HELD$BERTH_KEPT" ]; then
    # Bash establishes the group before running the subshell; no setsid startup race.
    set -m
    berth_renew_loop </dev/null &
    BERTH_RENEW_PID=$!
    set +m
  fi
fi
# ── Berth: claims taken ───────────────────────────────────────────

CHANGED=""
UNFILTERED_CHANGED=""
GATE_NARROW_BASE=ok
PUSH_TARGETS=""
LAST_LOCAL_SHA=""
DEV_LOCAL_SHA=""
for PUSH_LINE in "${PUSH_LINES[@]}"; do
  read -r LOCAL_REF LOCAL_SHA REMOTE_REF REMOTE_SHA <<<"$PUSH_LINE"
  # Skip delete pushes
  if [ "$LOCAL_SHA" = "$ZERO_SHA" ]; then
    continue
  fi

  # Track push targets — sweettest tiering keys off "is this the dev push?"
  PUSH_TARGETS="$PUSH_TARGETS $REMOTE_REF"
  LAST_LOCAL_SHA="$LOCAL_SHA"
  if [ "$REMOTE_REF" = "refs/heads/dev" ]; then
    DEV_LOCAL_SHA="$LOCAL_SHA"
  fi

  # Check EACH pushed object against its actual destination before filters or PVC
  # deferral. Never substitute the dirty checkout or last ref for the pushed tree.
  if [[ "$REMOTE_REF" =~ ^refs/heads/(dev|main)$ && "$REMOTE_SHA" != "$ZERO_SHA" ]]; then
    STYLE_BASE="$REMOTE_SHA"
  else
    STYLE_BASE="origin/dev"
  fi
  if ! bash "$(git rev-parse --show-toplevel)/genesis/orchestrator/rust-style.sh" --base "$STYLE_BASE" --head "$LOCAL_SHA"; then
    echo "Rust style reach gate refused this ref; fix findings or restore measurable inputs."
    exit 1
  fi

  # Determine what changed
  if [ "$REMOTE_SHA" = "$ZERO_SHA" ]; then
    # New branch - check recent commits against remote
    # HEAD~10 is a heuristic for detection, never a real narrowing base.
    GATE_NARROW_BASE=none
    NEW_CHANGED=$(git diff --name-only HEAD~10 HEAD 2>/dev/null || echo "")
  else
    if ! NEW_CHANGED=$(git diff --name-only "$REMOTE_SHA" "$LOCAL_SHA" 2>/dev/null); then
      GATE_NARROW_BASE=none
      NEW_CHANGED=""
    fi
  fi

  # Accumulate changed files across all refs
  if [ -n "$NEW_CHANGED" ]; then
    CHANGED=$(printf '%s\n%s' "$CHANGED" "$NEW_CHANGED")
    UNFILTERED_CHANGED=$(printf '%s\n%s' "$UNFILTERED_CHANGED" "$NEW_CHANGED")
  fi
done

# Keep the unfiltered union for package selection after .ci-ignore changes CHANGED.
GATE_CHANGED_FILES=$(mktemp)
gate_remove_changed_files() {
  [ -z "$GATE_CHANGED_FILES" ] || rm -f -- "$GATE_CHANGED_FILES"
}
HOOK_ON_EXIT="$HOOK_ON_EXIT gate_remove_changed_files"
if [ -z "$GATE_CHANGED_FILES" ] || ! printf '%s\n' "$UNFILTERED_CHANGED" > "$GATE_CHANGED_FILES"; then
  GATE_NARROW_BASE=none
fi
export GATE_CHANGED_FILES GATE_NARROW_BASE

# Gate evidence and household landing follow the exact object destined for dev. For pushes that
# do not include dev, preserve the former last-ref behavior for the ordinary union gate.
GATE_COMMIT="${DEV_LOCAL_SHA:-$LAST_LOCAL_SHA}"
export GATE_COMMIT
GATE_PUSH_TARGETS="${PUSH_TARGETS# }"
GATE_PUSH_BRANCHES=""
for GATE_PUSH_TARGET in $GATE_PUSH_TARGETS; do
  case "$GATE_PUSH_TARGET" in
    refs/heads/*) GATE_PUSH_BRANCHES="${GATE_PUSH_BRANCHES:+$GATE_PUSH_BRANCHES }${GATE_PUSH_TARGET#refs/heads/}" ;;
    *) GATE_PUSH_BRANCHES=""; break ;;
  esac
done
export GATE_PUSH_TARGETS GATE_PUSH_BRANCHES
CHECKED_OUT_HEAD=$(git rev-parse HEAD 2>/dev/null || echo "")
if [ -n "$GATE_COMMIT" ] && [ "$GATE_COMMIT" != "$CHECKED_OUT_HEAD" ]; then
  export GATE_EVIDENCE_RECORD=0
  echo "[pre-push] gate evidence will not be recorded: pushed SHA $GATE_COMMIT is not checked-out HEAD $CHECKED_OUT_HEAD"
else
  unset GATE_EVIDENCE_RECORD
fi

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
      # Scrubbed like the library harnesses below: an inherited GIT_DIR turns a test's temp-repo
      # `git commit` into a commit on the branch being pushed (2026-10-26, test_lsp_edit_event).
      if ! env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE -u GIT_PREFIX python3 "$hook_test"; then
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

# ── habit register freshness (projected from the .epr-meta habit atoms) ──
#
# A habit is declared in the .epr-meta governance package of the directory whose behaviour it
# describes; genesis/manifests/habits.yaml is the GENERATED projection of that walk. This leg is
# why the projection can never become a second hand-written home — the failure mode this repo has
# already paid for twice (cluster-state.yaml vs ELOHIM_REMOTE_COMPUTE_STATUS; the deployments.json
# suspended flags that drifted until they were made derived). --check also refuses an INVALID
# census: a duplicate id, a concern claimed by two habits, or the max-2-active WIP fence breached.
#
# ABOVE THE .ci-ignore FILTER, for the reason the two legs above it state: .ci-ignore drops the
# broad `.claude/` prefix, so a push touching only the PROJECTOR or the census library would exit
# at "All changes are .ci-ignore'd" before this ran — and a projector change is exactly what can
# make the committed projection stale. (The habit ATOMS and habits.yaml survive the filter, so
# this leg did fire for atom edits from below; the hole was the tooling that generates them.)
# Those two paths are in the trigger for the same reason.
#
# Pure-python (~ms) so it is PVC-EXEMPT BY OMISSION. Fail-open if python3 or the projector is
# absent (a branch predating the register must never be blocked).
if echo "$CHANGED" | grep -qE "\.habit\.md$|(^|/)\.epr-meta(/|$)|^genesis/manifests/habits\.yaml$|^\.claude/scripts/habits-project\.py$|^\.claude/scripts/_lib/epr_habits\.py$"; then
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

# ── agentic-library harnesses (the governance instruments' own tests) ──
#
# Sibling of the hook-boundary leg above, and it exists for the reason that leg names in its own
# comment: a verification that never runs. NOTHING ran `.claude/scripts/_lib/__tests__` — not
# pre-push, not the justfile, not package.json, not CI — so 46 harnesses guarding the resolver,
# the cascade, the census, the habit register and the measure vocabulary executed only when a
# human remembered to. The empty-glob guard below is copied from that leg deliberately: a renamed
# or deleted suite must not read as "gate passed" forever.
#
# MUST run on the raw (pre-.ci-ignore-filter) $CHANGED — `.claude/` is dropped by the filter, so
# below it this leg would never fire at all.
#
# Pure python (~28s, no cargo) — PVC-EXEMPT BY OMISSION: never in HEAVY_GATES, never deferrable.
# Fail-open if python3 is absent (degraded shell), same posture as the legs above.
if echo "$CHANGED" | grep -qE "^\.claude/scripts/"; then
  if command -v python3 >/dev/null 2>&1 && [ -d .claude/scripts/_lib/__tests__ ]; then
    lib_tests_ran=0
    lib_failed=""
    lib_parallel_tests=()
    lib_serial_tests=()
    if [[ ! "${HARNESS_JOBS:-4}" =~ ^[1-9][0-9]*$ ]]; then
      echo "[pre-push] ERROR: HARNESS_JOBS must be a positive integer."
      exit 1
    fi
    echo "[pre-push] Running agentic-library harnesses (.claude/scripts/_lib/__tests__)..."
    for lib_test in .claude/scripts/_lib/__tests__/*_test.py .claude/scripts/_lib/__tests__/test_*.py; do
      [ -f "$lib_test" ] || continue
      lib_tests_ran=$((lib_tests_ran + 1))
      # SERIAL holds repo-relative paths, with reason comments on separate lines. With one
      # worker, keep the original glob order; otherwise defer these until all workers finish.
      if [ "${HARNESS_JOBS:-4}" != 1 ] && [ -f .claude/scripts/_lib/__tests__/SERIAL ] &&
        grep -Fxq -- "$lib_test" .claude/scripts/_lib/__tests__/SERIAL; then
        lib_serial_tests+=("$lib_test")
      else
        lib_parallel_tests+=("$lib_test")
      fi
    done
    # git exports GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE/GIT_PREFIX into hooks; a harness that
    # spawns git inside a temp repo would then act on THIS repo (epr_habits_test red only under
    # the hook, 2026-09-11) — the same scrub gate-runner.mjs applies to gate children.
    if [ "${#lib_parallel_tests[@]}" -gt 0 ]; then
      if ! lib_failed=$(printf '%s\0' "${lib_parallel_tests[@]}" |
        xargs -0 -n 1 -P "${HARNESS_JOBS:-4}" bash -c '
          lib_test=$1
          if ! env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE -u GIT_PREFIX python3 "$lib_test" >/dev/null 2>&1; then
            printf "%s\n" "$lib_test"
          fi
        ' _); then
        echo "[pre-push] ERROR: agentic-library harness workers could not complete."
        exit 1
      fi
    fi
    for lib_test in "${lib_serial_tests[@]}"; do
      if ! env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE -u GIT_PREFIX python3 "$lib_test" >/dev/null 2>&1; then
        lib_failed="$lib_failed $lib_test"
      fi
    done
    if [ -n "$lib_failed" ]; then
      lib_failed=$(printf '%s\n' $lib_failed | LC_ALL=C sort)
      echo ""
      echo "!!============================================================!!"
      echo "!! AGENTIC-LIBRARY HARNESS FAILED:"
      for lib_t in $lib_failed; do
        echo "!!   $lib_t"
      done
      echo "!! These guard the governance instruments themselves — the resolver"
      echo "!! cascade, the seam census, the habit register, the measure vocabulary."
      echo "!! Re-run one for its output:  python3 <path>"
      echo "!!============================================================!!"
      echo ""
      exit 1
    fi
    # A glob that matches nothing must not print a success line — same guard, same reason as the
    # hook-boundary leg above.
    if [ "$lib_tests_ran" -eq 0 ]; then
      echo "[pre-push] ERROR: .claude/scripts changed but no library harnesses were found."
      echo "  Expected at least one .claude/scripts/_lib/__tests__/*_test.py"
      exit 1
    fi
    echo "[pre-push] agentic-library harnesses ✓ ($lib_tests_ran green)"
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
    echo "  Re-run '(cd genesis/seeder && pnpm run build:data)' and stage the changes before pushing."
    exit 1
  fi
  echo "[pre-push] humans/presences validation ✓"
fi

# ── CID-addressed artifact freshness (source .md ↔ generated CID twin) ──
#
# The seeder's cid-artifact-integrity guard lives in genesis/seeder, but the edit that
# BREAKS it is a source .md under genesis/docs/content/elohim-protocol — which selects the
# elohim-storybook gate project, never `genesis`. So the deterministic-floor guard existed
# and CI (elohim-genesis #1522) was the first observer: manifesto.json's blobHash/blobCid
# dangled while seven manifesto.md commits shipped. Key the freshness check to the SOURCE
# side, not the generator's directory. Pure node+tsx (~0.6s) → PVC-EXEMPT BY OMISSION.
if echo "$CHANGED" | grep -qE "^genesis/docs/content/elohim-protocol/|^genesis/data/lamad/content/"; then
  echo "[pre-push] Verifying CID-addressed artifacts are fresh relative to their source .md..."
  if ! (cd genesis/seeder && pnpm run sync:cid-artifacts:check); then
    echo "[pre-push] ERROR: a CID-addressed artifact drifted from its source .md — /blob/<hash> would 404."
    echo "  Run: (cd genesis/seeder && pnpm run sync:cid-artifacts) and stage the regenerated JSON."
    exit 1
  fi
  echo "[pre-push] CID-artifact freshness ✓"
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

# Runtime-manifest pin/render conformance. 2026-09-05: every active human is
# pinned, so Unpinned is now REFUSE (exit 65), alongside CID/resource drift.
# Use the same pool resolver as native gate.projects; no prebuilt binary needed.
if echo "$CHANGED" | grep -qE "bridges/k8s/|elohim/ark/core/|genesis/orchestrator/data/deployments\.json|genesis/orchestrator/manifests/runtime/|genesis/data/devices/archetype-resource-budgets\.json|scripts/ci/conductor-split-budget\.sh"; then
  echo "[pre-push] Verifying runtime manifest pins and rendered resources..."
  (
    k8s_repo_root="$(git rev-parse --show-toplevel)"
    source "$k8s_repo_root/genesis/agentic/bin/pool-lib.sh"
    k8s_pool_family="$(detect_family "$k8s_repo_root")"
    export CARGO_TARGET_DIR="$(slot_path "$k8s_pool_family" k8s-bridge dev)"
    export CARGO_BUILD_JOBS=2 RUSTFLAGS="" RUSTC_WRAPPER=""
    cargo run -q --manifest-path bridges/k8s/Cargo.toml --bin k8s-bridge -- verify --deployments genesis/orchestrator/data/deployments.json --manifests-dir genesis/orchestrator/manifests/runtime
  ) || exit 1
  echo "[pre-push] runtime manifest pin/render conformance ✓"
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

# ── T2 receipt (mandatory for serving paths) ───────────────────────
# The receipt script owns policy: serving paths require a complete current-source
# household proof; other dataplane paths warn unless T2_RECEIPT=strict.
# Advisory misses already exit 0. Propagate every nonzero result so a mandatory
# refusal or an unreadable gate can never become a successful push here.
T2_LIST=$(mktemp)
printf '%s\n' "$CHANGED" > "$T2_LIST"
if [ "${T2_RECEIPT-}" = "strict" ]; then
  bash genesis/orchestrator/scripts/t2-receipt.sh --changed "$T2_LIST" --strict || { rm -f "$T2_LIST"; exit 1; }
else
  bash genesis/orchestrator/scripts/t2-receipt.sh --changed "$T2_LIST" || { rm -f "$T2_LIST"; exit 1; }
fi
rm -f "$T2_LIST"

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

#
# A rust-analyzer the device's resident language server started (`epr lsp serve --device`
# marks each child ELOHIM_SESSION_ID=lsp-device) is never paused: it serves every attached
# session, and freezing it stalls all of their editors. Its flycheck builds already yield to a
# running gate (cargo-disk-guard Rule 4). Only strays — a per-session server, an editor's own — are paused.
# Read environ without a pipe so grep's early exit cannot trigger SIGPIPE under pipefail.
ra_strays() {
  local pid
  for pid in $(pgrep -f rust-analyzer 2>/dev/null || true); do
    if grep -qxz -- 'ELOHIM_SESSION_ID=lsp-device' "/proc/$pid/environ" 2>/dev/null; then
      continue
    fi
    echo "$pid"
  done
}

RA_PIDS=""
if command -v pgrep >/dev/null 2>&1; then
  RA_PIDS=$(ra_strays)
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
  # Run only when the pushed range touches what brit reads (BRIT_ADVISORY=always forces it):
  # the brit gitlink, brit's own tree, the brit scripts, any build-manifest.json, this hook.
  if [ "${BRIT_ADVISORY:-}" != "always" ] && git rev-parse --verify origin/dev >/dev/null 2>&1; then
    BRIT_BASE="$(git merge-base origin/dev HEAD 2>/dev/null || echo origin/dev)"
    if ! git diff --name-only "$BRIT_BASE" HEAD -- elohim/brit .brit genesis/orchestrator/scripts/brit-helper.sh genesis/orchestrator/scripts/brit-attest.mjs '**/build-manifest.json' .husky 2>/dev/null | grep -q .; then
      echo ""
      echo "── brit advisory (Stage 1a — warn-only): skipped — no brit-relevant path in the pushed range (BRIT_ADVISORY=always to force)"
      return 0
    fi
  fi
  echo ""
  echo "── brit advisory (Stage 1a — warn-only; each call bounded by BRIT_ADVISORY_TIMEOUT_S, default 60s) ─────────────────────"
  REPO_ROOT="$REPO_TOPLEVEL" "$HELPER" verify
  # plan against origin/dev (the merge target); if origin/dev is unreachable, skip.
  if git rev-parse --verify origin/dev >/dev/null 2>&1; then
    REPO_ROOT="$REPO_TOPLEVEL" "$HELPER" plan --since origin/dev || true
  fi
}

if [ -n "$RA_PIDS" ]; then
  RA_COUNT=$(echo "$RA_PIDS" | wc -l | tr -d ' ')
  echo "  Pausing $RA_COUNT rust-analyzer process(es) to free RAM..."
  for pid in $RA_PIDS; do
    kill -STOP "$pid" 2>/dev/null || true
  done
  # Resumed on any exit — normal, error or signal — beside the berth release (one EXIT trap).
  HOOK_ON_EXIT="resume_rust_analyzer $HOOK_ON_EXIT"
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

# Live gate events: every failure-level line a gate prints is appended to a run log on this
# device as it prints, so the session that started the push can begin on the first red while
# later gates still run (`gate-live.mjs follow`, Monitor-shaped). Nothing here decides the push;
# the gates' exit statuses below do. A kill closes the run as interrupted, never passed.
# Spec: genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md
# The close handler is registered before the run opens, so a signal during `open` cannot leave
# a run with no close; it does nothing while no run is open.
GATE_LIVE_RUN=""
gate_live_close_interrupted() {
  [ -n "$GATE_LIVE_RUN" ] || return 0
  node genesis/orchestrator/gate-live.mjs close --run "$GATE_LIVE_RUN" --outcome interrupted >/dev/null 2>&1
  return 0
}
HOOK_ON_EXIT="$HOOK_ON_EXIT gate_live_close_interrupted"
GATE_LIVE_RUN=$(node genesis/orchestrator/gate-live.mjs open --sha "$GATE_COMMIT" --target "$GATE_PUSH_TARGETS" 2>/dev/null) || GATE_LIVE_RUN=""
if [ -n "$GATE_LIVE_RUN" ]; then
  export GATE_LIVE_RUN
  echo "  Live failures: node genesis/orchestrator/gate-live.mjs follow --run $GATE_LIVE_RUN"
  echo ""
fi

TOTAL_START=$(date +%s)
FAILED=""
RESULTS=""
INDEX=0

for PROJECT in $PROJECTS; do
  INDEX=$((INDEX + 1))
  PROJECT_START=$(date +%s)

  # Declares what kind of run this is for the gate-cycle observation (gate-cycle.mjs).
  GATE_RUN_KIND=pre-push node genesis/orchestrator/gate-runner.mjs --target "$PROJECT"
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

# The run describes the gate loop; later steps (cargo-deny, landing) can still refuse the push.
if [ -n "$GATE_LIVE_RUN" ]; then
  node genesis/orchestrator/gate-live.mjs close --run "$GATE_LIVE_RUN" \
    --outcome "$([ -n "$FAILED" ] && echo failed || echo gates-passed)" \
    --deferred-from-log >/dev/null 2>&1 || true
fi

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

# ── Household head: land the pushed epr ─────────────────────────────
# A gate makes an epr build available in the device store. Only a clean, green commit actually
# landing on dev may move the household head; every refusal here is advisory to the push.
# The eprfs gate ingests what it tested (`_gate-eprfs`). When pre-push reused gate evidence
# instead (GATE_EVIDENCE=reuse), the evidence proved this closure was tested but built nothing, so
# `_eprfs-deliver` builds and ingests the pushed commit in the gate's own slot before landing.
BIN_STORE_CLI="${EPR_BIN:-$(command -v epr || true)}"
EPR_LAND_REASON=""
if [ -z "$DEV_LOCAL_SHA" ]; then
  EPR_LAND_REASON="push does not target refs/heads/dev"
fi
if [ -z "$EPR_LAND_REASON" ] && ! echo " $PROJECTS " | grep -q " eprfs "; then
  EPR_LAND_REASON="eprfs was not among the green gates"
fi
# Cleanliness is judged by `epr bin land --tool epr` over epr's own source closure; the main checkout's
# hook ledgers are always dirty and say nothing about the bytes being landed.
if [ -z "$EPR_LAND_REASON" ] && [ "$(git rev-parse HEAD 2>/dev/null)" != "$DEV_LOCAL_SHA" ]; then
  EPR_LAND_REASON="HEAD does not equal the SHA targeting dev"
fi
if [ -z "$EPR_LAND_REASON" ] && [ ! -x "$BIN_STORE_CLI" ]; then
  EPR_LAND_REASON="bin-store is not executable"
fi
epr_candidate_at_sha() {
  "$BIN_STORE_CLI" bin --tool epr status --commit "$DEV_LOCAL_SHA" --json 2>/dev/null |
    python3 -c 'import json,sys; rows=json.load(sys.stdin).get("candidates", []); print(rows[0].get("cid", "") if rows else "")' \
    2>/dev/null || true
}
if [ -z "$EPR_LAND_REASON" ]; then
  EPR_CANDIDATE_CID=$(epr_candidate_at_sha)
  if [ -z "$EPR_CANDIDATE_CID" ]; then
    echo "[pre-push] no epr candidate names $DEV_LOCAL_SHA; building it in the eprfs gate slot"
    GATE_RUN_KIND=pre-push node genesis/orchestrator/gate-runner.mjs --target eprfs --recipe _eprfs-deliver
    EPR_CANDIDATE_CID=$(epr_candidate_at_sha)
  fi
  if [ -z "$EPR_CANDIDATE_CID" ]; then
    EPR_LAND_REASON="no ingested epr candidate names the SHA targeting dev"
  elif EPR_LAND_ERROR=$("$BIN_STORE_CLI" bin --tool epr land "$EPR_CANDIDATE_CID" --commit "$DEV_LOCAL_SHA" --repo "$HOOK_WORK_TREE" 2>&1 >/dev/null); then
    echo "[pre-push] epr household head landed: $EPR_CANDIDATE_CID"
    # The launcher is built beside the landed epr in the gate's slot. A session's PATH execs it,
    # so a changed bin-launch is installed by the epr just landed (install-launcher verifies current).
    EPR_SLOT=$(node genesis/orchestrator/gate-runner.mjs --target eprfs --print 2>/dev/null |
      python3 -c 'import json,sys; print(json.loads(sys.stdin.readline()).get("resolvedTargetDir") or "")' 2>/dev/null || true)
    EPR_STORE=$("$BIN_STORE_CLI" bin --tool epr status --json 2>/dev/null |
      python3 -c 'import json,sys; print(json.load(sys.stdin).get("store") or "")' 2>/dev/null || true)
    EPR_LANDED=$("$BIN_STORE_CLI" bin --tool epr resolve --current 2>/dev/null || true)
    EPR_NEW_LAUNCHER="$EPR_SLOT/debug/bin-launch"
    if [ -z "$EPR_SLOT" ] || [ ! -x "$EPR_NEW_LAUNCHER" ] || [ -z "$EPR_STORE" ] || [ ! -x "$EPR_LANDED" ]; then
      echo "[pre-push] epr launcher not checked: no built bin-launch, store or landed epr to install it with"
    elif cmp -s "$EPR_NEW_LAUNCHER" "$(dirname "$EPR_STORE")/bin/bin-launch"; then
      :
    elif "$EPR_LANDED" bin --tool epr install-launcher --launcher "$EPR_NEW_LAUNCHER" --repo "$HOOK_WORK_TREE" >/dev/null 2>&1; then
      echo "[pre-push] epr launcher installed from $EPR_NEW_LAUNCHER"
    else
      echo "[pre-push] epr launcher not installed: bin-store refused (run: epr bin install-launcher --launcher $EPR_NEW_LAUNCHER)"
    fi
  elif echo "$EPR_LAND_ERROR" | grep -q unpublished; then
    # From a worktree the commit is on neither refs/heads/dev nor origin/dev until this push
    # completes. The candidate is ingested; the next session arrival's `follow` lands it.
    echo "[pre-push] epr household head deferred: $EPR_CANDIDATE_CID lands at the next session arrival once $DEV_LOCAL_SHA is on dev"
  else
    EPR_LAND_REASON="bin-store refused epr candidate health or provenance"
  fi
fi
if [ -n "$EPR_LAND_REASON" ]; then
  echo "[pre-push] epr household head not landed: $EPR_LAND_REASON"
fi
# ── Household head: end ────────────────────────────────────────────

# Stage 1a: advisory brit checks (never fail the push).
brit_advisory

exit 0
