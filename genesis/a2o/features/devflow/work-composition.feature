@e2e @devflow @concern:idle-is-free @requires:epr-cli @act:host
Feature: Read progress through a domain composition
  As a person coordinating work in a declared commons
  I want domain evidence translated into reusable work views
  So that activity, verified movement and unknown evidence remain distinguishable.

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
    # Repository-sized captures must finish before a 60-second workspace proxy deadline.
    And opening and refresh each capture successfully within 60 seconds
    And the board and burndown use one selected subject kind
    And offline retention includes the complete composition and sources
