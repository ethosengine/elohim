@recovery-m5 @defender-stub
Feature: submit_specialist_revocation gated by local defender role marker
  As an elohim-agent acting on a human's behalf
  I want submit_specialist_revocation to verify my defender role
  So that the structural quorum gate from M4 retains its meaning

  # ─────────────────────────────────────────────────────────
  # Stubbed Ok(false) in M5 — rejection path works naturally
  # ─────────────────────────────────────────────────────────

  Scenario: Without role marker — coordinator rejects
    Given the calling elohim-agent has no DefenderManifest configured
    When I call submit_specialist_revocation with a valid SubmitSpecialistRevocationInput
    Then the call returns an error mentioning "not a configured defender"

  # ─────────────────────────────────────────────────────────
  # Success path — gated on Phase-11 bridge (imagodei → elohim-agent wiring)
  # ─────────────────────────────────────────────────────────

  @phase11-pending
  Scenario: With role marker — coordinator accepts
    Given the calling elohim-agent has a DefenderManifest listing the target human
    When I call submit_specialist_revocation with a valid SubmitSpecialistRevocationInput
    Then the call returns an ActionHash
    And a KeyRevocation entry is committed with triggerType "specialist_attestation"
    And the KeyRevocation entry contains a revokedKey field
    And the KeyRevocation entry contains a createdAt field
