@e2e @deployment @hub-topology @wip @act:host
Feature: Hub Topology — Substrate Aggregation as Realm-Specific Implementations
  As the Elohim Protocol
  I want hub-and-spoke topology modeled as a narrow abstract Hub interface
  with intentionally separate HouseholdHub and CollectiveHub implementations
  So that substrate scaling reaches billions of humans through trusted hosting
  without collapsing realm-specific governance into config flags

  A hub is a substrate aggregation point — a Tier 3 (or smaller) node that
  carries spokes (phones, laptops, hosted-account members) on their behalf.
  Households are the primary case. Collectives (church basements, schools,
  community centers) follow the same plane but with different governance
  considerations — consent for adding spokes, institutional visibility
  defaults, community-vote eviction. They are intentionally separate
  implementations, mirroring how human-elohim, household-elohim, and
  collective-elohim are separate specializations rather than one parametric
  type with a "realm" parameter.

  All hubs share a narrow substrate-level interface: stewards (humans
  with authority over the hub), devices (the hardware composition),
  topology (how spokes attach), connectivity profile, WAN profile, and
  inclusion role (whether external spokes are carried). The realm-
  specific implementations carry governance contracts: consent flows,
  visibility defaults, custodial relationships, economic posture.

  These scenarios are @wip because real Tier 3 hardware does not yet
  exist for substrate-scale validation. The first phase locks the hub
  archetypes in as fixtures so design intent is preserved; a follow-up
  sprint adds a simulation harness (virtual hubs + mock spokes with
  declared connectivity profiles).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- Hub Portfolio Scope ---

  @wip
  Scenario: Hub portfolio includes HouseholdHub and CollectiveHub implementations
    Given the hub portfolio is loaded
    Then there should be at least 1 hub of type "HouseholdHub"
    And there should be at least 1 hub of type "CollectiveHub"
    And every hub should declare its stewards, devices, and governance contract
    And the abstract Hub interface should never be instantiated directly

  # --- HouseholdHub Archetypes ---

  @wip
  Scenario: Phone-only solo household participates without owning a hub
    Given hub archetype "Solo Phone-Only" from the hub portfolio
    Then the hub type should be "HouseholdHub"
    And the hub should declare 1 steward
    And the hub should declare 0 Tier 3 nodes
    And the hub stage distribution should include "hosted-user"
    And substrate participation should be mediated by doorway projection

  @wip
  Scenario: Young family with one Tier 3 is the canonical Stage-4 household
    Given hub archetype "Young Family with Family Node" from the hub portfolio
    Then the hub type should be "HouseholdHub"
    And the hub should declare 2 to 4 stewards
    And the hub should include device "Family Node (Base)"
    And the hub connectivity profile should be "always-on"
    And the hub governance model should be "steward-consent"

  @wip
  Scenario: Multi-generational household carries a hosted-only grandmother
    Given hub archetype "Multi-Gen Household" from the hub portfolio
    Then the hub type should be "HouseholdHub"
    And the hub should declare at least 4 stewards across age categories
    And the hub should include device "Family Node (Base)"
    And the hub should host custodial keys for at least 1 hosted-only steward
    And the hub stage distribution should span "hosted-user" through "node-operator"
    # The grandma case — substrate-level rights guaranteed without hardware ownership.

  @wip
  Scenario: Extended family with two hubs provides mutual relational backup
    Given hub archetype "Extended Family Two Hubs" from the hub portfolio
    Then the hub type should be "HouseholdHub"
    And the archetype should declare 2 paired hubs
    And each paired hub should hold backup shards for the other
    And cross-household substrate routing should be exercised

  # --- CollectiveHub Archetypes ---

  @wip
  Scenario: Church basement hub carries a hundred spokes through one Tier 3
    Given hub archetype "Church Basement Hub" from the hub portfolio
    Then the hub type should be "CollectiveHub"
    And the hub should include device "Family Node Extended"
    And the hub should declare at least 100 spokes
    And the hub governance model should be "delegated-consent"
    And the hub visibility default should be "institutional"
    # Concrete instantiation of the inclusion math — one Tier 3, hundreds of humans.

  @wip
  Scenario: Refugee camp shared hub operates under constrained bandwidth
    Given hub archetype "Refugee Camp Shared Hub" from the hub portfolio
    Then the hub type should be "CollectiveHub"
    And the hub WAN profile should include "satellite" or "lora" fallback
    And the hub connectivity profile should be "intermittent"
    And the hub should declare offline-first sync semantics

  # --- Realm Separation: Governance Differs in Shape, Not Settings ---

  @wip
  Scenario: HouseholdHub adds spokes through trust without explicit consent
    Given hub archetype "Multi-Gen Household" from the hub portfolio
    When a steward adds a new family member spoke
    Then the spoke is provisioned without a community-vote event
    And the spoke inherits household-economy and household-visibility defaults

  @wip
  Scenario: CollectiveHub adds spokes through governance consent
    Given hub archetype "Church Basement Hub" from the hub portfolio
    When a steward adds a new congregant spoke
    Then the spoke addition produces a community-governance event
    And the spoke inherits institutional visibility defaults
    And other stewards can recall the addition through the collective's governance model

  # --- Constitutional Rule: Hardware Must Be Accessible to Stewards ---

  @wip @constitutional
  Scenario: Hub hardware must be physically and operationally accessible to its stewards
    Given any hub from the hub portfolio
    Then every device in the hub should declare an access path for its stewards
    And the access path should be inspectable, modifiable, and retrievable
    And the access path should not depend on a third party who is not a steward
    # Elohim constitutional rule: hardware that carries a steward's life is theirs to reach.

  @wip @constitutional
  Scenario: Stewards can quarantine hardware that becomes inaccessible to them
    Given a hub with a device that has become inaccessible to its stewards
    When the stewards invoke quarantine through the hub's governance model
    Then the device is marked quarantined in the substrate
    And the device's stewardship duties are reassigned through normal contracts
    And data on the device is treated as unrecoverable until access is restored
    # Inaccessibility is a constitutional violation — stewards retain eviction power.

  @wip @constitutional
  Scenario: Stewards can evict hardware that refuses access entirely
    Given a hub with a device whose access path has been revoked by a non-steward party
    When the stewards invoke eviction through the hub's governance model
    Then the device is removed from the hub's declared composition
    And the substrate ceases routing through the evicted device
    And the eviction event is recorded as a constitutional notarization
    # Revocation by anyone other than a steward is grounds for full removal.

  # --- Degradation Behavior ---

  @wip
  Scenario: Hub remains usable through doorway projection when its Tier 3 goes offline
    Given hub archetype "Young Family with Family Node" from the hub portfolio
    When the family node becomes unreachable
    Then spokes can still participate through doorway projection
    And the hub is marked degraded but not failed
    And substrate-level rights of stewards are preserved through the degradation
