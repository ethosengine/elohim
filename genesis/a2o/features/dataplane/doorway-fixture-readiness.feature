@e2e @dataplane @concern:doorway-failover @act:i @requires:owned-substrate
Feature: The household holds the site before a visitor relies on its spare doorway
  A visitor can only rely on a second doorway if the household already holds
  the site and the content the visitor will ask for. An empty household cannot
  demonstrate continuity by reporting that its processes are healthy.

  This is the prerequisite to the storage fault in doorway-failover.feature;
  doorway-apex-transition.feature separately owns public-name continuity. The two doorways
  must serve the landing page at their root addresses and agree on its declared
  head, the notarized identity of the current version. The manifesto must also
  have a blob held by a peer other than the first doorway's primary storage
  peer. That second holder is what can answer during the later storage fault.
  In the steps, a peer named "alpha-A" or "elohim.host" is a doorway
  endpoint; a storage peer is a holder behind a doorway.
  These checks read the prepared household; they do not stage content or stop
  anything. Failure here means the drill's starting conditions are not met.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: Either doorway serves assets from the same declared landing bundle
    When I query "/" on peer "alpha-A" expecting raw text
    Then the raw response status is 200
    And the raw response body contains "app-root"
    When I query "/" on peer "elohim.host" expecting raw text
    Then the raw response status is 200
    And the raw response body contains "app-root"
    When a visitor asks peer "alpha-A" for the page at "/"
    Then every script and stylesheet the page from peer "alpha-A" names is one that peer serves
    And the page from peer "alpha-A" names the same browser entry point as the declared browser head of EPR "elohim-host-landing"
    And the build stamp peer "alpha-A" serves is the one the declared browser head of EPR "elohim-host-landing" carries
    When a visitor asks peer "elohim.host" for the page at "/"
    Then every script and stylesheet the page from peer "elohim.host" names is one that peer serves
    And the page from peer "elohim.host" names the same browser entry point as the declared browser head of EPR "elohim-host-landing"
    And the build stamp peer "elohim.host" serves is the one the declared browser head of EPR "elohim-host-landing" carries
    And every serving doorway among "alpha-A" and "elohim.host" resolves the same declared head for content "elohim-host-landing"

  Scenario: A second holder can supply the manifesto before the primary is stopped
    When I query "/db/content/manifesto" on peer "alpha-A" expecting raw text
    Then the raw response status is 200
    And content "manifesto" has a blob that a holder behind doorway "alpha-A" other than its primary answers for
