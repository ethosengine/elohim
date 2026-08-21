@e2e @content @landing @discovery @requires:doorway @act:i
Feature: The elohim.host doorstep invites a visitor into the protocol
  As a first-time visitor arriving cold at elohim.host — someone who has
  never heard the word "Lamad" and owes the protocol nothing
  I want the doorstep to tell me a story I can enter, not a manifesto I must
  decode, and to offer me a first step sized for a stranger
  So that the front door of a network organized around love reads as an
  invitation into learning, not a wall of doctrine — and so that a curious
  person becomes a learner, a learner a household, a household a steward.

  # This surface is the redesign of the home route from a hardcoded
  # manifesto-compression into a discovery surface. The doorstep now:
  #   - opens with the hero in the "technology organized around love" register
  #     and a single primary invitation ("Start the journey");
  #   - tells the story arc (what was traded away -> the substrate answer ->
  #     what is already here) with the pillar glosses;
  #   - shows six epic story cards that are LIVE reference cards — each resolves
  #     a real EPR head (title, description, reach glyph, steward badge) and
  #     opens its own EPR address, so the front page is dogfooded protocol
  #     content, not marketing copy;
  #   - offers several ways in for different kinds of visitor (the guided path,
  #     the who-are-you self-router quiz, the evolution-of-trust explorable, and
  #     a plain browse-everything door into lamad);
  #   - frames the whole invitation with the Progressive Stewardship ladder
  #     (Visitor -> Hosted -> App Steward -> Node Steward), so the visitor can
  #     see the road from stranger to steward from the very first screen.
  #
  # Cross-bundle links are plain hrefs to the universal EPR address, never a SPA
  # routerLink into lamad — a card follows the same content-addressed door a
  # shared link would, so the doorstep cannot silently centralize its own copy.
  #
  # Household floor: the landing and its EPR-head reads are served by the alpha
  # doorway over matthew's storage projection — no cross-node discovery, so no
  # @requires:shem (mirrors features/content/epr-content-facing-raw.feature).
  #
  # DOM contract (the data-testid vocabulary the redesigned home route emits;
  # documented here so the step-def author and the redesign share one surface):
  #   landing-hero                — the hero band (love register + headline)
  #   landing-cta-start           — the primary "Start the journey" invitation
  #   landing-story-arc           — the traded-away / answer / already-here arc
  #   landing-pillars             — the five pillar glosses
  #   landing-epic-cards          — the six live epic story cards, together
  #   landing-epic-card           — a single epic story card (app-epr-relationship-card)
  #   landing-ways-in             — the "Find your way in" section
  #   landing-wayin-quiz          — the who-are-you self-router door
  #   landing-wayin-trust         — the evolution-of-trust explorable door
  #   landing-wayin-browse        — the browse-everything door into lamad
  #   landing-stewardship-ladder  — the Progressive Stewardship ladder
  #   landing-manifesto-card      — the manifesto as one epr-linked reference card

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- (a) The visitor is met by a story and a first step -------------------

  @browser-only
  Scenario: A first-time visitor is welcomed by an invitation, not a wall of manifesto
    # The doorstep leads with warmth and a single clear step. A stranger should
    # feel invited in — the love register in the hero, one primary call — not
    # asked to swallow the whole doctrine before they may proceed.
    When I open the landing page in a browser
    Then the element [data-testid="landing-hero"] is visible
    And the element [data-testid="landing-hero"] text contains "love"
    And the element [data-testid="landing-cta-start"] is visible

  @browser-only
  Scenario: The story arc carries the visitor from what was traded away to what is already here
    # Below the invitation the doorstep tells a story the visitor can walk
    # through — the trade the old web asked of us, the answer the substrate
    # offers, and what is already standing here today — with the pillars named
    # as the shape of that answer.
    When I open the landing page in a browser
    Then the element [data-testid="landing-story-arc"] is visible
    And the element [data-testid="landing-pillars"] is visible

  @wip @browser-only
  Scenario: Start the journey carries the visitor into the onboarding learning path
    # The primary call is not a marketing scroll-jump — following it drops the
    # visitor at the start of the elohim-protocol onboarding journey, the guided
    # first path sized for someone who arrived a moment ago.
    When I open the landing page in a browser
    And I click the element [data-testid="landing-cta-start"]
    Then the start-here call carries the visitor into the "elohim-protocol" onboarding journey

  # --- (b) The epic story cards are living protocol content -----------------

  @browser-only
  Scenario: The six epic story cards render on the doorstep
    # The front page is dogfooded protocol content: the epics are shown as a
    # deck of reference cards, not hand-written blurbs. The deck renders, and at
    # least the first card stands up on the doorstep.
    When I open the landing page in a browser
    Then the element [data-testid="landing-epic-cards"] is visible
    And the element [data-testid="landing-epic-card"] is visible

  @wip @browser-only @requires:seeded-content
  Scenario: Each epic story card resolves a living EPR head and opens its own address
    # Every epic card resolves a real EPR head — its authored title and
    # description, the reach glyph that says who may receive it, and the steward
    # badge that names who carries it forward. Following a card walks the same
    # content-addressed door a shared link would, into that epic's own EPR
    # address — never a SPA hop that quietly forks a private copy.
    When I open the landing page in a browser
    Then each epic story card shows a living title, a reach glyph, and a steward badge
    And following an epic story card opens that epic's own EPR address

  # --- (c) Several ways in for several kinds of visitor ---------------------

  @browser-only
  Scenario: Find your way in offers the guided path, the self-router, and a door to browse everything
    # Not every visitor wants the same first step. The doorstep offers the
    # guided start-here path, a who-are-you self-router for the unsure, an
    # evolution-of-trust explorable for the curious, and a plain door to browse
    # everything for the impatient — all reachable from one screen.
    When I open the landing page in a browser
    Then the element [data-testid="landing-ways-in"] is visible
    And the element [data-testid="landing-wayin-quiz"] is visible
    And the element [data-testid="landing-wayin-trust"] is visible
    And the element [data-testid="landing-wayin-browse"] is visible

  @wip @browser-only
  Scenario: A visitor who is unsure is carried to the who-are-you self-router
    # The unsure visitor takes the self-router door, and the doorstep hands them
    # off to the who-are-you experience that will route them to the path that
    # fits — a stranger is never left to guess which door is theirs.
    When I open the landing page in a browser
    And I follow the "who-are-you" way in
    Then the visitor arrives at the who-are-you self-router

  @wip @browser-only
  Scenario: A curious visitor opens the evolution-of-trust explorable
    # The curious visitor takes the explorable door and lands in the
    # evolution-of-trust experience — an invitation to play with the idea before
    # committing to a path.
    When I open the landing page in a browser
    And I follow the "evolution-of-trust" way in
    Then the visitor arrives at the evolution-of-trust explorable

  # --- (d) The road from stranger to steward is visible from the door -------

  @browser-only
  Scenario: The stewardship ladder shows how a visitor can grow into a node steward
    # The invitation is framed by the graduation model, so from the very first
    # screen the visitor can see the whole road — from arriving as a Visitor,
    # to being Hosted, to stewarding an App, to running a Node — and understand
    # that participation deepens rather than gates.
    When I open the landing page in a browser
    Then the element [data-testid="landing-stewardship-ladder"] is visible
    And the element [data-testid="landing-stewardship-ladder"] text contains "Visitor"
    And the element [data-testid="landing-stewardship-ladder"] text contains "Hosted"
    And the element [data-testid="landing-stewardship-ladder"] text contains "App Steward"
    And the element [data-testid="landing-stewardship-ladder"] text contains "Node Steward"

  # --- (e) An unreachable card degrades to a legible fallback, never blank ---

  # The manifesto is shown as one epr-linked reference card. Its body is known
  # to fail to resolve on alpha today (reach drift on the manifesto atom returns
  # 403 for the anonymous reader — see the EprRouter / reach-enum-drift lineage).
  # A doorstep card that cannot reach its body must degrade gracefully: it keeps
  # its title and offers an honest "content unavailable" affordance the visitor
  # can act on. A blank card on the front door is the failure this guards.
  @wip @browser-only @requires:seeded-content
  Scenario: A reference card whose body cannot be reached still renders a legible fallback
    When I open the landing page in a browser
    Then the manifesto reference card renders a legible fallback with its title and an unavailable affordance
    And no reference card on the doorstep is left blank
