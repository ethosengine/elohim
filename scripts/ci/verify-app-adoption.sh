#!/bin/bash
# verify-app-adoption.sh — did every named peer ADOPT this app release?
# A measurement, not a delivery step (native-delivery sprint Lane N5;
# elected-content spec §12.8 slice 2).
#
# publish-app-release.sh authors a release once and exits. Each peer's own
# release-adoption controller then verifies the bytes (every browser shell must
# boot) and points its bound slug rows at them. This script reads each peer's
# own adoption report through the doorway — GET /db/p2p/adoption?peer=<name>,
# the doorway's projection of that peer's GET /admin/adoption — and answers
# whether the peer applied exactly this release.
#
# Usage: verify-app-adoption.sh <doorway-url> <release-cid> <peer> [<peer> ...]
#
# Per peer, per poll, the channel row decides:
#   applied   appliedRelease.cid == <release-cid>, or a verdict
#             {state: applied, releaseCid: <release-cid>}          → done
#   refused   verdict.refusal.reason == app_bundle_cannot_boot    → FAILURE now:
#             the release's own bytes cannot boot, and no later sweep changes that
#   anything else (idle, waiting, a transient refusal, an older release, an
#             unreadable report)                                   → poll again
#
# Output: one line per peer —
#   APP-ADOPTED <peer> release=<cid>
#   APP-NOT-ADOPTED <peer> state=<state> reason=<reason>
#   APP-CANNOT-BOOT <peer> detail=<detail>
#
# Exit: 0 every peer adopted · 1 any peer refused app_bundle_cannot_boot
#   (FAILURE) · 3 the bound elapsed with a peer not yet adopted (UNSTABLE —
#   delivered, not yet proven) · 64 usage.
#
# Env:
#   APP_RELEASE_CHANNEL        the channel row to read (default runtime:app-bundle:alpha:dev)
#   VERIFY_ADOPTION_BUDGET_SECS total bound (default 600). Priced by the
#                              push-delivers-within-budget habit
#                              (genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md):
#                              ten minutes is one soak (60 s) plus the controller's
#                              sweep cadence with room for the artifact pull — a
#                              measurement bound, never a wait for the fleet to
#                              become writable (that is a readiness precondition's job).
#   VERIFY_ADOPTION_POLL_SECS  poll cadence (default 15)
#   VERIFY_ADOPTION_NODE       node for the JSON question (default node; python is
#                              not on this path — scripts/ci/.epr-meta)
#   STORAGE_API_KEY_ADMIN      X-API-Key, when the doorway gates the projection
set -euo pipefail

if [ "$#" -lt 3 ]; then
    echo "usage: verify-app-adoption.sh <doorway-url> <release-cid> <peer> [<peer> ...]" >&2
    exit 64
fi
DOORWAY="${1%/}"
RELEASE_CID="$2"
shift 2
PEERS=("$@")
CHANNEL_ID="${APP_RELEASE_CHANNEL:-runtime:app-bundle:alpha:dev}"
BUDGET_SECS="${VERIFY_ADOPTION_BUDGET_SECS:-600}"
POLL_SECS="${VERIFY_ADOPTION_POLL_SECS:-15}"
NODE_BIN="${VERIFY_ADOPTION_NODE:-node}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

# Classify ONE adoption report for ONE channel and release. Prints
#   applied | cannot-boot\t<detail> | pending\t<state>\t<reason>
classify() {
    "${NODE_BIN}" -e '
        const fs = require("fs");
        const [file, channel, cid] = process.argv.slice(1);
        let report;
        try { report = JSON.parse(fs.readFileSync(file, "utf8")); }
        catch { process.stdout.write("pending\tunreadable\tthe report is not JSON"); process.exit(0); }
        const row = (report.channels || []).find(r => r.channelId === channel);
        if (!row) { process.stdout.write("pending\tunfollowed\tthe peer reports no row for the channel"); process.exit(0); }
        const verdict = row.verdict || {};
        const applied = (row.appliedRelease && row.appliedRelease.cid === cid)
            || (verdict.state === "applied" && verdict.releaseCid === cid);
        if (applied) { process.stdout.write("applied"); process.exit(0); }
        const refusal = verdict.refusal || null;
        if (verdict.state === "refused" && refusal && refusal.reason === "app_bundle_cannot_boot") {
            process.stdout.write("cannot-boot\t" + String(refusal.detail || "").replace(/\s+/g, " "));
            process.exit(0);
        }
        const reason = refusal ? refusal.reason : (verdict.reason || "-");
        process.stdout.write("pending\t" + (verdict.state || "unknown") + "\t" + reason);
    ' "$@"
}

deadline=$(( $(date +%s) + BUDGET_SECS ))
declare -A DONE=()
declare -A LAST=()
while :; do
    for peer in "${PEERS[@]}"; do
        [ -n "${DONE[$peer]:-}" ] && continue
        out="${TMP_DIR}/${peer}.json"
        status="$(curl -sS -o "${out}" -w '%{http_code}' -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
            --max-time 20 "${DOORWAY}/db/p2p/adoption?peer=${peer}" 2>/dev/null || echo 000)"
        if [ "${status}" != "200" ]; then
            LAST[$peer]="pending	unreachable	HTTP ${status}"
            continue
        fi
        verdict="$(classify "${out}" "${CHANNEL_ID}" "${RELEASE_CID}")"
        case "${verdict}" in
            applied)
                DONE[$peer]=1
                echo "APP-ADOPTED ${peer} release=${RELEASE_CID}" ;;
            cannot-boot*)
                echo "APP-CANNOT-BOOT ${peer} detail=${verdict#cannot-boot	}"
                exit 1 ;;
            *)
                LAST[$peer]="${verdict}" ;;
        esac
    done
    [ "${#DONE[@]}" -eq "${#PEERS[@]}" ] && exit 0
    if [ "$(date +%s)" -ge "${deadline}" ]; then
        for peer in "${PEERS[@]}"; do
            [ -n "${DONE[$peer]:-}" ] && continue
            IFS=$'\t' read -r _ state reason <<<"${LAST[$peer]:-pending	unknown	-}"
            echo "APP-NOT-ADOPTED ${peer} state=${state} reason=${reason}"
        done
        exit 3
    fi
    sleep "${POLL_SECS}"
done
