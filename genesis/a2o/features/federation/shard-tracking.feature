@e2e @federation
Feature: Shard Tracking for Content Auto-Recovery
  As a steward participating in the Elohim network
  I want my doorway to track which nodes custody which shards of my content
  So that when I lose my device, I can reconstruct my content through social recovery

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_BETA"
    And doorway "gamma" at "E2E_DOORWAY_GAMMA"

  Scenario: Publishing content creates traceable shard assignments across the network
    Given human "Timothy" is logged in on doorway "alpha" with device "phone"
    When "Timothy" creates a new pathway titled "My Life Story"
    Then doorway "alpha" should encode the content into Reed-Solomon shards
    And doorway "alpha" should register 7 ShardAssignments for the content in the node-registry
    And the node-registry on doorway "beta" should see the ShardAssignments for "Timothy"'s content

  @wip
  Scenario: Invalid shard index is rejected by the DHT
    Given human "Timothy" is logged in on doorway "alpha" with device "phone"
    When doorway "alpha" attempts to register a ShardAssignment with shard_index 99
    Then the node-registry should reject the assignment with "shard_index 99 exceeds maximum 6"

  @wip
  Scenario: Querying shard assignments by custodian
    Given human "Timothy" is logged in on doorway "alpha" with device "phone"
    And "Timothy" has published content with 7 shard assignments
    When doorway "beta" queries shard assignments for doorway "alpha"'s custodian DID
    Then doorway "beta" should see 7 shard assignments for that custodian
