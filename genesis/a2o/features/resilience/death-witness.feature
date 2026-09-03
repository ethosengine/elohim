@e2e @resilience @concern:death-witness @act:i @requires:owned-substrate
Feature: Death witness — a peer's runtime tells its household why a child died
  As the operator of a household peer
  I want the runtime that was a conductor's parent to keep its last words
  So that when a peer "won't start" I read the reason from the peer itself,
  and the people who already custody my node can read it when I cannot

  The words this story uses, so it can be read cold:

  - A HOUSEHOLD is a small group of people and their peers who share a mesh,
    trust each other's custody, and are the default audience for each
    other's records. A STORAGE PEER is a peer that holds data on disk and
    serves it. A DOORWAY is the gateway a human uses to reach the network
    from a browser; "alpha" names the test doorway.
  - A PEER is one household's node. It is made of processes; the one that
    matters here is the CONDUCTOR, the process that holds the household's
    keys and source chains and talks to the network.
  - The ENVELOPE is the parent process that spawns the conductor, holds its
    stdout and stderr pipes, decides when to restart it, and outlives it.
    Today nothing on the household mesh is the conductor's parent; this
    story only means something once the envelope is.
  - A DEATH WITNESS is the record the envelope writes the moment a child
    dies: the exit signal or code, how long the child ran, its last lines
    of stderr, the hash of the program the envelope actually started, and
    the envelope's own last decision about that child (for example "I
    reinstalled the app because the bundle drifted"). Deaths of the same
    child that follow each other belong to one INCIDENT; the incident is
    the durable record, and each death is one witness inside it. A witness
    is first written to the peer's own disk, where the envelope needs no
    network; it is ANCHORED later — committed to the distributed network
    through the peer's own conductor, so it survives the loss of that peer —
    which can only happen once a conductor is running again. The
    envelope's last decision may be "none: the child was killed from
    outside"; that is still a recorded decision.
  - A CUSTODIAN is another household peer that has agreed, in a signed
    commitment, to keep copies of this peer's witnesses. The commitment is
    counter-signed by the custodian, so a peer cannot name a custodian who
    never agreed. Witnesses are addressed by the hash of their content
    (a CID), so a custodian's copy is provably the same record.
  - The PASSPORT is the peer's description of itself: which programs it
    runs, by hash; how many times it has come back up (its INCARNATION);
    and the last VERDICT its envelope reached (restart, give up, keep
    waiting). The witness carries the passport at the moment of death.
  - REACH is who may read a record. A witness is readable by the household
    and its custodians and refused to anyone else.
  - The ATOM HOME is the page in the app that shows one record; a death
    witness is one kind of record it renders. It has a REACH CHIP (who may
    see this) and a FOCAL SLOT (the main area, which renders the record by
    its kind instead of dumping raw data).
  - An ANONYMOUS CALLER is a request carrying no authenticated peer or human
    identity at all — the network's stranger.

  The incident this story exists to make sayable, 2026-09-02: seven
  conductors died the same way and only a cluster administrator could read
  why. The sentence a household operator should have been able to read from
  the peer itself was "conductor died 2.3 s into boot: no genesis for five
  cells; the previous boot deleted the source chains during a reinstall
  that timed out; DHT data intact."

  # Spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §6, §10.
  # Habit: elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md (born red).
  #
  # @requires:owned-substrate — this suite may kill processes only on a mesh it owns.
  # Every peer named here is a dedicated drill fixture on that mesh (see
  # chaos-peer-churn.feature). Jessica, Matthew, and James are fixture household humans.
  #
  # S0 landed 2026-09-02: the household mesh launches its conductors under the envelope
  # (`MESH_CONDUCTOR_LAUNCH=ark`), so stations 1 and 2 are live; 3a, 3b and 4 stay @wip
  # until S1 lands rendering, reach enforcement and anchoring. Without the envelope every
  # assertion below would be vacuous by construction — the Background refuses to run then.
  # Stations are decomposed so the finish line (station 4) is untouched as earlier stations land.
  #
  # Reused steps: the household-mesh fixture + processControl (household-chaos.steps.ts),
  # custody assertions ("under custody on all N household peers"), the atom-home render
  # steps (steps/ui/epr-atom-home.steps.ts), reach refusals (reach-enforcement.steps.ts).
  # New steps: the envelope precondition, a per-conductor kill (today only
  # conductors-restart exists, all at once), the witness query by peer.

  Background:
    Given the household mesh is three storage peers: Jessica, Matthew, and James
    And each peer's conductor is running as a child of that peer's envelope

  @station-1
  Scenario: Station 1 — the envelope that held the pipes witnesses the death
    When Jessica's conductor is killed with SIGKILL
    Then within 10 seconds Jessica's peer lists a death witness for a new incident
    And the witness names the signal, how long the conductor ran, and its last stderr lines
    And the witness carries the envelope's own last decision about that conductor
    And the witness names the hash of the conductor program the envelope actually started
    And the witness carries Jessica's passport as it stood at the moment of death

  # The custody commitment is a fixture here: the prologue's `seed-spool-custody` leg has each custodian
  # author a standing custody-spool commitment on ITS OWN conductor (authorship is the counter-signature);
  # the story of offering and accepting that commitment between humans is a later feature.
  # Budget measured on 2026-09-03, after S0 landed, on the household mesh: kill → ward row (≤5 s ingest) → shard replication
  # tick (10 s dial) with paged peer round trips (~40 s under host load) → custody-blob authored on the
  # custodian's own conductor (5 s sweep) → bytes already pulled by replication; 60 s was an MVP guess.
  @station-2
  Scenario: Station 2 — the custodians Jessica already has hold the witness
    Given Matthew and James have each counter-signed a commitment to custody Jessica's witnesses
    When Jessica's conductor is killed with SIGKILL
    Then within 120 seconds Matthew and James each hold a copy of the witness with the same content hash
    And Matthew and James each record on their own peer that they received that witness from Jessica

  @wip @browser-only @station-3a
  Scenario: Station 3a — a custodian reads the witness as a death witness, not raw data
    Given Matthew has counter-signed a commitment to custody Jessica's witnesses
    And human "Matthew" is logged in on doorway "alpha" with a device
    When Matthew is viewing the atom home for Jessica's latest death witness
    Then the reach chip shows that the household and its custodians may see it
    And the focal slot renders the record as a death witness, not raw JSON

  @wip @station-3b
  Scenario: Station 3b — a stranger is refused
    Given Jessica's peer holds a death witness
    When an anonymous caller fetches that witness on peer "jessica"
    Then the fetch was refused with a non-success status

  @wip @station-4
  Scenario: Station 4 — the incident becomes a durable network record when the conductor returns
    Given Jessica's conductor was killed with SIGKILL and a witness was written
    When Jessica's conductor is restarted by the envelope
    Then the incident is anchored on the network from Jessica's own conductor
    And another household peer can fetch the anchored incident by its content hash
    And the incident counts exactly one death and one restart
    And Jessica's passport records the new incarnation and the verdict "restart"
