@e2e @resilience @act:i @requires:owned-substrate @concern:conductor-validation-spin
Feature: A household peer can be re-keyed without grinding the rest of the house
  As a household whose devices get replaced, reinstalled, and re-keyed
  I want the peers that stay home to keep serving at rest
  So that one member's fresh start is a small event on the others' machines,
  not a permanent tax on every machine in the house

  # Why this story exists — the fleet-scale version of the same event.
  #
  # On 2026-07-24 a DNA reinstall re-keyed the deployed conductors. Every
  # conductor got a new identity; every peer's local database kept pointing at
  # actions the old identities had authored. Those actions now belong to chains
  # that no longer exist, so nothing on the network can produce them.
  #
  # The conductor's validation worker does not know that. Its job is to hold
  # back any record whose dependencies it has not yet seen, and to keep asking
  # for them until they arrive — which is correct, and works, whenever they
  # eventually do. Here they never can. So every ten seconds it wakes, asks for
  # every dependency it is missing, receives none of them, floods its own
  # database read pool with the attempts, writes about a thousand log lines a
  # second describing the failure, and goes back to sleep. It has been doing
  # this for weeks. Every peer sits at whatever CPU ceiling it was given — two
  # cores or eight, it consumes exactly what it is given — while doing no work
  # anyone asked for. A process whose cost is its limit, whatever the limit is,
  # is a spin, not a workload.
  #
  # A household feels this as a laptop that is always hot, always loud, and
  # always slow, for a reason no member can see or name. This story makes the
  # event reproducible on the household mesh so the fix can be proven at a desk.
  #
  # Measured evidence and the full mechanism:
  #   genesis/data/timeline/backlog/alpha-conductor-sys-validation-spin-unfetchable-deps.md
  #
  # BORN RED, ON PURPOSE. The staged re-key is expected to produce exactly the
  # grind this story forbids, because today's conductor has no way to give up on
  # a dependency that will never arrive. The cure is a bounded retry with backoff
  # and an abandon budget in the conductor fork (elohim/holochain-conductor,
  # branch fix/sys-validation-unfetchable-deps-backoff), which ships as a
  # conductor image and moves no DNA hash. This scenario is what that branch has
  # to turn green, and what keeps it green afterwards.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the household mesh is running with peers "Matthew", "Jessica" and "James"
    # The doorway is the household's front door, and every scenario in this
    # suite establishes one. This story then works behind it, peer to peer,
    # because the grind it measures happens between the conductors themselves.

  # A CONDUCTOR is the per-peer process that validates, stores, and serves the
  # household's records. Each household member's machine runs exactly one.
  #
  # THE INSTRUMENT. "The spin detector" is a script an operator runs by hand on
  # any mesh — app/elohim-app/scripts/hc-mesh-spin-detector.sh. Over repeated
  # cycles it reads three things and reports a single verdict:
  #
  #   CPU per conductor, in cores, from the kernel's own process accounting.
  #     A spin is a constant amplitude — flat at the ceiling, cycle after cycle.
  #   The MISSING-DEPENDENCY BACKLOG, which the conductor prints itself every
  #     ten seconds: how many dependencies it is waiting on, and how many of
  #     them it managed to fetch this round. Weeks of "0 fetched of 873" is the
  #     shape of a question no one can answer.
  #   The READ-POOL SATURATION RATE: how often per second the conductor
  #     complains that its database read connections are all busy. Each peer has
  #     a small fixed pool of them (eight); every retry attempt takes one. On
  #     the deployed fleet this line appears 600-950 times a second and the pool
  #     reports itself forty times oversubscribed. On a resting household mesh
  #     it does not appear at all. The SPIN THRESHOLD is where the detector
  #     draws the line between those two worlds — five lines a second by
  #     default, which is two hundred times the resting mesh's ENTIRE log rate.
  #
  # ONE THING THE INSTRUMENT CANNOT DO, and says so. Every line it counts is
  # logged by the conductor at INFO, so a conductor started without RUST_LOG
  # emits ERROR only and those lines become unobservable rather than absent.
  # Measured on this mesh 2026-08-21: 86 ERROR lines, 0 INFO. The detector
  # reports that as a BLIND log leg instead of a confident zero, and the
  # saturation assertion below refuses to pass on it — a story that goes green
  # because nobody was looking is worse than one that fails.
  #
  # The verdict is SPIN only when BOTH legs hold: a backlog that is not
  # shrinking AND saturation over the threshold. Either alone is not the bug —
  # a draining backlog is a peer doing its job, and a busy read pool during a
  # real sync is a peer earning its keep. That is why each scenario below
  # asserts the verdict AND both legs separately: if the verdict is ever wrong,
  # the two component assertions say which half of it lied.

  Scenario: An undisturbed household is quiet, so a later measurement means something
    When the spin detector watches the household conductors for 3 minutes
    Then the spin detector's verdict is "QUIET"
    And no conductor reports a missing-dependency backlog
    And the conductors' read-pool saturation rate stays under the spin threshold
    # The alarm has to be silent when nothing is wrong, or its ringing later
    # proves nothing. Here the backlog must be absent outright, not merely
    # shrinking: nothing has been taken away from anyone, so there is nothing
    # legitimate to be waiting for. A resting household mesh runs its conductors
    # near a tenth of one core and writes a handful of log lines per minute.

  Scenario: A re-keyed household peer leaves the others at rest, not grinding
    Given James has authored content on his own peer
    And that content is anchored on James's own conductor, signed onto his source chain
    And Matthew's and Jessica's peers hold that content under James's anchor
    And the content James already stored stays on his disk through the re-key
    When James's peer is re-keyed, taking a new conductor identity
    And the spin detector watches the household conductors for 3 minutes
    Then the spin detector's verdict is "QUIET"
    And no conductor accumulates a missing-dependency backlog that stops shrinking
    And the conductors' read-pool saturation rate stays under the spin threshold
    # A SOURCE CHAIN is the running, signed record of everything one conductor
    # has ever done, and each entry is signed with that conductor's key. A
    # re-key starts a new one and abandons the old, which is why the old
    # signatures become unproducible for everybody at once.
    #
    # Authoring and anchoring are two steps, and the second is the one that
    # matters here: authoring writes the content to James's own machine, while
    # anchoring signs it onto his conductor's chain and publishes it, which is
    # what lets his neighbours refer to it at all. Those signatures are exactly
    # what a re-key destroys.
    #
    # Re-keying is the household-scale name for the fleet event. James's machine
    # comes back with a new conductor identity, so every action his old identity
    # signed is now unreachable for everyone — while the content those actions
    # described is still sitting on his disk, and still referenced by his
    # neighbours. That gap is the whole bug.
    #
    # Note this scenario asks for LESS than the quiet one above, on purpose. A
    # backlog here is legitimate and expected: Matthew and Jessica really are
    # missing dependencies now, and they are right to ask for them at least
    # once. What they must not do is ask forever. So the test is not "no
    # backlog" but "no backlog that stops shrinking" — the fix's bounded retry
    # and abandon budget must drain it, not prevent it.

  Scenario: The re-keyed peer gives an honest answer about content its old key signed
    Given James's peer has been re-keyed after authoring content
    When Matthew asks James's peer for that content
    Then James's peer either re-adopts that content under his new key or reports it as unfetchable
    And James's peer answers every request instead of shedding it or holding it open
    # Losing a key is allowed. Two answers are honest: "I have this again, under
    # my new name" or "I cannot prove this any more". The third answer — an open
    # request, held forever, behind a conductor burning a core to re-ask a
    # question no one can answer — is the one this story rules out. Honest
    # absence is a feature of a network people can trust; indefinite waiting is
    # how trust is spent without anyone deciding to spend it.
