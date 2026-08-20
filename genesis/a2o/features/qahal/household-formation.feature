@e2e @qahal @household-formation @requires:household-nodes
Feature: Household formation — recognition of the given
  A family — each member with a device, hub or not — forms a household and
  immediately sees the protocol working among themselves. The ceremony is the
  ONLY canonical mint of the reciprocity bundle (formation spec §1: emergent +
  marked interim fixtures). These structural scenarios assert the ceremony's
  OUTPUT as projected to elohim-storage; seed-household-formation.ts drives the
  real per-conductor choreography in CI.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  # The member ids below are the seeder's canonical triad
  # (genesis/seeder/src/seed-household-formation.ts HOUSEHOLD_MEMBERS) —
  # human-matthew-manager, human-jessica-spouse, human-james-son. `james` is the
  # SON, not "human-james-student": that spelling appeared nowhere in the product
  # (not in the corpus, not in the custody seeder, not in deployments.json), so
  # the custody legs below asserted an id that could never be found — a red that
  # would have outlived the fleet outage currently masking it.

  # Measured red, genesis #1489 (2026-08-20), and NOT a seed-ordering bug — the
  # ceremony ran that build. The cid is unreadable rather than unstamped: the
  # wire shape of GET /db/collectives/{id} carries no collectiveCid field at all
  # (elohim-views CollectiveView projects id/name/governanceLayer/reach/region/
  # metadata and stops), while the SQL column collectives.collective_cid is real
  # and is what the formation projection stamps. Live probe of the alpha peer
  # 2026-08-20 returned the family-dowell row with governanceLayer "family" and
  # no cid key present. Until that field is projected, this scenario cannot pass
  # however well the ceremony runs — and neither can the seeder's own
  # resolveExistingCollectiveCid probe or its 60s projection settle-wait, which
  # read the same absent field and therefore always time out.
  Scenario: The household collective is coherent — family-layer, CID-stamped
    When I fetch the collective "family-dowell"
    Then the collective has governance layer "family"
    And the collective is anchored with a canonical collective CID

  # Measured red, genesis #1489: the ceremony reported 2/3 affirmed — the nominal
  # founder human-matthew-manager was unbindable, his conductor's get_my_human
  # returning an id that is not the canonical one, so jessica was elected founder
  # and matthew could never affirm. But the projection this scenario reads is
  # emptier still: a live probe of the alpha peer 2026-08-20 returned
  # {"items": [], "count": 0} for family-dowell — none of the three, not merely
  # matthew. Both affirmations were authored on their own members' conductors and
  # have not been projected onto the peer the E2E reads. The triad assertion is
  # right; the identity binding and the cross-peer projection are what is broken.
  Scenario: All three members are affirmed participants
    When I list participants of collective "family-dowell"
    Then the participant set includes the canonical household triad

  @wip
  Scenario: James's membership is sponsored, not self-granted
    When I list participants of collective "family-dowell"
    Then the participation of "human-james-son" carries a sponsor

  Scenario: The ambient custody mesh emerged from the ceremony
    When I list active "custody-blob" commitments
    Then an active "custody-blob" commitment exists from "human-matthew-manager" to "human-james-son"
    And an active "custody-blob" commitment exists from "human-james-son" to "human-matthew-manager"
    And an active "custody-blob" commitment exists from "human-jessica-spouse" to "human-james-son"
    And an active "custody-blob" commitment exists from "human-james-son" to "human-jessica-spouse"

  @wip
  Scenario: Ceremony custody is anchored, fixture custody is marked
    When I list active "custody-blob" commitments
    Then every "custody-blob" commitment with ceremony provenance is DHT-anchored
    And every "custody-blob" commitment with fixture provenance declares its retirement
