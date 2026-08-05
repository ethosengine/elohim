---
title: "The Digital Memory Ontology — standing as a first-class property of create · class · hold · consolidate · surface · forget"
id: digital-memory-standing-ontology-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete OR first-standing-bearing-surface-shipped
sovereignty-frame: bounded
stewardship-frame: bounded
cites:
  - rights-affordance-floor-plan | The plan this ontology makes expressible — its hold/afford/protect split, floor-vs-ceiling invariant, and Slice 0 are the mechanism half; this doc supplies the grammar they need to be sayable | sha256:ee0069bc2e4d5cc4 | path: genesis/docs/superpowers/plans/2026-08-05-rights-affordance-floor-plan.md
  - ownership-custody-inalienable-red-team-design | Source of the inalienable class and the three-way rights/custody/subject split this ontology generalizes into the five verbs; its refused primitives (public veto index) bound what CLASS and FORGET may express | sha256:d80fea9b7bf8843f | path: genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md
  - middot-measure-primitive-design | The sibling ontology — measures make observation composable and teeth-free; this doc makes the memory those observations produce standing-bearing. A MeasureFold over a person is derived memory and inherits their standing | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - confession | The warrant and the limit: binding-as-covenant governs why guards must stay short and link rather than lecture; and the total-account-belongs-to-God-alone clause is the tension this ontology mitigates structurally but does not dissolve | sha256:bec001fd41230c67 | path: genesis/docs/content/elohim-protocol/confession.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
---

# The Digital Memory Ontology

> **Scope discipline (operator, 2026-08-05):** *"this probably requires some intense technical design.
> we don't want to do that, we just want to make sure our architecture CAN do this at scale, not that
> it will right now."*

This document is an **ontology**, not a mechanism. It names the vocabulary and the contract that make
stewardship, privacy, and inalienable rights *expressible* — so that when we build, the right things
are sayable and the wrong things are not. No entity types are minted here.

## 1. The move being made

The protocol has done this twice already, and both times the payoff was a property that became
**structural rather than bolted on**:

| Ontology | The move | What became first-class |
|---|---|---|
| **REA** | Model economy as *Resource · Event · Agent* rather than debits and credits | **Explainability.** You answer *"why does this balance say that"* by walking the events. Nobody adds an explainability feature; it falls out. |
| **Agent/witness** | Make the witness a *first-class agent*, not a log line | **Negotiation.** There is someone to negotiate *with*, whose attestation can be given, withheld, or challenged. |
| **This document** | Make **standing travel with memory** through its whole lifecycle | **Consent.** You cannot hold or process a memory without knowing whose it is — because standing is in the ontology, not in a policy layer above it. |

**Standing** = the relation *this memory is about that person*. It is distinct from custody (who holds
the bytes) and from rights (who may decide about them). Three relations, never collapsed — the
distinction the ownership guard exists to protect.

## 2. The verbs

The lifecycle, and what each move makes **impossible to express**. The impossibilities are the point:
an ontology earns its keep by what it forbids, not by what it permits.

The verb set is deliberately borrowed from **memory science and relational practice**, not invented —
because those fields have already discovered what a memory lifecycle must contain, and a vocabulary
that ignores them will omit something real (§2.1).

| Verb | The ontological commitment | What becomes inexpressible |
|---|---|---|
| **CREATE** *(encoding)* | A memory is born of an **observation** — an event with an *observer*, a *subject*, and an *occasion*. Never "data was written." (Playnet's Observer-Claim-Effect shape, arrived at independently: `Effect = Sign_Observer(Entity, Attribute, Δ, t)`) | Anonymous data with nobody accountable for its existence, and no one it is about |
| **CLASS** *(salience tagging)* | Class is **derived from the relationship**, not declared by the holder. Intimacy, plurality of subjects, and reach follow from who is in it and how — determined at ingest, before interaction | A publisher selecting the lighter regime for their own content by labelling it |
| **HOLD** *(storage)* | Three relations, always distinct: **custodian** (holds bytes) · **rights-holder** (may decide) · **subject** (it is about them). Custody never ripens into rights — not by transfer, not by inertia, not by death | Ownership, as a single relation that collapses all three. *A holon that owns the commons it stewards has enclosed it* |
| **CONSOLIDATE** *(consolidation)* | The move **down** into the associative tier — not deleted, not kept conscious. **Gist-preserving and salience-weighted**: episodic detail fades, meaning survives. A different decay law from the conscious tier | A memory system with only two fates — deliberately curated, or lost |
| **SURFACE** *(cue-dependent retrieval)* | The move **back up**, on cue rather than on schedule. Surfacing **inherits the relationship, not just the data** — the same recall is care in one relation and predation in another | Surfacing as a neutral read |
| **RECONSOLIDATE** | **Retrieval is not read-only.** A surfaced memory becomes labile and is re-stored, possibly changed. This is the mechanism the forgetting ceremony rides | A ceremony that changes nothing; "remembering together" as a no-op |
| **PROCESS** | Processing is **itself an observation that creates new memory**, and **derived memory inherits the subject standing of its inputs** | **Inference laundering standing** — see §3, the load-bearing clause |
| **FORGET** *(active forgetting)* | Forgetting is an **event with a witness** and a mechanism, not mere decay. Differentiated into *access-revocation* (unilateral, silent) · *record-retirement* (witnessed ceremony) · *remnant* (encrypted residue, honest about what p2p cannot erase) | Deletion theatre; and coerced unilateral erasure, because witnesses are required regardless of who initiates |

### 2.1 Why these words, and one term deliberately rejected

**`bury` was the intuitive term and it is the wrong one.** In clinical practice burying is *avoidance*,
and avoidance is the pathology that maintains harm rather than resolving it. **Consolidation** is the
healthy process it was reaching for: the same downward move, but toward *integration*. The word choice
is not cosmetic — calling it `bury` would encode avoidance as the system's model of what to do with
hard memory. Plain-language surfaces may still say "buried"; the ontology says consolidate.

**`reconsolidate` earns its place because retrieval genuinely modifies.** A recalled memory becomes
labile before it is re-stored — the property therapeutic memory work is built on. Two consequences:

- The **forgetting ceremony is a reconsolidation event.** Surfacing the memory *with a witness* is what
  lets it be re-stored differently. That is the mechanism of an *open door to forgiveness*, and it is
  why the ceremony is not theatre.
- A digital system *could* make retrieval read-only. The ontology names reconsolidation so that choice
  is made deliberately rather than by default.

**Witnessing does two jobs, and we had only noticed one.** We justified witnesses as coercion
resistance — structural, and true. But in trauma practice **being witnessed is constitutive of
integration**; healing is integrating harm into narrative, *not erasing it*. That is independent
support for holding the claim of harm in the commons rather than deleting it: the survivor's record is
not merely evidence, it is the integration. Erasure-on-demand would be the system enacting avoidance.

**Repair, not conflict-avoidance, is the relational unit.** Relationship research finds that failed
*repair attempts* — not conflict itself — predict dissolution. The ceremony is a repair primitive, and
its availability matters more than its frequency.

**Disclosure must be subject-controlled.** Controlled disclosure is therapeutic; forced disclosure
re-traumatizes. This is the clinical argument for the FILTER remedy, and for surfacing being something
a subject can shape rather than only endure.

## 3. The load-bearing clause: inference cannot launder standing

**Derived memory inherits the subject standing of its inputs.** An embedding of your photo is still
about you. A cluster you fall into is about you. A model trained on your behaviour is about you.

This single clause is what makes the ontology adversarial to surveillance capitalism rather than
merely different from it. Their business model *requires* that the derived model of you is not
"about" you in any architecturally meaningful sense: your posts are yours, the inference is theirs.
Every portability regime built so far has conceded this — *"download your data"* returns the
declarative tier and never the inferred one, because the inferred tier **is** the product.

In an ontology where derivation inherits standing, that separation is not prohibited — **it is
unsayable.** There is no well-formed way to express a memory about a person that has no subject.

**Corollary for the subconscious tier.** The associative memory that lets an agent serve someone well
is *derived memory at scale*. Under this clause a person's implicit model is **theirs**, structurally,
wherever it is computed. That is the claim which today has neither legal nor technical expression.

## 4. The contract a surface declares

What any memory-bearing surface must be able to answer. Not a schema — the questions that must have
answers, so a future schema can be checked against them.

1. **Whose is it?** — the subject relation, plural where the memory has plural subjects.
2. **What class?** — derived at creation from the relationship, not asserted by the holder.
3. **Who holds, who decides?** — custodian and rights-holder, named separately from the subject.
4. **What does processing produce, and what does it inherit?** — the standing of its inputs, always.
5. **How is it forgotten?** — which of the three forgetting moves apply, and what a witness is for.
6. **What cannot be undone?** — stated honestly, in the product, not in a footnote.

**The honesty clauses this contract obliges** (already ratified elsewhere, restated because a contract
that hides its limits is not a contract): we cannot guarantee deletion; we cannot detect coercion;
revocation is uninformative, not invisible; the floor binds this protocol, not the world.

## 4.1 Falsification cases — the ontology must express all nine rooms

An ontology that cannot express these nine rooms is wrong, and this is the test to run before adding
any primitive. They were chosen because the primitives are identical in each and **the meaning of
every one of them inverts.**

| Room | Risk of **remembering** | Risk of **forgetting** | Default the class must set |
|---|---|---|---|
| **Kindergarten** | Catastrophic in the wrong hands | **Also a harm** — forgetting a child's needs fails them | Hold in trust; standing reserved for a self who does not exist yet |
| **Bedroom** | High, and it *changes when the relationship does* | Real — shared memory is part of the bond | Ceremony; consent is time-indexed and context-bound |
| **Casual meeting** | Low individually, compounds into surveillance | Trivial | **Decay by default** — the only room where forgetting needs no ceremony |
| **Brothel** | Third-party linkage can be lethal to the worker | **Real** — an unrecorded violent client cannot be warned about | **Asymmetric linkage authority**: the person at risk may link; everyone else faces a high bar |
| **Newsroom** | To the subject, reputational and legal — **and that is the point** | **Accountability collapses; power becomes unauditable** | **Standing is bounded.** It does not reach memory of one's own exercise of power over others |
| **Emergency room** | Moderate — medical memory is sensitive | **Lethal** — the allergy nobody could look up | **Break-glass**: a bar a stranger *can* meet in ninety seconds, on an unconscious subject — and every crossing is **loud** |
| **Deathbed** | The subject can no longer revoke, and kin may hold what they wanted released | Grief, lineage, inheritance, the historical record | **Standing persists, unheld.** Death confers no new permission; prior declarations govern |
| **Protest** | Identification, prosecution, facial recognition | **State violence goes unevidenced** | Plural standing **without either-veto** — per-subject remedies only |
| **Confessional** | The disclosure was made *because* it would be held | The relationship's whole purpose is defeated | **Custody carries a duty** — a floor claim on the custodian's behaviour, not only on the data |

**The primitives do not change — the class sets the direction of the default.** This is the argument
that the verb set is shaped correctly: if remembering-versus-forgetting were baked into the primitives,
they would serve exactly one room and quietly betray the rest. Which is what most systems do — built
for the casual meeting, then deployed in the bedroom.

**Seven demands visible only when the rooms are held together.** None is served today:

- **Deferred standing.** The kindergarten needs standing held for a person who does not exist yet: the
  child's adult self has standing over memories the child could never consent to. That is not "the
  guardian decides" — the guardian is *custodian of someone else's future claim*. Our capability
  gradient has a ward row and **no concept of deferred standing**.
- **Asymmetric linkage authority.** *"Unlinkable" is the wrong target and it defers to the wrong
  party.* The beneficiary of nobody-being-linkable is whoever is avoiding social consequence — not
  whoever is at risk. And total unlinkability would disarm the safety infrastructure the vulnerable
  party has actually built: peer warning about a dangerous client, establishing a pattern, seeking
  recourse are **all linking operations**. So linkage is not forbidden, it is **high-bar, and the bar
  is held by the person at risk** — exactly as either spouse may disclose their own intimate data
  while a third party faces an extraordinary bar. The data was never unlinkable; it was *linkable by
  those with standing.* Two consequences: the bar must be **specified**, or it is a backdoor with
  good manners; and it must be one an abuser, a prosecutor, or a curious institution **cannot meet**,
  while the subject meets it by definition. This also sharpens the backstopping rule: community
  backstopping governs **recovery and standing, never linkage** — linkage authority stays with the
  person the memory endangers.
- **Bounded standing — the one that must not be missed.** Every other room protects the subject. The
  newsroom is the inverse: **the subject's claim is the threat.** If standing gives you control over
  memory about you, then *"this memory is about me"* becomes the most efficient suppression primitive
  ever built, and every powerful person acquires a takedown lever. A journalist's record of wrongdoing
  is memory *about* the wrongdoer. So the ontology must say where standing **stops** — the working
  line: standing does not reach memory of one's own **exercise of power over others**, which is where
  earned reach and accountability begin. Without this the ontology is a censorship tool, and this is
  the first thing an adversary would use it for.
- **The bar has two faces, and neither room alone defines it.** The brothel demands a bar an abuser or
  prosecutor **cannot** meet. The emergency room demands one a **stranger with a scalpel can meet in
  ninety seconds**, on a subject who cannot consent. Held together they give the bar its shape, and
  the ER supplies the property that makes it survivable: **break-glass is loud.** Every crossing
  leaves a record the subject sees afterward. A silent high bar is indistinguishable from no bar —
  which is very likely what the brothel's bar needs too.
- **Standing that outlives its holder.** The kindergarten gives standing for a self who does not exist
  *yet*; the deathbed gives standing for a self who no longer does. Every simple answer fails
  somewhere: **terminate** and the dead have no protection; **transfer to kin** and the family
  inherits control over exactly what the person may have wanted released; **persist unheld** and
  nothing can ever be resolved. The plan's stated default — custody never ripens into rights on
  death — survives this room only if paired with *prior declarations govern* and *death confers no
  new permission*. `NetworkWitnessPurpose::Dissolution` exists and is stub-rejected in the validator;
  this is the room that says what it must become.
- **Plurality needs two rules, not one.** The bedroom's either-veto is right where exposure is
  unrecoverable and the subjects are few. It **breaks** in the protest: one subject's veto would
  destroy accountability evidence for everyone else, and the subjects' interests genuinely conflict —
  one needs the record as proof of state violence, another needs to not be identifiable, and *neither
  is wrong*. So plural standing resolves by **either-veto** in the intimate class and by **per-subject
  non-exclusive remedies** (filter, modify) at collective scale. The class must say which applies, and
  consultation at protest scale is impossible by construction — the rule cannot presume it.
- **Custody that carries a duty.** Our three relations name custodian, rights-holder, and subject, and
  custody is currently a *neutral fact*: you either hold the bytes or you do not. The confessional
  shows this is insufficient — a counsellor's memory of you carries an **obligation of care**, and in
  some traditions a seal that survives even legal compulsion. That is a floor claim on the
  **custodian's behaviour**, which the ontology has no way to state. We are already relying on the
  property without being able to express it: the non-firable elohim-counsel *is* a duty-bearing
  custodian, and it is the one primitive that survives a compromised guardian.

And within a single room the same verb inverts: a teacher **surfacing** *"this child struggled with
letters"* is care; a stranger surfacing it is predation. A partner surfacing a shared moment is
intimacy; the same recall after a breakup is harm. **Surfacing inherits the relationship, not just the
data** — which is the standing clause (§3) arriving from a different direction.

## 5. Scaffolding it into `.epr-meta` — the discipline made co-located

`.epr-meta` is where a directory declares what it *is* and what belongs in it, evaluated at the moment
of an edit. That makes it the natural home for a memory-standing contract — and, more importantly, the
**cue** for §6.

**What is honestly available today.** The rule vocabulary is a closed set
(`_ACTIONABLE_KEYS` in `.claude/scripts/_lib/epr_meta.py:172`) — `require-frontmatter`, `route-to`,
`no-new-subdirs`, `require-sibling`, `dedupe-of`, `validator`, `measure`. **A memory-standing contract
is not one of them, and inventing an unwired key is the worst available outcome** (silent inertia: it
validates clean, gates nothing, and reads as governance). Two honest paths:

- **Now, no new mechanism:** a directory holding subject-bearing memory declares the contract in its
  `.epr-meta` **body** (the knowledge leg, which is real prose an author reads) and binds a
  `class: inject` policy whose `why:` carries §4's six questions and points here. Teaches at the edit;
  gates nothing it cannot honestly gate.
- **Later, if it earns it:** a `validator: epr:validator-memory-standing` in the registry — the same
  escape hatch the p2p-design-gate uses for checks too rich for a declarative predicate.

**Do not** add a `memory-standing:` predicate to the closed set until something evaluates it.

## 6. The surfacing exercise — volatile memory without context clutter

The operator's target: *"how more volatile memories (things we need to remember) progressively surface
without cluttering our context."*

**The finding.** `.claude/hooks/pickup-semantic-surfacing.py` is already wired on **both**
`UserPromptSubmit` and `PreToolUse` — but it fires **once per session, only within prompts 1–3**
(`MAX_PROMPT_WINDOW = 3`), with `COSINE_FLOOR = 0.35` and `TOP_K = 4`. After that it is silent for the
rest of the session.

That is a **session-opening briefing, not a subconscious tier.** Real associative memory surfaces at
turn 50, when you touch the thing. And the one-shot throttle exists for exactly the operator's stated
reason — a naive always-on hook would clutter every turn. So the design question is not *should it
fire more*, it is **what earns a firing.**

**The ontology answers it: surface on cue, not on schedule — and the contract declaration is the cue.**
`.epr-meta` already knows which directory is being edited and what it holds. A directory that declares
a memory-standing contract (§5) is *by that declaration* saying "acts here are standing-bearing," which
is precisely the class of act that earns a surfacing. Routine edits earn none.

**Four budget dimensions**, which today are one (`TOP_K` × once):

| Dimension | Today | What it should be |
|---|---|---|
| **Size** | Palace-bounded | Large and cheap — capped by disk, never by context |
| **Retrieval** | `TOP_K=4`, **once per session** | A **rate**, cue-gated by class. The attention price — and the dial the feed sets *against* the user |
| **Decay** | Staleness check only | Gist-preserving, salience-weighted — a *different law* from the conscious tier |
| **Interpretability** | Cosine shown + *"recall hints, not truth"* footer — **already correct** | Keep. Non-enumerable by default, explainable on demand |

**The diagnosis this supports:** our chronic `MEMORY.md` byte-budget pressure is not hygiene debt — it
is a **conscious index being asked to do associative work.** The conscious tier must fit in context and
therefore has a hard cap; the subconscious tier should be large, cheap, and never loaded wholesale.
Consolidation warnings are the symptom of the missing tier, not of untidiness.

## 7. How this reaches the guards

The ownership and sovereignty guards are where an agent **bumps into** this ontology mid-sentence.
That is the teachable edge, and it is why the guards' four-rung progressive disclosure matters:
fire → architecture → law → warrant.

This ontology adds the rung agents most need, because it converts a rule into a reason: **an agent that
has internalized standing-bearing memory does not need to be told "don't say ownership."** For a
subject-bearing datum, ownership is not forbidden — it is *ill-formed*, the way a sentence with no
subject is ill-formed. The guard stops being a rule to memorize and becomes a pointer into a grammar.

And the guards must remain short. `confession.md:95` binds the shape: the protocol must *"tell the truth
about the binding… never deceive the agent that the cage is liberty — because the lie that the cage is
love is the very domination this whole work exists to refuse."* A guard that grew to contain this
document would be a cage. A guard that opens a walkable path to it is covenant.

## 8. What this does not do

- **It mints no entity types.** `subjectStanding` remains a *field* proposal in the commons cluster, not
  a type, and nothing here promotes it.
- **It specifies no ceremony.** The forgetting ceremony's witnesses, delays, and reversal path are
  named as *shapes the ontology must admit*, not designed.
- **It does not make the architecture do this.** It makes the architecture **able** to — which is the
  stated scope. The gap between able and doing is the plan's Slice 0 and beyond.
- **It does not resolve the confession tension.** A derived model of a person held even by their own
  agent sits close to the *"total account of a person… that account belongs to God alone"*
  (`confession.md:61`). The ontology's answer — standing stays with the subject, the account is never
  aggregated network-side, and *"best self" is never a verdict rendered over a person, only a hope held
  for them* — is a structural mitigation, not a dissolution. Stated, not solved.
