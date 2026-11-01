@e2e @devflow @concern:sprint-lands-each-closure-once @act:host @requires:epr-cli @local
Feature: The landing queue — each change is checked once, against the dev it will land on
  As a developer whose workspaces and agent sessions all land their work on one shared dev branch
  I want each push checked before it takes the shared landing lock, against the dev it will actually land on
  So that these things hold:
    - the lock on dev is held for the landing alone, for seconds, never for the length of the checks;
    - a change already checked against the dev it lands on is not checked again: when dev moves under
      it, the push is refused once with "push again", and the second push lands with no check;
    - when something ahead of a change in the queue changes, only the checks whose inputs changed run
      again;
    - a queue the device cannot read is never read as an empty queue: the push checks against dev
      alone and says so;
    - a change is recorded as landed only when the server has it.

  The words this story uses, so it can be read cold.

  The repository and its checks

  - "matthew" is the fixture person. Every device and session in this story is his. The repository's
    DEV branch lives on a SERVER, here a local bare repository standing in for the forge.
  - A CHECKOUT is a working copy of the repository. One device may have several: the main one and
    LINKED WORKTREES. All checkouts of one device share one object store (the commits it has), one
    FLOW STORE and one device key. The flow store is a file of records the device only ever appends
    to, kept in a directory of the device's own state outside every checkout; the repository's
    "epr" tool reads and writes it, and the pre-push hook reaches it only through that tool.
  - A CHECK (also "gate") is the test a project must pass before its change may land. A PROJECT is a
    directory the repository declares as checked, with the command that checks it. This repository
    has one project, "app". Its check fails only when both "app/left.txt" and "app/right.txt" exist,
    and every time it runs it logs when it ran, so a scenario can count checks.
  - A commit's TREE is its content snapshot, the files and nothing else. Two commits with
    different parents or messages but identical files have the same tree.
  - Every check is identified by its KEY: a fingerprint of everything the check reads. Here "app"
    reads the whole repository tree, so any changed file changes its key. Two runs with the same key
    must give the same verdict, so the repository keeps the verdict of each key it has checked on
    this device. "app" is REUSE-ENABLED: when the key of a change matches a key already checked
    green on this device, the verdict is reused and the check does not run. "No check runs" in this
    story is said of reuse-enabled projects only.
  - The repository also lists paths that CI ignores (here everything under "docs/"). A push that
    touches only such paths, or only files no project owns, selects no check at all.
  - The PRE-PUSH HOOK runs when a session pushes to dev. It is the only thing in this story that
    checks, queues or lands.

  The queue

  - A CHANGE-ID is a line "Change-Id: I" followed by 40 hexadecimal characters that the repository's
    commit-message hook adds to each commit. It names the change across rebases, so the same change
    keeps its name when its commit is rewritten.
  - A LANDING INTENT is the record a push writes first: "this change, with this tree, checked on top
    of this base, wants to land". It holds the Change-Id, the tree, the BASE (the dev it was checked
    on top of), the change class (below) and the key of each project. Its identity is its CONTENT
    ADDRESS (CID: a content identifier, the hash-derived name the repository's tools give every
    record), computed from those fields, so raising the same intent twice yields the same record
    and the second raise appends nothing. The device writes it to its own flow store, and its peer daemon (below) tells the device's other
    copies.
  - The QUEUE is the set of live intents this device can read: its own, and those its sibling
    devices have announced. An intent is LIVE until it is landed, withdrawn, dismissed or replaced
    by a newer intent for the same change (SUPERSEDED).
  - The CHAIN of a change is the line of live intents ahead of it, each checked on top of the one
    before, starting from dev. Its PREDICTED BASE is the tree dev will have once the chain ahead
    lands: the tree of the last intent in the chain, each intent's tree already holding the ones
    beneath it. A push checks its change on top of its predicted base, so a green chain lands one by one,
    each as a fast-forward (dev moves to the pushed commit, with nothing merged in between).
  - Ties: two live intents on one base are ordered by the time each was raised, as stamped by its
    own device, when they were raised more than the policy's skew window apart (5 seconds here).
    Inside the window they are ordered by device, then by the lower CID (compared as text). One device has one
    identity, so two worktrees of one device are always ordered by CID inside the window. This is
    the queue's rule; the lock below has its own.
  - The queue never waits behind an intent of class GOVERNANCE (a change to the hook or the checks
    themselves, see below) and never behind an intent whose commit this device does not have; the
    chain ends just before such an intent. A chain also stops after 4 intents, and the push checks
    on top of the fourth.

  The two phases of a push

  - PHASE V is the checks. It holds nothing: it raises the push's intent, computes the chain and
    the predicted base, and runs the checks. When every check passes it prints
    "PRE-PUSH GATE: ALL CLEAR", the line that ends Phase V.
  - PHASE L is the landing. Only here does the push claim "dev-integration", the lock every
    checkout and device of matthew's honours before moving dev. The claim carries a time limit
    (10 minutes normally) after which it lapses even if never given back. Phase L confirms that dev
    is still the predicted base and then lets the push through to the server. The hook prints when
    it claimed the lock and how many seconds it held it, from the claim to the hook's exit. A
    separate background step of the hook gives the lock back once git's transfer has finished and
    records whether the change landed; it is observed only through the lock being free and the
    landed record appearing.
  - If dev moved after Phase V (something else landed first), the hook rebases the change onto the
    new dev in place, checks it again only where a key changed, raises its new intent, and REFUSES
    the push with "candidate moved to" the rebased commit and "push again". The hook cannot change
    which commit git is sending, so this one refusal is owed. The second push finds its own intent
    already checked and lands with no check.
  - A RESUME POINT is a reference the hook keeps for each change in flight
    ("refs/landing/" followed by the Change-Id). It is removed when the change lands.
  - LANDED means observed: the device records an intent as landed only when its commit is on dev
    on the server. A landing record says whether the change landed as it was pushed ("ff") or after
    a rebase ("moved"): "ff" means the commit was pushed as it was checked, "moved" that the hook
    rebased it first. It also says how many seconds of checks the rebase cost.
  - An intent leaves the queue by a WITHDRAW record (its push was interrupted or abandoned) or a
    DISMISS record (the change behind it found the chain red, see below), written by the device
    that raised it. A record about an intent written by any other device is ADVISORY: it is
    printed, and a push does not queue behind an intent under dispute, but the intent stays live.
  - DISMISS: when a check is red on top of the predicted base, the hook checks the change once more
    on dev alone, to tell "the chain ahead made it red" from "it is red by itself". If it is green
    on dev alone, the push dismisses the whole chain ahead of it, never one "culprit", lands on dev
    alone, and writes a dismissal record with
    the key at the predicted base, the key at dev, the failing project, the intents in the chain,
    when it last heard from each device, and the time. Nothing is reverted: a dismissed change is
    pushed again by its author, on its own.
  - CHANGE CLASS: every push has one class, the highest of the classes of the files it changes. A
    file of the hook (under ".husky/") or of the checking code (the gate modules under
    "genesis/orchestrator/"), as the checking code's own classifier declares, is of class
    GOVERNANCE, so a push touching any of them is a governance
    push as a whole, even if it changes "app" files too. A governance push is judged by the checks
    as they were where its branch left dev (the MERGE BASE), never by the changed code itself, and
    it still reuses verdicts.

  Devices

  - A DEVICE is a machine or workspace container that holds its own signing key. Here device "A"
    is matthew's first and device "B" his second; both are admitted by his ROSTER, the list of
    devices that stand for him.
  - Each device has a BERTH: the tool that decides which session may use a shared resource such as
    "dev-integration". A SESSION (a Claude or Codex agent, or a person at a terminal) MOORS with the
    berth, registering its name on that device, before it may claim anything. A granted claim is a
    LEASE naming the device and session that HOLD it. The berth's LEDGER is its append-only record
    of claims, releases and overrides. A lock "held by" a session is named by that device and
    session. Several scenarios moor a session "holder-1" whose only job is to hold the lock open,
    so that the test can arrange what happens while a push waits for it.
  - Each device runs a PEER DAEMON that announces the device's shareable records as a signed HEAD
    and FOLDS (reads and keeps) the heads of matthew's other devices. Only record kinds on the
    device's ALLOW-LIST cross between devices; a record of any other kind is counted as REFUSED and
    dropped.
  - DORMANT: an intent from another device is dormant when this device last folded that device's
    head longer ago than the policy's freshness limit (20 seconds here), judged on this device's
    own clock. A dormant intent is skipped, never waited for.
  - OVERRIDE: when another device may hold the lock but has gone silent, the hook cannot know
    whether it holds it, so it writes an override record in its berth ledger and waits until the
    override TAKES EFFECT before claiming. It takes effect once any lease the silent device could
    have taken must have lapsed: the lock's time limit, plus the skew window, plus the berth's
    FENCE MARGIN (a few seconds the berth adds for clocks that disagree), counted from the last
    time this device folded the silent device's head. This covers exactly one silent device.

  Tags and declared limits

  - "act:host" means the scenarios run on one workspace host, both devices inside one container.
    "requires:epr-cli" means they need the repository's epr tool. "wip" marks a scenario whose hook
    behaviour is not built yet; every scenario here is wip until its behaviour is observed. The tag
    "concern:sprint-lands-each-closure-once" ties them to the habit they are the proof for.
  - Not observed here, and proven by unit tests of the queue's ordering instead: the device leg of
    the tie (one device has one identity) and the limit of 4 intents (five live intents would take
    a quarter of an hour of test time).
  - Not reachable yet: a chain across devices. A sibling's commits do not reach this device before
    the push (the peer daemons share records, not commits, and the hook does not fetch them), so a
    sibling's intent always ends the chain at dev.
  - Because "app" reads the whole tree, "only the checks whose key changed" here means the one
    check runs again exactly when the tree under it changed. Narrowing a key to the files a check
    really reads is another story's (push-gate-pays-once).
  - A device that has been retired but still has push access reaches the landing, waits out its
    override, and lands; withdrawing its access is a separate act.

  Background:
    Given a fixture repository of "matthew" whose dev branch lives on a local server
    And its one checked project is "app", reuse-enabled, whose check fails only when both "app/left.txt" and "app/right.txt" exist
    And matthew's device "A" has a checkout of the repository, its own berth, its own key and its own peer daemon
    And session "claude-1" is moored on device A

  Rule: One checkout, alone (a queue of one)

    @wip
    Scenario Outline: A push is refused before any check runs, naming the cure
      Given <condition>
      When session "claude-1" pushes a change to "app/change.txt" to dev
      Then the push exits non-zero with the <refusal> line, which names the command that cures it
      And "app" was not checked
      And dev on the server has not moved

      Examples:
        | condition                                               | refusal                    |
        | "no epr tool is reachable from the push"                | "epr is required"          |
        | "the device's flow store is moved aside for the push"  | "flow store is unreachable" |

    @wip
    Scenario: A push alone raises its intent first, is checked against dev, and holds the lock only to land
      When session "claude-1" pushes a change to "app/change.txt" to dev
      Then the first queue line of the push raises a landing intent for the commit's Change-Id on dev's current tip, with a chain of 0, before any check line
      And every check of "app" finished before the push claimed "dev-integration"
      And the push reports "dev-integration" held for less than 120 seconds, a generous bound for a loaded test host
      And within 15 seconds of the push exiting, device A records the intent as landed "ff", with 0 seconds checked again
      And dev on the server is the pushed commit
      And the change's resume point is gone, and no session holds "dev-integration"

    @wip
    Scenario Outline: A push that selects no check still takes the landing
      When session "claude-1" pushes a change to <path> to dev
      Then no check runs, because <why>
      And the push still claims "dev-integration" and reports how long it held it
      And within 15 seconds of the push exiting, device A records the intent as landed

      Examples:
        | path           | why                                                           |
        | "README.md"    | "no checked project owns that file"                           |
        | "docs/note.md" | "the repository marks every file in the push as one CI ignores" |

    @wip
    Scenario: A push the server rejects is never recorded as landed
      Given the server refuses the next push to dev
      When session "claude-1" pushes a change to "app/change.txt" to dev
      Then the push exits non-zero
      And within 15 seconds no session holds "dev-integration"
      And device A has no landed record for the intent
      And the change's resume point is still there, and the intent is still live in device A's queue

    @wip
    Scenario: A change whose dev moved is rebased and checked once; the push is refused, and the second push lands with no check
      Given session "holder-1" is moored on device A and holds "dev-integration" for 600 seconds
      When session "claude-1" pushes a change to "app/change.txt" to dev
      And while that push waits for "dev-integration", another commit changing "app/other.txt" lands on dev on the server
      And session "holder-1" releases "dev-integration"
      Then the push rebases the change onto the new dev in place, checks "app" once more, and is refused with "candidate moved to" the rebased commit and "push again"
      When session "claude-1" pushes again
      Then the second push finds its own intent and raises no other, and device A's queue lists the intent once
      And "app" is not checked again
      And device A records the intent as landed "moved", with more than 0 seconds checked again
      And the landed commit carries exactly one "Change-Id:" line

    @wip
    Scenario: A change whose tree is unchanged by a move of dev is not checked again; the push is refused once and the second lands
      Given session "holder-1" is moored on device A and holds "dev-integration" for 600 seconds
      When session "claude-1" pushes a change to "app/change.txt" to dev
      And while that push waits for "dev-integration", an empty commit that changes no file lands on dev on the server
      And session "holder-1" releases "dev-integration"
      Then the push rebases the change, whose tree is the one already checked, and is refused with "candidate moved to" and "push again" without checking "app" again
      When session "claude-1" pushes again
      Then the second push lands as a fast-forward and "app" is not checked again

    @wip
    Scenario: A commit without a Change-Id lands without a place in the queue, and says how to get one
      Given session "claude-1" commits a change to "app/change.txt" with the commit-message hook skipped, so the commit has no Change-Id
      When session "claude-1" pushes it to dev
      Then the push prints that the commit has no Change-Id and no place in the queue, naming "git commit --amend" as the cure
      And the push lands
      And device A records no landing intent for it

    @wip
    Scenario: An interrupted landing withdraws its intent and keeps its resume point
      When session "claude-1" pushes a change to "app/change.txt" to dev
      And the push and its hook are interrupted once the first check line has printed
      Then the push prints that its intent was withdrawn and names "git push origin HEAD:dev" to resume
      And device A holds a withdraw record for the intent, written by device A
      And the change's resume point is still there

    @wip
    Scenario Outline: A queue the device cannot read is read as dev alone, and said so, never as an empty queue
      Given the push reaches an epr tool that <fault>
      When session "claude-1" pushes a change to "app/change.txt" to dev
      Then the push prints that the queue is unreadable, with the reason, and that it checks against dev
      And no line of the push says the queue is empty
      And the push lands

      Examples:
        | fault                                                |
        | "has no command to list landing intents"             |
        | "lists landing intents as something other than JSON" |

  Rule: Two worktrees of one device (one object store, one flow store, one identity)

    Background:
      Given device A has two linked worktrees of the repository, "wt-a" and "wt-b"
      And session "claude-1" works in "wt-a" and session "claude-2" works in "wt-b"

    @wip
    Scenario: A change in one worktree is checked on top of the other worktree's change ahead of it
      Given session "holder-1" is moored on device A and holds "dev-integration" for 600 seconds
      When session "claude-1" pushes a change to "app/a.txt" from "wt-a" to dev
      And while that push waits for "dev-integration", session "claude-2" pushes a change to "app/b.txt" from "wt-b" to dev
      Then wt-b's push raises its intent with a chain of 1, naming wt-a's intent, on the tree of wt-a's change
      And "app" is checked once for wt-b's change, with wt-a's change beneath it
      When session "holder-1" releases "dev-integration"
      Then wt-a's change lands as a fast-forward
      And dev on the server has moved under wt-b's waiting push to wt-a's commit, which is the base wt-b's change was checked on
      And wt-b's push rebases its change onto wt-a's commit, whose tree is the one already checked, and is refused once with "candidate moved to" and "push again", without checking "app" again
      When session "claude-2" pushes again from "wt-b"
      Then wt-b's change lands as a fast-forward and "app" is not checked again

    @wip
    Scenario Outline: When the intent ahead changes, a change is checked again only if its key changed
      Given session "holder-1" is moored on device A and holds "dev-integration" for 600 seconds
      And session "claude-1" has pushed a change to "app/a.txt" from "wt-a", whose push waits for "dev-integration" with its intent live
      And session "claude-2" has pushed a change to "app/b.txt" from "wt-b", whose intent chains on wt-a's and whose check of "app" ran with wt-a's change beneath it
      When wt-a's intent <fate>
      And session "claude-2" pushes from "wt-b" until its change lands
      Then "app" was checked <times> more for wt-b's change, because <why>

      Examples:
        | fate                 | times        | why                                                          |
        | "is withdrawn"       | "once"       | "the dev it lands on is no longer the one it was checked on" |
        | "lands as predicted" | "zero times" | "the dev it lands on is exactly the one it was checked on"   |

    @wip
    Scenario Outline: Two intents raised on the same dev are put in the same order in both worktrees
      Given the landing policy's skew window is 5 seconds
      When session "claude-1" in "wt-a" and session "claude-2" in "wt-b" each raise an intent on the same dev, <gap> apart
      Then device A's raise stamps on the two intents are <side> 5 seconds apart, so the timing the test arranged really happened
      And both worktrees put <first> first, and print the same chain

      Examples:
        | gap         | side        | first                                  |
        | "8 seconds" | "more than" | "the intent raised earlier"            |
        | "1 second"  | "less than" | "the intent with the lower CID"        |

    @wip
    Scenario: A change red on top of its predicted base but green on dev dismisses the chain, with a record, and reverts nothing
      Given session "claude-1" in "wt-a" has a live intent for a change adding "app/left.txt"
      When session "claude-2" in "wt-b" pushes a change adding "app/right.txt"
      Then "app" is checked twice for wt-b's change: red on top of wt-a's change, because that tree has both "app/left.txt" and "app/right.txt", then green on dev alone
      And wt-b's push prints that it dismissed the chain, naming every intent between dev and its predicted base and the dismissal record, and names no single culprit
      And the dismissal record holds the key at the predicted base, the key at dev, the failing project, the intents in the chain, when each device was last heard from, and the time
      And wt-a's intent is no longer live in either worktree, because both speak for device A
      And wt-b's change lands, and dev on the server loses no commit
      When session "claude-1" pushes again from "wt-a"
      Then it raises a new intent that names the dismissed one as superseded
      And its check of "app" fails on top of the new dev, so the conflict is now wt-a's to resolve, and nothing is reverted

    @wip
    Scenario: A change to the checks themselves is judged by the checks at the merge base, and nothing queues behind it
      When session "claude-1" in "wt-a" pushes a change that edits a comment in "genesis/orchestrator/gate-evidence.mjs" and a line of "app/README.md"
      Then the push prints "change class: governance", judged by the checks at the commit where wt-a's branch left dev, with verdict reuse on
      When session "claude-2" in "wt-b" pushes a change to "app/b.txt" while wt-a's intent is live
      Then wt-b's push skips wt-a's intent as governance-class and checks against dev alone

  Rule: Two devices of one steward

    Background:
      Given matthew's second device "B" has its own checkout of the repository, its own berth, its own key and its own peer daemon
      And matthew's roster admits both devices
      And session "codex-1" is moored on device B
      And the link between the two devices is up, and each device has folded the other's current head

    @wip
    Scenario: A lock held by the other device stops the landing after its checks, naming the holder, and the intent stays queued
      Given session "claude-1" on device A holds "dev-integration"
      When session "codex-1" on device B pushes a change to "app/b.txt" to dev
      Then B's checks run and the push prints "PRE-PUSH GATE: ALL CLEAR" before it is stopped
      And the push is stopped at the landing, needing "dev-integration", "held by" device A and session "claude-1"
      And B's intent is still live in B's queue

    @wip
    Scenario: A sibling's intent whose commit is not on this device ends the chain at dev, said so
      Given device A has a live intent for a change device B has never fetched
      When session "codex-1" on device B pushes a change to "app/b.txt" to dev
      Then B's push skips A's intent because its commit is not on this device, and checks against dev alone

    @wip
    Scenario: A dormant sibling's intent never blocks
      Given device A has a live intent
      And device A's peer daemon stops, and device B last folded A's head more than 20 seconds ago
      When session "codex-1" on device B pushes a change to "app/b.txt" to dev
      Then B's push skips A's intent as dormant
      And B's change lands

    @wip
    Scenario: A withdraw written by a device that did not raise the intent is advisory, and the intent stays
      Given device A has a live intent
      When session "codex-1" on device B writes a withdraw record for A's intent
      Then A's intent is still live in device A's queue, and both devices list B's withdraw as advisory
      And a push from device B skips A's intent, printing an advisory withdraw by device B
      And a push of A's change from device A lands

    @wip
    Scenario: A silent device is overridden at the landing only once the override takes effect, on the record
      Given device A's peer daemon stops
      And the time limit on "dev-integration" is lowered to 30 seconds for this test
      When session "codex-1" on device B pushes a change to "app/b.txt" to dev
      Then after its checks, B's push prints "override on the record" with when it takes effect and how long it waits
      And only after that wait does it print that it claimed "dev-integration"
      And B's change lands no later than 30 seconds after the override takes effect
      And B's berth ledger holds an override record naming device A

    @wip
    Scenario: A push whose own peer daemon is down is refused at the landing, naming the cure
      Given device B's peer daemon is stopped
      When session "codex-1" on device B pushes a change to "app/b.txt" to dev
      Then the push is refused at the landing, printing that this device's peer daemon is down and naming "epr peer serve"
      And dev on the server has not moved

    @wip
    Scenario: A sibling whose allow-list predates landing records counts them as refused, and this device's landing is unaffected
      Given device B's peer daemon is restarted with an allow-list that does not admit landing records
      When session "claude-1" on device A pushes a change to "app/a.txt" to dev
      Then device B's peer status counts A's landing record as refused
      And A's change lands
