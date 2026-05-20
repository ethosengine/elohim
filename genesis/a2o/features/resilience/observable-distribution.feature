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

  @resilience-p1
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

  # --- Topology pages: my-cluster, peer-topology, reciprocity --------------
  # Light-up-the-topology sprint (Phase 7-9). Operator-visible topology surfaces
  # built from substrate views (/api/v1/cluster, /peer-topology, /reciprocity).
  # Household is the resilience unit — peer-topology aggregates per-household,
  # not per-peer. Reciprocity is a stewardship flow (inflow/outflow/net), not
  # a moral score.

  @browser-only @resilience-p1
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

  @wip @resilience-p1
  Scenario: Cluster page shows offline device with last-seen freshness
    Given Matthew's household has device "matthew-mobile" archetype "mobile"
    And "matthew-mobile" went offline 4 minutes ago
    When Matthew opens "/shefa/cluster"
    Then the tile for "matthew-mobile" shows status "asleep · 4 min ago"

  @browser-only @resilience-p1
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

  @wip @resilience-p1
  Scenario: Peer-topology surfaces resilience-cliff warning
    Given one peer household holds the only external replica of any of Matthew's content
    When Matthew opens "/shefa/peers"
    Then the page renders a "resilience cliff" warning
    And the count of cliff households is non-zero

  @browser-only @resilience-p1
  Scenario: Reciprocity page shows inflow, outflow, and net hosting
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Adam has committed 5 GB to host Matthew's content and delivered 4.5 GB
    And Matthew has committed 3 GB to host Pete's content and delivered 3.1 GB
    When Matthew opens "/shefa/reciprocity"
    Then the reciprocity page renders
    And exactly 1 inflow row is visible (Adam)
    And exactly 1 outflow row is visible (Pete)
    And the "net" line reflects Matthew is net-hosted on balance

  @browser-only @resilience-p1
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

  @wip @resilience-p1
  Scenario: Concept card renders distribution badge when summary is hydrated
    Given content "concept-foo" has been distributed to multiple replicas
    And the substrate hydrates "concept.distribution" on the head response
    When Matthew sees a concept card for "concept-foo"
    Then the card renders an "elohim-distribution-badge" element
    And the badge shows reach, replica count and freshness
    # Constraint: the learner experience must not pay an extra round-trip per
    # card. Hydration happens once on the head response; the card just renders.

  @wip @resilience-p1
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

  @wip @resilience-p1
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

  @wip @resilience-p1
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
