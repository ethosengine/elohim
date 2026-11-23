# Tags: @devflow, developer-workflow suite; @concern, the habit this checks; @act:host, one host, no network peers;
# @requires:epr-cli, skipped without `epr`; @wip, no steps yet.
@e2e @devflow @concern:agents-arrive-briefed @act:host @requires:epr-cli @wip
Feature: An agent arrives briefed at its target, so Matthew does not have to repeat himself
  As an agent arriving to do one piece of work
  I want to be handed what the work is for, when it is done, why, who decides and how to run it, each with its source
  So that I work from the record, Matthew need not say it again, and the next agent is briefed with what misled the last one

  Matthew Dowell is the steward of record of this repository's devices and agents.
  `epr`, the device's own tool, composes each brief from records the repository and the device keep.

  Vocabulary:
  - Device: one workspace bound to Matthew, keeping its own `epr` notes; his roster, carried by git, lists them.
  - Runtime: the program hosting an agent, whose hooks can show it text.
  - Conductor: an agent that dispatches others. Seat: the role an agent is dispatched to fill, such as implementer; an isolated seat judges unaided.
  - Target: the directory the work is about. Governance package: a directory's governed files (purpose, rules, habits). Habit: a promise bound to a runnable check, the scenario tagged with its name.
  - Task: one piece of work, given by a prompt, a dispatch, a commitment (a unit of work in the device's ledger) or a contract, recorded when given.
  - Covenant: what Matthew permits one session, granted only after it opens, ending with it. Grant: a record Matthew signs letting what it names act for him, within a bound until an expiry.
  - Contract: an unattended scripted run's fixed declaration of its objective and the grant its steps act under.
  - Rung: one check on a task's done ladder, each stricter than the last. Acceptance of a done task is Matthew's alone.
  - Appreciation seat: a seat that did no part of the work, judging a done task from its artifact and trail (its evidence, commands and steering).

  The brief, under a heading naming the agent's level ("minimal" or "detail", bound to its model by its runtime's table) and seat, if any, has four parts:
  1. Goal, from the record that gave the task; rung: the done ladder its commitment or contract declares, target marked, else the covering habit's check.
  2. Guide-stars: system (the tracked root governance package, which git carries to every device); directory (the target's purpose and
     rules, its covering habit, ruling notes on it or a file in it); commitment (a reason its commitment records).
  3. Authority, from the record the agent acts under: its own covenant or grant, or "dispatched under the conductor's covenant", naming it.
  4. Where to look: the target's gate; open correction notes on it or a file in it; the task's working record (latest finding, open question, next action);
     at level "minimal" only, a course of action: one path from the rung's check, the file the goal names and the ruling on it, ending at the authority's stop-and-ask clause.
  - Each line carries its source's address and content identifier: a record, or one declared entry in a governance package or build manifest. A README is not shown.
  - A line with no source reads a marker, such as "unmeasured", with its reason; a taskless agent's goal reads "untasked".
  - Pushed: at an agent's start, before its first act; at its claim of a task; right after a context reset; once after a line's source changes or its marker is replaced,
    that line marked as changed. Otherwise composed only when the agent, its conductor or Matthew asks. A taskless agent is pushed none; hooks off, it comes once, in the launch prompt.
    An isolated seat is pushed none and composed none.

  Background:
    Given a test repository on Matthew's test device "device-a", the one device his roster names, where a fixture holding his roster identity performs his acts
    And its "garden" directory is governed by these records:
      | record          | what it says                                                                  | where it is recorded                          |
      | system star     | the record is the authority, not a remembered copy                            | the root governance package                   |
      | purpose         | keep the household's seed records true to what was planted                    | the garden governance package                 |
      | covering habit  | garden-records-stay-true: every total equals its records' sum                 | its file in the garden governance package     |
      | gate            | just gate garden                                                              | the garden build manifest                     |
      | ruling          | totals are derived from records, never typed, as a typed total drifted twice  | a ruling note on the garden totals file       |
      | open correction | import with the empty-date fixture; runs without it hid the rows they skipped | a correction note on the garden import script |
    And the garden README names a retired command for the import, whose route is now the garden gate
    And the test runtime's table of levels binds model "fixture-small" to "minimal" and "fixture-large" to "detail", and its hooks run unless a scenario says otherwise
    And the task "make the totals in the garden totals file match their records" is called the garden task, and nothing that gives it declares a target rung or a reason
    And the task "make the garden import keep rows with an empty date" is called the import task

  Scenario: An interactive session is pushed its brief at start, and again only when a line changes
    Given Matthew opens an interactive session running "fixture-large" at the garden directory and gives it the garden task in his opening prompt
    When the session starts, before Matthew can grant its covenant
    Then its brief is pushed before its first act, its authority reading "none held", with the reason that no covenant is granted yet
    When Matthew grants its covenant: it may change files under the garden directory, must ask before changing anything else, and may not push to the shared repository
    Then the brief is pushed once more, its heading naming the level "detail" and its four parts holding exactly these lines, in this order:
      | part          | the line                                     | its source                    |
      | Goal          | the garden task                              | Matthew's opening prompt      |
      | Goal          | the covering habit's check passes            | the habit's file              |
      | Guide-stars   | the system star                              | the root governance package   |
      | Guide-stars   | the garden purpose                           | the garden governance package |
      | Guide-stars   | the covering habit's invariant               | the habit's file              |
      | Guide-stars   | the ruling, with its reason                  | the ruling note               |
      | Guide-stars   | "unmeasured": no commitment records a reason | none                          |
      | Authority     | Matthew, and the covenant's three clauses    | the session's covenant        |
      | Where to look | the garden gate                              | the build manifest            |
      | Where to look | the open correction on the import script     | the correction note           |
      | Where to look | "unmeasured": no working record yet          | none                          |
    And every sourced line carries its source's address and content identifier
    And its authority line is marked as changed, and no other line is
    When the session makes further tool calls, and no source changes
    Then none of them is shown a brief
    When the ruling note is amended
    Then the brief is pushed once more, only its ruling line, now amended, marked as changed

  Scenario: A subagent sent with only a goal and a target reads the rest from the target, not its sender
    Given a conductor holding a covenant from Matthew works in the "orchard" directory, whose governance names a different purpose
    When the conductor dispatches a subagent with only the garden task and the garden directory's address
    Then its brief, pushed before its first act, gives its goal as the garden task, citing the dispatch
    And it names, each with its address, the system star, the garden purpose, the covering habit and its check as the target rung, the ruling, the gate and the open correction
    And nothing from the orchard directory's governance appears in it
    And its authority reads "dispatched under the conductor's covenant", with that covenant's address

  Scenario Outline: The <kind> receives all four parts of the brief
    Given the <kind> is started at the garden directory, given the garden task by <its anchor>
    And it acts under <its authority record>
    And its runtime's hooks are <hooks>
    When it begins work
    Then the brief reached it, before its first act, through <the delivery>
    And it holds the four parts
    And its goal cites its anchor, its target rung is the covering habit's check, and its guide-stars and gate come from the root governance package and the garden directory
    And its authority line reads <its authority line>, with that record's address

    Examples:
      | kind                              | its anchor                                | its authority record                  | hooks        | the delivery                    | its authority line                                   |
      | implementer on a commitment       | a commitment its conductor claimed for it | its conductor's covenant from Matthew | running      | its runtime's start hook        | "dispatched under the conductor's covenant"          |
      | unattended step of a scripted run | the scripted run's contract               | the grant the contract names          | switched off | its launch prompt, its one push | Matthew, the grant, its bound and expiry, as its own |

  Scenario: A successor after a restart inherits what was said, not what was granted
    Given Matthew gave a session at the garden directory the garden task in his opening prompt, and a covenant to change files there
    And it has written its next action to the garden task's working record
    When its runtime restarts, and the device records a new session as its successor on the garden task
    Then the successor's brief, pushed at its start, gives the same goal, target rung, guide-stars and gate, each sourced line with the same address
    And its where-to-look part carries the next action its predecessor wrote, with the working record's address
    And its authority reads "none held", and nothing its predecessor was granted is listed as its own

  Scenario Outline: An isolated seat receives no brief
    When a conductor holding a covenant from Matthew dispatches <seat>, handing it <what its dispatch hands it>
    Then it is pushed no brief, and nothing is added to what its dispatch hands it
    When it, its conductor or Matthew asks for its brief
    Then none is composed, with the reason that its seat is isolated

    Examples:
      | seat                                           | what its dispatch hands it                         |
      | a blind reader, judging one document unaided   | one document's path and one review profile         |
      | a stasis keeper, diagnosing from evidence only | the evidence of an incident the conductor gathered |
      | the appreciation seat                          | a done task's artifact address and trail's address |

  Scenario: An agent's brief is shaped to its level, a capability, not a model; an unbound model gets "minimal"
    Given a conductor holds a covenant from Matthew to change files under the garden directory, asking before changing anything else
    When the conductor dispatches two implementers with the garden task at the garden directory, one running "fixture-small" and one running "fixture-large"
    Then each brief's heading names the seat "implementer" and its level, "minimal" and "detail" respectively, and neither names a model
    And only the minimal brief's where-to-look part holds a course of action: run the scenario tagged garden-records-stay-true; work in the garden totals file, as the ruling on it says; stop and ask before changing anything outside the garden directory
    And otherwise both briefs hold the same lines, both authority lines reading "dispatched under the conductor's covenant"
    When the conductor dispatches an implementer running "fixture-unknown", which the table does not bind
    Then that brief's heading names the level "minimal", and names "fixture-unknown" as an unrecognised model

  Scenario: Done is reaching the target rung the commitment declares
    Given a conductor holding a covenant from Matthew dispatches an implementer on the import task at the garden directory
    And the import task's commitment records its done ladder: the import's unit tests, the scenario tagged garden-records-stay-true, a run by simulated people on a test network, marked as its target rung
    When the implementer starts
    Then its brief states that done ladder in order, with the commitment's address, marking the simulated-people run as the target rung
    When a fixture records against the commitment a pass of that rung's check, and nothing for either lower rung
    Then the commitment shows the simulated-people run as the rung reached, the highest with a pass, lower rungs needing none, and the task as done
    And it shows Matthew's acceptance as awaited

  Scenario: The appreciation of a done task is testimony, not acceptance
    Given the import task's commitment shows the task done at its target rung, and Matthew's acceptance awaited
    And a conductor holding a covenant from Matthew dispatched the task's implementer
    And the appreciation seat is a fixture whose verdict is "approved"
    When the conductor dispatches the appreciation seat with the import task's artifact and trail addresses
    Then `epr` records that verdict as a note under the conductor's covenant, with the seat as its provenance, citing the artifact and the trail
    And the commitment still shows the task's rung unmoved and Matthew's acceptance as awaited

  Scenario: A poor journey leaves a correction on the surface that misled, and the done task stays done
    Given the import task's commitment shows the task done at its target rung
    And a conductor holding a covenant from Matthew dispatched the task's implementer
    And the implementer's trail shows it ran the README's retired command, failed, and was steered back twice by Matthew
    And the appreciation seat is a fixture whose verdict is "changes-requested", naming the README as the surface that misled and the garden gate as the route
    When the conductor dispatches the appreciation seat with the import task's artifact and trail addresses
    Then `epr` records that verdict as a note under the conductor's covenant
    And the commitment still shows the task as done
    And `epr`, as the verdict names a surface that misled, records a correction note on the garden README under that covenant, quoting the verdict, with the trail's address

  Scenario: The next agent at the garden directory is shown the README's correction before it can read the README
    Given a correction note on the garden README, naming the garden gate as the import's route, is open
    When a conductor holding a covenant from Matthew dispatches an implementer with a new task at the garden directory
    Then the brief pushed before its first act lists the correction among the open notes, with the note's address, naming the README it is on
    And the README's own text holds no line in that brief

  Scenario: Only recorded steering that cites an open correction is counted
    Given the correction note on the garden README is open
    And `epr`'s count of repeated steerings, the recorded steering notes citing an open correction, stands at zero
    And an agent at the garden directory was pushed a brief listing that correction
    When Matthew steers the agent away from the README's retired command, and the agent records a steering note citing the correction note's address
    Then the count is one, naming the correction note's address
    When Matthew steers it so again, and it records no steering note, and then steers it elsewhere, and it records a steering note citing no correction
    Then the count stays at one

  Scenario: A context reset mid-task keeps the goal, the rung and the next action
    Given an agent works the garden task, having written its latest finding, open question and next action to the garden task's working record
    When its runtime resets its context, and it continues
    Then a brief is pushed right after the reset, carrying the same goal and target rung
    And its where-to-look part carries that latest finding, open question and next action, with the working record's address

  Scenario: A sponsored runner's brief, asked for by Matthew, says what its grant cannot supply
    Given a sponsored runner, a program that runs no model, reads no prompt and claims no work, works under a grant Matthew signed that bounds its compute until an expiry and names no task or directory
    When it starts
    Then no brief is pushed to it
    When Matthew asks for the runner's brief
    Then its heading names no level, and it holds no course of action
    And its authority names Matthew, the grant, its bound and its expiry, with the grant's address
    And its system-level guide-star line is the system star, with the root governance package's address
    And its goal reads "untasked", and its rung, directory-level and commitment-level guide-stars, gate, open notes and working record "unmeasured", each with the reason that the grant names no task or directory

  Scenario: An always-on agent with no task is pushed no brief until it claims one
    Given an always-on agent, which takes work by claiming it, runs under a grant Matthew signed naming the garden directory as where it works
    And it holds no task, and a commitment for the garden task is open and unclaimed at the garden directory
    When it starts
    Then no brief is pushed to it
    When it asks for its brief
    Then its goal reads "untasked", and its rung, commitment-level guide-star and working record "unmeasured", each with the reason that it has claimed no work
    And its other lines come from the root governance package and the garden directory, its authority naming Matthew and the grant, held as its own
    When it asks for the participation read of the garden directory, the list of its open, unclaimed commitments
    Then the list holds the garden task's commitment, with its address
    When it claims that commitment
    Then its brief for the garden task is pushed as the claim's answer, before its next act
    And that brief's goal cites the commitment's address, and its target rung is the covering habit's check

  Scenario: On a second device, notes read as unknown, and a lesson crosses only once graduated
    Given Matthew enrolls a second test device "device-b" on this host, and his roster, as git carries it, now names "device-a" and "device-b"
    And "device-b" holds the same repository commit as "device-a", and no notes of its own
    And the open correction's lesson is also held in a runtime's private memory on "device-a"
    When a conductor on "device-b", holding a covenant Matthew granted there, dispatches an agent with the garden task at the garden directory
    Then its system star, garden purpose, covering habit, check and gate are as on "device-a", read from the tracked files
    And its ruling notes and its open notes are each one line reading "unknown — local to device-a", with the reason that notes stay on the device that wrote them
    When an agent on "device-a" asks for its brief
    Then after its own ruling and open notes it holds one line each reading "unknown — local to device-b"
    And no brief on either device cites the runtime's private memory
    When an agent on "device-a" graduates that lesson into the garden governance package as a rule with its reason, naming the correction note it closes, and the commit reaches "device-b"
    Then on "device-a" the correction note shows as closed, citing the garden governance package, and no longer appears in briefs there
    And the next agent on "device-b" sees that rule, with its reason, as a new directory-level guide-star line, with the garden governance package's address
    And its two "unknown — local to device-a" lines remain, as notes do not travel
