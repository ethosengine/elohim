@devflow @wip @concern:release-retention
Feature: An app follows its person without making the hub a dependency
  Matthew wants to open the same work from his hub or his laptop and keep working offline.
  App developers use an adapter: an implementation of the shared app lifecycle interface.
  An adapter must say when a capability is unsupported, rather than falsely reporting success.
  Lifecycle contracts distinguish held bytes, execution, authority and private continuity.
  Foundation scenarios (tagged lifecycle-contract) exercise contracts; pending scenarios (tagged
  wip) require real runtime delivery.

  The lifecycle SDK and its step adapter arrive in a separate port, so the contract scenarios stay
  pending in this repository for now.

  Each scenario names which of the four facts it exercises. Held bytes are the executable and data
  a device holds; execution is where the app runs; authority is who may reach it; private
  continuity is Matthew's own work and observations following him between his devices. The
  attention-dashboard scenarios belong here under private continuity: the dashboard shows
  observations his devices wrote privately, so whether a refresh is complete, incomplete or merely
  retained decides if his work follows him honestly when the hub is unreachable.

  Glossary. Che is the Eclipse Che cloud workspace where the app can run. Rakia is an Elohim app
  run under Ark, the resident runtime that admits power and resources. A continuation is a
  private, versioned handoff that points at the existing working version; execution moves only
  once it is ready, and it is activated exactly once. A stream epoch is the publisher's boot
  epoch: it changes when the publisher restarts and positions are ordered only within one epoch.
  Observations queue for background reconciliation between his own devices; opening the dashboard
  promotes that queue to foreground priority for the duration of a refresh. A pinned release is
  one Matthew has asked to keep; a running release is one currently executing; both are protected
  from reclaim. The device residency policy is set by the device's operator and decides how long
  an unpinned app may stay inactive before its executable bytes are reclaimed.

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
  Scenario: Che exposes an authorized runtime without a private launcher link
    # Category: authority and execution.
    Given Matthew's workspace has device consent and permission to run Rakia
    And Jessica is another person with no permission on Matthew's workspace
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
    Then the dashboard may show that the recorded refresh target was reached
    And it still shows the hub as unreachable with unknown current freshness
    And his retained observations remain readable
    And retained position and current remote reachability are reported as independent facts
