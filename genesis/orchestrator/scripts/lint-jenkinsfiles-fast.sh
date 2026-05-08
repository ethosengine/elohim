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

if [ -z "$JENKINS_URL" ]; then
    echo "[jenkinsfile-lint] JENKINS_URL not set — skipping (CI will validate)"
    exit 0
fi

# Collect files: arguments win, else discover via git.
FILES=()
if [ "$#" -gt 0 ]; then
    FILES=("$@")
else
    REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    while IFS= read -r f; do
        FILES+=("$REPO_ROOT/$f")
    done < <(cd "$REPO_ROOT" && git ls-files '*Jenkinsfile*' \
        | grep -v -E '/(node_modules|\.git|target)/' || true)
fi

if [ "${#FILES[@]}" -eq 0 ]; then
    echo "[jenkinsfile-lint] No Jenkinsfiles to validate"
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
