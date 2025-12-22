# Holochain Orchestrator Implementation Plan

## Executive Summary

This document outlines the implementation plan for the Elohim Holochain Orchestrator, a distributed system that enables plug-and-play node discovery, automatic disaster recovery, and Byzantine-fault-tolerant consensus for the Elohim network.

**Key Design Principle**: "Organ Donation" Model - Opt-in by default with clear benefits, explicit opt-out available.

## Architecture Overview

### Three-Tier Control Plane

```
┌─────────────────────────────────────────────────────────────┐
│                    GLOBAL LAYER (Tier 3)                     │
│  Node Registry DHT - Byzantine consensus, immutable ledger   │
│  - Node registrations, heartbeats, health attestations      │
│  - Custodian assignments via quorum voting                   │
│  - Latency: ~500ms-2s (global DHT consensus)                │
└─────────────────────────────────────────────────────────────┘
                            ▲
                            │
┌─────────────────────────────────────────────────────────────┐
│                   REGIONAL LAYER (Tier 2)                    │
│  Regional Coordinators - Fast local decisions via Raft      │
│  - Quick custodian selection within region                   │
│  - Performance monitoring and load balancing                 │
│  - Latency: ~10-150ms (WAN within region)                   │
└─────────────────────────────────────────────────────────────┘
                            ▲
                            │
┌─────────────────────────────────────────────────────────────┐
│                     LOCAL LAYER (Tier 1)                     │
│  Doorway Gateways - HTTP bridge to Holochain               │
│  - User authentication and session management                │
│  - Graceful degradation when personal node offline          │
│  - Latency: <1ms (local rack) or ~10ms (nearby doorway)    │
└─────────────────────────────────────────────────────────────┘
```

### Multi-Tier Resilience Model

**Tier 1: Local Rack Modules**
- RAID arrays within single rack
- Module-level redundancy (multiple SSDs, redundant power)
- Latency: <1ms
- Use case: Hot data, active workloads

**Tier 2: Family Network Racks**
- WAN-distributed family member racks
- Relationship-based via HumanRelationship structure
- Automatic custody commitments for intimate content
- Latency: 10-150ms
- Use case: Disaster recovery for personal data

**Tier 3: Community Custodians**
- Economic participation via Shefa model
- Public/commons content replication
- Performance/availability optimization
- Latency: Variable (50ms-2s)
- Use case: Content delivery, community resilience

## The "Organ Donation" Model

### Core Philosophy

By default, every Elohim node contributes to network resilience, similar to how organ donation creates societal benefit. Users can opt-out explicitly, but the default encourages participation with clear benefits.

### Default Configuration

```rust
// Default NodeRegistration configuration (embedded in node setup)
pub struct DefaultNodeConfig {
    // DEFAULT: Participate in network custodianship
    pub custodian_opt_in: bool = true,

    // DEFAULT: 10% of available storage
    pub max_custody_gb: Option<f64> = Some(total_storage_tb * 0.10),

    // DEFAULT: 20% of bandwidth during off-peak hours
    pub max_bandwidth_mbps: Option<u32> = Some(total_bandwidth_mbps * 0.20),

    // DEFAULT: 15% CPU utilization for custodianship
    pub max_cpu_percent: Option<f64> = Some(15.0),

    // DEFAULT: Steward tier based on initial commitment
    pub steward_tier: String = calculate_tier_from_capacity(),
}
```

### User Interface

**First-Time Setup Screen**:
```
┌────────────────────────────────────────────────────────┐
│  Welcome to Elohim Network Participation               │
│                                                        │
│  Your node can help strengthen the network by         │
│  storing copies of community content and supporting   │
│  other nodes in your region.                          │
│                                                        │
│  ✓ You contribute: 10% storage, 20% bandwidth         │
│  ✓ You receive: Faster content delivery               │
│  ✓ You earn: Shefa points and reputation              │
│                                                        │
│  [Continue with Recommended Settings]                 │
│  [Customize Participation]                            │
│  [Opt Out (Not Recommended)]                          │
└────────────────────────────────────────────────────────┘
```

**Settings UI** (accessible anytime):
```
Network Participation Settings
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[ ✓ ] Participate in network custodianship
      Help strengthen the decentralized network by storing
      encrypted copies of community content.

Storage Contribution:
  [=====>·····] 50 GB / 500 GB available (10%)

Bandwidth Contribution:
  [====>······] 20 Mbps / 100 Mbps available (20%)
  Off-peak hours only: 10pm-6am local time

CPU Contribution:
  [==>········] 15% maximum utilization

Your Benefits:
  • 2.5x faster content delivery from community cache
  • 150 Shefa points/month earned
  • "Guardian" steward tier (Tier 2)

Privacy Controls:
  [ ✓ ] Only share aggregate capacity metrics
  [ ✓ ] Encrypt all custodied content
  [   ] Allow emergency access for disaster recovery

[Save Changes]  [Reset to Defaults]  [Opt Out Completely]
```

### Benefits of Participation

| Participation Level | Storage | Bandwidth | Benefits |
|---------------------|---------|-----------|----------|
| **Caretaker** (Tier 1) | 5-10% | 10-20% | 1.5x faster delivery, 50 Shefa/mo |
| **Guardian** (Tier 2) | 10-20% | 20-40% | 2.5x faster delivery, 150 Shefa/mo |
| **Steward** (Tier 3) | 20-40% | 40-60% | 4x faster delivery, 500 Shefa/mo, priority support |
| **Pioneer** (Tier 4) | 40%+ | 60%+ | 10x faster delivery, 2000 Shefa/mo, governance rights |

## Implementation Roadmap

### Phase 1: Node Registry DNA (Weeks 1-2)

**Goal**: Create distributed ledger for node discovery and health tracking

**Tasks**:
1. ✅ Create integrity zome with entry types:
   - NodeRegistration
   - NodeHeartbeat
   - HealthAttestation
   - CustodianAssignment

2. ⏳ Create coordinator zome with functions:
   ```rust
   // Node lifecycle
   pub fn register_node(registration: NodeRegistration) -> ExternResult<ActionHash>
   pub fn update_node_capacity(node_id: String, updates: CapacityUpdates) -> ExternResult<ActionHash>
   pub fn deregister_node(node_id: String, reason: String) -> ExternResult<()>

   // Health tracking
   pub fn heartbeat(node_id: String, status: NodeHeartbeat) -> ExternResult<ActionHash>
   pub fn attest_health(attestation: HealthAttestation) -> ExternResult<ActionHash>
   pub fn get_node_health(node_id: String) -> ExternResult<NodeHealthSummary>

   // Discovery
   pub fn get_nodes_by_region(region: String) -> ExternResult<Vec<NodeRegistration>>
   pub fn get_available_custodians(filters: CustodianFilters) -> ExternResult<Vec<NodeRegistration>>
   pub fn get_nodes_by_tier(tier: String) -> ExternResult<Vec<NodeRegistration>>

   // Assignments
   pub fn assign_custodian(assignment: CustodianAssignment) -> ExternResult<ActionHash>
   pub fn get_assignments_for_content(content_id: String) -> ExternResult<Vec<CustodianAssignment>>
   pub fn get_assignments_for_node(node_id: String) -> ExternResult<Vec<CustodianAssignment>>
   ```

3. Add validation rules:
   ```rust
   fn validate_node_registration(node: &NodeRegistration) -> ExternResult<ValidateCallbackResult> {
       // Verify signature matches agent_pub_key
       verify_signature(&node.signature, &node.agent_pub_key)?;

       // Ensure reasonable capacity values (prevent overflow attacks)
       if node.cpu_cores > 1024 || node.memory_gb > 10000 || node.storage_tb > 10000.0 {
           return Ok(ValidateCallbackResult::Invalid("Unreasonable capacity values".to_string()));
       }

       // Validate region is known
       if !KNOWN_REGIONS.contains(&node.region.as_str()) {
           return Ok(ValidateCallbackResult::Invalid(format!("Unknown region: {}", node.region)));
       }

       // Validate steward tier
       if !STEWARD_TIERS.contains(&node.steward_tier.as_str()) {
           return Ok(ValidateCallbackResult::Invalid(format!("Invalid tier: {}", node.steward_tier)));
       }

       Ok(ValidateCallbackResult::Valid)
   }

   fn validate_node_heartbeat(heartbeat: &NodeHeartbeat) -> ExternResult<ValidateCallbackResult> {
       // Verify signature
       verify_signature(&heartbeat.signature, &heartbeat.node_id)?;

       // Ensure timestamp is recent (< 60 seconds old)
       let now = sys_time()?;
       let heartbeat_time = parse_timestamp(&heartbeat.timestamp)?;
       if now.as_seconds_and_nanos().0 - heartbeat_time.as_seconds_and_nanos().0 > 60 {
           return Ok(ValidateCallbackResult::Invalid("Stale heartbeat".to_string()));
       }

       // Validate status
       if !NODE_STATUS.contains(&heartbeat.status.as_str()) {
           return Ok(ValidateCallbackResult::Invalid(format!("Invalid status: {}", heartbeat.status)));
       }

       Ok(ValidateCallbackResult::Valid)
   }
   ```

4. Create DNA manifest and Cargo.toml
5. Build and test DNA package

**Deliverables**:
- `/holochain/dna/node-registry/` complete DNA package
- Unit tests for all coordinator functions
- Integration test demonstrating node registration and heartbeat

### Phase 2: Orchestrator Integration (Weeks 3-4)

**Goal**: Integrate Node Registry with existing Elohim infrastructure

**Tasks**:
1. Update Doorway to query Node Registry DHT:
   ```typescript
   // elohim-doorway/src/services/custodian-selection.service.ts

   async getCustodiansForContent(contentId: string, reach: string): Promise<CustodianNode[]> {
     // OLD: Query MongoDB for available custodians
     // const custodians = await this.mongoService.findCustodians({ status: 'online' });

     // NEW: Query Node Registry DHT
     const filters = {
       status: 'online',
       min_storage_gb: this.calculateStorageNeeded(contentId),
       region: this.getPreferredRegions(contentId),
       tier: this.getMinTier(reach),
     };

     const nodes = await this.holochainClient.callZome({
       role_name: 'node_registry',
       zome_name: 'node_registry_coordinator',
       fn_name: 'get_available_custodians',
       payload: filters,
     });

     // Apply selection strategy
     return this.selectOptimalCustodians(nodes, contentId, reach);
   }
   ```

2. Add automatic node registration on startup:
   ```typescript
   // elohim-edge-node/src/bootstrap/node-registration.ts

   export async function registerNodeOnStartup() {
     const nodeConfig = await loadNodeConfig();
     const systemInfo = await detectSystemCapacity();

     const registration: NodeRegistration = {
       node_id: nodeConfig.nodeId,
       agent_pub_key: nodeConfig.agentPubKey,
       display_name: nodeConfig.displayName || `${os.hostname()}'s Rack`,

       // Capacity detection
       cpu_cores: os.cpus().length,
       memory_gb: Math.floor(os.totalmem() / (1024**3)),
       storage_tb: await getAvailableStorage(),
       bandwidth_mbps: await detectBandwidth(),

       // Location
       region: nodeConfig.region || await detectRegion(),
       latitude: nodeConfig.location?.latitude,
       longitude: nodeConfig.location?.longitude,

       // Capabilities
       zomes_hosted: await getInstalledZomes(),
       steward_tier: nodeConfig.stewardTier || 'caretaker',

       // Participation (ORGAN DONATION MODEL)
       custodian_opt_in: nodeConfig.custodianOptIn ?? true,  // DEFAULT: true
       max_custody_gb: nodeConfig.maxCustodyGb ?? (storage_tb * 0.10),
       max_bandwidth_mbps: nodeConfig.maxBandwidthMbps ?? (bandwidth_mbps * 0.20),
       max_cpu_percent: nodeConfig.maxCpuPercent ?? 15.0,

       // Health
       uptime_percent: await calculateUptimePercent(),
       last_heartbeat: new Date().toISOString(),

       // Metadata
       registered_at: new Date().toISOString(),
       updated_at: new Date().toISOString(),

       // Proof
       signature: await signRegistration(),
     };

     await holochainClient.callZome({
       role_name: 'node_registry',
       zome_name: 'node_registry_coordinator',
       fn_name: 'register_node',
       payload: registration,
     });

     // Start heartbeat loop
     startHeartbeatLoop(registration.node_id);
   }
   ```

3. Implement heartbeat loop:
   ```typescript
   function startHeartbeatLoop(nodeId: string) {
     setInterval(async () => {
       const heartbeat: NodeHeartbeat = {
         node_id: nodeId,
         timestamp: new Date().toISOString(),
         status: await determineNodeStatus(),
         current_load: os.loadavg()[0] / os.cpus().length,
         active_connections: await getActiveConnections(),
         signature: await signHeartbeat(),
       };

       await holochainClient.callZome({
         role_name: 'node_registry',
         zome_name: 'node_registry_coordinator',
         fn_name: 'heartbeat',
         payload: heartbeat,
       });
     }, 30_000); // Every 30 seconds
   }
   ```

4. Add network participation settings UI:
   - Settings page in Elohim Dashboard
   - Real-time capacity monitoring
   - Benefits calculator ("If you contribute X, you receive Y")

**Deliverables**:
- Updated Doorway with DHT integration
- Automatic node registration on Edge Node startup
- Settings UI for network participation
- Migration guide from MongoDB to DHT

### Phase 3: Auto-Healing and Disaster Recovery (Weeks 5-6)

**Goal**: Automatically detect node failures and re-replicate content

**Tasks**:
1. Create health monitoring daemon:
   ```rust
   // node-registry coordinator zome

   #[hdk_extern]
   pub fn detect_failed_nodes(_: ()) -> ExternResult<Vec<String>> {
       let now = sys_time()?;
       let all_nodes = get_all_registered_nodes()?;
       let mut failed_nodes = Vec::new();

       for node in all_nodes {
           // Get latest heartbeat
           let query = LinkQuery::try_new(
               hash_entry(&node)?,
               LinkTypes::NodeToHeartbeat
           )?;
           let links = get_links(query, GetStrategy::default())?;

           if links.is_empty() {
               failed_nodes.push(node.node_id);
               continue;
           }

           let latest_heartbeat = get_latest_heartbeat(&links)?;
           let heartbeat_time = parse_timestamp(&latest_heartbeat.timestamp)?;

           // If no heartbeat in 60 seconds, mark as failed
           if now.as_seconds_and_nanos().0 - heartbeat_time.as_seconds_and_nanos().0 > 60 {
               failed_nodes.push(node.node_id);
           }
       }

       Ok(failed_nodes)
   }

   #[hdk_extern]
   pub fn trigger_disaster_recovery(failed_node_id: String) -> ExternResult<Vec<ActionHash>> {
       // Get all content custodied by failed node
       let query = LinkQuery::try_new(
           hash_entry(&anchor_for_node(&failed_node_id))?,
           LinkTypes::NodeToAssignment
       )?;
       let assignments = get_links(query, GetStrategy::default())?;

       let mut new_assignments = Vec::new();

       for assignment_link in assignments {
           let assignment = get_assignment(&assignment_link)?;

           // Find replacement custodians
           let filters = CustodianFilters {
               exclude_nodes: vec![failed_node_id.clone()],
               region: assignment.preferred_region,
               min_tier: assignment.required_tier,
               min_storage_gb: assignment.content_size_gb,
           };

           let available_custodians = get_available_custodians(filters)?;

           if available_custodians.is_empty() {
               // Escalate to regional coordinator
               emit_signal(Signal::DisasterRecoveryFailed {
                   content_id: assignment.content_id.clone(),
                   reason: "No available custodians".to_string(),
               })?;
               continue;
           }

           // Create new assignment
           let new_assignment = CustodianAssignment {
               assignment_id: format!("recovery-{}-{}", assignment.content_id, sys_time()?),
               content_id: assignment.content_id,
               content_hash: assignment.content_hash,
               custodian_node_id: available_custodians[0].node_id.clone(),
               strategy: assignment.strategy,
               shard_index: assignment.shard_index,
               decided_by: "disaster_recovery_daemon".to_string(),
               decision_round: assignment.decision_round + 1,
               votes_json: "".to_string(),
               created_at: timestamp_now()?,
               expires_at: calculate_expiration()?,
           };

           let hash = create_entry(EntryTypes::CustodianAssignment(new_assignment.clone()))?;
           new_assignments.push(hash);

           // Emit signal to trigger actual content transfer
           emit_signal(Signal::ReplicateContent {
               content_id: new_assignment.content_id,
               from_shard_holders: find_other_custodians(&assignment.content_id)?,
               to_custodian: new_assignment.custodian_node_id,
               strategy: new_assignment.strategy,
           })?;
       }

       Ok(new_assignments)
   }
   ```

2. Implement content re-replication service:
   ```typescript
   // elohim-doorway/src/services/replication.service.ts

   export class ReplicationService {
     async onDisasterRecoverySignal(signal: DisasterRecoverySignal) {
       const { content_id, from_shard_holders, to_custodian, strategy } = signal;

       // Retrieve content from remaining custodians
       const shards = await this.retrieveShards(content_id, from_shard_holders);

       if (strategy === 'threshold_split') {
         // Reconstruct from Shamir's Secret Sharing
         const reconstructed = await this.shamirReconstruct(shards);
         await this.transferToNewCustodian(reconstructed, to_custodian);
       } else if (strategy === 'erasure_coded') {
         // Reconstruct from Reed-Solomon
         const reconstructed = await this.reedSolomonReconstruct(shards);
         await this.transferToNewCustodian(reconstructed, to_custodian);
       } else {
         // full_replica: just copy from any holder
         await this.transferFromCustodian(content_id, from_shard_holders[0], to_custodian);
       }

       logger.info(`Disaster recovery complete: ${content_id} → ${to_custodian}`);
     }
   }
   ```

3. Add "Google Account" recovery flow:
   ```typescript
   // elohim-doorway/src/routes/recovery.routes.ts

   router.post('/api/recovery/initiate', async (req, res) => {
     const { agentId, authToken } = req.body;

     // Verify user owns this agent ID (social recovery, seed phrase, etc.)
     await verifyOwnership(agentId, authToken);

     // Check if their node is offline
     const nodeStatus = await getNodeStatus(agentId);
     if (nodeStatus !== 'offline') {
       return res.status(400).json({ error: 'Node is online, recovery not needed' });
     }

     // Find intimate relationships with custody enabled
     const relationships = await holochainClient.callZome({
       role_name: 'lamad_spike',
       zome_name: 'content_store',
       fn_name: 'get_intimate_relationships',
       payload: agentId,
     });

     // Get custodian nodes
     const custodians = relationships
       .filter(r => r.auto_custody_enabled && r.consent_given_by_a && r.consent_given_by_b)
       .map(r => r.party_a_id === agentId ? r.party_b_id : r.party_a_id);

     if (custodians.length === 0) {
       return res.status(404).json({
         error: 'No family custodians found',
         suggestion: 'Set up family relationships with custody enabled'
       });
     }

     // Create recovery session
     const recoverySession = await createRecoverySession({
       agentId,
       custodians,
       initiatedAt: new Date(),
       status: 'pending_verification',
     });

     res.json({
       recoverySessionId: recoverySession.id,
       custodiansFound: custodians.length,
       estimatedDataSize: await estimateCustodiedData(agentId, custodians),
       nextSteps: [
         'Verify identity with M-of-N family quorum',
         'Prepare replacement node',
         'Initiate data sync from custodians',
       ],
     });
   });

   router.post('/api/recovery/sync', async (req, res) => {
     const { recoverySessionId, newNodeId } = req.body;

     const session = await getRecoverySession(recoverySessionId);
     if (session.status !== 'verified') {
       return res.status(403).json({ error: 'Recovery not verified yet' });
     }

     // Trigger sync from all custodians to new node
     const syncJobs = await Promise.all(
       session.custodians.map(custodianId =>
         syncDataFromCustodian({
           fromCustodian: custodianId,
           toNode: newNodeId,
           contentFilters: { author: session.agentId, reach: ['private', 'intimate'] },
         })
       )
     );

     res.json({
       syncJobIds: syncJobs.map(j => j.id),
       estimatedDuration: calculateSyncDuration(syncJobs),
       status: 'syncing',
     });
   });
   ```

**Deliverables**:
- Health monitoring daemon detecting failed nodes
- Automatic content re-replication on failure
- "Google Account" style recovery API endpoints
- End-to-end disaster recovery test

### Phase 4: Byzantine Fault Tolerance (Weeks 7-8)

**Goal**: Enable peer-to-peer health attestation and quorum-based decisions

**Tasks**:
1. Implement health attestation protocol:
   ```rust
   #[hdk_extern]
   pub fn attest_peer_health(subject_node_id: String) -> ExternResult<ActionHash> {
       let my_node_id = get_my_node_id()?;

       // Prevent self-attestation
       if my_node_id == subject_node_id {
           return Err(wasm_error!("Cannot attest to own health"));
       }

       // Ping subject node
       let start_time = sys_time()?;
       let ping_result = ping_node(&subject_node_id)?;
       let end_time = sys_time()?;

       let response_time_ms = (end_time.as_micros() - start_time.as_micros()) / 1000;

       let attestation = HealthAttestation {
           attester_node_id: my_node_id,
           subject_node_id,
           response_time_ms: response_time_ms as u32,
           success: ping_result.is_ok(),
           timestamp: timestamp_now()?,
           signature: sign_attestation()?,
       };

       create_entry(EntryTypes::HealthAttestation(attestation))
   }

   #[hdk_extern]
   pub fn get_node_health_consensus(node_id: String) -> ExternResult<HealthConsensus> {
       // Get all health attestations for this node
       let query = LinkQuery::try_new(
           hash_entry(&anchor_for_node(&node_id))?,
           LinkTypes::NodeToAttestations
       )?;
       let links = get_links(query, GetStrategy::default())?;

       let mut successful_pings = 0;
       let mut failed_pings = 0;
       let mut total_response_time = 0u64;

       for link in links {
           let attestation = get_attestation(&link)?;

           // Only consider recent attestations (< 5 minutes old)
           if is_recent(&attestation.timestamp, 300)? {
               if attestation.success {
                   successful_pings += 1;
                   total_response_time += attestation.response_time_ms as u64;
               } else {
                   failed_pings += 1;
               }
           }
       }

       let total_attestations = successful_pings + failed_pings;
       if total_attestations == 0 {
           return Ok(HealthConsensus {
               status: "unknown".to_string(),
               confidence: 0.0,
               avg_response_time_ms: None,
           });
       }

       let health_ratio = successful_pings as f64 / total_attestations as f64;

       let status = if health_ratio >= 0.90 {
           "healthy"
       } else if health_ratio >= 0.60 {
           "degraded"
       } else {
           "unhealthy"
       };

       Ok(HealthConsensus {
           status: status.to_string(),
           confidence: health_ratio,
           avg_response_time_ms: if successful_pings > 0 {
               Some((total_response_time / successful_pings as u64) as u32)
           } else {
               None
           },
       })
   }
   ```

2. Implement quorum-based custodian assignment:
   ```rust
   #[hdk_extern]
   pub fn propose_custodian_assignment(proposal: AssignmentProposal) -> ExternResult<ActionHash> {
       // Get eligible voters (nodes in same region)
       let voters = get_nodes_by_region(proposal.region.clone())?;

       // Create voting record
       let vote_record = VoteRecord {
           proposal_id: proposal.id.clone(),
           voters: voters.iter().map(|n| n.node_id.clone()).collect(),
           votes: HashMap::new(),
           created_at: timestamp_now()?,
           voting_deadline: calculate_deadline(120)?, // 2 minute voting window
           quorum_threshold: 0.66, // 2/3 majority required
       };

       create_entry(EntryTypes::VoteRecord(vote_record))
   }

   #[hdk_extern]
   pub fn cast_vote(vote: CustodianVote) -> ExternResult<ActionHash> {
       let my_node_id = get_my_node_id()?;

       // Get vote record
       let vote_record = get_vote_record(&vote.proposal_id)?;

       // Verify I'm eligible to vote
       if !vote_record.voters.contains(&my_node_id) {
           return Err(wasm_error!("Not eligible to vote on this proposal"));
       }

       // Verify voting window still open
       if is_past_deadline(&vote_record.voting_deadline)? {
           return Err(wasm_error!("Voting deadline passed"));
       }

       // Record vote
       vote_record.votes.insert(my_node_id, vote.approve);
       update_entry(vote_record)?;

       // Check if quorum reached
       check_and_execute_quorum(&vote.proposal_id)
   }

   fn check_and_execute_quorum(proposal_id: &str) -> ExternResult<ActionHash> {
       let vote_record = get_vote_record(proposal_id)?;
       let total_votes = vote_record.votes.len();
       let approval_votes = vote_record.votes.values().filter(|&&v| v).count();

       // Need quorum (2/3) to execute
       if approval_votes as f64 / total_votes as f64 >= vote_record.quorum_threshold {
           // Execute assignment
           let proposal = get_proposal(proposal_id)?;
           let assignment = CustodianAssignment {
               assignment_id: proposal.id,
               content_id: proposal.content_id,
               content_hash: proposal.content_hash,
               custodian_node_id: proposal.custodian_node_id,
               strategy: proposal.strategy,
               shard_index: proposal.shard_index,
               decided_by: "quorum".to_string(),
               decision_round: proposal.round,
               votes_json: serde_json::to_string(&vote_record.votes)?,
               created_at: timestamp_now()?,
               expires_at: calculate_expiration()?,
           };

           create_entry(EntryTypes::CustodianAssignment(assignment))
       } else {
           Err(wasm_error!("Quorum not yet reached"))
       }
   }
   ```

3. Add Byzantine attack detection:
   ```rust
   // Detect nodes providing false attestations
   fn detect_byzantine_attackers() -> ExternResult<Vec<String>> {
       let all_nodes = get_all_registered_nodes()?;
       let mut suspicious_nodes = Vec::new();

       for node in all_nodes {
           // Get attestations made BY this node
           let attestations_by = get_attestations_by_node(&node.node_id)?;

           for attestation in attestations_by {
               // Get consensus about the subject
               let consensus = get_node_health_consensus(attestation.subject_node_id.clone())?;

               // If this node's attestation contradicts consensus
               let contradicts = (attestation.success && consensus.status == "unhealthy") ||
                                (!attestation.success && consensus.status == "healthy");

               if contradicts {
                   // Mark as suspicious
                   suspicious_nodes.push(node.node_id.clone());

                   // Reduce reputation
                   reduce_reputation(&node.node_id, 10)?;
               }
           }
       }

       Ok(suspicious_nodes)
   }
   ```

**Deliverables**:
- Peer-to-peer health attestation protocol
- Quorum-based voting for custodian assignments
- Byzantine attack detection and reputation system
- Security audit and penetration testing report

## Integration Points

### 1. HumanRelationship → Auto-Custody

When intimate relationship is confirmed:
```rust
// Triggered in create_human_relationship()
if relationship.auto_custody_enabled
   && relationship.consent_given_by_a
   && relationship.consent_given_by_b {

    create_auto_custody_commitments(&relationship, &relationship_hash)?;

    // This creates CustodianCommitment entries which the orchestrator
    // uses to automatically replicate intimate content
}
```

### 2. Doorway → Node Registry DHT

Doorway queries for custodians:
```typescript
// Old: MongoDB query
const custodians = await db.custodians.find({ status: 'online' });

// New: DHT query
const custodians = await holochainClient.callZome({
  role_name: 'node_registry',
  fn_name: 'get_available_custodians',
  payload: filters,
});
```

### 3. Content Creation → Replication Trigger

When content with intimate/private reach is created:
```rust
// Future enhancement to create_content()
if content.reach == "intimate" || content.reach == "private" {
    let relationships = get_intimate_relationships(author_agent_id)?;

    for relationship in relationships {
        // Find custodian node for this family member
        let custodian_node = get_node_for_agent(relationship.party_b_id)?;

        // Create assignment
        let assignment = CustodianAssignment {
            content_id: content.id,
            custodian_node_id: custodian_node.node_id,
            strategy: "full_replica".to_string(),
            decided_by: "relationship_policy".to_string(),
            // ...
        };

        create_custodian_assignment(assignment)?;

        // Trigger actual replication (via signal to Doorway)
        emit_signal(Signal::ReplicateContent { ... })?;
    }
}
```

### 4. Shefa Economic Model → Participation Incentives

Custodian participation earns Shefa points:
```rust
// When content is successfully delivered from custodian cache
fn reward_custodian_for_delivery(custodian_node_id: String, content_size_mb: f64) -> ExternResult<()> {
    // Base points: 1 point per MB delivered
    let base_points = content_size_mb as u64;

    // Multiplier based on steward tier
    let node = get_node_registration(custodian_node_id)?;
    let tier_multiplier = match node.steward_tier.as_str() {
        "pioneer" => 4.0,
        "steward" => 2.5,
        "guardian" => 1.5,
        "caretaker" => 1.0,
        _ => 1.0,
    };

    let points = (base_points as f64 * tier_multiplier) as u64;

    // Create ShefaTransaction via Economic Events
    create_economic_event(EconomicEvent {
        action: "earn".to_string(),
        provider: custodian_node_id,
        receiver: custodian_node_id.clone(),
        resource_quantity: ResourceQuantity {
            has_numerical_value: points as f64,
            has_unit: "shefa_points".to_string(),
        },
        note: Some(format!("Content delivery reward: {} MB", content_size_mb)),
        // ...
    })?;

    Ok(())
}
```

## Testing Strategy

### Unit Tests

Test each coordinator function in isolation:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_register_node() {
        let registration = NodeRegistration { /* ... */ };
        let hash = register_node(registration).unwrap();
        assert!(hash.len() > 0);
    }

    #[test]
    fn test_heartbeat_creates_link() {
        let heartbeat = NodeHeartbeat { /* ... */ };
        heartbeat(heartbeat).unwrap();

        let links = get_links_for_node("test-node").unwrap();
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn test_detect_failed_nodes() {
        // Create node with old heartbeat
        create_stale_node("old-node", 120); // 120 seconds ago

        let failed = detect_failed_nodes(()).unwrap();
        assert!(failed.contains(&"old-node".to_string()));
    }
}
```

### Integration Tests

Test end-to-end flows:
```typescript
describe('Disaster Recovery Flow', () => {
  it('should automatically re-replicate when custodian fails', async () => {
    // Setup: Create content with 3 custodians
    const contentId = await createTestContent();
    const custodians = await assignCustodians(contentId, 3);

    // Simulate failure of one custodian
    await simulateNodeFailure(custodians[0].nodeId);

    // Wait for health monitoring to detect failure
    await sleep(65_000); // 65 seconds

    // Verify disaster recovery triggered
    const newAssignments = await getAssignmentsForContent(contentId);
    expect(newAssignments.length).toBe(3); // Should maintain 3 replicas
    expect(newAssignments.map(a => a.custodianNodeId)).not.toContain(custodians[0].nodeId);
  });

  it('should recover user data when node hit by lightning', async () => {
    // Setup: Alice with intimate relationship to Bob
    const alice = await createTestAgent('alice');
    const bob = await createTestAgent('bob');
    await createIntimateRelationship(alice.id, bob.id);

    // Alice creates private photos
    const photoIds = await createPrivatePhotos(alice.id, 5);

    // Verify Bob's node has custody
    const bobCustody = await getNodeAssignments(bob.nodeId);
    expect(bobCustody.map(a => a.contentId)).toEqual(expect.arrayContaining(photoIds));

    // Simulate Alice's node destroyed
    await destroyNode(alice.nodeId);

    // Alice logs in from doorway
    const session = await loginFromDoorway(alice.credentials);
    expect(session.nodeStatus).toBe('offline');

    // Initiate recovery
    const recovery = await initiateRecovery(alice.id);
    expect(recovery.custodiansFound).toBe(1);
    expect(recovery.custodians).toContain(bob.id);

    // Setup new node and sync
    const newNode = await setupReplacementNode(alice.id);
    await syncFromCustodians(recovery.id, newNode.id);

    // Verify all photos restored
    const restoredPhotos = await getContentOnNode(newNode.id);
    expect(restoredPhotos.map(p => p.id)).toEqual(expect.arrayContaining(photoIds));
  });
});
```

### Byzantine Scenario Tests

Test attack scenarios:
```rust
#[test]
fn test_false_health_attestations_detected() {
    // Setup: Healthy node
    let honest_node = create_test_node("honest");

    // Attacker provides false attestation
    let attacker = create_test_node("attacker");
    let false_attestation = HealthAttestation {
        attester_node_id: attacker.node_id,
        subject_node_id: honest_node.node_id,
        success: false, // LIE: node is actually healthy
        // ...
    };
    attest_health(false_attestation).unwrap();

    // Other nodes provide truthful attestations
    for i in 0..10 {
        let honest_attester = create_test_node(&format!("honest-{}", i));
        attest_health(HealthAttestation {
            attester_node_id: honest_attester.node_id,
            subject_node_id: honest_node.node_id,
            success: true, // TRUTH
            // ...
        }).unwrap();
    }

    // Run Byzantine detection
    let attackers = detect_byzantine_attackers().unwrap();
    assert!(attackers.contains(&attacker.node_id));

    // Verify attacker's reputation reduced
    let attacker_node = get_node_registration(attacker.node_id).unwrap();
    assert!(attacker_node.reputation < 100); // Started at 100
}

#[test]
fn test_sybil_attack_prevented() {
    // Attacker creates 100 fake nodes
    for i in 0..100 {
        register_node(create_fake_node(&format!("sybil-{}", i))).unwrap();
    }

    // Try to vote on custodian assignment
    let proposal = create_assignment_proposal();

    // All sybil nodes vote
    for i in 0..100 {
        cast_vote(CustodianVote {
            proposal_id: proposal.id.clone(),
            voter_node_id: format!("sybil-{}", i),
            approve: true,
        }).expect_err("Should fail signature verification");
    }

    // Verify proposal did NOT pass (sybil votes rejected)
    let vote_record = get_vote_record(&proposal.id).unwrap();
    assert!(vote_record.votes.len() == 0); // No valid votes recorded
}
```

## Deployment Guide

### Phase 1: Pilot Deployment (Weeks 9-10)

**Goal**: Deploy to small trusted network for validation

1. **Select Pilot Participants**:
   - 10-20 families with existing Elohim racks
   - Geographic diversity (US West, US East, Canada, EU)
   - Mix of steward tiers

2. **Deploy Node Registry DNA**:
   ```bash
   # Build DNA
   cd holochain/dna/node-registry
   cargo build --release --target wasm32-unknown-unknown
   hc dna pack . -o workdir/node_registry.dna

   # Deploy to each pilot node
   for node in $(cat pilot_nodes.txt); do
     scp workdir/node_registry.dna $node:/tmp/
     ssh $node "holochain-admin install-dna /tmp/node_registry.dna node_registry"
   done
   ```

3. **Update Doorway Software**:
   ```bash
   # Deploy updated Doorway with DHT integration
   git pull origin feature/orchestrator-integration
   npm run build
   pm2 restart elohim-doorway
   ```

4. **Monitor Metrics**:
   - Node registration rate
   - Heartbeat reliability
   - False positive health failures
   - Average custodian selection time
   - User feedback on "organ donation" messaging

### Phase 2: Gradual Rollout (Weeks 11-14)

**Goal**: Expand to broader network with staged rollout

1. **Week 11**: Deploy to 100 nodes (10% of network)
2. **Week 12**: Deploy to 300 nodes (30% of network)
3. **Week 13**: Deploy to 700 nodes (70% of network)
4. **Week 14**: Full network deployment (100%)

**Rollback Plan**:
- If >5% node registration failures, pause rollout
- If >10% heartbeat failures, rollback to previous Doorway version
- If custodian selection latency >5s, disable DHT queries and fallback to local cache

### Phase 3: Production Hardening (Weeks 15-16)

1. **Performance Optimization**:
   - Implement DHT query caching (5-minute TTL)
   - Add regional coordinator election (Raft consensus)
   - Optimize heartbeat frequency based on network size

2. **Security Hardening**:
   - Enable signature verification in all validation rules
   - Implement rate limiting on heartbeat submissions
   - Add anomaly detection for unusual registration patterns

3. **Monitoring & Alerting**:
   - Grafana dashboards for orchestrator metrics
   - PagerDuty alerts for disaster recovery failures
   - Weekly health reports emailed to network operators

## Success Metrics

### Network Health
- **Node Registration Rate**: >95% of online nodes registered within 5 minutes of startup
- **Heartbeat Reliability**: >99% of heartbeats delivered within 30-second window
- **Health Attestation Coverage**: Each node receives attestations from >3 peers every 5 minutes

### Disaster Recovery
- **Detection Time**: Failed nodes detected within 60 seconds of last heartbeat
- **Recovery Initiation**: Disaster recovery triggered within 120 seconds of failure detection
- **Recovery Completion**: Full content re-replication completed within 30 minutes for <10GB datasets

### User Experience
- **Participation Rate**: >80% of users opt-in to network custodianship
- **Recovery Success Rate**: >95% of users successfully recover data after node failure
- **Time to Recovery**: Users can access data from doorway within 5 seconds, full restoration to new node within 24 hours

### Economic
- **Custodian Earnings**: Average custodian earns 500-2000 Shefa points/month
- **Network Efficiency**: >90% of content served from local/regional cache (not original author)
- **Storage Utilization**: 60-80% of contributed custodian storage actually used

## Next Steps

1. **Immediate** (Next Sprint):
   - Complete Node Registry coordinator zome implementation
   - Build unit tests for all coordinator functions
   - Create pilot deployment plan

2. **Short-Term** (Next Month):
   - Deploy to pilot network
   - Gather feedback on "organ donation" messaging
   - Measure baseline metrics

3. **Medium-Term** (Next Quarter):
   - Full network rollout
   - Implement Byzantine fault tolerance
   - Launch Shefa economic rewards for custodianship

4. **Long-Term** (Next Year):
   - Cross-regional disaster recovery
   - Advanced threat detection
   - Fully autonomous self-healing network

## Appendix: Known Regions

```rust
pub const KNOWN_REGIONS: [&str; 20] = [
    // North America
    "us-west", "us-central", "us-east",
    "ca-west", "ca-central", "ca-east",
    "mx-central",

    // Europe
    "eu-west", "eu-central", "eu-north", "eu-south",

    // Asia-Pacific
    "ap-south", "ap-southeast", "ap-northeast", "ap-east",

    // Other
    "sa-east",      // South America
    "af-south",     // Africa
    "me-central",   // Middle East
    "oc-southeast", // Oceania
    "global",       // Not region-specific
];
```

## Appendix: Shard Strategies Explained

**full_replica**: Complete copy on each custodian
- Pros: Fastest recovery, simplest implementation
- Cons: Highest storage cost
- Use case: Small personal files, intimate content

**threshold_split**: Shamir's Secret Sharing (M-of-N)
- Pros: Privacy (no single custodian has full content), efficient storage
- Cons: Requires M custodians online to reconstruct
- Use case: Sensitive documents, encryption keys

**erasure_coded**: Reed-Solomon erasure coding
- Pros: Optimal storage/redundancy tradeoff, tolerates many failures
- Cons: Computational overhead for encoding/decoding
- Use case: Large media files, public content

## Appendix: Steward Tier Benefits Matrix

| Tier | Storage | Bandwidth | CPU | Benefits | Shefa/mo | Governance |
|------|---------|-----------|-----|----------|----------|------------|
| Caretaker | 5-10% | 10-20% | 5-10% | 1.5x delivery, priority routing | 50 | None |
| Guardian | 10-20% | 20-40% | 10-15% | 2.5x delivery, guaranteed uptime | 150 | Regional voting |
| Steward | 20-40% | 40-60% | 15-25% | 4x delivery, dedicated support | 500 | Qahal proposals |
| Pioneer | 40%+ | 60%+ | 25%+ | 10x delivery, network leadership | 2000 | Constitutional amendments |

---

**Document Version**: 1.0
**Last Updated**: 2025-01-15
**Author**: Claude (Elohim AI Assistant)
**Status**: Ready for PR Merge
