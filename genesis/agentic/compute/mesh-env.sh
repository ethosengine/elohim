#!/usr/bin/env bash
# The provider-owned mesh's non-secret topology, shared by bootstrap and guest wrapper.
# Never source an operator env file here: this script also runs as UID 65534.
compute_image_root="${COMPUTE_IMAGE_ROOT:-/opt/compute}"
compute_state_root="${COMPUTE_STATE_ROOT:-/var/lib/compute}"
export MESH_DIR="$compute_state_root/mesh"
export MESH_RESET=0 MESH_PORTAL=0 MESH_DOORWAYS=1 MESH_DOORWAY_GAMMA=0
export MESH_PEERS=matthew,jessica,james MESH_TRANSPORT_BACKEND=dual
export MESH_STORAGE_BASE_PORT=18090
export MESH_CONDUCTOR_BASE_PORT=14444 MESH_P2P_BASE_PORT=19701 MESH_METRICS_BASE_PORT=19464
export DOORWAY_PORT=18888 DOORWAY_B_PORT=18889 DOORWAY_C_PORT=18890
export DOORWAY_A_HEALTH_PORT=18079 DOORWAY_B_HEALTH_PORT=18089 DOORWAY_C_HEALTH_PORT=18099
export MESH_RELAY_PORT=13340 MESH_RELAY=1 MESH_MEMBERSHIP=0
export MESH_CONDUCTOR_LAUNCH=direct MESH_FOREGROUND=0
export STORAGE_BIN="$compute_image_root/mesh-bin/elohim-storage"
export DOORWAY_BIN="$compute_image_root/mesh-bin/doorway"
export HOLOCHAIN_BIN="$compute_image_root/mesh-bin/holochain"
export MESH_FORK_BIN_DIRS="$compute_image_root/mesh-bin"
export MESH_RELAY_BIN="$compute_image_root/mesh-bin/iroh-relay"
export BEACON_BIN="$compute_image_root/mesh-bin/relay-addr-beacon"
export MESH_HAPP_PATH="$compute_image_root/elohim.happ"
# Doorways declare Mongo explicitly; retain an isolated archive even though this
# initial profile casts nobody. Never attach the host's default-port archive.
export MONGOD_BIN=/usr/local/bin/mongod MONGO_PORT=37017 MESH_DOORWAY_MAX_AGENTS=0
export MESH_API_KEY_ADMIN=provider-private-mesh-fixture
export MESH_COMPUTE_LOCAL_TOKEN=provider-private-mesh-fixture-local-only
export E2E_DOORWAY_ALPHA=http://localhost:18888
export E2E_DOORWAY_B=http://localhost:18889 E2E_DOORWAY_BETA=http://localhost:18889
export E2E_STORAGE_URL=http://localhost:18090 E2E_STORAGE_B=http://localhost:18091
export PATH="$compute_image_root/mesh-bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
unset MESH_DOORWAY_ENV_SET MESH_KEEP_DOORWAY_DB MESH_PEER_TRANSPORTS MESH_HOLOCHAIN_BIN_EXPLICIT
unset MESH_ALLOW_STALE_BINARY MESH_ALLOW_STALE_HAPP MESH_ALLOW_TOOLCHAIN_SKEW
unset E2E_DOORWAY_GAMMA E2E_SHEM_HOST
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  exec "$@"
fi
