@e2e @storage @wip
Feature: Households commit to back each other up
  As a family in a small circle of people we trust
  I want our home to keep a copy of another family's memories, and they keep a copy of ours
  So that if our house burns down, the photos and the plans we made together still survive

  Two families promise to hold each other's content. When both sides keep their
  word, each family sees their shared content as safe. When one side never keeps
  its end, the family that did its part is never penalized — the network simply
  and gently names the family that didn't reciprocate.

  Background:
    Given the Smith family runs a small server at home
    And the Garcia family runs a small server at home
    And both families have signed in as the steward of their own home

  Scenario: Both families keep their promise; both see their memories protected
    When Maria of the Smith family promises to keep 50 GB of the Garcia family's memories safe
    And Carlos of the Garcia family promises to keep 50 GB of the Smith family's memories safe within 14 days
    Then both families see their shared content marked "protected"
    And neither family is shown an imbalance notice
    And a check of the two families shows them as fully matched, each holding up their end

  Scenario: One family never returns the promise; the network names them, gently
    When Maria of the Smith family promises to keep 50 GB of the Garcia family's memories safe
    And 15 days pass without the Garcia family promising anything back
    Then the network raises a notice that the Garcia family has not reciprocated
    And the Garcia family's good standing dips a little, in a way they can recover from
    And Maria's promise still holds — she did nothing wrong; the Garcias simply never gave back

  @wip @regression
  Scenario: A scoped promise still matches the content it was made for
    Given Maria's promise to the Garcia family covers only their content records
    When the Garcia server announces a new family recipe document
    Then Maria's server recognizes the recipe as covered by her promise
    And fetches a protective copy without being asked
    # Constraint (storage-tier review 2026-06-04, finding #2): the gossip hint's kind and
    # the promise's scope filter MUST share ONE vocabulary (the EprKind schema enum, e.g.
    # "Content") — the producer once spoke content_format ("markdown") while the scope
    # spoke EprKind names, so every scoped promise silently fetched nothing.
    # Fixture rule: never hand-pick identical strings on both sides of a wire; fixtures
    # come from the schema enum (hand-picked pairs masked this for a whole sprint).
    # Guarded by: inventory_broadcaster::gathered_hint_scores_high_against_schema_valid_scope

  @wip @regression
  Scenario: A friend's contribution to our family album is still backed up by our steward
    Given a friend from another household authored a story in the Smith family album
    And the story lives on the Smith family's server
    When the Smith server announces its content to the circle
    Then the Garcia server recognizes the story as Smith-family content it promised to protect
    And keeps a protective copy, even though its author lives elsewhere
    # Constraint (storage-tier review 2026-06-04, finding #3): replication matching keys on
    # the CUSTODY dwelling (peer_id → agent_cid → dwelling_hub_id via peer_identity_bindings
    # — the spec's "build it once" reusable chain), never on authorship. Author-keyed hints
    # silently broke all cross-household stewardship.
    # Guarded by: inventory_broadcaster::gather_hints_recipient_hub_from_advertising_peer_not_author
