Feature: EPR Cross-Peer Content Resolution
  As a learner navigating a path
  I want content stewarded by another peer to resolve transparently
  So that learning paths work regardless of stewardship partitioning

  Background:
    Given the EPR protocol "/elohim/epr/1.0.0" is active between peers
    And the shard protocol "/elohim/shard/1.0.0" is active between peers

  Scenario: Content stewarded by another peer resolves with full body
    Given peer "alpha" has content "fct-module-01-church-dilemma" stewarded by Pete
    And peer "staging" does not have "fct-module-01-church-dilemma" locally
    When peer "staging" requests content "fct-module-01-church-dilemma"
    Then the content is resolved via EPR protocol from peer "alpha"
    And the content body is fetched via shard protocol
    And the content is persisted to local SQLite on peer "staging"
    And subsequent requests return the content without P2P resolution

  Scenario: EPR Heads publish to DHT on ingestion
    Given peer "alpha" ingests content "test-concept"
    Then the DHT contains an EPR Head for "test-concept"
    And peer "staging" can discover "test-concept" via Kademlia lookup

  Scenario: Content GET returns 404 when no peer has the content
    Given no peer has content "nonexistent-concept"
    When peer "alpha" requests content "nonexistent-concept"
    Then the response is 404 Not Found

  Scenario: Single content create publishes EPR Head
    Given peer "alpha" creates content "new-concept" via POST /db/content
    Then the DHT contains an EPR Head for "new-concept"

  Scenario: P2P-resolved content is tagged for diagnostics
    Given peer "staging" resolves "cross-steward-concept" via P2P
    Then the local content record has metadata "resolved_via" = "p2p"

  @wip
  Scenario: Community reach content accessible only to collective members
    Given peer "alpha" has content "community-governance-guide" with reach "community"
    And human "Matthew" is a consented member of collective "local-church"
    And human "Frank" has no collective memberships
    When human "Matthew" requests content "community-governance-guide" from peer "alpha"
    Then the content is served successfully
    When human "Frank" requests content "community-governance-guide" from peer "alpha"
    Then the response is 403 with reason "No consented collective membership"

  @wip
  Scenario: Trusted reach content requires relationship with steward
    Given peer "alpha" has content "advanced-theology" with reach "trusted"
    And human "Pete" is a steward of "advanced-theology"
    And human "Matthew" has a "trusted" relationship with human "Pete"
    And human "Frank" has no relationships with any steward of "advanced-theology"
    When human "Matthew" requests content "advanced-theology" from peer "alpha"
    Then the content is served successfully
    When human "Frank" requests content "advanced-theology" from peer "alpha"
    Then the response is 403 with reason "No trusted relationship with content steward"

  @wip
  Scenario: Attestation-gated content requires prerequisite mastery
    Given peer "alpha" has content "calculus-201" with reach "public"
    And content "calculus-201" requires prerequisite mastery of "calculus-101"
    And human "Matthew" has mastery of "calculus-101"
    And human "Timothy" does not have mastery of "calculus-101"
    When human "Matthew" requests the body of content "calculus-201"
    Then the content body is served successfully
    When human "Timothy" requests the body of content "calculus-201"
    Then the response is 403 with reason "Prerequisite mastery required"

  @wip
  Scenario: Recognition distributes proportionally to stewards on P2P delivery
    Given peer "alpha" has content "economics-primer" stewarded by "Pete" at 60% and "Timothy" at 40%
    When peer "staging" resolves "economics-primer" via P2P from peer "alpha"
    Then recognition events are created for steward "Pete" and steward "Timothy"
    And steward "Pete" receives approximately 60% of the recognition
    And steward "Timothy" receives approximately 40% of the recognition

  @wip
  Scenario: Policy ceiling blocks content above device reach level max
    Given peer "alpha" has content "intimate-journal" with reach "intimate"
    And human "Timothy" has a device policy with reach_level_max of 3
    When human "Timothy" requests content "intimate-journal" from peer "alpha"
    Then the response is 403 with reason matching "Reach level .* exceeds maximum"
