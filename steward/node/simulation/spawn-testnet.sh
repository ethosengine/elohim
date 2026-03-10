#!/usr/bin/env bash
#
# Spawn a lightweight P2P testnet: N elohim-node processes on one box.
#
# No Docker. No k8s. No WASM. Just processes with different ports and data dirs.
# Each node gets its own Ed25519 identity (auto-generated on first run).
# mDNS discovers all peers on loopback. Automerge CRDT sync works identically
# to a real multi-box deployment.
#
# Usage:
#   ./spawn-testnet.sh start [N] [--families F] [--dir DIR] [--binary PATH]
#   ./spawn-testnet.sh stop
#   ./spawn-testnet.sh status
#   ./spawn-testnet.sh partition NODE_NUM     # Kill a specific node
#   ./spawn-testnet.sh heal NODE_NUM          # Restart a killed node
#   ./spawn-testnet.sh logs NODE_NUM          # Tail a node's logs
#   ./spawn-testnet.sh clean                  # Stop + remove all data
#
# Examples:
#   ./spawn-testnet.sh start 5                # 5 nodes, flat mesh
#   ./spawn-testnet.sh start 20 --families 4  # 20 nodes, 4 families of 5
#   ./spawn-testnet.sh status                 # Health check all nodes
#   ./spawn-testnet.sh partition 7            # Kill node-7
#   ./spawn-testnet.sh heal 7                 # Restart node-7
#   ./spawn-testnet.sh stop                   # Stop everything
#
# Resource usage (approximate):
#   5 nodes:  ~500 MB RAM, <1 CPU core idle
#   20 nodes: ~2 GB RAM, <4 CPU cores idle

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
DEFAULT_DIR="/tmp/elohim-testnet"
DEFAULT_BINARY=""
PIDFILE_DIR=""

find_binary() {
  # Try to find elohim-node binary
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

cmd_start() {
  local node_count="${1:-5}"
  shift || true

  local families=0
  local testnet_dir="$DEFAULT_DIR"
  local binary="${DEFAULT_BINARY}"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --families) families="$2"; shift 2 ;;
      --dir)      testnet_dir="$2"; shift 2 ;;
      --binary)   binary="$2"; shift 2 ;;
      *)          log_error "Unknown flag: $1"; exit 1 ;;
    esac
  done

  # Find binary
  if [[ -z "$binary" ]]; then
    binary="$(find_binary)"
  fi
  if [[ -z "$binary" || ! -x "$binary" ]]; then
    log_error "Cannot find elohim-node binary."
    echo "  Build it:  cd $SCRIPT_DIR/.. && RUSTFLAGS=\"\" cargo build --release"
    echo "  Or specify: $0 start $node_count --binary /path/to/elohim-node"
    exit 1
  fi
  log_info "Using binary: $binary"

  PIDFILE_DIR="$testnet_dir/pids"
  local logs_dir="$testnet_dir/logs"
  mkdir -p "$PIDFILE_DIR" "$logs_dir"

  # Generate configs if they don't exist
  local configs_dir="$testnet_dir/configs"
  if [[ ! -d "$configs_dir" ]] || [[ $(ls "$configs_dir"/*.toml 2>/dev/null | wc -l) -ne "$node_count" ]]; then
    log_info "Generating configs for $node_count nodes..."
    local gen_args=("$node_count" "--out" "$testnet_dir")
    if [[ "$families" -gt 0 ]]; then
      gen_args+=("--families" "$families")
    fi
    bash "$SCRIPT_DIR/gen-configs.sh" "${gen_args[@]}"
  fi

  # Check if already running
  local already_running=0
  for pidfile in "$PIDFILE_DIR"/node-*.pid; do
    [[ -f "$pidfile" ]] || continue
    if kill -0 "$(cat "$pidfile")" 2>/dev/null; then
      already_running=$((already_running + 1))
    fi
  done
  if [[ "$already_running" -gt 0 ]]; then
    log_warn "$already_running nodes already running. Use 'stop' first or 'status' to check."
    return 1
  fi

  log_info "Starting $node_count elohim-node processes..."
  echo ""

  for i in $(seq 1 "$node_count"); do
    local node_dir="$testnet_dir/node-$i"
    local config_file="$configs_dir/node-$i.toml"
    local log_file="$logs_dir/node-$i.log"
    local pidfile="$PIDFILE_DIR/node-$i.pid"

    mkdir -p "$node_dir"

    # Start node in background
    ELOHIM_DATA_DIR="$node_dir" \
    RUST_LOG="${RUST_LOG:-info,elohim_node=debug}" \
      "$binary" --config "$config_file" \
      > "$log_file" 2>&1 &

    local pid=$!
    echo "$pid" > "$pidfile"

    local http_port=$((8080 + i))
    printf "  ${GREEN}●${NC} node-%-3d  pid=%-7d  http=localhost:%-5d  log=%s\n" \
      "$i" "$pid" "$http_port" "$log_file"
  done

  echo ""
  log_info "Testnet started. Waiting for mDNS peer discovery..."
  sleep 3

  echo ""
  log_info "Dashboards:"
  for i in $(seq 1 "$node_count"); do
    local http_port=$((8080 + i))
    echo "  node-$i: http://localhost:$http_port"
  done

  echo ""
  log_info "Commands:"
  echo "  $0 status              # Health check"
  echo "  $0 logs 1              # Tail node-1 logs"
  echo "  $0 partition 5         # Kill node-5"
  echo "  $0 heal 5              # Restart node-5"
  echo "  $0 stop                # Stop all"

  # Save metadata for other commands
  echo "$node_count" > "$testnet_dir/.node_count"
  echo "$binary" > "$testnet_dir/.binary_path"
}

cmd_stop() {
  local testnet_dir="${1:-$DEFAULT_DIR}"
  PIDFILE_DIR="$testnet_dir/pids"

  if [[ ! -d "$PIDFILE_DIR" ]]; then
    log_warn "No testnet found at $testnet_dir"
    return 0
  fi

  log_info "Stopping testnet..."
  local stopped=0

  for pidfile in "$PIDFILE_DIR"/node-*.pid; do
    [[ -f "$pidfile" ]] || continue
    local pid
    pid=$(cat "$pidfile")
    local node_name
    node_name=$(basename "$pidfile" .pid)

    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      echo -e "  ${YELLOW}○${NC} $node_name (pid=$pid) stopped"
      stopped=$((stopped + 1))
    fi
    rm -f "$pidfile"
  done

  if [[ "$stopped" -eq 0 ]]; then
    log_info "No nodes were running."
  else
    log_info "Stopped $stopped nodes."
  fi
}

cmd_status() {
  local testnet_dir="${1:-$DEFAULT_DIR}"
  PIDFILE_DIR="$testnet_dir/pids"

  if [[ ! -d "$PIDFILE_DIR" ]]; then
    log_warn "No testnet found at $testnet_dir"
    return 0
  fi

  echo ""
  echo "=== Testnet Status ==="
  echo ""

  local running=0
  local total=0
  local healthy=0

  for pidfile in $(ls "$PIDFILE_DIR"/node-*.pid 2>/dev/null | sort -V); do
    [[ -f "$pidfile" ]] || continue
    total=$((total + 1))

    local pid
    pid=$(cat "$pidfile")
    local node_name
    node_name=$(basename "$pidfile" .pid)
    local node_num="${node_name#node-}"
    local http_port=$((8080 + node_num))

    if kill -0 "$pid" 2>/dev/null; then
      running=$((running + 1))
      if curl -sf "http://localhost:$http_port/health" > /dev/null 2>&1; then
        healthy=$((healthy + 1))
        printf "  ${GREEN}●${NC} %-10s  pid=%-7d  http=:%d  healthy\n" \
          "$node_name" "$pid" "$http_port"
      else
        printf "  ${YELLOW}○${NC} %-10s  pid=%-7d  http=:%d  starting\n" \
          "$node_name" "$pid" "$http_port"
      fi
    else
      printf "  ${RED}✗${NC} %-10s  pid=%-7d  dead\n" "$node_name" "$pid"
    fi
  done

  echo ""
  echo "  $running/$total running, $healthy healthy"
  echo ""
}

cmd_partition() {
  local node_num="$1"
  local testnet_dir="${2:-$DEFAULT_DIR}"
  local pidfile="$testnet_dir/pids/node-$node_num.pid"

  if [[ ! -f "$pidfile" ]]; then
    log_error "node-$node_num not found"
    return 1
  fi

  local pid
  pid=$(cat "$pidfile")
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid"
    log_info "node-$node_num (pid=$pid) killed. Use 'heal $node_num' to restart."
  else
    log_warn "node-$node_num was already dead"
  fi
}

cmd_heal() {
  local node_num="$1"
  local testnet_dir="${2:-$DEFAULT_DIR}"
  local config_file="$testnet_dir/configs/node-$node_num.toml"
  local node_dir="$testnet_dir/node-$node_num"
  local log_file="$testnet_dir/logs/node-$node_num.log"
  local pidfile="$testnet_dir/pids/node-$node_num.pid"

  if [[ ! -f "$config_file" ]]; then
    log_error "Config for node-$node_num not found at $config_file"
    return 1
  fi

  # Check if already running
  if [[ -f "$pidfile" ]]; then
    local old_pid
    old_pid=$(cat "$pidfile")
    if kill -0 "$old_pid" 2>/dev/null; then
      log_warn "node-$node_num is already running (pid=$old_pid)"
      return 0
    fi
  fi

  # Find binary
  local binary
  if [[ -f "$testnet_dir/.binary_path" ]]; then
    binary=$(cat "$testnet_dir/.binary_path")
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

  local http_port=$((8080 + node_num))
  log_info "node-$node_num restarted (pid=$pid, http=:$http_port)"
}

cmd_logs() {
  local node_num="$1"
  local testnet_dir="${2:-$DEFAULT_DIR}"
  local log_file="$testnet_dir/logs/node-$node_num.log"

  if [[ ! -f "$log_file" ]]; then
    log_error "Log file not found: $log_file"
    return 1
  fi

  tail -f "$log_file"
}

cmd_clean() {
  local testnet_dir="${1:-$DEFAULT_DIR}"

  cmd_stop "$testnet_dir"

  log_warn "Removing all testnet data at $testnet_dir..."
  rm -rf "$testnet_dir"
  log_info "Clean."
}

cmd_help() {
  echo "elohim-node Lightweight P2P Testnet"
  echo ""
  echo "Spawns N elohim-node processes on one box. No Docker, no k8s."
  echo "Each node gets its own identity, ports, and data directory."
  echo "mDNS discovers peers on loopback. Automerge CRDT sync works"
  echo "identically to a real multi-box deployment."
  echo ""
  echo "Usage: $0 <command> [args]"
  echo ""
  echo "Commands:"
  echo "  start [N] [opts]    Start N nodes (default: 5)"
  echo "    --families F      Group into F family clusters"
  echo "    --dir DIR         Data directory (default: /tmp/elohim-testnet)"
  echo "    --binary PATH     Path to elohim-node binary"
  echo "  stop                Stop all nodes"
  echo "  status              Health check all nodes"
  echo "  partition NODE      Kill a specific node (simulate failure)"
  echo "  heal NODE           Restart a killed node"
  echo "  logs NODE           Tail a node's log output"
  echo "  clean               Stop and remove all data"
  echo ""
  echo "Resource usage:"
  echo "  5 nodes:  ~500 MB RAM"
  echo "  10 nodes: ~1 GB RAM"
  echo "  20 nodes: ~2 GB RAM"
  echo ""
  echo "Examples:"
  echo "  $0 start 20                     # 20-node flat mesh"
  echo "  $0 start 20 --families 4        # 4 families of 5 nodes"
  echo "  $0 partition 7                  # Kill node-7"
  echo "  $0 heal 7                       # Restart node-7"
  echo "  $0 logs 3                       # Watch node-3"
  echo "  $0 stop                         # Stop everything"
}

# Main dispatch
case "${1:-help}" in
  start)     shift; cmd_start "$@" ;;
  stop)      shift; cmd_stop "${1:-$DEFAULT_DIR}" ;;
  status)    shift; cmd_status "${1:-$DEFAULT_DIR}" ;;
  partition) cmd_partition "$2" "${3:-$DEFAULT_DIR}" ;;
  heal)      cmd_heal "$2" "${3:-$DEFAULT_DIR}" ;;
  logs)      cmd_logs "$2" "${3:-$DEFAULT_DIR}" ;;
  clean)     shift; cmd_clean "${1:-$DEFAULT_DIR}" ;;
  help|*)    cmd_help ;;
esac
