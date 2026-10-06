@e2e @auth @device-consent-grant @requires:doorway @requires:device-node @act:i
# The grant is served by the person's own node, not by the doorway; the steps call that node directly over loopback.
# E2E_DEVICE_NODE_URL is the storage URL of a fourth "workspace" node (`just mesh join-peer workspace`); unset, these scenarios are skipped.
Feature: A device asks, and its person approves in a portal
  As Matthew, working on a machine that is not yet mine to act from
  I want to sign in through my doorway's portal and approve that machine there
  So that it can act for me without anyone copying secrets between machines by hand

  A device is a machine running its own node on the network. To act for a
  person it must be enrolled: a record on the network, signed by the person's
  side and by the device, binds the device to that person, and any peer can
  check it.

  The device asks from a terminal, meaning the command-line program Matthew runs
  on it. The terminal never sees Matthew's password. It prints a link to the
  portal, which is the sign-in site his doorway serves. Matthew signs in there,
  reads which device is asking and what it wants to do, and approves. Matthew's
  account is hosted: the doorway runs the node that speaks for him and keeps its
  key. Approving makes that node sign the enrollment. The doorway then issues a
  one-time code.

  The code reaches the terminal in one of two ways. When the browser and the
  terminal are on the same machine, the browser hands the code to the terminal.
  When the device is remote, the portal shows the code and Matthew pastes it into
  the terminal. Only the terminal that asked can use the code.

  Every approval produces a consent record: what the terminal asked for, what
  Matthew agreed to, for which device. The portal shows each thing asked for
  separately, and Matthew may agree to all of them or only some. The device ends
  up with exactly what the record lists, never more than was asked.

  A terminal can ask for two things:
  - Enroll the device, so the network recognizes it as one of Matthew's.
  - Also bind the device's root key. That is a second key, kept on the device
    outside any project, which signs what the device produces. Binding it lets
    anyone trace those bytes back to Matthew instead of to an unknown key. Only
    an enrolled device can have it bound.

  Being recognized as Matthew's device does not, by itself, let the device
  change any content. That is decided separately, by whether Matthew has
  standing over the content, meaning the network recognizes his right to
  change it.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And Matthew has an account that doorway "alpha" hosts for him
    And a device "workspace" with its own node that is not yet enrolled
    And Jessica, another person on the network, runs her own node, which doorway "alpha" does not host
    And a one-time code is good for five minutes

  # held: no node route answers whether a device is bound to a person (device-recognition rows 7, 3)
  @wip
  Scenario: Matthew approves a remote device by pasting the code
    When the terminal on device "workspace" asks doorway "alpha" to enroll the device, returning the code by paste
    Then the terminal prints a portal link and waits for a code
    When Matthew opens the link, signs in, and approves the device "workspace"
    Then the portal shows Matthew a code to paste
    When Matthew pastes the code into the terminal
    Then the device "workspace" is enrolled under Matthew's identity
    When Jessica's node is asked whether the device "workspace" is bound to Matthew
    Then Jessica's node answers yes from the network's own record, without calling the doorway
    And the record it read carries a signature from Matthew's hosted node and one from the device

  Scenario: Matthew approves a device on the same machine without pasting
    When the terminal on device "workspace" asks doorway "alpha" to enroll the device, returning the code to the terminal's own listener
    And Matthew opens the link, signs in, and approves the device "workspace"
    Then the portal shows no code to paste
    And the browser hands the code to the terminal
    And the device "workspace" is enrolled under Matthew's identity

  # held: needs the rendered portal; on loopback the node's active session always counts as signed in (device-recognition row 11)
  @wip
  Scenario: Matthew is asked to sign in before he is asked to approve
    Given Matthew is not signed in to the portal
    When the terminal on device "workspace" asks doorway "alpha" to enroll the device, returning the code by paste
    And Matthew opens the link
    Then the portal asks Matthew to sign in
    And the portal shows no consent screen until he has

  Scenario: The consent screen names the device and what it asks for
    When the terminal on device "workspace" asks doorway "alpha" to enroll the device, returning the code by paste
    And Matthew opens the link and signs in
    Then the portal shows the name "workspace" and a short form of the device's key
    And the portal lists "enroll this device" as the only thing being asked

  # held: a root-key binding is recorded only in the consent, not observable on the network (device-recognition row 3)
  @wip
  Scenario: A device also binds its root key when it asks to
    When the terminal on device "workspace" asks doorway "alpha" to enroll the device and bind its root key, returning the code by paste
    And Matthew opens the link and signs in
    Then the portal lists "enroll this device" and "bind this device's root key" as two separate things being asked
    And the portal shows a short form of each key
    When Matthew approves and pastes the code into the terminal
    Then the terminal reports a consent record that lists enrollment and the root key as agreed
    And the device "workspace" is enrolled under Matthew's identity
    And the root key of device "workspace" is bound to Matthew

  # held: a root-key binding is recorded only in the consent, not observable on the network (device-recognition row 3)
  @wip
  Scenario: Matthew agrees to less than the terminal asked for
    When the terminal on device "workspace" asks doorway "alpha" to enroll the device and bind its root key, returning the code by paste
    And Matthew opens the link and signs in
    And Matthew agrees to enrollment and does not agree to binding the root key
    And Matthew pastes the code into the terminal
    Then the terminal reports a consent record that lists enrollment as agreed and the root key as declined
    And the device "workspace" is enrolled under Matthew's identity
    And the root key of device "workspace" is not bound to Matthew

  Scenario: A root key cannot be bound without enrolling the device
    When the terminal on device "workspace" asks doorway "alpha" to bind its root key without enrolling the device
    Then the doorway refuses with code "request_acts_incoherent"
    And the terminal prints no portal link

  # held: no step drives a device publish yet (device-recognition row 21)
  @wip
  Scenario: An enrolled device still cannot change content on that basis alone
    Given the device "workspace" has been enrolled with a code
    And Matthew is the author of the content "garden-notes"
    When the device "workspace" tries to publish an update to "garden-notes"
    Then the update is not accepted as the content's current version
    And the device "workspace" remains enrolled under Matthew's identity

  # held: needs the rendered portal; the node exposes no decline route to call (device-recognition row 11)
  @wip
  Scenario: Declining leaves the device unenrolled
    When the terminal on device "workspace" asks doorway "alpha" to enroll the device, returning the code by paste
    And Matthew opens the link, signs in, and declines
    Then the portal shows no code
    And the device "workspace" is not enrolled

  Scenario: A code seen by someone else is useless to them
    Given Matthew has approved the device "workspace" and the portal has shown a code
    When a terminal on another machine, which did not ask, presents that code to doorway "alpha"
    Then the doorway refuses with code "redemption_verifier_mismatch"
    And the doorway refuses that code from then on, even from the terminal that asked
    And the device "workspace" is not enrolled

  Scenario: A code is used once
    Given the device "workspace" has been enrolled with a code
    When the terminal on device "workspace" presents that code to doorway "alpha" again
    Then the doorway refuses with code "redemption_already_redeemed"

  # held: needs a clock seam on the node or six real minutes of waiting (device-recognition row 21)
  @wip
  Scenario: A code left unused expires
    Given Matthew has approved the device "workspace" and the portal has shown a code
    When the terminal on device "workspace" presents that code six minutes later
    Then the doorway refuses with code "redemption_expired"
    And the device "workspace" is not enrolled

  Scenario: A request for something the doorway does not recognise never reaches Matthew
    When the terminal on device "workspace" asks doorway "alpha" for "content.publish"
    Then the doorway refuses with code "act_unknown"
    And the terminal prints no portal link
