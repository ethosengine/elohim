@wip @auth @recovery @identity-lineage @requires:household-nodes @act:i @e2e
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

  # Added 2026-10-20: the two scenarios below cover the identity row of the placement-contract
  # design (genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md).
  @wip @concern:identity-lineage-recovery
  Scenario: Grandma has lost every device she had, and a new tablet opens none of her documents but tells her how to ask her quorum to stand for her
    The chain-root above is the network's name for Grandma, her identity root, and the network
    holds it, not any one device. Its cid is its content identifier: a name computed from the
    first node of her identity lineage, so it never changes when a key does. A device is enrolled
    under it when it holds the current key of her identity head, the key her lineage names as
    acting for her now: that is "key-original" from the Background, and her phone and her laptop
    both hold it. The other peers of the mesh hold copies of the
    documents she keeps, which is how her data outlives her devices. Copies surviving is not
    the same as getting back in: without a device enrolled under her chain-root, she cannot
    open them, and nobody else can open them for her. This scenario is written ahead of its
    implementation and is expected to fail until the app can tell an enrolled device from one
    that is not.
    Given Grandma's phone and her laptop are the only devices enrolled under her chain-root
    And Grandma's phone and her laptop both hold "key-original", the current key of her identity head
    And the other peers of the mesh hold copies of the documents Grandma keeps
    When Grandma loses her phone and her laptop
    And with them Grandma has lost access to "key-original"
    And Grandma starts the app on a new tablet that has never been enrolled under her chain-root
    Then the app on the tablet opens none of her documents, because the tablet is not enrolled under her chain-root, although the other peers of the mesh still hold copies of them
    And the app on the tablet tells her how to ask her community-recovery quorum to stand for her

  @wip @concern:identity-lineage-recovery
  Scenario: Grandma's quorum enrolls her new tablet, and the app on it opens what was hers, as herself
    This continues the scenario above: Grandma has lost her phone and her laptop, and the app on
    her new tablet has opened nothing yet. Enrolling the tablet is a key rotation, the same kind
    the first scenario walks: her community-recovery quorum authorizes a new key, "key-tablet",
    held on the tablet, as the new current key of her identity head in place of "key-original",
    the key the lost phone and laptop held. That
    needs the KeyRotation mint path, which is not built yet, so this scenario is written ahead of
    it and is expected to fail until it lands. Her claimed contributor presence is the network's
    record that the content she contributed is
    hers, claimed under her chain-root rather than under any one key; it is what has to keep
    recognizing her after the rotation. Each copy the peers hold is named by a fingerprint
    computed from its contents, so a copy that anyone has altered no longer matches its name;
    that is how the tablet knows that what it fetches from other peers is what she kept.
    Given Grandma's phone and her laptop are the only devices enrolled under her chain-root
    And Grandma's phone and her laptop both hold "key-original", the current key of her identity head
    And the other peers of the mesh hold copies of the documents Grandma keeps
    And each copy the peers hold is named by a fingerprint computed from its contents
    And Grandma has lost her phone and her laptop, and with them her access to "key-original"
    And Grandma has started the app on a new tablet that has never been enrolled under her chain-root
    And the app on the tablet has opened none of her documents, because the tablet is not enrolled under her chain-root
    When the community-recovery quorum authorizes a rotation to a new key "key-tablet" held on the new tablet
    Then a KeyRotation version node is appended to Grandma's identity lineage
    And the chain-root cid of Grandma's identity is unchanged
    And Grandma's current head key resolves to "key-tablet"
    And the new tablet is enrolled under Grandma's chain-root
    And the phone and the laptop are no longer enrolled, so anything newly signed with "key-original" is refused
    And the app on the tablet opens Grandma's documents, fetched from the other peers of the mesh, each checked against the fingerprint that names it
    And each document reads the same as it did before she lost her devices
    And Grandma's claimed contributor presence still resolves through the chain-root
