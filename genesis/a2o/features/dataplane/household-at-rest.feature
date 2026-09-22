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
