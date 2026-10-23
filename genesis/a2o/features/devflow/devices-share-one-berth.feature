@e2e @devflow @concern:devices-share-one-berth @act:host @wip
Feature: Devices share one berth — a shared resource is held once across all of a person's workspaces
  As a developer who works in two development workspaces that are both my devices
  I want a claim on a resource the two workspaces share to wait until it has heard from my other device
  So that no agent session in one workspace lands work, dispatches a build or deploys while a session
    in the other workspace is already doing it, and a silent workspace is never mistaken for an idle one

  The words this story uses, so it can be read cold:

  - "matthew" is the fixture person. He develops in two WORKSPACES (development containers, each
    with its own checkout of the repository), and agent SESSIONS (Claude or Codex) work inside both.
  - A DEVICE is a machine or workspace container that holds its own signing key; the key is made on
    the device and never copied off it. Here the two workspaces are matthew's devices "A" and "B".
  - The ROSTER is matthew's list of the devices that stand for him, kept as a file in the repository.
    A device listed there is ADMITTED: its peers will talk to it. A CONTEST row on the roster names
    one device's entry as no longer valid, and that REVOKES the device.
  - The BERTH is the per-workspace tool that decides which session may use a resource. A session
    MOORS (registers itself) with the berth, then CLAIMS a resource. A granted claim is a LEASE: a
    record of who holds the resource, from which device and session, and since when. Each lease
    names its HOLDER as the device and session that claimed it.
  - A CONTAINER-SCOPED resource belongs to one workspace alone (its mesh, its build slots, its
    disk-heavy work). Each workspace has its own, so a claim on one never asks the other device.
  - A STEWARD-SCOPED resource is shared by every device of the same person (the steward). There are
    four: "dev-integration" (landing work on the shared dev branch), "jenkins-dispatch" (starting a
    build on the shared build server), "alpha-fleet" (deploying to the shared test fleet), and LANE
    claims named "lane:" followed by a lane name, which declare who is working on a piece of work.
  - Each device keeps a CHANNEL: the berth records it is allowed to share (moorings, claims, releases,
    overrides, notes), and nothing else. The device signs and announces the current state of that
    channel as a HEAD, with a sequence number that only ever grows. It announces after every change
    and at least every 30 seconds, and only to admitted devices of the same person.
  - To FOLD a head is to fetch the records it covers from the device that announced it, check each one,
    and add them to this device's own copy. A device's berth reads the other device's leases only from
    what it has folded, never by asking the other side at claim time.
  - A claim on a steward-scoped resource is EFFECTIVE only when, for every other admitted device, a
    head announced after the claim and no older than the FRESHNESS WINDOW (90 seconds) has been folded,
    and no folded lease held by another holder overlaps it. Otherwise the berth REFUSES: it exits with
    code 3 and prints one reason line, in one of two forms:
      held by <device>/<session> since <time>
      unconfirmed: <device> last seen <age> ago
    An absent device is never read as a free resource. A person may still take the lease knowingly
    with "--offline-ok"; that writes an OVERRIDE record, which the other device sees once it is back.
  - When two claims were made before either device had folded the other's, the TIE-BREAK settles
    them the same way on both devices: the earlier claim time wins; on equal times, the device whose
    signing key sorts lower wins.
  - The PRE-PUSH HOOK runs the repository's checks before a push. For a push to dev it first claims
    "dev-integration", and a refusal stops the push before any check runs.
  - The tag "act:host" means the scenario runs on a workspace host, not on a household mesh or the
    fleet. The first four scenarios run both devices inside one container, each with its own berth,
    own key and own peer daemon. The last runs them in two real workspaces, which needs the second
    machine ("shem"); without it that scenario is held, not failed.

  Background:
    Given two devices of "matthew", "A" and "B", each with its own berth, its own key and its own peer daemon
    And matthew's roster admits both devices
    And session "claude-1" is moored on device A and session "codex-1" is moored on device B
    And each device has folded the other's current head

  Scenario: A claim on a shared resource held by the other device is refused naming who holds it
    Given session "claude-1" on device A holds the steward-scoped resource "dev-integration"
    And device B has folded a head from device A announced after that claim
    When session "codex-1" on device B claims "dev-integration"
    Then the claim exits with code 3
    And the reason line reads "held by" device A and session "claude-1" since the time A claimed it
    And device B's berth status lists that lease under device A, read from B's own fold
    And a push to dev from device B is stopped before any check runs, printing the same reason line
    When session "claude-1" releases "dev-integration"
    And device B folds device A's next head
    And session "codex-1" on device B claims "dev-integration" again
    Then the claim is effective and exits with code 0

  Scenario: A silent device is read as unconfirmed, never as free
    Given no session on either device holds "dev-integration"
    And device A's peer daemon stops, and its last folded head is 3 minutes old
    When session "codex-1" on device B claims "dev-integration"
    Then the claim exits with code 3
    And the reason line reads "unconfirmed:" naming device A, last seen 3 minutes ago
    And no line of the output says the resource is free
    And device B holds no effective lease on "dev-integration"
    But a claim by "codex-1" on the container-scoped resource "cargo" is effective at once
    When session "codex-1" claims "dev-integration" again with "--offline-ok"
    Then device B holds the lease and its berth records an override
    And once device A's daemon restarts and folds B's next head, A's berth status shows B's override

  Scenario Outline: Two claims made before either device heard of the other are settled once, the same way on both
    Given neither device has folded a head from the other since the claims below were made
    When session "claude-1" on device A claims "dev-integration" at <a_time>
    And session "codex-1" on device B claims "dev-integration" at <b_time>
    And each device folds the other's next head
    Then exactly one lease on "dev-integration" is effective, and it is held by <winner>
    And the berth status on device A and on device B names that same holder
    And the losing device's berth prints the refusal naming the winner and releases its own lease

    Examples:
      | a_time         | b_time         | winner                                 |
      | "12:00:00.200" | "12:00:00.700" | "device A"                             |
      | "12:00:00.700" | "12:00:00.200" | "device B"                             |
      | "12:00:00.200" | "12:00:00.200" | "the device whose key sorts lower"     |

  Scenario: A revoked device is ignored without restarting the device that stays
    Given matthew's roster gains a Contest row naming device B's entry
    And device A reads the updated roster while its daemon keeps running
    When device B announces a head that carries a claim on "dev-integration"
    Then device A folds nothing from that head and counts it as ignored
    And device A's peer daemon refuses device B's connection exactly as it refuses a stranger's
    And device A's berth status no longer lists device B among matthew's other devices
    And a claim by "claude-1" on "dev-integration" on device A is effective at once
    And device A's peer daemon was never restarted

  @requires:shem
  Scenario: Across two real workspaces, a held integration claim is refused naming who holds it
    Given device A is the workspace on the first machine and device B the workspace on "shem"
    And each workspace's peer daemon is running and reaches the other over their own link
    And session "claude-1" on device A holds "dev-integration"
    When session "codex-1" on device B claims "dev-integration"
    Then the claim exits with code 3
    And the reason line reads "held by" device A and session "claude-1" since the time A claimed it
    And each workspace's berth status lists the other's moorings and leases, read from its own fold
    And nothing in this exchange was carried by a git push, a doorway or the person
