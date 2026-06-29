# RED-FIRST: fails until dataplane-peer-fallback-and-blob-replication lands;
# this IS the acceptance gate.
@e2e @dataplane @concern:epr-projection-fallback @requires:multi-node
Feature: EPR projection fallback — root resolution on federation peer
  On elohim.host (the alpha-b federation peer), resolving "/" returns HTTP 404 with
  body {"error": "App not found: elohim-host-landing"} because the EPR content record
  has blobHash: null and the doorway EprRouter cannot find the blob to serve. The fix
  requires either (a) peer-fallback: the EprRouter detects null blobHash and proxies or
  redirects to a peer that has the blob so the SPA is served transparently; or (b) the
  blobHash propagation gap is closed (blob-replication concern) and the router can serve
  directly from the local blob store.

  Live state observed 2026-06-29:
    GET https://elohim.host/  → HTTP 404  {"error": "App not found: elohim-host-landing"}
    GET https://doorway-alpha.elohim.host/  → HTTP 200  SPA bundle served (working correctly)

  Fix target: dataplane-peer-fallback-and-blob-replication (backlog item D5). When this
  concern passes, elohim.host serves the landing SPA (peer-fallback path) OR returns a
  syncing status {"eprHead":..., "blob":{"state":"syncing"|"ready"}} rather than
  "App not found". Either outcome unblocks the user landing experience on elohim.host.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: alpha-A root resolves without App-not-found (baseline)
    # Green baseline. alpha-A is the deploy-time author peer and has a non-null blobHash,
    # so the EprRouter resolves the landing app correctly. This scenario passes today
    # and confirms the surface works on a peer with a healthy EPR record.
    Then resolving "/" on peer "alpha-A" does NOT return App-not-found

  Scenario: elohim.host root resolves without App-not-found (RED — the gap)
    # RED-FIRST. elohim.host returns HTTP 404 {"error":"App not found: elohim-host-landing"}
    # because the EPR content record has blobHash: null. This scenario FAILS today with:
    #   AssertionError: Resolving "/" on elohim.host returned 404
    #     — route may not be registered
    # (the 404 check fires before the body-text check; both would fail independently)
    # When it passes, elohim.host serves the SPA or a syncing status instead of App-not-found.
    Then resolving "/" on peer "elohim.host" does NOT return App-not-found
