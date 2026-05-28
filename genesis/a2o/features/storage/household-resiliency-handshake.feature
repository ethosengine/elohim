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
