@e2e @resilience @resilience-dimensions
Feature: Resilience dimensions — the matrix that proves the felt-durability surface
  As a person deciding whether to trust this network with what matters to me
  I want every dimension of the resilience story — who protects it, how many
  peers are alive, where on earth it lives, what backs the promise — to be
  something the system can demonstrate, not assert
  So that the progressive icon next to a title means what it says

  # Dimensional index (spec: genesis/docs/superpowers/specs/
  # 2026-06-12-resilience-dimensions-proof-suite-design.md):
  #   D1 protection-status ladder . this file (thresholds in Rust:
  #      elohim-storage/tests/household_resilience.rs pins all 7 edges)
  #   D2 peer counts ............. this file
  #   D3 commitment-backing ...... this file
  #   D4 diversity score ......... Rust boundary tests (formula edges)
  #   D5 local/regional/global ... this file
  #   D6 progressive icon ........ this file (BOTH vocabularies)
  #   D7 high availability ....... carried by the flow features, not duplicated:
  #      - federation/peer-loss-failover.feature  (reads keep serving)
  #      - resilience/app-blob-heal-on-read.feature (race-fetch heal + REA event)
  #      - resilience/substrate-reconciliation.feature (scale-down names the dark)
  #      - federation/peer-recovery.feature       (wipe-and-reconverge drill)
  #
  # @wip rows map 1:1 to the data gaps verified 2026-06-12: the humans
  # household junction is unpopulated on alpha (D1/D2), provide commitments
  # exist only in test fixtures (D3), no collectives.region rows (D5).
  # Un-wipping these rows IS workstream D's acceptance gate.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- D1: protection-status ladder ------------------------------------------

  @wip
  Scenario: Content stewarded by no household reads at-risk, honestly
    Given a "commons"-reach content item "dim-orphan" with no stewarding household
    When I request "/api/v1/resilience/dim-orphan/household"
    Then the response field "protectionStatus" is "at-risk"
    And the response field "stewardingCollectives" is 0

  @wip
  Scenario: Two stewarding households lift content to partial
    Given content "dim-pair" is stewarded by households "matthew-home" and "jessica-home"
    When I request "/api/v1/resilience/dim-pair/household"
    Then the response field "protectionStatus" is "partial"
    And the response field "stewardingCollectives" is 2

  @wip
  Scenario: Three households with two live peers reach protected
    Given content "dim-triad" is stewarded by 3 distinct households
    And at least 2 peers across those households are online
    When I request "/api/v1/resilience/dim-triad/household"
    Then the response field "protectionStatus" is "protected"
    # Threshold truth: protected requires BOTH floors (households>=3 AND
    # peers>=2) — the Rust ladder pins the near-miss cases (3h/1p, 2h/2p).

  # --- D2: peer counts --------------------------------------------------------

  @wip
  Scenario: The tooltip's peers-online number counts only stewarding households
    Given content "dim-triad" is stewarded by 3 distinct households
    And a peer in an unrelated household is online
    When I request "/api/v1/resilience/dim-triad/household"
    Then the response details field "onlinePeerCount" counts only peers within the stewarding households
    # Regression anchor: list_by_household shipped as a stub returning ALL
    # peers, multiplying counts per household — inflating status toward
    # protected. Pinned deterministically in the Rust D2 tests.

  Scenario: The header connection chip shows a live peer count
    When I request "/health" on doorway "alpha"
    Then the response reports a non-negative "peerCount"

  # --- D3: commitment-backing -------------------------------------------------

  @wip
  Scenario: A stewarding household with an active provide commitment is commitment-backed
    Given household "matthew-home" stewards content "dim-backed"
    And "matthew-home" holds an active "provide" commitment scoped "content:commons"
    When I request "/api/v1/resilience/dim-backed/household"
    Then the response field "commitmentBackedCollectives" is at least 1
    # Verifiable durability beats claimed durability: the count is the
    # notarized promise, not the observed bytes.

  @wip @regression
  Scenario: A household-reach content can be commitment-backed (not just commons)
    # Stage B (hash-neutral): the provide commitment generalized from commons-only
    # to reach-general. A household's OWN content — never destined for the commons —
    # earns the same demonstrable commitment-backing. The provide commitment carries
    # reach "household" with reach_ceiling "commons" (the bound that keeps the DNA
    # integrity zome unchanged); the snapshot, scoped to the content's own reach,
    # counts it. This guards the regression where non-commons content was silently
    # uncounted because the author hard-coded reach "commons".
    Given household "matthew-home" stewards content "dim-household-backed" at reach "household"
    And "matthew-home" holds an active "provide" commitment scoped "content:household"
    When I request "/api/v1/resilience/dim-household-backed/household"
    Then the response field "commitmentBackedCollectives" is at least 1

  @wip
  Scenario: A commons-reach provide commitment does NOT back a household-reach content
    # The scope is exact: a content:commons provide row is not counted for a
    # household-reach content (the snapshot scopes to the content's own reach).
    # Reach is the discriminator, not a wildcard.
    Given household "matthew-home" stewards content "dim-household-only" at reach "household"
    And "matthew-home" holds an active "provide" commitment scoped "content:commons"
    When I request "/api/v1/resilience/dim-household-only/household"
    Then the response field "commitmentBackedCollectives" is 0

  # --- D5: local / regional / global projection -------------------------------

  @wip
  Scenario: Geographic distribution buckets stewards relative to the viewer
    Given household "matthew-home" has region "us-east" and stewards "dim-geo"
    And household "remote-home" has region "eu-west" and stewards "dim-geo"
    And the viewer's household has region "us-east"
    When I request "/api/v1/resilience/dim-geo/household" as the viewer
    Then the regional distribution reports 1 "local" and 1 "regional"

  @wip
  Scenario: Stewards without region data are honest unknowns, not zeros
    Given content "dim-noregion" is stewarded by a household with no region row
    When I request "/api/v1/resilience/dim-noregion/household"
    Then the regional distribution reports the steward under "unknown"
    And the snapshot panel shows "no region data" rather than an empty section

  # --- D8: storage aggregation triptych (free / used / committed) -------------
  # Truth: cluster_view::compose_totals (device sums + custody-blob committed,
  # clamped) and peer_capacity_service (pledged-vs-held, saturate-never-wrap).
  # Boundary edges pinned in Rust (cluster_view in-module D8 tests +
  # peer_capacity_service's existing 7). These rows prove the felt surface.

  @wip @browser-only
  Scenario: The cluster page shows an honest free/used/committed triptych
    Given human "Matthew" is logged in on doorway "alpha" with device
    And Matthew's devices report storage totals
    When Matthew opens "/shefa/cluster"
    Then the totals show used and total bytes summed across reporting devices
    And devices that report no storage are absent from the sums, not zeroed
    And the committed figure reflects only custody-blob commitments by Matthew's bound peers

  @wip
  Scenario: A peer's capacity view never wraps an over-pledge into compliance
    Given a peer with no known disk capacity holds a storage pledge
    When I request the peer's capacity view
    Then the pledge percentage reads saturated, not wrapped
    And the ratio compliance reports a violation rather than silence
    # Regression anchor: storage-tier review 2026-06-04 finding #1 — a plain
    # u8 cast truncated mod 256 and an unbounded over-pledge could read
    # donut-compliant.

  # --- D6: the progressive icon, both vocabularies pinned together ------------

  @wip @browser-only
  Scenario: A partial content shows the half glyph AND the partial status class
    Given content "dim-pair" is stewarded by 2 households
    When I open the content viewer for "dim-pair"
    Then the EPR relationship card icon shows "◐"
    And the resilience snapshot icon has class "status-partial"
    # Two threshold vocabularies exist (icon: steward count ≥3/1–2/0;
    # snapshot: households+peers compound). This row makes divergence visible.

  @wip @browser-only
  Scenario: A protected content shows the full glyph AND the protected status class
    Given content "dim-triad" is stewarded by 3 households with 2 peers online
    When I open the content viewer for "dim-triad"
    Then the EPR relationship card icon shows "●"
    And the resilience snapshot icon has class "status-protected"
    And the tooltip mentions 3 collectives
