@e2e @lamad @act:i @wip
Feature: A steward publishes a commons learning path from his own peer, and seeding never overrides what he signed
  Matthew is the steward of "Foundations for Christian Technology": the person
  who holds the authority to publish updates to it. Every piece of content
  declares its reach, how far it may travel, on a scale that runs from intimate
  (inside one household) through trusted, familiar and community circles out to
  the commons (anyone). The course's authors, Matthew among them, judged it fit
  for the commons and declared it at commons reach; publishing stays within the
  reach they declared.

  Matthew has reworked the course. The learning path now has three levels
  (movements, modules and pieces): it holds five movements (Waking Up, Turning,
  Reordering Loves, Rebuilding Common Life, and Abundant Life & Sending), each
  movement holds modules (Waking Up opens with "The Church Dilemma"), and each
  module is made of smaller pieces: a lesson, the scripture it rests on, a
  story, a discussion and practice. Each piece names its links to the others (this
  scripture is the lesson's anchor, that practice is meant to be done at home), and those
  links are part of the piece itself. Movements and modules are the path's own
  structure, published inside the path rather than on their own, so below "item"
  means anything the course publishes: the path and every piece. Before the rework, the path already
  included the "evolution-of-trust" simulation, an interactive game about when
  trust pays. Other stories depend on it, so the rework must keep it on the path.

  The course's items are written as files in the repository. Matthew publishes
  them from his own peer, his own node on the network, by running the
  steward-publish command there: it reads those files and writes them to his
  peer. No build pipeline is involved. Matthew's agent (the identity his node
  signs with) signs each item and declares it the version every peer should
  serve. That signed version is the item's head: each peer records the head it
  serves and whose agent signed it, and a different head means a different
  write. The head, and its signer, are how the test tells Matthew's version from
  any other. Jessica, who shares Matthew's household, runs a peer of her own; if her
  peer serves Matthew's head, the update travelled.

  Seeding still happens: the build pipeline writes each peer from the same
  repository files. When it runs after Matthew's publish, nothing has changed in
  the repository, so it must see every item as already current and leave it
  alone. Seeding writes through each peer's own agent, so whatever it writes is
  signed as a new head by that peer's agent, not Matthew's; an item with a new
  head after seeding is an item seeding overwrote, even with identical bytes.
  A developer may later correct an item in the repository. Once Matthew has
  signed an item, seeding cannot outrank his signature, so it reports the item
  as stewarded and leaves the correction to him; he publishes it the same way he
  published the course. Seeding reports each item as one of these: already
  current (the repository and the peer agree), stewarded (they differ, but a
  steward signed the peer's head), or written (it wrote the item).

  The household's private content must never be caught up in this. Matthew and
  Jessica's love map is a path at intimate reach: it is theirs, inside their home.
  The steward-publish command has no business touching it.

  The doorway named in the background is the household's gateway: the address
  through which a browser, or this test, reaches the peers. The feature is
  tagged work in progress until its steps are wired to the running household.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the learning path "foundations-christian-technology" is declared at commons reach
    And the learning path "love-map-matthew-jessica" is declared at intimate reach
    And the path "foundations-christian-technology" includes the "evolution-of-trust" simulation

  Scenario: Matthew's update reaches another peer without the pipeline
    Given Matthew has reworked "foundations-christian-technology" into five movements
    When Matthew runs steward-publish for the reworked path and its items
    Then Jessica's peer serves the path's head signed by Matthew's agent
    And the path's movements on Jessica's peer are "Waking Up", "Turning", "Reordering Loves", "Rebuilding Common Life" and "Abundant Life & Sending"
    And the lesson of "The Church Dilemma", the first module of "Waking Up", on Jessica's peer links to its anchor scripture, its story, its discussion and its practice
    And the "evolution-of-trust" simulation is still on the path

  Scenario: Seeding after the publish leaves the current course untouched
    Given Matthew has run steward-publish for the reworked path and its items
    And nobody has changed the course in the repository since
    When the peers are seeded from the repository
    Then seeding reports every item of the course as already current
    And every item of the course on Jessica's peer keeps the head Matthew's agent signed

  Scenario: A correction to a signed item is published by its steward, not by seeding
    Given Matthew has run steward-publish for the reworked path and its items
    And a developer then corrects the text of the lesson of "The Church Dilemma" in the repository
    When the peers are seeded from the repository
    Then seeding reports that lesson as stewarded and writes nothing for it
    And every item of the course on Jessica's peer keeps the head Matthew's agent signed
    When Matthew runs steward-publish for that lesson
    Then Jessica's peer serves the corrected text under a new head signed by Matthew's agent

  Scenario: steward-publish refuses anything held at intimate reach
    When Matthew runs steward-publish for "love-map-matthew-jessica"
    Then the command refuses before writing anything, naming the love map's intimate reach
    And the love map on every peer keeps its head and its intimate reach
