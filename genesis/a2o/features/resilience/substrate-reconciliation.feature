@resilience-p1 @substrate @concern:reconcile-inventory @dataplane
Feature: Delivery reconciles to the available compute when a peer goes offline
  As a household that learns on its own hardware
  I want a distant peer's machine going dark to never interrupt us
  So that the people in this home keep going, and the system plainly says
  what it could not reach instead of failing in a way no one can read

  Most of the people we model live on a single shared remote peer; the
  household — Matthew, Jessica, and James — runs on its own machines at home.
  When the remote peer is offline, the run scales DOWN to the household and
  names who it could not reach: loud, but never fatal. When the remote peer
  returns, a re-run restabilizes to the full set of people, with no one
  editing any config.

  This is the test-bench guard for that contract. It reads the substrate
  signal this run was given (ELOHIM_REMOTE_COMPUTE_STATUS) and asserts the invariant
  that holds whether the remote peer is up or down: the household is always
  reachable, and only remote-only people are ever scaled out.

  Background:
    Given the substrate signal has been read for this run

  @e2e
  Scenario: The household is never gated by a remote peer
    Then household person "Matthew" is reachable for testing
    And household person "Jessica" is reachable for testing
    And household person "James" is reachable for testing

  @e2e
  Scenario: A remote-only person is reachable exactly when the remote peer is up
    Then remote-only person "Gertrude" is reachable only when the remote peer is available

  @e2e
  Scenario: The run only ever scales down remote-only people, never the household
    When the household and a remote-only person are checked for reachability
    Then no household person has been scaled out this run
    And every person scaled out this run is a remote-only person

  @wip
  Scenario: A household learner keeps reading while the remote peer is offline
    Given the remote peer is offline
    And human "Matthew" is logged in on doorway "alpha" with device
    When Matthew opens a learning path that lives on the household
    Then the content is delivered from the household
    And the run is marked reduced-scope, naming the people it could not reach

  # P1 reconciliation stream: edge-triggered ReaProjectionSignals are the only
  # thing that lands an REA commitment into a peer's local projection. A peer
  # that misses the signal (stale binary, restart window, gossip race) stays
  # divergent, and a reseed on the originator collapses to 409 so the signal
  # never re-fires. This is the gap that left adam divergent for 10 days. The
  # cure: peers discover what each other holds, and each peer heals the rows it
  # is missing from its OWN conductor (the DHT notary view), never from peer
  # bytes. Convergence becomes a property of the running mesh, not of a lucky
  # signal at author time.
  @wip @regression
  Scenario: A peer-discovered commitment converges from the own conductor
    Given a commitment exists on the household mesh that one peer missed the signal for
    And that peer's projection is missing the commitment
    When the projection-reconcile sweep asks its connected peers what they hold
    And a connected peer advertises the missing commitment id in its inventory
    Then the peer heals the commitment row from its own conductor, not from the peer
    And the healed row carries the same dht anchor hash the conductor notarized
    And the heal is named as a mutual-aid event identifying the discovering peer
