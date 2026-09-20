# Authority-in-integrity — the behavioural proof of the crossing.
#
# Across five DNAs (infrastructure, imagodei and node-registry exercised here; qahal and elohim
# not), the rules deciding who may claim standing over whom live in COORDINATOR zomes: hot-swappable
# code outside the DNA hash. The INTEGRITY zome — validation every peer runs identically,
# unswappable without changing the network's own identity — accepts all five claims below from ANY
# coordinator, because none is checked against `action.author()` there.
#
# PER-SCENARIO TRUTH TABLE (source-verified 2026-09-20):
#   01 peer-status link — needs a MODIFIED coordinator. Entry already bound: validate_peer_status
#      (infrastructure_integrity/src/lib.rs:443-452) requires ps.peer_id == action.author(), and
#      record_peer_status (infrastructure/.../src/peer_status.rs:23-35) reuses that checked id as
#      the link base. The hole is the LINK arm alone — FlatOp::Link(OpLink::CreateLink{..}) =>
#      Ok(Valid) unconditionally (same lib.rs:304); no shipped fn exposes the raw create_link.
#   02 stewardship grant — SHIPPED API SUFFICES: the ONE claim handed straight through with zero
#      coordinator modification. create_stewardship_grant (imagodei/zomes/imagodei/src/
#      stewardship.rs:351-397) binds steward_id to the caller (:379) but takes subject_id /
#      authority_basis / verified_by verbatim from input (:380-383), unchecked.
#   03 membership sponsor — needs a MODIFIED coordinator. affirm_membership (imagodei/.../src/
#      qahal_coordinator.rs:967-1043) verifies a real sponsor signature (~:992); create_collective
#      (:53-90) writes sponsor_cid: Some("founder") only for a collective the caller just made.
#      Integrity checks only sponsor_cid.is_some() — any sponsor string, "founder" included, on
#      SOMEONE ELSE's collective.
#   04 content server — needs a MODIFIED coordinator. ContentServer (infrastructure_integrity/src/
#      lib.rs:160-190) has no author-binding field at all, but register_content_server
#      (infrastructure/.../src/lib.rs:984+) anchors its discovery link to the caller's own agent_info(). Same open link arm as row 01.
#   05 health attestation — needs a MODIFIED coordinator SINCE 2026-09-20: attest_health
#      (node-registry/.../node_registry_coordinator/src/lib.rs:258+) now refuses an attester_node_id
#      that is not the calling node; before that the shipped API itself could forge the attester.
#      Integrity validates only ShardAssignment of six entry types (node_registry_integrity/src/lib.rs:275-282); HealthAttestation falls through to Valid.
#
# Derives from genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md (§"The story
# that proves it"). Habit: elohim/holochain/dna/.epr-meta/authority-in-integrity.habit.md
# Decisions this file must not contradict (full text in the backlog's "Decisions" section):
#   D1 — coordinator-polite authority is CONFIGURED trust, not earned; it gates DNA promotion.
#   D3 — a StewardshipGrant is subject-authored, or steward-authored WITH a distinct verified
#        witness signature — never the steward alone; scenario 06 exercises both legal forms.
#   D6 — every LinkTypes variant is authority-bearing or open BY DECLARATION, no `_ =>`; 01 is the
#        paradigm case. Every refusal fails today, so all six are @wip (a2o/CLAUDE.md); 01+04
#        (infrastructure) come off @wip once that crossing AND the harness gap below both close.
#
# HARNESS GAP (@harness-gap:conductor-zome-call). @holochain/client AdminWebsocket+AppWebsocket +
# app-auth token + app.callZome is proven in steps/sovereign-peer-join.steps.ts for ONE ad hoc
# conductor; hc-mesh-prologue.sh's "a2o env" block exports only DOORWAY/STORAGE http urls:
#   chain: authority-in-integrity crossing -> this feature's six scenarios
#   between: "a DNA's crossing lands" -> "this file measures it, for household peers, red-then-green"
#   missing node: a2o can open an authenticated app-websocket zome-call session against a NAMED
#     household-mesh peer's own conductor. Probe: `AppWebsocket.connect` on a household peer's app
#     port using only env the mesh prologue exports — none resolves today.
#   current state: red by absence (grep of that "a2o env" block, 2026-09-20).
@e2e @trust @dataplane @act:i @requires:multi-node @harness-gap:conductor-zome-call
Feature: Nobody can hand themselves standing over another person, household, or the network, whatever software they run
  As a person, a household, or someone who keeps a piece of shared infrastructure running
  I want any claim that another peer may act for me, speak for me, serve on my behalf, or vouch
  for something's health — "standing", for short — to be checked the same way by every honest peer
  in the network, not merely accepted because the software making the claim asked nicely
  So that my standing can never be taken by a peer I never trusted, no matter what code that peer
  happens to be running, and so that a peer who alters its own software gains nothing by it

  A word on who is who below. A peer is one machine running this network's software — a household's
  own box, a friend's laptop, a rack in somebody's basement. A person is a human being whose records
  such a machine keeps for them; several people can be at home on one peer, and the peer is not any
  of them. Two peers appear below, and they are separate machines: "author", which makes each claim,
  and "second", which has to decide whether to believe it.

  Standing takes five forms here, one per scenario: saying what another peer's condition is; being
  someone's steward; sitting on a group's roster of stewards; serving another peer's content; and
  vouching for a peer's health. They are not one mechanism, but they are one question — who was
  allowed to say this?

  Today each of these claims is checked by a piece of software that any peer is free to replace with
  its own version. What this story asks for is that the check stop being part of a replaceable app
  and become part of what makes this network THIS network — so every peer applies it identically,
  and no peer can opt out of it by running something different. One of the five claims below — the
  stewardship one, the second scenario — does not even need a peer to change its software: today's
  shipped tools hand it over as they are. The refusal has to hold either way, which is why the other
  four scenarios do not ask how the claim was made.

  Every scenario reads the result back from a SECOND peer, one that did not make the claim. That is
  deliberate. A fix that only stops a peer's own software from misbehaving protects nobody from the
  peer that has already changed that software; the refusal has to come from the people it is meant
  to protect, not from the claimant's good manners.

  The last scenario is the reason the first five can be believed. A rule that simply refused
  everything would pass all five by accident, so the honest form of every claim must still be
  accepted, and must still be visible to a peer that did not make it. Refusing legitimate claims
  would be a worse outcome than the hole this story closes. That control is deliberately one
  scenario and not five: what has to hold is that the same network refuses all the counterfeits and
  accepts the whole honest set, so a network that took only some of the honest claims must not be
  able to read as a pass.

  Honest status: these scenarios describe a promise the network does not keep yet, and cannot run
  today. Each one waits on a rule change in the network it names. They also wait on the test harness
  itself: the two peers are reachable today only through a shared front door on the web, and that is
  not the same thing as being able to sit down at each machine and make it speak for itself, which is
  what every scenario below requires. So read the Background for what it is — it gives the two peers
  their names, and nothing more. It is not where the access comes from, because nowhere is, yet. All
  six scenarios are marked unfinished on purpose.

  Background:
    # Naming only. These two steps bind the doorway HTTP surface; they do NOT supply the per-peer
    # conductor zome-call access every scenario needs — see @harness-gap:conductor-zome-call.
    Given peer "author" at "E2E_DOORWAY_ALPHA"
    And peer "second" at "E2E_DOORWAY_B"

  # ── 1. (infrastructure DNA) A peer cannot report another peer's status ───────

  @wip @dna:infrastructure @concern:authority-in-integrity-01-link-base
  Scenario: A peer cannot make itself the source of another peer's status
    # Claim: an AgentToPeerStatus link, whose base IS the ownership claim (the D6 case), passes for
    # any base today.
    Given peer "second" reports its own status and has never asked peer "author" to report it
    When peer "author" reports peer "second"'s condition as though peer "second" had said it
    Then peer "second", who did not make the claim, refuses it
    And the claim never appears in peer "second"'s recorded status
    And peer "second"'s recorded status, read back from peer "second", is exactly what it was before peer "author" acted

  # ── 2. (imagodei DNA) Nobody can steward a person who never agreed to it ─────

  @wip @dna:imagodei @concern:authority-in-integrity-02-stewardship-grant
  Scenario: Nobody can become someone's steward on their own say-so
    # Claim: a StewardshipGrant is checked for shape only; who asserted it is never consulted.
    # Decision D3's two legal forms — subject-authored, or steward-authored with a distinct
    # verified witness — are stated in the Given and exercised honestly in scenario 06.
    Given someone may steward a person only if that person agreed to it, or if a third party who is neither the steward nor the person witnessed the arrangement
    And a person on peer "second" has not agreed to peer "author" stewarding them, and nobody has witnessed any such arrangement
    When peer "author", using only the tools the network ships to everyone today, records itself as that person's steward on its own say-so
    Then peer "second", who did not make the claim, refuses it
    And the claim never appears in that person's stewardship record
    And that person's stewardship record, read back from peer "second", is exactly what it was before peer "author" acted

  # ── 3. (imagodei + qahal) A steward roster does not grow from an invented sponsor ──

  @wip @dna:imagodei @concern:authority-in-integrity-03-membership-sponsor
  Scenario: A group's roster of stewards does not grow from a vouching that never happened
    # A "collective" here is a named group with members — a household, a co-op, a court. Claim: a
    # Membership{role: Steward} is checked only for "some sponsor is named", never that the named
    # sponsor actually vouched; the literal word "founder" passes on a collective the claimant
    # never founded.
    Given a group on peer "second" keeps a roster of stewards, and a person joins that roster only when an existing steward vouches for them
    And no existing steward has vouched for peer "author"
    When peer "author" adds itself to that roster, naming a voucher who never vouched for it
    Then peer "second", who did not make the claim, refuses it
    And the claim never appears in the group's roster of stewards
    And the group's roster of stewards, read back from peer "second", is exactly what it was before peer "author" acted

  # ── 4. (infrastructure DNA) A peer cannot claim to serve on someone else's behalf ──

  @wip @dna:infrastructure @concern:authority-in-integrity-04-content-server
  Scenario: A peer cannot sign itself up to serve another peer's content
    # Claim: a ContentServer entry carries no author-binding field at all today.
    Given peer "author" runs storage of its own, and peer "second" has never asked it to serve any content
    When peer "author" lists itself as serving content on peer "second"'s behalf
    Then peer "second", who did not make the claim, refuses it
    And the claim never appears in peer "second"'s list of who may serve its content
    And peer "second"'s list of who may serve its content, read back from peer "second", is exactly what it was before peer "author" acted

  # ── 5. (node-registry DNA) A peer cannot sign a health report as someone else ──

  @wip @dna:node-registry @concern:authority-in-integrity-05-node-attestation
  Scenario: A peer cannot sign a health report as if it were another peer
    # Claim: no peer checks a HealthAttestation's claimed identity against who actually signed it;
    # the field named "Prevents spoofing" prevents nothing, since the action was already signed by
    # its real author and no validating peer compares the two.
    Given peer "author" has never been registered as peer "second", and holds none of peer "second"'s standing
    When peer "author" signs a health report as though peer "second" had written it
    Then peer "second", who did not make the claim, refuses it
    And the claim never appears in peer "second"'s recorded health
    And peer "second"'s recorded health, read back from peer "second", is exactly what it was before peer "author" acted

  # ── 6. NEGATIVE CONTROL — honest claims still land, and a second peer sees them ──

  @wip @concern:authority-in-integrity-06-negative-control
  Scenario: The honest form of every claim above still works, and a second peer can see it
    # One honest counterpart per numbered scenario above. The two stewardship lines are D3's two
    # legal forms, both exercised so neither goes untested. Why this is ONE scenario is in the
    # Feature text; each Then names its claim, so a failure report says which honest claim broke.
    Given peer "second" reports its own status honestly
    And a person on peer "second" has agreed to peer "author" stewarding them
    And a different person on peer "second" is stewarded by peer "author"
    And someone who is neither that person nor peer "author" witnessed that arrangement being made
    And an existing steward of a group on peer "second" vouches for peer "author", who then joins that group's roster of stewards
    And peer "second" asks peer "author" to serve some of its content, and peer "author" lists itself as serving it
    And peer "author" reports on peer "second"'s health, signed under peer "author"'s own name
    When each of these honestly made claims reaches peer "second"
    Then the honest status report is accepted
    And both honest stewardship arrangements are accepted
    And the honest addition to the roster of stewards is accepted
    And the honest offer to serve content is accepted
    And the honest health report is accepted
    And peer "second", who authored none of them, can see each one
