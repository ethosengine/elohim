#!/usr/bin/env bash
#
# Compute Budget Tracker for Persona Testnet
#
# Tracks CPU and memory usage per-persona using the protocol's own vocabulary:
#   ServiceRequest  = "I need compute"     (the test run itself)
#   ComputeMetrics  = "What am I using?"   (sampled every N seconds)
#   EconomicEvent   = "What was consumed?" (final settlement per persona)
#
# Outputs a JSON ledger compatible with shefa pillar models.
#
# Usage:
#   ./compute-budget.sh sample [--dir DIR]              # One-shot sample
#   ./compute-budget.sh watch  [--dir DIR] [--interval 10]  # Continuous monitoring
#   ./compute-budget.sh settle [--dir DIR]              # Final report (economic events)
#   ./compute-budget.sh check  [--dir DIR]              # Budget compliance check
#
# Requires: jq, /proc filesystem (Linux)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PERSONAS_FILE="$SCRIPT_DIR/personas.json"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Defaults
TESTNET_DIR="/tmp/elohim-persona-testnet"
INTERVAL=10
LEDGER_FILE=""

# Parse flags after command
parse_flags() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dir)      TESTNET_DIR="$2"; shift 2 ;;
      --interval) INTERVAL="$2"; shift 2 ;;
      *)          echo "Unknown flag: $1"; exit 1 ;;
    esac
  done
  LEDGER_FILE="$TESTNET_DIR/compute-ledger.jsonl"
}

# Get CPU time (user + system) for a PID in seconds
get_cpu_seconds() {
  local pid="$1"
  if [[ -f "/proc/$pid/stat" ]]; then
    # Fields 14 (utime) and 15 (stime) in /proc/pid/stat, in clock ticks
    local ticks
    ticks=$(awk '{print $14 + $15}' "/proc/$pid/stat" 2>/dev/null || echo 0)
    local hz
    hz=$(getconf CLK_TCK 2>/dev/null || echo 100)
    echo "scale=2; $ticks / $hz" | bc 2>/dev/null || echo "0"
  else
    echo "0"
  fi
}

# Get RSS memory in MB for a PID
get_memory_mb() {
  local pid="$1"
  if [[ -f "/proc/$pid/status" ]]; then
    local rss_kb
    rss_kb=$(grep VmRSS "/proc/$pid/status" 2>/dev/null | awk '{print $2}' || echo 0)
    echo "scale=1; ${rss_kb:-0} / 1024" | bc 2>/dev/null || echo "0"
  else
    echo "0"
  fi
}

# Resolve PID for a human ID from the testnet pidfiles
get_pid_for_human() {
  local human_id="$1"
  local pidfile="$TESTNET_DIR/pids/$human_id.pid"
  # Fallback: try node-N.pid format
  if [[ ! -f "$pidfile" ]]; then
    local node_index
    node_index=$(jq -r ".clusters[].personas[] | select(.humanId == \"$human_id\") | .nodeIndex" "$PERSONAS_FILE" 2>/dev/null)
    pidfile="$TESTNET_DIR/pids/node-$node_index.pid"
  fi
  if [[ -f "$pidfile" ]]; then
    cat "$pidfile"
  else
    echo ""
  fi
}

# Sample all personas once — emit ComputeMetrics records
cmd_sample() {
  parse_flags "$@"
  local timestamp
  timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  local total_cpu=0
  local total_mem=0
  local over_budget=0

  local per_node_cpu_budget
  per_node_cpu_budget=$(jq -r '.computeBudget.perNodeBudget' "$PERSONAS_FILE")
  local per_node_mem_limit
  per_node_mem_limit=$(jq -r '.computeBudget.perNodeMemoryLimit' "$PERSONAS_FILE")

  echo ""
  printf "  %-20s  %10s  %10s  %s\n" "PERSONA" "CPU (s)" "MEM (MB)" "STATUS"
  printf "  %-20s  %10s  %10s  %s\n" "───────" "───────" "────────" "──────"

  local cluster_count
  cluster_count=$(jq '.clusters | length' "$PERSONAS_FILE")

  for c in $(seq 0 $((cluster_count - 1))); do
    local cluster_name
    cluster_name=$(jq -r ".clusters[$c].name" "$PERSONAS_FILE")
    local persona_count
    persona_count=$(jq ".clusters[$c].personas | length" "$PERSONAS_FILE")

    for p in $(seq 0 $((persona_count - 1))); do
      local human_id display_name
      human_id=$(jq -r ".clusters[$c].personas[$p].humanId" "$PERSONAS_FILE")
      display_name=$(jq -r ".clusters[$c].personas[$p].displayName" "$PERSONAS_FILE")

      local pid
      pid=$(get_pid_for_human "$human_id")

      if [[ -z "$pid" ]] || ! kill -0 "$pid" 2>/dev/null; then
        printf "  ${RED}%-20s  %10s  %10s  %s${NC}\n" "$display_name" "—" "—" "NOT RUNNING"
        continue
      fi

      local cpu_s mem_mb status_icon
      cpu_s=$(get_cpu_seconds "$pid")
      mem_mb=$(get_memory_mb "$pid")

      # Budget check
      local cpu_ok mem_ok
      cpu_ok=$(echo "$cpu_s <= $per_node_cpu_budget" | bc 2>/dev/null || echo 1)
      mem_ok=$(echo "$mem_mb <= $per_node_mem_limit" | bc 2>/dev/null || echo 1)

      if [[ "$cpu_ok" -eq 1 && "$mem_ok" -eq 1 ]]; then
        status_icon="${GREEN}within budget${NC}"
      else
        status_icon="${RED}OVER BUDGET${NC}"
        over_budget=$((over_budget + 1))
      fi

      printf "  %-20s  %10s  %10s  " "$display_name" "${cpu_s}s" "${mem_mb}M"
      echo -e "$status_icon"

      total_cpu=$(echo "$total_cpu + $cpu_s" | bc 2>/dev/null || echo "$total_cpu")
      total_mem=$(echo "$total_mem + $mem_mb" | bc 2>/dev/null || echo "$total_mem")

      # Emit ComputeMetrics record to ledger
      if [[ -n "$LEDGER_FILE" ]]; then
        echo "{\"type\":\"ComputeMetrics\",\"humanId\":\"$human_id\",\"displayName\":\"$display_name\",\"cluster\":\"$cluster_name\",\"pid\":$pid,\"cpuSeconds\":$cpu_s,\"memoryMb\":$mem_mb,\"timestamp\":\"$timestamp\"}" >> "$LEDGER_FILE"
      fi
    done
  done

  local total_budget
  total_budget=$(jq -r '.computeBudget.totalBudget' "$PERSONAS_FILE")
  local total_mem_budget
  total_mem_budget=$(jq -r '.computeBudget.totalMemoryLimit' "$PERSONAS_FILE")

  echo ""
  echo -e "  ${CYAN}Total:  CPU=${total_cpu}s / ${total_budget}s budget    MEM=${total_mem}M / ${total_mem_budget}M budget${NC}"
  if [[ "$over_budget" -gt 0 ]]; then
    echo -e "  ${RED}$over_budget persona(s) over budget${NC}"
  fi
  echo ""
}

# Watch continuously
cmd_watch() {
  parse_flags "$@"
  echo -e "${GREEN}Compute budget tracker — sampling every ${INTERVAL}s${NC}"
  echo "  Ledger: $LEDGER_FILE"
  echo "  Press Ctrl+C to stop and run 'settle' for final report."
  echo ""

  # Record ServiceRequest (the test run)
  local start_time
  start_time=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  echo "{\"type\":\"ServiceRequest\",\"action\":\"request-compute\",\"description\":\"Persona testnet run\",\"startTime\":\"$start_time\",\"interval\":$INTERVAL}" >> "$LEDGER_FILE"

  while true; do
    clear
    echo -e "${GREEN}═══ Persona Testnet — Compute Budget ═══${NC}"
    echo "  $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
    cmd_sample "$@"
    sleep "$INTERVAL"
  done
}

# Settle — produce EconomicEvent records (final accounting)
cmd_settle() {
  parse_flags "$@"

  if [[ ! -f "$LEDGER_FILE" ]]; then
    echo "No ledger found at $LEDGER_FILE. Run 'watch' or 'sample' first."
    exit 1
  fi

  local settle_time
  settle_time=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  echo ""
  echo -e "${GREEN}═══ Economic Settlement ═══${NC}"
  echo ""

  local events_file="$TESTNET_DIR/economic-events.json"
  echo "[" > "$events_file"
  local first=true

  local cluster_count
  cluster_count=$(jq '.clusters | length' "$PERSONAS_FILE")

  for c in $(seq 0 $((cluster_count - 1))); do
    local persona_count
    persona_count=$(jq ".clusters[$c].personas | length" "$PERSONAS_FILE")

    for p in $(seq 0 $((persona_count - 1))); do
      local human_id display_name
      human_id=$(jq -r ".clusters[$c].personas[$p].humanId" "$PERSONAS_FILE")
      display_name=$(jq -r ".clusters[$c].personas[$p].displayName" "$PERSONAS_FILE")

      # Get last metrics for this human from ledger
      local last_cpu last_mem
      last_cpu=$(grep "\"humanId\":\"$human_id\"" "$LEDGER_FILE" 2>/dev/null | tail -1 | jq -r '.cpuSeconds // 0' 2>/dev/null || echo 0)
      last_mem=$(grep "\"humanId\":\"$human_id\"" "$LEDGER_FILE" 2>/dev/null | tail -1 | jq -r '.memoryMb // 0' 2>/dev/null || echo 0)

      if [[ "$first" == "true" ]]; then first=false; else echo "," >> "$events_file"; fi

      # EconomicEvent: deliver-service (compute consumed)
      cat >> "$events_file" <<EOFJ
  {
    "type": "EconomicEvent",
    "action": "deliver-service",
    "provider": "$human_id",
    "receiver": "testnet-orchestrator",
    "resourceType": "compute",
    "resourceQuantity": {
      "cpu": { "value": $last_cpu, "unit": "cpu-second" },
      "memory": { "value": $last_mem, "unit": "megabyte" }
    },
    "settledAt": "$settle_time",
    "displayName": "$display_name"
  }
EOFJ

      printf "  %-20s  CPU: %8ss  MEM: %8sM\n" "$display_name" "$last_cpu" "$last_mem"
    done
  done

  echo "]" >> "$events_file"

  echo ""
  echo -e "  ${CYAN}Settlement written to: $events_file${NC}"
  echo ""
}

# Budget compliance check (pass/fail for CI)
cmd_check() {
  parse_flags "$@"

  if [[ ! -f "$LEDGER_FILE" ]]; then
    echo "No ledger found. Run 'sample' first."
    exit 1
  fi

  local per_node_cpu
  per_node_cpu=$(jq -r '.computeBudget.perNodeBudget' "$PERSONAS_FILE")
  local per_node_mem
  per_node_mem=$(jq -r '.computeBudget.perNodeMemoryLimit' "$PERSONAS_FILE")
  local total_cpu
  total_cpu=$(jq -r '.computeBudget.totalBudget' "$PERSONAS_FILE")
  local total_mem
  total_mem=$(jq -r '.computeBudget.totalMemoryLimit' "$PERSONAS_FILE")

  local violations=0

  # Check each persona's last reading
  local cluster_count
  cluster_count=$(jq '.clusters | length' "$PERSONAS_FILE")

  for c in $(seq 0 $((cluster_count - 1))); do
    local persona_count
    persona_count=$(jq ".clusters[$c].personas | length" "$PERSONAS_FILE")

    for p in $(seq 0 $((persona_count - 1))); do
      local human_id
      human_id=$(jq -r ".clusters[$c].personas[$p].humanId" "$PERSONAS_FILE")
      local display_name
      display_name=$(jq -r ".clusters[$c].personas[$p].displayName" "$PERSONAS_FILE")

      local last_cpu last_mem
      last_cpu=$(grep "\"humanId\":\"$human_id\"" "$LEDGER_FILE" 2>/dev/null | tail -1 | jq -r '.cpuSeconds // 0' 2>/dev/null || echo 0)
      last_mem=$(grep "\"humanId\":\"$human_id\"" "$LEDGER_FILE" 2>/dev/null | tail -1 | jq -r '.memoryMb // 0' 2>/dev/null || echo 0)

      local cpu_ok mem_ok
      cpu_ok=$(echo "$last_cpu <= $per_node_cpu" | bc 2>/dev/null || echo 1)
      mem_ok=$(echo "$last_mem <= $per_node_mem" | bc 2>/dev/null || echo 1)

      if [[ "$cpu_ok" -eq 0 ]]; then
        echo -e "  ${RED}FAIL${NC} $display_name: CPU ${last_cpu}s exceeds budget ${per_node_cpu}s"
        violations=$((violations + 1))
      fi
      if [[ "$mem_ok" -eq 0 ]]; then
        echo -e "  ${RED}FAIL${NC} $display_name: MEM ${last_mem}M exceeds limit ${per_node_mem}M"
        violations=$((violations + 1))
      fi
    done
  done

  if [[ "$violations" -eq 0 ]]; then
    echo -e "${GREEN}PASS${NC}: All personas within compute budget."
    exit 0
  else
    echo -e "${RED}FAIL${NC}: $violations budget violation(s)."
    exit 1
  fi
}

# Help
cmd_help() {
  echo "Compute Budget Tracker for Persona Testnet"
  echo ""
  echo "Tracks per-persona resource usage using shefa vocabulary:"
  echo "  ServiceRequest  → the test run"
  echo "  ComputeMetrics  → per-sample readings"
  echo "  EconomicEvent   → final settlement"
  echo ""
  echo "Usage: $0 <command> [--dir DIR] [--interval N]"
  echo ""
  echo "Commands:"
  echo "  sample     One-shot snapshot of all personas"
  echo "  watch      Continuous monitoring (Ctrl+C to stop)"
  echo "  settle     Produce final EconomicEvent records"
  echo "  check      CI-friendly budget compliance check (exit 0/1)"
  echo ""
}

case "${1:-help}" in
  sample)  shift; cmd_sample "$@" ;;
  watch)   shift; cmd_watch "$@" ;;
  settle)  shift; cmd_settle "$@" ;;
  check)   shift; cmd_check "$@" ;;
  help|*)  cmd_help ;;
esac
