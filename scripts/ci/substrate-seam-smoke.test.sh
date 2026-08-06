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

curl() {
  local arg url="" http1=0
  for arg in "$@"; do
    url="$arg"
    [ "$arg" = "--http1.1" ] && http1=1
  done

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
      printf '{"headActionHash":"uhCkk-test-head"}\n'
      ;;
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

assert_fails_with redirect '/generate_204=301'
assert_fails_with bad-ws-status 'WS=400'
assert_fails_with missing-protocol 'protocol=missing'
assert_fails_with wrong-protocol 'protocol=not-iroh'
assert_fails_with n0 'seam-smoke[n0-contamination]: FAIL'
assert_fails_with tx5 'seam-smoke[no-lingering-tx5]: FAIL'

echo "substrate-seam-smoke: relay-sovereignty regression tests passed"
