@e2e @resilience @felt @requires:shem @requires:multi-replica @wip
Feature: An under-replicated blob is salvaged by the deterministically-closest opt-in peer
  As a household whose content is held by too few peers to be safe
  I want an always-on peer that has opted in to spare capacity to notice the
  gap and adopt custody on its own — without any operator telling it to —
  but only if it is one of the peers the network would have chosen anyway
  So that resilience heals itself coordination-free, no peer is conscripted,
  and the steward who adopts is recognized in the REA ledger for the mutual aid.

  # Phase 3 of Blob Custody Reconciliation — the "Good-Samaritan salvage" door
  # Phase 2 left open (2026-05-02-blob-custody-reconciliation-design.md). Decision
  # function: genesis/docs/superpowers/specs/
  # 2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md.
  #
  # XOR-distance is the MVP placement STRATEGY behind a seam, NOT the final
  # purposeful policy — these scenarios assert the SALVAGE BEHAVIOUR (opt-in,
  # under-replication-triggered, closest-N self-selection, replica rises), which
  # any future intentional strategy (household-diversity-first, P3-8) must also
  # satisfy. The strategy is swappable; the felt outcome is the contract.
  #
  # Flow under test: salvage_pass (reconcile/custody.rs) detects honored < target,
  # the opted-in closest peer authors a custody-blob commitment via the conductor
  # (notarized), and the existing Phase-2 provider branch race-fetches the bytes
  # → serve-blob REA event → replica count rises.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  # === The healing path: under-replicated → closest opt-in peer adopts ========
  Scenario: An under-replicated blob is adopted by the closest opt-in peer and the replica count rises
    Given a content item "salvage-target" with a custody-blob commitment honored by only 1 peer
    And the target replication level for "salvage-target" is 2
    And an always-on peer "rescuer" has opted in to salvage capacity
    And "rescuer" is among the closest holders for "salvage-target" by the active placement strategy
    When the custody reconciler runs a salvage pass on "rescuer"
    Then "rescuer" authors a custody-blob commitment naming itself as provider for "salvage-target"
    And the commitment names the original content steward as receiver
    And on the next reconcile pass "rescuer" race-fetches the bytes for "salvage-target"
    And the honored replica count for "salvage-target" rises to 2

  # === Consent: a node that has NOT opted in is never drafted =================
  Scenario: A peer that has not opted in to salvage capacity is never conscripted
    Given a content item "consent-target" with a custody-blob commitment honored by only 1 peer
    And the target replication level for "consent-target" is 2
    And an always-on peer "bystander" has NOT opted in to salvage capacity
    And "bystander" would be among the closest holders for "consent-target"
    When the custody reconciler runs a salvage pass on "bystander"
    Then "bystander" authors no custody-blob commitment for "consent-target"
    And the salvage outcome records the blob as under-replicated but skipped for opt-out

  # === Coordination-free: only the closest-N adopt, not every spare peer ======
  Scenario: A spare peer that is not among the closest holders defers to those who are
    Given a content item "diversity-target" with a custody-blob commitment honored by only 1 peer
    And the target replication level for "diversity-target" is 2
    And an opted-in peer "far-peer" is NOT among the closest holders for "diversity-target"
    When the custody reconciler runs a salvage pass on "far-peer"
    Then "far-peer" authors no custody-blob commitment for "diversity-target"
    And the salvage outcome records the blob as skipped because the peer is not closest

  # === Idempotence: an already-resilient blob triggers no salvage =============
  Scenario: A blob already held at its target level triggers no salvage adoption
    Given a content item "settled-target" with custody-blob commitments honored by 2 peers
    And the target replication level for "settled-target" is 2
    When the custody reconciler runs a salvage pass on any opted-in peer
    Then no new custody-blob commitment is authored for "settled-target"
    And the salvage outcome records "settled-target" as not under-replicated

  # === Felt safety: the family sees the gap close, in human terms =============
  @felt
  Scenario: The family sees a thin blob become protected after a peer adopts it
    Given a content item "family-album" honored by only 1 household
    And an opted-in always-on household peer is among its closest holders
    When salvage adoption completes for "family-album"
    And I read "/api/v1/resilience/family-album/household"
    Then the response field "feltStatus.reassurance" is "protected"
    And the "feltStatus.heldBy" list grows to include the household that adopted it
