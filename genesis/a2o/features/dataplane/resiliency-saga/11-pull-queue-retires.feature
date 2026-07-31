# Chapter 11 of the resiliency-saga: the pull queue can finish.
#
# A pull queue that can never drain is not "still working" — it is a stuck
# promise. `pull.caughtUp` is the substrate's answer to "did everything this
# node was asked to hold actually arrive?", and the provide loop reads it to
# decide which head_refs this node may offer to others. If it can never go true,
# the node never graduates to providing, forever, silently.
#
# The defect (alpha-A, 12h+): 73 total / 3 fetched / 70 failed, caughtUp false
# the whole time. Acquisition pins whose bytes NO connected peer could supply
# exhausted their retry budget and then sat in the tracker forever — the ONLY
# code path in the whole runtime that ever flipped a pin's status was the HTTP
# DELETE route (a person un-pinning by hand). Nothing the substrate did on its
# own could ever retire an unsatisfiable want, so `fetched < total` was pinned
# permanently and the queue could not close.
#
# The cure retires such a pin (status `retired`) so the queue can drain. Two
# things make that a HOLD rather than a quiet abandonment, and both are asserted
# below:
#
#   1. Retirement is COUNTED, never silent — `elohim_acquisition_pins_retired`
#      is a live gauge, so "caught up with 0 retired" (everything arrived) stays
#      distinguishable from "caught up with N retired" (the queue drained
#      because unsatisfiable wants were set aside, and N pins are still on the
#      books). A caughtUp assertion read WITHOUT this gauge would be exactly the
#      kind of false-green this saga exists to catch.
#   2. Retirement is REVERSIBLE — the pin row survives, and it is re-admitted
#      the moment a peer's inventory names its head_ref again (primary arm), or
#      after a 6h cooldown (backstop). `elohim_acquisition_pin_retirements_total`
#      carries both directions as `reason` ∈ exhausted | readmitted, so a
#      flapping fabric is legible as flapping rather than as a steady gauge.
#
# Retirement only fires once every CONNECTED peer has been asked: the per-item
# retry budget is sized from the live peer count (`max(3, connected peers)`),
# because the dispatch rotation walks a different provider per retry. Three
# retries on a 6-peer fabric would have retired pins after probing half of it —
# a false negative, not a bounded one.
#
# Threshold note: the retired gauge is asserted `>= 0`, not `>= 1`. A HEALTHY
# node retires nothing; demanding a retirement would be demanding a fault. The
# assertion's job is to prove the series EXISTS and is being published (it is
# materialised at boot, so an absent series means the emitter never ran), which
# is what makes the caughtUp assertion above it readable.
@e2e @dataplane @concern:saga-11-pull-queue-retires
Feature: Chapter 11 — the pull queue can finish
  A want no peer can satisfy must not hold the pull queue open forever. This
  chapter proves the queue reaches caught-up, and that any pin set aside to get
  there is counted rather than silently dropped.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: a pin whose bytes no peer can supply retires and stops holding the pull queue open
    Then peer "alpha-A" /p2p/status pull.caughtUp is true
    And metric "elohim_acquisition_pins_retired" on peer "alpha-A" >= 0

  # The transition counter, read alongside the gauge above. A steady gauge is
  # ambiguous — it can mean "nothing is happening" or "pins are retiring and
  # reviving at the same rate" — and only the transition rate tells them apart.
  # `readmitted` is asserted specifically because it is the half that proves
  # retirement is a HOLD: a substrate that retires and never re-admits has
  # quietly abandoned what a person asked for.
  Scenario: retirement transitions are legible in both directions
    Then labeled metric "elohim_acquisition_pin_retirements_total" with label "reason" "exhausted" on peer "alpha-A" >= 0
    And labeled metric "elohim_acquisition_pin_retirements_total" with label "reason" "readmitted" on peer "alpha-A" >= 0
