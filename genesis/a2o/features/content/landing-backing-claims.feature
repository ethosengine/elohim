# The DATA leg of the landing surface's backing claims. Track A's
# omni-claims-links.feature covers the UI leg (what the omni renders); this file
# covers what the seed path must have PUT into the substrate for that UI to have
# anything true to render.
#
# Why this exists:
#   The landing surface shipped with a fully instrumented p2p/resiliency plane
#   and an empty conscience. Its EPR atom carried `claims: []`, and its
#   governance coupling leg was sha256("elohim-host-governance-v1") — a hash of
#   a literal string, self-described in the seeder as "informational only", with
#   nothing on the other end of it. A link that points nowhere is not weaker
#   governance; it is the *appearance* of governance, which is worse, because
#   the omni would render it as though something stood behind it.
#
#   The seeder now mints a small acyclic atom family instead
#   (genesis/seeder/src/seed-epr-atom.ts): a schema manifest, a governance-state
#   manifest whose payload IS the governance state document verbatim, a
#   stewardship Claim, a custody Claim, and the landing atom — whose governance
#   leg points at the governance atom's real CID and whose claims array holds
#   the two Claim CIDs. Every leg resolves to something that exists.
#
# What "real" is pinned to here:
#   - the nav-context projection answers at all (it reads epr_atoms /
#     epr_coupling / epr_claims — an empty claims table means an empty `related`)
#   - `related` is non-empty and every entry carries a CID (outbound coupling
#     targets PLUS claim CIDs, per EprNavContextView)
#   - `derivedFrom` discloses which relationship kinds contributed — the
#     substrate saying out loud how it knows what it is showing
@e2e @content @epr @stewardship @requires:doorway @requires:seeded-content @act:i
Feature: The landing surface carries real backing claims
  As a visitor arriving at the protocol's own front door
  I want the page's stewardship, custody and governance context to be
  something the substrate can actually resolve
  So that "who tends this, who holds its bytes, what governs it" is a
  disclosure rather than a decoration.

  Background:
    Given peer "alpha" at "E2E_DOORWAY_ALPHA"

  # --- The atoms exist at all ---

  Scenario: The landing EPR resolves a nav-context projection
    # Precondition for everything below: the seeder's atom family landed, and
    # the slug-fallback lookup (epr_nav_context_view::project) finds it.
    Then EPR "elohim-host-landing" nav-context is reachable on peer "alpha"

  Scenario: The nav-context projection answers with a related-atom skeleton
    When I query "/api/v1/epr/elohim-host-landing/nav-context" on peer "alpha"
    Then the surface response status is 200
    And the surface response has field "cid"
    And the surface response has field "related"
    And the surface response has field "derivedFrom"

  Scenario: Every related entry is addressable
    # `related` = outbound coupling targets + claim CIDs. A non-empty array
    # whose entries all carry a cid is the regression anchor for the empty
    # `claims: []` era: with no claims and one dangling governance leg, this
    # array carried a single unresolvable stranger.
    When I query "/api/v1/epr/elohim-host-landing/nav-context" on peer "alpha"
    Then the surface response status is 200
    And each entry under "related" has field "cid"

  # --- The claims say something true ---

  # @wip: needs one new step-def — an assertion that `related` contains an entry
  # whose `label` equals a given schema_key. The projection already returns
  # `label` (EprNavRef.label = the referenced atom's schema_key, populated
  # whenever the atom is held locally — epr_nav_context_view::nav_ref_for_cid),
  # so this is a step-glue gap only, not a substrate gap.
  # Gate condition: lifts when steps/dataplane.steps.ts (or a content-side
  # sibling) gains `Then the entry labelled {string} appears under {string}`.
  @wip
  Scenario: The governance leg resolves to the landing surface's own governance state
    When I query "/api/v1/epr/elohim-host-landing/nav-context" on peer "alpha"
    Then the entry labelled "elohim-host-landing-governance" appears under "related"

  @wip
  Scenario: The claims name the stewardship and custody assertions
    # Not "there are claims" — WHICH claims. The landing atom asserts exactly
    # two: who stewards this content and on what provenance, and that a
    # custody-blob pledge is held for its SSR artifact.
    When I query "/api/v1/epr/elohim-host-landing/nav-context" on peer "alpha"
    Then the entry labelled "elohim-host-landing-stewardship-claim" appears under "related"
    And the entry labelled "elohim-host-landing-custody-claim" appears under "related"

  # --- The stewardship claim agrees with the stewardship rows ---

  # @wip: needs one new step-def — a by-content-id allocation query. Every
  # existing query step in steps/stewardship.steps.ts is category/tag-scoped and
  # deliberately picks a MULTI-steward representative, so neither can address a
  # single-steward surface like the landing page by id.
  # Gate condition: lifts when steps/stewardship.steps.ts gains
  # `When I query stewardship allocations for content {string}` (a thin wrapper
  # over the client.getAllocationsForContent call the category step already
  # makes) — the existing `... should be listed as a steward` /
  # `... contribution type should be {string}` assertions then apply unchanged.
  @wip @stewardship
  Scenario: The landing surface is stewarded, and the steward is the one it declares
    # The stewardship claim's payload is produced by the SAME resolver that
    # writes stewardship_allocations (seed-stewardship.ts resolveStewardship),
    # so the claim cannot drift from the rows without one of them being wrong.
    # The landing content declares stewardedBy human-matthew-manager, role
    # author — previously inert data that no seeder read: the allocation fell
    # through to the bootstrap default and the note said "uncategorized".
    When I query stewardship allocations for content "elohim-host-landing"
    Then Matthew should be listed as a steward
    And the contribution type should be "original_creator"
