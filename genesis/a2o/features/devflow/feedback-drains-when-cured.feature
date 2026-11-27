@e2e @devflow @concern:private-feedback-loop @act:host @requires:epr-cli @local
Feature: A felt problem is named, answered, drained and remembered
  As a person whose tools sometimes refuse or fail
  I want each failure kept as one named problem that grows while it is unanswered,
  reaches someone who can fix it only by my household's own choice,
  and is cleaned off my device once a fix has reached me and held
  So that these things hold:
    - my device does not fill with one record per failed command;
    - nobody outside my household learns what I was working on when the tool failed;
    - a fix that reaches me cleans up what it fixed, and what remains is one short note;
    - a problem that comes back after its fix is seen as the same problem returning.

  The words this story uses, so it can be read cold.

  The tool and its problems

  - "ferry" is the fixture tool. It has two RELEASES. Release 1 cannot do "ferry list --since";
    asked to, it refuses. Release 2 can. Every other "ferry" command works in both.
  - A CLAIM is a named kind of problem a tool can report. Each release of "ferry" ships a
    DECLARATION: the list of claims it can report, by name. The claim for the refusal above is
    named "ferry/capability-absent:list.since". A claim's name is a fixed word from that list. It
    is not built from what the person typed.
  - Release 2's declaration also lists what it CURES: the claim above, a CITATION naming where the
    cause is written up, and one SENTENCE saying what was fixed.
  - The declaration gives the claim a BOUND: the number of sessions it may be felt in before the
    device's person is told. Here the bound is 3.
  - The WATCH PERIOD is how long a fixed problem must stay fixed before it is called gone. Here it
    is 30 days. The watch is PASSED when two things are true: the command has passed in a later
    session, and 30 days have gone by since with no refusal. Time alone does not pass it.
    Scenarios move the device's clock to cross it.

  The people, the devices and the records

  - The Dowell household is a FIXTURE household: its people ("matthew", "jessica") are simulated,
    and everything their devices send is marked "fixture", so that whoever receives it keeps it apart
    and does not add it to any count of real people.
    "adam" is a fixture person outside the household who looks after "ferry": its STEWARD.
  - "matthew" has two DEVICES, A and B. "jessica" has one, device J. A SESSION is one sitting of a
    person or their agent at a device; a session knows whose it is.
  - Each device keeps a private FEEDBACK STORE. A RECORD in it says that a claim was observed in a
    session, by whom, from which release, and whether the command was refused or PASSED (it
    worked).
    The store is on the device only. Nothing in this story copies a record to another device.
  - The device's JUDGE reads the store against the declarations and the release the device now
    runs, and gives each claim a READING: "open", "behind", "cure available", "on watch",
    "retired" or "reopened". "Behind" means the claim was felt from a release older than the one
    the device now has. Reading changes nothing. A separate command, "retire", acts on a reading.
  - To RETIRE a claim is a command given by a session at the device. It is given twice in a
    claim's life. Given when the judge reads "cure available", it drains the claim's records and
    the claim is then "on watch". Given once the watch is passed, it leaves the note and the claim
    is then "retired". The judge does not retire anything by itself.
  - A session keeps the release of "ferry" it started with. When a device takes a newer release,
    sessions already running stay on the older one until they are restarted.
  - A SUMMARY is the only thing about a claim that leaves a device under a household's consent: the claim's name, the
    release it was felt from, how many sessions, the first and last day, and the mark "fixture".
    It does not say what the person was working on, what they typed, who they are, or which
    session it was. A summary that arrives from another device is counted there. It does not
    become a record in that device's store, and that device's judge gives it no reading.
  - A household's CONSENT is a statement by one of its people, in their own words, of which
    summaries may be sent to whom, until when. The Dowell household has given "adam" consent to
    receive summaries of "ferry" claims. It has given nobody consent to receive anything else.
    The same kind of statement is what lets summaries pass between one person's own devices; a
    scenario says so when "matthew" has made it.
  - An EXAMPLE is the text of one refused command, as it was typed. Consent does not cover it. It
    leaves a device only when someone the household has given consent to asks for one, the person
    is shown that exact text, and the person agrees to send that one. Each person answers for
    their own sessions.
  - The store's REPORT is what the device prints about its own feedback store: how much it holds,
    and how many bytes each fix returned when its records were drained.

  @wip
  Scenario: A refused command names its problem, and one session leaves one record
    Given device A runs "ferry" release 1 and its feedback store is empty
    When a session of "matthew" on device A runs "ferry list --since yesterday" five times
    Then each refusal ends with the claim's name "ferry/capability-absent:list.since"
    And device A's feedback store holds one record for that claim, in matthew's name
    And that record does not contain the word "yesterday"

  @wip
  Scenario: The same problem in more sessions is one growing problem
    Given device A runs "ferry" release 1
    When two sessions of "matthew" on device A each run "ferry list --since yesterday"
    Then the judge on device A lists one claim, "ferry/capability-absent:list.since", read "open", felt in 2 sessions
    And it lists no second entry for the same refusal

  @wip
  Scenario: A problem felt past its bound is told to the person whose device it is
    Given device A runs "ferry" release 1
    When three sessions of "matthew" on device A each run "ferry list --since yesterday"
    Then the next session of "matthew" on device A is told, when it starts, that the claim has reached its bound of 3
    And the telling names the claim and the count and nothing the sessions typed

  @wip
  Scenario: The person's other device counts the problem without learning what it was about
    Given "matthew" has said, in his own words, that summaries of "ferry" claims may pass between his devices
    And device A holds the claim "ferry/capability-absent:list.since" felt in 2 sessions
    When device B next hears from device A
    Then device B lists that claim as felt on one other device of matthew's, in 2 sessions
    And what device B received holds the claim's name, the release, the counts, the days and the mark "fixture"
    And it holds no session, no person's name and nothing that was typed
    And device B's feedback store holds no record for that claim

  @wip
  Scenario: Without that statement, nothing about the problem leaves the device
    Given "matthew" has made no statement about summaries passing between his devices
    And device A holds the claim "ferry/capability-absent:list.since" felt in 2 sessions
    When device B next hears from device A
    Then device B lists no claim from device A

  @wip
  Scenario: The tool's steward asks who feels a problem, and receives only summaries
    Given device A and device J each hold the claim "ferry/capability-absent:list.since"
    When "adam" asks who feels that claim
    Then "adam" receives one summary from the Dowell household for each of the two devices, each marked "fixture"
    And no summary says what "matthew" or "jessica" was working on

  @wip
  Scenario: Without the household's consent, the tool's steward learns nothing
    Given device A and device J each hold the claim "ferry/capability-absent:list.since"
    And the Dowell household has withdrawn its consent for "adam" to receive summaries
    When "adam" asks who feels that claim
    Then "adam" receives no summary from the Dowell household
    And nothing "adam" receives shows whether any device in that household holds the claim

  @wip
  Scenario: A person asked for an example sees exactly what would be sent, and may decline
    Given device A holds the claim "ferry/capability-absent:list.since"
    When "adam" asks the household for an example of the refused command
    Then "matthew" is shown the exact text that would be sent and the name of who asked
    When "matthew" declines
    Then "adam" is told only that the ask was declined
    And "adam" receives no example

  @wip
  Scenario: A person who agrees sends one example and no other
    Given device J holds the claim "ferry/capability-absent:list.since" from two sessions of "jessica"
    When "adam" asks the household for an example of the refused command
    And "jessica" agrees to send the example from her first session
    Then "adam" receives that one example
    And "adam" receives nothing from her second session

  @wip
  Scenario: A release that cites the problem drains its records to a citation and a sentence
    Given device A holds the claim "ferry/capability-absent:list.since" felt in 4 sessions
    When device A takes "ferry" release 2
    Then the judge on device A reads the claim "cure available" and names release 2
    When a session of "matthew" on device A retires the claim
    Then device A's feedback store holds, for that claim, one record that says it was felt in 4 sessions and over which days
    And beside it the citation and the sentence from release 2's declaration
    And the four session records are gone
    And the judge reads the claim "on watch"

  @wip
  Scenario: A fix that has arrived is not yet a problem gone
    Given device A has taken "ferry" release 2 and retired the claim "ferry/capability-absent:list.since" to "on watch"
    And no session on device A has run "ferry list --since" after that
    When the device's clock moves 31 days
    Then the judge still reads the claim "on watch"
    And it says the claim is waiting for a session in which the command passed

  @wip
  Scenario: A problem proven gone leaves one note that names the release that fixed it
    Given device A has taken "ferry" release 2 and retired the claim "ferry/capability-absent:list.since" to "on watch"
    And in a later session of "matthew" on device A, "ferry list --since yesterday" passed
    When the device's clock moves 31 days and a session of "matthew" retires the claim
    Then the judge reads the claim "retired"
    And device A's feedback store holds one note for it: the claim's name, release 2, 4 sessions dropped and the days they spanned
    And the store's own report says how many bytes that fix returned to the device
    And the note is still there after the store next cleans itself

  @wip
  Scenario: A problem that returns after its fix is the same problem, reopened
    Given the claim "ferry/capability-absent:list.since" is "retired" on device A with a note naming release 2
    When a session of "matthew" on device A runs "ferry list --since yesterday" on release 2 and it is refused
    Then the judge reads the claim "reopened" and names release 2 as the fix it counts against
    And the note naming release 2 is kept

  @wip
  Scenario: Whoever was told a problem was gone is told when it returns
    Given the claim "ferry/capability-absent:list.since" is "retired" on device A with a note naming release 2
    And "adam" has received a summary saying that claim was retired on a device of the Dowell household
    When a session of "matthew" on device A runs "ferry list --since yesterday" on release 2 and it is refused
    Then "adam" receives a summary saying the claim was reopened, naming release 2

  @wip
  Scenario: A session on an out-of-date tool is told the cure is its own, not a fix to wait for
    Given device A has taken "ferry" release 2
    And one session on device A was started before that and still runs release 1
    When that session runs "ferry list --since yesterday" and is refused
    Then the judge reads the claim "behind", and names the cure: restart the session
    And the claim is not counted toward the bound that tells the person a fix is needed
