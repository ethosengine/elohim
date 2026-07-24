@e2e @federation @resilience
Feature: Doorway multi-address failover — the browser keeps working when the primary doorway address goes dark
  As a household member using the elohim app from a browser
  I want my session to keep serving content through a configured fallback
  doorway address when the primary address stops answering
  So that one doorway's WAN outage never strands my view of the household's
  content — reads keep flowing through whichever address is actually up,
  writes tell me the truth instead of pretending, and my client remembers
  where it landed instead of flapping between addresses on every request.

  # This is the browser leg of "logical anycast" doorway failover (option
  # §3a, multi-A + client retry): ElohimClient SDK and the Angular
  # apiBaseUrlInterceptor both read a configured fallback list
  # (`mode.doorway.fallbacks` / `environment.client.doorwayFallbacks`) and
  # fail reads over to it on a network error, sticking to whichever address
  # last worked. Writes never auto-retry across addresses — they only
  # advance which address is "sticky preferred" once a subsequent read
  # proves it live. The conductor plane (bootstrap_url/signal_url) does NOT
  # get this treatment — those fields are single-typed upstream and the far
  # path is a tx5-fork patch, filed separately.
  # Spec: genesis/docs/superpowers/specs/2026-07-16-dual-wan-utility-plane-failover-design.md (§3a, §7)

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "alpha" is configured with a fallback doorway address "E2E_DOORWAY_ALPHA_FALLBACK"

  Scenario: Reads keep serving through the fallback address when the primary goes dark
    Given the person is viewing a lamad content page served by the primary doorway address
    When the primary doorway address stops answering
    And the person navigates to another piece of content
    Then the content loads through the fallback doorway address
    And the person's session and content view are not lost
    And subsequent requests from the client stay sticky to the fallback address

  Scenario: A write during the outage fails honestly, then later traffic moves to the fallback
    Given the primary doorway address stops answering
    When the person submits a write while the client is still pointed at the primary address
    Then the write fails with an honest error and is not silently retried against another address
    When the person's next read succeeds through the fallback doorway address
    Then subsequent writes are attempted against the fallback doorway address

  Scenario: With no fallback configured, an outage degrades exactly as today
    Given doorway "alpha" has no fallback doorway address configured
    When the primary doorway address stops answering
    And the person navigates to another piece of content
    Then the request fails the same way it did before multi-address failover existed
    And no fallback address is attempted
