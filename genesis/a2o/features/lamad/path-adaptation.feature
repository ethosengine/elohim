@e2e @lamad @browser-only @adaptation
Feature: Adaptive Path Progression
  As a learner with prior knowledge or self-discovery insights,
  I want the learning path to recognize what I already know,
  So that I am guided — not imprisoned — by the sequential fog-of-war.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device
    And the "Elohim Protocol" path exists

  # ═══════════════════════════════════════════════════════════════════════════
  # Layer 1: Mastery-Aware Fog-of-War
  # ═══════════════════════════════════════════════════════════════════════════

  @mastery-unlock @wip
  Scenario: Prior mastery unlocks steps beyond the sequential window
    Given Matthew has "understand" mastery on the content for step 4
    And Matthew has completed steps 0 and 1
    When I view the "Elohim Protocol" path overview
    Then step 4 should be accessible with reason "Prior mastery"
    And step 4 should display a "mastery-unlocked" visual indicator
    But step 5 should remain in fog-of-war

  @mastery-unlock @sequential @wip
  Scenario: Sequential guidance still shown for mastery-unlocked steps
    Given Matthew has "understand" mastery on the content for step 4
    And Matthew has completed steps 0 and 1
    When I view the "Elohim Protocol" path overview
    Then the recommended next step should be step 2
    And step 4 should be marked as "available, not recommended next"

  @attestation-gate @hard-wall @wip
  Scenario: Attestation gates are not bypassed by mastery
    Given step 4 requires the "Basic Understanding" attestation
    And Matthew has "create" mastery on the content for step 4
    But Matthew does not have the "Basic Understanding" attestation
    When I check if step 4 is accessible
    Then step 4 should not be accessible
    And the reason should be "Requires attestation: Basic Understanding"

  @mastery-unlock @no-progress @wip
  Scenario: New learner with mastery on scattered content
    Given Matthew has no progress on "Elohim Protocol"
    But Matthew has "understand" mastery on content for steps 3 and 7
    When I view the path overview
    Then step 0 should be accessible as the start of the path
    And step 3 should be accessible with reason "Prior mastery"
    And step 7 should be accessible with reason "Prior mastery"
    But step 1 should not be accessible without prior mastery

  @mastery-threshold @wip
  Scenario: Bloom level "remember" does NOT unlock steps
    Given Matthew has "remember" mastery on the content for step 4
    And Matthew has completed steps 0 and 1
    When I check if step 4 is accessible
    Then step 4 should not be accessible
    And the reason should be "Complete previous steps first"

  # ═══════════════════════════════════════════════════════════════════════════
  # Layer 2: Skip-Ahead Integration
  # ═══════════════════════════════════════════════════════════════════════════

  @skip-ahead @wip
  Scenario: Pre-assessment skip-ahead unlocks section steps
    Given Matthew completed a pre-assessment for the "Elohim Protocol" path
    And the pre-assessment marked section "foundations" as skippable
    When I view the path overview
    Then all steps in section "foundations" should be accessible
    And the section should show a "Skipped via pre-assessment" indicator

  # ═══════════════════════════════════════════════════════════════════════════
  # Layer 3: Graph-Aware Recommendations
  # ═══════════════════════════════════════════════════════════════════════════

  @graph-recommendation @wip
  Scenario: Failed quiz surfaces prerequisite content from content graph
    Given Matthew is on step 4 of the "Elohim Protocol" path
    And the content for step 4 has a PREREQUISITE relationship to "foundations-of-trust"
    When Matthew fails the mastery quiz for the current section with score 30%
    Then a "Strengthen Your Foundations" section should appear
    And it should contain an EPR-linked card for "foundations-of-trust"
    And the card should show context "Foundation for concepts you need"
    And the recommendation should also appear in the path overview

  @graph-recommendation @dismiss @wip
  Scenario: Dismissing a recommendation removes it from both surfaces
    Given Matthew has an active recommendation for "foundations-of-trust"
    When Matthew dismisses the recommendation
    Then the recommendation should not appear in the quiz result
    And the recommendation should not appear in the path overview

  @graph-recommendation @gate-clear @wip
  Scenario: Passing the gate clears recommendations for that section
    Given Matthew has active recommendations from a failed quiz
    When Matthew passes the mastery quiz for the section
    Then all recommendations from that section should be cleared

  # ═══════════════════════════════════════════════════════════════════════════
  # Layer 4: Discovery-Informed Recommendations
  # ═══════════════════════════════════════════════════════════════════════════

  @discovery @recommendation
  Scenario: Elohim recommends a path after discovery completion
    Given Matthew has completed the "Values Hierarchy" discovery assessment
    When the elohim presence processes the discovery completion
    Then a banner insight should appear with a path recommendation
    And the "View Recommended Path" action should be available

  @discovery @navigation
  Scenario: Clicking "View Recommended Path" navigates to the path
    Given Matthew sees an elohim insight with a "View Recommended Path" action
    And the recommended path is "know-thyself"
    When Matthew clicks "View Recommended Path"
    Then Matthew should be navigated to the "know-thyself" path overview
