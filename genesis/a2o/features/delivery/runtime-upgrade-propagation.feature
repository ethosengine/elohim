# Design: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md
# (§4 constitutional posture; §10 the receipt chain each Station below is one link of).
# Implementation status lives in backlog task-runtime-upgrade-a2o-receipt, not here.
@e2e @delivery @runtime-upgrade @concern:runtime-upgrade-propagation @requires:household-nodes @act:i
Feature: The household's runtime stays current the way its content does — by election, not by prompt

  This story is rung 5 of the household's upgrade-velocity ladder — the
  runtime itself joining the same convergence its content already rides.
  Rungs 1-4 (hot-swapping a running peer's coordinator code, splitting its
  conductor into its own workload, taking a staggered fleet roll, and
  reloading its own config in place) are already landed: the ground this
  rung stands on.

  matthew's household already trusts one thing completely: when he publishes
  a photo, every peer in the house converges on the same version with no one
  clicking anything. This story asks the household to trust the same
  mechanism for the software running its own peers. A coordinator release is
  not a special kind of update pushed in from outside — it is content,
  addressed the same way, elected the same way, carried by the same
  channels. Each peer's own runtime stewards itself onto the elected head;
  nobody in the house is ever shown an "update available" dialog.

  Vocabulary the scenarios below lean on, because several of these words
  carry real weight and only get more confusing left to context.

  The COORDINATOR is a peer's behavioral layer — hot-swappable in a running
  peer, no restart required — while the INTEGRITY layer is the shared
  validation identity every peer holds each other to; changing THAT needs a
  different, heavier ceremony this story does not cover. In this substrate
  that validation-rule identity IS the household's DNA lineage: "no change to
  the DNA lineage" and "the same validation-rule identity" name one fact.
  The DOORWAY is the household's gateway to the ordinary web; it is how a
  test reaches the peers, and it plays no part in the upgrade ceremony.

  A RELEASE CHANNEL is a content identity whose versions are release
  manifests and whose canonical head is the channel's current release.
  `runtime:coordinators:elohim:commons` is the household's shared,
  constitutional channel — the one every peer ultimately converges on. It
  carries ONE head with two tiers, and the household's SOAK is that head's
  staging tier: every peer resolves a staged candidate through its own
  conductor, but only the canary is expected to actually apply and attest it
  before the household trusts it enough to promote it to earned. Soak and
  commons are not two channels a peer has to reconcile; they are the same
  head before and after evidence. `runtime:coordinators:elohim:canary-james`
  IS a second channel — james's own PERSONAL EXPERIMENT channel, a separate
  content identity followed by no one but him, that never gets promoted and
  is never expected to converge with commons at all.

  The COMPATIBILITY ENVELOPE is what makes a release, or a whole diverging
  channel, welcome rather than refused: the same validation-rule identity as
  the household's integrity layer, wire changes that only add and never
  remove or repurpose what already exists, and a declared lineage parent
  that actually matches the channel's real history. Diverging INSIDE the
  envelope is welcome — that is what james's personal channel does, all
  story long. Breaking it is refused outright, never merely discouraged.

  STAGING and EARNED are the same two-tier ELECTION every other piece of
  content in this house already uses. Election here names a deterministic
  RULE — not a ballot, not a vote, and no designated judge — that every peer
  applies locally to the same declared candidates (earned beats staging,
  newest breaks any remaining tie); convergence emerges because identical
  rules run over identical data on every peer, not because anyone counted
  votes or held a round. Staging is a candidate; earned is a promotion that
  carries evidence, and a release's own builder can never earn its own
  promotion — someone else's device has to have actually run it and said
  so. An ATTESTATION is that someone-else saying so: not just "it worked"
  but the CONTEXT it worked in — whose device, what kind of device, and a
  concrete thing it checked (for example, that the new coordinator code
  still answers a real request within budget, on real hardware). A channel's
  ATTESTATION DISCIPLINE is the rule its releases carry for how that evidence
  is counted: how long a canary must run a release clean before its
  attestation counts, and how many devices must attest before promotion. The
  discipline is written into each release manifest, and once a channel has a
  head, every later release on that channel inherits the head's discipline
  unless the steward deliberately changes it — a number nobody typed is not a
  discipline. This household is three devices of one kind, so its commons
  channel's discipline is a single attester: james.

  Three verbs carry different weight in what follows. To FOLLOW a channel is
  to declare interest in it — a runtime will track its head, nothing more.
  To RESOLVE a channel is to discover its current head through your OWN
  conductor, the one place that resolution is ever trusted to happen; a peer
  hint or a signal can point a runtime at a channel worth checking, but it
  never stands in for the check itself — the conductor still has to look.
  To ADOPT is the full act built on a resolve: fetch the release's bytes,
  verify them against what the runtime already has installed, apply them in
  place, and attest the outcome; a restart would be a failure of the
  mechanism, not a cost of it. REVERT is not a different mechanism from any
  of this — it is the ceremony declaring an earlier head canonical again,
  and every runtime converging backward through the identical loop it used
  to converge forward.

  A CEREMONY is the steward's deliberate act on a channel — publishing a
  candidate, promoting one to earned, or reverting to an earlier head. It is
  the only way a head ever moves. It is distinct from the election, which
  every peer runs on its own: the ceremony declares, the election decides
  what each peer honours. And a ceremony cannot conjure evidence: promoting
  to earned requires an attestation from a device that is not the builder's,
  so a canary's attestation and the steward's ceremony are each necessary
  and neither is sufficient alone.
  A channel is long-lived: the steward publishes on the same channel for as
  long as the household keeps it. Publishing a new candidate on a channel that
  already has an earned head requires the steward's own runtime to be running
  that head — a steward cannot push what they have not themselves adopted —
  and the candidate names that head as the release it builds on. The earned
  head stays the head while the candidate is staged beneath it; only the
  promotion ceremony moves the head.

  A runtime's PASSPORT is its own self-report of who it is and what it runs:
  its agent identity (the key by which every other peer knows it), its CELLS
  — each shared network it participates in under that identity, one per
  validation-rule identity — and the hashes of the coordinator code it
  currently runs. Station 3 reads it before and after adoption to prove that
  only the last of those changed. An OPERATOR, in Station 8, is whoever
  reads the household's runtimes back — matthew, or anyone with the same
  read access — and the point of that station is that what they read comes
  from each runtime's own reports, never from the ceremony's intent.

  Three people share this house. matthew is trusted to steward the
  household's channels and run the ceremony. jessica has never once been
  asked whether her software should update, and never will be — her
  protection is that the choice was taken out of anyone's hands, not that
  she gets to make it herself. james is the one who tries things first: he
  is both the canary — first to actually apply and attest what is staged on
  the household's commons channel — and the one member of the house who
  keeps a second, personal channel of his own running alongside it, a
  compatible variant nobody asks him to give up, and nobody lets outvote the
  house.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "E2E_STORAGE_MATTHEW"
    And peer "jessica" at "E2E_STORAGE_JESSICA"
    And peer "james" at "E2E_STORAGE_JAMES"
    And the household's runtime follows release channel "runtime:coordinators:elohim:commons"
    And james is designated the canary on that channel's staging tier
    And james's runtime additionally follows his own personal channel "runtime:coordinators:elohim:canary-james"

  # Station 1: publish — a release starts as a staging candidate, never straight to earned.

  Scenario: Station 1 — matthew publishes a coordinator fix as a staging candidate on the commons channel, never straight to earned
    Given matthew has built a coordinator fix — new behavior, no change to the household's DNA lineage
    When matthew publishes it as a release manifest on the channel "runtime:coordinators:elohim:commons"
    Then the release is declared staging, not earned
    And the release is not visible as the earned head of the commons channel

  # Station 2: staging election converges on every peer, each through its own conductor.

  Scenario: Station 2 — every peer's own runtime resolves the same staged candidate through its own conductor
    Given matthew has published a staging release on channel "runtime:coordinators:elohim:commons"
    When each household peer's runtime next resolves that channel
    Then matthew's, jessica's, and james's runtimes all resolve the identical staged release
    And each peer's runtime reports its own conductor as the resolution path — a peer hint may have pointed it at the channel worth checking, but the record shows a verified local resolve, never the hint itself adopted as fact

  # Station 3: the canary adopts and attests with context.

  Scenario: Station 3 — james, the canary, adopts the staged release and says what his device saw
    Given the staged release on channel "runtime:coordinators:elohim:commons" has resolved on james's runtime
    When james's runtime applies the release
    Then applying it changes nothing about who james is to the rest of the household — his runtime's own passport reports the same agent identity and the same cells, with only the coordinator behavior different
    And james's runtime attests the outcome, naming his device's hardware profile and a concrete thing it checked
    And james's own attestation alone could never be enough to earn the release for the household

  # Station 4: earned promotion — declared on evidence, never on the publisher's own say-so.

  Scenario: Station 4 — the ceremony promotes the release to commons on james's evidence, never on matthew's word alone
    Given james's staging attestation for the release on channel "runtime:coordinators:elohim:commons" is recorded
    When matthew runs the promotion ceremony for that release
    Then the release becomes the earned head of channel "runtime:coordinators:elohim:commons"
    And the promotion names james's attestation as the evidence it rests on — the single attester the commons channel's discipline asks for

  # Station 5: fleet convergence — nobody restarts, nobody is asked.

  Scenario: Station 5 — the whole household converges on the promoted release without anyone clicking "update"
    Given channel "runtime:coordinators:elohim:commons" now declares the promoted release earned
    When each household peer's runtime next resolves the commons channel
    Then matthew's, jessica's, and james's runtimes all apply the release without anyone's device restarting (each conductor process keeps the same PID it had before)
    And jessica is shown no prompt, asked no question, and given nothing to click
    And nothing about jessica's own content, files, or recorded agreements with the rest of the household changes because of the upgrade

  # Station 6: revert by re-election — the household finds it wanting.

  @wip
  Scenario: Station 6 — the household finds the change wanting, and reverting needs nothing but the ceremony saying so
    Given the household has converged on the promoted release and now judges it a regression
    When matthew runs the revert ceremony, re-declaring the prior release the earned head of channel "runtime:coordinators:elohim:commons"
    Then matthew's, jessica's, and james's runtimes all return to the prior coordinator behavior, converging backward through the identical loop they used to converge forward
    And nothing outside the ceremony itself was needed to get there — no operator flag, no re-key, no DHT reset

  # Station 7: throughout — the experiment channel is heard, never outvoted.

  @wip
  Scenario: Station 7 — james's personal channel rides alongside the ceremony, compatible and never forced to converge
    Given matthew has run the ceremony that staged, promoted, and reverted a release on the commons channel
    And james's runtime reported on both of its channels at each of those three moments, as they happened
    When the report james's runtime gave after staging is read back
    Then his personal channel was diverging from commons, inside the same compatibility envelope
    And james's runtime was converged on commons at that moment, exactly like matthew's and jessica's
    When the report james's runtime gave after promotion is read back
    Then his personal channel was still diverging from commons, inside the same compatibility envelope
    And james's runtime was converged on commons at that moment, exactly like matthew's and jessica's
    When the report james's runtime gave after the revert is read back
    Then his personal channel was still diverging from commons, inside the same compatibility envelope
    And james's runtime was converged on commons at that moment, exactly like matthew's and jessica's
    And nobody promotes james's channel to commons and nobody forces james off it

  # Station 8: the observed proof — the whole ceremony, read back honestly, not asserted from intent.

  @wip
  Scenario: Station 8 — the observed version matrix shows every transition the household actually went through
    Given matthew has run the ceremony that staged, promoted, and reverted a release on the commons channel
    And james's personal channel ran alongside that ceremony the entire time
    When an operator reads the household's observed version matrix
    Then the matrix shows matthew's, jessica's, and james's runtimes moving staging, then earned, then back, in that order
    And the matrix shows james's personal channel diverging compatibly the whole time
    And every row in the matrix is read from what each runtime itself reports, never asserted from the ceremony's own intent

  # Station 9: the channel is long-lived — the NEXT release rides the same head, it does not mint a new channel.
  # (Seam: the driver refused to publish over an earned head, so every release needed a fresh channel; and a
  # peer already running the target bytes was refused by content identity. This station proves both are healed.)

  Scenario: Station 9 — matthew's next fix rides the same commons channel: publish over the adopted head, and a peer already running the bytes is current, not refused
    Given the household has converged on the earned head of channel "runtime:coordinators:elohim:commons" and matthew's own runtime has adopted it
    And matthew has built a second coordinator fix on top of the one the household runs
    When matthew publishes the second fix as a release manifest on the same channel "runtime:coordinators:elohim:commons"
    Then the publish is admitted because matthew's runtime already runs the channel's earned head, and the second fix is declared staging beneath that earned head
    And the release manifest carries the commons channel's attestation discipline, inherited from the head — a single attester — not a default matthew never typed
    And every peer's runtime keeps running the earned head while it resolves the staged second fix
    When james, the canary, adopts and attests the staged second fix and matthew runs the promotion ceremony on that evidence, following the same ceremony Stations 3–4 proved for the first fix
    Then the second fix becomes the earned head of channel "runtime:coordinators:elohim:commons", james's attestation meeting the same single-attester discipline the first fix met
    And matthew's and jessica's runtimes converge on the second fix without any device restarting, within the same convergence window the first fix needed
    And james's runtime — already running the second fix's bytes from his canary adoption — reports the earned head applied and current, never refused on lineage grounds

  # Structural protection, not a veto — the constitutional posture named (spec §4).

  @wip
  Scenario: jessica's runtime has no opt-out control for an individual upgrade, and what she gets instead is named, not abstract
    Given jessica's runtime is following release channel "runtime:coordinators:elohim:commons"
    When that channel's earned head changes
    Then jessica's runtime adopts it without asking her permission
    And jessica's runtime exposes no setting, flag, or control that lets her defer, decline, or veto that individual upgrade
    But jessica can still read the release's own explanation of what changed and why, and can raise it to the steward if her stored content, her recorded agreements, or her identity were mishandled by it — and the revert ceremony is the household's remedy

  # Negative control: the compatibility envelope is floor-protected, not a courtesy.

  @wip
  Scenario: a release that breaks the compatibility envelope is refused by every peer's own verification, not merely discouraged
    Given a release manifest was built against a different validation-rule identity than the one the household actually runs
    When any household peer's runtime verifies that release locally
    Then no one's device is put at risk by the mismatch — that peer refuses the release outright, naming a typed reason
    And no household peer ever applies it, no matter which channel declared it earned
