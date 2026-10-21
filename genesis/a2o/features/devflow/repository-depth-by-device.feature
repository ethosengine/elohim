# What the tags below mean, for a reader meeting them here first:
# @e2e and @devflow route this file into the developer-workflow suite.
# @act:host — these scenarios run on one workspace host and need no household peer. The two
# devices and the coding collective's hub are each played by a separate store on that host.
# @requires:brit-cli — every scenario runs `brit`, the native developer tool; the suite holds the
# scenarios (recognised and skipped, never failed) when the binary is absent.
# @concern:brit-native-daily-driver joins this proof to the habit of that name
# (elohim/brit/.epr-meta/brit-native-daily-driver.habit.md): a developer works through brit day to day.
#
# HELD. No step definitions exist; every scenario is @wip. What it waits on: brit cannot yet open a
# repository partially and fetch a file's contents the first time that file is read, and no device yet
# makes its own first guess of how much of a repository to hold. In the design
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (sections 7.2 and 12.7) those are slice 7 of section 17 (brit opening by following the current
# version and fetching only what is read), standing on slice 3 (contents that arrive on first read)
# and slice 5 (the device's own first guess).
@e2e @devflow @act:host @requires:brit-cli @concern:brit-native-daily-driver
Feature: One repository opens partial on a Chromebook and as a full clone on a workstation, with no setting changed, and either can change later without starting over

  Matthew Dowell writes code for the Elohim repository, which is large: tens of thousands of files
  and years of history. He works from two devices. His Chromebook has a small disk and often runs
  on battery. His workstation at home has plenty of room and is always plugged in. He belongs to the
  repository's coding collective, and the collective's hub holds the whole repository and its history.

  With ordinary tools a developer has to know, before the first download, whether to copy
  everything or only part, and to choose the right flags for each machine. A wrong guess means a
  full disk on the Chromebook, or a workstation that cannot work offline; fixing it means deleting
  the copy and downloading again. The promise here is that Matthew types the same command on both
  devices, with no flags, and each opens the repository in the way that suits it, and that either
  can later hold more or less of it without starting over.

  Some words, defined once:
  - A coding collective is the group of developers who work on one repository together.
  - The collective's hub is an always-on machine the collective keeps. It holds the whole repository
    and its whole history, and answers a device that asks it for part of either.
  - brit is the developer tool used here. It is built on git, and plain git still works in every
    copy it opens.
  - A repository's address names the repository and the coding collective that keeps it. brit
    reaches the collective's hub from that address, so running `brit clone` with the address is
    what connects a device to the hub.
  - Opening a repository means running `brit clone` with the repository's address and no other
    options. The command is called clone whichever way the copy ends up held, partial or full.
    Opening a file is different: it means reading the file's contents, or editing them, in the
    working copy, by Matthew or by any tool he runs. Listing a file's name or path is not opening it:
    `ls`, or `find` by name, opens nothing. Reading what is inside is: `cat`, an editor, a build, a
    `grep` through contents, or `git diff`, each opens every file whose contents it reads. The line
    brit holds is between a file's bytes and everything else about it: anything that reads only a
    file's name, its path or other details about it, as `ls`, `find` by name and `stat` do, fetches
    no contents, whichever tool does the reading; anything that reads the file's bytes opens it.
  - A working copy is the folder on a device where Matthew reads and edits the repository.
  - Full clone: the whole repository and its whole history are on the device, and stay there.
    Everything works with no network.
  - Partial: every file of the repository appears in the working copy's folders under its own name
    and path, as a placeholder holding none of its contents. A placeholder is a marker in the
    folder carrying the file's name and path and none of its bytes; none of those bytes are stored
    anywhere on the device, and brit reports the file as not yet arrived. Looking through the
    folders brings no contents; the first time a file is opened, its contents arrive and replace the
    placeholder, and stay on the device afterwards. A file's contents are released when its bytes
    are removed from the device and the file becomes a placeholder again. Only the version of a file at the current commit arrives this
    way. Older versions of files are not held on the device,
    so a question about history (who last changed a line, how a file changed over time) is asked of
    the collective's hub, which answers from the history it holds.
  - Changing how much a device holds is a request to brit, and never means starting over. Asking brit
    to keep the whole repository on a device turns a partial copy into a full clone in the same
    working copy. Asking brit to hold the repository partially turns a full clone into a partial copy,
    and the request names which files' contents to keep; every other file's contents, and the older
    history the hub also holds, are released. One way to name the files to keep is by time: the files
    opened on that device within a span Matthew gives, such as the past seven days. brit records on
    each device when each file was last opened there, and that record stays on the device. When brit
    tells Matthew how a copy was opened, it shows him the command that would make the other request.
  - A fingerprint is a short code computed from exact bytes. Each commit's files, taken together,
    have one fingerprint, called the commit's root; the same commit has the same root everywhere.
  - A device's facts are what it knows about itself: free disk, whether it is on battery, whether
    it stays on. brit's first guess of how to open a repository is made on the device from those
    facts and from what the collective's hub holds: a partial copy is guessed only when the hub holds
    the whole repository and its history, so that it can answer for anything the device lacks.
    Matthew can always override the guess with either request described above. A device's facts never leave it for the hub: the hub keeps
    a record of every request a device sends it, and that record is how the scenarios below check that
    no free-disk, battery or stays-on value reached the hub.

  Background:
    Given the coding collective's hub holds the whole Elohim repository and its history
    And the Elohim repository's address leads brit to the collective's hub
    And Matthew has chosen no setting about how much of a repository to hold, on either device

  @wip
  Scenario: On the Chromebook the repository opens partial, and Matthew is told why and how to change it
    Given the Chromebook has little free disk and is running on battery
    When Matthew runs `brit clone` with the Elohim repository's address on his Chromebook
    Then every file of the repository appears in the working copy under its own name and path
    And no file's contents have arrived on the Chromebook yet
    And brit tells Matthew the copy is partial because the Chromebook has little free disk and is on battery, and the collective's hub holds the whole repository and its history to answer from
    And brit shows Matthew the command that asks it to keep the whole repository on the Chromebook
    And the hub's record of the Chromebook's requests holds no value for its free disk, its battery or whether it stays on
    And plain git reports the working copy's current commit and its changes
    When Matthew opens the repository's README for the first time
    Then the README's contents arrive from the collective's hub and stay on the Chromebook
    And no other file's contents have arrived

  @wip
  Scenario: On the workstation the same command opens a full clone that works with no network
    Given the workstation has plenty of free disk, is plugged in, and stays on
    When Matthew runs `brit clone` with the Elohim repository's address on his workstation, the same command he ran on the Chromebook
    Then the whole repository and its whole history are on the workstation
    And brit tells Matthew the copy is a full clone because the workstation has room and stays on
    And the hub's record of the workstation's requests holds no value for its free disk, its battery or whether it stays on
    When the workstation's network is switched off
    Then Matthew can see who last changed each line of any file, and the history of any file
    And plain git reports the working copy's current commit and its changes

  @wip
  Scenario: Both copies are the same repository, and a question about history gets the same answer on both
    Given Matthew's Chromebook holds the Elohim repository partially, with none of its older history on the device
    And Matthew's workstation holds the Elohim repository as a full clone, with its whole history
    And both working copies are at the same commit
    When Matthew asks each device for that commit's root
    Then the Chromebook and the workstation give the same fingerprint
    When Matthew asks on each device who last changed line 10 of the repository's README
    Then both devices give the same answer
    And the workstation answered from its own history, and the Chromebook asked the collective's hub

  @wip
  Scenario: The Chromebook's partial copy becomes a full clone without starting over, and Matthew's edits stay
    Given Matthew's partial copy on the Chromebook holds the contents of 40 files he has opened
    And Matthew has edited 2 of those files and not yet committed the edits
    When Matthew runs the brit command that asks it to keep the whole repository on the Chromebook, the command brit showed him when the copy was opened
    Then the same working copy now holds the whole repository and its whole history
    And Matthew's 2 uncommitted edits are unchanged
    And the 40 files already on the Chromebook were not downloaded again

  @wip
  Scenario: The workstation's full clone falls back to partial without starting over, and its unpushed commits, their files and the uncommitted edit stay
    Given Matthew's full clone on the workstation has 3 commits the collective's hub does not yet have
    And Matthew has 1 uncommitted edit on the workstation
    And Matthew opened 25 of the repository's files on the workstation in the past seven days
    When Matthew asks brit to hold the repository partially on the workstation, naming as the files to keep those he opened there in the past seven days
    Then the contents of those 25 files stay on the workstation
    And the contents of every other file, and the older history the hub also holds, are released from the workstation
    And the 3 commits the hub does not have, and the files they change, stay on the workstation
    And Matthew's uncommitted edit is unchanged
    When Matthew opens a file whose contents were released
    Then its contents arrive again from the collective's hub, the same bytes as before
