@e2e @dataplane @concern:doorway-failover @act:i @requires:owned-substrate
Feature: A packaged app reads content through the doorway that served it
  A household visitor can see a landing page yet still be unable to read anything
  if its packaged app sends content requests to an unrelated production host.
  A package must work at either household doorway without rebuilding its addresses.

  The prepared household already holds the canonical landing package and manifesto.
  The two doorways named below are HTTP entrances to that household, not storage
  peers. Each visit starts in a fresh anonymous browser. Success means the real app
  requests the manifesto from that entrance and displays its title and body; merely
  painting the app shell, issuing a test-side fetch, or hiding CORS errors is insufficient.
  This read-only station proves endpoint selection while both entrances are healthy.
  Sibling selection during an outage and WAN ingress continuity remain separate stories.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"
    And every serving doorway among "alpha-A" and "elohim.host" resolves the same declared head for content "elohim-host-landing"

  Scenario Outline: The same package lets a visitor read the manifesto at either entrance
    When an anonymous household reader opens "manifesto" at "/epr/manifesto" through doorway "<doorway>"
    Then the app has fetched that content successfully through the visited doorway
    And the reader sees that content's title and rendered body
    And the app has sent no substrate requests to a different origin

    Examples:
      | doorway     |
      | alpha-A     |
      | elohim.host |
