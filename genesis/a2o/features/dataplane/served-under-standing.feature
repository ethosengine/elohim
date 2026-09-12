# BORN @wip, EVERY SCENARIO. No step definition in this file exists yet: the
# serving path today folds nothing at serve time, and the doorway's warm shell
# and file cache answer from bytes alone. So a run of this file reports
# undefined steps, and undefined is not evidence. Each scenario below is the
# assertion that must be true before the matching Rust may be called done; the
# habit atom doorway/doorway-service/.epr-meta/served-under-standing.habit.md
# is the only thing that flips them.
#
# Design source: the 2026-09-12 operator ruling, recorded in the read-path
# workstream of
# genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md.
# The companion half — WHICH doorway answers a name — is
# features/federation/name-routing.feature under the same concern tag. This
# file is the other half: WHETHER it may answer at all, and what it must show
# the person while it does.
@e2e @dataplane @concern:served-under-standing @act:i @requires:owned-substrate
Feature: A doorway serves only under a standing it re-asks, and shows the person that standing
  A doorway putting a page in front of a stranger looks, from the outside, exactly
  like any other web server. It is not one. It holds no authority of its own over
  what it serves: the right to project this record, to this person, right now is a
  privilege the network affords it, derived from records anyone can read back, and
  it lasts only as long as those records say it does. When a collective decides
  something it stewards should no longer be public, that decision has to reach the
  stranger standing at a doorway the collective has never heard of — and it has to
  reach them in words they can act on, not a bare refusal. And when the doorway
  does serve, the person should be able to see what governs the place they have
  walked into, what was traded for their visit, and how to object — without knowing
  a single word of this protocol before they arrived.

  This story is written from the visitor's side of that door. It is the reason the
  serving path may not be a cache with a hostname.

  THE WORDS THIS STORY USES, fixed here so every assertion below is checkable
  without leaving the file.

  A DOORWAY is a machine that puts substrate records in front of ordinary web
  visitors — a projection point, not an owner of what it shows. "alpha" and "beta"
  below are two of them.

  AN EPR is a published resource the protocol addresses by its own identity — a
  page, a site, a course, an app bundle — carrying its own declared reach and the
  record of who stewards it. What a doorway serves is always an EPR.

  THE SHARED LEDGER is the record every peer holds a copy of and can read back for
  itself: notarised, so a reader verifies a record rather than trusting whoever
  handed it over. Contracts, reach declarations, rulings, receipts and commitments
  all live there. It is why a decision can reach a doorway nobody told.

  A CONTRACT is the promise, notarised on that ledger, that a named doorway will
  project a named EPR at a named address. It is the doorway's licence to answer for
  that EPR at all, it names an issuer, and it can lapse or be withdrawn. No
  contract, no serve — and a doorway cannot write itself one.

  REACH is the audience an EPR has earned: who may receive it. It is declared on
  the EPR itself by whoever stewards it, and it is independent of which version is
  current and of how many peers hold the bytes. Only reach decides who may read.
  Reach is a ladder of named audiences, and COMMONS is its widest rung: anyone at
  all, including a visitor who has shown nothing. Commons is the only rung this
  story names, because it is the rung a narrowing LEAVES. Where a narrowing lands
  is the collective's to say, and these scenarios deliberately never fix it — they
  assert only that the new reach admits the household's members and no longer
  admits an anonymous stranger.

  STANDING is what a requester can show for themselves — their membership,
  relationships and history — as their OWN conductor states it. A conductor is the
  runtime a person holds their own record in. A doorway never invents standing and
  never keeps its own copy to consult later; it asks, at the moment of the request,
  and the answer is the requester's to give.

  LIVENESS is the doorway's own honest account of whether it can serve right now.
  Shedding is liveness saying no, out loud.

  THE FOLD is those four terms taken together — contract, reach, standing,
  liveness — resolved at the moment of the request into one answer: serve, or
  refuse and say which term failed. The fold is the whole of the eligibility
  decision. A doorway-local list of who may read, a flag in its configuration, or
  a remembered "this one was allowed last time" are each, by themselves, the defect
  this story exists to refuse. Bytes may be held warm; the fold may not. RE-FOLDING
  is a doorway redoing that resolution from the ledger on its own reconcile, with
  nobody having to tell it to. A refusal NAMES, on its face, which of the four terms
  failed — and that naming is how every assertion below observes the fold at all. A
  doorway that refuses without naming a term has not been shown to have folded; it
  has only been shown to have said no. The named term must also be TRACEABLE: it
  points at the ledger record that carries it — the contract, the reach declaration,
  the ruling — so "decided by the fold" is checkable as "the reason it gave reads
  back to a record", and a doorway-local list, which can name no such record, fails
  that check even when it happens to reach the same answer.

  THE NEAREST LIVE HOLDER is whichever doorway holding a live contract for the EPR
  is chosen to supply the bytes — health first, then the order the contracts
  declare. How that choice is made and proved is name-routing.feature's subject; in
  this file there is one holder, so the phrase asserts only that the serve came
  through a holder that was alive, and that the chrome named which one.

  A COLLECTIVE here is a community holon that stewards an EPR and can rule on its
  reach. Its RULING is a governance act recorded on the shared ledger, which is why
  it can reach a doorway nobody told about it.

  THE CHROME is the frame the doorway renders around whatever it serves — the part
  the doorway itself contributes, as opposed to the content. It is where the
  standing becomes visible to someone who knows nothing of this protocol: the
  governance mark (what reach this is at, who ruled it, which doorway is holding
  the contract), the fair-trade receipt, and the way to be heard. It speaks as a
  friend would; the exact protocol form of anything it says is one request away,
  never the first thing said.

  A RECEIPT is the chrome's plain-language account of the exchange this one serve
  was: what was given, by whom, and who was credited for it. It is a reading of an
  economic record on the ledger, not a number the doorway made up.

  AN OWED RESPONSE is the difference between feedback and a suggestion box. When a
  visitor challenges a standing, the challenge is witnessed as a commitment with a
  named party who owes an answer and a due date fixed before the visitor sent it.

  THE HOUSEHOLD is the group running this mesh on hardware they hold: two doorways
  they operate, "alpha" and "beta", and three peers behind them — Matthew's,
  Jessica's and James's. Where a scenario says the household or a collective does
  something, it is those people acting through their own machines. Matthew, Jessica
  and James also appear as the people in the story; a peer named for someone and the
  person of that name are deliberately the same household, which is what makes the
  standing readable from a conductor they actually run. Owning the hardware is what
  the feature's `@requires:owned-substrate` tag asks for: a run that does not own
  its substrate cannot rule, narrow or freeze anything here, so these scenarios are
  HELD on such a run rather than failed — a scenario that cannot be exercised has
  proved nothing either way.

  FORWARDING, NAMED ONCE. Beta holds no contract for the EPR these scenarios use,
  so when beta serves it, beta fetched it from a holder and relayed it. That
  forwarding hop is proved in name-routing.feature and is assumed working here.
  What this file asserts about it is only what the fold and the chrome must say:
  that the serve went through a live holder, and that the chrome names which one.

  WHICH TERMS THIS FILE EXERCISES. The fold has four terms and these scenarios move
  two of them: REACH (narrowed by a ruling) and STANDING (Matthew admitted, James
  refused in the same minute). The other two are measured where they already have a
  home, and are not re-proved here: CONTRACT — that a doorway with no live hosting
  contract is never chosen to serve a name — is name-routing.feature's last
  scenario; LIVENESS — that a shedding doorway says so and its shed is not handed to
  a visitor — is doorway-failover.feature and doorway-apex-transition.feature. The
  one term-failure with no home yet is a contract that LAPSES or is WITHDRAWN while
  bytes are still warm, which is the contract-side twin of the warm-copy scenario
  below; it is named here as the next scenario to write rather than left to be
  noticed as an absence.

  WHAT THIS FILE DOES NOT CLAIM. It never proves the wide-area path: these are two
  doorways on one household mesh, and continuing ingress from the open internet is a
  separate prerequisite. It does not decide which doorway a public name resolves to
  (shared membership does, in doorway-apex-transition.feature) and it does not prove
  the forwarding hop (name-routing.feature does). A scenario here that passes while
  its refusal came from a doorway-local rule has proved the opposite of what it says.

  Background:
    # "E2E_DOORWAY_B" is the household lane's own name for the second doorway —
    # the CI vocabulary, not a typo for a missing "_BETA".
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_B"
    # The control for every scenario below. Without a real declared reach on a real
    # EPR and a collective that can actually rule on it, "refused because reach
    # narrowed" is indistinguishable from "this doorway has never heard of it", and
    # every assertion in this file passes vacuously.
    And the collective "the Dowell household" stewards the EPR "community-garden-club" at commons reach
    And doorway "alpha" holds a live projection contract for "community-garden-club"
    And doorway "beta" holds no contract for "community-garden-club"
    # Read, never assumed: the household's fixture manifest declares how long a
    # doorway may take to re-fold after a ruling. Without a declared bound,
    # "eventually" would satisfy every timing assertion below.
    And the household's declared reconcile window is read from its fixture manifest

  # The ruling has to travel further than the collective's own doorway. Beta was
  # never told; beta is not where the EPR lives; beta re-folds from the ledger like
  # everyone else and answers the stranger standing in front of it. The refusal is
  # the interesting part: a stranger who is told "not found" learns nothing and can
  # do nothing, so the refusal has to name reach as the reason, name who decided,
  # and offer the way to say something about it.
  @wip
  Scenario: A collective narrows an EPR's reach and every doorway's fold answers
    Given an anonymous visitor at doorway "beta" can read "community-garden-club" today
    When the collective "the Dowell household" rules that "community-garden-club" is no longer at commons reach
    # Not a prompt from the harness: the run waits for each doorway's own reconcile,
    # because the property being proved is that nobody had to tell it.
    And each doorway re-folds from the ledger on its own reconcile, unprompted
    Then an anonymous visitor at doorway "beta" is refused "community-garden-club"
    And the refusal names reach as the term that failed, not absence and not an error
    And the refusal names "the Dowell household" as the collective whose ruling narrowed it
    And the chrome of the refusal offers the visitor a way to be heard about that decision
    # The anti-vacuity assertion, and the reason this scenario is not just "a 403
    # appeared": a doorway that refused because it holds no contract, or because its
    # operator listed the EPR somewhere, would satisfy every line above by accident.
    And the refusal was decided by the fold, not by a doorway-local rule or list

  # The other side of the same ruling. Narrowing reach is not hiding a thing; it is
  # naming a smaller audience, and the people in that audience must still be served
  # — through whichever doorway they happen to be standing at, from whichever holder
  # is nearest and alive. Matthew is in the audience because his own conductor says
  # so, and beta learns it by asking him, not by looking him up.
  @wip
  Scenario: A member whose conductor carries the standing is served through the nearest live holder
    Given "community-garden-club" has been narrowed to a reach that admits members of "the Dowell household"
    And Matthew's conductor states his membership in "the Dowell household"
    # The control for the pair below: James is the same kind of requester, at the
    # same doorway, differing in exactly one term of the fold.
    And James's conductor states no membership in "the Dowell household"
    When Matthew asks doorway "beta" for "community-garden-club" as "Matthew"
    Then Matthew is served "community-garden-club"
    And the standing that admitted him was read from his own conductor
    And doorway "beta" served it through the nearest live holder of the contract
    And the chrome names the reach that admitted him and the holder that served the bytes
    # James in the same minute is what proves the fold discriminates rather than
    # having simply reopened.
    When James asks doorway "beta" for "community-garden-club" as "James"
    Then James is refused "community-garden-club"
    And the refusal names reach as the term that failed, not absence and not an error

  # The named gap, written as the scenario that must fail on today's binaries before
  # it can pass. Doorway "alpha" is the holder: it has the bytes warm and can answer
  # without asking anyone anything. That is exactly the state in which a narrowing
  # goes unnoticed — the shell keeps handing the old audience the same page because
  # nothing in the byte path ever asks again. The window is the household's declared
  # one, read in the Background, so "eventually" is not an answer; and holding the
  # bytes after the refusal is not a failure, it is the point: cache is bytes, never
  # permission.
  @wip
  Scenario: The holder's warm copy stops answering anonymously inside the declared window
    Given doorway "alpha" has served "community-garden-club" to an anonymous visitor and holds it warm
    When the collective "the Dowell household" rules that "community-garden-club" is no longer at commons reach
    Then within the household's declared reconcile window an anonymous visitor at doorway "alpha" is refused "community-garden-club"
    And the refusal names reach as the term that failed, not absence and not an error
    # Observed from alpha's own account of what it is holding, not inferred from the
    # refusal: a doorway that quietly dropped the bytes would refuse identically, and
    # would have cured this by forgetting rather than by asking.
    And doorway "alpha" still holds the bytes warm
    And no eviction of those bytes was required for the refusal

  # What was traded. A visit is not free and it is not a favour: someone held the
  # bytes, someone projected them, and the network books that work. The receipt is
  # the chrome telling the visitor so in a sentence they would say themselves, with
  # the ledger record behind it for anyone who asks for the precise form. A doorway
  # that credits itself for a holder's work is the failure this pins.
  @wip
  Scenario: The fair-trade receipt names what was exchanged for this serve and who was credited
    Given "community-garden-club" is at a reach that admits Matthew
    When Matthew is served "community-garden-club" through doorway "beta"
    Then the chrome carries a fair-trade receipt for this serve
    And the receipt names what was given in exchange for the serve
    And the receipt credits the holder peer that provided the bytes
    And the receipt credits doorway "beta" for the projection, separately from the holder
    And every credit on the receipt reads back to a record on the shared ledger
    # The one assertion here a machine cannot settle: the runner checks the checkable
    # half — that no protocol identifier is the first thing the receipt says, and that
    # the precise form is reachable in one request — and a person judges the rest.
    And the receipt is written in the words a friend would use, with the protocol form one request away

  # Redress with teeth. The first scenario's refusal told James where to be heard;
  # this is what happens when he takes that offer. The scenario stands on its own —
  # it produces the refusal it needs rather than inheriting one — because the point
  # being proved is what the challenge BECOMES, not how the refusal arose. The due
  # date matters because it is fixed before he sends anything and is not the
  # doorway's to set or to answer: the doorway carries his challenge to the
  # collective that ruled, and then gets out of the way. A challenge that lands in a
  # queue nobody owes anything to is the suggestion box this protocol replaces.
  @wip
  Scenario: A visitor's challenge becomes a witnessed commitment with a due window
    Given the collective "the Dowell household" has ruled that "community-garden-club" is no longer at commons reach
    And James asked doorway "beta" for "community-garden-club" and was refused naming reach
    When James uses the way to be heard in that refusal's chrome to challenge the standing
    Then his challenge is witnessed on the shared ledger as a commitment
    And the commitment names "the Dowell household" as the party that owes the response
    And the commitment carries a due date that was declared before James sent it
    And James can read the owed response and its due date from the same chrome
    And doorway "beta" did not answer the challenge on the collective's behalf
