@browser @browser-only @auth @portal @requires:doorway @act:ii @e2e @concern:doorway-portal-login
Feature: The deployed doorway portal signs a hosted human in (Act II twin)
  As a hosted human arriving at a doorway deployed to the shared multi-peer cluster
  I want the sign-in form the deployed doorway shows me to be the one that really signs me in
  So that a portal that ships is a portal that works, and the pipeline says so

  The suite runs in acts, each against a different substrate: Act I, the HOUSEHOLD, runs
  against a local mesh the run itself owns and may rewrite; Act II, the NEIGHBOURHOOD, runs
  against the deployed cluster (the "fleet") that no test may rewrite. A scenario belongs to
  exactly one act, so this file is the Act II twin of `doorway-portal-login.feature`: the same
  two claims — the form is really on the page AND the doorway confirms the session it minted —
  made against the deployed doorway, in the browser (playwright) leg the pipeline already runs.

  What the Background leaves behind, plainly: it registers one throwaway hosted human through
  the doorway's HTTP API, exactly as the household twin does. That is a row in the doorway's own
  account store — never a DHT entry, never replicated to peers — so the run rewrites nothing the
  neighbourhood contract protects. It is not swept afterwards: the fleet accrues one such account
  per pipeline run. If that ever reads as noise, the cure is a sweeper for test-registered
  accounts, not a weaker proof.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a hosted human is registered on doorway "alpha"
    And a browser is open on doorway "alpha"

  Scenario: The deployed portal renders and the doorway signs the human in
    # Two claims in one flow, on purpose: the form is present, THEN the sign-in is real.
    When the browser opens the doorway sign-in portal
    Then the portal renders its sign-in form

    When the human signs in through the portal
    Then the doorway confirms a session for that human

  Scenario: A wrong password is refused at the deployed portal, and no session is created
    When the browser opens the doorway sign-in portal
    And the human submits a wrong password through the portal
    Then the portal shows a sign-in error
    And the doorway confirms no session for that human
