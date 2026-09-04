# STEP DEFINITIONS ARE NOT WIRED YET — this feature is honestly @wip, not green.
#
# The executing proof lives in the focused Rust assertions for `epr flow claim`,
# `epr flow fulfill --on`, note ordering, and `epr flow context`. No step in
# genesis/a2o/steps drives this story today. The context-blind `a2o-story`
# review loop is dispatched by the orchestrator after this authoring pass.
#
# A scenario repository below is always an isolated scratch repository. The
# steps never claim, fulfil, annotate, or inspect work in the live checkout.
#
# Spec: genesis/docs/superpowers/specs/2026-09-05-valueflow-authoring-surface-design.md

@e2e @devflow @wip @concern:valueflow-authoring @requires:epr-cli @act:host
Feature: A task moves through one readable valueflow from claim to verdict
  As an orchestrator handing planned work to implementers and reviewers
  I want claims, reports, rulings, verdicts, habits, and gates joined through
  the repository's valueflow rather than reconstructed from conversation
  So that I can see who owns a task, whether it was actually discharged, which
  decisions govern it, and how to verify nearby work before I dispatch again.

  # Vocabulary for a reader who has only this file. Every command named below
  # (claim, fulfill, context, project) is a subcommand of the `epr` CLI —
  # `epr flow claim`, `epr flow fulfill`, `epr flow context`, `epr flow
  # project` — so `@requires:epr-cli` names one binary, not a family of tools.
  #
  #   gap id       — the stable id of one work item decomposed from a plan.
  #   projected plan — a plan document whose checkbox tasks have been
  #                  decomposed into gap items and projected into the
  #                  valueflow by `epr flow project`, so each task exists as
  #                  an intent an implementer can claim.
  #   commitment   — the still-open promise minted when an actor claims that
  #                  gap; its provider is the incumbent actor.
  #   discharge    — a fulfillment event that names the commitment and closes
  #                  it. Describing blocked work is evidence, not discharge.
  #   ruling       — a binding decision recorded as a note against a gap or
  #                  plan instead of being left only in a progress document.
  #   verdict      — the reviewer's note, with an approved or
  #                  changes-requested value in its dedicated verdict slot.
  #   context      — one `epr flow context <path-or-gap>` read that assembles
  #                  identity, open work, notes newest first, the covering
  #                  habit, and the owning gate without changing the tree.
  #   habit        — a repository promise tied to a runnable concern check.
  #   gate         — the `just gate <name>` command declared for the project
  #                  that owns a source path, not a recipe guessed by an agent.

  Scenario: A gap can have one incumbent claim, never two
    Given a scratch repository whose projected plan contains an unclaimed gap item
    And two implementers with distinct actor identities are present
    When an implementer claims that gap with a task brief and an actor identity
    Then the valueflow holds exactly one open commitment for that gap
    And that commitment names the claiming actor as its provider
    When the other implementer, a distinct actor identity, attempts to claim the same gap
    Then the second claim is refused with a non-zero exit status
    And the refusal names the incumbent actor
    And the valueflow still holds exactly one commitment for that gap

  # Records are content-addressed, so re-running a fulfilment against the same
  # commitment must not double-count the drain that the equilibrium habit
  # measures — "exactly once" is what keeps that measurement honest.
  Scenario: Only a discharging report closes its claimed commitment
    Given two gap items in a scratch repository are claimed as open commitments
    When the first commitment receives a task report with status "DONE"
    And the second commitment receives a task report with status "BLOCKED"
    Then a fulfillment naming the first commitment is appended exactly once
    And the first commitment no longer appears among the open commitments
    And no fulfillment names the second commitment
    And the second commitment remains open with its blocked report readable as evidence

  Scenario: Context shows the newest review decision before the ruling it followed
    Given a ruling note is recorded against a claimed gap
    And a reviewer later records an approved verdict note against that same gap
    When an agent reads context for the gap
    Then both the ruling and the verdict are readable in the notes section
    And the newer verdict is rendered before the older ruling
    And the verdict is rendered from its dedicated slot as "approved"

  Scenario: Context names the promise and command that govern a storage source file
    Given a storage source file covered by a declared habit and a build-manifest gate project
    When an agent reads context for that source-file path
    Then the habit section names the covering habit, its status, active flag, and first check
    And the gate section names the declared "just gate" command that owns the file
    And the gate section includes any Cargo target directory and Rust flags declared by that project
    And the gate section is sufficient to run the verification command without guessing the build configuration
    And neither the habit nor the gate is guessed when its register has no matching declaration
