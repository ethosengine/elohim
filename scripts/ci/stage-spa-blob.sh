#!/bin/bash
# stage-spa-blob.sh — upload ONE pillar-EPR browser bundle and (optionally)
# PATCH it onto its content row. Extracted verbatim from the Jenkinsfile's
# stageSpaBlobs helper (2026-06-10) when the inline heredoc pushed the CPS
# method past the JVM 64KB MethodTooLargeException limit — the bash lives
# here, the Jenkinsfile keeps a one-line call. See Jenkinsfile
# "STAGE HELPER METHODS" for the pillar-EPR decomposition rationale.
#
# Usage: stage-spa-blob.sh <dist-dir> <slug> <doorway-epr-url>
# Env:   STORAGE_API_KEY_ADMIN  admin key for the PATCH+verify step
#        DO_PATCH               "1" to PATCH+verify, anything else skips (WARN)
set -euo pipefail

DIST_DIR="$1"
SLUG="$2"
DOORWAY_EPR_URL="$3"
DO_PATCH="${DO_PATCH:-0}"

cd "${DIST_DIR}"

# Angular 19 SSR mode emits index.csr.html (Client-Side Rendered fallback)
# instead of index.html. For static SPA delivery through the protocol's
# /apps/{slug}/index.html route we need a literal index.html (storage's /apps
# handler is literal-path, doesn't fall back to index.csr.html). Pure SPAs
# (app/lamad) pass through unchanged.
if [ ! -f index.html ] && [ -f index.csr.html ]; then
    cp index.csr.html index.html
    echo "  [${SLUG}] materialized index.html from index.csr.html (Angular SSR-mode dist)"
fi

zip -r spa-bundle.zip .
SPA_HASH="sha256-$(sha256sum spa-bundle.zip | awk '{print $1}')"
SPA_SIZE="$(du -h spa-bundle.zip | cut -f1)"
echo "[${SLUG}] blob hash: ${SPA_HASH}"
echo "[${SLUG}] blob size: ${SPA_SIZE}"

# 1. Upload ZIP as blob via doorway's seed-blob route.
#
# Why /admin/seed/blob and not /blob/{hash}: doorway's forward_blob_to_storage
# (storage_proxy.rs) is a read-through cache that hardcodes client.get() — PUT
# requests get silently downgraded to GET, storage returns 404 on the
# not-yet-uploaded hash, the build fails with curl exit 22.
#
# /admin/seed/blob is the seeder's write-through path: doorway validates the
# X-Blob-Hash header, caches locally, then server-side forwards PUT
# /blob/{hash} to elohim-storage. (Preserves dev fix f853fb665 across the B21
# multi-bundle rewrite.)
curl -fSs -X PUT \
    -H 'Content-Type: application/zip' \
    -H "X-Blob-Hash: ${SPA_HASH}" \
    --data-binary @spa-bundle.zip \
    "${DOORWAY_EPR_URL}/admin/seed/blob"
echo "  ✓ [${SLUG}] blob uploaded (via /admin/seed/blob)"

# 2. Link blob to the content row (PATCH+verify) — only when admin key present.
# Regression seatbelt (PATCH path only): after each PATCH, GET the row and
# assert blobHash matches the SHA just written. set -euo pipefail + curl -fSs
# (no || echo swallow) means any 4xx/5xx FAILS the build — surfacing silent
# CI/storage drift as a red build instead of a stuck production surface.
if [ "${DO_PATCH}" = "1" ]; then
    curl -fSs -X PATCH \
        -H 'Content-Type: application/json' \
        -H "X-API-Key: ${STORAGE_API_KEY_ADMIN}" \
        -d "{\"blobHash\":\"${SPA_HASH}\"}" \
        "${DOORWAY_EPR_URL}/db/content/${SLUG}" \
        >/dev/null
    echo "  ✓ patched ${SLUG}"

    ACTUAL=$(curl -fSs "${DOORWAY_EPR_URL}/db/content/${SLUG}" \
        | python3 -c "import sys, json; print(json.load(sys.stdin).get('blobHash',''))")
    if [ "${ACTUAL}" != "${SPA_HASH}" ]; then
        echo "ERROR: ${SLUG} blobHash drifted after PATCH" >&2
        echo "  expected: ${SPA_HASH}" >&2
        echo "  actual:   ${ACTUAL}" >&2
        exit 1
    fi
    echo "  ✓ verified ${SLUG} blobHash = ${SPA_HASH}"
else
    echo "  ⊘ WARN: skipping PATCH+verify for ${SLUG} — no admin credential available"
    echo "    content row retains seed-time blobHash"
    echo "    blob bytes uploaded and content-addressable via PUT /blob/${SPA_HASH}"
fi

rm -f spa-bundle.zip
