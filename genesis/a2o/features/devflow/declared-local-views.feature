@e2e @devflow @concern:idle-is-free @requires:epr-cli @act:host
Feature: Read a declared collection of work and keep the exact sources behind it
  As an engineer choosing what to work on next
  I want a local reading whose selection, evidence and source documents I can inspect
  So that I can revisit an earlier reading without mistaking missing work or narrative claims for completion.

  # Native reader, capture-race, corruption and confinement cases also execute in
  # elohim/eprfs/epr-cli/tests/view.rs and src/view/mod.rs under the EPRFS gate.
  # These fixtures prove the projection journey, not acceptance of their fictional work.
  Background:
    Given a workshop and an orchard with their own local view declarations

  Scenario: The same reader understands two independently organized collections
    When I read each collection with the same native binary
    Then the workshop shows its table task and the orchard shows its north bed
    And neither collection needs repository-specific runtime configuration

  Scenario: A declaration controls what is selected and shown
    When the orchard declaration selects apples instead of pears
    Then the orchard reading contains no matching subjects
    When the orchard declaration follows its moved plots and shows their category
    Then the north bed appears from its new location with only the requested field

  Scenario: An authored status and a narrative inspection do not establish acceptance
    When I read the workshop collection
    Then the table retains its authored open status
    And structured acceptance and measurement readings are explicitly unavailable

  Scenario: Source navigation opens the bytes used by the reading
    When I save the workshop reading
    Then its source address resolves to the complete original document including frontmatter

  Scenario: A saved reading remains usable without its original checkout
    When I save the workshop reading
    And the workshop checkout is removed
    Then I can reopen the saved reading without consulting a checkout

  Scenario: Comparing readings preserves the original cohort
    When I save the workshop reading
    And the table document is replaced by a chair document
    Then comparison reports the table as missing and the chair as added
    And reopening the earlier reading still shows the table as open

  Scenario: A broken saved source is reported rather than replaced by current bytes
    When I save the workshop reading
    And the retained table source is corrupted
    Then reopening refuses with a corruption diagnostic

  Scenario: Reading a collection leaves its source records untouched
    When I read the workshop collection
    Then every workshop file is unchanged and no workflow store has been created
