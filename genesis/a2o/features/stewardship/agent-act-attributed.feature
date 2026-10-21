# HELD (2026-10-20). Every scenario is @wip: no step below has a definition yet, and the record it
# reads is not built. That record is the ordered list, kept with each note in the shared memory,
# of every act on it and the participant (and any sponsor) who made it. The placement-contract design
# (genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md)
# names the missing node in §15 ("agent act to attributed event": the per-contribution record of
# acts, of which none exists yet). §17 lists agent attribution among the planes that build beside
# its numbered slices. This story is held until that record lands.
#
# This needs no household mesh.
#
# The governance rule for this stewardship directory (its .epr-meta file) asks every story here
# to show how the person affected can contest an authority acting over them. No one in this story
# holds authority over anyone, so there is nothing to contest; the description's last paragraph
# says so, and the last scenario carries the answer that takes the place of an appeal.
@e2e @stewardship @agents @concern:acts-attributed-to-participants @wip
Feature: An agent's edit is recorded as the agent's own act, with the person who sponsors it named
  As Jessica, reading a note Matthew wrote that was later changed
  I want to see which change Matthew made and which change an agent made for him
  So that I know who did what, and whom to answer when I disagree

  Matthew and Jessica belong to a small team that keeps a SHARED MEMORY: a store of notes, each
  one a CONTRIBUTION, that the team writes, edits and reviews together. The shared memory is kept
  as files in a repository the team shares. Reloading it means reading the record again from those
  repository files. A reload guards against two failures: an act that was held only in memory and
  never written to the files, which a reload loses, and a reading that adds, merges or rewrites
  acts, which a second reload with nothing changed would expose. A contribution changes only by ACTS:
  authoring it, editing it, reviewing it, retracting it, or graduating it (marking it as knowledge
  the team has accepted). Each act is recorded in order, recording an act writes it to the
  repository files at once, and an act once recorded is never rewritten. Every act names the PARTICIPANT who did it, and a participant is either a person or
  an agent. The second scenario reloads twice and so meets both failures: the first reload finds
  the newest edit only if it was written to the files, and the second, with nothing changed, finds
  the same count only if reading added nothing. Retracting and graduating name their participant
  and, for an agent, its sponsor, by the same rules as every other act. Whether a retraction or a
  graduation is allowed is not this story's question, so its scenarios exercise only authoring,
  editing and reviewing.

  Every participant acts under a name it claimed. Names follow one convention: a person's name
  begins "human:" and an agent's begins "agent:", so the record always shows which kind of
  participant acted. A person appears in the record only under the name they claimed for
  themselves, such as "human:matthew". Their email address or any other account name is never
  copied in. A change that reaches a contribution's file directly, made by software that claimed
  no name and so made no act, is still recorded when the files are read again: its participant
  appears as "(unclaimed)", and it is never credited to anyone's name.

  An AGENT is a piece of software that works for the team, such as an AI model asked to tidy a
  note. It is not a person and does not speak as one. Its name says what it does and, after the
  "@", which model it runs on: "agent:editor@model-one" is an editor running on the model
  "model-one". It acts only because a person SPONSORS it. A person sponsors an agent by declaring,
  in the shared memory, that the agent works on their behalf; from then on that person answers for
  what it does. That declaration is the only gate, and it must come first: an act by an agent that
  no one has yet declared sponsoring is refused and not recorded, and a declaration made later does
  not reach back to it. The sponsor is named on every act the agent records. Being named as
  sponsor does not make the act the sponsor's.

  Authorship is the first act. The AUTHOR of a contribution is whoever made its authoring act,
  and that does not change, however many edits follow. A REVIEW counts as independent only when
  the reviewer took no part in shaping the contribution: they did not author it, did not edit it,
  and do not sponsor an agent that authored or edited it, since a sponsor answers for every act
  that agent makes.

  Stories about stewardship are held to a rule: each must show how the person affected can contest
  an authority acting over them. Nobody in the team holds authority over another person's work or
  account, so there is no permission here for anyone to contest. What a person needs instead is standing to answer an act
  that touched their work, and to know whom they are answering: the participant who made the act,
  and, for an agent, the person who sponsors it. That standing is an OBJECTION: a participant's
  recorded answer to one act. An objection names the single act it answers, never the
  contribution as a whole. It is kept in the contribution's record beside the acts but is not
  itself an act, so it does not change the contribution and is not counted among its acts. It is
  shown to the participant who made the act and, when that participant is an agent, to the
  person who sponsors it.

  Background:
    Given Matthew has claimed the name "human:matthew" in the team's shared memory
    And Jessica has claimed the name "human:jessica" in the team's shared memory
    And James has claimed the name "human:james" in the team's shared memory
    And Matthew has declared in the team's shared memory that he sponsors an agent claiming the name "agent:editor@model-one"
    And James has declared in the team's shared memory that he sponsors an agent claiming the name "agent:tidier@model-two"
    And Matthew has authored a contribution titled "How we keep the meal rota"

  Scenario: An agent's edit is the agent's act, and Matthew stays the author
    When "agent:editor@model-one", sponsored by Matthew, edits the contribution "How we keep the meal rota"
    Then the contribution's record holds two acts in order: authored by "human:matthew", then edited by "agent:editor@model-one"
    And the edit act names Matthew as the agent's sponsor
    And the edit act names "model-one" as the model the agent ran on
    And the contribution's author is still "human:matthew"
    And no act in the record names Matthew as the one who edited it
    And no act in the record carries Matthew's email address

  Scenario: Every further edit adds one act and never rewrites the author
    Given "agent:editor@model-one", sponsored by Matthew, has edited the contribution "How we keep the meal rota" once
    When "agent:editor@model-one" edits the contribution again
    And the shared memory is read again from the repository files
    Then the contribution's record holds three acts, the first still authored by "human:matthew"
    And the third act is the second edit by "agent:editor@model-one", so that edit was written to the files and not held only in memory
    And the contribution's author is still "human:matthew"
    When the shared memory is read again from the repository files with nothing changed
    Then the contribution's record still holds three acts

  Scenario: An edit that no participant claimed is shown as unclaimed, never as the sponsor's
    When the file of the contribution "How we keep the meal rota" is changed directly by software that has claimed no name
    And the shared memory is read again from the repository files
    Then the contribution's record holds two acts, the second an edit whose participant is "(unclaimed)"
    And the edit act does not name Matthew as the participant

  Scenario: An agent no one has declared sponsoring cannot act, and a later declaration does not reach back
    Given an agent has claimed the name "agent:summariser@model-three" and no one has declared sponsoring it
    When that agent tries to edit the contribution "How we keep the meal rota"
    Then the edit is refused because the agent has no sponsor
    And the contribution's record still holds one act, authored by "human:matthew"
    When Jessica then declares in the team's shared memory that she sponsors "agent:summariser@model-three"
    Then the contribution's record still holds one act
    And no act in the record names Jessica as a sponsor

  Scenario: The author's review is not independent, even when someone else's agent made the edit
    Given "agent:tidier@model-two", sponsored by James, has edited the contribution "How we keep the meal rota"
    When Matthew reviews the contribution
    Then the review is refused as independent, naming Matthew's authoring as the only part he took
    And the refusal does not name the edit by "agent:tidier@model-two" as a part Matthew took, since James, not Matthew, sponsors that agent

  Scenario: The sponsor of an agent that edited the contribution cannot review it as independent
    Given "agent:tidier@model-two", sponsored by James, has edited the contribution "How we keep the meal rota"
    When "agent:tidier@model-two" reviews the contribution
    Then the review is refused as independent, naming the agent's own edit
    When James reviews the contribution
    Then the review is refused as independent, naming the edit made by "agent:tidier@model-two", which he sponsors
    When Jessica reviews the contribution
    Then the review is recorded as independent, made by "human:jessica"

  Scenario: Jessica answers an agent's change, and the answer reaches the sponsor who answers for it
    Given "agent:editor@model-one", sponsored by Matthew, has edited the contribution "How we keep the meal rota"
    When Jessica records an objection to the agent's edit
    Then the objection names the agent's edit act, not the contribution as a whole
    And the objection is shown to Matthew as the agent's sponsor
    And Matthew's own authoring act is not marked as objected to
    And the contribution's record still holds two acts
    And the contribution's content is the same as it was after the agent's edit
