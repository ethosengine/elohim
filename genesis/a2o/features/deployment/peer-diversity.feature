@e2e @deployment @p2p @peer-diversity @requires:seeded-content
Feature: Peer Diversity — Operations Adapt to Device Constraints
  As the Elohim Protocol
  I want every operation to be aware of the device it runs on
  So that 7 billion humans on 20 billion devices can all participate
  according to what their hardware can offer

  The protocol doesn't get to wish people had better hardware. It serves
  them where they are. A phone with 3GB RAM is a full citizen. A family
  node with 64GB is a backbone. An IoT sensor with 4MB is a witness.
  Each finds its niche — like species in an ecosystem, not racks in a
  data center.

  # --- Capability Gradient ---

  @wip
  Scenario: Device portfolio covers the full capability gradient
    Given the device portfolio is loaded
    Then there should be at least 1 device at capability level 0
    And there should be at least 1 device at capability level 1
    And there should be at least 1 device at capability level 2
    And there should be at least 1 device at capability level 3
    And there should be at least 1 device at capability level 4
    And there should be at least 1 device at capability level 5

  # --- Memory Pressure (Backpressure) ---

  @wip @regression
  Scenario: Phone pauses sync during bulk content download
    Given device "2019 Android Phone" from the device portfolio
    # 3GB RAM, carrier-grade NAT, 4G
    And the device has 2 connected peers syncing 500 items each
    When the device downloads a 50MB learning path
    Then sync is paused during the download
    And peak memory stays within the device sync budget
    # Operational parameter: sync_budget = memoryGb * 0.05 = 150MB

  @wip @regression
  Scenario: K8s pod pauses sync during account import
    Given device "K8s Pod (256MB)" from the device portfolio
    # 256MB memory, 5 peers, 3400+ inventory items
    And the device has 5 connected peers syncing 3400 items each
    When an account package with 200 items is imported
    Then sync is paused for the duration of the import
    And the import completes without OOM
    # This is the scenario that discovered the backpressure need.

  @wip
  Scenario: Family node does not need backpressure for normal imports
    Given device "Family Node (Base)" from the device portfolio
    # 64GB RAM — sync overhead is negligible
    And the device has 10 connected peers syncing 5000 items each
    When an account package with 500 items is imported
    Then sync continues running during the import
    And no backpressure is triggered
    # 64GB node has headroom to sync and import concurrently.

  # --- Stewardship Boundaries ---

  @wip
  Scenario: Phone cannot accept stewardship requests
    Given device "2019 Android Phone" from the device portfolio
    Then the device capability level should be 2
    And the device cannot steward content

  @wip
  Scenario: Raspberry Pi can steward modest content volumes
    Given device "Raspberry Pi 4" from the device portfolio
    Then the device capability level should be 3
    And the device can steward content
    And the device should be always-on

  @wip
  Scenario: Family node is primary stewardship backbone
    Given device "Family Node (Base)" from the device portfolio
    Then the device capability level should be 5
    And the device can steward content
    And the device should be always-on
    And the device degradation mode should be "modular"

  # --- Network Resilience ---

  @wip
  Scenario: Carrier-grade NAT devices must use relay
    Given device "2019 Android Phone" from the device portfolio
    Then the device NAT type should be "carrier-grade-nat"

  @wip
  Scenario: Offline-first devices stream to paired nodes
    Given device "Environmental Sensor" from the device portfolio
    Then the device capability level should be 1
    And the device NAT type should be "offline-first"

  # --- Hardware Health Self-Awareness ---

  @wip
  Scenario: Family node reports full health surface
    Given device "Family Node (Base)" from the device portfolio
    Then the device should report health surfaces:
      | surface          |
      | smart            |
      | thermal          |
      | power            |
      | usb-enumeration  |
      | memory-ecc       |
      | fan-rpm          |

  @wip
  Scenario: Phone reports minimal health surface
    Given device "2019 Android Phone" from the device portfolio
    Then the device should report health surfaces:
      | surface         |
      | battery-health  |

  # --- Lifecycle and Circularity ---

  @wip
  Scenario: Modular devices schedule their own maintenance
    Given device "Family Node (Base)" from the device portfolio
    Then the device degradation mode should be "modular"
    And the device serviceability should be "full"

  @wip
  Scenario: Cliff-degradation devices need proactive replication
    Given device "2019 Android Phone" from the device portfolio
    Then the device degradation mode should be "cliff"
    And the device serviceability should be "none"

  # --- Attestation Surface ---

  @wip
  Scenario: Biometric fob provides strongest identity attestation
    Given device "Biometric Fob" from the device portfolio
    Then the device capability level should be 0
    And the device should support attestation:
      | capability              |
      | hardware-key-signing    |
      | biometric-identity      |

  @wip
  Scenario: Environmental sensor provides place-based attestation
    Given device "Environmental Sensor" from the device portfolio
    Then the device should support attestation:
      | capability                |
      | environmental-conditions  |
