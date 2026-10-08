# This story and sensor-labels-bounded.feature together prove the concern
# sensor-labels-bounded: that story covers the labels on a peer's gauges, this
# one covers the surfaces those gauges and rows are read through.
@e2e @observation @concern:sensor-labels-bounded @act:i
Feature: An outside observer learns nothing — participation is not readable from the observation plane
  As a person in a household
  I want anyone outside my household to learn nothing from my peer's observation surfaces about whether I participate, with whom, or how much
  So that the plane that lets my household see itself is not a window for strangers

  This story is one half of the concern "sensor-labels-bounded": the sibling story,
  sensor-labels-bounded.feature, proves that no gauge carries a label naming a person;
  this story proves that none of the surfaces those gauges and rows are read through
  gives an outsider a person or a count. Both carry the one concern tag because the
  habit they prove is one invariant with two faces.

  The words this story uses, so it can be read cold:

  - A HOUSEHOLD is a small group of people and their peers sharing one mesh; here
    the test household of Matthew, Jessica and James. Each person's PEER is two
    processes on their own machine: a STORAGE PEER that holds and serves data,
    and a CONDUCTOR that holds their keys and their source chain (their own
    tamper-evident log of actions). The household runs two DOORWAYS, Matthew's
    and Jessica's: gateways a browser uses to reach the household from outside.
  - A FIXTURE HUMAN is one of the test cast (Matthew, Jessica, James; and on
    the alpha fleet, Adam, who is not a member of this household) with a
    known AGENT KEY, the public key that names a person on the network; agent
    keys begin with the letters "uhCAk".
  - Tags: "act:i" marks a scenario that runs on this household mesh alone;
    "act:ii" marks one that needs the alpha fleet, the project's seven-peer
    test neighbourhood; "requires:owned-substrate" means the scenario must own
    the peers it stops and starts.

  - An OBSERVATION is one row a peer writes about something it saw, under a
    declared KIND. Each kind declares a REACH: AGENT-PRIVATE rows never leave
    the peer that wrote them; HOUSEHOLD rows reach the household's peers; PUBLIC
    rows may be served to anyone and by design name no person and no count.
  - A GAUGE is a named number a peer publishes about its own running state
    (rows held, bytes stored, uptime). The observation plane's SURFACES are:
    the EXPOSITION (the peer's gauges as text), the BY-SUBJECT VIEW (rows about one subject), the DIVERSITY VIEW
    (how many distinct observers and households have rows per kind), and the
    NETWORK POSTURE (each peer's declared lifecycle and flags).
  - Peers share rows by TOPIC, one topic per kind: a subscriber receives a
    CURSOR ANNOUNCEMENT when a new row is available, then fetches it into its
    PROJECTION, the table it builds from received rows. The HOUSEHOLD NAMESPACE
    is the set of topics scoped to this household.
  - An OUTSIDE OBSERVER is any caller with no household binding: an anonymous
    request, or a peer that is not a member. PUBLIC rows are a standing
    property of every peer (its public posture and counts of its own machine
    kinds); they exist without any scenario creating them, and they reach an
    outsider through the read surfaces and a separate public topic, never
    through the household namespace's topics. A PER-PERSON COUNT is any
    number from which a participant's identity or the number of participants
    could be inferred; a count of a peer's own machine kinds is not one.
    A scenario inherits the feature's act tag unless it declares its own.

  @wip
  Scenario: Anonymous and non-member reads reveal no participant and no count
    Given Jessica's peer holds agent-private and household observations
    When an anonymous caller reads the exposition, the by-subject view, the diversity view and the network posture
    And a peer with no household binding reads the same surfaces
    Then no response names an agent key
    And no response carries a per-person count
    And a request for a kind whose reach is household is refused with a reason
    And the refusal does not reveal whether that kind exists

  @wip
  Scenario: No cursor announcement reaches a non-member peer
    Given a peer with no household binding subscribes to the household namespace's topics
    When Jessica's peer appends a household row
    Then the non-member peer receives no cursor announcement for it
    And the non-member peer's projection holds zero rows for the household namespace

  @wip @act:ii
  Scenario: A fleet peer that does not host this household learns nothing about it
    Given Adam's peer on the alpha fleet is not bound to the test household
    When Adam's peer reads the exposition, the by-subject view, the diversity view and the network posture of Jessica's peer
    And Adam's peer subscribes to the household namespace's topics
    Then no response names an agent key
    And no response carries a per-person count
    And Adam's peer receives no cursor announcement for any household row
    And the only rows Adam's peer receives are public rows
    And no public row it receives names an agent key or carries a per-person count
