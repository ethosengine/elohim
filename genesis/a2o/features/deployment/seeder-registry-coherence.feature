@e2e @deployment @seeder @registry-coherence
Feature: Seeder respects the deployment registry
  As a protocol operator running the seed pipeline
  I want the seeder to import only accounts whose humans are deployed
  So that undeployed packages don't produce 502 errors, and the
  deployment registry remains the single source of truth for what
  exists on the cluster right now.

  Scenario: Seeder imports only deployed humans
    Given the deployment registry lists humans "adam, matthew, frank"
    And account packages exist for "adam, matthew, frank, charlie, eve"
    When the seeder runs against "https://doorway-alpha.elohim.host"
    Then the seeder attempts import for "adam, matthew, frank"
    And the seeder marks "charlie, eve" as staged
    And the seeder exits with status 0

  Scenario: Registry entry without a package is a warning
    Given the deployment registry lists human "ghost-human"
    And no account package exists for "ghost-human"
    When the seeder runs
    Then the seeder emits warning "registry references ghost-human, no package found"
    And the seeder exits with status 0

  @wip
  Scenario: Seeder is idempotent across reruns
    # Requires a live doorway + storage fixture; dry-run alone cannot
    # exercise the rerun path. Covered indirectly by the Rust integration
    # test in elohim/elohim-storage/tests/account_import_idempotency.rs.
    Given a successful seed run completed
    When the seeder runs a second time with unchanged packages
    Then all deployed humans report outcome "imported"
    And no package reports outcome "failed"

  Scenario: --deployed-humans flag overrides the registry file
    Given the deployment registry lists humans "adam, matthew"
    When the seeder runs with "--deployed-humans=human-adam-firstman"
    Then the seeder attempts import for "adam" only
