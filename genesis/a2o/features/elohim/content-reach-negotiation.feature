@e2e @elohim @reach-negotiation @wip
Feature: Content Reach Negotiation
  As a human creating content in the Elohim Protocol,
  I want the Elohim agent to negotiate the appropriate reach for my content
  before it leaves my device,
  so that distribution is governed by constitutional wisdom, not algorithmic amplification.

  "Free speech does not mean free reach.
   There is no right to algorithmic amplification."

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ============================================================================
  # Trust-Adaptive Pipeline Depth
  # ============================================================================

  @trust-adaptive
  Scenario: High-trust author gets lightweight pipeline
    Given human "Miriam" is logged in on doorway "alpha" with device
    And "Miriam" has a trust score of 0.87 with 47 contributions in the last 30 days
    When "Miriam" creates a content post requesting "community" reach
    Then the context assembly pipeline uses depth "light"
    And only layers "constitutional, author-profile, payload" are assembled
    And the assembly completes in under 100 milliseconds

  @trust-adaptive
  Scenario: New author gets full pipeline
    Given human "Ezra" is logged in on doorway "alpha" with device
    And "Ezra" has a trust score of 0.0 with 0 contributions in the last 30 days
    When "Ezra" creates a content post requesting "community" reach
    Then the context assembly pipeline uses depth "deep"
    And all six context layers are assembled
    And the safety scan level is "deep"
    And the graph neighborhood uses 2-hop traversal

  @trust-adaptive
  Scenario: Moderate-trust author gets standard pipeline
    Given human "Levi" is logged in on doorway "alpha" with device
    And "Levi" has a trust score of 0.55 with 8 contributions in the last 30 days
    When "Levi" creates a content post requesting "municipal" reach
    Then the context assembly pipeline uses depth "standard"
    And layers "constitutional, author-profile, graph-neighborhood, payload" are assembled
    And the safety scan level is "standard"

  # ============================================================================
  # Reach Negotiation
  # ============================================================================

  @negotiation
  Scenario: Trusted author receives recommended reach matching request
    Given human "Miriam" is logged in on doorway "alpha" with device
    And "Miriam" has a trust score of 0.87 with 47 contributions in the last 30 days
    When "Miriam" creates a content post requesting "community" reach
    Then the Elohim recommends "community" reach
    And the safety review is cleared
    And the transparency report shows the decision factors
    And "Miriam" sees the reach recommendation before the content leaves her device

  @negotiation
  Scenario: Untrusted author has reach reduced with explanation
    Given human "Ezra" is logged in on doorway "alpha" with device
    And "Ezra" has a trust score of 0.2 with 1 contribution in the last 30 days
    When "Ezra" creates a content post requesting "municipal" reach
    Then the Elohim recommends "local" reach
    And a reach delta explains the reduction from "municipal" to "local"
    And the transparency report includes "Author trust score" as a decision factor
    And an attestation path is suggested to earn "municipal" reach

  @negotiation
  Scenario: Author accepts reduced reach
    Given human "Ezra" is logged in on doorway "alpha" with device
    And the Elohim has recommended "local" reach for Ezra's post
    When "Ezra" accepts the recommended reach
    Then the content is committed to the local source chain with "local" reach
    And a birth context is embedded in the content node

  @negotiation
  Scenario: Author re-negotiates reach with explanation
    Given human "Levi" is logged in on doorway "alpha" with device
    And the Elohim has recommended "local" reach for Levi's post
    When "Levi" requests re-negotiation with reason "This is relevant to the whole municipality"
    Then the Elohim re-evaluates with the additional context
    And either upgrades the recommendation or maintains it with reasoning

  # ============================================================================
  # Birth Context (Immutable Provenance)
  # ============================================================================

  @birth-context
  Scenario: Content carries birth context after creation
    Given human "Miriam" is logged in on doorway "alpha" with device
    And "Miriam" creates a post with negotiated "community" reach
    When the content is committed to the source chain
    Then the content node includes a birth context
    And the birth context contains the constitutional stack hash
    And the birth context contains the author's trust snapshot
    And the birth context contains the negotiated reach level

  @birth-context @privacy-gradient
  Scenario: Private content gets minimal birth context
    Given human "Levi" is logged in on doorway "alpha" with device
    And "Levi" creates a post with negotiated "private" reach
    When the content is committed to the source chain
    Then the birth context contains only "createdAt" and "constitutionalStackHash"
    And no community mood or governance markers are included

  @birth-context @privacy-gradient
  Scenario: Commons content gets full birth context
    Given human "Miriam" is logged in on doorway "alpha" with device
    And "Miriam" creates a post with negotiated "commons" reach
    When the content is committed to the source chain
    Then the birth context includes community mood
    And the birth context includes governance markers
    And the birth context includes neighborhood summary
    And the birth context includes author intent

  @birth-context
  Scenario: Reply carries chain of provenance
    Given human "Levi" is logged in on doorway "alpha" with device
    And content "post-123" exists with a birth context
    When "Levi" creates a comment on "post-123"
    Then the comment's birth context includes "parentBirthContextHash"
    And the hash references "post-123"'s birth context

  # ============================================================================
  # Transparency
  # ============================================================================

  @transparency
  Scenario: User can inspect full transparency report
    Given human "Miriam" is logged in on doorway "alpha" with device
    When "Miriam" creates a content post and receives a reach recommendation
    Then the transparency report lists all context layers considered
    And each decision factor shows influence, weight, and evidence
    And the report shows author standing summary
    And the report shows community health assessment

  @transparency
  Scenario: Content without birth context is flagged
    Given content "stripped-post" exists without a birth context
    When human "Levi" views "stripped-post" on doorway "alpha"
    Then a provenance warning is displayed
    And the trust score for "stripped-post" is reduced

  # ============================================================================
  # Anti-Weaponization
  # ============================================================================

  @anti-weaponization
  Scenario: Recontextualization is detectable
    Given content "original-post" was created during community mood "heated"
    And "original-post" has a birth context with governance marker "during municipal election"
    When "original-post" is shared in a different context
    Then viewers can compare the birth context to the new framing
    And the birth context's immutable governance markers reveal the original context

  @anti-weaponization
  Scenario: Trust earned not purchased
    Given human "Ezra" is logged in on doorway "alpha" with device
    And "Ezra" has a trust score of 0.2
    And "Ezra" has unlimited compute budget via BYOK
    When "Ezra" creates a content post requesting "commons" reach
    Then the pipeline depth is still "deep" regardless of compute budget
    And the reach recommendation is based on trust score not resources
    And the BYOK budget does not influence the reach decision
