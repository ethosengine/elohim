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
    And human "Timothy" is logged in on doorway "alpha"

  # --- Landing Page EPR Link ---

  @wip
  Scenario: EPR link on landing page resolves to manifesto content
    Given Timothy is on the landing page
    When Timothy clicks the "Elohim Protocol Manifesto" EPR card
    Then Timothy sees the manifesto content rendered as markdown
    And the page URL contains "/lamad/resource/manifesto"

  # --- Context-Aware Resolution ---

  @wip @browser-only
  Scenario: EPR link in markdown resolves within current path
    Given Timothy is on step 2 of path "protocol-foundations"
    And step 4 of path "protocol-foundations" contains resource "rea-foundations"
    When Timothy clicks an "epr:rea-foundations" link in the markdown content
    Then Timothy navigates to step 4 of path "protocol-foundations"
    And the resolution type is "in-path"

  @wip @browser-only
  Scenario: EPR link resolves as standalone when not in a path
    Given Timothy is viewing resource "manifesto-foundations" directly
    When Timothy clicks an "epr:rea-foundations" link in the markdown content
    Then Timothy navigates to the standalone resource view for "rea-foundations"
    And the resolution type is "standalone"

  # --- CID Content Addressing ---

  @wip
  Scenario: Blob content loads via CID
    Given content "manifesto-foundations" has a blob reference
    When the content renderer fetches the blob
    Then the blob content is returned as decoded text
    And the content renders as markdown

  # --- Three-Pillar Metadata ---

  @wip
  Scenario: EPR Head carries three-pillar metadata
    Given content "manifesto-foundations" exists in storage
    When Timothy requests the EPR Head for "manifesto-foundations"
    Then the EPR Head contains lamad context with title and content type
    And the EPR Head contains qahal context with reach level
    And the EPR Head response uses DAG-CBOR content type

  # --- EPR Popover ---

  @wip @browser-only
  Scenario: EPR link shows three-pillar popover on hover
    Given Timothy is viewing a page with an EPR link to "manifesto-foundations"
    When Timothy hovers over the EPR link
    Then a popover appears showing the content title
    And the popover shows the content type badge
    And the popover shows the reach level
