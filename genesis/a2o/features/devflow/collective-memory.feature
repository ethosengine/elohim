@concern:dev-system-equilibrium
Feature: Collaborate through governed memory without carrying its whole history
  As a repository agent joining another agent's investigation
  I want a small context view that traces back to governed evidence
  So that I can correct misleading context and share only warranted repository knowledge

  The repository declares its local collective relationship under .epr-meta.
  That declaration is local governance, not proof of network membership.
  The evidenced finding is "The trial reader opened the cited passage".
  The disputed alternative is "The whole workflow has independent acceptance".
  The source records that passage opening succeeded and whole-workflow acceptance is undecided.
  Both claims remain separately attributable; a projection must not erase the undecided qualification.
  A local-only passage must not escape through either an assertion or its receipt.
  Measurements describe a declared scope; missing measurements are not zero cost.

  Scenario: A second agent follows shared context and preserves the dispute after reset
    Given a declared local collective with an evidenced finding and a contested alternative
    When another agent opens that collective through the ceremony
    Then its context identifies the collective, purpose, evidence and unresolved alternative
    When a fresh process resumes the collective ceremony
    Then the same governed inputs remain traceable without importing private native memory

  Scenario: A challenge names the projection that misled the reader
    Given a declared local collective with an evidenced finding and a contested alternative
    When the reader challenges a projection's omission rather than its source assertion
    Then the feedback identifies the exact viewed projection and its omitted undecided qualification
    And the source claim remains unchanged pending its own review

  Scenario: Repository graduation keeps local evidence local
    Given a declared local collective with an evidenced finding and a contested alternative
    When the agent rehearses repository reach for a finding depending on local-only evidence
    Then the graduation refuses to widen the finding and its revealing receipt

  Scenario: The closing lens compares explicit observations without grading judgment
    Given a ceremony with an explicit scoped burden baseline
    When a duplicate is consolidated and the ceremony records its closing observation
    Then the deterministic report shows active content and total retained byte changes with exact paired inputs
    And archived bytes and new run artifacts are counted rather than mistaken for deletion
    And the report leaves the contested judgment unresolved
