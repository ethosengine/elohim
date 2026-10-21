# Design provenance and held status are carried in the feature description below, where a
# rendered report keeps them. Design: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
@e2e @auth @recovery @act:i @concern:root-held-by-network @wip
Feature: Nancy starts alone on one laptop, is told plainly that her identity is provisional, and is secured the first time she reaches the network
  As Nancy, who started using the app on her own laptop with no network nearby
  I want the app to tell me honestly what I would lose if the laptop were lost before it ever reached anyone
  So that I can trust it when it later tells me that losing the laptop would no longer lose me

  STATUS: HELD as of 2026-10-20. Every scenario is marked @wip: no step below is wired to the
  system yet, and most of what the scenarios check is not built. The story comes from the design
  "What every app inherits: the placement contract" (2026-10-20). That design rules that a
  person's identity root is held by the network, and that a person who starts alone holds an
  identity that is provisional until it first reaches the network. It names securing an identity
  at first contact, and enrolling a new device after every old one is lost, as pieces still to
  build. Which peers form a recovery quorum, how many, and how quickly the network takes up an
  identity root are questions it leaves open for the people who run the network. The first
  scenario needs no network at all; the others need the household mesh described below, and are
  tagged so.

  "The app" in this story is not one particular product. It means any app built on this protocol,
  and what the scenarios check is a contract every such app inherits from the protocol rather than
  something one app chose to build. The checks would read the same for any of them. The
  contract fixes only one thing about where an app tells a person how safe their identity is:
  it is the first screen they see each time they open the app. This story calls that screen the
  app's HOME SCREEN, whatever else an app puts on it.

  Nancy is a neighbour of the Dowell household. She installs the app on her laptop at home, where
  there is no network connection, and starts using it. The app works fully on its own: she
  writes notes, opens them and changes them, and nothing waits on the network. Writing a note is
  only an example of using the app on her own device. It stands for anything she creates in it,
  and says nothing about what the app is for.

  The words this story turns on. A person's IDENTITY ROOT is the lasting name of their identity on
  the network. It stays the same whichever devices they use and however often a key changes. A DEVICE of
  theirs is ENROLLED under the identity root: the network holds a record that this device's key
  acts for them. Each device signs what it does with its own key, and the network accepts a
  signed action only from a key that is enrolled. The rule this story proves is that the
  identity root is held by the network, never by one device alone, so that losing every device
  does not lose the person. It is held under a RECOVERY QUORUM: a set of the network's peers
  that together can enroll a new device for the person when every old one is lost. No one of
  them can do that alone. In this story the
  only network Nancy ever reaches is the Dowell household, so her recovery quorum is drawn from
  the household's peers. This story does not say how many of them it takes, which of them are
  chosen, or how quickly the network takes up an identity root. Those are still open questions for
  the people who run the network, so the scenarios ask only that the quorum is made of more than
  one household peer and that no single peer can act for Nancy alone.

  Before the network can hold anything, it has to be reached. Until then the identity root
  exists only on the one device that created it. That identity is PROVISIONAL: losing the laptop
  now would lose it, and the app says so in plain words, not in a warning that sounds like
  something went wrong. It says so on the app's home screen, every time she opens the app, so the
  honesty is where she cannot miss it rather than buried in a settings page. FIRST CONTACT is the
  first time the device reaches any peer of the network. Here that is one of the Dowell
  household's peers. At first contact, the network takes
  up the identity root under a recovery quorum without Nancy doing anything more. From then on the
  laptop is one enrolled device among the ones she may add. The app tells her that the change has
  happened once, the first time she opens it after first contact, and from then on its home screen
  no longer calls her identity provisional. She does not have to go looking to find out she is now safe.

  Securing her identity is not sharing her notes. What the network takes up at first contact is
  her identity root and what a recovery quorum needs to stand for her. It is not what she wrote.
  Her notes stay on her laptop until she shares them.

  The household mesh is a small cluster of three always-on peers kept by the Dowell household:
  Matthew, Jessica and their son James. Nancy's laptop, and in the last scenario her new phone,
  are devices the story adds to it. The test harness removes them again after each scenario; that
  is cleanup, not something the story checks.

  If every device of hers is later lost, Nancy starts again from a new device that the network has
  never seen. From that device she sends a RECOVERY REQUEST: a message to the network's peers that
  asks her recovery quorum to enroll this new device under her identity root. Each peer of the
  quorum decides for itself whether the request is really Nancy's before it agrees. How a person
  with no enrolled device proves to those peers that she is who she says is not yet designed. So
  the last scenario states the outcome the network commits to, not a flow that can run today: it
  becomes testable only once that proof is designed.

  Scenario: Alone, everything works, and the app says plainly that her identity is provisional
    Given Nancy's laptop has never reached any peer of the network
    When Nancy creates her identity in the app on her laptop
    And Nancy writes a note titled "Seeds to order"
    Then the app keeps the note and opens it again after the laptop restarts
    And the app's home screen tells Nancy that her identity is held only on this laptop for now
    And the app's home screen tells Nancy that losing this laptop before it reaches the network would lose her identity
    And the app's home screen tells Nancy that her identity will be secured the first time the laptop reaches the network
    And nowhere in the app is her identity described as secured or recoverable
    When the laptop restarts and Nancy opens the app again, still without reaching any peer of the network
    Then the app's home screen again tells Nancy that her identity is held only on this laptop for now

  @requires:household-nodes
  Scenario: At first contact the network takes up her identity root, and the app stops calling it provisional
    Given Nancy has created her identity on her laptop with no network
    When Nancy's laptop reaches a peer of the Dowell household for the first time
    Then without Nancy doing anything more, her identity root is held under a recovery quorum drawn from the Dowell household's peers
    And every peer of that recovery quorum is one of the household's peers: Matthew's, Jessica's or James's
    And that recovery quorum is made up of more than one of those peers
    And no single peer of the network can enroll a new device for Nancy without at least one other peer of her quorum agreeing
    And her laptop is enrolled under that identity root as one of her devices
    And the first time Nancy opens the app after that, its home screen tells Nancy once that her identity has just been secured by the network
    And the app's home screen tells Nancy that losing this laptop would no longer lose her identity
    And the app's home screen no longer tells her that her identity is provisional

  @requires:household-nodes
  Scenario: Securing her identity sends none of her notes
    Given Nancy has written a note titled "Seeds to order" on her laptop with no network
    When Nancy's laptop reaches a peer of the Dowell household for the first time
    Then her identity root is held under a recovery quorum
    And no household peer holds the note "Seeds to order" or any copy of its text
    And the note is still on Nancy's laptop

  @requires:household-nodes
  Scenario: Committed, not yet runnable: after first contact, losing her only device would not lose Nancy
    This scenario states the outcome the network commits to, not a flow that runs today. How each
    peer of the quorum checks that a request from an unknown device is really Nancy's is not yet
    designed. The step that says each peer has satisfied itself names that check without defining
    it, and the scenario can run only once that check is designed.

    Given Nancy's identity root is held under a recovery quorum since her laptop's first contact
    And her laptop is the only device enrolled under it
    When Nancy's laptop is destroyed
    And Nancy starts the app on a new phone that no peer of the network has seen before
    And from the new phone she sends a recovery request to the household's peers
    And each peer of her recovery quorum has satisfied itself that the request is Nancy's, by a check not yet designed
    Then at least two peers of her recovery quorum have each separately agreed that the request is Nancy's
    And the new phone is enrolled under the same identity root her laptop was, and was not enrolled before more than one peer had agreed
    And the destroyed laptop is no longer enrolled, so anything newly signed with its key is refused
    And the app's home screen on the new phone does not call her identity provisional
