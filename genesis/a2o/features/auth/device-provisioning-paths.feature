@e2e @auth @device-provisioning @act:i @wip
Feature: A person's identity starts on one node and grows to others
  As Matthew, setting up the machines I work from
  I want my identity to begin on whichever node I have first and spread to my others one approval at a time
  So that I am never locked out by a node that is missing, and never need a host in order to begin

  A node is a machine running the network's software with its own key. Every
  node in this story is on the same network, and what one node records there
  the others can read. A person's identity is a record on the network that
  names which nodes may speak for that person. Those nodes are the identity's
  stewards. The first steward is simply the node the identity was created on.

  A new node joins by asking. Its terminal, meaning the command-line program
  Matthew runs on it, prints a link. Matthew opens the link in a portal. A
  portal is the web page a steward serves, where Matthew signs in, sees what
  his identity rests on, and approves requests like this one. He reads which
  node is asking and approves. The portal then shows a one-time code, and
  Matthew types it into the terminal that asked. The code proves to the steward
  that this terminal is the one that asked, so a link seen by someone else is
  no use to them. The new node then writes a joining record to the network,
  signed by the steward that approved and by itself, that any peer can check.

  One steward's approval is enough. Matthew's other stewards do not have to be
  reachable, and nothing waits for them. When they next see the joining record,
  each adds its own signature to it. This is called affirming. A joining record
  one steward signed is valid; one that more stewards have affirmed is
  stronger. Matthew may instead choose, when he sets up his identity or at any
  time after, that a new node needs the approval of more than one steward.
  That is his choice and is off unless he makes it. When it is on, he opens the
  same link in the portal of each steward that must approve, one after another,
  and the code is shown by the last of them, once enough have approved.

  A doorway is a node run as a service for other people. It can hold a
  person's key for them, so they can sign in from a browser with a password,
  and it can help them recover an account. When a doorway holds the key, an
  approval made in its portal is signed by the doorway on the person's behalf,
  and the portal says so. When a doorway is the one joining, its browser page
  takes the place of the terminal: it shows the link and takes the code. A
  doorway is one kind of steward a person may have. It is never required: an
  identity can be created, and can grow to a second node, on a network where
  no doorway exists.

  There are three ways an identity can begin and grow, and each must work:
  - It begins on a doorway, which holds Matthew's key, and grows to a node of his own.
  - It begins on a node of his own and grows to a second node of his own.
  - It begins on a node of his own and later gains a doorway.

  Background:
    Given a node "workspace" that Matthew runs himself
    And a node "home" that Matthew runs himself
    And doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Rule: An identity can begin on a node of the person's own, with no doorway

    Scenario: Matthew's identity begins on his own node
      Given no doorway is reachable from node "workspace"
      When Matthew creates his identity on node "workspace"
      Then node "workspace" is the only steward of Matthew's identity
      And the portal on node "workspace" tells Matthew that his identity rests on this node alone

    Scenario: A second node of his own joins without a doorway
      Given Matthew's identity began on node "workspace"
      And no doorway is reachable from node "workspace" or node "home"
      And Jessica, another person on the same network, runs her own node, which no doorway hosts
      When the terminal on node "home" asks to join Matthew's identity
      And Matthew opens the link in the portal on node "workspace" and approves
      And Matthew types the code into the terminal on node "home"
      Then node "home" is one of Matthew's nodes
      When Jessica's node checks for itself whether node "home" belongs to Matthew
      Then Jessica's node answers yes from the joining record it read on the network
      And no doorway was contacted at any step

  Rule: An identity can begin on a doorway and grow to a node of the person's own

    Scenario: An identity that began on a doorway gains a node of Matthew's own
      Given Matthew has an account that doorway "alpha" hosts for him
      And doorway "alpha" is the only steward of Matthew's identity
      When the terminal on node "workspace" asks to join Matthew's identity
      And Matthew opens the link in the portal on doorway "alpha", signs in, and approves
      And Matthew types the code into the terminal on node "workspace"
      Then node "workspace" is one of Matthew's nodes
      And the portal on doorway "alpha" tells Matthew that the doorway holds his key and signed for him

  Rule: An identity that began on the person's own node can later gain a doorway

    Scenario: Matthew adds a doorway so he can sign in from a browser
      Given Matthew's identity began on node "workspace"
      When Matthew opens doorway "alpha" in a browser and asks it to join his identity
      And doorway "alpha" shows a link, which Matthew opens in the portal on node "workspace" and approves
      And Matthew types the code into the page doorway "alpha" is showing
      Then doorway "alpha" is one of Matthew's nodes
      And Matthew can sign in at doorway "alpha" from a browser

    Scenario: Losing the doorway takes nothing away from his own node
      Given Matthew's identity began on node "workspace"
      And doorway "alpha" has since become one of Matthew's nodes
      When doorway "alpha" becomes unreachable
      Then Matthew can still approve a new node in the portal on node "workspace"
      And node "workspace" is still a steward of Matthew's identity

  Rule: One steward's approval is enough, and the others affirm later

    Scenario: A third node joins while one steward is away
      Given node "workspace" and node "home" are both stewards of Matthew's identity
      And node "home" is switched off
      When the terminal on a new node "laptop" asks to join Matthew's identity
      And Matthew opens the link in the portal on node "workspace" and approves
      And Matthew types the code into the terminal on node "laptop"
      Then node "laptop" is one of Matthew's nodes
      And the joining record carries the signature of node "workspace" and of node "laptop"

    Scenario: A steward that was away affirms what it finds
      Given node "laptop" joined with the approval of node "workspace" alone
      When node "home" is switched on and sees the joining record for node "laptop"
      Then node "home" adds its own signature to that joining record
      And Matthew was not asked to approve anything for that
      And node "laptop" was one of Matthew's nodes the whole time

    Scenario: Matthew chooses to require two stewards
      Given node "workspace" and node "home" are both stewards of Matthew's identity
      And Matthew has chosen that a new node needs the approval of two stewards
      When the terminal on a new node "laptop" asks to join Matthew's identity
      And Matthew opens the link in the portal on node "workspace" and approves
      Then the portal tells Matthew that one more of his stewards must approve
      And node "laptop" is not yet one of Matthew's nodes
      When Matthew opens the same link in the portal on node "home" and approves
      Then the portal on node "home" shows the code
      When Matthew types the code into the terminal on node "laptop"
      Then node "laptop" is one of Matthew's nodes
