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
  # Success paths
  # ─────────────────────────────────────────────────────────

  Scenario: Approve a pending recovery
    When I navigate to /account/security
    And I click "Approve" on the pending recovery card
    Then a RevocationVote entry is committed with decision "approve"
    And the pending recovery card disappears from my view

  Scenario: Reject a pending recovery
    When I navigate to /account/security
    And I click "Reject" on the pending recovery card
    Then a RevocationVote entry is committed with decision "reject"
