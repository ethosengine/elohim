@e2e @auth @hosted-human @chaperone @requires:doorway @wip
Feature: Conductor Pool Recovery for Hosted Users
  As a hosted human whose doorway pool composition changed across deploys
  I want chaperone connection to self-heal
  So that login does not break when conductor IDs no longer match my persisted mapping

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # The bug class this feature pins down:
  #
  # Doorway's conductor map is in-memory (rebuilt from CONDUCTOR_URLS each
  # boot) but the agent→conductor map is persisted to MongoDB. When pool
  # composition changes (reorder, count change, host rename), persisted
  # rows can reference conductor_ids that no longer exist. Before the fix,
  # POST /hc/connect returned 502 {"error":"Conductor info not found"} for
  # those agents — they were stuck in a permanent reconnect loop. The
  # chaperone now treats stale Path-2 mappings as a re-provision trigger
  # and recovers via the existing idempotent provisioner.

  Scenario: Hosted human reconnects after the conductor pool composition changed
    Given human "Susan" is logged in on doorway "alpha"
    And Susan's agent→conductor mapping references a conductor no longer in the pool
    When Susan's browser POSTs to /hc/connect
    Then the response status should be 200
    And the response body should include "appToken"
    And the response body should include "appPort"
    And Susan's registry mapping should now reference a current conductor

  Scenario: Doorway exposes orphan-mapping count for operators
    When Matthew queries the admin conductors endpoint on doorway "alpha"
    Then totalAgents should equal the sum of per-conductor agentCount values
