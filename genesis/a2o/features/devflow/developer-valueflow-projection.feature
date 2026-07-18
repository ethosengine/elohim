@e2e @devflow @concern:epr-rea-valueflow-fabric
Feature: The developer valueflow, projected from the repository itself
  As a developer (human or agentic) changing an artifact anywhere in the pipeline
  I want the repo's own manifesto → epic → spec → plan → scenario → code → gate →
  verdict chain projected as REA flow records
  So that I can walk from any changed artifact to the lineage that produced it and
  to the frontier of work still owed, instead of re-deriving cross-surface coherence
  by memory each time

  # Spec: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  # Recipe: elohim-dev-pipeline@1 (.claude/epr-meta/recipes.yaml) — the dogfood instance;
  # stages are manifesto, epic, architecture-seed, spec, plan, intent, scenario, validation.
  # Contract: the frontier is the mechanical form of "if you change A, walk the
  # valueflow to the finish before you claim coherence" — a change is coherent only
  # when its downstream commitments are fulfilled, never asserted from memory.

  Background:
    Given the repository declares the "elohim-dev-pipeline@1" recipe
    And the sidecar flow store starts empty

  # ── 1. project — the repo's artifacts become flow records ────────────────

  @wip
  Scenario: Projecting the repository derives flow records for every recipe stage
    Given a spec, its cited seeds, an open gap-item, and a parsing a2o scenario each sit under one of the recipe's stage paths
    When the repository's valueflow is projected
    Then a flow record is appended to the sidecar for every discovered artifact
    And each appended record is labeled with its recipe stage and artifact kind
    And the projection reports how many intents, commitments, and events it derived

  # ── 2. walk back — lineage to cited seeds and the produce event ──────────

  @wip
  Scenario: Walking a spec shows lineage to its cited seeds and its produce event
    Given a spec's sealed cites name its epic and architecture-seed as its cited seeds
    And the repository's valueflow has been projected
    When the value chain is walked backward from that spec
    Then the lineage names the epic and the architecture-seed as the spec's inputs
    And the lineage names the produce event that born-linked the spec into the recipe's spec stage
    And that produce event's provenance is walkable, not merely asserted

  # ── 3. walk forward — the unfulfilled frontier is the coherence check ────

  @wip
  Scenario: Walking forward from a spec surfaces its unfulfilled frontier
    Given the spec's plan has been decomposed into gap-item intents
    And one gap-item has been claimed as a commitment
    And no a2o scenario has yet gone green for that commitment
    When the value chain is walked forward from that spec
    Then the frontier names the claimed commitment as unfulfilled
    And the frontier names the scenario stage as the crossing still owed
    And an unfulfilled frontier is how "walk the valueflow to the finish" is answered mechanically rather than from memory

  # ── 4. fulfillment shrinks the frontier ───────────────────────────────────

  @wip
  Scenario: A fulfilled commitment leaves the frontier
    Given a commitment sits open in a spec's forward frontier
    When the a2o scenario that commitment promised goes green
    And the repository's valueflow is projected again
    Then a fulfilling event is appended whose fulfillment names the commitment
    And that commitment no longer appears in the spec's forward frontier
    And the commitment's fulfillment ratio reads complete

  # ── 5. tamper is loud, never silent drift ─────────────────────────────────

  @wip @regression
  Scenario: A tampered sidecar line fails as an integrity error, never silent drift
    Given the sidecar flow store holds an appended flow record
    When a byte of that record's line is altered on disk after it was appended
    Then reading the sidecar raises an integrity error naming the stored and recomputed identities
    And the value chain never treats the tampered record as merely absent or as new

  # ── 6. re-projecting never duplicates ─────────────────────────────────────

  @wip
  Scenario: Re-projecting the repository is idempotent
    Given the repository's valueflow has already been projected once
    When the repository's valueflow is projected again with nothing changed
    Then no new flow records are appended
    And the reported intent, commitment, and event counts are unchanged from the first projection
    And every record's identity from the second projection matches the one appended by the first
