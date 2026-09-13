@e2e @federation @concern:served-under-standing @act:i @requires:owned-substrate
Feature: Jessica reaches a hosted site through a doorway and gets a bounded 404 for a stale entry request
  Jessica knows the site "garden", not which machine holds it. She can ask a
  doorway she knows and receive the site from its holder. If that doorway's
  knowledge is stale, this proof checks a request for the archive's published
  entry document, index.html: she gets HTTP 404 within 15 seconds instead of a
  timeout, and the request does not keep circulating. The stale-case outcome for
  the extension-less site path is not covered; that path may show a gateway page.

  Doorways protect, absorb, project, balance, and route over authoritative holons
  and the mesh of peers. This implementer-facing acceptance proof measures routing
  on an owned household test mesh: two HTTP gateways, alpha and beta. It assumes
  Jessica may receive the site; authorization is a separate proof.

  Here garden is a site path, not a DNS hostname. Jessica supplies only beta's
  address and a unique path such as /nrt-garden-123/. Alpha's address is discovered
  by beta. Registrations publish doorway addresses; hosting contracts identify
  which sites they host. Published records reach each doorway's local registry
  asynchronously. Each scenario stages its own archive and hosting contract and
  waits for local service, so old runs cannot satisfy a new scenario.

  "Jessica is served garden" means HTTP 200 containing the staged archive's
  garden marker and this scenario's unique nonce. The successful relay also
  compares the returned page byte-for-byte with a direct request to alpha.
  Access and relay logs are captured on both processes for the unique path during
  Jessica's request. The extra byte-comparison request is outside that capture;
  the advertised-address scenario's second request gets its own observation window.

  An instrumented test client can read the x-elohim-served-by response header,
  called the returned origin here, and use that holder address for another request.
  The second scenario proves protocol address usability for implementers; ordinary
  browsers are not claimed to navigate to that address automatically.

  Pausing alpha means stopping its owned process with SIGSTOP and confirming the
  kernel reports it stopped: it cannot answer requests. The local-holder scenario
  checks that beta serves garden without a logged relay attempt while alpha is
  paused, then resumes alpha and checks its health. It does not simulate a 503
  overload response or prove a production load-shedding policy.

  A local holder serves directly; otherwise a doorway may forward once to a
  registry holder. This lane measures the resulting requests and absence of
  onward forwarding, not the forwarded request's header bytes. It exercises one
  remote holder, not health-based selection among several remote holders.

  Runner prerequisites: E2E_DOORWAY_ALPHA and E2E_DOORWAY_B name the two gateways.
  The household lane owns the processes it faults and restores induced faults
  even after failure. The tags select that local lane and join its results to
  the served-under-standing check; without process ownership faults are skipped.
  Public DNS membership, internet ingress, unknown sites and unreachable holders
  need separate proofs. Neither these local routes nor a registry listing grants
  authority to serve content.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_B"
    And both doorways can read the registry of doorway registrations and hosting contracts
    And both doorways are registered in the registry
    And Jessica is a visitor asking as an ordinary web client

  Scenario: A non-holder doorway routes once to the registered live holder
    Given the household stages the root "garden" as hosted by doorway "alpha" only
    And doorway "beta" still resolves "garden" to doorway "alpha" in its local registry
    And doorway "beta" has no contract to host "garden" locally
    And doorway "alpha" is serving
    When Jessica asks doorway "beta" for "garden"
    Then Jessica is served "garden"
    And the relayed page matches a direct request to doorway "alpha"
    And doorway "beta" logged a relay resolved through its hosting-contract registry
    And doorway "beta" forwarded the request exactly once
    And doorway "alpha" received the forwarded request exactly once
    And doorway "alpha" forwarded nothing onward for that request
    And the reply names doorway "alpha" as the origin for "garden"
    And doorway "beta" never answered that it does not host "garden"

  # Protocol usability check for implementers: an instrumented client reads the
  # advertised address and verifies it directly. This does not promise automatic
  # navigation in Jessica's production browser. Origin means the address returned
  # in x-elohim-served-by, not the HTTP Origin request header.
  Scenario: An instrumented client can use the advertised holder address directly
    Given the household stages the root "garden" as hosted by doorway "alpha" only
    And doorway "beta" still resolves "garden" to doorway "alpha" in its local registry
    And doorway "beta" has no contract to host "garden" locally
    And doorway "alpha" is serving
    When Jessica asks doorway "beta" for "garden"
    Then Jessica is served "garden"
    And the reply names doorway "alpha" as the origin for "garden"
    When the test client representing Jessica uses the returned origin to ask for "garden" again
    Then the request went directly to doorway "alpha"
    And Jessica is served "garden"
    And doorway "beta" forwarded nothing for that second request

  # Owner order means holder candidate order. Alpha is placed first to prove
  # candidate order cannot override beta's own ability to serve locally.
  Scenario: A local holder serves while its sibling is paused
    Given the household stages the root "garden" as hosted by doorway "alpha" and doorway "beta"
    And doorway "alpha" is the first holder in owner order
    And doorway "beta" is the next holder after it
    When the household pauses doorway "alpha"
    And Jessica asks doorway "beta" for "garden"
    Then Jessica is served "garden"
    And doorway "beta" served "garden" itself, taking no hop
    And doorway "alpha" was never contacted for that request
    When the household restores doorway "alpha"
    Then doorway "alpha" is serving again

  # Alpha has withdrawn its hosting contract, while beta's independently
  # refreshed registry still names it. This proof observes beta contacting alpha
  # once, alpha forwarding nothing onward, and Jessica receiving HTTP 404 within
  # 15 seconds. It makes no claim about where the final response bytes originated.
  # Request index.html explicitly: an extension-less root may instead answer
  # with the doorway's unrelated landing page. The recipient-side access log
  # proves alpha was contacted; its relay log proves it forwarded nothing onward.
  Scenario: Beta returns 404 after a stale relay without further forwarding
    Given the household stages the root "garden" as hosted by doorway "alpha" only
    And doorway "beta" still resolves "garden" to doorway "alpha" in its local registry
    When the household withdraws the contract for "garden" on doorway "alpha" before doorway "beta" next refreshes its registry
    Then doorway "alpha" has observed the withdrawal of its local hosting contract for "garden"
    And doorway "beta" still resolves "garden" to doorway "alpha" in its local registry
    And doorway "beta" has no contract to host "garden" locally
    When Jessica asks doorway "beta" for "garden" at its published entry document
    Then doorway "beta" forwarded the request to doorway "alpha" exactly once
    And doorway "alpha" received the forwarded request exactly once
    And doorway "alpha" forwarded nothing onward for that request
    And Jessica received HTTP 404 for "garden"
    And neither registered doorway served content for the lapsed site
    And Jessica received that refusal in less than 15 seconds
