@e2e @resilience @resilience-card-lighting @concern:reconcile-inventory @dataplane @act:i
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

  # --- Second root cause (2026-07-17): provider namespace mismatch ---
  # A follow-on investigation into the same "card stays dark" complaint found a
  # SECOND, independent cause behind D3b: some provide commitments were
  # authored with `provider` set to a libp2p transport id (12D3KooW…) instead
  # of the steward's agent_cid (uhCAk…). The join `humans.agent_pub_key ==
  # rea_commitments.provider` is namespace-strict — a transport-id provider
  # silently never joins, and the row drops out of the count with no error.
  # Fix: the provide author now resolves an agent_cid (session key, falling
  # back to its own conductor cell key) and SKIPS authoring (with a metric)
  # rather than write a provider it cannot express as an agent_cid.

  @wip @regression
  Scenario: Transport-id provider commitments never light the household card
    Given a provide commitment whose provider is a libp2p transport id "12D3KooW…"
    And a healed humans row with agent_pub_key "uhCAk…" for the same steward
    When the household resilience snapshot is computed for commons content
    Then the commitment-backed collectives count excludes that commitment
    # Constraint: the humans.agent_pub_key == rea_commitments.provider join is
    # namespace-strict (uhCAk agent_cid only); a transport-id provider silently
    # never joins — no error surfaces. Discovered 2026-07-17: all three M/J/J
    # alpha pods ran the provide-loop with a 12D3Koo self_cid.

  @wip
  Scenario: Provide author skips rather than writing an unjoinable provider
    Given a storage node with no local session and no resolvable agent key
    When the provide-loop tick attempts to author a commons provide commitment
    Then no commitment row is written
    And the provider-unresolved skip signal increments
    And the provide latch returns to needs-commitment so the next tick retries
    # Operational parameters: 60s tick cadence; WARN once per process then
    # DEBUG; PROVIDE_PROVIDER_UNRESOLVED metric counts every skip.
    # Diagnostic constraint: a SILENT provide-loop (zero "provide author tick"
    # log lines) means an EMPTY desired set (pin starvation) — not a broken
    # loop. The only acquisition_pins writer is POST /api/v1/pins; nothing
    # auto-pins seeded content.
