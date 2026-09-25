#!/usr/bin/env bash
# measure.sh — D2: the developer verb that sends a measure-class a2o stage to a household
# compute provider instead of holding the dev berth (sprint plan R3/R4/R5,
# /projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md). Not the `just measure`
# recipe line itself — that lives in justfile, owned by a different lane; this script IS the
# verb it execs.
#
# Sub-verbs:
#   measure.sh grant  --on jessica|adam
#   measure.sh worker --on jessica|adam
#   measure.sh <feature-path> [--on jessica|adam] [--gap <id>]
#   measure.sh status
#   measure.sh poll
#   measure.sh fixture --on jessica|adam <feature-path>
#
# v1: one feature path only (stage spec D2) — a tag expression is refused, tags reserved.
# `--on adam` refuses everywhere except as a named target this script recognizes but cannot
# act on yet: adam is a k8s pod on shem, provisioned by the operator (see the sprint plan's
# Operator items table and genesis/agentic/compute/operations.md "The grant a stage spends").
#
# MEASURE_DRY_RUN=1 prints the resolved env and, for grant/worker/<feature-path>, the exact
# command(s) it would run, without running them.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
PROVIDERS_JSON="$HERE/providers.json"
WORKSPACE_MJS="$HERE/workspace.mjs"
WORKER_MJS="$HERE/worker.mjs"
BUILD_STAGE_MJS="$HERE/stage/build-stage-task.mjs"

usage() {
  cat >&2 <<'USAGE'
usage: measure.sh grant  --on jessica|adam
       measure.sh worker --on jessica|adam
       measure.sh <feature-path> [--on jessica|adam] [--gap <id>]
       measure.sh status
       measure.sh poll
       measure.sh fixture --on jessica|adam <feature-path>
USAGE
}

refuse() { # <exit-code> <message>...
  local code="$1"; shift
  printf 'measure: %s\n' "$*" >&2
  exit "$code"
}

# ---- providers.json reads — no hostname, URL-to-cluster, or queue name in a TASK; this is
# the requester-local roster (R11), never notarized. ------------------------------------------
provider_field() { # <name> <field> -> value on stdout, exit 1 if absent/null
  node -e '
    const fs = require("fs");
    const providers = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const row = providers[process.argv[2]];
    const value = row ? row[process.argv[3]] : undefined;
    if (value === undefined || value === null) process.exit(1);
    process.stdout.write(String(value));
  ' "$PROVIDERS_JSON" "$1" "$2"
}

refuse_adam_not_provisioned() { # <verb>
  refuse 2 "--on adam refuses: adam's $1 is an operator item — a k8s pod on shem provisioned by the cluster operator (adam-compute-worker.json 'enabled', agent key, worker image digest, Secret token; see the sprint plan's Operator items table and operations.md). Nothing here can act on adam's behalf."
}

require_known_provider() { # <name> <verb>
  local name="$1" verb="$2"
  case "$name" in
    jessica) return 0 ;;
    adam) refuse_adam_not_provisioned "$verb" ;;
    *) refuse 2 "unknown provider '$name' (providers.json names: jessica, adam)" ;;
  esac
}

# ---- household env — the same sourcing pattern `just test mesh` uses (justfile:52-58): the
# optional-binary probes inside hc-mesh.sh legitimately exit non-zero at load under `set -e`. --
set +e
# shellcheck source=/dev/null
source "$ROOT/app/elohim-app/scripts/hc-mesh.sh"
set -e
mesh_seed_env

export COMPUTE_API_URL="$STORAGE_URL"
export ELOHIM_COMPUTE_LOCAL_TOKEN="${MESH_COMPUTE_LOCAL_TOKEN:-}"
export COMPUTE_PERFORMER="$(peer_agent_key 0 matthew)"
export COMPUTE_INBOX_ROOT="${COMPUTE_INBOX_ROOT:-$ROOT/genesis/a2o/reports/compute}"

# ---- the pinned rakia executor — check the built pin first, never derive its digest from the
# stage spec's own (stale) path. -----------------------------------------------------------
PINNED_EXECUTOR="/projects/.cargo-target-pool/family/dev/elohim__rakia/debug/compute-executor"
PINNED_ARK="/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark"

resolve_executor() {
  if [ -n "${COMPUTE_EXECUTOR:-}" ] && [ -x "$COMPUTE_EXECUTOR" ]; then
    return 0 # an explicit override on the caller's own env wins
  fi
  if [ -x "$PINNED_EXECUTOR" ]; then
    export COMPUTE_EXECUTOR="$PINNED_EXECUTOR"
  elif command -v compute-executor >/dev/null 2>&1; then
    export COMPUTE_EXECUTOR="$(command -v compute-executor)"
  else
    refuse 2 "no compute-executor binary found (checked $PINNED_EXECUTOR, \$COMPUTE_EXECUTOR, then PATH) — commit rakia-executor and move the pin (genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md), or build it locally in the rakia submodule pool slot"
  fi
  export COMPUTE_RUNTIME_IMAGE="sha256:$(sha256sum "$COMPUTE_EXECUTOR" | cut -d' ' -f1)"
}

resolve_ark() {
  if [ -n "${COMPUTE_ARK:-}" ] && [ -x "$COMPUTE_ARK" ]; then
    return 0
  fi
  if [ -x "$PINNED_ARK" ]; then
    export COMPUTE_ARK="$PINNED_ARK"
  elif command -v ark >/dev/null 2>&1; then
    export COMPUTE_ARK="$(command -v ark)"
  else
    refuse 2 "no ark binary found (checked $PINNED_ARK, \$COMPUTE_ARK, then PATH)"
  fi
}

dry_run() { [ "${MEASURE_DRY_RUN:-0}" = "1" ]; }

print_env_block() {
  cat <<ENV
COMPUTE_API_URL=$COMPUTE_API_URL
ELOHIM_COMPUTE_LOCAL_TOKEN=${ELOHIM_COMPUTE_LOCAL_TOKEN:+<redacted>}
COMPUTE_PERFORMER=$COMPUTE_PERFORMER
COMPUTE_EXECUTOR=${COMPUTE_EXECUTOR:-<unresolved>}
COMPUTE_RUNTIME_IMAGE=${COMPUTE_RUNTIME_IMAGE:-<unresolved>}
COMPUTE_INBOX_ROOT=$COMPUTE_INBOX_ROOT
MESH_DIR=$MESH_DIR
ENV
}

# ---- grant --------------------------------------------------------------------------------
cmd_grant() {
  local on=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --on) on="$2"; shift 2 ;;
      *) refuse 2 "grant: unknown argument $1" ;;
    esac
  done
  [ -n "$on" ] || refuse 2 "usage: measure.sh grant --on jessica|adam"
  require_known_provider "$on" "grant issuance"

  local api_port index grant_dir grant_file now_iso until_iso
  api_port="$(provider_field jessica api)" || refuse 2 "providers.json has no jessica.api"
  index="$(provider_field jessica index)" || refuse 2 "providers.json has no jessica.index"
  grant_dir="$MESH_DIR/compute"
  grant_file="$grant_dir/grant-jessica.json"
  now_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  until_iso="$(date -u -d '+7 days' +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -v+7d +%Y-%m-%dT%H:%M:%SZ)"

  if dry_run; then
    print_env_block
    echo "[dry-run] mkdir -p $grant_dir"
    echo "[dry-run] write $grant_file: recipient=$COMPUTE_PERFORMER scope=measure-stage rate_per_hour=8 rotation_ttl_days=1 window=$now_iso..$until_iso"
    echo "[dry-run] COMPUTE_API_URL=http://127.0.0.1:$api_port COMPUTE_PERFORMER=\$(peer_agent_key $index jessica) node $WORKSPACE_MJS grant $grant_file"
    return 0
  fi

  mkdir -p "$grant_dir"
  cat > "$grant_file" <<JSON
{
  "recipient": "$COMPUTE_PERFORMER",
  "scope": "measure-stage",
  "validFrom": "$now_iso",
  "validUntil": "$until_iso",
  "bounds": {
    "epr_scope": ["*"],
    "reach_ceiling": "commons",
    "rate_per_hour": 8,
    "rotation_ttl_days": 1
  }
}
JSON

  local jessica_performer grant_output grant_action_hash
  jessica_performer="$(peer_agent_key "$index" jessica)"
  [ -n "$jessica_performer" ] || refuse 2 "could not resolve jessica's own agent key (peer_agent_key $index jessica) — is the household mesh up? just mesh start"
  grant_output="$(COMPUTE_API_URL="http://127.0.0.1:$api_port" COMPUTE_PERFORMER="$jessica_performer" ELOHIM_COMPUTE_LOCAL_TOKEN="$ELOHIM_COMPUTE_LOCAL_TOKEN" node "$WORKSPACE_MJS" grant "$grant_file")"
  echo "$grant_output"
  grant_action_hash="$(node -e 'const o=JSON.parse(process.argv[1]||"{}"); process.stdout.write(o.grantActionHash||"")' "$grant_output")"
  if [ -n "$grant_action_hash" ]; then
    printf '%s' "$grant_action_hash" > "$grant_dir/grant-jessica.action-hash"
  fi
}

# ---- worker ---------------------------------------------------------------------------------
cmd_worker() {
  local on=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --on) on="$2"; shift 2 ;;
      *) refuse 2 "worker: unknown argument $1" ;;
    esac
  done
  [ -n "$on" ] || refuse 2 "usage: measure.sh worker --on jessica|adam"
  require_known_provider "$on" "worker"
  resolve_executor
  resolve_ark

  local api_port index jessica_performer worker_root
  api_port="$(provider_field jessica api)"
  index="$(provider_field jessica index)"
  jessica_performer="$(peer_agent_key "$index" jessica)"
  worker_root="$MESH_DIR/compute/worker-jessica"

  if dry_run; then
    print_env_block
    echo "[dry-run] mkdir -p $worker_root"
    echo "[dry-run] COMPUTE_API_URL=http://127.0.0.1:$api_port COMPUTE_PERFORMER=$jessica_performer COMPUTE_RUNTIME_IMAGE=$COMPUTE_RUNTIME_IMAGE COMPUTE_ARK=$COMPUTE_ARK COMPUTE_WORKER_ROOT=$worker_root node $WORKER_MJS (detached)"
    return 0
  fi

  mkdir -p "$worker_root"
  local log="$worker_root/worker.log"
  (
    COMPUTE_API_URL="http://127.0.0.1:$api_port" \
    COMPUTE_PERFORMER="$jessica_performer" \
    COMPUTE_RUNTIME_IMAGE="$COMPUTE_RUNTIME_IMAGE" \
    COMPUTE_EXECUTOR="$COMPUTE_EXECUTOR" \
    COMPUTE_ARK="$COMPUTE_ARK" \
    COMPUTE_WORKER_ROOT="$worker_root" \
    ELOHIM_COMPUTE_LOCAL_TOKEN="$ELOHIM_COMPUTE_LOCAL_TOKEN" \
    nohup node "$WORKER_MJS" >"$log" 2>&1 &
    echo $! > "$worker_root/worker.pid"
    disown
  )
  echo "worker started for jessica: pid $(cat "$worker_root/worker.pid"), log $log"
}

# ---- the chmod blast radius (operations.md "The capacity grant a provider must make") --------
check_writable_or_refuse() {
  local reports_dir="$ROOT/genesis/a2o/reports"
  echo "measure: the guest (UID 65534) needs write access to:"
  echo "  chmod o+w $reports_dir"
  local peer
  for peer in matthew jessica james; do
    echo "  chmod o+w $MESH_DIR/$peer/runtime-config.toml"
  done
  local missing=0
  [ -w "$reports_dir" ] || { echo "measure: not writable: $reports_dir" >&2; missing=1; }
  for peer in matthew jessica james; do
    local rc="$MESH_DIR/$peer/runtime-config.toml"
    [ -w "$rc" ] || { echo "measure: not writable: $rc" >&2; missing=1; }
  done
  [ "$missing" -eq 0 ] || refuse 2 "run the chmod lines above, then retry"
}

# ---- <feature-path> [--on jessica|adam] [--gap <id>] -----------------------------------------
cmd_run_feature() {
  local feature="$1"; shift
  local on="jessica" gap=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --on) on="$2"; shift 2 ;;
      --gap) gap="$2"; shift 2 ;;
      *) refuse 2 "unknown argument $1" ;;
    esac
  done
  require_known_provider "$on" "a measure run"

  # v1: one feature = one stage (stage spec D2); a tag expression is refused, tags reserved.
  case "$feature" in
    @*|*' and '*|*' or '*|*' not '*)
      refuse 2 "one stage = one feature (stage spec D2); tags reserved — got '$feature'"
      ;;
  esac
  [ -n "$feature" ] || refuse 2 "usage: measure.sh <feature-path> [--on jessica|adam] [--gap <id>]"

  resolve_executor

  local grant_dir grant_file grant_hash_file
  grant_dir="$MESH_DIR/compute"
  grant_file="$grant_dir/grant-jessica.json"
  grant_hash_file="$grant_dir/grant-jessica.action-hash"
  [ -f "$grant_file" ] || refuse 2 "no grant on record for jessica — run: measure.sh grant --on jessica"
  [ -s "$grant_hash_file" ] || refuse 2 "no grant action hash on record for jessica — run: measure.sh grant --on jessica"
  export COMPUTE_GRANT_ACTION="$(cat "$grant_hash_file")"

  local worker_pid_file="$MESH_DIR/compute/worker-jessica/worker.pid"
  if [ ! -s "$worker_pid_file" ] || ! kill -0 "$(cat "$worker_pid_file")" 2>/dev/null; then
    refuse 2 "no live worker for jessica — run: measure.sh worker --on jessica"
  fi

  local index jessica_performer feature_file feature_slug date_str out_dir
  index="$(provider_field jessica index)"
  jessica_performer="$(peer_agent_key "$index" jessica)"
  feature_file="$(basename "$feature")"
  feature_slug="${feature_file%.feature}"
  date_str="$(date -u +%Y-%m-%d)"
  out_dir="$ROOT/genesis/a2o/reports/peer-stage/$date_str/$feature_slug"

  local build_cmd=(node "$BUILD_STAGE_MJS" --feature "$feature" --requester "$COMPUTE_PERFORMER" --provider "$jessica_performer" --out "$out_dir" --executor "$COMPUTE_EXECUTOR" --repo-root "$ROOT")
  local submit_args=(submit "$out_dir/task.json" "$out_dir/stage-runner.sh" "$out_dir/$feature_file" --rung H)
  [ -n "$gap" ] && submit_args+=(--on "$gap")

  if dry_run; then
    print_env_block
    echo "[dry-run] ${build_cmd[*]}"
    echo "[dry-run] node $WORKSPACE_MJS ${submit_args[*]}"
    echo "[dry-run] node $WORKSPACE_MJS start"
    return 0
  fi

  check_writable_or_refuse

  "${build_cmd[@]}" >/dev/null

  local submit_output request_hash task_cid
  submit_output="$(node "$WORKSPACE_MJS" "${submit_args[@]}")"
  request_hash="$(node -e 'const o=JSON.parse(process.argv[1]); process.stdout.write(o.requestActionHash||"")' "$submit_output")"
  task_cid="$(node -e 'const o=JSON.parse(process.argv[1]); process.stdout.write(o.taskCid||"")' "$submit_output")"

  local lock="$COMPUTE_INBOX_ROOT/listener.lock"
  if [ ! -f "$lock" ] || ! kill -0 "$(cat "$lock" 2>/dev/null)" 2>/dev/null; then
    node "$WORKSPACE_MJS" start >/dev/null
  fi

  echo "requestActionHash: $request_hash"
  echo "taskCid: $task_cid"
  echo "inbox: $COMPUTE_INBOX_ROOT/inbox"
  echo "measure.sh status   # to check progress"
  echo "measure.sh poll     # to force a recovery pass"
}

# ---- status / poll ----------------------------------------------------------------------------
cmd_status() { node "$WORKSPACE_MJS" status; }
cmd_poll()   { node "$WORKSPACE_MJS" poll; }

# ---- fixture --on jessica|adam <feature-path> --------------------------------------------------
# Writes the PEER_STAGE_A2O_CONFIG JSON genesis/a2o/steps/compute/peer-executed-stage.steps.ts
# expects: { stageDir, requesterApi, providerApi, requesterKey, providerKey, env, timeoutSeconds }.
# Builds the stage (never submits it — that stays the live probe's own job).
cmd_fixture() {
  local on="" feature=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --on) on="$2"; shift 2 ;;
      *) feature="$1"; shift ;;
    esac
  done
  [ -n "$on" ] || refuse 2 "usage: measure.sh fixture --on jessica|adam <feature-path>"
  [ -n "$feature" ] || refuse 2 "usage: measure.sh fixture --on jessica|adam <feature-path>"
  require_known_provider "$on" "a fixture"
  resolve_executor

  local index jessica_performer out_dir config_path api_port
  index="$(provider_field jessica index)"
  api_port="$(provider_field jessica api)"
  jessica_performer="$(peer_agent_key "$index" jessica)"
  out_dir="$MESH_DIR/compute/fixture-stage"
  config_path="$MESH_DIR/compute/peer-stage-a2o-config.json"

  local build_cmd=(node "$BUILD_STAGE_MJS" --feature "$feature" --requester "$COMPUTE_PERFORMER" --provider "$jessica_performer" --out "$out_dir" --executor "$COMPUTE_EXECUTOR" --repo-root "$ROOT")

  if dry_run; then
    print_env_block
    echo "[dry-run] ${build_cmd[*]}"
    echo "[dry-run] write $config_path"
    return 0
  fi

  "${build_cmd[@]}" >/dev/null
  node -e '
    const fs = require("fs");
    const config = {
      stageDir: process.argv[1],
      requesterApi: process.argv[2],
      providerApi: process.argv[3],
      requesterKey: process.argv[4],
      providerKey: process.argv[5],
      env: {
        COMPUTE_API_URL: process.argv[2],
        COMPUTE_PERFORMER: process.argv[4],
        ELOHIM_COMPUTE_LOCAL_TOKEN: process.argv[6],
        COMPUTE_GRANT_ACTION: process.argv[7],
        COMPUTE_EXECUTOR: process.argv[8],
        COMPUTE_INBOX_ROOT: process.argv[9],
      },
      timeoutSeconds: 1800,
    };
    fs.writeFileSync(process.argv[10], JSON.stringify(config, null, 2) + "\n", { mode: 0o600 });
  ' "$out_dir" "$COMPUTE_API_URL" "http://127.0.0.1:$api_port" "$COMPUTE_PERFORMER" "$jessica_performer" \
    "$ELOHIM_COMPUTE_LOCAL_TOKEN" "$(cat "$MESH_DIR/compute/grant-jessica.action-hash" 2>/dev/null || echo "")" \
    "$COMPUTE_EXECUTOR" "$COMPUTE_INBOX_ROOT" "$config_path"
  echo "wrote $config_path"
}

# ---- dispatch -----------------------------------------------------------------------------
[ $# -ge 1 ] || { usage; exit 2; }
verb="$1"; shift
case "$verb" in
  grant)   cmd_grant "$@" ;;
  worker)  cmd_worker "$@" ;;
  status)  cmd_status "$@" ;;
  poll)    cmd_poll "$@" ;;
  fixture) cmd_fixture "$@" ;;
  -h|--help) usage; exit 0 ;;
  *)       cmd_run_feature "$verb" "$@" ;;
esac
