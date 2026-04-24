@stage1-structural @recovery-m3
Feature: Freeze-Floor Gate Blocks Intimate-Layer Rotation

  When an elohim defender (or, pre-M5, a manually committed IdentityFreeze)
  has flagged the intimate recovery layer as frozen for a human, the
  coordinator pre-commit gate on commit_key_rotation must reject an
  IntimateQuorum rotation even when enough witnesses are present. A
  CryptographicQuorum rotation on the same human must still pass — the
  cryptographic layer is orthogonal per design §1.1.

  Stage-1 structural acceptance: the gate runs structurally even though
  M5 is where defenders author freezes in volume. We exercise the gate by
  committing a freeze directly in the test fixture.

  Background:
    Given the shem topology is running
    And Abby has 3 emergency contacts (Ben, Cara, Dan) with emergency_access_enabled = true
    And an open RecoveryRequest exists for Abby with required_witness_count equal to 3
    And Ben, Cara, Dan have submitted intimate witnesses so count equals threshold

  Scenario: An active intimate freeze blocks the IntimateQuorum rotation
    Given an active IdentityFreeze targeting Abby at the "intimate" layer
    When Abby invokes commit_key_rotation with IntimateQuorum carrying witness_hashes for ben, cara, and dan
    Then the coordinator pre-commit gate rejects with a freeze-floor error
    And no KeyRotation entry is committed

  Scenario: A CryptographicQuorum rotation is exempt from the freeze-floor gate
    Given an active IdentityFreeze targeting Abby at the "intimate" layer
    And Abby has a valid KeyStewardship with a threshold-reached quorum signature
    When Abby invokes commit_key_rotation with CryptographicQuorum carrying stewardship_hash for abby's stewardship
    Then the rotation succeeds
    And the storage projection records the KeyRotation
