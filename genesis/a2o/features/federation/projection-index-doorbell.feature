@e2e @federation @concern:doorway-failover @act:i @requires:owned-substrate
Feature: A doorway learns what its sibling now hosts within seconds, without anyone refreshing it
  Every doorway keeps a local registry of which sites its sibling doorways host, so it can
  send a visitor onward when it does not hold a site itself. Staging a site means telling a
  doorway "you host this path now"; withdrawing means telling it "you no longer do." That
  registry used to update only on a 60-second cycle: after a site was staged on one doorway,
  or withdrawn from it, a sibling could take up to 60 seconds to notice, and every visitor
  asking the sibling in between got the stale answer — an unreachable site read as missing, or
  a withdrawn one still offered. This matters for the same reason failover matters: a sibling
  that cannot promptly learn which doorway can currently serve a site is a sibling that cannot
  promptly stand in.

  This proof measures a doorbell that closes most of that gap. The moment a doorway's own
  hosted list changes — a site staged or withdrawn — it sends a short message to every sibling
  it knows about: not the change itself, only a marker (its digest, a short fingerprint of the
  doorway's whole current hosted list) meaning "my list is different now, go check for
  yourself." The sibling then asks that doorway directly for its published list, the same
  list anyone could ask for at any time, and refreshes its own registry to match — without
  waiting for its own next scheduled refresh (the 60-second cycle above, called the discovery
  cycle in this proof), and without an operator asking it to refresh. A sibling that never
  hears the doorbell — network trouble, or a deliberately silenced test double as below —
  still catches up on its own next discovery cycle; the doorbell only makes the common case
  fast.

  This implementer-facing acceptance proof measures on an owned household test mesh: two HTTP
  gateways, alpha and beta, run by the household operating the mesh (referred to below as "the
  household," the same actor who stages and withdraws sites in every scenario). Alpha is the
  doorway whose hosted list changes; beta is the sibling learning about it. Neither doorway
  hosts every site itself: when Jessica, an ordinary visitor, asks a doorway for a site it does
  not host locally, that doorway forwards ("relays") her request once to whichever sibling its
  registry says holds it, and returns the answer — this is what "forwarded" and "relay" mean
  below. Garden is one such site path Jessica asks for (not a DNS hostname), staged and
  withdrawn as a whole unit of hosting; a request to its bare path may be answered by a
  fallback page even when the site is absent, so the withdrawal scenario below asks for its
  published entry document (the specific file the site's root page answers with) to get an
  unambiguous absent-or-present reading. Every doorway registration and hosting contract is
  notarized once, in one shared registry the whole mesh can read (the Background below reads
  it directly); each doorway's own "local registry" this proof measures is that doorway's OWN
  projection of that shared registry, built by asking siblings for their published lists — the
  doorbell is what accelerates building that projection. Doorway registrations and hosting
  contracts reach each doorway's local registry asynchronously, as in every routing proof in
  this directory.

  The first two scenarios below — a newly hosted site becoming reachable, and a withdrawn one
  stopping — show what a working doorbell delivers. The third scenario is this feature's own
  control: with the doorbell deliberately silenced beforehand, the same "within seconds" claim
  must NOT come true quickly — it only comes true on the discovery cycle that already existed
  before this doorbell was added. Without that control, the first two scenarios would only be
  proving that beta's ordinary discovery cycle happened to land inside their 10-second window,
  not that the doorbell did anything at all. One control, staged only for the newly-hosted
  case, stands for both directions: staging and withdrawing both change alpha's digest and
  trigger the identical doorbell send, so proving the doorbell is the reason staging propagates
  fast is proving the same mechanism for withdrawal too.

  Runner prerequisites: E2E_DOORWAY_ALPHA and E2E_DOORWAY_B name the two gateways. The
  household lane owns the processes and fixture overrides it sets and clears them even after
  a failure.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_B"
    And both doorways can read the registry of doorway registrations and hosting contracts
    And both doorways are registered in the registry
    And Jessica is a visitor asking as an ordinary web client

  Scenario: A newly hosted site is reachable through the sibling before its next refresh
    Given the household stages the root "garden" as hosted by doorway "alpha" only, without waiting for propagation
    Then within 10 seconds doorway "beta" resolves "garden" to doorway "alpha" in its local registry
    And doorway "beta" logged a doorbell pull from doorway "alpha" naming the digest that holds "garden"
    When Jessica asks doorway "beta" for "garden"
    Then Jessica is served "garden"
    And doorway "beta" forwarded the request exactly once

  Scenario: A withdrawn site stops being relayed within seconds
    Given the household stages the root "garden" as hosted by doorway "alpha" only
    And doorway "beta" still resolves "garden" to doorway "alpha" in its local registry
    When the household withdraws the contract for "garden" on doorway "alpha"
    Then within 10 seconds doorway "beta" no longer resolves "garden" to any holder
    And doorway "beta" logged a doorbell pull from doorway "alpha"
    When Jessica asks doorway "beta" for "garden" at its published entry document
    Then Jessica received HTTP 404 for "garden"
    And doorway "beta" attempted no relay for that request

  # The control, standing for both directions (see the feature description). Beta is made
  # deaf to doorbells FIRST, before garden is ever staged, so no doorbell for THIS root can
  # ever reach it. Garden still becomes reachable through beta — just on beta's own next
  # discovery cycle, not within the doorbell's few-second window. 150 seconds of deafness
  # comfortably outlasts the 20-second margin the final check allows past one 60-second
  # discovery cycle, so the deafness cannot lift before that check gets its answer. This is
  # what tells the first scenario's speed apart from a lucky discovery-cycle tick landing
  # inside the same window by coincidence.
  Scenario: A lost doorbell is caught by the next refresh
    Given doorway "beta" is deaf to doorbells for 150 seconds
    And the household stages the root "garden" as hosted by doorway "alpha" only, without waiting for propagation
    Then after 10 seconds doorway "beta" still does not resolve "garden" to any holder
    And doorway "beta" resolves "garden" to doorway "alpha" in its local registry within one discovery cycle plus 20 seconds
