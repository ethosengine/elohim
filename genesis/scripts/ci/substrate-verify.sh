#!/usr/bin/env bash
# substrate-verify.sh — runtime-layer substrate validation for the genesis pipeline.
#
# Replaces the former "M1"-named stages with assertions that prove, from the
# RUNNING substrate (never from seed-side bookkeeping):
#
#   mesh         peer mesh formed: per-pod libp2p peer counts, adjacency,
#                version parity, doorway pool health
#   upload       stage blob-backed content onto the genesis peer via doorway,
#                plus a BUILD-UNIQUE propagation probe blob (fossil-proof:
#                a cross-pod fetch of it can never be satisfied by leftovers
#                from a previous run — the #1110 "pass" was exactly that)
#   propagation  cross-pod blob distribution: custody manifest present,
#                target pod serves the probe blob via the p2p data plane
#                (polled — heal-on-read + custody sweep need real time),
#                bytes persisted on disk (inventory-parity filesystem delta)
#   delivery     serve-blob REA EconomicEvents emitted for this build's
#                transfers (the delivery layer of the three-surface model)
#   projection   projector cursors caught up per pod; replication/pull/
#                projection-reconcile streams report caught_up
#   federation   doorway federation membership + p2p bootstrap surface
#   resilience   conductor-signal-driven posture (peer-statuses, network
#                posture) — stage-gated on CONDUCTOR_SEEDING_READY because
#                these projections only fill once conductor seeding runs
#
# Design notes:
#   - Blob presence checks use GET, never HEAD (head/get asymmetry: HEAD does
#     not trigger heal-on-read and lies about fetchability).
#   - Assertions accumulate; the report is always written; exit 1 iff any
#     assertion FAILED. Stages wrap calls in catchError(UNSTABLE).
#   - Spec lineage: genesis/docs/superpowers/specs/
#     2026-05-02-blob-custody-reconciliation-design.md (manifest/reality/diff);
#     successor plan genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md.
#
# Env (common):
#   PEER_STORAGE_URLS   comma-separated name=host:port (storage HTTP, :8090)
#   INTERNAL_DOORWAY_URL host:port of in-cluster doorway (no scheme)
#   REPORT_DIR          where substrate-verify-<cmd>.json lands (default .)
#   STATE_DIR           cross-stage scratch (default /tmp/substrate-verify)
set -uo pipefail

CMD="${1:-}"
REPORT_DIR="${REPORT_DIR:-.}"
STATE_DIR="${STATE_DIR:-/tmp/substrate-verify}"
CURL_TIMEOUT="${CURL_TIMEOUT:-10}"
mkdir -p "$STATE_DIR"

command -v jq >/dev/null || { echo "FATAL: jq is required"; exit 2; }
[[ "${BASH_VERSINFO[0]}" -ge 4 ]] || { echo "FATAL: bash 4+ required for associative arrays (got $BASH_VERSION)"; exit 2; }

# ---------------------------------------------------------------------------
# assertion plumbing
# ---------------------------------------------------------------------------
ASSERTIONS_FILE="$(mktemp)"
FAIL_COUNT=0

note() { printf '   %s\n' "$1"; }

record() { # record <status> <name> <detail>
  jq -cn --arg s "$1" --arg n "$2" --arg d "$3" \
    '{status:$s, name:$n, detail:$d}' >>"$ASSERTIONS_FILE"
}

pass() { record pass "$1" "$2"; printf '   ✅ %s — %s\n' "$1" "$2"; }
warn() { record warn "$1" "$2"; printf '   ⚠️  %s — %s\n' "$1" "$2"; }
fail() { record fail "$1" "$2"; printf '   ❌ %s — %s\n' "$1" "$2"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

write_report() { # write_report <cmd> <extra-json>
  local out="$REPORT_DIR/substrate-verify-$1.json"
  # NB: NOT ${2:-{}} — inside ${...} the first } would close the expansion
  # and append a literal } to any provided value (caught in smoke test).
  local extra="${2:-}"
  [ -n "$extra" ] || extra='{}'
  jq -n \
    --arg cmd "$1" \
    --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --argjson extra "$extra" \
    --slurpfile assertions "$ASSERTIONS_FILE" \
    '{schemaVersion:"1", command:$cmd, generatedAt:$at,
      failed:([$assertions[]|select(.status=="fail")]|length),
      assertions:$assertions} + {context:$extra}' >"$out" \
    || jq -n --arg cmd "$1" --slurpfile assertions "$ASSERTIONS_FILE" \
         '{schemaVersion:"1", command:$cmd, error:"report-context-assembly-failed", assertions:$assertions}' >"$out"
  echo ""
  echo "Report: $out ($FAIL_COUNT failed assertion(s))"
}

finish() { # finish <cmd> <extra-json>
  write_report "$1" "${2:-}"
  [ "$FAIL_COUNT" -eq 0 ]
}

# http_get <url> -> body on stdout, exit 0 iff HTTP 200. Status is written to
# a state file (NOT a shell var — callers invoke this inside command
# substitutions, i.e. subshells, where variable assignments are lost).
http_get() {
  local url="$1" body status
  if body="$(curl -sS -m "$CURL_TIMEOUT" -w '\n%{http_code}' "$url" 2>/dev/null)"; then
    status="${body##*$'\n'}"
    echo "$status" >"$STATE_DIR/.last_status"
    printf '%s' "${body%$'\n'*}"
    [ "$status" = "200" ]
  else
    echo "000" >"$STATE_DIR/.last_status"
    return 1
  fi
}
last_status() { cat "$STATE_DIR/.last_status" 2>/dev/null || echo "000"; }

http_status_only() { # GET, status only (still a real GET — triggers heal-on-read)
  curl -s -o /dev/null -m "$CURL_TIMEOUT" -w '%{http_code}' "$1" 2>/dev/null || echo "000"
}

# iterate PEER_STORAGE_URLS entries: sets PEER_NAME / PEER_URL per call
peers() { # usage: for entry in $(peers); do split_peer "$entry"; ... done
  echo "${PEER_STORAGE_URLS:-}" | tr ',' '\n' | sed '/^$/d'
}
split_peer() { PEER_NAME="${1%%=*}"; PEER_URL="http://${1#*=}"; }

peer_url_for() { # peer_url_for <name> -> http://host:port or empty
  local entry
  for entry in $(peers); do
    if [ "${entry%%=*}" = "$1" ]; then echo "http://${entry#*=}"; return 0; fi
  done
  return 1
}

# ===========================================================================
# mesh — peer mesh formed
# ===========================================================================
cmd_mesh() {
  local min_reachable="${MESH_MIN_REACHABLE:-2}"
  local min_connected="${MESH_MIN_CONNECTED:-1}"
  local source_peer="${SOURCE_PEER:-matthew}"
  local target_peer="${TARGET_PEER:-jessica}"
  local reachable=0 entry peers_json='[]' versions='' status_json

  echo "═══════════════════════════════════════════════════════════"
  echo "VERIFY PEER MESH — per-pod libp2p state, adjacency, parity"
  echo "═══════════════════════════════════════════════════════════"

  declare -A PEER_IDS=()
  for entry in $(peers); do
    split_peer "$entry"
    if status_json="$(http_get "$PEER_URL/p2p/status")"; then
      reachable=$((reachable + 1))
      local pid connected nat
      pid="$(jq -r '.peerId // empty' <<<"$status_json")"
      connected="$(jq -r '.connectedPeers // 0' <<<"$status_json")"
      nat="$(jq -r '.natStatus // "?"' <<<"$status_json")"
      PEER_IDS[$PEER_NAME]="$pid"
      peers_json="$(jq -c --arg n "$PEER_NAME" --argjson s "$status_json" \
        '. + [{name:$n, peerId:$s.peerId, connectedPeers:$s.connectedPeers, natStatus:$s.natStatus, relayMode:$s.relayMode}]' <<<"$peers_json")"
      if [ "$connected" -ge "$min_connected" ]; then
        pass "mesh.$PEER_NAME.connected" "connectedPeers=$connected nat=$nat"
      else
        fail "mesh.$PEER_NAME.connected" "connectedPeers=$connected < $min_connected (nat=$nat)"
      fi
      local ver
      ver="$(http_get "$PEER_URL/version" | jq -r '.commit // .version // empty' 2>/dev/null || true)"
      versions="$versions $PEER_NAME=$ver"
    else
      warn "mesh.$PEER_NAME.reachable" "storage HTTP unreachable ($PEER_URL, status $(last_status))"
    fi
  done

  if [ "$reachable" -ge "$min_reachable" ]; then
    pass "mesh.reachable" "$reachable peer pod(s) reachable (floor $min_reachable)"
  else
    fail "mesh.reachable" "only $reachable peer pod(s) reachable; floor is $min_reachable"
  fi

  # Adjacency: the propagation source must list the target in its live peer set
  # — precondition for the cross-pod fetch (heal-on-read races CONNECTED peers).
  local src_url tgt_id
  if src_url="$(peer_url_for "$source_peer")" && tgt_id="${PEER_IDS[$target_peer]:-}" && [ -n "$tgt_id" ]; then
    if http_get "$src_url/p2p/peers" | jq -e --arg id "$tgt_id" '(.peers // []) | map(.peerId // .peer_id) | index($id) != null' >/dev/null; then
      pass "mesh.adjacency" "$source_peer lists $target_peer ($tgt_id) in its connected peer set"
    else
      fail "mesh.adjacency" "$source_peer does NOT list $target_peer ($tgt_id) — cross-pod fetch will starve"
    fi
  else
    warn "mesh.adjacency" "skipped — $source_peer or $target_peer unreachable/unidentified"
  fi

  # Version parity (warn-only: partial rollouts are a real, deploy-owned state)
  if [ "$reachable" -ge 1 ]; then
    local distinct
    distinct="$(echo "$versions" | tr ' ' '\n' | sed '/^$/d' | cut -d= -f2 | sort -u | sed '/^$/d' | wc -l)"
    if [ "$distinct" -le 1 ]; then
      pass "mesh.version-parity" "all reachable pods on one binary version (${versions# })"
    else
      warn "mesh.version-parity" "version skew across pods:${versions} — partial rollout? (DHT-partition risk class)"
    fi
  fi

  # Doorway view (informational)
  if [ -n "${INTERNAL_DOORWAY_URL:-}" ]; then
    local dw
    if dw="$(http_get "http://$INTERNAL_DOORWAY_URL/health")"; then
      note "doorway: p2p.peerCount=$(jq -r '.p2p.peerCount // "?"' <<<"$dw") pools=$(jq -r '"\(.conductor.pools_healthy // "?")/\(.conductor.pools_total // "?")"' <<<"$dw")"
    fi
  fi

  finish mesh "$(jq -cn --argjson p "$peers_json" '{peers:$p}')"
}

# ===========================================================================
# upload — stage content blob + build-unique propagation probe blob
# ===========================================================================
upload_one() { # upload_one <file> <label> -> sets UP_HASH / UP_SIZE
  local file="$1" label="$2" status
  UP_SIZE="$(wc -c <"$file")"
  UP_HASH="sha256-$(sha256sum "$file" | awk '{print $1}')"
  status="$(curl -s -o "$STATE_DIR/upload-$label.out" -w '%{http_code}' -m 30 \
    -X PUT "http://${INTERNAL_DOORWAY_URL}/admin/seed/blob" \
    -H "X-Blob-Hash: $UP_HASH" \
    -H 'Content-Type: application/octet-stream' \
    --data-binary @"$file")"
  case "$status" in
    200|201) pass "upload.$label" "$UP_HASH ($UP_SIZE bytes) → HTTP $status" ;;
    409)     pass "upload.$label" "$UP_HASH ($UP_SIZE bytes) → HTTP 409 (already present, idempotent)" ;;
    *)       fail "upload.$label" "HTTP $status — $(head -c 300 "$STATE_DIR/upload-$label.out" 2>/dev/null)" ;;
  esac
}

cmd_upload() {
  local content_path="${CONTENT_PATH:-genesis/docs/content/elohim-protocol/manifesto.md}"

  echo "═══════════════════════════════════════════════════════════"
  echo "UPLOAD BLOB-BACKED CONTENT — genesis peer ingest via doorway"
  echo "═══════════════════════════════════════════════════════════"
  [ -n "${INTERNAL_DOORWAY_URL:-}" ] || { fail "upload.preflight" "INTERNAL_DOORWAY_URL unset"; finish upload; return; }
  date -u +%Y-%m-%dT%H:%M:%SZ >"$STATE_DIR/build_start_iso"

  [ -f "$content_path" ] || { fail "upload.content" "$content_path missing"; finish upload; return; }
  upload_one "$content_path" content
  echo "$UP_HASH" >"$STATE_DIR/content_blob_hash"
  echo "$UP_SIZE" >"$STATE_DIR/content_blob_size"

  # Build-unique probe blob: its hash has never existed before this build, so
  # the downstream cross-pod fetch can only be satisfied by live propagation.
  local probe="$STATE_DIR/propagation-probe.txt"
  {
    echo "elohim substrate propagation probe"
    echo "build: ${BUILD_TAG:-${BUILD_NUMBER:-local}}"
    echo "commit: ${GIT_COMMIT:-unknown}"
    echo "generated: $(cat "$STATE_DIR/build_start_iso")"
    echo "nonce: $(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')"
  } >"$probe"
  upload_one "$probe" probe
  echo "$UP_HASH" >"$STATE_DIR/probe_blob_hash"
  echo "$UP_SIZE" >"$STATE_DIR/probe_blob_size"
  note "probe blob is build-unique — a fossil copy on the target is impossible by construction"

  finish upload "$(jq -cn \
    --arg c "$(cat "$STATE_DIR/content_blob_hash")" \
    --arg p "$(cat "$STATE_DIR/probe_blob_hash")" \
    --arg t "$(cat "$STATE_DIR/build_start_iso")" \
    '{contentBlobHash:$c, probeBlobHash:$p, buildStartIso:$t}')"
}

# ===========================================================================
# propagation — cross-pod distribution via the p2p data plane
# ===========================================================================
cmd_propagation() {
  local source_peer="${SOURCE_PEER:-matthew}" target_peer="${TARGET_PEER:-jessica}"
  local content_hash="${CONTENT_BLOB_HASH:-$(cat "$STATE_DIR/content_blob_hash" 2>/dev/null)}"
  local probe_hash="${PROBE_BLOB_HASH:-$(cat "$STATE_DIR/probe_blob_hash" 2>/dev/null)}"
  local timeout_secs="${PROPAGATION_TIMEOUT_SECS:-300}" poll_secs="${PROPAGATION_POLL_SECS:-15}"
  local src_url tgt_url

  echo "═══════════════════════════════════════════════════════════"
  echo "VERIFY SUBSTRATE PROPAGATION — $target_peer must serve blobs staged on $source_peer"
  echo "═══════════════════════════════════════════════════════════"
  [ -n "$content_hash" ] || { fail "propagation.preflight" "CONTENT_BLOB_HASH unset"; finish propagation; return; }
  [ -n "$probe_hash" ]   || { fail "propagation.preflight" "PROBE_BLOB_HASH unset"; finish propagation; return; }
  src_url="$(peer_url_for "$source_peer")" || { fail "propagation.preflight" "no PEER_STORAGE_URLS entry for $source_peer"; finish propagation; return; }
  tgt_url="$(peer_url_for "$target_peer")" || { fail "propagation.preflight" "no PEER_STORAGE_URLS entry for $target_peer"; finish propagation; return; }

  # Manifest layer: a custody-blob commitment for the content blob must exist
  # (three-surface model: manifest / reality / diff).
  if http_get "$src_url/api/v1/commitments?limit=200" | \
     jq -e --arg h "$content_hash" '[.[]? | select(.action=="custody-blob" and ((.resourceClassifiedAs // []) | index($h)))] | length >= 1' >/dev/null; then
    pass "propagation.custody-manifest" "custody-blob commitment row present for $content_hash on $source_peer"
  else
    fail "propagation.custody-manifest" "no custody-blob commitment for $content_hash on $source_peer — Seed Custody Commitments leg broken"
  fi

  # Source must hold both blobs (GET, not HEAD — head/get asymmetry).
  local s
  s="$(http_status_only "$src_url/blob/$content_hash")"
  [ "$s" = "200" ] && pass "propagation.source-content" "$source_peer serves content blob" \
                   || fail "propagation.source-content" "$source_peer GET content blob → $s (upload leg broken)"
  s="$(http_status_only "$src_url/blob/$probe_hash")"
  [ "$s" = "200" ] && pass "propagation.source-probe" "$source_peer serves probe blob" \
                   || fail "propagation.source-probe" "$source_peer GET probe blob → $s (upload leg broken)"

  # Reality layer baseline on the target.
  local fs_before fs_after parity
  parity="$(http_get "$tgt_url/api/v1/diagnostics/inventory-parity")" \
    && fs_before="$(jq -r '.filesystem_count // .filesystemCount // empty' <<<"$parity")" \
    || fs_before=""
  [ -n "$fs_before" ] && note "$target_peer filesystem_count before: $fs_before" \
                      || warn "propagation.parity-baseline" "inventory-parity unavailable on $target_peer (older image?) — byte-persistence delta unprovable this run"

  # The probe fetch: poll — each GET arms heal-on-read; the custody sweep tick
  # (120s) and inventory gossip need real time after a fresh upload.
  local waited=0 attempts=0 status="000"
  while [ "$waited" -le "$timeout_secs" ]; do
    attempts=$((attempts + 1))
    status="$(http_status_only "$tgt_url/blob/$probe_hash")"
    [ "$status" = "200" ] && break
    sleep "$poll_secs"; waited=$((waited + poll_secs))
  done
  if [ "$status" = "200" ]; then
    pass "propagation.probe-fetch" "$target_peer served build-unique probe blob after ${waited}s / $attempts attempt(s) — p2p data plane moved bytes THIS build"
  else
    fail "propagation.probe-fetch" "$target_peer GET probe blob → $status after ${waited}s / $attempts attempts — substrate did not propagate (check inventory gossip + heal-on-read race + adjacency)"
  fi

  # Content blob on the target (may be served from a prior build's replica —
  # that's persistence, also worth proving; the probe above is the live test).
  s="$(http_status_only "$tgt_url/blob/$content_hash")"
  [ "$s" = "200" ] && pass "propagation.content-fetch" "$target_peer serves content blob (live or persisted replica)" \
                   || fail "propagation.content-fetch" "$target_peer GET content blob → $s"

  # Reality layer: bytes persisted, not just streamed.
  if [ -n "$fs_before" ]; then
    parity="$(http_get "$tgt_url/api/v1/diagnostics/inventory-parity")" \
      && fs_after="$(jq -r '.filesystem_count // .filesystemCount // empty' <<<"$parity")" || fs_after=""
    if [ -n "$fs_after" ] && [ "$fs_after" -gt "$fs_before" ]; then
      pass "propagation.bytes-persisted" "$target_peer filesystem_count $fs_before → $fs_after (replica on disk)"
    elif [ "$status" = "200" ]; then
      warn "propagation.bytes-persisted" "probe served but filesystem_count did not grow ($fs_before → ${fs_after:-?}) — streamed-not-persisted, or parity lag"
    fi
  fi

  finish propagation "$(jq -cn --arg c "$content_hash" --arg p "$probe_hash" \
    --arg w "$waited" --arg a "$attempts" \
    '{contentBlobHash:$c, probeBlobHash:$p, waitedSecs:($w|tonumber), attempts:($a|tonumber)}')"
}

# ===========================================================================
# delivery — serve-blob REA events emitted for this build
# ===========================================================================
cmd_delivery() {
  local after="${SUBSTRATE_BUILD_START_ISO:-$(cat "$STATE_DIR/build_start_iso" 2>/dev/null)}"
  local min_events="${DELIVERY_MIN_EVENTS:-1}" total=0 entry per_peer='{}'

  echo "═══════════════════════════════════════════════════════════"
  echo "VERIFY DELIVERY EVENTS — serve-blob REA events since $after"
  echo "═══════════════════════════════════════════════════════════"
  [ -n "$after" ] || { fail "delivery.preflight" "no build-start timestamp (upload stage skipped?)"; finish delivery; return; }

  for entry in $(peers); do
    split_peer "$entry"
    local count body
    if body="$(http_get "$PEER_URL/api/v1/economic-events?action=serve-blob&after=$after&limit=50")"; then
      count="$(jq -r 'length' <<<"$body" 2>/dev/null || echo 0)"
    else
      count=0
      warn "delivery.$PEER_NAME" "economic-events query unreachable/non-200 ($(last_status))"
    fi
    per_peer="$(jq -c --arg n "$PEER_NAME" --argjson c "${count:-0}" '. + {($n): $c}' <<<"$per_peer")"
    note "$PEER_NAME: $count serve-blob event(s) since $after"
    total=$((total + count))
  done

  if [ "$total" -ge "$min_events" ]; then
    pass "delivery.serve-blob-events" "$total serve-blob event(s) across peers — transfers leave an REA delivery trail"
  else
    fail "delivery.serve-blob-events" "0 serve-blob events since $after — either no transfer happened (see propagation) or the event-emission leg (blob_fetch atomic pair) regressed"
  fi

  finish delivery "$(jq -cn --argjson p "$per_peer" --arg a "$after" '{perPeer:$p, after:$a}')"
}

# ===========================================================================
# projection — projector cursors + sync streams caught up
# ===========================================================================
cmd_projection() {
  local max_lag="${PROJECTION_MAX_LAG_SECS:-120}" retries="${PROJECTION_RETRIES:-3}" entry

  echo "═══════════════════════════════════════════════════════════"
  echo "VERIFY PROJECTION SYNC — cursors + replication/pull streams"
  echo "═══════════════════════════════════════════════════════════"

  for entry in $(peers); do
    split_peer "$entry"
    local body laggy
    if body="$(http_get "$PEER_URL/api/v1/status/projector")"; then
      laggy="$(jq -r --argjson m "$max_lag" '[.lag[]? | select(.lagSeconds != null and .lagSeconds > $m)] | length' <<<"$body" 2>/dev/null || echo "")"
      if [ "$laggy" = "0" ]; then
        pass "projection.$PEER_NAME.lag" "no cursor lag > ${max_lag}s"
      elif [ -n "$laggy" ]; then
        fail "projection.$PEER_NAME.lag" "$laggy cursor(s) lag > ${max_lag}s: $(jq -c --argjson m "$max_lag" '[.lag[]? | select(.lagSeconds != null and .lagSeconds > $m)]' <<<"$body")"
      fi
    else
      warn "projection.$PEER_NAME.projector" "status/projector unavailable ($(last_status)) — pod-direct route, older image?"
    fi

    # Sync streams: retry briefly — right after seeding the drain is legitimate.
    local attempt=0 ok=false detail=""
    while [ "$attempt" -lt "$retries" ]; do
      attempt=$((attempt + 1))
      if body="$(http_get "$PEER_URL/p2p/status")"; then
        local repl pull projr
        repl="$(jq -r '.replication.caughtUp // .replication.caught_up // empty' <<<"$body")"
        pull="$(jq -r 'if .pull == null then "null" else (.pull.caughtUp // .pull.caught_up // false | tostring) end' <<<"$body")"
        projr="$(jq -r 'if .projectionReconcile == null and .projection_reconcile == null then "null" else ((.projectionReconcile // .projection_reconcile).caughtUp // (.projectionReconcile // .projection_reconcile).caught_up // false | tostring) end' <<<"$body")"
        detail="replication=$repl pull=$pull projection_reconcile=$projr"
        if [ "$repl" = "true" ] && [ "$pull" != "false" ] && [ "$projr" != "false" ]; then ok=true; break; fi
      else
        detail="p2p/status unreachable ($(last_status))"
      fi
      sleep 10
    done
    if $ok; then
      case "$detail" in
        *null*) warn "projection.$PEER_NAME.streams" "$detail — null streams are 'not computable', NEVER caught-up (spec §4.3); not failing, but not proven" ;;
        *)      pass "projection.$PEER_NAME.streams" "$detail" ;;
      esac
    else
      fail "projection.$PEER_NAME.streams" "$detail after $retries attempt(s)"
    fi
  done

  finish projection
}

# ===========================================================================
# federation — doorway federation membership + p2p bootstrap surface
# ===========================================================================
cmd_federation() {
  echo "═══════════════════════════════════════════════════════════"
  echo "VERIFY FEDERATION LAYER — doorway membership + bootstrap surface"
  echo "═══════════════════════════════════════════════════════════"
  [ -n "${INTERNAL_DOORWAY_URL:-}" ] || { fail "federation.preflight" "INTERNAL_DOORWAY_URL unset"; finish federation; return; }
  local base="http://$INTERNAL_DOORWAY_URL" body

  if body="$(http_get "$base/api/v1/federation/doorways")"; then
    local n online
    n="$(jq -r '.doorways | length' <<<"$body" 2>/dev/null || echo 0)"
    online="$(jq -r '[.doorways[]? | select(.status=="online")] | length' <<<"$body" 2>/dev/null || echo 0)"
    if [ "${online:-0}" -ge 1 ]; then
      pass "federation.doorways" "$n doorway(s) known, $online online"
    else
      fail "federation.doorways" "$n doorway(s) known but none online"
    fi
    # Cross-doorway breadth needs shem (@requires:shem) — report, don't assert.
    [ "${n:-0}" -ge 2 ] && note "cross-doorway breadth present ($n) — shem-dependent, informational" \
                        || note "single-doorway federation (cross-doorway breadth needs shem)"
  else
    fail "federation.doorways" "GET /api/v1/federation/doorways → $(last_status)"
  fi

  if body="$(http_get "$base/api/v1/federation/p2p-peers")"; then
    if jq -e '.total >= 1' <<<"$body" >/dev/null 2>&1; then
      pass "federation.p2p-peers" "bootstrap surface lists $(jq -r '.total' <<<"$body") peer(s)"
    else
      fail "federation.p2p-peers" "bootstrap surface empty: $(jq -c '{total}' <<<"$body" 2>/dev/null)"
    fi
    if jq -e '[.peers[]? | select((.capabilities // []) | index("shard"))] | length >= 1' <<<"$body" >/dev/null 2>&1; then
      pass "federation.shard-capability" "at least one advertised peer carries the shard capability"
    else
      warn "federation.shard-capability" "no advertised peer carries shard capability"
    fi
  else
    fail "federation.p2p-peers" "GET /api/v1/federation/p2p-peers → $(last_status)"
  fi

  finish federation
}

# ===========================================================================
# resilience — conductor-signal-driven posture (gated upstream)
# ===========================================================================
cmd_resilience() {
  echo "═══════════════════════════════════════════════════════════"
  echo "VERIFY RESILIENCE SIGNALS — peer-status projections + network posture"
  echo "═══════════════════════════════════════════════════════════"
  # Stage-level `when` gates on CONDUCTOR_SEEDING_READY: these projections are
  # written ONLY by conductor-originated PeerStatusRecorded signals, so before
  # conductor seeding runs (netpol apply pending) they are empty BY DESIGN —
  # asserting then would false-fail (see backlog
  # ci-genesis-conductor-adminws-unreachable).
  local entry checked=false
  for entry in $(peers); do
    split_peer "$entry"
    local body
    if body="$(http_get "$PEER_URL/api/v1/peer-statuses")"; then
      checked=true
      local n
      n="$(jq -r 'length' <<<"$body" 2>/dev/null || echo 0)"
      if [ "${n:-0}" -ge 1 ]; then
        pass "resilience.$PEER_NAME.peer-statuses" "$n peer-status record(s) projected"
      else
        fail "resilience.$PEER_NAME.peer-statuses" "no peer-status records — PeerStatusRecorded signals not flowing despite conductor seeding"
      fi
      if body="$(http_get "$PEER_URL/api/v1/network/posture")"; then
        local active
        active="$(jq -r '.activePeers // 0' <<<"$body")"
        [ "${active:-0}" -ge 1 ] && pass "resilience.$PEER_NAME.posture" "activePeers=$active $(jq -c '{totalPeers,stalePeers,householdsReciprocating}' <<<"$body" 2>/dev/null)" \
                                 || fail "resilience.$PEER_NAME.posture" "activePeers=0: $(jq -c '.' <<<"$body" | head -c 200)"
      fi
      break # one healthy pod's projection answers for the fleet view
    fi
  done
  $checked || fail "resilience.reachable" "no peer pod answered /api/v1/peer-statuses"

  finish resilience
}

# ---------------------------------------------------------------------------
case "$CMD" in
  mesh)        cmd_mesh ;;
  upload)      cmd_upload ;;
  propagation) cmd_propagation ;;
  delivery)    cmd_delivery ;;
  projection)  cmd_projection ;;
  federation)  cmd_federation ;;
  resilience)  cmd_resilience ;;
  *) echo "usage: substrate-verify.sh {mesh|upload|propagation|delivery|projection|federation|resilience}"; exit 2 ;;
esac
