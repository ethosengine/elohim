@stage1-structural @recovery-m3
Feature: Intimate Quorum Happy Path — A Lost Pubkey Is Restored Through Emergency Contacts

  The claimant Abby has lost access to her agent key. She has three emergency
  contacts (Ben, Cara, Dan) set up ambiently via HumanRelationship entries
  with emergency_access_enabled = true. Under the revised Phase 2 design, no
  ceremonial seed setup exists — the relationships ARE the recovery fabric.

  At least one emergency contact's household hosts them on a *different*
  doorway-steward from Abby's. This exercises cross-doorway invitation
  fan-out on the libp2p mesh: the gossipsub publish from Abby's colocated
  elohim-storage must reach Ben's / Cara's / Dan's elohim-storage pods too.

  Stage-1 structural acceptance: humans tap confirm. Elohim-specialist
  attestation lands in M5.

  Background:
    Given the shem topology is running with at least two doorway-stewards
    And Abby is registered via "doorway-alpha"
    And Ben is registered via "doorway-beta"
    And Cara and Dan are registered via "doorway-alpha"
    And each of Ben, Cara, Dan has a HumanRelationship to Abby with emergency_access_enabled = true

  Scenario: A recovery request reaches Abby's required_witness_count of 3
    When Abby invokes create_recovery_request from a fresh agent key
    Then the returned request has human_id equal to Abby's human_id
    And the returned request has required_witness_count equal to 3
    And a RecoveryInvitation is published on the "recovery.invitation" topic
    And the elohim-storage pod colocated with doorway-beta logs "received invitation"

  Scenario: Three witnesses satisfy the threshold and rotation succeeds
    Given an open recovery request for Abby with required_witness_count equal to 3
    When Ben, Cara, and Dan each submit_intimate_witness on the request
    Then the recovery_witnesses projection for the request has count 3
    When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for ben, cara, and dan
    Then the rotation succeeds
    And the storage projection records the KeyRotation

  Scenario: Two witnesses fall short of the threshold and rotation is rejected
    Given an open recovery request for Abby with required_witness_count equal to 3
    And Ben and Cara have submitted intimate witnesses
    When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for ben and cara only
    Then the rotation is rejected by the M2 validator
    And the recovery_witnesses projection retains count 2

  Scenario: A non-contact cannot submit a witness
    Given an open recovery request for Abby
    And agent Evan has no HumanRelationship to Abby
    When Evan invokes submit_intimate_witness on the request
    Then the coordinator pre-commit gate rejects with an emergency-contact-membership error
    And no HumanityWitness entry is committed for Evan on this request
