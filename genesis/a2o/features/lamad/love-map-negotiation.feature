@e2e @lamad @love-map @requires:household-nodes @act:i
# Matthew/Jessica dyad (household spouse pair) — household-class compute. The privacy scenario's
# third-party non-participant check uses James, the household's other member.
Feature: Love Map Path Negotiation
  As intimate partners (Matthew and Jessica),
  we want to negotiate and follow an emergent learning path together,
  so that we grow through mutual teaching and shared discovery.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ============================================================================
  # Consent & Attestation Gates
  # ============================================================================

  @consent-gate
  Scenario: Love map requires intimate consent level
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew and Jessica have a "connection" level consent relationship
    When Matthew tries to initiate a love map negotiation with Jessica
    Then a message about requiring "intimate" consent level should appear
    And the negotiation should not be created

  @attestation-gate
  Scenario: Love map requires mutual attestation
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew has attested "spouse" relationship to Jessica
    But Jessica has not attested "spouse" relationship to Matthew
    When Matthew tries to access the love map path "Garden of Understanding"
    Then a message about requiring mutual attestation should appear
    And the path content should not be visible

  # ============================================================================
  # Negotiation Flow
  # ============================================================================

  @negotiation
  Scenario: Matthew proposes a love map to Jessica
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew and Jessica have an "intimate" consent relationship
    And both have mutually attested "spouse" relationship
    When Matthew initiates a love map negotiation with Jessica
    And Matthew selects the "complementary" bridging strategy
    And Matthew sends proposal message "Let me show you how I see stewardship"
    Then the negotiation status should be "proposed"

  @negotiation
  Scenario: Jessica accepts and emergent path is generated
    Given human "Jessica" is logged in on doorway "alpha" with device
    And Matthew has proposed a love map negotiation
    When Jessica views the negotiation proposal
    And Jessica accepts with strategy "complementary"
    Then the negotiation status should be "accepted"
    And an emergent path "Garden of Understanding" should be generated
    And the path visibility should be "intimate"
    And both Matthew and Jessica should be listed as participants

  # ============================================================================
  # Path Navigation
  # ============================================================================

  @path-navigation @browser-only @wip
  Scenario: Matthew and Jessica can follow the love map path
    Given human "Matthew" is logged in on doorway "alpha" with device
    And the love map path "Garden of Understanding" exists
    When Matthew navigates to the path
    Then Chapter 1 "In the Garden Together" should be visible
    And the path should indicate it requires mutual attestation
    And the estimated duration should be "3-4 hours"

  @complementary @browser-only @wip
  Scenario: Path shows mutual teaching structure
    Given human "Matthew" is logged in on doorway "alpha" with device
    And the love map path "Garden of Understanding" exists
    When Matthew views the path chapters
    Then Chapter 2 should be "Tending and Naming" about stewardship
    And Chapter 3 should be "Seeking and Questioning" about truth-seeking
    And the chapters should show complementary teaching directions

  # ============================================================================
  # Privacy
  # ============================================================================

  @privacy @browser-only @wip
  Scenario: Love map path is invisible to non-participants
    Given human "James" is logged in on doorway "alpha" with device
    And James is a member of the same household as Matthew and Jessica, but not the couple
    When James browses all available learning paths
    Then the "Garden of Understanding" path should not appear
    And searching for "love map" should return no results for James

  # ============================================================================
  # Revocation
  # ============================================================================

  @revocation @wip
  Scenario: Revoking attestation removes love map access
    Given human "Jessica" is logged in on doorway "alpha" with device
    And the love map path "Garden of Understanding" exists and is accessible
    When Jessica revokes her "spouse" attestation to Matthew
    Then the path should become inaccessible to both Matthew and Jessica
    And the negotiation status should be updated to reflect the change
