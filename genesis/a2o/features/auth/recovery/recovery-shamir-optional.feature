@recovery-shamir-optional @recovery-m4
Feature: Recovery succeeds with or without Shamir share custody
  As Matthew, whose key has been compromised
  I want to recover my account through people who know me
  So that the cryptographic-proof channel is icing, not foundation

  The substrate decides whether to use the Shamir transport based on
  whether a governance-action:shamir-custody-setup manifest exists for
  Matthew. The wisdom layer (the social-threshold quorum) is always the
  load-bearing path; Shamir augments it when configured but never gates
  it. The user never sees the choice and never sees raw key material.

  Background:
    Given Matthew has three emergency contacts: Jessica, Adam, and Abby
    And Matthew's required intimate-quorum threshold is 2

  @wip
  Scenario: Recovery succeeds without Shamir custody (Path A only)
    Given Matthew has NOT committed a governance-action:shamir-custody-setup
    When Matthew initiates a recovery request
    And Jessica and Adam each submit a recovery-approval attestation
    Then the recovery flow reaches Quorum
    And Matthew's key rotates successfully
    And no Shamir share request is dialled to any custodian
    And no attestation:shamir-reconstructed exists for Matthew

  @wip
  Scenario: Recovery succeeds with Shamir custody (Path A + Path B)
    Given Matthew has committed a governance-action:shamir-custody-setup
      naming Jessica, Adam, and Abby as custodians with threshold (m=2, n=3)
    And Jessica and Adam are online
    When Matthew initiates a recovery request
    And Jessica and Adam each submit a recovery-approval attestation
    Then the recovery flow reaches Quorum
    And Matthew's key rotates successfully
    And the substrate dials Jessica and Adam over /elohim/shamir-share/1.0.0
    And the ShareAssembler reconstructs Matthew's seed
    And an attestation:shamir-reconstructed exists for Matthew

  @wip
  Scenario: Recovery still succeeds when Shamir custodians are offline
    Given Matthew has committed a governance-action:shamir-custody-setup
      naming Jessica, Adam, and Abby as custodians with threshold (m=2, n=3)
    But Jessica, Adam, and Abby are all offline at recovery time
    When Matthew initiates a recovery request
    And the social-threshold attestations arrive from a separate quorum
    Then the recovery flow reaches Quorum
    And Matthew's key rotates successfully
    And no attestation:shamir-reconstructed exists for Matthew
    And Matthew is not informed of the Shamir attempt at all

  @wip
  Scenario: Recovery never asks Matthew to choose Path A or Path B
    When Matthew initiates a recovery request
    Then the recovery UI does not present a Shamir-vs-social toggle
    And Matthew is never shown raw seed bytes or share bytes
