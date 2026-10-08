@e2e @observation @concern:liveness-has-one-source @act:i
Feature: Liveness has one source — a killed peer leaves placement by presence, and nobody writes that it left
  As the steward of a household
  I want "is this peer here now" to have one writer on each peer, the transport's own presence
  So that a peer that dies is noticed by its siblings within one window and no chain action records a liveness fact

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

  - The TRANSPORT is the networking layer inside the storage peer that holds
    the connections to the other peers; it is the one process that can know,
    moment to moment, who is connected.
  - A STEWARD is the person responsible for running and watching a
    household's peers; here, the one reading the household's views.
  - PRESENCE is the transport's own knowledge that a peer is connected, with a
    short time-to-live (35 seconds). The transport ARMS presence automatically
    when it starts; presence is UNARMED only when a transport that cannot
    report liveness is the one in use (for example a transport that only
    forwards through a relay and keeps no direct connections). A peer's liveness is one of three readings: present, absent, or
    unmeasured. A LIVENESS CLAIM would be any field or record asserting that a
    peer is alive; the rule is that no such claim exists anywhere except the
    transport's own presence.
  - POSTURE is what a peer declares about itself (its lifecycle stage,
    whether it offers its spare capacity to the household pool, and the size
    class of machine it runs on), written only when it changes. The NETWORK POSTURE READ is the
    view of every peer's posture as the household's peers see it.
  - PLACEMENT is the household's current decision about which peers hold and
    serve copies (SHARDS) of its data. A peer that leaves placement is no longer
    chosen to receive or serve shards. Peers are SETTLED when every shard has
    the copies it was promised and nothing is still moving. After a peer
    leaves, the others RE-CONVERGE: they re-place the shards it held, which on
    this household takes a few minutes.
  - "Killing a peer" in this story kills both of its processes, the storage
    peer and the conductor.
  - The RESILIENCE CARD is the computed summary of each peer's health (presence,
    posture, shard responsibility) the household reads.
  - The INFRASTRUCTURE CHAIN is the part of the shared ledger that holds
    machine records; an ACTION is one write to it. The rule under test is that
    liveness never produces a write on any part of the shared ledger, by any
    author; the infrastructure chain is where such a write would land, and
    every chain is checked. Each person's source chain is part of the shared
    ledger; the infrastructure chain is the part that holds machine records.

  @wip @requires:owned-substrate
  Scenario: A killed peer leaves placement within the presence window and nothing is notarized
    Given three household peers are settled and holding each other's shards
    When James's peer is killed
    Then James's peer leaves shard placement within 35 seconds
    And James's resilience card reads absent by presence, posture unchanged since his last declaration, and no shard responsibility
    And the network posture read shows James's last declared posture, with no liveness claim beside it
    And the two survivors are still in placement twenty minutes later, long after re-convergence finished
    And in the twenty minutes since the kill, zero actions about liveness were written to any chain of the shared ledger by any author

  @wip
  Scenario: Unarmed presence reads unmeasured, never alive
    Given a peer whose transport has not armed presence
    When the resilience card is computed
    Then the peer's liveness reads "unmeasured"
    And it is neither counted as present nor as absent in placement
