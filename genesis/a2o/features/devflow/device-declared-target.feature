@concern:dataplane-convergence @device-declared-target @offline
Feature: A device declares what it follows and runs
  Matthew can set this device's intended runtime while disconnected.
  The declaration supplies the runtime's watched configuration and can restore it from disk.
  Matthew is told about a newer tool before choosing to switch his pinned session.

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
