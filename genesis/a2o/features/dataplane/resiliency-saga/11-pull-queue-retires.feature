# Chapter 11 of the resiliency-saga: the pull queue can finish.
#
# What a "pin" is, in human terms: a standing request that this node hold a
# specific piece of content (a `head_ref` — the content id a pin names,
# `epr:`-prefixed) for the people who depend on it. Content here is
# `elohim-host-landing`, the page a visitor first lands on. matthew's device
# (alpha-A) is the one that authored it; jessica, a second household member,
# pins the same content on her own node so her copy survives even if
# matthew's device goes offline. While a node is stuck unable to satisfy a
# pin, the person who asked for it experiences it as silence: nothing in the
# app tells her the request failed, or that it never will. `pull.caughtUp` is
# the substrate's answer to "did everything this node was asked to hold
# actually arrive?", and the provide loop reads it to decide which head_refs
# this node may offer to others (a "peer's inventory" is the set of head_refs
# a peer will tell others it can supply). If it can never go true, the node
# never graduates to providing, forever, silently — it quietly stops being a
# co-steward for anyone, and no one is told.
#
# Why alpha-A is this chapter's live subject: it is the device that actually
# carried the regression below, so a passing run here means the cure held on
# the node that broke, not merely on a node that never had the problem. That
# said, the assertion below is TIME-BOUND evidence, not a permanent
# discriminator: once alpha-A's specific 70 failed pins are cleared (whether
# by the cure working or by re-provisioning), `caughtUp is true` alone no
# longer proves a cure fired — it would read identically on a node that was
# always healthy. A permanent discriminator (e.g. "no pin ever sits in a
# terminal failed state") would need a new assertion step this chapter does
# not add — named here as follow-up work, not silently assumed away.
#
# A pin's status is one of: `active` (still being pursued), `retired` (an
# unsatisfiable want, set aside by the cure below), or terminal `failed`
# (the pre-cure regression state this chapter proves no longer persists).
#
# jessica is a fixture household human (same convention as the fixture
# humans used elsewhere in this suite): her node is NOT one of the live
# alpha-lettered peers this chapter's live scenarios observe, and it is
# instantiated directly by the two @wip scenarios below, not by anything
# in the live Background. Her pin is the motivating human case for why a
# stuck queue matters; only a fixture lane can hold a pin of hers
# mid-exhaustion long enough to assert against it — this live lane cannot.
#
# Fabric topology this chapter's Background assumes: alpha, the 6-peer
# household+multi-tenant fabric documented in resiliency-saga/README.md, of
# which alpha-A is one member. This chapter's Gherkin body needs no other
# chapter's steps to run, though it continues the same fabric and cast as its
# siblings (chapters 1-10).
#
# What this CI lane can and cannot prove: this lane observes a LIVE, already-
# running node (alpha-A) over HTTP. It cannot inject a fault (it cannot make a
# peer stop advertising a head_ref, or force a retry budget to exhaust) — it
# can only read the node's current, real state. So the scenarios below are
# titled for what they OBSERVE, not for a fault they cause: a healthy node
# reaching caught-up, and its retirement accounting being published. The full
# cause-and-effect chain (an unsatisfiable pin, its retry budget exhausting,
# and the resulting retirement) is proof-carried by the two @wip fixture-lane
# scenarios below, which state that chain explicitly because a fixture lane,
# unlike this live lane, can construct it.
#
# Regression note (observed 2026-07-30/31, 12h+ sustained): alpha-A ran 73
# total / 3 fetched / 70 failed, caughtUp false, the whole time. Acquisition
# pins whose bytes NO connected peer could supply exhausted their retry budget
# and then sat in the tracker forever — the ONLY code path in the whole
# runtime that ever flipped a pin's status was the HTTP DELETE route (a
# person un-pinning by hand). Nothing the substrate did on its own could ever
# retire an unsatisfiable want, so `fetched < total` was pinned permanently
# and the queue could not close.
#
# The cure retires such a pin (status `retired`) so the queue can drain. Two
# things make that a HOLD rather than a quiet abandonment:
#
#   1. Retirement is COUNTED, never silent — `elohim_acquisition_pins_retired`
#      is a live gauge, materialised at boot so an absent series would mean
#      the emitter never ran. The metric step used below treats an
#      UNREACHABLE series as a failure and a reachable-but-zero series as a
#      pass, so `>= 0` reads as "prove this is being published," not "prove
#      some positive count" — the number itself is not the point. Because the
#      gauge legitimately returns TOWARD 0 as re-admission succeeds, this
#      chapter deliberately does NOT assert `>= 1` — that would be a future
#      false-red the moment re-admission does its job.
#   2. Retirement is REVERSIBLE — the pin row survives, and it is re-admitted
#      the moment a peer's inventory names its head_ref again (primary arm),
#      or after a 6h cooldown (backstop — a second, still-unscenario'd arm of
#      the same guarantee; see the station below the re-admission scenario).
#      Both directions are published as distinct series
#      (`elohim_acquisition_pin_retirements_total` with `reason` = exhausted |
#      readmitted), so an operator can always tell "the queue is caught up
#      because everything arrived" apart from "the queue is caught up because
#      unsatisfiable wants were set aside" — a caught-up queue can never be
#      mistaken for a complete one.
#
# This chapter's two live scenarios prove those two series are PUBLISHED
# (reachable, distinct); they do not themselves observe a pin transitioning
# either direction — that causal proof is the @wip scenarios below. Retirement
# only fires once every CONNECTED peer has been asked: the per-item retry
# budget is sized from the live peer count (`max(3, connected peers)`),
# because the dispatch rotation walks a different provider per retry. Three
# retries on a 6-peer fabric would have retired pins after probing half of it
# — a false negative, not a bounded one. This sizing rule is a design
# parameter this live lane does not observe directly; it becomes assertable
# in the @wip scenarios below.
#
# Residue, stated plainly (two open gaps this chapter does not close):
#   - Provide-loop consequence: this chapter's proof stops at
#     `pull.caughtUp=true` — the INPUT the provide loop reads. No scenario
#     here (or elsewhere in the saga today) observes the actual consequence —
#     that this node subsequently offers head_refs to other peers as a
#     provider.
#   - Person-facing notice: the cure makes retirement visible to an OPERATOR
#     (a published metric series). It does not tell jessica anything — her
#     silence, named at the top of this file as the human harm, is unchanged
#     by this cure. Closing that gap (a person-facing notification of a
#     retired or re-admitted pin) is not attempted here.
@e2e @dataplane @concern:saga-11-pull-queue-retires @act:i
Feature: Chapter 11 — the pull queue can finish
  A want no peer can satisfy must not hold the pull queue open forever — left
  unfixed, this node would silently stop co-stewarding content the household
  depends on. This chapter proves the household's own ask — elohim-host-landing,
  the pin alpha-A actually holds — has flowed through the pull queue to
  caught-up within a bounded window, and that the two retirement-accounting
  series (a count and a labeled reason total) are published rather than
  absent — visible to an OPERATOR today, so "everything arrived" can be told
  apart from "unsatisfiable wants were set aside." (A person-facing notice is
  a separate, unclosed gap — see Residue in the header comment above.) The two @wip scenarios below
  NAME the causal proof this live lane cannot construct — a pin actually
  exhausting its budget, retiring, and being counted — but do not yet run
  one; they are blocked on a fixture that does not exist yet (see their
  station comments).

  Background:
    Given peer "alpha-A" at "alpha-A"

  # Stage split (2026-08-16, operator directive): this scenario previously
  # asserted alpha-A's WHOLE-NODE `/p2p/status pull.caughtUp` — every pin on
  # the node, not any one of them. Whole-node pull.caughtUp is a stage-1
  # bootstrap-quiesce predicate, gated pre-suite by
  # scripts/ci/fleet-quiesce-gate.sh (which bounded-waits out post-deploy
  # churn on exactly this field before any measurement run is allowed); a
  # regression there should red the quiesce gate, not this saga. This chapter
  # instead proves the FLOW its own story is about: the specific pin
  # (`epr:elohim-host-landing`, the content this household's story tracks
  # throughout the saga — see 05-co-steward-agreement.feature, which proves
  # the pin exists) reaches caught-up within a bounded window, using the same
  # per-item polling step chapter 5 already uses for the same content id.
  Scenario: the household's pin for elohim-host-landing reaches caught-up within a bounded window
    Then within 60 seconds doorway "alpha-A" reports the pull for "elohim-host-landing" is caught up
    And metric "elohim_acquisition_pins_retired" on peer "alpha-A" >= 0

  Scenario: both retirement reasons are published as distinct series
    Then labeled metric "elohim_acquisition_pin_retirements_total" with label "reason" "exhausted" on peer "alpha-A" >= 0
    And labeled metric "elohim_acquisition_pin_retirements_total" with label "reason" "readmitted" on peer "alpha-A" >= 0

  # Station (minted 2026-07-31, blind-reader revision): the missing node this
  # live lane cannot supply — the full cause-and-effect chain from an
  # unsatisfiable want to a retired, counted pin. The two scenarios above can
  # only read alpha-A's already-settled state; neither one CAUSES a pin to
  # become unsatisfiable or WATCHES its retry budget exhaust. This scenario
  # names that chain explicitly, mirroring the resiliency-saga's station
  # convention (see chapter 6's ghost-witness stations): a chain / between /
  # missing node / current state, minted so the next sprint measures the gap
  # directly instead of re-deriving it from Loki archaeology (the
  # after-the-fact log-reading this counter-wiring replaces).
  #
  # This scenario's Given explicitly instantiates jessica's node (a fixture
  # peer), NOT alpha-A — the shared Background above still runs first
  # (Gherkin always runs the Background before every scenario), the same way
  # chapter 6's cross-node scenarios layer a second `Given peer` on top of the
  # shared alpha-A one; jessica's node, named in this scenario's own Given, is
  # what the Then steps actually check against.
  #
  #   chain: saga-11-pull-queue-retires
  #   between: a pin admitted (jessica asks her node to hold
  #     elohim-host-landing) -> the queue reads caught-up (finish line above)
  #   missing node: the retry-budget exhaustion that turns "still trying" into
  #     "retired" — unobservable from the live lane because alpha-A's real
  #     pins are either already satisfied or already retired by the time this
  #     scenario runs; only a fixture lane can hold a pin mid-exhaustion long
  #     enough to assert against it.
  #   current state: @wip — needs a household-scale acquisition fixture (an
  #     injectable peer set that can be told to never advertise a given
  #     head_ref), the same fixture gap acquisition-pins.feature's sibling
  #     retry-rotation and in-flight-budget scenarios are already @wip behind
  #     (steps/delivery/acquisition-pins.steps.ts). Prefers those existing
  #     step wordings so it is runnable the day that fixture lands. The fabric
  #     steps below are the PARAMETERIZED form of the header's worked example
  #     (line ~96): the retry budget is peer-sized (`max(3, connected peers)`),
  #     so the expected probe count derives from whatever fabric is REALLY
  #     running — the household mesh reports 2 connected peers; alpha's
  #     worked-example 6 would read as 5 from any one member's own view — and
  #     the assertion's teeth (budget covers every connected peer; it exhausts;
  #     the pin retires) hold at any fabric size >= 1.
  @requires:household-nodes @requires:owned-substrate
  Scenario: an unsatisfiable pin retires once the peer-sized retry budget exhausts
    Given jessica's node, a fixture peer with no other outstanding pins
    And an acquisition fabric of at least 1 connected peer
    And jessica's node holds a pin for "epr:elohim-host-landing" that no connected peer advertises
    When the peer-sized retry budget exhausts
    Then the item was probed on every peer the peer-sized retry budget names
    And the pin's status becomes "retired"
    And the retirement series for reason "exhausted" increments
    And jessica's node reports pull.caughtUp as true

  # Station (minted 2026-07-31, blind-reader revision): reversibility is the
  # safeguard that keeps retirement from being a quiet abandonment (see the
  # header's point 2 above) — the header argues it at length, and before this
  # scenario nothing demonstrated it. Same fixture gap as the scenario above.
  #
  #   chain: saga-11-pull-queue-retires
  #   between: a pin retired (scenario above) -> the same want satisfied later
  #     without a person re-asking for it
  #   missing node: re-admission firing when a peer's inventory names the
  #     retired head_ref again — the primary arm proved here. The 6h-cooldown
  #     backstop is a second, still-unscenario'd arm of the same guarantee.
  #   current state: @wip — same household-scale acquisition fixture as above.
  @wip @requires:household-nodes
  Scenario: a retired pin is re-admitted once a peer's inventory names its head_ref again
    Given jessica's node, a fixture peer, has a pin for "epr:elohim-host-landing" in status "retired"
    When a connected peer's inventory names "epr:elohim-host-landing" again
    Then the pin's status becomes "active"
    And the retirement series for reason "readmitted" increments
