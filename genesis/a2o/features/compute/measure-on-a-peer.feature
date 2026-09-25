@e2e @act:i @compute @concern:measure-runs-on-a-peer
Feature: A long check finds its way to a neighbour's peer instead of holding the developer's desk

  Some of what a developer runs is not a quick check but a long one — a soak, a repeated
  proof, a run against a store that has grown for weeks — and the developer is the one who
  declares which kind it is before starting it, because only the developer knows whether the
  run they are about to start will hold anything for hours or hand it back in moments. The
  household's peers share one test environment, the mesh, and only one desk at a time may
  hold its lease — the single exclusive claim, granted to whichever desk is using the mesh,
  that lets a desk run anything against it at all. A quick check takes that lease and gives
  it back in moments; a long check that takes the same lease from the developer's own desk
  holds it for as long as the run lasts. A neighbour who needed that same lease waits behind
  it, sometimes for hours, for a run that was never theirs to wait on. This is the shape of
  that problem and the shape of its fix: a long check has its own place to run, and that
  place is never the desk it was born at.

  The place is a neighbour's peer — a household compute provider who has already signed a
  grant naming this developer as allowed to spend some of that peer's capacity, for this
  kind of work, up to a stated rate. Nothing runs on the strength of a mere request; the
  grant is what makes the run legitimate before a byte of it executes. The developer's own
  desk stays free the whole time the run is out; the neighbour does the work and signs what
  it found. What comes back is read the way any peer's work is read here: a signature proves
  who ran it, never that what they found is true, so the household reads the report the run
  actually produced rather than taking a claimed verdict on faith.

  Background:
    Given a household compute provider with a signed compute grant for this requester

  @requires:household-nodes
  Scenario: a long check refused at the developer's own desk is told where it belongs
    Given the developer is at their own desk, where ordinary work already holds the household mesh's one lease
    When the developer tries to start a long check there too
    Then the desk refuses the run outright — a long check is never allowed to compete for that lease, held or not
    And the refusal names the neighbour's-peer path as where it belongs instead
    And the refusal does not offer to queue the run and wait for the lease

  @requires:household-nodes
  Scenario: one command sends the check to a neighbour and the developer keeps working
    Given a long check the developer wants run
    When the developer sends it with one command to the household's compute provider
    Then that one command returns to the developer without waiting for the run to finish
    And the developer's own desk holds no lease for the run that just left it

  @requires:household-nodes
  Scenario: the neighbour's signed answer counts as household evidence because of the grant, not whose key signed it
    Given the household compute provider has run the developer's long check to completion
    When the developer's own workspace reads back what the provider signed
    Then the workspace admits it as household evidence
    And what makes it admissible is the grant naming this developer, not whose key signed the run
    And a signed run from a peer who never held that grant would be refused the same read

  @requires:household-nodes
  Scenario: the household's own record shows the neighbour's capacity as spent, not merely offered
    # The reciprocal direction — the developer's own use showing up on the PROVIDER's record
    # too, so the exchange reads as mutual rather than one-sided — is the next station, not
    # this one. This scenario proves only the direction this sprint actually lights: what
    # left the neighbour's peer and arrived at the developer's desk.
    Given the household compute provider has run the developer's long check to completion
    When the developer's workspace reads back the completed run
    Then the developer's own record shows the neighbour's capacity as spent, not merely offered
    And that record did not exist before this read

  @requires:shem
  Scenario: the same long check on Adam leaves the developer's desk free the whole time
    # The scenarios above prove delegation within the household's own mesh — neighbours who
    # already share the same machines. Adam is a household compute provider on a separate
    # machine entirely, reached over the network rather than sitting beside the developer's
    # desk on shared local hardware. This scenario proves the same desk-freeing property
    # holds even when the long check travels all the way off the household's own hardware.
    Given Adam is provisioned as a household compute provider on a machine of his own, apart from the household mesh
    When the developer sends the same long check to Adam
    Then the developer's desk holds no lease on the household mesh at any point while Adam runs it
    And the developer's desk is free to run its own ordinary verification at the same time
