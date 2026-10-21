# Design: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (§3 the spaces, §6 the needs an app declares, §7.2 the placement floor and its "Alone" rule, §7.4
# reading a placement back, §16 the contract's first story; held on §17 slices 5 and 9).
# @concern:app-inherits-placement is the tag the design (§17) proposes for this story's outcome; it is
# not declared yet.
@e2e @federation @local-first @act:i @wip @concern:app-inherits-placement @requires:household-nodes
Feature: One app, declared once, opens for one person offline and for a household, with nothing changed

  A household here is a small group of people and the machines they run together. Each person
  keeps a peer: a program on their own machine that holds their data and does work for them. In
  this story jessica's peer runs on her laptop, and matthew and james each run a peer at home. The
  three peers are one household. They know one another by key. An offer to hold copies is a
  standing setting a person turns on in their own peer, saying it may store copies for others in
  this household; it stays on until they turn it off. A peer holds copies for others only when its
  person has made the offer to hold copies; being able to is not the same as having agreed to.
  matthew and james have each made the offer to hold copies for the household; jessica has not.

  An app is a program the household uses; here it is a recipe book, shared with the whole
  household, which means each of the three peers already holds the same release of it. Its
  author describes it once, in a declaration that ships with every release of the app. The
  declaration says what the app needs and never where those needs are met. This one says
  three things: which files the app needs in order to start; that it must be able to open even when
  nothing else is reachable; and that every recipe written in it must survive the loss of any one
  device, which means at least two devices must hold each recipe. A recipe held on one device only has not met
  that promise yet: it is waiting for a second holder, which is a normal state, not a failure. The
  declaration names no laptop, no peer, no hub and no doorway (a doorway is a public web entry
  point). Files are named by a fingerprint, a short identifier computed from their exact
  bytes, so "the same release" is the same files on every machine. The starting files are part of
  the release itself: every peer that holds the release holds them, so that need is met on every
  peer before anyone writes anything, and nothing here waits on it. Only recipes, which people
  write after the release arrives, need placing.

  Who is present decides where each need is met. jessica alone on her laptop with no network is
  one situation; the three peers reaching one another is another. The app does not notice which
  one it is in, and nobody tells it. Each peer applies the same placement rules to what it can
  see. Peers do not always see the same things at the same moment, so they settle who holds what
  by reading which copies are actually held and acting on the difference. Nobody assigns work to
  anyone else. A peer learns which copies the others hold by reading the holding records they
  share, so "the peers have had time to see which copies one another hold" means each peer has read
  the latest holding records of every peer it can reach. Two peers may each take a copy before either sees the other's; a recipe then has
  three holders, which still meets "at least two" and is not a fault.

  A holder of a recipe is a device keeping a copy of it, whatever its reason for keeping it. Every
  holding is recorded, so anything the app promised can be read back. Each holding is in one of two
  states: confirmed, when a check of the copy has found it intact, or claimed, when the holder says
  it has the copy and no check has confirmed it yet. Those two states belong to one holder's copy.
  A promise in the declaration, such as "at least two devices", is read from them: it is shown as
  kept only when confirmed holdings meet it, and as unchecked when it rests on claimed holdings or
  nothing has checked it at all. Beside each recipe the app also shows, without being asked, a
  short holding line: either "held on this device only, waiting for a second device" or "held by
  enough devices". The holding line counts holders and does not say whether their copies were
  checked. Asking where a recipe is held and why opens the fuller answer behind that line, which
  does.

  jessica has told her laptop to keep the recipe book available offline. That is a setting for her
  own use, separate from the offer to hold copies for others: it asks her laptop to keep the app's
  starting files and every recipe in the book, including recipes matthew and james write, which it
  fetches whenever it can reach them. It does not offer her laptop to the household, so her laptop
  is never chosen to hold a copy on someone else's behalf; a copy it keeps for her still counts as a
  holder.

  This story describes behaviour that is not built yet. It follows the design named "What every
  app inherits", whose build order numbers its stages. Two of them must land first: stage 5, "the
  floor controller", which is the placement rules every peer runs; and stage 9, "the manifest
  needs", which lets an app declare its starting files, that it works offline, and how much loss
  it must survive. Every scenario is held until both have landed. Nothing outside this
  description is needed to follow the story; the design is named only so those stages can be found.

  Background:
    Given a household of three peers: matthew's and james's at home, and jessica's on her laptop
    And matthew and james have each made the offer to hold copies for the household, and jessica has not
    And a recipe book app, shared with the household, whose one declaration names the files it starts from, says it must be able to open even when nothing else is reachable, and says each recipe must survive the loss of any one device, which the placement rules read as at least two devices holding it
    And that declaration names no device, peer, hub or doorway
    And each of the three peers already holds the same release of the recipe book: the same starting files, by fingerprint, and the same declaration
    And jessica has told her laptop to keep the recipe book available offline

  Scenario: Alone with no network, jessica's recipe book opens and keeps what she writes
    Given jessica's laptop has no network connection and reaches no other peer
    When jessica opens the recipe book
    Then it starts from the files on her laptop, with no request left waiting on a network
    When she writes a new recipe, "Barley soup"
    Then the recipe is saved on her laptop
    And without her asking, the holding line beside "Barley soup" reads held on this device only, waiting for a second device
    And nothing she did was refused because the household could not be reached

  Scenario: Back with the household, her recipe is held twice and matthew reads it from the release he already had, with no new install and no changed setting
    Given jessica wrote "Barley soup" while her laptop had no network
    When her laptop reconnects to the household
    And the peers have had time to see which copies one another hold
    Then at least one of matthew's and james's peers, each having made the offer to hold copies, takes a copy of "Barley soup" because it saw one holder where the declaration asks for two, with no peer assigning it the work
    And "Barley soup" is held by her laptop and by at least one of those peers
    And the holding line beside "Barley soup" now reads held by enough devices, whether or not those copies have been checked yet
    And the app on her laptop is the same release it was while offline, with the same declaration, and nobody changed a setting
    When matthew opens the recipe book on his peer
    Then it starts from that same release and shows "Barley soup"
    And matthew installed no other version and changed no setting for the recipe book to open it, because the release he already held is the one jessica used

  Scenario: Asked where a recipe is held and why, the app reads the answer back instead of assuming it
    Given "Barley soup" is held by jessica's laptop and by at least one other household peer
    When jessica asks where "Barley soup" is held and why
    Then the answer names the need it serves: surviving the loss of any one device
    And it names the household peers that hold a copy
    And for each holder it says whether its copy is confirmed by a check or only claimed by the holder
    And the promise of at least two devices is shown as kept only if confirmed holdings meet it, and otherwise as unchecked, not as kept

  Scenario: Out of reach of the household again, jessica loses nothing she had
    Given "Barley soup" is held by jessica's laptop and by at least one other household peer
    When jessica's laptop loses its network connection again
    Then the recipe book still opens from her laptop and shows "Barley soup"
    And her laptop still holds its own copy of "Barley soup", although another peer holds one too
    And a recipe she writes now has a holding line that reads held on this device only, waiting for a second device, until the household is reachable again

  Scenario: A recipe matthew writes reaches jessica's laptop because she keeps the book offline, not because she offered to hold copies
    Given jessica's laptop is reachable by the household
    When matthew writes a new recipe, "Plum jam", on his peer
    And the peers have had time to see which copies one another hold
    Then jessica's laptop holds a copy of "Plum jam", fetched because she asked to keep the recipe book available offline
    And jessica's laptop was not chosen to hold "Plum jam" for anyone else, because she has not made the offer to hold copies
    When jessica's laptop loses its network connection
    Then the recipe book opens from her laptop and shows "Plum jam"
