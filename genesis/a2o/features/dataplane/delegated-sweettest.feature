@e2e @dataplane @concern:operator-runtime-surface @requires:shem
Feature: A developer delegates a sweettest while continuing other work
  A sweettest runs the Holochain feedback suite against a packed DNA bundle.
  The developer selects exact, content-addressed artifacts so Adam runs the same
  bytes the developer built. Adam's dedicated compute capacity protects his
  existing peer work. A compute grant is Adam's permission for this developer
  to use that capacity.

  The submitting client can exit while the test runs. The developer can keep
  using their workspace and later recover a signed receipt identifying the
  artifacts and outcome. Full logs have a configurable lifetime; the shorter
  receipt remains readable after those logs expire.

  Background:
    Given a configured native compute workspace and Adam's dedicated test worker
    And a reviewer is selected for delegated completions

  Scenario: Recovering a completion automatically starts the selected review
    Given the developer has selected exact test artifacts and a compute grant
    When the developer submits the pinned feedback sweettest to Adam
    And the submitting client exits while Adam runs the task
    Then another workspace command completes while Adam is still running the test
    When the workspace reconnects without its original completion notification
    Then it discovers Adam's signed completion for the submitted artifacts
    And recovering the receipt starts the selected agent and saves its review

  Scenario: The developer learns why an unauthorized request was refused
    When a developer without a matching compute grant submits the pinned feedback sweettest
    Then the developer receives Adam's refusal because no matching compute grant authorizes the request
    And no execution acceptance or completion is recorded for that request

  Scenario: The developer can still read the outcome after full logs expire
    Given a completed delegated run with a short configurable payload lifetime
    When that lifetime expires and the worker cleans retained payloads
    Then the developer is told the full logs have expired
    And the original compact receipt identifying the artifacts and outcome remains readable
