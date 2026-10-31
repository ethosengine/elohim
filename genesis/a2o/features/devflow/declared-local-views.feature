@e2e @devflow @concern:idle-is-free @requires:epr-cli @act:host
Feature: Read a declared collection of work and keep the exact sources behind it
  As an engineer choosing what to work on next
  I want a local reading whose selection, evidence and source documents I can inspect
  So that I can revisit an earlier reading without mistaking missing work or narrative claims for completion.

  # --- What this feature covers ---
  #
  # A local view declaration is a configuration, committed alongside a
  # collection of work documents, that tells the epr CLI which documents to
  # select and which of their fields to present. The epr CLI (the native
  # binary named in every scenario below, required by the @requires:epr-cli
  # tag) reads these declarations and produces a reading: a structured
  # snapshot of the selected documents, their authored status, and pointers
  # back to the source bytes each entry was drawn from.
  #
  # A reading can be saved to disk so it survives independently of the
  # directory tree it was produced from. Saved readings support comparison
  # (what changed between two points in time), source retrieval (open the
  # original document behind any entry), and corruption detection (refuse to
  # show an entry whose retained source no longer matches its recorded hash).
  #
  # Two fixture collections exercise the concept: a "workshop" containing a
  # task document called "table", and an "orchard" containing beds and plots
  # of fruit trees. Both are fictional test data invented for these scenarios;
  # they carry no real project content.
  #
  # The @concern:idle-is-free tag connects this feature to the habit that
  # asserts: merely reading a collection of work must leave every source file
  # and working directory exactly as it was, creating no side-effect stores.
  # The final scenario is the direct check.
  #
  # --- Glossary ---
  #
  # declaration    A per-collection configuration that names what to select
  #                (by path, pattern, or category) and what fields to show.
  #                Scenario 2 demonstrates changing one: selecting a different
  #                category, following relocated content, and restricting the
  #                visible fields.
  #
  # subject        A document matched by a declaration's selection criteria.
  #                When a declaration selects "apples" but the collection
  #                contains only pear documents, there are no matching
  #                subjects.
  #
  # checkout       A local working copy of the directory tree that holds the
  #                source documents. Removing a checkout means deleting that
  #                local tree. A saved reading should still open without it
  #                because the reading retains its own copy of the source
  #                bytes it needs.
  #
  # source address A pointer inside a saved reading that identifies the
  #                original document the reading was produced from. Resolving
  #                it retrieves the complete document, including any
  #                frontmatter the author wrote.
  #
  # moved plots    Documents whose path within the collection has changed
  #                since the declaration was first written. A declaration can
  #                follow relocated documents and present them under their
  #                new location.
  #
  # workflow store A side-effect directory or database that a workflow tool
  #                might create while processing files. The final scenario
  #                asserts that reading creates none.
  #
  # structured acceptance and measurement readings
  #                Formal computations that assess whether work is actually
  #                complete, as opposed to an author's own status field.
  #                Scenario 3 asserts these are explicitly absent from a
  #                local view reading: the reading shows what the author
  #                declared, not what the system has verified. This is the
  #                direct proof of the "So that" clause — an engineer can
  #                tell authored status from verified completion.
  #
  # --- Companion test coverage ---
  #
  # The native reader, capture-race, corruption, and confinement cases also
  # execute in elohim/eprfs/epr-cli/tests/view.rs and src/view/mod.rs under
  # the EPRFS gate.

  Background:
    Given a workshop and an orchard with their own local view declarations

  Scenario: The same epr binary reads two independently organized collections
    When I read each collection with the same native binary
    Then the workshop shows its table task and the orchard shows its north bed
    And neither collection needs repository-specific runtime configuration

  Scenario: A declaration controls what is selected and shown
    When the orchard declaration selects apples instead of pears
    Then the orchard reading contains no matching subjects
    When the orchard declaration follows its moved plots and shows their category
    Then the north bed appears from its new location with only the requested field

  Scenario: An authored status does not imply verified acceptance
    When I read the workshop collection
    Then the table retains its authored open status
    And structured acceptance and measurement readings are explicitly unavailable

  Scenario: A saved reading links back to its source bytes
    When I save the workshop reading
    Then its source address resolves to the complete original document including frontmatter

  Scenario: A saved reading remains usable after its source checkout is removed
    When I save the workshop reading
    And the workshop checkout is removed
    Then I can reopen the saved reading without consulting a checkout

  Scenario: Comparing readings preserves the original cohort
    When I save the workshop reading
    And the table document is replaced by a chair document
    Then comparison reports the table as missing and the chair as added
    And reopening the earlier reading still shows the table as open

  Scenario: A broken saved source is reported rather than silently replaced
    When I save the workshop reading
    And the retained table source is corrupted
    Then reopening refuses with a corruption diagnostic

  Scenario: Reading a collection produces no side effects
    When I read the workshop collection
    Then every workshop file is unchanged and no workflow store has been created
