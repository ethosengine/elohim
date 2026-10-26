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
  run log, a file of JSON lines on the developer's own device, as soon as the gate prints it.
  When the push starts, it prints the command that follows its run log; the developer's
  session (the terminal or coding agent that started the push) runs that command and sees
  one line per failure as it lands. Following is a choice: a session that does not follow
  still has the whole run log to read afterwards. Every record in the run log carries the time it
  was written, so the log itself shows how early each failure arrived. When the gate loop ends,
  the run log gets a closing result: gates passed, failed or interrupted. Later pre-push
  checks may still refuse a push after its gates pass. Nothing is ever decided
  from the run log: whether the gates allow the push is still decided by their exit statuses.

  Two later scenarios, about reusing a verdict and about an interrupted gate, use the words
  below. A closure is the inputs a gate's verdict depends on, including its recipe and environment.
  Its input root is a content identifier computed from that closure's canonical description.
  A derivation pairs the gate method's identifier with that input root and points to a saved
  verdict. The saved verdict, signed by this workspace's own key, is called an attestation.
  Ark is the device's local runner. Today it runs one gate, the orchestrator project's,
  which is why those scenarios name it; every other gate still runs directly and still
  reports its failures live. Ark is available when a current build of it is on the device;
  when it is not, the gate says so in one line and runs directly. Before starting checks,
  ark looks for an admissible saved verdict. With reuse enabled, a matching green verdict
  answers without running those checks again.

  Each gate's result is saved as a gate-verdict record in the run log. For an ark task,
  "Answered" means the gate finished or reused a verdict; a finished gate can still be red.
  "Unanswered" means no new verdict was supplied; its record instead gives a reason, when to come
  back and what to use meanwhile. "Use held" (written "use-held" in the log) points to an
  attestation already saved for these inputs. That pointer does not make it reusable or make
  the interrupted run pass. "Nothing" (written "nothing") explains why no verdict is
  available. "When to come back" names the event to wait for, here a rerun of the gate. I can read the
  full record in the run log even though the live follower prints a shorter line.

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
    And every other gate will pass
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
    And every other gate will pass
    When that gate prints its output
    And that gate exits with status <exit_status>
    And the remaining gates finish
    Then my push is <outcome>
    And the push outcome follows the exit status, not the printed output

    Examples:
      | output                                   | exit_status | outcome |
      | "test example_failure_output ... FAILED"  | 0           | allowed |
      | "custom checker finished"                | 1           | refused |

  Scenario: With reuse enabled, an unchanged green gate answers without running its checks again
    Given the orchestrator gate's newest saved verdict is green and signed by this workspace
    And the gate's inputs and repository tree are clean and unchanged since that verdict
    And this gate is allowed to reuse evidence and "GATE_EVIDENCE=reuse" is enabled
    And the local ark runner is available for that gate
    When that gate runs on a later push
    Then no build or test command starts for that gate
    And the gate output says "reused: derivation hit"
    And its gate-verdict record names the reused derivation's method and input root
    And that record says "Answered" with "hit" set to true
    And no new attestation is written: the verdict record points to the one already saved

  Scenario Outline: Interrupting a gate tells me what I can do next
    Given reuse is disabled for the orchestrator gate
    And <held_verdict> for that gate's unchanged inputs
    And the gate is running through the local ark runner
    When I interrupt that running task with <signal>
    Then the gate output tells me it was interrupted and has no new verdict
    And its gate-verdict record says "Unanswered" with reason "not-carried"
    And that record tells me to come back when I request a rerun
    And its meanwhile instruction is <meanwhile>
    When the remaining gates finish
    Then the completed run reads interrupted
    And its closing record retains that reason, come-back event and meanwhile instruction
    But the run is never shown as passed
    And no new verdict evidence is recorded for the interrupted gate

    Examples:
      | held_verdict                          | signal  | meanwhile                                                            |
      | a saved attestation exists            | SIGINT  | "use-held" with a command to find that attestation                     |
      | a saved attestation exists            | SIGTERM | "use-held" with a command to find that attestation                     |
      | no saved attestation exists           | SIGINT  | "nothing" because no verdict exists and an interrupted run is not one |
      | no saved attestation exists           | SIGTERM | "nothing" because no verdict exists and an interrupted run is not one |

  Scenario: A run whose process dies before closing reads interrupted
    Given the first gate has passed
    And a later gate is still running
    When the run's process ends before recording a closing result
    And my session reconnects to that run
    Then my session shows the run as interrupted
    And I can still see that the first gate passed
    But the run is never shown as passed
    And the missing closing result is not treated as permission to push
