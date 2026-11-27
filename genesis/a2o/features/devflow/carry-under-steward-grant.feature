@e2e @devflow @act:host @requires:epr-cli @local @wip @concern:sprint-lands-each-closure-once
Feature: A device carries a sibling's gate miss unattended, under its steward's grant, and answers for it
  Matthew develops on more than one of his own devices. When one device hands work over and its gate has
  no evidence yet, another of his devices can run that gate while no session on it was told to. Nothing
  runs unattended in his name unless he said so: his words grant it, for a short time, and the grant can
  be traced back to the moment he said them. Each run reads as the device's work, not as Matthew's own
  labour, and a live agent on that device answers for it if anyone asks what it is doing.

  Words used below:
  - A MISS is a gate result a device posts when it has no evidence yet for the exact inputs of a gate. It
    names the commit, the gate and the KEY: a fingerprint of everything the gate reads.
  - CARRYING a miss means another device runs that gate for that commit and records the verdict, so the
    device that posted the miss can reuse it instead of running the gate again.
  - The CARRIER is the background process on a device that carries misses with no session driving it. It
    runs under the device SUPERVISOR, the process that keeps a device's background services running.
  - A gate's RECIPE is the commands the gate runs, taken from the commit being checked; it is the first code
    from that commit that runs.
  - The LANDER is the hand-over step that moves work onto the shared integration branch. Before running a
    gate itself it looks for a verdict a sibling carried, and reuses it only if it can vouch for it.
  - A GRANT is Matthew's permission for one device's carrier to carry, recorded with his own words, the
    session and time he said them, the gates and branches it covers, and an expiry at most 24 hours ahead.
  - A TURN DIGEST is a fingerprint the device records of each thing Matthew says, when he says it; it lets
    a quoted grant be checked later without anything reading what he said.
  - The ANSWERING SESSION is a live agent session on the carrying device that acknowledges each carry
    before it starts and answers for it afterwards.
  - A STANDING device is one that Matthew's device roster binds to him.
  - A carry record names an ACTOR (the person whose labour it was), a STEWARD (the person who answers for
    it) and a RUNNER (the device that did it). Machine work names no actor, so it never counts as anyone's
    own labour.

  Background:
    Given two standing devices of Matthew, "laptop" and "workstation"
    And "workstation" runs a carrier under its device supervisor
    And an answering session is moored on "workstation" and reading each note addressed to it as it arrives

  Scenario: a granted carrier carries a miss with no session told, and the record names who did what
    Given Matthew said, in a session on "workstation", that its carrier may carry orchestrator gates on his
      hand-over branches for the next eight hours
    And that session recorded the grant with his words, and "workstation" holds a turn digest matching them
    And nobody tells any session on "workstation" about the coming hand-over
    When "laptop" hands over a branch and posts a miss for its orchestrator gate
    Then the answering session acknowledges the carry within 120 seconds
    And the carrier runs the gate and records a verdict for the exact commit and key the miss named
    And the carry claim and the verdict record the work as the device's, not Matthew's: they name no actor,
      name Matthew as the steward who answers for it, name "workstation" as the runner, and cite the grant
    And from "laptop", "epr flow carry show" answers who carried it, under which grant, until when, and how
      it ended

  Scenario: with no live answering session, the carrier declines instead of running alone
    Given a valid grant for the carrier on "workstation"
    But the answering session on "workstation" has stopped reading its notes
    When "laptop" posts a miss the grant covers
    Then no acknowledgement arrives within 120 seconds
    And the carrier records a declined carry with the reason "unanswered" and runs nothing
    And the declined carry expires the moment it is recorded, so nothing anywhere waits on it, including a
      device running a version of the tools that does not read the reason

  Scenario Outline: a miss outside the grant's bounds is declined before any of its code runs
    Given a valid grant for the carrier on "workstation" that covers orchestrator gates on hand-over branches
    When a miss arrives that <differs>
    Then the carrier records a declined carry with the reason "<reason>"
    And it declined at the step that checks it, before the gate's recipe from that commit ran

    Examples:
      | differs                                                        | reason            |
      | was posted by a device that does not stand for Matthew         | signer            |
      | names a gate project the grant does not cover                  | declined-by-grant |
      | names a commit not reachable from a branch the grant names     | ref               |
      | names a key that differs from the key the carrier recomputes   | key               |

  Scenario: a grant whose quote matches nothing Matthew said is void, and the lie is held against its writer
    Given a session on "workstation" recorded a grant quoting words that match no turn digest on that device
    When the answering session checks the grant before the first carry under it
    Then the check finds no matching turn and the carrier carries nothing under that grant
    And a revocation voids the grant
    And the breach is recorded naming the session that wrote the quote
    And feedback telling Matthew that a grant quoted words he did not say is captured for him

  Scenario: an expired grant carries nothing, and a lander does not reuse what it cannot vouch for
    Given a grant for the carrier on "workstation" that expired an hour ago
    When "laptop" posts a miss the grant would have covered
    Then the carrier records a declined carry with the reason "expired"
    And a lander that finds a verdict whose grant was expired when the verdict completed runs the gate itself
