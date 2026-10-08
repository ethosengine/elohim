@e2e @observation @concern:witness-rows-signed @act:i @requires:owned-substrate
Feature: Observed without a doorway — a witness row lands on a peer with nothing else reachable
  As a person whose peer holds her own key
  I want my observations to land, be signed and reach the other peers in my household while every doorway is down
  So that being seen by my own household never depends on a gateway being up

  The words this story uses, so it can be read cold:

  - A HOUSEHOLD is a small group of people and their peers sharing one MESH
    (the peers and the network they share, with no server in between); here
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

  - An OBSERVATION is one row a peer writes about something it saw, under a
    declared KIND. A kind declares its REACH (how far the row may travel:
    household-reach rows reach the household's peers and no further) and its
    WINDOW (how long the other peers in the household may take to receive it).
    A WITNESS ROW is an observation about something other than the observer. The witness row this story uses is a
    REACHABILITY CHECK: Jessica's peer asked Matthew's peer whether it was up
    and records the answer under the kind "infrastructure:peer-reachable",
    which declares household reach and a window of one minute. Peers hold
    rows by signature: a row whose signature they already hold is the same
    row, which is how a duplicate is recognised.
  - The DIRECT PATH is the app's connection to its own peer's port on the same
    machine, not through a doorway. The PEER-TO-PEER PLANE is the network
    between the household's peers, the path rows travel without any doorway.
    Peers accept rows signed by a known agent key; that is why signing is in
    the promise.
  - A person's STREAM is the view of their own observation log served by their
    own peer. A peer's PROJECTION is the table it builds from the rows it has
    received from others; a CURSOR is the position of the latest row of one
    observer's log that a projection has reached.
  - A doorway that returns after downtime CATCHES UP by asking the peers what it
    missed and may push what it learned back to peers; the risk this story
    guards against is that it re-posts rows the peers already hold, leaving
    duplicates in their projections.

  Background:
    Given the household mesh is running
    And both household doorways are stopped

  @wip
  Scenario: A household-reach observation lands and travels with every doorway stopped
    Given Jessica's app reaches her own peer on its direct path
    When Jessica's peer records a witness row of kind "infrastructure:peer-reachable" saying Matthew's peer answered its reachability check
    Then Jessica's peer stores it signed by her agent key
    And it appears in her stream served by her own peer
    And Jessica's cursor in Matthew's projection advances past that row over the peer-to-peer plane within one minute
    And Jessica's cursor in James's projection advances past that row within the same minute

  @wip
  Scenario: A returning doorway replays nothing the peers already hold
    Given Jessica's peer holds one signed household-reach witness row that Matthew's projection has already received
    When Matthew's doorway returns to service and catches up
    Then the doorway pushes no observation row back to the peers
    And Matthew's projection holds exactly one row for that observation, carrying Jessica's signature
