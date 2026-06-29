@e2e @resilience @felt @grandma-vertical @concern:blob-durability @dataplane
Feature: Grandma's photos survive a node loss; the family sees they're held
  As a family member of someone whose edge node has gone offline
  I want to see, in named human terms, that her memories are still held by people who love her
  So that the system's resilience proof becomes felt safety, not a faceless SLA

  # The spine of the vision-gap vertical (genesis/docs/superpowers/plans/
  # 2026-06-14-vision-gap-grandma-vertical-stub.md). It couples P-PROOFS' chaos
  # proof (O7) to felt safety (O1). The felt projection is the additive
  # `feltStatus` block on ResilienceSnapshotView, computed in
  # household_resilience.rs::snapshot() — floor-relative and unmeasured-aware.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  # === Couples O7's proof (P-PROOFS chaos) to O1 felt safety ================
  # Satisfiable now: the additive feltStatus projection over existing chaos infra.

  Scenario: Grandma's node goes offline but her photos remain protected
    Given a content item "summer-1974" stewarded by 3 distinct households each with an active "intimate" provide commitment
    And each stewarding household has a human-readable collective name
    When grandma's edge node goes offline
    And I read "/api/v1/resilience/summer-1974/household"
    Then the response field "feltStatus.reassurance" is "protected"
    And the response field "feltStatus.headline" matches "^Held by 3 households: "
    And every "feltStatus.heldBy" entry has a non-empty "label"
    And the response does not contain a faceless SLA percentage in "feltStatus.headline"

  # === Couples to O7's RS-reconstruct + placement-diversity proofs (#7a/#7b) ==

  Scenario: A holder lapses but coverage survives — the family is reassured, not alarmed
    Given a content item "summer-1974" stewarded by 3 distinct households
    And a placement-gap event has fired for "summer-1974" because one household stopped holding a shard
    When I read "/api/v1/resilience/summer-1974/household"
    Then the response field "feltStatus.reassurance" is "watching"
    And the response field "feltStatus.headline" contains "still"
    And no red "at-risk" verdict is shown in "feltStatus"

  # === Couples to O3 limit-respect: the limit shown is the operator's, not a verdict on grandma ==
  # The unmeasured≠zero honesty discipline made felt (resilience-snapshot-view.schema.json distributionState).

  Scenario: The system cannot yet see the album — it says so honestly
    Given a content item "private-letters" that has never entered the distribution plane
    When I read "/api/v1/resilience/private-letters/household"
    Then the response field "distributionState" is "unmeasured"
    And the response field "feltStatus.reassurance" is "not-yet-seen"
    And the response field "feltStatus.headline" contains "can't confirm"
    And the response field "feltStatus.suggestedAction" is "Invite a household to help hold these"
    And the response field "feltStatus.reassurance" is not "at-risk"

  # === The attach point for O5 (data agency) and O2 (observed care) ==========

  Scenario: When protection is short, the family is offered a pro-social action
    Given a content item "wedding-album" stewarded by only 1 household
    When I read "/api/v1/resilience/wedding-album/household"
    Then the response field "feltStatus.reassurance" is "needs-help"
    And the response field "feltStatus.suggestedAction" is "Invite a household to help hold these"

  # === The felt SURFACE renders names, not nines (eyes-first) ================
  # @browser-only: needs the <elohim-memory-safety> Family Vault component wired
  # into a reachable route + a live deploy. The integrator's CI browser stage is
  # the fresh-trigger confirmation; locally the story renders via `pnpm graphos`.

  @browser-only @wip
  Scenario: The Family Vault surface shows the holders by name
    Given "summer-1974" is held by 3 named households
    When the family opens the Memory Safety surface for "summer-1974"
    Then they see the holders rendered by name, not by shard hash or percentage
    And they see the reassurance state as warm language, not a red SLA

  # === Real-data: a blob-backed album lights stewarding (Epic B) ==============
  # The card honestly reads dark for inline-body content (DHT-replicated, never
  # shard-distributed). A photo album IS the blob case: it has a blob_hash, so it
  # enters the distribution plane. These two scenarios prove the card LIGHTS for
  # genuinely shard-distributed content — first deterministically (the operator
  # seed lever, for demo/test/thin-mesh), then via the real P2P path on a 2-node
  # mesh. The honesty boundary: the seed lever is OFF by default
  # (ALLOW_SEED_SHARD_MANIFEST=1) and marks rows status="seeded", so a real
  # deployment can never fabricate distribution.

  @resilience-p1 @requires:seeded-content
  Scenario: A blob-backed album lights stewarding via the operator seed lever
    # Deterministic lever (PUT /admin/seed/shard-manifest): writes a shard
    # manifest + agent-keyed shard_locations for blob-backed content WITHOUT
    # waiting on full P2P gossip. The card lights because the rows are real
    # operational projection rows (Category C), agent-keyed to join humans →
    # household_id exactly like the runtime distribute_shards path.
    Given a blob-backed content item "grandma-album-1974" exists with reach "intimate"
    And the operator seed-shard-manifest lever is enabled
    When the operator seeds a shard manifest for "grandma-album-1974" stewarded by 3 distinct households
    And I read "/api/v1/resilience/grandma-album-1974/household"
    Then the response field "distributionState" is "measured"
    And the response field "stewardingCollectives" is at least 3
    And the response field "feltStatus.reassurance" is "protected"

  @resilience-p1 @local @requires:household-nodes
  Scenario: A blob-backed album distributes across the live household mesh
    # The GENUINE path: POST /db/content with a blob_hash triggers distribute_shards,
    # which selects committed peers and pushes shards, writing shard_locations on
    # placement. On the household P2P mesh (Matthew/Jessica/James — the stable
    # floor, itself a multi-node cluster) this lands ≥2 stewarding households.
    # Constraint: requires ≥2 nodes online with active "commons" provide
    # commitments and the in-pod conductor cells enabled (see backlog
    # resilience-card-self-cid-provide-loop-gate). Household-testable, not @shem.
    Given the cluster has peers in at least 2 distinct households each with an active "commons" provide commitment
    When I ingest a blob-backed "commons"-reach content item "mesh-album-1974"
    Then within 60 seconds "/api/v1/resilience/mesh-album-1974/household" reports "stewardingCollectives" >= 2
    And the response field "distributionState" is "measured"

  # === Sibling-stub ATTACH POINTS (named now so the spine shows where they land) ==
  # These assert the EFFECT of the pro-social action, owned by the sibling stubs.
  # Held @wip until each sibling is greenlit; the felt surface above is their anchor.

  @wip @s-agency
  Scenario: Accepting "who holds my photos" surfaces the holders and a revoke control
    # O5 data-agency stub (genesis/.../vision-gap-data-agency-stub.md)
    Given the family is viewing the Memory Safety surface for "summer-1974"
    When they open "who holds these"
    Then they can see each holder and revoke a holder's access

  @wip @s-care
  Scenario: Accepting an invite to help emits an observed-care economic event
    # O2 care-valueflows stub (genesis/.../vision-gap-care-valueflows-stub.md)
    Given the family is offered "Invite a household to help hold these"
    When a household accepts the invite to help hold "wedding-album"
    Then an observed-care EconomicEvent is emitted naming the helping household

  @wip @s-spine
  Scenario: The "watching" message names which holder lapsed
    # Follow-on: needs gap→holder linkage in the custody emit path (PlacementGapView
    # carries shard_hash, not holder identity). See backlog: the felt surface names
    # the lapsed holder once the gap event carries the lapsed household id.
    Given a placement-gap event has fired for "summer-1974" because "Aunt Ruth" stopped holding a shard
    When I read "/api/v1/resilience/summer-1974/household"
    Then the response field "feltStatus" names the household that lapsed
