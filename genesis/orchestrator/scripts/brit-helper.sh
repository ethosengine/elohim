#!/usr/bin/env sh
# brit-helper.sh — safe wrapper for brit CLI invocations during Stage 1a/1b migration.
#
# Stage 1a: brit not yet installed on ci-builder; helper logs WARN and exits 0.
# Stage 1b: brit installed; helper invokes it but keeps failures advisory.
# Stage 2:  helper retired in favor of direct brit calls (when failure should be load-bearing).
#
# Usage:
#   brit-helper.sh verify
#   brit-helper.sh plan --since refs/notes/brit/build-baselines/__global__
#   brit-helper.sh build-ref build put --step <name> --inputs-hash <hash> ...
#   brit-helper.sh attest <actual-build-graph.json>
#       one signed build attestation per pipeline the orchestrator run awaited, as a
#       git note under refs/notes/brit/build/<pipeline> (scripts/brit-attest.mjs holds
#       the field mapping). BRIT_NOTES_REMOTE=<remote> fetches and pushes those notes.
#       Never fails the build.

set -e

# Locate brit separately from legacy rakia. An explicit BRIT_BIN is a path-scoped
# selection: an invalid path must not silently fall back to an older PATH tool.
# Legacy rakia is accepted only for build planning, with a deprecation warning.
#
# REPO_ROOT default of /projects/elohim is the Eclipse Che dev path. CI callers
# (Jenkins, ci-builder) MUST set REPO_ROOT=$WORKSPACE explicitly. If REPO_ROOT
# is unset on CI, the fallback path won't exist and the helper degrades to
# "binary not installed" → WARN + exit 0 (still safe, but the fallback is
# intentionally dev-only).
BRIT_BIN_EXPLICIT=0
if [ "${BRIT_BIN+x}" = x ]; then
    BRIT_BIN_EXPLICIT=1
    [ -n "$BRIT_BIN" ] && [ -f "$BRIT_BIN" ] && [ -x "$BRIT_BIN" ] || BRIT_BIN=""
elif command -v brit >/dev/null 2>&1; then
    BRIT_BIN=brit
elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit" ]; then
    BRIT_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit"
else
    BRIT_BIN=""
fi

RAKIA_BIN=""
if [ "$BRIT_BIN_EXPLICIT" -eq 0 ]; then
    if command -v rakia >/dev/null 2>&1; then
        RAKIA_BIN=rakia
    elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/rakia" ]; then
        RAKIA_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/rakia"
    fi
fi

# BRIT_BUILD_REF_BIN set by the caller wins (a local build in a private CARGO_TARGET_DIR).
BRIT_BUILD_REF_BIN="${BRIT_BUILD_REF_BIN:-}"
if [ -n "$BRIT_BUILD_REF_BIN" ]; then
    [ -x "$BRIT_BUILD_REF_BIN" ] || BRIT_BUILD_REF_BIN=""
elif command -v brit-build-ref >/dev/null 2>&1; then
    BRIT_BUILD_REF_BIN=brit-build-ref
elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit-build-ref" ]; then
    BRIT_BUILD_REF_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit-build-ref"
fi

# Subcommand routing.
case "${1:-}" in
    verify)
        shift
        if [ -z "$BRIT_BIN" ]; then
            if [ "$BRIT_BIN_EXPLICIT" -eq 1 ]; then
                echo "[brit-helper] WARN: explicit BRIT_BIN is not executable; repository-integrity advisory skipped" >&2
            else
                echo "[brit-helper] WARN: brit not installed; repository-integrity advisory skipped (Stage 1a)" >&2
            fi
            exit 0
        fi
        # This checks Git repository integrity, not EPR authorization or native governance.
        echo "[brit-helper] running repository-integrity advisory: $BRIT_BIN verify $*" >&2
        "$BRIT_BIN" verify "$@" || {
            rc=$?
            echo "[brit-helper] WARN: brit verify exited $rc — advisory only, not failing the build" >&2
            exit 0
        }
        ;;
    plan)
        shift
        if [ -n "$BRIT_BIN" ] && "$BRIT_BIN" build plan --help >/dev/null 2>&1; then
            echo "[brit-helper] running: $BRIT_BIN build plan $*" >&2
            "$BRIT_BIN" build plan "$@" || {
                rc=$?
                echo "[brit-helper] WARN: brit build plan exited $rc — advisory only, not failing the build" >&2
                exit 0
            }
        elif [ -n "$RAKIA_BIN" ]; then
            echo "[brit-helper] WARN: deprecated legacy rakia plan fallback; install unified brit" >&2
            "$RAKIA_BIN" plan "$@" || {
                rc=$?
                echo "[brit-helper] WARN: legacy rakia plan exited $rc — advisory only, not failing the build" >&2
                exit 0
            }
        elif [ "$BRIT_BIN_EXPLICIT" -eq 1 ] && [ -z "$BRIT_BIN" ]; then
            echo "[brit-helper] WARN: explicit BRIT_BIN is not executable; plan advisory skipped" >&2
        elif [ -n "$BRIT_BIN" ]; then
            echo "[brit-helper] WARN: $BRIT_BIN lacks build plan; plan advisory skipped" >&2
        else
            echo "[brit-helper] WARN: brit not installed; plan advisory skipped (Stage 1a)" >&2
        fi
        ;;
    build-ref)
        shift
        if [ -z "$BRIT_BUILD_REF_BIN" ]; then
            echo "[brit-helper] WARN: brit-build-ref not installed; attestation skipped (Stage 1a)" >&2
            exit 0
        fi
        echo "[brit-helper] running: $BRIT_BUILD_REF_BIN $*" >&2
        "$BRIT_BUILD_REF_BIN" "$@" || {
            rc=$?
            echo "[brit-helper] WARN: brit-build-ref exited $rc — advisory only, not failing the build" >&2
            exit 0
        }
        ;;
    attest)
        shift
        if [ -z "$BRIT_BUILD_REF_BIN" ]; then
            echo "[brit-helper] WARN: brit-build-ref not installed; build attestations skipped (Stage 1a)" >&2
            exit 0
        fi
        if ! command -v node >/dev/null 2>&1; then
            echo "[brit-helper] WARN: node not on PATH; build attestations skipped" >&2
            exit 0
        fi
        echo "[brit-helper] attesting builds from ${1:-(no graph)} with $BRIT_BUILD_REF_BIN" >&2
        BRIT_BUILD_REF_BIN="$BRIT_BUILD_REF_BIN" REPO_ROOT="${REPO_ROOT:-$(pwd)}" \
            node "$(dirname "$0")/brit-attest.mjs" "$@" || \
            echo "[brit-helper] WARN: brit-attest exited $? — advisory only, not failing the build" >&2
        exit 0
        ;;
    *)
        echo "[brit-helper] usage: $0 {verify|plan|build-ref|attest} [args...]" >&2
        exit 64
        ;;
esac
