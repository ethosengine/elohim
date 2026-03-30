@e2e @federation @peer-advertisement @requires:doorway
Feature: Peer Advertisement — The Network's Self-Awareness Heartbeat
  As a node operator
  I want every peer to broadcast what it is, what it can do, and what state it is in
  So that the network can make intelligent routing, replication, and degradation decisions
  based on actual peer diversity rather than assumptions

  The Elohim Protocol network is not homogeneous. It contains laptops that sleep,
  home nodes that serve a family, network nodes that serve the public, and doorways
  that absorb web2 traffic. Each has different storage, compute, cache, and
  availability profiles. The network must understand this diversity in real time.

  This is the gossipsub heartbeat layer — the 30-second broadcast loop that tells
  every connected peer "here is what I am and what I can do right now." It uses
  the CapacityAnnouncement struct on the /elohim/compute/capacity/1.0.0 topic,
  encoded as MessagePack with 4-byte BE length prefix.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- Peer Profile Identity ---------------------------------------------------

  @wip
  Scenario: Laptop peer advertises intermittent profile
    Given human "Timothy" has a Tauri node "timothy-laptop" configured as laptop
    And "timothy-laptop" has capabilities: storage=true, always_on=false, max_storage=10GB, cache_budget=200MB
    When "timothy-laptop" joins the network
    Then "timothy-laptop" broadcasts a CapacityAnnouncement within 30 seconds
    And the announcement includes node_id, timestamp, and ready=true
    And the announcement capabilities include "storage" but not "always-on"
    And connected peers receive the announcement via gossipsub

  @wip
  Scenario: Home node advertises always-on family-serving profile
    Given human "Matthew" has a home node "matthew-home" configured as home_node
    And "matthew-home" has capabilities: storage=true, always_on=true, max_storage=100GB, cache_budget=2GB, serve_family=true
    When "matthew-home" joins the network
    Then "matthew-home" broadcasts a CapacityAnnouncement within 30 seconds
    And the announcement capabilities include "storage", "always-on", and "serve-family"
    And the announcement shows estimated_tokens_per_sec based on measured throughput

  @wip
  Scenario: Network node advertises public-serving profile
    Given a network node "community-node" is configured as network_node
    And "community-node" has capabilities: always_on=true, max_storage=500GB, cache_budget=10GB, serve_public=true
    When "community-node" joins the network
    Then "community-node" broadcasts a CapacityAnnouncement within 30 seconds
    And the announcement capabilities include "serve-public"
    And the announcement budget_remaining reflects compute allocation

  @wip
  Scenario: Doorway advertises its profile alongside storage peers
    Given doorway "alpha" is running with projection cache enabled
    When doorway "alpha" participates in gossipsub
    Then doorway "alpha" broadcasts a CapacityAnnouncement
    And the announcement capabilities include "projection-cache" and "web2-absorption"
    And the announcement is distinguishable from storage peer announcements by capability set

  # --- Gossipsub Heartbeat Mechanics -------------------------------------------

  @wip
  Scenario: Peer broadcasts capacity every 30 seconds
    Given "matthew-home" is connected to the gossipsub mesh
    When 90 seconds elapse
    Then "matthew-home" has broadcast at least 3 CapacityAnnouncements
    And each announcement has an incrementing timestamp
    And the announcements are published to "/elohim/compute/capacity/1.0.0"

  @wip
  Scenario: Receiving peer builds neighbor table from announcements
    Given "timothy-laptop" is connected to "matthew-home" via gossipsub
    When "matthew-home" broadcasts a CapacityAnnouncement
    Then "timothy-laptop" decodes the MessagePack announcement
    And "timothy-laptop" records the announcement in its neighbor capacity table
    And the table entry is keyed by node_id with last_seen timestamp

  @wip
  Scenario: Stale announcements are evicted from neighbor table
    Given "timothy-laptop" has a neighbor table entry for "matthew-home"
    And the entry was last updated 120 seconds ago
    And the staleness threshold is 90 seconds (3 missed heartbeats)
    When "timothy-laptop" checks the neighbor table
    Then the entry for "matthew-home" is marked as stale
    And "matthew-home" is not considered for routing or replication decisions

  # --- Dynamic State Changes ---------------------------------------------------

  @wip
  Scenario: Storage filling updates capacity announcement
    Given "matthew-home" has max_storage=100GB and current usage is 40GB
    And "matthew-home" is broadcasting announcements with budget_remaining reflecting available storage
    When content replication fills storage to 95GB
    Then the next CapacityAnnouncement shows reduced budget_remaining
    And connected peers see the storage pressure in their neighbor tables

  @wip
  Scenario: Compute budget exhaustion is reflected in announcement
    Given "matthew-home" has been processing inference requests
    And the compute budget drops to 0
    When the next heartbeat fires
    Then the CapacityAnnouncement shows budget_remaining=0
    And the announcement shows ready=false
    And peers stop routing compute requests to "matthew-home"

  @wip
  Scenario: Peer coming online announces immediately
    Given "timothy-laptop" was previously offline (lid closed)
    When Timothy opens the laptop and "timothy-laptop" reconnects to the network
    Then "timothy-laptop" broadcasts a CapacityAnnouncement within 5 seconds
    And the announcement reflects current state (cache may be cold)
    And the regular 30-second heartbeat resumes

  @wip
  Scenario: Peer going offline is detected by absence of heartbeats
    Given "timothy-laptop" is connected and broadcasting
    And "matthew-home" has a fresh neighbor table entry for "timothy-laptop"
    When Timothy closes the laptop (ungraceful disconnect)
    Then "matthew-home" receives no further announcements from "timothy-laptop"
    And after 90 seconds, "matthew-home" marks "timothy-laptop" as stale
    And after the connection timeout, the entry is evicted

  @wip
  Scenario: Cache warming updates readiness in announcement
    Given "matthew-home" has an empty ExtractionCache
    And "matthew-home" is broadcasting with ready_content empty
    When content "evolution-of-trust" is extracted into the cache
    Then the next CapacityAnnouncement includes "evolution-of-trust" in capabilities
    And peers recognize "matthew-home" can now serve that content

  @wip
  Scenario: Cache eviction removes content from announcement
    Given "matthew-home" has "evolution-of-trust" warm in its ExtractionCache
    And "matthew-home" is advertising "evolution-of-trust" in capabilities
    When the ExtractionCache evicts "evolution-of-trust" due to budget pressure
    Then the next CapacityAnnouncement no longer includes "evolution-of-trust"
    And peers stop considering "matthew-home" for delivery of that content

  # --- Peer Diversity on the Same Network --------------------------------------

  @wip @requires:multi-node
  Scenario: Network shows diverse peer profiles simultaneously
    Given the following peers are connected via gossipsub:
      | peer             | profile      | always_on | storage  | cache    | serve        |
      | timothy-laptop   | laptop       | false     | 10GB     | 200MB    | none         |
      | matthew-home     | home_node    | true      | 100GB    | 2GB      | family       |
      | community-node   | network_node | true      | 500GB    | 10GB     | public       |
      | doorway-alpha    | doorway      | true      | n/a      | mongodb  | public       |
    When each peer broadcasts its CapacityAnnouncement
    Then every peer's neighbor table contains entries for all other peers
    And the profiles are distinguishable by capability sets
    And no peer is assumed to have the same profile as another

  @wip @requires:multi-node
  Scenario: Heterogeneous network handles mixed availability
    Given "timothy-laptop" goes offline (intermittent)
    And "matthew-home" remains online (always-on)
    And "community-node" remains online (always-on)
    When "matthew-home" evaluates the neighbor table
    Then "matthew-home" sees 1 stale peer and 2 active peers
    And routing decisions exclude the stale laptop peer
    And no degradation occurs because always-on peers remain
