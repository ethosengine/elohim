@e2e @doorway @resilience @regression @requires:doorway @wip
Feature: Doorway peer-conductor connections back off and do not leak
  As a household member whose doorway fronts a pool of peer conductors
  I want the doorway to retreat politely when a conductor session is unstable
  So that one flapping or auth-rejecting conductor never burns the doorway
  in a tight reconnect loop, and a mass reconnect never staggers the fleet.

  # Engineering constraint (story-harvest, 2026-06-10 conductor reconnect storm):
  # the doorway's reconnect loops reset their delay on a *successful WebSocket
  # connect* (worker/conductor.rs reset reconnect_delay to 100ms on connect).
  # A conductor that accepts the WebSocket but rejects the authenticate
  # handshake — or drops the session within seconds — therefore reset the
  # backoff every cycle and the loop ran at ~10Hz. Worse, every pool-worker
  # reconnect recreated its ConductorConnection, leaking the previous detached
  # connection loop as an immortal ~10Hz spammer. Fleet measurement at peak:
  # ~510 connects/sec across two doorways, ~132k conductor auth-drop log lines
  # per 10 minutes, coredns at ~1030 q/s, conductors liveness-killed (exit 137)
  # in a roaming pattern. Constraints pinned here:
  # 1. unstable sessions must count as failures — backoff resets only after a
  #    session holds a minimum stable window (10s);
  # 2. connection tasks must die with their owners — one live connection task
  #    per worker, no leak across reconnect cycles;
  # 3. an unstable authenticated session must re-mint its app-auth token
  #    (conductor restart invalidates issued tokens; without re-mint the pool
  #    stays broken until the doorway restarts).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: Auth-rejected peer conductor is retried with exponential backoff
    Given a peer conductor in the pool accepts WebSocket connections but rejects the authenticate handshake
    When the doorway's worker attempts to connect 5 times
    Then each retry delay at least doubles the previous delay up to the 30 second cap
    And the doorway marks the conductor pool unhealthy while disconnected
    And the doorway's other pool conductors continue serving requests

  Scenario: Unstable sessions do not reset the backoff clock
    Given a peer conductor that accepts connections but drops every session within 2 seconds
    When the doorway reconnects through 5 flap cycles
    Then the reconnect delay keeps growing across cycles instead of resetting to the floor
    And the backoff resets only after a session stays healthy for the minimum stable window

  Scenario: Reconnect cycles do not leak connection tasks
    Given the doorway is connected to a peer conductor with an active signal subscriber
    When the conductor session drops and reconnects 10 times
    Then the doorway holds exactly one live connection task per conductor worker
    And the doorway's task count stays flat across the cycles

  Scenario: Conductor restart heals the worker pool without a doorway restart
    Given a peer conductor that restarts and invalidates previously issued app auth tokens
    When the doorway's pool workers reconnect with the stale token
    Then the doorway re-mints an app auth token from the conductor admin interface
    And zome calls through that conductor pool succeed within the recovery window

  Scenario: Reconnect churn is visible to operators
    Given a peer conductor that drops its signal subscriber session repeatedly
    When the operator reads the doorway status endpoint
    Then the peer health snapshot shows a growing reconnect attempt count for that conductor
    And the peer is marked degraded with reason "reconnecting"
    # Observability gate: this storm ran for hours while /status read
    # reconnect_attempts: 0 for every peer — the diagnostic had to come from
    # conductor-side log floods instead of the doorway's own health surface.

  @requires:shem
  Scenario: Conductor fleet survives a doorway reconnect storm
    Given a doorway fronting the full multi-tenant conductor fleet
    And every conductor session is severed at the same moment
    When all pool workers reconnect
    Then reconnect attempts escalate independently so no conductor sees a sustained synchronized herd
    And every conductor returns to healthy within the recovery window
    And no conductor reports more concurrent sessions than the worker pool size

  # Operational parameters (measured 2026-06-10, alpha fleet):
  #   base reconnect delay 100ms, ceiling 30s, stable-session window 10s,
  #   token re-mint rate limit 30s, subscriber wait 5s base -> 60s ceiling.
  #   Storm magnitude without these: ~510 connects/sec fleet-wide 30min after
  #   a fresh doorway boot, ~2,000/sec after 70min (leak accumulation),
  #   ~132k conductor auth-drops / 10min, coredns ~1,030 q/s, conductor
  #   /health starvation -> liveness SIGKILL (exit 137) roaming the fleet.
  # Informs: doorway deployment presets (worker_count x conductor count is the
  #   reconnect-pressure multiplier), conductor liveness timeout sizing,
  #   household-vs-multi-tenant peer diversity (WAN links amplify the blast).
  # Review after: Holochain app-interface auth changes, conductor pool scaling.
