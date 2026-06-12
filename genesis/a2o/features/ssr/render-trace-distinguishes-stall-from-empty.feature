@browser @ssr @delivery @observability @requires:doorway @wip
Feature: SSR render trace distinguishes a stalled fetch from a legitimately-empty render
  As a peer operator (and a learner on a slow substrate)
  I want a render whose upstream data never arrives to be distinguishable from a render that
  correctly produced an empty result
  So that a "blank page" self-classifies as either healthy-empty or degenerate-stall, and a
  stall falls back fast instead of hanging the visitor on a blank document

  The core elohim-render engine (consumed by BOTH doorway and a capable storage peer — SSR is
  p2p-native, not doorway-owned) captures a render trace below the framework, at the V8/DataFetcher
  boundary. The trace records a terminal classification that the flat fetched-inputs list cannot
  express:

    - rendered        — data fetches arrived; HTML carries content
    - rendered-empty   — a fetch returned 0 rows / 404; the empty state is TRUTHFUL
    - stalled          — a fetch never settled within the soft budget (the degenerate "arrivedPayloads: []")
    - timed-out        — the hard wall-time limit fired
    - errored          — a fetch or the render itself threw

  This is the cut that today's failures (EprRouter-empties-on-poisoned-row, DHT-anchor-gap 404s)
  collapse into an indistinguishable blank page. The terminal is surfaced operator-side as the
  `x-ssr-terminal` response header and threaded to logs via the `X-Observation-Id` correlation token.

  The deep timing mechanics (per-fetch offset/duration, soft-deadline breadcrumb) are covered by
  elohim-render Rust unit tests; this feature guards the human- and operator-observable contract.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @requires:ssr-bundle @regression
  Scenario: A healthy concept page reports a non-stalled terminal
    # A route whose upstream data arrives renders content and self-labels as rendered (not stalled).
    When the raw HTTP response for "/lamad/concept/fct-bible-micah-6-8" is captured
    Then the raw HTTP response status is 200
    And the raw HTTP response header "x-ssr-terminal" is present
    And the raw HTTP response header "x-ssr-terminal" is not "stalled"
    And the raw HTTP response header "x-observation-id" is present

  @requires:ssr-bundle @regression
  Scenario: An empty-but-healthy render is labelled rendered-empty, not stalled
    # A route whose upstream truthfully returns no content must NOT masquerade as a stall — the
    # empty state is a real measurement, not a missing payload.
    When the raw HTTP response for an SSR route whose upstream returns no content is captured
    Then the raw HTTP response status is 200
    And the raw HTTP response header "x-ssr-terminal" is "rendered-empty"

  @requires:ssr-bundle @regression
  Scenario: A stalled upstream falls back fast and self-labels as stalled
    # When a render's data fetch never settles, the visitor must receive the CSR shell promptly
    # (soft budget, not the full wall-time hang) and the operator must see the stall classified.
    When the raw HTTP response for an SSR route whose upstream stalls is captured
    Then the raw HTTP response header "x-ssr-terminal" is "stalled"
    And the raw HTTP response body contains the CSR shell fallback
    And the SSR fallback arrived before the hard wall-time limit
