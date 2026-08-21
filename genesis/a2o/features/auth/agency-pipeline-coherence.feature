@e2e @auth @browser-only @requires:doorway @agency-pipeline @act:i
Feature: Agency pipeline in doorway-account matches the elohim-app stages
  As a hosted human looking at the agency pipeline on doorway/account
  I want the pipeline to express the same lifecycle the agency badge in elohim-app shows
  So that the two surfaces don't tell me different stories about where I am

  "Agency" tracks how far a person has progressed from depending entirely on a
  hosting doorway toward running their own node. doorway/account renders an
  "Agency Pipeline" with steps {hosted, key_export, install_app, steward}.
  elohim-app's agency badge renders stages {visitor, hosted, hosted-steward,
  app-steward, node-steward}. The two MUST be coherent for a returning user —
  neither claims I am further along than the other does.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # @wip — the steps below are fully wired, but the cure is NOT built. Against the
  # shipped doorway-account.component.ts this scenario is a RED-defining contract on
  # three counts, all in doorway/doorway-app (out of this pass's write-set):
  #   1. isStepCompleted('steward') returns account.isSteward, so a steward whose
  #      conductor is offline has "Steward" ticked complete — doorway/account claims
  #      he is further along than the elohim-app badge ("Hosted Steward") does. That
  #      is precisely the drift this feature exists to catch.
  #   2. The in-between affordance renders as a generic .context-banner reading
  #      "Accessing through this doorway" — it never names the Hosted Steward state.
  #   3. That banner names no doorway host, so the page cannot read
  #      "Accessing through alpha.elohim.host".
  # Drop @wip when doorway-account distinguishes hosted-steward from steward.
  @elohim-visually-validated @wip
  Scenario: Matthew's pipeline shows hosted-steward as an in-between state
    Given human "Matthew" is a graduated steward whose conductor is offline
    When Matthew opens doorway/account
    Then the agency pipeline step "Hosted" is marked completed
    And the pipeline shows the in-between "Hosted Steward" affordance
    And the pipeline step "Steward" is NOT marked completed
    And the page reads "Accessing through alpha.elohim.host" near the header

  # @wip — steps wired; the first assertion is an open design question, not a
  # wired-up contract. doorway-account marks "Hosted" COMPLETED for every account and
  # marks "Key Export" CURRENT for a hosted visitor, because its pipeline tracks the
  # next GATE while the elohim-app badge tracks the current STAGE. Whether coherence
  # means the doorway should mark "Hosted" current (matching the badge) or the story
  # should assert the gate axis is a decision for the surface owners. Resolve that
  # before dropping @wip; the other two assertions already hold against the shipped
  # component.
  @wip
  Scenario: James's pipeline reflects no stewardship affordance
    Given human "James" is a hosted visitor with no portal host registered
    When James opens doorway/account
    Then the agency pipeline step "Hosted" is marked current
    And the pipeline does NOT show a "Hosted Steward" affordance
    And the Graduate-to-Steward CTA is visible with key_export listed as the next gate

  Scenario: agency_phase enum and AGENCY_STAGES progression do not drift
    When I compare auth_routes.rs agency_phase values to elohim-app AGENCY_STAGES stages
    Then every agency_phase value maps to an AGENCY_STAGES stage or an explicit non-stage state
    And no stage is reachable in one surface without being reachable in the other
