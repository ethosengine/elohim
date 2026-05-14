@e2e @elohim @compute
Feature: Elohim Compute Coordination
  As a learner interacting with the Elohim Protocol,
  I want compute resources to be coordinated transparently
  so that I see cost transparency, graceful degradation, and economic accountability.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ─── Budget & Insight ──────────────────────────────────────────────────────

  @browser-only
  Scenario: Authenticated learner receives insight within budget
    Given human "Terrance" is logged in on doorway "alpha" with device
    And the elohim node has inference budget remaining
    When the learner completes the assessment
    Then an elohim insight section appears below the results
    And the insight shows tokens processed
    And the insight shows processing time in milliseconds

  # ─── Capacity Exhaustion ───────────────────────────────────────────────────

  @browser-only @wip
  Scenario: Request deferred when budget exhausted
    Given human "Terrance" is logged in on doorway "alpha" with device
    And the elohim node has exhausted its inference budget
    When the learner completes the assessment
    Then the insight shows a capacity unavailable message

  # ─── Economic Events ──────────────────────────────────────────────────────

  @browser-only @wip
  Scenario: Compute cost recorded as economic event
    Given human "Terrance" is logged in on doorway "alpha" with device
    When the learner receives an elohim insight
    Then a compute economic event is recorded
    And the event captures the tokens used and model
