# @concern:operator-runtime-surface joins this proof to the habit register's checks: entry — the
# identifier CI, this feature, and the habit register share.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the feature needs a running household mesh. @requires:owned-substrate
# on each scenario: the scenario writes and withdraws real grants, so it runs only on a mesh this
# run owns outright, never on a shared fleet other people are using.
# @requires:compute-executor on one scenario: it needs the program that RUNS a stage, which this
# substrate does not have at present (a temporary gap, not a design limit); it is held until then.
# Fixture: CI_RUNNER_A2O_CONFIG, written by genesis/agentic/compute/runner-fixture.sh (absent ⇒
# every step is pending). Run it from the requester's desk, NOT under `just test mesh`: a stage's
# guest claims the household lease itself, so a lane holding it would refuse its own stage.
@e2e @act:i @compute @concern:operator-runtime-surface @requires:household-nodes
Feature: A steward sponsors a runner, and the runner works only inside the promises made to it

  A household here is a small group of people and the machines they run together: each person
  keeps a peer, an always-on machine that holds their data and does work for them, and the
  household's peers trust and check one another. A steward is one of those people, caring for
  their own peer and answerable for what it promises. Being a steward gives no special power
  over anyone else; it only means the promises made from that peer are theirs.

  A runner is a piece of borrowed machinery that does checking work for a household: a CI
  server, or a short-lived machine that plays out a rehearsal. It is not a person, and it does
  not speak for one. It has a key of its own, and the only things it may do are the things
  someone has promised it in writing. Those written promises are called grants. Each grant names
  who is promising, who receives the promise, what kind of work it covers, how far anything the
  receiver publishes may travel, and until when. That distance is a level the grant declares,
  called its reach, taken from one scale. This story needs only one boundary on that scale:
  the commons, the open, shared space anyone may read, against anything more private than it.
  The scale has more private levels than that, but none of them matters here beyond being more
  private than the commons. Every grant here keeps its reach at the commons. For a runner, that
  means it may only publish what anyone could be shown, such as the material a stage needs or
  the report it produces, and never something meant for one person's eyes. Publishing anything
  more private than the commons needs a person's own standing, which no runner has.

  Two kinds of grant matter here. A sponsor is a steward who vouches for the runner. The sponsor
  hosts the runner's key on their own peer, and records that hosting as a hosting grant. A
  provider is any peer willing to lend the runner capacity to run a stage, and it records that
  lending as a capacity grant. A provider's peer checks every stage it is sent against its own
  capacity grants by itself, and accepts or refuses it; nobody decides by hand.

  A stage is one feature file of the household's own checks (the acceptance scenarios it keeps,
  like this one), together with the report it produces, which says for each scenario whether it
  passed, failed or was skipped. A stage is pinned: the script that runs
  it and the file it checks are each named by a fingerprint of their exact bytes, so the provider
  and the runner agree on exactly what will run, and nothing can be swapped in without changing
  that name. How a pin is checked is proven by its own story
  (features/compute/peer-executed-stage.feature).
  Here a validly pinned stage is simply a starting fact: no scenario below tests how a pin is
  checked, and this story proves only who may submit a stage, and why. A grant that is withdrawn refuses the next request; work already running
  is not part of this story.

  Any steward may sponsor a runner, and any peer may provide capacity to one. Nobody's runner can
  spend a promise that was made to someone else's runner.

  The cast keeps the two roles apart. The two stewards who sponsor runners are "the first steward"
  and "the second steward". The two who lend capacity are "the first provider" and "the second
  provider". In this story no steward plays both roles, although nothing stops one person from
  doing so.

  Background:
    Given a household with two stewards, the first steward and the second steward, each running their own peer
    And two more stewards, the first provider and the second provider, each running a peer, who will provide capacity and sponsor no runner
    And neither of those two providers has granted any capacity yet
    And a stage taken from the household's own checks and pinned by the fingerprints of its bytes

  @requires:owned-substrate
  Scenario: A sponsored runner has a key of its own and no record of a person
    When the first steward sponsors a runner
    Then the runner has its own agent key, distinct from the steward's
    And no person's identity record is associated with the runner's key
    And exactly one hosting grant names the first steward's peer as provider and the runner as recipient
    And the hosting grant states its reach and when it ends

  # @requires:compute-executor: the stage must actually RUN for its report to come back, and the
  # program that runs stages is missing from this substrate for now; the scenario is held, not
  # failed, until it returns.
  @requires:owned-substrate @requires:compute-executor
  Scenario: A provider lends capacity, and the runner's stage runs and reports back
    Given the first steward has sponsored a runner
    When the first provider grants the runner capacity to run stages
    And the runner submits a pinned stage to the first provider
    Then the capacity grant states its reach and when it ends
    And the first provider accepts the stage under that capacity grant
    And the runner reads back from the first provider a report that names the pinned stage and each scenario's outcome

  @requires:owned-substrate
  Scenario: Two stewards each sponsor a runner, and neither runner can spend the other's grants
    Given the first steward has sponsored a runner
    And the second steward has sponsored a runner of their own
    And the first provider has granted capacity to the first steward's runner only
    When the second steward's runner submits a pinned stage to the first provider
    Then the first provider refuses the stage, naming the missing capacity grant
    And the first steward's runner can still have a stage accepted by the first provider

  @requires:owned-substrate
  Scenario: Withdrawing a capacity grant stops only that provider, and withdrawing the hosting grant stops the runner
    Given the first steward has sponsored a runner
    And both providers have each granted the runner capacity
    When the first provider withdraws its capacity grant
    Then the first provider refuses the runner's next stage
    And the second provider still accepts it
    When the first steward withdraws the hosting grant
    Then the runner's key still exists, but its next request is refused, naming the withdrawn hosting grant

  @requires:owned-substrate
  Scenario: A runner cannot speak as a person
    Given the first steward has sponsored a runner
    When the runner tries to approve a new device in the first steward's name
    Then the approval is refused
    When the runner tries to publish something more private than the commons
    Then the publication is refused, naming the grant's reach

  # Added 2026-10-20 from the placement-contract design
  # (genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md,
  # §16 agents row; the rule is §12.3 rule 2, "reach before bytes"). Held (@wip): no tier yet checks
  # a tree's reach before it serves the files under it. That lands in the design's §17 slice 2,
  # the remote tier, and the scenario stays held until it does.
  @wip @requires:owned-substrate
  Scenario: A runner cannot fetch files more private than its grant's reach
    The scenarios above are about what a runner may do; this one is about what it may read, and
    it defines two terms that no other scenario uses. A tree is a set of files the household
    keeps, named by a fingerprint of its contents the way a stage is pinned. A tree's reach is a
    level the household declares on that tree, taken from the same scale as a grant's reach,
    saying how far the files under it may travel. So in this scenario reach appears in two
    places: on the runner's grant, and on each tree. Before a stage runs, the files it reads are
    fetched onto the provider's machine: the stage names a tree and the paths under it that it
    needs, and only those files are fetched. The rule
    this scenario proves is that a runner may fetch from a tree only when the tree's declared
    reach is no more private than the reach its grant states; here every grant is at the
    commons, so the runner may fetch from a commons tree and from nothing more private. A runner
    asking for a tree it may not read is refused in the same words whether or not that tree
    exists, so a refusal tells it nothing it could not already see.
    This scenario is held for now: no part of the household yet checks a tree's reach before
    serving the files under it, so it is not run until that check exists.
    Given the first steward has sponsored a runner
    And the first provider has granted the runner capacity to run stages
    And the household keeps one tree of files at the commons and another more private than the commons
    When the runner's stage asks the first provider to fetch a path under the more private tree
    Then the fetch is refused, naming the grant's reach
    And no file from the more private tree is written to the first provider's machine for that stage
    When the runner's stage asks the first provider to fetch a path under a tree the household does not keep
    Then the refusal reads the same as the one before, naming the grant's reach and not whether the tree exists
    When the runner's stage asks the first provider to fetch a path under the commons tree
    Then the files under that path arrive on the first provider's machine, each checked by the first provider against the fingerprint of its contents
