@e2e @content @epr @requires:doorway @requires:seeded-content
Feature: EPR Content Addressing
  As a learner navigating the protocol
  I want content links to resolve contextually
  So that I experience knowledge as interconnected rather than isolated

  EPR (Elohim Protocol Reference) links are the protocol's native
  content addressing system. They carry three-pillar context
  (lamad/shefa/qahal) and resolve contextually based on where the
  learner is in their journey.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Timothy" is logged in on doorway "alpha" with device

  # --- Context-Aware Resolution ---

  @wip @browser-only
  Scenario: EPR link in markdown resolves as cross-path
    Given Timothy is on the manifesto step of path "elohim-protocol"
    And the manifesto content contains an "epr:rea-foundations" link
    When Timothy clicks the "epr:rea-foundations" link in the markdown content
    Then Timothy navigates to "rea-foundations" in path "hrea-care-economy"
    And the resolution type is "cross-path"

  @wip @browser-only
  Scenario: EPR link resolves as standalone when not in a path
    Given Timothy is viewing resource "manifesto" directly
    When Timothy clicks the "epr:rea-foundations" link in the markdown content
    Then Timothy navigates to the standalone resource view for "rea-foundations"
    And the resolution type is "standalone"

  # --- CID Content Addressing ---

  @wip
  Scenario: Blob content loads via CID
    Given content "manifesto" has a blob reference
    When the content renderer fetches the blob
    Then the blob content is returned as decoded text
    And the content renders as markdown

  # --- Three-Pillar Metadata ---

  @wip
  Scenario: EPR Head carries three-pillar metadata
    Given content "manifesto" exists in storage
    When Timothy requests the EPR Head for "manifesto"
    Then the EPR Head contains lamad context with title and content type
    And the EPR Head contains qahal context with reach level
    And the EPR Head response uses DAG-CBOR content type

  # --- EPR Popover ---

  @wip @browser-only
  Scenario: EPR link shows three-pillar popover on hover
    Given Timothy is viewing a page with an EPR link to "rea-foundations"
    When Timothy hovers over the EPR link
    Then a popover appears showing the content title
    And the popover shows the content type badge
    And the popover shows the reach level
