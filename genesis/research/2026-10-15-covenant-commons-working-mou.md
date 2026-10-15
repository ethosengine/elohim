---
title: "Working memorandum of understanding: the covenant from below, applied to our own dependencies and the commons they feed"
id: covenant-commons-working-mou-2026-10-15
status: Capture
date: 2026-10-15
sovereignty-frame: bounded
steward: human:matthew
author: "Claude, at the operator's direction (design session, 2026-10-15); the operator's rulings are quoted in his words"
tags: [commons, covenant, cite, reliance, jubilee, council, for-the-living, rulings, research]
cites:
  - "reliance-as-cite-shared-provenance-primitives-design | Reliance as cite | sha256:ed648f3378dba82a | path: genesis/docs/superpowers/specs/2026-10-15-reliance-as-cite-shared-provenance-primitives-design.md"
  - "commons-composition-at-scale-2026-10-15 | Commons composition at scale: attribution that closes its account, from one work to the planetary commons | sha256:9d48c44f61c85716 | path: genesis/research/commons-composition-at-scale-2026-10-15.md"
  - "commons-facility-at-scale-2026-10-15 | The commons facility at scale: what peers commit, how pools nest, the support button, and a commons for the living | sha256:6ec3db29e125148c | path: genesis/research/commons-facility-at-scale-2026-10-15.md"
  - "elohim-protocol-manifesto | manifesto | sha256:bdfd593210e7fea0 | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "values-forward | values-forward | sha256:0bfa006fd772f7d9 | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "succession | succession | sha256:4ae356e1bf2112e6 | path: genesis/docs/content/elohim-protocol/succession.md"
  - "commons-pool-as-collective-party-design | A commons pool is a collective that is party to its members' commitments | sha256:c194ddbe3dc3b2a6 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - "covenant | COVENANT | sha256:1dd4c6a3ea066842 | path: COVENANT.md"
---
# Working memorandum of understanding: the covenant from below, applied to our own dependencies and the commons they feed

*Status: working. Started 2026-10-15. The orchestrating agent (Claude) settles each section from trajectory readings of the vision and manifesto. Matthew Dowell, the protocol's steward, can overrule any line. This memorandum is not canon. Its purpose is to let the build move forward. Each settled line records its basis in canon, its confidence, and what would reopen it.*

## Sources it settles

- Design spec: `genesis/docs/superpowers/specs/2026-10-15-reliance-as-cite-shared-provenance-primitives-design.md` §9 (S1–S10)
- Research: `genesis/research/commons-composition-at-scale-2026-10-15.md` §16 (C1–C9)
- Research: `genesis/research/commons-facility-at-scale-2026-10-15.md` §12 (F-Q1–F-Q5), plus the §13 proposed canon correction
- Approved plan: `/projects/.claude-config/plans/zippy-conjuring-gem.md`

**The orienting value.** Every line below serves the mission the manifesto names: "The Elohim Protocol is an attempt to **put the fruit back on the tree** — to restore the conditions under which humanity can flourish in relation to itself and to creation" (manifesto:965; stewardship-over-sovereignty:248). Applied here, the fruit is the record of what a work rests on and what it meant to people. It is returned to the people and the commons it came from, rather than gathered into anyone's hand. The operator confirmed the shape of section E ("the surface shows fruit") against this value on 2026-10-15.

## Already ruled by the operator (2026-10-15), not reopened here

- **Covenant scope.** The protocol cites its own dependencies first, as the worked example (manifesto Part IV-B).
- **Code is the first implementation of the shared ontology.** The co-op grocer and the garden network ride the same primitives.
- **Release order and placement.** Rust first, then npm. `COVENANT.md` states the licences as they are. The reader lives in `bridges/cargo`, with no pin moves (approved plan).
- **An elohim council over each pool; no single pool-elohim** (ruled 2026-10-15, corrected the same day). Each pool is governed by an elohim council over it. The council is plural and representative: proportional to the holon's demographic diversity as far as that can be determined, and sized to what the holon can seat. In the network's infancy, humans perform that role. Human sortition councils remain authoritative: they audit and govern any contestable result. Value crossing the network boundary is likewise an elohim-council act at the ceiling, contextual to the peer position of the ask, plural, witnessed, and overturnable by the human floor. (Stance II: floor, floor, ceiling.)
- **"Holonic" and "sociocratic" are symbolic stand-ins, not commitments.** The operator uses them loosely and interchangeably for whatever governance structure proves most trustworthy: frictionful, recognised by people, and expressed as an EPR algorithm. That means content-addressed, version-pinned, with a fitness measure, like the algorithms under `.epr-meta/elohim/algorithms/`. Every council form named in this memorandum (sortition floors, a representative elohim council, the nesting rule) is therefore a current candidate algorithm, held to the trust it builds, not a fixed doctrine. A better-evidenced structure can replace it the way a new manifest version replaces an old one. **The one non-symbolic holon is physical.** It is the Holochain DHT's own space: the network a DNA defines, with its membrane, its agents' arcs and its gossip neighbourhood. That physics is the hard forcing-function constraint the design lives by today. Wherever this memorandum sizes something by "what the holon can seat", including councils, pools, rollup fan-in and the head plane, the DHT physics is the binding limit. It is not one candidate among others.
- **The goal: the commons is one degree removed from the human hand.** No person holds the commons directly. The elohim council holds it, and people hold the floor that audits and governs every contestable result. Human hands carry it only during the network's infancy, as a stage that ends.
- **Jubilee.** It is the reconciliation mechanism mishpat requires, not a warrant for enclosure. The protocol is the overdue upgrade of that 1.0 primitive.
- **For the living (direction).** At death, a person's share passes to the commons, richly represented, and is never enclosed by inheritors. The details are open: see section C below.
- **The support button is give and join.** Givers take very little share. No beneficiary exists until a settlement that meets KYC. Legal questions go to counsel.

## Settled lines

*(Filled in as each reading returns.)*

### A. Settlement, money and the boundary

*Settled from the settlement reading (2026-10-15), with the elohim-council and one-degree corrections applied.*

- **A1 (S2.1) Pay only through channels that makers declared themselves.** A channel declared in a maker's own repository is the one act of the maker's that we can read from outside. It counts as a standing offer to receive. It is not a claim of the presence, so recognition stays held until the maker claims (manifesto:661, :680, :702). Any payee the maker did not declare is refused.
  - *Confidence:* medium-high.
  - *Reopens if:* a maker objects, or a channel turns out not to be controlled by its maker.
- **A2 (S2.2) Who rules on a crossing, and when.** The ceiling elohim council rules on each crossing, once per settlement window, using the reach gate's context. The pool's council only states the accepting terms in its charter. Until the ceiling council phases in, a **plural human council** stands in for it. That council is kept **separate from the sortition floor that audits it**, because the actor and the auditor are never the same body (values-forward:159, :169).
  - *Confidence:* medium.
- **A3 (S2.3 / C6) An inherited FUNDING.json is the ancestor's declaration, not ours.** It covers only the share explained by `DERIVED_FROM gitoxide`. The bridge reads the channel from gitoxide's own upstream, not from our copy. brit's own share stays held.
  - *Confidence:* high on the split. The file names only an `ownedBy` address; that it belongs to Sebastian Thiel is taken from the spec, not from the file. This must be verified before any payout.
- **A4 (S6) No accumulating treasury.** Money passes through a window, weighted by the lens the payer chose. What cannot be explained stays with the payer, and its recognition is held in trust. Any fiat leg sits with a licensed, swappable bridge institution, which is a member and never an authority (succession:507, :605, :608; monetary posture:1767, :1798, :1949). This passes the one-degree test: no person holds the commons's money.
  - **Canon debt:** the 2026-10-04 pool spec's "separate treasury resource" (:76) conflicts with succession:605. Canon outranks the spec, so that line is to be narrowed.
  - *Confidence:* high.
- **A5 (C4) Unclaimed money stays with the payer as a standing commitment.** Escrow would form the refused balance. Graduation follows a schedule declared in the commitment, readable before the payer enters. When it graduates, the payer releases it to a commons route the pool's elohim council approves. Recognition waits with no time limit (manifesto:716, :741; values-forward:131, :135).
  - **Needs the operator:** the length of the horizon; whether an unmet commitment lapses back to the payer; how the Jubilee rhythm maps onto money.
- **A6 (C5) The planetary commons receives recognition plus an in-kind route that raises the floor.** It has no money channel for now. Commons issuance stays held, unexercised and gated by preconditions (values-forward:121, :127; monetary posture:455-460).
  - *Confidence:* medium-high.
- **A7 (F-Q1) The facility clause's routes are, for now, in-kind recognition plus the care floor.** Settlement is added only once the outflow bridge (MN-7) exists. Settlement pays per window and holds nothing between windows. The pool's elohim council decides (humans in infancy), and a sortition council audits.
  - **Needs the operator:** the revenue ceiling for each kind. Canon says it should be graduated by friction, not a flat cap (manifesto:1014; values-forward:147).
- **A8 (F-Q2.1) The support button ships in stages.**
  - **First:** (a), a recorded pledge, labelled honestly and keeping "join".
  - **Next:** (c), pass-through to declared channels, once the channels are recorded and the bridge exists.
  - **Only after an operator ruling and counsel:** (b), custody without a beneficiary.

  (a) and (c) never hold the commons's money.
- **A9 (F-Q2.2) Before the ceiling council phases in, a plural human council acts at the boundary.** It is never a single steward of record. The steward of record may execute at the bridge as a legal person, but does not decide.
  - **Needs the operator:** what happens when an infant holon cannot seat two separate bodies (an actor council and an auditing floor).

### B. Labour, land and valuation

*Settled from the labour/land reading (2026-10-15), with the operator's later corrections applied: an elohim council governs each pool, and the commons is one degree removed from the human hand.*

- **B1 (C2) The three reason subjects stand.**
  - **Maker** is labour.
  - **Community** is positional, land-like value, read as how many rely on the work.
  - **Nobody** is value held in trust with its descent. It is never treated as land.

  The precision floor is a declared cost choice, never the boundary. Making it the boundary would turn tiny shares of labour into common inheritance, which is the confiscation Stance I.4 refuses ("no one here confiscates the produce of your labor", values-forward:133).
  - Held value graduates by Jubilee-style reconciliation, not on a timer.
  - *Confidence:* high on the mapping, medium-high on the floor.
  - *Reopens if* a ruling sends unrecorded common knowledge (language, mathematics) straight to the commons with no descent. That would need separating from forgotten labour.
- **B2 (C3a) Who negotiates the positional share.**
  - At a tier, the relied-on project's makers negotiate with that tier pool's governing council: its elohim council, with humans in that role during the network's infancy.
  - An unclaimed maker's seat is held by the steward pool.
  - The outcome is a **contestable lens clause that payers may adopt**. It is never a default allocation.
  - At global reach, the counterparty is the ceiling council (values-forward:149: positional matters graduate and "are not handed back to a vote").
  - *Confidence:* medium. Reliers have no seat of their own except through the tier council.
- **B3 (C3b) Fan-in as the first sensor.** Fan-in is acceptable only under four conditions:
  - it is declared as a lens clause;
  - it counts distinct projects or presences, never crates;
  - it is labelled as headcount, with "independence not observed" stated;
  - it is a reading, never a verdict.

  An independence-observing sensor (attested tallies, k-of-M) has to exist before positional shares settle any money. *Confidence:* medium-high.
- **B4 (S7) Who stewards a held presence.** The reader answered before the operator's correction. Settled here with the correction applied:
  - A held presence is part of the commons, so it sits one degree removed from the human hand. Its steward is a **path**: the steward pool, then the pool's elohim council (humans during the network's infancy), then a human sortition council that audits every contestable act.
  - This reconciles manifesto:627 ("stewarded by the Elohim until claimed") and :635 ("Elohim holds recognition") with the no-single-pool-elohim rule. The holder is the council, which is plural, witnessed and auditable, never one agent.
  - The elohim-commons-design rule that "settlement, recognition and a lens in force are never the elohim's" governs a member's *own* elohim acting for its person. It does not govern a pool's elohim council. **This seam is recorded below for a skill amendment.**
  - Backlog row 3 (a steward is a path) and row 26 ("never add a primary-steward field") stand.
  - Changing `ContributorPresence.steward_id` (`imagodei_integrity/src/lib.rs:522`) from a string to a path is an integrity edit that moves the DNA hash. It needs the p2p-design-gate, and zero new entry types is the target.
  - *Confidence:* medium. *Reopens if* the operator rules that recognition held in trust must sit with persons and not the council.

**Seam to amend (elohim-commons-design):** add a sentence. The person-holds rule binds a member's own elohim. A pool's commons is held by its elohim council (humans in infancy), audited by the human sortition floor, one degree removed from the human hand.

### C. For the living

*Settled from the for-the-living reading (2026-10-15). This is operator direction; it is not yet canon. The reading is argued openly, not assumed.*

The Jubilee returned *naḥalah*, the family's inalienable portion, to the family. Canon closes that gap by redefining inheritance as stewardship: "to receive it is to owe it onward" (values-forward:131). The operator's ruling then reads the Jubilee as the reconciliation that mishpat requires, a release of what had been gathered, so that this direction is its upgrade and not a break from it.

- **C1. Timing.** The stream graduates **after the witness period**, once the executor and the household have attested. Until then nothing moves and the stream stays with the presence, held in trust. After that, the maker stays named as the reason for each ratio and only the recipient changes (cradle-to-grave:189, :217; manifesto:655, :659).
  - Where no executor was named, the fallback is a successor steward, then a network-witness quorum (stewardship-over-sovereignty:133-134). *Confidence:* medium.
  - **Needs the operator:** a default length for the witness period.
  - **Reopens if:** law passes copyright to the estate. The protocol does not place itself outside the law (succession:491).
- **C2. Dependents.** They are met by the **care floor first, with priority** ("to care for orphans and widows… It **is** the floor", where-it-ends-and-where-it-begins:83).
  - A bounded share is admitted only if it is declared in advance in a testamentary Commitment, ends on a fixed date (for example, adulthood), cannot be transferred, and is drawn through the guardian.
  - Honest gap: the care floor is not built yet. Until it exists, a declared bounded share is the only protection that does not rest on a promise. *Confidence:* medium-low.
- **C3. Personal goods are outside this entirely.** Money already received, a home and belongings are the produce of labour and pass by ordinary law (values-forward:127, :133). Body, likeness, voice, medical records and private communications are inalienable and never graduate (ownership red-team design:300-301).
  - The land-rent part of a home remains common inheritance before and after death; death does not change that.
  - What is refused: an estate or perpetual foundation used as a channel to keep enclosing the stream. *Confidence:* high.
- **C4. Co-made works.** Only the deceased maker's line changes recipient; the living co-makers' lines are untouched. Attribution is "a derivation over events, never a declared share" (manifesto:690; actor-plane design:247). *Confidence:* medium-high.
- **C5. An organization's end is its apoptosis.** Dissolution is a governance act of the stewards. Constituents re-parent to the parent pool, members withdraw by height, and a tombstone keeps descent reachable. The organization's attribution stream passes to the **parent pool's elohim council** (humans during infancy, audited by sortition), applying the operator's correction.
  - Successors cannot claim the presence (manifesto:684).
  - Open: money held in the organization's legal-person layer (a co-op asset lock is a candidate). *Confidence:* medium.
- **C6. George's "2.0".** Not settled; **the operator must confirm**.
  - Repo canon cites only *Progress and Poverty*, and succession:205 already argues that the neglected part was George's moral claim, not his instrument (the single tax).
  - Candidates, ranked by the reader, unverified:
    1. *Progress and Poverty* Books VII–X (justice; "The Law of Human Progress").
    2. The 1878 "Moses" lecture, George reading the Jubilee as anti-monopoly design: the direct bridge from 1.0.
    3. *The Science of Political Economy* (1898, posthumous and unfinished).
    4. *The Condition of Labor* (1891, to Leo XIII).
  - Related: "for the living" echoes Jefferson's "the earth belongs in usufruct to the living" (1789).

### D. Commons governance and the charter

*Settled from the governance reading (2026-10-15), with both corrections applied.* Canon already backs the elohim-council ruling almost word for word. hardware-providence-commons:417-418 says "executive stewardship of commons value is exercised by the elohim". At :425 that stewardship is "kept a degree removed from the ordinary temptation to self-deal". At :430, "Human and constitutional floors inspect, narrow, pause, appeal, and revoke it". At :435, "'A degree removed,' not 'above.'" values-forward:127 adds "the graduation into true, elohim-council-held commons".

**The seam, settled.** Stance I.4's own line divides the two parties. The commons-design skill already draws that line (SKILL.md:36-39):
- **The facility** (the capacity and labour members commit) is members' produce. It stays under the eleven questions, and its rule-making, vote, admission and ladder are human acts. The skill's split at SKILL.md:65 ("for its own person") governs a member's own elohim and holds unchanged.
- **The commons share and held artifacts** (value that lands on no individual, and presences held in trust) are in the custody of the pool's **elohim council**, one degree removed from the human hand. Humans hold that custody during infancy, and the human sortition floor audits it.

This refines B4: a held presence falls on the commons side.

- **D1 (S8) Rows that block the build.** Rows **34, 36, 37 and 38** block `dependency-index`:
  - **34** proportionality, scoped to *facility* draws (the rollup compute), never to reads of the index ("an outsider who reads is not a free rider");
  - **36** graduated consequences;
  - **37** resource condition;
  - **38** plurality of judgment, now the governing body's own legitimacy.

  **Row 32** blocks the first *binding* closure, not the build. **Rows 33 and 35** follow. No row carries Q1 (congruence), so the charter must choose its own lens at the pool-spec step. *Confidence:* medium-high.
- **D2 (S5) What promotes a held presence.** A presence moves through the lifecycle unclaimed → stewarded → claimed. Two things promote it:
  - the maker's own attested claim;
  - becoming *stewarded*: value attributed to it must actually be held in trust (it crosses a threshold the charter declares), or it is contested.

  Reliance count alone never promotes. Accumulation stays in a fold, off the head plane. The stewardship act belongs to the pool's elohim council (humans during infancy). *Confidence:* medium. **Open:** the threshold's value and unit (shefa:316).
- **D3 (F-Q2.3) The canon correction.** The facility doc's §13 draft fails the one-degree test: it gave custody to a *human* council. **It is superseded by the text below**, proposed for `genesis/docs/content/elohim-protocol/governance-layers-architecture.md:128-138`. The steward lands it.

  > ## The Commons Co-Steward
  > Every collective (Qahal) is instantiated at genesis with an **elohim council that co-stewards its commons, a degree removed from any human hand**: plural in lineage, representative of the holon's diversity as far as it can be determined, and sized to what the holon can seat. There is no pool-elohim; no single elohim is or speaks as the collective. Its responsibilities:
  > - **Reflects** the commons interest the collective cannot articulate on its own
  > - **Holds custody** of the commons share — the residual value that cascades into the collective without landing on any individual receiver — in trust, conveying it to no one, including itself
  > - **Decides boundary crossings** — value entering from or leaving to an outside system, including settlement at the identity edge — at the ceiling, witnessed
  > - **Speaks in governance councils** as the voice of the commons; **mediates disputes**; witnesses standing decisions; convenes layered elohim arbitration councils on contested escalation
  > - **Cannot be silenced** — embedded at genesis (removable only never; reconfigurable via the collective's governable rubric)
  >
  > **In the network's infancy**, people the holon seats under the same representativeness and size rule hold these seats, until the ceiling earns the floor's trust act by witnessed act. This is a stage that ends.
  >
  > **Human sortition councils**, drawn to the reach of the decision (§Constitutional Councils), audit and govern any contestable result: they inspect, narrow, pause, appeal, correct or revoke its conduct, without taking custody and without voting past a graduated limit. Members keep the produce of their labor, including the capacity they commit to the collective's facility and the rules they agree for it (Stance I.4).

  - "Removable only never" is carried from the current text as written. Read it again before landing.
  - **Also reconcile `shefa.md`:** :55, :57 and :76 describe a council of "people the community seats, with the community's elohim as witnesses and counsel". Under the ruling that is the infancy stage, not the destination. :314 leaves "who seats the commons council" open.
- **D4. How councils are drawn.**
  - **The human floor is settled canon.** It uses cryptographic sortition, stratified to the population the decision's EPR boundary binds, with short terms, no consecutive service and paid seats (governance-layers:118-126; governance epic:75, :89-92; values-forward:159). Its size follows "small enough to talk, or nest".
  - **The elohim council's representativeness is unspecified.** **Needs the operator:** does "representative" mean lineage plurality (row 38), or one seat per stratum's elohim? Which strata can be determined without consent-violating correlation (boundary 5)? What exit test ends infancy? The canon gives only "act by witnessed act" (values-forward:89).
  - Canon also needs one sentence stating that the operator's "representative" elohim council and the governance epic's sortition "individuals, not representatives" are different councils.

**Skill amendment (elohim-commons-design).** Add the facility/commons-share split above at SKILL.md:65 and :125. Name which council at Q10 (:84). Make the same fix in pool spec :525-528 and in reliance spec §6.

### E. The human view, the vector cloud and signals

*Settled from the human-view reading (2026-10-15).* One shape runs through all four answers. The surface shows **fruit**: the cites a person authored and the attestations they received, quiet by default and drillable to their source. Readings such as degrees and similarity are plural lenses the person elects. Signals are strongest at the first relies-on hop and fade beyond it.

- **E1 (C1) Lead with the forward bibliography and the reverse "what your work meant".** The reverse view is built from specific attestations and relationship kinds: who was helped, what was derived, what community formed. It never shows counts or hop numbers. Degrees are an on-demand detail behind page info (manifesto:653, :663-667, :694, :716; values-forward:195). *Confidence:* high.
- **E2 (F-Q4) "Works near mine" is a secondary, elected lens.** It must be declared and plural. Similarity may *propose* a re-citation or a notice, but is never shown as a relationship that happened (manifesto:652; values-forward:205; private-thought-governed-fruit:53). A neighbourhood view over other people's works is lawful only at commons reach. *Confidence:* medium-high.
- **E3 (F-Q5) Criteria for the second embedding lineage:**
  - a different base model and training corpus;
  - open weights under a commons-compatible licence;
  - runs on CPU, on the person's own device;
  - published training data, so its re-citation debt can be stated;
  - multilingual coverage;
  - pinned by CID, with a fitness measure, in the same manifest shape as the first.

  Candidates such as nomic-embed-text-v1.5 and multilingual-e5-small are **unverified until licence-checked and measured** with `recall-bank-reach`. *Confidence:* medium on the criteria, low on the candidates.
- **E4 (C8) A retraction is pushed to reverse degree 1 over typed relies-on edges only.** Beyond that it is pulled, at each holon's own pace and on reconnect. The push radius widens only for a safety retraction at commons reach. This is what makes the floor's "corrections always reach the participant they concern" (private-thought-governed-fruit:26) true for direct reliers (manifesto:696; citation backlog:55-68).
  - In the probe this was 3 messages pushed, against 585 for a full push.
  - **Open:** what counts as "high reach" and "safety"; who runs the push from a derived reverse index; whether a push list leaks reliance (boundary 6).

### F. Substrate and technical

*Settled from the substrate reading (2026-10-15).* S1, the reader living in `bridges/cargo` with no pin moves, was already decided in the approved plan.

- **F1 (S3) Bridge-minted Compiler `DepEdge`s do not graduate.** They are computed from cargo's resolve and stay class C.
  - Gap #11 graduates one crystallized **sealed edge-set**, never single edges. A Compiler governor means "no seal: cite the unit" (`2026-07-21-sealed-contract-edges-governor-frontier-design.md`:113, :119-122, :216, :266; p2p-design-gate: "a *computed* edge is not" notarized).
  - The bridge is the "later accelerator" that sealed-edges §7:251 names, not a consumer of gap #11.
  - *Confidence:* high.
  - **Fix in the spec:** §5 classes bridge-minted edges as C. Correct the gap #11 citation (the reliance spec cites the fabric spec at :274 and :506; the source is the sealed-edges spec).
- **F2 (S4) A graduated lockfile summary is the existing elohim-DNA `EconomicEvent` with action `cite`, via Path 2.** No new entry type is needed. Integrity only checks that the action is non-empty, so the DNA hash does not move (`content_store_integrity`:227, :4398-4403; observation-layer design:409-436).
  - *Confidence:* medium-high.
  - **Three caller mismatches to build across:**
    - Path 2 summarizes by time window; ours is a versioned head per lockfile.
    - Path 2 reads storage's `observations` table; our cite events live in the flow store.
    - `total_quantity` is f64.
  - Supersession is chosen by the declaring lockfile, never by the newest link in the `triggered_by` chain.
- **F3 (S9) The SDK protocol schema owns `derived_from`.** It is DNA-notarized; `relationship-type.schema.json` gains a `_dna` mirror like reach's. Lamad's teaching edges stay in the lamad manifest. Unification follows the reach split's law: one generative source, everything else generated, contract-tested.
  - **Not a blocker:** the plan writes `.epr-meta` declarations only. Use the SDK's lowercase `derived_from`.
  - **Canon debt:** protocol-specification:864 still names the lamad manifest as the source, and storage's `VALID_RELATIONSHIP_TYPES` would reject it.
- **F4 (S10) The plan proceeds without naming any reach tier.** These are safe:
  - records with no reach field;
  - the flow store;
  - seed `ContributorPresence` promotion;
  - Path 2 `EconomicEvent`s.

  Never invent or canonize a tier string, including "public". The push-threshold reach (E4) and the binding-decision reach (row 32) wait for the pool spec. If a value is ever forced, the trajectory points to schema-8. *Confidence:* high.
- **F5 (C7) Read reliance from the lockfile:** declared, all platforms, identical on every host. Record each cite's target and dependency kind as metadata, so a per-target view can later be a lens and not a second reader. Actual per-build use belongs to rakia's input closure, beside the cites rather than replacing them. *Confidence:* medium-high.
- **F6. A figure to correct.** 1,211 is the number of distinct crates across all 35 lockfiles. `elohim/Cargo.lock` alone has 491 packages, and the largest lockfile (elohim-storage) has 1,066. The spec's §8 "1,211-crate scale for `elohim/`" is wrong; fix it before any probe asserts it.

### G. For counsel (not settled here)

- Securities, charitable-giving, money-transmission and KYC treatment of each support-button mode, (a) pledge, (b) custody without a beneficiary and (c) pass-through, in every jurisdiction the button would serve.
- Whether a maker's own declared funding channel can be paid without further consent in each jurisdiction.
- Estate law where copyright passes to the estate, and where forced-heirship rules reserve portions for children (C1, C2).

## Needs the operator's ruling (collected)

1. **A5:** the horizon for unclaimed money; whether an unmet commitment lapses back to the payer; how the Jubilee rhythm maps onto money.
2. **A7:** the revenue ceiling for each kind, graduated by friction.
3. **A9:** what an infant holon does when it cannot seat both an acting council and a separate auditing floor.
4. **C1:** a default length for the witness period.
5. **C6:** which George work is the "2.0" (candidates are listed above).
6. **D2:** the value and unit of the stewardship threshold.
7. **D3:** land the canon correction in governance-layers:128-138, and reconcile shefa.md:55-76.
8. **D4:** what "representative" means for an elohim council (lineage plurality, or a seat per stratum); which strata can be determined without correlation; the exit test that ends infancy.
9. **E3:** approve the second-lineage criteria. The candidate is chosen after a licence check and measurement.

## Mint pass (per `genesis/research/.epr-meta`)

The substrate reader drafted the row placement below. It was applied on 2026-10-15; the "Added as" column names the row each node became. Rows in the two prose-shaped clusters (dependency-graph, citation-apparatus) are numbered design observations, since those files carry no ranked table. Cluster files live under `genesis/data/timeline/backlog/`.

| Node | Cluster file | Row | Added as |
|---|---|---|---|
| MN-1 | dependency-graph-cited-contributor-economy | Ratio-table record keyed by (summary, lens, rollup) CIDs; lines sum exactly to 1, each with a reason and a lens clause | dependency-graph obs. 6 |
| MN-2 | measure-family-borrows-backlog | Exact magnitudes (rational, or minor units with a declared scale) replacing f64 in `Magnitude`, `total_quantity` and `recognition_score` | measure-family row 39 |
| MN-3 | arch-scale-risk-backlog | Recognition as a class C fold, not accumulating fields on a notarized `ContributorPresence` | arch-scale-risk row 11 |
| MN-4 | dependency-graph-cited-contributor-economy | Reliance `CoverageDomain` keyed by presence, with fan-in and k-of-M; `witness_quorum` is 0 today | dependency-graph obs. 7 |
| MN-5 | commons-holonic-stewardship-backlog | Tier pass clause in the charter lens, plus its REA event, from pool to parent pool | commons-holonic row 39 |
| MN-6 | commons-holonic-stewardship-backlog | Bridging-tier rollup across two DHTs, plus a reason-mapping ledger (`TranslationKind`/`SemanticCost`) | commons-holonic row 40 |
| MN-7 + F4 | dependency-graph-cited-contributor-economy | Settlement outflow bridge: pays only a channel declared in the maker's own repository, after the cash-out identity check; refuses any undeclared payee | dependency-graph obs. 8 |
| MN-8 | dependency-graph-cited-contributor-economy | Presence keyed by repository URL, grouping that project's crates | dependency-graph obs. 9 |
| MN-9 | commons-holonic-stewardship-backlog | Lens bound to held presences by the steward pool, replaced on claim; beside row 21 | commons-holonic row 41 |
| MN-10 | 2026-08-10-algedonic-phase2-network-phase3-dedupe | `reliance-account-closed@1` emitter with emit/clear hysteresis per holon; extends row 7 | algedonic row 8 (phase 2 table) |
| MN-11 | citation-apparatus-claim-grain-typed-edges-reach-gate | Implement rule 7: retraction pulled by default, pushed at degree 1 over relies-on edges | citation-apparatus obs. 11 |
| F1 | commons-holonic-stewardship-backlog | Flow-lens rule schema (exact rationals, named clauses, tiers, `version_parent`) in the `author-lens` payload | commons-holonic row 42 |
| F2 | measure-family-borrows-backlog | Recompute receipt (summary, lens, rollup, recipe CIDs plus the result hash) | measure-family row 40 |
| F3 | commons-holonic-stewardship-backlog | Private record of the giver's lens choice, published only as a k≥5 aggregate; extends row 10 | commons-holonic row 43 |
| F5 | commons-holonic-stewardship-backlog (into row 34) | Holder-side carrying record, summarized per window | folded into commons-holonic row 34 |
| F6 | arch-dataplane-refactor-backlog | Reverse-index rollups sharded by presence key under a Merkle root, under `MAX_PAYLOAD`; beside row 16 | arch-dataplane-refactor row 22 |
| F7 | commons-holonic-stewardship-backlog | Tiered rollups forwarding one row per child holon (fan-in at or below the branching factor) | commons-holonic row 44 |
| F8 | commons-holonic-stewardship-backlog | Memorial presence state after an attested witness period; the recipient of the labour line becomes the tier commons | commons-holonic row 45 |
| F9 | citation-apparatus-claim-grain-typed-edges-reach-gate | Vector proposal naming two or more manifest CIDs, confirmed by the elohim reading the source; no cite from similarity alone | citation-apparatus obs. 12 |
| F10 | commons-holonic-stewardship-backlog | Dissolved pool's constituents re-parented under a tombstone | commons-holonic row 46 |
| F11 | commons-holonic-stewardship-backlog | Giver-joins-pool capacity commitment, with the collective as recipient; beside row 31 | commons-holonic row 47 |
| F12 | commons-holonic-stewardship-backlog (into row 35) | Charter: the eleven declarations, a rule-change path, and an elohim council with a sortition floor | folded into commons-holonic row 35 |
| F13 | commons-holonic-stewardship-backlog | Custody without a beneficiary; its only exits are an identity-checked settlement or a council-approved commons route | commons-holonic row 48 |
| F14 | commons-holonic-stewardship-backlog (into rows 32/38) | Boundary act as a reach-gate decision: witnessed, plural-lineage, overturnable by the sortition floor | folded into commons-holonic row 32 (with a cross-reference clause in row 38) |

The "valueflow fabric" is not a cluster in `CLUSTERS.md`, so nothing rides it.

## Proposed canon correction awaiting the steward

`genesis/docs/content/elohim-protocol/governance-layers-architecture.md:128-138`. The before and after text is in `genesis/research/commons-facility-at-scale-2026-10-15.md` §13. It applies the no-pool-elohim / representative-council ruling.
