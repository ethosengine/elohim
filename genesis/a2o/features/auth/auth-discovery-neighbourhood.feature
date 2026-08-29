@e2e @auth @discovery @requires:doorway @act:ii @concern:auth-discovery
Feature: A deployed doorway tells an app how to sign a human in (Act II twin)
  As an app served from a doorway deployed to the shared multi-peer cluster
  I want the same discovery guarantees the household proves to hold on the deployed doorway
  So that the pipeline that ships a doorway also proves it can still be signed in through

  The suite runs in acts, each against a different substrate: Act I, the HOUSEHOLD, runs
  against a local mesh the run itself owns and may rewrite; Act II, the NEIGHBOURHOOD, runs
  against the deployed cluster (the "fleet") that no test may rewrite. CI gates each act on
  its own substrate, so a scenario belongs to exactly one act. This file is the Act II twin of
  `auth-discovery.feature`: each scenario below mirrors a household-proved assertion, run
  against the deployed doorway `E2E_DOORWAY_ALPHA` names. They are read-only GETs — no write,
  re-seed, restart, kill or tail — which is exactly what makes them safe to run where nothing
  may be owned. One concern tag joins the twins; two act tags on one scenario is not allowed.

  ("The app shell" below is the single-page app's catch-all, which answers 200 with HTML for
  any path nothing else claimed — the adversary in these scenarios is a 200 that looks like
  success while being the wrong thing.)

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: The deployed doorway publishes where to sign in
    When the auth discovery document is fetched from doorway "alpha"
    Then the discovery document names the sign-in portal
    And the discovery document names the endpoints for signing in, reading the session, and handing it to a sibling app

  Scenario: The deployed document cannot redirect sign-in to another origin
    When the auth discovery document is fetched from doorway "alpha"
    Then every location in the discovery document is origin-relative

  Scenario: An unknown well-known path on the deployed doorway is refused rather than answered with the app shell
    When "/.well-known/not-a-real-document" is fetched from doorway "alpha"
    Then the doorway refuses it as not found

  Scenario: Everything the deployed document advertises is really served by the doorway
    When the auth discovery document is fetched from doorway "alpha"
    Then every advertised endpoint answers as an auth route, not the app shell
    And the advertised portal is the page built for that path, not the app shell
