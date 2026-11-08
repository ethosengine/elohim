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
# The whole hook is one brace group, read in full before any of it runs. Bash otherwise reads a
# script from disk as it executes, so an edit to this file during a push — another session in the
# shared main tree, or the pushing session itself — lands its later lines at stale byte offsets:
# 2026-11-01, a 6352 s run with every gate PASSED died at `syntax error near unexpected token )`.
{
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

# A notes ref (refs/notes/brit/* receipts) points at a notes commit whose tree is not a source
# tree: no style reach, no change detection, no gate closure applies. Every leg that iterates
# pushed refs skips it here; branch refs keep their full gating.
is_notes_ref() {
  case "$1" in refs/notes/*) return 0 ;; *) return 1 ;; esac
}

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
# A push to dev or main lands on the branch every device of this person integrates on. It holds the
# steward-scoped `dev-integration` through berth for its LANDING only (Phase L below): the CAS
# window after the gates, never while they run. The gates' verdicts are keyed on the pushed
# closure, so a refusal at the landing loses no check; the next push reuses them. Spec
# 2026-10-23-workspaces-cooperate-as-devices-design.md §3.5; the closure-keyed landing plane
# 2026-10-29, lane β.
#
# The lease is claimed by a LANDING AGENT (phase_l_agent), forked at the landing in its own process
# group with its own hook session (class verify, ttl 600 s, renewed every ttl/3, always
# `--offline-ok`). It is that session's envelope owner from birth until it releases: the release
# comes after `git push` has transmitted, when this hook has exited, and a session envelope honours
# only its own live owner (berth facets never transfer), so no other process could release it. The
# pushing session's own lease is never taken: when that session already holds `dev-integration`,
# the hook renews it as that session and leaves it held.
#
# `--offline-ok` is the rule: when one other device is silent, berth writes the override on the
# record and the agent waits until it takes effect (at most ttl + max_skew_s + fence_margin_s,
# 660 s) before landing. That covers exactly one silent device; with two the push is refused.
#
# A pushed commit carrying `[build:edge]` (or a tag list naming edge, or `[build:all]`) deploys to
# the test fleet, and any `[build:*]` tag dispatches CI. At its landing the hook checks that no
# other holder holds `alpha-fleet` / `jenkins-dispatch` (berth owner-check); it claims neither, so a
# session that watches such a build claims it itself. Every pushed commit's message is read, so
# this checks more than the orchestrator dispatches; over-checking is the safe side. The messages
# are matched with here-strings, never a pipe: under pipefail an early-exiting `grep -q` on more
# than a pipe buffer of messages read as "no tag" (SIGPIPE).
#
# A workspace that is one of its steward's devices but has no `epr` binary is refused every push to
# dev before any gate runs: the lease view would be unreadable at the landing, and an unreadable
# view is never a free resource. A checkout with no berth refuses integration pushes. HUSKY=0
# still bypasses the whole hook.
#
# core.hooksPath is absolute (.husky/hooks-path.mjs), so a push from a linked worktree runs the MAIN
# checkout's copy of this script against the worktree: the hook on dev governs pushes to dev.
HOOK_ON_EXIT=""
HOOK_OWNER_PID=$BASHPID
hook_on_exit() {
  HOOK_EXIT_RC=$? # the hook's exit status, read by handlers that report how the push ended
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
BERTH_TTL_S="${BERTH_PREPUSH_TTL_S:-600}"

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

BERTH_WATCHED=""
if [ -n "$BERTH_TARGETS" ]; then
  if [ ! -x "$BERTH_BIN" ]; then
    echo "[pre-push] REFUSED: this push needs dev-integration; restore executable $BERTH_BIN."
    exit 1
  fi
  # berth's own `no epr` rule, before any gate: a device's lease view is read through epr.
  BERTH_DEVICE_KEY="${ELOHIM_DEVICE_KEY_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/elohim/device/ed25519.seed}"
  BERTH_EPR="${EPR_BIN:-$(command -v epr || true)}"
  if [ -f "$BERTH_DEVICE_KEY" ] && [ ! -x "$BERTH_EPR" ]; then
    echo "[pre-push] REFUSED: epr is required before any gate — run: epr device arrive, or set EPR_BIN=<path>"
    exit 1
  fi
  # The orchestrator's own tag grammar (genesis/orchestrator/commit-tag-parser.mjs): [build:a,b-c].
  if grep -qiE '\[build:([a-z,-]*,)?(edge|all)(,[a-z,-]*)?\]' <<<"$BERTH_MESSAGES"; then
    BERTH_WATCHED="$BERTH_WATCHED alpha-fleet"
  fi
  if grep -qiE '\[build:[a-z,-]+\]' <<<"$BERTH_MESSAGES"; then
    BERTH_WATCHED="$BERTH_WATCHED jenkins-dispatch"
  fi
fi
# ── Berth: claims taken ───────────────────────────────────────────

# ── Phase L: land in the CAS window ───────────────────────────────
#
# Every successful exit of this hook from here on goes through hook_pass. For a push to dev/main
# it then:
#   1. claims dev-integration through the landing agent (or renews the pushing session's own lease
#      and leaves it held);
#   2. checks that no watcher holds alpha-fleet / jenkins-dispatch;
#   3. fetches the remote's tip and lands only as a fast-forward of it, judged on commit ancestry.
#      A moved tip releases the lease, rebases the candidate in place, re-gates it and refuses
#      "push again": one round, never a loop;
#   4. hands the landing to the agent, which waits for the real `git push` to finish, reads the
#      remote back, releases by generation token and appends one line to
#      <git-common-dir>/landing/settle.log.
# The parent talks to the agent through a FIFO (`land <sha> <push pid> <target>` or
# `release <why>`; EOF before `land` means release now) and reads a status file the agent appends
# (`say`/`berth` lines to print, then `held <claimed_at> <claim s>` or `refused <rc>`).
PHASE_L_REMOTE="${1:-origin}"
PHASE_L_STARTED=""
PHASE_L_DONE=""
PHASE_L_SIGNAL=""
PHASE_L_KEPT=""
PHASE_L_DIR=""
PHASE_L_AGENT=""
PHASE_L_FD=""
PHASE_L_LOG=""
PHASE_L_CONSUMED=0
PHASE_L_FINAL=""
PHASE_L_HELD_AT=""
PHASE_L_BASE=""
PHASE_L_LAND_SHA=""
PHASE_L_LAND_TARGET=""

phase_l_refuse() {
  echo "$1"
  PHASE_L_DONE=refused
  exit 1
}

phase_l_nap() {
  sleep "$1" </dev/null >/dev/null 2>&1 &
  wait "$!"
}

# max_skew_s fence_margin_s confirm_wait_s from the steward channel policy (berth's own defaults).
phase_l_channel() {
  node -e '
    let c = {};
    try { c = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).berth.channel || {}; } catch {}
    const n = (v, d) => (Number.isInteger(v) && v >= 0 ? v : d);
    console.log(n(c.max_skew_s, 30), n(c.fence_margin_s, 30), n(c.confirm_wait_s, 10));
  ' "$HOOK_WORK_TREE/genesis/agentic/pool-policy.json" 2>/dev/null || echo "30 30 10"
}

phase_l_rfc3339() {
  date -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ
}

# ---- the landing agent (runs detached; stdout/stderr go to settle.log) ----

phase_l_agent_say() { # kind, text (one status line per text line)
  local kind=$1 line
  while IFS= read -r line; do
    [ -n "$line" ] && printf '%s %s\n' "$kind" "$line" >>"$PHASE_L_STATUS"
  done <<<"$2"
}

phase_l_agent_claim() {
  PHASE_L_OUT=$("$BERTH_BIN" claim dev-integration --class verify --ttl "$BERTH_TTL_S" --offline-ok \
    --note "pre-push to${BERTH_TARGETS} for $PHASE_L_FOR" 2>&1)
}

phase_l_agent_token() {
  if [[ "$PHASE_L_OUT" =~ claimed_at=([0-9]+([.][0-9]+)?) ]]; then
    PHASE_L_CA="${BASH_REMATCH[1]}"
  fi
}

phase_l_agent_overridden() {
  [[ "$PHASE_L_OUT" == *"— override on the record"* ]]
}

# One renew. 0 = still held (a new generation is recorded), 1 = refused outright.
phase_l_agent_renew() {
  local rc
  phase_l_agent_claim
  rc=$?
  if [ "$rc" -eq 0 ]; then
    phase_l_agent_token
    return 0
  fi
  if { [ "$rc" -eq 3 ] && phase_l_agent_overridden; } ||
     { [ "$rc" -eq 1 ] && [[ "$PHASE_L_OUT" == "dev-integration: renewed here, but "* ]]; }; then
    return 0
  fi
  phase_l_agent_say berth "$PHASE_L_OUT"
  phase_l_agent_say say "[pre-push] REFUSED: renewing dev-integration while it waited to land was refused (above). The push stops."
  printf 'lost renew refused\n' >>"$PHASE_L_STATUS"
  return 1
}

# Wait one step for the parent: 0 = nothing came, 2 = it asked or went away (PHASE_L_WHY set).
phase_l_agent_listen() {
  local line rc=0
  read -r -t "$1" -u "$PHASE_L_IN" line || rc=$?
  if [ "$rc" -eq 0 ]; then
    case "$line" in
      land\ *) read -r _ PHASE_L_SHA PHASE_L_PUSH PHASE_L_TARGET <<<"$line"; return 2 ;;
      release\ *) PHASE_L_WHY="${line#release }"; return 2 ;;
    esac
    return 0
  fi
  # A timeout is above 128; EOF (every writer closed: the hook is gone) is 1.
  if [ "$rc" -le 128 ]; then
    PHASE_L_WHY="hook gone"
    return 2
  fi
  [ -z "$PHASE_L_WHY" ] || return 2
  return 0
}

# berth refused the claim with an override on the record: wait until it takes effect, renewing,
# then re-claim (R-K). 0 = held, 2 = the parent asked or went away, else berth's refusal.
phase_l_agent_override_wait() {
  local start now until_s deadline next ms="" rc step
  start=$(date +%s)
  [[ "$PHASE_L_OUT" =~ override_at_ms=([0-9]+) ]] && ms="${BASH_REMATCH[1]}"
  phase_l_agent_token
  until_s=$((start + PHASE_L_MAX))
  if [ -n "$ms" ] && [ $((ms / 1000)) -lt "$until_s" ]; then
    until_s=$((ms / 1000))
  fi
  [ "$until_s" -ge "$start" ] || until_s=$start
  deadline=$((start + PHASE_L_MAX + 2 * (PHASE_L_CW + 1) + 2))
  phase_l_agent_say berth "$PHASE_L_OUT"
  phase_l_agent_say say "[pre-push] Phase L: override on the record; effective at $(phase_l_rfc3339 "$until_s") — waiting $((until_s - start))s"
  if [[ "$PHASE_L_OUT" == *"epr peer serve"* ]]; then
    phase_l_agent_say say "[pre-push] Phase L: this device's peer daemon is down — run: epr peer serve"
  fi
  next=$((start + PHASE_L_INTERVAL))
  while :; do
    now=$(date +%s)
    if [ "$now" -ge "$until_s" ]; then
      if [ $((now + PHASE_L_CW + 1)) -gt "$deadline" ]; then
        PHASE_L_OUT="[pre-push] REFUSED: the override did not take effect within $((now - start))s (one silent sibling is covered; two are not). Nothing was pushed."
        return 3
      fi
      phase_l_agent_claim
      rc=$?
      if [ "$rc" -eq 0 ]; then
        phase_l_agent_token
        return 0
      fi
      phase_l_agent_overridden || return "$rc"
      phase_l_agent_token
      until_s=$((now + 2))
      continue
    fi
    step=$((until_s - now))
    [ "$step" -le 5 ] || step=5
    phase_l_agent_listen "$step"
    [ $? -eq 2 ] && return 2
    if [ "$(date +%s)" -ge "$next" ]; then
      phase_l_agent_renew || return 3
      next=$(($(date +%s) + PHASE_L_INTERVAL))
    fi
  done
}

phase_l_agent_end() { # what happened (the settle line's middle)
  local out released=ok
  if [ -n "$PHASE_L_CA" ]; then
    out=$("$BERTH_BIN" release dev-integration --session "$ELOHIM_SESSION_ID" \
      --expected-claimed-at "$PHASE_L_CA" 2>&1) || released="failed:${out//$'\n'/ }"
  fi
  "$BERTH_BIN" unmoor --session "$ELOHIM_SESSION_ID" >/dev/null 2>&1
  printf '%s %s held=%ss released=%s\n' "$(phase_l_rfc3339 "$(date +%s)")" "$1" \
    "$(($(date +%s) - ${PHASE_L_HELD_FROM:-$(date +%s)}))" "$released"
  # Before a hand-over the hook still reads the verdict from this directory and removes it itself.
  [ -z "$PHASE_L_SHA" ] || rm -rf "$PHASE_L_AGENT_DIR"
}

phase_l_agent() { # dir, session, for whom
  local rc principal out now remote result
  PHASE_L_AGENT_DIR=$1
  PHASE_L_STATUS="$1/status"
  PHASE_L_FOR=$3
  PHASE_L_CA=""
  PHASE_L_WHY=""
  PHASE_L_SHA=""
  PHASE_L_HELD_FROM=""
  trap - EXIT
  trap '' INT HUP
  trap 'PHASE_L_WHY=terminated' TERM
  # The parent's own end of the FIFO is not ours to hold: EOF must mean the parent is gone.
  exec {PHASE_L_FD}>&-
  exec {PHASE_L_IN}<"$1/fifo"
  # This process is the envelope owner of the session it holds the lease with (see Berth block).
  export ELOHIM_SESSION_ID="$2" ELOHIM_SESSION_OWNER_PID="$BASHPID"
  read -r PHASE_L_SKEW PHASE_L_MARGIN PHASE_L_CW <<<"$(phase_l_channel)"
  PHASE_L_MAX="${BERTH_PREPUSH_OVERRIDE_MAX_S:-$((BERTH_TTL_S + PHASE_L_SKEW + PHASE_L_MARGIN))}"
  PHASE_L_INTERVAL=$((BERTH_TTL_S / 3))
  [ "$PHASE_L_INTERVAL" -ge 1 ] || PHASE_L_INTERVAL=1
  principal=$(git config user.email || git config user.name || id -un)
  # $BASHPID inside $(…) would name the substitution's subshell, not this agent.
  if ! out=$("$BERTH_BIN" moor --session "$2" --pid "$ELOHIM_SESSION_OWNER_PID" --runtime git-hook \
      --principal "$principal" --task "landing for $3" 2>&1); then
    phase_l_agent_say berth "$out"
    printf 'refused 4\n' >>"$PHASE_L_STATUS"
    phase_l_agent_end "refused moor"
    return
  fi
  now=$(date +%s)
  phase_l_agent_claim
  rc=$?
  if [ "$rc" -eq 3 ] && phase_l_agent_overridden; then
    phase_l_agent_override_wait
    rc=$?
    if [ "$rc" -eq 2 ]; then
      printf 'refused 130\n' >>"$PHASE_L_STATUS"
      phase_l_agent_end "release ${PHASE_L_WHY} (during the override wait)"
      return
    fi
  fi
  if [ "$rc" -ne 0 ]; then
    phase_l_agent_say berth "$PHASE_L_OUT"
    printf 'refused %s\n' "$rc" >>"$PHASE_L_STATUS"
    phase_l_agent_end "refused claim rc=$rc"
    return
  fi
  phase_l_agent_token
  phase_l_agent_say berth "$PHASE_L_OUT"
  PHASE_L_HELD_FROM=$(date +%s)
  printf 'held %s %s\n' "$PHASE_L_CA" "$((PHASE_L_HELD_FROM - now))" >>"$PHASE_L_STATUS"
  # Held: renew while the parent checks, until it hands the landing over or lets go.
  local next=$((PHASE_L_HELD_FROM + PHASE_L_INTERVAL))
  while :; do
    phase_l_agent_listen 1
    [ $? -eq 2 ] && break
    if [ "$(date +%s)" -ge "$next" ]; then
      phase_l_agent_renew || { PHASE_L_WHY="renew refused"; break; }
      next=$(($(date +%s) + PHASE_L_INTERVAL))
    fi
  done
  if [ -z "$PHASE_L_SHA" ]; then
    phase_l_agent_end "release ${PHASE_L_WHY}"
    return
  fi
  # Handed over: wait for the real `git push` to finish (bounded at an hour), still renewing.
  local stop=$(($(date +%s) + 3600))
  while kill -0 "$PHASE_L_PUSH" 2>/dev/null && [ "$(date +%s)" -lt "$stop" ] && [ -z "$PHASE_L_WHY" ]; do
    sleep 0.5
    if [ "$(date +%s)" -ge "$next" ]; then
      phase_l_agent_renew || break
      next=$(($(date +%s) + PHASE_L_INTERVAL))
    fi
  done
  remote=$(timeout 30 git ls-remote "$PHASE_L_REMOTE" "refs/heads/$PHASE_L_TARGET" 2>/dev/null | cut -f1)
  if { [ "$PHASE_L_SHA" = none ] && [ -z "$remote" ]; } || [ "$remote" = "$PHASE_L_SHA" ]; then
    result=landed
  else
    result=did-not-land
  fi
  phase_l_agent_end "settle $PHASE_L_SHA $result $PHASE_L_TARGET=${remote:0:12}"
}

# ---- the hook's side ----

phase_l_await() { # print the agent's lines until its verdict; PHASE_L_FINAL = that verdict
  local lines i alive bound
  bound=$(($(date +%s) + BERTH_TTL_S + 2 * (PHASE_L_CW + 1) + 120))
  while :; do
    alive=1
    kill -0 "$PHASE_L_AGENT" 2>/dev/null || alive=""
    mapfile -t lines <"$PHASE_L_DIR/status" 2>/dev/null || lines=()
    for ((i = PHASE_L_CONSUMED; i < ${#lines[@]}; i++)); do
      PHASE_L_CONSUMED=$((i + 1))
      case "${lines[i]}" in
        say\ *) printf '%s\n' "${lines[i]#say }" ;;
        berth\ *) printf '%s\n' "${lines[i]#berth }" >&2 ;;
        *) PHASE_L_FINAL="${lines[i]}"; return 0 ;;
      esac
    done
    [ -n "$alive" ] || { PHASE_L_FINAL="refused agent-exited"; return 0; }
    [ "$(date +%s)" -lt "$bound" ] || { PHASE_L_FINAL="refused no-verdict"; return 0; }
    phase_l_nap 0.2
  done
}

phase_l_reap() {
  local i
  for ((i = 0; i < 150; i++)); do
    kill -0 "$PHASE_L_AGENT" 2>/dev/null || return 0
    sleep 0.1
  done
  echo "[pre-push] Phase L: the landing agent did not finish releasing within 15s; see $PHASE_L_LOG" >&2
}

phase_l_release_now() { # why
  [ -n "$PHASE_L_AGENT" ] || return 0
  printf 'release %s\n' "$1" >&"$PHASE_L_FD" 2>/dev/null
  phase_l_reap
  PHASE_L_AGENT=""
}

phase_l_claim() {
  local show kept whom out rc session
  # The pushing session's own lease: renewed as that session and left held.
  show=$("$BERTH_BIN" session-show 2>/dev/null) || show=""
  read -r kept whom < <(node -e '
    let j = {};
    try { j = JSON.parse(require("fs").readFileSync(0, "utf8")); } catch {}
    const held = j && j.leases && j.leases["dev-integration"] ? 1 : 0;
    console.log(held, (j && j.mooring && j.mooring.session) || "-");
  ' <<<"$show" 2>/dev/null || echo "0 -")
  if [ "${kept:-0}" = 1 ]; then
    out=$("$BERTH_BIN" claim dev-integration --class verify --ttl "$BERTH_TTL_S" --offline-ok \
      --note "pre-push to${BERTH_TARGETS}" 2>&1)
    rc=$?
    [ -n "$out" ] && printf '%s\n' "$out" >&2
    if [ "$rc" -eq 0 ] || { [ "$rc" -eq 1 ] && grep -q '^dev-integration: renewed here, but ' <<<"$out"; }; then
      PHASE_L_KEPT=1
      echo "[pre-push] Phase L: dev-integration claimed by this session already (renewed); it stays held after the push — release it when done: berth release dev-integration"
      return 0
    fi
    phase_l_refuse "[pre-push] REFUSED: this push needs dev-integration (reason above). Your gates are recorded; push again when it is free."
  fi
  [ "${whom:--}" != - ] || whom=$(git config user.email || git config user.name || id -un)
  if ! PHASE_L_DIR=$(mktemp -d) || ! mkfifo "$PHASE_L_DIR/fifo" || ! : >"$PHASE_L_DIR/status"; then
    phase_l_refuse "[pre-push] REFUSED: could not prepare the landing agent's channel (mktemp/mkfifo). Your gates are recorded."
  fi
  exec {PHASE_L_FD}<>"$PHASE_L_DIR/fifo"
  session="pre-push-${HOOK_OWNER_PID}-${EPOCHREALTIME//./}-$RANDOM"
  set -m
  phase_l_agent "$PHASE_L_DIR" "$session" "$whom" </dev/null >>"$PHASE_L_LOG" 2>&1 &
  PHASE_L_AGENT=$!
  set +m
  phase_l_await
  case "$PHASE_L_FINAL" in
    held\ *)
      local claim_s
      read -r _ _ claim_s <<<"$PHASE_L_FINAL"
      PHASE_L_HELD_AT=$(date +%s)
      echo "[pre-push] Phase L: dev-integration claimed in ${claim_s}s by $session for $whom (ttl ${BERTH_TTL_S}s)"
      ;;
    "refused 3")
      phase_l_refuse "[pre-push] REFUSED: this push needs dev-integration (reason above). Your gates are recorded; push again when it is free."
      ;;
    "refused 4")
      phase_l_refuse "[pre-push] REFUSED: the landing agent could not moor its session (above); repair berth's session/store error. Your gates are recorded."
      ;;
    *)
      phase_l_refuse "[pre-push] REFUSED: berth claim dev-integration failed (${PHASE_L_FINAL#refused }, above). Your gates are recorded. Bypass all gates: HUSKY=0 git push"
      ;;
  esac
}

phase_l_owner_checks() {
  local res out rc
  for res in $BERTH_WATCHED; do
    out=$("$BERTH_BIN" owner-check "$res" --action "push to dev" 2>&1)
    rc=$?
    [ -n "$out" ] && printf '%s\n' "$out" >&2
    case "$rc" in
      0) ;;
      3) phase_l_refuse "[pre-push] REFUSED: $res is held by a watcher (above); dev-integration is released. Nothing was pushed." ;;
      *) phase_l_refuse "[pre-push] REFUSED: berth owner-check $res exited $rc (above); repair berth's session/store error. dev-integration is released. Nothing was pushed." ;;
    esac
  done
}

phase_l_fetch_base() { # target, the base git sent
  local err
  if err=$(timeout 60 git -c gc.auto=0 -c maintenance.auto=false fetch --quiet --no-tags \
      --no-recurse-submodules "$PHASE_L_REMOTE" "refs/heads/$1" 2>&1) &&
     PHASE_L_BASE=$(git rev-parse --verify -q FETCH_HEAD); then
    return 0
  fi
  echo "[pre-push] Phase L: $PHASE_L_REMOTE unreadable (${err//$'\n'/ }); landing on the base git sent — the server's fast-forward check is the last word"
  PHASE_L_BASE=$2
}

phase_l_moved() { # target, pushed sha, the base git sent
  local new=$PHASE_L_BASE head upstream out files p rc tmp reused=0 ran=0 failed=""
  [ "$1" = dev ] ||
    phase_l_refuse "[pre-push] REFUSED: $1 moved to ${new:0:12} since this push was prepared; rebase onto it and push again"
  phase_l_release_now moved
  head=$(git rev-parse HEAD)
  [ "$head" = "$2" ] ||
    phase_l_refuse "[pre-push] REFUSED: dev moved to ${new:0:12}; ${2:0:12} is not checked out here — rebase it onto ${new:0:12} and push again"
  # git refreshes the base it sends from the remote's advertisement, so it may already be the moved
  # tip: rebase from where the candidate actually forked.
  upstream=$3
  if [ "$upstream" = "$ZERO_SHA" ] || ! git merge-base --is-ancestor "$upstream" HEAD 2>/dev/null; then
    upstream=$(git merge-base "$new" HEAD)
  fi
  echo "[pre-push] Phase L: dev moved ${upstream:0:12} → ${new:0:12}; released dev-integration, rebasing in place"
  if ! out=$(git rebase --autostash --onto "$new" "$upstream" 2>&1); then
    printf '%s\n' "$out"
    files=$(git diff --name-only --diff-filter=U 2>/dev/null | tr '\n' ' ')
    git rebase --abort >/dev/null 2>&1
    phase_l_refuse "[pre-push] REFUSED: dev moved to ${new:0:12} and ${2:0:12} does not rebase onto it cleanly (${files:-conflicts above}); rebase by hand and push again"
  fi
  head=$(git rev-parse HEAD)
  export GATE_COMMIT="$head"
  unset GATE_EVIDENCE_RECORD
  [ -z "${GATE_CHANGED_FILES-}" ] || git diff --name-only "$new" HEAD >"$GATE_CHANGED_FILES"
  export GATE_NARROW_BASE=ok
  tmp=$(mktemp)
  for p in ${PROJECTS-}; do
    (cd "$HOOK_WORK_TREE" && GATE_RUN_KIND=pre-push node genesis/orchestrator/gate-runner.mjs --target "$p") 2>&1 |
      tee "$tmp"
    rc=${PIPESTATUS[0]}
    if [ "$rc" -ne 0 ]; then
      failed="$failed $p"
    elif grep -q 'reused' "$tmp"; then
      reused=$((reused + 1))
    else
      ran=$((ran + 1))
    fi
  done
  rm -f "$tmp"
  if [ -n "$failed" ]; then
    phase_l_refuse "[pre-push] REFUSED: candidate rebased to ${head:0:12} is red on the moved base:$failed"
  fi
  echo "[pre-push] Phase L: rebased to ${head:0:12} on ${new:0:12}: $reused reused, $ran ran"
  echo "[pre-push] REFUSED: candidate moved to $head; push again"
  PHASE_L_DONE=moved
  exit 1
}

phase_l_push_pid() {
  if [ -n "${PHASE_L_PUSH_PID-}" ]; then
    echo "$PHASE_L_PUSH_PID"
    return
  fi
  # git → husky's `_/pre-push` sh → `sh -e .husky/pre-push` → exec bash: $PPID exits with the hook.
  local pid=$PPID i
  for ((i = 0; i < 8; i++)); do
    [ -r "/proc/$pid/comm" ] || break
    if [ "$(cat "/proc/$pid/comm")" = git ]; then
      echo "$pid"
      return
    fi
    pid=$(awk '/^PPid:/ {print $2}' "/proc/$pid/status" 2>/dev/null)
    [ -n "$pid" ] && [ "$pid" != 0 ] || break
  done
  echo "[pre-push] Phase L: no git ancestor found; the landing agent waits on $PPID" >&2
  echo "$PPID"
}

phase_l_handoff() {
  if [ -n "$PHASE_L_KEPT" ]; then
    PHASE_L_DONE=kept
    echo "[pre-push] Phase L: landing ${PHASE_L_LAND_SHA:0:12} on ${PHASE_L_BASE:0:12} as a fast-forward; dev-integration stays held by this session"
    return 0
  fi
  if grep -q '^lost ' "$PHASE_L_DIR/status" 2>/dev/null; then
    phase_l_await
    phase_l_refuse "[pre-push] REFUSED: dev-integration was lost while this push waited to land (above). Your gates are recorded; push again."
  fi
  printf 'land %s %s %s\n' "${PHASE_L_LAND_SHA:-none}" "$(phase_l_push_pid)" "$PHASE_L_LAND_TARGET" >&"$PHASE_L_FD"
  PHASE_L_DONE=handed
  echo "[pre-push] Phase L: dev-integration held $(($(date +%s) - PHASE_L_HELD_AT))s"
  echo "[pre-push] Phase L: landing ${PHASE_L_LAND_SHA:0:12} on ${PHASE_L_BASE:0:12} as a fast-forward; the landing agent releases dev-integration after the transfer (log: $PHASE_L_LOG)"
}

hook_pass() {
  [ -n "$BERTH_TARGETS" ] || exit 0
  local line local_sha remote_ref remote_sha target rc
  PHASE_L_STARTED=1
  trap 'PHASE_L_SIGNAL=INT; exit 130' INT
  trap 'PHASE_L_SIGNAL=TERM; exit 143' TERM
  trap 'PHASE_L_SIGNAL=HUP; exit 129' HUP
  PHASE_L_LOG="$(git rev-parse --path-format=absolute --git-common-dir)/landing/settle.log"
  mkdir -p "$(dirname "$PHASE_L_LOG")" ||
    phase_l_refuse "[pre-push] REFUSED: cannot create $(dirname "$PHASE_L_LOG"). Your gates are recorded."
  read -r _ _ PHASE_L_CW <<<"$(phase_l_channel)"
  echo "[pre-push] Phase L: claiming dev-integration (ttl ${BERTH_TTL_S}s)"
  phase_l_claim
  phase_l_owner_checks
  for line in "${PUSH_LINES[@]}"; do
    read -r _ local_sha remote_ref remote_sha <<<"$line"
    case "$remote_ref" in
      refs/heads/dev | refs/heads/main) ;;
      *) continue ;;
    esac
    target=${remote_ref#refs/heads/}
    if [ -z "$PHASE_L_LAND_TARGET" ] || [ "$target" = dev ]; then
      PHASE_L_LAND_TARGET=$target
      PHASE_L_LAND_SHA=$local_sha
      [ "$local_sha" != "$ZERO_SHA" ] || PHASE_L_LAND_SHA=none
    fi
    phase_l_fetch_base "$target" "$remote_sha"
    [ "$local_sha" != "$ZERO_SHA" ] && [ "$PHASE_L_BASE" != "$ZERO_SHA" ] || continue
    # Fast-forward is judged on commit oids (R-D). Only a definite "not an ancestor" is a move;
    # an unknown base leaves the verdict to the server.
    git merge-base --is-ancestor "$PHASE_L_BASE" "$local_sha" 2>/dev/null
    rc=$?
    [ "$rc" -ne 1 ] || phase_l_moved "$target" "$local_sha" "$remote_sha"
  done
  phase_l_handoff
  exit 0
}

phase_l_on_exit() {
  [ -n "$PHASE_L_STARTED" ] || return 0
  if [ -n "$PHASE_L_AGENT" ] && [ "$PHASE_L_DONE" != handed ]; then
    phase_l_release_now "${PHASE_L_DONE:-${PHASE_L_SIGNAL:-exit}}"
  fi
  if [ -z "$PHASE_L_DONE" ]; then
    echo "[pre-push] interrupted at the landing (${PHASE_L_SIGNAL:-exit}); nothing was pushed — resume: git push $PHASE_L_REMOTE HEAD:dev" >&2
  fi
  if [ -n "$PHASE_L_DIR" ] && [ "$PHASE_L_DONE" != handed ]; then
    rm -rf "$PHASE_L_DIR"
  fi
}
HOOK_ON_EXIT="phase_l_on_exit $HOOK_ON_EXIT"
# ── Phase L: end ──────────────────────────────────────────────────

# The object this push is judged by (the dev ref's sha, else the last ref's) and its targets, read
# before any check so the live run can open first; GATE_COMMIT below is derived the same way.
LIVE_SHA=""
LIVE_DEV_SHA=""
LIVE_TARGETS=""
for PUSH_LINE in "${PUSH_LINES[@]}"; do
  read -r _ L_SHA L_REF _ <<<"$PUSH_LINE"
  is_notes_ref "$L_REF" && continue # reported once, by the change-detection loop below
  [ "$L_SHA" = "$ZERO_SHA" ] && continue
  LIVE_TARGETS="${LIVE_TARGETS:+$LIVE_TARGETS }$L_REF"
  LIVE_SHA="$L_SHA"
  [ "$L_REF" = refs/heads/dev ] && LIVE_DEV_SHA="$L_SHA"
done
LIVE_SHA="${LIVE_DEV_SHA:-$LIVE_SHA}"

# Live gate events: every red this push raises — a check below, the preflight, or a line a gate
# prints — is appended to a run log on this device as it happens, so the session that started the
# push hears it at its next tool call (`gate-live-inject`) or wakes on it (`gate-live.mjs follow`,
# Monitor-shaped) while the rest of the push keeps reporting. Opened before the first check, so a
# seconds-cheap red is live in seconds. Nothing here decides the push; exit statuses do. The run
# closes by how the hook ends: passed, failed, or interrupted on a signal, never passed by default.
# Spec: genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md
# The close handler is registered before the run opens, so a signal during `open` cannot leave
# a run with no close; it does nothing while no run is open.
GATE_LIVE_RUN=""
gate_live_close_on_exit() {
  [ -n "$GATE_LIVE_RUN" ] || return 0
  local outcome=failed
  case "${HOOK_EXIT_RC:-1}" in
    0) outcome=passed ;;
    129 | 130 | 143) outcome=interrupted ;;
  esac
  node genesis/orchestrator/gate-live.mjs close --run "$GATE_LIVE_RUN" --outcome "$outcome" >/dev/null 2>&1
  # The integrator's harvest: recurring and late cheap reds across this device's recent runs.
  [ "$outcome" = interrupted ] || node genesis/orchestrator/gate-live.mjs harvest --last 10 2>/dev/null
  return 0
}
HOOK_ON_EXIT="$HOOK_ON_EXIT gate_live_close_on_exit"
GATE_LIVE_RUN=$(node genesis/orchestrator/gate-live.mjs open --sha "$LIVE_SHA" --target "$LIVE_TARGETS" 2>/dev/null) || GATE_LIVE_RUN=""
if [ -n "$GATE_LIVE_RUN" ]; then
  export GATE_LIVE_RUN
  echo "  Live failures: node genesis/orchestrator/gate-live.mjs follow --run $GATE_LIVE_RUN"
  echo "  Stop after the running gate: node genesis/orchestrator/gate-live.mjs stop --run $GATE_LIVE_RUN"
  echo ""
fi
# ── Live run open

# A red from a check is recorded and live at once; whether the push then stops is the
# integrator's call (`gate-live.mjs stop`) or the declared policy: GATE_ON_RED=continue|stop,
# default from genesis/agentic/pool-policy.json gate_on_red. Under continue every later check
# and gate still runs and reports, and the push is refused at the end.
GATE_ON_RED_MODE=$(node genesis/orchestrator/gate-preflight.mjs --on-red 2>/dev/null || echo continue)
PREGATE_FAILED=""
leg_red() {
  local leg="$1"
  shift
  echo "[pre-push] RED $leg: $*" >&2
  PREGATE_FAILED="$PREGATE_FAILED $leg"
  if [ -n "$GATE_LIVE_RUN" ]; then
    node genesis/orchestrator/gate-live.mjs red --run "$GATE_LIVE_RUN" --project "$leg" --message "$*" >/dev/null 2>&1 || true
    # Once per run: what letting the push continue will cost, and how to stop it.
    node genesis/orchestrator/gate-live.mjs outlook --run "$GATE_LIVE_RUN" --remaining "${PROJECTS-}" >/dev/null 2>&1 || true
  fi
  if [ "$GATE_ON_RED_MODE" = stop ]; then
    echo "[pre-push] GATE_ON_RED=stop — the push stops at its first red." >&2
    exit 1
  fi
  return 0
}
gate_stop_requested() {
  [ -n "$GATE_LIVE_RUN" ] && node genesis/orchestrator/gate-live.mjs stopped --run "$GATE_LIVE_RUN" >/dev/null 2>&1
}
# Every early pass below goes through here: a red an earlier check recorded still refuses.
pass_or_refuse() {
  if [ -n "$PREGATE_FAILED" ]; then
    echo "[pre-push] REFUSED: red before the gates:$PREGATE_FAILED (above)." >&2
    exit 1
  fi
  hook_pass
}
# ── Red recorder end

CHANGED=""
UNFILTERED_CHANGED=""
CHANGE_UNREADABLE=""
GATE_NARROW_BASE=ok
PUSH_TARGETS=""
LAST_LOCAL_SHA=""
DEV_LOCAL_SHA=""
for PUSH_LINE in "${PUSH_LINES[@]}"; do
  read -r LOCAL_REF LOCAL_SHA REMOTE_REF REMOTE_SHA <<<"$PUSH_LINE"
  if is_notes_ref "$REMOTE_REF"; then
    echo "notes: $REMOTE_REF skipped (not a commit tree)"
    continue
  fi
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
    leg_red rust-style "the style reach gate refused $REMOTE_REF (above)"
  fi

  # Determine what changed. A new branch, or a remote tip this clone has never fetched, is read
  # against its merge base with origin/dev: the whole branch, of the object being pushed. A range
  # that cannot be read at all refuses below; an unreadable diff never reads as "nothing changed".
  if [ "$REMOTE_SHA" = "$ZERO_SHA" ]; then
    GATE_NARROW_BASE=none
    if ! NEW_CHANGED=$(git diff --name-only "origin/dev...$LOCAL_SHA" 2>/dev/null); then
      CHANGE_UNREADABLE="$CHANGE_UNREADABLE $REMOTE_REF"
      NEW_CHANGED=""
    fi
  else
    if ! NEW_CHANGED=$(git diff --name-only "$REMOTE_SHA" "$LOCAL_SHA" 2>/dev/null); then
      GATE_NARROW_BASE=none
      if ! NEW_CHANGED=$(git diff --name-only "origin/dev...$LOCAL_SHA" 2>/dev/null); then
        CHANGE_UNREADABLE="$CHANGE_UNREADABLE $REMOTE_REF"
        NEW_CHANGED=""
      fi
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
# R-I: every gate may stand on this workspace's own green evidence for the same closure.
export GATE_EVIDENCE="${GATE_EVIDENCE:-reuse}"
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

if [ -n "$CHANGE_UNREADABLE" ]; then
  echo "[pre-push] REFUSED: could not read what changed for${CHANGE_UNREADABLE} (neither the remote tip nor origin/dev...<pushed sha> diffs). Run: git fetch origin" >&2
  exit 1
fi

# Nothing changed — let it through (a style refusal still refuses)
if [ -z "$CHANGED" ]; then
  pass_or_refuse
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
      leg_red package-projections "projection drift; pnpm run elohim-agent:packages:project"
    else
      echo "[pre-push] elohim-agent package projections ✓"
    fi
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
    hook_tests_red=0
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
        leg_red hook-boundary-tests "$hook_test failed"
        hook_tests_red=1
      fi
    done
    # A glob that matches nothing must not print a success line — a renamed or deleted
    # suite would otherwise read as "gate passed" forever, which is the same
    # verification-that-never-runs failure this gate exists to close.
    if [ "$hook_tests_ran" -eq 0 ]; then
      echo "[pre-push] ERROR: .claude/hooks changed but no boundary tests were found."
      echo "  Expected at least one .claude/hooks/tests/test_*.py"
      leg_red hook-boundary-tests "no boundary tests found"
    elif [ "$hook_tests_red" -eq 0 ]; then
      echo "[pre-push] Claude hook boundary tests ✓"
    fi
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
      leg_red habit-register "habits.yaml stale or census invalid; python3 .claude/scripts/habits-project.py"
    else
      echo "[pre-push] habit register freshness ✓"
    fi
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
      leg_red agentic-library-harnesses "$(printf '%s ' $lib_failed)"
    # A glob that matches nothing must not print a success line — same guard, same reason as the
    # hook-boundary leg above.
    elif [ "$lib_tests_ran" -eq 0 ]; then
      echo "[pre-push] ERROR: .claude/scripts changed but no library harnesses were found."
      echo "  Expected at least one .claude/scripts/_lib/__tests__/*_test.py"
      leg_red agentic-library-harnesses "no library harnesses found"
    else
      echo "[pre-push] agentic-library harnesses ✓ ($lib_tests_ran green)"
    fi
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
    pass_or_refuse
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
  pnpm run elohim-agent:test || leg_red elohim-agent-test "pnpm run elohim-agent:test failed"
  pass_or_refuse
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
  # Cheapest first (recorded p90): a red that takes seconds to find never waits behind an hour.
  PROJECTS=$(echo "$CHANGED" | node genesis/orchestrator/gate-runner.mjs --changed-file-list --names --order cost | tr '\n' ' ')
  [ $? -eq 0 ] || { echo "[pre-push] manifest gate selection failed (see error above)"; exit 1; }
else
  echo "[pre-push] node unavailable — manifest gate selection skipped (CI backstop)."
fi

# No grep fallback: build manifests are the sole positive gate detector.
# Trim leading space
PROJECTS=$(echo "$PROJECTS" | sed 's/^ //')

# ── Preflight (seconds) ──────────────────────────────────────────
# The careless misses that have each cost a long gate run: an uninitialised submodule, an
# unbuilt workspace dist, a Rust read the gate key cannot name, a dirty closure that will record
# no evidence. One line per check, each REFUSED/WARN naming its cure, every REFUSED live at once.
# gate-runner runs the same checks for a standalone `just gate`; GATE_PREFLIGHT_DONE skips them
# per gate here. Run on the selection before the PVC and sweettest filters (a superset) and before
# the slower legs below, so its reds are live first. cargo-deny bans lives here now, and is not
# optional: there is no switch that skips this block.
if [ -n "$PROJECTS" ]; then
  # shellcheck disable=SC2086 # PROJECTS is a word list of manifest keys
  if ! node genesis/orchestrator/gate-preflight.mjs --projects $PROJECTS; then
    leg_red preflight "a seconds check refused (above); its cure is printed beside it"
  fi
fi
export GATE_PREFLIGHT_DONE=1

# ── Humans/Presences Schema Validation + Freshness ───────────────
#
# When genesis/data/humans/ or genesis/data/presences/ markdown changes,
# validate schemas and ensure the generated humans.json / presences.json
# artifacts are fresh relative to their markdown sources.
if echo "$CHANGED" | grep -qE "genesis/data/humans/|genesis/data/presences/"; then
  echo "[pre-push] Validating humans and presences schemas..."
  humans_red=""
  (cd genesis/seeder && pnpm run validate:humans) || humans_red="validate:humans"
  (cd genesis/seeder && pnpm run validate:presences) || humans_red="$humans_red validate:presences"

  echo "[pre-push] Regenerating humans.json / presences.json from markdown sources..."
  (cd genesis/seeder && pnpm run build:data) || humans_red="$humans_red build:data"

  if ! git diff --quiet -- genesis/data/humans/humans.json genesis/data/presences/presences.json 2>/dev/null; then
    echo "[pre-push] ERROR: humans.json or presences.json is stale relative to markdown sources."
    echo "  Re-run '(cd genesis/seeder && pnpm run build:data)' and stage the changes before pushing."
    humans_red="$humans_red stale-json"
  fi
  if [ -n "$humans_red" ]; then
    leg_red humans-presences "$humans_red"
  else
    echo "[pre-push] humans/presences validation ✓"
  fi
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
    leg_red cid-artifacts "a CID twin drifted from its source .md; (cd genesis/seeder && pnpm run sync:cid-artifacts)"
  else
    echo "[pre-push] CID-artifact freshness ✓"
  fi
fi

# ── Device Archetype Validation ──────────────────────────────────
#
# When device archetype markdown files change, validate frontmatter
# against devices.schema.json and regenerate devices.json.
if echo "$CHANGED" | grep -qE "genesis/data/devices/"; then
  echo "[pre-push] Validating device archetypes..."
  devices_red=""
  (cd genesis/seeder && pnpm run validate:devices) || devices_red="validate:devices"

  echo "[pre-push] Regenerating devices.json from markdown sources..."
  (cd genesis/seeder && pnpm run generate:devices) || devices_red="$devices_red generate:devices"

  if ! git diff --quiet -- genesis/data/devices/devices.json 2>/dev/null; then
    echo "[pre-push] ERROR: devices.json is stale relative to markdown sources."
    echo "  Re-run 'pnpm --filter holochain-seeder run generate:devices' and stage the changes before pushing."
    devices_red="$devices_red stale-json"
  fi
  if [ -n "$devices_red" ]; then
    leg_red device-archetypes "$devices_red"
  else
    echo "[pre-push] device archetype validation ✓"
  fi
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
  if (cd genesis/seeder && pnpm run validate:deployments); then
    echo "[pre-push] deployments resource-conformance ✓"
  else
    leg_red deployments-conformance "pnpm run validate:deployments failed"
  fi
fi

# Runtime-manifest pin/render conformance. 2026-09-05: every active human is
# pinned, so Unpinned is now REFUSE (exit 65), alongside CID/resource drift.
# Use the same pool resolver as native gate.projects; no prebuilt binary needed.
if echo "$CHANGED" | grep -qE "bridges/k8s/|elohim/ark/core/|genesis/orchestrator/data/deployments\.json|genesis/orchestrator/manifests/runtime/|genesis/data/devices/archetype-resource-budgets\.json|scripts/ci/conductor-split-budget\.sh"; then
  echo "[pre-push] Verifying runtime manifest pins and rendered resources..."
  if (
    k8s_repo_root="$(git rev-parse --show-toplevel)"
    source "$k8s_repo_root/genesis/agentic/bin/pool-lib.sh"
    k8s_pool_family="$(detect_family "$k8s_repo_root")"
    export CARGO_TARGET_DIR="$(slot_path "$k8s_pool_family" k8s-bridge dev)"
    export CARGO_BUILD_JOBS=2 RUSTFLAGS="" RUSTC_WRAPPER=""
    cargo run -q --manifest-path bridges/k8s/Cargo.toml --bin k8s-bridge -- verify --deployments genesis/orchestrator/data/deployments.json --manifests-dir genesis/orchestrator/manifests/runtime
  ); then
    echo "[pre-push] runtime manifest pin/render conformance ✓"
  else
    leg_red runtime-manifest-pins "k8s-bridge verify refused (above)"
  fi
fi


# ── Account Package Schema Validation ────────────────────────────
#
# When account packages or their source data change, validate the
# generated JSON against account-package.schema.json. Catches field
# drift (e.g. missing relationshipType) before it reaches CI/deploy.
if echo "$CHANGED" | grep -qE "genesis/data/account-packages/|genesis/data/humans/|genesis/seeder/src/account-package\.ts"; then
  echo "[pre-push] Validating account packages against schema..."
  if (cd genesis/seeder && pnpm run validate:account-packages); then
    echo "[pre-push] account-packages validation ✓"
  else
    leg_red account-packages "pnpm run validate:account-packages failed"
  fi
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
      leg_red ci-ignore-freshness ".ci-ignore stale; python3 .claude/scripts/ci-ignore-projector.py"
    else
      echo "[pre-push] .ci-ignore freshness ✓"
    fi
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
  # The pushed object, not the checkout's HEAD: HEAD can differ (an explicit sha push) or move mid-push.
  RANGE_HEAD="${GATE_COMMIT:-HEAD}"
  RANGE_BASE=$(git merge-base origin/dev "$RANGE_HEAD" 2>/dev/null || echo "$RANGE_HEAD~1")
  if ! python3 .claude/scripts/epr-meta-git-gate.py --range "${RANGE_BASE}..${RANGE_HEAD}"; then
    echo "[pre-push] ERROR: .epr-meta compose-gate rejected a change in this push range."
    echo "  Acknowledge (ask-class): EPR_META_ACK=1 git push   |   bypass all: git push --no-verify"
    leg_red epr-meta-compose "the compose-gate rejected a change in this push range"
  else
    echo "[pre-push] .epr-meta compose-gate ✓"
  fi
fi

# ── T2 receipt (mandatory for serving paths) ───────────────────────
# The receipt script owns policy: serving paths require a complete current-source
# household proof; other dataplane paths warn unless T2_RECEIPT=strict.
# Advisory misses already exit 0. Propagate every nonzero result so a mandatory
# refusal or an unreadable gate can never become a successful push here.
T2_LIST=$(mktemp)
printf '%s\n' "$CHANGED" > "$T2_LIST"
T2_STATUS=0
if [ "${T2_RECEIPT-}" = "strict" ]; then
  bash genesis/orchestrator/scripts/t2-receipt.sh --changed "$T2_LIST" --strict || T2_STATUS=$?
else
  bash genesis/orchestrator/scripts/t2-receipt.sh --changed "$T2_LIST" || T2_STATUS=$?
fi
rm -f "$T2_LIST"
[ "$T2_STATUS" -eq 0 ] || leg_red t2-receipt "household serving receipt refused (exit $T2_STATUS, above)"

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
  pass_or_refuse
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
  pass_or_refuse
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
  # KNOWN GAP, accepted for a warn-only stage: `brit verify` checks the integrity of the WHOLE
  # repository, so an ordinary commit that rewrites history or corrupts the object store gets no
  # Stage 1a signal until a brit-relevant path changes. Stage 2 (the stage that makes this leg
  # blocking) must drop the path gate or run verify on every push within its bound — do not
  # inherit this gate unknowingly. (steward-space-reliability-plan, 2026-10-29)
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

TOTAL_START=$(date +%s)
FAILED=""
RESULTS=""
INDEX=0

NOT_RUN=""
STOP_REASON=""
# shellcheck disable=SC2206 # PROJECTS is a word list of manifest keys
PROJECT_LIST=($PROJECTS)
for PROJECT in $PROJECTS; do
  INDEX=$((INDEX + 1))
  if [ -z "$STOP_REASON" ] && gate_stop_requested; then
    STOP_REASON="stop requested on the live run"
  fi
  if [ -z "$STOP_REASON" ] && [ "$GATE_ON_RED_MODE" = stop ] && [ -n "$FAILED$PREGATE_FAILED" ]; then
    STOP_REASON="GATE_ON_RED=stop after a red"
  fi
  if [ -n "$STOP_REASON" ]; then
    NOT_RUN="$NOT_RUN $PROJECT"
    continue
  fi
  PROJECT_START=$(date +%s)
  # The gates after this one: the tap's first red mid-gate reports what continuing costs.
  export GATE_REMAINING="${PROJECT_LIST[*]:$INDEX}"

  # Declares what kind of run this is for the gate-cycle observation (gate-cycle.mjs).
  GATE_RUN_KIND=pre-push node genesis/orchestrator/gate-runner.mjs --target "$PROJECT"
  GATE_EXIT=$?

  PROJECT_END=$(date +%s)
  PROJECT_ELAPSED=$((PROJECT_END - PROJECT_START))

  if [ $GATE_EXIT -ne 0 ]; then
    echo "$PROJECT: FAILED (${PROJECT_ELAPSED}s)"
    FAILED="$FAILED $PROJECT"
    if [ -n "$GATE_LIVE_RUN" ]; then
      node genesis/orchestrator/gate-live.mjs outlook --run "$GATE_LIVE_RUN" \
        --remaining "${PROJECT_LIST[*]:$INDEX}" >/dev/null 2>&1 || true
    fi
  else
    echo "$PROJECT: PASSED (${PROJECT_ELAPSED}s)"
  fi
  echo ""

  RESULTS="$RESULTS  $PROJECT: $([ $GATE_EXIT -eq 0 ] && echo PASSED || echo FAILED) (${PROJECT_ELAPSED}s)\n"
done

TOTAL_END=$(date +%s)
TOTAL_ELAPSED=$((TOTAL_END - TOTAL_START))
if [ -n "$NOT_RUN" ]; then
  echo "[pre-push] stopped: $STOP_REASON; not run:$NOT_RUN"
  FAILED="$FAILED${FAILED:+ }stopped"
fi
FAILED="$FAILED$PREGATE_FAILED"

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

hook_pass
}
