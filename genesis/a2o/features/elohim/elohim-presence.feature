@e2e @elohim @browser-only @presence @wip
Feature: Elohim Presence
  As a learner, I want the elohim to offer transparent insights at key moments
  so that I understand the constitutional reasoning behind recommendations.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @discovery @insight
  Scenario: Learner sees elohim insight after discovery completion
    Given the learner navigates to a discovery assessment
    When the learner completes the assessment
    Then an elohim insight section appears below the results
    And the insight contains a learning path recommendation message
    And the insight includes a "Constitutional Reasoning" expandable section

  @discovery @reasoning
  Scenario: Elohim insight shows constitutional reasoning transparency
    Given the learner has completed a discovery assessment
    And the elohim insight is visible
    When the learner expands the reasoning details
    Then the primary constitutional principle is visible
    And the interpretation of the principle is visible
    And the computation cost is visible showing tokens and processing time

  @transparency @cost
  Scenario: Computation cost displayed after elohim interaction
    Given the learner has completed a discovery assessment
    And the elohim has responded with an insight
    Then the insight shows tokens processed
    And the insight shows processing time in milliseconds

  @config @banner
  Scenario: Test connection emits visible banner notification
    Given the learner navigates to "/doorway/elohim"
    When the learner clicks the "Test Connection" button
    Then the test result shows the mock backend is available
    And a banner notification appears confirming the connection
