@e2e @devflow @act:host @concern:rust-style
Feature: A developer can improve Rust style incrementally without hiding new debt

  Developers and coding agents share the same repository gate. Existing unexplained lint
  suppressions can be repaired a crate at a time. New suppressions need a reason, and a crate
  selected for a strict refactor must finish without unexplained suppressions in that scope.
  These scenarios run the native checker's isolated Git fixtures, with no application server.

  Scenario: A local repair cannot conceal the violation in a pushed commit
    Given a Rust change with a committed unexplained suppression and an uncommitted repair
    When I exercise the Rust style reach gate on isolated Git commits
    Then the committed violation is refused while the repaired working tree passes

  Scenario: One debt reduction satisfies the current change
    Given a Rust crate with two unexplained suppressions and a change repairing one
    When I check that repair twice, accept it as the integration baseline, and check a later change touching the same crate without further debt reduction
    Then both checks of the repair pass and the later change is refused

  Scenario: A module refactor can become strict while its crate pays down debt
    Given a Rust crate whose baseline has one legacy suppression and a refactor adding a new unexplained suppression in a strict module
    When I check the refactor, repair the strict module, and finally repair the legacy suppression
    Then the gate refuses both incomplete stages and accepts the clean strict module with its crate debt reduced
