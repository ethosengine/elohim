# A gate's failures reach the person pushing while the gates are still running.
# Habit: genesis/orchestrator/.epr-meta/gate-failures-arrive-live.habit.md. Spec:
# genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md.
#
# The tags are suite-routing labels, not behaviour: @e2e and @devflow route this file into
# the devflow suite; @act:host says it runs on the host alone with no mesh peer; the
# @concern: tag joins it to the habit that claims it. @wip until
# steps/devflow/gate-failures-arrive-live.steps.ts exists.

@e2e @devflow @act:host @wip @concern:gate-failures-arrive-live
Feature: Gate failures reach the developer while checks continue

  Before a push lands, the push runs a set of local checks called gates, one after another.
  Each gate builds and tests one project. Without this, a gate that fails at minute twelve
  of a thirty-minute push says nothing until the push ends, so the developer waits for the
  whole run before starting to look for the cause. A gate run now writes each failure to a
  run log, a file on the developer's own device, as soon as the gate prints it. The
  developer's session (the terminal or coding agent that started the push) follows that file
  and sees one line per failure as it lands. Every record in the run log carries the time it
  was written, so the log itself shows how early each failure arrived. When the last gate finishes, the run log gets a
  closing result: one final record saying the run passed or failed. Nothing is ever decided
  from the run log: whether the push may proceed is still decided only by each gate's exit
  status.

  Background:
    Given I am pushing a change whose local checks (gates) run one after another
    And my session is following that push's run log

  Scenario: A failure reaches me while later gates still run
    Given the first gate runs a cargo test named "rejects_stale_evidence"
    And a later gate will remain running until I let it finish
    When the first gate prints "test rejects_stale_evidence ... FAILED"
    And the first gate exits with status 1
    Then my session receives the failure for "rejects_stale_evidence"
    And the later gate starts and remains running
    And the recorded time to my first failure is earlier than the end of the run
    When I let the later gate finish
    Then the push is refused
    And the completed run lists the failure for "rejects_stale_evidence"

  Scenario: Every failure reaches me and remains listed when the run ends
    Given the first gate runs cargo tests named "rejects_stale_evidence" and "checks_signatures"
    And a later gate runs a cargo test named "preserves_exit_status"
    And every other required gate will pass
    When the first gate prints "test rejects_stale_evidence ... FAILED"
    Then my session receives the failure for "rejects_stale_evidence"
    When the first gate prints "test checks_signatures ... FAILED"
    Then my session receives the failure for "checks_signatures"
    When the first gate exits with status 1
    And the later gate prints "test preserves_exit_status ... FAILED"
    Then my session receives the failure for "preserves_exit_status"
    When the later gate exits with status 1
    And the remaining gates finish
    Then the completed run lists all three failures with their gates
    And the push is refused

  Scenario Outline: The exit status decides whether my push proceeds
    Given a gate is testing output handling and will print <output>
    And every other required gate will pass
    When that gate prints its output
    And that gate exits with status <exit_status>
    And the remaining gates finish
    Then my push is <outcome>
    And the push outcome follows the exit status, not the printed output

    Examples:
      | output                                   | exit_status | outcome |
      | "test example_failure_output ... FAILED"  | 0           | allowed |
      | "custom checker finished"                | 1           | refused |

  Scenario: A run whose process dies before closing reads interrupted
    Given the first gate has passed
    And a later gate is still running
    When the run's process ends before recording a closing result
    And my session reconnects to that run
    Then my session shows the run as interrupted
    And I can still see that the first gate passed
    But the run is never shown as passed
    And the missing closing result is not treated as permission to push
