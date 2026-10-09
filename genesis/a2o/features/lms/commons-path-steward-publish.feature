@e2e @lamad @act:i @concern:commons-path-steward-publish
Feature: A steward publishes a commons learning path from his own peer, and seeding never overrides what he signed
  The active scenario proves that a person can correct one commons learning item
  from an authorized device without rebuilding or reseeding the course. Matthew's
  publishing peer originally created that item and remains its root author. Che
  is the original workspace peer, recognized as a device of Matthew's existing
  Human identity. Che signs updates with its own key, distinct from Matthew's
  publishing peer's key. Matthew's root-author peer accepts each exact update,
  and Matthew's identity controller witnesses that device's exact exercise.

  Adam is a separate peer with a different key on the same network. The test
  reads Adam's own native election and ancestry, so Che's local copy cannot
  stand in for another peer's acceptance. Alpha and apex are two public
  doorways through which readers reach the peers. Both must serve the exact
  updated head and body within 75 seconds of Che starting that update. Only
  after the first update reaches Adam and both doorways does the second begin,
  with its own shared 75-second deadline starting when Che begins that update.
  The same item is changed again under the same root and permission,
  to show that ordinary editing continues after the first successful update.
  The fixture supplies two distinct replacement bodies for that item. For each
  round, those exact submitted bytes are the expected correction: the receiving
  peer and both doorways must serve them with the corresponding signed head.
  This reduced proof checks the person's editing and delivery workflow; it does
  not assess whether the supplied lesson text teaches well.

  The six scenarios tagged work in progress retain the wider course, seeding,
  privacy and withdrawal requirements. They are not part of this reduced proof.
  The course narrative below describes that wider intended behavior.

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
  story, a discussion, practice and a closing reflection. Each piece names its links to the others (this
  scripture is the lesson's anchor, that practice is meant to be done at home), and those
  links are part of the piece itself. Movements and modules are the path's own
  structure, published inside the path rather than on their own, so below "item"
  means anything the course publishes: the path and every piece. Before the rework, the path already
  included the "evolution-of-trust" simulation, an interactive game about when
  trust pays. Other stories depend on it, so the rework must keep it on the path.

  The course's items are written as files in the repository. Matthew publishes
  them from his own peer, his own node on the network, by running the
  steward-publish command there: it reads those files and writes them to his
  peer. No build pipeline is involved. Matthew's publishing peer's agent (the specific key his node
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

  The doorway named in the held scenarios is the household's gateway: the
  address through which a browser, or this test, reaches the peers. Those wider
  scenarios stay tagged work in progress until their steps are wired.

  Matthew can recognize an independently keyed workspace peer as another device
  of his existing identity. Matthew is that identity's controller: he can affirm
  or withdraw the device relationship without replacing the person. A signing
  key requests work; the peer that executes it signs under its own agent key.
  These keys and Matthew's continuing identity are distinct.

  Each course item's root is its original signed creation record. Its root
  author is the peer that signed that record; the author peer must grant a
  device permission for that exact item and accept the exact updated version.
  A credential lists those original records, the device, the permitted actions
  and this network. Calling a function alone confers no wider permission.
  The held original-publishing scenarios use Matthew's original publishing peer
  and its exact agent key. The workspace scenarios use the workspace peer's different key;
  recognizing it as Matthew's device does not make those keys interchangeable.
  An acceptance records the version and the original time its author approved it.

  Complete controller history means a signed witness for the exact update plus
  every consecutive signed record back to Matthew's identity-authority checkpoint.
  Peers verify the authors, predecessor hashes and ordering and inspect the records
  for withdrawal or a changed controller policy. A missing record or exhausted
  verification budget leaves the update pending; a discovery-link miss is no proof
  that withdrawal never happened. The broader withdrawal scenarios below remain
  work in progress until their native witnesses and household checks are wired.

  The course was first published as a single workbook, the item "fct-course":
  one web bundle holding all fifteen modules, with its own table of contents inside. The rework took that
  workbook apart into the path above, so the workbook is now the SOURCE the
  pieces were cut from, which each lesson still names, and never a step on the
  path: a step that carried the whole course subdivided inside itself would put a
  second, unwalkable table of contents inside the first module.

  Reach is also what a reader meets. Narrower still than intimate is private:
  held for its author alone. An older seeder gave that reach to every row it
  wrote without a declared one, and a peer keeps a row's reach until the
  author's new head reaches it, which is how a piece of a commons course can
  sit at private reach on a peer today. Standing is the reader's recognized
  relationship to the circle a reach names: anyone has it for commons, a
  signed-in member for community, a household member for intimate, nobody but
  the author for private. A peer that holds a piece at a narrower reach than
  the reader has standing for answers the read "held, at this reach": the piece
  exists there, and the hold is a decision. That is a different answer from "no
  such piece", and the reader must be told which one they got. A reader's device
  may keep an offline copy of a piece it was once allowed to read; a hold must
  not be answered with that copy as if the piece were open.

  # Sprint 1.4 closes one exact item; the full course and lifecycle scenarios stay held.
  Scenario: Che publishes two consecutive updates to Matthew's item without the pipeline
    Given the original Che peer has verified credentials for one Matthew-authored commons item
    When Che publishes an update with exact controller witnesses, waits for election and doorway delivery, then publishes a second update
    Then Adam elects each Che update and alpha and apex serve its exact head and body within 75 seconds of that update starting

  @wip
  Scenario: Seeding after the publish leaves the current course untouched
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the learning path "foundations-christian-technology" is declared at commons reach
    And the learning path "love-map-matthew-jessica" is declared at intimate reach
    And the path "foundations-christian-technology" includes the "evolution-of-trust" simulation
    Given Matthew has run steward-publish for the reworked path and its items
    And nobody has changed the course in the repository since
    When the peers are seeded from the repository
    Then seeding reports every item of the course as already current
    And every item of the course on Jessica's peer keeps the head Matthew's publishing peer's agent signed

  @wip
  Scenario: A correction to a signed item is published by its steward, not by seeding
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the learning path "foundations-christian-technology" is declared at commons reach
    And the learning path "love-map-matthew-jessica" is declared at intimate reach
    And the path "foundations-christian-technology" includes the "evolution-of-trust" simulation
    Given Matthew has run steward-publish for the reworked path and its items
    And a developer then corrects the text of the lesson of "The Church Dilemma" in the repository
    When the peers are seeded from the repository
    Then seeding reports that lesson as stewarded and writes nothing for it
    And every item of the course on Jessica's peer keeps the head Matthew's publishing peer's agent signed
    When Matthew runs steward-publish for that lesson
    Then Jessica's peer serves the corrected text under a new head signed by Matthew's publishing peer's agent

  @wip
  Scenario: steward-publish refuses anything held at intimate reach
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the learning path "foundations-christian-technology" is declared at commons reach
    And the learning path "love-map-matthew-jessica" is declared at intimate reach
    And the path "foundations-christian-technology" includes the "evolution-of-trust" simulation
    When Matthew runs steward-publish for "love-map-matthew-jessica"
    Then the command refuses before writing anything, naming the love map's intimate reach
    And the love map on every peer keeps its head and its intimate reach

  # Constraint: access to a function does not authorize every root the peer authored.
  @wip @regression
  Scenario: Matthew's publishing credential cannot authorize another course
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the learning path "foundations-christian-technology" is declared at commons reach
    And the learning path "love-map-matthew-jessica" is declared at intimate reach
    And the path "foundations-christian-technology" includes the "evolution-of-trust" simulation
    Given Matthew recognizes his workspace peer as a device of his existing identity
    And Matthew authorizes its signing key for the exact roots of "foundations-christian-technology"
    When the signing key asks the author peer to grant publishing authority for a different course
    Then the author peer refuses before signing a grant or recording acceptance
    And the other course keeps its head

  # Constraint: later withdrawal stops new exercise while preserving exact accepted history.
  @wip @regression
  Scenario: An interrupted accepted update remains recoverable after device withdrawal
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the learning path "foundations-christian-technology" is declared at commons reach
    And the learning path "love-map-matthew-jessica" is declared at intimate reach
    And the path "foundations-christian-technology" includes the "evolution-of-trust" simulation
    Given Matthew's workspace peer authored a course update
    And its root author accepted that exact version before the device was withdrawn
    And publication stopped before that version was declared
    When Matthew withdraws that device's publishing authority
    And the workspace peer resumes the interrupted publication
    Then the workspace peer declares the accepted version under its own agent key
    And the declaration carries its root author's original acceptance time
    And an update authored after withdrawal cannot obtain new acceptance
    And Matthew's identity and his other devices retain their standing

  # Constraint: absent lifecycle links cannot prove that a withdrawal never happened.
  @wip @regression
  Scenario: Missing withdrawal history does not become publishing permission
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the learning path "foundations-christian-technology" is declared at commons reach
    And the learning path "love-map-matthew-jessica" is declared at intimate reach
    And the path "foundations-christian-technology" includes the "evolution-of-trust" simulation
    Given Matthew withdrew his workspace device before an update was accepted
    And another peer has the device binding but lacks its withdrawal discovery link
    And it cannot obtain a required signed record in the controller history
    When that peer verifies the claimed authorization for the update
    Then it requires the controller's complete witnessed history for that exact exercise
    And missing history leaves publication pending

  @wip @regression
  Scenario: The workbook the course was cut from is not a step on the path
    # Regression 2026-10-08: the recomposed path carried the workbook as the twelfth step of
    # "The Church Dilemma", so module 1 ended in a second table of contents.
    Given the "Foundations for Christian Technology" path as Matthew published it: five movements, their modules, each module's pieces, all at commons reach
    And the workbook "fct-course" the course was cut from is published beside the path, not inside it
    When its steps are listed in walking order, the order a learner meets them from the first movement to the last piece
    Then no step is the workbook "fct-course"
    And "The Church Dilemma" ends with its own reflection piece, not with the workbook that once stood as its twelfth step
    And every lesson still names that workbook as the source it was cut from
    And each module's pieces follow its lesson with no gap in that order

  @wip @regression
  Scenario: A reader reaching a piece held at a narrower reach is told it is held, not missing
    # Regression 2026-10-08: the step view answered a refused read with "not yet available, it may
    # not have been seeded", the message for a piece that does not exist.
    Given a reader with no standing beyond commons opens "Foundations for Christian Technology" on doorway "alpha"
    And the peer behind that doorway still holds the lesson "The Church Dilemma" at "private" reach, the reach an older seeder gave it
    And Matthew's commons declaration for that lesson has not yet reached that peer
    When the reader opens the step that carries that lesson
    Then the step says the piece exists and names "private" as the reach it is held at
    And it offers the ways in: sign in with standing for that reach, or ask the steward to widen it
    And it does not say the piece is missing or was never seeded
    And the reader's offline copy of that piece, if any, is not shown in its place
