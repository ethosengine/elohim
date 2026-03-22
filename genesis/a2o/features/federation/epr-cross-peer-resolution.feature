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
