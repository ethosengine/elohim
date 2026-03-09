@e2e @lamad @browser-only @completion
Feature: Assessment Completion Feedback
  As a learner finishing an assessment,
  I want personalized, celebratory feedback at the completion moment,
  so that I feel accomplished and connected to my results.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ─── Discovery Completion ───────────────────────────────────────────────────

  @discovery @happy-path
  Scenario: Learner sees personalized discovery result after completing assessment
    Given human "Timothy" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance to the "Attachment Style" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the completion summary should be visible
    And the completion headline should contain a personalized type name
    And a hex badge preview should be displayed
    And the completion description should reference the primary type
    And the profile link should read "View on your profile"

  @discovery @subscales
  Scenario: Discovery completion shows subscale breakdown bars
    Given human "Susan" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance to the "Attachment Style" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the completion summary should be visible
    And the subscale breakdown should be visible
    And each subscale bar should have a non-zero width

  @discovery @attestation
  Scenario: Discovery completion records an attestation
    Given human "Timothy" is logged in on doorway "alpha" with device
    And Timothy has not completed any discovery assessments
    When I navigate to the "Know Thyself" path
    And I advance to the "Attachment Style" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the completion summary should be visible
    And the "attachment-aware" attestation should be recorded
    And I should see the "first-discovery" attestation badge on my profile

  @discovery @continue
  Scenario: Learner continues after discovery feedback
    Given human "Timothy" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance to the "Attachment Style" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the completion summary should be visible
    When I click the completion continue button
    Then the completion summary should not be visible

  # ─── Mastery Completion ─────────────────────────────────────────────────────

  @mastery @happy-path
  Scenario: Learner sees score feedback after passing a mastery quiz
    Given human "Matthew" is logged in on doorway "alpha" with device
    And the "Elohim Protocol" path exists
    When I start the "Elohim Protocol" path
    And I advance to a mastery quiz step
    And I answer all quiz questions correctly
    Then the completion summary should be visible
    And the completion headline should contain "Well Done"
    And the score display should show a percentage
    And the result card should have the "passed" style
    And the profile link should read "View your Dashboard"

  @mastery @failing
  Scenario: Learner sees encouragement after failing a mastery quiz
    Given human "Matthew" is logged in on doorway "alpha" with device
    And the "Elohim Protocol" path exists
    When I start the "Elohim Protocol" path
    And I advance to a mastery quiz step
    And I answer all quiz questions incorrectly
    Then the completion summary should be visible
    And the completion headline should contain "Keep Learning"
    And the result card should have the "failed" style
    And no hex badge preview should be displayed

  @mastery @no-attestation
  Scenario: Mastery completion does not record a discovery attestation
    Given human "Matthew" is logged in on doorway "alpha" with device
    And the "Elohim Protocol" path exists
    When I start the "Elohim Protocol" path
    And I advance to a mastery quiz step
    And I answer all quiz questions correctly
    Then the completion summary should be visible
    And no discovery attestation should be recorded

  # ─── Reflection Mode ────────────────────────────────────────────────────────

  @reflection
  Scenario: Reflection assessment shows generic completion
    Given human "Susan" is logged in on doorway "alpha" with device
    When I navigate to a reflection assessment
    And I answer all reflection prompts
    Then the completion summary should be visible
    And the completion headline should contain "Assessment Complete"
    And no hex badge preview should be displayed
    And no score display should be shown

  # ─── Edge Cases ─────────────────────────────────────────────────────────────

  @regression
  Scenario: No console errors during completion feedback
    Given human "Timothy" is logged in on doorway "alpha" with device
    When I navigate to the "Know Thyself" path
    And I advance to the "Attachment Style" assessment step
    And I start the assessment
    And I answer all likert-scale questions with varied selections
    Then the completion summary should be visible
    And no JavaScript errors should be captured
    And no failed network requests should be captured
