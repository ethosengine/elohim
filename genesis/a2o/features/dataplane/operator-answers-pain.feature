# WHY THIS FILE EXISTS, AND WHICH ACT IT IS. Measured 2026-10-09: two household-class peers had
# their conductor storage filled to zero free and sat crash-looping, a third had a dead conductor
# slot, and the answer to all three was a person doing, by hand, what no part of the protocol did:
# sensing the pain, deciding who could help, asking, judging, informing the parties. This file
# specifies that loop as something the household's own tending agent does, so the next peer that
# wedges is answered by the protocol and not by whoever happens to be awake. It is BORN RED on
# purpose: every step below is undefined today, because none of the mechanism exists yet, and the
# red is the habit register's honest statement of that. It is ACT I: every scenario runs on the
# household mesh this test run OWNS and may write to. The scenarios that stop a peer's conductor
# are destructive and run only when the operator sets A2O_ALLOW_DESTRUCTIVE=1; without it their
# mutating steps answer "skipped", and the non-destructive scenarios still run. The tag
# @requires:owned-substrate marks a scenario that writes to or stops peers, and it passes only on a
# mesh this run owns, never on the shared fleet; A2O_ALLOW_DESTRUCTIVE=1 is the operator's second,
# separate consent to actually stopping a peer. A scenario needs both. @requires:multi-node means
# the scenarios need the household's several peers running; @act:i is explained above.
@e2e @dataplane @concern:pain-is-answered @requires:multi-node @act:i
Feature: Pain is answered — a household peer that is about to fail, or has failed, is relieved by its neighbours under a plan it already agreed to

  A household is a small set of named peers, each a device in someone's home, each promising to
  hold a share of the household's bytes. Sometimes one of them is in trouble: its disk is nearly
  full, or the program that talks to the shared network on its behalf has stopped. Today, nothing
  in the protocol notices in time, and nothing decides what the neighbours should do. A person
  does. This feature specifies the day the protocol does it instead, and does it without taking
  anything from the peer in trouble that it did not already agree to give.

  Vocabulary, because every assertion below rests on it.

  The ELOHIM-OPERATOR is the household's tending agent: the one that senses trouble, decides what
  the household may do about it, and tells the people affected. It is a SEAT, not a person and not
  a peer: it acts only inside a written commitment that names what it may do alone and what it
  must hand upward, and it is never the human it serves. In this file "the seat" means the
  elohim-operator acting under that commitment.

  A BAND is the enrolled share of one device's capacity that has been given to one purpose. There
  are five: OWN (the device owner's private use), DWELLING (the household's shared use),
  COLLECTIVE (a wider community the owner belongs to), COMMONS (the open shared pool) and RUNTIME
  (the room the device's own network program needs in order to keep running at all). Each band is
  declared separately and can be in trouble separately; a peer's runtime band can be nearly full
  while its commons band is nearly empty.

  A PLEDGE COMMITMENT is the byte promise a peer makes at enrollment: "I will hold up to this many
  bytes in this band." It is a signed, shared record, and it is the band's declared LIMIT. Pain
  is recorded as a STATE LINK attached to the pledge commitment, because pain is a condition of
  the promise it threatens and not a separate event. A state link is a small signed note, readable
  by every peer, saying what condition the promise is in.

  There are two states, and they are two different requests, not two volumes of one alarm.
  APPROACH means "keep an eye on me": the peer writes it on its own pledge when a band reaches a
  declared percentage of its limit. Peers respond by raising their care, and NOTHING MOVES. A
  peer writes its Approach while it still can, because a peer whose storage is full cannot write
  anything at all; the Approach is the last act it is certain to be able to author. BREACH means
  "something broke": the limit has been crossed, or the peer has stopped. Nobody decides anything
  new at a Breach. Everyone executes the plan that was agreed at the Approach.

  The RELIEF PROPOSAL is that plan. It is minted at the Approach, names the neighbours who would
  take the peer's held bytes, and is voted on by every party to it, INCLUDING the peer in
  trouble, so that by the time of the Breach every consequence has already been consented to.

  A CUSTODY-BLOB COMMITMENT is a neighbour's own promise to hold specific bytes, made and signed
  by that neighbour. A wedged peer is never made to promise anything, and nobody promises on its
  behalf. A custody intent is a custody-blob commitment authored but INACTIVE: it records "I am
  prepared to hold these bytes" and obliges nothing until it is activated. SALVAGE is a neighbour
  taking up bytes from a peer that can no longer be relied on to serve them, by activating its own
  custody intent.

  HALF-ALIVE is a peer whose storage still answers requests while its conductor, the program that
  verifies records against the shared network's rules, cannot. Such a peer can still hand out a
  version it remembers. It cannot prove that version is current. UNVERIFIABLE is the honest answer
  a half-alive peer gives about a page's version: it says it cannot verify, instead of handing
  out a remembered one. A peer that receives "unverifiable" adopts nothing from the answerer.

  A GATE-DECISION is the seat's recorded judgment about one proposed move, and it is exactly one
  of three: ALLOW (the seat may do this alone), BLOCK (the seat refuses it), or PENDING (nobody
  with authority has answered yet). Pending is a witnessed, counted outcome, never a silence.
  A REFER is the seat declining to decide alone and naming the layer that may decide, together
  with the evidence it holds. The layers, from nearest to widest, are the intimate quorum (the
  few people closest to the peer's owner), the community consensus, and the governance act.
  A Refer is never treated as a refusal; a question nobody has answered is not a "no".

  Two paths follow from all this. The FAST PATH is Approach then Breach, with the plan already
  agreed: the seat allows the pre-agreed moves and cites the proposal that authorised them. The
  SLOW PATH is a Breach with no Approach before it, such as a conductor that was simply killed:
  there is no agreed plan, so the seat refers, with the evidence, to the nearest layer that may
  decide. A peer that declares Approach early buys the fast path for itself; that is the incentive.

  One more pair of terms. A page's ROOT AUTHOR is the identity that created its first record.
  A HEAD is the specific version of a page that a peer serves. A page's EARNED head is the one
  declared by its root author (or a device that author delegated); a STAGING head is a candidate
  declared by anyone else with a grant, and it sits visibly BENEATH the earned head and never
  replaces it. A STEWARD MEMBER of the household collective may be given a GRANT that lets them
  declare a staging head for a page whose root author is absent. The owner's return always wins:
  when the root author declares, their head is the earned head on every peer, and the staging
  work remains visible beneath it.

  The peers: the household has "matthew", "jessica" and "james". Jessica is the peer in trouble
  throughout. Matthew and james are her neighbours. The Approach percentage is the declared
  percentage of a band's limit at which a peer writes Approach; this file uses 85 percent. Every
  scenario uses the runtime band; the other four bands follow the same states and are not
  separately specified here.
  A TICK is one pass of a peer's own periodic check on its bands; "within one tick" means within one tick-length of time after the event, measured on the clock of the whole run, however many actors take part. The MESH is the running set of the household's peers and their connections. A peer that has stopped cannot write its own Breach, so the seat records the Breach for it as the seat's own judgment, never as the peer's act. A state link's value is the
  lowercase word "approaching" for an Approach and "breached" for a Breach.

  Five more terms the steps lean on. A DEVICE STEWARD is the person who owns or tends a device
  (distinct from a steward member of a collective, above); "jessica's steward" is the person who
  tends jessica. A SWEEP is one peer's periodic check, by asking another peer, of which page
  versions that peer holds and whether they are current; a sweep adopts a version only when the
  answer can be verified. A CUSTODY ROW is one peer's published record that it holds particular
  bytes; "jessica's own custody rows" are the rows saying JESSICA holds them (the neighbours'
  rows are theirs), and REVOKING a row means the holder stops being counted as one. A HOLDS GAUGE
  is the count of bytes a peer publishes as currently held, readable by any peer. A REFER'S
  WINDOW is the declared time the named layer has to answer before the Refer climbs to the next
  layer; its length is declared in the seat's commitment. Throughout, "the seat" acts only through
  the neighbours' own signed records: when a step says the seat records something, it is the
  seat's judgment that is recorded, and when a step says a neighbour holds or activates something,
  that neighbour authored it.

  Background:
    Given the household mesh has peers "matthew", "jessica" and "james"
    And each peer has an enrolled runtime band with a declared pledge commitment
    And the seat holds a commitment naming which moves it may allow alone and which it must refer
    And jessica holds bytes that no other peer holds at all

  # SCENARIO 1 — THE FAST PATH'S FIRST HALF. Approach is a request for care, and the proof that
  # it is only that is what did NOT happen: not one byte left jessica. Three things must exist by
  # the end of the tick, each one a piece of the plan a later Breach will execute: a note every
  # peer can read, intents waiting at the neighbours, and a proposal jessica herself has signed.
  Scenario: Approach raises care and moves nothing
    When jessica's runtime band reaches 85 percent of its limit
    Then within one tick of that moment jessica writes an "approaching" state link on her own pledge commitment
    And that state link is visible on matthew
    And within one tick of that moment, each neighbour that has seen the "approaching" state link authors, on its own initiative, a custody intent for jessica's held bytes, and at least two do
    And every one of those custody intents is inactive
    And no custody intent was authored by jessica or on her behalf
    And zero bytes have moved off jessica
    And within one tick of that moment the seat mints a relief proposal naming those neighbours
    And jessica's own vote is recorded on the relief proposal
    And matthew's and james's own votes are recorded on the relief proposal

  # SCENARIO 2 — APPROACH REFUSES NEW WEIGHT, BY NAME. A peer near its limit must stop taking on
  # more, and must say why in a way that can be counted: an unnamed refusal looks like a broken
  # peer, and an uncounted one cannot be seen to be happening. What it already holds it keeps
  # serving; refusing new work is not abandoning old work.
  Scenario: Approach declines new carrying by name and keeps serving what it holds
    Given jessica's pledge commitment carries an "approaching" state link
    And jessica serves a page she already holds
    When matthew asks jessica to take on custody of new bytes
    Then jessica declines the request with the reason "capacity"
    And the decline is counted under the reason "capacity"
    And jessica still serves the page she already held

  # SCENARIO 3 — THE FAST PATH'S SECOND HALF. DESTRUCTIVE: it stops jessica's conductor, so it
  # runs only with A2O_ALLOW_DESTRUCTIVE=1. Its point is what is ABSENT: no new vote, no new
  # question to anyone. The neighbours do the one thing they already agreed to do.
  @requires:owned-substrate
  Scenario: Breach executes the plan that was agreed at Approach, asking nobody again
    Given jessica's pledge commitment carries an "approaching" state link
    And a relief proposal naming matthew and james carries the votes of jessica, matthew and james
    And matthew and james each hold an inactive custody intent for jessica's held bytes
    When jessica's conductor is stopped
    Then the seat records a "breached" state link on jessica's pledge commitment as its own judgment
    And after that record matthew and james each activate their custody intent
    And jessica's held bytes arrive on matthew and on james, fetched from the shared network and verified by those neighbours, not taken from jessica's own storage
    And the seat records a gate-decision of "allow" that cites the relief proposal minted at Approach
    And no new consent is requested from anyone
    And the seat informs jessica's device steward of what the neighbours did

  # SCENARIO 4 — THE HONEST ANSWER. DESTRUCTIVE (stops a conductor; A2O_ALLOW_DESTRUCTIVE=1).
  # A half-alive peer is more dangerous than a dead one, because a dead one is plainly dead. This
  # scenario asks the question a reconcile sweep asks, of a peer that can still answer.
  @requires:owned-substrate
  Scenario: A half-alive peer says it cannot verify, and the asker adopts nothing from it
    Given jessica's storage still answers requests
    And jessica holds a page version that matthew does not hold
    When jessica's conductor is stopped
    And matthew runs a sweep of the page versions jessica holds
    Then matthew receives "unverifiable" for the versions jessica holds
    And matthew adopts no page version from jessica

  # SCENARIO 5 — THE SLOW PATH. DESTRUCTIVE (A2O_ALLOW_DESTRUCTIVE=1). The peer never said it was
  # in trouble, so there is no plan; the seat must not invent one. What it MAY do alone is
  # recorded in its commitment, and moving a stranger's bytes is not on that list.
  @requires:owned-substrate
  Scenario: A peer that never declared Approach gets a Refer, not a guess
    Given jessica's pledge commitment carries no "approaching" state link
    When jessica's conductor is stopped
    Then the seat records a "breached" state link on jessica's pledge commitment as its own judgment
    And the seat records a Refer naming the layer "intimate-quorum"
    And that Refer carries the evidence the seat holds: that jessica's conductor has stopped, that she declared no Approach, and which bytes only she holds
    And no custody activates on any neighbour until that layer answers

  # SCENARIO 6 — SILENCE IS WITNESSED. DESTRUCTIVE (A2O_ALLOW_DESTRUCTIVE=1). The failure this
  # prevents is a question that nobody answers and nobody can see nobody answering. The Refer
  # climbs one layer at a time when its window passes unanswered; at the widest layer it cannot
  # climb further, and it becomes a counted "pending", which is visible, not a "block".
  @requires:owned-substrate
  Scenario: A Refer nobody answers becomes a counted pending, never a block
    Given the seat has recorded a Refer about jessica at the widest layer
    When the Refer's window passes with no answer from that layer
    Then the seat records a gate-decision of "pending"
    And the pending outcome is counted
    And the Refer is not turned into a gate-decision of "block"

  # SCENARIO 7 — THE OWNER'S RETURN WINS. DESTRUCTIVE (A2O_ALLOW_DESTRUCTIVE=1: jessica is
  # stopped, then brought back). Matthew carries jessica's page forward only because the household
  # collective granted him that, and what he declares is a candidate, not a replacement. The
  # section that matters is the last two lines: nothing he did is thrown away, and nothing he did
  # outranks her.
  @requires:owned-substrate
  Scenario: Carried work stays visible beneath the owner's earned head
    Given jessica is the root author of a page
    And matthew is a steward member of the household collective under a grant for that page
    And jessica's conductor is stopped
    When matthew declares a successor head for that page
    Then that head is visible as a staging candidate on every peer
    And the head every peer serves as earned is still jessica's
    When jessica's conductor is brought back and she declares a head for that page
    Then the earned head on every peer is the one jessica declared
    And matthew's carried head remains visible beneath it on every peer

  # SCENARIO 8 — RELIEF IS RELEASED, BUT NOT TOO EARLY. DESTRUCTIVE (A2O_ALLOW_DESTRUCTIVE=1).
  # The neighbours took jessica's bytes so that she could come back. When she does, her own
  # custody rows are let go, but never before at least two OTHER peers hold the bytes, because
  # letting go while she is the only copy would trade one failure for another.
  @requires:owned-substrate
  Scenario: Return releases relief only once the bytes are safely held elsewhere
    Given matthew and james hold jessica's bytes under activated custody commitments
    When jessica's conductor is brought back
    Then jessica's own custody rows for those bytes are revoked only once at least two other peers hold them
    And jessica's holds gauge shows the released bytes let go
