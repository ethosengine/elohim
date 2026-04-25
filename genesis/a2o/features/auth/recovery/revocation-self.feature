@recovery-m4 @self-revocation
Feature: Self-Revocation of a Stolen Device Key

  A person who still controls at least one trusted device can immediately
  kill a stolen or compromised device's key from the device they still hold.
  No community vote is needed — it is their own key, and the protocol trusts
  them to act on it.

  The revoked key is rejected by the network from the moment the revocation
  is written to the DHT. The person's other devices and identity remain
  completely unaffected. This is the "stolen phone kill switch."

  Background:
    Given Matthew has two registered devices: "laptop" and "phone"
    And each device holds a separate agent key: "K_laptop" and "K_phone"
    And both keys are linked to Matthew's human identity in the network

  Scenario: Matthew kills his stolen phone's key from his laptop
    Given Matthew's phone is stolen
    When Matthew opens the security panel on his laptop
    And Matthew selects "I lost access to a device — revoke its key"
    And Matthew chooses "phone" from the list of his registered devices
    And Matthew confirms the revocation with reason "device stolen or lost"
    Then the phone's key "K_phone" is immediately revoked in the network
    And the revocation is marked effective at the moment Matthew confirmed
    And Matthew's laptop key "K_laptop" remains valid and unaffected
    And any future action signed by "K_phone" is rejected by the network
    And Matthew sees a confirmation: "Your phone's key has been revoked"

  Scenario: The revoked key cannot sign new actions after revocation
    Given Matthew has already revoked "K_phone" from his laptop
    When something attempts to act as Matthew using "K_phone"
    Then the network rejects the action with a revocation error
    And no entry is written to the DHT on behalf of that action

  Scenario: Matthew's other devices and relationships are unaffected
    Given Matthew has revoked "K_phone"
    When Matthew's emergency contacts check his profile
    Then they can still see Matthew's identity and relationships
    And Matthew's attestations and learning history are intact
    And Matthew can still act normally from his laptop using "K_laptop"

  Scenario: Matthew cannot accidentally revoke his only remaining trusted key
    Given Matthew has only one registered device: "laptop" with key "K_laptop"
    When Matthew attempts to revoke "K_laptop" using "K_laptop" itself
    Then the network rejects the request with a self-revocation error
    And Matthew sees a message explaining that he cannot revoke his only trusted key
    And "K_laptop" remains valid

  @wip
  Scenario: Matthew can initiate full recovery after revoking his only key
    Given Matthew has revoked all of his keys
    And Matthew is effectively locked out of his identity
    When Matthew's emergency contacts help him through the M3 recovery flow
    Then Matthew obtains a new agent key
    And his identity and history are restored to his new key
