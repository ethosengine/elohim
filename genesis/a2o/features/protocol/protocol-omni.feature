@e2e @protocol @protocol-omni
Feature: ProtocolOmniComponent makes protocol context legible at the top of the viewport
  As a visitor of any kind (anonymous cold hit or walked-from-protocol peer)
  I want a protocol chrome that announces the EPR I am viewing and lets me
  navigate the network's adjacency
  So that the substrate's context — content identity, resilience, in-network
  neighbors — is legible without leaving the page.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # @wip 2026-06-06: premature — no epr_atoms row for "elohim-host-landing" can exist
  # today. Every insert path (epr_service::ingest / ingest_with_cache / PUT /api/v1/epr
  # / P2P cold-fetch) enforces CID integrity (compute_cid == envelope.cid), so a
  # slug-keyed atom is unbuildable; slug-addressability of EPR atoms is an unmade
  # design decision (the omnibar plan is frontend-only, zero backend). Blocking work
  # tracked in genesis/data/timeline/backlog/seed-provenance-anchor-gap.md (landing
  # EPR seed path + slug-alias design question).
  @wip
  Scenario: The EPR nav-context endpoint serves a navigation projection
    When I GET "/api/v1/epr/elohim-host-landing/nav-context" from the doorway
    Then the doorway response status is 200
    And the response body has field "cid" equal to "elohim-host-landing"
    And the response body has field "partOf" which is an array
    And the response body has field "related" which is an array
    And the response body has field "derivedFrom" which is an array

  @browser-only
  Scenario: The protocol-omni chip appears on protocol-content routes
    When I open the landing page in a browser
    Then the element [data-testid="protocol-omni-chip"] is visible
    And the element [data-testid="protocol-omni-toolbar"] is not visible

  @browser-only
  Scenario: Clicking the chip expands the toolbar with EPR identifier
    When I open the landing page in a browser
    And I click the element [data-testid="protocol-omni-chip"]
    Then the element [data-testid="protocol-omni-toolbar"] is visible
    And the element [data-testid="protocol-omni-epr"] text contains "elohim-host-landing"

  @browser-only
  Scenario: The serving-context segment contextualizes the EPR on non-production environments
    When I open the landing page in a browser
    And I click the element [data-testid="protocol-omni-chip"]
    Then the element [data-testid="protocol-omni-env"] is visible
    And the element [data-testid="protocol-omni-env"] text contains "alpha"
