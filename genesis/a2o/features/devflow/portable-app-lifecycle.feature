@devflow @concern:release-retention
Feature: An app follows its person without making the hub a dependency
  Matthew wants to open the same work from his hub or his laptop and keep working offline.
  App developers use an adapter, which is an implementation of the shared app lifecycle
  interface. An adapter must say when a capability is unsupported rather than falsely
  reporting success. Lifecycle contracts distinguish four facts about an app on a device:
  held bytes, execution, authority, and private continuity.

  Held bytes are the executable and data a device stores for an app. Execution is which
  runtime is running the app and where. Authority is who may reach the running app. Private
  continuity is whether Matthew's own work and observations follow him between his devices
  honestly, even when a device is unreachable.

  The attention-dashboard scenarios belong here under private continuity. The dashboard shows
  observations his devices wrote privately, so whether a refresh is complete, incomplete, or
  merely retained decides whether his work follows him when the hub is unreachable.

  Foundation scenarios (tagged lifecycle-contract) exercise adapter contracts against the
  lifecycle interface using a programmatic client rather than a named person, because the
  contract under test is the adapter's response, not a human workflow. Pending scenarios
  (tagged wip) require a real runtime. Each scenario carries a category comment identifying
  which of the four facts it exercises.

  Glossary.

  A hub is a remote server that runs applications on Matthew's behalf and stores his
  observations. When the scenarios say "the hub is unreachable" they mean the server cannot
  be contacted from Matthew's current device; his locally retained work is unaffected.

  Ark is the resident runtime on a device, a program that runs packaged applications and
  governs their access to the device's processing time, memory, and storage. "Admits power
  and resources" means Ark decides which applications may draw on the device and how much
  they may use.

  Rakia is an application for the Elohim protocol (a distributed learning platform),
  packaged to run under Ark. When a scenario says Matthew has permission to run Rakia, it
  means the Ark instance on his workspace has been configured to allow that application to
  execute. When a step refers to "the declared app" it means the application named in
  the scenario's preceding Given; in the authority scenario, the declared app is Rakia.

  Che is an Eclipse Che cloud development workspace, the hosted environment where Matthew
  develops and runs applications remotely. The authority scenario uses Che to distinguish a
  workspace-hosted runtime (reachable only through an authenticated session) from an
  ordinary browser tab.

  A doorway is the authenticated gateway a person passes through to reach a running
  application on a workspace. A doorway URL is the network address of that gateway. The
  authority scenario asserts that the running app is reachable only through Matthew's own
  authenticated doorway session and not through any other path.

  Device consent is the device operator's recorded agreement that the device may run
  applications under Ark. A workspace that has device consent and permission to run a
  particular app satisfies both the device-level and the app-level prerequisites for
  execution.

  Within a release's held bytes, artifacts are the application's own executable files and
  data is the working content the application needs to function. Verifying the artifacts
  confirms the executable is intact; verifying the data confirms the content is ready for
  use. The held-bytes scenario asserts that verified artifacts alone, without verified data,
  do not make a release available offline.

  A private view is a person's own running application session. When the scenario says
  Jessica cannot access Matthew's private view it means she cannot see or interact with his
  running session through the same URL.

  A continuation is a private, versioned handoff that records a pointer to the existing
  working version of an app. The app moves execution to the continuation only once it is
  ready, and it activates each continuation exactly once. An explicit continuation is one
  that a person or their system deliberately creates, as distinct from one the platform
  creates automatically. The scenario states that an explicit continuation becomes ready
  alongside compatible local capabilities; it does not specify who initiates the
  continuation.

  Compatible local capabilities are the runtime, storage, and other device-level resources
  the local device needs before it can take over execution from a hub-served app. The
  scenarios do not enumerate a fixed list of capabilities; the adapter reports whether the
  local device satisfies what the particular app requires.

  A stream position is a numbered point in the sequence of observations a device has
  published within its current epoch, and a device declares a stream position by announcing
  the latest observation it holds at that number. A stream epoch is the publisher's boot
  epoch: it changes when the publisher restarts, and positions are ordered only within one
  epoch. Two different epochs cannot be compared by position number alone.

  A local cursor is the highest stream position a receiving device has obtained from
  another device's published observations within the same epoch. Stream position is what
  the publisher declares; local cursor is where the receiver stands relative to that
  declaration. A refresh completes when the local cursor reaches the refresh's target
  stream position.

  Freshness is whether a device's last declared stream position still reflects its
  current state. When a device is unreachable, its most recently declared position is
  retained, but its current freshness is unknown because the device may have published
  further observations since contact was lost.

  Observations queue for background reconciliation between Matthew's own devices. A lane is
  the reconciliation channel that carries that queue. Opening the dashboard promotes the lane
  to foreground priority for the duration of a refresh; when all pending refreshes end, the
  lane returns to background priority without losing queued observations. When a scenario
  says a refresh releases its foreground demand, it means the refresh has ended and the lane
  loses the foreground priority that refresh held.

  Local readings are the retained observations already present on a device. They appear
  without waiting for the reconciliation lane because no network round-trip is needed.

  A refresh deadline is the time limit a refresh sets for itself when it starts. If the
  refresh has not reached its target stream position by that time, it ends as incomplete.
  The deadline belongs to the refresh, not to the person or to the hub.

  A pinned release is one Matthew has asked to keep. A running release is one currently
  executing. Both are protected from reclaim. Recoverable executable bytes are stored
  application files the device may free when the residency policy permits; reclaiming them
  removes offline availability but preserves Matthew's identity and unsynced working state.

  The device residency policy is set by the device's operator and decides how long an
  unpinned, inactive app may stay before its executable bytes are reclaimed.

  @lifecycle-contract
  Scenario: An adapter with no runtime reports unsupported instead of opening a release
    # Category: execution.
    Given a portable app adapter with no runtime implementation
    When a client asks it to open a release
    Then the lifecycle operation explicitly reports unsupported

  @lifecycle-contract
  Scenario: Held application bytes alone do not promise offline use
    # Category: held bytes.
    Given a release whose artifacts are verified but whose data is not
    When a client asks whether the release is available offline
    Then the release is not available offline

  @wip
  Scenario: Che exposes an authorized runtime only through Matthew's authenticated session
    # Category: authority and execution.
    Given Matthew's workspace has device consent and permission to run Rakia
    And Jessica is another person with no permission on Matthew's workspace
    # "The declared app" is Rakia, named in the preceding Given.
    When he runs the declared app twice
    Then both invocations identify the same running instance
    And its doorway URL opens through his ordinary authenticated browser session
    And Jessica cannot access Matthew's private view through that URL

  @wip
  Scenario: A browser moves execution local without losing work
    # Category: execution and private continuity.
    Given Matthew is editing in a hub-served app in an ordinary browser tab
    And he has an unsaved edit named "Water the north orchard"
    When compatible local capabilities and an explicit continuation become ready
    Then the app activates the continuation exactly once on his device
    And closing and reopening offline preserves the unsaved edit "Water the north orchard"

  @wip @concern:attention-witnessed-privately
  Scenario: The attention dashboard eagerly drains the existing background lane
    # Category: private continuity.
    Given Matthew's authorized devices have queued private observations
    And one reachable device has declared observation stream position 10
    When he opens his attention dashboard and starts refresh A targeting position 10
    And that device declares position 20 in the same stream epoch
    And he starts refresh B targeting position 20 before refresh A finishes
    Then his local readings appear immediately
    And the queue that reconciles his observations between devices is promoted to foreground priority
    And refresh A completes when the local cursor reaches position 10
    And refresh B completes when the local cursor reaches position 20
    And later arrivals do not extend either refresh target
    And the lane returns to background priority after both demands end without losing queued observations

  @wip @concern:attention-witnessed-privately
  Scenario: A dashboard refresh cannot finish an unreachable target
    # Category: private continuity.
    Given Matthew's hub declared observation stream position 20 before becoming unreachable
    And his laptop has retained that stream only through position 10 in the same epoch
    When he refreshes his attention dashboard targeting the recorded position 20
    And the refresh reaches its declared deadline without reaching the hub
    Then the refresh ends as incomplete and releases its foreground demand
    And the dashboard shows the hub as unreachable with unknown current freshness
    And the retained observations and queued background work remain available

  @wip
  Scenario: Reclaiming an inactive app preserves personal work
    # Category: held bytes and private continuity.
    Given an inactive unpinned app has recoverable executable bytes on Matthew's hub
    And Matthew also holds a pinned release and a different running release
    When the device residency policy reclaims its executable bytes
    Then Matthew's identity and unsynced working state remain on his device
    And the app no longer advertises offline availability
    And neither the pinned release nor the running release is reclaimed

  @wip @regression @concern:attention-witnessed-privately
  Scenario: Retained progress and current remote reachability are independent facts
    # Category: private continuity.
    Given Matthew's laptop retained observations through its hub's recorded stream position 10
    And his last refresh targeted that same stream position and epoch
    When the hub becomes unreachable
    # The assertion: unreachability does not suppress the completed-refresh record.
    # "May show" means the dashboard is not prevented from displaying it; a test runner
    # passes when the record is present and fails when unreachability hides it.
    Then the dashboard may show that the recorded refresh target was reached
    And it still shows the hub as unreachable with unknown current freshness
    And his retained observations remain readable
    # Neither reading suppresses the other.
    And retained position and current remote reachability are reported as independent facts
