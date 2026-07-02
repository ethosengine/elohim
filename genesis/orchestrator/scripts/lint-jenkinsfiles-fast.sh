#!/bin/bash
# lint-jenkinsfiles-fast.sh [path…]
#
# Validates Jenkinsfiles against Jenkins's own pipeline-model-converter,
# which is the same parser Jenkins uses when loading a pipeline. Catches
# the failure class that breaks Jenkins on delivery (syntax errors,
# unbalanced braces, declarative-pipeline structure violations).
#
# With no arguments, validates every *Jenkinsfile* tracked by git.
# With arguments, validates only those paths.
#
# Requires JENKINS_URL. Auth is not needed — the validator endpoint is
# public on this Jenkins. If JENKINS_URL is unset or unreachable, the
# script warns and exits 0 (CI is the authoritative backstop).
#
# Exit codes:
#   0 = all Jenkinsfiles validated, OR Jenkins unreachable (skipped)
#   1 = at least one Jenkinsfile rejected by Jenkins
set -uo pipefail

JENKINS_URL="${JENKINS_URL:-}"
VALIDATE_URL="${JENKINS_URL%/}/pipeline-model-converter/validate"
CONNECT_TIMEOUT="${LINT_CONNECT_TIMEOUT:-3}"
MAX_TIME="${LINT_MAX_TIME:-15}"
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# Collect files: arguments win, else discover via git.
FILES=()
if [ "$#" -gt 0 ]; then
    FILES=("$@")
else
    while IFS= read -r f; do
        FILES+=("$REPO_ROOT/$f")
    done < <(cd "$REPO_ROOT" && git ls-files '*Jenkinsfile*' \
        | grep -v -E '/(node_modules|\.git|target)/' || true)
fi

if [ "${#FILES[@]}" -eq 0 ]; then
    echo "[jenkinsfile-lint] No Jenkinsfiles to validate"
    exit 0
fi

# ── Phase 1: LOCAL CPS bytecode-cost gate (no network, milliseconds). ──────
# Catches the MethodTooLargeException class that the Jenkins validate
# endpoint below structurally CANNOT: edge #1134 (2026-07-02) was a VALID
# declarative model that still died at Jenkinsfile compile because the
# stages-model CPS method exceeded 64KB JVM bytecode. Calibrated thresholds
# live in the hook (labeled compile outcomes); argv mode never reads stdin.
# Runs even when JENKINS_URL is unset/unreachable — this failure class must
# never reach a push unchecked.
HOOK="$REPO_ROOT/.claude/hooks/jenkinsfile-method-size.py"
if [ -f "$HOOK" ]; then
    if ! python3 "$HOOK" "${FILES[@]}"; then
        echo "[jenkinsfile-lint] FAIL: CPS method-size HARD limit (details above)."
        echo "    This class kills Jenkins at Jenkinsfile COMPILE (MethodTooLargeException)"
        echo "    before any stage runs — the build dies stageless-red on delivery."
        exit 1
    fi
    echo "[jenkinsfile-lint] CPS method-size gate: ${#FILES[@]} file(s) ok"
else
    echo "[jenkinsfile-lint] WARN: method-size hook not found at $HOOK — CPS-cost gate skipped"
fi

if [ -z "$JENKINS_URL" ]; then
    echo "[jenkinsfile-lint] JENKINS_URL not set — skipping remote validation (CI will validate)"
    exit 0
fi

# Reachability probe — one curl, fast-fail to skip if Jenkins is down.
PROBE=$(curl -sS -o /dev/null -w '%{http_code}' \
    --connect-timeout "$CONNECT_TIMEOUT" --max-time "$MAX_TIME" \
    "$JENKINS_URL/login" 2>/dev/null || echo "000")
if [ "$PROBE" = "000" ]; then
    echo "[jenkinsfile-lint] WARNING: Jenkins unreachable at $JENKINS_URL — skipping (CI will validate)"
    exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
FAIL=0
OK=0
SKIP=0
for FILE in "${FILES[@]}"; do
    if [ ! -f "$FILE" ]; then
        echo "[jenkinsfile-lint] SKIP $FILE (not a regular file)"
        SKIP=$((SKIP + 1))
        continue
    fi

    REL="${FILE#$REPO_ROOT/}"
    BODY=$(curl -sS --connect-timeout "$CONNECT_TIMEOUT" --max-time "$MAX_TIME" \
        -X POST -F "jenkinsfile=<$FILE" "$VALIDATE_URL" 2>&1)
    RC=$?

    if [ "$RC" -ne 0 ]; then
        echo "[jenkinsfile-lint] WARN $REL (curl rc=$RC) — skipping, CI will validate"
        SKIP=$((SKIP + 1))
        continue
    fi

    case "$BODY" in
        "Jenkinsfile successfully validated."*)
            echo "[jenkinsfile-lint] OK   $REL"
            OK=$((OK + 1))
            ;;
        *)
            echo "[jenkinsfile-lint] FAIL $REL"
            echo "$BODY" | sed 's/^/    /'
            FAIL=$((FAIL + 1))
            ;;
    esac
done

if [ "$FAIL" -gt 0 ]; then
    echo "[jenkinsfile-lint] ${FAIL} failed, ${OK} ok, ${SKIP} skipped"
    exit 1
fi

if [ "$SKIP" -gt 0 ]; then
    echo "[jenkinsfile-lint] ${OK} ok, ${SKIP} skipped (CI will validate skipped)"
else
    echo "[jenkinsfile-lint] ${OK} Jenkinsfile(s) validated ✓"
fi
exit 0
