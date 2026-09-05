@e2e @auth @browser-only @requires:doorway @hosted-human @act:i @concern:hosted-human-creating-an-account
Feature: A newcomer creates an account at the doorway's own portal
  As someone who has been browsing a doorway anonymously and has decided to stay
  I want the registration form to give me exactly the account it appeared to promise
  So that the first thing this network does for me is something I can check

  This is station 1 of the hosted-human series — one ordered life at a doorway, told a
  station at a time, where a station is a phase the person passes through and may assume every
  earlier station but no later one. (The numbered files in this directory do not run
  one-to-one with station numbers: some stations' stories were born elsewhere and are listed
  by path in the series README.) It assumes nothing before it but an anonymous
  visitor with a browser. A HOSTED human is one whose doorway keeps their identity for them:
  the doorway holds their credential, mints their session, and runs the signing machinery that
  makes their writes provably theirs — all on a machine the person does not own. Registration
  is the moment they hand those three things over, and it is the most ordinary story in the
  suite: a form, a name, a password. It is worth its own feature because every later station
  assumes it produced a real account rather than a well-drawn page.

  The PORTAL is the small application the doorway serves for its own humans; the registration
  form, the sign-in form, and the account page are three surfaces of that one portal, and it
  calls the doorway that served it rather than any other.

  Three words the scenarios lean on. The USERNAME is what the human types into the identifier
  field. The DOORWAY'S DOMAIN is the name this doorway answers to. The IDENTIFIER is the two
  joined — `wren@alpha.elohim.host` — and it is the account: what they type to sign in, what
  the doorway shows them on their account page, and what the doorway must hand back when
  asked who this session belongs to. The DISPLAY NAME is separate: the name they asked to be
  known by, which the doorway must show back unchanged.

  The doorway is not a general-purpose identity provider, and this is where that becomes
  concrete. Whatever the human types — a bare username, or a whole email address they have
  used everywhere else for years — the account they get is an account AT THIS DOORWAY. That
  is not a trap if they can see it happening before they commit, and it is a trap if they
  cannot, which is why the story asserts both the fold and its visibility.

  Registration is reached two ways: directly, by a person who decided to stay; or because an
  application sent them here to sign in and they had no account yet. In the second case the
  application's request travels with them, and finishing registration must put them back in
  the application that asked, carrying the same request it sent — otherwise they have an
  account and no way back to the thing they were trying to do.

  Judged from two sides throughout, because a form can be flawless and mint nothing: what the
  portal shows, and what the doorway answers when asked directly about the account and the
  session it claims to have created. Refusals are judged the same way — a refusal that leaves
  a half-made account behind is not a refusal.

  The person is created inside the story with a username minted for that run, so these
  scenarios run against the household mesh (a cluster of peers on the machine running the
  test), a hybrid mesh (that cluster joined to a deployed peer), or a deployed doorway —
  without colliding with anyone real.

  How to read the tags and the comments. Every scenario here carries `@wip` because the glue
  that drives these surfaces is not written yet — not because every claim in them is unbuilt.
  A comment beginning GAP names a place where the doorway's behaviour today falls short of the
  scenario beside it; the scenario still states what the human deserves and is deliberately
  not softened to match. And the two sides are legible from the step text itself: a step that
  begins "the portal", "the registration form", "the sign-in form" or "the account page" reads
  what the browser is showing, while a step that begins "the doorway" asks the doorway itself
  and ignores the browser entirely.

  Background:
    # "E2E_DOORWAY_ALPHA" is the environment variable holding the address of the doorway under
    # test, so the same scenarios run against whichever doorway that variable points at.
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a browser is open on doorway "alpha"

  # ---------------------------------------------------------------------------
  # What the account IS: a name given back, and an identifier at this doorway.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: The name I ask to be known by is the name the doorway gives back
    When the browser opens the doorway registration portal
    Then the portal renders its registration form

    When the newcomer types a username of their own into the registration form
    And the newcomer creates an account through the portal with the display name "Wren"
    Then the doorway confirms a session for that human
    And the doorway names that human "Wren"

  @wip
  Scenario: My identifier is my username at this doorway, and the form says so before I commit
    When the browser opens the doorway registration portal
    Then the registration form shows the doorway's domain beside the username field

    When the newcomer types a username of their own into the registration form
    And the newcomer creates an account through the portal with the display name "Wren"
    Then the doorway confirms a session for that human
    And the identifier the doorway issued joins that username to the doorway's own domain

    When the human opens their doorway account page
    Then the account page shows the identifier the doorway issued at registration

  # The fold itself is real — a whole email address becomes a local account here. What is
  # missing is the telling: the sign-in form strips an '@' in front of the human as they
  # type, and the registration form does not, so a newcomer typing their lifelong email
  # only learns what account they got afterwards.
  @wip
  Scenario: Typing my whole email address gives me an account here, and the form shows that first
    When the browser opens the doorway registration portal
    And the newcomer types their whole email address into the username field
    Then the registration form keeps only the part before the "@" as the username
    And the registration form shows the account name it will create at this doorway
    And the registration form does not offer to create an account at the domain they typed

    When the newcomer creates an account through the portal with the display name "Wren"
    Then the doorway confirms a session for that human
    And the identifier the doorway issued joins that username to the doorway's own domain
    And the identifier the doorway issued does not carry the domain they typed

  # ---------------------------------------------------------------------------
  # Refusals. Each one must be legible AND must leave nothing behind.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: A username someone already holds is refused, and the refusal says which problem it is
    Given a hosted human "Rook" is registered on doorway "alpha"
    When the browser opens the doorway registration portal
    And the newcomer tries to register with the username that human already holds
    Then the portal shows a registration error naming the username as already taken
    And the doorway holds exactly one account for that identifier
    And that account still belongs to the human who registered first

  @wip
  Scenario: A password the doorway will not accept is refused cleanly, and fixing it finishes the account
    When the browser opens the doorway registration portal
    And the newcomer tries to register with a password shorter than the doorway allows
    Then the portal shows a registration error naming the password as too short
    And the doorway holds no account for the username they typed

    When the newcomer tries to register with a confirmation that does not match the password
    Then the portal refuses to submit the registration
    And the doorway holds no account for the username they typed

    When the newcomer finishes creating the account with a password the doorway accepts
    Then the doorway confirms a session for that human
    And the doorway holds exactly one account for that identifier

  # ---------------------------------------------------------------------------
  # Getting out of registration without losing the work, in both directions.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: The link for people who already have an account reaches sign-in
    When the browser opens the doorway registration portal
    And the newcomer types a username of their own into the registration form
    And the newcomer follows the link for people who already have an account
    Then the portal renders its sign-in form

  # GAP: the link is a plain navigation, so the username the human just typed is dropped on
  # the way. This scenario is the one they deserve and it does not hold today; the one above
  # asserts only what does.
  @wip
  Scenario: Reaching sign-in that way does not make me type my username again
    When the browser opens the doorway registration portal
    And the newcomer types a username of their own into the registration form
    And the newcomer follows the link for people who already have an account
    Then the sign-in form already holds the username they had typed

  @wip
  Scenario: Registering because an application asked me to puts me back in that application
    Given an application sent the newcomer to this doorway to sign in
    When the browser opens the doorway registration portal
    Then the portal names the application that asked for the account

    When the newcomer creates an account through the portal with the display name "Wren"
    Then the doorway confirms a session for that human
    And the browser is handed back to the application that asked
    And the application is handed back the same request it sent
