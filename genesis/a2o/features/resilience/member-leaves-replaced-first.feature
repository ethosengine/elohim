# HELD until slice 5 of the placement-contract design ("the floor controller", section 17 of
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md).
# The rule this story proves is a missing node of that design (section 15, "Lapsed lease, re-placed
# first"). Every step is undefined, so the feature is @wip.
# @concern:blob-durability joins this proof to the habit of that name
# (elohim/elohim-storage/.epr-meta/blob-durability.habit.md): what the household keeps survives the
# loss of any one machine.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three peers must be running.
# @requires:owned-substrate: the scenarios take a peer off the network and end a membership, so they
# run only on a mesh this run owns outright.
@e2e @resilience @act:i @wip @concern:blob-durability @requires:household-nodes @requires:owned-substrate
Feature: When James leaves the household, another peer takes up what his peer kept for the household before his peer lets it go

  Matthew, Jessica and James Dowell share a household. Each of them keeps a peer: an always-on
  machine at home that holds their data and does work for them. Each peer is named after its
  person: matthew, jessica and james.

  The household declares how many peers should keep copies of what it shares, so that losing one
  machine loses nothing. Such a declaration is called a need. A need names no peer and is sent to
  no peer; it only states what should be true. Here the household needs its folder "Household papers"
  (scans of its bills, insurance and tax papers) kept on two peers. A peer that keeps a copy for a
  need signs a commitment: a record, kept inside the household, saying that it holds this copy for
  this need. Signing is a promise; it does not by itself make the peer count. A holder is honoured
  for a need when two things are both true: it has signed a commitment for that need, and its own
  list of what it holds names the copy. It is honoured from the moment the later of the two becomes
  true. It stops being honoured the moment either one stops being true: its commitment ends, or its
  list no longer names the copy. A need is met while as many holders as it asks for are honoured. A step that says a peer is
  an honoured holder of a copy states both things at once: the peer has signed a commitment for it,
  and the peer's list names the copy. Whether a holder really still has what its list names is
  checked by another story (features/resilience/observable-distribution.feature).

  Each peer also has an offer: its person's signed statement of how much room the peer gives to
  keeping copies for the household. A peer keeps a copy for the household only inside its offer;
  having free disk is not enough. No person and no peer hands out this work. Every peer runs the same
  fixed rules, called the floor, which decide from the records the peer already holds which peers
  should keep each copy. Each peer runs its floor by itself, on that peer: it reads only the records
  the peer already holds, and it sends no peer a message asking or telling it what to keep. Peers
  holding the same records therefore reach the same choice without asking each other. A peer whose
  floor chooses it takes the copy and signs the commitment itself; a peer whose floor tells it to let
  a copy go does so itself.

  Leaving and being away are different. A member leaves when they sign a withdrawal of their
  membership of the household. A member is away when their peer is off the network, for an hour or
  for a month, and they are still a member. Leaving ends what the leaver's peer keeps for the
  household, in a safe order: another peer takes up each copy and is honoured first, and only then
  does the leaver's peer let its copy go, so the household never has fewer honoured copies than it
  needs. To let a copy go is to end the commitment and delete the copy that was kept under it. Being
  away ends nothing: what the household keeps for an away member stays kept. While a peer is away,
  the household may also treat it as a lost holder of what it kept and have another peer take up a
  copy; that repair is a different story (features/resilience/salvage-placement.feature, in which a
  peer that has room takes up a copy kept by too few holders). The third scenario checks nothing
  about that repair; it checks only that nothing the away peer kept, or that was kept for its person,
  was ended.

  This story describes how the household behaves once every peer runs the floor. The floor belongs
  to the placement-contract design: the plan for how the household decides which peers keep which
  copies, built in numbered stages called slices, each adding one part of the plan. Slices 1 to 4
  build how the household finds and reads the copies it keeps; slice 5 adds the floor itself, and
  from slice 5 on every peer runs it. Slice 5 is not built yet, so this story is held until then;
  the @wip tag on the feature marks that hold, and it comes off when slice 5 is built.
  Today the household's peers add a holder only after one is lost, and nothing releases a copy
  when a member leaves. It sits beside
  features/resilience/household-reciprocity.feature, which names who keeps copies for whom and says
  nothing about what happens when a member leaves.

  The second scenario is one proof, not two: it follows one household through two events in order
  on a single timeline, so it has two When steps. First James signs his withdrawal while no other
  peer has room, and the household waits; later, with nothing reset in between, Matthew gives room
  on his peer, and the hand-over the first scenario describes then happens. Its last step looks
  back over both phases together, from James's signature to the moment peer "james" lets its copy
  go.

  James is leaving for university, and his peer goes with him.

  Background:
    Given the household's peers "matthew", "jessica" and "james" are running
    And Matthew, Jessica and James are members of the household
    And the household needs its folder "Household papers" kept on two peers
    And peers "jessica" and "james" are the two honoured holders of "Household papers": each has signed a commitment to keep a copy of it, and each one's list of what it holds names its copy
    And "Household papers" is the only thing peer "james" keeps for the household
    And peer "matthew" holds no copy of "Household papers"

  Scenario: James leaves, and peer "matthew" is honoured as a holder before peer "james" lets its copy go
    Given Matthew's offer gives room on peer "matthew" for copies kept for the household
    When James signs a withdrawal of his membership of the household
    Then peer "matthew" takes a copy of "Household papers" and signs a commitment for it
    And peer "james" lets its copy of "Household papers" go only after peer "matthew" is honoured as a holder
    And at every moment after James signed, at least two holders of "Household papers" were honoured
    And once peer "james" has let its copy go, no commitment by peer "james" to keep anything for the household is still in force
    And each copy taken or let go after James signed was chosen by the floor on the peer that did it
    And no person and no peer sent any peer an instruction to take or let go of a copy after James signed

  Scenario: When no other peer has room, peer "james" keeps its copy until one does, and everyone is told why
    Given Matthew's offer gives no room on peer "matthew" for copies kept for the household
    When James signs a withdrawal of his membership of the household
    Then peer "james" still holds its copy of "Household papers", and its commitment for it is still in force
    And James is told that peer "james" is still keeping "Household papers" for the household until another peer takes it up
    And Matthew and Jessica are told that "Household papers" is waiting for a second holder because no other peer offers room
    And peer "matthew" has taken no copy of "Household papers"
    And for as long as no other peer offers room, however long that is, peers "jessica" and "james" stay the two honoured holders of "Household papers"
    When later, with James's withdrawal still signed and nothing else changed, Matthew changes his offer to give room on peer "matthew" for copies kept for the household
    Then the floor on peer "matthew" chooses it to keep a copy of "Household papers"
    And peer "matthew" takes a copy of "Household papers" and is honoured as a holder
    And only after that does peer "james" let its copy of "Household papers" go
    And looking back over both phases, from James's signature while no other peer had room, through Matthew's change of offer, to peer "james" letting its copy go, at least two holders of "Household papers" were honoured at every moment

  Scenario: Being away is not leaving: what the household keeps for James, and what James's peer keeps for the household, are both still kept when his peer returns
    Given the household needs James's folder "Coursework" kept on two peers
    And peers "matthew" and "jessica" are the two honoured holders of "Coursework"
    When peer "james" is off the network for 30 days
    And peer "james" comes back onto the network
    Then peers "matthew" and "jessica" still hold their copies of "Coursework"
    And their commitments to keep "Coursework" for James are still in force
    And no commitment by peer "matthew" or peer "jessica" to keep anything for James was ended while peer "james" was away
    And peer "james" is still an honoured holder of "Household papers", and its commitment for it was not ended while it was away
    And James is still a member of the household
