@epic:elohim_value_scanner_protocol @category:value-scanner @status:planned
Feature: Care Economy Token Recognition
  As a family using the Elohim Value Scanner
  I want care activities to be recognized and valued
  So that invisible labor becomes visible and rewarded

  Background:
    Given a family has an Elohim home hub installed
    And the family includes parents and children
    And care tokens are enabled for the household

  Scenario: Recognizing care activities
    Given Tommy makes breakfast for his sister Emma
    And he remembers she doesn't like crusts
    And he cleans up without being asked
    When the home hub observes this activity
    Then Tommy should earn care tokens
    And time tokens for saving parent time
    And growth tokens for developing skills
    And the family should receive a gentle notification

  Scenario: Shopping mission generation
    Given the family Elohim notices milk is running low
    And tonight's meal needs olive oil
    And Emma requested strawberries at breakfast
    When Tommy's shopping mission is generated
    Then it should include required items
    And Emma's wishes
    And discovery budget for healthy foods
    And family allergy warnings
    And no parent intervention was required

  Scenario: Value scanner shopping assistance
    Given Tommy is at the corner store on a shopping mission
    When he scans a container of strawberries
    Then his phone should recognize Emma's preference
    And award care bonus for thinking of his sister
    And provide nutritional information
    And show value in care tokens, not just dollars

  Scenario: Privacy-preserving observation
    Given the home hub camera observes family activities
    When it recognizes a care activity
    Then it should understand the pattern
    But never record or store video
    And never share observations outside the home
    And respect family privacy boundaries
    And function like a wise, caring grandparent

  Scenario: Multi-dimensional value tracking
    Given Tommy has earned various token types
    When the family reviews the care economy dashboard
    Then they should see care tokens for helping others
    And time tokens for efficiency and responsibility
    And growth tokens for learning and development
    And treat tokens for appropriate choices
    And all values represented, not just money
