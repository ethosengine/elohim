#!/bin/bash
# verify-epr-mount.sh — end-to-end EPR serving seatbelt for ONE routed mount.
# Extracted verbatim from the Jenkinsfile's verifyEprMounts helper (2026-06-10,
# CPS 64KB limit). Rationale (2026-06-09 regression class): content rows can
# point at blob hashes the backing storage no longer holds — in that state
# /apps/{slug}/* keeps serving 200 from the doorway's own app cache while the
# EPR-routed mounts a human actually visits ('/', '/lamad') 404 with "App ZIP
# blob not found" for days, invisibly. So probe the routed mounts themselves,
# not /apps. Retries span the EPR router's 30s self-heal refresh window.
# The Jenkinsfile caller wraps in catchError->UNSTABLE: drift is surfaced
# without aborting the orchestrator dependency chain.
#
# Usage: verify-epr-mount.sh <url>
set -euo pipefail

url="$1"
for attempt in 1 2 3 4; do
    code=$(curl -sS -o /tmp/epr-probe-body -w '%{http_code}' --max-time 20 "${url}" || echo 000)
    if [ "${code}" = "200" ]; then
        echo "  ✓ ${url} serves (200)"
        exit 0
    fi
    echo "  … attempt ${attempt}/4: ${url} -> ${code} (router may still be refreshing)"
    sleep 20
done
echo "ERROR: EPR mount ${url} does not serve after blob staging" >&2
head -c 300 /tmp/epr-probe-body >&2 || true
exit 1
