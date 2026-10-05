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

  Joining makes a node one of Matthew's nodes: the network recognizes it as
  his. It does not by that alone make the node a steward. A node of his becomes
  a steward as well, able to approve other nodes, when Matthew says so on a
  node that already is one.

  One steward's approval is enough. Matthew's other stewards do not have to be
  reachable, and nothing waits for them. When they next see the joining record,
  each adds its own signature to it. This is called affirming. A joining record
  one steward signed is valid; one that more stewards have affirmed is
  stronger. Matthew may instead choose, when he sets up his identity or at any
  time after, that a new node needs the approval of more than one steward.
  That is his choice and is off unless he makes it. When it is on, he opens the
  same link in the portal of each steward that must approve, one after another,
  and the code is shown by the last of them, once enough have approved.

  Matthew may also tell a steward in advance which node he expects to join,
  by writing that node's key in the steward's settings. When that exact node
  asks, the steward approves without asking Matthew again, because he has
  already answered, and shows the code as it would after an approval.

  A doorway is a node run as a service for other people. It can hold a
  person's key for them, so they can sign in from a browser with a password,
  and it can help them recover an account. When a doorway holds the key, an
  approval made in its portal is signed by the doorway on the person's behalf,
  and the portal says so. When a doorway is the one joining, its browser page
  takes the place of the terminal: it shows the link and takes the code. A
  doorway is one kind of steward a person may have. It is never required: an
  identity can be created, and can grow to a second node, on a network where
  no doorway exists.

  Three things about a node are separate, and one person need not hold all
  three. Whose it is: the person whose identity it speaks for. Who operates
  it: whoever keeps the machine running and answers for what happens to it.
  Whether it is a steward: whether it may approve other nodes for that
  identity. Matthew's grandmother-in-law Gertrude needs a node of her own, and
  Matthew will be the one who operates it. That gives him no say over her
  identity. Where a person has nobody suitable to operate their node, the
  people on the network who look after it together, called here the
  community, provide a caretaker to do it, on the same terms.

  A node may have begun an identity of its own before anyone told it whose it
  is. When such a node asks to join, each side is told what is about to happen
  before it happens. The steward shows a recommended choice, which Matthew can
  accept in one step. The node that is joining is shown the same choice and
  must confirm it, because the change is being made to it. That confirmation
  is given by whoever operates the node, from its own terminal or its own
  portal. In this story Matthew runs both nodes, so he answers twice: once as
  the person the steward speaks for, and once as the operator of the node that
  is joining.

  Which choice is recommended depends on whether the node has made anything
  under the identity it began. If it has made nothing, the recommended choice
  is a clean start: the node gives up that identity and its key, makes a new
  key, and joins. If it has made things, the recommended choice is to keep
  everything: the node keeps its key and joins as it is, and what it made
  stays where it is, still tracing to the identity it began. The portal says
  plainly that it has been left as it was.

  When a node Matthew told a steward to expect turns out to have an identity
  of its own, both rules apply: the steward applies the recommended choice
  without asking Matthew again, and the node must still confirm it.

  Both of those are small decisions. There is a third, for when a lot rests on
  what the node made: going through that earlier work and settling what
  becomes Matthew's and on what terms. That is a larger undertaking and this
  story does not cover it.

  There are three ways an identity can begin and grow, and each must work:
  - It begins on a node of his own and grows to a second node of his own.
  - It begins on a doorway, which holds Matthew's key, and grows to a node of his own.
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

  Rule: A node Matthew expects joins without his being asked again, and a node becomes a steward only when he says so

    Scenario: An expected node with no identity of its own joins without a second answer
      Given Matthew's identity began on node "workspace"
      And Matthew has told node "workspace" that he expects node "home" to join
      And node "home" has no identity of its own
      When the terminal on node "home" asks to join Matthew's identity
      Then node "workspace" approves without asking Matthew
      And node "workspace" shows the code
      When Matthew types the code into the terminal on node "home"
      Then node "home" is one of Matthew's nodes

    Scenario: Matthew makes a node of his a steward
      Given Matthew's identity began on node "workspace"
      And node "home" is one of Matthew's nodes and is not a steward
      When Matthew says on node "workspace" that node "home" is a steward of his identity
      Then node "workspace" and node "home" are both stewards of Matthew's identity
      And Matthew can approve a new node in the portal on node "home"

  Rule: The person a node speaks for and the one who operates it can be different

    Scenario: Gertrude's identity begins on a node Matthew operates
      Given a node "cottage" that Matthew operates for Gertrude
      When Gertrude's identity is created on node "cottage"
      Then node "cottage" is the only steward of Gertrude's identity
      And Matthew is the operator of node "cottage"
      And node "cottage" is not one of Matthew's nodes

    Scenario: Operating a node gives no say over the identity it speaks for
      Given Gertrude's identity began on node "cottage", which Matthew operates
      When the terminal on a new node "tablet" asks to join Gertrude's identity
      Then the portal on node "cottage" asks for Gertrude's approval, not Matthew's
      And an approval given in Matthew's name is refused

    Scenario: The community's caretaker operates a node when nobody else can
      Given Gertrude's identity began on node "cottage", which Matthew operates
      When Matthew stops operating node "cottage" and the community's caretaker takes it on
      Then the community's caretaker is the operator of node "cottage"
      And node "cottage" is still the only steward of Gertrude's identity
      And Gertrude's identity is unchanged

  Rule: A node that already began an identity joins with a recommended choice, and confirms what will happen to it

    Scenario: A node that made nothing gets a clean start
      Given Matthew's identity began on node "workspace"
      And node "home" began an identity of its own and has made nothing under it
      When the terminal on node "home" asks to join Matthew's identity
      Then the portal on node "workspace" tells Matthew that node "home" already has an identity of its own
      And the portal on node "workspace" recommends a clean start
      When Matthew accepts the recommended choice
      Then node "home" tells its operator that it will give up the identity it began and its key, and make a new key
      And node "home" has changed nothing yet
      When the operator of node "home" confirms
      And Matthew types the code into the terminal on node "home"
      Then node "home" is one of Matthew's nodes
      And node "home" has a new key

    Scenario: A node that made things keeps everything
      Given Matthew's identity began on node "workspace"
      And node "home" began an identity of its own
      And node "home" published the content "sensor-log" under that identity
      When the terminal on node "home" asks to join Matthew's identity
      Then the portal on node "workspace" recommends keeping everything
      And the portal on node "workspace" tells Matthew that what node "home" made will be left as it was
      When Matthew accepts the recommended choice
      Then node "home" tells its operator that it will keep its key and what it made, and join as it is
      When the operator of node "home" confirms
      And Matthew types the code into the terminal on node "home"
      Then node "home" is one of Matthew's nodes
      And node "home" has the same key it had before
      And the content "sensor-log" still traces to the identity node "home" began

    Scenario: Choosing a clean start for a node that made things names what will be lost
      Given Matthew's identity began on node "workspace"
      And node "home" began an identity of its own
      And node "home" published the content "sensor-log" under that identity
      When the terminal on node "home" asks to join Matthew's identity
      And Matthew chooses a clean start in the portal on node "workspace"
      Then node "home" tells its operator that it will give up its key, and that the content "sensor-log" will no longer be its own to change
      And node "home" has changed nothing yet

    Scenario: The node's operator says no, and nothing changes
      Given Matthew's identity began on node "workspace"
      And node "home" began an identity of its own and has made nothing under it
      When the terminal on node "home" asks to join Matthew's identity
      And Matthew accepts the recommended choice in the portal on node "workspace"
      And the operator of node "home" does not confirm
      Then node "home" has the same key it had before
      And node "home" is not one of Matthew's nodes
      And the portal on node "workspace" shows that node "home" did not confirm

    Scenario: An expected node that already has an identity still confirms
      Given Matthew's identity began on node "workspace"
      And Matthew has told node "workspace" that he expects node "home" to join
      And node "home" began an identity of its own and has made nothing under it
      When the terminal on node "home" asks to join Matthew's identity
      Then node "workspace" applies the recommended choice without asking Matthew
      And node "home" tells its operator that it will give up the identity it began and its key, and make a new key
      And node "home" has changed nothing yet
      When the operator of node "home" confirms
      Then node "workspace" shows the code
      When Matthew types the code into the terminal on node "home"
      Then node "home" is one of Matthew's nodes
      And Matthew was not asked to approve anything on node "workspace"
