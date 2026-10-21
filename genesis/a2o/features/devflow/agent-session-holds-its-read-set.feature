# What the tags below mean, for a reader meeting them here first:
# @e2e and @devflow route this file into the developer-workflow suite.
# @act:host — these scenarios run on one workspace host and need no household peer. The session's
# machine and the coding collective's hub are each played by a separate store on that host.
# @requires:brit-cli — every scenario runs `brit`, the native developer tool; the suite holds the
# scenarios (recognised and skipped, never failed) when the binary is absent.
# @concern:brit-native-daily-driver joins this proof to the habit of that name
# (elohim/brit/.epr-meta/brit-native-daily-driver.habit.md): developers, and the agents who work
# beside them, work through brit day to day.
#
# HELD. No step definitions exist; every scenario is @wip. The design is
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (sections 12.2 and 12.7, whose probe this story states). The story waits on two steps of that
# design's build order (its section 17), each glossed in plain words:
#   slice 3 = a folder's names and a file's contents are fetched from the hub the first time they are
#             read, instead of the whole repository being copied up front;
#   slice 7 = brit works natively: an agent session is a draft laid over the repository, sealed or
#             thrown away, with the probe this story states. Slice 7 stands on slice 3.
@e2e @devflow @act:host @requires:brit-cli @concern:brit-native-daily-driver
Feature: An agent working in a large repository holds only the files it read and the changes it made, and its work stays a draft until a person takes it

  Matthew Dowell works on the Elohim repository, which is large: tens of thousands of files. He
  often asks an AI agent to do a piece of work in it, such as fixing one bug. The agent is not a
  person. Matthew sponsors it: it runs on a machine under his account, with a key of its own, and
  what it may do is bounded by a grant, a record Matthew signs that names the agent's key and the
  work it may do. This story does not test that boundary: the story of a sponsored runner
  (genesis/a2o/features/compute/ci-runner-stewarded.feature, concern operator-runtime-surface)
  proves that work outside a grant is refused. Here the grant matters because a sealed draft names
  it, so anyone can check who sponsored the agent that made the draft: they read the grant record
  the draft points to and see that Matthew signed it and that it names the key the draft is signed
  with.

  A fix like that usually means reading a dozen files and changing two or three. If every agent
  session began by copying the whole repository, each would cost minutes and gigabytes before it
  read a line, and ten sessions at once would cost ten copies. The promise here is that an agent
  session costs what it reads and what it changes, and nothing more. The second promise is that the
  agent's changes are a draft: they become part of the repository only when Matthew takes them, and
  a session that went wrong can be thrown away without leaving anything in the repository, and
  without leaving on its machine the files it read or the changes it made.

  Some words, defined once:
  - The coding collective is Matthew and the people he builds the repository with. Its hub is the
    one always-on store they share. It keeps every file and folder of the repository under its
    fingerprint and serves them one piece at a time, on demand: a session fetches only the pieces
    it asks for, when it asks, the way a content-addressed store hands out one object by its
    fingerprint. The hub plays the part a shared remote plays for a team using git, but cloning
    from a git remote copies everything in bulk before work starts; from the hub, nothing reaches a
    session before the session asks for it.
  - A machine holds a file's contents, or a folder listing, when those bytes are on that machine's
    own storage. A piece the session could fetch from the hub but has not is reachable, not held.
  - A session is one piece of work by the agent, from its start to the moment its draft is kept or
    thrown away. Throwing a session away is one act: at that moment its machine gives up both the
    session's draft and every piece the session fetched from the hub; nothing is left for a later
    session to clean up. A fresh session starts with nothing left over from any earlier one.
    Sessions are run by brit, the native developer tool that Matthew and his agents work through.
  - A session is started over one commit of the repository: it sees that commit's files exactly as
    they are, and its own changes lie on top of them, kept apart, so the commit underneath is never
    altered by what the session does.
  - A working copy is the folder where the session reads and edits the repository. In a session,
    every file of the repository can be reached from it. Starting a session fetches exactly one
    piece: the top folder's listing, because every search for a file begins there. After that, a
    folder's names arrive only when the agent first looks inside that folder, and a file's contents
    only when the agent first opens it.
  - A folder listing records the names of the files and folders inside one folder. A file's spine is the chain of folder listings
    from the repository's top folder down to that file; it is what is needed to find the file.
    A spine is a handful of lists of names, usually far smaller than the file it leads to, and
    files in nearby folders share most of their spines, so 12 spines cost little beside 12 files.
  - A fingerprint is a short code computed from exact bytes. A folder listing's fingerprint covers
    the fingerprints of everything inside it, so the top folder's fingerprint names the whole
    repository exactly.
  - A commit is one recorded state of the repository, named by its top folder's fingerprint. The
    repository's current commit is the one it stands at now; it moves only when someone takes
    changes into the repository.
  - The draft is the set of changes the session made. It is kept apart from the repository.
  - Sealing a draft gives it a root: one fingerprint naming the repository as it would be with the
    draft's changes in it. Every file and folder the draft did not change keeps the fingerprint it
    already had, so sealing stores anew only the changed files and the folder listings above them.

  Background:
    Given the coding collective's hub holds the whole Elohim repository at its current commit
    And Matthew has sponsored an agent to fix a bug in the repository

  @wip
  Scenario: A fresh session can reach the whole repository and holds only its top folder listing
    When the agent starts a fresh session over the repository at its current commit
    Then every file of the repository can be reached from the session's working copy
    And starting the session fetched exactly one piece from the hub: the repository's top folder listing
    And the session's machine holds that listing
    And it holds no other folder listing and no file's contents

  @wip
  Scenario: After reading 12 files and changing 3, the session holds those files, their spines and its changes, and nothing else
    Given the agent has started a fresh session over the repository at its current commit
    When the agent reads 12 files in the repository
    And the agent changes 3 of those files
    Then the session fetched each of the 12 files' contents from the hub when the agent first opened that file, and not before
    And it fetched each folder listing below the top folder on those 12 files' spines when the agent first looked inside that folder, and not before
    And it fetched nothing else from the hub
    And the session's machine holds the contents of the 12 files it read
    And it holds the folder listings on those 12 files' spines
    And it holds the agent's changes to the 3 files in the session's draft
    And it holds no other file's contents and no other folder listing of the repository

  @wip
  Scenario: Sealing the draft stores only the changes and leaves the repository where it was
    Given the agent has read 12 files and changed 3 of them in a session
    When the agent seals the session's draft
    Then the draft has a root of its own
    And sealing stored new copies of only the 3 changed files and the folder listings on their spines
    And the draft is signed with the agent's own key, not with Matthew's
    And the draft names Matthew's grant to that key
    And anyone who reads the grant record the draft names sees that Matthew signed it and that it names the key the draft is signed with
    And the repository's current commit is still the one the session started from

  @wip
  Scenario: Matthew takes some of a sealed draft's changes and leaves the rest
    Given the agent has sealed the draft of a session in which it changed 3 files
    When Matthew takes the changes to 2 of the 3 files into the repository and leaves the third
    Then the repository has a new current commit
    And that commit contains the changes to those 2 files and not the change to the third
    And the third file in that commit is as it was in the commit the session started from

  @wip
  Scenario: A session that went wrong is thrown away cleanly
    Given the agent has read 12 files and changed 3 of them in a session
    When Matthew throws the session away
    Then the repository's current commit is still the one the session started from
    And none of the agent's changes appears in the repository or in Matthew's own working copy
    And as soon as the session is thrown away, its machine no longer holds the agent's changes
    And by that same act its machine no longer holds the 12 files the agent read or any folder listing it fetched
