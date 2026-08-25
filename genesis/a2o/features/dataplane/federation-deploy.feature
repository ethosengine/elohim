# RED-FIRST: fails until blobHash-pointer propagation lands; asserts uniform all-peer EPR
# resolution to kill the per-host stageSpaBlob crutch.
@e2e @dataplane @concern:federation-deploy @requires:multi-node @act:i
Feature: Federation deploy uniformity — landing EPR resolves on all federation doorways
  The landing EPR (elohim-host-landing) must resolve correctly on EVERY federation doorway,
  not just the deploy-time author peer. Two conditions must hold on each doorway: the root path
  returns HTTP 200 (not "App not found"), and the EPR content record carries a non-null blobHash.
  Today, alpha-A (the author peer) passes both; elohim.host (the alpha-b federation peer) fails
  both — blobHash is null on elohim.host so the EprRouter cannot serve the landing SPA.

  This feature is the acceptance gate that makes the per-host Jenkinsfile stageSpaBlob stage
  un-shippable: a uniform deploy MUST mean every doorway can independently resolve the landing
  EPR, not just the peer that ran the CI blob-upload step. Fixing this requires blobHash metadata
  propagation from the author peer to all federation peers (backlog: dataplane-peer-fallback-and-
  blob-replication, item D5).

  Live state observed 2026-06-29:
    alpha-A  GET /                                          → HTTP 200  SPA bundle served (passing)
    alpha-A  /db/content/elohim-host-landing.blobHash      = sha256-1c3451873f57… (non-null, passing)
    elohim.host  GET /                                     → HTTP 404  {"error":"App not found: elohim-host-landing"}
    elohim.host  /db/content/elohim-host-landing.blobHash  = null  (the gap — causes App-not-found)

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: alpha-A fully resolves the landing EPR on both conditions (baseline — green)
    # Green baseline. alpha-A is the deploy-time author peer. The Jenkinsfile stageSpaBlob
    # stage wrote the non-null blobHash into the EPR content record and the SPA blob was
    # uploaded locally, so the EprRouter resolves the landing app correctly. Both assertions
    # pass today and confirm the author-peer surface is healthy.
    Then resolving "/" on peer "alpha-A" does NOT return App-not-found
    And EPR "elohim-host-landing" blobHash is non-null on peer "alpha-A"

  Scenario: elohim.host fully resolves the landing EPR on both conditions (RED — uniform deploy gap)
    # RED-FIRST. elohim.host is the alpha-b federation peer. The EPR content record on
    # elohim.host has blobHash: null — the blob metadata was never written on the peer even
    # though the blob bytes were placed there at deploy time by the per-host stageSpaBlob CI
    # crutch. Because blobHash is null, the EprRouter on elohim.host cannot find the blob to
    # serve and returns HTTP 404 {"error":"App not found: elohim-host-landing"}.
    #
    # This scenario FAILS today with:
    #   AssertionError: Resolving "/" on elohim.host returned 404 — route may not be registered
    #   (the 404 check fires before the body-text check; both conditions fail independently)
    #   AssertionError: EPR "elohim-host-landing" on elohim.host:
    #     blobHash is null — blob not yet attached
    #
    # When this scenario passes, blobHash metadata has propagated to the federation peer so
    # elohim.host can independently serve the landing app — and the per-host stageSpaBlob CI
    # stage is no longer needed to keep elohim.host alive.
    Then resolving "/" on peer "elohim.host" does NOT return App-not-found
    And EPR "elohim-host-landing" blobHash is non-null on peer "elohim.host"

  @wip @concern:federation-deploy
  Scenario: the deploy authority can stage bytes on a NON-author federation doorway
    # THE MISSING PRECONDITION NODE. The two scenarios above assert the END of the chain —
    # elohim.host resolves the landing EPR. They are silent on the step immediately before it:
    # whether the deploy pipeline is ALLOWED to put bytes on a doorway it did not author from.
    # It was not. From app #1672 every `seed elohim.host` leg answered 403, so the two
    # assertions above could not have gone green no matter how well blobHash propagated —
    # the bytes never arrived.
    #
    # The cause was a conflation, not a credential: `require_seed_authority` asked "is this
    # caller MY admin?" while one pipeline drives several doorways whose own admin identities
    # are deliberately distinct. It was masked for months because alpha-b.yaml set
    # DEV_MODE:"true" expressly to bypass this gate ("so the CI admin key lands the SPA blob
    # without a per-host credential. Remove when doorway-B federation auth hardens"), until the
    # loopback conjunct (62b658784) correctly closed that bypass and left no seed path at all.
    #
    # The fleet seed authority (API_KEY_SEED) answers the right question — "may this caller
    # seed?" — and is scoped to the seed/admin-cache routes only, so it can never stand in for
    # an operator identity. @wip until a fleet build carries the gate, matching the byte-route
    # gate scenario's precedent (34ac23125).
    Given peer "elohim.host" at "elohim.host"
    When the deploy authority stages a blob on peer "elohim.host"
    Then staging the blob on peer "elohim.host" is accepted
    # ANTI-REGRESSION: this must NOT be satisfiable by reopening the hole the gate closed.
    # A credential-free remote caller stays refused; a green here with this line failing means
    # the fix was a bypass, not an authority.
    And staging a blob on peer "elohim.host" with NO credential is refused
