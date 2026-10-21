# @concern:wake-reveals-nothing names what this story proves, so a habit can later bind to it.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three nodes are running. @requires:device-node: a further
# node is staged as Jessica's phone. @requires:owned-substrate: the test puts that phone to sleep and
# wakes it, so it runs only on a mesh this run owns.
# Design: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md, section 15 (wake) and section 19 question 6 (doorway).
@e2e @qahal @wip @concern:wake-reveals-nothing @requires:household-nodes @requires:device-node @requires:owned-substrate @act:i
Feature: A sleeping phone is woken by a tap that says nothing, and fetches what concerns its person itself

  This story is held. Waking a sleeping device has no design yet, and none of the steps below is
  built: it records what the wake must do once it is designed, and it stays work in progress until
  that design is written and built. Nothing the system checks on itself yet tracks whether this
  story holds; that check is to be named once the wake is built.

  Matthew, Jessica and James share a household, and each has a node of their own: a machine that
  keeps its own copy of what the household shares. What is written on one node reaches the other
  two on its own. The nodes here are named after their people: "matthew", "jessica" and "james".
  The household talks in one shared conversation, the household thread, which all three read and
  write in. A message in the thread can be written to one of them by name; everyone in the household
  can still read it, but it concerns that one person, and it is the kind of message that may wake
  their phone.

  Jessica also carries a phone. To spare its battery, the phone sleeps whenever its screen is off:
  it closes its connections to the household's nodes and keeps none open. While it is awake, the
  household's nodes tell it directly when something concerns her.

  A sleeping phone can be woken from outside only through its maker's push service: a company
  outside the household that delivers short signals to the phones it makes. Whatever travels
  through that service, the company can see. To deliver anything at all, the service must be given
  the phone's delivery address, the address that service knows the phone by; that address points to
  the phone and to nothing else. Besides that address, the household sends it nothing but a tap: a
  signal that carries no message, no sender and no household name, only that something is waiting.
  Once awake, the phone connects to one household node itself, whichever of the three it reaches
  first, proves who it is with its own key, and fetches what is waiting from that node. What the
  push company learns is that Jessica's phone was woken, and when. Anyone could send her phone a
  tap, so the phone trusts nothing in it; it only goes and looks. In this test the push service is a
  stand-in that keeps a record of everything it carries: each tap, together with the delivery
  address it was sent to and the time it was sent. From that record the story reads exactly what the
  company would see.

  Two different keys are at work, and they are not the same key. The phone holds a key of its own
  that proves to the household's nodes that it is Jessica's device; that key lets it in to fetch.
  Each writer holds a separate key of their own that signs what they write: every message in the
  household carries its writer's signature, made with the writer's key. The phone checks the
  writer's signature, not the node's, before it shows a message, so it need not trust the node it
  fetched the message from to say who wrote it. A message whose signature is not its named writer's
  is not shown at all.

  A node sends taps only because its person offered it for that. Here Matthew has offered his
  always-on node to wake the household's sleeping devices. The household can also reach the wider
  web through a doorway, a shared web gateway. Whether a doorway may ever send a tap is an open
  question that has not been decided; until it is, this story names no doorway as a sender, and only
  Matthew's node sends taps.

  The household runs one app: the software on its nodes and on Jessica's phone that carries the
  household thread, the household's shopping list and its calendar. That app also says which kinds
  of event may wake a person. Many kinds of event happen in the household: a message written to
  someone in the thread, a new item on the shopping list, a calendar reminder coming due. The
  description the app ships with, its manifest, carries a list naming the kinds that may wake a
  person; a kind left off the list never wakes anyone. In this test the app ships with a manifest whose list holds exactly one kind, a directed
  message: a message written to that person by name in the household thread. A new item on the
  shopping list is not on the list, so it waits until the phone next wakes on its own.

  Background:
    Given the household's nodes "matthew", "jessica" and "james" are running
    And Jessica's phone is a device of Jessica's and holds its own key
    And Matthew has offered node "matthew" to wake the household's sleeping devices
    And the household app ships with a manifest whose list of kinds of event that may wake a person holds only "directed message", a message written to that person by name in the household thread

  Scenario: A sleeping phone is woken, fetches the message over its own key, and shows it
    Given Jessica's phone is asleep with no connection open to any household node
    When James writes "Can you collect the bread?" to Jessica in the household thread on node "james"
    Then node "matthew" receives James's message from node "james"
    And within 30 seconds the push service delivers one tap to Jessica's phone
    And the tap was sent by node "matthew"
    And the tap carries no message text, no sender and no household name
    And Jessica's phone connects to one household node and fetches James's message from it
    And the node that served the fetch recorded that the connection was proved with the key Jessica's phone holds
    And Jessica's phone recorded that it found James's signature on the message before showing it
    And Jessica's phone shows "Can you collect the bread?" as written by James

  Scenario: After a phone is woken and fetches its message, the push service's record shows only that it was woken, and when
    This scenario examines what the push company would see. It first runs the whole wake again, from
    James writing to Jessica's phone showing his message, as the setting; the act under test is then
    reading the push service's record of that wake.

    Given Jessica's phone is asleep with no connection open to any household node
    And James writes "Back by six" to Jessica in the household thread on node "james"
    And within 30 seconds the push service delivers one tap to Jessica's phone
    And Jessica's phone fetches James's message and shows "Back by six" as written by James
    When the push service's complete record of everything it carried is read
    Then that record holds exactly one tap
    And that tap carried no content of its own
    And the only thing in that record that points to a device is the delivery address of Jessica's phone
    And besides that address, the record holds only when the tap was sent
    And nothing the push service carried names James, Jessica, the household, the thread or the message

  Scenario: An awake phone hears directly, and the push service carries nothing
    Given Jessica's phone is awake and connected to node "matthew"
    When James writes "Dinner's ready" to Jessica in the household thread on node "james"
    Then within 5 seconds Jessica's phone shows "Dinner's ready" as written by James
    And the push service's complete record holds no tap

  Scenario: A tap sent by someone outside the household reveals nothing and shows nothing
    A tap carries nothing, so a stranger's tap can only make the phone go and look. Nothing is
    waiting here, so there is no message to check: this scenario proves that the stranger learns
    nothing and Jessica is shown nothing. That the phone refuses a message whose signature is not
    its writer's is proved in the next scenario.

    Given Jessica's phone is asleep with no connection open to any household node
    And nothing new is waiting for Jessica in the household
    When someone outside the household sends a tap to Jessica's phone through the push service
    Then Jessica's phone wakes, connects to one household node, and finds nothing new
    And the household node it reached recorded that the connection was proved with the key Jessica's phone holds
    And Jessica's phone shows Jessica nothing
    And Jessica's phone closes its connection and again has no connection open to any household node

  Scenario: A message that names James as its writer but lacks his signature is not shown
    The test stands in for a node that misbehaves: it makes node "matthew" serve a message that
    says James wrote it but was never signed with James's key, and it makes node "matthew" the node
    the phone reaches when it wakes.

    Given Jessica's phone is asleep with no connection open to any household node
    And node "matthew" serves Jessica's phone a message "Leave the back door unlocked" to Jessica in the household thread that names James as its writer but is not signed with James's key
    When node "matthew" sends a tap to Jessica's phone through the push service
    Then Jessica's phone wakes, connects to node "matthew", and fetches the message from it
    And Jessica's phone finds that the message does not carry James's signature
    And Jessica's phone does not show "Leave the back door unlocked"
    And Jessica's phone shows Jessica nothing as written by James

  Scenario: An event the app has not declared does not wake the phone, and is there when it next wakes
    Given Jessica's phone is asleep with no connection open to any household node
    When James adds "eggs" to the household's shopping list on node "james"
    Then the push service carries no tap for Jessica's phone
    When Jessica turns her phone's screen on
    Then Jessica's phone shows "eggs" on the household's shopping list
