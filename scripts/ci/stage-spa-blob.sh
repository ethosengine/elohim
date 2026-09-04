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
#        STAGE_BLOB_ATTEMPTS    max attempts per leg (default 3)
set -euo pipefail

DIST_DIR="$1"
SLUG="$2"
DOORWAY_EPR_URL="$3"
KIND="${4:-browser}"
DO_PATCH="${DO_PATCH:-0}"
ATTEMPTS="${STAGE_BLOB_ATTEMPTS:-3}"

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
    declare_body_file="$(mktemp)"
    trap 'rm -f "${declare_body_file}"' EXIT
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
    while :; do
        attempt=$((attempt + 1))
        declare_raw=$(curl -sS -o - -w '\n%{http_code}' -X POST \
            -H 'Content-Type: application/json' \
            -H "X-API-Key: ${STORAGE_API_KEY_ADMIN}" \
            --data-binary "@${declare_body_file}" \
            "${DOORWAY_EPR_URL}/db/content/${SLUG}/canonical-head" 2>&1) || {
            echo "  ⚠ DECLARE_ONLY: curl error POSTing canonical-head to ${DOORWAY_EPR_URL}: ${declare_raw} — peer keeps its own head until gossip/heal converges" >&2
            exit 1
        }
        declare_status="${declare_raw##*$'\n'}"
        declare_body="${declare_raw%$'\n'*}"
        case "${declare_status}" in
            2??)
                echo "  ✓ canonical head propagated to ${DOORWAY_EPR_URL}: ${head_hash} (staging tier, attempt ${attempt})"
                declare_ok=1
                break
                ;;
            503|429)
                if [ "${attempt}" -lt "${max_attempts}" ]; then
                    echo "  … DECLARE_ONLY attempt ${attempt}/${max_attempts}: ${DOORWAY_EPR_URL} shedding writes (HTTP ${declare_status}: ${declare_body}) — retrying in 30s" >&2
                    sleep 30
                    continue
                fi
                echo "  ⚠ DECLARE_ONLY: ${DOORWAY_EPR_URL} still shedding after ${attempt} attempts (HTTP ${declare_status}: ${declare_body}) — peer keeps its own head until gossip/heal converges" >&2
                break
                ;;
            *)
                if [ "${attempt}" -lt "${max_attempts}" ] && \
                   printf '%s' "${declare_body}" | grep -q "not retrievable"; then
                    echo "  … DECLARE_ONLY attempt ${attempt}/${max_attempts}: target not retrievable yet on ${DOORWAY_EPR_URL} (DHT publish lag) — retrying in 90s" >&2
                    sleep 90
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
                if [ "${attempt}" -lt "${max_attempts}" ] && \
                   printf '%s' "${declare_body}" | grep -q "conductor admission: shed"; then
                    echo "  … DECLARE_ONLY attempt ${attempt}/${max_attempts}: ${DOORWAY_EPR_URL} shed the call at the conductor-admission gate (HTTP ${declare_status}: ${declare_body}) — nothing was dispatched, retrying in 30s" >&2
                    sleep 30
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

cd "${DIST_DIR}"

# Angular 19 SSR mode emits index.csr.html (Client-Side Rendered fallback)
# instead of index.html. For static SPA delivery through the protocol's
# /apps/{slug}/index.html route we need a literal index.html (storage's /apps
# handler is literal-path, doesn't fall back to index.csr.html). Pure SPAs
# (app/lamad) pass through unchanged. Server bundles have no index.html at
# all — skip this step entirely for KIND=server.
if [ "$KIND" = "browser" ]; then
    if [ ! -f index.html ] && [ -f index.csr.html ]; then
        cp index.csr.html index.html
        echo "  [${SLUG}] materialized index.html from index.csr.html (Angular SSR-mode dist)"
    fi
fi

# Local + deterministic — done ONCE, not retried (a zip/sha failure is not a
# transient network blip and re-zipping yields the identical content-addressed
# hash anyway).
zip -r spa-bundle.zip .
SPA_HASH="sha256-$(sha256sum spa-bundle.zip | awk '{print $1}')"
SPA_SIZE="$(du -h spa-bundle.zip | cut -f1)"
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
    local put_rc=0
    local put_response=""
    put_response="$(curl -fSs -X PUT \
        -H 'Content-Type: application/zip' \
        -H "X-Blob-Hash: ${SPA_HASH}" \
        -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
        --data-binary @spa-bundle.zip \
        "${DOORWAY_EPR_URL}/admin/seed/blob")" || put_rc=$?
    [ -n "${put_response}" ] && echo "${put_response}"
    if [ "${put_rc}" -eq 0 ]; then
        # A 200 with forwarded_to_storage:false means the bytes reached ONLY the
        # doorway's 1h write-through cache — storage (the authoritative store)
        # never durably accepted them. Declaring a head against those bytes
        # mints a declared-vs-available divergence (2026-08-22 local mesh: a
        # 69MB bundle hit a storage-side shard-verify drop; the leg stamped ✓
        # and every peer declared a bundle nobody could materialize). The
        # retry ladder is the right response: the doorway re-forwards from its
        # cache on each already_cached re-PUT, so a transient boot-pressure
        # failure heals on retry, and a persistent one reds the stage honestly.
        if printf '%s' "${put_response}" | grep -q '"forwarded_to_storage":false'; then
            echo "  ✗ [${SLUG}] doorway cached the blob but storage forwarding FAILED (forwarded_to_storage:false) — refusing to call this staged" >&2
            return 1
        fi
        echo "  ✓ [${SLUG}] blob uploaded (via /admin/seed/blob)"
    elif [ "${put_rc}" -eq 55 ] || [ "${put_rc}" -eq 56 ]; then
        if curl -fsS -o /dev/null --max-time 60 "${DOORWAY_EPR_URL}/blob/${SPA_HASH}"; then
            echo "  ✓ [${SLUG}] blob already stored (re-PUT short-circuited by cache; verified by content address ${SPA_HASH})"
        else
            echo "  ✗ [${SLUG}] PUT broke (curl ${put_rc}) and ${SPA_HASH} is NOT retrievable — real upload failure" >&2
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
        curl -fSs -X PATCH \
            -H 'Content-Type: application/json' \
            -H "X-API-Key: ${STORAGE_API_KEY_ADMIN:-}" \
            -d "{\"${HASH_FIELD}\":\"${SPA_HASH}\"}" \
            "${DOORWAY_EPR_URL}${PATCH_PATH}" \
            >/dev/null || return 1
        echo "  ✓ patched ${SLUG} (${HASH_FIELD})"

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

# Bounded retry with linear backoff. A conductor-bridged backend can 503
# transiently during cluster churn (the notarized PATCH round-trips the
# conductor); retry rather than surrender on a blip. A PERSISTENT non-zero exit
# is meaningful: for a byte-seed (DO_PATCH!=1) it means the blob upload failed;
# for an author attempt (DO_PATCH=1) it means THIS doorway could not author (no
# live conductor bridge / persistent 503), and the Jenkinsfile fails the head
# author over to the next doorway. The deploy is only UNSTABLE if NO doorway
# in the fabric can author the single head.
attempt=1
while true; do
    rc=0
    stage_once || rc=$?
    if [ "${rc}" -eq 0 ]; then
        break
    fi
    # A deterministic peer VERDICT (broken bundle, or NOT-JUDGED under strict
    # mode) is not a transport blip — retrying re-asks a question the peer has
    # already answered. Terminal, immediately, never counted against ATTEMPTS.
    if [ "${rc}" -eq 2 ]; then
        echo "ERROR: [${SLUG}] the peer judged ${SPA_HASH} BROKEN — not retrying a deterministic verdict" >&2
        rm -f spa-bundle.zip
        exit 2
    fi
    if [ "${attempt}" -ge "${ATTEMPTS}" ]; then
        echo "ERROR: [${SLUG}] stage failed after ${ATTEMPTS} attempt(s) against ${DOORWAY_EPR_URL} — host left STALE" >&2
        rm -f spa-bundle.zip
        exit 1
    fi
    backoff=$(( attempt * 5 ))
    echo "  ⚠ [${SLUG}] attempt ${attempt}/${ATTEMPTS} against ${DOORWAY_EPR_URL} failed — retrying in ${backoff}s" >&2
    attempt=$(( attempt + 1 ))
    sleep "${backoff}"
done

rm -f spa-bundle.zip
