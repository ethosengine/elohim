@e2e @resilience @resilience-card-lighting
Feature: The commitment-backed card lights for a healed household
  As a household deciding whether the network is holding what matters to me
  I want the "commitment-backed" count to reflect the provide commitments my
  household actually holds — not silently read zero because of how the
  classification was serialized
  So that the resilience card tells me the truth about who has promised to hold
  my content

  # Spec: genesis/docs/superpowers/specs/2026-06-13-non-commons-provide-commitments-design.md §11
  #       (Option A — uniform JSON-list classification + typed accessor)
  # Plan: genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md (Sprint 1)
  #
  # Verified root cause (operator cluster probes U1/U2, 2026-06-19): matthew's
  # provide row is present, active, provider == humans.agent_pub_key,
  # h_app_id='lamad', and his humans row is fully healed (agent_pub_key +
  # household_id both set). The card still read 0 ONLY because
  # rea_commitments.resource_classified_as was stored as the JSON-array string
  # ["content:commons"] while the snapshot join did scalar equality. The Rust
  # boundary pins the membership behavior deterministically:
  #   - elohim-storage/tests/household_resilience.rs  (D3b: array-wrapped scope counts)
  #   - elohim-storage/src/db/rea_commitments.rs       (classification accessor)
  #
  # Household floor (matthew/jessica/james) — no @requires:shem. The count
  # ceiling for the current 3-human / 2-household seed is 2 (matthew + adam);
  # "protected" (>=3 households) is structurally out of reach until more
  # households are seeded. This scenario asserts the dark→lit transition only.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @wip @regression
  Scenario: A healed household's commons provide commitment lights the card
    # The keystone: a healed humans row + an active commons provide commitment
    # renders a non-zero commitmentBackedCollectives. This is the dark→lit
    # transition Sprint 1 delivers — no reseed, no DNA change.
    Given household "matthew-home" has a healed steward identity
    And "matthew-home" stewards content "card-commons"
    And "matthew-home" holds an active "provide" commitment scoped "content:commons"
    When I request "/api/v1/resilience/card-commons/household"
    Then the response field "commitmentBackedCollectives" is at least 1

  @wip @regression
  Scenario: The card counts a commitment whose classification is a JSON list
    # The exact regression: classification stored as the JSON-array string
    # ["content:commons"] (the seeder / HTTP POST /commitments shape) must be
    # counted by membership, not missed by scalar equality. This is the precise
    # shape that read 0 on alpha (U1). Pinned at the Rust boundary; the human
    # experience is that the count is the same whether the row is bare or listed.
    Given household "matthew-home" has a healed steward identity
    And "matthew-home" stewards content "card-list"
    And "matthew-home" holds an active "provide" commitment classified as the list "[\"content:commons\"]"
    When I request "/api/v1/resilience/card-list/household"
    Then the response field "commitmentBackedCollectives" is at least 1
