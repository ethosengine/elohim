# What the tags on the line below mean, for a reader meeting them here first:
# @act:i — Act I, the household: this story runs against one household's own mesh, not against
# a deployed fleet. It is a lane, not an ordering.
# @hosted-human — Matthew and Susan do not run software of their own; the doorway hosts their
# sessions on the household's peer, which is why one peer keeps both of their accounts.
# @requires:seeded-humans — those two accounts must already exist on the doorway before this
# story runs; the household's setup creates them, this story does not.
# @concern:recall-reaches-authority: the concern this story is evidence for — when someone
# goes looking for something, what comes back must lead them to the authority behind it (here:
# the named, fetchable rules that ordered the results) rather than to an unattributable list.
# @requires:household-nodes: the search route is served by the household's OWN storage peer and
# read directly from it, so the run must own the household mesh it measures.
# @requires:doorway — a running doorway to sign in at; a fixture precondition, not a capability.
# @requires:unfolded-peer (last scenario only): a second peer, deliberately kept without an
# index, which no household runs by default — so that scenario is HELD (recognised and skipped,
# never counted as a failure) until one does.
@e2e @content @hosted-human @requires:doorway @requires:seeded-humans @requires:household-nodes @concern:recall-reaches-authority
Feature: A household search names the rules it ranked by, withholds what is not the asker's, and says when it has not looked
  A household keeps its own library on a computer it owns — a storage peer — and reaches it
  through a doorway, the web entrance each member signs in at. Everything in that library is
  written by someone in the household, and every item carries a reach: how widely it may
  travel. "private" is the narrowest — the item is for the person who made it and no one else.
  An item written without saying otherwise takes the household's ordinary, wider reach, so
  everyone here can find it; that contrast is what the second scenario below turns on.

  A peer cannot search its library by reading every item on every question. It first folds the
  library into an index: a compact summary of everything it holds, kept up to date as items are
  written, which is what a search actually reads. A peer can be running and serving and still
  hold no index — a brand-new peer, or one deliberately started without folding.

  Searching is not a neutral act. Something decided which of the matches came first, and in most
  software that something is unnamed and unavailable: the reader cannot see the rule, so they
  cannot argue with it. Worse, a search can leak the very thing it is meant to keep back — an
  excerpt in a preview, or a count that betrays that a hidden item exists at all.

  This story holds the peer to three promises.

  First, the answer names the rules it ranked by. It carries a recipe — a small file of ranking
  rules shipped with the peer — by name and by content address: a short string derived from the
  recipe's exact bytes, a fingerprint of that one text and no other. Anyone holding the address
  can fetch that recipe, read the rules that put one result above another, and contest them.
  (Fetching it is a separate act with its own story; what this one requires is that the address
  be there and be the same one every time.) The address is a fingerprint, so it is the same on every answer the
  same rules produced; the story asks twice and requires one address, so the peer cannot print a
  decorative label that changes between breaths.

  Second, the peer honours reach before a single word of the item leaves it. The gate runs on
  each match after the ranking and before any excerpt. Under the results the answer also carries
  tallies — counts of the results grouped by one attribute or another, and one of those
  attributes is reach. The peer counts only what it admitted, so a withheld item is missing from
  the results AND from the tally by reach, which is where a hidden item usually shows through.

  Third, the peer never dresses ignorance as an answer. The ranking is only as good as the index
  behind it, so every answer states plainly whether its ranking is known. A peer holding no
  index says it has none and that its ranking is not known, rather than returning an empty list
  that reads as "there is nothing like that here" when the honest answer is "I have not looked".

  The people: Matthew and Susan are both members of this household and both sign in at the
  doorway "alpha", so one peer keeps both of their accounts. Keeping them in one place must
  never mean mixing them.

  The peers: a household may run more than one. The first two promises are asked of the peer
  behind the doorway, the one that keeps the accounts and the library. The third is asked of a
  second peer the household also runs — same household, no index of its own.

  About the items this story writes: each is given a title made unique to the run, so that what
  the search finds is unambiguously the item this run just wrote and not a leftover of an
  earlier one, and each is tagged "search-e2e", which marks it as this story's doing so it can
  be told apart from the household's real library and cleared away afterwards.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  @act:i
  Scenario: Matthew finds the note he just wrote, under a ranking whose rules the answer names
    Given Matthew has created content titled "Composting In A Small Yard" made unique to this run, with tags "search-e2e"
    When Matthew searches the household library for that title
    Then the first result is that content
    And the answer says its ranking is known
    And the answer names the recipe it ranked by, with a content address
    And asking the same question again names the same recipe address

  # Matthew's own search runs first on purpose. It is the control: it proves the private item
  # really is in the library and really is findable by its holder, so Susan's empty-handed
  # search below means the peer withheld it, not that there was nothing to withhold.
  @act:i
  Scenario: Matthew's private note is absent from Susan's results and from her tallies
    Given human "Susan" is logged in on doorway "alpha"
    And Matthew has created content titled "What We Still Owe On The Roof" made unique to this run, with tags "search-e2e" and reach "private"
    When Matthew searches the household library for that title
    Then the first result is that content
    When Susan searches the household library for that title
    Then no result is that content
    And the answer's tally by reach names no private item at all

  # Held by design — skipped, not failed — until a household runs a second peer with no index
  # (@requires:unfolded-peer). Meanwhile the same promise is pinned by a storage integration
  # test that asks an unfolded store directly.
  @act:i @requires:unfolded-peer
  Scenario: A peer that holds no index says so instead of answering with an empty list
    Given the household also runs a peer that holds no index
    When Matthew asks that peer for anything at all
    Then the answer says that peer holds no index
    And the answer says its ranking is not known
    And the answer carries no results alongside that admission
