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
#   One pass, three questions per doorway, in order, stopping at the first NO:
#   1. GET /health/serving — the doorway's status-bearing serving probe
#      (doorway routes/health.rs `serving_check`). NOT READY when the body says
#      shedding / degrading, storageServing.status is refused / unreachable,
#      no conductor role is discovered (rolesDiscovered 0 — cell-not-running),
#      the warmup is empty, or the probe answers 503 for any other reason.
#   2. GET /db/content/<sentinel> with `x-federation-hop: 1` — a local-only read
#      on the route family a deploy's declaration uses (POST /db/content/<slug>/
#      canonical-head). It is the question the doorway's route sheds answer:
#      the upstream breaker, and the household's declared shed (PUT
#      /admin/dev/shed), which exempts /health/* and /admin/* (routes/admin_dev.rs
#      `dev_shed_exempt`) and so is invisible to questions 1 and 3. A 503
#      {"status":"catching-up"} is catching-up; "not held" (404) is ready.
#   3. PUT /admin/seed/blob with a ZERO-byte body — the same side-effect-free
#      re-ask stage-spa-blob.sh uses after a broken PUT. It is the auth and
#      storage-forward question, and it draws from the doorway's WRITE admission
#      pool (a GET draws from the read pool), so a real write-admission shed is
#      seen here (503 {"status":"catching-up"}). A routed probe on a hash the
#      doorway does not hold answers 409 before anything is cached (seed.rs
#      checks the hash before `cache.set`). With READINESS_PROBE_HASH naming a
#      blob the doorway DOES hold, the doorway re-runs the storage forward and
#      reports it, which is how a storage-forward timeout is seen.
#   An answer carrying CellDisabled on question 2 or 3 is cell-not-running.
#
# Output (stdout, and READINESS_OUT when set), one line per doorway, naming it
# by its ORIGIN (scheme://host[:port], lower-cased, default port and path
# dropped) — the household runs two doorways on one host, so a bare hostname
# would name neither:
#   FLEET-READY <origin>
#   FLEET-NOT-READY <origin> face=<face> retryAfter=<secs>
#   FLEET-READINESS-UNKNOWN <origin> reason=<reason>
# Faces: ONE vocabulary, scripts/ci/lib/readiness-faces.json, shared with the
#   household stations (genesis/a2o/steps/dataplane/app-delivery-refuses-fast.
#   helpers.ts). The plan's three — cell-not-running · catching-up ·
#   storage-forward-timeout — are the names stage-spa-blob.sh waits on; the
#   rest name serving regimes and transport answers. The list is read at run
#   time: a face it does not name is reported UNKNOWN, never printed.
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
#   READINESS_PROBE_CONTENT_ID
#                          the content id question 2 reads (default
#                          fleet-write-readiness-probe — never authored).
#   READINESS_FACES_FILE   the face vocabulary (default lib/readiness-faces.json
#                          beside this script).
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
PROBE_CONTENT_ID="${READINESS_PROBE_CONTENT_ID:-fleet-write-readiness-probe}"
FACES_FILE="${READINESS_FACES_FILE:-${HERE}/lib/readiness-faces.json}"

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

# The vocabulary, read once: " face face … " (a space on each side for the
# membership test). No list, no judgement — a probe that cannot say which faces
# exist cannot name one.
FACES=" $("${JSON_NODE}" -e '
const o = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
const f = (Array.isArray(o.faces) ? o.faces : []).map((x) => x && x.face)
  .filter((x) => typeof x === "string" && /^[a-z0-9-]+$/.test(x));
process.stdout.write(f.join(" "));
' -- "${FACES_FILE}" 2>/dev/null || true) "
if [ -z "${FACES// /}" ]; then
    echo "fleet-write-readiness: no face vocabulary at '${FACES_FILE}' — no answer can be named" >&2
    exit 2
fi
listed_face() {
    case "${FACES}" in *" $1 "*) return 0 ;; *) return 1 ;; esac
}

# ONE question about the ORIGINAL response bytes, answered inside node with a
# fixed token (the stage-spa-blob.sh discipline: a body copied through a shell
# variable loses NULs and trailing newlines, and a grep cannot see nesting).
# Rejects >1 MiB, NUL, invalid UTF-8, non-JSON and non-object roots as BAD.
#   health <file> <status>  -> OK | NOT <face> | BAD
#   read <file> <status>    -> OK | NOT <face> | UNKNOWN <reason>
#   put <file> <status>     -> OK | NOT <face> | BAD | UNKNOWN <reason>
# CellDisabled arrives inside a free-text conductor error, so read and put look
# for it in the raw bytes (the stage-spa-blob.sh `not_ready_face` rule).
#   retry <file>            -> the body's own top-level retryAfter (<=6 digits) or nothing
JSON_PROGRAM='
const fs = require("fs");
const [op, file, status] = process.argv.slice(1);
const say = (s) => { process.stdout.write(s); process.exit(0); };
const own = (x, k) => x !== null && typeof x === "object" && !Array.isArray(x)
  && Object.prototype.hasOwnProperty.call(x, k);
let o = null;
let b = null;
try { b = fs.readFileSync(file); } catch (e) { b = null; }
const cellDisabled = b !== null && b.includes("CellDisabled");
try {
  if (b !== null && b.length <= 1048576 && !b.includes(0)) {
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
  if (own(o, "rolesDiscovered") && o.rolesDiscovered === 0) say("NOT cell-not-running");
  if (own(o, "warmupEmpty") && o.warmupEmpty === true) say("NOT warmup-empty");
  if (status === "503") say("NOT not-serving");
  if (typeof o.shedding !== "boolean" || typeof o.degrading !== "boolean") say("BAD");
  say("OK");
}
const shedFace = () => own(o, "status") && o.status === "catching-up" ? "NOT catching-up" : "NOT write-shed";
if (op === "read") {
  if (cellDisabled) say("NOT cell-not-running");
  if (status === "503" || status === "429") say(shedFace());
  if (status === "404" || /^2[0-9][0-9]$/.test(status)) say("OK");
  if (status === "401" || status === "403") say("UNKNOWN read-unauthorized");
  if (/^5[0-9][0-9]$/.test(status)) say("NOT doorway-5xx");
  say("UNKNOWN read-http-" + (status || "none"));
}
if (op === "put") {
  if (cellDisabled) say("NOT cell-not-running");
  if (status === "503" || status === "429") {
    say(shedFace());
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

# A doorway URL -> its origin: scheme://host[:port], lower-cased, userinfo, path,
# query, fragment and the scheme's default port dropped (the WHATWG origin the
# a2o helper compares with). No scheme is read as http, as curl reads it.
origin_of() {
    local u="$1" scheme rest auth
    case "${u}" in
        *://*) scheme="${u%%://*}"; rest="${u#*://}" ;;
        *) scheme=http; rest="${u}" ;;
    esac
    auth="${rest%%[/?#]*}"
    auth="${auth##*@}"
    scheme="${scheme,,}"; auth="${auth,,}"
    case "${scheme}:${auth}" in
        http:*:80) auth="${auth%:80}" ;;
        https:*:443) auth="${auth%:443}" ;;
    esac
    printf '%s://%s' "${scheme}" "${auth}"
}

RESULT_LINES=()
NOT_READY=()        # "origin face retryAfter"
any_not_ready=0
any_unknown=0

record() {
    RESULT_LINES+=("$1")
    printf '%s\n' "$1"
}

# One bounded request; the answer lands in LEG_STATUS (three digits or empty),
# LEG_RC (curl exit), LEG_HDR, LEG_BODY and LEG_ERR.
leg() {
    local n="$1" name="$2"
    shift 2
    LEG_HDR="${TMP_DIR}/${n}.${name}.headers"
    LEG_BODY="${TMP_DIR}/${n}.${name}.body"
    LEG_ERR="${TMP_DIR}/${n}.${name}.err"
    : > "${LEG_HDR}"; : > "${LEG_BODY}"
    LEG_RC=0
    LEG_STATUS="$(curl -sS --max-time "${MAX_TIME}" -D "${LEG_HDR}" -o "${LEG_BODY}" \
        -w '%{http_code}' "$@" 2>"${LEG_ERR}")" || LEG_RC=$?
    case "${LEG_STATUS}" in [0-9][0-9][0-9]) : ;; *) LEG_STATUS="" ;; esac
}

# Settle one leg's verdict for <origin>. Returns 0 when the leg said OK (ask the
# next question) and 1 once this doorway's line is recorded. A transport failure
# is not ready: during a roll a restarting doorway refuses connections, and that
# IS the not-ready window.
settle() {
    local origin="$1" what="$2" verdict="$3" unparseable="$4" face ra
    if [ "${LEG_RC}" -ne 0 ]; then
        echo "  · ${origin}: ${what} did not answer (curl ${LEG_RC}: $(head -c 300 "${LEG_ERR}" 2>/dev/null))" >&2
        if [ "${LEG_RC}" -eq 28 ]; then verdict="NOT doorway-timeout"; else verdict="NOT doorway-unreachable"; fi
        : > "${LEG_HDR}"; : > "${LEG_BODY}"
    fi
    case "${verdict}" in
        OK) return 0 ;;
        NOT\ *)
            face="${verdict#NOT }"
            if ! listed_face "${face}"; then
                echo "  ⊘ ${origin}: ${what} named face '${face}', which ${FACES_FILE} does not list" >&2
                record "FLEET-READINESS-UNKNOWN ${origin} reason=unlisted-face-${face}"; any_unknown=1
                return 1
            fi
            ra="$(retry_after_of "${LEG_HDR}" "${LEG_BODY}")"
            [ "${LEG_RC}" -ne 0 ] || echo "  · ${origin}: ${what} HTTP ${LEG_STATUS}: $(head -c 400 "${LEG_BODY}" 2>/dev/null)" >&2
            record "FLEET-NOT-READY ${origin} face=${face} retryAfter=${ra}"
            NOT_READY+=("${origin} ${face} ${ra}"); any_not_ready=1 ;;
        UNKNOWN\ *)
            record "FLEET-READINESS-UNKNOWN ${origin} reason=${verdict#UNKNOWN }"; any_unknown=1 ;;
        *)
            echo "  · ${origin}: ${what} HTTP ${LEG_STATUS} body is not the contract: $(head -c 400 "${LEG_BODY}" 2>/dev/null)" >&2
            record "FLEET-READINESS-UNKNOWN ${origin} reason=${unparseable}"; any_unknown=1 ;;
    esac
    return 1
}

n=0
for url in "$@"; do
    n=$(( n + 1 ))
    base="${url%/}"
    origin="$(origin_of "${base}")"

    leg "${n}" health "${base}/health/serving"
    case "${LEG_STATUS}" in
        200|503) verdict="$(ask health "${LEG_BODY}" "${LEG_STATUS}")" ;;
        5[0-9][0-9]) verdict="NOT doorway-5xx" ;;
        *) verdict="UNKNOWN health-http-${LEG_STATUS:-none}" ;;
    esac
    settle "${origin}" "GET /health/serving" "${verdict}" health-unparseable || continue

    leg "${n}" read -H 'x-federation-hop: 1' -H 'Accept: application/json' \
        "${base}/db/content/${PROBE_CONTENT_ID}"
    settle "${origin}" "GET /db/content/${PROBE_CONTENT_ID}" \
        "$(ask read "${LEG_BODY}" "${LEG_STATUS}")" read-unparseable || continue

    leg "${n}" put -X PUT \
        -H 'Content-Type: application/zip' \
        -H "X-Blob-Hash: ${PROBE_HASH}" \
        -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
        --data-binary '' "${base}/admin/seed/blob"
    settle "${origin}" "zero-byte PUT /admin/seed/blob" \
        "$(ask put "${LEG_BODY}" "${LEG_STATUS}")" probe-unparseable || continue

    record "FLEET-READY ${origin}"
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
