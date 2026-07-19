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
