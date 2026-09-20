#!/usr/bin/env bash
# Focused regression coverage for the Wave-2 relay-sovereignty seam smokes.
# All network reads are an exported fake curl function; no live service is used.

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/substrate-seam-smoke.sh"
TEST_ROOT="$(mktemp -d)"

cleanup() {
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

fake_json_body() {
  local url="$1"
  case "$url" in
    */admin/bootstrap-coherence)
      printf '{"spaces":2,"agents":5}\n'
      ;;
    */db/p2p/conductor-diagnostics)
      case "${TEST_CASE:-healthy}" in
        n0)
          printf '%s\n' '{"agentCount":5,"agents":[{"url":"https://use1-1.relay.iroh.network.:443/a"},{"url":"https://relay.alpha.elohim.host.:443/b"},{"url":"https://relay.elohim.host.:443/c"},{"url":"https://relay.alpha.elohim.host.:443/d"},{"url":"https://relay.elohim.host.:443/e"}]}'
          ;;
        tx5)
          printf '%s\n' '{"agentCount":5,"agents":[{"url":"wss://relay.alpha.elohim.host.:443/a"},{"url":"https://relay.alpha.elohim.host.:443/b"},{"url":"https://relay.elohim.host.:443/c"},{"url":"https://relay.alpha.elohim.host.:443/d"},{"url":"https://relay.elohim.host.:443/e"}]}'
          ;;
        *)
          # Both sovereign hosts use iroh's canonical trailing-dot form.
          printf '%s\n' '{"agentCount":5,"agents":[{"url":"https://relay.alpha.elohim.host.:443/a"},{"url":"https://relay.elohim.host.:443/b"},{"url":"https://relay.alpha.elohim.host.:443/c"},{"url":"https://relay.elohim.host.:443/d"},{"url":"https://relay.alpha.elohim.host.:443/e"}]}'
          ;;
      esac
      ;;
    */db/content/elohim-host-landing/head)
      # Same notarized head on both doorways; the bytes under it differ only in
      # the same-head-different-bytes case (alpha fleet, 2026-09-19).
      if [ "${TEST_CASE:-healthy}" = "no-blob-served" ]; then
        printf '{"headActionHash":"uhCkk-test-head","blobHash":null,"updatedAt":"2026-09-14 23:13:44"}\n'
      elif [ "${TEST_CASE:-healthy}" = "same-head-different-bytes" ] && [ "${url#https://elohim.host/}" != "$url" ]; then
        printf '{"headActionHash":"uhCkk-test-head","blobHash":"sha256-bbbb","updatedAt":"2026-09-13 14:46:59"}\n'
      else
        printf '{"headActionHash":"uhCkk-test-head","blobHash":"sha256-aaaa","updatedAt":"2026-09-14 23:13:44"}\n'
      fi
      ;;
    *) return 1 ;;
  esac
}
export -f fake_json_body

# Fakes GET .../head-record — story-1.4a seam-6b (per-doorway torn-row check).
# `record` is base64 of arbitrary noise with a `sha256-...` run embedded, the
# same byte shape the real msgpack `Record` carries its `blob_cid` string in
# (containment target, not a real Holochain Record — the script only tests
# for the substring, never decodes structurally).
fake_head_record_response() { # url -> emits "<body>\n<http_code>"
  local url="$1" is_a=0
  [ "${url#https://doorway-alpha.elohim.host/}" != "$url" ] && is_a=1

  case "${TEST_CASE:-healthy}" in
    unreadable-head-record)
      printf '{"error":"Conductor error: Zome call failed: External API wire error: InternalError(\\"Conductor returned an error while using a ConductorApi: CellDisabled(CellId(...))\\")"}'
      printf '\n502'
      return 0
      ;;
    no-blob-served)
      # The script must never fetch head-record when the served blob is null
      # — proven by making this URL a hard failure if it is ever reached.
      echo "Unexpected fake curl URL: ${url} (head-record must not be fetched when no blob is served)" >&2
      return 99
      ;;
  esac

  local blob
  if [ "${TEST_CASE:-healthy}" = "torn-row" ] && [ "$is_a" -eq 1 ]; then
    # A serves sha256-aaaa but its OWN head record never named it — its
    # record instead names a different blob. This is the torn shape: two
    # doorways can agree with each other while one disagrees with itself.
    blob="sha256-deadbeef"
  elif [ "${TEST_CASE:-healthy}" = "same-head-different-bytes" ] && [ "$is_a" -eq 0 ]; then
    blob="sha256-bbbb"
  else
    blob="sha256-aaaa"
  fi

  local record_b64
  record_b64=$(SEAM_TEST_BLOB="$blob" python3 -c "
import base64, os
blob = os.environ['SEAM_TEST_BLOB'].encode()
payload = b'\x81\xa8blob_cid' + bytes([0xa0 + len(blob)]) + blob + b'trailing-msgpack-noise'
print(base64.b64encode(payload).decode())
")
  printf '{"headActionHash":"uhCkk-test-head","record":"%s"}' "$record_b64"
  printf '\n200'
}
export -f fake_head_record_response

curl() {
  local arg url="" http1=0 want_code=0
  for arg in "$@"; do
    url="$arg"
    [ "$arg" = "--http1.1" ] && http1=1
    # probe_json asks for the body followed by the status on its own line.
    [ "$arg" = '\n%{http_code}' ] && want_code=1
  done
  if [ "$want_code" -eq 1 ]; then
    url="${@: -1}"
    case "$url" in
      */head-record)
        fake_head_record_response "$url"
        return $?
        ;;
      *)
        fake_json_body "$url"
        printf '\n200'
        return 0
        ;;
    esac
  fi
  fake_json_body "$url" && return 0

  case "$url" in
    */ping)
      [ "$http1" -eq 1 ] || { printf '400'; return 0; }
      printf '200'
      ;;
    http://*/generate_204)
      [ "$http1" -eq 1 ] || { printf '400'; return 0; }
      if [ "${TEST_CASE:-healthy}" = "redirect" ]; then
        printf '301'
      else
        printf '204'
      fi
      ;;
    */relay)
      if [ "$http1" -ne 1 ] || [ "${TEST_CASE:-healthy}" = "bad-ws-status" ]; then
        printf 'HTTP/1.1 400 Bad Request\r\n\r\n'
      elif [ "${TEST_CASE:-healthy}" = "missing-protocol" ]; then
        printf 'HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\n\r\n'
      elif [ "${TEST_CASE:-healthy}" = "wrong-protocol" ]; then
        printf 'HTTP/1.1 101 Switching Protocols\r\nSec-WebSocket-Protocol: not-iroh\r\n\r\n'
      else
        printf 'HTTP/1.1 101 Switching Protocols\r\nSec-WebSocket-Protocol: iroh-relay-v1\r\n\r\n'
      fi
      ;;
    *)
      echo "Unexpected fake curl URL: ${url}" >&2
      return 99
      ;;
  esac
}
export -f curl

run_case() {
  local name="$1" output="${TEST_ROOT}/${name}.log"
  set +e
  TEST_CASE="$name" bash "$SCRIPT" \
    https://doorway-alpha.elohim.host https://elohim.host --gate \
    > "$output" 2>&1
  local status=$?
  set -e
  printf '%s\t%s\n' "$status" "$output"
}

assert_passes() {
  local name="$1" result status output
  result="$(run_case "$name")"
  status="${result%%$'\t'*}"
  output="${result#*$'\t'}"
  if [ "$status" -ne 0 ]; then
    echo "Expected ${name} to pass; got ${status}" >&2
    sed -n '1,200p' "$output" >&2
    exit 1
  fi
}

assert_fails_with() {
  local name="$1" expected="$2" result status output
  result="$(run_case "$name")"
  status="${result%%$'\t'*}"
  output="${result#*$'\t'}"
  if [ "$status" -eq 0 ]; then
    echo "Expected ${name} to fail" >&2
    sed -n '1,200p' "$output" >&2
    exit 1
  fi
  if ! grep -Fq "$expected" "$output"; then
    echo "${name} did not report '${expected}'" >&2
    sed -n '1,200p' "$output" >&2
    exit 1
  fi
}

assert_passes healthy
HEALTHY_OUTPUT="${TEST_ROOT}/healthy.log"
if [ "$(grep -Fc 'seam-smoke[relay-reachability]: OK' "$HEALTHY_OUTPUT")" -ne 2 ]; then
  echo "Healthy run did not verify both sovereign relays" >&2
  sed -n '1,200p' "$HEALTHY_OUTPUT" >&2
  exit 1
fi
grep -Fq 'seam-smoke[n0-contamination]: OK' "$HEALTHY_OUTPUT"
grep -Fq 'seam-smoke[no-lingering-tx5]: OK' "$HEALTHY_OUTPUT"
if grep -Fq 'seam-smoke[signal-bus]' "$HEALTHY_OUTPUT"; then
  echo "Retired signal-bus smoke still ran" >&2
  exit 1
fi

grep -Fq 'seam-smoke[dht-fetch]: OK — landing canonical head CONVERGED (uhCkk-test-head) serving sha256-aaaa' "$HEALTHY_OUTPUT"

# One notarized head, two different blobs under it: the head comparison alone
# read CONVERGED on edge/dev 1465 while the pair served different bytes.
# Advisory seam, so the run still passes — the verdict line is the assertion.
assert_passes same-head-different-bytes
SPLIT_OUTPUT="${TEST_ROOT}/same-head-different-bytes.log"
if grep -Fq 'seam-smoke[dht-fetch]: OK — landing canonical head CONVERGED' "$SPLIT_OUTPUT"; then
  echo "Same head with different bytes was reported CONVERGED" >&2
  sed -n '1,200p' "$SPLIT_OUTPUT" >&2
  exit 1
fi
grep -Fq 'seam-smoke[dht-fetch]: ADVISORY-SAME-HEAD-DIFFERENT-BYTES' "$SPLIT_OUTPUT"
grep -Fq 'A=sha256-aaaa (2026-09-14 23:13:44)' "$SPLIT_OUTPUT"
grep -Fq 'B=sha256-bbbb (2026-09-13 14:46:59)' "$SPLIT_OUTPUT"
# Each doorway is internally coherent here (A's own record names aaaa, B's
# names bbbb) — the cross-doorway split above must not read as a per-doorway
# tear.
grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=https://doorway-alpha.elohim.host head-record names served blob sha256-aaaa' "$SPLIT_OUTPUT"
grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=https://elohim.host head-record names served blob sha256-bbbb' "$SPLIT_OUTPUT"

assert_fails_with redirect '/generate_204=301'
assert_fails_with bad-ws-status 'WS=400'
assert_fails_with missing-protocol 'protocol=missing'
assert_fails_with wrong-protocol 'protocol=not-iroh'
assert_fails_with n0 'seam-smoke[n0-contamination]: FAIL'
assert_fails_with tx5 'seam-smoke[no-lingering-tx5]: FAIL'

# ── seam 6b: per-doorway head-record torn-row check ─────────────────────────

# agree → OK (both doorways of the healthy run are internally coherent).
grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=https://doorway-alpha.elohim.host head-record names served blob sha256-aaaa' "$HEALTHY_OUTPUT"
grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=https://elohim.host head-record names served blob sha256-aaaa' "$HEALTHY_OUTPUT"

# differ → ADVISORY-TORN-ROW, and it must stay advisory (exit 0, --gate given).
assert_passes torn-row
TORN_OUTPUT="${TEST_ROOT}/torn-row.log"
grep -Fq 'seam-smoke[dht-fetch]: ADVISORY-TORN-ROW doorway=https://doorway-alpha.elohim.host head=uhCkk-test-head served=sha256-aaaa record=sha256-deadbeef' "$TORN_OUTPUT"
grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=https://elohim.host head-record names served blob sha256-aaaa' "$TORN_OUTPUT"
if grep -Fq 'ADVISORY-TORN-ROW doorway=https://elohim.host' "$TORN_OUTPUT"; then
  echo "torn-row falsely flagged the coherent doorway B" >&2
  sed -n '1,200p' "$TORN_OUTPUT" >&2
  exit 1
fi

# no blob at all → OK, never advisory, and head-record must never be fetched
# (the fake would hard-fail the run if it were).
assert_passes no-blob-served
NO_BLOB_OUTPUT="${TEST_ROOT}/no-blob-served.log"
grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=https://doorway-alpha.elohim.host no blob served for elohim-host-landing (nothing to tear)' "$NO_BLOB_OUTPUT"
grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=https://elohim.host no blob served for elohim-host-landing (nothing to tear)' "$NO_BLOB_OUTPUT"
if grep -Fiq 'ADVISORY-TORN-ROW\|UNREADABLE-HEAD-RECORD' "$NO_BLOB_OUTPUT"; then
  echo "no-blob-served should never reach the head-record fetch" >&2
  sed -n '1,200p' "$NO_BLOB_OUTPUT" >&2
  exit 1
fi

# unreadable (CellDisabled) → UNREADABLE-HEAD-RECORD, never OK, never TORN.
assert_passes unreadable-head-record
UNREADABLE_OUTPUT="${TEST_ROOT}/unreadable-head-record.log"
grep -Fq 'seam-smoke[dht-fetch]: UNREADABLE-HEAD-RECORD doorway=https://doorway-alpha.elohim.host reason=cell-disabled' "$UNREADABLE_OUTPUT"
grep -Fq 'seam-smoke[dht-fetch]: UNREADABLE-HEAD-RECORD doorway=https://elohim.host reason=cell-disabled' "$UNREADABLE_OUTPUT"
if grep -Fq 'seam-smoke[dht-fetch]: OK — doorway=' "$UNREADABLE_OUTPUT"; then
  echo "unreadable-head-record falsely read as OK for a doorway" >&2
  sed -n '1,200p' "$UNREADABLE_OUTPUT" >&2
  exit 1
fi
if grep -Fq 'ADVISORY-TORN-ROW' "$UNREADABLE_OUTPUT"; then
  echo "unreadable-head-record falsely read as TORN" >&2
  sed -n '1,200p' "$UNREADABLE_OUTPUT" >&2
  exit 1
fi

echo "substrate-seam-smoke: relay-sovereignty regression tests passed"
