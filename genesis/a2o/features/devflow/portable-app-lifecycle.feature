@devflow @wip @concern:release-retention
# The lifecycle SDK and its step adapter are a separate rescue port; contract scenarios remain pending here.
Feature: An app follows its person without making the hub a dependency
  Matthew wants to open the same work from his hub or his laptop and keep working offline.
  App developers use an adapter: an implementation of the shared app lifecycle interface.
  An adapter must say when a capability is unsupported, rather than falsely reporting success.
  Lifecycle contracts distinguish held bytes, execution, authority and private continuity.
  Foundation scenarios exercise contracts; pending scenarios require real runtime delivery.

  @lifecycle-contract
  Scenario: An unavailable adapter cannot claim a running app
    Given a portable app adapter with no runtime implementation
    When a client asks it to open a release
    Then the lifecycle operation explicitly reports unsupported

  @lifecycle-contract
  Scenario: Held application bytes alone do not promise offline use
    Given a release whose artifacts are verified but whose data is not
    Then the release is not available offline

  @wip
  Scenario: Che exposes an authorized runtime without a private launcher link
    Given Matthew's workspace has device consent and permission to run Rakia
    When he runs the declared app twice
    Then both invocations identify the same running instance
    And its doorway URL opens through his ordinary authenticated browser session
    And Jessica cannot access Matthew's private view through that URL

  @wip
  Scenario: A browser moves execution local without losing work
    Given Matthew is editing in a hub-served app in an ordinary browser tab
    And he has an unsaved edit named "Water the north orchard"
    When compatible local capabilities and an explicit continuation become ready
    Then the app activates the continuation exactly once on his device
    And closing and reopening offline preserves the unsaved edit "Water the north orchard"

  @wip @concern:attention-witnessed-privately
  Scenario: The attention dashboard eagerly drains the existing background lane
    Given Matthew's authorized devices have queued private observations
    And one reachable device has declared observation stream position 10
    When he opens his attention dashboard and starts refresh A targeting position 10
    And that device declares position 20 in the same stream epoch
    And he starts refresh B targeting position 20 before refresh A finishes
    Then his local readings appear immediately
    And the same reconciliation queue is promoted to foreground priority
    And refresh A completes when the local cursor reaches position 10
    And refresh B completes when the local cursor reaches position 20
    And later arrivals do not extend either refresh target
    And the lane returns to background priority after both demands end without losing queued observations

  @wip @concern:attention-witnessed-privately
  Scenario: A dashboard refresh cannot finish an unreachable target
    Given Matthew's hub declared observation stream position 20 before becoming unreachable
    And his laptop has retained that stream only through position 10 in the same epoch
    When he refreshes his attention dashboard targeting the recorded position 20
    And the refresh reaches its declared deadline without reaching the hub
    Then the refresh ends as incomplete and releases its foreground demand
    And the dashboard shows the hub as unreachable with unknown current freshness
    And the retained observations and queued background work remain available

  @wip
  Scenario: Reclaiming an inactive app preserves personal work
    Given an inactive unpinned app has recoverable executable bytes on Matthew's hub
    And Matthew also holds a pinned release and a different running release
    When the device residency policy reclaims its executable bytes
    Then Matthew's identity and unsynced working state remain on his device
    And the app no longer advertises offline availability
    And neither the pinned release nor the running release is reclaimed

  @wip @regression @concern:attention-witnessed-privately
  Scenario: Retained progress does not make an offline device appear online
    Given Matthew's laptop retained observations through its hub's recorded stream position 10
    And his last refresh targeted that same stream position and epoch
    When the hub becomes unreachable
    Then the dashboard may show that the recorded refresh target was reached
    And it still shows the hub as unreachable with unknown current freshness
    And his retained observations remain readable
    # Constraint: retained position and current remote reachability are independent facts.
