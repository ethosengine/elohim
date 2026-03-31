Feature: Content-addressed delivery
  As a learner visiting an HTML5 app
  I want content served by both slug and content address
  So that my browser cache stays valid across versions

  Background:
    Given the doorway is connected to elohim-storage
    And an HTML5 app with slug "evolution-of-trust" is seeded
    And the app's blob hash is "sha256-abc123"

  Scenario: Slug URL serves content with content address header
    When I request "/apps/evolution-of-trust/index.html"
    Then the response status is 200
    And the response includes header "X-Content-Address" with value "sha256-abc123"
    And the response includes header "X-Content-Slug" with value "evolution-of-trust"

  Scenario: CID URL serves same content without slug lookup
    When I request "/apps/sha256-abc123/index.html"
    Then the response status is 200
    And the response includes header "X-Content-Address" with value "sha256-abc123"
    And the response body matches the slug URL response

  Scenario: Service worker caches by content address
    Given the service worker is active
    When I navigate to "/apps/evolution-of-trust/index.html"
    Then the service worker caches the response under "/apps/sha256-abc123/index.html"
    When I navigate to "/apps/sha256-abc123/index.html"
    Then the response is served from the service worker cache

  Scenario: Re-seeded content with new CID invalidates old mapping
    Given the service worker has cached files under "/apps/sha256-abc123/"
    When the app is re-seeded with blob hash "sha256-def456"
    And I navigate to "/apps/evolution-of-trust/index.html"
    Then the response includes header "X-Content-Address" with value "sha256-def456"
    And the old cache entries under "/apps/sha256-abc123/" are invalidated
