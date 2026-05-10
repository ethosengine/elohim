@e2e @auth @recovery-cross-stack @iroh @phase11-gate5 @wip
Feature: Recovery completes across mixed iroh/libp2p share-holder transports

  Cutover gate #5 (spec 2026-05-08-iroh-libp2p-complementarity.md line 514).
  The recovery flow today is libp2p-canonical. Phase 11 makes recovery topics
  dual-publish (gate #4, permanent post-cutover per spec line 513). These
  scenarios prove that a recovery completes regardless of which transport
  profile each share-holder supports — the share-holder is reached via
  whichever wire it speaks.

  Spec:  genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md line 514
  Plan:  genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md
  Rust:  elohim/elohim-storage/tests/iroh_recovery_cross_stack.rs

  Wire-interaction contract (locked by plan Task 2):
    - Send invitation:   P2PHandle::publish_recovery_invitation (routes through DualGossipPublisher)
    - Receive invitation: gossip subscribe on both stacks (dual-publish permanent per gate #4)
    - Submit witness:    imagodei zome fn submit_intimate_witness (Holochain Track 1)
    - Commit rotation:  imagodei zome fn commit_key_rotation (Holochain Track 1)
    - Send revocation:  P2PHandle::publish_recovery_revocation (routes through DualGossipPublisher)
    - Per-peer transport selection: PeerTransportManifest (Plan 1, src/p2p_iroh/peer_map.rs)

  Background:
    Given a 5-node cross-stack fixture is running with these share-holder profiles:
      | name | transport |
      | Ben  | iroh      |
      | Cara | iroh      |
      | Dan  | libp2p    |
      | Evan | libp2p    |
      | Faye | dual      |
    And Abby is registered with required_witness_count of 3
    And each of Ben, Cara, Dan, Evan, Faye has a HumanRelationship to Abby with emergency_access_enabled = true

  Scenario: Recovery completes when all share-holders speak iroh
    Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Cara", "Faye"]
    When Abby invokes create_recovery_request from a fresh agent key
    And each share-holder in ["Ben", "Cara", "Faye"] receives the recovery.invitation gossip
    And each share-holder in ["Ben", "Cara", "Faye"] submits submit_intimate_witness
    Then the recovery_witnesses projection for the request has count 3
    When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Cara, Faye
    Then the rotation succeeds
    And every share-holder receipt was tagged transport=iroh in the recovery::transport debug log

  Scenario: Recovery completes when all share-holders speak libp2p
    Given Abby's required_witness_count is satisfied by share-holders ["Dan", "Evan", "Faye"]
    When Abby invokes create_recovery_request from a fresh agent key
    And each share-holder in ["Dan", "Evan", "Faye"] receives the recovery.invitation gossip
    And each share-holder in ["Dan", "Evan", "Faye"] submits submit_intimate_witness
    Then the recovery_witnesses projection for the request has count 3
    When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Dan, Evan, Faye
    Then the rotation succeeds
    And every share-holder receipt was tagged transport=libp2p in the recovery::transport debug log

  Scenario: Recovery completes when share-holders are mixed (some iroh, some libp2p)
    Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Dan", "Faye"]
    When Abby invokes create_recovery_request from a fresh agent key
    And each share-holder in ["Ben", "Dan", "Faye"] receives the recovery.invitation gossip
    And each share-holder in ["Ben", "Dan", "Faye"] submits submit_intimate_witness
    Then the recovery_witnesses projection for the request has count 3
    When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Dan, Faye
    Then the rotation succeeds
    And the recovery::transport debug log contains at least one transport=iroh receipt
    And the recovery::transport debug log contains at least one transport=libp2p receipt

  Scenario: Recovery proceeds when one share-holder is offline
    Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Dan", "Faye"]
    And share-holder Cara is offline
    When Abby invokes create_recovery_request from a fresh agent key
    And share-holders ["Ben", "Dan", "Faye"] each receive the recovery.invitation gossip
    And share-holder Cara does NOT receive the recovery.invitation gossip
    And share-holders ["Ben", "Dan", "Faye"] each submit submit_intimate_witness
    Then the recovery_witnesses projection for the request has count 3
    When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Dan, Faye
    Then the rotation succeeds (k-of-n threshold met without Cara)

  Scenario: Recovery is rejected when a share-holder revokes
    Given Abby's required_witness_count is satisfied by share-holders ["Ben", "Dan", "Faye"]
    And Ben, Dan, Faye have each submitted submit_intimate_witness
    When share-holder Ben publishes a RecoveryRevocationMessage on the recovery.revocation topic
    Then the recovery_witnesses projection for the request reflects Ben's revocation
    And Abby's commit_key_rotation with IntimateQuorum carrying witness_hashes for Ben, Dan, Faye is rejected by the M2 validator
    And the rejection reason references the revoked HumanityWitness rule
