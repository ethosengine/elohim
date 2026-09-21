#!/bin/bash
# stage-spa-blob.sh — upload ONE pillar-EPR browser bundle and (optionally)
# PATCH it onto its content row. Extracted verbatim from the Jenkinsfile's
# stageSpaBlobs helper (2026-06-10) when the inline heredoc pushed the CPS
# method past the JVM 64KB MethodTooLargeException limit — the bash lives
# here, the Jenkinsfile keeps a one-line call. See Jenkinsfile
# "STAGE HELPER METHODS" for the pillar-EPR decomposition rationale.
#
# Usage: stage-spa-blob.sh <dist-dir> <slug> <doorway-epr-url> [kind]
#   kind: "browser" (default) or "server"
# Env:   STORAGE_API_KEY_ADMIN  admin key for the PATCH+verify step
#        DO_PATCH               "1" to PATCH+verify, anything else skips (WARN)
#        STAGE_BLOB_BUDGET_SECS total wall-clock retry budget per host+leg
#                               (default 360s) — the head PATCH ladder honors
#                               an advertised retryAfter (JSON body field or
#                               Retry-After header, the doorway's
#                               catching-up shed envelope carries both) and
#                               keeps re-offering within this budget instead
#                               of giving up inside the single shed window it
#                               was told about (elohim #1710, 2026-09-13).
#        STAGE_BLOB_ATTEMPTS    safety-net CEILING on attempt count, on top
#                               of the budget above (default 60 — well above
#                               what either cadence needs to fill 360s). A
#                               caller that wants the old fast-fail
#                               count-only behaviour can still pass a low
#                               value, e.g. STAGE_BLOB_ATTEMPTS=3.
#        STAGE_CELL_READY_BUDGET_SECS
#                               SEPARATE wall-clock budget for the NOT-READY
#                               window (see the three faces below), default
#                               7200s = 2h. It is ONE DEADLINE FOR THE WHOLE
#                               RUN, stamped by the first not-ready answer from
#                               any host on any leg and never reset — not a
#                               per-host, per-leg or per-window budget. 0
#                               restores the old immediately-structural
#                               behaviour. See the not-ready block below for
#                               the two dated measurements that justify it.
#        STAGE_CELL_READY_POLL_SECS
#                               poll cadence inside that window (default 60s).
#        STAGE_CELL_READY_STATE_DIR
#                               where that ONE run-level deadline is kept, in a
#                               single `run-deadline` file (default
#                               ${WORKSPACE:-${TMPDIR:-/tmp}}/.stage-cell-ready,
#                               plus /${BUILD_TAG} when Jenkins sets one). The
#                               deadline is shared by the legs of ONE run and by
#                               nothing else, so a caller that orchestrates
#                               several legs outside Jenkins MUST pass a fresh
#                               dir per run.
#        STAGE_JSON_NODE        interpreter used to answer ONE question about a
#                               response body (default `node`, which this script
#                               already needs for the SDK packaging step). With
#                               no interpreter nothing JSON-based classifies and
#                               no blob forward is confirmed — the raw-text
#                               CellDisabled face still works.
#        STAGE_HARD_TIMEOUT_SECS
#                               THE COMPLETION BOUND for this invocation, in
#                               seconds (default readiness deadline + transport
#                               budget + 1500). The readiness deadline below is
#                               a RETRY-START cutoff and bounds no request; this
#                               does, by re-execing the script under coreutils
#                               `timeout`. 0 disables it.
set -euo pipefail

DIST_DIR="$1"
SLUG="$2"
DOORWAY_EPR_URL="$3"
KIND="${4:-browser}"
DO_PATCH="${DO_PATCH:-0}"
ATTEMPTS="${STAGE_BLOB_ATTEMPTS:-60}"
STAGE_BUDGET_SECS="${STAGE_BLOB_BUDGET_SECS:-360}"
# Set by stage_once when a shed envelope advertises a retryAfter (seconds),
# NORMALISED where it is read — see normalize_retry_after. Read and cleared by
# the outer retry loop below.
RETRY_AFTER_HINT=""
# Set by stage_once when a write met one of the not-ready faces, so the outer
# loop can name the last answer in the readiness log without re-issuing the
# call.
CELL_STATUS=""
CELL_BODY=""
CELL_FACE=""
CELL_RETRY_AFTER=""

# THE NOT-READY WINDOW — three faces, ONE run-level deadline (2026-09-21).
#
# HISTORY, because the shape of the bound is the whole lesson.
#   1. PER-LEG. Every leg met the post-roll window alone and spent its own 360s
#      transport budget on it. App #1712 burned 63 minutes across eight
#      combinations and authored no head.
#   2. PER-HOST-WITH-CLEARING (a6b44de0f). A separate readiness budget, shared
#      per doorway host, cleared on recovery. Better, but a recovery replenished
#      the whole budget, so a run could spend 2h per host, twice, and the
#      "45 min" in the header bounded nothing a caller could reason about.
#   3. RUN-LEVEL DEADLINE (here). ONE stamp for the whole run, written by the
#      first not-ready answer from any host on any leg and NEVER reset. A second
#      window in the same run inherits what is left. That is the only shape in
#      which the number in STAGE_CELL_READY_BUDGET_SECS is the run's ceiling on
#      waiting rather than a per-something multiplier.
#
# THREE FACES, because "not ready" reached the wire three different ways and
# only the first was ever classified:
#   - cell-not-running: the body carries `CellDisabled` — an installed cell is
#     not in the conductor's `running_cells` map (conductor.rs:1663). A STATE,
#     not a cause and not a cure.
#   - catching-up: HTTP 503 whose top-level JSON `status` is "catching-up" —
#     the doorway declaring ITSELF shedding writes. Not 429, and not a 503
#     without that field: those stay on the transport ladder exactly as before.
#   - storage-forward-timeout: a blob PUT answered 200 with
#     `forwarded_to_storage` not true and a top-level `error` that starts
#     "could not reach storage to forward the blob" whose transport CAUSE names
#     a timeout. That is a forward that timed out — availability unknown; it is
#     retried on the readiness deadline, never read as "storage is up".
#
# THE TWO DATED MEASUREMENTS behind the 7200s default:
#   - 2026-09-21 (own-cell CellDisabled): a local-household conductor answered
#     CellDisabled for up to ~11min after start and then succeeded untouched;
#     on the live fleet (Loki, 72h, three restarts) matthew's and adam's
#     own-cell CellDisabled lines stopped 0-4min after that conductor's
#     "Conductor ready." line, and "Conductor ready." itself lands ~20-45min
#     after a roll.
#   - 2026-09-21 (app #1715, started right after a fleet roll): ZERO
#     CellDisabled. Instead every blob PUT answered 200 with
#     `"forwarded_to_storage":false` and a storage-forward TIMEOUT, and every
#     head PATCH was shed 503 {"status":"catching-up","retryAfter":30,...}. The
#     build ran 103 minutes and deployed nothing. Two hours after the roll both
#     doorways reported shedding:false.
# So the post-roll not-ready window has three faces and lasts ~100-120min, and
# 7200s is the first round number that covers the measured worst case.
#
# Exhausting the deadline is a MEASUREMENT, not a diagnosis — what it licenses
# is reading the doorway's /health/serving and the storage peer's state, which
# this script cannot see from the far side of a doorway.
CELL_READY_BUDGET_SECS="${STAGE_CELL_READY_BUDGET_SECS:-7200}"
CELL_READY_POLL_SECS="${STAGE_CELL_READY_POLL_SECS:-60}"

# THE COMPLETION BOUND — and it is NOT the readiness deadline (2026-09-21).
#
# Everything else in this file is a RETRY-START cutoff: it decides whether a new
# offer may BEGIN, and says nothing about when one ENDS. This script issues its
# requests without a per-request timeout (deliberately — a per-operation
# `--max-time` weave was tried and reviewed off), so one stalled connection, or a
# stalled deliverability gate, can outlive every budget here. A CI delivery stage
# needs an independently enforced bound, and it has to live in THIS file: the
# root Jenkinsfile is at its CPS bytecode ceiling and must not grow a wrapper per
# call site.
#
# So the script bounds ITSELF: it re-execs once under coreutils `timeout`,
# SIGTERM at the bound and SIGKILL 30s later. Honest worst cases:
#   per invocation: STAGE_HARD_TIMEOUT_SECS. Hard, enforced by signal.
#   per run of N legs: N x that in theory. Realistically one readiness deadline
#   B plus the sum of the per-leg transport budgets, because B is stamped once
#   for the whole run and every later leg inherits only what is left of it.
# The default is B + transport + 1500s — the browser deliverability gate's ~925s
# six-attempt ladder plus slack. No `timeout` on PATH is a WARN, not a refusal:
# running unbounded is what every version before this one did.
#
# ONE decision about `timeout`, made here and shared by the wrapper below AND by
# the JSON parser helpers further down. They used to ask separately, so a runner
# without `timeout` warned that it would continue unbounded and then failed every
# healthy upload, because the parser still invoked `timeout` unconditionally.
STAGE_TIMEOUT_OK=0
if command -v timeout >/dev/null 2>&1; then
    STAGE_TIMEOUT_OK=1
else
    echo "  ⊘ WARN: coreutils 'timeout' is not on PATH — this invocation runs with NO completion bound (only the retry-start cutoffs below) and the JSON parser runs unbounded. Both are what every version before this one did." >&2
fi

if [ "${STAGE_HARD_TIMEOUT_ACTIVE:-0}" != "1" ] && [ "${STAGE_TIMEOUT_OK}" -eq 1 ]; then
    STAGE_HARD_TIMEOUT_SECS="${STAGE_HARD_TIMEOUT_SECS:-$(( CELL_READY_BUDGET_SECS + STAGE_BUDGET_SECS + 1500 ))}"
    if [ "${STAGE_HARD_TIMEOUT_SECS}" -gt 0 ] 2>/dev/null; then
        # Resolve OUR path: this script is normally invoked as
        # `bash scripts/ci/stage-spa-blob.sh …`, so $0 is a relative path and
        # the re-exec must not depend on the child's working directory.
        stage_self="$0"
        case "${stage_self}" in
            /*) : ;;
            *) stage_self="$(cd "$(dirname "${stage_self}")" && pwd)/$(basename "${stage_self}")" ;;
        esac
        export STAGE_HARD_TIMEOUT_ACTIVE=1
        export STAGE_HARD_TIMEOUT_SECS
        # BACKGROUND + FORWARD. A caller that sends SIGTERM to the PID it
        # launched must actually stop the staging work; with a foreground
        # supervisor this shell died and `timeout`'s child kept uploading and
        # PATCHing. `<&0` keeps stdin inherited (bash otherwise points an async
        # command at /dev/null); stdout and stderr are inherited as usual.
        timeout --kill-after=30 "${STAGE_HARD_TIMEOUT_SECS}" \
            bash "${stage_self}" "$@" <&0 &
        hard_pid=$!
        trap 'kill -TERM "${hard_pid}" 2>/dev/null || true' TERM
        trap 'kill -INT "${hard_pid}" 2>/dev/null || true' INT
        trap 'kill -HUP "${hard_pid}" 2>/dev/null || true' HUP
        # `wait` returns 128+n when a trap fires rather than when the child
        # exits, so loop until the child is really gone or its status is real.
        hard_rc=0
        while :; do
            hard_rc=0
            wait "${hard_pid}" || hard_rc=$?
            if [ "${hard_rc}" -gt 128 ] && kill -0 "${hard_pid}" 2>/dev/null; then
                continue
            fi
            break
        done
        trap - TERM INT HUP
        if [ "${hard_rc}" -eq 124 ] || [ "${hard_rc}" -eq 137 ]; then
            echo "ERROR: [${SLUG}] hard per-invocation bound of ${STAGE_HARD_TIMEOUT_SECS}s reached — a request or the deliverability gate stalled past every budget in this script; host left STALE" >&2
            exit 1
        fi
        exit "${hard_rc}"
    fi
fi

# ONE temp dir for every response body this invocation captures, and ONE exit
# trap that owns it. Response bodies are written to FILES and parsed from those
# ORIGINAL BYTES: a body copied through a shell variable has already lost its
# NULs and its trailing newlines, and `{"forwarded_to_storage":tr<NUL>ue}` —
# invalid JSON on the wire — was read as a confirmed forward once that happened.
# The shell copies below are kept for LOGGING only.
STAGE_TMP_DIR="$(mktemp -d 2>/dev/null)" || STAGE_TMP_DIR=""
if [ -z "${STAGE_TMP_DIR}" ] || [ ! -d "${STAGE_TMP_DIR}" ]; then
    echo "ERROR: [${SLUG}] could not create a temporary directory for response bodies" >&2
    exit 2
fi
# package_dir is filled in further down; naming it here keeps ONE exit trap for
# the whole script (a second `trap … EXIT` would silently replace this one).
package_dir=""
trap 'rm -rf "${STAGE_TMP_DIR}" ${package_dir:+"${package_dir}"}' EXIT
# The deadline is shared by the legs of ONE run and by nothing else. Under
# Jenkins BUILD_TAG makes that scoping automatic; a caller that orchestrates
# several legs outside Jenkins (hc-mesh-prologue.sh, run-mesh-quiesce-stage.sh)
# creates one fresh dir per run and exports it. CELL_READY_DIR_SCOPED records
# which of those we got: the age guard below applies ONLY to the unscoped
# fallback, because in a scoped dir an "old" record still belongs to this run.
CELL_READY_STATE_DIR="${STAGE_CELL_READY_STATE_DIR:-${WORKSPACE:-${TMPDIR:-/tmp}}/.stage-cell-ready}"
CELL_READY_DIR_SCOPED=1
if [ -z "${STAGE_CELL_READY_STATE_DIR:-}" ]; then
    if [ -n "${BUILD_TAG:-}" ]; then
        CELL_READY_STATE_DIR="${CELL_READY_STATE_DIR}/$(printf '%s' "${BUILD_TAG}" | tr -c 'A-Za-z0-9._-' '_')"
    else
        CELL_READY_DIR_SCOPED=0
    fi
fi
CELL_READY_STATE_MAX_AGE_SECS=21600
CELL_WAIT_ENTERED=0   # this invocation has already logged the entering line
# The run's first-not-ready stamp, held in THIS shell as well as on disk. It is
# assigned by cell_ready_stamp_deadline, which the loop-owning shell calls
# directly — never inside a command substitution, whose subshell assignment
# would be lost and would let the next answer restamp the deadline.
CELL_READY_FIRST_SEEN=""
CELL_READY_WARNED=0

cell_ready_stamp() {
    date -u -d "@$1" '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || printf 'epoch %s' "$1"
}

# ONE QUESTION ABOUT THE ORIGINAL RESPONSE BYTES (2026-09-21).
#
# The classes above turn on JSON facts, and neither a grep nor a shell variable
# can carry them. A grep cannot see string boundaries or nesting, so
# `{"error":{"status":"catching-up"}}` and `[{"status":"catching-up"}]` would
# read as top-level declarations. A shell variable is worse: command substitution
# silently drops NUL bytes and trailing newlines, so `{"forwarded_to_storage":
# tr<NUL>ue}` — invalid JSON on the wire — reached the parser as valid `true` and
# a byte-seed reported "storage forward CONFIRMED". Each is a wrong routing
# decision worth up to two hours, or a head declared against bytes nobody holds.
#
# So: curl writes the body to a FILE, and this reads THAT FILE as a Buffer. It
# rejects, in order, more than 1 MiB of BYTES (not characters — 600k `é` is
# 1.2 MB in 600k characters), any NUL byte, anything that is not valid UTF-8,
# anything that is not JSON, any root that is not an object, and any field that
# is not an OWN property. Then it answers the ONE question asked and returns a
# fixed token or a fixed exit status — every comparison happens inside node, so
# no compared value ever passes through the shell.
#
# node, not jq and not python: jq is not in the checked-in CI-builder image and
# python is not on the deploy path (scripts/ci/.epr-meta), while node IS both —
# this script already shells out to it for the SDK packaging step, and the two
# non-Jenkins callers (hc-mesh-prologue.sh, run-mesh-quiesce-stage.sh) run in a
# dev workspace and that same image. STAGE_JSON_NODE names the interpreter so a
# harness can point it at a real node while faking the packaging one.
#
# Arguments ride after `--` so a field named `--version` cannot be read as a node
# option.
JSON_NODE="${STAGE_JSON_NODE:-node}"
JSON_PARSER_OK=0
if command -v "${JSON_NODE}" >/dev/null 2>&1; then
    JSON_PARSER_OK=1
else
    # FAIL CLOSED: with no parser, no answer is classified catching-up or
    # storage-forward-timeout (so a leg reds on the transport ladder rather than
    # waiting out an unread body) and no blob forward is ever confirmed (so a
    # byte-seed can never report success it cannot prove). The raw-text
    # CellDisabled face is unaffected — it is a substring match, not a field.
    echo "  ⊘ WARN: '${JSON_NODE}' is not executable — response bodies cannot be parsed. Only the raw-text CellDisabled face can be recognised, no catching-up or storage-forward-timeout answer will be waited out, and no blob forward can be confirmed; this leg fails closed." >&2
fi

# A PARSER THAT DID NOT RUN IS NOT A NEGATIVE ANSWER. The program answers with
# BOTH a one-character liveness token on stdout and a distinct exit code:
#   `Y` + exit 0  — yes; for the print ops the value follows the Y.
#   `N` + exit 1  — a clean negative ABOUT THE BODY: field absent, wrong type,
#                   value mismatch, or bytes that are not parsable JSON (over
#                   1 MiB, a NUL, not UTF-8, not an object root).
#   anything else — the parser could not run: `timeout` killed it (124), node
#                   crashed, the interpreter is missing. The token is what makes
#                   that distinguishable, because node's own crash exit is 1 too.
# Either way the caller FAILS CLOSED; the difference is that a could-not-run gets
# one explicit diagnostic naming it a runner problem, so an operator does not
# read it as the doorway's answer.
JSON_PROGRAM='
const fs = require("fs");
const LIMIT = 1048576;
const no = () => { process.stdout.write("N"); process.exit(1); };
const [file, op, field, operand] = process.argv.slice(1);
let buf;
try { buf = fs.readFileSync(file); } catch (e) { no(); }
if (buf.length > LIMIT) no();
if (buf.includes(0)) no();
let text;
try { text = new TextDecoder("utf-8", { fatal: true }).decode(buf); } catch (e) { no(); }
let o;
try { o = JSON.parse(text); } catch (e) { no(); }
if (o === null || typeof o !== "object" || Array.isArray(o)) no();
if (!Object.prototype.hasOwnProperty.call(o, field)) no();
const v = o[field];
const yes = (s) => { process.stdout.write("Y" + (s === undefined ? "" : s)); process.exit(0); };
if (op === "is_true") { if (v === true) yes(); no(); }
if (op === "str_eq") { if (typeof v === "string" && v === operand) yes(); no(); }
if (op === "str_prefix") { if (typeof v === "string" && v.startsWith(operand)) yes(); no(); }
if (op === "print_str") { if (typeof v === "string") yes(v); no(); }
if (op === "print_uint") {
  let s = null;
  if (typeof v === "number" && Number.isInteger(v) && v >= 0) s = String(v);
  else if (typeof v === "string") s = v;
  if (s !== null && /^[0-9]{1,6}$/.test(s)) yes(s);
  no();
}
no();
'

# 30s, not 10: the body is at most 1 MiB, so the only way to exceed this is a
# runner so loaded that ten seconds was a coin flip — which is exactly how a
# healthy upload started reporting an unconfirmed forward.
JSON_TIMEOUT_SECS=30

# Prints the program's raw answer (token + value); returns its exit code.
# Runs unbounded when `timeout` is absent, under the single WARN printed above.
json_run() {
    if [ "${STAGE_TIMEOUT_OK}" -eq 1 ]; then
        timeout "${JSON_TIMEOUT_SECS}" "${JSON_NODE}" -e "${JSON_PROGRAM}" -- "$@"
    else
        "${JSON_NODE}" -e "${JSON_PROGRAM}" -- "$@"
    fi
}

json_could_not_run() {
    echo "  ⊘ [${SLUG}] the local JSON parser did not complete (exit $1) while checking '$2' — treating the answer as unconfirmed; this is a runner problem, not the doorway's answer" >&2
    return 0
}

# Exit 0 only on a definite YES. Any other answer — a clean negative, a missing
# file, no parser, or a parser that could not run — is NO.
json_check() {
    local file="$1" rc=0 out=""
    shift
    [ "${JSON_PARSER_OK}" -eq 1 ] || return 1
    [ -f "${file}" ] || return 1
    out="$(json_run "${file}" "$@" 2>/dev/null)" || rc=$?
    case "${out}" in
        Y*) [ "${rc}" -eq 0 ] && return 0 ;;
        N)  [ "${rc}" -eq 1 ] && return 1 ;;
    esac
    json_could_not_run "${rc}" "$1"
    return 1
}

# Print a value node vouched for, or nothing. The printed text is used only for
# the cause heuristic and for a seconds count, never for a classification gate.
json_print() {
    local file="$1" rc=0 out=""
    shift
    [ "${JSON_PARSER_OK}" -eq 1 ] || return 0
    [ -f "${file}" ] || return 0
    out="$(json_run "${file}" "$@" 2>/dev/null)" || rc=$?
    if [ "${rc}" -eq 0 ]; then
        case "${out}" in
            Y*) printf '%s' "${out#Y}"; return 0 ;;
        esac
    elif [ "${rc}" -eq 1 ] && [ "${out}" = "N" ]; then
        return 0
    fi
    json_could_not_run "${rc}" "$1"
    return 0
}

# Exactly the contract the header promises: a blob PUT counts as delivered only
# on an own top-level boolean `true`. The string "true", a spaced `false`, an
# absent field, a NUL-corrupted body and a malformed body all fail.
forward_confirmed() {
    json_check "$1" is_true forwarded_to_storage
}

# The SINGLE place a Retry-After value is normalised, so the same rule governs
# the readiness nap and the ordinary transport sleep. At most 6 digits: bash
# cannot compare more, and a 39-digit value once reached `sleep` verbatim past
# every cap.
normalize_retry_after() {
    local v="$1"
    case "${v}" in ''|*[!0-9]*) return 0 ;; esac
    [ "${#v}" -le 6 ] || return 0
    printf '%s' "${v}"
}

# Header first (the doorway's shed envelope carries both), body second.
read_retry_after() {
    local headers_file="$1" body_file="$2" v=""
    v=$(grep -i '^Retry-After:' "${headers_file}" 2>/dev/null \
        | tail -1 | tr -d '\r' | sed 's/^[^:]*:[[:space:]]*//') || v=""
    v="$(normalize_retry_after "${v}")"
    [ -n "${v}" ] || v="$(json_print "${body_file}" print_uint retryAfter)"
    printf '%s' "${v}"
}

# Remove BALANCED parenthetical segments, innermost first, until none is left.
# One pass of `s/([^()]*)//g` removes only the innermost pair, so
# `connect error (timeout diagnostic (retry disabled)): connection refused`
# still carried the word "timeout" into the match and classified a refused
# connection as a timeout. The loop is bounded; whatever survives 32 passes is
# left alone and simply read as-is.
strip_balanced_parens() {
    local s="$1" prev i=0
    while [ "${i}" -lt 32 ]; do
        prev="${s}"
        s="$(printf '%s' "${s}" | sed 's/([^()]*)//g')"
        [ "${s}" = "${prev}" ] && break
        i=$(( i + 1 ))
    done
    printf '%s' "${s}"
}

# Which face of "not ready yet" is this answer, if any? Prints the face name, or
# nothing when the answer belongs to another class.
# $1 = HTTP status (may be empty when the leg could not read one),
# $2 = response BODY FILE (the original bytes), $3 = the same body as text, for
# the one raw-text match.
not_ready_face() {
    local status="$1" body_file="$2" body="$3" err
    # CellDisabled arrives inside a free-text conductor error, so it is matched
    # in the raw body on purpose — a substring of a message, not a field. Bash's
    # own substring test, not `grep -q`: grep exits at the first match and the
    # producer then takes SIGPIPE, which under `pipefail` made the whole pipeline
    # fail and the face vanish whenever the body ran past the matching line.
    if [[ "${body}" == *CellDisabled* ]]; then
        printf 'cell-not-running'
        return 0
    fi
    if [ "${status}" = "503" ] && json_check "${body_file}" str_eq status catching-up; then
        printf 'catching-up'
        return 0
    fi
    # Deliberately narrow. `forwarded_to_storage:false` alone is not enough, and
    # neither is the word "timeout" anywhere in the body: the doorway renders the
    # whole transport chain INCLUDING THE URL into that error (seed.rs
    # `transport_reason`), so a host named `timeout-storage`, or a DNS failure
    # against one, would otherwise buy a two-hour wait. The gate — field absent
    # or not `true`, plus an exact string prefix — is decided inside node;
    # balanced parentheses and http(s) tokens are then deleted and only what is
    # left is read for the cause. Connection-refused, "no storage_url is
    # configured" and every other named cause stay on the transport ladder, where
    # a persistent failure reds the leg in 360s.
    if ! forward_confirmed "${body_file}" \
       && json_check "${body_file}" str_prefix error 'could not reach storage to forward the blob'; then
        err="$(json_print "${body_file}" print_str error)"
        err="$(strip_balanced_parens "${err}")"
        if printf '%s' "${err}" \
            | sed 's#https\{0,1\}://[^[:space:]]*##g' \
            | grep -qi 'timed out\|timeout'; then
            printf 'storage-forward-timeout'
            return 0
        fi
    fi
    return 0
}

# The one-line human reading of a face, for the log.
not_ready_phrase() {
    case "$1" in
        cell-not-running)
            printf 'the cell this write must go through is not running on the conductor behind this doorway' ;;
        catching-up)
            printf 'the doorway declares itself catching up and is shedding writes' ;;
        storage-forward-timeout)
            printf 'the forward of the bytes to storage timed out — availability unknown; retried on the readiness deadline' ;;
        *)
            printf 'the holder behind this doorway is not ready to take a write' ;;
    esac
}

# ONE record for the whole run — not one per host, and never deleted by a
# recovery.
cell_ready_state_file() {
    printf '%s/run-deadline' "${CELL_READY_STATE_DIR}"
}

cell_ready_warn() {
    [ "${CELL_READY_WARNED}" -eq 0 ] || return 0
    CELL_READY_WARNED=1
    echo "  ⊘ WARN: $1" >&2
    return 0
}

# A stamp this script is willing to do arithmetic on: 1-11 decimal digits (11
# covers epoch seconds well past year 5000 and stays inside bash's signed
# range), and not in the future beyond 5s of clock skew. `99999999999999999999`
# passed the old digits-only read and produced 7.7e18 seconds "remaining"; a
# future stamp simply extended the configured budget. Both are garbage, and
# garbage takes the declared per-invocation fallback.
cell_ready_valid_stamp() {
    local v="$1" now="$2"
    case "${v}" in ''|*[!0-9]*) return 1 ;; esac
    [ "${#v}" -le 11 ] || return 1
    [ "${v}" -le "$(( now + 5 ))" ] || return 1
    return 0
}

cell_ready_read_stamp() {
    sed -n 's/^first_not_ready=\([0-9][0-9]*\)$/\1/p' "$1" 2>/dev/null | head -1
}

# Establish this RUN's first-not-ready stamp, once. Assigns CELL_READY_FIRST_SEEN
# in the caller's shell, so an unwritable dir or an unusable record degrades to a
# per-invocation budget with one loud WARN — never to a restamp on every answer.
# $1 = now (epoch).
cell_ready_stamp_deadline() {
    local now="$1" f persisted="" unusable=0 winner="" tmp=""
    [ -z "${CELL_READY_FIRST_SEEN}" ] || return 0
    f="$(cell_ready_state_file)"
    if [ -f "${f}" ]; then
        persisted="$(cell_ready_read_stamp "${f}")"
        if ! cell_ready_valid_stamp "${persisted}" "${now}"; then
            # Unusable bytes in THIS run's record. Not removed and not replaced:
            # that delete-and-replace is exactly the race that loses a first
            # stamp. Declared per-invocation fallback, one WARN.
            persisted=""
            unusable=1
            cell_ready_warn "the run-deadline record in '${CELL_READY_STATE_DIR}' holds no usable first_not_ready stamp — this run's deadline is held in THIS invocation only, so a later leg may start a fresh ${CELL_READY_BUDGET_SECS}s instead of inheriting what is left"
        elif [ "${CELL_READY_DIR_SCOPED}" -eq 0 ] \
           && [ "$(( now - persisted ))" -gt "${CELL_READY_STATE_MAX_AGE_SECS}" ]; then
            # A COMPLETE record from an earlier run, in the shared fallback dir.
            # Nothing in this run can be racing for it, so removing it and
            # publishing fresh is safe — and is the only way a later run in that
            # dir ever gets a clock.
            echo "  · [${SLUG}] ignoring a run-deadline record older than ${CELL_READY_STATE_MAX_AGE_SECS}s in the UNSCOPED fallback dir '${CELL_READY_STATE_DIR}' (left by an earlier run) — starting this run's deadline now" >&2
            persisted=""
            rm -f "${f}" 2>/dev/null || true
        fi
    fi
    if [ -n "${persisted}" ]; then
        CELL_READY_FIRST_SEEN="${persisted}"
        return 0
    fi
    CELL_READY_FIRST_SEEN="${now}"
    if [ "${unusable}" -eq 0 ] && mkdir -p "${CELL_READY_STATE_DIR}" 2>/dev/null; then
        # PUBLISH A COMPLETE RECORD, ATOMICALLY, then ADOPT THE WINNER.
        # `set -C` + printf was not atomic: it CREATED the file and only then
        # filled it, so another leg could read an empty record, call it garbage,
        # remove it and publish a later stamp — the first stamp lost. Writing the
        # whole record to a temp file in the SAME directory and hard-linking it
        # into place publishes only complete bytes, and `ln` fails rather than
        # overwriting a winner. Losing the link is the common case under a
        # parallel caller and costs nothing: we re-read either way.
        #
        # An UNUSABLE existing record is deliberately NOT deleted and replaced —
        # that delete-and-replace IS the race. It takes the declared
        # per-invocation fallback with the single WARN instead.
        tmp="${CELL_READY_STATE_DIR}/.run-deadline.$$"
        if printf 'first_not_ready=%s\n' "${now}" > "${tmp}" 2>/dev/null; then
            ln "${tmp}" "${f}" 2>/dev/null || true
        fi
        rm -f "${tmp}" 2>/dev/null || true
        winner="$(cell_ready_read_stamp "${f}")"
        if cell_ready_valid_stamp "${winner}" "${now}"; then
            CELL_READY_FIRST_SEEN="${winner}"
            return 0
        fi
    fi
    cell_ready_warn "could not record this run's readiness deadline in '${CELL_READY_STATE_DIR}' — it is held in THIS invocation only, so a later leg of the same run starts a fresh ${CELL_READY_BUDGET_SECS}s instead of inheriting what is left"
    return 0
}

# Seconds left of the run deadline. Only meaningful once it has been stamped.
cell_ready_remaining() {
    printf '%s' "$(( CELL_READY_BUDGET_SECS - ( $(date +%s) - CELL_READY_FIRST_SEEN ) ))"
}

# Cap an ordinary ladder sleep by what is left of an OPEN readiness window. With
# no window open the value is returned unchanged, so behaviour outside a window
# is exactly what it was.
cell_ready_cap_sleep() {
    local want="$1" left
    [ -n "${CELL_READY_FIRST_SEEN}" ] || { printf '%s' "${want}"; return 0; }
    left="$(cell_ready_remaining)"
    [ "${left}" -lt 0 ] && left=0
    [ "${want}" -gt "${left}" ] && want="${left}"
    printf '%s' "${want}"
}

# ONCE A WINDOW IS OPEN, NO NEW ATTEMPT MAY START AT OR AFTER THE DEADLINE — by
# ANY path. The readiness nap is not the only way time passes inside an open
# window: an ORDINARY transport shed sleeps on its own ladder, and the
# DECLARE_ONLY ladder sleeps on a third. Measured before this gate existed, a
# 12s budget meeting `catching-up -> ordinary 503 -> success` dispatched attempts
# at 0, 10 and 40s and returned 0. So every loop that can begin an attempt asks
# here first. With NO window open this is always true and nothing changes.
readiness_may_start() {
    [ -n "${CELL_READY_FIRST_SEEN}" ] || return 0
    [ "$(cell_ready_remaining)" -gt 0 ]
}

# Called with the not-ready answer a write just received. Returns
#   0 -> waited; the caller must RE-OFFER the same idempotent,
#        content-addressed call (no attempt and no transport budget consumed).
#   1 -> this RUN's readiness deadline is spent (or waiting is switched off):
#        the caller fails the leg.
#
# DEADLINE SEMANTICS, stated exactly as implemented: the deadline governs when a
# readiness RE-OFFER may START. It is checked before every attempt
# (readiness_may_start), immediately before each nap and again immediately after,
# and no attempt is ever started at or after expiry. An attempt already in flight
# may finish after the deadline — nothing here bounds a single request; the hard
# per-invocation timeout at the top of this file is what bounds completion.
#
# $1 = leg label, $2 = HTTP status, $3 = body, $4 = face, $5 = advertised
# retryAfter (already normalised, may be empty).
cell_ready_wait() {
    local label="$1" status="$2" body="$3" face="$4" advertised="$5"
    local now elapsed remaining nap

    if [ "${CELL_READY_BUDGET_SECS}" -le 0 ]; then
        echo "  ✗ [${SLUG}] ${label} via ${DOORWAY_EPR_URL} — $(not_ready_phrase "${face}") (face=${face}, HTTP ${status}): ${body}" >&2
        echo "    STAGE_CELL_READY_BUDGET_SECS=0 — the caller switched readiness waiting off, so this is reported without waiting. Host left STALE." >&2
        return 1
    fi

    now="$(date +%s)"
    cell_ready_stamp_deadline "${now}"
    elapsed=$(( now - CELL_READY_FIRST_SEEN ))
    remaining=$(( CELL_READY_BUDGET_SECS - elapsed ))

    if [ "${remaining}" -le 0 ]; then
        cell_ready_exhausted "${label}" "${status}" "${body}" "${face}" "${elapsed}"
        return 1
    fi

    if [ "${CELL_WAIT_ENTERED}" -eq 0 ]; then
        CELL_WAIT_ENTERED=1
        echo "  ⏳ [${SLUG}] ${label} via ${DOORWAY_EPR_URL} — $(not_ready_phrase "${face}") (face=${face}, HTTP ${status}): ${body}" >&2
        echo "    Treating it as the post-roll not-ready window rather than a final answer, on two dated measurements: 2026-09-20 own-cell CellDisabled ends 0-4min after a conductor's 'Conductor ready.' line and ready lands ~20-45min after a roll; 2026-09-21 (app #1715) catching-up and storage-forward timeouts persisted ~100-120min after a roll." >&2
        echo "    Waiting until this RUN's readiness deadline (${CELL_READY_BUDGET_SECS}s from the first not-ready answer at $(cell_ready_stamp "${CELL_READY_FIRST_SEEN}"), poll ${CELL_READY_POLL_SECS}s), shared by every leg and every host of this run and separate from the ${STAGE_BUDGET_SECS}s transport budget. The deadline governs when a re-offer may START; an attempt already in flight may finish past it." >&2
    fi

    # nap = min(poll, advertised retryAfter, remaining), with a 5s floor applied
    # only while there is more than 5s left (a retryAfter of 0 must not become a
    # hot loop, and must not become a full poll interval either).
    nap="${CELL_READY_POLL_SECS}"
    if [ -n "${advertised}" ] && [ "${advertised}" -lt "${nap}" ]; then
        nap="${advertised}"
    fi
    if [ "${nap}" -gt "${remaining}" ]; then
        nap="${remaining}"
    fi
    if [ "${remaining}" -gt 5 ]; then
        [ "${nap}" -lt 5 ] && nap=5
    else
        nap="${remaining}"
    fi
    echo "  … [${SLUG}] still not ready on ${DOORWAY_EPR_URL} (face=${face}${advertised:+, advertised retryAfter=${advertised}s}) — ${elapsed}s waited, ${remaining}s left of ${CELL_READY_BUDGET_SECS}s; re-offering the same content-addressed ${label} in ${nap}s" >&2
    sleep "${nap}"

    # Check again after the nap: a re-offer must not START at or after expiry.
    now="$(date +%s)"
    elapsed=$(( now - CELL_READY_FIRST_SEEN ))
    if [ "$(( CELL_READY_BUDGET_SECS - elapsed ))" -le 0 ]; then
        cell_ready_exhausted "${label}" "${status}" "${body}" "${face}" "${elapsed}"
        return 1
    fi
    return 0
}

cell_ready_exhausted() {
    local label="$1" status="$2" body="$3" face="$4" elapsed="$5"
    echo "  ✗ [${SLUG}] ${label} via ${DOORWAY_EPR_URL} — readiness deadline reached; the holder was still not ready after ${elapsed}s (last face=${face:-none}, budget ${CELL_READY_BUDGET_SECS}s, first not-ready answer of this run $(cell_ready_stamp "${CELL_READY_FIRST_SEEN}"); last answer HTTP ${status}: ${body}); an attempt in flight may have finished past it." >&2
    echo "    That is a measurement, not a diagnosis — read the doorway's /health/serving and the storage peer's state. Host left STALE." >&2
    return 0
}

# DECLARE-ONLY mode (canonical-head propagation). Cross-peer DHT gossip of the
# canonical link can lag or degrade (the F-T19 outbound class), leaving the
# non-authoring peer resolving its own root indefinitely. Declaring the SAME
# target hash through every doorway is idempotent-by-content — staging
# scaffold-over-scaffold with an identical target converges identically — and
# stamps each peer's row eagerly. Resolves the hash from the AUTHORING doorway
# (SOURCE_DOORWAY_URL), POSTs it to THIS doorway.
#
# LOAD-BEARING under R1 (2026-07-31 decision:
# genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md):
# there is no automatic arbitration between two competing declared heads —
# divergence escalates to a fresh authority declaration, and THIS channel is
# today's stand-in for that authority. A silent fan-out failure here would
# leave a doorway stale while the caller reports success, so this leg now
# exits non-zero whenever the declare could NOT be delivered to
# DOORWAY_EPR_URL (source/hash unresolvable, curl transport error, or the
# retry ladder exhausted on 503/429/"not retrievable"). It exits 0 only on
# confirmed propagation (HTTP 2xx). Callers that want the pre-R1 fire-and-forget
# behaviour should invoke with `sh(returnStatus: true, ...)` and only surface
# the failure as UNSTABLE, not a hard build failure (see Jenkinsfile
# authorHeadOnce).
if [ "${DECLARE_ONLY:-0}" = "1" ]; then
    SRC="${SOURCE_DOORWAY_URL:-}"
    if [ -z "${SRC}" ]; then
        echo "  ⚠ DECLARE_ONLY: SOURCE_DOORWAY_URL not set — skipping canonical-head propagation to ${DOORWAY_EPR_URL}" >&2
        exit 1
    fi
    head_hash=$(curl -fSs -H "X-API-Key: ${STORAGE_API_KEY_ADMIN}" \
        "${SRC}/db/content/${SLUG}/head" 2>/dev/null \
        | python3 -c "import sys, json; print(json.load(sys.stdin).get('headActionHash',''))" 2>/dev/null) || head_hash=""
    if [ -z "${head_hash}" ]; then
        echo "  ⚠ DECLARE_ONLY: could not resolve headActionHash from ${SRC}/db/content/${SLUG}/head — skipping propagation to ${DOORWAY_EPR_URL}" >&2
        exit 1
    fi
    # DECLARE-CARRIES-RECORD (sprint-3). Ask the AUTHORING doorway for the head
    # action's full serialized Record and carry it in the declare body. Without
    # it, the receiving conductor's only way to see the target is its local
    # `get` — and on a full-arc fleet (target_arc_factor=1) that cascade
    # short-circuits at authority WITHOUT a network fetch, so a gossip gap reads
    # as permanent absence and NO retry ladder can ever clear it. With it, the
    # receiver re-derives the action hash, author signature, and entry↔action
    # binding in wasm and honors the declaration on evidence rather than gossip.
    #
    # Strictly best-effort and strictly additive: any failure here (old peer
    # without the route, 404, malformed JSON) leaves record_b64 empty and the
    # declare falls back to the exact pre-sprint-3 body. The record is only
    # accepted when the source's own headActionHash matches the hash we are
    # about to declare — a mismatch means the head moved between the two reads,
    # and carrying a record for a different action would be refused anyway.
    #
    # This carried-record extraction is pure sed (bash + coreutils), honoring
    # the scripts/ci/.epr-meta deploy-script invariant — the head read further
    # up falls back to a python3 one-liner, but that is a SEPARATE pre-existing
    # usage, not a license to add python here. Both fields are base64/base64url,
    # so they contain no `"` and need no JSON escaping — sed extraction and
    # printf construction are exact.
    #
    # The two sed patterns are ANCHORED at the start of line (`^{`) so the
    # greedy `.*` prefix binds to the SINGLE top-level object rather than the
    # LAST occurrence of each key: a source that returned a decoy nested
    # "headActionHash"/"record" could otherwise be mis-matched by an unanchored
    # `.*"key"`. The /head-record body is a flat one-object response, so the
    # anchor pins each key to that object.
    record_b64=""
    record_head=""
    head_record_json=$(curl -fSs -H "X-API-Key: ${STORAGE_API_KEY_ADMIN}" \
        "${SRC}/db/content/${SLUG}/head-record" 2>/dev/null) || head_record_json=""
    if [ -n "${head_record_json}" ]; then
        record_head=$(printf '%s' "${head_record_json}" \
            | sed -n 's/^{.*"headActionHash"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
        if [ "${record_head}" = "${head_hash}" ]; then
            record_b64=$(printf '%s' "${head_record_json}" \
                | sed -n 's/^{.*"record"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
        fi
    fi
    # Build the declare body in a file: a multi-KB base64 payload must not ride
    # in argv (length limits) and must not be re-quoted by the shell. The body
    # is byte-identical to the pre-sprint-3 one when there is no carried record.
    # Lives in the ONE temp dir the single exit trap at the top already owns — a
    # second `trap … EXIT` here would silently replace that one.
    declare_body_file="${STAGE_TMP_DIR}/declare-request"
    declare_answer_file="${STAGE_TMP_DIR}/declare-answer"
    if [ -n "${record_b64}" ]; then
        printf '{"headActionHash":"%s","record":"%s"}' "${head_hash}" "${record_b64}" \
            > "${declare_body_file}"
        echo "  · DECLARE_ONLY: carrying head record from ${SRC} (${#record_b64} b64 chars) — target verifiable without gossip"
    else
        printf '{"headActionHash":"%s"}' "${head_hash}" > "${declare_body_file}"
        echo "  · DECLARE_ONLY: no carried record available from ${SRC} — declaring hash-only (pre-sprint-3 behaviour; 'not retrievable' stays possible)" >&2
    fi
    # Retry ladder for DHT publish lag AND peer backpressure. Two retryable
    # classes, everything else is structural (retrying just burns build time):
    #   - "not retrievable" refusal — a head authored SECONDS ago is
    #     legitimately not-yet-retrievable on the remote conductor (ops still
    #     publishing/gossiping). 90s cadence: the one directly-observed
    #     cross-conductor adoption landed ~10 min post-author (app #1605 →
    #     20:40:35Z), so the ladder must outlast the real publish window.
    #   - HTTP 503/429 — the peer's admission layer sheds writes while its
    #     reconcile drain holds the write pool ({"status":"catching-up"}).
    #     Backpressure clears in seconds-to-minutes, NOT structural: app #1612
    #     aborted here at attempt 3/8 and elohim.host kept the superseded head
    #     for the whole day. 30s cadence.
    # DECLARE_MAX_ATTEMPTS: cross-conductor retrievability today lands anywhere
    # in an 18-50min window post-author (edge #1187 adopted ~50min post-author;
    # app #1614's 18min ladder missed). The Jenkinsfile propagation leg sets a
    # higher ceiling; 12 stays the default for other callers.
    attempt=0
    max_attempts="${DECLARE_MAX_ATTEMPTS:-12}"
    declare_ok=0
    declare_nap=0
    declare_face=""
    declare_status=""
    declare_body=""
    while :; do
        # The pre-dispatch gate. Reached after this ladder's OWN capped sleeps as
        # well as after a readiness nap, which is the path that used to dispatch
        # a POST at exactly the deadline (measured: 0, 10, 12 on a 12s budget).
        if ! readiness_may_start; then
            cell_ready_exhausted "canonical-head declare" "${declare_status}" \
                "${declare_body}" "${declare_face}" \
                "$(( $(date +%s) - CELL_READY_FIRST_SEEN ))"
            echo "  ⚠ DECLARE_ONLY: this run's readiness deadline was reached before another declare could be offered to ${DOORWAY_EPR_URL} — peer keeps its own head until gossip/heal converges" >&2
            break
        fi
        attempt=$((attempt + 1))
        declare_err=""
        declare_status="$(curl -sS -o "${declare_answer_file}" -w '%{http_code}' -X POST \
            -H 'Content-Type: application/json' \
            -H "X-API-Key: ${STORAGE_API_KEY_ADMIN}" \
            --data-binary "@${declare_body_file}" \
            "${DOORWAY_EPR_URL}/db/content/${SLUG}/canonical-head" \
            2>"${STAGE_TMP_DIR}/declare-stderr")" || {
            declare_err="$(cat "${STAGE_TMP_DIR}/declare-stderr" 2>/dev/null)"
            echo "  ⚠ DECLARE_ONLY: curl error POSTing canonical-head to ${DOORWAY_EPR_URL}: ${declare_err} — peer keeps its own head until gossip/heal converges" >&2
            exit 1
        }
        # The body stays in its FILE for every classification; this copy is for
        # the log lines only.
        declare_body="$(cat "${declare_answer_file}" 2>/dev/null)"
        # Fourth class, and the reason it is handled ahead of the case: a
        # not-ready answer CAN reach this ladder — storage answers CellDisabled
        # 503 through `conductor_write_error` (so it would ride the shed arm's
        # 12x30s ≈ 6min ladder), a host still on a pre-fix binary answers it 502
        # with the marker in the body (so it would fall through to "structural"
        # and stop), and a doorway that is catching up sheds this route with the
        # same envelope the head PATCH meets. None of those is this ladder's
        # business: they take the SAME run-level readiness deadline.
        if [ "${declare_status#2}" = "${declare_status}" ]; then
            declare_face="$(not_ready_face "${declare_status}" "${declare_answer_file}" "${declare_body}")"
            if [ -n "${declare_face}" ]; then
                if cell_ready_wait "canonical-head declare" "${declare_status}" \
                    "${declare_body}" "${declare_face}" \
                    "$(read_retry_after /dev/null "${declare_answer_file}")"; then
                    attempt=$(( attempt - 1 ))
                    continue
                fi
                echo "  ⚠ DECLARE_ONLY: ${DOORWAY_EPR_URL} was still not ready when this run's readiness deadline was reached (see above) — peer keeps its own head until gossip/heal converges" >&2
                break
            fi
        fi
        case "${declare_status}" in
            2??)
                echo "  ✓ canonical head propagated to ${DOORWAY_EPR_URL}: ${head_hash} (staging tier, attempt ${attempt})"
                declare_ok=1
                break
                ;;
            503|429)
                # Capped by what is left of an OPEN readiness window, so this
                # ladder cannot sleep past the run deadline; unchanged when no
                # window is open.
                declare_nap="$(cell_ready_cap_sleep 30)"
                if [ "${attempt}" -lt "${max_attempts}" ] && [ "${declare_nap}" -gt 0 ]; then
                    echo "  … DECLARE_ONLY attempt ${attempt}/${max_attempts}: ${DOORWAY_EPR_URL} shedding writes (HTTP ${declare_status}: ${declare_body}) — retrying in ${declare_nap}s" >&2
                    sleep "${declare_nap}"
                    continue
                fi
                echo "  ⚠ DECLARE_ONLY: ${DOORWAY_EPR_URL} still shedding after ${attempt} attempts (HTTP ${declare_status}: ${declare_body}) — peer keeps its own head until gossip/heal converges" >&2
                break
                ;;
            *)
                declare_nap="$(cell_ready_cap_sleep 90)"
                if [ "${attempt}" -lt "${max_attempts}" ] && [ "${declare_nap}" -gt 0 ] && \
                   printf '%s' "${declare_body}" | grep -q "not retrievable"; then
                    echo "  … DECLARE_ONLY attempt ${attempt}/${max_attempts}: target not retrievable yet on ${DOORWAY_EPR_URL} (DHT publish lag) — retrying in ${declare_nap}s" >&2
                    sleep "${declare_nap}"
                    continue
                fi
                # Third retryable class, and the reason it did not ride the
                # 503|429 arm above: a CONDUCTOR-admission shed on this route
                # reached the wire as 502 with the marker in the body, so the
                # ladder that names this exact class in its own comment fell
                # through to "structural" and stopped (elohim/dev #1683, fp
                # f8003a16a985 — backlog
                # ci-canonical-head-declare-shed-laundered-as-502). Storage now
                # answers a shed as 503 (services/response.rs
                # `conductor_write_error`) so it lands on the arm above; this
                # match keeps the ladder correct against any host still running
                # a pre-fix binary, and costs nothing once none are.
                declare_nap="$(cell_ready_cap_sleep 30)"
                if [ "${attempt}" -lt "${max_attempts}" ] && [ "${declare_nap}" -gt 0 ] && \
                   printf '%s' "${declare_body}" | grep -q "conductor admission: shed"; then
                    echo "  … DECLARE_ONLY attempt ${attempt}/${max_attempts}: ${DOORWAY_EPR_URL} shed the call at the conductor-admission gate (HTTP ${declare_status}: ${declare_body}) — nothing was dispatched, retrying in ${declare_nap}s" >&2
                    sleep "${declare_nap}"
                    continue
                fi
                echo "  ⚠ DECLARE_ONLY: ${DOORWAY_EPR_URL} returned HTTP ${declare_status}: ${declare_body} — peer keeps its own head until gossip/heal converges" >&2
                break
                ;;
        esac
    done
    # R1 (2026-07-31): this leg IS the arbitration channel now — a fan-out
    # that never reached HTTP 2xx must be visible to the caller, not silently
    # advisory, so a stale doorway doesn't hide behind a green deploy.
    if [ "${declare_ok}" = "1" ]; then
        exit 0
    fi
    echo "  ⚠ DECLARE_ONLY: fan-out to ${DOORWAY_EPR_URL} did NOT converge (exhausted retry budget) — exiting non-zero so the caller can surface this" >&2
    exit 1
fi

# Package locally through the SDK's checked archive operation. The source dist
# is immutable; every retry uploads the exact same checked archive bytes.
SDK_PACKAGE="$(cd "$(dirname "$0")/../.." && pwd)/elohim/sdk/scripts/package-app.mjs"
# No trap here: the ONE exit trap at the top of this file already names
# package_dir, and a second `trap … EXIT` would silently replace it.
package_dir=$(mktemp -d)
app_dir=$(cd "$DIST_DIR/../../.." && pwd)
package_args=(--adapter "$(dirname "$SDK_PACKAGE")/package-angular.mjs" --dist "$DIST_DIR" --kind "$KIND" --out "$package_dir" --app-dir "$app_dir")
# Server/browser artifacts are published separately, but their build context is
# checked together. The actual server build must carry the same build stamp.
if [ "$KIND" = server ] && [ -f "$DIST_DIR/../browser/version.json" ]; then
    package_args+=(--version "$DIST_DIR/../browser/version.json")
fi
if ! node "$SDK_PACKAGE" "${package_args[@]}"; then
    echo "ERROR: [$SLUG] local package checks failed before upload" >&2
    exit 2
fi
SPA_ARCHIVE="$package_dir/$KIND.zip"
SPA_HASH="sha256-$(sha256sum "$SPA_ARCHIVE" | awk '{print $1}')"
SPA_SIZE="$(du -h "$SPA_ARCHIVE" | cut -f1)"
echo "[${SLUG}] blob hash: ${SPA_HASH}"
echo "[${SLUG}] blob size: ${SPA_SIZE}"

# Optional hand-off for the caller (Track-4 T4-2 verify-projected-head.sh):
# writes the just-computed content hash to a file so the Jenkinsfile can read
# it back (via readFile) and pass it as the EXPECTED hash to the served-vs-
# declared propagation probe, without re-deriving/re-zipping (which the
# authoring host already established is byte-reproducible across calls given
# an unchanged dist dir — see authorHeadOnce/stageSpaBlobs commentary).
if [ -n "${HASH_OUTPUT_FILE:-}" ]; then
    echo "${SPA_HASH}" > "${HASH_OUTPUT_FILE}"
fi

# Publish the content-addressed hash for the post-stage serve-verify (deploy
# resilience, 2026-06-30). Opt-in: writes ONLY when STAGE_HASH_OUT names a path
# (unset => no-op, so every existing caller is byte-for-byte unaffected). The
# Jenkinsfile points this at a per-(host,slug,kind) workspace file;
# verify-host-serves.sh reads it to assert the host actually SERVES this
# freshly-staged bundle, not a stale one. Defensive: an unwritable path WARNs
# and continues (the serve-verify degrades) — it must never abort a stage that
# would otherwise land, even under set -e.
if [ -n "${STAGE_HASH_OUT:-}" ]; then
    if ! printf '%s\n' "${SPA_HASH}" > "${STAGE_HASH_OUT}" 2>/dev/null; then
        echo "  ⊘ WARN: could not publish staged hash to STAGE_HASH_OUT='${STAGE_HASH_OUT}' — serve-verify will degrade to serves-200 + non-null checks" >&2
    fi
fi

# Field-by-kind (SSR row collapse): KIND=server PATCHes/reads serverBlobHash on
# the ONE elohim-host-landing EPR node (not a separate -ssr row); KIND=browser
# stays blobHash. Both ride db/content/{slug} with identical partial-update
# semantics — PATCHing one field never clobbers the other.
if [ "$KIND" = "server" ]; then
    HASH_FIELD="serverBlobHash"
else
    HASH_FIELD="blobHash"
fi

# Single notarized head. The blobHash PATCH is a DNA-notarized write
# (storage's patch_needs_conductor=true). The head is authored ONCE: this PATCH
# is routed through a doorway to a storage backend that has a live conductor
# bridge; the CONDUCTOR authors the DHT entry and the peer network (the
# Holochain DHT) WITNESSES it by validating + gossiping + anchoring. The doorway
# is only the gateway — never the witness. The Jenkinsfile fails this PATCH over
# across doorways until one reaches a live bridge, then stops; the single
# witnessed head gossips to every peer. A backend with NO conductor bridge 503s
# here, and that is CORRECT: that host does not author, it converges the head
# later via run_content_sweep. There is NO `?deployTier=amber` escape hatch
# anymore — a per-host diesel-direct write minted an un-witnessed head that could
# never green and diverged across backends. (The byte upload below still runs per
# host: blob BYTES are content-addressed and do not auto-replicate yet, so
# seeding them on each serving peer is legitimate load-spread, not a divergent
# write.)
PATCH_PATH="/db/content/${SLUG}"

# One PUT + (optional) PATCH+verify attempt. Returns non-zero on ANY failure
# WITHOUT exiting the script: stage_once is invoked in an `if` condition, so
# bash suspends `set -e` inside its body — each network op carries an explicit
# `|| return 1`, letting the retry loop below catch transient failures.
#
# 1. Upload ZIP as blob via doorway's seed-blob route.
#    Why /admin/seed/blob and not /blob/{hash}: doorway's forward_blob_to_storage
#    (storage_proxy.rs) is a read-through cache that hardcodes client.get() — PUT
#    requests get silently downgraded to GET, storage returns 404 on the
#    not-yet-uploaded hash, the build fails with curl exit 22. /admin/seed/blob
#    is the seeder's write-through path (preserves dev fix f853fb665).
#
#    AUTH (2026-06-27): doorway's require_seed_authority gate (commit 396779747,
#    2026-06-11) 401s an UNAUTHENTICATED PUT on any non-DEV_MODE doorway. alpha
#    (doorway-A) sets DEV_MODE=true and bypassed the gate, masking the omission;
#    elohim.host (doorway-B / alpha-b) does NOT set DEV_MODE, so its PUT 401'd
#    EVERY build → blob never uploaded → host silently stranded on a stale bundle
#    for ~2 weeks (the per-host deploy-lag class). The PATCH already sent the
#    admin key; the PUT was simply never updated when the gate landed. Send the
#    SAME admin X-API-Key here — seed.rs: "Admin API key (X-API-Key) also passes
#    — the operator credential CI uses." DEV_MODE hosts ignore it; gated hosts
#    require it. ${VAR:-} keeps it set -u-safe when no admin key is in scope (the
#    no-credential PUT then correctly stays unauthorized on a gated host).
# 2. Link blob to the content row (PATCH+verify) — only when admin key present.
#    Seatbelt: after each PATCH, GET the row and assert the hash field matches
#    the SHA just written; a drift returns non-zero (→ retried, then NAMED).
stage_once() {
    # Re-PUT short-circuit (fixed 2026-08-16, found by the hc-mesh CI quiesce
    # stage): when the target already holds this content-addressed blob, the
    # seed route answers 2xx from cache WITHOUT draining the request body, and
    # curl dies mid-upload with a broken pipe (exit 55 "Failed sending data" /
    # 56 "Failure receiving data") — i.e. the blob IS stored and the stage
    # failed anyway, burning the whole retry ladder on a no-op re-stage. Treat
    # those two exits as "verify by content address" rather than as failure;
    # every other exit still fails the attempt.
    #
    # NO `-f` (2026-09-21): curl's fail-fast discards the response BODY, and the
    # doorway's global admission gate can shed this very route with
    # 503 {"status":"catching-up",...} before it is ever routed (server/http.rs).
    # Under -f that body never reached the classifier and the leg spent its
    # transport budget on a readiness condition (#1715). Capture status, headers
    # and the body's ORIGINAL BYTES (to a file, never through a shell variable —
    # see the JSON block above), then classify.
    local put_rc=0 put_response="" put_status="" put_retry_after=""
    local put_headers_file="${STAGE_TMP_DIR}/put-headers"
    local put_body_file="${STAGE_TMP_DIR}/put-body"
    : > "${put_body_file}"
    : > "${put_headers_file}"
    put_status="$(curl -sS -D "${put_headers_file}" -o "${put_body_file}" -w '%{http_code}' -X PUT \
        -H 'Content-Type: application/zip' \
        -H "X-Blob-Hash: ${SPA_HASH}" \
        -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
        --data-binary "@$SPA_ARCHIVE" \
        "${DOORWAY_EPR_URL}/admin/seed/blob" 2>/dev/null)" || put_rc=$?
    # ONLY three digits is a status. A curl that did not honour -w leaves
    # something else (elohim/sdk/scripts/package-app.test.mjs drives this script
    # with such a stub), and that must never be mistaken for one.
    case "${put_status}" in
        [0-9][0-9][0-9]) : ;;
        *) put_status="" ;;
    esac
    # For the LOG and for the one raw-text (CellDisabled) match only. Every
    # classification below reads "${put_body_file}".
    put_response="$(cat "${put_body_file}" 2>/dev/null)"
    put_retry_after="$(read_retry_after "${put_headers_file}" "${put_body_file}")"
    [ -n "${put_response}" ] && echo "${put_response}"

    local put_face=""
    if [ "${put_rc}" -eq 0 ] && [ -n "${put_status}" ] \
       && [ "${put_status#2}" = "${put_status}" ]; then
        # A non-2xx answer to the PUT: a catching-up shed here is the same
        # not-ready window the PATCH leg meets, one leg earlier.
        put_face="$(not_ready_face "${put_status}" "${put_body_file}" "${put_response}")"
        if [ -n "${put_face}" ]; then
            CELL_STATUS="${put_status}"; CELL_BODY="${put_response}"
            CELL_FACE="${put_face}"; CELL_RETRY_AFTER="${put_retry_after}"
            return 4
        fi
        echo "  ✗ [${SLUG}] blob PUT via ${DOORWAY_EPR_URL} refused (HTTP ${put_status}): ${put_response}" >&2
        return 1
    fi

    if [ "${put_rc}" -eq 0 ]; then
        # SUCCESS IS AFFIRMED, NEVER ASSUMED. A 2xx alone says the doorway
        # answered; only a parsed top-level `forwarded_to_storage:true` says
        # storage took the bytes. The old whitespace-exact negative grep missed
        # `"forwarded_to_storage": false` and read a missing or malformed body as
        # success — a byte-seed could report ✓ having proved only that the
        # doorway's 1h write-through cache remembers what we just sent it, and a
        # head declared against those bytes is the declared-vs-available
        # divergence of 2026-08-22 (a 69MB bundle hit a storage-side shard-verify
        # drop; the leg stamped ✓ and every peer declared a bundle nobody could
        # materialize).
        if ! forward_confirmed "${put_body_file}"; then
            # A forward that TIMED OUT is the storage peer not ready yet (#1715:
            # 13 blob legs, all 200 + forwarded_to_storage:false, ~87min summed on
            # a condition that cleared by itself). Every other forward failure —
            # connection refused, DNS, no storage_url — names an absent or
            # misrouted peer, which waiting does not answer, so it stays on the
            # transport ladder.
            put_face="$(not_ready_face "${put_status}" "${put_body_file}" "${put_response}")"
            if [ -n "${put_face}" ]; then
                CELL_STATUS="${put_status:-200}"; CELL_BODY="${put_response}"
                CELL_FACE="${put_face}"; CELL_RETRY_AFTER="${put_retry_after}"
                return 4
            fi
            echo "  ✗ [${SLUG}] blob PUT via ${DOORWAY_EPR_URL} answered HTTP ${put_status:-2xx} but did NOT confirm the storage forward (needs top-level forwarded_to_storage:true) — refusing to call this staged: ${put_response}" >&2
            return 1
        fi
        echo "  ✓ [${SLUG}] blob uploaded and storage forward CONFIRMED (via /admin/seed/blob)"
    elif [ "${put_rc}" -eq 55 ] || [ "${put_rc}" -eq 56 ]; then
        # AFFIRMATIVE STORAGE EVIDENCE ONLY (2026-09-21). This arm used to accept
        # a successful `GET /blob/{hash}` as proof the bytes were stored. They are
        # not the same claim: that GET is cache-first (routes/blob.rs tries
        # `ctx.cache` before ever asking storage), so the doorway's own cache
        # answers it. The only affirmative evidence reachable from CI is the
        # doorway's own `forwarded_to_storage`.
        #   1. If the broken PUT still captured a parsable answer, use it.
        #   2. Otherwise re-ask with a ZERO-BYTE body. On the already-cached path
        #      the doorway answers before reading the body — the very behaviour
        #      that breaks the big upload — so the empty probe cannot break the
        #      same way, and it re-runs the forward and reports it. If the doorway
        #      does NOT hold the bytes, the empty body fails the hash check and
        #      answers 409 BEFORE anything is cached (seed.rs checks the hash
        #      before `cache.set`), so the probe is side-effect-free.
        #   3. Anything else re-offers. Never exit 0 on cache-only evidence.
        if ! forward_confirmed "${put_body_file}"; then
            local probe_rc=0 probe_status=""
            local probe_body_file="${STAGE_TMP_DIR}/probe-body"
            : > "${probe_body_file}"
            probe_status="$(curl -sS -o "${probe_body_file}" -w '%{http_code}' -X PUT \
                -H 'Content-Type: application/zip' \
                -H "X-Blob-Hash: ${SPA_HASH}" \
                -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
                --data-binary '' \
                "${DOORWAY_EPR_URL}/admin/seed/blob" 2>/dev/null)" || probe_rc=$?
            case "${probe_status}" in
                [0-9][0-9][0-9]) : ;;
                *) probe_status="" ;;
            esac
            if [ "${probe_rc}" -eq 0 ]; then
                # The probe's ORIGINAL BYTES replace the broken PUT's as the
                # evidence file for everything below.
                put_body_file="${probe_body_file}"
                put_status="${probe_status}"
                put_response="$(cat "${put_body_file}" 2>/dev/null)"
                echo "  · [${SLUG}] PUT broke (curl ${put_rc}); zero-byte confirmation probe answered HTTP ${put_status:-<none>}: ${put_response}"
            else
                : > "${probe_body_file}"
                put_body_file="${probe_body_file}"
                put_status=""
                put_response=""
                echo "  · [${SLUG}] PUT broke (curl ${put_rc}); the zero-byte confirmation probe also failed (curl ${probe_rc})" >&2
            fi
        fi
        if forward_confirmed "${put_body_file}"; then
            echo "  ✓ [${SLUG}] blob stored (re-PUT short-circuited by the doorway cache; storage forward CONFIRMED for ${SPA_HASH})"
        else
            put_face="$(not_ready_face "${put_status}" "${put_body_file}" "${put_response}")"
            if [ -n "${put_face}" ]; then
                CELL_STATUS="${put_status:-200}"; CELL_BODY="${put_response}"
                CELL_FACE="${put_face}"; CELL_RETRY_AFTER=""
                return 4
            fi
            echo "  ✗ [${SLUG}] PUT broke (curl ${put_rc}) and no answer CONFIRMS storage accepted ${SPA_HASH} — a doorway cache hit is not that proof; re-offering" >&2
            return 1
        fi
    else
        return 1
    fi

    # The peer judges the bytes we just staged; a broken bundle never gets a
    # head. Exit 2 from the gate is a VERDICT, not a transport blip — surface
    # it as the stage's own failure code so the retry ladder does not re-try
    # a deterministic answer. Server bundles have no index.html at all (they
    # carry index.server.html) — the gate's boot probe would read every one
    # of them as broken:no-index, so it only runs for kind=browser; server
    # bundles are judged by the renderer, slice 2.
    if [ "${KIND}" = "browser" ]; then
        local gate_rc=0
        bash "$(dirname "$0")/deliverability-gate.sh" "${DOORWAY_EPR_URL}" "${SPA_HASH}" || gate_rc=$?
        if [ "${gate_rc}" -eq 2 ] || [ "${gate_rc}" -eq 3 ]; then
            echo "BROKEN_HEAD ${SLUG} ${KIND} ${SPA_HASH}" > "${DELIVERABILITY_VERDICT_FILE:-/dev/null}" 2>/dev/null || true
            return 2
        elif [ "${gate_rc}" -ne 0 ]; then
            echo "  ⚠ [${SLUG}] deliverability gate could not run (rc=${gate_rc}) — proceeding unjudged (advisory)" >&2
        fi
    else
        echo "  ⊘ [${SLUG}] deliverability gate skipped for kind=${KIND} (server bundles are judged by the renderer, slice 2)"
    fi

    if [ "${DO_PATCH}" = "1" ]; then
        # Retry-aware PATCH. Modeled on the DECLARE_ONLY ladder above (same
        # shed envelope, same conductor-admission gate): capture the status
        # code AND response headers so a 503/429 shed can be classified and,
        # when the peer advertised HOW LONG the window is (Retry-After
        # header, or a "retryAfter" field in the JSON body — the doorway's
        # catching-up envelope carries both), that hint rides back to the
        # caller via RETRY_AFTER_HINT instead of the caller guessing with a
        # fixed 5/10s backoff. A "not retrievable" body is the pre-existing
        # DHT-publish-lag class and stays retryable. A CellDisabled body is the
        # cell-readiness class — return 4 so the outer loop waits on the
        # separate per-host readiness budget. Any OTHER 4xx (not 429, not "not
        # retrievable") is a structural answer — return 3 so the outer loop
        # fails fast instead of burning the time budget on a question the peer
        # has already answered.
        local patch_headers_file="${STAGE_TMP_DIR}/patch-headers"
        local patch_body_file="${STAGE_TMP_DIR}/patch-body"
        local patch_status patch_body
        : > "${patch_body_file}"
        : > "${patch_headers_file}"
        if ! patch_status=$(curl -sS -D "${patch_headers_file}" -o "${patch_body_file}" -w '%{http_code}' -X PATCH \
            -H 'Content-Type: application/json' \
            -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
            -d "{\"${HASH_FIELD}\":\"${SPA_HASH}\"}" \
            "${DOORWAY_EPR_URL}${PATCH_PATH}" 2>"${STAGE_TMP_DIR}/patch-stderr"); then
            echo "  ✗ [${SLUG}] ${HASH_FIELD} PATCH via ${DOORWAY_EPR_URL} curl error: $(cat "${STAGE_TMP_DIR}/patch-stderr" 2>/dev/null)" >&2
            return 1
        fi
        case "${patch_status}" in
            [0-9][0-9][0-9]) : ;;
            *) patch_status="" ;;
        esac
        # For the LOG and the one raw-text match only; the file holds the bytes.
        patch_body="$(cat "${patch_body_file}" 2>/dev/null)"

        if [ -n "${patch_status}" ] && [ "${patch_status#2}" != "${patch_status}" ]; then
            :
        else
            # Normalised ONCE, here, from header then body — the same value
            # feeds the readiness nap and the ordinary transport sleep below.
            local retry_after=""
            retry_after="$(read_retry_after "${patch_headers_file}" "${patch_body_file}")"

            # A not-ready answer is neither backpressure nor a structural
            # answer: it is the post-roll window (see the not-ready block at the
            # top of this file for the three faces and the two dated
            # measurements). Classified ahead of the status case because the
            # class is the BODY, not the code — CellDisabled reaches the wire as
            # 503 through storage's `conductor_write_error` and as 502 from a
            # host still running a pre-fix binary. Return 4: the outer loop waits
            # on the run-level readiness deadline, charging neither an attempt
            # nor a second of the transport budget.
            local patch_face
            patch_face="$(not_ready_face "${patch_status}" "${patch_body_file}" "${patch_body}")"
            if [ -n "${patch_face}" ]; then
                CELL_STATUS="${patch_status}"
                CELL_BODY="${patch_body}"
                CELL_FACE="${patch_face}"
                CELL_RETRY_AFTER="${retry_after}"
                return 4
            fi

            case "${patch_status}" in
                503|429)
                    if [ -n "${retry_after}" ]; then
                        RETRY_AFTER_HINT="${retry_after}"
                    fi
                    echo "  ✗ [${SLUG}] ${HASH_FIELD} PATCH via ${DOORWAY_EPR_URL} shed (HTTP ${patch_status}${retry_after:+, retryAfter=${retry_after}s}): ${patch_body}" >&2
                    return 1
                    ;;
                4*)
                    if printf '%s' "${patch_body}" | grep -q "not retrievable"; then
                        echo "  ✗ [${SLUG}] ${HASH_FIELD} PATCH via ${DOORWAY_EPR_URL} — target not retrievable yet (HTTP ${patch_status}): ${patch_body}" >&2
                        return 1
                    fi
                    echo "  ✗ [${SLUG}] ${HASH_FIELD} PATCH via ${DOORWAY_EPR_URL} failed structurally (HTTP ${patch_status}): ${patch_body}" >&2
                    return 3
                    ;;
                *)
                    echo "  ✗ [${SLUG}] ${HASH_FIELD} PATCH via ${DOORWAY_EPR_URL} failed (HTTP ${patch_status}): ${patch_body}" >&2
                    return 1
                    ;;
            esac
        fi
        echo "  ✓ patched ${SLUG} (${HASH_FIELD})"
        local authored_action
        authored_action=$(printf '%s' "${patch_body}" \
            | sed -n 's/.*"dhtAnchorHash"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
        if [ -z "${authored_action}" ]; then
            echo "  ✗ [${SLUG}] ${HASH_FIELD} PATCH returned no dhtAnchorHash: ${patch_body}" >&2
            return 3
        fi
        echo "[${SLUG}] authored action: ${authored_action}"

        local actual
        actual=$(curl -fSs "${DOORWAY_EPR_URL}/db/content/${SLUG}" \
            | HASH_FIELD="${HASH_FIELD}" python3 -c "import os, sys, json; print(json.load(sys.stdin).get(os.environ['HASH_FIELD'],''))") || return 1
        if [ "${actual}" != "${SPA_HASH}" ]; then
            echo "  ✗ [${SLUG}] ${HASH_FIELD} drift after PATCH: expected ${SPA_HASH}, got ${actual:-<empty>}" >&2
            return 1
        fi
        echo "  ✓ verified ${SLUG} ${HASH_FIELD} = ${SPA_HASH}"

        # 3. Declare the CROSS-ROOT canonical head (staging tier) — ADVISORY.
        #    Resolves the notary HEAD this same conductor just witnessed via
        #    GET /db/content/{id}/head, then names it the canonical head via
        #    POST /db/content/{id}/canonical-head so scenario 2 (multi-root
        #    convergence) has a declared answer to resolve. This leg is
        #    best-effort: the coordinator may be pre-cure (fn-not-found) on a
        #    host that hasn't picked up the hot-swap yet. NEVER fails the
        #    deploy — any failure here is a loud warning, not a retry/abort.
        local head_hash=""
        head_hash=$(curl -fSs \
            -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
            "${DOORWAY_EPR_URL}/db/content/${SLUG}/head" \
            2>/dev/null \
            | python3 -c "import sys, json; print(json.load(sys.stdin).get('headActionHash',''))" 2>/dev/null) || head_hash=""

        if [ -z "${head_hash}" ]; then
            echo "  ⚠ canonical-head declare FAILED — could not resolve headActionHash from GET ${DOORWAY_EPR_URL}/db/content/${SLUG}/head — coordinator may be pre-cure; scenario 2 will stay red" >&2
        else
            # No -f here (deliberately): a fn-not-found-class zome error comes
            # back as a non-2xx JSON body ({"error": "..."}) that we want to
            # surface verbatim (the hot-swap probe signal) — -f would discard
            # the body and leave only a bare curl exit code.
            #
            # BACKPRESSURE IS NOT A VERDICT (added 2026-09-02, elohim/dev #1683,
            # fp f8003a16a985). A conductor-admission SHED means storage's permit
            # gate refused to dispatch — "the conductor never saw the call" — so
            # the honest response is to re-offer it, exactly as
            # `conductor_admission::is_admission_shed` instructs every caller.
            # #1683 met `HTTP 502: {"error":"Request timeout: conductor admission:
            # shed: no conductor permit for content_store within 5000ms
            # (class=interactive, capacity=5, in_flight=5) — nothing was
            # dispatched"}` on a SINGLE attempt and surrendered, leaving scenario
            # 2 red for a gate that was merely busy for five seconds. Note the
            # capacity: 5 is the admission floor (PERMIT_FLOOR 8 − CONDUCTOR_RESERVE
            # 3), so an alpha storage pod saturates on very little concurrency and
            # a deploy landing mid-sweep meets a full gate routinely.
            #
            # Storage now answers a shed as 503 + Retry-After (services/response.rs
            # `conductor_write_error`), but this ladder ALSO matches the shed marker
            # in the body so it keeps working against a doorway/storage pair that
            # has not taken that fix yet — which is every fleet host until it rolls.
            # Still ADVISORY throughout: every arm returns success; this leg never
            # fails the deploy.
            local canonical_raw canonical_exit canonical_status canonical_body
            local declare_attempt=1
            local declare_attempts="${CANONICAL_HEAD_ATTEMPTS:-4}"
            while true; do
                canonical_raw=$(curl -sS -o - -w '\n%{http_code}' -X POST \
                    -H 'Content-Type: application/json' \
                    -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
                    -d "{\"headActionHash\":\"${head_hash}\"}" \
                    "${DOORWAY_EPR_URL}/db/content/${SLUG}/canonical-head" \
                    2>&1)
                canonical_exit=$?

                if [ "${canonical_exit}" -ne 0 ]; then
                    echo "  ⚠ canonical-head declare FAILED — curl error against POST ${DOORWAY_EPR_URL}/db/content/${SLUG}/canonical-head (exit ${canonical_exit}): ${canonical_raw} — coordinator may be pre-cure; scenario 2 will stay red" >&2
                    break
                fi

                canonical_status="${canonical_raw##*$'\n'}"
                canonical_body="${canonical_raw%$'\n'*}"
                if [ "${canonical_status#2}" != "${canonical_status}" ]; then
                    echo "  ✓ canonical head declared: ${head_hash} (staging tier)"
                    break
                fi

                # Backpressure, by either wire shape: the post-fix 503/429
                # (+Retry-After) or the shed marker carried verbatim in the body
                # of a pre-fix 502. Anything else is a real answer — surface it.
                if [ "${declare_attempt}" -lt "${declare_attempts}" ] \
                    && { [ "${canonical_status}" = "503" ] \
                        || [ "${canonical_status}" = "429" ] \
                        || printf '%s' "${canonical_body}" | grep -q 'conductor admission: shed'; }; then
                    local declare_backoff=$(( declare_attempt * 5 ))
                    echo "  ⚠ canonical-head declare BACKPRESSURE (HTTP ${canonical_status}) — the conductor was never asked; re-offering in ${declare_backoff}s (attempt ${declare_attempt}/${declare_attempts})" >&2
                    declare_attempt=$(( declare_attempt + 1 ))
                    sleep "${declare_backoff}"
                    continue
                fi

                echo "  ⚠ canonical-head declare FAILED — POST ${DOORWAY_EPR_URL}/db/content/${SLUG}/canonical-head returned HTTP ${canonical_status}: ${canonical_body} — coordinator may be pre-cure; scenario 2 will stay red" >&2
                break
            done
        fi
    else
        echo "  ⊘ [${SLUG}] byte-seed only (DO_PATCH!=1) — no head PATCH from this call."
        echo "    The single notarized head is authored once via a conductor-bridged"
        echo "    doorway; this peer converges it via run_content_sweep. Blob bytes"
        echo "    uploaded and content-addressable via PUT /blob/${SPA_HASH}."
    fi
    return 0
}

# Bounded retry, budget-first. A TIME BUDGET (STAGE_BLOB_BUDGET_SECS, default
# 360s) is now the primary bound so a shedding conductor gets several windows
# — the old fixed-3-attempts ladder (5s, 10s backoff) spent only 15s total
# before declaring the host STALE, inside the single ~30s catching-up window
# the peer advertised (elohim #1710, 2026-09-13: byte upload succeeded, the
# blobHash/serverBlobHash PATCH shed with {"status":"catching-up",
# "retryAfter":30,...} on both alpha and elohim.host, and the ladder gave up
# before the window it was told about even closed). STAGE_BLOB_ATTEMPTS is a
# safety-net CEILING on attempt count on top of the budget (default 60 — see
# header comment); a caller that wants the old fast-fail count-only
# behaviour can still pass a low value (run-mesh-quiesce-stage.sh does).
#
# A conductor-bridged backend can 503 transiently during cluster churn (the
# notarized PATCH round-trips the conductor); retry rather than surrender on
# a blip. A PERSISTENT non-zero exit is meaningful: for a byte-seed
# (DO_PATCH!=1) it means the blob upload failed; for an author attempt
# (DO_PATCH=1) it means THIS doorway could not author (no live conductor
# bridge / persistent backpressure), and the Jenkinsfile fails the head
# author over to the next doorway. The deploy is only UNSTABLE if NO doorway
# in the fabric can author the single head.
MAX_WAIT_SECS=60
attempt=1
start_ts=$(date +%s)
while true; do
    # THE PRE-ATTEMPT GATE. Reached after an ORDINARY transport sleep as well as
    # after a readiness nap — and the ordinary ladder is exactly how a 12s budget
    # still managed to dispatch at 0, 10 and 40s. With no window open this is
    # always true and this loop behaves exactly as it did.
    if ! readiness_may_start; then
        cell_ready_exhausted "${CELL_FACE:-not-ready} re-offer (${HASH_FIELD} leg)" \
            "${CELL_STATUS}" "${CELL_BODY}" "${CELL_FACE}" \
            "$(( $(date +%s) - CELL_READY_FIRST_SEEN ))"
        echo "ERROR: [${SLUG}] this run's readiness deadline was reached before another attempt could be offered to ${DOORWAY_EPR_URL} — host left STALE" >&2
        exit 1
    fi
    rc=0
    # Stamped BEFORE the attempt so an attempt that turns out to be a readiness
    # probe can have its WHOLE duration refunded, not just the nap after it.
    attempt_start_ts=$(date +%s)
    stage_once || rc=$?
    if [ "${rc}" -eq 0 ]; then
        if [ "${CELL_WAIT_ENTERED}" -eq 1 ]; then
            # The run deadline is NOT dropped here. A recovery ends this window,
            # not the run's ceiling on waiting: a second window later in the same
            # run inherits what is left, which is the only way the configured
            # number bounds the run rather than each window in it.
            echo "  ✓ [${SLUG}] ${DOORWAY_EPR_URL} took the write after $(( $(date +%s) - CELL_READY_FIRST_SEEN ))s of not-ready answers (last face=${CELL_FACE:-none}) — the same content-addressed offer went through, no intervention needed; $(cell_ready_remaining)s of this run's readiness deadline left for a later window"
        fi
        break
    fi
    # A deterministic peer VERDICT (broken bundle, or NOT-JUDGED under strict
    # mode) is not a transport blip — retrying re-asks a question the peer has
    # already answered. Terminal, immediately, never counted against the budget.
    if [ "${rc}" -eq 2 ]; then
        echo "ERROR: [${SLUG}] the peer judged ${SPA_HASH} BROKEN — not retrying a deterministic verdict" >&2
        exit 2
    fi
    # A structural 4xx (not 429, not the "not retrievable" DHT-lag class) is
    # a real answer, not backpressure — retrying just burns the budget on a
    # question the peer has already answered (stage_once already logged the
    # HTTP status/body above).
    if [ "${rc}" -eq 3 ]; then
        echo "ERROR: [${SLUG}] structural failure against ${DOORWAY_EPR_URL} (see above) — not retrying — host left STALE" >&2
        exit 1
    fi
    # A holder that is not ready yet is a WAIT, on the run-level readiness
    # deadline: the retried operation is this same idempotent, content-addressed
    # offer via stage_once, and neither the attempt counter nor the transport
    # budget is charged for it. The refund covers the WHOLE attempt (the PUT, the
    # deliverability check, the PATCH itself) plus the nap — charging just the
    # nap still let a slow readiness probe eat the transport budget it was
    # supposed to be separate from.
    if [ "${rc}" -eq 4 ]; then
        if cell_ready_wait "${CELL_FACE:-not-ready} re-offer (${HASH_FIELD} leg)" \
            "${CELL_STATUS}" "${CELL_BODY}" "${CELL_FACE}" "${CELL_RETRY_AFTER}"; then
            start_ts=$(( start_ts + $(date +%s) - attempt_start_ts ))
            continue
        fi
        echo "ERROR: [${SLUG}] the holder behind ${DOORWAY_EPR_URL} never became ready (see above) — host left STALE" >&2
        exit 1
    fi

    now_ts=$(date +%s)
    elapsed=$(( now_ts - start_ts ))
    remaining=$(( STAGE_BUDGET_SECS - elapsed ))
    if [ "${attempt}" -ge "${ATTEMPTS}" ] || [ "${remaining}" -le 0 ]; then
        echo "ERROR: [${SLUG}] stage failed after ${attempt} attempt(s) / ${elapsed}s against ${DOORWAY_EPR_URL} (budget ${STAGE_BUDGET_SECS}s, attempt cap ${ATTEMPTS}) — host left STALE" >&2
        exit 1
    fi

    if [ -n "${RETRY_AFTER_HINT}" ]; then
        wait_s="${RETRY_AFTER_HINT}"
        if [ "${wait_s}" -gt "${MAX_WAIT_SECS}" ]; then
            wait_s="${MAX_WAIT_SECS}"
        fi
        echo "  ⚠ [${SLUG}] attempt ${attempt}/${ATTEMPTS} against ${DOORWAY_EPR_URL} failed — server advertised retryAfter=${RETRY_AFTER_HINT}s, waiting ${wait_s}s (elapsed ${elapsed}s, ${remaining}s left of ${STAGE_BUDGET_SECS}s budget)" >&2
    else
        wait_s=$(( attempt * 5 ))
        if [ "${wait_s}" -gt "${MAX_WAIT_SECS}" ]; then
            wait_s="${MAX_WAIT_SECS}"
        fi
        echo "  ⚠ [${SLUG}] attempt ${attempt}/${ATTEMPTS} against ${DOORWAY_EPR_URL} failed — retrying in ${wait_s}s (elapsed ${elapsed}s, ${remaining}s left of ${STAGE_BUDGET_SECS}s budget)" >&2
    fi
    # Inside an OPEN readiness window this ordinary sleep is capped by what is
    # left of the run deadline — otherwise it sleeps straight past it and the
    # gate at the top of the loop, which does refuse, refuses far too late.
    # Outside a window the value is returned unchanged.
    wait_s="$(cell_ready_cap_sleep "${wait_s}")"
    RETRY_AFTER_HINT=""
    attempt=$(( attempt + 1 ))
    sleep "${wait_s}"
done
