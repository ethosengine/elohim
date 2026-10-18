@e2e @lamad @epr-decomposition @native-content-graph @regression @requires:doorway @requires:seeded-content @act:i
Feature: A resource's neighbours, authored and discovered, honestly told apart
  A learner reading a doctrinal page can see, beside it, the other pages it sits
  among. There are two kinds of neighbour. AUTHORED neighbours are the related
  pages the author named on purpose. DISCOVERED neighbours are pages the doorway
  found because they share topic tags with this one ("you might also explore").
  Discovery is offered for exploring, never dressed up as the author's choice:
  every discovered neighbour is marked as discovered, and its card's inference
  source (how the link was found) is never "explicit", the mark of an authored one.

  The learner meets these neighbours in two places. Inside a learning path, the
  lesson view has an "Explore" sidebar that lists them. Opened on its own, at its
  universal address /epr/{id} (EPR: Elohim Protocol Resource, any resource the
  protocol addresses), a page has its own home, the atom home, and the neighbours
  are listed there in the "See also" section under "Where this lives".

  Who proves what: features/content/epr-atom-home.feature proves that the atom
  home's "See also" tells the two kinds apart. This file proves the same for the
  learning-path sidebar, and, more modestly, that a doctrinal page opened on its
  own home renders its text and reaches its "See also" list.

  The witness pages are the doctrinal docs: the protocol's founding writings
  ("manifesto", "constitution", "confession", "theology"), seeded as commons
  (readable by anyone) markdown pages. Each of the four names the other three as
  related, so every one of them has authored neighbours. The Background checks
  that on "manifesto", and checks that the doorway also discovers at least one
  page sharing its topic tags that it does not name. "manifesto" is also a step
  of a seeded learning path, so the learner meets it inside a lesson. The
  doctrinal pages render through the markdown renderer, so none of this depends
  on the learning-path outline view.

  What the learner sees that tells the two kinds apart: authored neighbours are
  listed under the related-concepts heading; discovered neighbours sit in their
  own "you might also explore" group, and each discovered card carries the way
  it was found (its inference source), which is never "explicit".

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the doctrinal docs "manifesto","constitution","confession","theology" are seeded as commons markdown EPR nodes
    And "manifesto" names "constitution", "confession" and "theology" as related
    And doorway "alpha" finds at least one more neighbour of "manifesto" by shared topics

  # The scenario that checked authored and discovered neighbours on a page opened
  # on its own moved to features/content/epr-atom-home.feature ("See also" under
  # "Where this lives"): the address /epr/{id} is now the atom home, and the old
  # standalone viewer that carried this sidebar is gone.

  @browser-only
  Scenario: Inside a lesson, the Explore sidebar tells authored neighbours from discovered ones
    # The learner reaches "manifesto" as a step of a learning path and opens the
    # lesson's Explore sidebar. The three pages the author named appear as
    # authored; at least one page the doorway found by shared topics appears
    # apart from them, marked as discovered, never as the author's choice.
    Given the learner is on a path step rendering "manifesto"
    When the learner opens the lesson's Explore sidebar
    Then it lists "constitution", "confession" and "theology" as authored related concepts
    And it shows at least one discovered concept card
    And the discovered card's inference source is not "explicit"

  @browser-only
  Scenario: A doctrinal page opened on its own home renders its text and reaches its "See also" list
    # "theology" is a commons markdown page. Opened at its own address, outside
    # any learning path, its body renders through the markdown renderer, and the
    # "See also" section under "Where this lives" lists at least one neighbour:
    # the doctrinal reading stands on its own. This is a reachability check;
    # which neighbours are authored and which discovered is proven on the atom
    # home's own feature (see the description above).
    When the learner opens the EPR address for "theology"
    Then the markdown content body is rendered
    When the learner opens "See also" under "Where this lives"
    Then "See also" lists at least one neighbour
