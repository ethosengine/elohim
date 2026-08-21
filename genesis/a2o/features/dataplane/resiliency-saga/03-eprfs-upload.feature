# Chapter 3 of the resiliency-saga. The upload itself happens at deploy time:
# the root pipeline's stageSpaBlob step zips the elohim-app browser bundle,
# uploads it as a blob, and declares its hash on the content row — acting as
# matthew, the deploy-time author persona whose peer is "alpha-A". This chapter
# does not repeat that upload; the documentary Given names it as the prior
# action, and the Then steps prove it MATERIALIZED on matthew's own peer.
#
# Both Then steps are reused verbatim from steps/dataplane.steps.ts; the same
# pair is used by federation-deploy.feature and served-projected-head.feature
# as the alpha-A (author-peer) baseline. Their step text is a shared contract —
# rephrasing it (e.g. "blobHash" → "has content bytes attached") is a
# cross-feature step-definition migration, not a local edit.
#
# Probe mechanics: the content probe behind both steps rides the bounded
# catching-up admission shed (503 + {"status":"catching-up"}, retryAfter
# honored, 90s cap). A peer still shedding after the bound fails honestly with
# the shed body in the message.
@e2e @dataplane @concern:saga-03-eprfs-upload @act:i
Feature: Chapter 3 — matthew's deploy-time upload of elohim-host-landing materialized in his eprfs
  matthew is the author persona of peer "alpha-A" (his node in the alpha
  deployment; the Background binds every probe to it). His eprfs — the peer's
  content-addressed store of EPRs (EntityPortalReference, the protocol's
  addressable content record) — received "elohim-host-landing", the browser
  application bundle users load at the host landing page, at deploy time.
  This chapter proves that upload fully materialized on his own peer, in two
  halves: the served head (a head is the hash naming the current version of a
  content record) that his doorway — the gateway that serves this peer's
  content to browsers — actually serves matches the declared head on the
  content row — guarding against a stale host that answers 200 while serving an
  old materialization — and content bytes are attached to the record, because
  a bare content record with no blob is a stub, not an upload. Materialization
  is the whole concern: a doorway momentarily refusing requests while it
  catches up is NOT a failed upload — that admission plane is proven by its
  own concern (doorway-catching-up-page), and these probes wait out the
  bounded catch-up window rather than red on it. Chapter 1 proved matthew's
  device is awake and chapter 2 that his household formed; the co-steward and
  convergence chapters that follow build on this baseline.

  Background:
    Given peer "alpha-A" at "alpha-A"

  Scenario: elohim-host-landing resolves with a matching served head and an attached blob
    Given EPR "elohim-host-landing" was uploaded to peer "alpha-A" at deploy time
    Then the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"
    And EPR "elohim-host-landing" blobHash is non-null on peer "alpha-A"
