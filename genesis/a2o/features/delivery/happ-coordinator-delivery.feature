@e2e @delivery @happ-delivery @act:i
Feature: hApp coordinator delivery — a shipped zome fix reaches running conductors
  As a steward whose conductor holds years of household state
  I want fixes the community ships to reach my running conductor
  without my agent identity being destroyed and re-minted
  So that the network heals by upgrade, not by wipe

  A hApp bundles one or more DNAs. Each DNA has integrity zomes (which
  define the DNA hash and the validation rules peers hold each other to)
  and coordinator zomes (the application behavior). A conductor runs
  installed hApps under an agent key — the persistent identity whose DHT
  state accumulates for years. Because the DNA hash covers integrity zomes
  and modifiers ONLY, a coordinator-only fix does not move the hash: any
  hash-granular staleness check is structurally blind to it. That blindness
  is permanent Holochain behavior — the first scenario documents the
  constraint (it always passes; it is not a guard that a fix could regress),
  and the following scenarios prove the cure built around it: the
  update_coordinators hot-swap, which delivers new behavior with no re-key
  and no DHT churn.

  # Evidence anchor (2026-06-11, EPR durability arc): the attestation-lockout
  # fix (2f02879d) changed only the infrastructure COORDINATOR zome. A
  # Holochain DNA hash covers integrity zomes + modifiers only, so the new
  # bundle's DNA hashes were byte-identical to the installed app — the
  # DNA-hash drift check correctly said "no drift" and conductors served the
  # OLD coordinator wasm from their PVC for hours while every pipeline stage
  # (DNA build, harbor push, pod fetch) was provably healthy.
  # Constraint boundary: integrity-zome change → DNA hash moves → gated
  # reinstall/migration path; coordinator-zome change → DNA hash does NOT
  # move → update_coordinators hot-swap path (no re-key, no DHT churn).
  # Fix: elohim-storage happ_manager::sync_coordinators.
  # Plan: genesis/docs/superpowers/plans/2026-06-11-epr-durability-arc-workstreams-bcdef-pickup.md

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # Both lifecycle scenarios observe happ_manager::ensure_happ_installed —
  # the ONLY path that decides and logs coordinator drift
  # (elohim/elohim-storage/src/main.rs, the --embedded-conductor boot branch;
  # sync_coordinators in src/happ_manager.rs). Act I installs the hApp with
  # `hc sandbox generate` against an external conductor and launches storage
  # without --embedded-conductor, so that decision never runs on the household
  # lane — cluster-state.act1-household.yaml declares `embedded-conductor:
  # available: false` for exactly this. The tag makes the hold substrate-true
  # (HELD, not pending) until an embedded-conductor mesh mode exists. A second,
  # independent gap stays in the steps: no fixture authors a SECOND bundle whose
  # coordinator wasm alone differs (DNA-build tooling, not step glue).
  #
  # Constraint documentation, not a regression guard: this scenario proves
  # the permanent structural blindness the feature exists to work around.
  @requires:owned-substrate @requires:embedded-conductor
  Scenario: Without coordinator drift detection, a coordinator-only fix never deploys
    Given a conductor with the elohim hApp installed from an earlier bundle
    And a new bundle whose infrastructure coordinator zome changed but whose integrity zomes did not
    When the node restarts and the installer compares per-role DNA hashes only
    Then the drift check reports no drift even though the coordinator wasm differs
    And the conductor keeps serving the old coordinator behavior
    # Constraint: DNA hash = integrity zomes + modifiers ONLY. Any
    # DNA-hash-granular staleness check is structurally blind to
    # coordinator-only changes. uhCok… in errors is a WASM hash; uhC0k… a DNA hash.

  @requires:owned-substrate @requires:embedded-conductor
  Scenario: Coordinator drift is healed by hot-swap without re-keying the agent
    Given a conductor with the elohim hApp installed and an agent key holding DHT state
    And a new bundle whose infrastructure coordinator wasm hash differs from the installed cell's
    When the node starts with coordinator updates allowed
    Then the installer hot-swaps the coordinator zomes via update_coordinators
    And the agent key and cells are unchanged
    And subsequent zome calls run the new coordinator behavior
    # Operational parameters: gate ALLOW_COORDINATOR_UPDATE defaults to
    # ALLOW_DNA_REINSTALL (alpha=true, prod=false → detect-and-log-loudly).
    # Informs: prod can enable coordinator hot-swap WITHOUT enabling re-key
    # reinstalls — they are different upgrade classes.

  @requires:owned-substrate @requires:embedded-conductor
  Scenario: An operator can see which drift class the installer decided on
    Given a conductor that just completed its hApp lifecycle check
    When the operator reads the node's startup log
    Then the log states either no coordinator drift or the drifted roles and whether the swap was applied
    # Observability: the silent no-drift path was indistinguishable from
    # "check never ran" during the 2026-06-11 trace; the decision is now logged.
