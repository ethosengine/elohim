@e2e @auth @device-consent-grant @requires:doorway @act:i @wip
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

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And Matthew has an account that doorway "alpha" hosts for him
    And a device "workspace" with its own node that is not yet enrolled
    And Jessica, another person on the network, runs her own node, which doorway "alpha" does not host
    And a one-time code is good for five minutes

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

  Scenario: A code left unused expires
    Given Matthew has approved the device "workspace" and the portal has shown a code
    When the terminal on device "workspace" presents that code six minutes later
    Then the doorway refuses with code "redemption_expired"
    And the device "workspace" is not enrolled

  Scenario: A request for something the doorway does not recognise never reaches Matthew
    When the terminal on device "workspace" asks doorway "alpha" for "device.admin"
    Then the doorway refuses with code "act_unknown"
    And the terminal prints no portal link
