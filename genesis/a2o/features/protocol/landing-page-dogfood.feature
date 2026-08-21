@e2e @protocol @landing-dogfood @act:i
Feature: Elohim Protocol landing page is dogfooded as protocol content
  As Matthew, who stewards the Elohim Protocol public surface and operates
  the alpha doorway as the same agent
  I want the landing page at alpha.elohim.host to render bytes that flow
  through the protocol's content-addressing path
  So that the protocol's own marketing site is the first proof of the
  steward↔doorway hosting model — and is impossible to silently centralize.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: The elohim-host-landing ContentNode exists with html5-app format
    When I fetch the ContentNode "elohim-host-landing"
    Then the contentFormat is "html5-app"
    And the content.slug is "elohim-host-landing"
    And the content.entryPoint is "index.html"
    And the blobHash is a sha256 hex string

  Scenario: The landing bundle serves at the /apps path
    When I GET "/apps/elohim-host-landing/index.html" from the doorway
    Then the doorway response status is 200
    And the doorway response Content-Type contains "text/html"

  Scenario: An in-kind REA Commitment declares Matthew's hosting agreement
    When I list active REA commitments where provider is "matthew"
    Then at least one commitment has inScopeOf containing "host:alpha.elohim.host"
    And that commitment has inScopeOf containing "epr_root:elohim-host-landing"
    And that commitment's metadata signalKind is "compute-allocation"
    And that commitment's metadata triggerKind is "subscription"

  @browser-only
  # @wip 2026-06-04 shakeout: ProtocolSignalBadgeComponent was deliberately deleted
  # 2026-05-20 (980ea505d — replaced by shell-mounted ProtocolOmniComponent). The
  # standalone landing SPA may now carry NO protocol affordance — a product decision
  # the dogfood scenario correctly surfaced. Backlog:
  # genesis/data/timeline/backlog/landing-protocol-affordance-gap.md
  # HELD (2026-08-21): the badge component was deliberately deleted in 980ea505d; this
  # scenario asserts a surface that no longer exists (backlog landing-protocol-affordance-gap).
  @wip
  Scenario: The protocol-signal badge renders on the landing page
    When I open the landing page in a browser
    Then the element [data-testid="protocol-signal-badge-pill"] is visible
    When I click the protocol-signal badge
    Then the element [data-testid="protocol-signal-panel"] is visible
    And the panel text contains "elohim-host-landing"
