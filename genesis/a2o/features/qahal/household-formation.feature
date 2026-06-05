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

  Scenario: The household collective is coherent — family-layer, CID-stamped
    When I fetch the collective "family-dowell"
    Then the collective has governance layer "family"
    And the collective is anchored with a canonical collective CID

  Scenario: All three members are affirmed participants
    When I list participants of collective "family-dowell"
    Then the participant set includes the canonical household triad

  @wip
  Scenario: James's membership is sponsored, not self-granted
    When I list participants of collective "family-dowell"
    Then the participation of "human-james-student" carries a sponsor

  Scenario: The ambient custody mesh emerged from the ceremony
    When I list active "custody-blob" commitments
    Then an active "custody-blob" commitment exists from "human-matthew-manager" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-matthew-manager"
    And an active "custody-blob" commitment exists from "human-jessica-spouse" to "human-james-student"
    And an active "custody-blob" commitment exists from "human-james-student" to "human-jessica-spouse"

  @wip
  Scenario: Ceremony custody is anchored, fixture custody is marked
    When I list active "custody-blob" commitments
    Then every "custody-blob" commitment with ceremony provenance is DHT-anchored
    And every "custody-blob" commitment with fixture provenance declares its retirement
