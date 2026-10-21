# What the tags below mean, for a reader meeting them here first:
# @act:i — Act I, the household chapter of the suite: this story runs against one household's own
# machines, never against a shared fleet.
# @concern:materialize-what-is-read joins this proof to the habit of that name, declared at
# elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md: a machine holds what it uses and fetches
# no more than that, which here includes not making again what has already been made: a thing
# already made is found by what it was made from, so using it again is a read, not new work.
# @requires:household-nodes — the story needs the household's three peers (matthew, jessica,
# james) running, because it compares what two of them make and asks what a third may learn.
#
# HELD. No step definitions exist; every scenario is @wip. The design is
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (section 12.5). The story waits on slice 4 of that design's section 17, which records each made
# thing under the recipe and the input it was made from, so that another machine can look it up.
@e2e @storage @act:i @concern:materialize-what-is-read @requires:household-nodes
Feature: Two household machines that need the same thing made from the same files make it once and share it

  The Dowell household (Matthew, Jessica and James) keeps a family photo album. Each of them runs a
  peer: an always-on machine at home that holds the household's data and does work for it. A
  contact sheet, one page of small previews of every photo in the album, is something the
  household's apps make from the album when someone opens it.

  Making a contact sheet of a large album takes minutes of a machine's time. If every machine that
  needed one made its own, the household would pay for the same work three times and get three
  copies of the same page. The promise here is that a thing made from known files by a known recipe
  is made once and found again, and that finding it is a saving, never a dependency: a machine that
  cannot reach the copy can still make it itself and get the same result. The principle behind the
  promise is called derived by identity: a thing derived from known inputs by a known recipe is
  identified by those inputs and that recipe, not by where it is kept or who made it, so any machine
  that needs the same thing can look it up by what it was made from, or make it again and get the
  same result.

  Some words, defined once:
  - A fingerprint is a short code computed from a thing's exact bytes. Change one byte and the
    fingerprint changes, so a fingerprint names one exact thing.
  - The album's root is one fingerprint that names the whole album, every photo in it, as it stood
    at one moment. Adding a photo makes a new root.
  - A recipe is the exact program that turns an album into a contact sheet, named by its own
    fingerprint. Its author is the developer who wrote and published that program, not a household
    member. A recipe is repeatable when its author declares that, given the same album, it always
    produces the same bytes. The household's machines take that declaration at its word when deciding
    to reuse a contact sheet; nothing enforces it in advance, and making the contact sheet again is
    how a false declaration, or a machine whose record of its result is wrong (through a fault, damage
    or a lie), is caught. In every scenario below the declaration is true: the recipe really is
    repeatable. The scenario where a machine makes the contact sheet itself shows only that a true
    declaration gives the same bytes; it does not test
    the declaration. The two checking scenarios show how a machine tests a record rather than trusting
    it, and in the one where a record does not match, the record is false, not the recipe.
  - A making record is a short note saying "this recipe, run on this root, produced the output with
    this fingerprint", together with who made it. It is evidence of what someone made. It is not an
    order to believe them.
  - The album's reach is a list, declared on the album when it was created, of the people whose keys
    may read the album and receive its making records. It is one list for both on purpose: whoever
    may read the album may learn it was used, and no one else. Here it names the household's three members
    and no one else. A stranger is someone with a key of their own who is not on that list and has no
    standing in the household at all.
  - Recording a making means the maker keeps the record and, at the moment of recording, sends a copy
    of it to every other machine of a person on the album's reach, and to no other machine. So every
    household peer already holds its own copy of every making record of the album. When a peer looks
    one up by the recipe and the root, it searches its own copy and asks no other machine, so the
    lookup works even while the maker is switched off. The contact sheet itself is not copied: it
    stays on the machine that made it, and another machine must fetch it from there. The first
    scenario below proves this copying happens; every later scenario states the copy it relies on as
    one of its own Given steps.
  - Every household peer answers requests for making records from any machine that can reach it over
    the network. The asking machine signs each request with its owner's key, and the peer reads that
    key to decide what to answer. A stranger reaches a household peer exactly this way, with a key of
    its own, so the peer's answer is the only thing standing between the stranger and the records.
  - A making record shows that someone in the household used the album, and a root with a making
    record behind it shows that the album exists. A stranger who has learned a root fingerprint must not be able to find out either
    thing by asking a household peer for making records. So the refusal a stranger gets for a real
    album's root must read exactly like the refusal for a root no household machine has ever seen;
    any difference between the two would itself tell the stranger the album is real.
  - Two different checks appear below. Checking a fetched contact sheet against the fingerprint its
    record names proves the bytes are the ones the record names; it does not prove the recipe made
    them. Only running the recipe again on the same root and comparing proves that. The second
    scenario does the first kind of check; the two scenarios titled "by re-running the recipe" do the
    second.

  Background:
    Given the household album is named by one root, and its reach admits Matthew, Jessica and James only
    And the contact-sheet recipe is named by its own fingerprint, and its author has declared it repeatable

  @wip
  Scenario: Recording a making puts a copy of the record on every household peer and on no other machine
    When Jessica's peer makes the contact sheet for the album's root and records the making
    Then Matthew's peer and James's peer each hold a copy of that making record, without having asked for it
    And no machine outside the album's reach has received a copy of that making record
    And after Jessica's peer is switched off, James's peer still finds that making record in its own copy, by the recipe and the root alone

  @wip
  Scenario: The second machine finds the first machine's contact sheet and does not make another
    Given Jessica's peer has made the contact sheet for the album's root, and recorded the making
    And James's peer holds its own copy of that making record, sent to it when the making was recorded
    When James's peer needs the contact sheet for the same root and the same recipe
    Then James's peer finds Jessica's making record in its own copy, by the recipe and the root alone
    And James's peer fetches the output that record names from Jessica's peer
    And James's peer checks the fetched bytes against the fingerprint the record names, and they match
    And James's peer shows that contact sheet
    And James's peer ran the recipe zero times

  @wip
  Scenario: With the first machine unreachable, the second makes the contact sheet itself and gets the same result
    Given the contact-sheet recipe really is repeatable, as its author declared
    And Jessica's peer has made the contact sheet for the album's root, and recorded the making
    And James's peer holds its own copy of that making record, sent to it when the making was recorded
    And Jessica's peer is switched off
    When James's peer needs the contact sheet for the same root and the same recipe
    Then James's peer finds Jessica's making record in its own copy, but cannot fetch the contact sheet it names
    And James's peer runs the recipe itself
    And the contact sheet James's peer made has the same fingerprint as the one Jessica's peer made

  @wip
  Scenario: A photo added to the album makes a new root, and the old contact sheet is not offered for it
    Given Jessica's peer has made the contact sheet for the album's root, and recorded the making
    And James's peer holds its own copy of that making record, sent to it when the making was recorded
    And Matthew adds one photo to the album, which gives the album a new root
    When James's peer needs the contact sheet for the new root and the same recipe
    Then James's peer finds no making record for the new root
    And James's peer makes a new contact sheet, which includes the added photo

  @wip
  Scenario: A machine checks another's record by re-running the recipe, and a match confirms the record
    Given Jessica's peer has made the contact sheet for the album's root, and recorded the making
    And James's peer holds its own copy of that making record, sent to it when the making was recorded
    And James's peer treats that record as evidence of what Jessica's peer made, not as an order to believe it
    When James's peer checks that record by running the recipe on the same root itself
    Then the contact sheet James's peer made has the fingerprint Jessica's peer's record names
    And James's peer reports that Jessica's peer's record matches its own result

  @wip
  Scenario: A machine checks another's record by re-running the recipe, and a mismatch is reported, not believed
    # The cause of the mismatch may be a fault while making, a damaged copy of the output or a
    # deliberate lie. The harness sets up the same state for all three, so one scenario covers them.
    Given Jessica's peer has recorded a making whose output fingerprint does not match what the recipe produces from the album's root
    And James's peer holds its own copy of that making record, sent to it when the making was recorded
    And James's peer treats that record as evidence of what Jessica's peer made, not as an order to believe it
    When James's peer checks that record by running the recipe on the same root itself
    Then James's peer reports that Jessica's peer's record does not match its own result
    And James's peer shows the contact sheet it made, not the one the record names

  @wip
  Scenario: A stranger cannot learn from making records that the album exists or was used
    Given Jessica's peer has made the contact sheet for the album's root, and recorded the making
    And Matthew's peer holds its own copy of that making record, sent to it when the making was recorded
    And the stranger has a key of its own, is not on the album's reach and has no standing in the household
    And the stranger holds the album's root fingerprint, as though it had leaked
    And the stranger's machine can reach Matthew's peer over the network and signs its requests with the stranger's own key
    When the stranger asks Matthew's peer for any making record of the album's root
    Then Matthew's peer refuses, saying only that the stranger has no standing in the household
    And the refusal reads exactly like the one Matthew's peer gives for a root that no household machine has ever seen, so it tells the stranger nothing about whether the album exists
    And no household machine has sent a making record of the album's root to any machine outside the album's reach
