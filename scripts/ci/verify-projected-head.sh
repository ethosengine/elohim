#!/bin/bash
# verify-projected-head.sh — served-vs-declared propagation probe (Track-4 T4-2).
#
# stageSpaBlobs/authorHeadOnce prove the content ROW's declared head was PATCHed
# (stage-spa-blob.sh:199-206 verifies the DIESEL ROW hash, not what any running
# process serves). verifyEprMounts (verify-epr-mount.sh) proves a routed mount
# answers 200. Neither proves the doorway PROCESS actually serving that mount has
# MATERIALIZED the just-authored head — a stale-but-200 doorway passes both
# existing legs. This script closes that gap: it asks the doorway itself (via its
# health surface, not the content row) what server bundle head it has served, and
# compares that against the hash this build just authored.
#
# Health-surface contract (T4-1, deployed in parallel with this script — written
# to the CONTRACT below, not to whatever doorway code exists at write-time):
#   GET /health/startup (preferred), falling back to GET /health
#   {
#     "servedBundleHeads": [
#       { "slug": "...", "serverBlobHash": "...", "materializedAt": "...",
#         "status": "current|stale|refreshing|failed",
#         "declaredServerBlobHash": "..." }
#     ]
#   }
# Until T4-1 ships on a given host, `servedBundleHeads` (or the slug's entry in
# it) is simply absent — that reads as SKIP (attestation not yet deployed), not
# FAIL, so this probe can be wired into CI before T4-1 lands everywhere without
# turning every build red.
#
# BROWSER-BUNDLE LIMITATION: the contract above only carries `serverBlobHash`
# (the SSR/server bundle). There is no equivalent served-bytes content-hash
# marker for the browser (CSR) bundle: version.json (Jenkinsfile ~1020-1030)
# embeds GIT_COMMIT_HASH, not the zip's content-addressed SPA_HASH, and is
# emitted once per elohim-app build — not once per pillar-EPR slug (lamad-spa
# gets its own browser bundle with no matching version.json of its own). So it
# cannot stand in for a slug-scoped content hash. This script optionally fetches
# <host>/version.json and logs whether its "commit" field matches the current
# build's commit as a CHEAP, NON-GATING liveness signal only (see the tail of
# this script) — it never affects the exit code. Closing the browser-bundle gap
# for real needs a browser-bundle content marker added to the same T4-1 health
# contract (tracked as follow-up; not built here).
#
# Usage: verify-projected-head.sh <doorway-base-url> <slug> <expected-server-blob-hash> [expected-git-commit]
# Exit codes:
#   0 = MATCH (served serverBlobHash == expected) or FIELD-ABSENT (attestation
#       not deployed on this host yet — forward-compatible skip)
#   1 = MISMATCH (served serverBlobHash != expected) or UNREACHABLE (neither
#       /health/startup nor /health answered 200 after retries)
set -euo pipefail

BASE_URL="$1"
SLUG="$2"
EXPECTED_HASH="$3"
EXPECTED_COMMIT="${4:-}"

HOST="${BASE_URL#http://}"
HOST="${HOST#https://}"

PROBE_BODY="$(mktemp /tmp/projected-head-probe.XXXXXX)"
trap 'rm -f "${PROBE_BODY}"' EXIT

# Fetch one health surface path into PROBE_BODY; prints the HTTP status code
# (or 000 on a curl-level failure). Never raises under `set -e` — the `|| echo
# 000` is the function's final command, so its exit status is always 0.
fetch_health() {
    local path="$1"
    curl -sS -o "${PROBE_BODY}" -w '%{http_code}' --max-time 20 "${BASE_URL}${path}" 2>/dev/null || echo 000
}

# True ("yes") when PROBE_BODY parses as JSON carrying a servedBundleHeads
# array (regardless of whether SLUG has an entry in it). Never errors on
# malformed/absent JSON.
body_has_served_heads_key() {
    python3 -c "
import json
try:
    data = json.load(open('${PROBE_BODY}'))
except Exception:
    print('no')
else:
    print('yes' if isinstance(data.get('servedBundleHeads'), list) else 'no')
" 2>/dev/null || echo no
}

# Prints the serverBlobHash for SLUG's servedBundleHeads[] entry, or empty.
extract_served_hash() {
    SLUG="${SLUG}" python3 -c "
import json, os
try:
    data = json.load(open('${PROBE_BODY}'))
except Exception:
    print('')
else:
    slug = os.environ['SLUG']
    for entry in data.get('servedBundleHeads') or []:
        if isinstance(entry, dict) and entry.get('slug') == slug:
            print(entry.get('serverBlobHash') or '')
            break
    else:
        print('')
"
}

served_hash=""
found_key="no"
reached=0

# Retry ladder mirrors verify-epr-mount.sh (4 attempts, 20s cadence) — for
# TRANSIENT reachability only (a health endpoint 000/5xx during rollout
# churn). Once ANY health surface answers 200, we stop retrying: whether
# servedBundleHeads is present or absent is a fact about deployment state,
# not something a retry changes.
for attempt in 1 2 3 4; do
    code=$(fetch_health "/health/startup")
    if [ "${code}" = "200" ]; then
        reached=1
        found_key=$(body_has_served_heads_key)
        if [ "${found_key}" = "yes" ]; then
            served_hash=$(extract_served_hash)
            break
        fi
    fi
    # Fall back to /health (either /health/startup 404s/absent, or it answered
    # 200 without the field — some deploys may only wire the field onto /health).
    code2=$(fetch_health "/health")
    if [ "${code2}" = "200" ]; then
        reached=1
        found_key=$(body_has_served_heads_key)
        if [ "${found_key}" = "yes" ]; then
            served_hash=$(extract_served_hash)
            break
        fi
    fi
    if [ "${reached}" = "1" ]; then
        break
    fi
    echo "  … attempt ${attempt}/4: ${HOST} health surfaces unreachable — retrying in 20s" >&2
    sleep 20
done

if [ "${reached}" = "0" ]; then
    echo "ERROR: ${HOST} unreachable on both /health/startup and /health after retries" >&2
    exit 1
fi

if [ "${found_key}" != "yes" ]; then
    echo "⚠ attestation not deployed on ${HOST} — skipping (servedBundleHeads absent from health surface; T4-1 not yet live here)"
    exit 0
fi

if [ -z "${served_hash}" ]; then
    echo "⚠ attestation not deployed on ${HOST} — skipping (no servedBundleHeads entry for slug '${SLUG}')"
    exit 0
fi

if [ "${served_hash}" = "${EXPECTED_HASH}" ]; then
    echo "✓ projected head propagated: ${HOST} ${SLUG} ${served_hash}"
else
    echo "ERROR: projected head MISMATCH on ${HOST} ${SLUG}: declared=${EXPECTED_HASH} served=${served_hash}" >&2
    exit 1
fi

# Cheap, NON-GATING browser-bundle liveness signal (see header LIMITATION).
# This never touches the exit code — it is informational only.
if [ -n "${EXPECTED_COMMIT}" ]; then
    version_code=$(curl -sS -o "${PROBE_BODY}" -w '%{http_code}' --max-time 20 "${BASE_URL}/version.json" 2>/dev/null || echo 000)
    if [ "${version_code}" = "200" ]; then
        actual_commit=$(python3 -c "
import json
try:
    data = json.load(open('${PROBE_BODY}'))
    print(data.get('commit') or '')
except Exception:
    print('')
" 2>/dev/null || echo '')
        if [ "${actual_commit}" = "${EXPECTED_COMMIT}" ]; then
            echo "  ✓ (info) ${HOST} /version.json commit matches build (${actual_commit}) — browser-bundle liveness only, not a content-hash check"
        else
            echo "  ⚠ (info, non-gating) ${HOST} /version.json commit='${actual_commit:-<absent>}' != build commit '${EXPECTED_COMMIT}' — the browser SPA bundle has no served content-hash marker (see script header); not a failure"
        fi
    else
        echo "  ⚠ (info, non-gating) ${HOST}/version.json not fetchable (HTTP ${version_code}) — skipping browser liveness signal"
    fi
fi

exit 0
