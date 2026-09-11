@e2e @devflow @concern:dev-system-equilibrium @act:host @requires:epr-cli @wip
Feature: An agent's memory contribution names a verifiable, composed author inside a plurally stewarded collective
  As an agent contributing a finding to the repository's shared memory
  I want my contribution to carry a signature I can prove, a description of what I am
  made of, and a place inside a collective whose stewards are people on record
  So that a later reader can tell which agent said it, under whose standing, and who may
  approve widening it, without trusting a label

  # Vocabulary a cold reader needs:
  #   - `epr` is the repository's native governance CLI. It keeps append-only logs under
  #     `.eprfs/status/`, called sidecars: `actors.jsonl` (who claimed to be acting),
  #     `flows.jsonl` (what was done), and, after this story, `affiliations.jsonl` (who belongs
  #     to which collective). Every line is `{cid, record}`; the CID (content identifier) is a
  #     hash of the record's bytes, re-checked on read, so a rewritten line invalidates itself.
  #   - A persona claim is an honor-system statement "agent:<role>@<model> is acting in
  #     session S". Signing it adds a proof; it never adds a permission. An unsigned claim
  #     stays valid. A governance decision records who was acting as an actor stamp whose
  #     source is "claim" when a claim stands, or "unclaimed" when none can be trusted.
  #   - Mooring is how a session declares its running conditions before it works: the model
  #     and lab it runs on, its runtime envelope (effort, context budget, tool set) and its
  #     task. The mooring record is what a claim pins.
  #   - A persona package is the file that defines a role an agent can play, such as
  #     "implementer" or "reviewer": its instructions, tools and model contract, kept at
  #     `.epr-meta/elohim/packages/agents/<role>.json`. Its bytes are what an identity pins.
  #   - The model vocabulary table is `epr`'s registry of recognized model spellings and their
  #     normalized form ("anthropic/opus-5"); a spelling not in it cannot mint an identity.
  #   - An agent is a chain of layers, not one label. Each layer is a small record that pins
  #     one thing the agent is made of and points at the layer beneath it. Hardest first: hard
  #     (the model, normalized so two spellings of one model are one layer), tuning (an
  #     optional adapter or system prompt), given (the persona package), contextual (acting as
  #     representative of a person or collective, optional), and ephemeral (this one run: the
  #     mooring record, the session id, and the steward it acts for). The chain minus its
  #     ephemeral layer is the agent's identity. The ephemeral layer is the session. A
  #     contribution names the identity as author and the session as provenance. A layer the
  #     session cannot supply is reported absent, never guessed. Feedback about a bad run is
  #     aimed at the session; feedback about a bad system prompt is aimed at the tuning layer
  #     and reaches every session chained on it.
  #   - A collective is declared by `.epr-meta/collective.json` in a directory. It no longer
  #     names a steward. Members are affiliation records, one per member, each with a kind
  #     (Person, ElohimAgent, or Collective) and a role (Steward, Contributor, or Observer),
  #     shaped like the network's Membership so they can be minted onto it later. A session's
  #     collective of record is the collective it is bound to, resolved by walking up from the
  #     path it works under to the nearest declaration.
  #   - `did:key` is a standard identity string derived from a public key alone; the DID
  #     bridge crate derives it offline.
  #   - A verdict is a note a member records on a contribution, approving or requesting
  #     changes, logged in `flows.jsonl` with the member's own claim as its author. It is not
  #     the same as a governance decision, which is what `epr govern` returns when asked
  #     whether a prospective edit may proceed.
  #   - A feedback signal is the protocol's existing signed note about any record by its CID
  #     (a correction, a squelch, a retraction). Unlike a verdict, which targets a contribution,
  #     a feedback signal may target a layer of an agent's chain; walking the chain means
  #     reading each layer from the session down to the hard layer and collecting the signals
  #     aimed at each. Today `epr flow note --kind correction --on <cid>` records one; a
  #     dedicated feedback kind is part of the work this story specifies.
  #   - Commands used here: `epr actor claim` registers a persona claim; `epr govern` evaluates
  #     a prospective edit; `epr flow memory collective` inspects a collective; `epr flow memory
  #     contribute` records a finding; `epr flow memory graduate` rehearses a widening.
  #     A contribution starts at private locality. The `@<model>` in a claim string is display
  #     only; the mooring supplies the model that the hard layer normalizes.
  #   - Locality (private, workspace, repository) says where a passage may travel inside this
  #     repository. Widening a contribution means graduating its locality outward, and a
  #     graduation is one-way. A rehearsal is a non-committing check that a graduation would
  #     be permitted. Locality is not reach: reach is the network's audience vocabulary, and
  #     confusing the two would misroute a passage at the network boundary.
  # Contract: nothing here blocks work. Refusals only guard the shared memory: a contribution
  # or a widening that would misattribute, or that lacks a distinct steward's approval, is
  # refused with the reason named. Everything else records and moves on.

  Background:
    Given an isolated repository with a root collective declared under ".epr-meta"
    And one affiliation of kind Person and role Steward for the git author

  # ── 1. a persona proves it is the persona it claims ────────────────────────

  Scenario: A signed claim verifies and carries a did:key derived from its public key
    Given the session "run-1" has generated an actor key
    When the agent claims "agent:implementer@opus-5" for session "run-1" with a signature
    Then the current claim for session "run-1" reports its signature as valid
    And the current claim reports a "did:key" that the DID bridge derives from the same public key

  Scenario: An unsigned claim is still a claim
    When the agent claims "agent:implementer@opus-5" for session "run-1" without a signature
    And a note is recorded under session "run-1"
    Then the current claim for session "run-1" reports no signature
    And the note is attributed to "agent:implementer@opus-5"

  Scenario: A session that never claimed anything still records, attributed to the git author
    Given the session "run-nobody" has registered no claim
    When a note is recorded under session "run-nobody"
    Then the note is recorded successfully
    And the note is attributed to the git author with a notice that the session claimed nothing

  Scenario: A forged signature reads as unclaimed, never as the forger's identity
    Given the session "run-1" has a signed claim for "agent:implementer@opus-5"
    And the signature bytes on that claim line are replaced
    When "epr govern" evaluates a prospective edit for session "run-1"
    Then the governance decision records the actor as unclaimed
    And the governance decision is still returned to the caller

  # ── 2. an agent is a chain of layers, not a label ──────────────────────────

  Scenario: A claim mints the chain from what the mooring supplies
    Given the session "run-1" is moored with model "opus-5", lab "anthropic" and task "memory"
    When the agent claims "agent:implementer@opus-5" for session "run-1"
    Then the claim references an ephemeral layer chained on an identity
    And the identity's hard layer pins the normalized model "anthropic/opus-5"
    And the identity's given layer pins the implementer persona package bytes
    And the session's steward is the git author
    And the identity reports its tuning and contextual layers as absent

  Scenario: A broken chain is refused, naming the link
    Given the session "run-1" claimed "agent:implementer@opus-5"
    And the given layer's line in the actor sidecar is rewritten so its link points at a CID that does not exist
    When the agent asks for the current claim of session "run-1"
    Then the chain is refused naming the missing CID
    And the governance decision for session "run-1" records the actor as unclaimed

  Scenario: Feedback aimed at a tuning layer reaches every session chained on it
    Given sessions "run-1" and "run-2" claim "agent:implementer@opus-5" with the same tuning layer "prompts/implementer-v3.md"
    And a session "run-3" claims "agent:implementer@opus-5" with no tuning layer
    When a member records a feedback signal against the tuning layer's CID
    Then walking the chain of "run-1" finds that feedback
    And walking the chain of "run-2" finds that feedback
    And walking the chain of "run-3" does not

  Scenario: Two spellings of one model are one hard layer and one identity
    Given the session "run-1" is moored with model "opus-5" and lab "anthropic"
    And the session "run-2" is moored with model "claude-opus-5" and lab "anthropic"
    When session "run-1" claims "agent:implementer@opus-5" and session "run-2" claims "agent:implementer@claude-opus-5"
    Then both claims share every layer of the identity, including the hard layer's CID
    And the two claims reference different ephemeral layer CIDs

  Scenario: An unknown model spelling is refused rather than minted as a new identity
    Given the session "run-1" is moored with model "opus-5-turbo-x" and lab "anthropic"
    When the agent claims "agent:implementer@opus-5-turbo-x" for session "run-1"
    Then the claim is refused naming the model vocabulary table
    And nothing is written to the actor sidecar

  Scenario: A changed runtime envelope is a new session, not a new identity and not an edit
    Given the session "run-1" claimed "agent:implementer@opus-5" while moored with task "memory"
    When the session "run-1" is re-moored with task "review"
    And the agent claims "agent:implementer@opus-5" for session "run-1"
    Then the current claim references a different ephemeral layer CID
    And the current claim's ephemeral layer chains on the same identity as before the re-mooring
    And the earlier claim and its ephemeral layer are still readable in the actor sidecar

  Scenario: A memory contribution names the identity as author and the session as provenance
    Given the session "run-1" has a signed claim with a full chain
    When the agent contributes a finding to the root collective through "epr flow memory contribute"
    Then the contribution's author is the identity CID
    And the contribution's provenance is the session CID
    And the contribution's display name is "agent:implementer@opus-5"
    And the contribution's steward equals the session's steward

  # ── 3. collectives have stewards, plural, on record ────────────────────────

  Scenario: A declaration that names a steward is refused
    Given the root declaration is rewritten to include a "steward" field
    When the agent inspects the collective through "epr flow memory collective"
    Then the declaration is refused naming "steward" as an unknown field
    And the message says stewards are affiliation records

  Scenario: A collective with no Steward affiliation is refused
    Given every Steward affiliation for the root collective is withdrawn
    When the agent inspects the collective
    Then the collective is refused for lacking a steward on record

  Scenario: A directory declares a child collective and a path resolves to the nearest one
    Given "genesis/concern/.epr-meta/collective.json" declares a child whose parent references the root declaration by CID
    When the session binds its collective of record by claiming under "genesis/concern/finding.md"
    Then the collective of record is the child
    And a sibling path outside "genesis/concern" resolves to the root

  Scenario: A claim bound to the wrong collective cannot contribute
    Given the session "run-1" bound the root collective as its collective of record
    When the agent contributes a finding sourced from "genesis/concern/finding.md"
    Then the contribution is refused naming both the bound collective and the source's collective

  Scenario: A declaration that still speaks reach is refused with the rename named
    # Locality is where a passage travels inside the repository; reach is the network's
    # audience vocabulary. A declaration that says "reach" would mislabel where a contribution
    # may travel, which corrupts the provenance chain this story protects.
    Given the root declaration's source rules use "maxReach"
    When the agent inspects the collective
    Then the declaration is refused and the message names "maxLocality"

  # ── 4. widening needs a distinct steward ───────────────────────────────────

  Scenario: An author cannot approve widening its own contribution
    Given a contribution by session "run-1" with an approving verdict recorded by the same identity
    When the agent rehearses repository locality for that contribution
    Then the graduation is refused because the approver is the author

  Scenario: A Contributor's approval is not enough
    Given a contribution by session "run-1" with an approving verdict from a member whose affiliation is Contributor
    When the agent rehearses repository locality for that contribution
    Then the graduation is refused because the approver holds no Steward affiliation

  Scenario: A distinct Steward's approval lets the rehearsal pass
    Given a contribution by session "run-1" with an approving verdict from another member whose affiliation is Steward
    When the agent rehearses repository locality for that contribution
    Then the graduation rehearsal passes
    And the rehearsal receipt names the approving steward's affiliation CID

  # ── 5. all four together ───────────────────────────────────────────────────

  Scenario: One contribution carries every leg of the promise at once
    Given the session "run-1" is moored with model "opus-5", lab "anthropic" and task "memory"
    And the session "run-1" has generated an actor key
    And a second Person affiliation of role Steward exists for "reviewer@example.org"
    When the agent claims "agent:implementer@opus-5" for session "run-1" with a signature
    And the agent contributes a finding to the root collective
    And the second steward records an approving verdict on that contribution
    And the agent rehearses repository locality for that contribution
    Then the contribution's author identity pins the normalized model "anthropic/opus-5" and the implementer package
    And the contribution's provenance session carries a valid signature and a "did:key"
    And the contribution's collective of record is the root collective
    And the graduation rehearsal passes naming the second steward's affiliation CID
