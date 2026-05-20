@e2e @shefa @topology
Feature: Matthew sees real topology data after M1 substrate completion
  As Matthew, the household operator,
  I want to see real device, peer, reciprocity, and content data
  Across my topology surfaces
  So that I trust the substrate is actually doing the work it promises.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device

  @wip
  Scenario: Cluster page shows Matthew's device tile with real metrics
    When Matthew opens the cluster topology page at "/shefa/cluster"
    Then he sees at least one device tile labeled with his display name
    And the storage usage tile shows non-zero total bytes for his blob filesystem
    And the storage usage tile shows non-zero used bytes

  @wip
  Scenario: Peer topology page shows Terrance's household
    When Matthew opens the peer topology page at "/shefa/peers"
    Then he sees a peer-household-card for household-terrance
    And the peer-household-card displays Terrance's display name
    And the card displays Terrance's household as a connected peer

  @wip
  Scenario: Reciprocity page shows inflow from Terrance
    When Matthew opens the reciprocity page at "/shefa/reciprocity"
    Then he sees at least one inflow row whose counterparty is household-terrance
    And the committed bytes column shows a non-zero value for that row
    And the delivered bytes column shows a non-zero value once the cross-pod fetch has completed

  @wip
  Scenario: Manifesto chapter content viewer shows distribution badge
    When Matthew opens the M1 manifesto chapter resource in the content viewer
    Then he sees the distribution-badge component rendered in the header
    And the badge displays a replica-count value greater than zero
    And the badge shows at least one hosting peer in its peer list

  @wip
  Scenario: Manifesto chapter content viewer shows resilience snapshot
    When Matthew opens the M1 manifesto chapter resource in the content viewer
    Then he sees the resilience-snapshot component rendered side-by-side with the distribution-badge
    And the resilience snapshot displays the replica distribution across his connected peers

  @wip @compute-triptych @resilience-p1
  Scenario: Matthew's device tile shows free / used / stewarded compute breakdown
    When Matthew opens the cluster topology page at "/shefa/cluster"
    And he locates his laptop's device tile
    Then the device tile shows a compute triptych
    And the compute triptych "Free" cell has a non-empty byte value
    And the compute triptych "Used" cell has a non-empty byte value
    And the compute triptych "Stewarded" cell shows non-zero bytes when Matthew is hosting for another peer
