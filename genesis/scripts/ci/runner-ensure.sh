#!/usr/bin/env bash
# runner-ensure.sh — register-or-confirm the CI runner as a stewarded SERVICE AGENT,
# then mint its doorway bearer JWT.
#
# Sibling of doorway-seed-ensure.sh (same contract style, same zero-new-credential
# self-provisioning), but for a different actor with a different standing:
#
#   jenkins-ci (doorway-seed-ensure.sh)  — seed actor, registered through the HUMAN path,
#                                          promoted to Admin by the bootstrap key.
#   ci-runner-jenkins (this script)      — agent:ci-runner-jenkins-matthew, a stewarded
#                                          runner. Registered with agencyPhase "service-agent":
#                                          its cell is hosted on the sponsor's pool conductor
#                                          under a notarized hosted-cell promise, NO Human
#                                          record is authored, and it is NEVER Admin — the
#                                          bootstrap key only authorizes its creation
#                                          (doorway-service src/routes/service_agent.rs).
#
# The two never share an account: the seed actor seeds, the runner requests compute.
# Capacity (measure-stage grants) is NOT issued here — each provider peer grants the
# runner's agent key independently through its own compute-grant surface.
#
# SELF-PROVISIONING: the password is derived from the EXISTING doorway admin bootstrap
# key (= doorway API_KEY_ADMIN, the `doorway-admin-bootstrap-key` Jenkins credential)
# with its own domain salt `:ci-runner`, so only an ADMIN_KEY holder can derive it and
# no second secret is stored anywhere. Dev stakes on purpose; the production hardening
# (a Secret mount, short window, rotation, per-build tokens) is recorded in
# genesis/data/timeline/backlog/security-ci-substrate-authorization-grant-coherence.md.
# ROTATION CAVEAT (as for jenkins-ci): register never resets a password on conflict, so
# after rotating ADMIN_KEY close the runner account once and the next run re-provisions.
#
# SECRETS NEVER IN ARGV OR LOGS: request bodies are built by jq from environment
# variables (never --arg) and piped to curl on stdin (--data @-); the bearer header is
# piped too (-H @-). ADMIN_KEY, the derived password and the JWT are never echoed to
# stderr; only the JWT is printed, to stdout, on success.
#
# Called by runner-preflight.sh (genesis/Jenkinsfile runnerPreflight()).
#
# Env:
#   ADMIN_KEY                 admin bootstrap key (= doorway API_KEY_ADMIN). Empty/unset ⇒
#                             print nothing, exit 0 (dev/unauthenticated path).
#   DOORWAY_URL               base doorway URL (scheme + host).
#   RUNNER_IDENTIFIER         default ci-runner-jenkins@alpha.elohim.host
#   RUNNER_SERVICE_AGENT_ID   default agent:ci-runner-jenkins-matthew
#   CURL_TIMEOUT              per-request timeout seconds for login/read (default 15)
#   REGISTER_TIMEOUT          register timeout seconds — it provisions a cell and asks
#                             the pool peer for a promise (default 90)
#
# Contract:
#   - ADMIN_KEY empty/unset: empty stdout, notice on stderr, exit 0.
#   - Any failure with ADMIN_KEY present: stderr diagnostic naming the cause (unreachable vs
#     the doorway's machine code), empty stdout, exit 1.
#   - The identifier exists but is NOT a service agent (e.g. a human account squatting the
#     name), or the account is Admin, or it carries no hosting promise: exit 1 — a runner
#     with the wrong standing is refused, never used.
#   - On success: the JWT on stdout (no trailing prose), exit 0; one stderr line with the
#     sponsor promise's grant CID and days remaining.
set -uo pipefail

CURL_TIMEOUT="${CURL_TIMEOUT:-15}"
REGISTER_TIMEOUT="${REGISTER_TIMEOUT:-90}"
IDENTIFIER="${RUNNER_IDENTIFIER:-ci-runner-jenkins@alpha.elohim.host}"
SERVICE_AGENT_ID="${RUNNER_SERVICE_AGENT_ID:-agent:ci-runner-jenkins-matthew}"

if [ -z "${ADMIN_KEY:-}" ]; then
  echo "runner-ensure: ADMIN_KEY unset — emitting empty token (dev/unauthenticated path)" >&2
  exit 0
fi
if [ -z "${DOORWAY_URL:-}" ]; then
  echo "runner-ensure: DOORWAY_URL unset — cannot reach the doorway" >&2
  exit 1
fi
base="${DOORWAY_URL%/}"

# Derived password: sha256 over ADMIN_KEY + domain salt. printf is a shell builtin, so
# the key never appears in a process's argv.
PW="$(printf '%s' "${ADMIN_KEY}:ci-runner" | sha256sum | cut -c1-48)"

# post_json <timeout> <url> — body on stdin; prints "<payload>\n<status>".
post_json() {
  curl -sS -m "$1" -w '\n%{http_code}' -X POST "$2" \
    -H 'Content-Type: application/json' --data @- 2>/dev/null
}

# --- Ensure: idempotent register (201 created, 409 already provisioned) ---------------
reg_resp="$(RA_ID="$IDENTIFIER" RA_PW="$PW" RA_KEY="$ADMIN_KEY" RA_SA="$SERVICE_AGENT_ID" \
  jq -n '{
    identifier: env.RA_ID,
    identifierType: "email",
    password: env.RA_PW,
    displayName: "Jenkins CI runner (service agent)",
    agencyPhase: "service-agent",
    serviceAgentId: env.RA_SA,
    adminBootstrapKey: env.RA_KEY
  }' | post_json "$REGISTER_TIMEOUT" "$base/auth/register")" || {
  echo "runner-ensure: POST $base/auth/register failed (network/timeout — doorway unreachable)" >&2
  exit 1
}
reg_status="${reg_resp##*$'\n'}"
reg_payload="${reg_resp%$'\n'*}"
reg_code="$(printf '%s' "$reg_payload" | jq -r '.code // empty' 2>/dev/null)"

case "$reg_status" in
  200|201)
    echo "runner-ensure: registered $SERVICE_AGENT_ID as $IDENTIFIER (service agent, sponsor promise notarized)" >&2
    ;;
  409)
    echo "runner-ensure: $IDENTIFIER already exists (409 ${reg_code:-USER_EXISTS}) — confirming its standing via login" >&2
    ;;
  *)
    echo "runner-ensure: register HTTP $reg_status ${reg_code:+($reg_code)} for $IDENTIFIER — runner not provisioned" >&2
    case "$reg_code" in
      ADMIN_KEY_REJECTED|ADMIN_KEY_UNCONFIGURED|OPERATOR_KEY_REQUIRED)
        echo "runner-ensure: cure — ADMIN_KEY must equal this doorway's API_KEY_ADMIN" >&2 ;;
      SERVICE_AGENT_NO_SPONSOR)
        echo "runner-ensure: cure — set POOL_COMPUTE_URL/POOL_COMPUTE_TOKEN/POOL_COMPUTE_PERFORMER on the doorway (its pool peer must be able to promise hosting)" >&2 ;;
      SERVICE_AGENT_UNHOSTABLE)
        echo "runner-ensure: cure — this doorway operates no conductor pool; register against a doorway that does" >&2 ;;
      INVALID_AGENCY_PHASE)
        echo "runner-ensure: cure — this doorway predates service-agent registration; deploy a doorway that accepts agencyPhase service-agent" >&2 ;;
    esac
    echo "doorway said: $(printf '%s' "$reg_payload" | jq -r '.error // empty' 2>/dev/null | head -c 300)" >&2
    exit 1
    ;;
esac

# --- Login: mint the JWT ---------------------------------------------------------------
login_resp="$(RA_ID="$IDENTIFIER" RA_PW="$PW" jq -n '{identifier: env.RA_ID, password: env.RA_PW}' \
  | post_json "$CURL_TIMEOUT" "$base/auth/login")" || {
  echo "runner-ensure: POST $base/auth/login failed (network/timeout — doorway unreachable)" >&2
  exit 1
}
login_status="${login_resp##*$'\n'}"
login_payload="${login_resp%$'\n'*}"
if [ "$login_status" != "200" ]; then
  echo "runner-ensure: login HTTP $login_status as $IDENTIFIER — derived password rejected (ADMIN_KEY rotated? close the runner account once to re-provision)" >&2
  exit 1
fi
token="$(printf '%s' "$login_payload" | jq -r '.token // empty')"
if [ -z "$token" ]; then
  echo "runner-ensure: login succeeded but returned no token (unexpected response shape)" >&2
  exit 1
fi

# --- Confirm standing: a service agent, never Admin, with a hosting promise -------------
acct_resp="$(printf 'Authorization: Bearer %s\n' "$token" \
  | curl -sS -m "$CURL_TIMEOUT" -w '\n%{http_code}' -H @- "$base/auth/account" 2>/dev/null)" || {
  echo "runner-ensure: GET $base/auth/account failed (network/timeout)" >&2
  exit 1
}
acct_status="${acct_resp##*$'\n'}"
acct="${acct_resp%$'\n'*}"
if [ "$acct_status" != "200" ]; then
  echo "runner-ensure: GET /auth/account HTTP $acct_status — cannot confirm the runner's standing" >&2
  exit 1
fi
phase="$(printf '%s' "$acct" | jq -r '.agencyPhase // empty')"
perm="$(printf '%s' "$acct" | jq -r '.permissionLevel // empty')"
grant="$(printf '%s' "$acct" | jq -r '.hostedCellGrantCid // empty')"
until="$(printf '%s' "$acct" | jq -r '.hostedCellValidUntil // empty')"

if [ "$phase" != "service-agent" ]; then
  echo "runner-ensure: REFUSED — $IDENTIFIER is a '${phase:-unphased}' account, not a service agent (a human record squatting the runner's name?). Close it and re-run." >&2
  exit 1
fi
if [ "$perm" = "ADMIN" ]; then
  echo "runner-ensure: REFUSED — $IDENTIFIER holds ADMIN; a runner must never be this doorway's admin" >&2
  exit 1
fi
if [ -z "$grant" ]; then
  echo "runner-ensure: REFUSED — $IDENTIFIER carries no hosted-cell promise; an unsponsored runner is not used" >&2
  exit 1
fi

days="?"
if [ -n "$until" ]; then
  until_s="$(date -u -d "$until" +%s 2>/dev/null || echo "")"
  if [ -n "$until_s" ]; then
    days=$(( (until_s - $(date -u +%s)) / 86400 ))
    if [ "$days" -lt 0 ]; then
      echo "runner-ensure: REFUSED — the sponsor's promise $grant lapsed at $until; the pool peer must re-issue it" >&2
      exit 1
    fi
  fi
fi
echo "runner-ensure: ok — $SERVICE_AGENT_ID sponsored under $grant, ${days} days remaining (until ${until:-unknown})" >&2

printf '%s' "$token"
