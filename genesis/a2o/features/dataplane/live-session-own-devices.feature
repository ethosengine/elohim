# First runnable seam of features/federation/local-first-shared-sheet.feature ("They see each other's
# edits and cursors as they happen"). That story needs several people, a hub and a doorway. This one
# takes the smallest case that still proves the channel: two machines one person looks after, talking
# to each other directly. Every step is undefined; the feature stays @wip until the slice in
# genesis/docs/superpowers/plans/2026-10-14-live-session-slice-1-own-devices-direct.md is built.
@e2e @dataplane @wip @requires:household-nodes @requires:owned-substrate @requires:device-node @concern:live-session-direct @act:i
Feature: Two of Matthew's machines show each other who is here, and keep no record of it

  This is the smallest case of people seeing each other while they work on the same thing. The
  larger story, local-first-shared-sheet, has several people, a hub and a doorway; this one has one
  person and two machines, which is the least that still proves the channel works.

  Matthew is working on a shared sheet: a spreadsheet that several of his machines keep a copy of
  and edit together. He has it open on two machines he looks after: the small always-on node in the
  hallway cupboard, called "matthew" below, and the laptop he works at, called "workspace". Each
  machine runs its own node, so "node" and "machine" mean the same thing here.

  These machines belong to the household mesh: the few nodes in one home, here Matthew's two and a
  third belonging to another member of the household, each holding its own copy of what the
  household shares.

  While the sheet is open on both, each machine shows that the other is here, and a small value
  Matthew sets on one (which cell he has selected) appears on the other. This is "presence". It is
  not part of the sheet. It is not saved, it has no history, and nothing is ever decided from it.

  The two nodes talk to each other directly. No doorway, no hub and no third machine is involved.

  A node takes part only if Matthew has granted it leave to. The grant is a signed record he makes
  once for a machine, not once for each sheet. He makes them from his always-on node, including
  one for that node itself. Each node checks the grant of any node that asks to
  join it, so a node without one is refused by whichever node it asks, and is told why. The
  household has a third node, "james", which belongs to another member of the household. It keeps
  a copy of the sheet like the others, but Matthew has given it no such grant. It is the node the
  refusal is shown on.

  Presence is kept alive by a small renewal message while the sheet is open. A machine that closes
  the sheet says so, and the other shows it gone at once. A machine that simply vanishes (the lid
  shut, the power cut) says nothing. After 30 seconds without a renewal the other machine stops
  showing it as here and shows "unknown" instead. The scenario below allows 45 seconds: the 30
  seconds, plus time for the last renewal to have been sent just before the machine vanished. It
  never shows "left" for a machine that did not say it left, because silence does not tell the two
  cases apart.

  The cost matters as much as the feature. A node that could take part, with no sheet open, must
  do nothing at all for it: no message to the other node, and no request to its conductor. A
  conductor is the process beside each node that signs and records what the person does; a request
  to it is the costly step this household counts.

  Background:
    Given the household mesh is running
    And node "workspace" is one of Matthew's devices
    And Matthew has granted nodes "matthew" and "workspace" leave to take part in live sessions
    And a shared sheet "Winter budget" is held by nodes "matthew" and "workspace"

  Scenario: Each machine shows the other is here
    When Matthew opens "Winter budget" on node "matthew"
    And Matthew opens "Winter budget" on node "workspace"
    Then within 5 seconds node "matthew" shows node "workspace" as here in "Winter budget"
    And within 5 seconds node "workspace" shows node "matthew" as here in "Winter budget"

  Scenario: A value set on one machine appears on the other
    Given "Winter budget" is open on nodes "matthew" and "workspace"
    When Matthew sets his selection to "B7" on node "workspace"
    Then within 5 seconds node "matthew" shows node "workspace" with selection "B7"

  Scenario: A machine that closes the sheet is shown as gone at once
    Given "Winter budget" is open on nodes "matthew" and "workspace"
    When Matthew closes "Winter budget" on node "workspace"
    Then within 5 seconds node "matthew" shows node "workspace" as having left "Winter budget"

  Scenario: A machine that vanishes is shown as unknown, never as having left
    Given "Winter budget" is open on nodes "matthew" and "workspace"
    When node "workspace" is stopped without warning
    Then within 45 seconds node "matthew" shows node "workspace" as "unknown" in "Winter budget"
    And at no point in those 45 seconds did node "matthew" show node "workspace" as having left "Winter budget"

  Scenario: A node Matthew has not granted leave to is refused and told why
    Given "Winter budget" is open on node "matthew"
    And "Winter budget" is also held by node "james"
    And Matthew has not granted node "james" leave to take part in live sessions
    When "Winter budget" is opened on node "james"
    Then node "james" asks node "matthew" to join and is refused with the reason "no grant for live sessions"
    And node "james" is shown nobody as here in "Winter budget"
    And node "matthew" does not show node "james" as here in "Winter budget"

  Scenario: With no sheet open, a node that could take part does nothing for it
    Given no sheet is open on any node
    When the household is left alone for 5 minutes
    Then node "matthew" sent no live-session message
    And node "workspace" sent no live-session message
    And each node made the same number of requests to its conductor, the process that signs and records its acts, as it does with live sessions switched off

  Scenario: A session leaves nothing behind
    Given "Winter budget" was open on nodes "matthew" and "workspace" for 1 minute and is now closed on both
    Then no node's database holds a row describing that session
    And no node's chain gained an action because of that session
    And "Winter budget" holds no change made by presence alone
