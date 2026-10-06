#!/bin/bash
# app-release-stage.sh — the App pipeline's ONE native delivery step: publish this
# build's bundles as one release through ONE doorway, then measure adoption per
# peer. Never a per-host write (native-delivery N6 fleet leg; the retired per-host
# path is stageSpaBlobs + authorHeadOnce in the root Jenkinsfile, kept behind
# APP_DELIVERY_LEGACY for one release as rollback).
#
# Usage: app-release-stage.sh <doorway-url> <manifest-out> <peer>[,<peer>...]
#
# Composes two scripts, each with its own tests:
#   publish-app-release.sh <doorway-url> <manifest-out>
#       APP-RELEASE-PUBLISHED channel=<id> release=<cid>  → measure adoption of <cid>
#       APP-RELEASE-CURRENT channel=<id> release=<cid>    → nothing new to publish
#         (idempotent by CONTENT); adoption of the current release is STILL
#         measured, so a rebuild re-proves a peer that had not adopted yet
#       exit 2                                            → refused
#   verify-app-adoption.sh <doorway-url> <cid> <peer>...
#       0 every peer adopted · 1 a peer refused app_bundle_cannot_boot · 3 bound elapsed
#
# BRANCH GATE. Only the dev branch has a release channel (runtime:app-bundle:
# alpha:dev). APP_RELEASE_BRANCH names the pipeline's branch; any value other
# than `dev` prints `skipped=branch-<name>-has-no-channel` and exits 0 with no
# publish. Unset (a household or hand run) means no gate.
#
# STEWARD ACT. Only when APP_RELEASE_CHANNEL_CREATE=1 (the Jenkinsfile sets it
# for a tip commit carrying [app:channel-create]), before publishing, in order:
#   1. the channel answers 404 behind the doorway, OR its row answers 200 while
#      its /head names no notarized headActionHash (half-created: a create whose
#      notarize failed — the ceremony resumes it) →
#        release-ceremony.ts channel create <channel> --reach commons --transport doorway --doorway <url>
#      Every existence/head probe re-probes while the doorway answers 503
#      {"status":"catching-up"}, bounded by APP_RELEASE_CHANNEL_VISIBLE_SECS; any
#      other status than 200/404 refuses channel-create-failed.
#   2. every slug in APP_RELEASE_BUNDLES whose record does not name the channel →
#        release-ceremony.ts channel bind <slug> <channel> --transport doorway --doorway <url>
#      A release refused `app_slug_not_bound_to_channel` is never retried, so the
#      binding must precede the first publish — and is only done when each newly
#      bound slug READS BACK naming the channel on two consecutive reads within
#      APP_RELEASE_CHANNEL_VISIBLE_SECS (else refused channel-bind-not-elected).
#
# Output: everything the composed scripts print, then ONE summary line:
#   APP-RELEASE-CHANNEL-CREATED channel=<id> | APP-RELEASE-CHANNEL-EXISTS channel=<id>
#   APP-RELEASE-CHANNEL-RESUMED channel=<id>  (a half-created channel, now notarized)
#   APP-RELEASE-CHANNEL-BOUND slugs=<a,b> | APP-RELEASE-CHANNEL-ALREADY-BOUND slugs=<a,b>
#                                             (steward act only, before the publish)
#   APP-RELEASE-STAGE skipped=branch-<name>-has-no-channel
#   APP-RELEASE-STAGE released=<cid> adopted=<peer,...>
#   APP-RELEASE-STAGE released=<cid> adopted=<peer,...> pending=<peer,...>
#   APP-RELEASE-STAGE released=<cid> adopted=<peer,...> pending=<peer,...> bind-not-visible=<peer,...>
#                               (a pending peer refused the release as
#                               app_slug_not_bound_to_channel: its OWN slug row did
#                               not name the channel when the release arrived — a
#                               terminal refusal for this release on that peer. No
#                               per-peer slug-row read exists through the doorway
#                               (only GET /db/p2p/adoption?peer=), so the stage
#                               cannot wait for the bind to reach each peer before
#                               publishing; it names the cause instead.)
#   APP-RELEASE-STAGE refused=app_bundle_cannot_boot
#   APP-RELEASE-STAGE refused=channel-create-failed channel=<id> …
#   APP-RELEASE-STAGE refused=channel-bind-failed channel=<id> slug=<slug> …
#   APP-RELEASE-STAGE refused=channel-bind-not-elected channel=<id> slug=<slug> …
#   APP-RELEASE-STAGE refused=internal-error line=<n>   (this script's own failure)
#   APP-RELEASE-STAGE refused=<publish's last line | verify-exit-N | no-release-line>
#   APP-RELEASE-STAGE not-ready=upstream-catching-up doorway=<url> elapsed=<s>
#
# NOT READY (exit 5). When the ceremony's last line (a create, a bind or the
# publish — for the publish, the line before publish-app-release.sh's own
# APP-RELEASE-REFUSED) reads `still catching-up after <n>s`, its
# RELEASE_CEREMONY_RETRY_SECS budget ran out on the doorway's 503 catching-up.
# That is a READINESS condition, not a refused release: fleet-write-readiness.sh
# proves serving + a storage PUT, while a notarize/bind/publish needs the
# conductor path, and a storage projector flaps caughtUp for minutes at a time
# (App #1732: FLEET-READY alpha, then 120 s of catching-up on the notarize PATCH).
# The stage writes a deploy intent (DEPLOY_INTENT_OUT) the orchestrator's
# deploy-pending pass re-dispatches on, and exits 5 — never 2.
#
# Exit: 0 delivered+adopted, or skipped (no channel for this branch) · 1 refused
#   (the release cannot boot on a peer — the Jenkinsfile turns this into a hard
#   FAILURE) · 2 refused (channel create/bind, publish, or an adoption measure
#   that could not run) · 3 delivered, not yet proven (adoption bound elapsed)
#   · 5 not ready (the upstream was still catching up; deferred, intent written)
#   · 64 usage.
#
# Env:
#   APP_RELEASE_STAGE_OUT       also write every APP-* line to this file (the
#                               Jenkinsfile parses it into junit outcomes)
#   APP_RELEASE_BRANCH          the pipeline branch (the gate above)
#   APP_RELEASE_CHANNEL_CREATE  1 = the steward act above. Unset: an absent channel
#                               is publish-app-release.sh's own refusal, unchanged.
#   APP_RELEASE_CHANNEL_VISIBLE_SECS
#                               after a create, how long the channel may take to become
#                               readable through the doorway (default 60 — read-your-
#                               write, never a wait on the fleet)
#   APP_RELEASE_READBACK_POLL_SECS
#                               spacing of the bind read-back (default 3)
#   APP_RELEASE_TSX             the tsx runner (default <repo>/node_modules/.bin/tsx)
#   APP_RELEASE_NODE            node for the one JSON question per answer (default node;
#                               python is not on this path — scripts/ci/.epr-meta)
#   DEPLOY_INTENT_OUT           on exit 5, write the deploy intent here (the shape
#                               fleet-write-readiness.sh writes, plus "elect":"one" —
#                               this path publishes through ONE doorway — and
#                               "phase":"release"); unset: no intent
#   DEPLOY_INTENT_COMMIT        the intent's full commit (default: git HEAD of this repo)
#   DEPLOY_INTENT_ENV           the intent's env (default APP_RELEASE_BRANCH, else dev)
#   DEPLOY_INTENT_DOORWAYS      whitespace-separated doorways the re-dispatch probes
#                               (default the one doorway named)
#   APP_RELEASE_PUBLISH_SCRIPT  seam for tests (default publish-app-release.sh beside this)
#   APP_RELEASE_VERIFY_SCRIPT   seam for tests (default verify-app-adoption.sh beside this)
#   and everything publish-app-release.sh / verify-app-adoption.sh read
#   (STORAGE_API_KEY_ADMIN, APP_RELEASE_CHANNEL, APP_RELEASE_BUNDLES, …).
set -euo pipefail

if [ "$#" -ne 3 ] || [ -z "$3" ]; then
    echo "usage: app-release-stage.sh <doorway-url> <manifest-out> <peer>[,<peer>...]" >&2
    exit 64
fi
# Exit 1 means ONE thing — a peer judged the release unable to boot (the
# Jenkinsfile turns it into a hard FAILURE). Any other failed command is this
# script's own fault: refuse it as exit 2, naming the line, never a bare 1.
internal_error() {
    trap - ERR
    set +e
    echo "APP-RELEASE-STAGE refused=internal-error line=$1"
    if [ -n "${APP_RELEASE_STAGE_OUT:-}" ]; then
        echo "APP-RELEASE-STAGE refused=internal-error line=$1" >> "${APP_RELEASE_STAGE_OUT}" 2>/dev/null
    fi
    exit 2
}
trap 'internal_error "${LINENO}"' ERR
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${HERE}/../.." && pwd)"
DOORWAY="$1"
MANIFEST="$2"
read -r -a PEERS <<<"${3//,/ }"
PUBLISH="${APP_RELEASE_PUBLISH_SCRIPT:-${HERE}/publish-app-release.sh}"
VERIFY="${APP_RELEASE_VERIFY_SCRIPT:-${HERE}/verify-app-adoption.sh}"
TSX="${APP_RELEASE_TSX:-${REPO_ROOT}/node_modules/.bin/tsx}"
NODE_BIN="${APP_RELEASE_NODE:-node}"
CEREMONY="${REPO_ROOT}/genesis/a2o/scripts/release-ceremony.ts"
CHANNEL="${APP_RELEASE_CHANNEL:-runtime:app-bundle:alpha:dev}"
STAGE_OUT="${APP_RELEASE_STAGE_OUT:-}"
[ -n "${STAGE_OUT}" ] && : > "${STAGE_OUT}"

# Print, and keep every machine line (APP-*) for the Jenkinsfile's junit.
say() {
    printf '%s\n' "$1"
    if [ -n "${STAGE_OUT}" ]; then
        printf '%s\n' "$1" | grep -E '^APP-' >> "${STAGE_OUT}" || true
    fi
}

last_line() {
    printf '%s\n' "$1" | grep -v '^[[:space:]]*$' | tail -n1 || true
}

json_str() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="$(printf '%s' "${s}" | tr -d '\000-\037')"
    printf '"%s"' "${s}"
}

# The deferral's deploy intent: the fields timer-dispatch.mjs readinessRefusal
# requires (kind, full commit, doorways, a non-empty notReady), elect one.
write_deferral_intent() {
    local out="${DEPLOY_INTENT_OUT:-}" ra="$1" commit doorways="" sep="" d
    [ -n "${out}" ] || return 0
    commit="${DEPLOY_INTENT_COMMIT:-$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || true)}"
    for d in ${DEPLOY_INTENT_DOORWAYS:-${DOORWAY}}; do
        doorways+="${sep}$(json_str "${d%/}")"; sep=","
    done
    {
        printf '{"kind":"deploy-intent","version":1,"elect":"one","phase":"release",'
        printf '"commit":%s,' "$(json_str "${commit}")"
        printf '"env":%s,' "$(json_str "${DEPLOY_INTENT_ENV:-${APP_RELEASE_BRANCH:-dev}}")"
        printf '"doorway":%s,"face":"upstream-catching-up","retryAfter":%s,' "$(json_str "${DOORWAY%/}")" "${ra}"
        printf '"doorways":[%s],"notReady":[{"doorway":%s,"face":"upstream-catching-up","retryAfter":%s}],' \
            "${doorways}" "$(json_str "${DOORWAY%/}")" "${ra}"
        printf '"bundles":[],"recordedAt":%s}\n' "$(json_str "$(date -u '+%Y-%m-%dT%H:%M:%SZ')")"
    } > "${out}.tmp.$$" 2>/dev/null && mv -f "${out}.tmp.$$" "${out}" \
        || echo "app-release-stage: could not write the deploy intent to ${out}" >&2
}

# Exit 5 (NOT READY, see the header) when the ceremony's last line says its
# catching-up budget ran out; otherwise return, and the caller refuses as before.
defer_if_catching_up() {
    local last elapsed ra
    last="$(printf '%s\n' "$1" | grep -v -e '^[[:space:]]*$' -e '^APP-RELEASE-REFUSED ' | tail -n1 || true)"
    elapsed="$(printf '%s\n' "${last}" | sed -n 's/.*still catching-up after \([0-9][0-9]*\).*/\1/p')"
    [ -n "${elapsed}" ] || return 0
    ra="$(printf '%s\n' "${last}" | sed -n 's/.*"retryAfter":\([0-9][0-9]*\).*/\1/p')"
    write_deferral_intent "${ra:-2}"
    say "APP-RELEASE-STAGE not-ready=upstream-catching-up doorway=${DOORWAY%/} elapsed=${elapsed}"
    exit 5
}

# GET <doorway>/db/content/<id>: prints the status; the body lands in $2 when given.
content_status() {
    curl -sS -o "${2:-/dev/null}" -w '%{http_code}' -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
        --max-time 30 "${DOORWAY%/}/db/content/$1" 2>/dev/null || echo 000
}

# ── branch gate ─────────────────────────────────────────────────────────────
BRANCH="${APP_RELEASE_BRANCH:-}"
if [ -n "${BRANCH}" ] && [ "${BRANCH}" != "dev" ]; then
    say "APP-RELEASE-STAGE skipped=branch-${BRANCH//[^A-Za-z0-9._-]/-}-has-no-channel"
    exit 0
fi

# ── the steward act: create the channel, bind the slugs (only when asked) ────
if [ "${APP_RELEASE_CHANNEL_CREATE:-0}" = "1" ]; then
    body="$(mktemp)"
    trap 'rm -f "${body}"' EXIT
    # The doorway's "ask again shortly": 503 whose JSON says status catching-up
    # (its upstream is still catching up, e.g. right after a doorway restart).
    catching_up() {
        grep -Eq '"status"[[:space:]]*:[[:space:]]*"catching-up"' "$1" 2>/dev/null
    }
    # GET <doorway>/db/content/<path>, re-probed while the doorway answers
    # catching-up, bounded by APP_RELEASE_CHANNEL_VISIBLE_SECS. Prints the settled
    # status; the settled body lands in ${body}. Any other answer is final.
    settled_status() {
        local deadline st
        deadline=$(( $(date +%s) + ${APP_RELEASE_CHANNEL_VISIBLE_SECS:-60} ))
        while :; do
            : > "${body}"
            st="$(content_status "$1" "${body}")"
            if [ "${st}" != "503" ] || ! catching_up "${body}" || [ "$(date +%s)" -ge "${deadline}" ]; then
                break
            fi
            echo "app-release-stage: GET /db/content/$1 503 catching-up — re-probing" >&2
            sleep "${APP_RELEASE_READBACK_POLL_SECS:-3}"
        done
        printf '%s' "${st}"
    }
    # yes when the settled body names a notarized head.
    names_head() {
        "${NODE_BIN}" -e '
            const fs = require("fs");
            try { const b = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
                  if (b && typeof b.headActionHash === "string" && b.headActionHash) process.stdout.write("yes"); }
            catch {}' "${body}"
    }
    # The ceremony's channel create: creates an absent channel, and RESUMES one
    # whose row exists with no notarized head (a create whose notarize failed).
    run_create() {
        local crc=0 cout
        cout="$("${TSX}" "${CEREMONY}" channel create "${CHANNEL}" \
            --reach commons --transport doorway --doorway "${DOORWAY%/}" 2>&1)" || crc=$?
        say "${cout}"
        if [ "${crc}" -ne 0 ]; then
            defer_if_catching_up "${cout}"
            say "APP-RELEASE-STAGE refused=channel-create-failed channel=${CHANNEL} exit=${crc} $(last_line "${cout}")"
            exit 2
        fi
    }
    # Read-your-write after a create or resume: the channel's head is notarized.
    await_notarized() {
        local deadline
        deadline=$(( $(date +%s) + ${APP_RELEASE_CHANNEL_VISIBLE_SECS:-60} ))
        until [ "$(settled_status "${CHANNEL}/head")" = "200" ] && [ "$(names_head)" = "yes" ]; do
            if [ "$(date +%s)" -ge "${deadline}" ]; then
                say "APP-RELEASE-STAGE refused=channel-create-failed channel=${CHANNEL} the create exited 0 but ${DOORWAY} serves no notarized head for the channel"
                exit 2
            fi
            sleep "${APP_RELEASE_READBACK_POLL_SECS:-3}"
        done
    }

    status="$(settled_status "${CHANNEL}")"
    case "${status}" in
        200)
            hstatus="$(settled_status "${CHANNEL}/head")"
            if [ "${hstatus}" = "200" ] && [ "$(names_head)" = "yes" ]; then
                say "APP-RELEASE-CHANNEL-EXISTS channel=${CHANNEL}"
            elif [ "${hstatus}" = "200" ] || [ "${hstatus}" = "404" ]; then
                # Half-created: the row exists, the notary holds no head for it.
                run_create
                await_notarized
                say "APP-RELEASE-CHANNEL-RESUMED channel=${CHANNEL}"
            else
                say "APP-RELEASE-STAGE refused=channel-create-failed channel=${CHANNEL} head-status=${hstatus} (whether the channel is notarized could not be read through ${DOORWAY})"
                exit 2
            fi ;;
        404)
            run_create
            deadline=$(( $(date +%s) + ${APP_RELEASE_CHANNEL_VISIBLE_SECS:-60} ))
            until [ "$(content_status "${CHANNEL}")" = "200" ]; do
                if [ "$(date +%s)" -ge "${deadline}" ]; then
                    say "APP-RELEASE-STAGE refused=channel-create-failed channel=${CHANNEL} the create exited 0 but the channel is not readable through ${DOORWAY}"
                    exit 2
                fi
                sleep 3
            done
            say "APP-RELEASE-CHANNEL-CREATED channel=${CHANNEL}" ;;
        *)
            say "APP-RELEASE-STAGE refused=channel-create-failed channel=${CHANNEL} status=${status} (its existence could not be read through ${DOORWAY})"
            exit 2 ;;
    esac

    slugs=()
    for spec in ${APP_RELEASE_BUNDLES:-}; do
        slug="${spec%%:*}"
        case " ${slugs[*]:-} " in *" ${slug} "*) : ;; *) slugs+=("${slug}") ;; esac
    done
    if [ "${#slugs[@]}" -eq 0 ]; then
        say "APP-RELEASE-STAGE refused=channel-bind-failed channel=${CHANNEL} APP_RELEASE_BUNDLES names no slug to bind"
        exit 2
    fi
    # yes when the slug's record, as the doorway serves it, names the channel.
    slug_bound() {
        : > "${body}"
        [ "$(content_status "$1" "${body}")" = "200" ] || return 0
        "${NODE_BIN}" -e '
            const fs = require("fs");
            try { const b = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
                  if (b && b.metadata && b.metadata.releaseChannel === process.argv[2]) process.stdout.write("yes"); }
            catch {}' "${body}" "${CHANNEL}"
    }
    newly=()
    for slug in "${slugs[@]}"; do
        [ "$(slug_bound "${slug}")" = "yes" ] && continue
        brc=0
        bout="$("${TSX}" "${CEREMONY}" channel bind "${slug}" "${CHANNEL}" \
            --transport doorway --doorway "${DOORWAY%/}" 2>&1)" || brc=$?
        say "${bout}"
        if [ "${brc}" -ne 0 ]; then
            defer_if_catching_up "${bout}"
            say "APP-RELEASE-STAGE refused=channel-bind-failed channel=${CHANNEL} slug=${slug} exit=${brc} $(last_line "${bout}")"
            exit 2
        fi
        newly+=("${slug}")
    done
    for slug in "${newly[@]}"; do
        deadline=$(( $(date +%s) + ${APP_RELEASE_CHANNEL_VISIBLE_SECS:-60} ))
        seen=0
        while [ "${seen}" -lt 2 ]; do
            if [ "$(slug_bound "${slug}")" = "yes" ]; then seen=$(( seen + 1 )); else seen=0; fi
            [ "${seen}" -ge 2 ] && break
            if [ "$(date +%s)" -ge "${deadline}" ]; then
                say "APP-RELEASE-STAGE refused=channel-bind-not-elected channel=${CHANNEL} slug=${slug} the bind exited 0 but ${DOORWAY} does not serve the slug bound to the channel (an earned head out-votes a staging bind — promote it, then re-run); nothing published"
                exit 2
            fi
            sleep "${APP_RELEASE_READBACK_POLL_SECS:-3}"
        done
    done
    if [ "${#newly[@]}" -gt 0 ]; then
        say "APP-RELEASE-CHANNEL-BOUND slugs=$(IFS=,; echo "${newly[*]}")"
    else
        say "APP-RELEASE-CHANNEL-ALREADY-BOUND slugs=$(IFS=,; echo "${slugs[*]}")"
    fi
fi

# ── publish ─────────────────────────────────────────────────────────────────
# stderr is folded in: publish-app-release.sh prints its REFUSED line there, and
# that line is what the refusal summary must name.
prc=0
out="$(bash "${PUBLISH}" "${DOORWAY}" "${MANIFEST}" 2>&1)" || prc=$?
say "${out}"
if [ "${prc}" -ne 0 ]; then
    defer_if_catching_up "${out}"
    last="$(last_line "${out}")"
    say "APP-RELEASE-STAGE refused=${last:-publish-exit-${prc}}"
    exit 2
fi
line="$(printf '%s\n' "${out}" | grep -E '^APP-RELEASE-(PUBLISHED|CURRENT) ' | tail -n1 || true)"
cid="${line##*release=}"
if [ -z "${line}" ] || [ -z "${cid}" ] || [ "${cid}" = "unknown" ]; then
    say "APP-RELEASE-STAGE refused=no-release-line (publish exited 0 without naming a release)"
    exit 2
fi

# ── measure adoption (a CURRENT release too: a rebuild re-proves a lagging peer) ─
rc=0
vout="$(bash "${VERIFY}" "${DOORWAY}" "${cid}" "${PEERS[@]}")" || rc=$?
say "${vout}"
adopted="$(printf '%s\n' "${vout}" | sed -n 's/^APP-ADOPTED \([^ ]*\).*/\1/p' | paste -sd, -)"
pending="$(printf '%s\n' "${vout}" | sed -n 's/^APP-NOT-ADOPTED \([^ ]*\).*/\1/p' | paste -sd, -)"
# The pending peers whose refusal is "the bind had not reached my row": named so
# the summary carries the cause, not only the downstream refusal.
unbound="$(printf '%s\n' "${vout}" \
    | sed -n 's/^APP-NOT-ADOPTED \([^ ]*\) .*reason=app_slug_not_bound_to_channel.*/\1/p' | paste -sd, -)"
case "${rc}" in
    0) say "APP-RELEASE-STAGE released=${cid} adopted=${adopted}"; exit 0 ;;
    1) say "APP-RELEASE-STAGE refused=app_bundle_cannot_boot"; exit 1 ;;
    3) say "APP-RELEASE-STAGE released=${cid} adopted=${adopted} pending=${pending}${unbound:+ bind-not-visible=${unbound}}"; exit 3 ;;
    *) say "APP-RELEASE-STAGE refused=verify-exit-${rc}"; exit 2 ;;
esac
