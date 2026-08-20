@e2e @federation
Feature: Shard Tracking for Content Auto-Recovery
  As a steward participating in the Elohim network
  I want my doorway to track which nodes custody which shards of my content
  So that when I lose my device, I can reconstruct my content through social recovery

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_BETA"
    And doorway "gamma" at "E2E_DOORWAY_GAMMA"

  # @wip — this scenario's five steps have no definitions, and neither of the two
  # things it needs is step-wiring work:
  #   1. A THIRD doorway. The Background asks for "gamma" via E2E_DOORWAY_GAMMA, and
  #      this fleet deploys two doorways (alpha, beta) — CI sets neither BETA nor
  #      GAMMA (genesis/scripts/ci/e2e-verify-api.sh exports ALPHA and STAGING only),
  #      so the Background dies on "beta" before the scenario body is reached. Its two
  #      siblings below are already @wip for the same reason.
  #   2. The cure itself: publishing does not yet Reed-Solomon-encode content, and no
  #      node-registry ShardAssignment is written or queryable across doorways.
  # Left untagged it read as coverage while contributing nothing — an undefined step
  # scores zero in the gate but appears in the feature as if it were exercised.
  # Drop @wip when a third doorway is deployed AND publish registers ShardAssignments
  # in the node-registry that a second doorway can read back.
  @wip @requires:shem
  Scenario: Publishing content creates traceable shard assignments across the network
    Given human "Terrance" is logged in on doorway "alpha" with device "phone"
    When "Terrance" creates a new pathway titled "My Life Story"
    Then doorway "alpha" should encode the content into Reed-Solomon shards
    And doorway "alpha" should register 7 ShardAssignments for the content in the node-registry
    And the node-registry on doorway "beta" should see the ShardAssignments for "Terrance"'s content

  @wip @requires:shem
  Scenario: Invalid shard index is rejected by the DHT
    Given human "Terrance" is logged in on doorway "alpha" with device "phone"
    When doorway "alpha" attempts to register a ShardAssignment with shard_index 99
    Then the node-registry should reject the assignment with "shard_index 99 exceeds maximum 6"

  @wip @requires:shem
  Scenario: Querying shard assignments by custodian
    Given human "Terrance" is logged in on doorway "alpha" with device "phone"
    And "Terrance" has published content with 7 shard assignments
    When doorway "beta" queries shard assignments for doorway "alpha"'s custodian DID
    Then doorway "beta" should see 7 shard assignments for that custodian
