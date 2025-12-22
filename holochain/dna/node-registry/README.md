# Node Registry DNA

Distributed ledger for Elohim node orchestration, disaster recovery, and Byzantine-fault-tolerant consensus.

## Status: SCAFFOLDING / NOT YET BUILDABLE

This DNA has been scaffolded with complete architecture and logic, but requires HDI/HDK API updates to build successfully.

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

## Known Build Issues

The coordinator zome requires updates for HDI/HDK API compatibility:

1. **Entry Deserialization**: `to_app_option()` API has changed in HDI 0.5
   - Need to update pattern: `record.entry().to_app_option()` → use new deserialization pattern
   - Affects all functions that retrieve entries from DHT

2. **Type Conversions**: Several i64/u64 mismatches in timestamp handling
   - `max_age_seconds` needs `.try_into().unwrap()` conversion
   - Timestamp comparisons need consistent types

3. **Struct Field Updates**: Some structs need additional fields
   - NodeHeartbeat missing `active_connections` field
   - May need to align with latest HDI schema requirements

## Next Steps to Make Buildable

1. Update coordinator zome entry deserialization to match HDI 0.5 API
2. Fix type conversions (i64 ↔ u64) in timestamp handling
3. Ensure all struct fields match integrity zome definitions
4. Test build: `cargo build --release --target wasm32-unknown-unknown`
5. Package DNA: `hc dna pack . -o workdir/node_registry.dna`

## Integration with Elohim

Once built, this DNA enables:

- **Plug-and-play discovery**: Nodes automatically announce capacity
- **"Organ donation" model**: Opt-in by default (10% storage, 20% bandwidth)
- **Disaster recovery**: Automatic content re-replication on node failure
- **Byzantine tolerance**: Peer health attestation prevents false reports
- **Google Account recovery**: Login from doorway → detect offline → sync from family

See `HOLOCHAIN_ORCHESTRATOR_IMPLEMENTATION_PLAN.md` for full architecture and integration details.

## License

Part of the Elohim Protocol
