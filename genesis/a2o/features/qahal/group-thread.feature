# @concern:chat-is-a-record joins this story to the habit that will hold it; no habit names it yet.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three nodes are running. @requires:owned-substrate: the
# test cuts one node's network and restarts nodes, so it runs only on a mesh this run owns.
# Traceability only: chat's status is recorded in the repository document
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md.
# The story needs nothing from it; the feature description states that status for any reader.
@e2e @qahal @wip @concern:chat-is-a-record @requires:household-nodes @requires:owned-substrate @act:i
Feature: A message written with no network reaches the household when its writer reconnects

  Matthew, Jessica and James share a household. They keep one conversation together, the
  household thread, where they leave each other messages: who is collecting whom, what is needed
  from the shop. The thread is shared with the three of them and with nobody else. It exists
  because the household exists, and only its members hold it; how a thread is created, and how
  it is kept from anyone outside the household, is not this story's concern: every scenario starts
  with the thread already set up for the three of them.

  Each of them writes from a node of their own: a machine that keeps its own copy of what the
  household shares, the thread among it. The nodes here are named after their people: "matthew",
  "jessica" and "james". A node needs no network to show the thread or to take a new message.

  A message is a record. Once written it is signed by the person who wrote it, so that any node
  that receives it can check who wrote it before showing it. It is kept by every member's node, and
  still there tomorrow, like anything else the household keeps. That is what sets a message apart
  from presence (the "Jessica is typing" a node shows while she writes), which is passed along while
  it happens and never kept.

  The promise this story tests: a message is never lost because its writer had no network. It waits
  on the writer's own node, which marks it as "held on this node only" so the writer can see that
  nobody else has it yet, and it reaches everyone when the writer's node reconnects, without anyone
  being asked to send it again or to sync. Once the others have it, the mark goes. How the mark
  looks (a label, a symbol, a colour) is not designed yet, so the steps check a state, not a
  picture: that the writer's node knows the message has reached no other node yet and tells the
  writer so, and that it stops once the others have it.

  The steps wait at most 10 seconds after a node reconnects for its messages to reach the others.
  That is how long the test waits, not a promise about how fast chat must be; how quickly messages
  travel is not designed yet. A node the story does not say has lost its network is connected.

  When people write while apart, every node must still show the thread in one order. Each writer's
  messages keep the order that writer wrote them in. Between different writers, every node works
  out the same order for itself, so no node shows the thread differently from another. Which rule
  picks that order is not designed yet; this story asks only that every node agrees on it. So the
  second scenario does not check which order the nodes show: it checks that they all show the same
  one, and that Jessica's two messages keep the order she wrote them in. That is the test any
  ordering rule chosen later must pass.

  This story is written ahead of its design. Chat is not designed or built yet, so none of these
  steps can run today; the story is marked @wip and stays so until chat is designed and built. No
  habit holds this promise yet either: the tag @concern:chat-is-a-record names the concern that a
  habit will hold once chat is built and these steps run.

  Background:
    Given the household's nodes "matthew", "jessica" and "james" are running
    And the household thread has been set up with Matthew, Jessica and James as its members
    And all three nodes show the same messages in the household thread, in the same order

  Scenario: A message Jessica writes with no network reaches the others when she reconnects
    Given node "jessica" has lost its network connection
    When Jessica writes "I'll collect the bread" in the household thread on node "jessica"
    Then node "jessica" shows the message in the thread at once, marked as held on this node only
    And nodes "matthew" and "james" do not show the message
    When node "jessica" reconnects
    Then within 10 seconds nodes "matthew" and "james" show "I'll collect the bread" in the household thread
    And each of them shows it as written by Jessica, having checked that she signed it
    And node "jessica" no longer marks the message as held on this node only
    And no node showed anyone a prompt to send, retry or sync the message

  Scenario: Messages written apart are all kept, in the same order on every node
    Given node "jessica" has lost its network connection
    And nodes "matthew" and "james" are still connected to each other
    When Jessica writes "Milk too?" and then "Never mind, we have some" on node "jessica"
    And James writes "Back by six" on node "james"
    Then node "matthew" shows "Back by six" while node "jessica" is still without a network
    When node "jessica" reconnects
    Then within 10 seconds all three nodes show all three messages in the household thread
    And all three nodes show the three messages in the same order as each other
    And on every node "Milk too?" comes before "Never mind, we have some"

  Scenario: A message stays after every node restarts, and the fact that someone was typing does not
    Given Jessica has begun writing a message in the household thread on node "jessica"
    And node "matthew" shows "Jessica is typing" while she writes
    And Jessica has finished writing "Back late tonight" on node "jessica"
    And every node shows "Back late tonight" in the household thread
    When all three nodes are restarted
    Then every node still shows "Back late tonight" in the household thread, written by Jessica
    And no node has kept any record that Jessica was typing
