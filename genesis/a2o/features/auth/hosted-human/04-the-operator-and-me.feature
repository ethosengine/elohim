@e2e @auth @browser-only @requires:doorway @hosted-human @act:i @concern:hosted-human-the-operator-and-me
Feature: What a hosted human is told when the operator acts on their account
  As a hosted human whose doorway is run by somebody else
  I want to be told what changed about my account, in words that name what happened
  So that someone else's authority over my hosting is legible to me instead of arbitrary

  This is station 7 of the hosted-human series — one ordered life at a doorway, told a
  station at a time, where a station is a phase the person passes through and may assume every
  earlier station but no later one. (The numbered files in this directory do not run
  one-to-one with station numbers: some stations' stories were born elsewhere and are listed
  by path in the series README.) It assumes the stations before it and nothing
  after: the human has an account here and the PORTAL — the small application the doorway
  serves for its own humans, holding the sign-in form and the account page — has signed them
  in. This is the station where the arrangement shows its teeth. Being hosted means someone
  else runs the doorway that holds the identity.
  That someone — the OPERATOR — can end a human's access and can change how much of the
  doorway that human is allowed to use. Both are legitimate: a doorway with no way to stop a
  bad actor or to protect its own capacity is not a doorway anyone can afford to run.

  What is at stake is not whether the operator may act. It is whether the person on the other
  end can tell WHAT happened. Every operator act reaches the human as some door not opening,
  and the failures are indistinguishable from the inside unless the doorway names them: an
  account that was suspended looks exactly like a forgotten password, and a lowered ceiling
  looks exactly like a broken upload. A doorway that refuses without naming the reason turns
  ordinary administration into something the human can only experience as the system
  malfunctioning at them.

  Two acts are covered, in the human's terms. SUSPENSION: the operator stops the account from
  working, and later lifts it. A CEILING CHANGE: the operator moves an ALLOWANCE — one of the
  three ceilings the account page shows beside what the human has used (how much storage, how
  many daily queries, how much daily bandwidth). Storage is the proving case below; the other
  two ceilings are moved and shown the same way, so a scenario each would repeat the claim
  rather than extend it. Suspension is not deletion — the account survives it, and lifting must
  give back the same account rather than a fresh one. Closing an account for good is the
  human's own act, told in `05-leaving.feature`.

  Two surfaces speak, and the scenarios hold both to it, because they can fail apart. The
  DOORWAY is what actually refuses — it stops answering for the session and states a reason
  when asked. The PORTAL is what the human reads: it can receive that reason and pass it on
  in plain words, or discard it and show a bare sign-in form. A doorway that names the reason
  to a portal that swallows it leaves the human exactly as uninformed as no reason at all.

  This story is deliberately one-sided. The operator's surface — listing accounts, viewing
  another human's details, making the change — is asserted in `../user-management.feature` and
  is not repeated here. Everything below is what the hosted human sees and is told.

  The person is still created inside the story and signs in through the portal. The only
  operator hand in it is the act under test, made through the doorway's own administrative
  surface with whatever operator credential the deployment under test provides; nothing else
  is arranged by hand, so these scenarios run against the household mesh (a cluster of peers on
  the machine running the test), a hybrid mesh (that cluster joined to a deployed peer), or a
  deployed doorway.

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
  # Suspension, and the words that come with it.
  # ---------------------------------------------------------------------------
  # GAP: the doorway names the reason when it refuses an open session, but a suspended human
  # who tries to sign in again is told their credentials are invalid — the same words a
  # mistyped password gets. So the one act they can take next teaches them the wrong thing:
  # they go hunting for a password that was never wrong.
  @wip
  Scenario: When I am suspended, the refusal names the suspension rather than blaming my password
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the doorway's operator suspends that human
    And the human opens their doorway account page
    Then the doorway refuses the session with the reason "account suspended"
    And the portal tells the human their account is suspended

    When the human attempts to sign in through the portal with the password they registered with
    Then the portal tells the human their account is suspended
    And the portal does not tell the human their password was wrong

  @wip
  Scenario: Lifting the suspension gives me back the same account, not a new one
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    And the doorway's operator has suspended that human
    When the doorway's operator lifts the suspension on that human
    And the human signs in through the portal
    Then the doorway confirms a session for that human
    And the doorway names that human "Wren"

    When the human opens their doorway account page
    Then the account page shows the identifier the doorway issued at registration
    And every allowance the account page shows is the one the doorway answers with

  # ---------------------------------------------------------------------------
  # Ceilings, and knowing where they now are.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: When my storage ceiling is lowered, my account page shows the ceiling I now have
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    And the doorway's operator lowers that human's storage ceiling
    And the browser reloads the page
    Then the account page shows the lowered storage ceiling
    And every allowance the account page shows is the one the doorway answers with
    And the storage the account page shows as used is unchanged

  # A ceiling can land below what the human has already stored. That is a normal thing for an
  # operator to do and a confusing thing to be on the receiving end of, so the page has to say
  # it plainly: over the line, still yours, still open.
  @wip
  Scenario: A ceiling lowered below what I already use tells me I am over it, and does not lock me out
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    And the human has stored content on the doorway
    When the doorway's operator lowers that human's storage ceiling below what the human uses
    And the human opens their doorway account page
    Then the account page shows the human as over their storage ceiling
    And the account page shows the identifier the doorway issued at registration
    And the doorway confirms a session for that human
