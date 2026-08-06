# Doorway failover — the "two doorways, one name" invariant the resiliency saga
# ends on. Chapter 4 of the saga proves hosting is real in steady state (GET /
# serves the shell); THIS concern asserts the stronger thing: hosting SURVIVES —
# a person hitting the apex name gets the landing shell even while one doorway
# is dead or in its post-deploy catch-up shed window.
#
# Spine node: doorway-failover (genesis/manifests/habits.yaml). Sprint plan:
# genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md.
#
# Measurement philosophy (v1): NO synthetic chaos. Adam's post-deploy
# arc-convergence window reopens for hours after every edge deploy
# (self-heal-adam-projection-catchup-exhaustion-full-arc.md), so the live pair
# organically supplies real shed windows; scenarios assert the invariants that
# must hold THROUGH those windows. v2 escalation (not built): the kill-on-purpose
# shape from features/federation/peer-loss-failover.feature applied at the
# doorway layer.
#
# Shed-vs-dead vocabulary (steps/dataplane/failover.steps.ts):
#   serving  = GET / answers 200
#   shedding = 503 carrying the specified catching-up contract
#              ({"status":"catching-up"} body or an open upstream circuit /
#              rising admission.shedTotal in status.json) — honest degradation,
#              spec: 2026-07-19-doorway-catching-up-page-design
#   dead     = connect error / timeout on BOTH / and /health — an outage
#
# Born red 2026-07-31: scenario "The apex name survives its doorway's shed"
# live-probed 503 (elohim.host mid-catch-up) while doorway-alpha served 200.
# The assertion IS the specification for the cure (apex multi-A + client
# fallback + warm-boot shell cache + the adam provisioning ceiling) — do not
# weaken it to pass.
@e2e @dataplane @concern:doorway-failover
Feature: Doorway failover — two doorways, one name, one truth
  A person reaching for elohim.host should never inherit a single doorway's bad
  hour. The doorway pair must be honestly classifiable, someone must always be
  serving, the apex name must ride through a sibling's shed, and whoever serves
  must serve the same declared truth.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: Every doorway is honestly classifiable — shed is not death
    # A doorway mid-catch-up is DEGRADED, not down: it must answer its /health
    # and present the specified shed contract, never a silent connection void.
    # This is the readiness contract any routing layer (multi-A client
    # fallback today, LB health checks later) gets to route on.
    Then doorway "alpha-A" classifies as serving or shedding, not dead
    And doorway "elohim.host" classifies as serving or shedding, not dead

  Scenario: The pair floor holds — at least one doorway is serving
    # Both-shedding (or worse) means no human can reach the commons at all —
    # the floor this arc exists to keep. Green today via whichever sibling is
    # outside its deploy window; a correlated outage turns it red honestly.
    Then at least one of doorways "alpha-A" and "elohim.host" is serving

  Scenario: The apex name survives its doorway's shed
    # THE BORN-RED. elohim.host (the apex) is pinned to doorway-B; when B enters
    # its hours-long post-deploy catch-up, the NAME sheds even though a healthy
    # sibling doorway holds the identical converged content. Saga ch04 accepts
    # steady-state green; this scenario does not — the name, not the pod, is
    # what a person trusts.
    When I query "/" on peer "elohim.host" expecting raw text
    Then the raw response status is 200
    And the raw response body contains "app-root"

  Scenario: Whoever serves, serves the same declared truth
    # Failover that changes the answer is worse than an outage. Every doorway
    # currently classified as serving must resolve the declared head for the
    # landing content — and when both serve, their heads must be identical
    # (the ch10 "two doorways, one truth" bar, held through failover).
    Then every serving doorway among "alpha-A" and "elohim.host" resolves the same declared head for content "elohim-host-landing"
