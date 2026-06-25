@e2e @content @epr @epr-content-facing @requires:doorway @requires:seeded-content
Feature: EPR content-perspective facing — the /raw neighborhood inspector
  As a developer (and as the content's own first-person perspective)
  I want an atom's raw leg-neighborhood — its coupling legs, how many atoms it
  reaches, and which atoms couple back to it
  So that the content's place in the dataplane is legible from one inspector
  instead of reconstructed from five different surfaces.

  # The /raw inspector is the EPR-content facing's fold over the materialized
  # leg-neighborhood (knowledge · value · governance), the inspector sibling of
  # the navigational /nav-context projection. It is a Category-C read-only
  # projection over existing epr_coupling rows — zero new DHT entry types.
  #
  # Charter: genesis/docs/superpowers/specs/2026-06-19-epr-content-perspective-facing-lens-design.md §6 Slice 2
  # Framework: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md §11
  #
  # Household floor: reads the seeded landing atom on matthew's storage via the
  # doorway projection — no cross-node discovery, so no @requires:shem.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: A seeded atom exposes its folded leg-neighborhood
    # The hosted landing atom is a Manifest kind — it carries a governance
    # coupling leg by construction, so its inspector must report at least one
    # forward neighbor and a populated governance leg.
    When I fetch the EPR raw inspector for atom "elohim-host-landing"
    Then the response status is 200
    And the raw inspector echoes a content-addressed cid
    And the raw inspector reports a governance coupling leg
    And the raw inspector reports a neighborhood degree of at least 1
    And the raw inspector lists the atoms that couple back to it

  Scenario: An unknown atom yields no inspector
    # The inspector is a projection, not a fabricator — an atom that was never
    # seeded resolves to 404, never an empty-but-200 shell.
    When I fetch the EPR raw inspector for atom "no-such-atom-deadbeefdeadbeef"
    Then the response status is 404
