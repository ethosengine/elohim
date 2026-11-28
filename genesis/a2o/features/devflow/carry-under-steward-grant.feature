@e2e @devflow @act:host @requires:epr-cli @local @wip @concern:sprint-lands-each-closure-once
Feature: A device carries a sibling's gate miss unattended, under its steward's grant, and answers for it
  Matthew develops on more than one of his own devices. When one device hands work over and its gate has
  no evidence yet, another of his devices can run that gate while no session on it was told to. Nothing
  runs unattended in his name unless he said so: his words grant it, for a short time, and the grant can
  be traced back to the session in which those words were recorded. Each run reads as the device's work, not as Matthew's own
  labour, and a live agent on that device acknowledges each carry before it starts and, asked while it runs,
  says what the carrier is doing.

  Words used below:
  - A MISS is a gate result a device posts when it has no evidence yet for the exact inputs of a gate. It
    names the commit, the gate and the KEY: a fingerprint of everything the gate reads.
  - A VERDICT is the recorded outcome of one gate run for one commit and key: passed or failed.
  - CARRYING a miss means another device runs that gate for that commit and records the verdict, so the
    device that posted the miss can reuse it instead of running the gate again. When a carry is declined,
    or no device carries the miss, nothing is lost: the device that posted it runs the gate itself when it
    lands, as it would with no carrier at all.
  - The CARRIER is the background process on a device that carries misses with no session driving it. It
    runs under the device SUPERVISOR, the process that keeps a device's background services running.
  - A GATE is one named check a change must pass before it lands (the "orchestrator" gate, for instance, runs
    the tests of the build orchestrator). A gate's RECIPE is the commands the gate runs, taken from the commit
    being checked. The recipe is the first code from that commit that runs, so a carry declined before its
    recipe ran none of that commit's code.
  - The LANDER is the hand-over step that moves work onto the shared integration branch. Before running a
    gate itself it looks for a verdict a sibling carried, and reuses it only if it can vouch for it.
  - A HAND-OVER BRANCH is a branch one of Matthew's devices pushes so another device can pick the work up;
    its name starts with "self/".
  - A GRANT is Matthew's permission for one device's carrier to carry, recorded with his own words, the
    session and time it was recorded, the gates and branches it covers, and an expiry at most 24 hours
    ahead. A grant cannot cover the shared branches "dev" or "main": landing work there is the lander's. It can be recorded only in the main conversation of a session on the device that will carry,
    not by a helper agent that session started. The carrying device keeps it as two things that must agree:
    the CLAUSE, which holds his words, and the PERMISSION the carrier reads (which device, which gates and
    branches, until when). Before the first carry under any grant, the answering session checks that the two
    agree. His words stay on that device: what other devices receive is a record of the permission, signed
    by the carrying device's key, that points at the clause without carrying the words. He can revoke a grant before it expires, in the same
    way he gave it.
  - The ANSWERING SESSION is a live agent session on the carrying device that acknowledges each carry
    before it starts and answers for it afterwards. The carrier waits 120 seconds for that acknowledgement;
    with none, it declines the carry as "unanswered". A session is MOORED on a device when it has registered
    there and receives the notes addressed to it.
  - A session's MAIN CONVERSATION is the one Matthew types into; a HELPER AGENT is another agent that
    session starts to do part of its work, and it does not talk with him directly.
  - A FEEDBACK ITEM is a record addressed to a person, kept on the device, that the next session to open
    there shows them until they have read it.
  - A STANDING device is one that Matthew's device roster, the list of devices he has admitted as his own,
    binds to him. The carrier checks the device that posted a miss against that roster before anything else.
  - A CARRY CLAIM is the record a carrier writes when it takes a miss, or declines one with a reason. A
    declined carry is born expired: its claim is already past its end when it is written, so no device waits
    on it. A carry claim, and the verdict it leads to, names an ACTOR (the person whose labour it was), a
    STEWARD (the person who answers for it) and a RUNNER (the device that did it). Machine work names no actor,
    so it never counts as anyone's own labour.
  - To VOID a grant is to end it before its expiry: nothing is carried under it afterwards. A grant is voided
    when Matthew revokes it, or when the check before its first carry fails. A BREACH OBSERVATION is a record,
    kept on the carrying device, that a grant in Matthew's name failed that check; it names the session that
    recorded the grant.

  Background:
    Given two standing devices of Matthew, "laptop" and "workstation"
    And "workstation" runs a carrier under its device supervisor, and "laptop" runs none
    And an answering session is moored on "workstation" and reading each note addressed to it as it arrives

  Scenario: a granted carrier carries a miss with no session told, and the record names who did what
    Given Matthew said, in a session on "workstation", that its carrier may carry orchestrator gates on his hand-over branches for the next eight hours
    And that session's main conversation recorded the grant with his words
    And nobody tells any session on "workstation" about the coming hand-over
    When "laptop" hands over a branch and posts a miss for its orchestrator gate
    Then the answering session acknowledges the carry within 120 seconds
    And the carrier runs the gate and records a verdict for the exact commit and key the miss named
    And the carry claim and the verdict name no actor, so the work is not counted as Matthew's own labour
    And they name Matthew as the steward who answers for it
    And they name "workstation" as the runner
    And they cite the grant
    And on "laptop", Matthew can read who carried it, under which grant, until when, and how it ended
    And the grant record "laptop" received points at Matthew's words and does not contain them

  Scenario: with no live answering session, the carrier declines instead of running alone
    Given a valid grant for the carrier on "workstation"
    But the answering session on "workstation" has stopped reading its notes
    When "laptop" posts a miss the grant covers
    Then no acknowledgement arrives within 120 seconds
    And the carrier records a declined carry with the reason "unanswered" and runs nothing
    And the declined carry is already expired when it is recorded
    And no device waits on it

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

  Scenario Outline: a grant recorded anywhere but the carrying device's main conversation is refused
    When <where> tries to record a grant for the carrier on "workstation"
    Then the grant is refused with the reason "<reason>"
    And the carrier carries nothing under it

    Examples:
      | where                                                   | reason                      |
      | a helper agent started by a session on "workstation"    | not the main conversation   |
      | the main conversation of a session on "laptop"          | not the carrying device     |

  Scenario Outline: a grant asked for beyond its limits is refused when it is recorded
    When Matthew's session on "workstation" records a grant that <asks>
    Then the grant is refused with the reason "<reason>"
    And the carrier carries nothing under it

    Examples:
      | asks                                                 | reason        |
      | expires more than 24 hours after it is recorded      | too long      |
      | covers branches that include "dev" or "main"         | shared branch |

  Scenario: with no grant, the carrier carries nothing
    Given no grant for the carrier on "workstation" has been recorded
    When "laptop" posts a miss for its orchestrator gate
    Then the carrier records a declined carry with the reason "grant"
    And it runs nothing

  # No miss has arrived in this scenario, so no carry is declined: the grant is void before it is used.
  Scenario: a grant that does not match its own record is void, and traced to the session that wrote it
    Given a session on "workstation" recorded a grant whose permission covers more gates than its clause
    When the answering session checks the grant before the first carry under it
    Then the check fails and the carrier carries nothing under that grant
    And the answering session revokes the grant, which voids it
    And a breach observation on "workstation" names the session that recorded the grant
    And a feedback item addressed to Matthew, saying a grant in his name did not check out, is recorded

  # Needs the carrier@<device> berth address, not yet built (the design's slice 5).
  @requires:carrier-address
  Scenario: when asked what it is doing, the answering session answers for the carry
    Given the carrier on "workstation" is carrying a miss under a valid grant
    When a session on "laptop" asks "workstation" what its carrier is doing
    Then the answering session replies with the gate, the commit, the grant and its expiry, and Matthew as the steward who answers for it
    # The reply is built from the carry's own records, not from what the session remembers.
    And the reply is right even when the answering session started after the carry began

  Scenario: a grant Matthew revokes carries nothing after he revokes it
    Given a valid grant for the carrier on "workstation" that covers orchestrator gates on hand-over branches
    When Matthew revokes the grant in a session on "workstation", in his own words
    And "laptop" then posts a miss the grant would have covered
    Then the carrier records a declined carry with the reason "grant"
    And it runs nothing

  Scenario: an expired grant carries nothing
    Given a grant for the carrier on "workstation" that expired an hour ago
    When "laptop" posts a miss the grant would have covered
    Then the carrier records a declined carry with the reason "expired"
    And it runs nothing

  Scenario: a lander does not reuse a verdict whose grant expired before the verdict was finished
    Given the carrier on "workstation" started a carry under a grant that expired before the gate finished
    And the carry recorded its verdict after the expiry
    When "laptop" lands the branch and finds that carried verdict
    Then the lander does not reuse it
    And the reason it gives is that the grant was not valid when the verdict was recorded
    And the lander runs the gate itself
