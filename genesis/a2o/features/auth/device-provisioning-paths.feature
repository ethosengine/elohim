@e2e @auth @device-provisioning @act:i
# Every scenario is held (@wip). The household mesh casts three different people, one node each; these stories need
# several nodes run by one person (the "one-person multi-node topology"), which no fixture stages yet.
Feature: A person's identity starts on one node and grows to others
  As Matthew, setting up the machines I work from
  I want my identity to begin on whichever node I have first and spread to my others one approval at a time
  So that I am never locked out by a node that is missing, and never need a host in order to begin

  A node is a machine running the network's software with its own key. Every
  node in this story is on the same network, and what one node records there
  the others can read. A person's identity is a record on the network that
  names which nodes may speak for that person. The person is the steward of
  their identity; a node only speaks for them. The first node to speak for a
  person is simply the node the identity was created on.

  A new node joins by asking. Its terminal, meaning the command-line program
  Matthew runs on it, prints a link. Matthew opens the link in a portal. A
  portal is the web page served by a node that speaks for him, where Matthew signs in, sees what
  his identity rests on, and approves requests like this one. He reads which
  node is asking and approves. The portal then shows a one-time code, and
  Matthew types it into the terminal that asked. The code proves to that node
  that this terminal is the one that asked, so a link seen by someone else is
  no use to them. The new node then writes a joining record to the network,
  signed by the node that approved and by itself, that any peer can check.

  Joining makes a node one of Matthew's nodes, and every one of his nodes
  speaks for him. No node is special: the network holds his identity, not any
  one machine, and the first node is only where it began. A node's voice
  counts among Matthew's own nodes. So any node of his can approve the next
  one. What proves to anyone else that a node is Matthew's is the joining
  record on the network, never the node's own say-so.

  One such node's approval is enough. The others that speak for him do not have to be
  reachable, and nothing waits for them. When they next see the joining record,
  each adds its own signature to it. This is called affirming. A joining record
  one node signed is valid; one that more of them have affirmed is
  stronger. Matthew may instead choose, when he sets up his identity or at any
  time after, that a new node needs the approval of more than one of them.
  That is his choice and is off unless he makes it. When it is on, he opens the
  same link in the portal of each node that must approve, one after another,
  and the code is shown by the last of them, once enough have approved.

  Affirming is also what keeps this honest. Because any of Matthew's nodes can
  approve the next, the others each look at what was approved and stand
  behind it, and for every node of his Matthew can see which node approved it
  and how many of the others have affirmed it since.

  Matthew may also tell a node that speaks for him, in advance, which node he
  expects to join, by writing that node's key in its settings. When that exact node
  asks, the approving node approves without asking Matthew again, because he has
  already answered.

  Matthew does not always have to carry the link and the code himself. When
  the node that asks and a node that speaks for him can see each other on a
  private network, such as the one in his home, the request and the code
  travel between them directly. When they cannot, he carries both, as
  described above. This works whether or not Matthew said in advance that he
  expected the node; the scenarios below show it for a node he expected.

  A doorway is a node run as a service for other people. It can hold a
  person's key for them, so they can sign in from a browser with a password,
  and it can help them recover an account. When a doorway holds the key, an
  approval made in its portal is signed by the doorway on the person's behalf,
  and the portal says so. When a doorway is the one joining, its browser page
  takes the place of the terminal: it shows the link and takes the code. A
  doorway is one kind of node that can speak for a person. It is never required: an
  identity can be created, and can grow to a second node, on a network where
  no doorway exists.

  Two things about a node are separate, and one person need not hold both.
  Whose it is: the person whose identity it speaks for. Who operates it:
  whoever keeps the machine running and answers for what happens to it.
  Matthew's grandmother-in-law Gertrude needs a node of her own, and
  Matthew will be the one who operates it. That gives him no say over her
  identity. Where a person has nobody suitable to operate their node, the
  people on the network who look after it together, called here the
  community, provide a caretaker to do it, with no more say over her identity
  than Matthew had.

  A node may have begun an identity of its own before anyone told it whose it
  is. When such a node asks to join, each side is told what is about to happen
  before it happens. The approving node shows a recommended choice, which Matthew can
  accept in one step. The node that is joining is shown the same choice and
  must confirm it, because the change is being made to it. That confirmation
  is given by whoever operates the node, from its own terminal or its own
  portal. In this story Matthew runs both nodes, so he answers twice: once as
  the person the approving node speaks for, and once as the operator of the node that
  is joining.

  Which choice is recommended depends on whether the node has made anything
  under the identity it began. If it has made nothing, the recommended choice
  is a clean start: the node gives up that identity and its key, makes a new
  key, and joins. If it has made things, the recommended choice is to keep
  everything: the node keeps its key and joins as it is, and what it made
  stays where it is, still tracing to the identity it began. The portal says
  plainly that it has been left as it was. What becomes of the identity the
  node began, which it no longer acts under, is part of the larger question
  below.

  When a node Matthew said to expect turns out to have an identity
  of its own, both rules apply: the approving node applies the recommended choice
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

    # held: needs node "workspace" cut off from every doorway (network isolation) and its native portal (arch-device-recognition-backlog.md row 11)
    @wip
    Scenario: Matthew's identity begins on his own node
      Given no doorway is reachable from node "workspace"
      When Matthew creates his identity on node "workspace"
      Then node "workspace" is the only node that speaks for Matthew
      And the portal on node "workspace" tells Matthew that his identity rests on this node alone

    # held: needs every doorway out of reach (network isolation) and the one-person multi-node topology (arch-device-recognition-backlog.md row 2)
    @wip
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

    # held: needs a real browser on the doorway portal and the one-person multi-node topology (arch-device-recognition-backlog.md row 11)
    @wip
    Scenario: An identity that began on a doorway gains a node of Matthew's own
      Given Matthew has an account that doorway "alpha" hosts for him
      And doorway "alpha" is the only node that speaks for Matthew
      When the terminal on node "workspace" asks to join Matthew's identity
      And Matthew opens the link in the portal on doorway "alpha", signs in, and approves
      And Matthew types the code into the terminal on node "workspace"
      Then node "workspace" is one of Matthew's nodes
      And the portal on doorway "alpha" tells Matthew that the doorway holds his key and signed for him

    # held: needs a doorway taken down mid-scenario and the one-person multi-node topology (arch-device-recognition-backlog.md row 11)
    @wip
    Scenario: Having begun on a doorway, Matthew no longer depends on it
      Given Matthew's identity began on doorway "alpha" and node "workspace" has since joined it
      When doorway "alpha" becomes unreachable
      Then Matthew can still approve a new node in the portal on node "workspace"

  Rule: An identity that began on the person's own node can later gain a doorway

    # held: needs a real browser signing in at the doorway and the native portal (arch-device-recognition-backlog.md row 11)
    @wip
    Scenario: Matthew adds a doorway so he can sign in from a browser
      Given Matthew's identity began on node "workspace"
      When Matthew opens doorway "alpha" in a browser and asks it to join his identity
      And doorway "alpha" shows a link, which Matthew opens in the portal on node "workspace" and approves
      And Matthew types the code into the page doorway "alpha" is showing
      Then doorway "alpha" is one of Matthew's nodes
      And Matthew can sign in at doorway "alpha" from a browser

    # held: needs a doorway taken down mid-scenario and the one-person multi-node topology (arch-device-recognition-backlog.md row 11)
    @wip
    Scenario: Losing the doorway takes nothing away from his own node
      Given Matthew's identity began on node "workspace"
      And doorway "alpha" has since become one of Matthew's nodes
      When doorway "alpha" becomes unreachable
      Then Matthew can still approve a new node in the portal on node "workspace"
      And node "workspace" still speaks for Matthew

  Rule: One node's approval is enough, and the others that speak for the person affirm later

    # held: needs the one-person multi-node topology, with one node switched off (arch-device-recognition-backlog.md row 12)
    @wip
    Scenario: A third node joins while one of the nodes that speak for Matthew is away
      Given node "workspace" and node "home" both speak for Matthew
      And node "home" is switched off
      When the terminal on a new node "laptop" asks to join Matthew's identity
      And Matthew opens the link in the portal on node "workspace" and approves
      And Matthew types the code into the terminal on node "laptop"
      Then node "laptop" is one of Matthew's nodes
      And the joining record carries the signature of node "workspace" and of node "laptop"

    # held: needs the one-person multi-node topology, with a node switched off and back on (arch-device-recognition-backlog.md row 12)
    @wip
    Scenario: A node that was away affirms what it finds
      Given node "laptop" joined with the approval of node "workspace" alone
      When node "home" is switched on and sees the joining record for node "laptop"
      Then node "home" adds its own signature to that joining record
      And Matthew was not asked to approve anything for that
      And node "laptop" was one of Matthew's nodes the whole time

    # held: not built: the node's own portal showing approvals and affirmations (arch-device-recognition-backlog.md row 11)
    @wip
    Scenario: Matthew can see who approved a node and who has stood behind it
      Given node "workspace" and node "home" both speak for Matthew
      And node "laptop" joined with the approval of node "workspace" alone
      When Matthew looks at his identity in the portal on node "workspace"
      Then he sees that node "laptop" was approved by node "workspace" and has been affirmed by no other node yet
      When node "home" has affirmed the joining record for node "laptop"
      Then he sees that node "laptop" has been affirmed by one other node

    # held: not built: an approvals policy above one (arch-device-recognition-backlog.md row 5, superseded by row 12)
    @wip
    Scenario: Matthew chooses to require two approvals
      Given node "workspace" and node "home" both speak for Matthew
      And Matthew has chosen that a new node needs the approval of two of the nodes that speak for him
      When the terminal on a new node "laptop" asks to join Matthew's identity
      And Matthew opens the link in the portal on node "workspace" and approves
      Then the portal tells Matthew that one more of the nodes that speak for him must approve
      And node "laptop" is not yet one of Matthew's nodes
      When Matthew opens the same link in the portal on node "home" and approves
      Then the portal on node "home" shows the code
      When Matthew types the code into the terminal on node "laptop"
      Then node "laptop" is one of Matthew's nodes

  Rule: A node Matthew expects joins without his being asked again, and any node of his can approve the next

    # held: needs private-network discovery between two nodes (arch-device-recognition-backlog.md row 2, row 8)
    @wip
    Scenario: An expected node with no identity of its own joins without a second answer
      Given Matthew's identity began on node "workspace"
      And Matthew has told node "workspace" that he expects node "home" to join
      And node "home" has no identity of its own
      And node "workspace" and node "home" can see each other on a private network
      When node "home" asks to join Matthew's identity
      Then node "workspace" approves without asking Matthew
      And the code reaches node "home" over the private network
      And node "home" is one of Matthew's nodes
      And Matthew carried neither a link nor a code

    # held: needs the one-person multi-node topology (arch-device-recognition-backlog.md row 12)
    @wip
    Scenario: A node that joined approves the next one
      Given Matthew's identity began on node "workspace"
      And node "home" has joined Matthew's identity
      And node "workspace" is switched off
      And Jessica, another person on the same network, runs her own node, which no doorway hosts
      When the terminal on a new node "laptop" asks to join Matthew's identity
      And Matthew opens the link in the portal on node "home" and approves
      And Matthew types the code into the terminal on node "laptop"
      Then node "laptop" is one of Matthew's nodes
      When Jessica's node checks for itself whether node "laptop" belongs to Matthew
      Then Jessica's node answers yes from the joining records it read on the network

    # held: needs the one-person multi-node topology, four nodes of one person (arch-device-recognition-backlog.md row 12)
    @wip
    Scenario: Two of Matthew's nodes approve two new nodes at the same time
      Given node "workspace" and node "home" both speak for Matthew
      And node "workspace" and node "home" cannot reach each other
      When Matthew approves a new node "laptop" in the portal on node "workspace"
      And Matthew approves a new node "tablet" in the portal on node "home"
      And node "workspace" and node "home" can reach each other again
      Then node "laptop" is one of Matthew's nodes
      And node "tablet" is one of Matthew's nodes
      And neither approval undid or held up the other

  Rule: The person a node speaks for and the one who operates it can be different

    # held: not built: a node whose operator is not the person it speaks for (arch-device-recognition-backlog.md row 7)
    @wip
    Scenario: Gertrude's identity begins on a node Matthew operates
      Given a node "cottage" that Matthew operates for Gertrude
      When Gertrude's identity is created on node "cottage"
      Then node "cottage" is the only node that speaks for Gertrude
      And Matthew is the operator of node "cottage"
      And node "cottage" is not one of Matthew's nodes

    # held: not built: a node whose operator is not the person it speaks for (arch-device-recognition-backlog.md row 7)
    @wip
    Scenario: Operating a node gives no say over the identity it speaks for
      Given Gertrude's identity began on node "cottage", which Matthew operates
      When the terminal on a new node "tablet" asks to join Gertrude's identity
      Then the portal on node "cottage" asks for Gertrude's approval, not Matthew's
      And an approval given in Matthew's name is refused

    # held: not built: handing a node's operation to the community's caretaker (device-recognition rows 6, 21)
    @wip
    Scenario: The community's caretaker operates a node when nobody else can
      Given Gertrude's identity began on node "cottage", which Matthew operates
      When Matthew stops operating node "cottage" and the community's caretaker takes it on
      Then the community's caretaker is the operator of node "cottage"
      And node "cottage" is still the only node that speaks for Gertrude
      And Gertrude's identity is unchanged

  Rule: A node that already began an identity joins with a recommended choice, and confirms what will happen to it

    # held: not built: joining a node that already has an identity of its own (arch-device-recognition-backlog.md row 7)
    @wip
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

    # held: not built: joining a node that already has an identity of its own (arch-device-recognition-backlog.md row 7)
    @wip
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

    # held: not built: joining a node that already has an identity of its own (arch-device-recognition-backlog.md row 7)
    @wip
    Scenario: Choosing a clean start for a node that made things names what will be lost
      Given Matthew's identity began on node "workspace"
      And node "home" began an identity of its own
      And node "home" published the content "sensor-log" under that identity
      When the terminal on node "home" asks to join Matthew's identity
      And Matthew chooses a clean start in the portal on node "workspace"
      Then node "home" tells its operator that it will give up its key, and that the content "sensor-log" will no longer be its own to change
      And node "home" has changed nothing yet

    # held: not built: joining a node that already has an identity of its own (arch-device-recognition-backlog.md row 7)
    @wip
    Scenario: The node's operator says no, and nothing changes
      Given Matthew's identity began on node "workspace"
      And node "home" began an identity of its own and has made nothing under it
      When the terminal on node "home" asks to join Matthew's identity
      And Matthew accepts the recommended choice in the portal on node "workspace"
      And the operator of node "home" does not confirm
      Then node "home" has the same key it had before
      And node "home" is not one of Matthew's nodes
      And the portal on node "workspace" shows that node "home" did not confirm

    # held: needs private-network discovery between two nodes (arch-device-recognition-backlog.md row 2, row 8) and a node with its own identity (row 7)
    @wip
    Scenario: An expected node that already has an identity still confirms
      Given Matthew's identity began on node "workspace"
      And Matthew has told node "workspace" that he expects node "home" to join
      And node "home" began an identity of its own and has made nothing under it
      And node "workspace" and node "home" can see each other on a private network
      When node "home" asks to join Matthew's identity
      Then node "workspace" applies the recommended choice without asking Matthew
      And node "home" tells its operator that it will give up the identity it began and its key, and make a new key
      And node "home" has changed nothing yet
      When the operator of node "home" confirms
      Then the code reaches node "home" over the private network
      And node "home" is one of Matthew's nodes
      And Matthew was not asked to approve anything on node "workspace"
