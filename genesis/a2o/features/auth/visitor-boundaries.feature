@e2e @auth @visitor @reach @requires:doorway @wip
Feature: Visitor Boundaries
  As Traveler, an anonymous visitor to the network,
  I want to browse commons content without registering
  So that I can explore the network before committing to an identity.

  Visitors are the entry point of the graduated stewardship journey.
  They have no identity, no keys, no session — only what the doorway
  serves at commons reach. Every boundary tested here is a gate that
  the onboarding funnel must eventually open as the visitor graduates.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ============================================================================
  # Registration Boundary
  # ============================================================================

  @regression
  Scenario: Registering as "visitor" phase is rejected — must choose hosted or higher
    When an unauthenticated client sends POST "/auth/register" with:
      | field        | value                    |
      | identifier   | traveler@test.elohim.host |
      | password     | Test2026!                |
      | displayName  | Traveler                 |
      | agencyPhase  | visitor                  |
    Then the response status should be 400
    And the response body should include code "VISITOR_NO_REGISTER"
    # Constraint: the "visitor" phase means anonymous browsing — no identity.
    # To get an identity, the human must choose at least "hosted" as their
    # agency phase. This enforces the graduated stewardship boundary:
    # visitor → hosted → device → node.

  # ============================================================================
  # Content Reach Boundaries
  # ============================================================================

  Scenario: Visitor can access commons content
    When an unauthenticated client requests commons content from doorway "alpha"
    Then the response should succeed
    And the content should be served
    # Commons content is the network's public face — always accessible.

  Scenario: Visitor cannot access network-reach content
    When an unauthenticated client requests network-reach content from doorway "alpha"
    Then the response status should be 401 or 403
    # Network-reach requires authentication — the visitor must register first.

  Scenario: Visitor cannot access private content
    When an unauthenticated client requests private content from doorway "alpha"
    Then the response status should be 401 or 403
    # Private content requires both authentication and a specific relationship.

  # ============================================================================
  # Session Boundaries
  # ============================================================================

  Scenario: Visitor has no JWT token
    When an unauthenticated client sends GET "/auth/me" on doorway "alpha"
    Then the response status should be 401
    # Visitors have no session. /auth/me requires a valid JWT.

  Scenario: Visitor can check doorway health
    When an unauthenticated client sends GET "/health" on doorway "alpha"
    Then the response should succeed
    And the response should indicate the doorway is healthy
    # Health endpoint is always public — visitors and operators both need it.

  # ============================================================================
  # Onboarding Funnel
  # ============================================================================

  Scenario: Visitor can see the landing page
    When an unauthenticated client requests the landing page on doorway "alpha"
    Then the response should succeed
    And the page should present an onboarding path
    # The landing page is the visitor's first impression. It should invite
    # them to explore commons content and eventually register.

  Scenario: Visitor who registers as hosted gets an identity
    When an unauthenticated client sends POST "/auth/register" with:
      | field        | value                    |
      | identifier   | new-visitor@test.elohim.host |
      | password     | Test2026!                |
      | displayName  | New Visitor              |
      | agencyPhase  | hosted                   |
    Then the response status should be 201
    And the response body should include a token
    And the response body should include a humanId
    # This is the graduation from visitor to hosted — the first gate of agency.
    # The doorway creates an identity on the operator's conductor (custodial).
