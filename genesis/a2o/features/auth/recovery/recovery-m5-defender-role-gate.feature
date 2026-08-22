@recovery-m5 @defender-stub @act:i @e2e
Feature: submit_specialist_revocation gated by local defender role marker
  As an elohim-agent acting on a human's behalf
  I want submit_specialist_revocation to verify my defender role
  So that the structural quorum gate from M4 retains its meaning

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ─────────────────────────────────────────────────────────
  # Stubbed Ok(false) in M5 — rejection path works naturally
  # ─────────────────────────────────────────────────────────
  # @wip: both scenarios drive POST /api/v1/account/specialist-revocation,
  # an HTTP bridge route no service serves yet (doorway proxies to
  # elohim-storage, which answers "Unknown account route"). The imagodei
  # coordinator zome function exists; the Phase-11 bridge wiring
  # (imagodei → elohim-agent) is what these scenarios wait on. They shed
  # @wip when that route lands and go truly RED→green.

  @wip
  Scenario: Without role marker — coordinator rejects
    Given the calling elohim-agent has no DefenderManifest configured
    When I call submit_specialist_revocation with a valid SubmitSpecialistRevocationInput
    Then the call returns an error mentioning "not a configured defender"

  # ─────────────────────────────────────────────────────────
  # Success path — gated on Phase-11 bridge (imagodei → elohim-agent wiring)
  # ─────────────────────────────────────────────────────────

  @wip
  Scenario: With role marker — coordinator accepts
    Given the calling elohim-agent has a DefenderManifest listing the target human
    When I call submit_specialist_revocation with a valid SubmitSpecialistRevocationInput
    Then the call returns an ActionHash
    And a KeyRevocation entry is committed with triggerType "specialist_attestation"
    And the KeyRevocation entry contains a revokedKey field
    And the KeyRevocation entry contains a createdAt field
