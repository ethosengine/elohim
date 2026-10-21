# @concern:operator-runtime-surface joins this proof to the habit register's checks: entry in
# genesis/manifests/habits.yaml — the identifier CI, this feature, and the habit register share.
@e2e @act:i @compute @concern:operator-runtime-surface @requires:household-nodes
Feature: A peer runs one of the household's CI stages and the household can check its work

  A household here is the small cluster of peers one person or family runs and governs
  together — in this proof, three peers on one mesh: the requester and two neighbours, one
  of whom offers to run work for the others. A stage is a piece of the household's own
  verification: one feature file, the scenarios it declares, and a verdict. Until now a
  stage ran wherever the developer happened to be sitting. This one runs on a neighbour's
  peer instead, because that neighbour offered spare capacity and signed a compute grant —
  a one-time, revocable statement naming this requester as allowed to use it, for this kind
  of work, up to a stated rate. Nothing runs without a grant that names it.

  The stage itself travels pinned: the runner script that will execute it and the feature
  file it will exercise are each named by the hash of their own bytes, so the peer cannot
  swap in different code without changing the name the requester agreed to. In this proof
  the pinned stage is the household's own federation-convergence check — normally run
  wherever a developer happens to be — but the path proven here is generic: any pinned a2o
  stage could ride it the same way.

  What comes back is not a green tick. It is the report the run actually produced, carried
  as expiring bytes leased to the stage's own address, plus a signed completion — a receipt
  — stating that the peer ran this exact stage under this exact grant. A receipt proves who
  ran it; only the report proves what happened. The household reads the report and does not
  take the peer's word for the verdict, because a signature proves who spoke, never that
  what they said is true. The same run also leaves one economic event behind: the
  household's own record that it spent one use of the neighbour's grant. That event is the
  admission this proof counts — one admission per stage run, however many times the
  request is resubmitted.

  Background:
    Given a household compute provider with a signed compute grant for this requester

  # WRITES: the pinned stage picked for this proof stages a real disagreement on two peers
  # and flips an operator flag to heal it, so this scenario is only ever acceptable on
  # infrastructure this run owns outright — never a shared fleet a visitor might be reading
  # from at the same time.
  @requires:owned-substrate
  Scenario: a peer runs the household's federation-convergence stage and the household checks the report
    Given a pinned stage whose runner script and feature file are content-addressed
    When the requester submits the stage to the provider
    Then the provider accepts the stage and runs it on its own substrate
    And the requester recovers the provider's signed completion for the pinned stage
    And the returned report names the pinned feature file and every declared scenario
    And every declared scenario passed in the report, not merely in the receipt
    And the completion is attested by an economic event naming the grant
    And the requester's own record shows the neighbour's grant behind the completion
    When the requester submits the identical stage a second time
    Then the same request is recovered and the provider runs nothing new
    And exactly one admission event exists for that stage

  # Held until slice 6 of the placement-contract design ("task to berth", §17 of
  # genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md),
  # which sends a task to the peer that already holds its inputs. @wip: no step has a definition yet.
  # These two scenarios use a build stage: a CI stage that builds the household's software from its
  # source files rather than checking it, and that is too heavy for the requester's own machine.
  @wip @requires:owned-substrate
  Scenario: a heavy build stage runs on the peer that already holds its inputs, and only its outputs come back
    Given a build stage whose input files all sit in one tree, named by the fingerprint of the tree's exact contents
    And the provider's peer already holds that whole tree, and the requester's machine holds none of it
    When the requester asks for the build by naming only its pinned build script, the tree's fingerprint and the paths in the tree the build reads
    Then no input file travels from the requester to the provider
    And the provider runs the build on its own peer, under the compute grant that names this requester, inside a sandbox that keeps the build away from the rest of that peer
    And the requester receives the fingerprint of the build's output and a record of which script and which input tree produced it
    And the requester downloads only the output files it opens, and none of the others

  @wip @requires:owned-substrate
  Scenario: a heavy build stage with nowhere to run is kept and shown as not carried, not lost
    Given a build stage too heavy for the requester's own machine to run
    And the provider's peer is switched off, and no other peer holds a compute grant for this requester
    When the requester asks for the build
    Then the request is kept on the requester's machine and shown as not carried, waiting for a provider
    And the build does not start on the requester's own machine unless the requester chooses to run it there
    When the provider's peer is switched on again
    Then the provider runs the kept build, and only its outputs come back to the requester
