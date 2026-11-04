@e2e @devflow @concern:acceptance-aware-reconciliation @requires:epr-cli @act:host
Feature: A ceremony reader can distinguish completed checks from accepted work
  As a human deciding what agents should work on next
  I want the native valueflow context to distinguish delivery evidence from an
  independent agent's acceptance after trying the intended experience
  So that the memory ceremony neither repeats completed work nor calls an
  unexamined experience finished.

  # The native epr CLI records promises, delivery reports and review notes in a
  # repository-local append-only ledger. Acceptance is a separate verdict by an
  # explicitly appointed actor, naming the exact delivery, review and observations.
  # These scenarios use a throwaway repository and simulated acceptance evidence.
  # They verify the accounting distinction; they do not accept a real product.
  Scenario: Passing checks and technical approval leave acceptance unestablished
    Given an isolated reconciliation repository with one promised experience and simulated acceptance evidence
    And the implementer delivered the experience and technical review approved it
    When the ceremony reader asks native context what remains
    Then the experience has reconciliation state "acceptance-unestablished"
    And the delivery and technical approval remain visible as evidence

  Scenario: An appointed independent agent records acceptance after exercising the experience
    Given an isolated reconciliation repository with one promised experience and simulated acceptance evidence
    And the implementer delivered the experience and technical review approved it
    And an independent agent is appointed to accept that exact promise
    When the appointed agent records its exercised experience and approves acceptance
    And the ceremony reader asks native context what remains
    Then the experience has reconciliation state "accepted"
    And the delivery and technical approval remain visible as evidence

  @regression
  Scenario: Changed observation evidence cannot retain a previously accepted conclusion
    Given an isolated reconciliation repository with one promised experience and simulated acceptance evidence
    And the implementer delivered the experience and technical review approved it
    And an independent agent is appointed to accept that exact promise
    And the appointed agent records its exercised experience and approves acceptance
    When the external observation artifact changes without editing the ledger
    And the ceremony reader asks native context what remains
    Then the experience has reconciliation state "revalidation-required"
    And the original acceptance record remains in the ledger

  # The observation artifact records the candidate Git tree and the checks that
  # unrelated staged, unresolved and dirty work survived the resolution.
  @concern:brit-native-daily-driver @requires:brit-cli
  Scenario: A staged conflict resolution needs separate acceptance bound to its evidence
    Given an isolated promise to reconcile worker settings while preserving unrelated work
    When the developer stages a chosen worker resolution through Brit
    And the ceremony reader asks native context what remains
    Then the experience has reconciliation state "acceptance-unestablished"
    Given the implementer delivered the experience and technical review approved it
    When the ceremony reader asks native context what remains
    Then the experience has reconciliation state "acceptance-unestablished"
    Given an independent agent is appointed to accept that exact promise
    When the appointed agent records its exercised experience and approves acceptance
    And the ceremony reader asks native context what remains
    Then the experience has reconciliation state "accepted"
    When the external observation artifact changes without editing the ledger
    And the ceremony reader asks native context what remains
    Then the experience has reconciliation state "revalidation-required"
    And the original acceptance record remains in the ledger
