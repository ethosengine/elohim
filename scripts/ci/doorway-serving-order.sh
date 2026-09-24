#!/usr/bin/env bash
# doorway-serving-order.sh — order the doorway EPR URLs so the hosts that can
# author a head NOW are offered first.
#
# WHY (app #1725, 2026-09-23): authorHeadOnce in the root Jenkinsfile offered
# the head PATCH to the doorways in a FIXED order (alpha.elohim.host, then
# elohim.host), and stage-spa-blob.sh's readiness wait spends the run's ONE
# 7200 s deadline — stamped by the first not-ready answer, shared by every host
# and every leg — on whichever host is offered first. #1725 spent all of it on
# alpha (matthew: storage shedding writes because its content cell stayed
# CellDisabled after the edge roll), 21:40:08Z to ~23:42Z. elohim.host (adam)
# was offered the head only after that clock was gone: its blob PUT was
# confirmed, its one head re-offer answered HTTP 503 catching-up (circuit
# closed, errorStreak 1), the exhausted deadline was reached at once, and every
# bundle read "NO doorway could author". Whether adam would have accepted the
# PATCH earlier is UNMEASURED — its conductor had logged "apps enabled" at
# 22:48:27Z, which is a log line, not a serving reading. What is measured is
# that the deadline was spent on one host while the other was never asked
# while there was time. Asking each doorway first costs one bounded GET per
# host and re-aims the wait at whichever host says it can serve NOW.
#
# Usage: doorway-serving-order.sh <doorway-epr-url>...
#   stdout: the SAME URLs, verbatim, one per line — serving hosts first (their
#           relative order kept), then the rest in their original order. The
#           caller iterates this list for the AUTHOR leg only; the DECLARE_ONLY
#           fan-out and the propagation probes keep the canonical list.
#   stderr: one diagnostic per host, for the build log. Nothing else.
#   exit:   0 ALWAYS — this is advice about ORDER, never a gate. A host that
#           cannot be asked, answers non-JSON, or answers anything but serving
#           simply sorts after the serving ones; when none serves the output is
#           the input and the caller behaves exactly as before.
#
# SERVING means BOTH of:
#   1. the doorway's OWN verdict: `/health/serving` answers HTTP 200. The route
#      (doorway-service routes/health.rs, build_serving_response) 503s on ANY of
#      five arms — shedding (an open circuit), degrading (a non-zero upstream
#      error streak: the regime adam was in at ~23:42Z above), conductor-blind
#      (rolesDiscovered == 0: every zome call fails), warmupEmpty (an empty
#      projection), or the primary storage peer refused / unreachable. The
#      status code is the one place all five meet, so it is read, never
#      discarded: a body that "looks serving" under a 503 is NOT serving.
#   2. the two body fields the AUTHOR leg specifically needs: `shedding == false`
#      AND `storageServing.status == "serving"` — the declared primary storage
#      peer's own answer to "can I anchor writes through my conductor right
#      now?". The body is stricter than the status here: "not-observed" and
#      "not-configured" do not 503 the route but cannot anchor a head either.
# Anything else — a 503, a 404 from an image predating the route, a 502 from
# the ingress, a fetch failure, a non-JSON body — sorts after the serving
# hosts. Both body fields are type-checked: a string "false" is not false. The
# diagnostic names the status and all five fields so the build log shows WHICH
# arm a host failed on.
#
# Env: SERVING_ORDER_FETCH_CMD  the fetcher (default curl; the test points it
#                               at a stub). Invoked as:
#                                 <cmd> -sS -m 10 -o <body-file> -w '%{http_code}' <url>/health/serving
#                               stdout = the HTTP status code; the body goes to
#                               the file. Non-zero exit = could not be asked.
#
# node, not jq and not python — the same reasoning as stage-spa-blob.sh, which
# runs in the same step: jq is not in the checked-in CI-builder image and python
# is not on the deploy path (scripts/ci/.epr-meta), while node is what the
# calling step already runs. No node → every host reads not-serving → the input
# order comes back, with one warning naming the runner problem.
set -uo pipefail

FETCH="${SERVING_ORDER_FETCH_CMD:-curl}"

[ "$#" -gt 0 ] || exit 0

# Same shape as stage-spa-blob.sh's classifier: the body reaches node as a
# FILE, never through a shell variable; the HTTP status is argv[2]. Anything
# over 1 MiB, not JSON, not an object root, not HTTP 200, or not carrying BOTH
# author-leg fields with the right types is a clean negative. stdout carries a
# one-line summary for the diagnostic (http= first, then every field the
# doorway's 503 predicate reads, present or not); the exit status carries the
# verdict (0 serving, 1 not).
JSON_PROGRAM='
const fs = require("fs");
const http = process.argv[2] || "none";
const no = (why) => { process.stdout.write("http=" + http + " " + why); process.exit(1); };
let buf;
try { buf = fs.readFileSync(process.argv[1]); } catch (e) { no("body unreadable"); }
if (buf.length === 0) no("empty body");
if (buf.length > 1048576) no("body over 1 MiB");
let o;
try { o = JSON.parse(buf.toString("utf8")); } catch (e) { no("body is not JSON"); }
if (o === null || typeof o !== "object" || Array.isArray(o)) no("body is not an object");
const has = (obj, k) => obj !== null && typeof obj === "object" && Object.prototype.hasOwnProperty.call(obj, k);
const shed = has(o, "shedding") ? o.shedding : undefined;
const st = has(o.storageServing, "status") ? o.storageServing.status : undefined;
const summary = "shedding=" + JSON.stringify(shed)
  + " degrading=" + JSON.stringify(has(o, "degrading") ? o.degrading : undefined)
  + " rolesDiscovered=" + JSON.stringify(has(o, "rolesDiscovered") ? o.rolesDiscovered : undefined)
  + " warmupEmpty=" + JSON.stringify(has(o, "warmupEmpty") ? o.warmupEmpty : undefined)
  + " storageServing=" + JSON.stringify(st);
if (http === "200" && shed === false && st === "serving") { process.stdout.write("http=200 " + summary); process.exit(0); }
no(summary);
'

tmp="$(mktemp -d 2>/dev/null)" || tmp=""
if [ -z "${tmp}" ] || [ ! -d "${tmp}" ]; then
    echo "doorway-serving-order: WARN mktemp failed — cannot hold a response body; returning the input order" >&2
    printf '%s\n' "$@"
    exit 0
fi
trap 'rm -rf "${tmp}"' EXIT

HAVE_NODE=1
if ! command -v node >/dev/null 2>&1; then
    HAVE_NODE=0
    echo "doorway-serving-order: WARN node is not executable — no body can be classified, every host reads not-serving; returning the input order" >&2
fi

# serving_now <url>: 0 when the doorway says it is serving, 1 otherwise. One
# stderr line either way, naming the status and the reason, so the build log
# shows why a host was offered first or last.
serving_now() {
    local url="$1" body="${tmp}/body" err="${tmp}/err" rc code verdict
    [ "${HAVE_NODE}" -eq 1 ] || return 1
    : > "${body}"; : > "${err}"
    code="$("${FETCH}" -sS -m 10 -o "${body}" -w '%{http_code}' "${url%/}/health/serving" 2>"${err}")"; rc=$?
    if [ "${rc}" -ne 0 ]; then
        echo "doorway-serving-order: ${url} not serving — fetch failed (exit ${rc}: $(head -c 200 "${err}" | tr -d '\r\n'))" >&2
        return 1
    fi
    # %{http_code} is three digits; a stray newline from a fetcher is stripped,
    # anything longer is truncated so the diagnostic stays one bounded line.
    code="$(printf '%s' "${code}" | tr -d '[:space:]' | head -c 16)"
    verdict="$(node -e "${JSON_PROGRAM}" -- "${body}" "${code}" 2>/dev/null)"; rc=$?
    if [ "${rc}" -eq 0 ]; then
        echo "doorway-serving-order: ${url} serving (${verdict})" >&2
        return 0
    fi
    echo "doorway-serving-order: ${url} not serving (${verdict:-classifier could not run})" >&2
    return 1
}

serving=""
rest=""
for url in "$@"; do
    if serving_now "${url}"; then
        serving="${serving}${url}"$'\n'
    else
        rest="${rest}${url}"$'\n'
    fi
done
printf '%s%s' "${serving}" "${rest}"
exit 0
