# RED-FIRST: fails until dataplane-peer-fallback-and-blob-replication lands;
# this IS the acceptance gate.
@e2e @dataplane @concern:blob-replication @requires:multi-node @act:i
Feature: Blob replication — EPR blobCid metadata propagation to federation peer
  The landing EPR (elohim-host-landing) has a non-null blobCid on alpha-A (the
  deploy-time author peer): the SPA bundle was zipped, uploaded, and the EPR content
  record updated with the byte CID by the Jenkinsfile stageSpaBlob stage. On
  elohim.host (the alpha-b federation peer), the same EPR record shows blobCid: null
  — the blob bytes arrived at deploy time but the EPR DB record was never updated to
  reflect the hash. The doorway EprRouter cannot serve the landing app without a
  non-null blobCid, so root resolution on elohim.host returns "App not found"
  (see also epr-projection-fallback.feature).

  Live state observed 2026-06-29:
    alpha-A  /db/content/elohim-host-landing.blobCid  = bafkr4ieh6qzwkrzzpyvcd63agqmheievxv7xpot6nrhfhh626mq2mn5tsi  (non-null)
    elohim.host /db/content/elohim-host-landing.blobCid = null  (the gap — causes App-not-found)

  Fix target: dataplane-peer-fallback-and-blob-replication (backlog item D5). When this
  concern passes, the EPR blobCid has propagated from the author peer to the federation
  peer so elohim.host can resolve the landing app directly.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: alpha-A has a non-null blobCid for the landing EPR (baseline)
    # Green baseline. alpha-A is the deploy-time author peer. The Jenkinsfile
    # stageSpaBlob stage wrote the non-null blobCid into the content record at
    # deploy time. This scenario passes today and confirms the source is correct.
    Then EPR "elohim-host-landing" blobCid is non-null on peer "alpha-A"

  Scenario: elohim.host has a non-null blobCid for the landing EPR (RED — the gap)
    # RED-FIRST. elohim.host is the alpha-b federation peer. The EPR content record
    # on elohim.host has blobCid: null — the blob metadata was never written on the
    # peer even though the blob bytes were placed there at deploy time. This scenario
    # FAILS today with:
    #   AssertionError: EPR "elohim-host-landing" on elohim.host:
    #     blobCid is null — blob not yet attached
    # When it passes, EPR metadata has replicated and elohim.host can serve the landing app.
    Then EPR "elohim-host-landing" blobCid is non-null on peer "elohim.host"

  # ── Harvested 2026-07-23 (shift/reach-vocab-slice2): replica targets track declared reach ──
  # Constraint discovered: distribution's ReachClass parsed declared-reach strings
  # with unwrap_or(Private) — after the vocabulary went schema-8, every declared
  # "trusted"/"familiar"/"commons" row silently fell to the Private 2-replica
  # floor. Vocabulary drift between a declared enum and its projection enum is
  # a RESILIENCE bug, not a naming bug: under-replication is invisible until
  # hosts churn. Fixed view-schema-first (schema JSON → Rust → contract test →
  # codegen); parse fallback stays conservative (unknown → Private/2).
  # Operational parameters (pinned replica ladder): private 2 · self 2 ·
  # intimate 4 · trusted 6 · familiar 8 · community 12 · public 14 · commons 16.
  # Informs: peer-diversity presets, storage capacity planning per corpus mix.
  # Open follow-up: pre-migration legacy "public" rows parse canonically to
  # public/14 until the one-time public→commons backfill runs (strand doc).

  @wip @regression
  Scenario: Declared reach maps to its replica target — no silent floor collapse
    Given content is declared at "trusted" reach
    When the distribution view computes its replica target
    Then the replica target is 6
    And a declared "commons" content's replica target is 16
    And an unrecognized reach string falls back to the conservative 2-replica private floor
