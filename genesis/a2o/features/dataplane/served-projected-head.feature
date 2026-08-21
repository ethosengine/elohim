# Track-4 T4-2 — the served-vs-declared propagation probe's a2o twin to
# scripts/ci/verify-projected-head.sh. CI proves this once per deploy, against
# the bundle it just authored; this feature proves it stands independently of
# any particular deploy, against whatever head is currently declared.
@e2e @dataplane @concern:served-projected-head @requires:multi-node @act:i
Feature: Served-vs-declared projected head propagation
  federation-deploy.feature and blob-replication.feature pin that a routed mount
  answers 200 and that a content row's blobHash is non-null on every federation
  peer. Neither proves the doorway PROCESS behind that mount has actually
  materialized the head its own content row declares — a stale-but-200 host
  passes both of those checks while still serving yesterday's SSR bundle. This
  is the gap the T4-1 health-surface attestation (servedBundleHeads[]) exists to
  close: GET /health/startup (falling back to /health) reports what a doorway
  has actually served, distinct from what /db/content/{slug} declares it should
  serve.

  Until T4-1 ships on a given peer, servedBundleHeads (or the entry for a given
  slug) is simply absent from its health surface. That reads as an honest SKIP,
  not a failure — the same forward-compatible semantics scripts/ci/verify-
  projected-head.sh uses, so this feature can be wired in before every doorway
  carries the attestation without turning green builds red.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: Every doorway serves the declared projected head
    # elohim-host-landing is the root landing EPR (browser + server bundles);
    # lamad-spa is the lamad pillar EPR. Both carry a serverBlobHash on their
    # content row once authorHeadOnce (Jenkinsfile) has authored a head for them.
    Then the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"
    And the served head for EPR "elohim-host-landing" matches the declared head on peer "elohim.host"
    And the served head for EPR "lamad-spa" matches the declared head on peer "alpha-A"
    And the served head for EPR "lamad-spa" matches the declared head on peer "elohim.host"
