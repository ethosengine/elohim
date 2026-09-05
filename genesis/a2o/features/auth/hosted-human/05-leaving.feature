@e2e @auth @browser-only @requires:doorway @hosted-human @act:i @concern:hosted-human-lifecycle
Feature: A hosted human's whole life at a doorway, from the portal, leaving nothing behind
  As a newcomer who has been browsing a doorway anonymously
  I want to create an account, live in it, and close it again using only what the doorway shows me
  So that trying the network costs me nothing I cannot take back

  A HOSTED human is one whose doorway keeps their identity for them: the doorway holds their
  credential, mints their session, and runs a small conductor cell on their behalf on one of
  the conductors it operates (its "pool"). A cell is the doorway-side home of that human's
  own record chain — the thing that makes their writes theirs. The doorway calls this
  arrangement "hosting"; the agency pipeline on the account page calls this stage "Hosted".
  It is the first stage of agency and the only one this story covers. Graduating beyond it —
  exporting keys, running your own conductor, becoming a steward — is a later story and is
  deliberately not asserted here.

  Two words the scenarios lean on. The IDENTIFIER is the username the human chooses at
  registration, qualified by the doorway's own domain (`newcomer@alpha.elohim.host`): it is
  what they type to sign in, what the doorway shows them as their account, and what they
  must type back to confirm they mean to close it. The DISPLAY NAME is the name they asked
  to be known by; the doorway must show it back unchanged.

  Two things must be true for the stage to be real, and they are the two things a portal
  that merely paints forms cannot fake. First, an account is a person's own: the display name
  they typed is the name the doorway shows back, and the cell the doorway runs for them is
  theirs alone, not a shared one that hands every newcomer the operator's profile. Second,
  closing the account undoes the hosting: the credential no longer signs anyone in, the cell
  is gone from every pool conductor, and the account store no longer carries an active
  account under that name. "Clean" means the doorway's own resources are reclaimed. The
  network's notary keeps whatever it witnessed — that is what a notary is for — so this story
  never claims the network forgot the human, only that the doorway stopped hosting them.

  Every scenario drives the doorway's own origin, because the portal calls the doorway on the
  origin that serves it. Nothing here needs a
  fixture human: the person is created inside the story and removed by it, so the same
  scenarios run against the household mesh, a hybrid mesh, or a deployed doorway, and leave
  the deployment as they found it. That is also why the finish line is checked from two
  sides: what the browser shows, and what the doorway answers when asked directly.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a browser is open on doorway "alpha"

  # ---------------------------------------------------------------------------
  # The finish line: one life, start to end, nothing but the portal.
  # ---------------------------------------------------------------------------

  Scenario: A newcomer creates an account, is hosted as themselves, and closes it again
    When the browser opens the doorway registration portal
    Then the portal renders its registration form

    When the newcomer creates an account through the portal with the display name "Newcomer"
    Then the doorway confirms a session for that human
    And the doorway names that human "Newcomer"
    And the doorway holds a cell for that human on one of its pool conductors

    When the human opens their doorway account page
    Then the account page shows the identifier the doorway issued at registration
    And the agency pipeline marks "Hosted" as the current step
    And the agency pipeline marks no later step as completed

    When the human closes their account through the portal
    Then the portal returns to the signed-out doorway landing
    And the doorway confirms no session for that human

    When the browser opens the doorway sign-in portal
    And the human attempts to sign in through the portal with the password they registered with
    Then the portal shows a sign-in error
    And the doorway confirms no session for that human
    And no pool conductor holds a cell for that human
    And the doorway's account store holds no active account for that identifier

  # ---------------------------------------------------------------------------
  # Stations. Each one is a claim the finish line depends on, checked alone so a
  # red names the station that broke rather than the whole life. The account-page
  # claim ("Hosted" is the current step) is carried by agency-pipeline-coherence.feature
  # and is not repeated as a station here. Where a station starts from an already
  # registered human, the precondition registers a fresh human through the doorway's
  # API with the named display name and a generated identifier the story remembers.
  # ---------------------------------------------------------------------------

  Scenario: Registering through the portal gives the newcomer their own name and their own cell
    When the browser opens the doorway registration portal
    And the newcomer creates an account through the portal with the display name "Newcomer"
    Then the doorway confirms a session for that human
    And the doorway names that human "Newcomer"
    And the doorway holds a cell for that human on one of its pool conductors
    And that cell belongs to no other account on the doorway


  Scenario: Two newcomers registering on the same doorway are two different people
    When the browser opens the doorway registration portal
    And the newcomer creates an account through the portal with the display name "First"
    Then the doorway confirms a session for that human

    When a second browser opens the doorway registration portal
    And that browser's newcomer creates an account through the portal with the display name "Second"
    Then the doorway names the first human "First"
    And the doorway names the second human "Second"
    And the two humans hold different cells on the doorway's pool conductors


  Scenario: Closing the account asks the human to confirm, then signs them out
    Given a hosted human "Stranger" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    And the human begins closing their account
    Then the portal asks the human to confirm by typing their identifier

    When the human confirms with the wrong identifier
    Then the account is not closed
    And the doorway still confirms a session for that human

    When the human confirms with their own identifier
    Then the portal returns to the signed-out doorway landing
    And the doorway confirms no session for that human


  Scenario: A closed account cannot sign in, and the doorway keeps nothing that would host it
    Given a hosted human "Stranger" is registered on doorway "alpha"
    And the human has closed their account
    When the browser opens the doorway sign-in portal
    And the human attempts to sign in through the portal with the password they registered with
    Then the portal shows a sign-in error
    And the doorway confirms no session for that human
    And no pool conductor holds a cell for that human
    And the doorway's account store holds no active account for that identifier


  Scenario: Closing an account twice is harmless
    Given a hosted human "Stranger" is registered on doorway "alpha"
    And the human has closed their account
    When the closure is requested again with the session the human held before closing
    Then the doorway answers that the account is already closed
    And no pool conductor holds a cell for that human
