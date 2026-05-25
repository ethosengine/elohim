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
