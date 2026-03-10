@testnet @compute-allocation
Feature: Community compute allocation
  As Matthew, I have a distributed app to test.
  I request compute from my community, peers provision
  capacity, my test runs, and settlement happens.

  Background:
    Given human "Matthew" has a running steward node
    And elohim-storage is healthy at "http://localhost:8090"
    And compute mutual credit medium exists

  @e2e
  Scenario: Matthew requests compute from 5 community peers
    Given Matthew has a simulation requiring 5 peer nodes
    When he submits a ServiceRequest with budget 1800 cpu-seconds
    Then a provision envelope is emitted for each persona
    And 5 conductors are running within 30 seconds
    And compute-budget tracking is active

  @e2e
  Scenario: Compute settles after simulation completes
    Given 5 conductors are running for Matthew's simulation
    When the simulation workload completes
    Then a settle envelope is emitted for each persona
    And each EconomicEvent contains cpu-seconds and memory-mb
    And the total spend is within the 1800 cpu-second budget
    And the compute summary appears in the test report

  @e2e @rea
  Scenario: Compute commitments are persisted as REA records
    Given Matthew has a simulation requiring 5 peer nodes
    When he submits a ServiceRequest with budget 1800 cpu-seconds
    Then a 'take' commitment exists for Matthew with 1800 cpu-seconds
    And a 'give' commitment exists for each of the 5 personas
    And all commitments reference the CPU mutual credit medium

  @e2e @rea
  Scenario: Settlement produces paired EconomicEvents
    Given 5 conductors are running for Matthew's simulation
    When the simulation workload completes
    Then each persona has a 'deliver-service' EconomicEvent
    And Matthew has a 'take' EconomicEvent for the total
    And each event fulfills its corresponding commitment
    And each event has resourceQuantity in cpu-seconds
    And each event has effortQuantity in megabytes
    And all persona commitments are marked 'fulfilled'

  @e2e @circuit-breaker @wip
  Scenario: Budget exceeded triggers graceful degradation
    Given 5 conductors are running for Matthew's simulation
    And one persona is configured with a 60 cpu-second budget
    When that persona exceeds its budget
    Then it receives SIGTERM with a budget-exceeded envelope
    And the remaining 4 conductors continue
    And settlement records the partial delivery
