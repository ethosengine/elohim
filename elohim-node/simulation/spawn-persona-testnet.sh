#!/usr/bin/env bash
#
# Persona Testnet Orchestrator
#
# Spawns 20 elohim-node processes where each node IS a person from genesis stories.
# Tracks compute budgets. Cleans up afterwards.
#
# Usage:
#   ./spawn-persona-testnet.sh start [--dir DIR] [--binary PATH]
#   ./spawn-persona-testnet.sh start-subset PERSONA1,PERSONA2,... [REQUESTER] [--dir DIR] [--binary PATH]
#   ./spawn-persona-testnet.sh stop  [--dir DIR]
#   ./spawn-persona-testnet.sh status [--dir DIR]
#   ./spawn-persona-testnet.sh verify [--dir DIR]     # Health + budget check
#   ./spawn-persona-testnet.sh settle [--dir DIR]     # Economic settlement
#   ./spawn-persona-testnet.sh clean [--dir DIR]      # Stop + archive + remove
#   ./spawn-persona-testnet.sh partition HUMAN_ID [--dir DIR]
#   ./spawn-persona-testnet.sh heal HUMAN_ID [--dir DIR]
#   ./spawn-persona-testnet.sh logs HUMAN_ID [--dir DIR]
#
# Examples:
#   ./spawn-persona-testnet.sh start
#   ./spawn-persona-testnet.sh status
#   ./spawn-persona-testnet.sh partition human-pete-pastor
#   ./spawn-persona-testnet.sh heal human-pete-pastor
#   ./spawn-persona-testnet.sh verify
#   ./spawn-persona-testnet.sh settle
#   ./spawn-persona-testnet.sh clean

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PERSONAS_FILE="$SCRIPT_DIR/personas.json"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Defaults
DEFAULT_DIR="/tmp/elohim-persona-testnet"
DEFAULT_BINARY=""
TESTNET_DIR=""

find_binary() {
  local candidates=(
    "$SCRIPT_DIR/../target/release/elohim-node"
    "$SCRIPT_DIR/../target/debug/elohim-node"
    "$(command -v elohim-node 2>/dev/null || true)"
  )
  for bin in "${candidates[@]}"; do
    if [[ -n "$bin" && -x "$bin" ]]; then
      echo "$bin"
      return
    fi
  done
  echo ""
}

parse_flags() {
  TESTNET_DIR="$DEFAULT_DIR"
  local binary="$DEFAULT_BINARY"
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dir)    TESTNET_DIR="$2"; shift 2 ;;
      --binary) DEFAULT_BINARY="$2"; shift 2 ;;
      *)        break ;;
    esac
  done
}

# Get all human IDs from personas.json
all_human_ids() {
  jq -r '.clusters[].personas[].humanId' "$PERSONAS_FILE"
}

# Get display name for a human ID
display_name_for() {
  jq -r ".clusters[].personas[] | select(.humanId == \"$1\") | .displayName" "$PERSONAS_FILE"
}

# Get node index for a human ID
node_index_for() {
  jq -r ".clusters[].personas[] | select(.humanId == \"$1\") | .nodeIndex" "$PERSONAS_FILE"
}

cmd_start() {
  parse_flags "$@"
  local binary="${DEFAULT_BINARY}"

  if [[ -z "$binary" ]]; then binary="$(find_binary)"; fi
  if [[ -z "$binary" || ! -x "$binary" ]]; then
    log_error "Cannot find elohim-node binary."
    echo "  Build it:  cd $SCRIPT_DIR/.. && RUSTFLAGS=\"\" cargo build --release"
    echo "  Or specify: $0 start --binary /path/to/elohim-node"
    exit 1
  fi
  log_info "Using binary: $binary"

  local pids_dir="$TESTNET_DIR/pids"
  local logs_dir="$TESTNET_DIR/logs"
  mkdir -p "$pids_dir" "$logs_dir"

  # Generate persona configs
  log_info "Generating persona configs..."
  bash "$SCRIPT_DIR/gen-persona-configs.sh" --out "$TESTNET_DIR"
  echo ""

  # Check if already running
  local already=0
  for pidfile in "$pids_dir"/*.pid; do
    [[ -f "$pidfile" ]] || continue
    if kill -0 "$(cat "$pidfile")" 2>/dev/null; then
      already=$((already + 1))
    fi
  done
  if [[ "$already" -gt 0 ]]; then
    log_warn "$already personas already running. Use 'stop' first."
    return 1
  fi

  log_info "Starting 20 persona nodes..."
  echo ""

  for human_id in $(all_human_ids); do
    local name node_index config_file node_dir log_file pidfile http_port
    name=$(display_name_for "$human_id")
    node_index=$(node_index_for "$human_id")
    config_file="$TESTNET_DIR/configs/$human_id.toml"
    node_dir="$TESTNET_DIR/$human_id"
    log_file="$logs_dir/$human_id.log"
    pidfile="$pids_dir/$human_id.pid"
    http_port=$((8080 + node_index))

    mkdir -p "$node_dir"

    ELOHIM_DATA_DIR="$node_dir" \
    RUST_LOG="${RUST_LOG:-info,elohim_node=debug}" \
      "$binary" --config "$config_file" \
      > "$log_file" 2>&1 &

    local pid=$!
    echo "$pid" > "$pidfile"

    printf "  ${GREEN}●${NC} %-20s  %-25s  pid=%-7d  http=:%d\n" \
      "$name" "$human_id" "$pid" "$http_port"
  done

  echo "$binary" > "$TESTNET_DIR/.binary_path"

  echo ""
  log_info "Waiting for mDNS peer discovery..."
  sleep 3

  echo ""
  log_info "Dashboards by cluster:"
  local cluster_count
  cluster_count=$(jq '.clusters | length' "$PERSONAS_FILE")
  for c in $(seq 0 $((cluster_count - 1))); do
    local cname
    cname=$(jq -r ".clusters[$c].name" "$PERSONAS_FILE")
    echo -e "  ${CYAN}$cname:${NC}"
    local pcount
    pcount=$(jq ".clusters[$c].personas | length" "$PERSONAS_FILE")
    for p in $(seq 0 $((pcount - 1))); do
      local dname idx port
      dname=$(jq -r ".clusters[$c].personas[$p].displayName" "$PERSONAS_FILE")
      idx=$(jq -r ".clusters[$c].personas[$p].nodeIndex" "$PERSONAS_FILE")
      port=$((8080 + idx))
      echo "    $dname: http://localhost:$port"
    done
  done

  echo ""
  log_info "Budget tracking:"
  echo "  Start:   $SCRIPT_DIR/compute-budget.sh watch --dir $TESTNET_DIR"
  echo "  Sample:  $SCRIPT_DIR/compute-budget.sh sample --dir $TESTNET_DIR"
  echo "  Settle:  $SCRIPT_DIR/compute-budget.sh settle --dir $TESTNET_DIR"

  echo ""
  log_info "Commands:"
  echo "  $0 status"
  echo "  $0 verify                          # Health + budget"
  echo "  $0 partition human-pete-pastor      # Kill a persona"
  echo "  $0 heal human-pete-pastor           # Restart"
  echo "  $0 logs human-matthew-manager       # Tail logs"
  echo "  $0 settle                           # Economic events"
  echo "  $0 clean                            # Archive + remove"
}

# Emit a CoordinationEnvelope to the envelopes ledger
emit_envelope() {
  local verb="$1"
  local action="$2"
  local persona="$3"
  local value="$4"
  local envelope_dir="$TESTNET_DIR/envelopes"
  local timestamp
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  mkdir -p "$envelope_dir"

  jq -cn \
    --arg verb "$verb" \
    --arg action "$action" \
    --arg persona "$persona" \
    --arg value "$value" \
    --arg ts "$timestamp" \
    --arg sender "${TESTNET_REQUESTER:-matthew}" \
    '{
      verb: $verb,
      scope: { agents: (if ($persona | length) > 0 then [$persona] else [] end) },
      routing: { urgency: "near-realtime", fallback: "queue" },
      payload: {
        economicEvent: {
          action: $action,
          provider: $persona,
          resourceQuantity: { value: ($value | tonumber), unit: "cpu-second" },
          settlement: (if $action == "budget-exceeded" then "partial" else "pending" end)
        }
      },
      sender: { agentId: $sender, delegationChain: [] },
      timestamp: $ts
    }' >> "$envelope_dir/envelopes.jsonl"
}

# Emit a provision envelope with ServiceRequest payload
emit_provision_envelope() {
  local personas_json="$1"
  local requester="$2"
  local count="$3"
  local envelope_dir="$TESTNET_DIR/envelopes"
  local timestamp
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  mkdir -p "$envelope_dir"

  jq -cn \
    --argjson agents "$personas_json" \
    --arg requester "$requester" \
    --arg count "$count" \
    --arg ts "$timestamp" \
    '{
      verb: "provision",
      scope: { agents: $agents },
      routing: { urgency: "near-realtime", fallback: "queue" },
      payload: {
        serviceRequest: {
          resourceQuantity: { value: (($count | tonumber) * 360), unit: "cpu-second" },
          duration: { value: 30, unit: "minute" },
          trustFloor: "Community"
        }
      },
      sender: { agentId: $requester, delegationChain: [] },
      timestamp: $ts
    }' >> "$envelope_dir/envelopes.jsonl"
}

cmd_start_subset() {
  local persona_list="$1"; shift || true
  local requester="${1:-matthew}"; shift || true
  parse_flags "$@"

  if [[ -z "$persona_list" ]]; then
    log_error "Usage: $0 start-subset <persona1,persona2,...> [requester]"
    exit 1
  fi

  IFS=',' read -ra REQUESTED_PERSONAS <<< "$persona_list"
  local count=${#REQUESTED_PERSONAS[@]}

  local binary="${DEFAULT_BINARY}"
  if [[ -z "$binary" ]]; then binary="$(find_binary)"; fi
  if [[ -z "$binary" || ! -x "$binary" ]]; then
    log_error "Cannot find elohim-node binary."
    echo "  Build it:  cd $SCRIPT_DIR/.. && RUSTFLAGS=\"\" cargo build --release"
    echo "  Or specify: $0 start-subset personas requester --binary /path/to/elohim-node"
    exit 1
  fi
  log_info "Using binary: $binary"

  local pids_dir="$TESTNET_DIR/pids"
  local logs_dir="$TESTNET_DIR/logs"
  mkdir -p "$pids_dir" "$logs_dir"

  echo ""
  log_info "Starting $count of 20 persona nodes (requester: $requester)"
  echo "  Personas: ${REQUESTED_PERSONAS[*]}"
  echo ""

  # Emit provision envelope
  local agents_json
  agents_json=$(printf '%s\n' "${REQUESTED_PERSONAS[@]}" | jq -R . | jq -s .)
  export TESTNET_REQUESTER="$requester"
  emit_provision_envelope "$agents_json" "$requester" "$count"

  # Generate configs for all personas (subset will only spawn some)
  log_info "Generating persona configs..."
  bash "$SCRIPT_DIR/gen-persona-configs.sh" --out "$TESTNET_DIR"

  local spawned=0
  for human_id in $(all_human_ids); do
    # Check if this persona is in the requested list
    local matched=false
    for req in "${REQUESTED_PERSONAS[@]}"; do
      if [[ "$human_id" == *"$req"* ]]; then
        matched=true
        break
      fi
    done
    [[ "$matched" == "true" ]] || continue

    local name node_index config_file node_dir log_file pidfile http_port
    name=$(display_name_for "$human_id")
    node_index=$(node_index_for "$human_id")
    config_file="$TESTNET_DIR/configs/$human_id.toml"
    node_dir="$TESTNET_DIR/$human_id"
    log_file="$logs_dir/$human_id.log"
    pidfile="$pids_dir/$human_id.pid"
    http_port=$((8080 + node_index))

    mkdir -p "$node_dir"

    ELOHIM_DATA_DIR="$node_dir" \
    RUST_LOG="${RUST_LOG:-info,elohim_node=debug}" \
      "$binary" --config "$config_file" \
      > "$log_file" 2>&1 &

    local pid=$!
    echo "$pid" > "$pidfile"
    spawned=$((spawned + 1))

    printf "  ${GREEN}●${NC} %-20s  %-25s  pid=%-7d  http=:%d\n" \
      "$name" "$human_id" "$pid" "$http_port"
  done

  echo "$binary" > "$TESTNET_DIR/.binary_path"

  echo ""
  log_info "Started $spawned of $count requested personas."
  if [[ "$spawned" -lt "$count" ]]; then
    log_warn "Some personas not matched. Check humanId patterns in personas.json."
  fi

  echo ""
  log_info "Budget tracking:"
  echo "  Start:   $SCRIPT_DIR/compute-budget.sh watch --dir $TESTNET_DIR"
  echo "  With circuit breaker: COMPUTE_KILL_ON_EXCEED=true COMPUTE_TTL_SECONDS=1800 $SCRIPT_DIR/compute-budget.sh watch --dir $TESTNET_DIR"
}

cmd_stop() {
  parse_flags "$@"
  local pids_dir="$TESTNET_DIR/pids"

  if [[ ! -d "$pids_dir" ]]; then
    log_warn "No persona testnet found at $TESTNET_DIR"
    return 0
  fi

  log_info "Emitting settle envelopes..."
  local ledger_file="$TESTNET_DIR/compute-ledger.jsonl"

  for pidfile in "$pids_dir"/*.pid; do
    [[ -f "$pidfile" ]] || continue
    local node_name
    node_name=$(basename "$pidfile" .pid)

    # Get final CPU from ledger (if it exists)
    local final_cpu="0"
    if [[ -f "$ledger_file" ]]; then
      final_cpu=$(grep "\"humanId\":\"$node_name\"" "$ledger_file" 2>/dev/null \
        | tail -1 | jq -r '.cpuSeconds // 0' 2>/dev/null || echo "0")
    fi
    [[ -z "$final_cpu" || "$final_cpu" == "null" ]] && final_cpu="0"

    # Don't emit for already-settled (budget-exceeded) nodes
    local status_file="${pidfile}.status"
    if [[ -f "$status_file" ]] && grep -q "killed" "$status_file" 2>/dev/null; then
      continue
    fi

    emit_envelope "settle" "deliver-service" "$node_name" "$final_cpu"
  done

  log_info "Stopping persona testnet..."
  local stopped=0

  for pidfile in "$pids_dir"/*.pid; do
    [[ -f "$pidfile" ]] || continue
    local pid node_name
    pid=$(cat "$pidfile")
    node_name=$(basename "$pidfile" .pid)

    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      local name
      name=$(display_name_for "$node_name" 2>/dev/null || echo "$node_name")
      echo -e "  ${YELLOW}○${NC} $name ($node_name, pid=$pid) stopped"
      stopped=$((stopped + 1))
    fi
    rm -f "$pidfile" "${pidfile}.status"
  done

  log_info "Stopped $stopped personas."
}

cmd_status() {
  parse_flags "$@"
  local pids_dir="$TESTNET_DIR/pids"

  if [[ ! -d "$pids_dir" ]]; then
    log_warn "No persona testnet found at $TESTNET_DIR"
    return 0
  fi

  echo ""
  echo "═══ Persona Testnet Status ═══"
  echo ""

  local running=0 total=0 healthy=0
  local cluster_count
  cluster_count=$(jq '.clusters | length' "$PERSONAS_FILE")

  for c in $(seq 0 $((cluster_count - 1))); do
    local cname
    cname=$(jq -r ".clusters[$c].name" "$PERSONAS_FILE")
    echo -e "  ${CYAN}$cname${NC}"

    local pcount
    pcount=$(jq ".clusters[$c].personas | length" "$PERSONAS_FILE")
    for p in $(seq 0 $((pcount - 1))); do
      local human_id dname idx port pidfile
      human_id=$(jq -r ".clusters[$c].personas[$p].humanId" "$PERSONAS_FILE")
      dname=$(jq -r ".clusters[$c].personas[$p].displayName" "$PERSONAS_FILE")
      idx=$(jq -r ".clusters[$c].personas[$p].nodeIndex" "$PERSONAS_FILE")
      port=$((8080 + idx))
      pidfile="$pids_dir/$human_id.pid"

      total=$((total + 1))

      if [[ ! -f "$pidfile" ]]; then
        printf "    ${RED}✗${NC} %-20s  no pidfile\n" "$dname"
        continue
      fi

      local pid
      pid=$(cat "$pidfile")

      if kill -0 "$pid" 2>/dev/null; then
        running=$((running + 1))
        if curl -sf "http://localhost:$port/health" > /dev/null 2>&1; then
          healthy=$((healthy + 1))
          printf "    ${GREEN}●${NC} %-20s  pid=%-7d  http=:%d  healthy\n" "$dname" "$pid" "$port"
        else
          printf "    ${YELLOW}○${NC} %-20s  pid=%-7d  http=:%d  starting\n" "$dname" "$pid" "$port"
        fi
      else
        printf "    ${RED}✗${NC} %-20s  pid=%-7d  dead\n" "$dname" "$pid"
      fi
    done
    echo ""
  done

  echo "  $running/$total running, $healthy healthy"
  echo ""
}

cmd_verify() {
  parse_flags "$@"
  log_info "Verifying persona testnet..."

  # Status check
  cmd_status "$@"

  # Budget check
  log_info "Checking compute budget..."
  bash "$SCRIPT_DIR/compute-budget.sh" sample --dir "$TESTNET_DIR"
  bash "$SCRIPT_DIR/compute-budget.sh" check --dir "$TESTNET_DIR"
}

cmd_settle() {
  parse_flags "$@"
  bash "$SCRIPT_DIR/compute-budget.sh" settle --dir "$TESTNET_DIR"
}

cmd_partition() {
  local human_id="$1"; shift
  parse_flags "$@"
  local pidfile="$TESTNET_DIR/pids/$human_id.pid"
  local name
  name=$(display_name_for "$human_id" 2>/dev/null || echo "$human_id")

  if [[ ! -f "$pidfile" ]]; then
    log_error "$name ($human_id) not found"
    return 1
  fi

  local pid
  pid=$(cat "$pidfile")
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid"
    log_info "$name ($human_id, pid=$pid) partitioned. Use 'heal $human_id' to restart."
  else
    log_warn "$name was already dead."
  fi
}

cmd_heal() {
  local human_id="$1"; shift
  parse_flags "$@"
  local config_file="$TESTNET_DIR/configs/$human_id.toml"
  local node_dir="$TESTNET_DIR/$human_id"
  local log_file="$TESTNET_DIR/logs/$human_id.log"
  local pidfile="$TESTNET_DIR/pids/$human_id.pid"
  local name
  name=$(display_name_for "$human_id" 2>/dev/null || echo "$human_id")

  if [[ ! -f "$config_file" ]]; then
    log_error "Config for $name ($human_id) not found"
    return 1
  fi

  if [[ -f "$pidfile" ]]; then
    local old_pid
    old_pid=$(cat "$pidfile")
    if kill -0 "$old_pid" 2>/dev/null; then
      log_warn "$name is already running (pid=$old_pid)"
      return 0
    fi
  fi

  local binary
  if [[ -f "$TESTNET_DIR/.binary_path" ]]; then
    binary=$(cat "$TESTNET_DIR/.binary_path")
  else
    binary="$(find_binary)"
  fi

  if [[ -z "$binary" || ! -x "$binary" ]]; then
    log_error "Cannot find elohim-node binary"
    return 1
  fi

  ELOHIM_DATA_DIR="$node_dir" \
  RUST_LOG="${RUST_LOG:-info,elohim_node=debug}" \
    "$binary" --config "$config_file" \
    >> "$log_file" 2>&1 &

  local pid=$!
  echo "$pid" > "$pidfile"

  local idx
  idx=$(node_index_for "$human_id")
  local port=$((8080 + idx))
  log_info "$name ($human_id) healed (pid=$pid, http=:$port)"
}

cmd_logs() {
  local human_id="$1"; shift
  parse_flags "$@"
  local log_file="$TESTNET_DIR/logs/$human_id.log"

  if [[ ! -f "$log_file" ]]; then
    log_error "Log file not found: $log_file"
    return 1
  fi

  local name
  name=$(display_name_for "$human_id" 2>/dev/null || echo "$human_id")
  echo -e "${CYAN}═══ Logs: $name ($human_id) ═══${NC}"
  tail -f "$log_file"
}

cmd_clean() {
  parse_flags "$@"

  # Stop if running
  cmd_stop "$@"

  # Archive ledger before deletion
  if [[ -f "$TESTNET_DIR/compute-ledger.jsonl" ]]; then
    local archive="$TESTNET_DIR/../persona-testnet-$(date +%Y%m%d-%H%M%S).tar.gz"
    log_info "Archiving compute ledger, economic events, and envelopes..."
    tar czf "$archive" \
      -C "$(dirname "$TESTNET_DIR")" \
      "$(basename "$TESTNET_DIR")/compute-ledger.jsonl" \
      "$(basename "$TESTNET_DIR")/economic-events.json" \
      "$(basename "$TESTNET_DIR")/envelopes/" \
      2>/dev/null || true
    log_info "Archive: $archive"
  fi

  log_warn "Removing all persona testnet data at $TESTNET_DIR..."
  rm -rf "$TESTNET_DIR"
  log_info "Clean."
}

cmd_help() {
  echo "Persona Testnet Orchestrator"
  echo ""
  echo "Spawns 20 elohim-node processes, one per genesis human."
  echo "Nodes are named by person (human-matthew-manager), not number."
  echo "Clusters match trust topology. Compute tracked per-persona."
  echo ""
  echo "Usage: $0 <command> [args] [--dir DIR] [--binary PATH]"
  echo ""
  echo "Commands:"
  echo "  start                     Start all 20 personas"
  echo "  start-subset P1,P2,...    Start a subset (e.g. matthew,susan,pete)"
  echo "  stop                      Stop all personas"
  echo "  status                    Health check by cluster"
  echo "  verify                    Health + compute budget check"
  echo "  settle                    Produce EconomicEvent records"
  echo "  partition HUMAN_ID        Kill a specific persona"
  echo "  heal HUMAN_ID             Restart a killed persona"
  echo "  logs HUMAN_ID             Tail a persona's logs"
  echo "  clean                     Stop, archive ledger, remove data"
  echo ""
  echo "Personas (20 humans, 4 clusters):"
  echo ""

  local cluster_count
  cluster_count=$(jq '.clusters | length' "$PERSONAS_FILE")
  for c in $(seq 0 $((cluster_count - 1))); do
    local cname
    cname=$(jq -r ".clusters[$c].name" "$PERSONAS_FILE")
    echo "  $cname:"
    local pcount
    pcount=$(jq ".clusters[$c].personas | length" "$PERSONAS_FILE")
    for p in $(seq 0 $((pcount - 1))); do
      local dname hid
      dname=$(jq -r ".clusters[$c].personas[$p].displayName" "$PERSONAS_FILE")
      hid=$(jq -r ".clusters[$c].personas[$p].humanId" "$PERSONAS_FILE")
      printf "    %-20s  %s\n" "$dname" "$hid"
    done
    echo ""
  done
}

# Dispatch
case "${1:-help}" in
  start)         shift; cmd_start "$@" ;;
  start-subset)  shift; cmd_start_subset "$@" ;;
  stop)          shift; cmd_stop "$@" ;;
  status)    shift; cmd_status "$@" ;;
  verify)    shift; cmd_verify "$@" ;;
  settle)    shift; cmd_settle "$@" ;;
  partition) shift; cmd_partition "$@" ;;
  heal)      shift; cmd_heal "$@" ;;
  logs)      shift; cmd_logs "$@" ;;
  clean)     shift; cmd_clean "$@" ;;
  help|*)    cmd_help ;;
esac
