@epr-decomposition @b22 @doorway @projection @act:i
Feature: Doorway natively projects EPRs at author-declared URL paths
  As a steward of a doorway, I declare which EPRs my doorway hosts
  and at what URL paths, so visitors reach the protocol-native experience
  via clean web2.0 URLs without doorway-side hardcoding.

  Scenario: Bare hostname serves the landing EPR
    Given the alpha.elohim.host doorway has an active project-epr commitment for "elohim-host-landing" at urlPath "/"
    When an anonymous browser GETs "https://alpha.elohim.host/"
    Then the response serves the landing EPR's bundle entry file
    And the response is HTTP 200

  Scenario: Pillar path serves the pillar EPR
    Given the alpha.elohim.host doorway has an active project-epr commitment for "lamad-spa" at urlPath "/lamad"
    When an anonymous browser GETs "https://alpha.elohim.host/lamad/concept/fair-exchange"
    Then the response serves the lamad bundle's index.html (SPA fallback)
    And the lamad bundle's <base href> is "/lamad/"
    And Angular client-side router handles "/concept/fair-exchange"

  @cache-eviction @requires:owned-substrate
  Scenario: Bundle redeploy evicts doorway cache
    Given the lamad-spa EPR's blob is sha256-OLD
    When a deploy PATCHes the lamad-spa EPR with blobHash sha256-NEW
    Then within 5 seconds, the doorway's cache for "/lamad/index.html" is evicted
    And the next browser request to "/lamad/index.html" serves bytes from sha256-NEW

  @wip @federation
  Scenario: Federation — same EPR projected on second doorway serves same content
    Given the elohim.host doorway also has an active project-epr commitment for "lamad-spa" at urlPath "/lamad"
    When an anonymous browser GETs "https://elohim.host/lamad/"
    Then the response serves the same lamad bundle as alpha.elohim.host
    And both doorways' projections reference the same blob_hash

  @wip @route-claims @validation
  Scenario: An alias grant colliding with a reserved prefix is rejected at create time
    # Story-harvest (Slice 3, 2026-06-06): validate_project_epr_commitment existed
    # with an aspirational doc-comment and ZERO production call sites — validation
    # claimed-at-request-time was never wired. Now wired in ReaCommitmentService::create
    # (8b8ca9dd8). Constraint: a validator that exists but is not invoked is
    # indistinguishable from no validator; this scenario pins the WIRING, not the rules.
    Given a steward submits a project-epr commitment for "lamad-spa" at urlPath "/lamad"
    And its metadata declares redirectsFrom containing "/epr"
    When the commitment is POSTed to /api/v1/commitments
    Then the response is a validation rejection naming the reserved prefix
    And no rea_commitments row is created for it

  @route-claims @regrant @requires:doorway
  Scenario: Re-granting a projection's claims supersedes the grant-less row
    # Story-harvest (Slice 3, 2026-06-06) — constraint boundary RESOLVED: the commitment
    # id is content-addressed over (steward|action|scope), so re-seeding the SAME
    # projection with NEW grant metadata used to 409 and the old grant-less row kept
    # serving forever. The spec's §3.2/§3.3 re-grant ceremony is now wired as
    # SUPERSESSION-ON-CREATE: the steward POSTs a fingerprint-suffixed successor
    # carrying `supersedes:{predecessorId}`, and storage (create_with_supersession)
    # transactionally marks the predecessor `superseded` and inserts the successor.
    # find_active_projections excludes `superseded`, so within one doorway refresh
    # cycle ONLY the grant-bearing successor serves — no new mount, same urlPath. The
    # chain stays walkable via GET /api/v1/commitments/{id} (successor metadata carries
    # `supersedes`; predecessor state reads `superseded`). first-write-wins: a second
    # supersede of the same predecessor 409s.
    # Operational parameters:
    #   base id        = project-epr-sha256(stewardPeerId|project-epr|scope)[:16]
    #   successor id   = {base}-r{sha256(stableJson(projectionRelevantMetadata))[:8]}
    # informs: re-grant tooling (seed-projections.ts drift → supersede) + alpha re-seed runbook.
    Given the alpha doorway has an active grant-less project-epr commitment for "lamad-spa" at "/lamad"
    When the steward re-grants the "lamad-spa" projection with routeClaims for contentType "path"
    Then within one refresh cycle the active projection for "lamad-spa" carries the granted claims
    And the previous grant-less commitment is marked superseded and walkable on the chain
