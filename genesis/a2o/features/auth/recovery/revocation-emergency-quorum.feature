@recovery-m4 @emergency-contact-quorum
Feature: Emergency Contacts Kill a Captured Key by Quorum

  When a person's only key is captured by an attacker, that person cannot
  revoke it themselves — they have no trusted device left. Their emergency
  contacts can step in and vote to kill the key. When enough of them agree,
  the network revokes the key and the attacker is cut off.

  This is community-level defense. The protocol does not require a majority
  of all participants — only a threshold of the target person's designated
  emergency contacts. The threshold is calculated as: max(2, ceil(N/2)+1)
  where N is the number of active emergency contacts.

  Background:
    Given Matthew has one active agent key "K1"
    And Matthew has four active emergency contacts:
      | Contact  | Registered via     |
      | Jessica  | doorway-alpha      |
      | Timothy  | doorway-alpha      |
      | David    | doorway-beta       |
      | Sarah    | doorway-beta       |
    And the required vote threshold for Matthew is 3 (out of 4 contacts)

  # ─────────────────────────────────────────────────────────
  # Happy path — quorum reached, key killed
  # ─────────────────────────────────────────────────────────

  Scenario: Jessica initiates a revocation request for Matthew's captured key
    Given an attacker has captured "K1" and is acting as Matthew
    When Jessica opens the emergency panel in her app
    And Jessica selects "A contact's key may be compromised"
    And Jessica chooses Matthew from her emergency contacts
    And Jessica submits a revocation request for "K1" with reason "key compromised"
    Then a pending revocation record is created for "K1"
    And the revocation shows "0 of 3 votes received"
    And Matthew's emergency contacts receive a notification about the pending revocation

  Scenario: Three emergency contacts approve and the key is revoked
    Given a pending revocation request for Matthew's key "K1" with required votes 3
    When Jessica, David, and Sarah each approve the revocation request
    Then the revocation reaches the required threshold of 3 votes
    And Matthew's key "K1" is immediately revoked across the network
    And any future action signed by "K1" is rejected
    And Matthew's emergency contacts see: "Matthew's compromised key has been revoked"
    And Matthew can subsequently start a full recovery (M3) to obtain a new key

  Scenario: Matthew's profile and history remain accessible after revocation
    Given Matthew's key "K1" has been revoked by emergency contact quorum
    When anyone looks up Matthew's profile
    Then his identity, learning history, and relationships are still visible
    And his emergency contacts can still help him through the M3 recovery flow

  # ─────────────────────────────────────────────────────────
  # Threshold not met — revocation stays pending
  # ─────────────────────────────────────────────────────────

  Scenario: Only two contacts respond — revocation stays pending
    Given a pending revocation request for Matthew's key "K1" with required votes 3
    When only Jessica and David approve the revocation request
    And Timothy and Sarah have not responded
    Then the revocation record shows "2 of 3 votes received"
    And the revocation is still pending — the key has not been revoked
    And Matthew's key "K1" is still accepted by the network
    And the pending revocation remains open for additional contacts to vote

  Scenario: A vote can be submitted later to reach threshold
    Given a pending revocation with 2 of 3 votes received
    When Sarah approves the revocation request
    Then the threshold of 3 is reached
    And Matthew's key "K1" is revoked immediately upon Sarah's vote

  # ─────────────────────────────────────────────────────────
  # Anti-rotation gate — P2: revocation blocks key rotation
  # ─────────────────────────────────────────────────────────

  Scenario: An attacker cannot rotate the captured key while a revocation is pending
    Given a pending revocation request for Matthew's key "K1"
    And an attacker holds "K1" and attempts to rotate it to a new key "K_attacker"
    When the attacker attempts to commit a key rotation from "K1" to "K_attacker"
    Then the rotation is rejected with a "pending revocation" error
    And "K_attacker" is not recognized as Matthew's current key
    And Matthew's emergency contacts' revocation vote can still proceed normally

  Scenario: An attacker cannot rotate an already-revoked key
    Given Matthew's key "K1" has been revoked by emergency contact quorum
    When the attacker attempts to commit a key rotation from "K1" to "K_attacker"
    Then the rotation is rejected with an "effective revocation" error
    And the attacker cannot establish a new key as Matthew through this path

  # ─────────────────────────────────────────────────────────
  # Eligibility gate — only designated emergency contacts can vote
  # ─────────────────────────────────────────────────────────

  Scenario: A person who is not Matthew's emergency contact cannot initiate a revocation
    Given a person named "Stranger" who has no emergency-contact relationship with Matthew
    When Stranger attempts to file a revocation request for Matthew's key "K1"
    Then the request is rejected with an "not an active emergency contact" error
    And no revocation record is created for Matthew's key

  Scenario: A contact who already voted cannot vote again
    Given a pending revocation request for Matthew's key "K1"
    And Jessica has already submitted an approved vote
    When Jessica attempts to submit a second vote on the same revocation
    Then the second vote is rejected with an "already voted" error
    And the vote count remains unchanged at its previous value
