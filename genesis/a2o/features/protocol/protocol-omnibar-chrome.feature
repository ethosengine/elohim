@e2e @protocol @regression @requires:doorway
Feature: The doorway travels the protocol trust surface onto every page it serves
  As a person browsing the Elohim Protocol through a doorway — arriving at the
  front door, then wandering on into a pillar
  I want the protocol trust surface — the EPR omnibar that says whose content
  this is, at what reach, with what resilience — to ride along on every page
  the doorway hands me
  So that the chrome that turns a plain web page into a protocol surface — the
  first bridge toward an elohim-native browser — is never silently dropped, no
  matter which of the doorway's serving paths rendered the page.

  # This is a SERVED-HTML contract, not a rendered-pixels check: the trust
  # surface is spliced into the bytes the doorway emits, so an HTTP GET is
  # enough to prove it is there. No Playwright / browser tier — deliberately
  # lean.
  #
  # The regression this guards (class, not instance): the omnibar used to live
  # inside the Angular bundle. It left the bundle (887d81fd5) to become
  # doorway-spliced chrome (bd22e1559) — a step toward an elohim-native browser
  # where the trust surface belongs to the doorway, not to any one app. But the
  # first splice reached only the SSR dispatch arm, while the front door "/" and
  # the pillar mounts ("/lamad", ...) are actually served by the EPR router path
  # (dispatch_to_projected_epr in doorway/doorway-service/src/server/http.rs),
  # which did NOT inject — so the trust surface vanished from exactly the pages a
  # first-time visitor lands on. The contract restored, and pinned here: EVERY
  # 2xx text/html response the doorway serves THROUGH the EPR router carries the
  # omnibar chrome, spliced in before </body> exactly once —
  #   - a context island  <script type="application/json" id="elohim-omni-context">…</script>
  #   - a content-addressed loader <script src="/chrome/omni-element.<sha256>.js" defer></script>
  # (markers are the source of truth in elohim/elohim-chrome-asset/src/lib.rs —
  # CONTEXT_SCRIPT_ID + element_script_path(); this scenario asserts exactly what
  # inject_element() emits).
  #
  # Household floor: "/" and the pillar mounts are served by the alpha doorway
  # over matthew's storage projection — one node, no cross-node discovery — so no
  # @requires:shem (mirrors landing-discovery.feature).
  #
  # Reading a red here (2026-08-20): this contract can only be judged on a page
  # the doorway actually SERVED. A doorway that is shedding never reaches the
  # EPR router at all — it answers 503 either as the upstream/admission shed
  # (`{"status":"catching-up"}`) or as the degraded root page (empty EPR
  # projection) — so a 503 is the fleet being unavailable, not the trust surface
  # falling off. The serving-precondition step names which of the two happened,
  # in its own failure text, so that distinction never has to be re-derived by
  # hand: both scenarios failed this way in elohim-genesis #1489 and were
  # mis-filed as a code regression on the strength of a bare `503 !== 200`.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: The front door carries the protocol trust surface
    # The very first page a stranger meets — served through the EPR router, the
    # path the first splice missed — must arrive already wearing the omnibar.
    When I GET "/" from the doorway
    Then the doorway serves that page — status 200, not a shed page
    And the doorway response Content-Type contains "text/html"
    And the served page carries exactly one "elohim-omni-context" omnibar context island
    And the served page references the "/chrome/omni-element." omnibar element loader

  Scenario: A pillar mount carries the same trust surface as the front door
    # Wandering from the door into a pillar is the same serving-path class — the
    # EPR router again — so the trust surface must travel there unchanged. The
    # visitor never crosses a page where the protocol chrome quietly falls off.
    When I GET "/lamad" from the doorway
    Then the doorway serves that page — status 200, not a shed page
    And the doorway response Content-Type contains "text/html"
    And the served page carries exactly one "elohim-omni-context" omnibar context island
    And the served page references the "/chrome/omni-element." omnibar element loader
