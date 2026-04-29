@e2e @deployment @seeder @ingress @body-size-budget
Feature: Ingress body-size budget — diversity of peers, diversity of payloads
  As a protocol operator
  I want the seeder and the ingress to negotiate payload sizes
  So that thin-client peers (1 MB ingress) can host the protocol
  alongside backbone peers (500 MB ingress) without either side
  silently failing.

  The protocol can't assume every operator has a permissive ingress.
  A pastor running a doorway on a phone has a 1 MB ceiling. A family
  node has 500 MB headroom. The seeder must adapt to whichever it's
  pointing at — not require the operator to pre-tune nginx for the
  worst-case content load.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- Failure Regression: the recent alpha 413 incident ---

  @wip @regression
  Scenario: Without seeder chunking, default 1 MB ingress rejects bulk content
    Given the doorway ingress sits on the nginx-ingress 1 MB default
    And the seeder loads 3424 content items totalling ~3.5 MB
    When the seeder POSTs the bulk content to "/db/content/bulk"
    Then the ingress rejects the request with HTTP 413
    And the seeder pipeline aborts before path thumbnails are uploaded
    And alpha is left with content rows that reference blobs that never landed
    # Operational parameters: 3424 items, ~3.5 MB body, 1 MB ingress ceiling
    # Incident anchor: 2026-04-28 alpha re-seed (commit cb89d36f raised alpha to 100m)
    # Constraint: with default ingress, seeder ALWAYS fails — no operator with a
    # default-config doorway can seed without first tuning the ingress.

  @wip @regression
  Scenario: Without seeder chunking, default 1 MB ingress rejects HTML5 ZIPs
    Given the doorway ingress sits on the nginx-ingress 1 MB default
    And the seeder is asked to upload "evolution-of-trust" (6.75 MB ZIP)
    When the seeder POSTs the blob to "/blob/{hash}"
    Then the ingress rejects the request with HTTP 413
    And the HTML5-app blob never lands on storage
    # Operational parameters: 6.75 MB ZIP, 1 MB ingress ceiling
    # Constraint: HTML5 apps over 1 MB are universally unreachable to thin-client doorways.

  # --- Capability Proof: chunking + budget negotiation ---

  @wip
  Scenario: With chunked bulk POST, seeder succeeds against thin-client ingress
    Given the doorway ingress is configured with proxy-body-size "1m"
    And the seeder loads 3424 content items
    When the seeder POSTs the bulk content in chunks under the 1 MB budget
    Then every chunk is accepted with HTTP 2xx
    And the full 3424 items land on storage
    And the seeder pipeline proceeds to path thumbnails
    # Operational parameters: chunk_size <= 0.8 MB (20% headroom under 1 MB)
    # Informs: seeder configuration default for thin-client deployments

  @wip
  Scenario: Seeder discovers ingress budget before posting
    Given the doorway exposes its configured ingress body-size budget
    When the seeder asks the doorway for its payload budget
    Then the doorway responds with the effective proxy-body-size limit
    And the seeder sizes its chunks to fit within that budget minus 20% headroom
    # Observability gate: without this, the seeder either guesses small (slow)
    # or guesses large (413). Discovery makes the budget a first-class fact.

  # --- Capability Proof: archetype-tunable budgets ---

  @wip
  Scenario Outline: Different peer archetypes declare different body-size budgets
    Given a doorway is deployed on a "<archetype>" peer
    Then its configured proxy-body-size budget is "<budget>"
    And the seeder, when pointed at this doorway, chunks payloads under that budget

    Examples:
      | archetype          | budget |
      | thin-client        | 1m     |
      | laptop             | 10m    |
      | alpha-class        | 100m   |
      | family-node        | 500m   |
    # Operational parameters: thin-client = phone/IoT operator;
    #                         laptop      = personal dev;
    #                         alpha-class = doorway-alpha (current setting);
    #                         family-node = backbone storage peer
    # Informs: ingress manifest defaults per archetype, NodeCapabilities preset
    # Review after: any change to the largest legitimate single payload
    #               (today: 6.75 MB evolution-of-trust ZIP)
