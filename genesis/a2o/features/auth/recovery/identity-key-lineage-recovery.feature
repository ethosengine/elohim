@wip @auth @recovery @identity-lineage @requires:household-nodes
Feature: A recovered identity keeps its contribution, standing, and claims through the chain-root

  As a person who has lost the key to my identity
  I want the community that stands with me to authorize a new key
  So that everything I have contributed still recognizes me afterward

  The identity is a lineage DAG, not a single fragile key. Its durable name is
  the chain-root cid (the version_parent=[] genesis node), stable across every
  rotation and recovery. Contributor attribution, REA economic standing, and
  presence claims all point at the chain-root — never at a point-in-time key —
  so a key rotation authorized by the community-recovery quorum leaves every
  downstream recognition intact. The recovery quorum is a *controller* of the
  identity head, not an override bolted on (structural imago-dei): the human's
  controller-policy names the community authority in the same field that names
  self.

  # ─────────────────────────────────────────────────────────────────────────
  # WAVE-D ACCEPTANCE GATE — RED ON PURPOSE.
  #
  # This scenario is authored FIRST (story-first) as the culminating proof for
  # the identity-head / agent-key-lineage arc:
  #   genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
  #
  # It FAILS today and is EXPECTED to fail until Wave D. No rotation path exists
  # yet (Wave B builds `binds-identity` + `rotate_identity_key`; Wave C wires
  # resolution + binding). Wave A installs only the thin chain-root SEAM and the
  # re-pointings that target it — the plumbing this scenario will eventually walk.
  #
  # DO NOT implement steps to make this pass as part of Wave A. It is the
  # acceptance criterion for the WHOLE arc, and it goes GREEN in Wave D (D1),
  # end-to-end, once the community-recovery quorum can authorize a real
  # `rotate_identity_key`. Keep the `@wip` tag until then.
  #
  # STATUS 2026-07-18 (arc partially shipped; stays @wip by operator decision):
  #   PROVEN & BANKED — chain-root over the lineage DAG + root-stability
  #   (Wave B, pure-logic + sweettest); `binds-identity` declaration + validator
  #   (Wave B); the AUTHORIZATION-REFUSAL path below = scenario 2, proven at the
  #   sweettest layer (unauthorized rotate refused pre-write); did:elohim resolves
  #   real controllers + lineage, DHT-canonical head selection (Wave C1); the
  #   attribution / REA / claim re-pointings at the chain-root (Wave A).
  #   DEFERRED (why full end-to-end is still RED) — (1) no coordinator path mints
  #   a valid KeyRotation, so scenario 1's "a KeyRotation version node is appended"
  #   can't run end-to-end: backlog keyrotation-mint-path-witness-backed
  #   (CryptographicQuorum variant = the narrower unblock); (2) transport-id
  #   binding is signature-deferred: backlog agent-peer-binding-signing (C2).
  #   D1 flips @wip→green once the mint path lands.
  # ─────────────────────────────────────────────────────────────────────────

  Background:
    Given the household topology is running as a live multi-peer mesh
    And Grandma has a graduated identity whose current key is "key-original"
    And Grandma's identity head declares a community-recovery quorum as a controller
    And Grandma has contributed content that established a claimed contributor presence
    And Grandma holds active REA economic standing as a party to a commitment

  Scenario: A community-authorized key rotation preserves attribution, standing, and claims
    Given Grandma has lost access to "key-original"
    When the community-recovery quorum authorizes a rotation to a new key "key-recovered"
    Then a KeyRotation version node is appended to Grandma's identity lineage
    And the chain-root cid of Grandma's identity is unchanged by the rotation
    And Grandma's current head key resolves to "key-recovered"
    And Grandma's claimed contributor presence still resolves through the chain-root
    And Grandma's contributor attribution and recognition score are unbroken
    And Grandma's REA economic standing still resolves her as the same party through the chain-root
    And no contribution, claim, or standing is orphaned by the key change

  Scenario: An un-authorized key rotation cannot capture the identity
    Given Grandma has lost access to "key-original"
    When an actor who is NOT a controller attempts a rotation to a key "key-attacker"
    Then the rotation is refused
    And the chain-root cid of Grandma's identity is unchanged
    And Grandma's current head key does not resolve to "key-attacker"
