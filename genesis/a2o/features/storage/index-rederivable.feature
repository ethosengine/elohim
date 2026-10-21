# What the tags below mean, for a reader meeting them here first:
# @act:i — Act I, the household chapter of the suite. This story needs one household peer only
# (Matthew's), so it carries no @requires:household-nodes.
# @concern:recall-reaches-authority joins this proof to the habit of that name
# (.epr-meta/recall-reaches-authority.habit.md): when someone searches, what comes back leads to the
# authority behind it. Here the authority is the library itself: the search index is only ever a
# summary of it, so it can be thrown away and made again, and the remade one is the same.
# @wip — on every scenario: no step definitions exist yet. Why the story is held, and on what, is
# said in the Feature description below.
@e2e @storage @act:i @concern:recall-reaches-authority
Feature: A household's search index, deleted and rebuilt from the same library, comes back exactly as it was

  The Dowell household keeps its library (notes, letters, lessons, recipes) on Matthew's peer, an
  always-on machine at home that holds the household's data and answers for it. When anyone in the
  household searches, the peer does not read every item. It reads its index: a compact summary of
  the whole library, built from it by an indexing method. This story is about the peer's own
  storage: Matthew uses his peer directly, and no doorway or other gateway stands between him and it.

  An index is useful, and it is also a second copy of what the library says, in another shape. A
  household should be able to throw it away (to free room, or because it is suspected of being
  wrong) and lose nothing, because the library is the authority and the index is only made from it.
  That holds only if rebuilding gives the same index. If a rebuilt index could differ, a search
  would answer differently for no reason anyone could see, and nobody could tell which index was
  the true summary. Throwing an index away is also safe only if the time before it is rebuilt is
  honest: a search made while no index exists must say so, rather than answer as if the library
  held nothing. So this story proves both that the gap is honest and that the rebuilt index is the
  same.

  This story is held: it describes behaviour the peer does not have yet, and every scenario is
  marked @wip. It waits on a design change, not only on test code, and the change reaches the
  Background as well as the scenarios: the Background's second and fourth steps (a root for the
  whole library, and an index built from that root) describe the peer as it will be once the
  change lands, not as it is today. Two things are missing today. The library has no root:
  each item has a fingerprint of its own, but nothing names the library as a whole. And a peer
  names its index by the indexing method alone, so two indexes built from different libraries can
  carry the same name. What must change is that the peer gives its library a root and names each
  index by both the method that built it and the root it was built from (the words are defined
  below). Until it does, neither the Background's root nor an index built "from that root" can be
  shown, so no scenario can run. The story can run once a peer gives its library a root and names
  its index by method and root, as just described; what this story asks for is complete as written
  here. For a reader who wants the engineering detail, it is worked out further in sections 5.1
  ("Derived indexes") and 12.5 of the design
  genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md,
  whose section 17 schedules the change as a piece of work called slice 4. That document adds
  detail; it does not change what this story means.

  Some words, defined once:
  - A fingerprint is a short code computed from a thing's exact bytes. The same bytes always give
    the same fingerprint, and changing one byte changes it, so a fingerprint names one exact thing.
  - The library's root is one fingerprint that names the whole library as it stood at one moment.
    Adding an item makes a new root. Like the naming below, this is part of what the story waits
    on: today a peer keeps no such root for its library.
  - The indexing method is the exact program that builds the index, named by its own fingerprint.
    It must be deterministic: it may use no randomness and may not read the clock, so that given
    the same library it builds the same index every time. This story does not prove that by
    looking inside the method. The first scenario tests what it promises: a rebuild from the same
    root must give contents with the same fingerprint. Each run of that scenario is one trial, not
    a proof: a rebuild that gave a different fingerprint fails it, so a method that is not
    deterministic cannot keep passing it. A household could change its method one day;
    what that means for its index is another story. Here the method stays the same throughout.
  - A search reads only the index. Which items it returns, and in what order, is worked out
    from the index's contents and the words searched for, and nothing else. This story does not
    prescribe a ranking. Whatever ranking the search applies, the story asks only that the same index
    gives the same answer, in the same order, every time.
  - An index is named by the method that built it and the root it was built from. Its contents
    have a fingerprint too. This naming is what the story asks for and does not yet exist: today
    a peer names its index by the method alone.

  Background:
    Given Matthew searches on his own peer directly, with no doorway between them
    And Matthew's peer holds the household library, named by one root fingerprint for the whole library
    And the library's recipes include at least two that match a search for "lentil soup"
    And Matthew's peer has built its search index from that root with the household's indexing method

  @wip
  Scenario: The rebuilt index is the same index, and the same search gives the same answer
    Given Matthew searches the library for "lentil soup" and notes the answer
    And the index's contents have a fingerprint, which is noted
    When the index is deleted from Matthew's peer
    And Matthew's peer rebuilds the index from the same root with the same method
    Then the rebuilt index's contents have the noted fingerprint
    And the rebuilt index names the same method and the same root
    And searching for "lentil soup" gives the noted answer, in the same order

  @wip
  Scenario: While no index exists, a search says so instead of answering with nothing
    When the index is deleted from Matthew's peer
    And Matthew searches the library for "lentil soup" before the index is rebuilt
    Then the answer says that the peer holds no index, and that it cannot yet say which items match or in what order
    And the answer does not present an empty list as the result

  @wip
  Scenario: An index built after the library changed names the new root, and cannot be mistaken for the old one
    Given the index's contents have a fingerprint, which is noted
    When Matthew adds a recipe for "lentil soup with lemon" to the library, which gives the library a new root
    And Matthew's peer builds the index from the new root with the same method
    Then the new index names the new root
    And the new index's contents do not have the noted fingerprint
    And searching for "lentil soup" now finds the added recipe
