@e2e @devflow @concern:edit-diagnostics-reach-every-session @act:host @requires:epr-cli @local @wip
Feature: Matthew's editing sessions share diagnostics without competing for build capacity
  Matthew works with two agent editing sessions in one checkout. A language server checks their
  Rust files and reports errors and warnings. Attribution names the session whose edit caused a
  new diagnostic. Full diagnostics keep their file and source location. A session's identity is
  the id its runtime gives it plus the process that owns it; every edit it makes is logged under
  that identity, so "A" and "B" never share one. A build session is any other session that holds
  a cargo lease while it compiles, such as a gate run.

  A session receives in full the diagnostics in files it has opened or edited, and any diagnostic
  attributed to its own edit, wherever that diagnostic lands. Every other diagnostic folds. A fold
  is one summary per package and owner, shown on that package's "Cargo.toml" so it stands for the
  package as a whole, and it counts the errors and warnings that owner's work currently has there.

  A berth lease records who holds one unit of workspace capacity; "berth status" shows its
  holder and the available capacity. Flycheck is rust-analyzer's background Cargo check on save.
  The analysis window is the 30 seconds up to and including a diagnostic's publication, set by
  "lsp.attribution_window_ms": 30000 in "genesis/agentic/pool-policy.json". A new diagnostic is
  attributed to the session that edited inside that window. When two sessions both edited inside
  it, the diagnostic is marked ambiguous and names both. When nobody did, it is unattributed.
  The lsp pool slot is a dedicated directory for analysis build artifacts, beside the normal
  dev directory in Cargo's shared build cache. The pool slot stores artifacts, a cargo lease
  reserves build capacity, and berth tracks that lease separately from the lease that keeps
  the device mux alive. The berth policy declares how many cargo leases can be held at once.

  The device supervisor, ark, keeps a resident process running independently of editing
  sessions. That resident is the device mux: it shares connections and starts rust-analyzer,
  the Rust language server, only when a session needs diagnostics for a checkout.
  Its one toolchain lease lasts as long as the mux runs. The mux stays available after an
  idle checkout's analyzer exits. Shutdown is bounded: after 900 seconds without clients,
  it waits up to 5 seconds for a shutdown reply and 2 more for exit before forcing termination.

  A session attaches when its editor connects to the resident for Rust diagnostics.
  Attaching never starts the resident. Without it, a null server answers the editor and
  explains why diagnostics are unavailable; it does not analyze code or block editing.
  A session detaches when its connection closes, for example when the session ends. "Caused-by-me" is
  how the editing session sees a diagnostic its own edit produced; every other session sees the
  same diagnostic "attributed to" that session. "epr lsp show" prints, for one session, the
  diagnostics it has received, grouped as its own, caused by it, and others'.

  # Resident lifecycle: elohim/eprfs/epr-lsp/src/daemon.rs:569, elohim/eprfs/epr-lsp/src/daemon.rs:748.
  # Lease: elohim/eprfs/epr-cli/src/lsp.rs:387. Shutdown: elohim/eprfs/epr-lsp/src/limits.rs:21.
  # Missing resident: elohim/eprfs/epr-cli/src/lsp.rs:233, elohim/eprfs/epr-lsp/src/attach.rs:113.
  Background:
    Given Matthew has an isolated native Rust workspace with packages "core", "client", and "other"
    And package "client" uses the public types of package "core"
    And sessions "A" and "B" have distinct session identities in that checkout
    And the Rust language server policy pins toolchain "1.96.1" with an idle timeout of 900 seconds
    And the attribution window for new diagnostics is 30 seconds

  Scenario: Two sessions share an analyzer that exits when both leave
    Given the device supervisor is running the device mux
    When sessions "A" and "B" attach to Rust diagnostics for that checkout
    Then both sessions use the same language server process on toolchain "1.96.1"
    And the device mux holds one berth lease named "toolchain:lsp" for its whole lifetime
    And Matthew can read diagnostics in either session without starting a second language server
    When session "A" detaches and 900 seconds pass
    Then session "B" still receives diagnostics from that same process
    When session "B" detaches and 900 seconds pass
    And bounded shutdown finishes
    Then the rust-analyzer process for that checkout has exited
    And the device mux stays up and still holds its one berth lease

  Scenario: Editing continues when the device supervisor is not running
    Given neither the device supervisor nor the device mux is running
    When session "A" opens a Rust file and edits it
    Then session "A" receives "no diagnostics: device supervisor not running — epr device supervise"
    And Matthew's edit completes
    And no language server starts

  Scenario: An edit that breaks another package reaches both affected sessions
    Given the device supervisor is running the device mux
    And session "A" edits only package "core" and session "B" has opened a file in package "client"
    When session "A" changes a public type in "core" so the opened "client" file no longer type-checks
    And the server publishes the new diagnostic within 30 seconds with no other edits in that window
    Then session "A" receives the full "client" diagnostic marked "caused-by-me"
    And session "B" receives that diagnostic attributed to session "A"
    And "epr lsp show" for each session agrees with its delivered diagnostics

  Scenario: Two edits inside one window make a diagnostic ambiguous, never a guess
    Given the device supervisor is running the device mux
    And session "B" has opened a file in package "client"
    When sessions "A" and "B" both edit package "core" within the same 30-second window
    And the server then publishes a new error in the opened "client" file
    Then each session receives that error in full, marked ambiguous and naming both "A" and "B"
    And neither session sees it marked "caused-by-me"

  Scenario: Unrelated work reaches another session as one fold
    Given the device supervisor is running the device mux
    And session "A" has opened or edited only package "core"
    And session "B" alone edits package "other" within the 30-second analysis window before publication
    When that edit produces 2 errors and 1 warning in "other"
    Then session "B" receives all 3 diagnostics in full
    And session "A" receives one summary on "other/Cargo.toml" naming session "B", 2 errors, and 1 warning
    And session "A" receives no full diagnostics from package "other"

  Scenario: Builds holding every cargo lease defer flycheck without delaying the edit
    Given the device supervisor is running the device mux
    And the berth policy allows 2 cargo leases at once
    And two build sessions other than "A" and "B" each hold one of them
    When session "A" saves a Rust file
    Then that edit completes and flycheck launches no cargo check
    And session "A" receives "flycheck deferred: build slot held by" naming the holders
    And those build sessions keep their leases
    When the build sessions release their cargo leases and session "A" saves again
    Then flycheck checks only the saved file's package, tests included so a test-only break is not missed, in the dedicated lsp pool slot
    And flycheck releases its cargo lease when the check ends
