#!/usr/bin/env bash
# deliverability-gate.sh — ask the peer whether the head we just uploaded can boot.
#
# Usage: deliverability-gate.sh <doorway-epr-url> <blob-hash> [attempts]
#
# The storage peer judges a bundle from its bytes when it first extracts it
# (elohim-storage app_deliverability, spec 2026-09-05 §2). This gate forces that
# first extraction with one GET of index.html by CONTENT ADDRESS (never the
# slug — the slug still points at the previous head), then reads the verdict off
# HEAD /apps/{hash}/_capability. Exit codes:
#   0  boots (or not-judged after the attempts, advisory — see DELIVERABILITY_GATE)
#   2  broken — the reason is printed; the caller must NOT author this head
#   3  not-judged under DELIVERABILITY_GATE=strict
# Born from 2026-09-04: app #1691's bundle was authored, served and blank for a
# day; the peer could have said "missing-asset:main-EAKNZDUP.js" before the
# head existed.
set -uo pipefail
DOORWAY_EPR_URL="${1:?doorway epr url}"
BLOB_HASH="${2:?blob hash}"
ATTEMPTS="${3:-6}"
GATE_MODE="${DELIVERABILITY_GATE:-advisory}"

verdict=""; reason=""
for attempt in $(seq 1 "$ATTEMPTS"); do
    # Force extraction (and the judgement) by content address; ignore the body.
    curl -sS -o /dev/null --max-time 120 "${DOORWAY_EPR_URL}/apps/${BLOB_HASH}/index.html" || true
    headers="$(curl -sS -I --max-time 30 "${DOORWAY_EPR_URL}/apps/${BLOB_HASH}/_capability" || true)"
    verdict="$(printf '%s' "$headers" | tr -d '\r' | awk -F': ' 'tolower($1)=="x-deliverability"{print $2}')"
    reason="$(printf '%s' "$headers" | tr -d '\r' | awk -F': ' 'tolower($1)=="x-deliverability-reason"{print $2}')"
    case "$verdict" in
        boots)  echo "  ✓ deliverability: ${BLOB_HASH} boots (peer-judged)"; exit 0 ;;
        broken) echo "  ✗ deliverability: ${BLOB_HASH} is BROKEN — ${reason:-no reason} — refusing to author this head" >&2; exit 2 ;;
        *)      echo "  … deliverability: not judged yet (attempt ${attempt}/${ATTEMPTS}, header='${verdict:-absent}')"
                [ "${attempt}" -ne "${ATTEMPTS}" ] && sleep 5
                ;;
    esac
done
if [ "$GATE_MODE" = "strict" ]; then
    echo "  ✗ deliverability: ${BLOB_HASH} NOT-JUDGED after ${ATTEMPTS} attempts (strict) — refusing to author" >&2; exit 3
fi
echo "  ⚠ deliverability: ${BLOB_HASH} NOT-JUDGED after ${ATTEMPTS} attempts — the peer holds no verdict (pre-cure peer or bytes not walked); proceeding, advisory" >&2
exit 0
