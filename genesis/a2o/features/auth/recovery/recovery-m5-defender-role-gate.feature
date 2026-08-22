@recovery-m5 @defender-stub @act:i @e2e
Feature: submit_specialist_revocation gated by local defender role marker
  As an elohim-agent that a human designated as one of their recovery defenders
  I want submit_specialist_revocation to verify my defender role
  So that only the defenders the human chose can strip a compromised key —
  never an arbitrary agent claiming the power

  When a human's device key is compromised, their designated defenders — the
  peers the human chose in advance to guard their recovery — attest a
  specialist revocation that strips the compromised key. Recovery milestone
  M4 established the structural quorum: revocation takes agreement from the
  human's defender circle, not any one actor. This milestone (M5) adds the
  local half of that promise: an agent that is NOT in the human's
  DefenderManifest (the human's own declaration of who defends them) is
  refused at the coordinator, so an attacker who compromises one agent
  cannot impersonate a defender and vote a key away. Without this gate the
  M4 quorum counts votes from anyone — which is no quorum at all; the human
  consequence would be losing keys, and with them years of identity and
  household state, to whoever asks loudest.

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
  #
  # DEFERRED blind-reader findings (2026-08-22, rounds 1-2; BLOCKERs cleared,
  # remaining MAJORs deferred until the bridge lands because their repairs
  # rework @wip steps that cannot be verified yet):
  # - Acceptance scenario must prove the GATE was the deciding factor:
  #   identical inputs to the rejection scenario, differing only in the
  #   DefenderManifest — plus an assertion tying acceptance to the check.
  # - Introduce the compromised human by name in a Given (the "target
  #   human" currently has no antecedent).
  # - Resolve the manifest direction: it lists which humans THIS agent
  #   defends — make the Given text say so.
  # - Assert revokedKey VALUE matches the compromised key, not mere
  #   field presence.

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
