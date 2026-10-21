# What the tags below mean, for a reader meeting them here first:
# @act:i — Act I, the household chapter of the suite: this story runs against one household's own
# machines, never against a shared fleet.
# @concern:reach-enforced-everywhere joins this proof to the habit of that name
# (.epr-meta/reach-enforced-everywhere.habit.md): who may see a thing is decided before any byte
# of it leaves, on every route out of a machine. Its existing check covers the web listing route
# only; this story is the folder-tree route.
# @requires:household-nodes — the story needs the household's three peers (matthew, jessica,
# james) running, because a stranger's request is put to each of them.
#
# Design source, for maintainers:
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (section 12.3, rule 2; build slice 2 of section 17; open decision in section 19, question 10).
@e2e @dataplane @act:i @concern:reach-enforced-everywhere @requires:household-nodes
Feature: A private folder of public files shows a stranger nothing, not even that it exists

  Matthew, Jessica and James Dowell are a household. Each runs a peer: an always-on machine at
  home that holds the household's data and answers requests for it.

  The household is thinking about moving. Matthew keeps a folder called "Moving house" for the
  three of them. Every file in it is something anyone could find: a school district's published
  brochure, a published town map, a house listing copied from a public website. The folder itself is not
  public at all. Its file names, such as "listing-12-elm-street.pdf", and the fact that the
  folder exists, say that this family is leaving, and where it is looking. A folder's names can
  be more private than every file inside it.

  Some words, defined once:
  - A fingerprint is a short code computed from a thing's exact bytes, so it names that one thing.
    Anyone who holds a fingerprint can ask a machine for the bytes it names.
  - A folder listing records one folder's names, each with the fingerprint of what is under it.
    The folder's top listing has a fingerprint of its own, called the folder's root; it names the
    whole folder as it stood at one moment.
  - A root leads to a file when the root's listing, or a listing named inside it, eventually
    names that file's fingerprint. A peer also keeps the reverse: for every fingerprint it holds,
    it knows which of its roots lead to it, so when someone asks for bytes by fingerprint alone,
    the peer can still judge whether any of those roots admits the person asking.
  - A published record is an entry of its own in the network that makes one file findable and
    servable on its own terms, carrying its own reach. A file with a published record can be
    fetched through that record without knowing which folders contain it.
  - Reach is who may see a thing. A fingerprint is only a name, so a reach is attached to the
    thing a fingerprint names: when we say a root carries a reach, we mean the folder's top
    listing, which the root names, has that reach attached. The "Moving house" root carries a
    reach that admits the household's three members and no one else, and every listing and name
    under that root shares it. Here "private" means exactly that. Each public file inside it keeps its own public reach,
    through the published record it came from.
  - A stranger is someone with a key of their own who belongs to nothing the household belongs to,
    and so has no standing to see the folder.
  - A doorway is a public web entrance: a machine outside the household that serves public bytes
    to anyone on the web. It is not a household peer, and the folder's reach does not admit it.
  - Two answers are identical when nothing a stranger can see tells them apart: the same status,
    the same wording, the same length, and no detail present in one and missing from the other.

  The promise has two sides. A household peer gives a stranger nothing about the folder, and
  answers in a way that cannot be told apart from having no such folder at all. And a household
  member's own device, when it lacks part of the folder, asks only machines inside the folder's
  reach for it, because a machine that is asked has already learned what was asked, even if it
  refuses. Without that care a device would ask any machine it knows of, since bytes from any
  machine can be checked against their fingerprint; the second side is that the folder's reach
  narrows whom it asks.

  This guarantee does not hold yet, and the story is held until it does. Today, reach is recorded
  only on published records, so a folder tree whose files are public is listed to anyone the
  machine serves, and household peers do not yet answer requests for folder trees at all. The
  story waits on two things: the work that records reach on a folder's root and lets household
  peers answer folder-tree requests, and an open design decision on whether reach is declared on
  a folder's root at all. No scenario below runs yet; each is marked @wip.

  Background:
    Given Matthew's "Moving house" folder holds three public files: the school district's brochure, the town map, and the house listing named "listing-12-elm-street.pdf"
    And each of the three files has a published record of its own, which anyone may read
    And the names the three files have inside the folder are not public
    And the folder's root carries a reach that admits Matthew, Jessica and James and no one else
    And every household peer holds the whole folder
    And a stranger holds the folder's root fingerprint, as though it had leaked
    And the stranger also holds a second fingerprint, which names a folder that no household peer holds, so that a peer's answer about the leaked root can be compared with its answer about a folder it truly does not have

  @wip
  Scenario Outline: A household member lists the folder from any household peer
    When Jessica's laptop asks <peer> to list the "Moving house" folder
    Then <peer> lists the three file names and their fingerprints
    And Jessica's laptop opens "listing-12-elm-street.pdf" from that listing

    Examples:
      | peer           |
      | Matthew's peer |
      | Jessica's peer |
      | James's peer   |

  @wip
  Scenario Outline: A stranger asking a household peer about the folder gets the same refusal as for a folder that does not exist
    When the stranger asks <peer> to list the folder named by the leaked root
    And the stranger asks <peer> to list the folder named by the second fingerprint
    Then <peer> refuses both requests, and the two answers are identical
    And neither answer says whether the folder exists or why the request was refused
    And neither answer contains a file name, a fingerprint or a count from the "Moving house" folder
    But in its own records, which the stranger never sees, <peer> counts the first as refused for standing and the second as not held

    Examples:
      | peer           |
      | Matthew's peer |
      | Jessica's peer |
      | James's peer   |

  @wip
  Scenario: A stranger cannot learn the folder exists by asking whether a peer holds a piece of it
    When the stranger asks James's peer whether it holds the folder's top listing
    And the stranger asks James's peer whether it holds the listing the second fingerprint names
    Then James's peer answers both times that it does not hold it, and the two answers are identical

  @wip
  Scenario: A public file stays public through its own published record, and the folder is still not shown
    When the stranger asks James's peer for the school district's brochure through its published record
    Then James's peer sends the brochure
    And nothing in the answer names the "Moving house" folder or any name in it
    When the stranger asks James's peer for the brochure by naming it under the leaked folder root
    And the stranger asks James's peer for the brochure by naming it under the second fingerprint
    Then James's peer refuses both requests, and the two answers are identical

  @wip
  Scenario: A file that is reachable only through the private folder is refused even by its own fingerprint
    Given Matthew adds a fourth file to the folder, his own note, which has no published record
    And James's peer has received the note's bytes with the rest of the folder, so it holds them
    And James's peer knows that the only root it holds that leads to the note is the "Moving house" root
    And the stranger holds the note's fingerprint, as though it had leaked
    When the stranger asks James's peer for the bytes the note's fingerprint names
    And the stranger asks James's peer for the bytes the second fingerprint names
    Then James's peer refuses both requests, and the two answers are identical
    And it refuses the note because every root that it holds and that leads to the note has a reach that does not admit the stranger
    But in its own records, which the stranger never sees, James's peer counts the request as refused for standing

  @wip
  Scenario: A member's device asks only machines inside the folder's reach for what it lacks
    Given Jessica's laptop holds the folder's top listing and none of its files
    And Jessica's laptop knows two machines it may ask for bytes it lacks: James's peer, and the doorway "alpha", which it uses for public bytes
    And without the folder's reach to guide it, the laptop would ask either machine, since it can check bytes from any machine against their fingerprint
    When Jessica's laptop opens "listing-12-elm-street.pdf"
    Then Jessica's laptop received the file from James's peer
    And the doorway "alpha" received no request about the "Moving house" folder or any file in it
