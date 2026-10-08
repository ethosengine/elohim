@e2e @observation @concern:evidence-class-descends @act:i
Feature: Evidence class descends — fixture evidence can never become real
  As a steward of the household test mesh
  I want every observation, attestation and summary event to say whether a fixture or a real participant produced it
  So that seeded evidence can never settle as if a person had earned it

  The words this story uses, so it can be read cold:

  - A HOUSEHOLD is a small group of people and their peers sharing one mesh; here
    the test household of Matthew, Jessica and James. Each person's PEER is two
    processes on their own machine: a STORAGE PEER that holds and serves data,
    and a CONDUCTOR that holds their keys and their source chain (their own
    tamper-evident log of actions). The household runs two DOORWAYS, Matthew's
    and Jessica's: gateways a browser uses to reach the household from outside.
  - A FIXTURE HUMAN is one of the test cast (Matthew, Jessica, James) with a
    known AGENT KEY, the public key that names a person on the network; agent
    keys begin with the letters "uhCAk".
  - Tags: "act:i" marks a scenario that runs on this household mesh alone;
    "act:ii" marks one that needs the alpha fleet, the project's seven-peer
    test neighbourhood; "requires:owned-substrate" means the scenario must own
    the peers it stops and starts.

  - A STEWARD is the person responsible for running the household's peers
    and for the integrity of what they record. A CLIENT is any caller of a
    peer's write surface: a person's app, or a program.
  - An OBSERVATION is one row a peer writes about something it saw, under a
    DECLARED KIND (a kind the application's manifest lists, for example a
    peer's reachability check or a learning-session completion). An ATTESTATION is
    a signed claim kept on the shared ledger (the household's distributed hash
    table, the notary).
    A SUMMARY ECONOMIC EVENT is the other record that can be made from
    observations: a tally of work done or resources used that can SETTLE, that
    is, count toward what someone is owed. Both cite the observations they were
    made from as their EVIDENCE.
  - GRADUATION is the step that turns a set of observations into an
    attestation or a summary economic event.
  - An EVIDENCE CLASS is one of two words on every row and record: "fixture"
    (produced by the test cast or a seeder) or "real" (produced by a person).
    Every observation a fixture human produces carries "fixture" at write time.
    The class is set once and may only DESCEND: a record made from fixture
    evidence is fixture, and nothing turns fixture into real; a real row may be
    reclassified fixture, never the reverse. When graduation runs over a mixed
    batch, the result takes the lowest class present: one fixture row makes
    the record fixture. The GRADUATION EVALUATOR is the peer's own process that
    runs graduation; a steward may also run it by hand. "wip" on a scenario
    means its steps are not yet wired.

  Background:
    Given the household mesh is running with fixture humans Matthew, Jessica and James
    And a real participant, Ruth, has joined the household with her own key

  @wip
  Scenario: Fixture evidence stays fixture through graduation, in attestations and summary events
    Given the household's fixture humans have produced observations of a declared kind
    When the graduation evaluator runs over them
    Then every attestation read back carries "evidence_class: fixture"
    And the summary economic event read back carries "evidence_class: fixture" and cannot settle
    And no summary economic event of class "real" cites a fixture observation in its evidence

  @wip
  Scenario: Real evidence graduates real
    Given Ruth's peer has produced observations of a declared kind, each carrying "evidence_class: real"
    When the graduation evaluator runs over them
    Then the attestation read back carries "evidence_class: real"
    And the summary economic event read back carries "evidence_class: real"

  @wip
  Scenario: A mixed batch descends to fixture
    Given a batch holding Ruth's real observations and one fixture observation of the same declared kind
    When the graduation evaluator runs over the batch
    Then the attestation read back carries "evidence_class: fixture"
    And the summary economic event read back carries "evidence_class: fixture" and cannot settle
    And no summary economic event of class "real" is made from that batch

  @wip
  Scenario: A fixture row cannot be promoted to real
    Given a fixture observation stored on Jessica's peer
    When a client attempts to reclassify that row as "real"
    Then the peer refuses the change with a reason naming "evidence_class"
    And the row still reads "evidence_class: fixture"

  @wip
  Scenario: A row with no class is refused, never defaulted
    Given a client posts an observation with no evidence class to Jessica's peer
    When the peer evaluates the write
    Then the write is refused with a reason naming "evidence_class"
    And no row is stored
