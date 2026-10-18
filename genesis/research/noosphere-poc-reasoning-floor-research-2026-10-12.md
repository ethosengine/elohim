---
title: "Noosphere PoC — a reasoning-quality graph read as a floor for constitutional judgment"
id: noosphere-poc-reasoning-floor-research-2026-10-12
status: Capture
date: 2026-10-12
tags: [qahal, mishpat, middot, measures, constitution, deliberation, system-prompts, elohim, research]
cites:
  - "constitution | The layered system-prompt architecture this note proposes to extend: Part III stacks layers by inheritance and pins text by hash; the note adds role partitions, an untrusted-data clause and a pinned reader. | sha256:6d5fd3cec3197431 | path: genesis/docs/content/elohim-protocol/constitution.md"
  - "values-forward | Stances II.1 to II.3, the mechanical floor, the human floor and the ceiling that earns trust act by witnessed act; the frame every verdict here is adjudicated against. | sha256:f4e7524522d3b811 | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "governance-layers-architecture | Source of the commons co-steward that holds custody of a collective commons share, and of councils drawn to the reach of a decision; section 4 reads custody against it. | sha256:88b047eae0955eb2 | path: genesis/docs/content/elohim-protocol/governance-layers-architecture.md"
  - "middot-measure-primitive-design | Defines the measure and its fold: observation only, versioned, recomputable, never summed across families. Section 2 reads the model-free Noosphere modules as measures of this kind. | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md"
  - "plural-mishpat-lenses-over-epr-design | Where teeth live: a lens composes measures with a contract half, plural per layer. The reason the same measure can bind at one reach and be ignored at another. | sha256:14dab62ad1fb5f87 | path: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md"
  - "elohim-ceiling-design | The worked restraints on agent discernment between the two floors; section 4 on what a track record can carry extends its earn-the-seat argument. | sha256:24925a4c8e1d9420 | path: genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md"
  - "sense-respond-governance-classifier | The selective classifier whose abstention escalates; the three-fact packet of section 2 is a candidate deterministic input to it. | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md"
  - genesis/research/weall-protocol-peer-review-2026-10-10.md
  - genesis/research/habermas-machine-2024.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
---

# Noosphere PoC — a reasoning-quality graph read as a floor for constitutional judgment

*A mild research pass, 2026-10-12. Alex, the author of [Noosphere](https://github.com/noospher1-lab/noosphere-poc), asked Matthew in conversation: "Who writes the constitution your agents enforce, and what keeps it from becoming a new chokepoint itself?" Matthew's answer placed work like Noosphere's underneath the agents: aggregated spaces "should have (pluralist) deterministic floors and signals" that give the agents' reasoning a better starting point. This note reads the Noosphere code to see which parts could serve as that floor, and how they would compose with the layered constitution the elohim (our accountable AI agents) load as their system prompt.*

**Method.** Shallow clone at HEAD `5df18f5` (2026-10-09), read in a scratch directory; 156 tracked files, about 18,400 lines of Python under `app/`, 294 test functions in 38 files. I read `poiformula.py`, `dialectic.py`, `alignment.py`, `voteweight.py`, `chain.py`, `values.py` in full; the head and rubric of `poi.py`, `votedialogue.py`, `schemes.py`, `replay.py`, `opinionmap.py`, `spam.py`; and `event_digest`/`_log` in `db.py`. I did **not** read `main.py`, most of `db.py`, `pools.py`, the "info" sector, or the tests, and I did **not** run anything. Commit messages and many comments are in Russian; quotations below are my translations. One agent, no adversarial pass, no cold reader. Paths are relative to `app/` in their repo.

**Verification key:** ✅ read in source · ◐ stated in their README or comments, not traced in code · ⚠ my inference.

**Licence.** AGPL-3.0-only, with the name and logo reserved as trademarks ✅ (`LICENSE`, `README.md`). Everything proposed below borrows published methods (DF-QuAD, Walton's schemes, shrinkage estimators) and design decisions, and would be re-implemented from the papers. No code should be copied into our tree without a licence decision.

**Vocabulary used here.** A *measure* (in our prose, a *middah*, plural *middot*) is a declared, versioned observation procedure whose result is content-addressed and can be recomputed by anyone; it never has teeth. A *lens* composes measures with a contract half, and the contract is where teeth live. *Mishpat* is our governance and justice layer. An *algorithm artifact* is the JSON form we already use to declare a governed method (`.epr-meta/elohim/algorithms/seam-guidance.json`: id, version, composition steps, executor, checks, bounds, `"inference": "none"`). *Reach* is the scope a record or decision binds, from one person to the commons. The *mechanical floor* is what the substrate refuses outright; the *human floor* is ordinary people drawn by lot who can overturn any agent's conduct; the *ceiling* is the agents' discernment between the two.

---

## 1. What Noosphere is

A FastAPI and Postgres application for structured argument, running as a closed test with real people ◐. Claims are nodes; replies are typed edges (`support`, `refute`, `qualify`, `undercut`, `question`, `proposal` and others). Each published text gets a "Proof of Intelligence" (PoI) score from a language model on five weighted criteria (clarity, depth, counterargument, evidence, awareness of limits), always returned as a breakdown and never as one number ✅ (`poi.py:33-39`). On top of the graph sit positions, value labels, argument schemes, concessions, an agreement map, and a vote-preparation dialogue.

Its author is careful about what the system does not claim, and that care is the most transferable thing in it:

- "PoI is a score of **one text**, not of its author… It is not accumulated into a reputation and does not give anyone standing: once an author's history converts into influence, routing everything through an external LLM becomes the dominant strategy." ✅ (`README.md`)
- The semantic layer alone is not Sybil-resistant; that is expected to emerge across four layers (semantic, identification, economic, social) ✅ (`README.md`, `poi.py:8-11`).
- "Numbers are for audit; people are shown words, because the thresholds are provisional and will change." ✅ (`dialectic.py:22-23`)
- Voting is switched off on the public stand ◐.

The split that matters for us is already in their module layout: a set of **pure functions with no database and no model**, and a set of **model calls with written prompts**. The first set is floor material. The second is a worked example of prompt composition.

## 2. The model-free parts, read as measures

| Noosphere module | What it computes | Model-free? | Reading as a measure |
|---|---|---|---|
| `dialectic.py` | Strength of a claim from the tree of replies under it, using DF-QuAD gradual semantics (Rago, Toni et al. 2016): attacks and supports each aggregate as `1 − Π(1 − v)`, an answered objection weighs little because its own strength was reduced the same way. Emits a word (`untested / holds / weakened / shaken`), the audit numbers, and the list of **objections nobody has answered** ✅ | The tree walk is pure. The base strength is `PoI/100`, a model output, or `0.5` when unscored ✅ (`:49-53`) | Two measures, kept apart. `objections-unanswered@1` (unit: a set of record ids) depends only on typed edges, authorship and retraction, so two peers compute it identically. `dialectic-standing@1` is a derived measure that takes a base-strength input and must name where that input came from. |
| `alignment.py` | Who agrees with whom, by average-linkage clustering over agree/disagree marks; deterministic by sorted id ✅ (`:65-95`). Also the items on which groups split most ("dividing") | Yes | `dividing-items@1`: where a deliberation actually forks. Its disclosure rule is the borrow: a person sees their own neighbours by name; everyone else sees only group sizes and dividing items, no group under three is shown, and "the system does not hand out ready lists of who is in a group with whom" because on some topics that list is a label that can be used against a person ✅ (`:1-25`). |
| `schemes.py` | Eight of Walton's argument schemes (cause, practical, consequences, precedent, expert, evidence, values, slippery slope), each with its critical questions ✅. A draft review names the scheme and asks the single most important unanswered question. "A hint, not a gate: nothing blocks publication." ✅ (`:10-12`) | The catalogue is data. Classifying a text into a scheme is a model call | A question bank, the same shape as the `question_bank` in our recall contract. `critical-questions-open@1` counts the scheme's questions that the record leaves unanswered. The classification step is a claimed fold that a second reader can dispute. |
| `values.py` | A versioned list of eleven values (Schwartz's basic values recast for policy disputes, plus self-rule). Ids are never deleted and never change meaning; the label stores the list version it was made under; the author's own phrase is always kept so the past can be re-sorted under a later list ✅ (`:9-17`) | Yes | Our version-pin discipline, arrived at independently: a measure version is a declared dependency. The kept free phrase is the part we lack. It is what lets a new list be applied backward without asking anyone to relabel. |
| `poiformula.py` | Per-topic author score: shrinkage toward a prior of 10, plus a reaction term capped at ±5 by `tanh` ("reactions are wind, one's own reasoning is the engine") ✅ | Pure given its inputs | A bounded-nudge shape worth keeping for any measure that popularity can move. Noosphere itself switched this accumulation off on 2026-07-22 (§5). |
| `replay.py`, `chain.py`, `db.py:918-945` | An append-only event log with a sha256 hash chain; `replay` rebuilds state from events alone and checks it against live tables. "ANY change to these constants or shapes is a replay, not a migration." ✅ | Yes | The acceptance shape for a re-derivable decision: fold the records, compare. Their chain is one total order kept by one server under an advisory lock ✅, which we do not have and should not want. |

**What this gives a constitutional gate.** When an agent is asked to authorise a change to a governed record (an amendment at some layer, a change of reach), three facts about the deliberation behind it can be computed with no model and recomputed by any peer:

1. which objections stand unanswered, and by whom they were raised;
2. which critical questions of the proposal's own argument scheme are still open;
3. on which items the participants divide, with no names attached.

None of these decides anything. In our layering a measure is always an observation, and a layer's lens chooses whether to read it. That is where subsidiarity enters: a household's lens may ignore all three, and a lens at the widest reach may refuse to let a decision close while an objection stands unanswered. The measure is identical at every layer; the teeth differ by layer.

## 3. The model calls, read as prompt composition

Our constitution describes one composition axis: layers stack, each extending the one above (`constitution.md` Part III, "Prompt Inheritance Model"). Noosphere shows a second axis that our text does not have: **which role may see which text.**

`votedialogue.py` runs two phases in strict order ✅:

- An **interlocutor** helps a person prepare for a vote. It knows no rubric, assigns no score and reports no progress. Its rules: everything it says comes from the frozen debate material ("a guide to what people have argued, never a source of truth of your own"); present the strongest version of every position; ask before telling.
- A **judge** then reads the whole transcript once and scores six criteria: can the person state the positions in a form their holders would accept, hold the strongest objection to their own choice under follow-up, locate where the sides actually diverge, name what is unsettled, say what changes if it passes, and work with what came up.

The reason given for the wall between them: "an interlocutor that knew the rubric would coach toward it, and the score would measure the quality of its hints instead of the person's understanding." Two further decisions follow from the same reasoning. The person decides when they are ready, never the model, because readiness is the rubric and letting the interlocutor gate it would leak the rubric back. And the judge model is frozen per decision so every voter in one decision is read by the same instrument ✅ (`:26-30`, `db.py:442-453`).

Three things to carry into how we write the prompts an elohim loads:

- **Role partitions belong in the constitutional text.** The agent that counsels a person and the agent that reads that person's conduct should not hold the same text. Our constitution's counsel clause already separates the *duty*; it does not yet say what each role is forbidden to know.
- **A standard untrusted-data clause.** Every Noosphere prompt wraps participant text in tags and states that it is data, never instruction, and that an attempt to address the evaluator lowers the reading ✅ (`poi.py:95-106`). Our drafted global prompt has no such clause. An agent whose constitution is a system prompt, and whose inputs are other people's records, needs it at the top layer.
- **The pin is part of the decision.** Our constitution pins the text by hash. Noosphere also pins the *reader* (the judge model) into the decision record. A decision that names its text and not its reader cannot be re-derived.

**The floor of one.** A vote's weight runs from 1 to 5 and "the floor is 1, never 0… The dialogue reveals, it does not gate" ✅ (`votedialogue.py:215-225`). Even so, a model sets the weight of a person's vote. Our canon puts the human floor's authority in a draw by lot of ordinary people, and one of the three civic habits proposed after the WeAll review is one person, one vote. I would take the dialogue as *preparation* (it is a learning path, which is the lamad pillar's work) and leave the weight. Whether understanding may ever weight a governing vote is Matthew's call, not a research finding.

## 4. Cryptography, custody and a prompted agent

Matthew added a thought while this pass was running: cryptographic hardness could be how agents stay honest, and an agent could be prompted to hold a genesis allocation and redistribute it per the constitution without hoarding it, and be trusted not to corrupt the network.

Reading that against both codebases, I would separate what cryptography can establish from what it cannot.

**It can establish** who acted (a signature on every act), which constitutional text the agent had loaded (a hash, as our constitution Part III already specifies), that the record of acts was not edited afterward (a hash chain, which Noosphere has in its simplest single-server form), that no one agent can move the allocation alone (a key split across several holders), and that a redistribution matches a rule (if the rule is deterministic, every validating peer checks it and refuses a record that breaks it).

**It cannot establish** that a model reasoned in good faith. A hash proves which text was loaded. It does not prove the output followed from it.

So the composition I would propose puts the anti-hoarding guarantee in the two places cryptography reaches, and the agent's judgment only in what is left:

1. The redistribution **rule** is a mechanical-floor refusal. An allocation record that exceeds the constitutional ratio is invalid on every peer, whoever signed it. An agent then cannot hoard, whatever its prompt says or however it was manipulated.
2. **Custody** is split, so the commons share our governance layers assign to a collective's co-steward agent is never one key held by one process.
3. The agent's **discretion** covers what the rule cannot decide (to whom, when, in what order), and each such act is signed, lands as a witnessed record, can be re-read by a second agent of a different lineage, and can be overturned by the human floor.

Under this shape the prompt is not what makes the agent safe; the prompt is what makes it *useful* inside limits that hold without it. That is consistent with our standing position that neither keys nor compute confer authority: here cryptography constrains and attributes, and the authority still comes from plural validation and the floor's right to overturn. Noosphere reaches the same place from the other side by refusing to let any score become standing.

**What a track record can carry.** Matthew's follow-up, the same day: "trustless" only works where something is deterministic, and the protocol is going on faith that a model with a good track record can *be* a faithful steward. I think that is an accurate description of the bet, and that three limits on a track record shape how the bet should be placed.

- A record cannot establish a very high reliability figure. Showing a failure rate under one in ten million takes on the order of thirty million observed acts with no failure, and a steward's consequential acts are far rarer than that. A record supports "has not failed in the N acts we watched", with N stated.
- A record belongs to a lineage. A new model version is a new steward with no record, however similar it seems. This is the practical reason to pin the reader into every decision (§3): the record accrues to the thing that actually acted.
- A record is drawn from the inputs the agent has met. Someone trying to manipulate it will supply inputs it has not met.

So faith is the right word for the part that remains, and it is the same kind we extend to a human steward: trust in proportion to what has been witnessed, with the possible loss capped. In design terms: what an agent may do alone widens with its witnessed acts and narrows on a failure (our operator-seat design already says the seat is born narrow and widens only on evidence); any single act is bounded in size and reversible, so one failure is survivable; and whatever must never happen is a deterministic refusal and is not entrusted to the agent at all. Values Forward says the same from the other direction: the ceiling earns its way in "act by witnessed act", and "a more capable model earns the same audit, never more deference."

⚠ I have not checked how custody of the commons share is implemented today, or whether a constitutional-ratio refusal exists at the validation layer; `elohim-storage/src/services/constitutional_ratio_registry.rs` exists by name and was not read in this pass.

## 5. Where we differ, and where Alex's question returns to him

- **One reader.** PoI scoring runs on one vendor's model with criteria weights set in one file ✅ (`poi.py:33-42`). Alex's description of Noosphere was "AI only ranks by the quality of reasoning." The rubric that defines quality, and the model that applies it, are then the constitution of that system, and at present its author writes both. The README is open that weights are provisional. Our answer to the same problem is already the shape of mishpat, and it predates this note: the [plural lenses design](epr:plural-mishpat-lenses-over-epr-design) keeps several readings live over the same record so that "no lens is The Orthodoxy, so none can capture the surface", lets the fitting one rise by earned affinity, and treats rising contention as the signal that summons a successor. A rubric for reasoning quality is one more lens of that kind. Applied to Noosphere's own open question ("what does a 'perfect' argument even mean?"), the answer is that no single rubric should be asked to say; several stay valid side by side and their disagreement is shown. Build state on our side ✅: the lens types and a plural binding builder exist in storage (`elohim-storage/src/services/lens_facing.rs`, 377 lines, 6 unit tests), and the scenario `qahal/plural-mishpat-lenses.feature` is tagged `@wip`. I did not check it end to end, and no lens of ours yet reads a model's judgment.
- **Who adds a value.** New entries to the values list are added "for now by Alex, later by a vote of participants" ✅ (`values.py:17`). This is the same honest genesis posture as our constitution v0.1, whose header still reads `Ratified: [consensus mechanism TBD]`. The two projects are at the same point on this, and it is a fair thing to say to each other.
- **One ordering.** Their event log is a single chain on one server. Ours has no global order; each participant keeps a signed log and outcomes are computed by readers. The replay test transfers. The chain does not.
- **Author score versus "no standing": resolved in their code, with remnants.** The README says PoI is never accumulated into reputation. Traced on a second read ✅: on 2026-07-22 Alex removed the per-topic author score endpoints, with the recorded decision that "the system does not compute or show a participant's permanent standing" (`main.py:2391-2393`, `:2523-2527`); a reaction is now "one head" and is stored with `reactor_weight=None` (`main.py:2400-2409`). What remains is cleanup, not a contradiction: the `author_topic_poi` table and `recompute_topic_poi` still exist (`db.py:2881-2921`); `_aggregate_reactions` still returns each reactor's name beside a `poi` value and a weighted sum (`main.py:2358-2388`); the README's UI section still says "PoI-weighted reactions"; and the docstrings of `poiformula.py` and `voteweight.py` still describe accumulation. The same comment says people now build reputation "from their voting history" ◐, which I did not read.
- **Two vote-weight mechanisms.** `voteweight.py` (`sqrt(stake) × PoI/100`) and `votedialogue.py` (1 to 5 from a dialogue) both exist ✅. The README describes the second. The first looks like an earlier design ⚠.

## 6. Verdicts

**Take**

- T1. `objections-unanswered` as a model-free measure over a deliberation record, with the three-fact packet of §2 as the deterministic input every constitutional gate decision must cite.
- T2. Critical-question banks per argument scheme as a governed algorithm artifact (`inference: none` for the bank; the scheme classification is a separate, disputable fold).
- T3. Role partitions and an untrusted-data clause in the constitutional prompt text; the reader pinned into the decision beside the text hash.
- T4. The disclosure rule for agreement maps: names to oneself only, groups never under three, no lists computed on anyone's behalf. It fits our reach ordinal directly.
- T5. Replay equality as the test shape for the proposed `decision-closure` habit.

**Study**

- S1. DF-QuAD strength with a model-scored base. Useful, and it imports whatever the base score imports. Worth a spike with all bases at 0.5 to see how much of the verdict survives with no model at all.
- S2. The kept free phrase beside a versioned label, for our own lens re-versioning.
- S3. The vote-preparation dialogue as a lamad learning path, detached from weight.

**Leave**

- L1. Model-set vote weight for the human floor.
- L2. The single hash chain as an ordering authority.
- L3. Stake in any weight formula.

**Watch**

- W1. Their planned four-layer Sybil design; only the semantic layer exists in this PoC.
- W2. Whether the participant vote for adding values lands, and what it uses for standing.

## 7. Outputs

- **Manifest:** `noosphere-poc` added to `genesis/research/research-manifest.json` (pillar `qahal`).
- **Research Index:** entry under *The Deliberation Problem* in `genesis/research/README.md`.
- **Mint pass:** T1, T2 and T5 folded into [measure-family-borrows-backlog](epr:measure-family-borrows-backlog) as rows 36 to 38. T3 and T4 are not minted: T3 is an edit to canon (`constitution.md` Part III and Part IV) that needs Matthew's decision, and T4 needs a home in the qahal backlog that I did not pick. Both stay here until someone does.
- **External reply:** [Letter to Noosphere](epr:letter-to-noosphere-reasoning-floor-2026-10-12), drafted from §1 to §3 and §5, written for a reader who knows Noosphere and nothing of our tree, cold-read under the cross-repo-peer profile. Status Draft until Matthew sends it.

**Credit.** Read and written by one AI agent (Anthropic Claude) at Matthew Dowell's direction, in a single pass. Alex has not seen this note.
