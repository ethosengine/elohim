@e2e @observation @concern:sensor-labels-bounded @act:i @requires:owned-substrate
Feature: Sensor labels are bounded — a peer's self-sense never names a person
  As the steward of a household peer
  I want every gauge my peer and its conductor serve to carry only bounded, machine labels
  So that a monitoring tool reading my fleet can never become a ledger of who did what

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

  - SELF-SENSE is what a peer keeps about itself: its own gauges. An EXPOSITION
    is the endpoint where a process serves those gauges as text; a monitoring
    tool reads it by SCRAPING it. Each household peer has three expositions:
    the storage peer's, the doorway's, and the conductor's, the last bound to
    LOOPBACK (reachable only from the same machine). A monitoring tool such as
    PROMETHEUS reads expositions by scraping them and stores the series.
  - A SERIES is one named gauge with its set of LABELS (key=value pairs that
    say what the number is about). The ZOME-CALL DURATION series measures how
    long each function call inside the conductor took; the EMITTED-SIGNAL
    series counts the signals the conductor sent to its app.
  - A PARTICIPANT LABEL is any label value that names a person or their cell:
    an agent key, or a CELL ID (the pairing of a person's key with one
    application). The rule under test is that no series carries one.
  - The ENVELOPE is the supervisor process the household mesh launches each
    conductor under, so the harness can read its state (the `ark` binary).
  - The conductor's own instruments are meant never to attach a participant
    label. The exposition still FILTERS labels as a second line of defence,
    so a label that does slip through is removed and counted, never served.
    "requires:observability" marks the one scenario that needs the fleet's
    monitoring stack rather than a household peer.
  - This feature tests only the participant-label boundary. Sibling features in
    this directory cover witnessing, graduation and the observation plane.

  @wip
  Scenario: No exposition on a household peer carries a participant label
    Given the household mesh is running with its conductors under the envelope
    And each conductor's exposition is listening on its loopback metrics port
    When I scrape the storage exposition, the doorway exposition and the conductor exposition on every peer
    Then no label value on any series equals a fixture human's agent key
    And no label key on any series is "agent" or "cell_id"
    And every label key on the zome-call duration series is one of "dna_hash", "zome", "fn"

  @wip
  Scenario: A refused label is counted by name rather than silently dropped
    Given a conductor whose zome-call duration series carried an "agent" label before the exposition filtered it
    When the exposition is scraped
    Then the series is served without the "agent" label
    And the exposition's own counter of refused label keys shows "agent" incremented by one

  @wip @act:ii @requires:observability
  Scenario: The fleet's Prometheus holds no per-participant series for the conductor
    Given the alpha fleet's conductors are scraped by its monitoring stack
    When Prometheus is asked for the label set of the zome-call duration series
    Then the label "agent" is absent from that set
    And the label "cell_id" is absent from the emitted-signal series
