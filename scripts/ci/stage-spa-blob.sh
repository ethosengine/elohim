#!/bin/bash
# stage-spa-blob.sh — upload ONE pillar-EPR browser bundle and (optionally)
# PATCH it onto its content row. Extracted verbatim from the Jenkinsfile's
# stageSpaBlobs helper (2026-06-10) when the inline heredoc pushed the CPS
# method past the JVM 64KB MethodTooLargeException limit — the bash lives
# here, the Jenkinsfile keeps a one-line call. See Jenkinsfile
# "STAGE HELPER METHODS" for the pillar-EPR decomposition rationale.
#
# Usage: stage-spa-blob.sh <dist-dir> <slug> <doorway-epr-url> [kind]
#   kind: "browser" (default) or "server"
# Env:   STORAGE_API_KEY_ADMIN  admin key for the PATCH+verify step
#        DO_PATCH               "1" to PATCH+verify, anything else skips (WARN)
#        STAGE_BLOB_ATTEMPTS    max attempts per leg (default 3)
set -euo pipefail

DIST_DIR="$1"
SLUG="$2"
DOORWAY_EPR_URL="$3"
KIND="${4:-browser}"
DO_PATCH="${DO_PATCH:-0}"
ATTEMPTS="${STAGE_BLOB_ATTEMPTS:-3}"

cd "${DIST_DIR}"

# Angular 19 SSR mode emits index.csr.html (Client-Side Rendered fallback)
# instead of index.html. For static SPA delivery through the protocol's
# /apps/{slug}/index.html route we need a literal index.html (storage's /apps
# handler is literal-path, doesn't fall back to index.csr.html). Pure SPAs
# (app/lamad) pass through unchanged. Server bundles have no index.html at
# all — skip this step entirely for KIND=server.
if [ "$KIND" = "browser" ]; then
    if [ ! -f index.html ] && [ -f index.csr.html ]; then
        cp index.csr.html index.html
        echo "  [${SLUG}] materialized index.html from index.csr.html (Angular SSR-mode dist)"
    fi
fi

# Local + deterministic — done ONCE, not retried (a zip/sha failure is not a
# transient network blip and re-zipping yields the identical content-addressed
# hash anyway).
zip -r spa-bundle.zip .
SPA_HASH="sha256-$(sha256sum spa-bundle.zip | awk '{print $1}')"
SPA_SIZE="$(du -h spa-bundle.zip | cut -f1)"
echo "[${SLUG}] blob hash: ${SPA_HASH}"
echo "[${SLUG}] blob size: ${SPA_SIZE}"

# Field-by-kind (SSR row collapse): KIND=server PATCHes/reads serverBlobHash on
# the ONE elohim-host-landing EPR node (not a separate -ssr row); KIND=browser
# stays blobHash. Both ride db/content/{slug} with identical partial-update
# semantics — PATCHing one field never clobbers the other.
if [ "$KIND" = "server" ]; then
    HASH_FIELD="serverBlobHash"
else
    HASH_FIELD="blobHash"
fi

# Single notarized head. The blobHash PATCH is a DNA-notarized write
# (storage's patch_needs_conductor=true). The head is authored ONCE: this PATCH
# is routed through a doorway to a storage backend that has a live conductor
# bridge; the CONDUCTOR authors the DHT entry and the peer network (the
# Holochain DHT) WITNESSES it by validating + gossiping + anchoring. The doorway
# is only the gateway — never the witness. The Jenkinsfile fails this PATCH over
# across doorways until one reaches a live bridge, then stops; the single
# witnessed head gossips to every peer. A backend with NO conductor bridge 503s
# here, and that is CORRECT: that host does not author, it converges the head
# later via run_content_sweep. There is NO `?deployTier=amber` escape hatch
# anymore — a per-host diesel-direct write minted an un-witnessed head that could
# never green and diverged across backends. (The byte upload below still runs per
# host: blob BYTES are content-addressed and do not auto-replicate yet, so
# seeding them on each serving peer is legitimate load-spread, not a divergent
# write.)
PATCH_PATH="/db/content/${SLUG}"

# One PUT + (optional) PATCH+verify attempt. Returns non-zero on ANY failure
# WITHOUT exiting the script: stage_once is invoked in an `if` condition, so
# bash suspends `set -e` inside its body — each network op carries an explicit
# `|| return 1`, letting the retry loop below catch transient failures.
#
# 1. Upload ZIP as blob via doorway's seed-blob route.
#    Why /admin/seed/blob and not /blob/{hash}: doorway's forward_blob_to_storage
#    (storage_proxy.rs) is a read-through cache that hardcodes client.get() — PUT
#    requests get silently downgraded to GET, storage returns 404 on the
#    not-yet-uploaded hash, the build fails with curl exit 22. /admin/seed/blob
#    is the seeder's write-through path (preserves dev fix f853fb665).
#
#    AUTH (2026-06-27): doorway's require_seed_authority gate (commit 396779747,
#    2026-06-11) 401s an UNAUTHENTICATED PUT on any non-DEV_MODE doorway. alpha
#    (doorway-A) sets DEV_MODE=true and bypassed the gate, masking the omission;
#    elohim.host (doorway-B / alpha-b) does NOT set DEV_MODE, so its PUT 401'd
#    EVERY build → blob never uploaded → host silently stranded on a stale bundle
#    for ~2 weeks (the per-host deploy-lag class). The PATCH already sent the
#    admin key; the PUT was simply never updated when the gate landed. Send the
#    SAME admin X-API-Key here — seed.rs: "Admin API key (X-API-Key) also passes
#    — the operator credential CI uses." DEV_MODE hosts ignore it; gated hosts
#    require it. ${VAR:-} keeps it set -u-safe when no admin key is in scope (the
#    no-credential PUT then correctly stays unauthorized on a gated host).
# 2. Link blob to the content row (PATCH+verify) — only when admin key present.
#    Seatbelt: after each PATCH, GET the row and assert the hash field matches
#    the SHA just written; a drift returns non-zero (→ retried, then NAMED).
stage_once() {
    curl -fSs -X PUT \
        -H 'Content-Type: application/zip' \
        -H "X-Blob-Hash: ${SPA_HASH}" \
        -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
        --data-binary @spa-bundle.zip \
        "${DOORWAY_EPR_URL}/admin/seed/blob" || return 1
    echo "  ✓ [${SLUG}] blob uploaded (via /admin/seed/blob)"

    if [ "${DO_PATCH}" = "1" ]; then
        curl -fSs -X PATCH \
            -H 'Content-Type: application/json' \
            -H "X-API-Key: ${STORAGE_API_KEY_ADMIN}" \
            -d "{\"${HASH_FIELD}\":\"${SPA_HASH}\"}" \
            "${DOORWAY_EPR_URL}${PATCH_PATH}" \
            >/dev/null || return 1
        echo "  ✓ patched ${SLUG} (${HASH_FIELD})"

        local actual
        actual=$(curl -fSs "${DOORWAY_EPR_URL}/db/content/${SLUG}" \
            | HASH_FIELD="${HASH_FIELD}" python3 -c "import os, sys, json; print(json.load(sys.stdin).get(os.environ['HASH_FIELD'],''))") || return 1
        if [ "${actual}" != "${SPA_HASH}" ]; then
            echo "  ✗ [${SLUG}] ${HASH_FIELD} drift after PATCH: expected ${SPA_HASH}, got ${actual:-<empty>}" >&2
            return 1
        fi
        echo "  ✓ verified ${SLUG} ${HASH_FIELD} = ${SPA_HASH}"
    else
        echo "  ⊘ [${SLUG}] byte-seed only (DO_PATCH!=1) — no head PATCH from this call."
        echo "    The single notarized head is authored once via a conductor-bridged"
        echo "    doorway; this peer converges it via run_content_sweep. Blob bytes"
        echo "    uploaded and content-addressable via PUT /blob/${SPA_HASH}."
    fi
    return 0
}

# Bounded retry with linear backoff. A conductor-bridged backend can 503
# transiently during cluster churn (the notarized PATCH round-trips the
# conductor); retry rather than surrender on a blip. A PERSISTENT non-zero exit
# is meaningful: for a byte-seed (DO_PATCH!=1) it means the blob upload failed;
# for an author attempt (DO_PATCH=1) it means THIS doorway could not author (no
# live conductor bridge / persistent 503), and the Jenkinsfile fails the head
# author over to the next doorway. The deploy is only UNSTABLE if NO doorway
# in the fabric can author the single head.
attempt=1
while true; do
    if stage_once; then
        break
    fi
    if [ "${attempt}" -ge "${ATTEMPTS}" ]; then
        echo "ERROR: [${SLUG}] stage failed after ${ATTEMPTS} attempt(s) against ${DOORWAY_EPR_URL} — host left STALE" >&2
        rm -f spa-bundle.zip
        exit 1
    fi
    backoff=$(( attempt * 5 ))
    echo "  ⚠ [${SLUG}] attempt ${attempt}/${ATTEMPTS} against ${DOORWAY_EPR_URL} failed — retrying in ${backoff}s" >&2
    attempt=$(( attempt + 1 ))
    sleep "${backoff}"
done

rm -f spa-bundle.zip
