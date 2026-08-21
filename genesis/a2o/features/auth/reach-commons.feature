@e2e @reach @act:i
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
    # "commons". Account-packages assign it "community" (rank 6) but the
    # seeder's raise-only logic must honour the authored "commons" (rank 8) as
    # the floor — a higher rank is a broader audience, so the account-package
    # assignment can only ever raise. Any regression here returns HTTP 403.
    #
    # Measured red, genesis #1489 (2026-08-20): 403. The seeder computes the
    # authored floor correctly — the row on the fleet is stale. Content rows are
    # bulk-INSERTed and never UPDATEd, so an existing row keeps whatever reach it
    # was first seeded with; the re-seed's reach-reconcile PATCH is the only heal
    # and it circuit-breaks on every batch, because a reach-carrying PATCH is
    # routed through conductor re-notarization that bulk-seeded (never
    # DHT-authored) rows cannot satisfy. So the authored grade is real and the
    # STORED grade is stale — a grading/reconcile gap on the reach plane, not a
    # regression in the reach GATE.
    When an anonymous client GETs content "manifesto"
    Then the content is served to the anonymous reader

  # ============================================================================
  # Outline — other canonical commons/public surfaces
  # ============================================================================

  Scenario Outline: Anonymous reader can read <label> content (commons/public reach)
    When an anonymous client GETs content "<contentId>"
    Then the content is served to the anonymous reader

    Examples:
      | contentId          | label                    |
      | elohim-protocol    | elohim-protocol path     |

  # ============================================================================
  # Negative — community-reach content requires authentication
  # ============================================================================

  Scenario: Anonymous reader is rejected for community-reach content (403 with requiredReach)
    # Community reach requires a session. The 403 body must declare the required
    # reach so the client can present the correct onboarding prompt.
    #
    # The fixture must be a row whose community grade the SEED guarantees, so
    # that every seeded peer reproduces the gate: "community-garden-club" is
    # authored "reach": "community" in genesis/data/lamad/content/. A fixture
    # picked instead from an observed 403 tests whatever grade that one fleet
    # happens to be carrying — which is how the previous fixture, an epic
    # authored "commons", stood here as the community case.
    When an anonymous client GETs content "community-garden-club"
    Then the response status is 403
    And the 403 body requiredReach is "community"
