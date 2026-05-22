@e2e @auth @browser-only @requires:doorway @agency-context
Feature: Agency badge distinguishes hosted-steward from hosted-visitor
  As a returning user opening elohim-app
  I want the agency badge to tell me how the system actually understands me
  So that a confirmed steward is never called a visitor

  The agency badge surfaces the current stage. Calling a graduated steward
  a "Hosted Visitor" inverts the protocol's stance on agency — it tells
  the human that the system has forgotten who they are. The stage
  "hosted-steward" exists precisely for the in-between case: a steward
  whose peer-native infrastructure is authoritative but who is presently
  signed in through a doorway.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @elohim-visually-validated
  Scenario: Matthew sees the Hosted Steward badge after OAuth login through doorway
    Given human "Matthew" is a graduated steward
    When Matthew logs in at elohim-app via the doorway OAuth flow
    And elohim-app loads the agency badge
    Then the agency stage badge reads "Hosted Steward"
    And the tagline reads "A steward, currently signed in through a doorway"

  @elohim-visually-validated
  Scenario: Susan sees the Hosted Visitor badge after OAuth login through doorway
    Given human "Susan" is a hosted visitor with no portal host registered
    When Susan logs in at elohim-app via the doorway OAuth flow
    And elohim-app loads the agency badge
    Then the agency stage badge reads "Hosted Visitor"
    And the tagline reads "A doorway holds your keys"

  Scenario: Agency badge upgrades when hosting account confirms stewardship
    Given human "Matthew" is a graduated steward
    When Matthew logs in at elohim-app and the hosting account has not loaded yet
    Then the agency stage badge initially reads "Hosted Visitor"
    When the hosting account loads with isSteward=true
    Then the agency stage badge updates to "Hosted Steward"

  Scenario: getNextStage from hosted-steward points to app-steward
    When I read the AGENCY_STAGES progression
    Then the stage after "hosted-steward" is "app-steward"
    And the order ranking is visitor < hosted < hosted-steward < app-steward < node-steward
