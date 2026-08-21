@e2e @content @epr @protocol @requires:doorway @act:i
Feature: The omnibar links out to the claims that back the EPR it wraps
  As a person reading any page a doorway serves me
  I want the protocol chrome to show me what BACKS this content — the story it
  rests on, the value it carries, the governance it answers to, and the atoms
  it is part of or related to — as links I can actually follow
  So that a page is never a dead end that merely asserts itself: its claims are
  one click away, addressed by content, not by a marketing promise.

  # The omnibar already says WHICH EPR this is (the copy-CID group) and HOW
  # RESILIENT it is (the dial). What it never said is what the EPR is COUPLED
  # to — and that omission was a mutual deferral, not a missing capability:
  # the doorway's context island left nav "to the element's own client-side
  # resolution", while the element hard-nulled navBack/navForward because "the
  # doorway supplies those". Each side waited for the other, so the links never
  # rendered from either. Two Category-C read-only projections were already
  # serving exactly this data the whole time:
  #   GET /api/v1/epr/{slug|cid}/nav-context → prev · next · partOf[] · related[]
  #   GET /api/v1/epr/{slug|cid}/raw         → coupling{knowledge,value,governance}
  # Client-side resolution wins: on first expand the element fetches BOTH (in
  # parallel, independent failure handling) and fills its nav slots + a compact
  # "backing claims" card, each row an epr-link to /epr/{cid} — the universal
  # EPR address. The island now supplies only what the client cannot know
  # (the authoritative slug, and whether the request was authenticated).
  #
  # Implementation: elohim/elohim-chrome-asset/src/omni-element.js
  #   (maybeLoadClaims / renderClaimsCard / fillNavSlot); the structural
  #   producer/consumer contract is pinned in that crate's lib.rs tests.
  #
  # Household floor: reads the alpha doorway over matthew's storage projection
  # — one node, no cross-node discovery — so no @requires:shem.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # Fleet-independent by construction: `/chrome/*` is an in-binary static
  # handler that touches neither storage nor the conductor, so this scenario
  # answers even while the fleet sheds content reads. (Measured 2026-08-20 on a
  # shedding alpha: `GET /health` 200 and `GET /chrome/omni-element.js` 404 from
  # the same doorway — the 404 was the missing route, not the outage.)
  #
  # The address asked for is the STABLE alias `elohim-chrome-asset` publishes
  # (`STABLE_ELEMENT_PATH`) precisely FOR references that cannot embed a content
  # hash — which is what a scenario is. Both runtimes that link the chrome crate
  # now serve it: the storage sidecar (`handle_chrome_asset` in
  # elohim/elohim-storage/src/http.rs) and the doorway
  # (doorway/doorway-service/src/routes/chrome.rs), each with
  # `Cache-Control: public, max-age=0, must-revalidate` + an `ETag` of the
  # content address — revalidate, never immutable, because the alias' bytes move
  # with the element.
  Scenario: The doorway serves the element that carries the claims resolution
    # The claims leg lives entirely inside the runtime-served element, so the
    # element asset being reachable is the precondition for every scenario
    # below. (The served-HTML half — island + content-addressed loader — is
    # pinned in protocol/protocol-omnibar-chrome.feature.)
    When I GET "/chrome/omni-element.js" from the doorway
    Then the doorway response status is 200
    And the doorway response Content-Type contains "javascript"

  # @wip: the browser-tier assertions below need step definitions wired in
  # genesis/a2o/steps/protocol/native-omni-chrome.steps.ts (the nav/claims
  # siblings of the resilience steps already there). They are ALSO data-blocked
  # today: the epr atom seeder is manual-only, so "elohim-host-landing" has no
  # epr_atoms row on alpha and both projections 404 — which is precisely why
  # the second scenario (the quiet path) is the one that currently describes
  # live behaviour. Sheds @wip when the steps land and the atom is seeded.
  @wip @browser-only
  Scenario: Expanding the omnibar reveals the claims backing this EPR
    Given I open the doorway landing page in the browser
    When I expand the native omni chrome
    Then the native omni claims-loaded state settles to "applied"
    And the native omni claims group is revealed
    When I click the native omni claims glyph
    Then the native omni claims card lists the "knowledge" leg as an epr-link
    And the native omni claims card lists the "governance" leg as an epr-link
    And every native omni claim link addresses an EPR at "/epr/"

  @wip @browser-only
  Scenario: An EPR with no recorded claims leaves the omnibar quiet
    # Degrading gracefully means STAYING QUIET, not apologizing: no empty
    # flyout, no "unavailable" chrome, no dead affordance a person can click
    # and get nothing from. The tri-state marker still testifies what happened,
    # so a probe can tell "nothing recorded" from "the fetch never ran".
    Given I open the doorway landing page in the browser
    When I expand the native omni chrome
    Then the native omni claims-loaded state settles to "unmatched"
    And the native omni claims group stays hidden
    And the native omni claims card offers no empty flyout

  @wip @browser-only
  Scenario: Nav links resolve client-side when the context island is silent
    # The mutual-deferral regression, pinned: the island supplies neither
    # navBack nor navForward, so if the element defers back to it the toolbar
    # is left without any adjacency at all.
    Given I open the doorway landing page in the browser
    When I expand the native omni chrome
    Then the native omni back-nav slot is filled from the nav-context projection
    And the native omni forward-nav slot is filled from the nav-context projection
