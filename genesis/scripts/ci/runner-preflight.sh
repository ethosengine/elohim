#!/usr/bin/env bash
# runner-preflight.sh — prove the CI runner's standing before a build sends any stage.
#
# One line per check, in the `just mesh preflight` idiom:
#   ok <name>: <what holds>
#   REFUSED <name>: <why> — <cure>
# Exit 1 if any line is REFUSED, 0 otherwise. Called (informational) by genesis/Jenkinsfile runnerPreflight().
#
# Checks:
#   standing           the runner is a service agent on this doorway, never Admin, under a
#                      live sponsor hosting promise (days remaining) — via runner-ensure.sh.
#   provider:<name>    for every provider in providers.json that declares `runnerGrant`
#                      (the action hash of its measure-stage grant TO the runner): the grant
#                      is active, names this runner as recipient, and how long it has left —
#                      read through the runner's own cell (compute-runner grant-status).
#
# Env (secrets only through the environment, never argv or stdout):
#   ADMIN_KEY, DOORWAY_URL      as runner-ensure.sh and compute-runner read them
#   COMPUTE_RUNNER              how to invoke compute-runner (default: tsx in genesis/a2o)
#   PROVIDERS_JSON              default genesis/agentic/compute/providers.json
#   RUNNER_ENSURE               default this directory's runner-ensure.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
PROVIDERS_JSON="${PROVIDERS_JSON:-$ROOT/genesis/agentic/compute/providers.json}"
RUNNER_ENSURE="${RUNNER_ENSURE:-$HERE/runner-ensure.sh}"
if [ -n "${COMPUTE_RUNNER:-}" ]; then
  read -r -a RUNNER_CMD <<< "$COMPUTE_RUNNER"
else
  RUNNER_CMD=("$ROOT/genesis/a2o/node_modules/.bin/tsx" "$ROOT/genesis/a2o/scripts/compute-runner.ts")
fi

refused=0
ok() { printf 'ok %s: %s\n' "$1" "$2"; }
refuse() { printf 'REFUSED %s: %s — %s\n' "$1" "$2" "$3"; refused=1; }

# ---- 1. standing: runner-ensure.sh registers-or-confirms and prints only the JWT ----------
ensure_log="$(mktemp)"
trap 'rm -f "$ensure_log"' EXIT
token="$(bash "$RUNNER_ENSURE" 2>"$ensure_log")"
ensure_exit=$?
if [ "$ensure_exit" -eq 0 ] && [ -n "$token" ]; then
  line="$(grep -E '^runner-ensure: ok' "$ensure_log" | tail -n 1)"
  ok standing "${line#runner-ensure: ok — }"
elif [ "$ensure_exit" -eq 0 ]; then
  refuse standing "ADMIN_KEY is unset, so the runner cannot sign in" \
    "export ADMIN_KEY from the doorway-admin-bootstrap-key credential"
else
  why="$(grep -E 'REFUSED|runner-ensure:' "$ensure_log" | tail -n 1)"
  cure="$(grep -E 'cure' "$ensure_log" | tail -n 1)"
  refuse standing "${why:-runner-ensure.sh failed (exit $ensure_exit)}" \
    "${cure:-follow the cause runner-ensure.sh names}"
fi

# ---- 2. one line per provider grant -------------------------------------------------------
if ! providers="$(jq -r 'to_entries[] | select(.value | has("runnerGrant")) | "\(.key) \(.value.runnerGrant // "")"' "$PROVIDERS_JSON" 2>/dev/null)"; then
  refuse providers "$PROVIDERS_JSON is unreadable" "restore genesis/agentic/compute/providers.json"
  providers=""
fi
if [ -z "$providers" ]; then
  refuse providers "no provider in $PROVIDERS_JSON declares a runnerGrant" \
    "add \"runnerGrant\": \"<grant action hash>\" to each provider that grants the runner capacity"
fi

while read -r name grant; do
  [ -n "$name" ] || continue
  if [ -z "$grant" ]; then
    refuse "provider:$name" "no runner grant recorded" \
      "measure.sh grant --on $name --recipient <runner agent key>, then record its action hash as runnerGrant in providers.json"
    continue
  fi
  if [ -z "$token" ]; then
    refuse "provider:$name" "the runner has no session, so the grant cannot be read" \
      "clear the standing refusal first"
    continue
  fi
  status="$(RUNNER_JWT="$token" "${RUNNER_CMD[@]}" grant-status "$grant" 2>&1)"
  if ! state="$(printf '%s' "$status" | jq -r '.state' 2>/dev/null)" || [ -z "$state" ] || [ "$state" = "null" ]; then
    refuse "provider:$name" "$(printf '%s' "$status" | tail -n 1)" "fix the cause compute-runner names"
    continue
  fi
  days="$(printf '%s' "$status" | jq -r '.daysRemaining // "?"')"
  mine="$(printf '%s' "$status" | jq -r '.forThisRunner')"
  case "$state" in
    active)
      if [ "$mine" = "true" ]; then
        ok "provider:$name" "measure-stage grant active, $days days remaining"
      else
        refuse "provider:$name" "the grant is active but names a different recipient" \
          "measure.sh grant --on $name --recipient <runner agent key>, then record the new action hash"
      fi
      ;;
    lapsed)
      refuse "provider:$name" "the grant lapsed" \
        "measure.sh grant --on $name --recipient <runner agent key> --renew, then record the new action hash"
      ;;
    withdrawn)
      refuse "provider:$name" "the provider withdrew the grant" \
        "ask $name for a new grant (a withdrawn grant is never reactivated)"
      ;;
    unknown)
      refuse "provider:$name" "no such grant is readable from the runner's cell" \
        "check the runnerGrant action hash, and that the provider's grant has gossiped"
      ;;
    *)
      refuse "provider:$name" "the grant is $state" \
        "ask $name to re-issue it with measure.sh grant --on $name --recipient <runner agent key>"
      ;;
  esac
done <<< "$providers"

exit "$refused"
