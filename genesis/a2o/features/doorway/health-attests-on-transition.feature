@e2e @doorway @concern:health-attests-on-transition @act:i @requires:owned-substrate
Feature: Health attests on transition — a peer's health reaches the ledger only when it changes, and only through witnesses
  As the steward of a household
  I want my doorway to notarize a peer's health only when a status period closes or changes
  So that an idle network writes nothing, a lone probe never becomes a record, and silence reads as unobserved

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

  - Each doorway PROBES the peers it serves every few minutes: it asks the
    peer's conductor whether it is up, and the conductor ANSWERS or does not.
    Both doorways probe all three peers. Today every probe writes a record to
    the shared ledger; this story says that must stop.
  - A PROBE OBSERVATION is the row a doorway writes on its own observation log
    about one probe: which peer, up or not, when. Rows stay on that log, the
    OBSERVATION SUBSTRATE, where a household member reads them through the
    PERIOD READER, the view that shows each peer's open period and the periods
    that closed, with start, end and observation count. When a period is open
    and no observation has arrived, the period reader reports "unobserved" as
    a status of its own, not as a blank.
  - The INFRASTRUCTURE CHAIN is the part of the shared ledger (the notary, a
    Holochain DHT) that holds machine records such as health. An ACTION is one
    write to it.
  - A PERIOD is the span between two health transitions of one peer. A WINDOW
    is the span of time one scenario watches. One record closes each period.
    Silence after a period's end reads "unobserved", never "healthy".
  - A KIND is a declared type of observation (here, the doorway's health
    probe) whose declaration in the application's manifest fixes its reach,
    its retention and its graduation threshold.
  - GRADUATION is the step that turns probe observations into one record on the
    ledger: it runs when a status changed and enough DISTINCT HOUSEHOLDS (three,
    as the kind declares) observed it. One household can never meet that bar by
    itself, so on this household mesh a transition closes a period on the
    observation substrate and nothing reaches the ledger; the ledger record is
    proven with observations from three households supplied as fixtures. A
    lone probe, or any number of probes from one household, never becomes a
    record. The rule holds in both directions: a peer going down closes a
    period exactly as a peer coming back does.

  Background:
    Given the household mesh is running with Matthew's and Jessica's doorways
    And James's conductor was stopped before this window began

  @wip
  Scenario: Twenty unchanged minutes write nothing to the ledger
    When twenty minutes pass with no change in James's status
    Then the infrastructure chain gains zero actions from any doorway in that window
    And Matthew reads James's health as "unobserved" from his doorway's observation substrate

  @wip
  Scenario: A return closes exactly one period on the substrate, and the ledger still gains nothing
    When James's conductor returns and answers the next probe from each doorway
    Then exactly one period closes for James on the observation substrate in the window
    And the period reader shows that period to the household with its start, its end and the count of observations made within it
    And the infrastructure chain gains zero actions, because one household cannot meet the graduation threshold

  @wip
  Scenario: Going down closes a period the same way coming back does
    Given James's conductor was restarted after the Background and has been answering probes for an hour
    When James's conductor is stopped and misses the next probe from each doorway
    Then exactly one period closes for James on the observation substrate
    And the period reader shows James as "unobserved" from that point

  @wip
  Scenario: One household's probes never graduate, however many there are
    Given five probe observations of one peer's transition, all written by doorways in this one household
    When graduation runs over them
    Then no health record is issued to the ledger
    And graduation's stated reason is that too few distinct households observed the change

  @wip
  Scenario: Three households' probes graduate into one ledger record
    Given probe observations of one peer's transition written by doorways in three distinct households, supplied as fixtures
    When graduation runs over them
    Then exactly one health record for that transition is issued to the infrastructure chain
    And the record names the period's start, its end and the three households that observed it
