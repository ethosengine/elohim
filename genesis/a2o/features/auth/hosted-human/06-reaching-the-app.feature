@e2e @auth @browser-only @requires:doorway @hosted-human @act:i @concern:hosted-human-reaching-the-app
Feature: A hosted human reaches an application through their own doorway
  As a hosted human who opened an application and was asked to sign in
  I want the application to hand me to my own doorway and then take me back where I was going
  So that the only place I ever type my password is the one that already keeps it

  This is station 4 of the hosted-human series — one ordered life at a doorway, told a station
  at a time, where a station may assume every earlier station but no later one. It assumes the
  person has an account (station 1) and that their doorway can sign them in (station 2). What
  it adds is the crossing: the moment a person stops looking at their doorway and starts using
  something built on top of it.

  An APPLICATION here is a program served over the protocol that is not the doorway's own
  portal — the learning app, a future app someone else writes. An application is a RELYING
  PARTY: it can be told who someone is, but it cannot decide it. There are exactly two sign-in
  surfaces in the whole protocol, and an application is neither of them. A hosted human's is
  their doorway's own portal, on the doorway's own origin — the one every station in this
  series is told against. A person who runs their own machinery signs in at that machinery's
  own portal instead, which answers for their key rather than holding it; that is the
  graduation series and not this one. So the interesting question at this station is not "does
  the application let me in" — it is "how does the application get out of the way, and does it
  get me back afterwards".

  Five words the scenarios lean on.

  THE IDENTIFIER is what a person types to say both who they are and where they sign in: their
  name joined to their doorway's own domain, `linnet@alpha.elohim.host`, exactly as station 1
  issued it. RESOLVING it is the one thing the application needs from them before it steps
  aside: it reads the host after the `@` and asks that host, for itself, whether it is a
  doorway and where its portal is. The host answers or it does not; the application believes
  nothing else and invents nothing. Resolving is not signing in — nothing about the person is
  proved by it, and no password has been typed anywhere yet.

  THE APPLICATION'S REQUEST is what the application asks the doorway for when it sends someone
  away to sign in: which application is asking, which address the answer may be delivered to,
  what kind of answer it wants, and an opaque value of its own that must come back unchanged so
  it can tell this answer apart from one somebody else forged. If any of that is dropped in
  transit, the person can sign in perfectly and still be stranded — signed in to nothing, with
  no way back to the thing they were trying to do.

  THE PLACE THEY WERE GOING is where the person was headed when the application asked them to
  sign in — `/lamad`, the learning part of the application, in every scenario below, standing
  in for any destination inside it. It survives the whole crossing, so that signing in feels
  like a pause rather than a restart.

  The TRUST CHROME is the small standing mark the application keeps in view saying whose
  hosting the person is relying on right now. Before they sign in it names the doorway
  ("Hosted via" and the doorway's own name); after they sign in it names them. It matters
  because a sign-in page that does not say whose password box it is about to show you is
  exactly the page a phishing site imitates.

  The FLYWHEEL HINT is the small mark beside it saying that being hosted is a stage and not a
  destination — this doorway can hand the person their own machinery later. It is the first
  place in an ordinary person's life where the protocol admits that the arrangement they are in
  is not the only one available.

  Judged from two sides throughout, because a page can look signed-in and hold nothing: a step
  that begins "the application", "the portal", "the trust chrome" or "the browser" reads what
  the browser is showing, while a step that begins "the doorway" asks the doorway itself and
  ignores the browser entirely.

  The person is created inside the story and removed by it, so these scenarios run against the
  household mesh (a cluster of peers on the machine running the test), a hybrid mesh, or a
  deployed doorway, and leave the deployment as they found it.

  Two neighbours own the halves this story deliberately does not repeat. What the doorway does
  with the request when it arrives — refusing an unregistered application, refusing a delivery
  address outside the bound, spending a code exactly once — is
  `../oauth-authorization-code.feature`. What an application is allowed to learn about the
  doorway that serves it, and why that answer can never name a foreign origin, is
  `../auth-discovery.feature`. This story is the person walking the path those two describe.

  How to read the tags and comments. Every scenario carries `@wip` because the glue that drives
  the application's side of the crossing is not written yet — not because the claims are unbuilt.
  A paragraph beginning "GAP today" inside a scenario names a place where behaviour falls short
  of the scenario it sits in; it is written into the scenario body rather than a comment so that
  the shortfall travels with the story wherever the story is read. The scenario states what the
  person deserves and is deliberately not softened to match.

  Background:
    # "E2E_DOORWAY_ALPHA" is the environment variable holding the address of the doorway under
    # test, so the same scenarios run against whichever doorway that variable points at.
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a browser is open on the application served by doorway "alpha"

  # ---------------------------------------------------------------------------
  # Before the crossing: the application says where it is about to send them.
  # ---------------------------------------------------------------------------

  @wip
  Scenario: The application asks which doorway I sign in at, and names it before I go anywhere
    The chrome here names the doorway that SERVES this application, which the application knows
    from the origin it was loaded from before anyone types anything. That is the point: a person
    can read whose sign-in they are about to be shown before they have given up a keystroke. No
    registered human is needed for it — this is the page any visitor meets.

    GAP today: the application advances from this step to a password card of its own, so it asks
    for a password on a page the doorway does not serve. That third surface is what this station
    exists to remove; the scenario asserts the shape after the removal.

    When the browser opens the application's sign-in
    Then the application asks which doorway signs this human in
    And the application never asks for a password
    And the trust chrome reads "Hosted via" and names doorway "alpha"
    And the trust chrome shows the flywheel hint

  # ---------------------------------------------------------------------------
  # The crossing itself, end to end. This is the station's finish line.
  # ---------------------------------------------------------------------------

  # The browser-side twin of "A signed-out human is redirected to log in without losing the
  # request" in ../oauth-authorization-code.feature, which asserts the same hand-over from the
  # doorway's side. Here it is judged as the person experiences it, and carried through to the
  # far end: they are only actually back when the place they were going is on the screen.
  @wip
  Scenario: My doorway takes the password, and the application takes me back where I was going
    Given a hosted human "Linnet" is registered on doorway "alpha"
    When the browser opens the application's sign-in on the way to "/lamad"
    And the human resolves their identifier at the application
    Then the browser is at the sign-in portal of doorway "alpha"
    And the portal renders its sign-in form
    And the portal carries the request the application sent

    When the human signs in through the portal
    Then the browser is handed back to the application that asked
    And the doorway confirms a session for that human
    And the application opens "/lamad"
    And the trust chrome names the human rather than only the doorway
    And the human's password was typed only on the doorway's own origin

  # ---------------------------------------------------------------------------
  # A refusal, and where it is allowed to happen.
  # ---------------------------------------------------------------------------

  # A wrong password is a doorway's business, and it must stay there: the application is not
  # told, is not partly signed in, and does not get to soften or restate the refusal. What the
  # person keeps through it is their bearings — the mark saying whose doorway just said no, and
  # a way to do something about it.
  @wip
  Scenario: A wrong password is refused at my doorway, and the application never gets a session
    GAP today: the recovery link the shipped chrome offers points back into the application's own
    routes, so a person standing on the doorway's portal is sent off it to recover a credential
    the doorway holds. The offer must resolve at the doorway that refused them.

    Given a hosted human "Linnet" is registered on doorway "alpha"
    When the browser opens the application's sign-in on the way to "/lamad"
    And the human resolves their identifier at the application
    And the human submits a wrong password through the portal
    Then the portal shows a sign-in error
    And the trust chrome still names doorway "alpha"
    And the portal offers the human a way to recover their account
    And the browser was not handed back to the application
    And the doorway confirms no session for that human

  # ---------------------------------------------------------------------------
  # Refusing to invent a doorway. The identifier names a host; the application may only
  # believe hosts that have said, for themselves, that they are doorways.
  # ---------------------------------------------------------------------------

  # An application that guessed here would be a redirect cannon: type a name ending in any
  # domain and the application would send the browser — and the request it is carrying — to
  # whatever answered there. The refusal is the feature. ../auth-discovery.feature asserts the
  # other half of the same guard: what a doorway publishes about itself cannot name a foreign
  # origin either.
  @wip
  Scenario: An identifier at a host no doorway has claimed is refused rather than guessed at
    No registered human here either, and deliberately so: the refusal must hold for any browser
    and anyone typing into it, including someone who has never had an account anywhere.

    When the browser opens the application's sign-in
    And the human resolves an identifier at a host no doorway has claimed
    Then the application refuses to resolve that identifier
    And the browser is still at the application's sign-in
    And the browser was never sent to the host named in that identifier
    And the application never asks for a password

  # ---------------------------------------------------------------------------
  # No account yet. The same crossing, aimed at the other portal surface.
  # ---------------------------------------------------------------------------

  # The browser-side twin of "A signed-out human asking to create an account is sent to
  # registration without losing the request" in ../oauth-authorization-code.feature. What
  # happens once they are standing there — the form, its refusals, and being put back in the
  # application afterwards — is station 1, `01-creating-an-account.feature`, and is deliberately
  # not repeated here. This scenario asserts only the hand-over: that asking for an account
  # takes them to the doorway rather than to a form the application drew itself, and that the
  # application's request survives the trip.
  @wip
  Scenario: Asking for an account takes me to my doorway's registration, carrying the same request
    GAP today: the application still serves a registration form of its own and posts it straight
    to the doorway, which is the second half of the third surface this station removes.

    When the browser opens the application's sign-in on the way to "/lamad"
    And the human asks the application for an account instead
    Then the browser is at the registration portal of doorway "alpha"
    And the portal renders its registration form
    And the portal carries the request the application sent
