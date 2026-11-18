@concern:dev-system-equilibrium
Feature: Collaborate through governed memory without carrying its whole history
  As a repository agent joining another agent's investigation
  I want a small context view that traces back to governed evidence
  So that I can correct misleading context and share only warranted repository knowledge

  The repository declares its local collective relationship under .epr-meta.
  That declaration is local governance, not proof of network membership.
  The evidenced finding is "The trial reader opened the cited passage".
  The disputed alternative is "The whole workflow has independent acceptance".
  The source records that passage opening succeeded and whole-workflow acceptance is undecided.
  Both claims remain separately attributable; a projection must not erase the undecided qualification.
  A local-only passage must not escape through either an assertion or its receipt.
  Measurements describe a declared scope; missing measurements are not zero cost.

  A ceremony is one bounded pass over shared memory. It opens on a stated concern, gathers evidence,
  acts, and closes with a recorded observation; a later process can resume it from what it recorded.
  A projection is a view generated from source claims, such as the context a joining agent is shown.
  A receipt is the record of what a reading opened, so a receipt can itself reveal what was read.
  To rehearse repository reach is to try, without publishing, whether a finding may be shared with
  whoever works in the repository.
  A burden baseline is a measurement, taken when a ceremony opens, of how many bytes the memory in
  its scope holds. The closing observation is compared with it, which is how a pass shows what it
  kept and what it shed without carrying the whole history forward.

  Memory written in one person's sessions is inherited by the next person who clones the repository.
  So each memory entry says whom it applies to, and the author says so when writing it:
  "device" means true of one machine only and of no use on another;
  "steward" means one named person's direction, or their description of themselves, kept on record as theirs;
  "repository" means context crafted from the work, true for whoever works here.
  An entry written before authors were asked to say this takes its audience from its kind:
  a note about a person and a person's direction are a steward's; project context and references are the repository's.
  The memory index a session reads at its start is a single file in the repository.
  A reader who clones the repository gets the same index as its author, so the index says whose each entry is.

  Scenario: A second agent follows shared context and preserves the dispute after reset
    Given a declared local collective with an evidenced finding and a contested alternative
    When another agent opens that collective through the ceremony
    Then its context identifies the collective, purpose, evidence and unresolved alternative
    When a fresh process resumes the collective ceremony
    Then the same governed inputs remain traceable without importing private native memory

  Scenario: A challenge names the projection that misled the reader
    Given a declared local collective with an evidenced finding and a contested alternative
    When the reader challenges a projection's omission rather than its source assertion
    Then the feedback identifies the exact viewed projection and its omitted undecided qualification
    And the source claim remains unchanged pending its own review

  Scenario: Repository graduation keeps local evidence local
    Given a declared local collective with an evidenced finding and a contested alternative
    When the agent rehearses repository reach for a finding depending on local-only evidence
    Then the graduation refuses to widen the finding and its revealing receipt

  Scenario: The closing lens compares explicit observations without grading judgment
    Given a ceremony with an explicit scoped burden baseline
    When a duplicate is consolidated and the ceremony records its closing observation
    Then the deterministic report shows active content and total retained byte changes with exact paired inputs
    And archived bytes and new run artifacts are counted rather than mistaken for deletion
    And the report leaves the contested judgment unresolved

  Scenario: A second developer's first session can tell whose each inherited memory is
    Given a collective whose memory holds a shared finding, the first steward's direction and a note describing the first steward
    When a second developer's session reads the projected memory index
    Then the shared finding is listed as held by the collective
    And the first steward's direction is listed under the first steward's name
    And the note describing the first steward is listed under the first steward's name
    And no row under the first steward's name is listed as held by the collective

  Scenario: A newcomer's first correction is declared as theirs when it is written
    Given a collective whose memory holds a shared finding, the first steward's direction and a note describing the first steward
    When the second developer's session writes a correction that does not say whom it applies to
    Then the memory gate refuses the entry and names the three audiences
    When the session writes the same correction as applying to its steward
    Then the entry is accepted
    And after import the correction is listed under the second developer's name

  Scenario: An entry true of one machine does not reach the repository
    Given a collective whose memory holds a shared finding, the first steward's direction and a note describing the first steward
    When an entry declared as applying to the device is imported
    Then its contribution reaches no further than the device it was written on

  @wip
  Scenario: A newcomer's entry becomes the collective's only by another steward's verdict
    Given a collective whose memory holds a shared finding, the first steward's direction and a note describing the first steward
    And the second developer has written a correction that applies to its steward
    When the second developer rehearses graduating that correction to the repository
    Then the graduation waits for a verdict from a steward who is not its author
    When the first steward approves it
    Then the correction is listed as held by the collective and still names who wrote it

  @wip
  Scenario: Two stewards' directions disagree and neither overwrites the other
    Given a collective whose memory holds a shared finding, the first steward's direction and a note describing the first steward
    And the second developer has written a correction that applies to its steward
    And that correction and the first steward's direction say opposite things about the same practice
    When the memory is consolidated
    Then both entries remain listed, each under its own steward's name
    And the disagreement is recorded as a consolidation awaiting judgment

  @wip
  Scenario: A newcomer inherits no private picture of another person
    Given a collective whose memory holds a note describing the first steward
    And that note applies to the first steward and has not been graduated to the repository
    When a second developer initialises a workspace from the repository
    Then the note is not among the memories the second developer's session can read
