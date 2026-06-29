# RED-FIRST: fails until dataplane-peer-fallback-and-blob-replication lands;
# this IS the acceptance gate.
@e2e @dataplane @concern:blob-replication @requires:multi-node
Feature: Blob replication — EPR blobHash metadata propagation to federation peer
  The landing EPR (elohim-host-landing) has a non-null blobHash on alpha-A (the
  deploy-time author peer): the SPA bundle was zipped, uploaded, and the EPR content
  record updated with the sha256 hash by the Jenkinsfile stageSpaBlob stage. On
  elohim.host (the alpha-b federation peer), the same EPR record shows blobHash: null
  — the blob bytes arrived at deploy time but the EPR DB record was never updated to
  reflect the hash. The doorway EprRouter cannot serve the landing app without a
  non-null blobHash, so root resolution on elohim.host returns "App not found"
  (see also epr-projection-fallback.feature).

  Live state observed 2026-06-29:
    alpha-A  /db/content/elohim-host-landing.blobHash  = sha256-1c3451873f5754929691ffd3310958b22beb53f4f15ec38e3b4ca21e8476d4f2  (non-null)
    elohim.host /db/content/elohim-host-landing.blobHash = null  (the gap — causes App-not-found)

  Fix target: dataplane-peer-fallback-and-blob-replication (backlog item D5). When this
  concern passes, the EPR blobHash has propagated from the author peer to the federation
  peer so elohim.host can resolve the landing app directly.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: alpha-A has a non-null blobHash for the landing EPR (baseline)
    # Green baseline. alpha-A is the deploy-time author peer. The Jenkinsfile
    # stageSpaBlob stage wrote the non-null blobHash into the content record at
    # deploy time. This scenario passes today and confirms the source is correct.
    Then EPR "elohim-host-landing" blobHash is non-null on peer "alpha-A"

  Scenario: elohim.host has a non-null blobHash for the landing EPR (RED — the gap)
    # RED-FIRST. elohim.host is the alpha-b federation peer. The EPR content record
    # on elohim.host has blobHash: null — the blob metadata was never written on the
    # peer even though the blob bytes were placed there at deploy time. This scenario
    # FAILS today with:
    #   AssertionError: EPR "elohim-host-landing" on elohim.host:
    #     blobHash is null — blob not yet attached
    # When it passes, EPR metadata has replicated and elohim.host can serve the landing app.
    Then EPR "elohim-host-landing" blobHash is non-null on peer "elohim.host"
