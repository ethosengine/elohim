@e2e @observation @concern:witness-rows-signed @act:i
Feature: Witness rows are signed, or say that they are not
  As a person whose peer records what it witnesses
  I want every row on my observation log to carry a signature my peer verified, or to say plainly it has none
  So that nothing built on my witness can quietly stand on a row nobody signed

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

  - An OBSERVATION is one row a peer writes about something it saw: who saw
    (the OBSERVER), what, of whom, when, under which declared KIND. Rows live
    on the observer's own append-only OBSERVATION LOG. A WITNESS ROW is an
    observation about something other than the observer itself. The witness
    row this story uses is a REACHABILITY CHECK: Jessica's peer asked
    Matthew's peer whether it was up, and records the answer under the kind
    "infrastructure:peer-reachable".
  - Tags not listed above are test-harness filters: "wip" means the steps are
    not yet wired; "browser-only" means the scenario needs a driven browser.
  - A person's STREAM is the time-ordered view of their own observation log,
    served by their own peer to them alone.
  - Two client postures: in a DIRECT SESSION the person's APP talks to their
    own peer, which holds their key and signs; in a HOSTED SESSION the
    person's BROWSER talks to a doorway that holds their key and signs on their
    behalf. The two paths sign differently, so each is proven on its own.
  - The peer's acknowledgement names the signing state ("signed: agent" when
    signed by the agent key, "signed: absent" when not); the stream marks
    unsigned rows "signature: absent" so a reader sees the state without
    re-verifying; a row posted with an empty signature is what the peer
    classifies as absent. A row reaches the log one of two ways: the peer
    records it itself, or the person's app POSTS it to the peer; either way the
    peer is the write authority and classifies the signature. The ROUTING
    RULE: a row is posted to, and lives on, the observer's own peer; nothing
    routes it to its subject's peer.
  - Every row either carries a SIGNATURE the peer verified against the
    observer's key, or is marked ABSENT. A row that carries a signature which
    fails verification is refused before it reaches the log; that contract is
    not this story's.
  - The DIVERSITY SUMMARY counts, per kind, how many distinct observers and
    distinct households have rows in one window. GRADUATION is the step that
    reads it: when enough distinct observers agree that a status changed, one
    record of the change is issued to the shared ledger. An unsigned row counts
    toward no summary and so graduates to nothing.

  Background:
    Given the household mesh is running
    And Jessica is a member of the household whose peer holds her agent key

  @wip
  Scenario: A witness row is acknowledged as signed by the agent key
    Given Jessica is signed in to a direct session with her own peer
    When Jessica's peer records that Matthew's peer answered its reachability check
    Then her peer acknowledges the row with "signed: agent"
    And her stream carries no "signature: absent" line for that row
    And the row counts toward the diversity summary for its kind

  @wip
  Scenario: An unsigned row is accepted, labelled, and excluded from every threshold
    Given Jessica's conductor is unreachable, so nothing can sign on her behalf
    And Jessica's app posts a reachability-check row with an empty signature to Jessica's peer
    When the peer acknowledges it
    Then the acknowledgement says "signed: absent"
    And the row is visible in Jessica's stream marked "signature: absent"
    And the row is counted in no diversity summary and graduates to nothing

  @wip @browser-only
  Scenario: The browser client signs with the key its session holds
    Given Jessica's browser is in a hosted session with the doorway that holds her key
    When Jessica's browser records that Matthew's peer answered its reachability check
    Then the row's observer is Jessica's agent key
    And the signature verifies against the key the doorway holds for her
    And her peer acknowledges the row with "signed: agent"
