@e2e @resilience @local @concern:blob-durability @dataplane @act:i
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

  @requires:owned-substrate
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

  # @regression for the #3 serve-blob gap (RCA genesis/docs/content/elohim-protocol/history/2026-06-18-genesis-seed-stages-unstable-resilience-card-rca.md):
  # the PROACTIVE quilt-draw replication path moved bytes via a bare blob_store.store()
  # and booked NO serve-blob event — so "Verify Delivery Events" read 0 even though
  # propagation passed. finalize_quilt_draw now routes the proactive draw through the
  # SAME atomic pair as an on-demand heal (store + peer_blob_inventory row + serve-blob
  # event), so a steward who replicates content is recognized in the REA ledger too —
  # not just one who serves an on-demand read.
  @wip @regression
  Scenario: Proactive replication leaves a serve-blob delivery trail, like an on-demand heal
    Given content "replicated-target" with a blob held by a peer the steward replicates from
    And the steward's local blob store is missing the blob for "replicated-target"
    When the steward proactively draws the blob during content replication
    Then the steward's local blob store now holds the blob for "replicated-target"
    And listing economic events with action "serve-blob" for that blob hash returns at least one event
    And that event names the peer it drew from as provider

  @wip
  Scenario: No peer holds the bytes — the 404 names the missing blob
    Given content "orphan-target" is an "html5-app" whose ZIP blob no peer holds
    When I GET "/apps/orphan-target/index.html" from the doorway
    Then the doorway response status is 404
    And the doorway response body contains "App ZIP blob not found"

  # ── Harvested 2026-08-23 (household mesh repro, all three storage peers) ──
  # The landing SSR bundle crossed RS_THRESHOLD (64 MiB) once it started
  # carrying source maps — 71,763,974 bytes. PUT panicked the storage HTTP task:
  #   range start index 71763976 out of range for slice of length 71763974
  # Ingest asked ShardEncoder::create_manifest for the manifest (which above the
  # threshold mints "rs-4-7": 4 data + 3 parity shards over PADDED data) and then
  # hand-sliced the RAW body as if every non-"none" encoding were sequential
  # chunks. Shard 3 mis-hashed; shard 4 indexed past the end of the body. The
  # client saw the connection drop after 100 Continue and the blob 404'd on every
  # peer. Nothing in the suite had ever PUT a blob above the threshold, so the
  # erasure-coded band had never worked through PUT at all — a durability hole
  # for any artifact over 64 MiB, exactly the artifacts most worth holding.
  # Operational parameter pinned here: 64 MiB is a real ingest boundary, not an
  # internal detail — bundle growth crosses it silently.
  @requires:owned-substrate @regression
  Scenario: An artifact over the erasure-coding threshold is accepted whole and served whole
    Given an artifact of 68 MiB, above the erasure-coding threshold
    When I PUT the artifact to the storage peer under its own hash
    Then the storage peer accepts the artifact
    And GET "/blob" for that artifact from the same storage peer returns it byte-identical
