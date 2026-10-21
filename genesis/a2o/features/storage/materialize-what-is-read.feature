# What the tags below mean, for a reader meeting them here first:
# @act:i — Act I, the household chapter of the suite: these scenarios run against one household's
# own machines, never against a shared fleet.
# @concern:materialize-what-is-read joins this proof to the habit of the same name, declared at
# elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md (born unwired, 2026-10-20).
# @requires:household-nodes on a scenario or an Examples block: that case reads from another
# household machine, so it needs the household's three peers (matthew, jessica, james) running.
# The first scenario needs none of them: it runs on one device alone.
# @requires:household-hub on one Examples block: that case needs a household hub, the machine the
# household shares (defined in the feature's description). For as long as the household test
# fixture has no hub, that case is held, recognised and skipped rather than failed.
#
# HELD. Nothing here runs yet, and no step definitions exist; every scenario is @wip. The design
# is genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (section 12). The first scenario waits on slice 1 of that design's section 17 (a reading device
# that visits only the paths it names, on one device). Every other scenario waits on slice 2 (reading
# from other household machines through the same request); the hub case also waits on a hub.
@e2e @storage @act:i @concern:materialize-what-is-read
Feature: A device that reads three files from a large shared archive keeps those three files and the folders above them, and nothing more

  The Dowell household keeps a family archive together: recipes, old letters and photographs,
  10,000 files in 400 folders. Matthew, Jessica and James each run a peer: a machine of
  their own at home, always on, that holds the household's data and answers for it. Jessica
  also has a laptop, and a phone with little room on it.

  Most of the time Jessica wants three or four files from the archive, not the archive. A device
  that copied the whole archive to open one recipe would fill her phone, cost her battery and
  data, and tell every machine it asked that she was fetching everything. The promise here is
  that a device holds what its person read and the little it needed to find it, and no more.

  Some words, defined once:
  - A fingerprint is a short code computed from a thing's exact bytes. Change one byte and the
    fingerprint changes, so a fingerprint names one exact thing and nothing else.
  - A folder listing is the record of one folder: the names it holds, each with the fingerprint of
    the file or folder under that name. The archive's top folder listing has a fingerprint too,
    and that one fingerprint names the whole archive as it stood at one moment. It is called the
    archive's root. The root is the fingerprint alone, never the listing it names: a device that
    knows the root has a short code and nothing more, and must get the top folder listing from a
    holder, like any other folder listing, before it can find anything below it.
  - A file's spine is the chain of folder listings from the top folder down to that file. A device
    needs the spine to find a file starting from the root, and needs nothing else to do it.
    The three files read below each sit two folders under the top folder, in three different
    branches, so their spines share only the top folder listing: one top folder listing plus two
    folder listings on each of the three paths makes 7 folder listings in all.
  - To check an object is to compute its fingerprint and compare it with the one the folder above
    it names. A checked object is genuine no matter which machine sent it, so a reading device need
    not trust whoever answers; it only has to check.
  - A holder is any machine that has a copy of some of the archive: Jessica's own device,
    another household peer, or a household hub. Each reading device keeps its own list of the
    holders it may ask, in order.
  - A household hub differs from a peer in whose it is: a peer is one person's machine, while a hub
    belongs to no one member. It is a machine the whole household shares to keep copies for all of
    its members and their devices. For this feature a hub is asked and checked exactly as a peer
    is; its case is tested on its own because it is a different kind of holder, and the read must
    behave the same whichever kind answers.
  - A request is what a reading device sends to ask for one object. It names the object only by its
    fingerprint, never by which holder should answer or where that holder is. The device offers the
    same request to each holder on its list in turn, until one sends bytes that pass the check.

  A device's own store is one more holder on its list, not a trusted shortcut. What the device's
  own store hands back is asked for with the same request and checked the same way as what a peer
  or a hub sends, so a read behaves alike wherever the bytes happen to be.

  Background:
    Given the Dowell household archive holds 10,000 files in 400 folders, and its root is one fingerprint
    And Jessica's phone and Jessica's laptop each know the archive's root, its fingerprint alone
    And a reading on either device starts with only the root, not the top folder listing it names

  @wip
  Scenario: Reading three files from a complete copy on the same device visits only those files and their spines
    Given a complete copy of the archive sits in a store on Jessica's laptop itself, so no network is asked anything
    When an app on Jessica's laptop reads these three files of the archive:
      | path                     |
      | recipes/soups/lentil.md  |
      | letters/1998/grandma.txt |
      | photos/2019/lake.jpg     |
    Then the app shows the contents of the three files
    And the reading asked the laptop's own store for exactly these 7 folder listings, and checked each one:
      | folder listing                       |
      | the top folder, which the root names |
      | recipes                              |
      | recipes/soups                        |
      | letters                              |
      | letters/1998                         |
      | photos                               |
      | photos/2019                          |
    And the reading touched none of the other 9,997 files and no other folder listing

  @wip @requires:household-nodes
  Scenario: Reading the same three files from a household peer asks that peer for only those files and their spines
    Given Jessica's phone holds nothing of the archive yet
    And James's peer holds the whole archive
    And Jessica's phone lists only James's peer as a holder of the archive
    When Jessica's phone reads these three files of the archive:
      | path                     |
      | recipes/soups/lentil.md  |
      | letters/1998/grandma.txt |
      | photos/2019/lake.jpg     |
    Then Jessica's phone shows the contents of the three files
    And James's peer sent Jessica's phone the 3 files and exactly these 7 folder listings, and nothing else:
      | folder listing                       |
      | the top folder, which the root names |
      | recipes                              |
      | recipes/soups                        |
      | letters                              |
      | letters/1998                         |
      | photos                               |
      | photos/2019                          |
    And Jessica's phone now holds those 7 folder listings and 3 files and nothing else of the archive

  @wip
  Scenario Outline: The same read gives the same files whichever kind of holder answers it, and every object is checked
    The two scenarios above showed that the read is small: given a holder with the whole archive,
    it asked for and kept only the three files and their spines. This one shows something else:
    that the same read works alike whichever kind of holder answers, the phone's own store, another
    peer or the hub, and that the phone checks everything it receives from each. So here each
    holder is given only the three files and their spines, which makes it the one place they can
    come from; that the read takes no more than this is already shown above, not here.
    Given each case below starts afresh, with Jessica's phone and every holder on its list holding nothing of the archive
    And <holder> is then given the three files and their spines, and nothing else of the archive
    And <holder> is on the list of holders Jessica's phone may ask, so it is the only holder on that list with any of the three files
    When Jessica's phone reads these three files of the archive:
      | path                     |
      | recipes/soups/lentil.md  |
      | letters/1998/grandma.txt |
      | photos/2019/lake.jpg     |
    Then Jessica's phone shows the same three files, byte for byte, that the archive's root leads to through their spines
    And Jessica's phone checked every folder listing and file it received before keeping it
    And every request Jessica's phone made named only the fingerprint of the folder listing or file it wanted, and none named <holder> or any other holder

    Examples: Held on the phone itself, which is checked like any other holder
      | holder          |
      | Jessica's phone |

    @requires:household-nodes
    Examples: Held by another household peer
      | holder       |
      | James's peer |

    @requires:household-nodes @requires:household-hub
    Examples: Held by the household hub
      | holder             |
      | the household hub  |

  @wip @requires:household-nodes
  Scenario: A holder that sends altered bytes is not believed, and the true bytes come from another holder
    Given Jessica's phone holds nothing of the archive yet
    And James's peer and Matthew's peer each hold the whole archive
    And Jessica's phone lists James's peer first and Matthew's peer second
    And James's peer will alter the bytes of "photos/2019/lake.jpg" before sending them
    When Jessica's phone reads these three files of the archive:
      | path                     |
      | recipes/soups/lentil.md  |
      | letters/1998/grandma.txt |
      | photos/2019/lake.jpg     |
    Then Jessica's phone keeps nothing of what James's peer sent for "photos/2019/lake.jpg"
    And Jessica's phone shows the true photo, received from Matthew's peer and checked
    And Jessica's phone shows the other two files and kept every folder listing on the three spines, each received from James's peer and checked
    And Jessica's phone reports that James's peer sent bytes that did not match their fingerprint
