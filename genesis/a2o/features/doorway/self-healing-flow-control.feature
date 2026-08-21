@e2e @doorway @resilience @regression @requires:doorway @act:i
Feature: Doorway self-protects and stays within capacity under load
  As a household member whose doorway fronts storage and a pool of peer conductors
  I want the gateway to protect itself — gate broken upstreams during warm-up,
  shed inbound load it cannot absorb, and break the circuit to a failing upstream —
  So that no surge, no flapping upstream, and no cold-start churn can ever drive
  the node into the freeze/overwhelm class, and the node stays responsive without
  an operator.

  # This is the DURABLE structural layer the 2026-06-13 doorway-alpha freeze
  # pointed to. The freeze itself (deadlock under load, /health included) and its
  # immediate cures (conductor-call timeout, explicit tokio workers, SSR shed) are
  # harvested in doorway/peer-conductor-connection-resilience.feature. THIS file
  # captures the self-healing control-plane built on shift/self-healing-control-plane
  # (commits b7998c1f3..1bdbab4a8): warm-up upstream self-protection (Plan A),
  # bilateral inbound/outbound flow control (Plan B), and the self-healing read
  # model that lets an operator OR an agent see which mechanism is exhausted (Plan C).
  #
  # The headline invariant across every scenario below: STRUCTURAL NO-OVERWHELM —
  # an unhealthy peer, a saturated inbox, or a failing storage upstream degrades to
  # a bounded, advertised shed; it never parks the runtime or partitions the node.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ─── Plan A: warm-up upstream self-protection ──────────────────────────────
  # The freeze's CPU-bound arm: warm_stream::spawn_stream_task looped ~10 broken
  # upstreams SEQUENTIALLY through a 5x backoff ladder with a per-stream timeout
  # that EXCEEDED the liveness-kill window. The cure gates known-bad upstreams out
  # of the serial pass, caps the whole pass with a total budget under the kill
  # window, and yields the worker periodically so warm-up never starves liveness.

  @regression @requires:owned-substrate
  Scenario: A repeatedly-failing warm-up upstream is gated out of the serial pass
    # Constraint: without the breaker, every broken upstream re-entered the full
    # backoff ladder one at a time, serializing CPU churn that froze the gateway.
    Given a storage upstream that has failed its warm-up stream 3 times this pass
    When the doorway starts the next warm-up pass
    Then that upstream's circuit is Open and it is skipped before the serial loop runs
    And the doorway records the skip as a self-heal event for the elevate arm

  @regression
  Scenario: Warm-up obeys a total budget shorter than the liveness-kill window
    # Constraint: the per-stream 45s timeout could stack across upstreams past the
    # ~150-210s liveness-kill window. A single total budget bounds the whole pass.
    Given several storage upstreams are slow to answer their warm-up stream
    When the cumulative warm-up time reaches the total warm-up budget
    Then the doorway abandons the remaining streams for this pass
    And the liveness probe answers throughout the warm-up pass

  @regression @requires:owned-substrate
  Scenario: The upstream gate never empties the node off the network
    # Anti-self-partition invariant (gate_upstreams never returns []). If EVERY
    # upstream looks unhealthy, the gate must still hand back the full set to try —
    # an empty gate would isolate the node from the whole DHT, a worse failure than
    # a slow warm-up. The trip is recorded as an elevate-worthy self-heal event.
    Given every storage upstream's warm-up circuit is Open
    When the doorway computes the warm-up upstream set
    Then the set is non-empty and contains every configured upstream
    And the doorway records an "anti-self-partition" self-heal event

  # ─── Plan B: bilateral inbound/outbound flow control ───────────────────────
  # WE OWN BOTH ENDPOINTS, so every edge is a two-way contract: inbound admission
  # sheds what the node cannot absorb; the outbound breaker stops hammering a
  # failing upstream and surfaces the upstream's own backpressure to the client.
  # Shedding ALWAYS advertises a Retry-After — the client is told to slow down,
  # never silently dropped or blocked behind an unbounded queue.

  @regression
  Scenario: Inbound saturation sheds with 503 catching-up, never an unbounded queue
    # Constraint: the freeze's queue arm — a capacity-1 render path back-pressured
    # onto the runtime. The general cure is a bounded inbound permit set: when it is
    # exhausted the request is SHED (try_acquire, never await a permit), not queued.
    Given the doorway's inbound permits are exhausted
    When a non-exempt request arrives
    Then the request is shed with HTTP 503 and a Retry-After header
    And the body advertises status "catching-up"
    And the doorway never blocks the caller waiting for an inbound permit

  @regression
  Scenario: Liveness and WebSocket upgrades are exempt from inbound shedding
    # Constraint: the freeze killed /health too. Liveness MUST answer even while
    # the node sheds everything else, or the orchestrator SIGKILLs a node that is
    # correctly self-protecting. WebSocket upgrades (signal relay) are also exempt.
    Given the doorway is shedding inbound requests because its permits are exhausted
    When the liveness probe and a WebSocket upgrade arrive
    Then both are served without consuming an inbound permit
    And only non-exempt requests receive the 503 catching-up shed

  @regression @requires:owned-substrate
  Scenario: A failing storage upstream trips the breaker and the doorway stops hammering it
    # Constraint: outbound self-protection. After the failure threshold the breaker
    # opens and the doorway sheds catching-up WITHOUT calling storage at all — it
    # neither piles load onto a struggling upstream nor blocks its own callers.
    Given the storage upstream has returned errors past the upstream failure threshold
    When a request that would proxy to storage arrives
    Then the doorway sheds it with 503 catching-up without calling storage
    And the shed advertises a Retry-After header

  @regression
  Scenario: The breaker records exactly one outcome per terminal proxy path
    # Constraint discovered in verification: an early "record from status, then
    # record again from the body" double-counted, so a 200-then-stall (headers OK,
    # body never completes) recorded a SUCCESS and the breaker NEVER opened for that
    # class. The breaker now records once, at exactly the terminal path, so a
    # body-phase failure counts toward opening the circuit.
    Given a storage upstream that returns 200 headers then stalls before the body completes
    When that failure mode repeats past the upstream failure threshold
    Then the breaker opens for that upstream
    And subsequent requests are shed catching-up rather than retried into the stall

  @regression
  Scenario: A client-error response during half-open does not consume the recovery trial
    # Constraint discovered in verification: a half-open breaker allows ONE trial
    # call. An early-return on a 405/400 (a legitimate client-class response from a
    # HEALTHY upstream) consumed that trial WITHOUT recording an outcome, wedging
    # the breaker half-open forever. The is_open() check now brackets exactly the
    # send(), so a client-error reply records a success and the breaker can close.
    Given the storage upstream's breaker is half-open and allowing one trial call
    When the trial call returns a client-error status from a healthy upstream
    Then the trial is recorded as a success, not consumed without an outcome
    And the breaker is eligible to close on the next healthy response

  @wip
  Scenario: The storage breaker recovers after its cooldown
    # Unlike the warm-up breaker (single startup pass, frozen tick — recovery there
    # is intentionally inert), the storage proxy breaker runs on a real wall clock.
    # After the cooldown elapses it admits a half-open trial, and a success closes it.
    Given the storage upstream's breaker is Open
    And the upstream has recovered and now answers normally
    When the breaker's cooldown window elapses and a request arrives
    Then the doorway admits a single half-open trial to the upstream
    And the successful trial closes the breaker and normal proxying resumes

  @regression
  Scenario: Storage sheds per request instead of blocking its accept loop
    # Constraint: storage previously acquired its inflight permit on the ACCEPT
    # loop, so a saturated node stopped accepting connections (a silent stall). The
    # gate moved per-request: an over-capacity request is shed with Retry-After and
    # the accept loop keeps running so /health and /version still answer.
    Given elohim-storage's inflight permits are exhausted
    When a non-exempt request arrives at storage
    Then storage sheds it with HTTP 503 and a Retry-After header
    And storage continues accepting connections so liveness still answers

  @regression
  Scenario: The doorway surfaces an upstream's own backpressure instead of a bare 502
    # Constraint: when storage itself sheds (429/503 with its own Retry-After), the
    # doorway must HONOR and forward that backpressure as catching-up — not mask it
    # as a generic 502, which would tell the client to retry immediately and amplify.
    Given elohim-storage responds with 503 and its own Retry-After header
    When the doorway proxies a request to storage
    Then the doorway surfaces 503 catching-up to the client preserving the upstream Retry-After
    And the doorway does not return a bare 502

  # ─── Plan C: the self-healing read model (observe / elevate) ───────────────
  # The control plane is observable. One node-local read model (Category C —
  # node-local operational state, never DHT-notarized) lets an operator, a future
  # controls UI, OR an AI agent self-debugging the runtime see which mechanism is
  # exhausted. The external elevate-arm poller (Plan D, runtime-harvest) READS this
  # endpoint; the poller itself is agent-runtime infrastructure, not a node-served
  # human capability, so it is not harvested as an a2o scenario — this endpoint is.

  @wip
  Scenario: An operator or agent can read the unified self-healing model
    # Observability gate: GET /admin/self-healing returns one read model covering
    # upstreams, projector, peers, render, warm-up, and conductor health so the
    # exhausted mechanism is visible without scraping logs. NOTE: the admission,
    # upstreams, and autoPreset blocks are present in the wire contract but report
    # empty/null pending the inbound-admission, upstream-breaker, and auto-config
    # wire-up follow-ons — the scenario asserts the SHAPE, not those pending values.
    Given the doorway is serving
    When an operator reads "/admin/self-healing"
    Then the response includes the projector, peers, render, warmup, and conductor blocks
    And the projector block reports whether the node is caught up
    And the render block reports the degenerate (stalled or timed-out) rate

  # Operational parameters (parameter-bearing — branch shift/self-healing-control-plane):
  #   WARMUP_STREAM_TIMEOUT_SECS        45    per-stream warm-up timeout
  #   WARMUP_TOTAL_BUDGET_SECS          75    whole-pass budget (proven <= kill/2 in-code)
  #   WARMUP_CIRCUIT_FAIL_THRESHOLD     3     fails before a warm-up upstream is gated out
  #   WARMUP_CIRCUIT_COOLDOWN_TICKS     5     (inert in doorway: single-pass frozen tick=0)
  #   WARMUP_YIELD_EVERY_N              64    yield-to-scheduler cadence during warm-up
  #   DEFAULT_MAX_INFLIGHT              256   doorway inbound permit ceiling
  #   MIN_MAX_INFLIGHT                  8     floor below which inbound never drops
  #   DOORWAY_ADMISSION_RETRY_AFTER_SECS 2    advertised on an inbound shed
  #   STORAGE_SHED_RETRY_AFTER_SECS      2    advertised on a storage per-request shed
  #   UPSTREAM_CIRCUIT_FAIL_THRESHOLD   3     fails before the storage proxy breaker opens
  #   UPSTREAM_CIRCUIT_COOLDOWN_SECS    30    wall-clock cooldown before a half-open trial
  #   STORAGE_PROXY_CONNECT_TIMEOUT_SECS 3    proxy connect deadline (bounded, never unbounded)
  #   STORAGE_PROXY_REQUEST_TIMEOUT_SECS 12   proxy request deadline
  #   elevate-arm poller (runtime-harvest): OPEN/SHED/LAG_POLLS=3, LAG_SECONDS=30,
  #     DEGEN_RATE=0.25, WINDOW=8, CLOSE_STREAK=3, MAX_NEW_FINDINGS=12 — the
  #     sustained-condition thresholds that turn a transient blip into an elevated finding.
  # Informs: doorway/storage deployment presets (inflight ceiling x permit floor is
  #   the node's absorb-vs-shed budget; it scales with cgroup mem/cpu and peer count —
  #   a laptop floor sheds sooner than a network node), liveness timeout sizing
  #   (must exceed WARMUP_TOTAL_BUDGET_SECS), and peer-diversity presets.
  # Review after: tokio major upgrades, conductor app-interface changes, storage proxy
  #   client changes, and when the auto-config (arc-policy) thread lands its derive() —
  #   the inflight ceiling and warm-up budget become Auto-derived rather than constants.
