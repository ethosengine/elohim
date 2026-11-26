---
title: "Memorilla and Claude's dreaming — a learned memory and a scheduled consolidator, read from living memory, the EPR and the attunement limit"
id: memorilla-claude-dreaming-cross-pollination-2026-11-18
status: Draft (v2, single author, steward-corrected on the attunement reading, no challenge pass)
version: 2
date: 2026-11-18
revised: 2026-11-18 (v2, same day — the steward corrected v1's reading of "never pooled" as a rule about where bytes live; the limit is on the join. Memorilla moved from LEAVE to STUDY with a path to the same utility; §4 re-cut around the join; see Method)
cites:
  - "elohim-protocol-manifesto | principle 6 (living memory) and Witnessed Humanity, the source of the attunement limit this note reads both systems against | sha256:21e5057f62b72ed3 | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "private-thought-governed-fruit | the ruling that thoughts are private and never enter a notarized plane | sha256:2945eb608257be1e | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "memory-lifecycle-design | the consolidation-event section and the unbuilt proposal-only /dream that verdicts T1 and T2 take up | sha256:6a3557a18e2ceb09 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md"
---

# Memorilla and Claude's Dreaming — November 2026

Two things arrived in the same month that bear on how we keep agent memory. **[Memorilla](https://github.com/snap-stanford/memorilla)** (Stanford SNAP, [arXiv 2610.21375](https://arxiv.org/abs/2610.21375)) replaces retrieved text with sixteen learned vectors. **Dreaming** is Anthropic's name for a scheduled pass in which Claude rewrites its own memory files from past sessions; it now exists in three forms, one of which is inside the Claude Code binary this workspace runs. This note reads both against three questions the steward raised on 2026-11-18:

1. Why is our memory ceremony hard to prioritise, and is that a bias or a signal?
2. We now run several workspaces. Which channel should carry memory between them?
3. Where does a person's own memory (imagodei) end and crafted, shared memory context begin?

The short answers, argued below. (1) It is a signal: we run consolidation as cleanup, and the manifesto says cleanup is the wrong frame, so the work produces nothing a person can see. (2) The network may hold the bytes; what a channel between workspaces must carry carefully is the *join* that ties bytes to a person, and each entry must state its own reach. (3) The boundary is drawn by who may join what, from where and for what purpose. It is a shape with several dimensions, and our memory directory records none of them yet.

One aim stands over the Memorilla reading: **the protocol should reach the same usefulness Memorilla demonstrates, by means that keep the join with the person and put what is shared into commons-stewarded form.** §1 sets out that path.

**Verification key:** ✅ read directly in source or in a primary document · ◐ inference, or text read from the installed binary that no official page documents · ⚠ press or third-party blog · ✎ the steward's direction given in this session, not yet found in a canon document. **MR** = `genesis/research/repos/memorilla/` at commit `6864db7`. **CC** = the installed Claude Code, version 2.1.327. **M** = `genesis/docs/content/elohim-protocol/manifesto.md`. **LM** = `…/living_memory/epic.md`. **RE** = `…/resilience/README.md`.

**Prior coverage.** The [seams note](epr:local-first-to-council-memory-search-seams-2026-09-12) refused OpenAI's dreaming and called silent consolidation an injection vector. The 2026-09-06 horizon scan read Anthropic's Dreams API and noted that it offers no measure of benefit. The 2026-05-10 memory lifecycle design proposed our own `/dream` skill, proposal-only, which was never built. No document mentions Memorilla. This note extends those three; it does not re-verdict them.

---

## 0. The ground this is read from

Six commitments do most of the work in this note. They are stated here once so the later sections can refer to them.

**G1. Memory has the character of life.** "It grows where tended, fades where not, heals where faced, coalesces where it recognizes itself in kin" (M, principle 6). "*Most data is supposed to fade.*" (LM, comet shape). The two refused alternatives are "preserve everything forever" and "delete on demand". Memory classes (contextual, archival, identity, relational, operational, attestation, wisdom) each get the lifecycle that fits them; an attestation "can be superseded, but it can never be forgotten" (LM).

**G2. Consolidation is judgment.** "When two records turn out to be about the same thing, [most systems] treat the recognition as a cleanup task… In the Elohim Protocol, that recognition is a moment of *judgment*, and judgment carries flow… The protocol surfaces all of this. None of it is silent." (LM). Each consolidation discovers a truth, shifts something economic (the noticer is rewarded; freed capacity is redistributed), updates relationships, adjusts reach and records a governance moment.

**G3. What loses reach goes somewhere.** It submerges to a tier its steward's elohim can still see; it is released only with "a small audit record of what was released and why, so the trajectory of your forgetting is itself walkable backward" (LM). The question the protocol asks of data is "*who is going to hold this well?*"

**G4. The attunement limit is a limit on the join.** An elohim comes to know a person through what they say and what they show. "Neither of these is you. Under both is… your *imagodei*… something in each of us that nobody makes and nobody owns, not even you." The picture an elohim builds "is talked through with you. It is always shown to you so you can push back on it." The limit: that picture "is, pooled across a network, the most dangerous object this protocol could produce, so it is never pooled… never gossiped, never joined by an outside observer, and never imported into anything that judges. Only what a person says and does in shared space is judged" (M, *Witnessed Humanity*).

Version 1 of this note read "never pooled" as a rule about where bytes live. The steward corrected that ✎, and G5 below shows the canon agrees: the dangerous object is the *assembled* picture, bytes joined to a person and to each other, in the hands of someone who was not given it. The operative words are "joined" and "judges". Two more points from the same section and from the steward ✎: the picture is a mixture of self and community, because humanity is "witnessed, never self-declared" and witnesses sign part of it; and what would be surveillance elsewhere is *witness* here because its purpose is constitutionally bounded to flourishing, negotiated with the person, respectful of attention and protected by the network. The companion ruling still holds for an agent's own reasoning: "Thoughts are private… Thoughts never enter a notarized plane" (`genesis/docs/architecture/private-thought-governed-fruit.md`).

**G5. Custody is separate from access.** Resilience requires that private data be held by others. The resilience epic names three postures of holding (RE, Part V) ✅: *encrypted custody* ("Private shards for people you back up. You cannot read these… the steward's elohim agent does not have the decryption material"), *social custody* (the group's governance decides who reads) and *commons custody* (anyone reads; "attribution travels with the bytes"). The same passage says custody tiers are "a distinct axis from the reach vocabulary". The steward's direction extends this ✎: opening private data should need more than one gradient, each ignorant of the others, such as the person privately and a decontextualised reference. A person can hold a video about their own medical condition whose bytes they also help replicate for the commons, so long as nothing held by others ties those bytes to them.

**G6. The EPR's three legs, reach earned at compose, and the bounded elohim.** Every link carries knowledge, value (who stewards it and how they are recognised) and governance (its reach and the authority behind it). "You cannot create a link without all three… information without governance is propaganda, and value without attribution is extraction." Reach is a position on a ladder (`elohim/epr/src/reach.rs`: Private, Self, Intimate, Trusted, Familiar, Community, Public, Commons) and a link cannot be composed above what its author has earned: the gate answers Allowed, Blocked or Pending. Versions are joined by sealed `supersedes` edges. Serving content emits a recognition event to its stewards. The substrate sets those gates and an elohim may escalate, witness or defer but "cannot unilaterally override" them (RE, substrate floor and elohim ceiling).

---

## 1. Memorilla, mechanically

Memorilla is a learned **read path**. It is not a memory system in the sense G1 uses the word.

- **Write.** A frozen embedding model (Qwen3-Embedding-4B) turns each document chunk into one vector ✅ (`MR/docs/data.md:85`). Nothing else happens at write time. There is no merge, no update, no forgetting, no notion of time and no handling of contradiction. An optional cache precomputes the attention keys and values for a collection ✅ (`MR/memorilla/memory.py:322`); it costs four times the memory of the embeddings and equals the uncached result up to rounding.
- **Read.** Sixteen learned slot vectors attend first to the question embedding, then twice across every document vector in the collection, and are projected into the decoder's embedding width ✅ (`memory.py:223-256`). The sixteen results overwrite the embeddings at sixteen placeholder positions in the prompt of a frozen decoder ✅ (`MR/memorilla/llm.py:12-38`). The module is about 194M parameters (⚠ press figure, consistent with the layer sizes in `memory.py`).
- **Training.** Only the module is trained, on the decoder's next-token loss, in three stages that take about 19 hours on four H100 GPUs and 310 GiB of data ✅ (`MR/docs/training.md`). The third stage fine-tunes on the training splits of all eight evaluation benchmarks, so the headline results are in-distribution ✅. The baselines run under different prompts from Memorilla's own ✅ (`MR/docs/evaluation.md:61`). The numbers are the authors' own.
- **Scoping.** The only boundary is the *collection*: one user's chats, one book ✅. A padding mask excludes documents from a read ✅ (`memory.py:281, 415`).
- **As agent memory.** In TextWorld the module is trained from game reward alone, and each stored document carries the reward its turns earned ✅ (`MR/rl/README.md:39-47`). Average success rises from 18.6% to 24.2% on 40 held-out prompts with three rollouts each, a small sample.

### What it shows is worth having

The README's demonstration ✅: a user once asked whether diabetes on the mother's side raises risk more than on the father's. Later they ask for breakfast ideas, and the model answers with advice suited to a family history of type 2 diabetes. A plain retrieval system would not connect the two. An elohim attuned to a person should. The PersonalizationV4 benchmark measures exactly this: questions that need two facts about a person which their history shows and never states ✅ (`MR/personalizationv4/README.md`). Its 149 personas are fictional, so the reader was trained without assembling any real person's picture. Version 1 called the benchmark "the pooled picture, used as the measure of success"; that was wrong, and it is a fair measure of a usefulness we want.

### Its parts already fall on our lines

Memorilla separates into three parts, and each has a home on our ground:

| Part | What it is | Where it belongs here |
| --- | --- | --- |
| The trained reader | Learned patterns of how facts about a person combine. Shared across everyone who uses it. | A commons artifact, elohim-stewarded, with a CID and named stewards (G5 commons custody, G6). This is where archetype or persona embeddings would live if they are needed: pooled resources that help a person relate themselves to the community they are embodied in ✎. |
| The per-person collection | One vector per document: the material of the picture. | Encrypted custody. The network may hold the shards; the keys and the join stay with the person (G5). |
| The read | Sixteen vectors for one question. | Made at the edge by the elohim the person has given that reach, for that question, and no longer-lived than the question. |

The padding mask is the mechanism that would let two different joins be made over one collection: the elohim at the doctor's and the elohim at grandma's table draw on different documents ◐.

### What is missing from the released code

- **It cannot be shown.** The attention weights that say which documents contributed are computed and thrown away ✅ (`memory.py:252`, `attended, _ =`). So the person cannot be told "I am drawing on your question from March about diabetes on your mother's side; is that about you?", and G4 requires that they can. This is recoverable: the weights exist at the moment of the read ◐.
- **It announces nothing.** The breakfast answer applies the inference without saying so. That is a choice of behaviour around the model, and an elohim would choose otherwise.
- **It has no value or governance leg (G6).** With no attribution, no steward can be recognised for what was served, and the reach of a blend cannot be checked. Exposed attention weights would supply both: allocation ratios for a recognition event, and the list of sources whose reach bounds the answer.
- **Nothing fades (G1).** The store only grows. Memory classes and lifecycles would have to be applied to the collection from outside.
- **It needs a model we can write into.** The reader injects vectors into the decoder's input embeddings and is trained for one decoder and one encoder. Hosted Claude offers neither. This path runs on local open weights on household compute.

### Other ideas worth keeping

- *Write once, read per question.* The cache is a rebuildable projection of the documents, never the record: "the content is synced; the index never is", reached from the other direction.
- *Memories that carry their outcome.* The TextWorld documents record what each action earned. Ours record what was learned and why, never whether acting on the memory later helped. That is half of G2's flow, missing.

---

## 2. Dreaming, in the three forms that exist

### 2.1 The Dreams API (Managed Agents, research preview) ✅

Source: [platform docs](https://platform.claude.com/docs/en/managed-agents/dreams), read 2026-11-18.

A dream takes one memory store and 1 to 100 session transcripts and produces a **new** store: "duplicates merged, stale or contradicted entries replaced with the latest value, and new insights surfaced." "The input store is never modified, so you can review the output and discard it." The dream is itself a session whose events can be watched and whose transcript is kept. Free-text instructions steer the whole pass but cannot make line edits. It is billed at ordinary token rates and is not eligible for zero data retention.

The [memory stores](https://platform.claude.com/docs/en/managed-agents/memory) it works on ✅: a store is a directory of text files mounted into a session. A session mounts at most eight. Each mount is `read_only` or `read_write`, "enforced at the filesystem level". Every change makes an immutable version "attributed to the session", kept 30 days, and a version can be redacted while its audit row stays. The recommended layout is one store per user, one for shared domain knowledge, one per project.

### 2.2 The cookbook workflow ✅

Source: [Dreaming with a dynamic workflow](https://platform.claude.com/cookbook/managed-agents-cma-dreaming-with-a-dynamic-workflow).

A hand-built dream. A lead agent on a mid-tier model writes a workflow and never reads a transcript. One small-model analyzer per transcript verifies existing memory against it and adds to it. One organizer runs alone at the end, merges duplicates and rebuilds the index. Transcripts are "data, not instructions". The sandbox has no network. A nightly cron takes the sessions since the last run. Stated cost: $0.36 for eleven transcripts, about $8 for fifty long ones.

### 2.3 Auto-dream inside Claude Code ◐

No official Claude Code page mentions dreaming ✅ (the [memory page](https://code.claude.com/docs/en/memory) and the docs index were both checked). Third-party posts describe it ⚠. The installed binary settles what exists, though not what is switched on:

- A setting, `autoDreamEnabled`: "Enable background memory consolidation (auto-dream). When set, overrides the server-side default." ◐ So it is on or off by a server flag unless we set it.
- A gate of 24 hours and 5 sessions since the last dream ◐.
- A four-phase prompt (orient, gather recent signal, consolidate, prune and index) that works **in place** on the memory directory, with no copy ◐. It tells the dreamer to delete contradicted facts, to rewrite `MEMORY.md` under its size limits, and to reconcile memories against `CLAUDE.md`.
- A `team/` subdirectory: "memories shared across everyone working in this repo." The dream may merge within it and must prune it conservatively: "DO NOT delete a team memory just because you don't recognize it." And: "Do not promote personal memories into `team/` during a dream — that's a deliberate choice the user makes via `/remember`." ◐
- **Synced project memory**: memory stores mounted into the local memory directory and synced to a server, governed by an organisation policy, with a per-directory write opt-in, read-only mounts, and the rule that "the next sync pull will overwrite local edits with server content." ◐

Separately and documented ✅, [Projects](https://code.claude.com/docs/en/claude-projects) (public beta on individual plans) keep a server-side project memory that every cloud thread reads. It is "separate from the auto memory Claude Code keeps on your machine", and documented auto memory is "machine-local… not shared across machines or cloud environments."

### Read from our ground

**The frame is the one G2 names as the mistake.** The Dreams page opens: a store "accumulates duplicates, contradictions, and stale entries. Dreams let Claude clean that up." Every form treats consolidation as cleanup. None emits anything when two entries turn out to be one, or when an entry turns out to be wrong: no record of who noticed, what was freed, whose entry it was. The API form is the most careful (a copy, a watchable session, attributed versions). The Claude Code form is silent in the sense G2 forbids: it edits in place and returns "a brief summary".

**It deletes where we submerge (G1, G3).** "Deleting contradicted facts" is right for a contextual note and wrong for an attestation, which may be superseded and never forgotten. No form knows memory classes. Versions expire after 30 days, so the trajectory of forgetting stops being walkable.

**Two of its rules are ours.** Other people's entries are pruned conservatively (ours: folding another author's entry needs a non-curator Steward's verdict). Promotion outward is never done by a dream. In G6's terms the second rule is exact: widening an entry is composing it at a higher ring, and that passes the reach gate or it does not happen.

**The sharing model puts bytes, keys and join in one hand.** In synced project memory the server holds the readable copy, its copy wins, and an organisation's policy decides who writes. In Managed Agents, "one store per user" means the operator's workspace holds each person's picture in the clear. Holding the bytes is not the fault (G5). Holding bytes, keys and the tie to the person together is: that is central custodianship, which the resilience epic exists to replace.

**A local hazard follows directly.** Our auto-memory directory is a symlink into the git-tracked `.claude/memory/` ✅, and its `MEMORY.md` is a generated projection ✅. If the server flag turns auto-dream on, a background agent would edit those 320 files in place: rewriting a generated index by hand, deleting entries that carry a steward-of-record mark, with no `supersedes:` edge and no Steward's verdict. Whether our edit-time gate catches a dream's writes is untested. Setting `autoDreamEnabled` to `false` removes the question (verdict T5).

---

## 3. Question 1 — why the ceremony is hard to prioritise

**It is a signal, and G2 says what it signals.** We have been running consolidation as cleanup. Cleanup has no visible product, so it loses to any work that has one.

Our own record agrees. The efficacy analysis (2026-09-09) found the ceremony "improved recall… did not prove lower token cost." The 2026-09-06 scan found nobody in the field can show what a consolidation bought. Today's session headline shows cleanup pressure at 278 against a ceiling of 120, the index 2,454 files behind, 10 of 26 recall journeys failing and the standing reader 55 days stale, and work continued through all of it. Those measures count *debt*. None of them counts what a consolidation *found*.

G2 lists what a consolidation is supposed to produce. Set against what the ceremony emits today:

| G2 says each consolidation… | The ceremony today |
| --- | --- |
| discovers a truth (this equals that; this is wrong) | folds entries; the discovery is not recorded as one |
| rewards the one who noticed | nothing flows to the lens or session that noticed |
| frees capacity, which is redistributed | the index is 19,775 bytes of a 20,000 soft budget; freed bytes are not counted as a gain |
| updates relationships and adjusts reach | no entry carries its own reach, so none can change |
| records a governance moment | a chronicle entry, written by hand |

So the ceremony does the judging and throws away the evidence that judging happened. That is why its value cannot be read.

Anthropic's forms add a second, practical lesson: who does the noticing, and when.

| | Our ceremony | Dreams API / cookbook |
| --- | --- | --- |
| Who starts it | The steward, by choosing to | A schedule |
| Who reads the raw material | Four top-tier lenses | One small-model reader per transcript |
| What the human is asked for | Attention through several phases | One review of a finished result |

Put together, the shape that follows from our canon is neither their dream nor our present ceremony:

- **Noticing is labour, and can be scheduled and cheap.** Small readers go over recent sessions and the memory directory and *propose* consolidations. This is what the 2026-05-10 design's `/dream` already specified and nobody built.
- **Each proposal is a consolidation event, never a silent edit.** It names the truth found, the entries and their stewards, what is freed, and any change of reach. Our valueflow verbs already exist to carry it (`intend`, `claim`, `produce`, `verify`, `fulfil`), so the noticer is on record and the event is a thing `epr flow` can count.
- **Judging stays with those who stand for the entries.** A Steward's verdict where another author's entry is touched; the storyteller's graduate, memorialise and hold; the reach gate for anything moving outward.
- **The result is shown as fruit.** Recall journeys run before and after: "journeys passing: 16 → 21", bytes freed from the head, entries submerged with their audit rows. A dream that moves no journey is evidence too.

The ceremony then becomes the rare, human-paced part, and it is prioritised by what the events show, in the way a red gate is.

---

## 4. Questions 2 and 3 — workspaces, reach and the edge of the person

### 4.1 Three axes, where version 1 had one

Version 1 asked of each memory entry "how far may it travel?" and treated that as the boundary. Three separate questions are in play:

| Axis | The question | Where it is defined |
| --- | --- | --- |
| **Reach** | How far may this be seen? | `reach.rs`; earned at compose (G6) |
| **Custody** | Who holds the bytes, and can they read them? | Encrypted, social, commons (G5) |
| **Join** | Who may tie these bytes to a person and to each other, from where, and for what? | The attunement limit (G4); not yet modelled |

The edge of the person is on the third axis. A person's intimate data can sit, unreadable, on many peers. Parts of it can even circulate in the commons with nothing tying them to that person. What stays with the person is the join.

The manifesto's three layers read this way:

1. **The imagodei.** Nobody makes it and nobody owns it. It is never written down. No memory entry is it or stands for it. A personalisation profile here is a *reflection* of it, held as such ✎.
2. **The picture.** What a person says and shows, joined by an elohim they have given that reach, mixed with what their community has witnessed of them. Shown to them, contestable, never joined by an outside observer, never an input to judgment.
3. **Fruit.** What is said and done in shared space. This is what may be judged, what earns reach, and what crafted, shared memory context is made from.

### 4.2 A join has a shape

The steward's direction ✎: who can do the joining matters. A household elohim has more intimate reach than one invoked to act at community level. As levels or dimensions of reach are invoked, the elohim that act, and where they join *from*, are part of the reach-shape that sets the boundary of privacy. Visiting a doctor has a different shape, and different answers, than visiting grandma at Thanksgiving. Same person, same underlying facts, two joins.

The steward's list of what must be knowable reads as the fields of one record, made each time an elohim correlates things about a person:

| Field | Nearest thing we have |
| --- | --- |
| Which elohim acted, at which tier | The reach ladder; steward of record |
| Where in the network it was invoked from, and for what | A session covenant's declared purpose and write set |
| What sources it drew on | The recall contract's routes and receipts |
| What it had to correlate | Nothing yet |
| Which other elohim it consulted, at which tier | Nothing yet |
| How far each datum was carried, and what was dropped at aggregation, when and why | Living memory's submerge and release, with an audit row (G3) |
| What semantics, ontologies or embeddings it built | The index as a rebuildable projection |
| Where those live and how long: a personal device or a commons pool | Memory classes and lifecycles (G1); custody classes (G5) |

Rules this seems to imply ◐:

- **A join reaches no further in than its invoker's ring.** A community-tier elohim asking about a person gets the answer a community-tier join can make, even when the bytes it would need sit on the same peers.
- **Planes stay ignorant of each other by construction.** The holder of shards does not know whose they are. The holder of the join need not hold the bytes. A consultation between tiers passes an answer, never the sources behind it.
- **What a join builds has a ring and a lifetime of its own.** An embedding built at the household tier does not become a commons resource because a commons pool could store it. Moving it there is a compose through the reach gate, with whatever must be dropped on the way dropped and recorded.
- **The substrate bounds the elohim.** By G6 an elohim may escalate, witness or defer; it does not override the gate that says which join it may make.

This is contextual integrity in the sense Helen Nissenbaum gave the term: information flows carry the norms of the context they arose in. The reach-shape is that idea given a record and a gate.

### 4.3 Where we are

All 320 memories live in one directory. The collective declares that path at `maxLocality: repository` and its own reach as `familiar` ✅ (`.epr-meta/collective.json`), and the entries travel by git push to a forge. On the three axes:

- **Reach:** no entry states its own, so in G6's terms an entry has a knowledge leg, part of a value leg (the steward-of-record mark) and no governance leg. It is not yet a whole EPR.
- **Custody:** the forge holds every entry in the clear. There is no encrypted custody.
- **Join:** the `user`-type entries name the steward and sit beside everything else. The tie to the person is written into the file. Nothing is decontextualised.

So the `user`-type entries are the one place where bytes, readability and the join all leave the person together. The 2026-06-01 design already says "Push is a reach boundary… Git tracking is not consent", and the workspace-arrival spec says "no narrower ring exists on the record yet." Shem's gitignored `.claude/memory/device/` is, in practice, a second ring nobody declared ✅. Recall receipts under `.eprfs/status/recall/` are a third, and the only one with a ruling behind it.

Nothing moves memory over the peer plane ◐. The sibling exchange already acknowledges heads between the two devices ✅, so the carrier exists. The backlog seed for it "serves no declared habit yet and waits for one."

### 4.4 What the workspaces need

- **Every entry states its own reach.** Then the path rule becomes a ceiling and the entry becomes a whole EPR. Widening an entry is a compose at a higher ring through the reach gate.
- **The join travels between a person's own devices; the bytes may travel wider.** What crosses directly between enrolled devices is the context that ties entries to the person. The entries themselves can be held in encrypted custody by whoever has agreed to hold them. Neither enters the DHT, which is the notary. A habit that would give the backlog seed its reason: *a correction given on one device is in force on the sibling before that sibling's next session.* It is observable with two devices and one entry.
- **A session declares the joins it may make.** A covenant already declares purpose and write set. The same record could declare which collections the session's agent may join, at which ring, read-only or read-write. That is Anthropic's store-mount idea with a reach-shape where they have an API workspace.
- **Consolidation as supersedence.** A merged or corrected entry points at what it replaced with a `supersedes` edge signed where it was written. The act then travels with the record, which answers the workspace-arrival spec's §6.3 ("the record travels, the act does not").
- **Fading leaves a row.** An entry that loses its place submerges to a tier its steward's elohim still reads, per G3. Deleting a file and trusting git history to remember is the janitorial version.

### 4.5 The first rung, and graduation with care

**The first rung is already built, and memory is not on it yet.** The steward channel between one person's devices (`2026-10-23-workspaces-cooperate-as-devices-design`, policy in `pool-policy.json` under `berth.channel.crosses`) has four parts ✅:

1. **Admission by roster.** Only devices bound to the same person are let in.
2. **A default-deny crossing list.** Only named kinds of record cross: berth events, gate verdicts, landing intents, artifact custody.
3. **Fold as theirs, with acknowledgement.** A sibling's artifact record is "only that device's custody, never local custody". Each device signs a head and says how far it has folded the other's.
4. **Thoughts stay.** "Never crosses: recall sessions and their receipts, private memory, session transcripts… only what is said or done crosses."

For memory at this rung, relevance is declared on the entry ◐:

| An entry applies to | Examples | Travels by |
| --- | --- | --- |
| The device | Paths, pool slots, installed toolchains, the hardware target, local mesh state | Nowhere. Shem keeps a gitignored `.claude/memory/device/`; this workspace has no such directory. |
| The steward | Direction and corrections given to an agent; what the person has said of themselves | The channel, device to device. Today it goes by git to a forge, wider and slower than it needs to be. |
| The repository | Context crafted from the work | Git, as now. |

A transcript is thought; a correction given in it is a thing said. So each device does its own noticing over its own transcripts and only the resulting entry crosses. The sibling learns what was ruled without holding the conversation.

**Graduation needs intention and care at the moment it happens.** The steward's concern ✎: when one peer works with another, a conversation that should be shared, or a memory that should be kept collectively, has to graduate deliberately, with care taken at its creation and at its graduation. Much of the machinery exists ✅:

- The collective's charter: "widening a contribution needs an approving verdict from a Steward who is not its author" (`.epr-meta/collective.json`). The verbs are `contribute`, `feedback` and `graduate`.
- The reach brief's §6b names three operations, construct, discover and evaluate, and holds that "a signed claim and an affirmed claim must remain distinguishable" (`reach-delivery-subsystem-solutioning-bootstrap`).
- The reach gate can answer Pending, awaiting sponsorship.
- The manifesto: "reach is set before the gift", and withdrawing a gift is a request the network honours as far as it can.

What is missing ◐:

- **Intention at creation.** An entry is born with no declared audience: no "applies to", no reach, no intended holders. The moment of writing is when the author knows this best and when stating it costs least.
- **A shared conversation is jointly made.** Neither party may graduate it alone. Each may graduate what they themselves said; a joint record needs every participant's assent. Sealed supersedence already uses a two-of-two seal, which is the nearest precedent.
- **The distinct Steward at the first rung.** Between two devices of one person there is no second Steward. At that rung the author's own deliberate act is the whole of the assent, as Claude Code's `/remember` is. The rule about a Steward who is not the author begins at the first rung that holds a second person.
- **What care consists of.** A graduating act should say why, for whom, what context is stripped on the way (so what leaves is decontextualised where it must be), which memory class and lifecycle the result takes, and how it would be withdrawn.
- **Noticing is separate from graduating.** A dream may notice "this looks like it should be shared" and propose it. It never graduates anything (T4).

**What each rung must supply.** Read from the first rung, a rung is these six things, and building the next one means answering them again:

| A rung supplies | First rung (one person's devices) | Next rung (a household), expected ◐ |
| --- | --- | --- |
| Roster: who is admitted | Devices bound to one person | People of one household, each with their devices |
| Crossing list: which kinds cross, default deny | Berth events, verdicts, intents, artifact custody; memory entries to be added | Narrower, and named by the household |
| Fold: how another's records are held | As theirs, with an acknowledgement vector | The same |
| Graduation: whose assent widens | The author's deliberate act | A Steward who is not the author; every participant for a joint record |
| Custody: can the holder read | Yes, the holders are the same person | Encrypted custody begins here, because holder and subject differ |
| Join: who may tie records to a person | That person's own agents | A household elohim, at the reach the person gave it |

---

## 5. Verdicts

**TAKE**

- **T1. Scheduled, cheap noticing that only proposes.** Build the thin `/dream` the lifecycle design specified: small readers, one organizer, transcripts as untrusted data, output as a branch and a diff. Never an in-place edit.
- **T2. Each proposal is a consolidation event on the valueflow** (truth found, entries and stewards, capacity freed, reach changed), per G2. This is the piece neither Anthropic form has.
- **T3. Recall journeys before and after every dream**, shown with the diff.
- **T4. Widening is a compose through the reach gate; a dream never does it.** Consolidation may *lower* reach at discovered harm and *propose* raising it.
- **T5. Decide the native auto-dream deliberately.** This survey recommended `autoDreamEnabled: false`, so no undeclared actor edits a generated, governed directory. The steward ruled the other way on 2026-11-18: leave it on and learn from it until there is much more confidence in subsuming it. The directory is git-tracked, so each dream arrives as a diff to read before it is committed.
- **T6. Usefulness parity as a stated aim.** The protocol should answer the two-fact, never-stated question as well as Memorilla does. Adopt PersonalizationV4's shape as a measure, run where the person's own elohim makes the join.

**STUDY**

- **S1. The path to Memorilla's usefulness on our ground** (§1): a commons-stewarded reader, a person-held join over collections in encrypted custody, a read at the edge, and attribution surfaced so the answer can be shown, contested and recognised. Open parts: whether archetype embeddings are needed at all, who stewards the reader, and which local model carries it.
- **S2. The join record** (§4.2): the fields, and the two that have no home yet (what was correlated; who was consulted, at which tier).
- **S3.** Reach on each memory entry, making it a whole EPR.
- **S4.** Entries that record whether acting on them helped (Memorilla's reward-annotated documents).
- **S5.** Joins declared per session beside the write set.

**LEAVE**

- **L1. Memorilla as released, in the governed plane.** Until per-document attribution is surfaced its output cannot be shown, cited or recognised. This is a verdict on the code as it stands, and S1 is the way past it.
- **L2. Any arrangement where one party holds bytes, keys and the join together**: synced project memory and operator-held per-user stores as our sharing channel.
- **L3. In-place dreaming, and deletion as the default for a contradicted entry.**

**WATCH**

- Auto-dream and `team/` memory reaching the official documentation.
- Memorilla, or a successor, exposing per-document attribution.
- Projects memory on team plans.

## 6. Open questions for the steward

1. **The `user`-type entries are readable, contextualised and on a forge.** They are the one place bytes, readability and the join leave the person together. The steward may have chosen this; no record of the choice exists. Recommended: hold them on a device ring now, and in encrypted custody with a person-held join when that exists.
2. **Does a derived artifact inherit the narrowest ring among its sources, or the ring of the elohim that built it?** Recommended: the narrowest source, until a compose through the gate says otherwise.
3. **When data is aggregated upward, what must be dropped, and who witnesses that it was?** No recommendation; this is the heart of the reach-shape and wants its own design.
4. **Is the join record a private thought or contestable fruit?** A recall receipt is private by ruling. But "shown to you so you can push back" needs the person, at least, to see every join made about them. Recommended: private to the person and their elohim; never to the network.
5. **Are archetype or persona embeddings needed, and if so who stewards the pool?** They are the commons half of S1.
6. **Should the native auto-dream ever run here?** Recommended: no. Our own, proposing, on a schedule.
7. **Does the sibling-memory habit in §4.4 stand as worded?**
8. **Who is the noticer rewarded in T2 when the noticer is a small model on a schedule?** Recommended: the event names the session and its steward of record, and recognition is held against the work.
9. **Which memory classes do our entries fall into?** `feedback` entries that record a ruling read as attestation (supersede, never forget); most `project` entries read as contextual (fade). The `/dream` cannot be class-aware until entries say.

## Method

Single author (Claude Fable 5.2), one session, 2026-11-18. Memorilla was read in source: `memory.py` and `llm.py` in full, the README, the RL, training, data and evaluation guides. Nothing was run; no checkpoint was downloaded. The Dreams and memory-store pages were fetched whole. The Claude Code findings marked ◐ come from strings in the installed 2.1.327 binary; they show what the client contains, and say nothing certain about what is enabled for this account. The ground in §0 was read directly: the manifesto's principles 2 and 6 and *Witnessed Humanity*, the living-memory epic from the comet shape onward, the resilience epic's three custody classes and its floor-and-ceiling section, `reach.rs`, `.epr-meta/collective.json`, the lifecycle design's consolidation-event section, and the EPR content-addressing skill. One read-only agent mapped our other documents; its line references were not all re-checked. A librarian pass against the Research Index returned EXTEND (companion to the seams note). There was no challenge pass.

**Version 2.** Version 1 read the attunement limit as "the picture never leaves the person's device", wrote Memorilla off on that basis, and left persona benchmarks. The steward corrected this in the same session: resilience requires that private data be held across the network; what protects it is that holders cannot read it and that opening it needs gradients ignorant of each other; the picture is a mixture of self and community; and the protocol should reach Memorilla's usefulness by means that put what is shared into commons-stewarded form. The steward then set out the reach-shape of a join (§4.2). The resilience epic was read afterwards and supports the custody half ✅. The decontextualised-reference mechanism and the reach-shape are marked ✎: they are the steward's direction, and no canon document found in this session states them. The nearest canon is the reach solutioning brief (`genesis/data/timeline/backlog/reach-delivery-subsystem-solutioning-bootstrap.md`), found afterwards and read in part. Its tension 1 asks "Is reach one axis or several?", which the three axes of §4.1 answer provisionally. Its tension 5 settles on "holder ≠ reader" and its agenda item 7 asks which planes may move bytes to non-readers "(custody, encrypted only)", which is G5. Its §6b supplies the construct, discover and evaluate operations used in §4.5. Section 4.5 was added in a third pass on the steward's further direction about the first rung and about graduation.

## Outputs

- Manifest entry `memorilla` in `research-manifest.json`; Research Index entry under *The Discovery Problem*, beside the seams note.
- Horizon scan `genesis/docs/analysis/horizon-scans/2026-11-18.md`.
- **Mint pass, 2026-11-18** (after the steward widened the session's write set). `agentic-context-tooling-consolidation-queue` items 28–31 (T1–T3; T5; S3 with §4.5's fields; T6 and S4). `native-peer-dev-channel-and-collective-memory-seed`: the first-rung shape, graduation with care, a candidate habit and what earlier deliveries should leave open (§4.5, T4). `reach-delivery-subsystem-solutioning-bootstrap` agenda item 15: join as a named axis and the join record (S2, S5, open questions 2–4). `arch-confidentiality-plane-backlog` row 16: person-held join over encrypted custody with a commons-stewarded reader (S1, open question 5). `CLUSTERS.md` rows groomed to match. Not minted: T5 remains a settings change for the steward; open questions 1 and 6–9 stay here.
