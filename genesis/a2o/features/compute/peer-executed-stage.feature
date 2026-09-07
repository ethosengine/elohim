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
    When the requester submits the identical stage a second time
    Then the same request is recovered and the provider runs nothing new
    And exactly one admission event exists for that stage
