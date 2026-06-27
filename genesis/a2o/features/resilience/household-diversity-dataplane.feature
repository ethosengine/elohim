@e2e @resilience @requires:household-nodes @wip @regression
Feature: The household-diversity dataplane reads the household projection
  As a household whose content must survive losing any single home,
  I want salvage and ingest placement to actually know which peers live in
  which household by reading the household_id the substrate already records,
  So that replicas spread across DISTINCT households instead of piling onto
  one, and a household going dark never takes the only copies with it.

  # Substrate-coherence guard for the humans-projection scope reconciliation
  # (2026-06-27-humans-projection-scope-reconciliation-plan). Production writes
  # humans under h_app_id="imagodei" (the identity pillar); the placement readers
  # used to filter under the content scope "lamad" and so enriched ZERO households,
  # silently degrading diversity placement to pure XOR. The fix routes every humans
  # reader through the canonical HUMANS_HAPP_ID. This scenario pins the felt outcome
  # (the candidate set carries real households) at the household floor.
  #
  # HONEST CEILING: this guards the SCOPE leg only. Full production efficacy also
  # needs (a) populated humans.agent_pub_key (per-pod registration, the humans-replayer
  # arc) and (b) agent_cid-namespace candidate ids (SELF_CID, the transport-id resolver).
  # The live cross-peer "replica count rises across households" proof is held to the
  # mesh (it needs @requires:shem); this scenario asserts the household-floor property
  # the salvage unit fixtures already prove.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  Scenario: Salvage candidates carry real households once humans are imagodei-populated
    Given a household "dowell" whose members have humans rows with populated agent keys under the identity scope
    And a content blob that is under-replicated
    When the node builds its salvage candidate pool
    Then each candidate that maps to a known household carries that household_id
    And the diversity placement strategy can span distinct households
