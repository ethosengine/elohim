@e2e @deployment @modeling @human-device-mapping
Feature: Human × Device × Deployment mapping is internally consistent
  As a protocol operator deploying personas to K8s
  I want every deployment record to name a real human and a real device archetype
  So that persona personas, hardware expectations, and deployment topology
  tell the same story — no dangling references, no forgotten humans when a
  manifest is updated, no pod sized for hardware that doesn't exist.

  The mapping lives in three files:

    genesis/data/humans/humans.json              — who the persona is
    genesis/data/devices/devices.json            — what hardware they run on
    genesis/orchestrator/data/deployments.json   — where K8s deploys them

  Each deployment record links a humanId to a deviceArchetype. The scenarios
  below are executed against the generated JSON artifacts — the source of
  truth for `genesis/data/*` markdown frontmatter.

  # --- Cross-reference integrity ---

  @wip
  Scenario: Every deployed human resolves in the humans registry
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    And the human registry from "genesis/data/humans/humans.json"
    Then every deployment record humanId exists in the human registry
    # No dangling humanIds — every record projects a real persona.

  @wip
  Scenario: Every deployed human names a real device archetype
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    And the device portfolio from "genesis/data/devices/devices.json"
    Then every deployment record deviceArchetype exists in the device portfolio
    # The pod is sized for a specific hardware shape that has a name, a
    # capability level, and a degradation mode in the portfolio.

  # --- Pattern dispatch ---

  @wip
  Scenario: Legacy-pattern records reference a real template file
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    When the deployment record pattern is "legacy"
    Then the record's template path exists on disk
    And the record declares edgenode memory and CPU requests and limits
    # Legacy humans render through a shared template + per-human sed args.

  @wip
  Scenario: Consolidated-pattern records reference a real manifest file
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    When the deployment record pattern is "consolidated"
    Then the record's manifest path exists on disk
    # Consolidated humans have a hand-crafted manifest (Adam is the current
    # reference impl while elohim-node 0.0.0.0 binding is unverified).

  # --- Resource alignment ---

  @wip
  Scenario: Pod resources fit within the device archetype envelope
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    And the device portfolio from "genesis/data/devices/devices.json"
    When the deployment record pattern is "legacy"
    Then the edgenode memory limit is no greater than the device's memoryGb
    And the edgenode CPU limit is no greater than the device's cpuCores in milliCPU
    # Terrance on a Chromebook (4GB) can't request a 6Gi limit. Ensures pods
    # declare what their archetype can actually deliver.

  @wip
  Scenario: Level-5 humans declare a level-5-capable archetype
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    And the device portfolio from "genesis/data/devices/devices.json"
    When the deployment record role is "manager" or "firstman"
    Then the referenced deviceArchetype has capabilityLevel of at least 4
    # Matthew (manager) and Adam (firstman) are household backbones — they
    # need storage + inference capability at minimum.

  # --- Node affinity vocabulary ---

  @wip
  Scenario: Every nodeTypes entry is in the allowed cluster vocabulary
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    Then every nodeTypes value is one of:
      | nodeType    |
      | performance |
      | operations  |
      | edge        |
      | remote      |
    # Catches typos like "performence" that would leave pods unschedulable.

  # --- Completeness ---

  Scenario: The six protocol humans are all represented in the deployment registry
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    Then the registry contains a record for each name:
      | name    |
      | adam    |
      | matthew |
      | jessica |
      | pete    |
      | terrance |
      | frank   |
    # Any new human added to K8s must be declared here — the Jenkinsfile
    # reads this file as the source of truth for which StatefulSets to apply.

  @wip
  Scenario: humanId matches the convention "human-<humanLabel>"
    Given the deployment registry from "genesis/orchestrator/data/deployments.json"
    Then for every record the humanId equals "human-" concatenated with humanLabel
    # Convention check — prevents mismatched labels between K8s resources
    # and persona records (e.g. elohim-human label vs the Human entry id).
