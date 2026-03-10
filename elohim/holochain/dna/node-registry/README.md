# Node Registry DNA

Distributed ledger for Elohim node orchestration, disaster recovery, and Byzantine-fault-tolerant consensus.

## Status: BUILDABLE

This DNA compiles successfully with HDI 0.5 / HDK 0.4 API patterns.

## Architecture

### Entry Types

- **NodeRegistration**: Nodes publish capacity, location, capabilities
- **NodeHeartbeat**: Lightweight health updates every 30 seconds
- **HealthAttestation**: Peer-to-peer health verification
- **CustodianAssignment**: Orchestration decisions about content custody

### Link Types

- **Discovery**: RegionToNode, StatusToNode, TierToNode, IdToNodeRegistration, CustodianToNode
- **Health**: NodeToHeartbeat, NodeToAttestations
- **Assignments**: ContentToAssignment, NodeToAssignment

## Functions Implemented

### Node Lifecycle
- `register_node(registration)` - Register new node with "organ donation" defaults
- `update_node_capacity(node_id, updates)` - Update capacity information
- `deregister_node(node_id, reason)` - Deregister node from network

### Health Tracking
- `heartbeat(heartbeat_data)` - Submit heartbeat signal
- `attest_health(attestation)` - Attest to peer health (Byzantine tolerance)
- `get_node_health(node_id)` - Get health consensus from attestations

### Discovery
- `get_nodes_by_region(region)` - Find nodes in geographic region
- `get_available_custodians(filters)` - Find nodes for custodianship with filters
- `get_nodes_by_tier(tier)` - Find nodes by steward tier

### Custodian Assignments
- `assign_custodian(assignment)` - Create custodian assignment
- `get_assignments_for_content(content_id)` - Get all custodians for content
- `get_assignments_for_node(node_id)` - Get all assignments for node

### Disaster Recovery
- `detect_failed_nodes()` - Find nodes with stale heartbeats (>60s)
- `trigger_disaster_recovery(failed_node_id)` - Automatically re-replicate content

## Build Instructions

```bash
# From the node-registry directory:
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown

# Package DNA:
hc dna pack . -o workdir/node_registry.dna
```

## API Patterns Used

This zome uses HDI 0.5 / HDK 0.4 patterns:

- **GetLinksInputBuilder**: `get_links(GetLinksInputBuilder::try_new(...).build())`
- **Action Hash Conversion**: `link.target.into_action_hash()`
- **Entry Deserialization**: Custom `deserialize_*()` helper functions with `SerializedBytes`
- **Timestamp Types**: Consistent `i64` for comparisons with `sys_time().as_seconds_and_nanos().0`

## Integration with Elohim

Once built, this DNA enables:

- **Plug-and-play discovery**: Nodes automatically announce capacity
- **"Organ donation" model**: Opt-in by default (10% storage, 20% bandwidth)
- **Disaster recovery**: Automatic content re-replication on node failure
- **Byzantine tolerance**: Peer health attestation prevents false reports
- **Google Account recovery**: Login from doorway → detect offline → sync from family

## Integration

To integrate with other DNAs:

1. **Package the DNA**:
   ```bash
   hc dna pack . -o workdir/node_registry.dna
   ```

2. **Add to hApp manifest** alongside other roles (lamad, etc.)

3. **Bridge calls** from other DNAs to query custodians, register nodes, etc.

## License

Part of the Elohim Protocol
