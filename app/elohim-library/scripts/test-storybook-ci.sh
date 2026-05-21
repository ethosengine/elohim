#!/usr/bin/env sh
# Serve the static Storybook bundle and run @storybook/test-runner against
# every indexed story. Shared by .husky/pre-push (elohim-storybook gate) and
# app/elohim-library/Jenkinsfile.
#
# Preconditions:
#   - cwd is app/elohim-library/
#   - dist/storybook/ exists (run build-storybook first)
#   - playwright-core's chromium browser is installed (run
#     `pnpm exec playwright install chromium` once; idempotent)
#
# Exit codes: 0 pass, 2 missing dist, 3 server did not come up, otherwise
# the test-runner's exit code is propagated.

set -e

PORT="${STORYBOOK_TEST_PORT:-6006}"
WAIT_BUDGET="${STORYBOOK_WAIT_SECONDS:-30}"

if [ ! -f dist/storybook/index.json ]; then
  echo "[test-storybook-ci] dist/storybook/index.json not found — run build-storybook first" >&2
  exit 2
fi

# Ensure playwright's bundled browser is on disk. No-op if already present.
pnpm exec playwright install chromium >/dev/null 2>&1 || true

SERVER_LOG=$(mktemp)
SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -f "$SERVER_LOG"
}
trap cleanup EXIT INT TERM

echo "[test-storybook-ci] serving dist/storybook on :$PORT"
pnpm exec http-server dist/storybook -p "$PORT" --silent >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

i=0
while [ "$i" -lt "$WAIT_BUDGET" ]; do
  if curl -fsS "http://localhost:$PORT/index.json" >/dev/null 2>&1; then
    break
  fi
  i=$((i + 1))
  sleep 1
done

if [ "$i" -ge "$WAIT_BUDGET" ]; then
  echo "[test-storybook-ci] server failed to come up on :$PORT within ${WAIT_BUDGET}s" >&2
  echo "--- http-server log ---" >&2
  cat "$SERVER_LOG" >&2
  exit 3
fi

echo "[test-storybook-ci] running @storybook/test-runner"
pnpm exec test-storybook \
  --url "http://localhost:$PORT" \
  --maxWorkers 2 \
  --testTimeout 30000
