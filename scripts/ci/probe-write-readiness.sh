#!/bin/bash
# probe-write-readiness.sh — per-host WRITE-readiness pre-probe for the App
# SPA-blob deploy (deploy-to-healthy, 2026-06-30).
#
# WHY THIS EXISTS
#   The app deploy stages each SPA bundle per host (PUT /admin/seed/blob then
#   PATCH /db/content/{slug} to set blobHash -> the SPA mount). When a host's
#   storage backend is wedged — the conductor's DHT get_links timing out, the
#   notarize write path 503ing — every stage leg fails and the host is left
#   serving a STALE bundle (the elohim.host-404 class). /health is too SHALLOW
#   to catch this: the doorway answers /health even when the storage WRITE path
#   behind it is dead. This probe exercises the REAL storage read path so the
#   caller can SKIP a clearly-dead host (deploy-to-healthy) instead of either
#   silently staging-as-stale or dragging a healthy multi-host deploy to red.
#
# WHAT IT PROBES (reads that traverse the storage path, NOT /health):
#   GET {host}/db/stats            — storage aggregate; a true storage read
#   GET {host}/db/content/{slug}   — a content-row read through storage
#   GET {host}/health              — ONLY to establish "doorway is reachable",
#                                    which disambiguates "backend dead" from
#                                    "this probe has no network at all".
#
# EXIT CONTRACT (the caller routes on these — read carefully):
#   0  READY / INDETERMINATE  -> DEPLOY this host. This is the FAIL-OPEN default
#                                (critic guard a). Returned for a healthy host
#                                AND for EVERY ambiguous outcome: probe network
#                                error, probe timeout, an unexpected curl code,
#                                a slug that simply 404s, or the doorway not
#                                being reachable at all. An ambiguous probe must
#                                NEVER skip a deployable host — we let the real
#                                stage leg fail LOUDLY instead.
#   2  CLEARLY DEGRADED       -> SKIP this host. Returned ONLY on a positive,
#                                unambiguous degraded signal: the doorway is
#                                reachable (/health answered < 500) BUT both
#                                /db/stats AND /db/content/{slug} return 5xx/000
#                                on ALL retries — i.e. the doorway is up as a
#                                process but its storage WRITE path is dead.
#   (any OTHER non-zero exit — e.g. a usage error — is treated as NON-2 by the
#    caller and therefore deploys; the integration stays fail-open.)
#
# Env (all optional):
#   PROBE_ATTEMPTS   storage-endpoint retries before declaring degraded (def 3)
#   PROBE_BACKOFF    seconds between retries (def 3)
#   PROBE_TIMEOUT    per-request --max-time seconds (def 10)
#
# Usage: probe-write-readiness.sh <doorway-epr-url> [slug]
#
# NOTE: deliberately does NOT `set -e` — a curl returning non-zero on a dead
# host is EXPECTED, not a script failure. Every risky op defaults to 000 and the
# only non-fail-open exit is the single explicit `exit 2`.
set -u

PROBE_ATTEMPTS="${PROBE_ATTEMPTS:-3}"
PROBE_BACKOFF="${PROBE_BACKOFF:-3}"
PROBE_TIMEOUT="${PROBE_TIMEOUT:-10}"

url="${1:?usage: probe-write-readiness.sh <doorway-epr-url> [slug]}"
slug="${2:-elohim-host-landing}"
base="${url%/}"

# Echo the HTTP status code for a GET, or 000 on any connection-level failure.
# Never returns non-zero (so it is safe under set -u without set -e).
probe_code() {
    local out
    out=$(curl -s -o /dev/null -w '%{http_code}' --max-time "${PROBE_TIMEOUT}" "$1" 2>/dev/null) || out=""
    [ -n "${out}" ] || out="000"
    printf '%s' "${out}"
}

# A code is "bad" (storage path not serving) iff it is a 5xx or 000 (no
# response). A 1xx/2xx/3xx/4xx means the server ANSWERED — including a 404,
# which is a valid "not found", not a degraded backend.
is_bad() {
    case "$1" in
        5??|000) return 0 ;;
        *) return 1 ;;
    esac
}

# True (0) iff EVERY attempt against the endpoint was bad (5xx/000). A single
# served (<500) response anywhere proves the storage path is up -> false (1).
endpoint_all_bad() {
    local probe_url="$1" i code
    for (( i = 1; i <= PROBE_ATTEMPTS; i++ )); do
        code=$(probe_code "${probe_url}")
        if ! is_bad "${code}"; then
            return 1
        fi
        if [ "${i}" -lt "${PROBE_ATTEMPTS}" ]; then
            sleep "${PROBE_BACKOFF}"
        fi
    done
    return 0
}

# Step 1: is the doorway reachable at all? If we cannot even get an HTTP answer
# from /health, we CANNOT distinguish "host backend down" from "this probe has
# no network" — fail open (deploy), let the real stage fail loudly if the host
# is genuinely down.
health_code=$(probe_code "${base}/health")
case "${health_code}" in
    2??|3??|4??) host_reachable=1 ;;
    *) host_reachable=0 ;;
esac

if [ "${host_reachable}" -ne 1 ]; then
    echo "probe[${base}]: /health -> ${health_code} (doorway not clearly reachable) — INDETERMINATE, fail-open (deploy this host)"
    exit 0
fi

# Step 2: the doorway is up. Is the storage WRITE path dead? Require BOTH
# storage reads to be bad across all retries — a conservative AND-gate so a
# single served response (even a 404) keeps us in fail-open territory.
if endpoint_all_bad "${base}/db/stats" && endpoint_all_bad "${base}/db/content/${slug}"; then
    echo "probe[${base}]: doorway UP (/health ${health_code}) but storage path DEAD — /db/stats and /db/content/${slug} returned 5xx/000 on all ${PROBE_ATTEMPTS} attempts. CLEARLY DEGRADED — skip this host." >&2
    exit 2
fi

echo "probe[${base}]: write path ready (or indeterminate) — deploy this host"
exit 0
