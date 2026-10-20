#!/usr/bin/env bash
# runner-fixture.sh — write the CI_RUNNER_A2O_CONFIG fixture the ci-runner-stewarded story
# reads (genesis/a2o/steps/compute/ci-runner-stewarded.steps.ts), from the RUNNING household.
#
#   runner-fixture.sh --second-provider <peer-name>:<index>
#
# The story's cast is four stewards, no one in two roles:
#   first steward  = matthew — sponsors through doorway A on his own conductor
#   second steward = james   — sponsors through doorway C on his own conductor
#   first provider = jessica — lends capacity from her own storage peer
#   second provider = a fourth peer (`just mesh join-peer <fresh-name>`), named here
#
# Refuses before writing, naming the cure, when a role cannot be played:
#   - doorway C hosts nobody (its ceiling is 0) or has no pool performer:
#       MESH_DOORWAY_ENV_SET=$'DOORWAY_MAX_AGENTS_PER_CONDUCTOR=2\nPOOL_COMPUTE_PERFORMER=<james>' \
#         just mesh doorway-restart c
#     (or start the household with MESH_DOORWAY_C_MAX_AGENTS=2)
#   - no second provider named, or its storage does not answer.
#
# Prints the fixture path on stdout. Everything in it is dev-stakes household material (the
# fixed household admin key and compute token), never a fleet secret.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"

refuse() { printf 'runner-fixture: REFUSED %s\n' "$*" >&2; exit 2; }

second=""
while [ $# -gt 0 ]; do
  case "$1" in
    --second-provider) second="${2:-}"; shift 2 ;;
    *) refuse "unknown argument $1 — usage: runner-fixture.sh --second-provider <peer-name>:<index>" ;;
  esac
done
[ -n "$second" ] || refuse "no second provider named — stage one with \`just mesh join-peer <fresh-name>\` and pass --second-provider <name>:<index>"
second_name="${second%%:*}"
second_index="${second##*:}"
[[ "$second_index" =~ ^[0-9]+$ ]] || refuse "--second-provider wants <name>:<index>, got '$second'"

set +e
# shellcheck source=/dev/null
source "$ROOT/app/elohim-app/scripts/hc-mesh.sh"
set -e
mesh_seed_env

# The live environment of the doorway listening on <port>, one KEY=VALUE per line.
doorway_env() { # <port>
  local pid
  pid="$(ss -ltnp 2>/dev/null | grep ":$1 " | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2)"
  [ -n "$pid" ] || return 1
  tr '\0' '\n' < "/proc/$pid/environ"
}
env_value() { sed -n "s/^$1=//p" | head -1; }

matthew="$(peer_agent_key 0 "${PEERS[0]}")"
jessica="$(peer_agent_key 1 "${PEERS[1]}")"
james="$(peer_agent_key 2 "${PEERS[2]}")"
fourth="$(peer_agent_key "$second_index" "$second_name")"
[ -n "$matthew" ] && [ -n "$jessica" ] && [ -n "$james" ] \
  || refuse "household agent keys unreadable — is the mesh up? just mesh start"
[ -n "$fourth" ] || refuse "no agent key for $second_name at index $second_index — stage it: just mesh join-peer $second_name"
curl -sf -m 5 "http://127.0.0.1:$(http_port "$second_index")/health" >/dev/null \
  || refuse "$second_name's storage does not answer on :$(http_port "$second_index")"

c_env="$(doorway_env "$DOORWAY_C_PORT")" || refuse "doorway C is not running on :$DOORWAY_C_PORT — MESH_DOORWAY_GAMMA=1 just mesh start"
c_ceiling="$(printf '%s\n' "$c_env" | env_value DOORWAY_MAX_AGENTS_PER_CONDUCTOR)"
c_performer="$(printf '%s\n' "$c_env" | env_value POOL_COMPUTE_PERFORMER)"
if [ "${c_ceiling:-0}" = "0" ] || [ "$c_performer" != "$james" ]; then
  refuse "doorway C cannot sponsor a runner for james (ceiling ${c_ceiling:-0}, performer ${c_performer:-<unset>}) — MESH_DOORWAY_ENV_SET=\$'DOORWAY_MAX_AGENTS_PER_CONDUCTOR=2\\nPOOL_COMPUTE_PERFORMER=$james' just mesh doorway-restart c"
fi

out="$MESH_DIR/compute/ci-runner-a2o-config.json"
mkdir -p "$MESH_DIR/compute"
node -e '
  const [out, admin, token, dA, pA, mA, dC, pC, mC, pJ, kJ, n4, p4, k4] = process.argv.slice(1);
  const config = {
    adminKey: admin,
    localToken: token,
    stageFeature: "features/dataplane/doorway-fixture-readiness.feature",
    sponsors: [
      { name: "matthew", doorway: dA, poolApi: pA, performer: mA },
      { name: "james", doorway: dC, poolApi: pC, performer: mC },
    ],
    providers: [
      { name: "jessica", api: pJ, key: kJ },
      { name: n4, api: p4, key: k4 },
    ],
    timeoutSeconds: 120,
  };
  require("fs").writeFileSync(out, JSON.stringify(config, null, 2) + "\n", { mode: 0o600 });
' "$out" "${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}" "$MESH_COMPUTE_LOCAL_TOKEN" \
  "http://localhost:$DOORWAY_PORT" "http://127.0.0.1:$(http_port 0)" "$matthew" \
  "http://localhost:$DOORWAY_C_PORT" "http://127.0.0.1:$(http_port 2)" "$james" \
  "http://127.0.0.1:$(http_port 1)" "$jessica" \
  "$second_name" "http://127.0.0.1:$(http_port "$second_index")" "$fourth"
echo "$out"
