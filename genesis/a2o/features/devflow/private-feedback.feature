@act:host @requires:epr-cli @concern:private-feedback-loop
Feature: A participant can carry an unmet need through private feedback
  A developer needs their experience to survive a tool failure without exposing
  private arguments or confusing a passing build with recovery in daily use.
  Evidence stays on this workspace until sharing is separately authorized.
  Genesis is the acceptance-test reporter; its findings can summarize repeated
  occurrences, and its declared concerns name the coverage expected from a run.

  Scenario: A human need survives reopening without becoming a repair commitment
    Given a private feedback workspace with no Git repository
    And the workspace has no repair commitment or execution admission
    When a participant records a need for bounded history
    And the participant opens feedback again
    Then the original need remains readable
    And feedback repair reports missing commitment and execution admission without starting work

  Scenario: Reingesting a Genesis report retains aggregate counts and missing coverage
    Given a private feedback workspace with no Git repository
    When a Genesis report with one finding reported nine times, private diagnostics and a URL, one skipped concern, and one absent declared concern is ingested twice
    Then its finding remains one aggregate with nine reported occurrences
    And the feedback list shows skipped coverage distinct from unmeasured coverage
    And private diagnostics and URLs are absent from feedback retention

  Scenario: Governed capacity reclaims old successes while preserving a participant's need
    Given a private feedback workspace with no Git repository
    And a version-pinned local retention policy and a migrated feedback store
    When old automatic successes reach the warning threshold beside a protected need
    Then a dry run shows eligible bytes without changing retained evidence
    When the participant executes feedback reclamation
    Then the protected need keeps its address and capacity returns below reset
    And the capacity report explains the bounded history window and policy identity

  Scenario: A developer checks whether their tool pair can turn use into a repair proposal
    Given an explicitly selected Brit and EPR pair for a private fixture
    When the developer checks feedback from a successful command and a refused command
    Then both experiences survive reopening without retaining the private argument
    And the refused experience produces a native proposal without admitting repair
    And a recorder failure preserves the command exits
    And the check identifies candidate selection without claiming installed delivery

  Scenario: A retained need remains the source of work without becoming approval
    Given a private feedback workspace with no Git repository
    When a participant records a need for bounded history
    And the participant proposes work from that retained need
    Then EPR shows the original need as the available source for the proposal
    And recognizing the evidence grants no repair acceptance
