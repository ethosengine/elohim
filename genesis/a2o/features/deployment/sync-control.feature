@e2e @deployment @sync-control @peer-diversity
Feature: User-controlled sync — mobile, wifi, and the operator's data plan
  As a human running a peer on a metered connection
  I want to control when sync runs and over which network
  So that participating in the protocol doesn't bankrupt me on
  cellular overage charges, and I can opt back in when I'm
  somewhere with cheap bandwidth.

  Automatic backpressure (memory, drain, import) is already covered
  by features/deployment/p2p-validation.feature and the auto-pause
  scenarios in peer-diversity.feature. This file is about the OTHER
  axis: the human's deliberate choice. Three modes:

  - sync             — sync continuously regardless of network
  - paused           — operator has put their peer to sleep
  - wifi-only        — sync runs on unmetered networks, pauses on cellular

  The protocol cannot quietly burn somebody's data plan to keep its
  DHT warm. A pastor on a phone is a full citizen — but only if
  participation has a "not right now" button.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- Failure Regression: silent cellular sync ---

  @wip @regression
  Scenario: Without sync mode control, mobile device burns cellular data
    Given device "2019 Android Phone" from the device portfolio
    # 3GB RAM, carrier-grade NAT, 4G cellular as the only network
    And the device is currently on cellular (not wifi)
    And the peer has no sync-mode preference set
    When 5 connected peers gossip 50 MB of new content over an hour
    Then the device transfers ~50 MB over cellular without user consent
    # Operational parameter: 50 MB / hour cellular = ~36 GB / month idle
    # Constraint: any default that syncs over cellular is hostile to phone operators.
    # Discovery anchor: 2026-04-29 brainstorm — the protocol must default to wifi-only
    # for mobile archetypes, otherwise adoption capped to wifi-only households.

  # --- Capability Proof: the three modes ---

  @wip
  Scenario: Operator pauses sync explicitly
    Given the peer's sync mode is "sync"
    When the operator sets sync mode to "paused"
    Then ongoing sync cycles complete their current chunk and stop
    And no new sync cycles start until the mode changes
    And the P2P status reports syncMode "paused"
    # The pause is sticky — it survives restart and network changes.

  @wip
  Scenario: Operator resumes sync after pause
    Given the peer's sync mode is "paused"
    And there are 200 pending DHT updates from connected peers
    When the operator sets sync mode to "sync"
    Then sync cycles resume on the next interval
    And the 200 pending updates are absorbed
    # Pause does not lose work — it defers it.

  @wip
  Scenario: Wifi-only mode pauses sync when cellular is the active network
    Given device "2019 Android Phone" with sync mode "wifi-only"
    And the device is currently on cellular
    Then sync cycles do not run
    And the P2P status reports syncMode "wifi-only" and networkClass "cellular"
    # Detection: device emits networkClass on heartbeat;
    # peer sync loop treats wifi-only + cellular as effective-paused.

  @wip
  Scenario: Wifi-only mode resumes sync when device joins wifi
    Given device "2019 Android Phone" with sync mode "wifi-only"
    And the device is currently on cellular with sync paused
    When the device joins a wifi network
    Then sync cycles resume within 30 seconds
    And queued updates from peers begin draining
    # Operational parameter: resume detection ≤ 30s after network change
    # The user does nothing — the device knows its own network class.

  # --- Capability Proof: archetype-tunable defaults ---

  @wip
  Scenario Outline: Each archetype has a sensible default sync mode
    Given device "<device>" from the device portfolio
    Then the device's default sync mode should be "<default>"
    And the operator can override the default at any time

    Examples:
      | device                 | default    |
      | 2019 Android Phone     | wifi-only  |
      | Raspberry Pi 4         | sync       |
      | Family Node (Base)     | sync       |
      | Environmental Sensor   | wifi-only  |
    # Operational parameters: defaults respect data-plan economics.
    # Phones and IoT sensors default wifi-only — cellular is unmetered only by exception.
    # Family nodes and Pis default to always-sync — they're on home wifi by definition.
    # Informs: NodeCapabilities preset, settings-pane initial state per archetype.

  # --- Observability: the operator can see what's happening ---

  @wip
  Scenario: Sync state is visible in the operator dashboard
    Given the operator opens their device's sync settings
    Then the current sync mode is shown
    And the last successful sync timestamp is shown
    And if mode is "wifi-only", the current network class is shown
    # Without visibility, "is my peer participating?" requires a CLI.

  @wip
  Scenario: Sync mode transitions are logged for auditability
    Given the peer's sync mode has changed 4 times in the last 24 hours
    When the operator queries the sync mode history
    Then the response shows each transition with timestamp and trigger
    # Triggers: "user", "network-change", "low-battery", "system"
    # Observability gate: operators learn whether their phone is "actually paused"
    # vs. silently failing to detect wifi.
