# @concern:bridge-optional joins this proof to the habit register; no habit declares the concern
# yet, so the tag is new.
# @act:i: this story belongs to Act I, the household chapter of the test suite. It needs no
# household mesh: every scenario runs on Jessica's peer alone, beside a stand-in for the outside
# network.
# Held (@wip): no step definitions exist, and the behaviour waits on two pieces that are not built.
# First, the place in an app's manifest where the app declares that its posts may be sent to an
# outside network: the manifest has no such field yet, and none will be added until some running
# code reads it. Second, a bridge that actually sends an app's posts to an outside social network:
# no such bridge exists. Until both exist, nothing here can run. For detail, the placement contract
# spec names them (genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md,
# the `outward` need in §6 and §15, its landing in §17 slice 9, the outward bridge in §5.6).
@e2e @federation @concern:bridge-optional @act:i @wip
Feature: A household can refuse to send its app's notices to an outside network, and the app loses nothing by it

  The Dowell household is a small group of people, Matthew, Jessica and James, who run their own
  machines together. Each of them keeps a peer: an always-on machine that holds their data and
  runs their apps. The household uses a notice board app for its garden club: a member posts a
  notice, such as a workday or a seed swap, and the members read it. The garden club's members
  are Matthew and Jessica; James is in the household but not in the garden club. A notice's reach
  is who may see it. The board gives every garden club notice the same reach: the garden club's
  members, and no one else. The scenarios check that reach by asking whether Matthew can see a
  notice and whether James cannot; they rely on the board's own membership check being correct
  and do not test it.

  An outside network is a social network that does not run on this protocol, which people
  elsewhere use. A bridge is a piece of software that translates between this protocol and an
  outside network. An outward projection is a bridge sending copies of an app's posts out to an
  outside network so that people there can read them.

  An app may declare in its manifest, the description it is built and installed with, that its
  posts may be projected outward, and to which outside networks. The declaration is fixed for that
  version of the app; it is not switched on or off while the app runs. Declaring it sends nothing;
  it only makes an offer possible. A bridge then makes the offer to the household as a governed
  offer: a written proposal that says exactly what would be sent, to which network, and what the
  bridge would be able to do. "Governed offer" is a working name for a mechanism that is not built
  or specified yet; no other story tests it, and this one tests only the outcomes listed below.
  In this story a household member sees the offer as a proposal in the household's list of open
  proposals: they can read it and choose to approve it or decline it, and once it is approved it
  offers them a choice to withdraw it. Declining is choosing decline on that proposal; withdrawing
  is choosing withdraw on the approved one; leaving it unanswered is making neither choice.
  Nothing is sent until a household member who did not author the offer approves it, and any
  household member can withdraw an approved offer at any time, not only the member who approved
  it. Here the bridge authored the offer, so any household member's approval meets that rule; it
  would become a real test only if a household member wrote an offer, which no scenario here
  does. How an offer is written, reviewed and approved is not tested here. This story starts from an offer that already exists, unapproved,
  and already appears in the household's list of open proposals, and tests only what follows when
  a member declines it, when no member answers it, or when one member approves it and another
  later withdraws it. Whether one member's decline closes the offer for the whole household is not
  decided here; where Jessica declines, no other member approves it afterwards, so the scenario
  does not depend on that rule.

  The promise is that refusing is free. A household that declines the offer, or never answers
  it, keeps an app that does everything it did before the offer was made. No action is taken
  away, no notice's reach changes, and the board does not keep prompting about that offer: once
  Jessica has declined it or no member has answered it, Jessica sees no prompt about that same
  offer while she posts, lists and searches notices in the rest of the scenario. A new offer, from
  this bridge or another, would be a new question and may be asked; no scenario here tests that.
  Nothing is sent to the outside network.

  The same holds after one member approves the offer and another later withdraws it: here Matthew
  approves and Jessica withdraws. After the withdrawal no action is taken away, no notice's reach
  changes, and nothing more reaches the stand-in for the outside network described below. When
  she withdraws, the board shows Jessica one message: the withdrawal stops future sending and does
  not recall what was already sent. That message is the only thing the board shows her about the offer
  afterwards; it does not ask her to approve the offer again while she posts and lists notices.

  Each scenario uses a stand-in for the outside network: a test server that records every message
  it receives. "Nothing was sent" is read from that record, so it covers what reached the
  stand-in, not every connection Jessica's peer might make. The steps say so in their own words:
  each one names the stand-in as the thing observed, so the step text and this paragraph make the
  same claim, and a step must check the stand-in's full record, no less. Proving that Jessica's
  peer sent nothing to the outside network by any route would need a test that watches all of her
  peer's network traffic, which this story does not do.

  An action, in this story, is something the notice board lets Jessica do with notices: posting
  one, reading one, listing them, searching them, and anything else the board offers her. Before
  any offer exists, each scenario records every action the board offers Jessica at that moment,
  read from the board itself rather than from this paragraph; posting, reading, listing and
  searching are examples, not the whole list. "Every action it offered before" means every action
  in that record, and a step must find each one still offered, no fewer. Reach is not an action:
  who can and cannot see a notice is a property of the notice, so "every action it offered
  before" says nothing about reach. Each scenario checks reach in its own step, by asking whether
  Matthew can see the notices and whether James cannot.

  Who would speak for whom on the outside network once an offer is approved is not decided here,
  and no scenario depends on it.

  Background:
    Given Jessica's peer runs the notice board app for the Dowell household
    And the garden club's members are Matthew and Jessica, and James is not a member
    And the notice board app's manifest declares that its notices may be projected outward to the outside network
    And a stand-in for the outside network records every message it receives
    And before any outward offer exists, every action the notice board's interface offers Jessica is recorded, read from the board itself
    And a bridge, not a household member, has authored an offer to the Dowell household of an outward projection of its notices to the outside network
    And the offer names what would be sent, the outside network it would go to, and what the bridge would be able to do
    And the offer appears in the household's list of open proposals
    And no household member has approved the offer

  Scenario Outline: A household that refuses the outward offer keeps a notice board that does everything it did
    When <response>
    And Jessica posts the notice "Seed swap — Saturday" to the garden club
    Then Jessica's notice board lists "Seed swap — Saturday"
    And Jessica finds "Seed swap — Saturday" by searching the notice board
    And the notice board offers Jessica every action it offered before the outward offer was made
    And Matthew can see "Seed swap — Saturday" and James cannot
    And the stand-in for the outside network has recorded no message
    And while Jessica posted, listed and searched, the notice board showed her no prompt about this outward offer

    Examples:
      | response                                                                                     |
      | Jessica declines the outward offer for the Dowell household, and no other member approves it |
      | no member of the Dowell household answers the outward offer                                  |

  Scenario: Withdrawing an approved outward projection stops what goes out, and the board loses nothing
    Given Matthew has approved the outward offer for the Dowell household
    And Jessica's notice "Workday — Sunday" has reached the stand-in for the outside network
    When Jessica withdraws the outward projection for the Dowell household
    And Jessica posts the notice "Workday moved — Monday" to the garden club
    Then the stand-in for the outside network records no message after the withdrawal
    And Jessica's notice board lists both "Workday — Sunday" and "Workday moved — Monday"
    And the notice board offers Jessica every action it offered before the outward offer was made
    And Matthew can see both "Workday — Sunday" and "Workday moved — Monday" and James can see neither
    And the notice board tells Jessica that the withdrawal stops future sending and does not recall what was already sent
    And apart from that one message, the notice board showed Jessica no prompt about this outward offer while she posted and listed
