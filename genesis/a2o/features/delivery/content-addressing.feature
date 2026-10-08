@e2e @delivery @concern:content-addressing @requires:doorway @requires:seeded-content @act:i
Feature: Content-addressed delivery
  As a learner visiting an HTML5 app
  I want content served by both slug and content address
  So that my browser cache stays valid across versions

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the doorway is connected to elohim-storage
    And an HTML5 app with slug "evolution-of-trust" is seeded
    And the app's blob hash is "bafkreiplaceholder"

  Scenario: Slug URL serves content with content address header
    When I request "/apps/evolution-of-trust/index.html"
    Then the response status is 200
    And the response includes header "X-Content-Address" with value "bafkreiplaceholder"
    And the response includes header "X-Content-Slug" with value "evolution-of-trust"

  Scenario: CID URL serves same content without slug lookup
    When I request "/apps/bafkreiplaceholder/index.html"
    Then the response status is 200
    And the response includes header "X-Content-Address" with value "bafkreiplaceholder"
    And the response body matches the slug URL response

  # Every seeded app is also an EPR — the protocol's named, versioned content record — and
  # `/epr-head/<slug>` answers its current version (its head) together with the head's
  # content address. That answer may also carry a witness of the election that chose the
  # head: when the winning declaration was made, its tier, and which declaration won. The
  # witness is reported beside the address, never inside it. The address is a hash of the
  # head's canonical bytes (the dag-cbor encoding the same URL serves on request), and those
  # bytes must not mention the election. If they did, two doorways holding the same head but
  # different records of its election would hand the browser two addresses for one thing,
  # and every cache keyed on the address would split.
  Scenario: Attaching the election witness does not move the head's address
    When the head of EPR "evolution-of-trust" is read from doorway "alpha"
    Then the address in the answer is computed from bytes that do not include the election witness

  @browser-only @wip
  Scenario: Service worker caches by content address
    Given the service worker is active
    When I navigate to "/apps/evolution-of-trust/index.html"
    Then the service worker caches the response under "/apps/bafkreiplaceholder/index.html"
    When I navigate to "/apps/bafkreiplaceholder/index.html"
    Then the response is served from the service worker cache

  @browser-only @wip
  Scenario: Re-seeded content with new CID invalidates old mapping
    Given the service worker has cached files under "/apps/bafkreiplaceholder/"
    When the app is re-seeded with blob hash "bafkreinewplaceholder"
    And I navigate to "/apps/evolution-of-trust/index.html"
    Then the response includes header "X-Content-Address" with value "bafkreinewplaceholder"
    And the old cache entries under "/apps/bafkreiplaceholder/" are invalidated
