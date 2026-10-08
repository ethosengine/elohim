@e2e @devflow @concern:ledgers-fold-once @act:host
Feature: Ledgers fold once — every dev-system witness is one fold on one store
  As the operator of this workspace
  I want every dev-system witness (a berth claim and its release, a guard shed, a finding opened and closed, a quiesce verdict) to be one flow event on the one sidecar store
  So that one report reads them all and no script keeps its own ledger with its own cursor

  The words this story uses, so it can be read cold:

  - The DEV SYSTEM is the tooling that watches this repository's own health: the
    workspace BERTH (the lease that says which session may use the shared mesh or
    the build pool), the RAM and IO GUARDS (daemons that watch memory and disk
    pressure), the CI and runtime HARVESTERS (scripts that poll the build server
    and the live fleet for failures), and the QUIESCE TIMELINE (the record of how
    long the fleet took to settle after each deploy).
  - A guard SHEDS a process when it pauses or terminates a child to bring memory
    or disk pressure back under its budget. The shed is a witness: who, what, why.
  - A FINDING is one failure a harvester noticed. It is OPENED when first seen and
    CLOSED when it disappears. Opening and closing are two separate witnesses.
  - A QUIESCE VERDICT is the harvester's reading of one deploy's settle: how long
    it took, whether the reading can be trusted, and which legs blocked it.
  - EPR is the protocol's record type (Elohim Protocol Record: knowledge, value
    and governance travelling together); the EPR CLI (`epr`) is the workspace
    command that writes and reads records and their observations.
  - The SIDECAR STORE is a file-based event log that rides beside the repository
    without being part of its committed content (`.eprfs/status/flows.jsonl`).
    A FOLD is one observation written onto it through `epr flow note` and read
    back by `epr flow report`.
  - The tag "act:host" means this scenario runs on the workspace host itself,
    not on a household mesh or the fleet.
  - A BOUND is a declared ceiling the report checks. `ledger-planes-ceiling@1`
    names the ceiling (version 1) on the number of LEDGER PLANES: append-only
    files that a script other than their own writer parses with its own cursor
    and closure rule. The rule under test is that there is exactly one.
  - The DELIVERY SCOREBOARD is the one-line verdict per build job the operator
    reads at session start. "Derived from" means it is computed from the report,
    never from a file of its own.

  @wip
  Scenario: Every kind of dev-system witness lands as one fold on one store
    Given a fresh, empty repository with the epr CLI and the dev-system tools installed
    And each tool is driven for real in this scenario, not simulated through the CLI
    When the berth tool claims a lease
    And the berth tool releases that lease
    And the RAM guard sheds a child process for exceeding its memory budget
    And the IO guard sheds a child process for exceeding its disk-write budget
    And a harvester opens a finding
    And that harvester closes the same finding
    And a quiesce verdict is recorded
    Then each of those seven witnesses is exactly one fold on the sidecar store
    And the number of ledger planes reported by "epr flow report --bound ledger-planes-ceiling@1" is 1
    And the delivery scoreboard's verdicts equal the verdicts in "epr flow report --json"
    And the delivery scoreboard opened no file other than the sidecar store to produce them
