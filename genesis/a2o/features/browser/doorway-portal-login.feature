@browser @auth @portal @requires:doorway @act:i @e2e @concern:doorway-portal-login
Feature: The doorway portal signs a hosted human in
  As a hosted human arriving at a doorway
  I want the sign-in form I am shown to be the one that really signs me in
  So that I am never locked out by a portal that looks like it works

  A HOSTED human is one whose doorway holds their identity for them — that is what "chaperone"
  means here — as opposed to a human running their own conductor and holding it themselves.
  Two ways in; this feature covers the first.

  The portal they sign in at is `doorway-app`, an Angular SPA the doorway serves under
  `/threshold/*`. It calls the doorway on its OWN origin (`environment.doorwayUrl` is the empty
  string, meaning same-origin), so the portal is only exercisable through the doorway that
  serves it — never against the SPA's dev server directly, where those calls would go nowhere.
  That is why every scenario here drives the doorway origin.

  This feature exists because "the portal renders" and "the human is signed in" are two
  different claims and only the first is cheap. A portal can paint a perfect form and mint
  nothing. So each scenario asserts BOTH: the form is really on the page, AND the doorway
  itself confirms the resulting session by answering for it — the token the portal stored is
  presented back to `GET /auth/me`, and the doorway must name the same human. A scenario that
  only looked at the DOM would pass against a portal that authenticates nobody.

  The human is registered through the API in the Background rather than drawn from the fixture
  cast, so this runs against any doorway — the local mesh, a hybrid mesh, or a deployed one —
  by pointing E2E_DOORWAY_ALPHA at it and changing nothing else.

  What is deliberately NOT asserted: where the portal routes after a successful sign-in. Served
  from a dev server behind the doorway proxy, the SPA's dynamic component imports fail, so it
  stays on the sign-in step even though the session is real. Asserting navigation would make
  this feature red locally and green deployed for a reason that has nothing to do with whether
  anyone got signed in — so the session, not the route, is the finish line.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a hosted human is registered on doorway "alpha"
    And a browser is open on doorway "alpha"

  Scenario: The portal renders and the doorway signs the human in
    When the browser opens the doorway sign-in portal
    Then the portal renders its sign-in form

    When the human signs in through the portal
    Then the doorway confirms a session for that human

  Scenario: A wrong password is refused at the portal, and no session is created
    When the browser opens the doorway sign-in portal
    And the human submits a wrong password through the portal
    Then the portal shows a sign-in error
    And the doorway confirms no session for that human
