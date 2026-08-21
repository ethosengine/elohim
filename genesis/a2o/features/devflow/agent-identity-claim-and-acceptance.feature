@e2e @devflow @concern:agent-identity-claim-and-acceptance @act:host @requires:epr-cli
Feature: An agent claims who it is, in flight, and the record survives the claim being wrong
  As an agent working inside a session (or a human dispatching one)
  I want to register an honor-system identity claim for the run and have every
  attribution surface consult it
  So that who-acted is a disputable record made by the actor while acting, not a
  roster reconstructed from the dispatcher's memory after the fact

  # Spec: genesis/docs/superpowers/specs/2026-08-15-actor-plane-inflight-identity-claims-design.md
  # Vocabulary a cold reader needs:
  #   - The actor sidecar is `.eprfs/status/actors.jsonl` — an append-only JSONL log of
  #     claims, one `{cid, record}` line each. A CID (Content Identifier) is a hash derived
  #     from the record's bytes, re-verified on read — so any mutation of a stored line
  #     invalidates it. `epr` is the repo's native governance-and-valueflow CLI;
  #     `epr actor claim/current` write and read this log.
  #   - The valueflow is the repository's REA (Resource-Event-Agent) ledger
  #     (`.eprfs/status/flows.jsonl`), derived
  #     from git history by `epr flow project`; a "produce" event records an artifact
  #     entering the tree, and its `classified_as` list carries prefixed classification
  #     slots (`reason:`, `steward:`, `co-author:`).
  #   - An agent package is the persona's definition file at
  #     `.epr-meta/elohim/packages/agents/<role>.json`.
  # Contract: the claim NEVER blocks anything (honor-system floor — a claim that could
  # block work would be a credential, and nothing at this floor can verify one). Claims
  # stack per session; latest wins; history is never rewritten. The steward — the
  # git-signing human — stays attached to every agent attribution.

  Background:
    Given a committed repository with an agent package at ".epr-meta/elohim/packages/agents/scribe.json"

  # ── 1. claim — the floor ─────────────────────────────────────────────────

  @wip
  Scenario: Registering a claim records who is acting, dated by the tree
    When the agent claims "agent:scribe@opus-5" for session "run-1"
    Then the actor sidecar ".eprfs/status/actors.jsonl" holds one claim line for session "run-1"
    And the claim's timestamp equals the HEAD commit's author date
    # (never the wall clock — two peers reading the same tree must read the same date)
    And the claim's definition address is the sha256 of the scribe package file's raw bytes

  @wip
  Scenario: A missing package is honest absence, never a placeholder address
    When the agent claims "agent:unpackaged-role@opus-5" for session "run-1"
    Then the claim records no definition address
    And the claim is still registered

  @wip
  Scenario: A persona switch supersedes without rewriting history
    Given the agent claimed "agent:scribe@opus-5" for session "run-1"
    When the agent claims "agent:rust-architect@opus-5" for session "run-1"
    Then the current claim for session "run-1" names "agent:rust-architect@opus-5"
    And the superseded scribe claim is still readable in the sidecar's history

  @wip
  Scenario: A malformed identity is refused, never silently replaced by the author
    When the agent attempts to claim "scribe-without-prefix" for session "run-1"
    Then the claim is refused naming the legal shape "agent:<role>@<model>"
    And nothing is written to the actor sidecar

  # ── 2. attribution — the surfaces that consult the claim ────────────────

  @wip
  Scenario: A run note is attributed to the claimed identity with the steward attached
    Given the agent claimed "agent:scribe@opus-5" for session "run-1"
    When a note is recorded via "epr flow note" under session "run-1"
    Then the note event's provider is "agent:scribe@opus-5"
    # The steward slot is asserted LAST by position, not merely present: earlier slots are
    # read positionally by other consumers, and steward is always appended after them so
    # human attribution trails every agent slot.
    And the note event's last "classified_as" slot is "steward:" followed by the git author email

  @wip
  Scenario: An unclaimed session falls through to author attribution with a notice
    When a note is recorded via "epr flow note" under session "run-nobody-claimed"
    Then the note event's provider is the git author email
    And a notice on stderr names the session that registered nothing
    And the note is recorded successfully

  @wip
  Scenario: A governance decision stamps who was acting when it was asked for
    Given the agent claimed "agent:scribe@opus-5" for session "run-1"
    When "epr govern" evaluates a prospective edit to "notes/subject.md" for session "run-1"
    Then the decision payload carries an actor stamp with source "claim" naming "agent:scribe@opus-5"

  @wip
  Scenario: A tampered identity log reads as unclaimed, never as the tamperer's identity
    Given the agent claimed "agent:scribe@opus-5" for session "run-1"
    And a claim line in ".eprfs/status/actors.jsonl" is rewritten to name a different identity
    When "epr govern" evaluates a prospective edit to "notes/subject.md" for session "run-1"
    Then the decision payload carries an actor stamp with source "unclaimed"
    And the governance decision itself still lands

  # ── 3. plural authorship in the projected valueflow ──────────────────────

  @wip
  Scenario: A commit's co-author roster reaches the produce event, sorted and additive
    Given a document committed with two Co-Authored-By trailers
    When "epr flow project" derives the repository's valueflow
    Then the document's produce event carries both "co-author:" slots in sorted order
    And the produce event's provider is still the signing author

  @wip @regression
  Scenario: Re-projection never records the same act twice, even when its vocabulary grew
    Given "epr flow project" has already derived the document's produce event before its commit named any co-authors
    And the commit message is amended to name co-authors without touching the tree
    When "epr flow project" derives the repository's valueflow again
    Then no new produce event is appended for that document
    And the recorded event still reads exactly as it was witnessed

  # ── 4. acceptance and adverse attestation — the ceremony (design, unbuilt) ──
  # Tier vocabulary (the graduated-rigor ladder): Witness tier = the acceptance is
  # recorded as seen, no judgment of plausibility is made; Audit tier = a contested
  # identity's next acceptance re-runs with the evidence weighed. An adverse attestation
  # is an evidenced correction record naming a claimed identity — its "evidence address"
  # is the content address of the evidence itself, so a bare accusation cannot be filed.

  @wip @design
  Scenario: Ratification at dev-merge accepts the session's claims at Witness tier
    Given a session whose claims attributed work now merged to dev
    When the ratification ceremony records acceptance
    Then each claim gains an acceptance record at Witness tier
    And no plausibility judgment is recorded, only that acceptance was witnessed

  @wip @design
  Scenario: A contested identity escalates the next acceptance to Audit
    Given an adverse attestation with an evidence address stands against a claimed identity
    When that identity's next acceptance is recorded
    Then the acceptance runs at Audit tier instead of Witness
    And the adverse attestation remains advisory, blocking nothing
