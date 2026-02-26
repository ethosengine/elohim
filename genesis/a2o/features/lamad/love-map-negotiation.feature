@e2e @lamad @love-map
Feature: Love Map Path Negotiation
  As intimate partners (Adam and Eve),
  we want to negotiate and follow an emergent learning path together,
  so that we grow through mutual teaching and shared discovery.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ============================================================================
  # Consent & Attestation Gates
  # ============================================================================

  @consent-gate
  Scenario: Love map requires intimate consent level
    Given human "Adam" is logged in on doorway "alpha" with device
    And Adam and Eve have a "connection" level consent relationship
    When Adam tries to initiate a love map negotiation with Eve
    Then a message about requiring "intimate" consent level should appear
    And the negotiation should not be created

  @attestation-gate
  Scenario: Love map requires mutual attestation
    Given human "Adam" is logged in on doorway "alpha" with device
    And Adam has attested "spouse" relationship to Eve
    But Eve has not attested "spouse" relationship to Adam
    When Adam tries to access the love map path "Garden of Understanding"
    Then a message about requiring mutual attestation should appear
    And the path content should not be visible

  # ============================================================================
  # Negotiation Flow
  # ============================================================================

  @negotiation
  Scenario: Adam proposes a love map to Eve
    Given human "Adam" is logged in on doorway "alpha" with device
    And Adam and Eve have an "intimate" consent relationship
    And both have mutually attested "spouse" relationship
    When Adam initiates a love map negotiation with Eve
    And Adam selects the "complementary" bridging strategy
    And Adam sends proposal message "Let me show you how I see stewardship"
    Then the negotiation status should be "proposed"

  @negotiation
  Scenario: Eve accepts and emergent path is generated
    Given human "Eve" is logged in on doorway "alpha" with device
    And Adam has proposed a love map negotiation
    When Eve views the negotiation proposal
    And Eve accepts with strategy "complementary"
    Then the negotiation status should be "accepted"
    And an emergent path "Garden of Understanding" should be generated
    And the path visibility should be "intimate"
    And both Adam and Eve should be listed as participants

  # ============================================================================
  # Path Navigation
  # ============================================================================

  @path-navigation @browser-only @wip
  Scenario: Adam and Eve can follow the love map path
    Given human "Adam" is logged in on doorway "alpha" with device
    And the love map path "Garden of Understanding" exists
    When Adam navigates to the path
    Then Chapter 1 "In the Garden Together" should be visible
    And the path should indicate it requires mutual attestation
    And the estimated duration should be "3-4 hours"

  @complementary @browser-only @wip
  Scenario: Path shows mutual teaching structure
    Given human "Adam" is logged in on doorway "alpha" with device
    And the love map path "Garden of Understanding" exists
    When Adam views the path chapters
    Then Chapter 2 should be "Tending and Naming" about stewardship
    And Chapter 3 should be "Seeking and Questioning" about truth-seeking
    And the chapters should show complementary teaching directions

  # ============================================================================
  # Privacy
  # ============================================================================

  @privacy @browser-only @wip
  Scenario: Love map path is invisible to non-participants
    Given human "Matthew" is logged in on doorway "alpha" with device
    When Matthew browses all available learning paths
    Then the "Garden of Understanding" path should not appear
    And searching for "love map" should return no results for Matthew

  # ============================================================================
  # Revocation
  # ============================================================================

  @revocation @wip
  Scenario: Revoking attestation removes love map access
    Given human "Eve" is logged in on doorway "alpha" with device
    And the love map path "Garden of Understanding" exists and is accessible
    When Eve revokes her "spouse" attestation to Adam
    Then the path should become inaccessible to both Adam and Eve
    And the negotiation status should be updated to reflect the change
