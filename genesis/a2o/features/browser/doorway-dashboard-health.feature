@e2e @browser-only @doorway-dashboard @requires:doorway
Feature: Doorway Dashboard Health
  As Matthew, the genesis steward operating the alpha doorway
  I want the dashboard to load without console errors
  And degrade gracefully when some services are unavailable

  Matthew is the only operator on the alpha deployment. The dashboard
  should give him clear, clean feedback about his doorway's state
  rather than a wall of console errors.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  @wip
  Scenario: Dashboard overview tab loads cleanly
    When Matthew opens the doorway dashboard
    Then the dashboard should render without JavaScript errors
    And the overview tab should be active

  @wip
  Scenario: Dashboard shows capabilities status
    When Matthew opens the doorway dashboard
    Then the capabilities badge should indicate which features are enabled
    And disabled features should show a placeholder instead of errors

  @wip
  Scenario: Dashboard handles missing orchestrator gracefully
    When Matthew opens the nodes tab
    And no orchestrator is configured
    Then the nodes tab should show a "No orchestrator" message
    And no console errors should be logged
