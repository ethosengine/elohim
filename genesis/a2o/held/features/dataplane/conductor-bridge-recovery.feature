@e2e @dataplane @wip @regression @requires:owned-substrate
Feature: A storage peer recovers its Holochain bridge after an independent conductor restart
  As Matthew, a household node operator
  I want my running storage peer to re-authenticate when its conductor restarts
  So that its PeerStatus record stays current and serving health tells the truth

  PeerStatus is the Holochain-authored liveness record used by peer and resilience
  views. Publishing it requires an authenticated app websocket. The conductor's
  admin websocket can remain live while that app websocket or its token is dead,
  so an admin-only probe cannot establish that the storage peer can call its zomes.

  Matthew's storage health reports each role's zome-path state and a reconnect
  counter. The counter increments when the supervisor detects a dead app bridge
  and starts a full re-authentication; a successful authenticated zome probe is
  passive evidence only and cannot initiate a bridge connection. Serving health
  is 503 when any supervised role is dead and returns to 200 when no supervised
  role is dead.

  App-only recovery is bounded from detection; restart recovery is bounded from
  conductor admin readiness because no client can authenticate while the conductor
  is down. This regression is tagged @wip until the owned-substrate fault-injection
  and restart steps are wired. Its bounds are a 20-second supervisor probe, recovery
  within 30 seconds, and two conductor restarts without a storage-process restart.

  Background:
    Given every supervised role in Matthew's storage health reports a live zome path
    And Matthew's storage serving-health endpoint returns 200

  Scenario: A live admin path cannot hide a dead authenticated app path
    Given Matthew's conductor admin interface lists its installed apps
    And Matthew's authenticated infrastructure zome probe succeeds
    And Matthew records the infrastructure bridge reconnect count as the baseline
    When the authenticated app websocket is closed without closing the admin websocket
    Then the conductor admin interface still lists its installed apps
    And no later than the next 20-second supervisor probe one serving-health response is 503 with the infrastructure zome path dead
    And that response reports the infrastructure bridge reconnect count as the baseline plus 1
    And without operator intervention a serving-health response is 200 with the infrastructure zome path live within 30 seconds of detection
    And a passive authenticated infrastructure zome probe succeeds

  Scenario: PeerStatus and serving health recover through two conductor restarts
    Given Matthew's storage peer is publishing PeerStatus through a live authenticated app websocket
    And Matthew records the pre-restart timestamp from his storage peer-status projection
    And Matthew records the storage process identifier
    When Matthew initiates the first conductor restart
    Then the current authenticated app websocket closes
    And no later than the next 20-second supervisor probe one serving-health response is 503 with the infrastructure zome path dead
    When the restarted conductor admin interface becomes ready
    Then without operator intervention within 30 seconds one serving-health response is 200 with every supervised zome path live
    And a passive authenticated infrastructure zome probe succeeds
    And within one 60-second heartbeat tick Matthew's storage peer-status projection shows and records the first recovered timestamp later than the pre-restart timestamp
    When Matthew initiates the second conductor restart
    Then the first recovered authenticated app websocket closes
    And no later than the next 20-second supervisor probe one serving-health response is 503 with the infrastructure zome path dead
    When the restarted conductor admin interface becomes ready again
    Then without operator intervention within 30 seconds one serving-health response is 200 with every supervised zome path live
    And a passive authenticated infrastructure zome probe succeeds again
    And within one 60-second heartbeat tick Matthew's storage peer-status projection shows a timestamp later than the first recovered timestamp
    And Matthew's storage process identifier is unchanged
