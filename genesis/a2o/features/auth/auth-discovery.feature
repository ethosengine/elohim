@e2e @auth @discovery @requires:doorway @concern:auth-discovery
Feature: A doorway tells an app how to sign a human in
  As an app being served from a doorway
  I want to learn where to send a human without being configured
  So that adding an app to the protocol never means hand-rolling its own login

  The one thing a page always knows without being told is the origin it was served from.
  `GET /.well-known/elohim-auth` turns that single fact into everything a client needs, so an
  app carries no login path, no endpoint list, and no doorway address of its own. It names two
  things: the SIGN-IN PORTAL — the page a human is sent to in order to authenticate — and the
  endpoints an app talks to afterwards, including the pair that hands an existing session to a
  sibling app, which is the part no client could invent for itself.

  Three properties make this safe to trust, and all three are asserted here rather than assumed.

  First, EVERY value is an origin-relative path. A discovery document that could name another
  origin would be an open-redirect primitive: whoever answered it could aim a Login button at
  an attacker's portal. This one cannot express a foreign origin at all, so the client resolves
  what it reads against the origin it already trusted enough to load from.

  Second, it lives under `/.well-known/`, which the doorway treats as a service path. An
  unknown path there is refused. That matters because the obvious alternative fails in the
  worst way: `/auth/*` paths the doorway does not own fall through to the app shell and answer
  200 with HTML, so a client probing `/auth/config` gets a JSON parse error instead of a
  branchable "no such thing" — the failure is misdiagnosed rather than handled.

  Third, the document does not lie about what it offers. Every path it advertises is one the
  doorway genuinely owns — a path it did NOT own would fall through to the app shell and answer
  200 with HTML, so a client following the document would be handed a web page where it expected
  an endpoint. That is the same misdiagnosis as the second property, arriving through the front
  door instead of the back, and the two lists that have to agree are maintained by hand.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @act:i
  Scenario: The doorway publishes where to sign in
    When the auth discovery document is fetched from doorway "alpha"
    Then the discovery document names the sign-in portal
    And the discovery document names the endpoints for signing in, reading the session, and handing it to a sibling app

  @act:i
  Scenario: The document cannot point an app at another origin
    When the auth discovery document is fetched from doorway "alpha"
    Then every location in the discovery document is origin-relative

  @act:i
  Scenario: An unknown well-known path is refused rather than answered with the app shell
    When "/.well-known/not-a-real-document" is fetched from doorway "alpha"
    Then the doorway refuses it as not found

  @act:i
  Scenario: Everything the document advertises is really served by the doorway
    When the auth discovery document is fetched from doorway "alpha"
    Then every advertised endpoint answers as an auth route, not the app shell
    And the advertised portal answers as a page
