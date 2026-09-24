#!/usr/bin/env bash
# doorway-serving-order.sh — order the doorway EPR URLs so the hosts that can
# author a head NOW are offered first.
#
# WHY (app #1725, 2026-09-23): authorHeadOnce in the root Jenkinsfile offered
# the head PATCH to the doorways in a FIXED order (alpha.elohim.host, then
# elohim.host), and stage-spa-blob.sh's readiness wait spends the run's ONE
# 7200 s deadline on whichever host it is offered first. #1725 waited on alpha
# (matthew: storage shedding writes because its content cell stayed
# CellDisabled after the edge roll) from 21:40:08Z to ~23:41:58Z while
# elohim.host (adam) had logged "apps enabled" at 22:48:27Z — about 53 minutes
# on a host that could not author while another could. Asking each doorway
# first costs one bounded GET per host.
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
# SERVING means the doorway's own `/health/serving` body (doorway-service
# routes/health.rs, ServingHealth) says `shedding == false` AND
# `storageServing.status == "serving"` — the declared primary storage peer's
# own answer to "can I anchor writes through my conductor right now?". The
# other observed values are "refused" (storage alive, 503: cannot anchor) and
# "unreachable" (doorway could not ask); "not-observed" / "not-configured" are
# not serving either. The HTTP status is deliberately not the verdict: the route
# 503s with the SAME body, and the body carries the finer answer the author leg
# needs. Both fields are type-checked: a string "false" is not false.
#
# Env: SERVING_ORDER_FETCH_CMD  the fetcher (default curl; the test points it
#                               at a stub). Invoked as:
#                                 <cmd> -sS -m 10 -o <body-file> <url>/health/serving
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
# FILE, never through a shell variable; anything over 1 MiB, not JSON, not an
# object root, or not carrying BOTH fields with the right types is a clean
# negative. stdout carries a one-line summary for the diagnostic; the exit
# status carries the verdict (0 serving, 1 not).
JSON_PROGRAM='
const fs = require("fs");
const no = (why) => { process.stdout.write(why); process.exit(1); };
let buf;
try { buf = fs.readFileSync(process.argv[1]); } catch (e) { no("body unreadable"); }
if (buf.length === 0) no("empty body");
if (buf.length > 1048576) no("body over 1 MiB");
let o;
try { o = JSON.parse(buf.toString("utf8")); } catch (e) { no("body is not JSON"); }
if (o === null || typeof o !== "object" || Array.isArray(o)) no("body is not an object");
const shed = Object.prototype.hasOwnProperty.call(o, "shedding") ? o.shedding : undefined;
const ss = o.storageServing;
const st = (ss !== null && typeof ss === "object" && Object.prototype.hasOwnProperty.call(ss, "status")) ? ss.status : undefined;
const summary = "shedding=" + JSON.stringify(shed) + " storageServing=" + JSON.stringify(st);
if (shed === false && st === "serving") { process.stdout.write(summary); process.exit(0); }
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
# stderr line either way, naming the reason, so the build log shows why a host
# was offered first or last.
serving_now() {
    local url="$1" body="${tmp}/body" err="${tmp}/err" rc verdict
    [ "${HAVE_NODE}" -eq 1 ] || return 1
    : > "${body}"; : > "${err}"
    "${FETCH}" -sS -m 10 -o "${body}" "${url%/}/health/serving" >/dev/null 2>"${err}"; rc=$?
    if [ "${rc}" -ne 0 ]; then
        echo "doorway-serving-order: ${url} not serving — fetch failed (exit ${rc}: $(head -c 200 "${err}" | tr -d '\r\n'))" >&2
        return 1
    fi
    verdict="$(node -e "${JSON_PROGRAM}" -- "${body}" 2>/dev/null)"; rc=$?
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
