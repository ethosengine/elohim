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

  The two scenarios tagged recall-reaches-authority belong to a second concern. Every scenario above
  reaches its evidence through one command — the RECALL ENTRY, which opens an investigation, reads a
  bounded passage, and states a purpose and a byte count for each. The second concern asks whether
  that one entry serves every agent question, including a question no ceremony posed. It is exercised
  against this same ceremony because the recall entry is the same command exercised by both concerns.
  A fresh agent arrives with no prior ceremony context and carries a question of its own, and the
  entry is documented as "open with that question". That question is what the investigation is FOR.
  The recall algorithm also carries a standing description of what recall is generally for, which it
  states as the purpose whenever nobody brings a question — a sentence about the method, naming no
  concern and no question. When that standing description stands in place of the agent's question,
  every later receipt and completion is accounted against a purpose nobody held — and the handover
  the other scenarios build is addressed to no one.
  The surfaces such an agent must recall include the tooling it works through — skill files, meaning
  instruction documents that describe one discipline an agent follows. A declared source scope names
  which paths the entry may read at all: a path outside it is REFUSED, so naming the tooling
  directory inside the scope is the whole reason a skill is readable here.
  What the entry reads it counts, because the entry bounds it: the recall algorithm declares a packet
  limit — the bytes of source evidence one bounded pass may read — and completion reports the counted
  bytes against that limit. A journey that answered correctly by reading everything would prove
  nothing about the entry.
  The same entry also serves an agent RESUMING interrupted work on one tracked promise — a concern
  the repository keeps with a check that proves it and a dated record of the evidence the check
  last produced. A passing result recorded on one date says nothing about changes that landed
  after it, so the resuming agent must be told which later commits the recorded evidence does not
  cover, and which check would cover them, without the entry running that check or accepting the
  changes on the old result's authority.

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

  @concern:recall-reaches-authority
  Scenario: A fresh agent's question becomes the purpose, its tooling is in reach, and what it read is receipted and bounded
    Given a ceremony whose declared source scope covers the tooling directory
    And the recall entry, the one command that opens an investigation, reads a bounded passage and reports a purpose and a byte count
    And a skill file in that directory naming the command that rebuilds a stale index
    And the standing description that entry states as its purpose when nobody brings a question
    And a declared packet limit wider than that skill passage and far narrower than the repository
    When a fresh agent with no prior ceremony context opens the recall entry carrying the question "Which command rebuilds the stale index now the old scripts are gone?"
    And it reads the passage of that skill file which names the command
    And it attempts the same read against a path outside the declared scope
    Then the investigation's stated purpose is that question, not the standing description
    And the skill passage is preserved as a receipt recording the exact bytes the agent read
    And the out-of-scope read was refused, so the declared scope is what put the skill in reach
    And the investigation's completion report names that same question, and the counted bytes stay under the packet limit

  @concern:recall-reaches-authority
  Scenario: An agent resuming interrupted work learns which changes its last evidence does not cover
    Given a tracked concern whose evidence record says its check last passed on an illustrative date, 5 September
    And a commit on 10 September that changed that concern's plan after the check passed
    When a fresh agent opens the recall entry to resume that concern
    Then the resumption view names the concern, its recorded standing and the 5 September evidence date
    And it lists the 10 September commit as implemented but unverified, not as accepted
    And it names the concern's check to rerun, as a handover rather than a result

  Scenario: Finish the reference repair and hand over the unaccepted claim
    Given the revised report records the trial reader opening a cited passage and leaves whole-workflow acceptance undecided
    When the agent updates only the supported reference and checks the affected entries
    Then the concern view shows the supported reference current and the disputed entry still awaiting review against its old reference
    And completion reports the reference repair without claiming independent acceptance
    And the handover identifies the disputed whole-workflow claim as undecided and gives the next agent its purpose and owner-identification task
