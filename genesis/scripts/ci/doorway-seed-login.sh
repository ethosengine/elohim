#!/usr/bin/env bash
# doorway-seed-login.sh — mint a doorway bearer JWT for CI seeding.
#
# Stage A of the jenkins-seed-bearer-gate plan: the genesis pipeline's substrate
# upload stage (substrate-verify.sh upload → PUT /admin/seed/blob) is bearer-gated
# on a non-dev doorway. This helper logs the registered `jenkins-ci` account in
# (POST /auth/login) and prints ONLY the JWT to stdout, so the Jenkinsfile can
# capture it into SEED_DOORWAY_TOKEN and pass it to substrate-verify.sh via env —
# keeping secrets out of argv and the Jenkinsfile heredoc-free (64KB CPS limit).
#
# Why a JWT, not an X-API-Key: the bearer carries a login event + actor (identity +
# audit), which is the entire point of Stage A — see the plan's "Why this is the
# honest Stage A". The credential pair comes from the Jenkins credential
# `doorway-seed-jenkins-ci` (username/password), surfaced as env.
#
# Env (all required except CURL_TIMEOUT):
#   DOORWAY_URL               base doorway URL (scheme + host), e.g. https://doorway-alpha.elohim.host
#   SEED_DOORWAY_IDENTIFIER   jenkins-ci account identifier (e.g. jenkins-ci@alpha.elohim.host)
#   SEED_DOORWAY_PASSWORD     jenkins-ci account password
#   CURL_TIMEOUT              per-request timeout seconds (default 15)
#
# Contract:
#   - On success: prints the JWT (no trailing prose) to stdout, exit 0.
#   - On missing creds: prints NOTHING to stdout, a notice to stderr, exit 0 —
#     the caller treats an empty token as "dev/unauthenticated" so the dev-mode
#     doorway path stays unbroken. (Non-dev alpha then 401s loudly downstream,
#     which is the intended fail-closed signal that creds were not wired.)
#   - On login failure (creds present but rejected/unreachable): stderr diagnostic,
#     empty stdout, exit 1 — a hard CI failure (creds are wrong/expired).
set -uo pipefail

CURL_TIMEOUT="${CURL_TIMEOUT:-15}"

if [ -z "${SEED_DOORWAY_IDENTIFIER:-}" ] || [ -z "${SEED_DOORWAY_PASSWORD:-}" ]; then
  echo "doorway-seed-login: SEED_DOORWAY_IDENTIFIER/PASSWORD unset — emitting empty token (dev/unauthenticated seed path)" >&2
  exit 0
fi

if [ -z "${DOORWAY_URL:-}" ]; then
  echo "doorway-seed-login: DOORWAY_URL unset — cannot reach /auth/login" >&2
  exit 1
fi

base="${DOORWAY_URL%/}"
body="$(printf '{"identifier":%s,"password":%s}' \
  "$(jq -Rn --arg v "$SEED_DOORWAY_IDENTIFIER" '$v')" \
  "$(jq -Rn --arg v "$SEED_DOORWAY_PASSWORD" '$v')")"

resp="$(curl -sS -m "$CURL_TIMEOUT" -w '\n%{http_code}' \
  -X POST "$base/auth/login" \
  -H 'Content-Type: application/json' \
  --data "$body" 2>/dev/null)" || {
  echo "doorway-seed-login: POST $base/auth/login failed (network/timeout)" >&2
  exit 1
}

status="${resp##*$'\n'}"
payload="${resp%$'\n'*}"

if [ "$status" != "200" ]; then
  echo "doorway-seed-login: login HTTP $status as $SEED_DOORWAY_IDENTIFIER — check doorway-seed-jenkins-ci credential (bad/expired) and that the account is Admin" >&2
  echo "doorway said: $(printf '%s' "$payload" | head -c 300)" >&2
  exit 1
fi

token="$(printf '%s' "$payload" | jq -r '.token // empty')"
if [ -z "$token" ]; then
  echo "doorway-seed-login: login succeeded but /auth/login returned no token (unexpected response shape)" >&2
  exit 1
fi

printf '%s' "$token"
