@concern:dataplane-convergence @device-declared-target @offline
Feature: A device declares what it follows and runs
  Matthew can set this device's intended runtime while disconnected.
  The declaration supplies the runtime's watched configuration and can restore it from disk.
  Matthew is told about a newer tool before choosing to switch his pinned session.

  A pin can name a set of variants: builds of the same tool for different hardware.
  A variant's requirements say which CPU instructions and runtime libraries it needs.
  The device checks those requirements against what it observes locally.

  # Variant adoption steps are not wired yet; both variants are held locally so this
  # scenario can exercise the declared target while disconnected.
  @wip @concern:sprint-lands-each-closure-once @concern:dataplane-convergence
  Scenario: The older laptop runs the compatible variant of its declared pin
    Given the older laptop observes an x86-64-v2 CPU with AVX but without AVX2
    And it holds verified v2 and v3 variants of its declared tool pin while disconnected
    And it meets both variants' runtime library requirements
    When the older laptop applies its declared target
    Then it refuses the v3 variant before launch with reason "unmet" on axis "isa.x86"
    And the refusal names the required and observed CPU features, including the missing AVX2
    And it starts the v2 variant and records that variant's content fingerprint locally
    And its declared pin is unchanged

  Scenario: A device declares what it follows and runs, offline, and the runtime reconciles to it
    Given an isolated device with the committed workspace runtime base and no network
    When the device declares "bin:epr" in "apply" mode
    And the device declares "runtime:coordinators:elohim:workspace" in "observe" mode
    And the device declares that it runs "epr"
    Then the device's watched follow line matches its declared target
    And each declaration supersedes the previous manifest by CID
    When the device's watched configuration is lost and the device applies its target again
    Then the device's watched follow line matches its declared target
    When a newer landed tool is available to the pinned session
    Then the session receives one notice naming the old and new tool CIDs and a command to switch
    And the session's recorded tool pin is unchanged
    When the device's private manifest chain is removed
    Then the device returns to the committed base with epr and brit followed in apply mode
