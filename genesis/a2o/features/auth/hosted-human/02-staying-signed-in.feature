@e2e @auth @browser-only @requires:doorway @hosted-human @act:i @concern:hosted-human-staying-signed-in
Feature: A hosted human's session lasts as long as it should, and not one moment longer
  As a hosted human who signed in a while ago and is still working
  I want my session to survive the ordinary things I do and end when it is supposed to
  So that I am neither thrown out mid-thought nor left signed in when I am not

  This is station 3 of the hosted-human series — one ordered life at a doorway, told a
  station at a time, where a station is a phase the person passes through and may assume every
  earlier station but no later one. (The numbered files in this directory do not run
  one-to-one with station numbers: some stations' stories were born elsewhere and are listed
  by path in the series README.) It assumes station 2 and nothing later: the human
  has an account at this doorway, and the portal has signed them in with the password they
  registered with. What is at stake here is everything AFTER that moment.

  Two words carried in from the earlier stations. The IDENTIFIER is the account itself — the
  username the human chose, qualified by this doorway's own domain (`wren@alpha.elohim.host`).
  It is what they type to sign in and what the doorway hands back when asked whose session
  this is, so it is the anchor every scenario here checks against: the page must be showing
  THIS human's account and not a page that would look right for anybody. The PORTAL is the
  small application the doorway serves for its own humans — the sign-in form, the account
  page, and the toolbar with the sign-out control in it are all surfaces of that one portal.

  A SESSION is the doorway's answer to "who is this?". The portal keeps a copy of it in the
  browser so it does not have to ask the human for a password on every page; the doorway is
  the one that decides whether it still means anything. Those are two different things, and
  most of what can go wrong at this station is the two disagreeing: a browser that has
  forgotten a session the doorway still honours, or a browser that still shows a signed-in
  page for a session the doorway has stopped accepting.

  So each scenario is judged from both sides. The portal side asks what the human can see and
  reach. The doorway side asks the doorway directly whether it still answers for that session
  and, when it does not, what reason it gives. A page that keeps rendering because nobody
  asked the doorway is not a session; a doorway that keeps answering for a session the human
  ended is not a sign-out.

  Three endings are told apart on purpose, because they feel identical to a human staring at
  a sign-in form and are entirely different things. SIGNING OUT is the human ending it.
  EXPIRING is the session reaching the end of the life the doorway gave it. SUSPENSION is the
  operator ending it on the human's behalf, and it is the only one where the human deserves
  a reason they did not ask for. The operator's side of that act — how the suspension is made
  and lifted — is station 7's story, not this one; here it is only the thing that ends a
  session mid-life.

  The person is created inside the story and signs in through the portal, so these scenarios
  run against the household mesh (a cluster of peers on the machine running the test), a
  hybrid mesh (that cluster joined to a deployed peer), or a deployed doorway.

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
  # Staying.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: Reloading the page does not sign me out
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    Then the account page shows the identifier the doorway issued at registration

    When the browser reloads the page
    Then the account page shows the identifier the doorway issued at registration
    And the doorway confirms a session for that human

  # The doorway mints sessions that stand on their own — signing in makes another one rather
  # than replacing the one that exists. That is what makes a second device usable at all, and
  # it is the reason the sign-out scenario below has to be checked at the doorway and not
  # only in the browser.
  @wip
  Scenario: Signing in on a second browser leaves me signed in on the first
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When a second browser opens the doorway sign-in portal
    And the human signs in through the portal on the second browser
    Then the doorway confirms a session for that human on the second browser
    And the doorway confirms a session for that human on the first browser

    When the human opens their doorway account page on the first browser
    Then the account page shows the identifier the doorway issued at registration

  # ---------------------------------------------------------------------------
  # Ending it myself.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: After I sign out, my account page asks me to sign in again
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human signs out from the doorway toolbar
    And the human opens their doorway account page
    Then the portal renders its sign-in form
    And the account page is not shown

  # GAP — this scenario states what should be true, and today the second assertion is not.
  # Signing out clears the browser's copy and tells the doorway as a courtesy; the doorway
  # keeps no record of the sessions it has ended, so a copy taken before sign-out keeps
  # answering until it reaches the end of its own life. On a shared or borrowed machine,
  # signing out is not yet the guarantee the word carries.
  @wip
  Scenario: Signing out ends the session at the doorway, not only in my browser
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human signs out from the doorway toolbar
    Then the doorway confirms no session for that human
    And the doorway refuses the session it minted before the human signed out

  # ---------------------------------------------------------------------------
  # Endings I did not choose.
  # ---------------------------------------------------------------------------
  # GAP: a session the doorway no longer accepts returns the human to the sign-in form with
  # nothing said. "Your session ended, sign in again" and "something is broken" look the same
  # from there, and only one of them is worth waiting out.
  @wip
  Scenario: A session the doorway no longer accepts is refused honestly, and I can sign in again
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the session the doorway minted for the human reaches the end of its life
    And the human opens their doorway account page
    Then the portal renders its sign-in form
    And the portal tells the human their session ended rather than that something went wrong

    When the human signs in through the portal
    And the human opens their doorway account page
    Then the account page shows the identifier the doorway issued at registration
    And the doorway confirms a session for that human

  # A suspension has to reach a session that is already open, or a suspended human simply
  # keeps working until they happen to sign in again. What the human is told when it happens
  # is station 7's subject; what this scenario holds is that it happens at all.
  @wip
  Scenario: A suspended account's open session stops working, and the doorway names the reason
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the doorway's operator suspends that human
    And the human opens their doorway account page
    Then the doorway refuses the session with the reason "account suspended"
    And the account page is not shown
