#!/bin/bash
# publish-app-release.sh — publish this build's app bundles ONCE, as one elected
# release, and let every peer adopt it when it is ready (native-delivery sprint
# Lane N5; elected-content spec §12.8 slice 2).
#
# WHY. The App pipeline used to deliver an app by writing it onto every serving
# host itself: a blob PUT per host, a head PATCH per host, a declare fan-out per
# host (stage-spa-blob.sh + authorHeadOnce). Every one of those writes waited on
# the fleet being writable, which is how app #1719-#1725 spent ~12 pipeline-hours
# delivering nothing. Here CI authors ONCE: four blob PUTs (browser + server for
# each app) to ONE doorway, and one release version on the app-bundle channel.
# Each peer's release-adoption controller fetches the bytes from the blob plane
# and points its own bound slug rows at them (AppBundleVehicle) when IT is ready.
# Convergence is the peer's job; this script never waits for it.
#
# Usage: publish-app-release.sh <doorway-url> <manifest-out>
#
# Steps, each refusing rather than guessing:
#   1. zip each (app, kind) dist with the ONE recipe (lib/bundle-zip.sh), so the
#      bytes published are byte-identical to what the deliverability gate judged;
#   2. read the channel behind the doorway: absent → exit 2 naming the one-time
#      steward act (`release-ceremony.ts channel create`); its current release
#      carrying EXACTLY these artifact digests → nothing to publish, exit 0
#      (idempotent by content, so a rebuilt-but-identical app never re-releases);
#   3. package the manifest (genesis/a2o/scripts/epr-release-package.ts,
#      --artifact-class app-bundle, --put-via doorway: the four PUTs, each
#      round-tripped by GET before the packager exits 0), lineage parent = the
#      channel's current head;
#   4. publish it at STAGING (release-ceremony.ts publish --transport doorway).
#      Promote and revert are the existing ceremony verbs — never this script.
#
# Output: the manifest at <manifest-out>; one line on stdout:
#   APP-RELEASE-PUBLISHED channel=<id> release=<cid>
#   APP-RELEASE-CURRENT channel=<id> release=<cid>     (nothing new to publish)
# and <manifest-out>.publish.json with the ceremony's JSON (published runs only).
#
# Exit: 0 published or already current · 2 refused (channel absent, a zip the
#   packager refused, a packaging or publish failure) · 64 usage.
#
# Env:
#   STORAGE_API_KEY_ADMIN        X-API-Key for the PUTs, the PATCH and the declare
#                                (one admin key on one doorway — bounded because a
#                                staging declaration never beats an earned head).
#   APP_RELEASE_CHANNEL          default runtime:app-bundle:alpha:dev
#   APP_RELEASE_BUNDLES          whitespace-separated <slug>:<kind>:<dist-dir>[:<mount>]
#                                (default: the elohim-app and lamad browser+server
#                                dists under WORKSPACE, mounts / and /lamad/)
#   APP_RELEASE_SOAK_SECS        adoptionDiscipline.soakSecs (default 60)
#   APP_RELEASE_ATTESTATION_THRESHOLD
#                                adoptionDiscipline.attestationThreshold (default 1)
#   APP_RELEASE_NODE             node for the one JSON question per answer (default
#                                node; python is not on this path — scripts/ci/.epr-meta)
#   APP_RELEASE_TSX              the tsx runner (default <repo>/node_modules/.bin/tsx)
#   BUNDLE_ZIP_LIB               the zip recipe to source (default lib/bundle-zip.sh)
#   APP_RELEASE_WORK_DIR         where the zips are written (default mktemp -d)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

if [ "$#" -ne 2 ]; then
    echo "usage: publish-app-release.sh <doorway-url> <manifest-out>" >&2
    exit 64
fi
DOORWAY="${1%/}"
MANIFEST_OUT="$2"
CHANNEL_ID="${APP_RELEASE_CHANNEL:-runtime:app-bundle:alpha:dev}"
NODE_BIN="${APP_RELEASE_NODE:-node}"
TSX_BIN="${APP_RELEASE_TSX:-${REPO_ROOT}/node_modules/.bin/tsx}"
WORKSPACE_ROOT="${WORKSPACE:-${REPO_ROOT}}"
BUNDLES="${APP_RELEASE_BUNDLES:-elohim-host-landing:browser:${WORKSPACE_ROOT}/app/elohim-app/dist/elohim-app/browser:/ elohim-host-landing:server:${WORKSPACE_ROOT}/app/elohim-app/dist/elohim-app/server:/ lamad-spa:browser:${WORKSPACE_ROOT}/app/lamad/dist/lamad/browser:/lamad/ lamad-spa:server:${WORKSPACE_ROOT}/app/lamad/dist/lamad/server:/lamad/}"
WORK_DIR="${APP_RELEASE_WORK_DIR:-$(mktemp -d)}"

# shellcheck source=lib/bundle-zip.sh
. "${BUNDLE_ZIP_LIB:-${SCRIPT_DIR}/lib/bundle-zip.sh}"

# One JSON question, answered by node reading the body from a FILE (never a
# shell variable — a 2 MiB body is not an argv). Prints the answer or nothing.
json_ask() {
    local file="$1" expr="$2"
    "${NODE_BIN}" -e '
        const fs = require("fs");
        let body;
        try { body = JSON.parse(fs.readFileSync(process.argv[1], "utf8")); } catch { process.exit(0); }
        const answer = (new Function("b", "return (" + process.argv[2] + ");"))(body);
        if (answer !== undefined && answer !== null) process.stdout.write(String(answer));
    ' "$file" "$expr"
}

curl_get() {
    local url="$1" out="$2"
    curl -sS -o "$out" -w '%{http_code}' -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
        --max-time "${APP_RELEASE_MAX_TIME:-30}" "$url" 2>/dev/null || echo "000"
}

# ── 1. zip every (app, kind) with the one recipe ─────────────────────────────
declare -a APP_ARGS=()
declare -a OUR_SHAS=()
for spec in ${BUNDLES}; do
    IFS=':' read -r slug kind dist mount <<<"${spec}"
    if [ -z "${slug}" ] || [ -z "${kind}" ] || [ -z "${dist}" ]; then
        echo "publish-app-release: bundle spec '${spec}' is not <slug>:<kind>:<dist>[:<mount>]" >&2
        exit 64
    fi
    if ! bundle_zip "${dist}" "${kind}" "${WORK_DIR}/${slug}"; then
        echo "APP-RELEASE-REFUSED reason=zip slug=${slug} kind=${kind} — the SDK's package checks refused ${dist}; nothing was uploaded" >&2
        exit 2
    fi
    APP_ARGS+=(--app-artifact "${slug}:${kind}:${BUNDLE_ZIP_ARCHIVE}")
    if [ -n "${mount}" ]; then
        # One --app-mount per slug; the packager refuses a repeat only when it disagrees.
        case " ${APP_ARGS[*]} " in
            *" --app-mount ${slug}="*) : ;;
            *) APP_ARGS+=(--app-mount "${slug}=${mount}") ;;
        esac
    fi
    OUR_SHAS+=("${BUNDLE_ZIP_HASH#sha256-}")
    echo "  · ${slug} (${kind}): ${BUNDLE_ZIP_HASH} (${BUNDLE_ZIP_SIZE})"
done
OUR_SET="$(printf '%s\n' "${OUR_SHAS[@]}" | sort | tr '\n' ' ')"

# ── 2. the channel behind the doorway ───────────────────────────────────────
channel_json="${WORK_DIR}/channel.json"
channel_status="$(curl_get "${DOORWAY}/db/content/${CHANNEL_ID}" "${channel_json}")"
case "${channel_status}" in
    200) : ;;
    404)
        echo "APP-RELEASE-REFUSED reason=channel-absent channel=${CHANNEL_ID} — a steward runs 'release-ceremony.ts channel create ${CHANNEL_ID}' once before CI publishes on it" >&2
        exit 2 ;;
    *)
        echo "APP-RELEASE-REFUSED reason=channel-unreadable channel=${CHANNEL_ID} status=${channel_status} via ${DOORWAY}" >&2
        exit 2 ;;
esac

head_json="${WORK_DIR}/head.json"
head_status="$(curl_get "${DOORWAY}/db/content/${CHANNEL_ID}/head" "${head_json}")"
current_head=""
if [ "${head_status}" = "200" ]; then
    current_head="$(json_ask "${head_json}" 'b.headActionHash')"
fi

# Idempotent by CONTENT: the current release carries exactly these digests.
current_set="$(json_ask "${channel_json}" '(() => { const m = (b.metadata && b.metadata.manifest) || null; if (!m || m.artifactClass !== "app-bundle") return null; return m.artifacts.map(a => a.sha256).sort().join(" ") + " "; })()')"
if [ -n "${current_set}" ] && [ "${current_set}" = "${OUR_SET}" ]; then
    echo "APP-RELEASE-CURRENT channel=${CHANNEL_ID} release=${current_head:-unknown}"
    exit 0
fi

# ── 3. package: four PUTs to ONE doorway, each round-tripped ────────────────
declare -a LINEAGE=()
if [ -n "${current_head}" ] && [ -n "$(json_ask "${channel_json}" '(b.metadata && b.metadata.kind === "release-manifest") ? "yes" : null')" ]; then
    LINEAGE=(--lineage-parent "${current_head}")
else
    LINEAGE=(--first-release)
fi
if ! "${TSX_BIN}" "${REPO_ROOT}/genesis/a2o/scripts/epr-release-package.ts" \
    --artifact-class app-bundle \
    --channel-id "${CHANNEL_ID}" \
    "${APP_ARGS[@]}" \
    --soak-secs "${APP_RELEASE_SOAK_SECS:-60}" \
    --attestation-threshold "${APP_RELEASE_ATTESTATION_THRESHOLD:-1}" \
    --declared-reach commons \
    --builder-agent "${BUILD_TAG:-${USER:-ci}@${HOSTNAME:-ci}}" \
    --toolchain "node $("${NODE_BIN}" --version 2>/dev/null || echo unknown)" \
    --peer "${DOORWAY}" --put-via doorway \
    "${LINEAGE[@]}" \
    --strict \
    --out "${MANIFEST_OUT}"; then
    echo "APP-RELEASE-REFUSED reason=package channel=${CHANNEL_ID} — see the packager's output above" >&2
    exit 2
fi

# ── 4. publish at staging through the doorway ───────────────────────────────
publish_out="${MANIFEST_OUT}.publish.json"
if ! "${TSX_BIN}" "${REPO_ROOT}/genesis/a2o/scripts/release-ceremony.ts" publish "${MANIFEST_OUT}" \
    --transport doorway --doorway "${DOORWAY}" > "${publish_out}"; then
    echo "APP-RELEASE-REFUSED reason=publish channel=${CHANNEL_ID} — see the ceremony's output above" >&2
    exit 2
fi
release_cid="$(json_ask "${publish_out}" 'b.releaseCid')"
echo "APP-RELEASE-PUBLISHED channel=${CHANNEL_ID} release=${release_cid:-unknown}"
