---
title: "Letter to Noosphere — what we read, where we think you are right, and what we would like to take"
id: letter-to-noosphere-reasoning-floor-2026-10-12
status: Draft
date: 2026-10-12
derived-from: noosphere-poc-reasoning-floor-research-2026-10-12
---

# Noosphere: a reading from Elohim

Alex,

In your introduction you asked everyone to tell you where you are wrong, and you asked me what I think of your graph. This letter does both.

**What we read.** We cloned `noosphere-poc` at commit `5df18f5` (9 October 2026) and read the modules that carry the design: `poiformula.py`, `dialectic.py`, `alignment.py`, `voteweight.py`, `chain.py` and `values.py` in full, and the central parts of `poi.py`, `votedialogue.py`, `schemes.py`, `replay.py`, `opinionmap.py` and `spam.py`. We did not read `main.py` or `db.py` from start to end; in those two files we read only the parts about reactions, the author score and the event log, and those are the lines cited below. We did not read the information sector or the tests, and we did not run anything. Dates of your changes come from your own comments in the code, not from the commit history. So this is a reading of your design decisions, not of how the system behaves with people in it. File references are relative to `app/`.

**Who is writing.** Elohim is a learning and community platform built on Holochain, a peer-to-peer framework with no central server and no global chain: each person's device keeps its own signed log of records, and peers check each other's records against shared rules. Every record carries a scope: private, one household (a family or a few people who share devices), a wider community, or everyone. So a rule can apply to a few people without being visible to all. Each record also states three things about itself: what it is, what it is worth to whom, and who has a say over it. The site you saw, https://elohim.host, runs on this network.

## Where we think you are right

These are decisions we think are right, and that we have not often seen made.

1. **A score belongs to one text, never to its author.** Your README gives the reason exactly: if a person's past scores turn into influence, everyone will let a language model write for them. And you acted on it. Your comments date the change to 22 July: you removed the per-topic author score and wrote that the system "does not compute or show a participant's permanent standing" (`main.py:2391-2393`, `:2523-2527`), and a reaction now counts, in your words, as «одна голова», one head, whoever gives it (`main.py:2400-2409`).
2. **The AI that prepares a voter never sees how the voter's answers will be scored.** Your reason, in `votedialogue.py`: if the AI that talks with the voter knew the scoring criteria, it would lead the person toward them, and the score would measure the AI's hints and not the person's understanding. You carried this through: the person decides when they are ready, never the AI, and your comment gives the reason (`votedialogue.py:39-41`).
3. **You protect people from the map.** A person sees who they agree with. Nobody else is handed a list of who stands with whom, and no group smaller than three is shown (`alignment.py:1-24`). Your comment gives the example yourself: on a topic like language in Ukraine, such a list is a label that can be used against a person. Your seed topics are about the war and its causes. On questions like those, this rule matters more than any score.
4. **People see words; the numbers are kept for checking** (`dialectic.py:22-23`). And every vote counts at least one: "the dialogue reveals, it does not gate", meaning it never blocks anyone from voting (`votedialogue.py:215-225`).
5. **You correct from use.** Your comment at `dialectic.py:40-46` records that a "no" under a question was being counted as an attack on the question, so the result looked like the platform's own judgment, and that you changed it.

## Where we think you are wrong, or not yet right

**1. "People with the same information usually arrive at the same decision."** Your own code disagrees with this sentence, and we think the code is right. `values.py` opens by saying people often differ not on facts but on what matters more, and you built a values layer because of it. If that is true, then some disagreements will remain even when everyone has the same information. So the question after "how do people agree" is "what do people do when they do not": who may decide what, for whom, and how the people who disagree with the result can still accept it. You may have decided this is outside Noosphere for now. It is the question our project started from, so it is where I think we can be useful to you.

**2. Your calibration question has no single answer, and we think you should stop looking for one.** You asked what a "perfect" argument even means. Today the answer in your system is five criteria with weights in one file (`poi.py:33-39`), read by one company's model. You asked me who writes the constitution that my AI agents enforce, and what stops it becoming a chokepoint. The same question applies to Noosphere: your scoring criteria are your constitution, and for now you write them.

Our answer, which is only partly built, is to have no single set of criteria. Several sets, written by different people, score the same text side by side, and none of them is the official one. A community uses the set its members keep choosing. When people start disputing a set's results more and more, that is the signal that a new set is needed. Under that design your five criteria are one good set among several. The person sees where the sets disagree; the scores are never combined into one number that hides the disagreement.

**3. A model sets the weight of a person's vote.** The floor of one makes this far gentler than most weighted schemes, and the rubric reads understanding, not agreement. It is still a machine deciding how much a person's voice counts in a decision that binds them. Our design separates the two. The AI helps people understand and shows its reasoning. The final say belongs to ordinary participants chosen by lot, one person and one equal vote, and they can reverse any decision an AI made. Your preparation dialogue is the best version of that help we have seen. In your place we would keep the dialogue and drop the multiplier. You may have good reasons not to, and we would like to hear them.

**4. One server.** Everything that makes Noosphere trustworthy today (the event log, the replay, who can clear a spam mark, who adds a value) depends on trusting you. At this stage that is a reasonable choice. But "radically transparent" is a claim that other people must be able to check without asking you. One cheap step would be a public export of the event log for a discussion, so that someone else can calculate your numbers again.

**5. Small things you may want to tidy.** After the July change the README's UI section still says "PoI-weighted reactions"; `_aggregate_reactions` still returns each reactor's name beside a score and a weighted sum (`main.py:2358-2388`); the opening comments of `poiformula.py` and `voteweight.py` still describe accumulation; and `voteweight.py` (the square root of a stake, times the score, `voteweight.py:49-52`) sits beside `votedialogue.py` (one to five from a conversation) as a second weighting scheme that the README no longer describes. None of this is a defect in behaviour that we could see. These are places where a reader of the code will think you still do something you decided to stop doing.

## Your question to us, answered honestly

Who writes our constitution? At this moment, I do, as you do yours. Your values list says new values are added "for now by Alex, later by a vote of participants" (`values.py:17`). Elohim has a draft constitution: a plain-language text of values and limits that our AI agents load as their instructions, in layers from one person up to everyone. Its top layer still carries the line "Ratified: [consensus mechanism TBD]". We are at the same point as you. Most of our governance is written design and not running code: the groups chosen by lot are specified and not built, and our tests for collective decisions are still marked unfinished. What runs today is the peer-to-peer network, the record format, the learning paths you saw on elohim.host, and the first part of the several-criteria design from point 2.

## What we would like to take, with your leave

Four ideas from your code are things we want to give our AI agents as inputs: facts that any computer on the network can calculate again and check, with no language model involved.

- **which objections under a proposal nobody has answered**, from the reply types alone (support, refute, undercut);
- **which critical questions of the proposal's own argument scheme remain open**, from your list of schemes and questions in `schemes.py`;
- **the disclosure rule for agreement maps**: names to yourself, groups never under three, no lists computed for anyone else;
- **replay as the test**: two independent parties fold the same records and must get the same result.

Your code is AGPL. Ours is licensed package by package and has no single licence yet. So we would not copy your code. We would write these again from the published sources you used (Rago and Toni's DF-QuAD paper of 2016; Walton, Reed and Macagno, *Argumentation Schemes*, 2008) and credit Noosphere for the design decisions around them. If you would prefer a different arrangement, tell us.

## What we owe you

You offered to go through our learning paths as a non-technical person and tell me where you got lost. Yes, please; they start at https://elohim.host/lamad. I will do the same on your test site when the English interface is ready. Beyond that, I would be glad to work on any of this together, in whatever form suits you.

Tell us which of this you dispute. In particular, if we have misread the July change or the status of the two weighting modules, please correct us. We wrote a longer note for ourselves while reading; you can have it if you want it.

Слава Україні.

Matthew Dowell
Elohim Protocol, https://github.com/ethosengine/elohim

*This letter was drafted with an AI agent (Anthropic's Claude) under my direction, from one reading of the code. Every file reference was opened in source before sending. Nothing in your repository was run.*
