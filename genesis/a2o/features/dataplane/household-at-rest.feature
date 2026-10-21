# The runaway guard. This story does not test any one loop; it watches a whole
# household while nothing happens and counts what the nodes did anyway. It runs
# on the household mesh — three nodes on one machine that the test lane owns —
# so a loop that invents work for itself is caught here, before the fleet sees it.
@e2e @dataplane @concern:idle-is-free @act:i @requires:owned-substrate
Feature: A household at rest is quiet

  Matthew, Jessica and James share a household of three nodes — small machines in
  an ordinary home. Tonight nobody is using them. Their content is already in
  sync; no person or external client is asking them to write, read or repair.
  Whatever autonomous maintenance the nodes do in the next five minutes is work
  they invented for themselves, and it is paid for in
  electricity, in the fan in the hallway cupboard, and above all in the node not
  being free to answer when someone does ask for something.

  That last cost is why this story exists. On 2026-09-19 every node on the
  deployed fleet had its processor pinned while serving no one, and no page
  could be published because the nodes had no time left to answer. The same day
  this household, holding 27 pieces of content and doing nothing, made 50 to 74
  conductor calls a minute on each node and kept one and a half processor cores
  busy. The budgets below are the target, not that reading: the story fails
  until the nodes earn it.

  Each node runs two processes side by side: a storage service — the "storage
  peer" in the steps below — and a Holochain conductor. One node, one storage
  peer, one conductor. A "conductor call" is one request from a node's storage
  service to the conductor beside it. It is the costly hop — each one crosses
  a socket, runs WebAssembly and reads the database. A node admits at most five
  such calls at once and refuses the sixth; a "refused permit" is that refusal.
  A node with nothing to do should never have five in flight.

  The call budget averages 6 a minute across the whole five-minute window. It
  leaves room for the one thing a quiet node
  legitimately does — tell its peers once a minute that it is alive — plus a
  bounded periodic check, and nothing else. The processor budget of 10 seconds a
  minute is likewise a five-minute average: the sum across all three conductors,
  read from the operating system's own accounting of each conductor process.
  Ten is one sixth of one core for the whole household; sixty would be one core
  pinned.

  "Nothing to do" means no person or external client is asking the household to
  author, read or sync. Autonomous maintenance still runs, and this story measures
  all of the calls and processor time that maintenance invents for itself.

  After setup confirms the three peers are in sync, two snapshots bracket the same
  300-second observation window. Cumulative counters are compared by difference,
  and per-minute rates use the window's actual elapsed time. Every table value is
  an inclusive maximum: calls and refused permits are judged separately for each
  peer, while processor time is the total across the household. The owned-lane
  lease is the precondition that the harness and its user introduce no workload;
  the story does not claim to detect every possible outside request. Autonomous
  work remains visible. If an owned conductor is missing or restarts, the
  observation is invalid and fails instead of reporting that process as zero.

  Background:
    Given the household's three storage peer/conductor pairs
    And every storage peer reports its content in sync

  Scenario: With nothing to do, the nodes do almost nothing
    When no person or external client authors, reads or syncs for 300 seconds
    Then the quiet household stayed within its resource budgets:
      | average conductor calls per peer per minute |  6 |
      | refused conductor permits per peer           |  0 |
      | average household CPU seconds per minute     | 10 |

  @wip
  Scenario: With every node's posture declared and nothing to do, the nodes still do almost nothing
    A posture is a signed record a node keeps of what it is willing to do for the other nodes. A
    node can offer two roles. A node that hosts live sessions, whenever a session is open, passes
    along to everyone in that session who has the shared document open and where each person's
    cursor is. A node that carries content stores copies of the other nodes' content and hands a
    copy to any node that asks for one. Each role is real work, but only while someone uses it: an
    open session to pass messages for, or a node asking for a copy. A node takes on a role only when
    its person declares it, and its posture is rewritten only when that choice changes, never on a
    timer. A node that offers a role nobody is using must cost no more than a node that offers
    nothing, so the budgets are the same as when no posture is declared.
    Given Matthew has declared in his node's signed posture that it will host live sessions for the household and carry content for the other nodes
    And Jessica has declared in her node's signed posture that it will carry content for the other nodes
    And James has declared in his node's signed posture that it offers nothing to the other nodes
    And no live session is open on any node
    And Matthew, Jessica and James each leave that declared choice unchanged for the 300 seconds
    When no person or external client authors, reads or syncs for 300 seconds
    Then the quiet household stayed within the same resource budgets as a household with no posture declared, because a role nobody uses costs nothing:
      | average conductor calls per peer per minute |  6 |
      | refused conductor permits per peer           |  0 |
      | average household CPU seconds per minute     | 10 |
    And no node rewrote its posture during the 300 seconds, because a posture changes only when its person's choice changes
    And no node sent a live-session message during the 300 seconds
