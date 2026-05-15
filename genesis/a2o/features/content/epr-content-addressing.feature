@e2e @content @epr @requires:doorway @requires:seeded-content
Feature: EPR Content Addressing
  As a learner navigating the protocol
  I want content links to resolve contextually
  So that I experience knowledge as interconnected rather than isolated

  EPR (Elohim Protocol Reference) links are the protocol's native
  content addressing. Each link is a verified address — content-derived,
  signature-anchored, three-pillar-aware. They carry knowledge (lamad),
  value (shefa), and governance (qahal) context together, and resolve
  based on where the learner is in their journey.

  # Why this matters:
  # The web's URL was a pointer to a place. EPR's reference is a pointer
  # to a *meaning* — what the content is, who steward it, what reach it
  # carries, whether the reader has standing to receive it. When a learner
  # follows a link, they don't navigate to a server; they enter a
  # relationship with what's been authored. Resolution is contextual
  # because authorship is relational.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Terrance" is logged in on doorway "alpha" with device

  # --- Context-Aware Resolution ---

  @browser-only
  Scenario: EPR link in markdown resolves as cross-path
    # Terrance is reading the manifesto. A reference to rea-foundations
    # appears inline — not as a footnote, but as a living link into
    # another curriculum that develops the same idea further. Following
    # it should land him in the destination path, not lose his bearings.
    Given Terrance is on the manifesto step of path "elohim-protocol"
    And the manifesto content contains an "epr:rea-foundations" link
    When Terrance clicks the "epr:rea-foundations" link in the markdown content
    Then Terrance navigates to "rea-foundations" in path "hrea-care-economy"
    And the resolution type is "cross-path"

  @browser-only
  Scenario: EPR link resolves as standalone when not in a path
    # When Terrance reads a resource directly (no enclosing curriculum),
    # the same EPR reference resolves into a standalone view — the
    # destination's own self-contained presentation, not a path-step.
    # Same address, different context, different render.
    Given Terrance is viewing resource "manifesto" directly
    When Terrance clicks the "epr:rea-foundations" link in the markdown content
    Then Terrance navigates to the standalone resource view for "rea-foundations"
    And the resolution type is "standalone"

  # --- CID Content Addressing ---

  Scenario: Blob content loads via CID
    # The body of every notarized piece is content-addressed (CID).
    # The renderer fetches by hash, not by location — so the same bytes
    # can come from any peer that holds them, and the reader can verify
    # what they got matches what was authored.
    Given content "manifesto" has a blob reference
    When the content renderer fetches the blob
    Then the blob content is returned as decoded text
    And the content renders as markdown

  # --- Three-Pillar Metadata ---

  Scenario: EPR Head carries three-pillar metadata
    # The EPR Head is the verifiable preview — what the content is
    # (lamad), what stewardship governs it (qahal), and (when applicable)
    # what value flows it carries (shefa). DAG-CBOR encoding makes the
    # head deterministic-on-the-wire so any peer can verify it.
    Given content "manifesto" exists in storage
    When Terrance requests the EPR Head for "manifesto"
    Then the EPR Head contains lamad context with title and content type
    And the EPR Head contains qahal context with reach level
    And the EPR Head response uses DAG-CBOR content type

  # --- EPR Popover ---

  @browser-only
  Scenario: EPR link shows three-pillar popover on hover
    # Before clicking, Terrance can see what the link points to. A hover
    # popover surfaces the verified preview from the EPR Head — so he
    # makes an informed decision about whether to follow. No "trust me,
    # click this" — every link is its own little disclosure.
    Given Terrance is viewing a page with an EPR link to "rea-foundations"
    When Terrance hovers over the EPR link
    Then a popover appears showing the content title
    And the popover shows the content type badge
    And the popover shows the reach level

  # --- New Wave 3 scenarios — dramatize the human moments the surface enables ---

  # @wip retained: shefa-context population on content fixtures + popover shefa/lamad/qahal
  # phrasing step-defs not yet implemented. The popover surface itself works (covered by
  # scenario 5 above); this scenario asserts the three-pillar fan-out which requires
  # shefa-context-populated fixtures.
  @wip @browser-only
  Scenario: EPR popover surfaces all three pillars when present
    # When content carries shefa context (a stewardship commitment, an
    # economic flow), the popover shows it alongside the lamad/qahal
    # info. Terrance sees not just "what is this" but "who carries it
    # forward and what does it cost them" — full disclosure at the link.
    Given content "rea-foundations" has lamad, qahal, AND shefa context populated
    And Terrance is viewing a page with an EPR link to "rea-foundations"
    When Terrance hovers over the EPR link
    Then the popover shows the lamad title and content type
    And the popover shows the qahal reach level
    And the popover shows the shefa stewardship summary

  # @wip retained: origin-context-aware destination rendering + back-affordance UI not yet
  # built in elohim-app. The cross-path navigation itself works (covered by scenario 1
  # above); this scenario asserts the destination tailors its render based on origin —
  # a substantial UX surface still in design.
  @wip @browser-only
  Scenario: Following an EPR link transfers reading context to the destination
    # The renderer at the destination should know Terrance arrived FROM
    # the manifesto, not from a search or a bookmark. Context-transfer
    # lets the destination tailor its render — a "you came from here,
    # so let's connect to that thread" affordance, not a cold landing.
    Given Terrance is on the manifesto step of path "elohim-protocol"
    And the manifesto content contains an "epr:rea-foundations" link
    When Terrance clicks the "epr:rea-foundations" link in the markdown content
    Then the destination "rea-foundations" view renders with origin context "manifesto"
    And the destination shows a back-affordance to the originating manifesto step

  # @wip retained: supersedence model + "historical version" UI affordance not yet built.
  # The EPR codec supports supersedence references but the renderer does not yet display
  # "this version was superseded" notices. Requires both content-side supersedence wiring
  # and a UI affordance pass.
  @wip @browser-only
  Scenario: EPR link to a versioned-since-authored CID degrades gracefully
    # An EPR reference points to a specific CID. If the content has been
    # superseded by a newer version, the link still resolves — but the
    # reader is told the version they reached is historical, with a path
    # to the current canonical version if they want to follow it.
    Given the manifesto contains an "epr:rea-foundations" link to a CID that has been superseded
    When Terrance clicks the historical EPR link
    Then the historical content body renders with a "this version was superseded" notice
    And the notice offers to navigate to the current canonical version
    And both versions remain content-addressable and verifiable

  # @wip retained: DAG-CBOR decode + Ed25519 signature verification test infrastructure
  # not yet wired in a2o steps. The existing EPR Head steps (scenario 4 above) assert
  # response-shape only — verifying the signature end-to-end requires importing a CBOR
  # decoder + signature-verify library into the step-defs.
  @wip
  Scenario: EPR Head signature is verifiable end-to-end
    # The Head isn't just metadata — it's a signed claim by a known
    # author that "this content, with this reach, with these couplings,
    # is mine." Any peer receiving the Head can verify it independently.
    # No central authority; the signature is the authority.
    Given content "manifesto" exists in storage with author agent "agent-author"
    When Terrance requests the EPR Head for "manifesto"
    Then the EPR Head includes an Ed25519 signature by "agent-author"
    And the signature verifies against the canonical envelope bytes
    And the CID computed from the canonical envelope matches the requested CID
