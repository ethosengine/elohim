@e2e @content @hosted-human @requires:doorway @requires:seeded-humans
Feature: Content Lifecycle
  As a hosted human on a doorway
  I want to create, read, and discover content
  So that I can participate in the knowledge graph

  Matthew is the primary content creator. Susan and Terrance discover
  his content — validating that hosted humans share a common content
  space through the doorway.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  # --- Create ---

  Scenario: Create content
    When Matthew creates content titled "Governance Basics" with tags "governance,family-systems"
    Then the content should be created successfully
    And the content should have an id

  # --- Read ---

  Scenario: Read own content
    Given Matthew has created content titled "My Article" with tags "test"
    When Matthew reads the content by id
    Then the content title should be "My Article"

  # --- Discover ---

  Scenario: Discover content by tag
    Given Matthew has created content titled "Tagged Content" with tags "e2e-discovery,governance"
    When Matthew searches for content with tag "e2e-discovery"
    Then the search results should include "Tagged Content"

  # --- Cross-human discovery (same doorway) ---

  Scenario: Susan discovers Matthew's content
    Given Matthew has created content titled "Family Governance" with tags "e2e-shared,family-systems"
    And human "Susan" is logged in on doorway "alpha"
    When Susan searches for content with tag "e2e-shared"
    Then the search results should include "Family Governance"

  Scenario: Terrance discovers Matthew's content
    Given Matthew has created content titled "Learning Science" with tags "e2e-learning,education"
    And human "Terrance" is logged in on doorway "alpha"
    When Terrance searches for content with tag "e2e-learning"
    Then the search results should include "Learning Science"
