#!/bin/bash
# verify-host-serves.sh — post-stage proof that a host actually SERVES the
# freshly-staged SPA bundle (deploy resilience, 2026-06-30).
#
# WHY THIS EXISTS
#   stage-spa-blob.sh can upload the blob, PATCH the content row, and even
#   self-verify the row's blobHash, and STILL leave a host serving a stale or
#   dark '/'. The classic failure (2026-06-09 class): the content row's blobHash
#   is fresh but the EPR-routed '/' 404s ("App ZIP blob not found") because the
#   backing storage no longer holds the blob — invisible for days while the
#   build reports green/UNSTABLE. This script proves the human-visible root
#   actually serves AND (when a staged-hash record is available) that it points
#   at the freshly-staged bundle, not an old one.
#
# CHECKS (all must hold for exit 0):
#   1. GET {host}/                  -> 200. The root EPR mount a human visits
#                                     actually serves. Retried across the EPR
#                                     router's self-heal refresh window.
#   2. GET {host}/db/content/{slug} -> blobHash present + NON-NULL/non-empty.
#   3. (optional) If an expected-hash file is given AND non-empty (written by
#      stage-spa-blob.sh via STAGE_HASH_OUT — see that script), the row's
#      blobHash must EQUAL the staged hash. When no staged-hash record is
#      available the check degrades gracefully to 1+2 (which already catch the
#      404 / null-hash classes).
#
# EXIT: 0 = host serves the staged bundle correctly.
#       non-0 = host left STALE/dark — the caller DOWNGRADES this host's deploy
#               leg to a NAMED junit failure (loud, never silently green).
#
# This is the LOUD direction (opposite of probe-write-readiness.sh's fail-open):
# we tolerate transient blips via retries across the self-heal window, but a
# persistent can't-prove-it-serves is reported as STALE.
#
# Usage: verify-host-serves.sh <doorway-epr-url> <slug> [expected-hash-file]
set -uo pipefail

VERIFY_ATTEMPTS="${VERIFY_ATTEMPTS:-4}"
VERIFY_BACKOFF="${VERIFY_BACKOFF:-20}"
VERIFY_TIMEOUT="${VERIFY_TIMEOUT:-20}"

url="${1:?usage: verify-host-serves.sh <doorway-epr-url> <slug> [expected-hash-file]}"
slug="${2:?usage: verify-host-serves.sh <doorway-epr-url> <slug> [expected-hash-file]}"
hash_file="${3:-}"
base="${url%/}"

# Check 1: the root EPR mount must serve 200 (retry across the router's
# self-heal window — same tolerance as verify-epr-mount.sh).
served=0
for (( attempt = 1; attempt <= VERIFY_ATTEMPTS; attempt++ )); do
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time "${VERIFY_TIMEOUT}" "${base}/" 2>/dev/null) || code=000
    [ -n "${code}" ] || code=000
    if [ "${code}" = "200" ]; then
        served=1
        break
    fi
    echo "  … [${base}] GET / -> ${code} (attempt ${attempt}/${VERIFY_ATTEMPTS}; router may still be refreshing)"
    if [ "${attempt}" -lt "${VERIFY_ATTEMPTS}" ]; then
        sleep "${VERIFY_BACKOFF}"
    fi
done
if [ "${served}" -ne 1 ]; then
    echo "STALE: ${base}/ does not serve 200 after ${VERIFY_ATTEMPTS} attempts — host left STALE/dark" >&2
    exit 1
fi

# Check 2: the content row's blobHash must be present + non-empty. Retry
# briefly — the row read also rides the storage path.
row_hash=""
for (( attempt = 1; attempt <= VERIFY_ATTEMPTS; attempt++ )); do
    row_hash=$(curl -s --max-time "${VERIFY_TIMEOUT}" "${base}/db/content/${slug}" 2>/dev/null \
        | python3 -c 'import sys, json
try:
    v = json.load(sys.stdin).get("blobHash") or ""
except Exception:
    v = ""
print(v)' 2>/dev/null) || row_hash=""
    if [ -n "${row_hash}" ] && [ "${row_hash}" != "null" ]; then
        break
    fi
    if [ "${attempt}" -lt "${VERIFY_ATTEMPTS}" ]; then
        sleep "${VERIFY_BACKOFF}"
    fi
done
if [ -z "${row_hash}" ] || [ "${row_hash}" = "null" ]; then
    echo "STALE: ${base} db/content/${slug} blobHash is null/empty — '/' serves but the content row points at no bundle" >&2
    exit 1
fi

# Check 3 (optional): assert the served row equals the freshly-staged hash.
if [ -n "${hash_file}" ] && [ -s "${hash_file}" ]; then
    expected=$(head -n1 "${hash_file}" | tr -d '[:space:]')
    if [ -n "${expected}" ] && [ "${row_hash}" != "${expected}" ]; then
        echo "STALE: ${base} db/content/${slug} blobHash=${row_hash} != freshly-staged ${expected} — host is serving an OLD bundle" >&2
        exit 1
    fi
    echo "  ✓ [${base}] serves 200; content-row blobHash ${row_hash} == freshly-staged"
else
    echo "  ✓ [${base}] serves 200; content-row blobHash ${row_hash} (non-null; no staged-hash record to compare)"
fi
exit 0
