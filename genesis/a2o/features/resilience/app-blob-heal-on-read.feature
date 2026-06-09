@e2e @resilience @local @wip
Feature: App ZIP blobs heal on read via peer race-fetch
  As a household member visiting a projected EPR app
  I want a serving peer that lost the app's ZIP bytes to fetch them from
  a peer that still holds them, on my first visit
  So that verifiable durability is something the substrate does, not
  something an operator does at 2am — and the steward who served the
  bytes is recognized in the REA ledger for the mutual aid.

  # Evidence anchor (2026-06-09 /deliver iter-0): matthew's storage lost the
  # landing + lamad ZIP bytes (ephemeral storage_dir, rows persisted) and the
  # apps-resolver 404'd "App ZIP blob not found" for days; a manual
  # GET /blob/{hash} healed both surfaces through the existing T17 race-fetch
  # (peer_blob_inventory -> race_fetch -> finalize_fetch_success + serve-blob
  # REA event). This feature pins that the apps-resolver path heals itself.
  # Journal: .claude/deliver/journal-resilient-dual-doorway.md

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"

  @wip
  Scenario: First request to an app with locally-missing bytes heals and serves
    Given content "heal-target" is an "html5-app" with its ZIP blob held by at least one peer
    And the serving peer's local blob store is missing the ZIP for "heal-target"
    When I GET "/apps/heal-target/index.html" from the doorway
    Then the doorway response status is 200
    And the doorway response Content-Type contains "text/html"
    And the serving peer's local blob store now holds the ZIP for "heal-target"

  @wip
  Scenario: The heal books a serve-blob REA event for the source peer
    Given the serving peer healed "heal-target" from a peer race-fetch
    When I list economic events with action "serve-blob" for the healed blob hash
    Then at least one event names the source peer as provider

  @wip
  Scenario: No peer holds the bytes — the 404 names the missing blob
    Given content "orphan-target" is an "html5-app" whose ZIP blob no peer holds
    When I GET "/apps/orphan-target/index.html" from the doorway
    Then the doorway response status is 404
    And the doorway response body contains "App ZIP blob not found"
