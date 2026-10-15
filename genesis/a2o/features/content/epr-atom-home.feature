@e2e @content @epr @concern:epr-atom-home @requires:doorway @requires:seeded-content @act:i
Feature: The EPR atom home — every reachable resource has one place of its own
  As anyone who arrives at a protocol resource
  I want the universal address to show me the thing itself, who holds it, where it lives, and who is talking about it
  So that a link a neighbour shares opens onto the resource's own front door, not into someone else's app

  Every resource in the protocol — an EPR, an Elohim Protocol Resource — has one
  durable address, /epr/{id}. Today that address wears the learning app's clothes:
  a "Back to Lamad" button whether or not you came from there, mastery controls
  before you have seen the content, and an empty page dressed as a full one when
  the resource cannot be reached. The atom home replaces that with one frame that
  is the same whether the resource is a simulation or an article: what this is,
  the content itself, and four supporting sections we call the legs — who holds
  it, where it lives, how it is governed, where it came from — with the
  conversation around it beside them.

  # Why this matters:
  # Whoever arrives — from a learning path, from a collective, from a cold link —
  # sees the same honest frame, and the words are household words: "held by 1 of 3
  # households", not a percentage; "we can't reach this one from here", not a wall.

  # Vocabulary a stranger needs, in the order it appears:
  # - reach: how widely a resource may be shared. "Commons" means anyone can reach it;
  #   the reach chip is the small label that says so.
  # - notarized: the resource's record is anchored in the shared peer network (the
  #   DHT); the notarized chip says so, and whether this doorway has verified the anchor.
  # - arrival chip: the small label at the top naming the resource you just came
  #   from — a prior stop, never the app. A cold link (an address opened directly,
  #   with no prior stop) has no arrival chip.
  # - focal slot: the main content area. A simulation fills the full width
  #   ("immersive"); an article reads in a column with the legs in a rail beside it,
  #   starting level with the top of the content ("reading").
  # - rail: the side column that holds the legs. Beside an article it starts level
  #   with the top of the text; under a full-width simulation it sits below.
  # - the legs: the four supporting sections that ground a resource in its network —
  #   Who holds it, Where this lives, How it's governed, Where it came from.
  # - household: a family or group that runs its own computer on the network and
  #   counts as one holder of a copy, however many people are in it.
  # - holding sentence: the one plain sentence about who keeps copies, reported by the
  #   doorway itself, e.g. "Held by only 1 household — invite another to help hold these".
  #   Technical holding detail (piece maps, copy counts) stays folded away under
  #   "Network detail" until someone opens it; it is never on the page before that.
  # - household floor: how many households currently hold a copy, out of how many
  #   the resource wants for safety — shown as "N of M", both numbers read from
  #   the doorway's own household report, never hardcoded.
  # - fold: a closed section under a leg's one-line summary. It opens when you press
  #   its title, or when you arrive at an address ending in that leg's name (for
  #   example /epr/{id}#holds opens the fold under "Who holds it"). Each leg has one:
  #   "Network detail" under Who holds it, "See also" under Where this lives,
  #   "Attestations" under Where it came from, and one under How it's governed.
  # - pieces: a resource is stored as pieces spread across the peers (the computers
  #   households run). Some pieces carry the content and some are spares that can
  #   rebuild a lost one. "Whole" means enough pieces can still be found to rebuild
  #   the resource.
  # - the old address: before the atom home, a resource also had a viewer page at
  #   /resource/{id} with tabs (network, governance, trust). That address now hands
  #   you to the atom home and opens the fold holding what the tab used to show:
  #   network → Who holds it, governance → How it's governed, trust → Where it came from.
  # - steward: a person or household who has taken on keeping the resource. Each
  #   steward's row gives their share of that work and how the work is recognised.
  # - attestation: a signed statement someone has made about the resource (that they
  #   reviewed it, for example), shown with who made it and when.
  # - raise a concern: challenge the resource or give feedback about it, through a
  #   form that opens over the page. Raising one needs an account on this doorway.
  # - Who's here: the section beside the legs that says, in a sentence, how many
  #   times the resource has been viewed and finished. The doorway counts these as
  #   protocol events, not as analytics about the people who viewed it.
  # - neighbours: other resources shown under "See also". An authored neighbour is
  #   one the author named as related; a discovered neighbour is one the doorway
  #   found because the two share topic tags, and its row says "discovered".
  # - the tender: the short note above the reply box, written as if by the person's
  #   own agent, saying who a message will reach and that the author's standing
  #   travels with it.
  # - standing ring: a person's accumulated good-faith participation (their standing),
  #   shown as a small ring of dots beside their name rather than a number anyone
  #   could chase.
  # - Where people stand: the short-statements part of the conversation (agree /
  #   disagree / pass on one-line claims), shown beside the four legs.
  # - bridging: a statement that draws agreement across households, not just within one.
  #
  # A doorway is the hosted gateway a learner reaches the protocol through.
  # "evolution-of-trust" is a seeded html5 simulation; "succession" is a seeded
  # markdown article; "concept-bidirectional-trust" is an address nothing on this
  # doorway holds; "foundations-christian-technology" is a seeded learning path;
  # "confession", "constitution", "manifesto" and "theology" are seeded doctrinal
  # articles that name one another as related. Matthew is signed in on every
  # scenario; "a visitor who has not signed in" is a second, separate browser.
  # A scenario whose fixture is missing from the doorway (no piece map, no
  # stewards, no relationships) is held as pending rather than failed.
  # "What the doorway reports" means the test asks the doorway itself during the
  # run and compares the page with its answer, so the page can never show a
  # number of its own making; the scenarios name no fixed values for that reason.
  # @wip marks a scenario whose behaviour is not built yet; it is skipped in runs.
  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha" with device

  # --- The frame ---

  @browser-only
  Scenario: A simulation opens at its own home with the frame and no learning-app chrome
    # Matthew opens the universal address of an html5 simulation directly.
    # The page is the atom's home: identity, the content at full width, the legs.
    # Nothing on it belongs to the learning app.
    When Matthew opens the atom home for "evolution-of-trust"
    Then the atom home shows the title "The Evolution of Trust"
    And the atom home shows the reach chip "Commons"
    And the atom home shows the notarized chip
    And the focal slot renders the content at full width
    And the atom home shows the legs "Who holds it", "Where this lives", "How it's governed", "Where it came from"
    And the atom home shows no "Back to Lamad" control

  @browser-only
  Scenario: An article opens in the reading shape with the rail alongside
    # The same frame, a different focal shape: markdown reads at a column width
    # with the legs in a rail beside it, level with the top of the content — not
    # stacked under it the way the old page put its rail under the feedback controls.
    When Matthew opens the atom home for "succession"
    Then the focal slot renders the content in the reading shape
    And the legs sit in a rail beside the content

  @browser-only
  Scenario: The holding verdict is one sentence in household words
    # The holding sentence is the only verdict on the page — the same sentence the
    # doorway reports for this resource (today: "Held by only 1 household — invite
    # another to help hold these").
    When Matthew opens the atom home for "evolution-of-trust"
    Then the leg "Who holds it" reads the holding sentence the doorway reports for "evolution-of-trust"
    And the leg "Who holds it" shows the household floor the doorway reports for "evolution-of-trust"
    And the atom home shows no trust percentage
    And the shard map and replica counts stay behind a "Network detail" link

  @browser-only
  Scenario: Arrival is where you actually came from
    # Matthew reaches the simulation from another resource. The arrival chip
    # names that stop, not the learning app.
    Given Matthew is viewing the atom home for "succession"
    When Matthew follows a link to the atom home for "evolution-of-trust"
    Then the arrival chip names "Succession Without Conquest"
    And the atom home shows no "Back to Lamad" control

  @browser-only
  Scenario: A cold link has no arrival chip
    # Nobody walked Matthew here; there is nothing to go back to. The place the
    # resource lives is carried by the "Where this lives" leg instead.
    When Matthew opens the atom home for "evolution-of-trust" as a cold link
    Then the atom home shows no arrival chip
    And the leg "Where this lives" is present

  @browser-only
  Scenario: An address nothing here holds renders the designed gate
    # The resource is named as related by the simulation but no peer this doorway
    # can ask holds it. The gate says what is known and where to go; it never
    # offers to edit, rate, or invite a household to hold something it cannot see.
    Given "concept-bidirectional-trust" is not held by any peer doorway "alpha" can ask
    And Matthew is viewing the atom home for "evolution-of-trust"
    When Matthew follows the related link to "concept-bidirectional-trust"
    Then the out-of-reach gate is shown for "concept-bidirectional-trust"
    And the gate names "The Evolution of Trust" as the referring resource
    And the gate offers to go back to "The Evolution of Trust"
    And the atom home shows no edit, affinity, or invite controls

  # --- The legs unfold ---

  @browser-only
  Scenario: Network detail unfolds under "Who holds it", health in words before any number
    # Matthew has read the holding sentence and wants to see for himself. Until he
    # asks, the technical picture stays folded, so nobody is handed piece counts
    # they did not ask for. When he opens "Network detail" he reads first, in
    # words, how healthy the copies are; then where the pieces are, one cell for
    # each; and he can check, right now, whether the resource can still be rebuilt.
    # The answer is one line: whole, or not whole.
    Given doorway "alpha" knows how "evolution-of-trust" is split into pieces
    When Matthew opens the atom home for "evolution-of-trust"
    Then no piece counts are on the page while "Network detail" is folded
    When Matthew opens "Network detail" under "Who holds it"
    Then "Network detail" starts with the health of the copies in words, above the pieces
    And "Network detail" shows where the pieces are, one cell for each piece
    When Matthew presses "Check it's still whole" in "Network detail"
    Then one line answers whether "evolution-of-trust" is whole

  @browser-only
  Scenario: The old address lands on the atom home with the matching fold open
    # Links written before the atom home existed still point at the old viewer
    # page and name one of its tabs; this one names the network tab. Matthew
    # follows it. He should not land on a second page about the same resource,
    # nor be sent back to the learning app. The old address hands him to the atom
    # home with the fold that now holds what the tab used to show already open.
    When Matthew opens the old address "/resource/evolution-of-trust#network"
    Then the browser shows the atom home address "/epr/evolution-of-trust#holds"
    And "Network detail" is open under "Who holds it"
    And the atom home shows no "Back to Lamad" control

  @browser-only
  Scenario: Stewards and attestations sit under "Where it came from", with no trust percentage
    # The old viewer reduced trust to a percentage. Here Matthew reads who has
    # taken on keeping the resource, each steward with their share of the work
    # and how that work is recognised. When he opens "Attestations" he sees each
    # signed statement made about it, or one honest line saying nobody has made
    # one yet. Nothing on the leg turns trust into a score.
    Given doorway "alpha" knows who stewards "evolution-of-trust"
    When Matthew opens the atom home for "evolution-of-trust"
    Then every steward under "Where it came from" is named with a share and a recognition
    When Matthew opens "Attestations" under "Where it came from"
    Then "Attestations" lists each attestation the doorway reports for "evolution-of-trust", or reads "No one has attested to this yet."
    And nothing under "Where it came from" is a trust percentage

  @browser-only
  Scenario: Raising a concern happens on the atom home itself
    # Matthew thinks something in the simulation is wrong. Under "How it's
    # governed" he starts a challenge, and the form opens over the page he is
    # reading: objecting to a resource never sends him to another page. The
    # scenario stops at the open form; it files nothing.
    When Matthew opens the atom home for "evolution-of-trust"
    And Matthew opens the fold under "How it's governed"
    And Matthew raises a concern and chooses to challenge the resource
    Then the concern form is open on the atom home
    And the browser still shows the atom home for "evolution-of-trust"

  @browser-only
  Scenario: Someone who has not signed in is told how to raise a concern
    # A neighbour opens a shared link in a browser that is not signed in to this
    # doorway. Raising a concern needs an account here, so in place of a form
    # they cannot send, the page tells them in one line to sign in first.
    When a visitor who has not signed in opens the atom home for "evolution-of-trust"
    And the visitor opens the fold under "How it's governed"
    Then the visitor reads "Sign in to raise a concern." under "How it's governed"
    And the visitor is offered no concern form

  @browser-only
  Scenario: "Who's here" counts attention in words, never as analytics
    # Matthew wants to know whether anyone else has spent time with this. The
    # page gives him the same counts the doorway reports, said as a sentence
    # ("Viewed 12 times by 5 people · 3 completions"), or says that no one has
    # been counted here yet. There are no charts, gauges or percentages.
    When Matthew opens the atom home for "evolution-of-trust"
    Then "Who's here" says in words the attention the doorway reports for "evolution-of-trust"
    And "Who's here" shows no charts, gauges or percentages

  @browser-only
  Scenario: "See also" under "Where this lives" shows authored and discovered neighbours, told apart
    # The article "confession" was written alongside three others its author named
    # as related. The doorway also finds articles that share its topics. Matthew
    # sees both kinds under "See also", and every discovered one says so: a
    # suggestion offered for exploring is never passed off as the author's choice.
    # Each neighbour opens at its own atom home.
    Given "confession" names "constitution", "manifesto" and "theology" as related
    And doorway "alpha" finds at least one more neighbour of "confession" by shared topics
    When Matthew opens the atom home for "confession"
    And Matthew opens "See also" under "Where this lives"
    Then "See also" lists "constitution", "manifesto" and "theology" as authored neighbours
    And "See also" shows at least one neighbour marked "discovered"
    And every "See also" neighbour opens at its own atom home

  # --- The commons ---

  @wip @browser-only
  Scenario: The conversation opens empty with one honest line and a reply box
    # No one has spoken about the article here. The page says so in one line and
    # offers the first word; it does not render a blank feed or zero counts. The
    # tender above the reply box names where a message will reach — this resource
    # is commons, so "the commons".
    When Matthew opens the atom home for "succession"
    Then the conversation reads "No one has spoken about this here yet"
    And the reply box is present
    And the tender says a message will reach "the commons"

  @wip @browser-only
  Scenario: A message carries its author's household and standing, not a score
    # Matthew speaks. His message shows who he is and where he stands, as a shape
    # (the standing ring), never as a number others can chase.
    Given Matthew is viewing the atom home for "succession"
    When Matthew says "Reading this before the manifesto, as suggested." to the commons
    Then the conversation shows a message by "Matthew" with a standing ring
    And the message shows no upvote count

  @wip @browser-only
  Scenario: Where people stand surfaces a bridging statement
    # "Where people stand" is the short-statements section of the conversation
    # (agree / disagree / pass), beside the four legs. A statement agreed across
    # households is marked as bridging so the page elevates shared ground, not the
    # loudest claim.
    Given the statement "Trust needs repeat interaction more than it needs good intentions." exists for "evolution-of-trust" and is bridging
    When Matthew opens the atom home for "evolution-of-trust"
    Then the section "Where people stand" shows that statement tagged "Bridging"
    And Matthew can agree, disagree, or pass on it

  # --- The learning lens ---

  @browser-only
  Scenario: The learning app is one lens away
    # A learning path is a resource the learning app (Lamad) knows how to teach,
    # so its atom home offers a way into that context. The atom home is the door;
    # the learning app is one lens through it.
    When Matthew opens the atom home for "foundations-christian-technology"
    Then the atom home offers "Open in Lamad"
    And following it lands in the learning app's path view for "foundations-christian-technology"
