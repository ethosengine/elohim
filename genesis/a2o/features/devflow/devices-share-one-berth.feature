@e2e @devflow @concern:devices-share-one-berth @act:host @wip
Feature: Devices share one berth — a shared resource is held once across all of a person's workspaces
  As a developer who works in two development workspaces that are both my devices
  I want a claim on a resource the two workspaces share to count only after my other device has seen it
  So that no agent session in one workspace lands work on dev, starts a build or deploys to the test
    fleet while a session in the other workspace is doing the same, since a collision deploys one
    side's unfinished work over the other's test or leaves a merge to untangle by hand, and so that a
    silent workspace is never mistaken for an idle one, unless a session deliberately overrides a
    silent device, which leaves a named record that binds that device until the lease ends

  The words this story uses, so it can be read cold.

  People and devices

  - "matthew" is the fixture person. He develops in two WORKSPACES (development containers, each
    with its own checkout of the repository), and agent SESSIONS (Claude or Codex) work inside both.
  - A DEVICE is a machine or workspace container that holds its own signing key; the key is made on
    the device and never copied off it. Here the two workspaces are matthew's devices "A" and "B".
  - The ROSTER is matthew's list of the devices that stand for him, kept as a file in the repository.
    A device listed there is ADMITTED: its peers will talk to it. A CONTEST row on the roster names
    one device's entry as no longer valid, and that REVOKES the device. Each device reads admission
    from the roster file in its own checkout, so a Contest written on a device takes effect there at
    once. Roster rows travel to the other devices by git, with the repository; the coordination
    records below never do.

  The berth and its resources

  - The BERTH is the per-workspace tool that decides which session may use a resource. A session
    MOORS (registers itself) with the berth, then CLAIMS a resource. A granted claim is a LEASE: a
    record of who holds the resource, from which device and session, and since when. Each lease
    names its HOLDER as the device and session that claimed it. A lease binds until its holder
    releases it or its time limit runs out.
  - "berth status" shows the moorings and leases of every admitted device, grouped by device, read
    from this device's own records and what it has folded from the others (see below).
  - A CONTAINER-SCOPED resource belongs to one workspace alone: its mesh, its build slots ("cargo")
    and its disk-heavy work. Each workspace has its own, so a claim on one never asks the other device.
  - A STEWARD-SCOPED resource is shared by every device of the same person (the steward). There are
    four: "dev-integration" (landing work on the shared dev branch), "jenkins-dispatch" (starting a
    build on the shared build server), "alpha-fleet" (deploying to the shared test fleet), and LANE
    claims named "lane:" followed by a lane name, which declare who is working on a piece of work.
    Lane claims use the same mechanism as the other three; this story does not test them separately.

  How the two devices stay in step

  - Each device keeps a CHANNEL: the subset of its berth's records that it may share (moorings,
    claims, releases, overrides, notes), and nothing else.
  - Each device runs a PEER DAEMON. It announces the device's channel as a signed HEAD, with a
    sequence number that only ever grows, after every change and at least every 30 seconds. It serves
    the records a head covers to admitted devices of the same person and to no one else. And it FOLDS
    the other device's heads: it fetches the records a head covers, checks each one, and adds them to
    this device's own copy. A berth reads the other device's leases only from what it has folded.
  - Every head also carries an ACKNOWLEDGEMENT: for each other device, the newest of that device's
    heads this device has folded. A device that folds a new head announces its own head again within
    about a second, so while both are up an acknowledgement comes back within a couple of seconds.
  - The two peer daemons talk over their own LINK: directly to each other, or through a relay that
    forwards only encrypted traffic it cannot read.
  - "epr peer status" prints one line per other device: its last head, how old that head is, how many
    records were folded from it, and how many of its connections were refused.

  Claims on a shared resource

  - A claim on a steward-scoped resource first writes a PENDING lease, which the device's next head
    carries to the other device. The claim then waits up to 10 seconds for every other admitted
    device to acknowledge that head. It becomes EFFECTIVE, and the command exits 0, when every other
    device has acknowledged it and no folded lease held by another holder overlaps it, unless the
    tie-break below gives this claim precedence. Otherwise the berth REFUSES: it exits with code 3,
    releases the pending lease, and prints one reason line in one of two forms:
      held by <device>/<session> since <time>
      unconfirmed: <device> last seen <age> ago
    An absent device is never read as a free resource. When only one device is admitted, there is no
    one to wait for, and a claim is effective at once.
  - Two claims are CONCURRENT when each was made before its device had folded the other's. The
    TIE-BREAK settles them the same way on both devices: the earlier claim time wins; on equal times,
    the device whose signing key sorts lower wins. Each claim carries the claim time its own device
    stamped on it, and both devices compare those same recorded values, never their own clocks.
  - A session may take a lease refused as "unconfirmed" anyway with "--offline-ok"; it never overrides
    a "held by" refusal. An agent session may do so on its own judgment. The safeguard is the
    OVERRIDE record it writes: the record reaches the other device when it is back, and from then on
    that device's berth refuses its own sessions on that resource, naming the overriding device and
    session, until the overriding lease is released or runs out.

  Pushes, builds and deploys

  - A BUILD TAG is a marker in a commit message, such as "[build:edge]" or "[build:app]", that asks
    the shared build server to build when the commit reaches dev; "[build:edge]" also deploys to the
    test fleet. The PRE-PUSH HOOK runs the repository's checks before a push. For a push to dev it
    first claims "dev-integration", then "alpha-fleet" when a pushed commit carries "[build:edge]",
    and "jenkins-dispatch" when a pushed commit carries any build tag. A refusal stops the push before
    any check runs, and the hook releases whatever it had claimed. The hook holds its claims only for
    the push itself; a session that watches a build or a deploy through to the end takes its own
    explicit claim on that resource for as long as it watches.

  Tags

  - The tag "act:host" means the scenario runs on a workspace host, not on a household mesh or the
    fleet. All scenarios but the last run both devices inside one container, each with its own berth,
    own key and own peer daemon. The last runs them in two real workspaces, which needs the second
    machine ("shem"); without it that scenario is held, not failed.
  - The tag "wip" marks scenarios whose steps are not implemented yet. The tag
    "concern:devices-share-one-berth" ties these scenarios to the habit they are the proof for.

  Background:
    Given two devices of "matthew", "A" and "B", each with its own berth, its own key and its own peer daemon
    And matthew's roster admits both devices
    And session "claude-1" is moored on device A and session "codex-1" is moored on device B
    And the link between the two devices is up, and each device has folded the other's current head

  Scenario: A claim on a shared resource held by the other device is refused naming who holds it
    Given session "claude-1" on device A holds the steward-scoped resource "dev-integration"
    And device B has folded the head from device A that carries that lease
    When session "codex-1" on device B claims "dev-integration"
    Then the claim exits with code 3
    And the reason line reads "held by" device A and session "claude-1" since the time A claimed it
    And device B's berth status lists that lease under device A, read from B's own fold
    When session "claude-1" releases "dev-integration"
    And device B folds the head from device A that carries the release
    And session "codex-1" on device B claims "dev-integration" again
    Then the claim waits until device A's head acknowledges it, within 10 seconds, and exits 0

  Scenario Outline: A push that would land, build or deploy is stopped while the other device holds what it needs
    Given session "claude-1" on device A holds the steward-scoped resource <resource> because <held_because>
    When session "codex-1" on device B pushes to dev commits that carry <build_tag>
    Then the pre-push hook stops the push before any check runs
    And it prints the reason line "held by" device A and session "claude-1" for <resource>
    And no commit from device B reaches dev
    And device B holds none of "dev-integration", "alpha-fleet" or "jenkins-dispatch" afterwards

    Examples:
      | resource           | held_because                                                  | build_tag      |
      | "dev-integration"  | "its own push to dev is running its checks"                   | "no build tag" |
      | "alpha-fleet"      | "it claimed it explicitly to watch a deploy through to the end" | "[build:edge]" |
      | "jenkins-dispatch" | "it claimed it explicitly to watch a build through to the end"  | "[build:app]"  |

  Scenario: A silent device is read as unconfirmed, never as free
    Given no session on either device holds "dev-integration"
    And device A's peer daemon stops, and the last head device B folded from A is 3 minutes old
    When session "codex-1" on device B claims "dev-integration"
    Then the claim waits 10 seconds for device A's acknowledgement and exits with code 3
    And the reason line reads "unconfirmed:" naming device A, last seen 3 minutes ago
    And no line of the output says the resource is free
    And device B holds no lease on "dev-integration"
    But a claim by "codex-1" on the container-scoped resource "cargo" is effective at once

  Scenario: An override taken while the other device is silent binds that device until the lease ends
    Given no session on either device holds "dev-integration"
    And device A's peer daemon stops, and the last head device B folded from A is 3 minutes old
    When session "codex-1" on device B, on its own judgment, claims "dev-integration" with "--offline-ok"
    Then the claim exits 0 and device B's berth records an override naming device B and session "codex-1"
    When device A's peer daemon starts again and folds device B's next head
    Then device A's berth status shows device B's override on "dev-integration"
    And a claim by "claude-1" on "dev-integration" on device A exits 3 with the reason line "held by" device B and session "codex-1"
    And once "codex-1" releases "dev-integration" and device A folds that release, a claim by "claude-1" on device A exits 0

  Scenario Outline: Two concurrent claims are settled once, the same way on both devices
    Given device A's signing key sorts lower than device B's
    And the link between the two devices is down
    When session "claude-1" on device A claims "dev-integration", stamped <a_time> by device A
    And session "codex-1" on device B claims "dev-integration", stamped <b_time> by device B
    And the link comes back before either claim's 10-second wait ends, and each device folds the other's head
    Then the claim on <winner> exits 0 and the claim on the other device exits 3
    And the losing claim's reason line reads "held by" <winner> and names its session
    And the losing device holds no lease on "dev-integration"
    And the berth status on device A and on device B names the same holder

    Examples:
      | a_time         | b_time         | winner     |
      | "12:00:00.200" | "12:00:00.700" | "device A" |
      | "12:00:00.700" | "12:00:00.200" | "device B" |
      | "12:00:00.200" | "12:00:00.200" | "device A" |

  Scenario: A retired device stops counting without restarting the device that stays
    Given device B is an old workspace whose work matthew has moved to device A, and it is still running
    And session "codex-1" on device B still holds "alpha-fleet", left over from a deploy it was watching
    And device A has folded that lease, so a claim by "claude-1" on "alpha-fleet" on device A is refused naming device B
    When matthew, on device A, contests device B's entry, signing the Contest row on A and writing it into A's own roster file
    Then device A's berth status no longer lists device B or its lease on "alpha-fleet", with no pull and no restart
    And a claim by "claude-1" on "alpha-fleet" on device A exits 0 at once, with no other admitted device to wait for
    And device A drops device B's connections and refuses new ones exactly as it refuses a stranger's
    And "epr peer status" on device A counts the refused connections from device B
    And a push to dev from session "codex-1" on device B is refused with the reason line "unconfirmed:" naming device A, because A no longer acknowledges B
    And device A's next push to dev is not blocked, and it carries the Contest row to matthew's other devices
    And device A's peer daemon was never restarted, so the sessions moored on A and the leases they hold were not interrupted

  @requires:shem
  Scenario: Across two real workspaces, a held integration claim is refused naming who holds it
    Given device A is the workspace on the first machine and device B the workspace on "shem"
    And matthew's roster, admitting both devices, reached each workspace through its checkout
    And each workspace's peer daemon reaches the other over their own link
    And session "claude-1" on device A holds "dev-integration"
    When session "codex-1" on device B claims "dev-integration"
    Then the claim exits with code 3
    And the reason line reads "held by" device A and session "claude-1" since the time A claimed it
    And each workspace's berth status lists the other's moorings and leases, read from its own fold
    And no coordination record in this exchange was carried by a git push, by a shared server that can read or keep it, or by the person
    But an encrypted relay that only forwarded the link's traffic is allowed
