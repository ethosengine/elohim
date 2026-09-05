# Design: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md
# (§3-§5: the channel, the compatibility envelope, the receipt chain) and the rung-5 workspace
# orchestration plan, Task 4. The household half of this ceremony is the sibling story
# features/delivery/runtime-upgrade-propagation.feature, Stations 1-9; this story is the crossing
# from a developer's own workstation to the deployed fleet, and it deliberately stops at the
# first safe rung: the fleet reads the release and applies nothing.
@e2e @delivery @workspace-release @concern:runtime-upgrade-propagation @act:iii @requires:shem
Feature: A developer's own peer carries a coordinator release to the fleet, and nothing out there applies it

  Today a one-line coordinator fix reaches the deployed peers only one way: a developer pushes,
  a pipeline builds an image, and an operator rolls the pods. The roll restarts the very peers
  whose delivery we were trying to measure, so the measurement costs more than the fix.

  A COORDINATOR is the behavioral half of a peer's runtime — the code that answers requests —
  and it is hot-swappable in a running peer, which is why it is the only class of change this
  story carries. The other half, the validation rules every peer holds each other to, cannot be
  swapped that way; changing it is a heavier ceremony this story does not touch.

  The household already proved the alternative on three peers of its own: in the sibling story
  (features/delivery/runtime-upgrade-propagation.feature) one household member published a
  coordinator fix as content, the other two peers resolved it through their own runtimes, one of
  them applied it and said what its device saw, and the whole house converged with no device
  restarting and nobody clicking anything. That proof is the ground this story stands on. What it
  did NOT do is leave the house. This story asks whether the same act reaches OUT: whether
  matthew, sitting at his workstation with his own peer joined to the fleet's network, can put a
  release where the fleet can see it, with no pipeline anywhere in the path.

  It stops one step short of the fleet applying anything, and that restraint is the point. The
  first time a workstation reaches a live fleet, the safe question is not "did it upgrade?" but
  "would it have been allowed to?" — asked in a way that cannot change a single running peer.

  Vocabulary this story leans on.

  MATTHEW is the developer this story follows: one of the household's own people, at his own
  workstation, with a coordinator change in hand. He stewards this channel, and he is the only
  person in the story — nobody on the fleet side is asked to do anything at all.

  A peer's CONDUCTOR is the process that holds its identity, keeps its copy of the network's
  records, and answers for it on the network. "Through his own conductor" means the answer was
  computed by his own machine from records it holds, rather than accepted from someone else's.

  The WORKSPACE PEER is matthew's own runtime — a conductor started on his workstation and
  JOINED to the fleet's network, so it holds the fleet's validation-rule identity and gossips on
  the fleet's own network, exactly like any deployed peer. It is not a client of the fleet and it
  is not an operator console; it is a peer, and everything this story reads about the fleet is
  read through it. Nobody here logs into a deployed machine, and no deployed machine is asked a
  question over the web — the reading path is the peer's own conductor and nothing else.

  A RELEASE CHANNEL is a content identity whose versions are releases and whose head is the
  channel's current one. Its name is a path read left to right: the channel used here names the
  runtime plane, then coordinator artifacts, then this network, then a channel called "workspace".
  A THROWAWAY CHANNEL is one minted for a single measurement and never promoted: a
  channel nobody's runtime is asked to treat as authoritative. It exists so this first crossing
  can be made on a channel whose worst case is that it is ignored. The channel these scenarios
  name IS that throwaway — it carries this measurement and nothing the fleet depends on.

  A head has two tiers and moves only by a deliberate act. Publishing declares a release STAGING:
  visible, fetchable, resolvable by every peer, and authoritative for nobody. A separate
  PROMOTION ceremony — never run in this story — is what would move that same head to EARNED,
  the tier an applying runtime acts on. So "still staging" at the end is not a tautology: it is
  the statement that the one act which could have armed the fleet was never performed.

  Every peer resolves the channel's head for itself by ELECTION: a deterministic rule each
  runtime applies locally to the declarations it can see (earned beats staging, newest breaks a
  tie). Nobody votes and nobody adjudicates; convergence comes from identical rules over
  identical data. To resolve a channel is to run that rule through your own runtime and no one
  else's.

  OBSERVE is the weakest of the three ways a runtime can follow a channel. An observing runtime
  watches the channel's head, fetches the release's bytes, verifies them against what it itself
  has installed, and reports what it found. It applies nothing, ever — applying is what the other
  two modes ask for by name, per channel. Every deployed peer follows this story's channel in
  observe, and that is declared in the fleet's own DEPLOYMENT DATA — the committed, reviewed
  record of what each deployed peer is, which reaches the peers only when the fleet is next
  rendered from it. It is not arranged by this test and cannot be changed by it: an applying mode
  would have to be committed, reviewed and rendered first, by a person, deliberately.

  ADMISSIBLE is the verdict this story is after: the release passed verification, so a peer that
  WAS in an applying mode would have been allowed to proceed. Admissible is not applied, and this
  story never lets the two blur. Three things must hold for that verdict, and each is checked by
  the adopting peer itself, against what that peer has installed:

    - the same VALIDATION-RULE IDENTITY — the hash of the validation rules the peers of one
      network hold each other to. Two peers share it or they are not on the same network at all,
      and a release built against a different one is refused outright rather than merely
      discouraged;
    - an ADDITIVE-ONLY WIRE CHANGE — the new coordinator may add requests and fields, but it may
      not remove or repurpose any that peers already running the old code depend on, so mixed
      versions keep talking through the swap;
    - a LINEAGE PARENT that matches — the release names the release it builds on, and that name
      has to be the channel's real current head, so a candidate can never quietly fork the
      channel's history.

  How an apply would be seen, if one happened. A peer's own verification verdict is private to
  that peer; what a peer cannot keep private is the SOAK ATTESTATION it authors after it applies
  a release and runs it clean for the declared window — a notarized entry, anchored on the
  release, readable by any peer on the network. So the absence of any soak attestation for this
  release, read through the workspace peer's own conductor across the whole observation window,
  is the network's own evidence that nobody out there applied it. It is a negative read of a
  positive record, not an assumption of quiet.

  What this story deliberately does NOT claim: it does not read each deployed peer's private
  verdict back. Those verdicts live on the peers that formed them, and the operator reads them
  from the fleet's own reports; asserting them here would mean reaching into deployed machines,
  which is exactly the path this story exists to avoid. What is proven here is that the release
  crossed, that it is admissible against the reality the fleet declares it runs, and that no
  peer applied it.

  Background:
    Given the workspace peer's conductor is joined to the fleet's network
    And the fleet's deployment data enrols every active peer in release channel "runtime:coordinators:elohim:workspace" in observe mode
    And the workspace peer follows that same channel in observe mode

  # Station 1: the release is minted here, against the reality this peer itself runs.

  Scenario: Station 1 — matthew mints a coordinator release on his own workstation, bound to the identity his peer actually runs
    Given matthew has a coordinator change he wants the fleet to see
    When he packages it as a release manifest against his own peer's installed reality
    Then the manifest declares the workspace channel and the same validation-rule identity his peer runs
    And the artifact is addressed by its own content and served straight from his peer's content-addressed store, so any peer can fetch the exact bytes without asking a pipeline for them

  # Station 2: the head moves by ceremony, on the developer's own key.

  Scenario: Station 2 — publishing is one act by the developer's own peer, with no pipeline in the path
    Given matthew has packaged the release for channel "runtime:coordinators:elohim:workspace"
    When he publishes it on that channel through his own peer
    Then the channel's head is the release he minted, declared staging by that act alone
    And the peer that declared the head is his own workstation peer, signing with its own key — nothing was built, pushed, or deployed to move it

  # Station 3: the crossing itself — the fleet's own network carries the head.

  Scenario: Station 3 — the release crosses to the fleet's network, read back through the developer's own peer
    Given matthew has published the release on channel "runtime:coordinators:elohim:workspace"
    When the channel's election is resolved through the workspace peer's own conductor on the fleet's network
    Then the election resolves to exactly the release minted on the workstation
    And the resolve is a local read through that peer's own conductor, never a report handed to it by someone else

  # Station 4: admissible, and applied by nobody — the whole measurement, in one scenario.
  # (Recall the soak attestation: the notarized entry a peer authors after it applies a release
  # and runs it clean. Nobody can apply quietly, so an empty count is the network's own answer.)

  Scenario: Station 4 — the release is admissible against the reality the fleet declares, and no peer applies it
    Given the release has crossed to the fleet's network on channel "runtime:coordinators:elohim:workspace"
    When every peer following that channel has been given its observation window
    Then the workspace peer's own runtime reports the release verified and admissible, with nothing applied, because observe is the only thing it was asked for
    And no soak attestation for that release exists anywhere on the network, read through the workspace peer's own conductor — the network's own record that no peer applied it
    And the channel's head is still staging, because no promotion ceremony was ever run

  # Station 5: the promise this ceremony makes to the developer, and its measure.
  # The whole point of taking the pipeline out is that the developer's own path is SHORTER than
  # the one it replaces. A ceremony that crosses to the fleet but takes a page of setup to run
  # has not replaced anything, so the recipe's own length is part of what this story proves.

  Scenario: Station 5 — a developer does all of this in five commands or fewer, and is told which step is still done by hand
    Given a developer who has never run this ceremony reads the workspace release recipe
    When the commands it asks them to run are counted
    Then there are five or fewer, covering mint, publish, and observe
    And the recipe names the one step that is still manual — enrolling a peer in the channel, which is deployment data and a render on the fleet, and one API call on a household mesh
