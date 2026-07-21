# Landed with the doorway catching-up shed page (spec:
# 2026-07-19-doorway-catching-up-page-design). The shed contract itself —
# browser navigations get the staged HTML recovery page, every other client
# keeps the exact legacy {"status":"catching-up","retryAfter":N} JSON, both
# always 503 + Retry-After — is unit-anchored in doorway-service
# (routes::catching_up tests: negotiation matrix, template render, header
# preservation). Forcing a live shed against a healthy peer would be
# flaky-by-design, so this feature pins what is ALWAYS live-observable:
# the recovery-progress fields the page polls, and the diagnostics-bypass.
#
# Motivating incident (2026-07-19): doorway-alpha-b's breaker to its storage
# peer (adam) flapped open/half-open for hours; every route on elohim.host —
# including the diagnostic ones — served only the bare shed JSON. The
# diagnostics scenario pins the self-blinding fix: probe routes bypass the
# breaker, or an incident hides its own evidence.
@e2e @dataplane @concern:doorway-catching-up-page
Feature: Doorway catching-up page — staged, honest shed progress for people

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: status.json always carries the recovery-progress fields
    # The catching-up page's poll target is doorway-local (never proxied,
    # never shed), so its fields must be present on a HEALTHY doorway too —
    # that is what lets the page observe recovery the moment it happens.
    When I query "/status.json" on peer "alpha-A"
    Then the surface response status is 200
    And the surface response has field "upstreams"
    And the surface response has field "admission"

  Scenario: diagnostic probes are never answered with the doorway shed body
    # /p2p/status and /db/p2p/conductor-diagnostics bypass the upstream
    # breaker entirely (neither shed nor recorded): during an upstream
    # incident they return the upstream's real response or its real failure —
    # never the doorway's own shed. Healthy substrate → trivially green;
    # incident → this is the probe-stays-answerable guarantee.
    When I query "/p2p/status" on peer "alpha-A"
    Then the surface response is not the doorway shed body
    When I query "/db/p2p/conductor-diagnostics" on peer "alpha-A"
    Then the surface response is not the doorway shed body

  # Harvested from the 2026-07-20/21 adam slow-link incident (finding:
  # history/2026-07-20-adam-slow-link-write-guard-saturation.md). What looked like
  # a "flapping" breaker was, for 3+ hours, a half-open LATCH: the doorway shed
  # every render while awaiting a trial outcome no terminal code path recorded. The
  # only reason latch was distinguishable from flap was that status.json's upstream
  # carried a circuit state plus a monotonic errorStreak that sat pinned. Pin those
  # fields so the diagnostic that named the second defect can never silently regress.
  @wip @regression
  Scenario: each upstream in status.json carries circuit state and error streak
    When I query "/status.json" on peer "alpha-A"
    Then the surface response status is 200
    And each entry under "upstreams" has field "circuit"
    And each entry under "upstreams" has field "errorStreak"
    And the surface response field "admission" has field "shedTotal"
    # circuit in {closed, half-open, open}; errorStreak climbs per failure and
    # resets to 0 on a recorded success. A half-open circuit whose errorStreak
    # climbs then freezes is a LATCH (no terminal record()), not a flap — and that
    # distinction is the entire diagnosis. shedTotal counts gate-shed attempts
    # (admission refused with no upstream try), separating shed from upstream error.

  # The bridge that holds: a person on elohim.host stays fast even while its storage
  # peer is mid-storm, because the doorway serves the SSR shell from its warm hot
  # cache with no synchronous upstream fetch. Live-observable in the always-true
  # direction — a cache-warm healthy doorway renders with zero upstream fetches — so
  # we pin THAT invariant rather than force a flaky live degradation.
  @wip @regression
  Scenario: a cache-warm doorway renders the shell without a synchronous upstream fetch
    Given the doorway hot cache on peer "alpha-A" is warm
    When I request "/" on peer "alpha-A"
    Then the surface response status is 200
    And the response header "x-ssr-rendered" is "1"
    And the response header "x-ssr-fetches" is "0"
    # x-ssr-fetches: 0 proves the render did not block on the storage upstream — the
    # insulation that keeps people fast while a degraded peer converges behind the
    # breaker. Operational witness: during the adam storm elohim.host served
    # x-ssr-rendered:1 x-ssr-fetches:0 while adam sat pinned at ~5 cores.
