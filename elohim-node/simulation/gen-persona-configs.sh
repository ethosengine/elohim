#!/usr/bin/env bash
#
# Generate TOML configs for 20 elohim-node instances where each node IS a person.
#
# Unlike gen-configs.sh (infrastructure nodes), this generator reads personas.json
# to create configs where:
#   - node ID = human ID (human-matthew-manager, not node-1)
#   - cluster_name = trust cluster (matthews-household, not testnet)
#   - bootstrap_nodes connect cross-cluster bridges from the relationship graph
#   - compute budget limits are encoded per-node
#
# Usage:
#   ./gen-persona-configs.sh                          # Default: /tmp/elohim-persona-testnet
#   ./gen-persona-configs.sh --out /path/to/testnet   # Custom output dir
#
# Requires: jq

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PERSONAS_FILE="$SCRIPT_DIR/personas.json"

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Defaults
OUT_DIR="/tmp/elohim-persona-testnet"

# Parse flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT_DIR="$2"; shift 2 ;;
    *)     echo "Unknown flag: $1"; exit 1 ;;
  esac
done

if ! command -v jq &>/dev/null; then
  echo "ERROR: jq is required. Install with: apt install jq / brew install jq"
  exit 1
fi

if [[ ! -f "$PERSONAS_FILE" ]]; then
  echo "ERROR: personas.json not found at $PERSONAS_FILE"
  exit 1
fi

CONFIGS_DIR="$OUT_DIR/configs"
mkdir -p "$CONFIGS_DIR"

echo -e "${GREEN}Generating persona-based testnet configs${NC}"
echo "  Source: $PERSONAS_FILE"
echo "  Output: $CONFIGS_DIR"
echo ""

# Read compute budget
PER_NODE_MEM=$(jq -r '.computeBudget.perNodeMemoryLimit' "$PERSONAS_FILE")
PER_NODE_CPU_BUDGET=$(jq -r '.computeBudget.perNodeBudget' "$PERSONAS_FILE")
TRACKING_INTERVAL=$(jq -r '.computeBudget.trackingInterval' "$PERSONAS_FILE")

# Read cluster count and iterate
CLUSTER_COUNT=$(jq '.clusters | length' "$PERSONAS_FILE")

for c in $(seq 0 $((CLUSTER_COUNT - 1))); do
  CLUSTER_NAME=$(jq -r ".clusters[$c].name" "$PERSONAS_FILE")
  CLUSTER_TRUST=$(jq -r ".clusters[$c].trustFloor" "$PERSONAS_FILE")
  PERSONA_COUNT=$(jq ".clusters[$c].personas | length" "$PERSONAS_FILE")

  echo -e "${CYAN}Cluster: $CLUSTER_NAME${NC} (trust: $CLUSTER_TRUST, nodes: $PERSONA_COUNT)"

  # Find this cluster's primary node index for bootstrap
  PRIMARY_INDEX=$(jq -r ".clusters[$c].personas[] | select(.role == \"cluster-primary\") | .nodeIndex" "$PERSONAS_FILE")
  PRIMARY_TCP=$((4000 + PRIMARY_INDEX))

  for p in $(seq 0 $((PERSONA_COUNT - 1))); do
    NODE_INDEX=$(jq -r ".clusters[$c].personas[$p].nodeIndex" "$PERSONAS_FILE")
    HUMAN_ID=$(jq -r ".clusters[$c].personas[$p].humanId" "$PERSONAS_FILE")
    DISPLAY_NAME=$(jq -r ".clusters[$c].personas[$p].displayName" "$PERSONAS_FILE")
    CATEGORY=$(jq -r ".clusters[$c].personas[$p].category" "$PERSONAS_FILE")
    ROLE=$(jq -r ".clusters[$c].personas[$p].role" "$PERSONAS_FILE")
    DESIGN_INTENT=$(jq -r ".clusters[$c].personas[$p].designIntent" "$PERSONAS_FILE")

    # Port scheme
    TCP_PORT=$((4000 + NODE_INDEX))
    QUIC_PORT=$((4000 + NODE_INDEX))
    HTTP_PORT=$((8080 + NODE_INDEX))
    GRPC_PORT=$((9090 + NODE_INDEX))

    NODE_DIR="$OUT_DIR/$HUMAN_ID"
    CONFIG_FILE="$CONFIGS_DIR/$HUMAN_ID.toml"

    # Pod config: primaries are active, others dry-run
    if [[ "$ROLE" == "cluster-primary" ]]; then
      POD_DRY_RUN="false"
    else
      POD_DRY_RUN="true"
    fi

    # Bootstrap: non-primaries bootstrap to their cluster primary.
    # Primaries bootstrap to other cluster primaries (cross-cluster bridges).
    BOOTSTRAP_SECTION="bootstrap_nodes = []"
    if [[ "$ROLE" == "cluster-primary" ]]; then
      # Cross-cluster: point to all other cluster primaries
      ENTRIES=""
      for oc in $(seq 0 $((CLUSTER_COUNT - 1))); do
        [[ "$oc" -eq "$c" ]] && continue
        OTHER_PRIMARY=$(jq -r ".clusters[$oc].personas[] | select(.role == \"cluster-primary\") | .nodeIndex" "$PERSONAS_FILE")
        OTHER_PORT=$((4000 + OTHER_PRIMARY))
        if [[ -n "$ENTRIES" ]]; then ENTRIES="$ENTRIES, "; fi
        ENTRIES="$ENTRIES\"/ip4/127.0.0.1/tcp/$OTHER_PORT\""
      done
      BOOTSTRAP_SECTION="bootstrap_nodes = [$ENTRIES]"
    elif [[ "$NODE_INDEX" -ne "$PRIMARY_INDEX" ]]; then
      BOOTSTRAP_SECTION="bootstrap_nodes = [\"/ip4/127.0.0.1/tcp/$PRIMARY_TCP\"]"
    fi

    cat > "$CONFIG_FILE" <<EOF
# ──────────────────────────────────────────────────────────
# $DISPLAY_NAME ($HUMAN_ID)
# Cluster: $CLUSTER_NAME | Category: $CATEGORY | Role: $ROLE
# Intent: $DESIGN_INTENT
# ──────────────────────────────────────────────────────────

[node]
id = "$HUMAN_ID"
data_dir = "$NODE_DIR"
cluster_name = "$CLUSTER_NAME"

[node.persona]
display_name = "$DISPLAY_NAME"
category = "$CATEGORY"
human_id = "$HUMAN_ID"

[p2p]
listen_addrs = [
  "/ip4/127.0.0.1/tcp/$TCP_PORT",
  "/ip4/127.0.0.1/udp/$QUIC_PORT/quic-v1"
]
$BOOTSTRAP_SECTION

[cluster]
mdns_enabled = true
trust_floor = "$CLUSTER_TRUST"

[api]
http_port = $HTTP_PORT
grpc_port = $GRPC_PORT

[storage]
max_capacity = "1GB"
shard_redundancy = 2

[sync]
sync_interval_ms = 1000
max_document_size = 10485760

[pod]
enabled = true
decision_interval_secs = 10
dry_run = $POD_DRY_RUN
max_actions_per_hour = 20

[compute_budget]
cpu_seconds = $PER_NODE_CPU_BUDGET
memory_mb = $PER_NODE_MEM
tracking_interval = "$TRACKING_INTERVAL"
EOF

    printf "  %-20s  %-25s  tcp=:%d  http=:%d\n" "$DISPLAY_NAME" "$HUMAN_ID" "$TCP_PORT" "$HTTP_PORT"
  done
  echo ""
done

# Write cross-cluster bridge summary
BRIDGE_COUNT=$(jq '.crossClusterBridges | length' "$PERSONAS_FILE")
echo -e "${CYAN}Cross-cluster bridges: $BRIDGE_COUNT${NC}"
for b in $(seq 0 $((BRIDGE_COUNT - 1))); do
  FROM=$(jq -r ".crossClusterBridges[$b].from" "$PERSONAS_FILE")
  TO=$(jq -r ".crossClusterBridges[$b].to" "$PERSONAS_FILE")
  VIA=$(jq -r ".crossClusterBridges[$b].via" "$PERSONAS_FILE")
  echo "  $FROM → $TO ($VIA)"
done

echo ""
echo -e "${GREEN}Done.${NC} Configs in: $CONFIGS_DIR"
echo "  Data dirs: $OUT_DIR/<human-id>/"
echo ""
echo "Next: ./spawn-persona-testnet.sh start --dir $OUT_DIR"
