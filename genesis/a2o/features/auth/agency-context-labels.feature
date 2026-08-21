@e2e @auth @browser-only @requires:doorway @agency-context @act:i
Feature: Agency badge distinguishes hosted-steward from hosted-visitor
  As a returning user opening elohim-app
  I want the agency badge to tell me how the system actually understands me
  So that a confirmed steward is never called a visitor

  The agency badge surfaces the current stage. Calling a graduated steward
  a "Hosted Visitor" inverts the protocol's stance on agency — it tells
  the human that the system has forgotten who they are. The stage
  "hosted-steward" exists precisely for the in-between case: a steward
  whose peer-native infrastructure is authoritative but who is presently
  signed in through a doorway — recovering because their own device or
  conductor is temporarily unreachable, not because they lost standing.
  Recovery resolves by restoring the steward's own device; it is never a
  rung to graduate past.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @elohim-visually-validated
  Scenario: Matthew sees the Hosted Steward badge after OAuth login through doorway
    Given human "Matthew" is a graduated steward
    When Matthew logs in at elohim-app via the doorway OAuth flow
    And elohim-app loads the agency badge
    Then the agency stage badge reads "Hosted Steward"
    # The tagline is asserted verbatim against AGENCY_STAGES['hosted-steward'].tagline
    # in app/elohim-app/src/app/imagodei/models/agency.model.ts — the shipped wording,
    # which names recovery explicitly rather than only "signed in through a doorway".
    And the tagline reads "A steward in recovery — doorway temporarily hosts your conductor"

  @elohim-visually-validated
  Scenario: James sees the Hosted Visitor badge after OAuth login through doorway
    Given human "James" is a hosted visitor with no portal host registered
    When James logs in at elohim-app via the doorway OAuth flow
    And elohim-app loads the agency badge
    Then the agency stage badge reads "Hosted Visitor"
    And the tagline reads "A doorway holds your keys"

  Scenario: Agency badge upgrades when hosting account confirms stewardship
    Given human "Matthew" is a graduated steward
    When Matthew logs in at elohim-app and the hosting account has not loaded yet
    Then the agency stage badge initially reads "Hosted Visitor"
    When the hosting account loads with isSteward=true
    Then the agency stage badge updates to "Hosted Steward"

  # A recovery mode is ranked between the stages it sits between, so the badge can
  # say where the human is — but it is NOT a rung, so nothing offers "upgrade from
  # recovery". The resolution of hosted-steward is restoring your own device, not
  # graduating further. Both halves are asserted against the shipped AGENCY_STAGES
  # and getNextStage() in app/elohim-app/src/app/imagodei/models/agency.model.ts.
  Scenario: hosted-steward is ranked between hosted and app-steward without joining the ladder
    When I read the AGENCY_STAGES progression
    Then "hosted-steward" is flagged as a recovery mode, not a progression step
    And the stage after "hosted" is "app-steward"
    And getNextStage from "hosted-steward" is null — a recovery mode has no next rung
    And the order ranking is visitor < hosted < hosted-steward < app-steward < node-steward
