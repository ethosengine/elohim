@e2e @elohim @network-health @requires:doorway
Feature: Network Health Posture — Aggregate Awareness and Attestation-Gated Introspection
  As a node operator or the elohim agent
  I want to understand the network's aggregate health and inspect individual peers
  with appropriate trust boundaries
  So that the network can self-heal, operators can diagnose issues, and privacy is
  respected through attestation-gated access to internal state

  Individual service health exists (/health, /version, ComputeReport). Peer
  advertisement exists (gossipsub heartbeat, neighbor table). This feature
  connects them: how the network reasons about itself as a whole, how an operator
  queries a specific peer's internals, and how the right to inspect is earned
  and revocable through attestations.

  The key insight: DetailLevel (error/warn/info/debug/trace) is the shared
  vocabulary. Today it is controlled by a query parameter. Tomorrow it is
  controlled by an attestation with scope "compute:debug" or "compute:trace".
  The labels don't change; only the access model evolves.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" as operator

  # --- Network Posture (Aggregate View) ----------------------------------------

  @wip
  Scenario: Operator sees network posture summary from neighbor table
    Given the neighbor table on doorway "alpha" contains:
      | peer             | ready | budget_remaining | always_on | last_seen      |
      | matthew-home     | true  | 42               | true      | 10 seconds ago |
      | terrance-laptop   | true  | 15               | false     | 20 seconds ago |
      | community-node   | true  | 100              | true      | 5 seconds ago  |
    When Matthew queries the network posture
    Then the posture shows 3 active peers, 0 stale peers
    And the posture shows total_budget_remaining across all peers
    And the posture shows always_on_count=2, intermittent_count=1
    And the posture shows aggregate storage capacity and usage

  @wip
  Scenario: Network posture degrades when always-on peers go offline
    Given the neighbor table shows 2 always-on peers and 1 intermittent peer
    When both always-on peers become stale (missed 3 heartbeats)
    Then the network posture changes from "healthy" to "degraded"
    And the posture indicates only intermittent peers remain
    And the posture warns that replication and serving capacity is reduced

  @wip
  Scenario: Network posture reflects compute exhaustion across the network
    Given all peers have budget_remaining > 0
    And the network posture shows compute_available=true
    When all peers report budget_remaining=0 in their announcements
    Then the network posture shows compute_available=false
    And the posture warns that inference requests will be deferred
    And this state is visible without inspecting individual peer internals

  @wip
  Scenario: Network posture shows storage pressure across the network
    Given "matthew-home" is at 95% storage capacity
    And "community-node" is at 30% storage capacity
    And "terrance-laptop" is at 60% storage capacity
    When Matthew queries the network posture
    Then the posture shows 1 peer under storage pressure
    And the posture shows total available storage across all peers
    And the posture identifies "matthew-home" as the constrained peer

  @wip
  Scenario: Posture includes peer diversity health
    Given the network has peers with diverse profiles
    When Matthew queries network posture
    Then the posture includes a diversity summary:
      | metric                 | description                        |
      | total_peers            | count of all known peers           |
      | always_on_peers        | count of always-on peers           |
      | serving_peers          | count of peers that serve others   |
      | intermittent_peers     | count of intermittent peers        |
      | storage_capable_peers  | count of peers with storage=true   |
    And the diversity summary helps Matthew understand the network's resilience

  # --- Individual Peer Introspection (DetailLevel) -----------------------------

  @wip
  Scenario: Info-level health is available without attestation
    Given "matthew-home" is a connected peer
    When Matthew requests the health of "matthew-home" at info level
    Then the response includes service_id, build info, health status, and uptime
    And the response does not include resource snapshots, peer health, or extensions
    And no attestation is required for info-level data

  @wip
  Scenario: Debug-level health requires a compute:debug attestation
    Given "matthew-home" is a connected peer
    And Matthew holds an attestation with scope "compute:debug" from "matthew-home"
    When Matthew requests the health of "matthew-home" at debug level
    Then the response includes resource snapshots and per-peer health data
    And the response includes active_connections, managed_storage_bytes, and request counts
    And the response does not include trace-level extensions

  @wip
  Scenario: Trace-level health requires a compute:trace attestation
    Given "matthew-home" is a connected peer
    And Matthew holds an attestation with scope "compute:trace" from "matthew-home"
    When Matthew requests the health of "matthew-home" at trace level
    Then the response includes everything: resources, peers, and extensions
    And the extensions include semaphore_permits, cache_hit_rate, and db_pool_stats

  @wip
  Scenario: Peer rejects introspection request above granted attestation level
    Given "community-node" is a connected peer
    And Matthew holds an attestation with scope "compute:debug" from "community-node"
    When Matthew requests the health of "community-node" at trace level
    Then the response is filtered to debug level
    And the response does not include trace-level extensions
    And the response indicates the granted access level

  @wip
  Scenario: Introspection without attestation defaults to info level
    Given "community-node" is a connected peer
    And Matthew holds no compute attestation from "community-node"
    When Matthew requests the health of "community-node" at debug level
    Then the response is filtered to info level only
    And a message indicates that a "compute:debug" attestation is required for deeper access

  # --- Attestation Lifecycle ---------------------------------------------------

  @wip
  Scenario: Operator grants diagnostic attestation to a peer
    Given Matthew operates "matthew-home"
    When Matthew grants a diagnostic attestation to "terrance-laptop":
      | field    | value          |
      | grantor  | matthew-home   |
      | grantee  | terrance-laptop |
      | scope    | compute:debug  |
      | expires  | 24 hours       |
    Then "terrance-laptop" can request debug-level health from "matthew-home"
    And the attestation is stored in the trust cache with TTL

  @wip
  Scenario: Diagnostic attestation expires and access reverts
    Given "terrance-laptop" holds a "compute:debug" attestation from "matthew-home"
    And the attestation expires
    When "terrance-laptop" requests debug-level health from "matthew-home"
    Then the response is filtered to info level
    And "terrance-laptop" is informed the attestation has expired

  @wip
  Scenario: Operator revokes diagnostic attestation
    Given "terrance-laptop" holds a "compute:debug" attestation from "matthew-home"
    When Matthew revokes the attestation
    Then "terrance-laptop" can no longer request debug-level health from "matthew-home"
    And the revocation takes effect within the trust cache TTL

  # --- Elohim Agent Network Reasoning ------------------------------------------

  @wip
  Scenario: Elohim agent incorporates network posture into resilience assessment
    Given the network posture shows 3 active peers with diverse profiles
    And the resilience profile for Matthew shows "protected"
    When "matthew-home" goes offline
    And the network posture changes to "degraded"
    Then the elohim agent incorporates the posture change into resilience reasoning
    And the resilience profile may update to reflect reduced network capacity

  @wip
  Scenario: Elohim agent requests diagnostic attestation to investigate degradation
    Given "terrance-laptop" reports health=degraded in its CapacityAnnouncement
    When the elohim agent on "matthew-home" investigates the degradation
    Then the agent requests a "compute:debug" attestation from "terrance-laptop"
    And the request is presented to "terrance-laptop"'s operator for approval
    And the agent does not proceed with introspection without explicit operator consent

  @wip
  Scenario: Network posture informs compute routing decisions
    Given the network posture shows:
      | peer             | ready | budget_remaining | queue_depth |
      | matthew-home     | true  | 42               | 2           |
      | community-node   | true  | 100              | 0           |
      | terrance-laptop   | false | 0                | 10          |
    When a compute request arrives
    Then the request is routed to "community-node" (highest budget, lowest queue)
    And "terrance-laptop" is excluded (not ready)
    And the routing decision is based on live gossipsub data, not stale state

  # --- Transition: Query Param to Attestation ----------------------------------

  @wip
  Scenario: Same DetailLevel filtering serves both access models
    Given "matthew-home" serves /health with ?detail= query parameter support
    When Matthew requests /health?detail=debug from "matthew-home" directly
    Then the response includes debug-level fields
    And the same filtering logic will apply whether the level came from:
      | source       | mechanism                           |
      | query param  | ?detail=debug (today, no auth)      |
      | attestation  | scope=compute:debug (future, gated) |
    And the DetailLevel enum is the shared vocabulary between both access models

  # --- Edge Cases --------------------------------------------------------------

  @wip
  Scenario: Single-node network has valid but minimal posture
    Given Matthew is the only node on the network
    And no other peers are in the neighbor table
    When Matthew queries the network posture
    Then the posture shows 1 active peer, 0 stale peers
    And the posture shows always_on_count based on Matthew's own profile
    And the posture does not claim the network is degraded
    And the posture notes the absence of redundancy

  @wip
  Scenario: All peers are intermittent with no always-on nodes
    Given only laptop peers are connected (all always_on=false)
    When Matthew queries the network posture
    Then the posture warns that no always-on peers exist
    And the posture suggests that a home node or network node would improve resilience
    And the network still functions but with reduced availability expectations
