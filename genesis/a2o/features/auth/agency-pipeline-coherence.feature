@e2e @auth @browser-only @requires:doorway @agency-pipeline
Feature: Agency pipeline in doorway-account matches the elohim-app stages
  As a hosted human looking at the agency pipeline on doorway/account
  I want the pipeline to express the same lifecycle the agency badge in elohim-app shows
  So that the two surfaces don't tell me different stories about where I am

  doorway/account renders an "Agency Pipeline" with steps {hosted, key_export,
  install_app, steward}. elohim-app's agency badge renders stages {visitor,
  hosted, hosted-steward, app-steward, node-steward}. The two MUST be coherent
  for a returning user — neither claims I am further along than the other does.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @elohim-visually-validated
  Scenario: Matthew's pipeline shows hosted-steward as an in-between state
    Given human "Matthew" is a graduated steward whose conductor is offline
    When Matthew opens doorway/account
    Then the agency pipeline step "Hosted" is marked completed
    And the pipeline shows the in-between "Hosted Steward" affordance
    And the pipeline step "Steward" is NOT marked completed
    And the page reads "Accessing through alpha.elohim.host" near the header

  @requires:shem
  Scenario: Susan's pipeline reflects no stewardship affordance
    Given human "Susan" is a hosted visitor with no portal host registered
    When Susan opens doorway/account
    Then the agency pipeline step "Hosted" is marked current
    And the pipeline does NOT show a "Hosted Steward" affordance
    And the Graduate-to-Steward CTA is visible with key_export listed as the next gate

  Scenario: agency_phase enum and AGENCY_STAGES progression do not drift
    When I compare auth_routes.rs agency_phase values to elohim-app AGENCY_STAGES stages
    Then every agency_phase value maps to an AGENCY_STAGES stage or an explicit non-stage state
    And no stage is reachable in one surface without being reachable in the other
