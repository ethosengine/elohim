@e2e @resilience @resilience-p1 @local
Feature: Observable + contract-aware auto-distribute
  As an operator running a household mesh
  I want ingested content to land on diverse households within contract bounds
  So I can trust the dashboards and plan recruitment when coverage is short

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  # --- Full placement across two households --------------------------------

  @resilience-p1
  Scenario: Full placement across two households
    Given the cluster has peers in at least 2 distinct households each with an active "commons" provide commitment
    When I ingest a "commons"-reach content item "content-alpha"
    Then within 30 seconds "/api/v1/resilience/content-alpha/household" reports "stewardingCollectives" >= 2
    And the response field "placementGaps" is empty
    And the response field "protectionStatus" is "protected" or "partial"

  # --- Placement gap on short commitments ----------------------------------

  @resilience-p1
  Scenario: Placement gap when commitments are short
    Given the cluster has peers in 2 households but only 1 has an active "commons" provide commitment
    When I ingest a "commons"-reach content item "content-beta"
    Then within 30 seconds "/api/v1/placement-gaps?contentId=content-beta" returns at least one row
    And the row has "gapKind" matching "contracts-short" or "under-committed"

  # --- Content-viewer tooltip ----------------------------------------------

  @wip @resilience-p1
  Scenario: Content-viewer resilience tooltip is live
    Given "content-alpha" has been distributed to at least 2 households
    When I open the content-viewer for "content-alpha"
    Then the resilience icon has class "status-protected" or "status-partial"
    And the tooltip mentions the household count

  # --- Shefa signals card --------------------------------------------------

  @wip @resilience-p1
  Scenario: Shefa signals card reflects current placement gaps
    Given at least one placement gap exists in "/api/v1/placement-gaps"
    When I open "/shefa/dashboard"
    Then the signals card shows a non-zero gap count
    And clicking a gap signal scrolls to or links to a shefa recruitment surface

  # --- Doorway admin snapshot icons ----------------------------------------

  @wip @resilience-p1
  Scenario: Doorway admin content list shows resilience snapshot icons
    Given "content-alpha" is in the admin content list on doorway "alpha"
    When I open the admin content list
    Then each row renders an elohim-resilience-snapshot icon
    And hovering a row shows the household summary
