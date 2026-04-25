@recovery-m5 @account-pillar @recovery-vote
Feature: Voting on recovery as an emergency contact
  As an emergency contact
  I want to approve or reject pending recovery requests
  So that the human's recovery can proceed under graduated authority

  Background:
    Given I am authenticated
    And I am an emergency contact for a human with a pending RecoveryRequest
    And the RecoveryRequest has fields proposedAuthorityKind and createdAt

  # ─────────────────────────────────────────────────────────
  # M5 reality — POST mutations return 503 until Phase-11 bridge
  # ─────────────────────────────────────────────────────────

  Scenario: Vote mutation returns 503 PHASE_11_PENDING in M5
    When I navigate to /account/security
    And I click "Approve" on the pending recovery card
    Then the POST to /api/v1/account/recovery/*/vote returns 503
    And the response body contains errorCode "PHASE_11_PENDING"
    And the pending recovery card remains visible with an informative error

  Scenario: Reject vote mutation returns 503 PHASE_11_PENDING in M5
    When I navigate to /account/security
    And I click "Reject" on the pending recovery card
    Then the POST to /api/v1/account/recovery/*/vote returns 503
    And the response body contains errorCode "PHASE_11_PENDING"

  # ─────────────────────────────────────────────────────────
  # Success paths — will work once the Phase-11 bridge exists
  # ─────────────────────────────────────────────────────────

  @phase11-pending
  Scenario: Approve a pending recovery
    When I navigate to /account/security
    And I click "Approve" on the pending recovery card
    Then a RevocationVote entry is committed with decision "approve"
    And the pending recovery card disappears from my view

  @phase11-pending
  Scenario: Reject a pending recovery
    When I navigate to /account/security
    And I click "Reject" on the pending recovery card
    Then a RevocationVote entry is committed with decision "reject"
