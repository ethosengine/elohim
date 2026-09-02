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
  # - the legs: the four supporting sections that ground a resource in its network —
  #   Who holds it, Where this lives, How it's governed, Where it came from.
  # - holding sentence: the one plain sentence about who keeps copies, reported by the
  #   doorway itself, e.g. "Held by only 1 household — invite another to help hold these".
  #   Technical holding detail (shard maps, replica counts) lives behind a "Network
  #   detail" link, never on the home itself.
  # - household floor: how many households currently hold a copy, out of how many
  #   the resource wants for safety — shown as "1 of 3".
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
  # doorway holds; "foundations-christian-technology" is a seeded learning path.
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
    And the leg "Who holds it" shows the household floor as "1 of 3"
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
