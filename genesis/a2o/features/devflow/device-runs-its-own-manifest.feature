# Habit: elohim/eprfs/.epr-meta/device-supervises-its-manifest.habit.md.
# Runtime story: elohim/eprfs/epr-cli/tests/device_supervise.rs,
# ark_keeps_the_declared_resident_and_reloads_a_moved_head (requires ARK_BIN).
# @act:host runs on this device alone; @offline requires no network.
# @wip: these Gherkin steps are not wired; the Rust story is the existing live check.

@e2e @devflow @act:host @offline @wip @concern:device-supervises-its-manifest
Feature: My workspace device keeps its declared residents running offline

  As a developer, I want my workspace device to keep the programs I depend on running,
  even while disconnected. A resident is a program I choose to keep running locally, such
  as the shared language server my editor sessions attach to. I declare its arguments,
  ready line and restart policy ("permanent" restarts it after any exit, "transient" only
  after a failure, "temporary" never) with
  "epr device run <tool> [--ready-line <text>] [--restart ...] -- <args>".
  A ready line is text the resident prints on standard output once it can serve; ark also
  checks that the running executable is the declared one. A resident declared without a
  ready line makes no readiness claim: it is still restarted when it dies, and the
  supervisor counts it as running while its process is alive, never as ready.
  "epr device show" prints what is declared.

  Each declaration is appended to my device's private manifest chain, which keeps every
  earlier version; this feature only reads the newest one, the head. The head is named by
  its CID (content identifier), a hash of the manifest's contents, so a changed
  declaration always has a new CID.

  Ark is the local process supervisor that runs those residents. I start it with
  "epr device supervise". It keeps its running state and death records in one data root,
  a directory under my checkout's private status area that supervise creates on first use
  and reuses across reloads. Only one ark may run per data root, however it was started
  (by supervise, or by running "ark run" against that data root by hand); supervise checks
  the running processes for that, independently of any lease. A berth lease is the
  workspace's record of which agent session holds a shared resource, so other sessions can
  see it; supervise holds the daemon lease "device-ark", which has no expiry timer, and
  refuses while another live session holds it.
  A death witness records which resident died and its exit reason. Ark's incarnation is
  its generation number, shown by "epr device supervise --status" and carried forward when
  I reload a changed manifest, so I can tell whether the running ark has reloaded. I request
  that reload by running supervise again. "epr device supervise --stop" stops ark and
  its children. Ark enforces no resource bounds and provides no process isolation.

  "just status device" reports each condition of my workspace device on its own line. A
  "degraded" line means the device works and the line names the command that cures it; a
  "REFUSED" line means a prerequisite is missing and dependent work cannot proceed.

  Background:
    Given I am a developer using a workspace device with no network
    And ark and the resident's executable are already available on my device
    And the resident is a test program that prints "fixture-ready" when it can serve and exits when asked to stop
    And my session can claim the "device-ark" berth lease

  Scenario: I declare a resident and one supervisor keeps it ready
    Given my device declares no residents and no ark is running
    When I declare the resident with its arguments, ready line "fixture-ready" and restart policy "permanent"
    Then my device's manifest chain has a new head containing that declaration
    And the resident is not running yet
    When I run "epr device supervise"
    Then exactly one ark runs for my device's data root at the declared head CID
    And "epr device supervise --status" shows the resident ready with a process ID
    And "just status device" shows the "device supervisor" line as "ok"
    And "berth status" shows the daemon lease "device-ark"
    When I run "epr device supervise --stop"
    Then that ark and its resident are no longer running

  Scenario: A killed resident returns and I can see why it died
    Given my resident is ready under ark with restart policy "permanent"
    When I kill the resident with signal 9
    Then the resident becomes ready again with a different process ID
    And the supervisor still reports the same manifest CID
    And I can read a death witness naming the killed process ID and signal 9

  Scenario: I reload a changed declaration without leaving the old ark running
    Given my resident is ready under the ark started by "epr device supervise"
    When I declare new resident arguments that also produce the ready line "fixture-ready"
    Then my device's head has a new CID
    And the running supervisor still reports the previous manifest CID
    And "just status device" shows "device supervisor" as "degraded" with cure "epr device supervise"
    When I run "epr device supervise"
    Then the previous ark is no longer running
    And exactly one ark runs for my device's data root under the new head CID
    And its process ID differs from the previous ark's process ID
    And its incarnation is greater than before the reload
    And the resident is ready under the new manifest
    And "just status device" shows the "device supervisor" line as "ok"

  Scenario: A missing supervisor tells me the cure and leaves local work available
    Given my device declares a resident but no ark is running
    And every other device status check is "ok"
    When I run "just status device"
    Then the "device supervisor" line is "degraded" with cure "epr device supervise"
    And that line is never "REFUSED"
    And the status command succeeds
    When I read my declaration with "epr device show"
    Then I can see the declared resident without starting a supervisor

  Scenario: A second supervisor is refused when another ark already owns the data root
    Given an ark I started by hand with "ark run" already runs my resident from this data root
    When I run "epr device supervise" for that same data root
    Then the request is refused and names the running ark's process ID
    And the refusal tells me to stop that ark first
    And the existing ark and its resident remain running
    And no second ark starts for that data root

  Scenario: Repeating supervise at the unchanged head leaves my resident running
    Given my resident is ready under the ark started by "epr device supervise"
    And my device's head has not changed
    When I run "epr device supervise" again
    Then the command succeeds and names the existing ark and current manifest CID
    And the ark and resident keep their process IDs
    And the incarnation is unchanged
    And exactly one ark remains running for my device's data root
