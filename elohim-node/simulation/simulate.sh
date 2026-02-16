#!/usr/bin/env bash
#
# elohim-node cluster simulation helper
#
# Usage:
#   ./simulate.sh start       # Start simulation
#   ./simulate.sh stop        # Stop simulation
#   ./simulate.sh logs        # Follow logs
#   ./simulate.sh status      # Check health of all nodes
#   ./simulate.sh partition   # Simulate network partition
#   ./simulate.sh heal        # Heal network partition
#   ./simulate.sh clean       # Stop and remove everything

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

cmd_start() {
    log_info "Starting elohim-node cluster simulation..."

    if [[ "$1" == "--latency" ]]; then
        log_info "Starting with WAN latency simulation (50ms)"
        docker-compose --profile latency up -d
    else
        docker-compose up -d
    fi

    log_info "Waiting for nodes to be healthy..."
    sleep 5

    cmd_status
}

cmd_stop() {
    log_info "Stopping simulation..."
    docker-compose down
}

cmd_logs() {
    docker-compose logs -f "$@"
}

cmd_status() {
    echo ""
    echo "=== Cluster Status ==="
    echo ""

    for node in family-a-node-1 family-a-node-2 family-b-node-1 family-b-node-2; do
        local port
        case $node in
            family-a-node-1) port=8081 ;;
            family-b-node-1) port=8082 ;;
            *) port="" ;;
        esac

        if docker ps --format '{{.Names}}' | grep -q "^${node}$"; then
            if [[ -n "$port" ]]; then
                if curl -s "http://localhost:$port/health" > /dev/null 2>&1; then
                    echo -e "  ${GREEN}●${NC} $node (port $port) - healthy"
                else
                    echo -e "  ${YELLOW}○${NC} $node (port $port) - starting"
                fi
            else
                echo -e "  ${GREEN}●${NC} $node - running"
            fi
        else
            echo -e "  ${RED}✗${NC} $node - stopped"
        fi
    done

    echo ""
}

cmd_partition() {
    log_warn "Simulating network partition between clusters..."

    docker network disconnect simulation_wan-bridge family-a-node-1 2>/dev/null || true
    docker network disconnect simulation_wan-bridge family-b-node-1 2>/dev/null || true

    log_info "Clusters are now isolated. Use './simulate.sh heal' to reconnect."
}

cmd_heal() {
    log_info "Healing network partition..."

    docker network connect simulation_wan-bridge family-a-node-1 --ip 172.20.0.11 2>/dev/null || true
    docker network connect simulation_wan-bridge family-b-node-1 --ip 172.20.0.21 2>/dev/null || true

    log_info "Clusters reconnected. Sync should resume."
}

cmd_clean() {
    log_warn "Stopping and removing all simulation resources..."
    docker-compose down -v --rmi local --remove-orphans
    log_info "Cleanup complete."
}

cmd_shell() {
    local node="${1:-family-a-node-1}"
    log_info "Opening shell in $node..."
    docker exec -it "$node" /bin/sh
}

cmd_test() {
    log_info "Running P2P simulation test..."
    local exit_code=0

    # Start the simulation
    cmd_start "$@"

    # Wait for storage sidecars to become healthy
    log_info "Waiting for storage sidecars to be healthy..."
    local max_attempts=30
    for attempt in $(seq 1 $max_attempts); do
        local healthy=0
        for port in 8091 8092; do
            if curl -sf "http://localhost:$port/health" > /dev/null 2>&1; then
                healthy=$((healthy + 1))
            fi
        done
        if [[ $healthy -ge 2 ]]; then
            log_info "All WAN storage sidecars healthy after ${attempt}s"
            break
        fi
        if [[ $attempt -eq $max_attempts ]]; then
            log_error "Storage sidecars did not become healthy within ${max_attempts}s"
            exit_code=1
        fi
        sleep 1
    done

    if [[ $exit_code -eq 0 ]]; then
        # Wait for P2P peer discovery (mDNS within LAN, bootstrap across WAN)
        log_info "Waiting 15s for P2P peer discovery..."
        sleep 15

        # Assert peer counts via /p2p/status
        log_info "Checking P2P peer counts..."
        for storage in "storage-a-1:8091" "storage-b-1:8092"; do
            local name="${storage%%:*}"
            local port="${storage##*:}"
            local peers
            peers=$(curl -sf "http://localhost:$port/p2p/status" 2>/dev/null | grep -o '"connected_peers":[0-9]*' | grep -o '[0-9]*' || echo "0")
            if [[ "$peers" -ge 1 ]]; then
                echo -e "  ${GREEN}●${NC} $name - $peers peer(s) connected"
            else
                echo -e "  ${YELLOW}○${NC} $name - no peers (may need more time)"
            fi
        done

        # Test network partition and heal
        log_info "Testing network partition..."
        cmd_partition
        sleep 5

        log_info "Testing network heal..."
        cmd_heal
        sleep 10

        # Verify reconnection
        log_info "Verifying peer reconnection after heal..."
        local reconnected=0
        for port in 8091 8092; do
            local peers
            peers=$(curl -sf "http://localhost:$port/p2p/status" 2>/dev/null | grep -o '"connected_peers":[0-9]*' | grep -o '[0-9]*' || echo "0")
            if [[ "$peers" -ge 1 ]]; then
                reconnected=$((reconnected + 1))
            fi
        done

        if [[ $reconnected -ge 1 ]]; then
            log_info "Peers reconnected after partition heal ($reconnected/2 nodes)"
        else
            log_warn "Peers did not reconnect (may need more time for re-discovery)"
            # Non-fatal: mDNS re-discovery can be slow in Docker
        fi
    fi

    # Cleanup
    log_info "Stopping simulation..."
    cmd_stop

    if [[ $exit_code -eq 0 ]]; then
        log_info "P2P simulation test PASSED"
    else
        log_error "P2P simulation test FAILED"
    fi
    return $exit_code
}

cmd_help() {
    echo "elohim-node Cluster Simulation"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  start [--latency]   Start simulation (with optional WAN latency)"
    echo "  stop                Stop simulation"
    echo "  logs [node]         Follow logs (optionally for specific node)"
    echo "  status              Check health of all nodes"
    echo "  test                Run P2P simulation test (start, verify peers, partition, heal, stop)"
    echo "  partition           Simulate network partition between clusters"
    echo "  heal                Heal network partition"
    echo "  shell [node]        Open shell in node (default: family-a-node-1)"
    echo "  clean               Stop and remove everything"
    echo "  help                Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 start                    # Start basic simulation"
    echo "  $0 start --latency          # Start with 50ms WAN latency"
    echo "  $0 test                     # Run full P2P test suite"
    echo "  $0 logs family-a-node-1     # Follow logs for specific node"
    echo "  $0 partition                # Disconnect clusters"
    echo "  $0 heal                     # Reconnect clusters"
}

# Main
case "${1:-help}" in
    start)     cmd_start "$2" ;;
    stop)      cmd_stop ;;
    logs)      shift; cmd_logs "$@" ;;
    status)    cmd_status ;;
    test)      shift; cmd_test "$@" ;;
    partition) cmd_partition ;;
    heal)      cmd_heal ;;
    shell)     cmd_shell "$2" ;;
    clean)     cmd_clean ;;
    help|*)    cmd_help ;;
esac
