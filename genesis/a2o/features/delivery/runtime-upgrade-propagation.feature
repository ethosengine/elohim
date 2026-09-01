# Design: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md
# (§4 is the constitutional posture this story stages; §10 names the exact
# receipt chain — publish, staging converges, canary adopts + attests,
# earned promotion, fleet converges, revert-by-re-election, observed matrix —
# and each numbered "Station" scenario below is one link of that chain, so
# the future step definitions and receipt script bind to it 1:1).
#
# This is the STORY half of the graduation-trigger artifact (backlog:
# task-runtime-upgrade-a2o-receipt). It is @wip until the sibling atoms land:
# the release-manifest schema and channel-ceremony driver (T1-T2), the
# adoption-controller observe surface (T3), and the apply vehicles (T4) — the
# receipt script and step wiring are a separate, blocked task.
@e2e @delivery @runtime-upgrade @concern:runtime-upgrade-propagation @requires:household-nodes @wip @act:i
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
  different, heavier ceremony this story does not cover.

  A RELEASE CHANNEL is a content identity whose versions are release
  manifests and whose canonical head is the channel's current release.
  `runtime:coordinators:elohim:commons` is the household's shared,
  constitutional channel — the one every peer ultimately converges on.
  `runtime:coordinators:elohim:canary-a` is the household's shared SOAK
  channel: every peer follows it, but only the canary is expected to
  actually apply and attest what is on it before the household trusts it
  enough to promote. `runtime:coordinators:elohim:canary-james` is different
  again — james's own PERSONAL EXPERIMENT channel, followed by no one but
  him, that never gets promoted and is never expected to converge with
  commons at all.

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
  still answers a real request within budget, on real hardware).

  Three verbs carry different weight in what follows. To FOLLOW a channel is
  to declare interest in it — a runtime will track its head, nothing more.
  To RESOLVE a channel is to discover its current head through your OWN
  conductor, the one place that resolution is ever trusted to happen; a peer
  hint or a signal can point a runtime at a channel worth checking, but it
  never stands in for the check itself — the conductor still has to look.
  To ADOPT is the full act built on a resolve: fetch the release's bytes,
  verify them against what the runtime already has installed, apply them,
  and attest the outcome. REVERT is not a different mechanism from any of
  this — it is the ceremony declaring an earlier head canonical again, and
  every runtime converging backward through the identical loop it used to
  converge forward.

  Three people share this house. matthew is trusted to steward the
  household's channels and run the ceremony. jessica has never once been
  asked whether her software should update, and never will be — her
  protection is that the choice was taken out of anyone's hands, not that
  she gets to make it herself. james is the one who tries things first: he
  is both the canary — first to actually apply and attest what is staged on
  the household's shared soak channel — and the one member of the house who
  keeps a second, personal channel of his own running alongside it, a
  compatible variant nobody asks him to give up, and nobody lets outvote the
  house.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "E2E_STORAGE_MATTHEW"
    And peer "jessica" at "E2E_STORAGE_JESSICA"
    And peer "james" at "E2E_STORAGE_JAMES"
    And the household's runtime follows release channel "runtime:coordinators:elohim:commons"
    And the household's runtime also follows its shared soak channel "runtime:coordinators:elohim:canary-a"
    And james is designated the canary on that soak channel
    And james's runtime additionally follows his own personal channel "runtime:coordinators:elohim:canary-james"

  # Station 1: publish — a release starts on the shared soak channel, never straight to commons.

  Scenario: Station 1 — matthew publishes a coordinator fix to the household's soak channel, not straight to commons
    Given matthew has built a coordinator fix — new behavior, no change to the household's DNA lineage
    When matthew publishes it as a release manifest on the soak channel "runtime:coordinators:elohim:canary-a"
    Then the release is declared staging, not earned
    And the release is not visible as a candidate on the commons channel

  # Station 2: staging election converges on every peer, each through its own conductor.

  Scenario: Station 2 — every peer's own runtime resolves the same staged candidate through its own conductor
    Given matthew has published a staging release on channel "runtime:coordinators:elohim:canary-a"
    When each household peer's runtime next resolves that channel
    Then matthew's, jessica's, and james's runtimes all resolve the identical staged release
    And each peer's runtime reports its own conductor as the resolution path — a peer hint may have pointed it at the channel worth checking, but the record shows a verified local resolve, never the hint itself adopted as fact

  # Station 3: the canary adopts and attests with context.

  Scenario: Station 3 — james, the canary, adopts the staged release and says what his device saw
    Given the staged release on channel "runtime:coordinators:elohim:canary-a" has resolved on james's runtime
    When james's runtime applies the release
    Then applying it changes nothing about who james is to the rest of the household — his runtime's own passport reports the same agent identity and the same cells, with only the coordinator behavior different
    And james's runtime attests the outcome, naming his device's hardware profile and a concrete thing it checked
    And james's own attestation alone could never be enough to earn the release for the household

  # Station 4: earned promotion — declared on evidence, never on the publisher's own say-so.

  Scenario: Station 4 — the ceremony promotes the release to commons on james's evidence, never on matthew's word alone
    Given james's soak attestation for the release on channel "runtime:coordinators:elohim:canary-a" is recorded
    When matthew runs the promotion ceremony for that release
    Then the release becomes the earned head of channel "runtime:coordinators:elohim:commons"
    And the promotion names james's attestation as the evidence it rests on

  # Station 5: fleet convergence — nobody restarts, nobody is asked.

  Scenario: Station 5 — the whole household converges on the promoted release without anyone clicking "update"
    Given channel "runtime:coordinators:elohim:commons" now declares the promoted release earned
    When each household peer's runtime next resolves the commons channel
    Then matthew's, jessica's, and james's runtimes all apply the release without anyone's device restarting (each conductor process keeps the same PID it had before)
    And jessica is shown no prompt, asked no question, and given nothing to click
    And nothing about jessica's own content, files, or recorded agreements with the rest of the household changes because of the upgrade

  # Station 6: revert by re-election — the household finds it wanting.

  Scenario: Station 6 — the household finds the change wanting, and reverting needs nothing but the ceremony saying so
    Given the household has converged on the promoted release and now judges it a regression
    When matthew runs the revert ceremony, re-declaring the prior release the earned head of channel "runtime:coordinators:elohim:commons"
    Then matthew's, jessica's, and james's runtimes all return to the prior coordinator behavior, converging backward through the identical loop they used to converge forward
    And nothing outside the ceremony itself was needed to get there — no operator flag, no re-key, no DHT reset

  # Station 7: throughout — the experiment channel is heard, never outvoted.

  Scenario: Station 7 — james's personal channel rides alongside the ceremony, compatible and never forced to converge
    Given matthew has run the ceremony that staged, promoted, and reverted a release on the commons channel
    When james's runtime is asked what his personal channel is doing after staging
    Then his personal channel is diverging from commons, inside the same compatibility envelope
    And james's runtime is converged on commons at that point, exactly like matthew's and jessica's
    When james's runtime is asked the same question after promotion
    Then his personal channel is still diverging from commons, inside the same compatibility envelope
    And james's runtime is converged on commons at that point, exactly like matthew's and jessica's
    When james's runtime is asked the same question after the revert
    Then his personal channel is still diverging from commons, inside the same compatibility envelope
    And james's runtime is converged on commons at that point, exactly like matthew's and jessica's
    And nobody promotes james's channel to commons and nobody forces james off it

  # Station 8: the observed proof — the whole ceremony, read back honestly, not asserted from intent.

  Scenario: Station 8 — the observed version matrix shows every transition the household actually went through
    Given matthew has run the ceremony that staged, promoted, and reverted a release on the commons channel
    And james's personal channel ran alongside that ceremony the entire time
    When an operator reads the household's observed version matrix
    Then the matrix shows matthew's, jessica's, and james's runtimes moving staging, then earned, then back, in that order
    And the matrix shows james's personal channel diverging compatibly the whole time
    And every row in the matrix is read from what each runtime itself reports, never asserted from the ceremony's own intent

  # Structural protection, not a veto — the constitutional posture named (spec §4).

  Scenario: jessica's runtime has no opt-out control for an individual upgrade, and what she gets instead is named, not abstract
    Given jessica's runtime is following release channel "runtime:coordinators:elohim:commons"
    When that channel's earned head changes
    Then jessica's runtime adopts it without asking her permission
    And jessica's runtime exposes no setting, flag, or control that lets her defer, decline, or veto that individual upgrade
    But jessica can still read the release's own explanation of what changed and why, and can reach escalation if her intimate context was mishandled

  # Negative control: the compatibility envelope is floor-protected, not a courtesy.

  Scenario: a release that breaks the compatibility envelope is refused by every peer's own verification, not merely discouraged
    Given a release manifest was built for a different DNA lineage than the one the household actually runs (its declared per-role hashes do not match)
    When any household peer's runtime verifies that release locally
    Then no one's device is put at risk by the mismatch — that peer refuses the release outright, naming a typed reason
    And no household peer ever applies it, no matter which channel declared it earned
