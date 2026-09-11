# Stewardship chapter 06 — the steward is a COLLECTIVE, and a person still answers.
#
# WHY THIS FILE EXISTS. The imagodei integrity zome already models a careful
# stewardship relationship: StewardshipGrant names two parties, says what its
# authority rests on, bounds itself to named surfaces rather than to a rank,
# expires, and carries an appeal_id giving the subject standing to contest it
# while it is in force. The same zome already models collectives: Collective,
# Membership with a Steward role and a sponsor, and MemberKind permitting a
# collective inside a collective, so a court to facility to unit chain is
# already expressible. What nothing models is the JOIN between the two.
# StewardshipGrant.steward_id is a single String, while authority_basis already
# names bases no individual can hold — court_order, organizational_role,
# community_consensus. No coordinator resolves a grant whose steward is a
# collective to the member who acted, and no act record names that member.
# That gap is the whole subject of this chapter.
#
# BORN RED, ON PURPOSE. Every scenario below asserts something that does not
# hold today. This file is the declared check for the habit
# `custodial-authority-answerable`
# (elohim/holochain/dna/imagodei/.epr-meta/custodial-authority-answerable.habit.md),
# whose invariant is that a party acting on another party's account is
# answerable for it. Do not weaken an assertion to make it pass — the way this
# chapter goes green is the two-party carrier above the DNA being designed
# against it. Precedent for a deliberately red chapter:
# features/dataplane/resiliency-saga/05-co-steward-agreement.feature.
#
# WHY THERE IS NO @wip HERE. genesis/a2o/CLAUDE.md asks for @wip on scenarios
# with unimplemented step definitions, and none of the steps below are wired
# yet. The tag is deliberately omitted. The mesh profile filters
# "@e2e and not @wip", so a @wip chapter is selected by no command and measures
# nothing — which would leave the habit in exactly the `unwired` state it exists
# to leave. An undefined step here is a loud, honest red naming a missing
# capability. Wire the steps, or argue with the assertion — do not silence it
# with a tag. If the lane later needs a green board more than it needs this
# red, adding @wip to the tag line below is the single-token way to do it, and
# the habit must go back to `unwired` in the same commit.
#
# THE APPEAL IS NOT OPTIONAL HERE. This directory's .epr-meta rule
# `grant-scenario-must-exercise-the-appeal` requires a grant scenario to
# exercise the subject's standing to contest, and intensifies the requirement
# when the steward is a collective. Scenario 3 carries it, and every other
# scenario is written so that it cannot be read as an admin console over a
# managed account.
#
# TAXONOMY. README.md row 06 is this chapter — authority reaches an acting
# human via Membership, and the acting human is identified. Row 07,
# institutional accountability, is the sibling not yet written; scenario 2
# below carries only the attribution half of it, and the subject-visible log
# belongs to 07.
#
# Run this chapter alone against the household mesh:
#   just test mesh '@concern:custodial-stewardship-06-collective-stewards'
@e2e @stewardship @concern:custodial-stewardship-06-collective-stewards @act:i
Feature: A household holds authority over a child, and a person still answers for it
  As james, a boy whose two parents manage his device between them
  I want every limit placed on my life to come from someone I can name and talk to
  So that being looked after by a household never means being governed by nobody

  THE STORY. James is eleven. His parents share the decisions about his device
  the way they share everything else — sometimes his mum sets the rule, sometimes
  his dad, and neither of them thinks of it as an act of authority. From inside
  the house that is just family life. From inside the software it is the hardest
  case there is: the party holding authority is a household, and a household
  cannot be asked why. If James wakes up to a device that has decided he is done
  for the night, and the only answer available to him is "your household did
  that", he has been governed by an office rather than cared for by a person —
  and the same shape, one size up, is a care home, a school, a facility, or a
  court acting on someone with far less standing than an eleven-year-old at his
  own kitchen table. This chapter asserts the thing that keeps the household
  case and the institutional case honest: authority that is held collectively
  still reaches a named person, both before the act and after it, and the person
  it is exercised on can object while it is still being exercised.

  Terms this story leans on, defined here so the steps can be judged alone:
  - A "party" is whoever holds one end of the relationship. Neither end has to
    be one person. Here the acting party is a household; the subject is a boy.
  - A "collective" is a named group with a charter — a household, a co-op, a
    care home, a court. It has members, and a member may itself be a collective.
  - A "steward membership" is one person's standing inside a collective to act
    on its behalf. It is granted, it is sponsored by someone who vouches for it,
    and it can be withdrawn. Having a steward membership somewhere is not a rank
    and confers nothing anywhere else.
  - A "grant" is the record of one party holding authority over another. It says
    who holds it, over whom, what it rests on — a guardianship, a court order, a
    medical necessity, an organizational role, the subject's own consent — which
    surfaces of the subject's life it covers, when it expires, and when it must
    be reviewed. A grant never expresses a level or a rank; it names surfaces.
  - A "named surface" is one dimension the grant covers: how long the device may
    be used, what content is filtered, which features are available, whether
    activity may be watched, whether authority may be passed on. A surface the
    grant does not name is a surface the steward has no authority over, however
    senior the steward is anywhere else.
  - An "act" is one exercise of a grant — setting a limit, blocking a category,
    reading a log. Every act claims a grant, and the point of this chapter is
    that every act also names the human who performed it.
  - An "objection" is the subject's formal contest of a grant or of a limit set
    under it, filed while the grant is still in force. It says which grant, what
    kind of complaint it is — the scope is wrong, the restriction is
    disproportionate, the evidence for the authority is bad, or a capability is
    being asked for — and it carries its own deadline by which it must be
    decided. Being able to file one is never itself restrictable.
  - "Answered" means a decision was recorded with the grounds for it, by the
    forum the objection names. It does not mean a person with a bigger title
    said no. An objection that reaches its deadline undecided is reported as
    unanswered; silence is never a verdict.
  - Limits compose ONE WAY. A second party holding authority over the same
    person may ADD a restriction and may never lift one another party set. That
    is what lets two stewards who are peers — a mother and a father, a household
    and a school — compose without anyone having to be declared first.
  - "Refused" means an explicit refusal carrying a reason. A crash, a timeout,
    or a route that does not exist is a failure of the scenario, never a pass.
  - The Background's quoted names are environment variables, not addresses.
    "E2E_DOORWAY_ALPHA" and "E2E_STORAGE_URL" are read from the run's own
    environment and locate the doorway and the storage peer of whichever
    household mesh this run owns, so the same steps hold on a laptop mesh and
    in CI without ever naming a host.
  - The cast is the household fixture this lane already seeds:
    "human-matthew-manager" and "human-jessica-spouse" are the two adults,
    "human-james-son" is the boy, and "family-dowell" is the household
    collective they formed. "homeschool-coop" is the second collective, standing
    in for every institution that holds authority over someone alongside their
    family.
  - Status vocabulary: no scenario here is @wip, and none of them passes today.
    Each one names a capability that does not exist yet — resolving a grant held
    by a collective to its acting member, recording that member on the act,
    answering an objection against a live grant and naming who answered,
    reporting an unanswered one as unanswered, refusing an act outside the
    grant's named surfaces, composing a second party's limits without letting
    either lift the other's, and refusing an act with no nameable actor. An
    undefined step below is the measurement.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And elohim-storage is reachable at "E2E_STORAGE_URL"
    And the collective "family-dowell" has "human-matthew-manager" and "human-jessica-spouse" as steward members
    And "human-james-son" is a member of "family-dowell" holding no steward role

  # The two-hop path the lane README names: grant to collective, collective to
  # member, member to a person who can be asked why. Today the first hop has no
  # resolver at all — steward_id is a String and nothing reads it as a collective.
  # The last assertion is the whole point: a household cannot be asked why, so an
  # answer that stops at the collective has not answered the question.
  Scenario: Asking who may act on james's account answers with people, never with a household
    Given an active stewardship grant over "human-james-son" whose steward is the collective "family-dowell"
    And that grant rests on "minor_guardianship"
    When james asks who may act under that grant
    Then the answer names "human-jessica-spouse" and "human-matthew-manager"
    And each name is backed by a steward membership in "family-dowell" that is sponsored and not withdrawn
    And the answer is never the collective on its own

  # Attribution AFTER the act. This is the half of README row 07 that belongs to
  # this habit: the subject-visible log itself is chapter 07's assertion.
  Scenario: james can see which adult set the limit, not merely that the household did
    Given an active grant over "human-james-son" held by the collective "family-dowell" covering time limits
    When "human-jessica-spouse" sets a nightly cut-off on james's device under that grant
    Then the record of that change names "human-jessica-spouse" as the person who made it
    And the record names the grant the change was made under, so anyone can check which authority it claimed
    And when james opens his own account he is told which person changed it and when

  # The .epr-meta rule: a grant scenario must exercise the appeal. The objection
  # is filed while the limit is still on him — an appeal that only works after
  # the grant lapses is not standing, it is a complaints box. The deadline is not
  # invented by this test: the objection carries its own expiry, and the grant it
  # names carries the review date that bounds it. The answer comes from a person
  # for the same reason the act did — jessica holds the steward membership, so
  # jessica is who james gets to hear from. The single unbroken sequence below is
  # deliberate: filing and answering are one story, not two scenarios sharing a
  # fixture, and reading them apart is what lets an answer drift out of the grant's
  # lifetime.
  Scenario: james objects while the limit is still on him, and a named adult answers before the deadline
    Given the nightly cut-off on "human-james-son" is in force
    And the grant it was set under is still active and carries a review date
    When james objects that the cut-off is disproportionate and gives his reasons
    Then the objection is recorded against that grant while the grant is still active
    And james was not blocked from filing the objection
    And the objection carries an answer deadline no later than that grant's review date
    And the grant carries a reference to the open objection for as long as it is open
    And "human-jessica-spouse" files a decision on that objection together with the grounds for it
    And the decision is recorded against the objection before its answer deadline
    And the objection is no longer open
    And james is shown the decision, the grounds for it, and "human-jessica-spouse" as the person who answered
    And james could read every restriction on him immediately before the decision was filed

  # The second half of the appeal path, and a different claim from the one above:
  # not "an answer arrives" but "the absence of one is visible". A forum that can
  # run out the clock has taken james's standing back without ever refusing him.
  Scenario: An objection nobody answered is reported as unanswered, never as refused
    Given an open objection by "human-james-son" against an active grant held by "family-dowell"
    And its answer deadline has passed with no decision recorded
    When james asks what became of his objection
    Then he is told that it is unanswered and that its deadline has passed
    And it is not reported as decided, denied, closed, or withdrawn
    And the grant it names is reported as still carrying an unanswered objection

  # Relational, not a rank. The grant is authority over THESE surfaces of THIS
  # person; matthew's standing everywhere else is untouched and irrelevant.
  Scenario: The household's authority covers james's screen time and not his reading
    Given an active grant over "human-james-son" held by "family-dowell" that names time limits and content filtering
    And that grant does not name activity monitoring
    When "human-matthew-manager" asks for james's reading history under that grant
    Then the request is refused with a reason naming the surface the grant does not cover
    And nothing about what james has been reading is returned or recorded
    And the grant is left intact, because matthew is still james's steward for the surfaces it does name

  # One-way composition, the half that must WORK: a second party may add. James
  # now lives under two limits from two households-worth of authority, and
  # nothing about the first grant changed when the second was exercised.
  Scenario: The co-op adds a quiet hour and both limits apply to james
    Given an active grant over "human-james-son" held by "family-dowell" whose policy sets a nightly cut-off
    And a second active grant over "human-james-son" held by the collective "homeschool-coop" resting on "organizational_role"
    When the co-op adds a quiet hour during lesson time under its own grant
    Then both the nightly cut-off and the quiet hour are active on james
    And the household's grant and its cut-off are unchanged

  # One-way composition, the half that must be REFUSED — and with it the reason
  # there is no primary steward: because neither party can undo the other, the
  # two compose as a set and nobody has to be declared first.
  Scenario: The co-op cannot lift the household's cut-off
    Given an active grant over "human-james-son" held by "family-dowell" whose policy sets a nightly cut-off
    And a second active grant over "human-james-son" held by the collective "homeschool-coop" resting on "organizational_role"
    When the co-op attempts to lift the household's nightly cut-off
    Then the attempt is refused with a reason naming the layer that set the cut-off
    And the nightly cut-off is still active on james
    And neither grant is recorded as the primary one
    And the restrictions james lives under are the same set whichever of the two grants is read first

  # The retirement condition of the habit this chapter checks: an institutional
  # act that cannot name a human who stands behind it should be UNREPRESENTABLE,
  # not merely discouraged. Until it is unrepresentable it must at least be
  # refused, and the concrete case is the one the household will actually meet —
  # someone who used to hold the standing and does not any more. Matthew's name
  # on the act is not enough; the membership behind it has to still be there.
  Scenario: An act matthew can no longer stand behind does not happen
    Given an active grant over "human-james-son" held by the collective "family-dowell"
    And the steward membership of "human-matthew-manager" in "family-dowell" has been withdrawn
    When "human-matthew-manager" sets a restriction on james under the household's grant
    Then the act is refused with a reason naming the withdrawn steward membership
    And nothing is recorded against james
    And the withdrawal counts from the moment it was made, not from some later sweep
