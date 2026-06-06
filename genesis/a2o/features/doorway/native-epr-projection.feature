@epr-decomposition @b22 @doorway @projection
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

  @wip @cache-eviction
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

  @wip @route-claims @regrant
  Scenario: Updating a projection's granted claims takes effect without a new mount
    # Story-harvest (Slice 3, 2026-06-06) — constraint boundary: the commitment id is
    # content-addressed over (steward|action|scope), so re-seeding the SAME projection
    # with NEW grant metadata 409s and the old grant-less row keeps serving. The spec's
    # §3.4 re-grant ceremony (claims-stale → steward re-grants) therefore needs a
    # metadata-update or supersession path — none exists yet. Surfaced in Task-14 sweep;
    # carried on the alpha deploy watch (the seeded lamad grant only reaches doorways
    # whose projection row is created AFTER 3777bd185).
    # Operational parameters: idempotency key = sha256(stewardPeerId|project-epr|scope);
    # informs: re-grant tooling design + alpha re-seed runbook.
    Given the alpha doorway has an active project-epr commitment for "lamad-spa" with no routeClaims
    When the steward re-grants the projection with routeClaims for contentType "path"
    Then the doorway's claims index serves /epr/{path-id} as a 302 to /lamad within one refresh cycle
    And the supersession is walkable on the commitment chain
