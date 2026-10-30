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

set -e

# An explicit BRIT_BIN is a path-scoped
# selection: an invalid path must not silently fall back to an older PATH tool.
BRIT_BIN_EXPLICIT=0
if [ "${BRIT_BIN+x}" = x ]; then
    BRIT_BIN_EXPLICIT=1
    [ -n "$BRIT_BIN" ] && [ -f "$BRIT_BIN" ] && [ -x "$BRIT_BIN" ] || BRIT_BIN=""
elif command -v brit >/dev/null 2>&1; then
    BRIT_BIN=brit
else
    BRIT_BIN=""
fi

# BRIT_BUILD_REF_BIN set by the caller wins (a local build in a private CARGO_TARGET_DIR).
BRIT_BUILD_REF_BIN="${BRIT_BUILD_REF_BIN:-}"
if [ -n "$BRIT_BUILD_REF_BIN" ]; then
    [ -x "$BRIT_BUILD_REF_BIN" ] || BRIT_BUILD_REF_BIN=""
elif command -v brit-build-ref >/dev/null 2>&1; then
    BRIT_BUILD_REF_BIN=brit-build-ref
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
        # Bounded: a real verify on this repository is ~15-17 s wall / ~60 s CPU (brit daily-driver
        # sprint, 2026-10-29); past BRIT_ADVISORY_TIMEOUT_S (default 60) it is a warn, never a wait.
        echo "[brit-helper] running repository-integrity advisory (timeout ${BRIT_ADVISORY_TIMEOUT_S:-60}s): $BRIT_BIN verify $*" >&2
        timeout "${BRIT_ADVISORY_TIMEOUT_S:-60}" "$BRIT_BIN" verify "$@" || {
            rc=$?
            if [ "$rc" -eq 124 ]; then
                echo "[brit-helper] WARN: brit verify timed out after ${BRIT_ADVISORY_TIMEOUT_S:-60}s — advisory only, not failing the build" >&2
            else
                echo "[brit-helper] WARN: brit verify exited $rc — advisory only, not failing the build" >&2
            fi
            exit 0
        }
        ;;
    plan)
        shift
        if [ -n "$BRIT_BIN" ] && "$BRIT_BIN" build plan --help >/dev/null 2>&1; then
            echo "[brit-helper] running: $BRIT_BIN build plan $*" >&2
            timeout "${BRIT_ADVISORY_TIMEOUT_S:-60}" "$BRIT_BIN" build plan "$@" || {
                rc=$?
                echo "[brit-helper] WARN: brit build plan exited $rc — advisory only, not failing the build" >&2
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
    *)
        echo "[brit-helper] usage: $0 {verify|plan|build-ref} [args...]" >&2
        exit 64
        ;;
esac
