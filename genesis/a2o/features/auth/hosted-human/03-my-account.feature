@e2e @auth @browser-only @requires:doorway @hosted-human @act:i @concern:hosted-human-my-account
Feature: The account page tells a hosted human who they are here and what they are using
  As a hosted human who is living on someone else's doorway
  I want one page that says who I am here, what I am using, and how far along I am
  So that being hosted is an arrangement I can see rather than one I have to trust blindly

  This is station 5 of the hosted-human series — one ordered life at a doorway, told a
  station at a time, where a station is a phase the person passes through and may assume every
  earlier station but no later one. (The numbered files in this directory do not run
  one-to-one with station numbers: some stations' stories were born elsewhere and are listed
  by path in the series README.) It assumes the stations before it and nothing
  after: the human has registered an account here, the portal has signed them in, and they
  have been using the doorway. The PORTAL is the small application the doorway serves for its
  own humans — the sign-in form and the account page are two surfaces of that one portal, and
  it is the only surface a hosted human is given. The account page is therefore the only place
  the arrangement becomes legible: it is where a hosted human finds out what their host knows
  about them.

  Being HOSTED means the doorway carries real cost on the human's behalf — it stores their
  bytes, answers their queries, and moves their traffic. So the page owes them two numbers per
  resource, not one: what they have USED, and what they are ALLOWED. A used figure with no
  ceiling cannot be acted on, and a ceiling with no used figure cannot be judged against. Both
  numbers belong to the doorway, so both are checked against what the doorway answers rather
  than against anything the page worked out for itself.

  The page also carries the AGENCY PIPELINE — the ladder from being hosted here toward running
  a node of one's own. It is drawn as four rungs in order: Hosted, then exporting one's own
  key, then installing the application, then standing as a steward. All four are drawn for
  everyone, so "nothing later is ticked" is a claim about the marks on the rungs and not about
  which rungs are on the page. This story asserts only the honest floor: a newly hosted human
  is at Hosted, and no rung beyond it is ticked as done. Whether the doorway's pipeline and the
  application's own badge tell the same story about a human further up that ladder is a
  different claim with its own home (`../agency-pipeline-coherence.feature`) and is not
  repeated here. Nothing in this story reaches past the hosted stage.

  Judged from two sides, because a page can render a plausible number for anybody: what the
  page shows, and what the doorway answers when asked about this account directly. The
  identifier is the anchor — the page must be showing THIS human's account, not a page that
  looks right for any account.

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
  # Who I am here.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: My account page shows the account the doorway actually issued me
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    Then the account page shows the identifier the doorway issued at registration
    And the identifier the account page shows is the one the doorway answers with

  # GAP: the doorway's answer about an account carries no display name, so the page shows the
  # identifier and nothing else. The human typed a name for themselves at registration and
  # never sees it again on the surface that is meant to be about them.
  @wip
  Scenario: My account page shows me back the name I asked to be known by
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    Then the account page shows the display name the human registered with

  # ---------------------------------------------------------------------------
  # What I use, against what I am allowed.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: What I use is shown against what I am allowed, for each thing the doorway spends
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    Then the account page shows storage used and the storage the human is allowed
    And the account page shows daily queries used and the daily queries the human is allowed
    And the account page shows daily bandwidth used and the daily bandwidth the human is allowed
    And every allowance the account page shows is the one the doorway answers with

  @wip
  Scenario: A brand new account reads as nothing used rather than as nothing known
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    Then the account page reads as nothing used for each thing the doorway spends
    And the doorway answers that nothing has been used on that account
    And each allowance is still shown

  # ---------------------------------------------------------------------------
  # Where I stand.
  # ---------------------------------------------------------------------------
  @wip
  Scenario: The pipeline puts me where I am and claims nothing further
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    Then the agency pipeline draws every rung from "Hosted" to standing as a steward
    And the agency pipeline marks "Hosted" as the current step
    And the agency pipeline marks no later step as completed

  # GAP: the link into the application carries nothing of the session with it, so the
  # application is reached as a stranger and asks the human to sign in again — from the one
  # page whose whole subject is that the doorway already knows who they are.
  @wip
  Scenario: The link to my full profile takes me into the application as me
    Given a hosted human "Wren" is registered on doorway "alpha"
    And the human signs in through the portal
    When the human opens their doorway account page
    And the human follows the link to their full profile in the application
    Then the application opens on that human's own profile
    And the application does not ask the human to sign in again
