# Held until slice 5 of the placement-contract design ("the floor controller", §17 of
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md).
# No peer runs this floor yet, so the feature is @wip and none of its steps has a definition.
# @concern:placement-floor names this proof for the habit register; no habit checks it yet.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three peers must be running.
# @requires:owned-substrate: the scenarios write holding records and hold back one peer's offer from
# another, so they run only on a mesh this run owns outright.
@e2e @act:i @compute @wip @concern:placement-floor @requires:household-nodes @requires:owned-substrate
Feature: With no elohim running anywhere, the household still decides who keeps what, and every peer ends up agreeing

  Matthew, Jessica and James share a household. Each of them keeps a peer: an always-on machine
  at home that holds their data and does work for them. Each peer is named after its person:
  matthew, jessica and james. The household asks its peers to keep copies of what it shares, so
  that losing one machine loses nothing. Deciding which peers keep which copies is called placement.

  A need is a request to have something kept. Here Jessica asks for her folder "Garden plans" to
  be kept on two of the household's peers. An offer is a peer's signed statement of what it will
  do for others, such as how much room it gives to copies. A peer has no offer until its person
  decides to make one; nothing makes an offer by default, and a peer without one keeps no copies
  for others. In each scenario below, each person makes an offer for their own peer. A peer that
  keeps a copy signs a commitment: a record that says it holds this, for this need. A holder is
  honoured when it lists the copy among what it holds, in the list each peer keeps of its own
  holdings. The two are separate moments. A peer signs its commitment as soon as it decides to keep
  a copy. If the copy is not on it yet, the peer then fetches it from a peer that already has it;
  "Garden plans" starts on Jessica's own peer, jessica, so the first copies come from there. The
  peer becomes honoured only once the copy is on it and shows up in its holdings list. If a fetch
  fails, the peer stays committed but not honoured. A need is met when as many holders as it asked
  for are honoured. Each peer's holdings list reaches the other two peers the same way its offers
  do, a little after it changes, and a peer counts another as honoured once the list it has received
  from that peer shows the copy. Nothing in this story checks that a holder really still has what
  it lists; that check belongs to another story.

  Jessica asked for her folder to be kept within the household and shown to nobody outside it. A
  commitment is kept as private as the need it answers, so the commitments for "Garden plans" stay
  among the household's three peers. They never appear on the shared network. The shared network
  is the public record space that every household in the protocol takes part in: what is written
  there is copied to, kept by and checked by peers in many households, and anyone on the network
  can read it. It is separate from the household's own peers speaking directly to one another,
  which reaches only matthew, jessica and james.

  Nobody is in charge of placement. No server hands out work, and no peer tells another what to
  keep. Every peer runs the same fixed rules on its own, called the floor. From the needs, offers
  and commitments it has already received, a peer's floor lists the peers allowed to meet each
  need, called the eligible peers, and puts them in order by a fixed calculation. For each eligible
  peer it mixes the need's identifier with that peer's public key into a score, a hash that looks
  random but comes out the same on every machine, and sorts the peers lowest score first. Because
  the score depends on the need, different needs put different peers first, and no peer is ahead
  because of its name or when it joined. Ties break the same way everywhere, and the calculation
  uses no clock, no chance and nothing a peer keeps to itself. So, given the same records, it
  gives the same order on every peer. A peer that finds itself among the first two in that order
  for "Garden plans" takes a copy on itself and signs the commitment. A floor never fetches a
  record it lacks before deciding: a peer whose offer it has not received is not eligible yet, and
  the arrival of that offer makes the floor run again. The floor's rules can change from one
  software version to the next, and peers on different versions could compute different orders, so
  this story starts with all three peers on the same version.

  A need's identifier is a fingerprint computed from the need itself, and each peer's public key
  is known to the test. So the test can work out the order before the need is made, by the same
  calculation every floor uses. The test does not choose the order; it reads it. Where a scenario
  names an order, it names the order the test worked out, and with other keys the same parts in
  the story would fall to other peers. The test's working is itself checked: the first scenario
  fails if the two holders the floors choose are not the first two in that order, so a wrong
  working cannot pass unnoticed.

  Peers do not always hold the same records at the same moment, because an offer reaches the
  other peers a little after it is made: a peer makes its offer by signing it at home, and each of
  the others receives it only later. A peer whose offer has been made may therefore still be
  unknown to a peer that has not yet received it. The test can stretch that window on purpose: it
  can delay one peer's record on its way to one other peer, and to that peer alone, until the
  test lets it through. Records between every other pair of peers flow as usual. This models
  the ordinary delay at a length the test controls, so a scenario can fix which peer has not
  yet received which offer. So two peers can make different first guesses, and for a while too
  many peers, or too few, may hold a copy. The household does not count on every peer guessing
  alike. Each floor keeps reading how many holders are honoured and acts only on the
  difference: a missing holder is filled, and when there are too many, the extra holder ranked
  lowest lets its copy go, once the holdings lists it has received show the holders that should
  keep it honoured. The test records
  how long that settling takes and how many extra copies were made on the way.

  Each person may also have an elohim: an AI helper that works for its own person, notices when
  placement is not serving them, and drafts changes for the people concerned to agree to. An elohim
  is never needed to place anything. In this story no elohim runs on any peer.

  Each time in a scenario is a limit the test enforces: a step that names one fails if what it
  describes has not happened by then. The limits are this story's targets, chosen before any peer
  runs this floor, not readings measured today. They are set to leave wide room for three peers on
  one home network, so that a missed limit points at the floor rather than at a slow link.

  No peer runs this floor yet. This story is held until the floor is built, in slice 5 of the
  placement-contract design, and until then every scenario in it is marked work in progress and
  no habit checks it.

  Background:
    Given the household peers matthew, jessica and james are running
    And no elohim is running on any of the three peers
    And all three peers run the same version of the floor rules
    And the test has worked out, from the identifier of the need "Garden plans" and the three peers' public keys, that with every peer's offer in hand the order is matthew, then jessica, then james

  Scenario: A new need is placed, and every peer names the same two holders
    Given each person has made an offer of room on their own peer for the household's copies, and every peer has received all three offers
    When Jessica asks for her folder "Garden plans" to be kept on two of the household's peers
    Then within 60 seconds of Jessica's request, two peers have each signed a commitment for "Garden plans"
    And within 60 seconds of Jessica's request, both of those peers are honoured: each has the copy on it, fetched from jessica where it was not already there, and lists it in its own holdings list
    And so the need "Garden plans" is met: as many holders as it asked for, two, are honoured
    And those two are matthew and jessica, the first two in the order the test worked out, which is the order every peer's floor computes for the need
    And each of the three peers, asked who holds "Garden plans", names the same two holders
    And no person approved the placement: no peer holds an approval record for it
    And no peer told another to take a copy: no peer received a message, addressed to it alone, asking it to keep "Garden plans"
    And no record on the shared network, the public record space beyond the household, names "Garden plans" or any commitment for it

  Scenario: Peers that first saw different offers still end up agreeing
    Given the test is delaying any offer matthew makes on its way to james, and to james alone, while matthew's other records, its holdings list among them, reach james as usual
    And each person has made an offer of room on their own peer for the household's copies
    And so matthew and jessica have received all three offers, while james has received jessica's and its own but not matthew's, and the order james can compute is jessica, then james
    When Jessica asks for her folder "Garden plans" to be kept on two of the household's peers
    Then james takes a copy of "Garden plans", because it is among the first two in the order it can compute
    And matthew and jessica each take a copy, because they are the first two in the order they compute
    And james receives matthew's and jessica's holdings lists, each showing the copy, and still keeps its own copy, because without matthew's offer its order is still jessica, then james
    When the test lets matthew's offer through, and it reaches james
    Then within 60 seconds of receiving matthew's offer, james lets its copy go, because its order is now matthew, jessica, james and the holdings lists it has received show matthew and jessica both honoured
    And each of the three peers, asked who holds "Garden plans", names matthew and jessica
    And the test records how long the household took to settle and that it made one extra copy on the way

  Scenario: Once the household has settled, no peer does placement work until something changes
    Given each person has made an offer of room on their own peer for the household's copies, and every peer has received all three offers
    And "Garden plans" is held by matthew and jessica, and each of the three peers names those two holders
    When no need, offer, commitment or peer changes for 5 minutes
    Then no peer's floor runs during those 5 minutes
    When James lowers the room his peer offers
    Then each peer's floor runs again within 60 seconds of receiving James's new offer
