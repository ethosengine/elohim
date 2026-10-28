@e2e @devflow @wip @concern:idle-is-free @requires:epr-cli @act:host
Feature: Read progress through a domain composition
  As a person coordinating work in a declared commons
  I want one unchanged set of work views to read progress in any domain's evidence
  So that activity, verified movement and unknown evidence remain distinguishable, and I can keep a reading offline.

  Vocabulary. A work composition is a data-only declaration (schema
  elohim/sdk/schemas/v1/manifest/work-composition.schema.json, version 2) that groups declared
  views over a repository's files and links them: the coding composition for Rakia covers habits,
  backlog items, features and sprints; the orchard composition covers the orchard sample's own
  subjects. The native reader is the `epr view` command, which captures a composition: it reads
  the declared files once and records an observation (the declarations, the exact source bytes and
  the resolved links). A subject kind is the type of work item a view lists, for example a habit
  or a backlog item. The board lists subjects of one kind with authored status; the burndown plots
  completion across observations over a fixed cohort, namely the subjects in the baseline, which is
  the earliest observation selected for comparison. "Reusable" is shown in the last scenario: one
  pinned Avodah build (the work pillar of the Elohim client) reads both compositions without
  change. A version one declared view is a single-view declaration older than compositions
  (composition files are version 2). An executable declaration is one that carries an instruction
  to run something, for example an `execute` key.
  Concern: tagged idle-is-free because the coding composition's real subjects include that habit's
  backlog; no habit names this feature as a check and it proves reading, not idle cost.

  Scenario: A composition preserves sources, claims and independent conditions
    Given coding and orchard work compositions with related subject kinds
    When the same native reader captures both compositions
    Then each composition retains its declarations, exact sources and explicit relationships
    And authored claims remain claims even when their text says completed
    And unavailable evidence never becomes acceptance

  Scenario: Captures remain private and older declarations remain readable
    Given a retained composition observation and a version one declared view
    When a domain declaration changes and another observation is captured
    Then the earlier observation reopens with its original interpretation
    And version one views still reopen without migration
    And an executable declaration or a reference outside the repository is refused

  Scenario: Progress and board remain independent of domain vocabulary
    Given a pinned Avodah build reading a work composition
    When I compare observations and inspect a linked work subject
    Then I can inspect baseline, authored claims and evidence availability
    And opening and refresh each capture of a repository-sized composition successfully within 60 seconds, the workspace proxy deadline
    And the board and burndown use one selected subject kind
    And offline retention includes the complete composition and sources
