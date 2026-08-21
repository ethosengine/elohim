@e2e @lamad @browser-only @discovery @act:i
Feature: Know Thyself Discovery Path
  As a learner beginning their self-discovery journey,
  I want to complete discovery assessments on the know-thyself path,
  so that I receive meaningful personality/values insights.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # @wip: the discovery-result assertions below drifted against dead testids
  # (discovery-subscale-score / discovery-profile). The shipped result view is the
  # assessment-completion-summary component (app/lamad, completion-* testids) — canonical
  # coverage lives in assessment-completion-feedback.feature. See backlog
  # ci-genesis-discovery-result-testid-drift. Held from CI until reconciled/consolidated.
  # HELD (2026-08-21): Superseded: the file's own comment says canonical coverage now lives in assessment-completion-feedback.feature; dead testids (backlog ci-genesis-discovery-result-testid-drift).
  @values-hierarchy @wip @act:iii
  Scenario: Terrance completes the Values Hierarchy assessment
    Given human "Terrance" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance to the "Values Hierarchy" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the assessment should complete without console errors
    And the resonance result should have non-zero subscale scores
    And the "values-examined" attestation should be recorded
    And I should see my values profile breakdown

  # @wip: asserts dead testid discovery-subscale-score (superseded by completion-subscales in the
  # shipped assessment-completion-summary component). See backlog ci-genesis-discovery-result-testid-drift.
  # HELD (2026-08-21): Superseded by assessment-completion-feedback.feature (duplicate coverage).
  @attachment-style @wip @act:iii
  Scenario: Jessica completes the Attachment Style assessment
    Given human "Jessica" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance to the "Attachment Style" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the assessment should complete without console errors
    And the resonance result should have non-zero subscale scores
    And the "attachment-aware" attestation should be recorded

  # @wip: the badge assertion targets dead testid discovery-attestation-badge and depends on the
  # completion flow firing. See backlog ci-genesis-discovery-result-testid-drift.
  # HELD (2026-08-21): Superseded by assessment-completion-feedback.feature (duplicate coverage).
  @attestation @wip @act:iii
  Scenario: First discovery assessment earns a milestone attestation
    Given human "Terrance" is logged in on doorway "alpha" with device
    And Terrance has not completed any discovery assessments
    When I navigate to the "Know Thyself" path
    And I advance to the "Values Hierarchy" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the assessment should complete without console errors
    And the "values-examined" attestation should be recorded
    And the "first-discovery" attestation should be recorded
    And I should see the "first-discovery" attestation badge on my profile

  @regression
  Scenario: Single radio/likert selection does not select all options
    Given human "Terrance" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance to the "Values Hierarchy" assessment step
    And I start the assessment
    And I select value "3" on the first likert-scale question
    Then only one selection should be active
    And no other options should appear selected

  @regression
  Scenario: No console errors during assessment navigation
    Given human "Terrance" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance through each chapter
    Then no JavaScript errors should be captured
    And no failed network requests should be captured
