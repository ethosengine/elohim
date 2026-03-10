# elohim-node Cluster Simulation

Simulate multi-family cluster topology with network conditions.

## Quick Start

```bash
# Start the simulation (2 families, 2 nodes each)
docker-compose up -d

# View logs
docker-compose logs -f

# Stop
docker-compose down

# Stop and clean volumes
docker-compose down -v
```

## Network Topology

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SIMULATION TOPOLOGY                                │
│                                                                              │
│   CLUSTER A (Johnson Family)          CLUSTER B (Smith Family)              │
│   cluster-a-lan: 172.20.1.0/24        cluster-b-lan: 172.20.2.0/24         │
│                                                                              │
│   ┌─────────────────────┐             ┌─────────────────────┐               │
│   │ family-a-node-1     │             │ family-b-node-1     │               │
│   │ 172.20.1.11         │             │ 172.20.2.11         │               │
│   │ Primary             │             │ Primary             │               │
│   │ HTTP: localhost:8081│             │ HTTP: localhost:8082│               │
│   │ gRPC: localhost:9091│             │ gRPC: localhost:9092│               │
│   │                     │             │                     │               │
│   │ family-a-node-2     │             │ family-b-node-2     │               │
│   │ 172.20.1.12         │             │ 172.20.2.12         │               │
│   │ Replica             │             │ Replica             │               │
│   └─────────┬───────────┘             └─────────┬───────────┘               │
│             │                                   │                            │
│             │         WAN Bridge                │                            │
│             │       172.20.0.0/24               │                            │
│             │                                   │                            │
│             │  family-a-node-1: 172.20.0.11    │                            │
│             │  family-b-node-1: 172.20.0.21    │                            │
│             │                                   │                            │
│             └───────────────────────────────────┘                            │
│                                                                              │
│   LAN: ~0ms latency (same cluster)                                          │
│   WAN: ~50ms latency (cross-cluster, with --profile latency)                │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Services

| Service | IP (LAN) | IP (WAN) | HTTP Port | gRPC Port |
|---------|----------|----------|-----------|-----------|
| family-a-node-1 | 172.20.1.11 | 172.20.0.11 | 8081 | 9091 |
| family-a-node-2 | 172.20.1.12 | - | - | - |
| family-b-node-1 | 172.20.2.11 | 172.20.0.21 | 8082 | 9092 |
| family-b-node-2 | 172.20.2.12 | - | - | - |

## Testing Scenarios

### 1. Intra-Cluster Sync (Fast)

Test sync within a single family cluster:

```bash
# Create content on node 1
curl -X POST http://localhost:8081/api/documents \
  -H "Content-Type: application/json" \
  -d '{"id": "doc1", "content": "Hello from node 1"}'

# Verify it appears on node 2 (same cluster)
# Should be near-instant via mDNS discovery
docker exec family-a-node-2 curl -s http://localhost:8080/api/documents/doc1
```

### 2. Cross-Cluster Sync (WAN)

Test sync between family clusters:

```bash
# Create content on Family A
curl -X POST http://localhost:8081/api/documents \
  -H "Content-Type: application/json" \
  -d '{"id": "shared-doc", "reach": "neighborhood", "content": "Shared content"}'

# Verify it appears on Family B (cross-WAN)
# May take a few seconds due to WAN latency
curl http://localhost:8082/api/documents/shared-doc
```

### 3. Network Partition

Simulate network failure between clusters:

```bash
# Disconnect WAN bridge
docker network disconnect simulation_wan-bridge family-a-node-1
docker network disconnect simulation_wan-bridge family-b-node-1

# Create content on both sides
curl -X POST http://localhost:8081/api/documents \
  -d '{"id": "conflict-doc", "content": "Family A version"}'
curl -X POST http://localhost:8082/api/documents \
  -d '{"id": "conflict-doc", "content": "Family B version"}'

# Reconnect
docker network connect simulation_wan-bridge family-a-node-1 --ip 172.20.0.11
docker network connect simulation_wan-bridge family-b-node-1 --ip 172.20.0.21

# Watch CRDT merge resolve the conflict
curl http://localhost:8081/api/documents/conflict-doc
curl http://localhost:8082/api/documents/conflict-doc
```

### 4. Node Failure

Simulate a node going down:

```bash
# Kill primary node
docker stop family-a-node-1

# Verify replica takes over
curl http://localhost:8081/health  # Should fail
# Replica should still have all data

# Bring node back
docker start family-a-node-1

# Verify sync catches up
docker logs -f family-a-node-1
```

## With Latency Simulation

To add realistic WAN latency (50ms ± 10ms jitter):

```bash
# Start with latency profile
docker-compose --profile latency up -d

# This uses Pumba to inject network delay on WAN traffic
```

## Monitoring

### Logs

```bash
# All nodes
docker-compose logs -f

# Specific node
docker-compose logs -f family-a-node-1

# Filter by level
docker-compose logs -f | grep -E "(INFO|WARN|ERROR)"
```

### Metrics

If metrics are enabled, Prometheus endpoints are available:

```bash
curl http://localhost:8081/metrics
curl http://localhost:8082/metrics
```

### Health Checks

```bash
# Check all nodes
for port in 8081 8082; do
  echo "Node on port $port:"
  curl -s http://localhost:$port/health | jq .
done
```

## Configuration

Node configurations are in `configs/`:

```
configs/
├── family-a-node-1.toml   # Primary node, cluster A
├── family-a-node-2.toml   # Replica node, cluster A
├── family-b-node-1.toml   # Primary node, cluster B
└── family-b-node-2.toml   # Replica node, cluster B
```

Environment variables can override config:
- `ELOHIM_NODE_ID` - Node identifier
- `ELOHIM_CLUSTER_NAME` - Cluster name
- `ELOHIM_CLUSTER_KEY` - Cluster authentication key
- `RUST_LOG` - Log level (e.g., `debug`, `info,elohim_node=debug`)

## Cleanup

```bash
# Stop containers
docker-compose down

# Remove volumes (data)
docker-compose down -v

# Remove images
docker-compose down --rmi all

# Full cleanup
docker-compose down -v --rmi all --remove-orphans
```
