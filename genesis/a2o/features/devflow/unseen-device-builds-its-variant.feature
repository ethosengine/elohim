@e2e @devflow @act:host @wip @concern:workspace-device-arrival
Feature: The first device of a new class makes the next device's arrival easier
  A steward brings a device whose hardware class has no native build on the network.
  A seed (a small set of portable programs the steward copies onto the device, enough
  to reach a peer and run the portable tier) provides an engine for portable programs
  and a transport for reaching peers.
  The portable tier performs the checks on which peers must agree, such as verifying
  content fingerprints. Native variants are builds fitted to particular hardware.
  The first device can participate before it has those faster native builds, then
  build and prove a variant that a later device of its class can reuse.
  A pin is the reference naming the variants a device may run.

  # Open in spec section 7.5: the seed's concrete members and size, and whether one
  # seed can also serve devices too small to build. This story uses a device able to build.
  # These steps are not wired yet; the seed and the steward's open ruling on how a device repairs and shares variants (H11) remain open.
  Background:
    Given the steward's new device can build software but has no compatible native variant available from peers
    And the steward has provided a seed that runs on it and can reach a peer

  Scenario: The first device participates through the portable tier
    When the new device starts its seed and runs the portable tier
    Then it exchanges content with a peer and verifies the content's fingerprint
    And both devices reach the same result on the shared validation checks
    And the new device participates without first building an optimized native variant

  Scenario: The first device builds and proves a variant for itself
    Given the new device is participating through its seed
    And it has a recipe and the named conformance suite for a native variant, with the required build inputs
    And the steward permits the first device's variants to be shared with the steward's own devices
    When it builds the variant and runs that suite on its own hardware
    Then the suite passes and its proof names the exact variant bytes and the suite it passed
    And the variant and its proof are readable by the steward's other devices and by no device outside the steward's own devices

  Scenario: A later device resolves the proven variant without repeating the first device's work
    Given the first device has built and proved a variant for its class
    And the steward permits the first device's variants to be shared with the steward's own devices
    And the steward's later device can reach a peer and resolve a pin that includes the proven variant
    And the later device observes that it meets the variant's requirements
    When the later device resolves that pin
    Then it fetches and verifies the proven variant and runs it
    And it needs neither a newly ported seed nor a local build of that variant
