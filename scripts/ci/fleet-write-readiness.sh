#!/bin/bash
# fleet-write-readiness.sh — can the fleet take this deploy's writes RIGHT NOW?
# A PRECONDITION that answers in seconds, never a wait (native-delivery sprint
# Lane A1, 2026-09-24).
#
# WHY. App builds #1719-#1725 spent ~12 pipeline-hours and delivered nothing:
# the App pipeline (dependsOn elohim-edge) is dispatched the minute an edge roll
# ends, straight into the fleet's post-roll not-ready window (~20-120 min), and
# stage-spa-blob.sh then WAITED that window out from inside the pipeline. Waiting
# is measurement-by-blocking. This script asks once per doorway, names the face
# of "not ready" it saw, and exits; the caller refuses the deploy, records a
# deploy intent, and a later EVENT (the fleet became writable) re-dispatches it.
#
# Usage: fleet-write-readiness.sh <doorway-url> [<doorway-url> ...]
#   One pass, two questions per doorway, in order, stopping at the first NO:
#   1. GET /health/serving — the doorway's status-bearing serving probe
#      (doorway routes/health.rs `serving_check`). NOT READY when the body says
#      shedding / degrading, storageServing.status is refused / unreachable,
#      the conductor is blind (rolesDiscovered 0), the warmup is empty, or the
#      probe answers 503 for any other reason.
#   2. PUT /admin/seed/blob with a ZERO-byte body — the same side-effect-free
#      re-ask stage-spa-blob.sh uses after a broken PUT. The doorway's admission
#      gate sheds a catching-up doorway before routing (503 {"status":
#      "catching-up"}); a routed probe on a hash the doorway does not hold
#      answers 409 before anything is cached (seed.rs checks the hash before
#      `cache.set`). With READINESS_PROBE_HASH naming a blob the doorway DOES
#      hold, the doorway re-runs the storage forward and reports it, which is
#      how a storage-forward timeout is seen.
#
# Output (stdout, and READINESS_OUT when set), one line per doorway:
#   FLEET-READY <host>
#   FLEET-NOT-READY <host> face=<face> retryAfter=<secs>
#   FLEET-READINESS-UNKNOWN <host> reason=<reason>
# Faces: shedding · degrading · storage-refused · storage-unreachable ·
#   conductor-blind · warmup-empty · not-serving · catching-up · write-shed ·
#   storage-forward-timeout · storage-forward-failed · doorway-unreachable ·
#   doorway-timeout · doorway-5xx
#
# Exit: 0 every doorway ready · 3 any doorway NOT READY · 2 usage, or an answer
#   this script cannot judge (unparseable body, unexpected status, unauthorized
#   probe) and no doorway was NOT READY. It NEVER sleeps and never retries.
#
# Env:
#   STORAGE_API_KEY_ADMIN  X-API-Key for the write probe (a gated, non-DEV_MODE
#                          doorway answers 401 without it — reported, exit 2).
#   READINESS_PROBE_HASH   X-Blob-Hash for the zero-byte probe (default: a
#                          never-held all-zero sha256, which asks the admission
#                          gate only and cannot be cached).
#   READINESS_MAX_TIME     per-request bound in seconds (default 20). The
#                          doorway's own storage probe is bounded well inside it.
#   READINESS_DEFAULT_RETRY_AFTER
#                          retryAfter reported when the doorway advertised none
#                          (default 60).
#   READINESS_JSON_NODE    interpreter for the ONE JSON question per answer
#                          (default `node` — jq is not in the CI-builder image and
#                          python is not allowed on this path; scripts/ci/.epr-meta).
#   READINESS_OUT          also write the per-doorway lines to this file.
#   DEPLOY_INTENT_OUT      on NOT READY, write the deploy intent JSON here:
#                          {commit, env, doorway, face, retryAfter, doorways,
#                          notReady[], bundles[{slug,kind,sha256}]}. An
#                          Ephemeral CI artifact, reconstructable from the build.
#   DEPLOY_INTENT_COMMIT, DEPLOY_INTENT_ENV
#   DEPLOY_INTENT_BUNDLES  whitespace-separated `slug:kind:dist-dir` triples;
#                          each sha256 is the archive CID lib/bundle-zip.sh
#                          produces — the same bytes stage-spa-blob.sh uploads.
#
# Requires: bash, coreutils, curl, node (JSON + the SDK packager for intents).
set -euo pipefail

if [ "$#" -lt 1 ]; then
    echo "usage: fleet-write-readiness.sh <doorway-url> [<doorway-url> ...]" >&2
    exit 2
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
JSON_NODE="${READINESS_JSON_NODE:-node}"
MAX_TIME="${READINESS_MAX_TIME:-20}"
DEFAULT_RETRY_AFTER="${READINESS_DEFAULT_RETRY_AFTER:-60}"
PROBE_HASH="${READINESS_PROBE_HASH:-sha256-0000000000000000000000000000000000000000000000000000000000000000}"

TMP_DIR="$(mktemp -d 2>/dev/null)" || TMP_DIR=""
if [ -z "${TMP_DIR}" ] || [ ! -d "${TMP_DIR}" ]; then
    echo "fleet-write-readiness: could not create a temporary directory" >&2
    exit 2
fi
trap 'rm -rf "${TMP_DIR}"' EXIT

if ! command -v "${JSON_NODE}" >/dev/null 2>&1; then
    echo "fleet-write-readiness: '${JSON_NODE}' is not executable — no answer can be judged" >&2
    exit 2
fi

# ONE question about the ORIGINAL response bytes, answered inside node with a
# fixed token (the stage-spa-blob.sh discipline: a body copied through a shell
# variable loses NULs and trailing newlines, and a grep cannot see nesting).
# Rejects >1 MiB, NUL, invalid UTF-8, non-JSON and non-object roots as BAD.
#   health <file> <status>  -> OK | NOT <face> | BAD
#   put <file> <status>     -> OK | NOT <face> | BAD | UNKNOWN <reason>
#   retry <file>            -> the body's own top-level retryAfter (<=6 digits) or nothing
JSON_PROGRAM='
const fs = require("fs");
const [op, file, status] = process.argv.slice(1);
const say = (s) => { process.stdout.write(s); process.exit(0); };
const own = (x, k) => x !== null && typeof x === "object" && !Array.isArray(x)
  && Object.prototype.hasOwnProperty.call(x, k);
let o = null;
try {
  const b = fs.readFileSync(file);
  if (b.length <= 1048576 && !b.includes(0)) {
    const v = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(b));
    if (v !== null && typeof v === "object" && !Array.isArray(v)) o = v;
  }
} catch (e) { o = null; }
if (op === "retry") {
  if (!o || !own(o, "retryAfter")) say("");
  const v = o.retryAfter;
  const s = typeof v === "number" && Number.isInteger(v) && v >= 0 ? String(v)
    : typeof v === "string" ? v : "";
  say(/^[0-9]{1,6}$/.test(s) ? s : "");
}
if (op === "health") {
  if (!o) say("BAD");
  if (own(o, "shedding") && o.shedding === true) say("NOT shedding");
  if (own(o, "degrading") && o.degrading === true) say("NOT degrading");
  if (own(o, "storageServing") && own(o.storageServing, "status")) {
    const s = o.storageServing.status;
    if (s === "refused") say("NOT storage-refused");
    if (s === "unreachable") say("NOT storage-unreachable");
  }
  if (own(o, "rolesDiscovered") && o.rolesDiscovered === 0) say("NOT conductor-blind");
  if (own(o, "warmupEmpty") && o.warmupEmpty === true) say("NOT warmup-empty");
  if (status === "503") say("NOT not-serving");
  if (typeof o.shedding !== "boolean" || typeof o.degrading !== "boolean") say("BAD");
  say("OK");
}
if (op === "put") {
  if (status === "503" || status === "429") {
    say(own(o, "status") && o.status === "catching-up" ? "NOT catching-up" : "NOT write-shed");
  }
  if (status === "409") say("OK");
  if (status === "401" || status === "403") say("UNKNOWN probe-unauthorized");
  if (/^5[0-9][0-9]$/.test(status)) say("NOT doorway-5xx");
  if (!/^2[0-9][0-9]$/.test(status)) say("UNKNOWN probe-http-" + (status || "none"));
  if (!o) say("BAD");
  if (own(o, "forwarded_to_storage") && o.forwarded_to_storage === true) say("OK");
  let err = own(o, "error") && typeof o.error === "string" ? o.error : "";
  if (err.startsWith("could not reach storage to forward the blob")) {
    // The doorway renders the whole transport chain INCLUDING THE URL into this
    // error, so parentheticals and URLs are removed before the cause is read.
    for (let i = 0; i < 32; i++) { const n = err.replace(/\([^()]*\)/g, ""); if (n === err) break; err = n; }
    err = err.replace(/https?:\/\/\S*/g, "");
    if (/timed out|timeout/i.test(err)) say("NOT storage-forward-timeout");
  }
  say("NOT storage-forward-failed");
}
say("BAD");
'

ask() {
    "${JSON_NODE}" -e "${JSON_PROGRAM}" -- "$@" 2>/dev/null || true
}

normalize_uint() {
    case "$1" in ''|*[!0-9]*) return 0 ;; esac
    [ "${#1}" -le 6 ] || return 0
    printf '%s' "$1"
}

# Header first (the doorway sends Retry-After on every failing serving probe and
# on its catching-up envelope), then the body's own field, then the default.
retry_after_of() {
    local headers="$1" body="$2" v=""
    v="$(grep -i '^Retry-After:' "${headers}" 2>/dev/null | tail -1 | tr -d '\r' \
        | sed 's/^[^:]*:[[:space:]]*//')" || v=""
    v="$(normalize_uint "${v}")"
    [ -n "${v}" ] || v="$(normalize_uint "$(ask retry "${body}")")"
    [ -n "${v}" ] || v="${DEFAULT_RETRY_AFTER}"
    printf '%s' "${v}"
}

# curl exit -> face, for a doorway that did not answer at all. During a roll a
# restarting doorway refuses connections; that IS the not-ready window.
transport_face() {
    if [ "$1" -eq 28 ]; then printf 'doorway-timeout'; else printf 'doorway-unreachable'; fi
}

RESULT_LINES=()
NOT_READY=()        # "host face retryAfter"
any_not_ready=0
any_unknown=0

record() {
    RESULT_LINES+=("$1")
    printf '%s\n' "$1"
}

n=0
for url in "$@"; do
    n=$(( n + 1 ))
    base="${url%/}"
    host="${base#*://}"
    hdr="${TMP_DIR}/${n}.health.headers"
    body="${TMP_DIR}/${n}.health.body"
    : > "${hdr}"; : > "${body}"
    rc=0
    status="$(curl -sS --max-time "${MAX_TIME}" -D "${hdr}" -o "${body}" -w '%{http_code}' \
        "${base}/health/serving" 2>"${TMP_DIR}/${n}.health.err")" || rc=$?
    if [ "${rc}" -ne 0 ]; then
        face="$(transport_face "${rc}")"
        echo "  · ${host}: GET /health/serving did not answer (curl ${rc}: $(head -c 300 "${TMP_DIR}/${n}.health.err" 2>/dev/null))" >&2
        record "FLEET-NOT-READY ${host} face=${face} retryAfter=${DEFAULT_RETRY_AFTER}"
        NOT_READY+=("${host} ${face} ${DEFAULT_RETRY_AFTER}"); any_not_ready=1
        continue
    fi
    case "${status}" in
        200|503) verdict="$(ask health "${body}" "${status}")" ;;
        5[0-9][0-9]) verdict="NOT doorway-5xx" ;;
        *) verdict="UNKNOWN health-http-${status:-none}" ;;
    esac
    case "${verdict}" in
        OK) : ;;
        NOT\ *)
            face="${verdict#NOT }"
            ra="$(retry_after_of "${hdr}" "${body}")"
            echo "  · ${host}: /health/serving HTTP ${status}: $(head -c 400 "${body}" 2>/dev/null)" >&2
            record "FLEET-NOT-READY ${host} face=${face} retryAfter=${ra}"
            NOT_READY+=("${host} ${face} ${ra}"); any_not_ready=1
            continue ;;
        UNKNOWN\ *)
            record "FLEET-READINESS-UNKNOWN ${host} reason=${verdict#UNKNOWN }"; any_unknown=1
            continue ;;
        *)
            echo "  · ${host}: /health/serving HTTP ${status} body is not the serving contract: $(head -c 400 "${body}" 2>/dev/null)" >&2
            record "FLEET-READINESS-UNKNOWN ${host} reason=health-unparseable"; any_unknown=1
            continue ;;
    esac

    hdr="${TMP_DIR}/${n}.put.headers"
    body="${TMP_DIR}/${n}.put.body"
    : > "${hdr}"; : > "${body}"
    rc=0
    status="$(curl -sS --max-time "${MAX_TIME}" -D "${hdr}" -o "${body}" -w '%{http_code}' -X PUT \
        -H 'Content-Type: application/zip' \
        -H "X-Blob-Hash: ${PROBE_HASH}" \
        -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
        --data-binary '' "${base}/admin/seed/blob" 2>"${TMP_DIR}/${n}.put.err")" || rc=$?
    if [ "${rc}" -ne 0 ]; then
        face="$(transport_face "${rc}")"
        echo "  · ${host}: zero-byte PUT /admin/seed/blob did not answer (curl ${rc}: $(head -c 300 "${TMP_DIR}/${n}.put.err" 2>/dev/null))" >&2
        record "FLEET-NOT-READY ${host} face=${face} retryAfter=${DEFAULT_RETRY_AFTER}"
        NOT_READY+=("${host} ${face} ${DEFAULT_RETRY_AFTER}"); any_not_ready=1
        continue
    fi
    case "${status}" in [0-9][0-9][0-9]) : ;; *) status="" ;; esac
    verdict="$(ask put "${body}" "${status}")"
    case "${verdict}" in
        OK) record "FLEET-READY ${host}" ;;
        NOT\ *)
            face="${verdict#NOT }"
            ra="$(retry_after_of "${hdr}" "${body}")"
            echo "  · ${host}: zero-byte PUT HTTP ${status}: $(head -c 400 "${body}" 2>/dev/null)" >&2
            record "FLEET-NOT-READY ${host} face=${face} retryAfter=${ra}"
            NOT_READY+=("${host} ${face} ${ra}"); any_not_ready=1 ;;
        UNKNOWN\ *)
            record "FLEET-READINESS-UNKNOWN ${host} reason=${verdict#UNKNOWN }"; any_unknown=1 ;;
        *)
            record "FLEET-READINESS-UNKNOWN ${host} reason=probe-unparseable"; any_unknown=1 ;;
    esac
done

if [ -n "${READINESS_OUT:-}" ]; then
    if ! printf '%s\n' "${RESULT_LINES[@]}" > "${READINESS_OUT}" 2>/dev/null; then
        echo "  ⊘ WARN: could not write READINESS_OUT='${READINESS_OUT}'" >&2
    fi
fi

# --- the deploy intent (NOT READY only) --------------------------------------
json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="$(printf '%s' "${s}" | tr -d '\000-\037')"
    printf '"%s"' "${s}"
}

write_intent() {
    local out="$1" first host face ra item slug kind dist rest sep="" bundles="" not_ready="" doorways=""
    shift
    read -r host face ra <<< "${NOT_READY[0]}"
    for item in "${NOT_READY[@]}"; do
        local h f r
        read -r h f r <<< "${item}"
        not_ready+="${sep}{\"doorway\":$(json_str "${h}"),\"face\":$(json_str "${f}"),\"retryAfter\":${r}}"
        sep=","
    done
    sep=""
    for first in "$@"; do
        doorways+="${sep}$(json_str "${first%/}")"; sep=","
    done
    sep=""
    if [ -n "${DEPLOY_INTENT_BUNDLES:-}" ]; then
        # shellcheck source=lib/bundle-zip.sh
        . "${HERE}/lib/bundle-zip.sh"
        local i=0
        for item in ${DEPLOY_INTENT_BUNDLES}; do
            i=$(( i + 1 ))
            slug="${item%%:*}"; rest="${item#*:}"
            kind="${rest%%:*}"; dist="${rest#*:}"
            if bundle_zip "${dist}" "${kind}" "${TMP_DIR}/bundle-${i}" >&2; then
                bundles+="${sep}{\"slug\":$(json_str "${slug}"),\"kind\":$(json_str "${kind}"),\"sha256\":$(json_str "${BUNDLE_ZIP_HASH}")}"
            else
                echo "  ⊘ WARN: ${slug} (${kind}): could not package ${dist} for the intent — sha256 recorded as null" >&2
                bundles+="${sep}{\"slug\":$(json_str "${slug}"),\"kind\":$(json_str "${kind}"),\"sha256\":null}"
            fi
            sep=","
        done
    fi
    mkdir -p "$(dirname "${out}")" 2>/dev/null || true
    {
        printf '{"kind":"deploy-intent","version":1,'
        printf '"commit":%s,' "$(json_str "${DEPLOY_INTENT_COMMIT:-}")"
        printf '"env":%s,' "$(json_str "${DEPLOY_INTENT_ENV:-}")"
        printf '"doorway":%s,"face":%s,"retryAfter":%s,' "$(json_str "${host}")" "$(json_str "${face}")" "${ra}"
        printf '"doorways":[%s],"notReady":[%s],"bundles":[%s],' "${doorways}" "${not_ready}" "${bundles}"
        printf '"recordedAt":%s}\n' "$(json_str "$(date -u '+%Y-%m-%dT%H:%M:%SZ')")"
    } > "${out}.tmp.$$" && mv -f "${out}.tmp.$$" "${out}"
}

if [ "${any_not_ready}" -eq 1 ]; then
    if [ -n "${DEPLOY_INTENT_OUT:-}" ]; then
        if write_intent "${DEPLOY_INTENT_OUT}" "$@"; then
            echo "  · deploy intent recorded at ${DEPLOY_INTENT_OUT}" >&2
        else
            echo "  ⊘ WARN: could not write the deploy intent to '${DEPLOY_INTENT_OUT}'" >&2
        fi
    fi
    exit 3
fi
[ "${any_unknown}" -eq 1 ] && exit 2
exit 0
