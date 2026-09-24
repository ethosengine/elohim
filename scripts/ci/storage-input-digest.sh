#!/usr/bin/env bash
# storage-input-digest.sh — content identity of the elohim-storage image's
# BUILD INPUTS, so the deploy can tell "this commit changed what storage runs"
# from "this commit rebuilt the same thing under a new tag".
#
# WHY (backlog staggered-conductor-fleet-restarts, 2026-09-22 CI wall-clock
# repair): the storage image tag is commit-derived (1.0.0-dev-<sha8>) and the
# image itself bakes GIT_COMMIT_* / BUILD_TIMESTAMP build args, so neither the
# tag nor the OCI digest can ever repeat across commits. Every edge deploy
# therefore moved all seven storage pod templates and restarted every peer,
# even for a doorway-only commit. The only stable answer to "did storage
# change?" is a digest of what the Docker build CONSUMES.
#
# WHAT IS HASHED — the git object ids (from HEAD) of:
#   · every build-context source the storage Dockerfile COPYs (all stages;
#     `COPY --from=` stage copies excluded). storage-build-inputs.test.mjs in
#     genesis/orchestrator already holds the Dockerfile's COPY set to be a
#     superset of the crate's `path = …` dependency closure, so this list is
#     the same one the build-trigger globs are checked against;
#   · the Dockerfile itself (base image tags, build steps, features);
#   · scripts/ci/build-storage-image.sh (the build args it passes);
#   · the elohim/holochain-conductor gitlink (CONDUCTOR_SOURCE_IMAGE — the
#     embedded conductor binary);
#   · a schema salt, bumped whenever this derivation changes meaning.
# A source path that lives INSIDE a submodule resolves to that submodule's
# gitlink (its pinned commit), which is exactly what determines its bytes.
#
# FAIL SAFE: any source that cannot be resolved in HEAD, or any tracked input
# with uncommitted modifications in the workspace, prints NOTHING and exits 1.
# The caller treats an empty digest as "cannot judge" and rolls with the
# freshly built image — a false "unchanged" is worse than a wasted roll.
#
# KNOWN, ACCEPTED GAP: mutable base-image tags (rust:1.94.0-slim-bookworm,
# debian:bookworm-slim) can move upstream without a repo change. Holding the
# live image then means base-layer updates wait for the next storage source
# change or an operator `[storage-roll]`.
#
# Usage: storage-input-digest.sh [dockerfile]   (run from the repo root)
# Prints: sha256:<64 hex>
set -euo pipefail

DOCKERFILE="${1:-elohim/elohim-storage/Dockerfile}"
# Seven per-human deploy branches call this in parallel against one work tree:
# never take the index lock for git status's opportunistic index refresh.
export GIT_OPTIONAL_LOCKS=0
SALT="storage-input-digest/v1"

die() { echo "storage-input-digest: $*" >&2; exit 1; }

[ -f "${DOCKERFILE}" ] || die "Dockerfile not found: ${DOCKERFILE}"
git rev-parse --verify -q HEAD >/dev/null || die "not in a git work tree with a HEAD"

# Build-context COPY sources: every token of a non-`--from` COPY line except
# flags and the final destination. Normalise `./x`, `x/`.
copy_sources() {
    awk '
        /^[[:space:]]*COPY[[:space:]]/ {
            if ($0 ~ /--from[=[:space:]]/) next
            n = 0
            for (i = 2; i <= NF; i++) {
                if ($i ~ /^--/) continue
                tok[++n] = $i
            }
            for (i = 1; i < n; i++) print tok[i]
        }
    ' "$1" | sed -e 's#^\./##' -e 's#/*$##' | sort -u
}

# Object id of <path> in HEAD; for a path inside a submodule, the gitlink of
# the nearest enclosing submodule. Empty on failure.
resolve_oid() {
    local path="$1" probe oid
    oid="$(git rev-parse -q --verify "HEAD:${path}" 2>/dev/null || true)"
    if [ -n "${oid}" ]; then
        printf '%s' "${oid}"
        return 0
    fi
    probe="${path}"
    while [ "${probe}" != "${probe%/*}" ]; do
        probe="${probe%/*}"
        # A gitlink shows as mode 160000 / type commit in the parent tree.
        if git ls-tree HEAD -- "${probe}" 2>/dev/null | awk '{print $2}' | grep -qx commit; then
            GITLINK_OF_LAST="${probe}"
            git rev-parse -q --verify "HEAD:${probe}" 2>/dev/null || true
            return 0
        fi
    done
    return 0
}

TMP_OID="$(mktemp)"
trap 'rm -f "${TMP_OID}"' EXIT

SOURCES="$(copy_sources "${DOCKERFILE}")"
[ -n "${SOURCES}" ] || die "no COPY sources parsed from ${DOCKERFILE}"

LINES="salt ${SALT}"$'\n'
STATUS_PATHS=()
for src in ${SOURCES} "${DOCKERFILE}" scripts/ci/build-storage-image.sh elohim/holochain-conductor; do
    GITLINK_OF_LAST=""
    # Not a command substitution: resolve_oid must set GITLINK_OF_LAST here.
    resolve_oid "${src}" > "${TMP_OID}"
    oid="$(cat "${TMP_OID}")"
    [ -n "${oid}" ] || die "cannot resolve '${src}' in HEAD — refusing to guess"
    LINES+="${src} ${oid}"$'\n'
    if [ -z "${GITLINK_OF_LAST}" ] && git ls-tree HEAD -- "${src}" 2>/dev/null | awk '{print $2}' | grep -qx commit; then
        GITLINK_OF_LAST="${src}"
    fi
    if [ -n "${GITLINK_OF_LAST}" ]; then
        # A pathspec INSIDE a submodule makes `git status` fatal, and an
        # uncloned `update = none` submodule is normal in CI. Judge a gitlink
        # by its checkout instead: absent/uncloned is fine (the build cannot
        # read it either); cloned at a DIFFERENT commit than HEAD pins is dirt.
        if [ -e "${GITLINK_OF_LAST}/.git" ]; then
            checked="$(git -C "${GITLINK_OF_LAST}" rev-parse HEAD 2>/dev/null || true)"
            pinned="$(git rev-parse -q --verify "HEAD:${GITLINK_OF_LAST}" 2>/dev/null || true)"
            [ -n "${checked}" ] && [ "${checked}" = "${pinned}" ] \
                || die "submodule ${GITLINK_OF_LAST} checked out at ${checked:-?}, HEAD pins ${pinned:-?} — cannot judge"
        fi
    else
        STATUS_PATHS+=("${src}")
    fi
done

# The Docker build reads the WORKSPACE, not HEAD. A tracked input edited in
# place (a pipeline step rewriting a source before the build) would make the
# HEAD ids lie, so any such modification voids the digest. Untracked files are
# not considered: build outputs regenerated deterministically from tracked
# sources are the normal case under these paths.
DIRTY="$(git status --porcelain --untracked-files=no -- "${STATUS_PATHS[@]}" 2>&1 || echo '?? status-failed')"
[ -z "${DIRTY}" ] || die "tracked storage inputs modified in the workspace — cannot judge: $(printf '%s' "${DIRTY}" | head -3 | tr '\n' ' ')"

printf 'sha256:%s\n' "$(printf '%s' "${LINES}" | sha256sum | awk '{print $1}')"
