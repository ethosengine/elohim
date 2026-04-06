# The Protocol Pipeline

## The Pillars

The protocol has pillars at two levels. Three are **substrate** — they are the protocol's coupling infrastructure. Two are **domains** — they are implementations over the pipeline, each with their own manifests, instruments, and vocabulary.

**Substrate pillars** (the protocol's infrastructure):

| Pillar | Domain | The capture risk |
|--------|--------|-----------------|
| **imagodei** | Identity & attestation | Surveillance platforms |
| **shefa** | Value & economics | Rent extraction, banking |
| **qahal** | Governance & consent | Captured institutions |

**Domain pillars** (implementations over the pipeline):

| Pillar | Domain | The capture risk | Pipeline instrument examples |
|--------|--------|-----------------|------------------------------|
| **lamad** | Learning & wisdom | Credentialing gatekeepers | DEMONSTRATE via assessment, WITNESS via retention check |
| **avodah** | Work & action | Labor exploitation | DEMONSTRATE via work product, WITNESS via peer review |

Lamad and avodah are peers — both project onto the same seven-verb pipeline, both declare their vocabulary via domain manifests, both produce evidence that flows through the substrate pillars. Lamad is the first implementation being built. Avodah will be the second. The pipeline is what they share; the instruments are what they own.

No pillar stands alone. Wisdom without economics is ivory-tower irrelevance. Economics without governance is extraction. Governance without identity is tyranny of the anonymous. The protocol's promise is that these pillars are *coupled* — you cannot exercise power through one without bearing responsibility through the others.

## The Pipeline

The pipeline is the protocol's composition of the pillars. It is the path that any content, any claim, any action takes as it moves through the coupled system. Every stage of the pipeline touches value (shefa) and governance (qahal). Identity (imagodei) grounds who is acting. Domain pillars like lamad and avodah implement the pipeline with their own instruments and vocabulary — lamad's DEMONSTRATE is an assessment, avodah's DEMONSTRATE is a work product, but both produce evidence that flows through the same substrate.

No vocabulary is perfect. The seven verbs below are the protocol's names for these stages. But the same pattern has been discovered by every tradition that studies how humans coordinate — cybernetics, economics, philosophy of language, theology. This document is a Rosetta Stone. The protocol verb is the interface. The other lenses are the documentation.

When a verb feels abstract, the other columns give it texture: the cybernetician explains *why* the stage is structurally necessary, the REA economist shows *what economic primitive* it maps to, the speech act philosopher reveals *what kind of binding commitment* it creates, and the Hebrew root reminds us *what the verb meant before we flattened it into English*.

---

## The Seven Verbs

| # | Protocol Verb | Hebrew Root | Cybernetics | REA Economics | Speech Acts | One Sentence |
|---|--------------|-------------|-------------|---------------|-------------|-------------|
| 1 | **CLAIM** | טָעַן (ta'an) — bear a burden | Reference signal | Conditional Commitment | Assertive + Commissive | Assert a burden you are willing to carry, with a validity horizon and falsifiable outcomes |
| 2 | **ENCOUNTER** | פָּגַשׁ (pagash) — meet on the road | Perturbation / boundary crossing | Resource use (attention) | Perlocutionary effect | Two parties meet; attention is given; neither controls the meeting |
| 3 | **DEMONSTRATE** | עָשָׂה (asah) — make, produce | System output | Transformation process | *(folded into commitment)* | Produce observable evidence — make the thing, don't just describe it |
| 4 | **WITNESS** | עוּד (he'id) — cause to stand as testimony | Sensor / measurement | **Attestation** *(new primitive)* | Assertive + Commissive hybrid | Bind yourself to what you saw; testimony before community |
| 5 | **BECOME** | הָיָה (hayah) — I will be what I will be | *(not modeled)* | *(not modeled)* | *(not modeled)* | The agent is transformed — not credentialed, *changed* |
| 6 | **SETTLE** | שָׁלֵם (shalem) — wholeness restored | Actuator / correction | EconomicEvent | Declarative | Wholeness restored — value flows because obligation is complete |
| 7 | **RENEW** | חָדַשׁ (chadash) — make new | Model update / integration | *(claim at different scope)* | *(redundant if upstream strong)* | The mind updates; what counts as valid evidence evolves |

### What the lenses reveal

**BECOME and RENEW are invisible to economics and linguistics.** REA doesn't model transformation of the agent. Speech act theory doesn't model learning. These are the stages that make this a *wisdom* protocol, not just a transaction protocol. Only process theology — the tradition that sees reality as *becoming*, not *being* — names them as irreducible.

**ENCOUNTER is contested.** Cybernetics and Hebrew see it as a real stage (boundary crossing, the contingent meeting on the road). REA calls it telemetry below the ontological level. Speech acts say it's not a binding act at all. The protocol keeps it because *attention is the first gift*, and gifts that aren't named aren't valued.

**WITNESS is stronger than OBSERVE.** Three of four lenses independently arrived at the same conclusion: the observer must bind themselves to what they saw. Passive observation has no regulatory force in a distributed system with no central authority. Witness implies attestation — a second party stakes their reputation on the record.

**The genuine gap in REA is conditionality.** The REA theorist's sharpest contribution: REA knows commitments (planned events) and events (actual transfers). It does not know "this commitment only matures when evidence clears a warrant threshold." That conditionality — claims that are satisfied only through witnessed demonstration — is the protocol's real extension to economic ontology.

---

## Developer Ontology

The seven verbs are the protocol's vision. The schema is what the developer actually touches. This section maps between them — starting from what the schema already declares, walking back to the pipeline verbs.

### Schema primitives → pipeline verbs

| Schema Primitive | Type | Pipeline Verb | What it does |
|-----------------|------|---------------|-------------|
| `coupling.claims[]` | Manifest declaration | **CLAIM** | Content type asserts outcomes, contradictions, validity horizons |
| `coupling.claims[].validityHorizon` | ISO 8601 duration | **CLAIM** | How long before the claim must be re-demonstrated |
| `coupling.claims[].contradictedBy` | Observation reference | **CLAIM** → **WITNESS** | Names the negative observation that would falsify this claim |
| `SubstrateSignal` enum | Protocol enum | **ENCOUNTER** | `attention`, `compute`, `storage`, `bandwidth`, `energy`, `time`, `resource` |
| `EngagementType` enum | Protocol enum | **ENCOUNTER** | `view`, `quiz`, `practice`, `discuss`, `create`, `peer`, `teach`, `apply` |
| `ContentType` enum | Protocol enum | **DEMONSTRATE** | The artifact vocabulary — what kinds of evidence a domain produces |
| `ContentFormat` enum | Protocol enum | **DEMONSTRATE** | How evidence is rendered — `markdown`, `sophia-quiz-json`, `epr-composite`, etc. |
| `CreateContentInput` | Wire type (input) | **DEMONSTRATE** | Creates the artifact that serves as evidence |
| `CreateAttestationInput` | Wire type (input) | **WITNESS** | Agent attests to content — binds themselves to what they saw |
| `InstrumentArchetype` enum | Protocol enum | **WITNESS** | `retention-check`, `outcome-correlation`, `distribution-health`, `cost-accumulation`, `outcome-divergence`, `community-report` |
| `ObservationPolarity` enum | Protocol enum | **WITNESS** | `positive` or `negative` — every observation either supports or strains a claim |
| `vocabulary.observations` | Manifest declaration | **WITNESS** | Domain's evidence vocabulary — must include at least one negative-polarity observation |
| `MasteryLevel` enum | Protocol enum | **BECOME** | `not_started` → `seen` → `remember` → ... → `create` (Bloom's taxonomy) |
| `CreateEconomicEventInput` | Wire type (input) | **SETTLE** | Full REA event — `action`, `provider`, `receiver`, `resourceConformsTo`, quantities |
| REA `action` field | String (REA vocabulary) | **SETTLE** | `use`, `consume`, `produce`, `transfer`, `cite` |
| `coupling.value.onConsume/onComplete/onContribute` | Manifest declaration | **SETTLE** | Declares what REA events each interaction produces |
| `ValidationStatus` enum | Protocol enum | **RENEW** | `valid`, `migrated`, `degraded`, `healing` |

### Pipeline verbs → schema maturity

| Pipeline Verb | Schema Maturity | What exists | What's missing |
|--------------|----------------|-------------|----------------|
| **CLAIM** | Manifest-only | Claims declared per content type in manifest with validity horizons and contradictions | No runtime wire type — no `CreateClaimInput` to track individual claim instances, no claim lifecycle (active → challenged → expired → renewed) |
| **ENCOUNTER** | Mature | `SubstrateSignal` + `EngagementType` enums, captured automatically via signal harness | Nothing missing — encounters are protocol infrastructure, not domain declarations |
| **DEMONSTRATE** | Mature | `ContentType` + `ContentFormat` enums, `CreateContentInput` wire type, full rendering pipeline | Nothing missing — this is the most established surface |
| **WITNESS** | Strong | `CreateAttestationInput` wire type, `InstrumentArchetype` + `ObservationPolarity` enums, manifest observations | No observation accumulation wire type — individual attestations exist but the running tally against a claim is not schematized |
| **BECOME** | Lamad-only | `MasteryLevel` enum covers learning transformation | No protocol-level transformation primitive — `MasteryLevel` is lamad's projection of BECOME, but qahal ("the polity learned something") and shefa ("contributor standing changed") have no equivalent. Missing: a generic `TransformationRecord` that any domain can project onto. |
| **SETTLE** | Mature | Full REA wire types (`CreateEconomicEventInput`, `EconomicEventView`), well-typed actions | Nothing missing — REA is the most complete schema surface |
| **RENEW** | Implicit | `validityHorizon` on claims, `ValidationStatus` enum (`valid`/`degraded`/`healing`) | No explicit renewal cycle — validity decay is declared but the re-evaluation trigger, the renewal event, and the updated claim are not schematized. The loop closure is conceptual, not typed. |

### The three gaps

Three pipeline verbs have schema gaps that would need to be filled for a domain beyond lamad to implement the full pipeline. These are documented here for review — they become load-bearing when a second domain (qahal, shefa, avodah) tries to implement the full pipeline.

**Gap 1: CLAIM has no runtime lifecycle.**

Claims exist in the manifest (what a content type *asserts*) but not as runtime objects (what a *specific piece of content* has claimed, whether that claim is still active, and what evidence has accumulated against it). A developer building on the SDK can declare claims but cannot query "show me all degraded claims in my domain."

*Possible primitive:* `ClaimInstance` — created when content is published, tracks state (`active` → `challenged` → `expired` → `renewed`), links to the manifest claim declaration and the accumulated observations.

**Gap 2: BECOME has no protocol-level transformation type.**

`MasteryLevel` is lamad's answer to "what does transformation look like?" But it's domain-specific — Bloom's taxonomy is a learning concept, not a governance or economics concept. If qahal needs to record "this community's governance posture shifted after deliberation," there's no protocol type for that.

*Possible primitive:* `TransformationRecord` — generic enough for any domain. Fields: `agentId`, `domain`, `fromState`, `toState`, `evidence` (links to witnesses), `authorizedBy` (the elohim discernment that approved it). Each domain declares what states mean in their manifest.

**Gap 3: RENEW has no explicit cycle.**

Validity horizons exist. `ValidationStatus` captures degradation. But the moment when accumulated negative observations trigger re-evaluation, and the event of a claim being renewed (or retired), are not typed. The loop closure happens "somehow" — it needs to be a first-class protocol event.

*Possible primitive:* `RenewalEvent` — triggered when a claim's validity expires or accumulated observations cross a threshold. Records: which claim, what triggered it, what the new validity horizon is (or that the claim was retired). Links a `ClaimInstance` back to fresh evidence, closing the loop.

---

## The Elohim Layer

The elohim layer is not a single gate between two pipeline stages. It operates across a range — from producing witnesses to authorizing transformation to narrating what happened.

```
CLAIM → ENCOUNTER → DEMONSTRATE → WITNESS ──→ BECOME → SETTLE → RENEW
                                     ↑    ↓              ↑        |
                                     |  [elohim]         |        |
                                     |  discern ──────────┘        |
                                     |  observe                    |
                                     └─────────────────────────────┘
```

The cybernetics agent called this the **comparator** — the element every viable control system requires to compare output against reference. The speech acts agent called it **adjudication** — the declarative act that changes institutional state. Both independently discovered the same structural necessity: *someone must compare evidence against claims before value flows*.

In the protocol, that someone is not a human judge, not a smart contract, not a voting mechanism. It is the elohim — AI mediators who carry the constitutional context, the accumulated witness, and the therapeutic posture needed to discern well.

The elohim do not decide *for* people. They create conditions where good discernment happens.

### Three postures across the pipeline

| Posture | Pipeline range | What they do |
|---------|---------------|--------------|
| **Observer** | WITNESS | Produce attestations from system signals, automated instruments, pattern detection. These are first-class witnesses, not second-class telemetry. |
| **Discerner** | WITNESS → BECOME | Compare evidence against claims. Weigh context. Authorize transformation. The elohim push left into WITNESS (producing observations) and right toward BECOME (evaluating sufficiency). |
| **Narrator** | BECOME → SETTLE | Tell honest stories about what happened and what it means for value flow. Mirror (to the learner), Advisor (to the steward), Sentinel (to governance). |

RENEW feeds all three postures. Each cycle through the pipeline — each claim validated or invalidated, each transformation witnessed, each settlement completed — updates the model of what "good and acceptable and perfect" looks like. The elohim get wiser because the system renews.

> *Do not be conformed to this age, but be transformed by the renewing of the mind, so that you may discern what is the will of God — what is good and acceptable and perfect.* — Romans 12:2

The verse names the entire pipeline: transformation (BECOME), renewal (RENEW), discernment (elohim). The system learns to discern better through accumulated renewal.

---

## Domain Projections

The seven verbs are the protocol. Each domain fills in what they mean. The following tables show how four domains — two that exist today (lamad, a2o) and two being built (shefa, qahal) — project onto the same pipeline.

### Lamad (learning)

| Verb | What it looks like |
|------|--------------------|
| CLAIM | "This lesson teaches distributed consensus" — validity 30 days, contradicted by retention failure |
| ENCOUNTER | Learner opens the content; attention signal emitted |
| DEMONSTRATE | Learner completes a Sophia assessment; score produced |
| WITNESS | Sophia scores automatically; peer reviews attest quality; retention check at interval |
| *elohim* | Compares assessment evidence against mastery claim; considers learner's path context |
| BECOME | Mastery level advances — the learner *can now do something they couldn't before* |
| SETTLE | Mastery credit flows to learner; recognition flows to content steward |
| RENEW | Retention failures shorten validity horizons; stewards revise content; elohim recalibrate what "mastery" means for this concept |

### Shefa (economics)

| Verb | What it looks like |
|------|--------------------|
| CLAIM | "This work will produce value for the commons" — commitment with deliverables |
| ENCOUNTER | Worker engages with the task; resource allocation begins |
| DEMONSTRATE | Work product delivered — artifact, service, contribution |
| WITNESS | Peer review, outcome measurement, community report |
| *elohim* | Evaluates whether delivered work matches commitment; considers contributor history |
| BECOME | Contributor standing grows — the worker's *relationship to the commons changes* |
| SETTLE | Value flows to contributor; stewardship standing adjusts |
| RENEW | Accumulated outcomes refine what "value for the commons" means; demurrage cycles prevent hoarding |

### Qahal (governance)

| Verb | What it looks like |
|------|--------------------|
| CLAIM | "This proposal improves the commons" — with stated outcomes and validity horizon |
| ENCOUNTER | Community member reads proposal; deliberation begins |
| DEMONSTRATE | Votes cast, arguments made, consent or objection registered |
| WITNESS | Results tallied; participation attested; dissent recorded |
| *elohim* | Weighs outcome against proposal's claims; carries minority voice; checks constitutional alignment |
| BECOME | The community's governance posture shifts — *the polity learned something* |
| SETTLE | Policy enacted; governance weight adjusts; obligations created |
| RENEW | Policy outcomes validate or invalidate the proposal's claims; governance mechanisms evolve |

### a2o (the protocol testing itself)

| Verb | What it looks like |
|------|--------------------|
| CLAIM | "This scenario describes correct behavior" — BDD feature file as falsifiable assertion |
| ENCOUNTER | System executes the scenario; CI pipeline runs |
| DEMONSTRATE | Pass/fail result produced; observation report generated |
| WITNESS | Test results attested; linked to scenario EPR; coverage tracked |
| *elohim* | Compares regression patterns against system health claims; detects systemic drift |
| BECOME | The system's *reliability posture changes* — confidence earned or lost |
| SETTLE | Build confidence adjusts; deployment authorization granted or withheld |
| RENEW | Regression patterns inform new scenarios; the protocol's understanding of its own health evolves |

---

## The SDK Contract

When you build a domain on the protocol, you declare your half. The protocol provides its half. The pipeline is the seam between them.

### What you declare (in your domain manifest)

| Pipeline stage | What your manifest declares |
|---------------|---------------------------|
| CLAIM | `coupling.claims[]` — what outcomes each content type asserts, what contradicts them, validity horizons |
| ENCOUNTER | Nothing — the protocol tracks encounters via substrate signals. You get this for free. |
| DEMONSTRATE | `vocabulary.contentTypes` — what artifacts your domain produces as evidence (assessments, work products, ballots, scenarios) |
| WITNESS | `vocabulary.observations` — the evidence vocabulary: positive and negative observations, mapped to instrument archetypes. Must include at least one negative-polarity observation. |
| BECOME | `coupling.value.onComplete` — what transformation your domain recognizes (mastery attestation, contributor standing, governance weight). Coupled to demonstrated evidence. |
| SETTLE | `coupling.value.onConsume`, `onComplete`, `onContribute` — REA event declarations: what action, what resource, what recognition flows |
| RENEW | `coupling.claims[].validityHorizon` — how long before the claim must be re-demonstrated. The protocol handles decay, obligation generation, and re-evaluation cycles. |

### What the protocol provides

| Pipeline stage | What the protocol gives you |
|---------------|---------------------------|
| CLAIM | Conditional commitment infrastructure — claims stored, tracked, expired |
| ENCOUNTER | Attention tracking, substrate signal capture, encounter history |
| DEMONSTRATE | Content rendering, assessment infrastructure (Sophia), artifact storage |
| WITNESS | Attestation infrastructure, instrument harness, observation accumulation |
| *elohim* | Observer (automated witnesses), Discerner (evidence evaluation), Narrator (honest stories) |
| BECOME | Transformation records, identity state updates, capability graph |
| SETTLE | REA economic event processing, value flow, recognition distribution |
| RENEW | Validity decay, obligation generation, claim re-evaluation cycles |

### The boundary test

**"Am I declaring a stage of the pipeline, or building an instrument that serves a stage?"**

Stages are protocol. Your manifest declares how your domain fills them.

Instruments are yours. A Sophia quiz is lamad's instrument for DEMONSTRATE. A ballot is qahal's instrument for DEMONSTRATE. A peer review rubric is shefa's instrument for WITNESS. The protocol doesn't care *how* you demonstrate or witness. It cares *that* you declared claims with validity horizons, observations with negative polarity, and coupling across all three legs.

### Substrate primitives

Beneath the seven verbs, the protocol measures substrate signals. These are domain-agnostic — they flow regardless of whether the domain is learning, economics, or governance. They are the protocol's own witness.

| Substrate Signal | What it measures | Pipeline stages where it flows |
|-----------------|-----------------|-------------------------------|
| **attention** | Time and focus given to content | ENCOUNTER, DEMONSTRATE |
| **compute** | Processing resources consumed | DEMONSTRATE, WITNESS, elohim |
| **storage** | Data persisted to the network | CLAIM, DEMONSTRATE, WITNESS |
| **bandwidth** | Data moved between peers | ENCOUNTER, SETTLE |
| **energy** | Physical resource cost of participation | All stages |
| **time** | Duration — the irreversible substrate | All stages |
| **space** | Place occupied, cohesion, carrying capacity | ENCOUNTER, DEMONSTRATE, SETTLE |

Space is the oldest captured substrate — landlordism predates every other form of rent extraction. The protocol already senses space (`Place`, `CarryingCapacity`, `atLocation` on economic events) but naming it as a substrate signal makes it governable. Space applies at every level: physical (land use, permaculture vs monoculture, suburban sprawl vs community cohesion), digital (namespace, DHT neighborhood), social (governance jurisdictions, household colocation), and pedagogical (where learning happens shapes what can be demonstrated).

This is an emergent, open set — these are the substrate signals discovered so far. As new domains are built on the protocol, new substrate primitives may reveal themselves.

### What is a substrate?

A substrate is an underlying resource or condition that a system requires to function, which the system's own success tends to degrade as an unaccounted externality (Schmachtenberger's "generator functions of existential risk"). Industrial agriculture's success erodes soil. The attention economy's success erodes attention quality. Suburban development's success erodes community cohesion and land. Financial acceleration's success erodes trust. The pattern recurs: success generates externalities that accumulate in the substrate until the substrate collapses under the system's own weight.

The test for whether something belongs in this list: **can the protocol's own success degrade it?** If more content competes for finite attention, attention is a substrate. If more data makes retrieval harder, storage is a substrate. If more development degrades carrying capacity, space is a substrate. If the protocol doesn't measure it, it becomes invisible — and invisible substrates are the ones that collapse.

### Measurable substrates vs emergent substrates

The seven signals above are **measurable substrates** — resource dimensions the protocol can directly instrument. But the protocol also depends on conditions that emerge from healthy substrate flows:

| Emergent substrate | Degrades when... | Made visible through... |
|-------------------|-------------------|------------------------|
| **Trust** | Transactions happen without good witnessing | Pattern of attestations, observation polarity, claim validity decay |
| **Coherence** | Knowledge accumulates without organization | BECOME frequency, renewal cycles, claim expiration rates |
| **Relationship** | Interactions happen without depth | Encounter duration, reciprocity in witness patterns, community signal health |

The protocol doesn't measure trust directly — it measures the pattern of attestations and polarity that trust is made of. It doesn't measure coherence directly — it measures whether BECOME is happening (agents transforming) or stalling (credentials accumulating without change). The measurable substrates are the instrumentation. The emergent substrates are what the pipeline produces when those measurements are governed well, and what degrades when they aren't.

This is where the elohim layer becomes essential: the elohim infer emergent substrate health from patterns in the measurable signals. An elohim can detect that trust is eroding by reading accumulating negative-polarity witnesses, even though "trust" isn't a signal any domain emits. The measurable substrates are what domains declare. The emergent substrates are what the elohim discern.

Domains map their semantic signals onto substrate signals in the manifest:

```
Domain signal (semantic)     →  manifest mapping  →  Substrate signal (measured)
"learning-signal"            →  substrateSignal   →  attention
"assessment-completed"       →  substrateSignal   →  compute
"contribution-created"       →  substrateSignal   →  compute
"peer-review-completed"      →  substrateSignal   →  compute
```

The protocol captures substrate measurements automatically. Your domain gives them meaning; the protocol gives them measurement. This is what makes the elohim's Observer posture possible — the elohim don't need to understand lamad's vocabulary to detect that a piece of content is consuming attention without producing demonstrations. The substrate tells the story the domain might not.

---

## Appendix: Four Lenses

The seven verbs were not designed by committee. They emerged from asking four independent intellectual traditions to evaluate the same pipeline, each from first principles. The traditions were chosen because they don't share assumptions: cybernetics studies control systems, REA studies economic ontology, speech act theory studies binding commitments, and Hebrew process theology studies relational becoming.

What follows are the full analyses, presented as received. Where they agree, we have convergence. Where they disagree, we have the tensions that keep the vocabulary honest.

### A. Cybernetics (Beer, Wiener, Meadows)

The current proposal has a structural problem: it conflates loop initiation, loop execution, and loop closure without distinguishing the **error signal** — the most important element in any control system.

More critically: there's no MODEL stage. Every viable control system requires an internal model of the world against which observations are compared. Without it, you have a pipeline, not a regulator.

**Derived verb set (7 stages):**

1. **ASSERT** — stake a claim with a validity horizon and expected evidence signature. Creates the reference signal. Without a reference, there's nothing to regulate against.

2. **ENGAGE** — an agent enters the system boundary and is perturbed by the asserted content. ENCOUNTER is passive. ENGAGE implies the boundary crossing that makes observation possible. This is Ashby's "requisite variety" moment.

3. **DEMONSTRATE** — produce behavior that can be witnessed as evidence. The output of the controlled system.

4. **WITNESS** — record observations against the expected evidence signature. OBSERVE is fine but WITNESS implies attestation — a second party binds their reputation to the record.

5. **ARBITRATE** — compare witnessed evidence against the asserted reference signal; compute the error. This is the comparator — the stage everyone forgets. The comparator IS the intelligence of the system. In Beer's Viable System Model this is System 3's role.

6. **SETTLE** — route value and obligation based on the error signal from ARBITRATE.

7. **INTEGRATE** — update the system's model of what constitutes valid evidence for this class of claim. This is learning — the model update that makes the system a regulator rather than a one-shot pipeline. Meadows calls this a "thermostat that recalibrates itself."

**Verdict:** The original six stages collapse ARBITRATE into SETTLE (dangerous — hides the comparator) and name FEEDBACK when they mean INTEGRATE. Seven stages. No stage is optional without losing regulatory viability.

### B. Speech Act Theory (Austin, Searle, Habermas)

The existing proposal conflates *events* with *acts*. Encounter and Observe are events. The protocol needs acts — utterances that create binding obligations and transfer accountability.

Habermas's validity claims reveal what's missing: every genuine speech act implicitly raises *truth* (is this accurate?), *rightness* (is this appropriate in this context?), and *sincerity* (does the speaker stand behind it?).

**Derived verb set (5 stages):**

1. **INVOKE** (declarative) — "I open this space for a specific kind of value to be demonstrated." Creates the arena. Establishes what counts as evidence.

2. **COMMIT** (commissive) — "I bind myself to producing evidence by a horizon, with something at stake." Not just assertion — obligation. This is what transforms CLAIM + ENCOUNTER into a speech act pair with accountability.

3. **ATTEST** (assertive + commissive hybrid) — "I witnessed this and stand behind its truth." The observer binds themselves. Separates testimony from mere observation.

4. **ADJUDICATE** (declarative) — "Given the attestations, the claim is validated or invalidated." Changes the institutional state of the claim. This is where Habermas's *rightness* validity is established by the community.

5. **SETTLE** (declarative) — value flows.

**Verdict:** FEEDBACK collapses into the gap between ATTEST and ADJUDICATE. If attestations are properly bound, accumulation *is* the protocol. The unit of analysis isn't stages in a pipeline — it's *who bears accountability at each transition* and *what kind of binding act creates that accountability*.

### C. Hebrew Process Theology (Whitehead, Heschel)

The pipeline moves from DEMONSTRATE to OBSERVE to SETTLE without a stage for *transformation of the agent*. In process theology, each occasion of experience must produce a new subject, not merely a transaction record. Hebrew verbal morphology enforces this: qal stems describe action, but niphal/hithpael stems describe reflexive transformation. The pipeline is almost entirely qal.

**Derived verb set (7 stages):**

1. **CLAIM — טָעַן (ta'an)** — assert a burden you are willing to carry. Ta'an carries weight-bearing connotation — the claimant picks up an obligation, not just a proposition.

2. **ENCOUNTER — פָּגַשׁ (pagash)** — two parties meet on the road; neither controls the meeting. Better than יָדַע (yada') here — yada' is knowledge-through-intimacy, which belongs later.

3. **DEMONSTRATE — עָשָׂה (asah)** — produce the thing — not perform it, *make* it. Asah is the verb of Genesis 1's creative acts: observable, external, assessable output.

4. **WITNESS — עוּד (he'id)** — cause something to stand as testimony before a community. He'id is not passive observation — the witness takes on legal/covenantal standing.

5. **BECOME — הָיָה (hayah)** — **the missing stage.** Hayah is not static being; "I will be what I will be" is pure processual becoming. After demonstration and witnessing, the agent must be transformed. Without this, the pipeline produces transactions, not learners.

6. **SETTLE — שָׁלֵם (shalem)** — not merely payment. Shalem means wholeness restored. Cognate with shalom. Settlement without wholeness-accounting reproduces debt logic, not gift-circulation.

7. **VALIDATE — חָזַק (hazak)** — strengthen or confirm what has accumulated. Observations accumulate until a claim stands or falls. Hazak (to strengthen, as in "be strong and courageous") captures that accumulated witness either reinforces or breaks the original claim.

**Verdict:** The pipeline enforces coupling only if BECOME is non-skippable. If a domain can SETTLE without BECOME, value can flow without transformation, and the three legs decouple. Make BECOME the axle.

### D. REA Accounting Theory (McCarthy, Geerts, Hruby)

**What REA already covers:**

SETTLE is a standard EconomicEvent. ENCOUNTER maps to a Commitment or planned event (attention as reciprocal resource flow). DEMONSTRATE is partially covered when it produces a resource artifact.

**What is genuinely outside REA:**

REA's ontological scope is economic exchange. It deliberately excludes *why* a transfer is warranted. The genuine gap is **epistemic warrant** — the causal chain that authorizes an EconomicEvent.

**The minimal extension is two concepts:**

1. **Claim** — a conditional commitment: "EconomicEvent E will occur if and only if evidence E' is produced and observed." REA Commitment says "I plan to transfer." Claim says "transfer is authorized when assertion is substantiated."

2. **Attestation** — a signed observation that grounds the conditional. Not operational logging, but a warranting act that advances the epistemic state of a Claim toward settlement authorization.

FEEDBACK is not a distinct stage — it's an Attestation whose target is a Claim's aggregate validity rather than a single event. Same concept, different scope.

**On duality:** The pipeline preserves REA duality only if Claim is modeled as a bilateral commitment: the claimant commits to demonstrate, the counterparty commits to observe and settle. If Claim is unilateral assertion, duality breaks.

**Verdict:** The protocol isn't reinventing REA. It's adding one missing layer: conditionality. REA knows commitments and events. It doesn't know "this commitment only matures when evidence clears a warrant threshold." That's the real gap.

---

## Why This Matters

Every form of domination depends on hiding an externality. The landlord hides the soil erosion. The platform hides the attention degradation. The credentialing body hides the fact that the gate serves them, not the learner. The financial system hides the trust it consumes. The pattern is always the same: make the substrate invisible, extract from it until it collapses, move on.

The seven verbs are not a software pipeline. They are a description of how humans actually grow in community — you take on a burden (CLAIM), you meet something that challenges you (ENCOUNTER), you produce something real (DEMONSTRATE), someone sees it and stakes themselves on what they saw (WITNESS), you are changed (BECOME), wholeness is restored (SETTLE), and your understanding of what matters deepens (RENEW). This cycle is ancient. It happens in apprenticeships, in congregations, in families, in therapy. It is how people have always grown.

What is new is that this cycle can be captured. The university captures DEMONSTRATE → WITNESS → BECOME and charges rent at the gate. The platform captures ENCOUNTER and sells the attention. The bank captures SETTLE and takes a cut. Every institution that extracts value is a parasite on one stage of a cycle that should be whole.

The protocol re-couples the cycle. You cannot settle without witnessing. You cannot witness without demonstration. You cannot claim without bearing the burden. And the substrates — the soil the whole thing grows in — are measured, so the externalities cannot hide.

The elohim layer exists because measurement alone is not enough. Substrates need interpretation. Evidence needs context. Transformation needs discernment. The elohim are not judges — they are honest storytellers. They make the invisible visible: where the coupling is fraying, where power is concentrating without corresponding responsibility, where the substrate is being consumed faster than it renews.

If you are building on this SDK, you are not building an app. You are building a domain where the ancient cycle of human growth — claim, encounter, demonstrate, witness, become, settle, renew — can happen without anyone capturing the gate. Your manifest declares what that cycle looks like in your domain. The protocol ensures it stays whole.

The fruit goes back on the tree.

---

*This document was produced through collaborative design on 2026-04-05/06. The four-lens analysis was generated by independent agents, each constrained to derive from first principles of their tradition without anchoring on the existing proposal. The convergences and tensions between them shaped the final vocabulary.*
