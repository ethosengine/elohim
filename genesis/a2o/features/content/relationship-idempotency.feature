@e2e @content @relationships @idempotency
Feature: Relationship import converges under bidirectional authorship

  Executable contract for the account-import idempotency guarantee. The
  underlying storage-level behavior is also covered by the Rust integration
  test at elohim/elohim-storage/tests/account_import_idempotency.rs — these
  scenarios are @wip pending an a2o-level live-storage fixture.

  @wip
  Scenario: A spouse relationship authored by both parties is created once
    Given Adam's account package declares spouse relationship with Eve
    And Eve's account package declares spouse relationship with Adam
    When both packages are imported in sequence
    Then exactly one human_relationships row exists for the pair
    And the second import reports relationshipsSkipped=1

  @wip
  Scenario: Re-importing an account package does not error
    Given Adam's account package has been imported successfully
    When Adam's account package is imported a second time
    Then the import exits successfully
    And relationshipsCreated equals 0
    And relationshipsSkipped equals the number of relationships in the package

  @wip @regression
  Scenario: Adam-Eve UNIQUE constraint does not fail the seed
    # Regression guard for the 2026-04-17 seed pipeline failure where
    # Adam's package created (adam, eve, spouse) and subsequent Eve
    # imports crashed with UNIQUE constraint violation on the
    # human_relationships(h_app_id, party_a, party_b, type) index.
    Given a clean database
    When Adam's account package is imported
    And Eve's account package is imported
    Then both imports exit successfully
    And no errors mention "UNIQUE constraint"
