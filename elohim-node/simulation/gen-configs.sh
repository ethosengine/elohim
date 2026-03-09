#!/usr/bin/env bash
#
# Generate TOML configs for N elohim-node instances on a single box.
#
# Usage:
#   ./gen-configs.sh 20                    # 20 nodes, flat mesh
#   ./gen-configs.sh 20 --families 4       # 20 nodes, 4 families of 5
#   ./gen-configs.sh 20 --out /tmp/testnet # Custom output dir
#
# Each node gets unique ports and data directory. All listen on 127.0.0.1
# so mDNS discovers peers on loopback without Docker or network namespaces.

set -euo pipefail

# Defaults
NODE_COUNT="${1:-5}"
FAMILIES=0
OUT_DIR=""

# Parse flags
shift || true
while [[ $# -gt 0 ]]; do
  case "$1" in
    --families) FAMILIES="$2"; shift 2 ;;
    --out)      OUT_DIR="$2"; shift 2 ;;
    *)          echo "Unknown flag: $1"; exit 1 ;;
  esac
done

if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="/tmp/elohim-testnet"
fi

CONFIGS_DIR="$OUT_DIR/configs"
mkdir -p "$CONFIGS_DIR"

echo "Generating $NODE_COUNT node configs in $CONFIGS_DIR"
if [[ "$FAMILIES" -gt 0 ]]; then
  NODES_PER_FAMILY=$(( NODE_COUNT / FAMILIES ))
  echo "  $FAMILIES families, $NODES_PER_FAMILY nodes each"
fi

for i in $(seq 1 "$NODE_COUNT"); do
  NODE_DIR="$OUT_DIR/node-$i"
  CONFIG_FILE="$CONFIGS_DIR/node-$i.toml"

  # Port scheme: base + node index
  TCP_PORT=$((4000 + i))
  QUIC_PORT=$((4000 + i))
  HTTP_PORT=$((8080 + i))
  GRPC_PORT=$((9090 + i))

  # Cluster assignment
  if [[ "$FAMILIES" -gt 0 ]]; then
    FAMILY_IDX=$(( ((i - 1) / NODES_PER_FAMILY) + 1 ))
    CLUSTER_NAME="family-$FAMILY_IDX"
    IS_PRIMARY=$(( ((i - 1) % NODES_PER_FAMILY) == 0 ))
  else
    CLUSTER_NAME="testnet"
    IS_PRIMARY=1
  fi

  # Pod config: primaries are active, secondaries are dry-run
  if [[ "$IS_PRIMARY" -eq 1 ]]; then
    POD_DRY_RUN="false"
  else
    POD_DRY_RUN="true"
  fi

  # Bootstrap nodes: for multi-family mode, primaries bootstrap to each other
  BOOTSTRAP_SECTION="bootstrap_nodes = []"
  if [[ "$FAMILIES" -gt 0 && "$IS_PRIMARY" -eq 1 ]]; then
    # Point to the other families' primary nodes
    BOOTSTRAP_ENTRIES=""
    for f in $(seq 1 "$FAMILIES"); do
      OTHER_PRIMARY=$(( (f - 1) * NODES_PER_FAMILY + 1 ))
      if [[ "$OTHER_PRIMARY" -ne "$i" ]]; then
        OTHER_PORT=$((4000 + OTHER_PRIMARY))
        # PeerId is unknown until first run; use address-only bootstrap
        # Kademlia will learn PeerId via identify protocol
        if [[ -n "$BOOTSTRAP_ENTRIES" ]]; then
          BOOTSTRAP_ENTRIES="$BOOTSTRAP_ENTRIES, "
        fi
        BOOTSTRAP_ENTRIES="$BOOTSTRAP_ENTRIES\"/ip4/127.0.0.1/tcp/$OTHER_PORT\""
      fi
    done
    BOOTSTRAP_SECTION="bootstrap_nodes = [$BOOTSTRAP_ENTRIES]"
  fi

  cat > "$CONFIG_FILE" <<EOF
# Auto-generated config for node-$i
# Cluster: $CLUSTER_NAME

[node]
id = "node-$i"
data_dir = "$NODE_DIR"
cluster_name = "$CLUSTER_NAME"

[p2p]
listen_addrs = [
  "/ip4/127.0.0.1/tcp/$TCP_PORT",
  "/ip4/127.0.0.1/udp/$QUIC_PORT/quic-v1"
]
$BOOTSTRAP_SECTION

[cluster]
mdns_enabled = true

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
EOF

  echo "  node-$i: cluster=$CLUSTER_NAME tcp=$TCP_PORT http=$HTTP_PORT pod_dry_run=$POD_DRY_RUN"
done

echo ""
echo "Done. Configs in: $CONFIGS_DIR"
echo "Data dirs will be created at: $OUT_DIR/node-*/"
echo ""
echo "Next: ./spawn-testnet.sh start $NODE_COUNT --dir $OUT_DIR"
