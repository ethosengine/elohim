@e2e @reach
Feature: Commons and public reach content is accessible to anonymous visitors
  Content that holds an authored commons reach grade must be readable by any
  anonymous client — no session, no token, no relationship required. This is
  the "earned reach" principle: the seeder grades content at authoring time,
  and the storage gate trusts that grade. An account-package relationship
  assignment at a lower reach level (e.g. community) must never drag the
  authored grade down.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # ============================================================================
  # Manifesto — the regression that triggered this feature
  # ============================================================================

  @regression
  Scenario: Anonymous reader can read the manifesto (earned commons reach)
    # The manifesto is THE public protocol document. Its authored reach is
    # "commons". Account-packages assign it "community" (rank 5) but the
    # seeder's raise-only logic must honour the authored "commons" (rank 7)
    # as the floor. Any regression here returns HTTP 403.
    When an anonymous client GETs content "manifesto"
    Then the response status is 200

  # ============================================================================
  # Outline — other canonical commons/public surfaces
  # ============================================================================

  Scenario Outline: Anonymous reader can read <label> content (commons/public reach)
    When an anonymous client GETs content "<contentId>"
    Then the response status is 200

    Examples:
      | contentId          | label                    |
      | elohim-protocol    | elohim-protocol path     |

  # ============================================================================
  # Negative — community-reach content requires authentication
  # ============================================================================

  Scenario: Anonymous reader is rejected for community-reach content (403 with requiredReach)
    # Community reach requires a session. The 403 body must declare the required
    # reach so the client can present the correct onboarding prompt.
    # autonomous-entity-epic is community-reach (live-verified 403); rea-foundations
    # is public (200) and would not exercise the negative gate.
    When an anonymous client GETs content "autonomous-entity-epic"
    Then the response status is 403
    And the 403 body requiredReach is "community"
