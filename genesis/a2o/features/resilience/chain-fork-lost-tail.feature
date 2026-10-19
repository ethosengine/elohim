@e2e @resilience @chaos @dataplane @concern:chain-fork-recovery @act:i @requires:household-nodes
Feature: A peer that loses its last published action — replicate the chain fork, then prove each cure
  As the steward who operates a household mesh whose nodes can lose power
  I want the chain fork that a lost write causes to be reproducible on demand
  So that every cure is proven against the same failure, not argued from one bad night

  Every peer keeps a source chain per DNA: its own numbered, signed actions, seq 0, 1, 2 and on.
  Each action is published to the other peers, which hold copies as authorities. A chain fork is
  two different actions by one author at the same seq. The protocol cannot tell an honest node
  that lost a write from a forger, so it treats both the same way. An authority that sees both
  actions writes a chain-fork warrant against the author's key, and from then on every authority
  that has validated the warrant drops all of that key's traffic in that DNA, and content the key
  authored loses its "notarized" trust, the level that says the network witnessed its author. Today nothing ends
  or lifts that block; this feature calls it exile. Exile is per DNA: each DNA is its own network
  with its own chains, so a key forked in lamad is cut off in lamad and keeps working in the rest.
  Lamad is the content DNA: learning content and apps live there, so it is where a peer authors
  most and where the drill forks. The other four (imagodei for identity, mishpat for governance,
  infrastructure and node_registry for the network's own records) carry no content and serve as
  the control.

  How an honest node forks: the conductor (the Holochain runtime process on each node, which
  hosts the DNAs and keeps their chains) commits each action to SQLite in write-ahead-log mode
  at synchronous=Normal. At that setting, a committed action survives a process crash, but a
  power cut can roll it back. If the action had already been published, its author loses it
  locally while the authorities keep it, and the author's next action re-uses that seq: a fork.
  This happened to the household peer james on 2026-10-18.

  The household is the Dowell family's own three-node mesh: matthew, jessica and james, each a
  storage peer (one machine running a conductor, plus a storage service that serves content over
  HTTP and plays no part in this failure), with doorways (gateway services) in front that serve
  the household's apps. Recasting the household means wiping every peer's identity and starting
  fresh; on 2026-10-18 that was the only way to clear the exile.

  The lost-tail drill reproduces the same chain shape without cutting power; scenarios that say
  "by running the lost-tail drill" run these four steps and wait for the exile Scenario 1 asserts:
  1. Stop a drill peer's conductor, copy every database file in its data directory, and start it
     again (the snapshot).
  2. Let it author one action and publish it.
  3. Stop it and put the copied files back in place of the current ones, so the action is gone
     from its own chain.
  4. Restart it and let it author again.

  A drill peer is a fresh, disposable peer joined to the running household for one drill and
  never used again. It needs the household mesh running (the household-nodes capability), and its subject is never a
  household member. A successful replication exiles a key forever, so the
  subject is always a fresh peer that joined the running household with its own conductor and
  storage, running all five DNAs (`just mesh join-peer forkdrill-<run>`). Each scenario that needs
  a drill peer joins its own; no scenario depends on another's leftovers.

  The scenarios, in order:
    the fork is reproducible, and the exile is permanent and touches no household member
    exile is named for what it is, not mistaken for lag
    the warrant says at which seq the chain forked
    a peer that comes back behind its own published chain re-imports instead of re-authoring
    an action is durable on disk before any other peer can hold it
    a forked peer can come back under a new key
  The last one waits on a design decision, called D1 below. A forked key can return in one of two ways:
  - re-key with lineage: the peer continues under a new key that a KeyRotation entry binds to the
    same identity, naming the warranted key as its predecessor. This is upstream Holochain's model,
    where a validated fork is unrecoverable for that key.
  - operator unblock: every authority lifts its block on the old key by hand. There is no upstream
    precedent for this.
  Re-key keeps the protocol's defence against forgers intact: the forked key stays warranted.
  Unblock restores the old key, at the price of accepting a chain the network has proven forked.
  D1 is open as of 2026-10-19. The household's steward decides it; the backlog item steward-key-chain-fork-warrant-exile
  records the ruling and maps each scenario to the concern it closes.

  Every time bound below is headroom over the 2026-10-18 measurements recorded after this
  description (for example, 10 minutes for a warrant that took 5 min 37 s).

  # Measured 2026-10-18 on the household (the regression bounds below):
  # - The warrant arrived 5 min 37 s after the first gossip (the peers' routine exchange of data) following the fork.
  # - Five warrant lines, one action pair.
  # - Blocked traffic was confined to lamad: 221,069 of the 601,060 messages james sent matthew;
  #   under 10 in each other DNA.
  # - 0 completed lamad gossip rounds with james, against 43–49 in each other DNA.
  # - A 30-minute settle wait measured nothing; only a full recast (MESH_RESET=1) cleared it.
  # - Informs: the conductor's database durability setting (its disk-write cost is still to be
  #   measured), the exile detector's thresholds, and the runbook for a peer that "lags" after a
  #   hard stop.
  # - Review after: every conductor pin move, any change to the durability setting, and the D1 ruling.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the household mesh is three storage peers, matthew, jessica and james
    And each household conductor runs five DNAs: lamad, imagodei, infrastructure, node_registry and mishpat
    # The doorway line is every a2o feature's required harness setup (genesis/a2o/CLAUDE.md); it
    # binds a URL and makes no call, and these scenarios route nothing through it.

  # If this ever stops reproducing without a cure having landed, the constraint has moved, not
  # disappeared; re-read the conductor pin before deleting this scenario.
  @wip @regression
  Scenario: A drill peer that loses its last published action forks its chain and is exiled
    A PASS here does not mean the household is resilient. It confirms the failure is still real on
    today's conductor: an honest lost write still ends in exile. The cure scenarios below are the
    ones that should turn green.

    Given a fresh drill peer has joined the household and every household peer lists its agent in all five DNAs
    And the drill peer's conductor databases are snapshotted
    When the drill peer adds one content entry (a piece of learning content), which its conductor commits as one action on its lamad chain
    And within 120 seconds matthew's and jessica's conductors hold that action
    And the drill peer's conductor is stopped and its databases are restored to the snapshot, so its own chain no longer holds that action
    And the drill peer's conductor starts and commits a different content entry at the same seq
    Then within 10 minutes at least one of the three household conductors writes a chain-fork warrant naming the drill peer's agent key, for exactly one action pair
    And over a 60-second observation, matthew's and jessica's count of blocked lamad messages from the drill peer rises
    And over that same 60-second observation, their count of blocked messages from the drill peer stays below 10 in each other DNA
    And neither matthew's nor jessica's lamad peer list includes the drill peer's agent
    And 10 minutes after it was committed, the drill peer's second content entry has not reached matthew or jessica

  @wip @regression
  Scenario: The exile survives a restart and touches no household member
    Like the scenario above, a PASS confirms the failure: restarting does not end the exile. Its
    last two steps are the drill's safety check, which must pass whatever the conductor does.

    Given a fresh drill peer has joined the household and been exiled by running the lost-tail drill on it
    When the drill peer's conductor is restarted
    Then 10 minutes later matthew's and jessica's lamad peer lists still exclude the drill peer's agent
    And their count of blocked lamad messages from the drill peer is still rising
    And matthew, jessica and james keep the agent keys they had before the drill
    And a content entry james adds still reaches matthew and jessica within 120 seconds

  @wip
  Scenario: The household names an exiled peer instead of calling it slow
    The steward meets exile in three places: the status they read by hand, the preflight that
    gates every household start, and the test lanes. All three must say "exiled", not "slow".

    Given a fresh drill peer has joined the household and been exiled by running the lost-tail drill on it
    When the steward reads the household mesh status
    Then the status shows the drill peer as EXILED in lamad, with matthew's and jessica's blocked and received message counts
    And mesh preflight refuses with a line naming the drill peer and lamad
    And a test lane that waits for every peer to answer about content the drill peer authored fails by naming the drill peer as exiled, not by timing out as if the peer were slow

  @wip
  Scenario: The warrant shows whose chain forked, how, and at which seq
    Given a fresh drill peer has joined the household and been exiled by running the lost-tail drill on it
    When the steward reads matthew's conductor diagnostics
    Then they list a validated warrant against the drill peer's agent key whose kind is chain fork
    And the warrant's seq is the seq of the content entry the snapshot restore erased
    And matthew's conductor log line for that warrant carries the same seq

  @wip
  Scenario: A drill peer that comes back behind its own published chain re-imports before it authors
    This proves a cure that does not exist yet: a conductor that, after an unclean stop, asks the
    other peers for its own chain head before authoring anything, and copies back any actions
    they hold that it lost. The cure ships as a conductor binary; the drill peer is joined on that
    binary while the household keeps its own; its design is tracked in the backlog item
    steward-key-chain-fork-warrant-exile. On today's conductor, which lacks the cure, this
    scenario fails: the drill peer forks and is exiled, exactly as the first scenario shows. When
    the cure ships, it passes instead.

    # This binary does not exist yet: until the cure ships, this step cannot be satisfied and the
    # scenario stays @wip. It is a proof obligation for the cure, not something the harness provides.
    Given a fresh drill peer has joined the household running a conductor binary built with the re-import cure
    And every household peer lists its agent in all five DNAs
    And the drill peer's conductor databases are snapshotted
    When the drill peer adds one content entry, which its conductor commits as one action on its lamad chain
    And within 120 seconds matthew's and jessica's conductors hold that action
    And the drill peer's conductor is stopped and its databases are restored to the snapshot, so its own chain no longer holds that action
    And the drill peer's conductor starts and adds a different content entry
    Then before that second entry is committed, the drill peer's chain holds the erased action again, at its original seq and with its original hash
    And the second entry takes the next seq
    And within 15 minutes no household conductor has written a chain-fork warrant naming the drill peer
    And both content entries reach matthew and jessica within 120 seconds of being committed
    And matthew's and jessica's count of blocked messages from the drill peer stays below 10 in every DNA

  @wip
  Scenario: Every household conductor is configured to commit an action durably before publishing it
    A power cut cannot be produced inside the development workspace, so this scenario proves only
    that the setting is in force; the next scenario proves the effect on owned hardware. Today
    every household conductor runs at synchronous=Normal, so this scenario fails until one of two
    options is in place: synchronous=Full (asserted here), or a proposed conductor change that
    forces only authored actions to disk before they can be published, which would replace this
    step when built.

    When the steward reads each household conductor's database durability setting
    Then each conductor commits every write at synchronous=Full

  @wip @requires:owned-substrate
  Scenario: A power cut during authoring leaves no household chain forked
    Every other scenario runs on the household mesh inside the development workspace, where power
    cannot be cut. This one needs a household node on hardware the steward owns and can switch
    off (the owned-substrate capability); it is skipped wherever that hardware is absent. It proves
    what the previous scenario cannot: that the durable setting actually survives a real power cut.
    It is meaningful only once the previous scenario passes; with synchronous=Normal it would
    simply reproduce the fork.

    Given every household conductor is configured to commit an action durably before publishing it
    When power is cut to an owned household node while that node's peer is adding content entries
    And the node restarts and its peer adds one more content entry
    Then no source chain on that node holds two different actions at one seq
    And within 15 minutes no household conductor has written a chain-fork warrant naming that peer

  @wip
  Scenario: A forked drill peer returns under a lineage key without recasting the household
    Written for re-key with lineage, the recommended answer to decision D1. The household re-key
    command does not exist yet; it is part of the cure this scenario proves. If the steward rules
    operator unblock instead, every step below is wrong and this scenario must be rewritten before
    anyone wires it.

    Given a fresh drill peer has joined the household and been exiled by running the lost-tail drill on it
    When the steward re-keys the drill peer's lamad cell with the household re-key command
    Then the drill peer authors under a new agent key, bound to the same identity by a KeyRotation entry that names the warranted key as its predecessor
    And within 10 minutes matthew's and jessica's lamad peer lists include the new key
    And a content entry the drill peer adds reaches every household peer within 120 seconds
    And the warranted key stays warranted
    And no content the drill peer authored under the warranted key is reported with trust "notarized"
    And matthew, jessica and james keep the agent keys they had before the drill
