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

  # ─────────────────────────────────────────────────────────────────────────
  # Engineering constraint (story-harvest, 2026-06-13 doorway-alpha freeze):
  # the gateway did not crash — it DEADLOCKED under load and stopped serving
  # everything, /health included (all 10 OS threads parked in futex_wait, 0
  # log lines for 60s, nginx upstream 499/504). Three structural defects
  # compounded on a pod with limits.cpu: 1 (so tokio ran a SINGLE worker thread):
  #   1. conductor/zome calls had NO timeout — an intermittent cluster NXDOMAIN
  #      ("Failed to connect to conductor: Name or service not known") or a
  #      never-settling record_heartbeat zome call parked the calling task forever;
  #   2. one synchronously-blocked await on the lone tokio worker froze the WHOLE
  #      runtime (a futex-blocked worker costs no CPU, so the cure is more worker
  #      threads — NOT more CPU);
  #   3. the single sequential SSR isolate used a BLOCKING send onto a capacity-1
  #      queue, so a stuck render back-pressured onto the runtime instead of
  #      shedding. A /bootstrap PUT flood was the trigger.
  # Constraints pinned here (the invariant: an unresponsive conductor degrades to
  # a bounded error and the gateway STAYS RESPONSIVE):

  @regression
  Scenario: A conductor that never answers yields a bounded error, not a hang
    # The load-bearing invariant. A zome/admin call to an unreachable or
    # silent conductor must resolve to an Err within the hard deadline — never
    # park the caller forever (which, on the cpu-limited pod, froze the gateway).
    Given a peer conductor that accepts the connection but never answers a zome call
    When the doorway issues a zome call through that conductor
    Then the call returns an error within the conductor-call hard deadline
    And the doorway clears that conductor connection so the next call reconnects
    And the doorway keeps serving other requests throughout

  @regression
  Scenario: The gateway stays responsive while one request path is wedged
    # Even if a single handler blocks, the runtime must schedule other work —
    # explicit (not CPU-derived) tokio worker threads break the single-blocked-
    # await wedge regardless of the cpu limit.
    Given the doorway runs with explicitly configured tokio worker threads
    And one request path is blocked on a slow upstream
    When a concurrent request hits the gateway during the block
    Then the concurrent request is served without waiting for the blocked path
    And the liveness probe on the main listener continues to answer

  @regression @requires:ssr-bundle
  Scenario: A saturated SSR render queue sheds to the CSR shell instead of queuing
    # The single sequential isolate must not let a stuck render back-pressure
    # onto the runtime. When the render queue is saturated the request is shed
    # fast (CSR shell) rather than blocked behind a possibly-stuck render.
    Given the SSR render isolate is busy with an in-flight render
    And its bounded render queue is already saturated
    When another SSR request arrives
    Then that request is shed to the CSR shell fallback promptly
    And the gateway does not block waiting for render-queue capacity

  # Operational parameters (2026-06-13 freeze fix — parameter-bearing):
  #   DOORWAY_ZOME_CALL_TIMEOUT_MS  conductor-call hard deadline, default 10000ms
  #                                 (mirrors the discover_existing_agents connect
  #                                 timeout and the SSR reqwest client timeout);
  #                                 0/garbage falls back to default, NEVER unbounded.
  #   DOORWAY_WORKER_THREADS        explicit tokio worker count, default 4. Set
  #                                 explicitly so the cgroup cpu limit cannot
  #                                 collapse the runtime to one worker. Fixes the
  #                                 wedge even at limits.cpu: 1.
  #   DOORWAY_SSR_FETCH_SOFT_BUDGET_MS  per-fetch SSR soft budget, default 1200ms
  #                                 (TracingFetcher); a fetch past it is recorded
  #                                 Stalled and rejected so the render falls back fast.
  #   render queue                  capacity-1 sequential isolate; try_send (not
  #                                 blocking send) sheds with RenderError::Busy ->
  #                                 CSR shell when saturated.
  #   DOORWAY_HEALTH_PORT           optional isolated probe listener; LEFT UNSET on
  #                                 alpha — it shares the main runtime, so a killer
  #                                 probe pointed at it would mask a partial wedge.
  # Informs: doorway deployment presets (worker_threads is the freeze-resistance
  #   knob, independent of limits.cpu; raise cpu only for SSR render throughput).
  # Review after: tokio major upgrades, conductor app-interface changes, any move
  #   to pool >1 SSR isolate or add V8-level render interrupt.
