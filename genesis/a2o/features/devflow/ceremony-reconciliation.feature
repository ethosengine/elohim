@concern:dev-system-equilibrium
Feature: Preserve purpose while reconciling memory
  As a software agent maintaining instructions for people and later agents
  I want to repair supported references while preserving unresolved acceptance questions
  So that the next reader can distinguish evidence from an unsupported promise

  A ceremony is a saved investigation ending in a handover, which may include open questions.
  Its purpose here is to help the next reader distinguish the supported repair from a contested promise.
  Its governing rule is that a source reference update cannot establish independent acceptance.
  Two instruction entries each contain one claim and cite an older verification report.
  The first claim is "A trial reader opened a cited passage".
  The second is "An independent reader accepted the whole investigation workflow".
  The report describes an earlier trial of a workflow for inspecting an instruction's evidence and handing over the result.
  Its revised text records that following a citation opened the passage, but says whole-workflow acceptance is undecided.
  Only the first entry's reference can be verified against this revision; the second needs independent evaluation.
  Acceptance requires a separately appointed reader to try the workflow and record a judgment.
  The entry's owner is unknown, so the handover must ask the next agent to identify the owner and arrange that evaluation.
  The ceremony concern view exposes each reference's current or stale state; the old disputed instruction is not silently endorsed.
  A receipt saves the exact inspected passage, its bytes and the agent's observation.
  Reusing the observation that the report says "acceptance undecided" does not decide actual acceptance.
  In the "Recover an inspected report observation and the remaining owner task" scenario,
  the claim, report, governing instructions and review records also remain unchanged.
  The governing search recipe permits both local file discovery and optional semantic search.
  Search hits are candidates requiring source inspection. Their ordering cannot establish authority,
  so the search view discloses that local discovery cannot explain another provider's undisclosed ranking.

  Scenario: Find both entries affected by a report revision
    Given a ceremony with two assertions depending on the same changed source
    When the agent opens the saved investigation and chooses an assertion
    Then the selected claim shows its stale report reference and the purpose of preventing unsupported acceptance
    And both affected entries are listed as separately reviewable claims

  Scenario: Recover an inspected report observation and the remaining owner task
    Given the agent inspected the disputed acceptance claim and saved the question of who should arrange independent evaluation
    And the inspected passage has not changed since that receipt
    When a fresh process resumes the ceremony
    Then it recovers the purpose, inspected passage receipt and task of identifying the owner to arrange evaluation
    And the report observation is reusable while the acceptance question remains open

  Scenario: Withdraw a saved observation when its passage changes
    Given the agent inspected the disputed acceptance claim and saved the question of who should arrange independent evaluation
    When the inspected passage changes and a fresh process resumes the ceremony
    Then the saved report observation is marked unusable until the changed passage is inspected again

  Scenario: Locate a report through permitted local search when semantic search is unavailable
    Given a ceremony with an unavailable optional retrieval provider
    When the agent chooses an authorized local alternative
    Then the local search finds the report for direct passage inspection
    And the local result retains the repair purpose and explicitly requires inspecting its source before using it as evidence
    And the search view says the unavailable service's ranking is unknown

  Scenario: Finish the reference repair and hand over the unaccepted claim
    Given the revised report records the trial reader opening a cited passage and leaves whole-workflow acceptance undecided
    When the agent updates only the supported reference and checks the affected entries
    Then the concern view shows the supported reference current and the disputed entry still awaiting review against its old reference
    And completion reports the reference repair without claiming independent acceptance
    And the handover identifies the disputed whole-workflow claim as undecided and gives the next agent its purpose and owner-identification task
