#!/usr/bin/env sh
# brit-helper.sh — safe wrapper for brit CLI invocations during Stage 1a/1b migration.
#
# Stage 1a: brit not yet installed on ci-builder; helper logs WARN and exits 0.
# Stage 1b: brit installed; helper invokes brit and forwards its exit code.
# Stage 2:  helper retired in favor of direct brit calls (when failure should be load-bearing).
#
# Usage:
#   brit-helper.sh verify
#   brit-helper.sh plan --since refs/notes/brit/build-baselines/__global__
#   brit-helper.sh build-ref build put --step <name> --inputs-hash <hash> ...

set -e

# Locate brit binaries: prefer system PATH (`brit` or `rakia` — local build names it `rakia`),
# fall back to local submodule build at known path. Stage 1b ci-builder image installs as `brit`.
#
# REPO_ROOT default of /projects/elohim is the Eclipse Che dev path. CI callers
# (Jenkins, ci-builder) MUST set REPO_ROOT=$WORKSPACE explicitly. If REPO_ROOT
# is unset on CI, the fallback path won't exist and the helper degrades to
# "binary not installed" → WARN + exit 0 (still safe, but the fallback is
# intentionally dev-only).
BRIT_BIN=""
if command -v brit >/dev/null 2>&1; then
    BRIT_BIN=brit
elif command -v rakia >/dev/null 2>&1; then
    BRIT_BIN=rakia
elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/rakia" ]; then
    BRIT_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/rakia"
elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit" ]; then
    BRIT_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit"
fi

BRIT_BUILD_REF_BIN=""
if command -v brit-build-ref >/dev/null 2>&1; then
    BRIT_BUILD_REF_BIN=brit-build-ref
elif [ -x "${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit-build-ref" ]; then
    BRIT_BUILD_REF_BIN="${REPO_ROOT:-/projects/elohim}/elohim/brit/target/release/brit-build-ref"
fi

# Subcommand routing.
case "${1:-}" in
    verify)
        shift
        if [ -z "$BRIT_BIN" ]; then
            echo "[brit-helper] WARN: brit not installed; verify advisory skipped (Stage 1a)" >&2
            exit 0
        fi
        # brit verify is itself a stub today (Phase 2B); call it anyway so when it lands we get real output.
        echo "[brit-helper] running: $BRIT_BIN verify $*" >&2
        "$BRIT_BIN" verify "$@" || {
            rc=$?
            echo "[brit-helper] WARN: brit verify exited $rc — advisory only, not failing the build" >&2
            exit 0
        }
        ;;
    plan)
        shift
        if [ -z "$BRIT_BIN" ]; then
            echo "[brit-helper] WARN: brit not installed; plan advisory skipped (Stage 1a)" >&2
            exit 0
        fi
        echo "[brit-helper] running: $BRIT_BIN plan $*" >&2
        "$BRIT_BIN" plan "$@" || {
            rc=$?
            echo "[brit-helper] WARN: brit plan exited $rc — advisory only, not failing the build" >&2
            exit 0
        }
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
