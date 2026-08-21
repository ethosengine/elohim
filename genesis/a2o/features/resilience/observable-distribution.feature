# @local is scenario-level here, never feature-level: only the two auto-distribute
# scenarios ingest into the mesh they measure, and a blanket header tag would
# withdraw the whole file from both CI gates. Same discipline as
# features/resilience/grandma-photos-survive-node-loss.feature.
@e2e @resilience @resilience-p1 @concern:blob-durability @dataplane
Feature: Observable + contract-aware auto-distribute
  As an operator running a household mesh
  I want ingested content to land on diverse households within contract bounds
  So I can trust the dashboards and plan recruitment when coverage is short

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  # --- Full placement across two households --------------------------------

  # @local: this scenario INGESTS a new content item and then requires the live
  # mesh to place it across >=2 households within 30s. That write-plus-placement
  # loop is what the local-stack profile exists for; against a deployed shared
  # fleet it would author test content into the substrate it is measuring.
  @resilience-p1 @local @act:i
  Scenario: Full placement across two households
    Given the cluster has peers in at least 2 distinct households each with an active "commons" provide commitment
    When I ingest a "commons"-reach content item "content-alpha"
    Then within 30 seconds "/api/v1/resilience/content-alpha/household" reports "stewardingCollectives" >= 2
    And the response field "placementGaps" is empty
    And the response field "protectionStatus" is "protected" or "partial"

  # --- Placement gap on short commitments ----------------------------------

  # @local: same ingest-then-place loop as the scenario above.
  @resilience-p1 @local @act:i
  Scenario: Placement gap when commitments are short
    Given the cluster has peers in 2 households but only 1 has an active "commons" provide commitment
    When I ingest a "commons"-reach content item "content-beta"
    Then within 30 seconds "/api/v1/placement-gaps?contentId=content-beta" returns at least one row
    And the row has "gapKind" matching "contracts-short" or "under-committed"

  # --- Content-viewer tooltip ----------------------------------------------

  # @wip: openContentViewerStub() in steps/resilience.steps.ts returns 'pending'
  # unconditionally, so both Then assertions are unreachable and this scenario measures
  # nothing. Sheds @wip when the step drives a real content-viewer — it is a browser
  # scenario, so it wants @browser-only then.
  @resilience-p1 @wip @act:i
  Scenario: Content-viewer resilience tooltip is live
    Given "content-alpha" has been distributed to at least 2 households
    When I open the content-viewer for "content-alpha"
    Then the resilience icon has class "status-protected" or "status-partial"
    And the tooltip mentions the household count

  # --- Shefa signals card --------------------------------------------------

  @wip @resilience-p1 @act:i
  Scenario: Shefa signals card reflects current placement gaps
    Given at least one placement gap exists in "/api/v1/placement-gaps"
    When I open "/shefa/dashboard"
    Then the signals card shows a non-zero gap count
    And clicking a gap signal scrolls to or links to a shefa recruitment surface

  # --- Doorway admin snapshot icons ----------------------------------------

  @wip @resilience-p1 @act:i
  Scenario: Doorway admin content list shows resilience snapshot icons
    Given "content-alpha" is in the admin content list on doorway "alpha"
    When I open the admin content list
    Then each row renders an elohim-resilience-snapshot icon
    And hovering a row shows the household summary

  # --- Topology pages: my-cluster, peer-topology, reciprocity --------------
  # Light-up-the-topology sprint (Phase 7-9). Operator-visible topology surfaces
  # built from substrate views (/api/v1/cluster, /peer-topology, /reciprocity).
  # Household is the resilience unit — peer-topology aggregates per-household,
  # not per-peer. Reciprocity is a stewardship flow (inflow/outflow/net), not
  # a moral score.

  @browser-only @resilience-p1 @act:i
  Scenario: Operator can see their household device cluster
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew's household has 2 devices joined to a single steward
    And device "matthew-laptop" has archetype "desktop" and is online
    And device "matthew-node" has archetype "node" and is online
    When Matthew opens "/shefa/cluster"
    Then the cluster page renders
    And the cluster summary is visible
    And the page lists at least one device tile
    # Iter-1 visual proof: the page must render without crashing. Tile-count
    # and archetype-label exact-text assertions tighten in iter-2+ once the
    # seeded multi-device shape is verified live.

  @wip @resilience-p1 @act:i
  Scenario: Cluster page shows offline device with last-seen freshness
    Given Matthew's household has device "matthew-mobile" archetype "mobile"
    And "matthew-mobile" went offline 4 minutes ago
    When Matthew opens "/shefa/cluster"
    Then the tile for "matthew-mobile" shows status "asleep · 4 min ago"

  @browser-only @resilience-p1 @requires:shem @act:iii
  Scenario: Peer-topology page aggregates by household, not by peer
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew's substrate is reciprocally hosting with 3 distinct households
    And one of those households is the Adam household with 2 active devices
    When Matthew opens "/shefa/peers"
    Then the peer-topology page renders
    And the summary shows "3 peer households · 3 reciprocating"
    And the Adam household appears as a single peer-household card
    # Constraint: per-peer rows would be drilldown noise; the resilience unit is
    # the household. See memory: "Household is the resilience unit".

  @wip @resilience-p1 @act:i
  Scenario: Peer-topology surfaces resilience-cliff warning
    Given one peer household holds the only external replica of any of Matthew's content
    When Matthew opens "/shefa/peers"
    Then the page renders a "resilience cliff" warning
    And the count of cliff households is non-zero

  @browser-only @resilience-p1 @requires:shem @act:iii
  Scenario: Reciprocity page shows inflow, outflow, and net hosting
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Adam has committed 5 GB to host Matthew's content and delivered 4.5 GB
    And Matthew has committed 3 GB to host Pete's content and delivered 3.1 GB
    When Matthew opens "/shefa/reciprocity"
    Then the reciprocity page renders
    And exactly 1 inflow row is visible (Adam)
    And exactly 1 outflow row is visible (Pete)
    And the "net" line reflects Matthew is net-hosted on balance

  @browser-only @resilience-p1 @act:i
  Scenario: Doorway operator dashboard topology tab is reachable
    Given human "Matthew" is logged in on doorway-app "alpha" with device
    And an operator opens the doorway admin dashboard
    When the operator clicks the "topology" tab
    Then the tab renders a federation snapshot from "/admin/dashboard/topology"
    And the snapshot shows known stewards and recent gossip windows
    # Observability gate: the operator must be able to see what doorway sees of
    # the substrate without leaving the admin shell.

  # --- Inline distribution telemetry on content cards ----------------------
  # T49+T50. Substrate hydrates DistributionSummary on EPR head responses; the
  # learner-facing concept card surfaces it inline (no extra fetch). Details
  # tier (4-dot expansion + diversity hint) is fetched lazily on tooltip open.

  @wip @resilience-p1 @act:i
  Scenario: Concept card renders distribution badge when summary is hydrated
    Given content "concept-foo" has been distributed to multiple replicas
    And the substrate hydrates "concept.distribution" on the head response
    When Matthew sees a concept card for "concept-foo"
    Then the card renders an "elohim-distribution-badge" element
    And the badge shows reach, replica count and freshness
    # Constraint: the learner experience must not pay an extra round-trip per
    # card. Hydration happens once on the head response; the card just renders.

  @wip @resilience-p1 @act:i
  Scenario: Concept card hides badge when distribution is not yet known
    Given content "concept-bar" has no blob_hash yet (pre-distribution)
    When Matthew sees a concept card for "concept-bar"
    Then the card renders no "elohim-distribution-badge" element
    # Boundary: undefined distribution is a valid state (pre-distribution
    # content, or historical projections not re-hydrated through EPR head).

  # --- Two-dimension coherence (distribution + resilience side-by-side) ----
  # 2026-05-03 coherence sub-pass. Distribution ("where are the bytes?") and
  # resilience ("is it safe?") are orthogonal dimensions surfaced together
  # on the content-viewer header so a steward sees both at once. The two
  # widgets share a row; they do NOT merge data shapes.

  @wip @resilience-p1 @act:i
  Scenario: Content-viewer header renders distribution and resilience together
    Given content "content-alpha" has been distributed to at least 2 households
    And the EPR head response hydrates both "resilience" and "distribution" fields
    When Matthew opens the content-viewer for "content-alpha"
    Then the header renders an "elohim-resilience-snapshot" element
    And the header also renders an "elohim-distribution-badge" element
    And both widgets are siblings on the same header row
    # Constraint: a grandmother needs to see "where is my photo" AND "is it
    # safe" at a glance, in the same place at the same time. Stacking or
    # tab-switching the two dimensions raises cognitive load past the
    # "credible to a grandmother" bar. Operational: both widgets render off
    # the single EPR head response — no extra round-trip.

  # --- Protocol omni resilience indicator -----------------------------------
  # Omnibar-consolidation follow-up (spec §9.6): the trust surface's resilience
  # segment wires to the live household snapshot. Glance signal only — the
  # omni is a navigation/provenance tool, not an analytics display; the
  # headline is stewarding collectives (the household/collective is the
  # resilience unit), with peers-online as tooltip drilldown. Never cries
  # wolf: until a snapshot loads the segment is a neutral glyph that makes
  # no status claim, and a fetch error stays neutral rather than alarming.

  @browser-only @resilience-p1 @act:i
  Scenario: Protocol omni toolbar surfaces the live resilience snapshot
    Given "content-alpha" has been distributed to at least 2 households
    When I open the EPR resource page for "content-alpha"
    And I expand the protocol omni toolbar
    Then the omni resilience segment renders a live resilience icon
    And the omni resilience icon has class "status-protected" or "status-partial"
    And the omni resilience tooltip mentions stewarding collectives

  # --- Native chrome-asset omni resilience contract (regression guard) ------
  # Distinct from the Angular protocol-omni scenario above: the doorway ALSO
  # SSR-splices a hand-written, framework-free chrome element onto every
  # served page (id="elohim-omni",
  # elohim/elohim-chrome-asset/src/omni-element.js). protocol-omnibar-chrome
  # .feature already pins the SERVED-HTML half of that contract (the context
  # island + content-addressed loader script) but is deliberately HTTP-only —
  # this scenario is the missing browser-tier coverage of its BEHAVIOR.
  #
  # Regression anchor: the element shipped reading data.glyph/standing/reach
  # — fields that never existed on ResilienceSnapshotView (the real contract
  # is protectionStatus + feltStatus, /api/v1/resilience/{slug}/household).
  # The fetch always succeeded against the live endpoint, but the mapper
  # always computed null, so the segment stayed the neutral ◉ glyph forever
  # ("Resilience snapshot unavailable") — a false-neutral, never caught at
  # runtime because the only a2o omni coverage targeted the Angular
  # protocol-omni component above, not this element. Fixed in cf7679688 (the
  # real protectionStatus/feltStatus contract mapping + drilldown card) and
  # b13d4d04e (the tri-state data-omni-resilience-loaded marker: loading →
  # applied | unmatched, so the DOM itself testifies whether the mapper found
  # mappable fields). "unmatched" is precisely the phantom-contract
  # regression state this scenario guards against.
  #
  # De-@wip'd 2026-07-12: the fixed element IS the one served — live Playwright
  # probes on BOTH doorways (doorway-alpha.elohim.host + elohim.host) confirmed
  # expand → data-omni-resilience-loaded settles to "applied" (not "unmatched",
  # not the phantom contract). The landing-slug target remains valid: live
  # probes (2026-07-11) confirmed BOTH doorways return 200 with
  # protectionStatus (+ feltStatus on /household) for elohim-host-landing.
  @browser-only @resilience-p1 @regression @act:i
  Scenario: Native omni chrome resilience segment speaks the real snapshot contract
    Given I open the doorway landing page in the browser
    When I expand the native omni chrome
    Then the native omni resilience-loaded state settles to "applied"
    And the native omni resilience glyph shows a live protection state
    And the native omni resilience title is no longer the phantom placeholder
    When I click the native omni resilience glyph
    Then the native omni resilience drilldown card is visible with a headline

  @browser-only @resilience-p1 @regression @act:i
  Scenario: Omni resilience tooltip folds down into the viewport, never up out of it
    # Regression anchor: the icon-density tooltip was hard-coded to flip UP
    # (bottom: 125%); protocol-omni is fixed to the top viewport edge, so on
    # desktop the tooltip rendered above y=0 — invisible. Top-chrome
    # affordances fold DOWN, inline-start-aligned (editor-menu convention,
    # matching the distribution badge's top:100%/left:0). Omnibar spec §11.
    Given "content-alpha" has been distributed to at least 2 households
    When I open the EPR resource page for "content-alpha"
    And I expand the protocol omni toolbar
    And I hover the omni resilience icon
    Then the omni resilience tooltip is fully inside the viewport
    And the omni resilience tooltip renders below the resilience icon

  @browser-only @resilience-p1 @act:i
  Scenario: Clicking the omni resilience icon folds down the resilience hypercard
    # Progressive disclosure (omnibar spec §11): tooltip is the zero-click
    # glance; click folds down a hypercard panel with the context-density
    # body — collectives, diversity, gaps — plus an action row. The panel
    # speaks the protocol's HyperCard idiom (pillar-EPR-decomposition §7.4).
    Given "content-alpha" has been distributed to at least 2 households
    When I open the EPR resource page for "content-alpha"
    And I expand the protocol omni toolbar
    And I click the omni resilience icon
    Then the resilience hypercard panel is visible below the icon
    And the resilience hypercard names the stewarding collective count
    And the resilience hypercard offers a "View full resilience" action

  @browser-only @resilience-p1 @regression @act:i
  Scenario: Resilience hypercard stays fully inside a phone viewport
    # Regression anchor: the hypercard pinned inset-inline-start:0 to the tiny
    # icon wrap with a 240px min width; from the omni toolbar on a 390px phone
    # the panel projected ~173px off the right screen edge — unreadable unless
    # the phone was rotated to landscape. End-side chrome now end-aligns the
    # panel (align="end", omnibar spec §11.2 amendment) and the panel clamps
    # its width to the viewport. The icon trigger also meets the WCAG 2.5.8
    # minimum tap target (it was a 7x14px glyph).
    Given "content-alpha" has been distributed to at least 2 households
    And the browser viewport is the "phone" archetype
    When I open the EPR resource page for "content-alpha"
    And I expand the protocol omni toolbar
    And I click the omni resilience icon
    Then the resilience hypercard panel is fully inside the viewport
    And the resilience hypercard panel is visible below the icon
    And the omni resilience icon meets the minimum tap target size

  @wip @browser-only @resilience-p1 @act:i
  Scenario: Content-viewer resilience fold-downs stay inside a phone viewport
    # Capability proof for the SECOND host of the same constraint: the
    # content-viewer's resilience icon trails the title's last line, so its
    # viewport position varies with title length. A start-pinned 240px panel
    # overflows a 390px phone for any last line wider than ~135px (most
    # titles); the viewer passes panelAlign="end" so the fold-down grows back
    # into the viewport. Operational parameters: 240px panel min-inline-size,
    # 390x844 phone archetype, ~135px title-line tipping point.
    Given "content-alpha" has been distributed to at least 2 households
    And the browser viewport is the "phone" archetype
    When I open the content viewer for "content-alpha"
    And I click the content-viewer resilience icon
    Then the content-viewer resilience hypercard is fully inside the viewport

  @browser-only @resilience-p1 @act:i
  Scenario: View full resilience flips the hypercard in place without navigating
    # HyperCard semantics: cards flip in place — deepening disclosure never
    # requires leaving the page. No full-resilience route exists, and none
    # is needed.
    Given "content-alpha" has been distributed to at least 2 households
    When I open the EPR resource page for "content-alpha"
    And I expand the protocol omni toolbar
    And I click the omni resilience icon
    And I choose the "View full resilience" hypercard action
    Then the resilience hypercard shows the full resilience card
    And the browser URL is unchanged

  @browser-only @resilience-p1 @act:i
  Scenario: Escape closes the resilience hypercard and returns focus to the icon
    Given "content-alpha" has been distributed to at least 2 households
    When I open the EPR resource page for "content-alpha"
    And I expand the protocol omni toolbar
    And I click the omni resilience icon
    And I press Escape in the resilience hypercard
    Then the resilience hypercard panel is not visible
    And the omni resilience icon has focus

  @wip @resilience-p1 @act:i
  Scenario: Distribution badge defers details fetch until tooltip opens
    Given a content-viewer is open for "content-alpha"
    And the distribution badge has a "blobHash" but no expanded details yet
    When Matthew first views the page
    Then no request is made to "/api/v1/blob/{hash}/distribution/details"
    When Matthew expands the distribution tooltip
    Then exactly one request is made to "/api/v1/blob/{hash}/distribution/details"
    And subsequent expansions reuse the cached response
    # Constraint: at card-grid scale (a learning path with 30+ concept cards)
    # eager detail fetch would be 30+ round-trips on render. The badge MUST
    # fetch only on tooltip-open and cache the result.
    # Operational parameters: 1 round-trip per blob_hash per session; informs
    # path-page rendering budget (≤1 head fetch + lazy details on demand).

  # --- coverageShortfall: how far short of safe, not just how many copies -----
  #
  # The vocabulary these two scenarios turn on, since the numbers mean nothing without it:
  #
  #   HOUSEHOLD    — the people and machines at one place, sharing one fate. A power cut, a
  #                  stolen laptop or a house fire takes the whole household at once, which is
  #                  why copies inside one household barely count as copies.
  #   COLLECTIVE   — the group that stewards a copy on its own hardware. DISTINCT collectives
  #                  are what survive each other, so this — not peers, not devices — is the
  #                  thing worth counting.
  #   FLOOR        — how many distinct collectives a piece of content needs before we are
  #                  willing to say it is safe. It is set by the content's TIER: "standard"
  #                  content has a floor of 3.
  #   SHORTFALL    — floor minus what was achieved. The number that says how much recruiting
  #                  is left to do, which a bare count of stewards can never say.
  #
  # A count alone cannot tell an operator "one collective short"; that is the whole reason the
  # snapshot aggregates in a way that preserves the deficit rather than collapsing to a total.
  #
  # BOTH SCENARIOS ARE @act:ii, and not because their steps are missing. A household is ONE
  # collective — matthew, jessica and james share one fate — so an Act I mesh cannot place a
  # content item across 2, let alone meet a floor of 3. Measured 2026-08-21: a search across 40
  # content items on the household mesh found none with any other collective count. Diversity
  # floors are an Act II property; the neighbourhood, where adam's household federates, is the
  # smallest stage on which these can be true or false at all. See layering/code-reds.md
  # § What Act I cannot witness.

  # Read-only: the step looks for a content item this mesh has ACTUALLY placed across the
  # named number of collectives rather than staging one — a manufactured item would prove
  # only that the arithmetic runs. Holds, naming the gap, when no such item exists here.
  @regression @act:ii
  Scenario: Resilience snapshot reports the diversity shortfall, not just a count
    Given a "standard"-tier content item with 2 distinct stewarding collectives (floor is 3)
    When the operator requests "/api/v1/resilience/{cid}/household"
    Then "stewardingCollectives" equals 2
    And "coverageShortfall" equals 1
    # Constraint: coverageShortfall = floor - achieved (CoverageRollup.deficit.measure()).
    # The visible payoff of replacing rows.len() with descent-preserving aggregation —
    # the count alone could not say "1 collective short of the floor."

  @act:ii
  Scenario: coverageShortfall is a present 0 when the floor is met (absent != zero)
    Given a "standard"-tier content item with 3+ distinct stewarding collectives (floor is 3)
    When the operator requests "/api/v1/resilience/{cid}/household"
    Then the response contains "coverageShortfall" with value 0
    # Constraint: the 3-state contract — present-N (short), present-0 (graph branch
    # measured the floor met), ABSENT (relational household_resilience path, not computed).
    # A consumer MUST distinguish "measured: met" (0) from "not computed" (absent).
