#!/bin/bash
# verify-projected-head.sh — served-vs-declared propagation probe (Track-4 T4-2).
#
# Verify three distinct phases: the storage row declares the desired server head,
# the doorway materializes that head, and the declared route actually renders.
# The expected argument is a build target, not proof that the peer adopted it.
# Missing storage declaration fails within 90s; only an observed declaration
# earns the separate renderer-adoption window (400s by default).
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
# Missing attestation enters the same bounded convergence window as a stale
# head. Absence is a failed proof, never a successful skip.
#
# The health contract describes the server bundle. Browser boot and build-stamp
# identity are checked separately by verify-served-shell.sh through the browser
# mount. The optional /version.json commit log below is informational only; it
# cannot replace either executable-hash or browser-boot proof.
#
# Usage: verify-projected-head.sh <doorway-base-url> <slug> <expected-server-blob-hash> [expected-git-commit] [ssr-path=/]
# ssr-path must be a declared SSR-renderable route; a browser mount need not render.
# Exit codes:
#   0 = MATCH (served serverBlobHash == expected, either immediately or after
#       convergence retries)
#   1 = STILL-DIVERGENT (served serverBlobHash != expected after the full
#       convergence window) or UNREACHABLE (neither /health/startup nor
#       /health answered 200 after retries)
#
# Env knobs (overridable; defaults sized comfortably past one doorway
# reconcile tick — see MISMATCH convergence-window note below):
#   PROJHEAD_DECLARE_WINDOW    storage declaration propagation budget (def 90)
#   PROJHEAD_DECLARE_INTERVAL  seconds between declaration reads (def 10)
#   PROJHEAD_CONVERGE_INTERVAL  seconds between convergence re-probes (def 30)
#   PROJHEAD_CONVERGE_WINDOW    total seconds to keep probing a MISMATCH
#                                before failing for real (def 400)
set -euo pipefail

PROJHEAD_DECLARE_WINDOW="${PROJHEAD_DECLARE_WINDOW:-90}"
PROJHEAD_DECLARE_INTERVAL="${PROJHEAD_DECLARE_INTERVAL:-10}"
PROJHEAD_CONVERGE_INTERVAL="${PROJHEAD_CONVERGE_INTERVAL:-30}"
PROJHEAD_CONVERGE_WINDOW="${PROJHEAD_CONVERGE_WINDOW:-400}"

BASE_URL="$1"
SLUG="$2"
EXPECTED_HASH="$3"
EXPECTED_COMMIT="${4:-}"
SSR_PATH="${5:-/}"
EXPECTED_HEADING="${6:-}"
case "$SSR_PATH" in
    //*) echo "ERROR: SSR probe path must be local to the doorway" >&2; exit 2 ;;
    /*) ;;
    *) echo "ERROR: SSR probe path must start with /" >&2; exit 2 ;;
esac

HOST="${BASE_URL#http://}"
HOST="${HOST#https://}"

PROBE_BODY="$(mktemp /tmp/projected-head-probe.XXXXXX)"
trap 'rm -f "${PROBE_BODY}"' EXIT

# Read through this doorway's storage route, keeping transport/JSON failures
# distinct from an observed row with no declared server identity.
read_declaration() {
    local budget="$1" code
    observed_hash=""
    code=$(curl -sS -o "$PROBE_BODY" -w '%{http_code}' --max-time "$budget" \
        "$BASE_URL/db/content/$SLUG" 2>/dev/null) || code=000
    declaration_reason="HTTP-$code"
    [ "$code" = 200 ] || return 0
    observed_hash=$(python3 -c '
import json, sys
try:
    row = json.load(open(sys.argv[1]))
    if not isinstance(row, dict): raise ValueError("not an object")
    head = row.get("serverBlobHash")
    if head is not None and not isinstance(head, str): raise ValueError("not a hash")
    print(head or "")
except Exception:
    sys.exit(1)
' "$PROBE_BODY") || { declaration_reason="malformed-row"; return 0; }
    declaration_reason="missing-serverBlobHash"
    [ -z "$observed_hash" ] || declaration_reason="different-serverBlobHash"
}

declaration_start=$SECONDS
declaration_attempted=0
echo "  … SSR declaration: $HOST $SLUG expected=$EXPECTED_HASH (up to ${PROJHEAD_DECLARE_WINDOW}s)" >&2
while :; do
    remaining=$((PROJHEAD_DECLARE_WINDOW - (SECONDS - declaration_start)))
    if [ "$declaration_attempted" = 0 ] || [ "$remaining" -gt 0 ]; then
        request_budget=$((remaining > 0 && remaining < 20 ? remaining : 20))
        read_declaration "$request_budget"
        declaration_attempted=1
    fi
    if [ -n "$observed_hash" ] && { [ "$EXPECTED_HASH" = auto ] || [ "$observed_hash" = "$EXPECTED_HASH" ]; }; then
        EXPECTED_HASH="$observed_hash"
        echo "✓ SSR declaration observed: $HOST $SLUG $EXPECTED_HASH"
        break
    fi
    remaining=$((PROJHEAD_DECLARE_WINDOW - (SECONDS - declaration_start)))
    if [ "$remaining" -le 0 ]; then
        echo "ERROR: SSR declaration failed on $HOST $SLUG: expected=$EXPECTED_HASH observed=${observed_hash:-<absent>} reason=$declaration_reason; verify canonical server-head authoring and peer propagation before renderer adoption" >&2
        exit 1
    fi
    echo "  … declaration pending: $HOST $SLUG observed=${observed_hash:-<absent>} reason=$declaration_reason" >&2
    sleep "$((remaining < PROJHEAD_DECLARE_INTERVAL ? remaining : PROJHEAD_DECLARE_INTERVAL))"
done

# Fetch one health surface path into PROBE_BODY; prints the HTTP status code
# (or 000 on a curl-level failure). Never raises under `set -e` — the `|| echo
# 000` is the function's final command, so its exit status is always 0.
fetch_health() {
    local path="$1" budget="${2:-20}"
    curl -sS -o "${PROBE_BODY}" -w '%{http_code}' --max-time "$budget" "${BASE_URL}${path}" 2>/dev/null || echo 000
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
            print((entry.get('serverBlobHash') or '') if entry.get('status') == 'current' else '')
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

if [ "${served_hash}" = "${EXPECTED_HASH}" ]; then
    echo "✓ projected head propagated: ${HOST} ${SLUG} ${served_hash}"
else
    # MISMATCH here does not (yet) mean divergence: the doorway only
    # materializes a just-authored head on its own reconcile tick (~300s
    # cadence), and this probe runs immediately after the head is authored —
    # i.e. right before the swap, every time a build changes the SSR bundle
    # (elohim #1628/#1629 UNSTABLE on exactly this). The reachability ladder
    # above is for TRANSIENT unreachability only; this is a SEPARATE
    # convergence-window ladder for a reachable-but-stale host: keep probing
    # on a slower cadence until a window comfortably past one reconcile tick
    # closes, and only then report real divergence.
    adoption_start=$SECONDS
    elapsed=0
    converged=0
    echo "  … SSR adoption pending on ${HOST} ${SLUG}: expected=${EXPECTED_HASH} served=${served_hash:-<absent>} — doorway may still be converging (reconcile tick ~300s); entering renderer adoption window (up to ${PROJHEAD_CONVERGE_WINDOW}s, re-probing every ${PROJHEAD_CONVERGE_INTERVAL}s)" >&2

    # ACTUATE the reconcile instead of only waiting for it.
    #
    # `POST /admin/ssr-bundle/refresh` is documented as "the actuation twin of
    # the background reconcile tick": it re-resolves each slug's declared
    # serverBlobHash, re-materializes on mismatch, and hot-swaps the registry
    # entry IN PLACE — converging without a doorway restart. It is idempotent
    # (an already-current slug reports `current` and does no work), degrades
    # safely (`failed`/`unreachable` keep the old bundle), and is the same
    # no-auth Cat-C operator-seat class as /admin/steward-peers/refresh.
    #
    # Waiting passively for the host's own tick is what made this probe fail
    # for NINE consecutive builds (elohim #1632→#1640, both elohim.host slugs):
    # a doorway whose tick is not converging never converges just because we
    # watched it for 400s, and the failure text then told a human to "trigger a
    # restart" — a kubectl action CI cannot take. Asking the host to reconcile
    # is the action the message was really describing, and the pipeline can do
    # it itself. Non-gating: a refresh that fails or 404s (older doorway
    # without the route) leaves the convergence ladder below exactly as it was.
    refresh_code=skipped
    if [ "$PROJHEAD_CONVERGE_WINDOW" -gt 0 ]; then
        refresh_budget=$((PROJHEAD_CONVERGE_WINDOW < 30 ? PROJHEAD_CONVERGE_WINDOW : 30))
        refresh_code=$(curl -sS -o /tmp/ssr-refresh.$$ -w '%{http_code}' --max-time "$refresh_budget" \
            -X POST "${BASE_URL}/admin/ssr-bundle/refresh" 2>/dev/null || echo 000)
    fi
    if [ "${refresh_code}" = "200" ]; then
        echo "  ↻ asked ${HOST} to reconcile its SSR bundles: $(head -c 400 /tmp/ssr-refresh.$$ 2>/dev/null)" >&2
    else
        echo "  ↻ ssr-bundle refresh on ${HOST} returned ${refresh_code} (non-gating — falling back to the passive convergence window)" >&2
    fi
    rm -f /tmp/ssr-refresh.$$
    while :; do
        remaining=$((PROJHEAD_CONVERGE_WINDOW - (SECONDS - adoption_start)))
        [ "$remaining" -gt 0 ] || break
        sleep "$((remaining < PROJHEAD_CONVERGE_INTERVAL ? remaining : PROJHEAD_CONVERGE_INTERVAL))"
        remaining=$((PROJHEAD_CONVERGE_WINDOW - (SECONDS - adoption_start)))
        [ "$remaining" -gt 0 ] || break
        code=$(fetch_health "/health/startup" "$((remaining < 20 ? remaining : 20))")
        remaining=$((PROJHEAD_CONVERGE_WINDOW - (SECONDS - adoption_start)))
        if [ "${code}" != "200" ] && [ "$remaining" -gt 0 ]; then
            code=$(fetch_health "/health" "$((remaining < 20 ? remaining : 20))")
        fi
        elapsed=$((SECONDS - adoption_start))
        if [ "${code}" = "200" ] && [ "$(body_has_served_heads_key)" = "yes" ]; then
            served_hash=$(extract_served_hash)
            if [ -n "${served_hash}" ] && [ "${served_hash}" = "${EXPECTED_HASH}" ]; then
                converged=1
                break
            fi
        fi
        echo "  … SSR adoption still pending after ${elapsed}s/${PROJHEAD_CONVERGE_WINDOW}s: ${HOST} ${SLUG} served=${served_hash:-<absent>}" >&2
    done

    if [ "${converged}" = "1" ]; then
        echo "✓ projected head converged after ${elapsed}s: ${HOST} ${SLUG} ${served_hash}"
    else
        echo "ERROR: projected head still divergent after full ${PROJHEAD_CONVERGE_WINDOW}s convergence window on ${HOST} ${SLUG}: expected=${EXPECTED_HASH} served=${served_hash:-<absent>}" >&2
        exit 1
    fi
fi

# A declaration observed earlier must still hold when adoption completes.
read_declaration 20
if [ "$observed_hash" != "$EXPECTED_HASH" ]; then
    echo "ERROR: SSR declaration changed or unreadable after adoption on $HOST $SLUG: expected=$EXPECTED_HASH observed=${observed_hash:-<absent>} reason=$declaration_reason" >&2
    exit 1
fi
echo "  … SSR render: $BASE_URL$SSR_PATH at $EXPECTED_HASH" >&2

# A live registry alone cannot prove that requests take the SSR path.
mount="$SSR_PATH"
render_headers=$(mktemp)
if ! curl -fsS --max-time 60 -D "$render_headers" -o "$PROBE_BODY" -w '%{http_code}' "$BASE_URL$mount" | grep -q '^200$' || ! tr -d '\r' < "$render_headers" | grep -qi '^x-ssr-rendered: 1$'; then
    echo "ERROR: $BASE_URL$mount did not return SSR output at attested head $EXPECTED_HASH" >&2
    cat "$render_headers" >&2
    rm -f "$render_headers"
    exit 1
fi
rm -f "$render_headers"
if [ -n "$EXPECTED_HEADING" ]; then
    if ! python3 - "$PROBE_BODY" "$EXPECTED_HEADING" <<'PYHEADING'
from html.parser import HTMLParser
import sys
class Headings(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts = None
        self.headings = []
    def handle_starttag(self, tag, attrs):
        if tag == 'h1': self.parts = []
    def handle_endtag(self, tag):
        if tag == 'h1' and self.parts is not None:
            self.headings.append(' '.join(''.join(self.parts).split()))
            self.parts = None
    def handle_data(self, data):
        if self.parts is not None: self.parts.append(data)
p = Headings()
p.feed(open(sys.argv[1]).read())
sys.exit(0 if ' '.join(sys.argv[2].split()) in p.headings else 1)
PYHEADING
    then
        echo "ERROR: $BASE_URL$mount SSR HTML lacks the expected rendered h1 '$EXPECTED_HEADING' at $EXPECTED_HASH; a loading page or serialized state is not content delivery" >&2
        exit 1
    fi
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
