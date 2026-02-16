@epic:elohim-p2p-infrastructure @category:deployment @status:implemented
Feature: P2P Peer Validation
  As a deployment pipeline
  I want to validate that P2P peers are connected and syncing
  So that I can verify the distributed data layer is operational

  Scenario: Doorway reports connected P2P peers
    Given the doorway health endpoint is accessible
    When I check the P2P status
    Then connected_peers should be greater than 0

  Scenario: P2P sync documents are tracked
    Given the doorway health endpoint is accessible
    When I check the P2P status
    Then sync_documents count should be available
