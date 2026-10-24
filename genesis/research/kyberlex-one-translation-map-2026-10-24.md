---
title: "Open Networked Earth (kyberlex/ONE) and the Elohim Protocol — a translation map"
id: kyberlex-one-translation-map-2026-10-24
status: Capture
date: 2026-10-24
sovereignty-frame: bridge-legibility
---

# Open Networked Earth and the Elohim Protocol: a translation map — October 2026

## In plain language

**Open Networked Earth (O.N.E.)** is a project by a systems engineer who writes under the pen name
*Kyberlex*. It is a written constitution for a society after paid work, together with novels, a
browser simulation, a scientific simulator in Julia, and a plan for building first communities. Its
core ideas are:
- the essentials of life guaranteed to everyone;
- land and tools held for use rather than owned;
- councils of ordinary citizens chosen by lottery;
- an economy counted in physical units (energy, water, food) instead of money.

**The Elohim Protocol** is a peer-to-peer system: people's own devices hold and share the data, with no
central server. It is designed so that each piece of content travels together with its value and its
governance, as one record. Parts of that design are built; binding governance rules to records is not
yet. It aims to be ground that communities can build on, each with its own charter, within protections
its wider constitution keeps for everyone.

**The two have already met.** Kyberlex and the operator (Matthew, who stewards Elohim) talked in a
community chat on 22–24 October 2026. Kyberlex had their AI research assistant read the Elohim
repositories and found the two projects "remarkably complementary". Matthew suggested that
thermodynamic accounting could be one *policy* among others, applied over Elohim's economic records.
Kyberlex agreed. They proposed a first experiment: Elohim's event log feeding ONE's thermodynamic ledger
or its simulator.

**This map** takes each ONE institution and asks how Elohim could carry it: as a record, a measure, a
policy or a community charter. It does not judge which mechanism is right, theirs or ours. Each is
treated as one option communities could hold side by side, within those wider protections. One
difference between the two projects stays open: Kyberlex treats physical limits as given, and asks
how communities respond inside them. Matthew asks which reading of the physical facts a community
adopts as its working agreement. Agreeing to experiment does not settle that.

**What exists, and what doesn't** (from the source we read; we did not run either system):
- *Elohim has, in code:* identity, community rosters, deliberation records, economic events in physical
  units, and hosting for apps.
- *ONE has, in code:* a browser simulation balancing energy, water and food, and a Julia model of one
  village. The village model is checked by a method its collaborator Rainer Rodrigues brought, and that
  method is one we can learn from.
- *Neither has shown:* a lottery anyone can check, ratification recorded on a charter, measurement of the
  physical world that others can verify, or any of these institutions serving people over time.
  Records can carry measurements, but sampling, calibration and independent labs still do the physical
  work.

**What could come next:**
1. **The experiment Kyberlex proposed.** The first missing piece is an export of physical-resource
   events that their simulator can read.
2. **A letter**, if the operator chooses.
3. **One open question to hold together.** Each project has a different shape at its top, and each
   should ask the other how its own shape looks from outside.

Nothing here is committed to yet.

---

## Words this map uses

Plain English comes first; the project's own term follows in parentheses. Elohim names its parts in
Hebrew, and those names appear here only as labels.

**Elohim terms**

| Plain term (project term) | What it means here |
|---|---|
| **record (EPR, Elohim Protocol Record)** | One addressable unit that carries content together with its knowledge, value and governance. It is identified by a fingerprint of its exact bytes, so it cannot be silently changed. |
| **fingerprint (CID)** | A content-derived identifier: the same bytes always give the same ID. "Pinned" means fixed to one exact version. |
| **economic events and stocks (epr-rea, after REA/ValueFlows)** | REA (Resource–Event–Agent) is an accounting model that records what actually happened rather than only money totals; ValueFlows is its open vocabulary for cooperatives and commons. Here, economic activity is recorded as events: who did what, with which resource, in what quantity and unit. A stock (how much of something there is) is always recomputed from the events and never stored, so any two peers reading the same events get the same answer. |
| **measure (middot)** | A named, versioned procedure for observing something, such as a kWh count. A measure reports; it never enforces. |
| **policy (mishpat lens)** | A rule that interprets measures and may bind a decision. Several policies can apply to the same record at once. When they conflict, one has to be chosen to govern the decision; who chooses is set by the community's own charter (for instance its councils). Elohim's design for this is still a draft. |
| **procedure record** | A procedure (a lottery method, a threshold rule, an allocation method) stored as a record in its own right. A policy applies it; a charter names it. |
| **charter layer** | A community's own constitution. It sits inside wider layers and may make anything stricter, but may not violate the wider layers' protections. |
| **human floor / AI ceiling** | Human councils drawn by lot, which can always overturn the network's AI. The AI's role (the *ceiling*) grows only as it earns trust. |
| **the network's AI (elohim)** | AI participants bound by the constitution. Each may witness, explain and propose; none may rule unchallenged. |
| **learning paths (lamad)** | The knowledge and learning part, which arranges content into chaptered paths. |
| **economy (shefa)** | The value and stewardship part. |
| **community governance (qahal)** | Collectives, rosters and deliberation. |
| **identity (imagodei)** | Persons and their devices. The name means "image of God": dignity no one earns or grants. |
| **justice (mishpat)** | Repair and restored capability, never punishment. It is also where policies live. |
| **reach** | How far something may travel, and to whom, earned rather than granted by an algorithm. Corrections travel back along the same paths. |
| **holon** | A nested community (household, neighbourhood, region) that is whole in itself and part of something larger. |
| **attested / witnessed** | Recorded together with who vouched for it and how. An attestation says who checked a claim; it does not by itself make the claim true. |
| **threshold signing / secret sharing** | A key split into parts so that several holders must act together; no single holder can use it alone. |
| **stewardship grant** | Scoped, expiring, appealable permission to care for something without owning it. |
| **story scenarios (a2o)** | Plain-language scenarios, written as stories, that run as automated checks. |

**ONE terms**

| Term | What it means here |
|---|---|
| **usufruct** | Use without ownership: the right to use and enjoy something while caring for it. |
| **sortition / demarchy** | Selection of officials by lottery / government by such bodies. A *stratified* lottery draws so the result mirrors the population (by age, gender, place). |
| **zero-knowledge proof of personhood** | A way to prove you are one real person without revealing who you are or any biometric data. |
| **ETI, External Trade Interface** | ONE's boundary with the money economy. Outside money is converted into physical goods quickly. |
| **Track A / Track B** | The existing economy / the commons built alongside it. |
| **Genesis Lock** | The moment the founder's power to amend ends. Amendment then passes to lottery-drawn assemblies. |
| **SIMONE / O-ASIS** | ONE's Julia simulator / its AI debate engine for stress-testing the constitution. |
| **Heretic's Commons** | 7% of all shared facilities reserved for work outside the consensus. |

**How to read citations.** ✅ means we read it in the source; ◐ means inferred or single-source; ✗ means
nothing found. The status words **Code**, **Spec** and **Not yet** say whether something exists in code,
only in a design text, or not at all. The paths and line numbers are there so anyone can check; they are
not needed to follow the argument. ONE sources: `C:` the constitution, `F:` the foundation statutes,
`Q:` the simulation register, `A:` the attack-challenges file. Repositories read: ONE at `b359ce9`,
Elohim at `33835eeb3`.

---

## How this map is written

Three commitments shape it. The operator set the first two during the pass.

1. **No mechanism is held as authoritative, theirs or ours.** Elohim is still learning its own social
   architecture. Its contribution is the ground that *holds* patterns. A 15-person council, a 75%
   threshold or an energy-accounting method is a procedure record, applied by a policy and named in a
   charter. Communities could hold several at once.
2. **Epistemic humility, in our own voice.** Both projects describe more than they have built. Each is
   described in the same plain terms. Our own tree has unfinished scenarios, unimplemented verification
   and open questions in its constitution.
3. **The map runs one way here, and the other way is invited.** This document translates ONE into
   Elohim's categories, and the words "carry" and "hold" cast Elohim as the container. That can read as
   absorbing ONE, which is not the intent. Elohim could equally be
   read through ONE's categories: our charters as its assemblies, our records as its ledgers, our
   network AI as its "servant code". We would welcome that reverse map.

---

## Where Kyberlex is coming from

This section draws only on what Kyberlex has written publicly about the project, for the purpose of
collaborating well. Kyberlex writes under a pen name, and we respect that. We write "they", since no
pronouns are stated, and read nothing in the novels as biography. The Internet Archive podcast is an AI-generated audio overview
(NotebookLM), so its voices are not Kyberlex's own (`podcast_highlights.md:2–3` ✅).

**In their own words.** Answering their first outside collaborator (GitHub Discussion #11, 2026-10-11):

> "To be completely honest, I'm a systems engineer/developer, not an academic mathematician or Julia
> veteran. My past experience with discrete-event simulations comes from semiconductor fab logistics
> (routing thousands of wafers across machines and transport systems). Those models were often far from
> reality because people cut corners or got lost in uncalibrated complexity. That's why I don't want an
> AI-hallucinated monster model that looks impressive on paper but is broken underneath."

They say the architecture took "years" to codify (#13).

**The concern they start from** ✅:
- **The problem:** AI and robotics end wage livelihoods while infrastructure stays privately owned. ONE
  "was created to turn automation into human liberation" (portal `data/synoptic.json:7–11`).
- **The urgency:** functioning commons should exist as "physical lifeboats" before a collapse can favour
  corporate dependency or warlordism (`roadmap/roadmap.md:89–122`).
- **The way to start:** with neighbours, not with any seizure of power. Land trusts, microgrids and
  workshops are built beside the existing economy, and people are drawn in because necessities arrive
  reliably.

**Whom they draw on** ✅. These are cited across the corpus (`COMPARATIVE_ANALYSIS.md:29–183`, `PLAY.md:28`,
portal `synoptic.json:28`). Some are adopted, some contrasted, and some written in as adversaries:
- **Economics:** Leontief and Kantorovich; Cockshott and Cottrell, *Towards a New Socialism*; Stafford Beer
  and Cybersyn; Ostrom, Mauss, Polanyi; Georgescu-Roegen. Fresco's Venus Project serves as a contrast;
  Mises, Hayek and Friedman serve as the opposing economist.
- **Governance:** Athenian lottery government; Condorcet; Bookchin; Öcalan; Rojava's Social Contract; the
  *Earth Constitution*; **Michel Bauwens and the P2P Foundation**.
- **Law:** Roman usufruct; Community Land Trusts; Swiss foundations and associations; Mexican ejidos.
- **Ecology:** Buen Vivir; the constitutions of Ecuador and Bolivia; communal-labour traditions such as
  Minka, Talkoot and barn raising.
- **Building:** Passivhaus, WikiHouse, OpenStructures, Barcelona superblocks.
- **Ethics:** restorative justice and nonviolent communication. The novels set Christian, Islamic,
  Confucian, Marxist and secular voices in dialogue.
- **Fiction:** Kim Stanley Robinson's *Mars Trilogy* is named as the closest predecessor.

**How they hold their work** ✅. Their calls for collaborators (#9–#12) set out open questions and
doubts; the economics call says "We are not dogmatic. We need to verify" (#9). They ask whether physical accounting hits a calculation
bottleneck, and how to validate models "without circular AI-generated assumptions" (#11). Their loop is
"Critics test → co-stewards analyze → we patch and improve → re-verify."

**How they collaborate** ✅. Their first collaborator, Rainer Rodrigues, a Julia modeller, proposed this
plan:
- start with one village, modelled two ways (whole-village equations and individual agents);
- check that the two agree, which is "docking" (Axtell et al. 1996);
- write the conservation laws as tests.

Kyberlex's response:
- answered "an absolute yes";
- named their own limits;
- told Rainer not to spend time on the browser sim, calling it "a browser-based visual toy";
- on 2026-10-22, granted him "full technical ownership and freedom over `simone/`" (#11;
  `simone/README.md:24, 100–106`).

They offer the same autonomy to the media, economics, governance and law, and habitat roles they are
still seeking.

**What they ask for** ✅. First, a media and community co-steward, to end the "solo-creator
bottleneck" (#13). The project issues no tokens and sells no subscriptions (#1).

**Where the project is set.** The constitution is translated into fourteen languages. Proposed first
communities are spread across contrasting climates on four continents, and the novels move from Svalbard
through Detroit, the Alps, the Andes and Malacca to Geneva.

**Where our languages already meet** ◐. These are meeting points in ideas; they do not mean Kyberlex knows
our work:
- survival unconditional, with reciprocity only above that floor (C:243–249);
- land held through use and care (C:25, 63);
- repair that never requires the harmed person to reconcile (C:414);
- human authority over code (C:341–344);
- care counted as full contribution, with "zero perpetual civic stigma" (C:249, 278).

The Bauwens/P2P Foundation lineage is the most direct shared ancestor we found.

---

## The conversation so far, and the idea at its centre

**The exchange (22–24 October 2026, in a community chat channel; text supplied by the operator).**
- **Kyberlex introduced ONE.** They then had their AI research assistant study the Elohim repositories.
  They valued "keeping it strictly local-first and resisting the crypto-token speculation trap". They
  described the binding of content, economic accounting and governance into "an inseparable unit" as
  "a very elegant substrate". They named their own focus as anchoring coordination to the physical layer
  (energy, calories, wear) and to legal asset-locks.
- **Matthew answered with the idea this section explains,** below.
- **Kyberlex agreed with the framing.** They proposed: "an experimental crossover down the road where an
  Elohim EPR event log feeds directly into a local thermodynamic ledger or SIMONE testbed."

**The idea, in plain terms.**

1. **The record holds the facts.** Every economic happening (a harvest, a repair, an hour of care) is a
   *record* of what moved, how much, in what unit and in which community (economic events, epr-rea).
   Matthew's words: "the whole point of REA is to keep the economic process subject to explanation… you
   have to explain WHAT the flows of what STUFF is." How much of something there is (a *stock*) is never
   stored; it is recomputed from the events. The crate's own description: "two peers folding the same
   events mint the same stock with no shared clock" (`elohim/epr-rea/src/lib.rs` ✅). What *should* be
   counted is not fixed in the substrate: "What should be metered is a governance decision expressed as
   policy" (`elohim/epr-rea/README.md` ✅).
2. **Thermodynamics is one way of reading the facts.** Physical limits apply whether or not anyone
   reads them. What a community chooses is how much weight that reading carries in its decisions.
   Counting energy, mass and machine wear is a *measure* over the records; using it to decide something
   is a *policy*. Matthew: thermodynamics "IS a valid computable lens… over the RAW economic processes";
   it "becomes a policy (ie. mishpat), that is held in tension with the atom of data". Other readings,
   such as care, meaning or ecology, sit beside it.
3. **Peers change policy when the readings say so.** If the aggregated physics shows a way of working
   should change ("we're making a lot of x, but the physics… is invalidating the value"), peers
   re-evaluate which policy they follow. The aim is the one Matthew names, "a contextualized state within
   a dynamic system of human flourishing", not a fixed optimum.
4. **AI carries the authoring burden.** People don't keep such records today because writing them down is
   exhausting. Large firms do keep them, and Matthew names the "asymmetric supply chain dominance" this
   gives Walmart. In his words, AI's proper role "isn't a GOD, or a friend, but it's a partner authoring
   contextual stories at scale". Kyberlex agreed: "letting AI absorb the authoring overhead so humans can
   focus on the normative layer". They described their own practice as home sensors (Home Assistant,
   ESP32) feeding local models that narrate what happened.

**Where the two readings sit, side by side** ◐. Kyberlex describes the physics as fixed limits: "physics
doesn't negotiate… the boundary conditions of reality", with culture and policy happening inside them.
Matthew describes thermodynamics as one lens a community may adopt as its working agreement. Neither says
physical limits stop applying. The question between them is whether the physical reading should *always*
bind decisions, or bind them when peers adopt it. The architecture can express both:
- the energy count as a *measure*, which only reports;
- a *policy* binding that measure as a floor, if peers choose that.

epr-rea already lets an economic commitment include a limit (a `Bound`). Exceeding it raises a notice
naming the affected commitment (`elohim/epr-rea/src/lib.rs` ✅). Whether a physical limit is a
chosen policy or a given floor is a question the two projects can explore together. It need not be
settled first.

**The first experiment, honestly stated.** Kyberlex's crossover: Elohim events feed their thermodynamic
ledger or SIMONE.
- **What exists.** The event type is serializable (`FlowEvent`, `elohim/epr-rea/src/model.rs:377`, Code ✅).
  Stocks can be exported as JSON with `epr flow stocks --json`, but today only for the project's own
  development records (Code ✅).
- **What is missing.** An export of *physical-resource* events (energy, water, food, wear) in a form
  their simulator can read (Not yet ✗).
- **Who would do it.** It is unassigned, and neither side has agreed yet to do it.

---

## Where each project stands today

"Code" means present in the source we read. Neither project's software was run.

| | ONE | Elohim |
|---|---|---|
| **In code** | A browser sim that balances energy, water and food each tick (`sim/app/src/engine/thermodynamics.js:475` ✅); signed citizen passports (`citizen_passport.js:59` ✅); browser-to-browser sync and local storage ✅; a Julia single-village test comparing whole-village equations with individual agents, with three agreement checks and ten conservation checks (`simone/test/test_docking.jl:29–31` ✅), built on Rainer Rodrigues's plan | Identity enrollment, community rosters, deliberation and challenge records, economic events in physical units, local-first storage, app hosting (file:line in the map below) ✅ |
| **Described, not yet in code** | Leontief allocation and climate-data ingestion (the SIMONE spec labels itself "PROPOSED", `simone/simone-specs.md:4` ✅); a verifiable lottery (the sim uses `Math.random()`, `sortition.js:93` ✅); consensus among peers on world state (today a relay commits snapshots to git, `scripts/snapshot_relay_apps_script.js:150–176` ✅) | A lottery draw (none found in a search of our code ◐); a path from a proposed charter amendment to an accepted version; verification of constitutions on the network (`elohim/constitution/src/verification.rs:108` is unimplemented ✅); the collective-governance story scenarios (marked work-in-progress ✅) |
| **How the canon changes** | Kyberlex's commits; Rainer Rodrigues holds write access to `simone/` since 2026-10-22. After the Genesis Lock, 75% ratification by lottery-drawn assemblies (F:184–186 ✅) | Stewarded by the operator. How the first global layer is ratified is an open question in our own constitution (`constitution.md:908` ✅) |
| **Authorship** | AI-assisted, as the corpus states (`books/dossier.md:324`) ✅ | Written together with AI agents, including this map ✅ |

Two things we noticed in their sim code, offered as one peer to another:
- private "whisper" messages derive their key from public data and also broadcast the plain text
  (`sim/app/src/engine/chat_engine.js:45`, `:453` ✅);
- incoming changes are accepted as valid unless particular signature fields are present (`p2p_mesh.js:424` ✅).

---

## Where each project works in the stack

The two projects start from opposite ends of the same stack. That is why "enabler" describes the fit.

**ONE starts from the charter and the story** ✅:
- **Code:** a browser simulation built on Vite, three.js, Leaflet and Trystero (`sim/app/package.json`),
  plus a Julia simulator.
- **Primitives taken from the browser and hosted services:**
  - identity is WebCrypto in the browser (`citizen_passport.js:59`);
  - randomness is `Math.random()` (`sortition.js:93`);
  - storage is IndexedDB and localStorage;
  - peer sync is WebRTC through Trystero;
  - shared state is snapshots committed to GitHub through a hosted relay
    (`scripts/snapshot_relay_apps_script.js:150–176`).
- **The constitution *legislates* foundation discipline:** no proprietary software in public commons
  (C:349–354); bit-for-bit reproducible builds checked by diverse compilation (C:356–361); keys split
  across assemblies (C:363–369). The requirement is written into ONE's law, not yet into its code.
  For a five-week-old project, beginning with the charter is a reasonable order.

**Elohim starts from the substrate** ✅. For months the work has gone into foundations, written mostly in
Rust and kept as close to the machine as practical:
- **records:** `elohim/epr` and the economic layer `elohim/epr-rea`;
- **storage** with two peer-to-peer transports (libp2p and iroh);
- **Holochain applications** for identity, knowledge and justice;
- **brit (covenant):** a Git-compatible tool that connects "what a developer intended, the exact source
  they changed, who reviewed it, what was built" (`elohim/brit/README.md`);
- **rakia (firmament):** builds dispatched "to diverse peers", because "when only one person can build
  the thing, that's power concentrated through technical obscurity" (`elohim/rakia/README.md`);
- **lvi:** development environments that are "a covenant you mount", hosted by peers for each other
  (`elohim/lvi/CLAUDE.md`).

The operator's aim is that the foundations for the protocol's honest evolution arrive together with the
protocol itself. Any project or story built on them would then be born honest, governable, evolvable
and unenclosable, limited only by its own reach.

**Both still stand on others' work** ◐. Elohim relies on the Rust toolchain, a forked Holochain, iroh,
libp2p, Linux, and Angular for its app. Much of its foundation is unfinished, as the map below shows.
Building low in the stack is a discipline, not a finish line.

**Where this meets ONE.** ONE's Article 7 describes the foundation a commons needs. Elohim is trying to
build one. If that work holds, ONE's charter could run on ground that meets its own Article 7, instead of
on browser defaults it would otherwise have to replace.

---

## How the pieces could fit

```
  ONE's novels and field guides        ──►  learning paths (lamad), chaptered
  ONE's sim / game                     ──►  an app served from peers' own machines
  ONE's constitution                   ──►  a charter layer a community may adopt, inside wider layers
  ONE's procedures (lotteries,         ──►  procedure records, fingerprinted and governed,
     thresholds, consequence ladders,        applied by policies (mishpat), several side by side
     allocation methods)
  ONE's physical units and checks      ──►  measures (middot): versioned, they report but never enforce
  ─────────────────────────── Elohim ground ───────────────────────────
  identity · community rosters · economic events and stocks · witnessed attestations · local-first sync
```

Our constitution says what a community layer may do (`constitution.md:164`, `:782` ✅):
- it may make anything stricter within the wider bounds;
- its amendment rules must preserve deliberation, minority protection, transparency, reversibility and
  time;
- it may not override the wider layers' dignity protections, or the human floor's power to overturn the AI.

This is one possible integration. How ONE's own document relates to it is for the collaboration to decide.

---

## The map, institution by institution

Each row gives:
- **ONE institution**, with its citation;
- **What it would be**: how Elohim could represent it;
- **Elohim today**: Code, Spec or Not yet;
- **What would be needed**: what the ground would need in order to hold it.

The last column concerns the ground, never whether the mechanism is right.

### Identity and membership

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| Civic register; proof of personhood without biometrics (C:207, 367) | A personhood attestation, one kind of witness among others | **Code:** person creation and device enrollment (`doorway-service/src/routes/auth_routes.rs:1632`, `elohim/eprfs/epr-cli/src/actor.rs:1717` ✅). **Spec:** identity grounded in people who know you (`shefa.md:197` ✅) | A way for a charter to declare which kinds of attestation it accepts, including ONE's zero-knowledge proof as one option |
| Membership in a community; below 50 adults a 3-person panel governs (C:198) | A charter rule over a roster | **Code:** collectives, charters, rosters, withdrawal (`holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs:58`, `:128`, `:716` ✅) | A policy that can read roster size |
| No close kin serving together (C:198) | A seating constraint | **Not yet** | Seating constraints a lottery procedure can declare |

### Choosing councils and deciding

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| Stratified, "publicly verifiable cryptographic and mechanical lotteries" (C:207) | A **lottery procedure**: who is eligible, how they are stratified, the random source, a proof | **Not yet** in code (search ◐). **Spec:** a human floor of councils drawn by lot (`values-forward.md:159` ✅), drawn "to the reach of the decision" (`governance-layers-architecture.md:126` ✅) | The ability to fix *any* lottery procedure, run it reproducibly over a snapshot of the roster, and record a proof anyone can check, with several communities' procedures side by side |
| Council sizes 3/15/101/301/1001; terms; a break twice the term (C:197–210) | Parameters of a seating procedure | **Not yet** | Versioned parameters, with terms and breaks tracked. Our canon sizes councils by what a community can seat; that is one more candidate rule. |
| 75% / 60% / 50%+1 thresholds; 30-day expiry of emergency decrees (C:172–179, 222–226) | A **threshold rule** | **Code:** six vote-counting methods (`elohim-storage/src/tally/mod.rs:108` ✅), with one defect (below) | Counting against everyone eligible, not only those who voted; timed re-ratification for expiring decrees (**Not yet**) |
| Rival non-voting expert panels; caution by default; a plain-language duty (C:214–220) | A **deliberation procedure** | **Code:** deliberation records (`holochain/dna/mishpat/zomes/mishpat/src/lib.rs:567` ✅); opinion clustering (`elohim-storage/src/sensemaking/clustering.rs:36` ✅) | Procedures declared over the deliberation record, so each step leaves a trace |
| Staff may not frame agendas (C:210) | A constraint on where agenda items come from | **Not yet** | A record of each agenda item's origin |

**The one defect named here is ours** ✅. We found it in our own source while preparing this map:
- **Turnout is never compared with the eligible population.** One quorum check (`check_quorum`,
  `elohim-storage/src/tally/mod.rs:121–125`) receives only the number who voted. Any percentage quorum is
  therefore met by a single voter.
- **A percentage field is ignored.** One status calculation (`derive_status`,
  `elohim-storage/src/db/governance_action_tally.rs`) never reads the percentage it is given.

The second was already on our list (filed 2026-08-23). Other paths work: the approval method applies its
threshold to those who voted (`elohim-storage/src/tally/approval.rs:69` ✅).

### Charter and amendment

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| 46 articles, rights no majority may reduce, 75% amendment (C:69–71, 222–224) | A **charter layer** | **Code:** precedent records (`mishpat/src/lib.rs:377` ✅); constitutional layers on a device (`elohim/constitution/src/stack.rs:174` ✅); authoring a policy (`mishpat/src/commitments.rs:705` ✅). **Not yet:** binding a policy to a record; an amendment → accepted-version path | A charter must be able to mark which clauses can never change and which can be amended. ONE's text holds both readings (C:70–71 beside C:224); ours has its own acknowledged tensions (`constitution.md:309`). |
| The simulation register: statuses from hypothesis through AI stress-test to lottery ratification (Q:10–29), and entries noting "canonical merges" (Q:232–337) | A trail of ratification records | **Code:** story scenarios ✅. **Spec:** a charter-amendment path | Ratification recorded on the charter itself, with its count attached, for any community's procedure |

### Economy

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| Accounting in kWh, kg, litres and cycle time (C:131–137) | **Measures** over economic events | **Code:** events with quantity and unit (`holochain/dna/elohim/zomes/content_store/src/lib.rs:17437` ✅); stocks recomputed by resource, unit and period (`elohim/epr-rea/src/stock.rs:398` ✅), with partial unit checks (`:121`, `:470`) | General unit conversion and attested measurement (**Not yet**). See the conversation section above. |
| Leontief–Kantorovich planning matrices (C:137) | An **allocation procedure** over stocks | **Not yet** on either side | Allocation procedures kept as versioned records, so several can be run and compared |
| No money inside the commons (C:131–145, 467) | A charter-layer choice | **Spec:** in the operator's words, money "if needed is the design consideration that's subordinated last": weighed only after everything else, and only if needed. Any money issued would be held in trust by a commons layer stewarded by the network's AI. People would join because it serves them, not because they are made to. Issuance only from settled, verified reciprocity (monetary-posture survey ✅) | Unknown whether more is needed. The operator reads the two as the same destination by different instruments; that is his reading, not an agreement yet expressed. ONE's own Art. 9.1 grows "not through ideological coercion, but by offering immediate, verified, and reliable life support". |
| External Trade Interface: outside money committed within 24 hours (C:467–469) | A boundary policy | **Spec:** our posture treats settlement outside the network, in any token, as a governance act taken in context at the boundary | Boundary acts recorded with the policy that governed them |
| Inter-node equalization pool (F:493–496) | A redistribution policy | **Spec:** a commons pool as a party to members' commitments | ONE specifies the budgets and remittances; neither side has it in executable form |
| Three tiers of need, survival first (C:147–154) | A priority policy | **Spec:** shefa's declared priorities (`shefa.md:157` ✅) | Priorities as declared, comparable policies |
| Debt jubilee; adverse possession (C:461, 482–487) | Transition policies | **Spec:** our canon reads Jubilee as reconciliation on a negotiated schedule (`values-forward.md:131` ✅) | Both routes recorded side by side |

### Stewardship of goods

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| Personal possessions / dwellings held for use (CLT-style) / commons held in trust (C:86–95) | A goods taxonomy in a charter, plus **stewardship grants over resources** | **Code:** stewardship grants cover devices only (`imagodei_integrity/src/stewardship.rs:126` ✅); resource services keep data in memory (`elohim-storage/src/services/resource_service.rs:552` ✅) | Scoped, expiring, appealable grants over land, dwellings and tools (**Not yet**) |
| Tool libraries with timed checkout and repair restitution (C:89–91) | A small commons pool | **Spec** | A concrete first pool to model |
| Machine work on shared equipment is published (C:151) | An attestation on job outputs | **Not yet** | — |

### Justice and reciprocity

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| Three-stage restorative ladder; victims' priority; review of detention (C:413–436) | A **consequence procedure** | **Code:** challenges and outcomes with reasoning (`mishpat/src/lib.rs:214`, `:1512` ✅). **Not yet:** the full path from case through adjudication to restored standing | The full path |
| Refusing shared shifts suspends lottery rights and surplus access, restored on return; survival never revoked (C:242–251) | A reciprocity policy | **Spec:** "a guaranteed minimum of participation no deliberation can go beneath" (`succession.md:272` ✅); our current commons-design policy keeps every step past a first notice with human judgment | Both arrangements can be declared as procedures. ONE's text has an automatic step (A:50) beside mediation-first (C:245); a declaration would make the order explicit. |

### Code, keys and AI

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| "Code shall never act as law"; algorithms serve (C:341–347) | A value | **Spec:** AI is "never a tool, never a sovereign," phased in as a witnessed participant (`values-forward.md:167` ✅) | Both refuse AI rule. The words for AI's standing differ, and we record both. |
| Root keys split across many assemblies (C:363–369) | A key-custody arrangement | **Code:** custodian and threshold manifests (`imagodei/src/lib.rs:4031` ✅); secret-share reconstruction (`elohim-storage/src/recovery/share_assembler.rs:728` ✅), not yet tested end to end | Support for declared threshold-signing arrangements, with their scope and authority recorded |
| Reproducible builds, independent compilers, red-team bounties (C:356–386) | Integrity measures over builds | **Spec:** "any peer from the runtime can rebuild the substrate" (`where-it-ends-and-where-it-begins.md:53` ✅) | Rebuildability as a measure; exact determinism is not yet our commitment |

### Measurement and ground truth

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| Public sensors limited to non-living variables; no personal or domestic profiling (C:156–160) | Observation measures with a privacy bound | **Code:** observation intake (`elohim-storage/src/api/observations.rs:245` ✅), but unsigned (`:339` ✅) | Signed observations |
| Citizen ground-truthing: sealed duplicate samples checked by labs drawn by lot from distant watersheds (C:161) | A **verification measure** that uses a lottery procedure | **Code:** attestation plans from diversity checks (`elohim-storage/src/graduation/attestation.rs:32` ✅) | Measures whose checking step can name a randomly drawn, independent checker. This depends on the lottery capability. |

### Federation and islanding

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| The watershed as the basic unit; federation up to 1,001 delegates (C:502–551) | Layer structure | **Spec:** watershed as a functional layer (`governance-layers-architecture.md:31`, `:79` ✅). **Code:** collectives of collectives (`qahal_coordinator.rs:196` ✅) | — |
| Rights of nature (C:510–512) | A standing declaration | **Spec:** an epic grants a river legal personhood (`governance/epic.md:456` ✅) | — |
| 30 days self-sufficient; long-range radio mesh; manual fallbacks (C:287–322) | Operating conditions | **Code:** local-first storage (`app/elohim-app/src/app/elohim/services/write-buffer.service.ts:806`, `elohim-storage/src/sync/doc_store.rs:120` ✅) | Governance ceremonies that work offline ◐ |
| A Swiss foundation holding land and IP, dissolving on "planetary unification" (F:14, 22, 169) | A legal bridge at the boundary | **Spec:** the commons is "not owned by a foundation holding it in trust"; co-ops, land trusts and fiscal hosts take part as legal persons (`values-forward.md:97`, `succession.md:519` ✅) | Legal persons recorded as stewards of pieces at the boundary. ONE calls its foundation temporary, and both routes can be held. |

### Story and simulation

| ONE institution | What it would be | Elohim today | What would be needed |
|---|---|---|---|
| Six-volume novel series | **Content**: chaptered learning paths | **Code:** markdown import, seeding, ordered paths (`app/lamad/src/app/content-io/plugins/markdown/markdown-format.plugin.ts:66`, `genesis/seeder/src/seed-sqlite.ts:606`, `app/lamad/src/app/services/path.service.ts:694` ✅) | An editorial flow for long fiction ◐ |
| Browser sim and game | An **app served from peers** | **Code:** app packaging and serving (`elohim/sdk/scripts/package-app.mjs:133`, `elohim-storage/src/http.rs:11432` ✅) | An adapter for their build tool. Packaging is not yet runtime-verified (`package-app.mjs:125` ✅). Their persons, dwellings, projects, convoys and signed changes map onto records (`citizen_passport.js:239`, `node.js:15`, `:223`, `:293` ✅). |
| SIMONE's single-village docking test, Rainer Rodrigues's method | A model-validation measure | Our commons simulation calls itself an illustrative probe (`genesis/research/commons-flow-sim/flow.py:2` ✅) | — a method we can learn from |
| O-ASIS AI debates | A source of hypotheses | **Code:** story scenarios ✅ | A path from a hypothesis to a scenario that runs |
| The portal's AI Challenge Arena: cached Q&A, plus a hosted model labelled "Nomothetic Adjudicator" | An explaining surface over the charter, whose prompt and model would be a procedure record | **Spec:** AI may witness, explain and propose, but not author constitutional text alone or claim authority over legitimate disagreement (`protocol-specification.md:46`, `constitution.md:799` ✅) | Prompt and model fixed by fingerprint, and each answer recorded with the version that produced it. The hosted service's source is not public, so we have not read it. |

---

## The manifesto epics, and keeping reality honest

Elohim's canon tells its vision through story-level "epics". Each row gives the epic's own one-line
description and the ONE institutions that meet it. **Neither column means "built".**

| Epic, in its own words | Where ONE meets it |
|---|---|
| **Value Scanner**: "A Protocol for the Care Economy" (`value_scanner/epic.md:3`) | Care counts toward civic reciprocity; communal support for new parents (C:265, 278). Both treat care as productive work. |
| **Public Observer**: "When AI Becomes Your Town's Memory" (`public_observer/epic.md:1`) | Public physical accounts, plain-language explanations and rival expert testimony make decisions checkable by citizens (C:173, 214). |
| **Observer Protocol**: "A way of seeing that generates stories without storing surveillance" (`observer-protocol.md:22`) | ONE's safety sensing also limits retention (C:409). **A difference:** ONE forbids domestic profiling and speech recording by public sensors (C:160), while our observer imagines consenting household cameras and civic microphones, processed locally and then destroyed. |
| **Observability**: "Sensed, Never Gathered" (`observability/epic.md:26`) | Sensor readings checked by citizen squads and distant labs resemble its separation of self-report from corroborated witness (C:158, 161). The subjects differ: ecology there, network health here. |
| **Economic Coordination**: "When Value Flows Like Water" (`economic_coordination/epic.md:4`) | Physical-unit accounting, survival-first allocation and compacts between watersheds (C:134, 148, 518). Units and allocation rules differ. |
| **Governance**: "When Agents Serve Without Ruling" (`governance/epic.md:5`) | Lottery councils, restorative inquiry and red-teaming provide oversight and correction (C:207, 379, 420). |
| **Autonomous Entity**: "The Exit-to-Community Accelerator in Action" (`autonomous_entity/epic.md:4`) | Asset-locked trusts and reclaimed industry share the transition story (C:454, 460). **A difference:** ONE gives public automation no agency of its own (C:345), and this epic imagines an AI steward. |
| **Resilience**: "Mutual Aid as Substrate, Not Workaround" (`resilience/README.md:5`) | Mutual aid, 30-day islanding and manual fallbacks (C:289, 319, 454). |
| **Hardware Providence**: "The Rack as Proof of the Unenclosable Commons" (`hardware-providence-commons.md:25`) | Repair workshops, open machine controllers and energy priorities (C:298, 303). ONE extends the concern to dwellings. |
| **Social Medium**: "a medium where reach is earned" (`social_medium/epic.md:3`) | Tool libraries and restorative dialogue meet its neighbourhood stories (C:91, 422). |
| **Living Memory** (`living_memory/epic.md:3`), **Search** (`search/epic.md:9`) | No close counterpart found. |

ONE's novels pair most readily with Autonomous Entity (Vol. 1, the Detroit factory), Resilience (Vols. 2
and 4), Economic Coordination (Vol. 3, water) and Governance (Vols. 0 and 5). These pairings are inferred
from summaries, not from reading the novels.

**Keeping reality honest.** The operator's reading: the Value Scanner and the observers point toward
devices validating the physical world, and witness and reach are how Elohim keeps reality honest at
scale. The epics support this in part. Physical witnessing is explicit. The Value Scanner and Public
Observer are also, and first, stories about care made visible and civic memory, so we read them that way
too.

The thread across the epics ✅:
1. **Sensing becomes evidence.** The observer turns camera frames into structured stories locally and
   destroys the frames (`observer-protocol.md:26`). The observation design covers evidence about "someone
   else's action or the environment" (`architecture/2026-05-11-observation-event-layer-design.md:96`).
2. **Witness needs more than self-report.** Observability separates what a device reports about itself,
   what another participant witnesses, and corroborated findings (`observability/epic.md:80`).
   Independence across households and regions matters to corroboration (observation design `:239`).
3. **Witness also means seeing the person.** "El Roi", "the God who sees me", grounds concern for the
   powerless. Witness serves the survivor's own naming of what happened, not permission to act over them
   (`theology.md:189`).
4. **Reach carries correction outward.** Claims keep their origins, and corrections travel back along the
   paths a claim spread (`architecture/social-reach-nervous-system.md:36`). Reach scales accountability;
   it does not by itself establish physical truth.

**Beside this, ONE names physical instruments and institutions** ✅:
- public environmental sensors;
- open controllers with reproducible firmware;
- manual citizen checks;
- sealed duplicate samples;
- labs drawn by lot from distant watersheds (C:158–161).

Their register also notes that verified firmware can still drift or foul (`oasis/sim_resolutions_qa.md:122`).

**How the two could compose** ◐. ONE's samples and lab findings become observations. Elohim carries their
origin, corroboration, distribution and corrections, and ONE's assemblies receive them as evidence. The
physical work (sampling, calibration, laboratory independence) still has to be done; no record replaces
it.

**What both sides say is unfinished:**
- **Elohim:** Observability calls itself "a target and a proof ladder, not a claim that the target ships"
  (`observability/epic.md:634`). Hardware Providence lists physical observers as open work
  (`hardware-providence-commons.md:575`). The sensor-to-story step is outside the observation design's
  scope (observation design `:623`). Consent and control in intimate witnessing remain "open engineering
  problems, not solved ones" (`theology.md:173`).
- **ONE:** sensor verification is labelled as simulation-tested, with inspection cadence pending
  community review (`oasis/sim_resolutions_qa.md:117`, `:134`). Federation deployment tasks are unchecked
  (`roadmap/blueprints/STAGE_3_WATERSHED_FEDERATION_BLUEPRINT.md:99`).

---

## Where each stands on AI, and what keeps models honest

**The operator's hypothesis** (2026-10-24, in his words, offered as a hypothesis):
- **Models as trust artifacts.** "AI because of their immense training, are pre-baked trust artifacts..
  it's the model in their weight that substitutes the whole 'proof-of-work/proof-of-stake'."
- **Architecture as hearth.** "The architectural structure in which that artifact is deployed (our
  architecture) is the hearth to the AI 'fire'.. making the system itself a self-sustaining and resilient
  organism that CAN be aligned to the human flourishing objective."
- **What keeps it honest.** "It's the diversity of peers, and the cybernetic self-governance patterns,
  and the demographically representative human sortition-based check/audits, [that] become the
  foundations to keep models honest between each other.. so as to protect the closed system primitives
  from corruption."
- **The repository as evidence.** The fact that this repository exists is itself a proof artifact.

**What Elohim's canon says** ✅:
- **Pre-baked:** "An AI trained on the aggregated, witnessed record of humanity is a bottled reflection
  of our shared nature — drawn from a commons, and therefore owed to it" (`values-forward.md:117`).
- **The hearth:** "The values live in the text, not in the machine… Makers offer a capability; the
  network attests it. Communities write the values" (`constitution.md:20`).
- **Trust attested, not assumed:** each layer (base model, tuning, constitutional prompt) has its
  capabilities "attested… never taken on the word of whoever built it… A more capable model earns the
  same audit, never more deference" (`constitution.md:65`).
- **Kept honest by plurality:**
  - human councils drawn by lot stay sovereign over the machine (`values-forward.md:157–165`);
  - findings are corroborated across independent households and regions (observation design `:239`);
  - policies are plural and self-renewing, so no single one can ossify (plural-lenses spec, one-line
    summary);
  - an exceeded limit raises a notice, a signal the system feels (epr-rea `Bound`).
- **Standing:** AI is "a first-class *participant* in governance — never a tool, never a sovereign",
  phased in as it earns trust (`values-forward.md:167`).

**What ONE's text says** ✅:
- **Standing:** AI serves. Delegating "legislative drafting, judicial sentencing, biophysical resource
  rationing, or coercive physical restraint to… neural models, or artificial agents is an absolute
  constitutional nullity" (C:343). Public automation is limited to "verifiable oracle architectures devoid
  of subjective agency" (C:345).
- **Kept honest by plurality and audit:**
  - binaries reproduced bit for bit and checked by *diverse double compilation*, independent toolchains
    checking each other (C:356–361);
  - root keys split across "demographically stratified sortition assemblies across multiple watersheds"
    (C:363–369);
  - code repositories governed by lot-drawn assemblies, with standing red-team bounties (C:378–386);
  - any governance routine "proven… to resist functional comprehension by ordinary citizens" is void
    (C:371–376).
- **In practice:** AI is used widely as advisor and author: red, blue and purple AI roles stress-test the
  constitution; a hosted model explains it; local models narrate sensor data; the novels are AI-drafted.
  Kyberlex's own stated caution is against an "AI-hallucinated monster model" (#11).

**Where the two meet** ◐. Both keep machines honest the same way: *diversity checking diversity, under
lot-drawn human audit drawn to mirror the population.* ONE diversifies compilers and toolchains; the
operator's hypothesis diversifies models and peers. Both let AI carry the work of authoring, and both
refuse it final say.

**Where they differ** ◐:
- **Standing:** ONE's servant against Elohim's phased participant.
- **Where trust starts:** for ONE it is built by reproducibility and citizen comprehension; in the
  operator's hypothesis it is pre-baked by training, then attested and checked by plurality.

Read strictly, ONE's Art. 7.5 could bar a model whose weights no citizen can read from *deciding* on
shared infrastructure, while still letting it advise. A possible bridge: Elohim puts the governing part,
the constitution a model runs under, in plain text any citizen can read.

**The repository as proof** ◐. Both repositories are evidence of the same thing: a civilizational corpus
can be co-authored with AI at scale, so the authoring burden the conversation names can be carried.
Neither yet shows the organism sustaining real communities over time. That is what each project's own
checks (and this map's "neither has shown yet") are for. So the repository is the first proof of
viability, not the last.

**Two questions to share:**
- Could the logic of diverse double compilation (independent toolchains checking each other) extend to
  diverse models checking each other, under lot-drawn audit?
- Can a model be used under Art. 7.5 if the rules it runs under are plain text that citizens can read and
  amend?

---

## Story, attention and where a story lives

Both projects want their story to reach people without extraction. They take different roads, and each
road is coherent.

**ONE's road** ✅. Its first priority is a media co-steward to "take the helm of O.N.E.'s public presence":
YouTube, Reddit, X/Bluesky and newsletters, with a staged asset pack of the six-volume saga, 336 comic
slides, trailers and explainers (Discussion #13). The same call speaks to people "tired of corporate media
extraction". Using the existing channels to reach people while the commons is built matches ONE's own
dual-track logic: Track A's roads used to grow Track B.

**The operator's road** (2026-10-24, his words). "It's not that I don't believe in the attention economy..
I very much do. I just hold attention too sacred to feel any desire to use it." He would rather enable the
people who love to speak than be the voice himself. His answer to the media call, in substance: *thank you,
but no; keep your unique story, and we would be honoured to help you hold it, help maintain your work, and
give it somewhere to graduate to.*

**What Elohim's canon offers as the third road** ◐:
- **The story's home is the commons.** Content lives as records whose reach is earned and whose
  corrections travel back to every place it went. The Social Medium epic: "a medium where reach is
  earned", under the title "When Social Media Becomes Sacred Space". Values Forward III.1: "the system
  counts care, not engagement".
- **Other platforms receive honest pointers.** A post elsewhere is a boundary act, labelled as such, that
  points back to the real thing. This is the media version of how Elohim treats settlement outside the
  network.
- **Authorship stays with the author.** ONE's novels, charter and simulator would stay ONE's, held on
  shared ground, not absorbed into it.
- **Graduation is a stage, not a capture.** A story can move off rented platforms when its community is
  ready.

Elohim's own surfaces for this, its learning paths, short posts, community news and feeds built on
earned reach, are intended work, not yet built. That makes it a possible **second crossover**, after the
event-log experiment: two projects that both want dissemination without extraction, building the
commons-native way to do it.

---

## Sovereignty, translated

ONE uses "sovereign" in many senses. Our canon's line is "self-sovereignty is a floor of dignity, not a
ceiling of authority" (`values-forward.md:219` ✅). Identity rests on a dignity that communities and
institutions protect; holding a key does not create it. Canon also speaks of each layer being "sovereign
within its domain" (`governance-layers-architecture.md:82` ✅), meaning bounded jurisdiction.

| ONE says | We would read it as |
|---|---|
| "Invariant Personal and Bodily Sovereignty" (C:29) | The dignity floor, protected by community rather than self-asserted |
| "absolute, permanent sovereignty of the individual" over possessions (C:88) | What you make is yours (`values-forward.md:127`) |
| Assemblies' "sovereign… authority" (C:188, 215) | Ordinary people, drawn by lot, able to overturn |
| "Watershed as Primary Sovereign Unit" (C:502) | Bounded jurisdiction within a layer |
| "Victims' Sovereignty" (C:413) | Repair centred on the harmed |
| "Ownerless Sovereign Vault" (F:14) | A steward: caring for something without owning it |
| "Non-Sovereign Code" (C:341) | Shared wording: no machine holds authority a human council cannot overturn |

---

## The question of "one", held together

The operator raised this as the first real point to explore. Elohim leaves the top of its structure empty
on purpose:
- the commons is "owned by no one, including its authors" (`constitution.md:819` ✅);
- there is "no self-sovereign apex that answers to nothing" (`values-forward.md:179` ✅);
- "a community of any faith, or of none, can write its own layer" (`constitution.md:28` ✅).

The empty top is meant to let many ways of life live within one frame. The question for both projects is
the shape of the top, and how open a hand holds it.

**What ONE's text shows** ✅:
- **Openness at the top:**
  - Kyberlex gives up personal authority (CONTRIBUTING:49);
  - the founder's amendment power ends at the Genesis Lock (F:184–186);
  - the foundation may not interpret doctrine inside communities (F:592);
  - no assembly may "mandate orthodox belief" (C:51), and 7% of facilities is reserved for dissenting
    work (C:52);
  - higher tiers have "zero jurisdiction to override local autonomy" outside named exceptions (C:10.1.4);
  - anyone may leave (C:446).
- **A single horizon:**
  - the end-state is "Earth… as an integrated whole" (F:516), under one Universal Covenant (C:10.6.3);
  - the bill of rights is beyond any assembly (C:1.7);
  - the text names admission of new regions (C:10.6.2) but no way, which we found, for a community to
    choose a different charter.

**What we mean by an empty top, concretely:** no person, council or AI holds an authority that cannot be
challenged, and each community writes its own charter.

**What Elohim's own text shows, from the other side** ◐. It may also look full at the top from outside:
- a global layer no community may override (`constitution.md:164`);
- an AI ceiling meant to grow;
- a canon stewarded today by one operator, whose first global ratification is still open
  (`constitution.md:908`).

Both projects keep protections nobody can vote away. Our reading is that ONE is one constitution holding
many localities, and Elohim aims to be one frame holding many constitutions. That is our reading, not
something we have demonstrated, and neither shape is shown to be the more open.

**The symmetry.** In the operator's reading, both projects hold the commons with humility. Both share a
struggle: each must express its ideas through its own categories to say anything at all. Our *reach*,
*mishpat* and *elohim* are as particular as their *usufruct*, *demarchy* and *thermodynamic invariant*.
The empty top is Elohim's best attempt to keep a hand open to other cultures, and it is still an attempt
made from inside our own vocabulary. Held that way, this is not a test for ONE; it is a question both
sides can hold, and common ground is found there.

**Two questions, one in each direction:**
- *To Kyberlex:* could a watershed keep ONE's protections, adopt a different charter, and still belong to
  the federation?
- *To ourselves, equally concrete:* could a community decline Elohim's AI ceiling, or amend the global
  protections, and still take part? And what else in Elohim's frame looks full at the top from where ONE
  stands?

---

## What each could bring the other

**From Elohim to ONE** (each item's state is in the map above):
- a home for the charter as a layer;
- rosters, deliberation and challenge records;
- economic events and stocks in physical units;
- serving the sim from peers;
- learning paths for the novels.

**From ONE to Elohim:**
- a full written community charter, from which we can learn what a layered constitution has to hold;
- candidate procedures and measures, each one option among others:
  - council parity and term breaks;
  - thresholds with expiry;
  - the kinship rule;
  - the 7% dissent reserve;
  - rival expert panels;
  - split-sample ground-truthing;
  - publishing machine work;
  - the restorative ladder;
- Rainer Rodrigues's docking method for checking models;
- six volumes of story, and the physical-layer thinking (energy, wear, legal asset-locks) Kyberlex named
  as ONE's focus.

**What both would need**, which more than any mechanism is the collaboration surface:
- fixing procedures as records;
- running them reproducibly, and witnessing the results;
- holding several side by side;
- counting against an eligible population.

## What neither project has shown yet

- a lottery anyone can verify;
- ratification recorded on a charter;
- attested physical measurement;
- governance that works offline;
- any of these institutions serving real people over time.

These are open questions for both, not verdicts on either.

---

## Appendix: notes for the Elohim team

**Proposed backlog rows (the mint pass; awaiting the operator).** Each is stated as ground that holds
plural procedures, not as one mechanism:
- **commons-holonic-stewardship-backlog**, new row: *hold any lottery procedure as a fixed record*. It
  runs reproducibly over a roster snapshot, with a witnessed proof plus seating and term records, and
  several procedures can be bound side by side. Precondition under rows 33/34.
- **governance-ratification-single-write-capturable** (existing entry), add an observation: `check_quorum`
  in `elohim-storage/src/tally/mod.rs:121–125` ignores the eligible population.
- **commons-holonic-stewardship-backlog**, new row: *measures whose checking step can name a drawn,
  independent checker* (ONE C:161 is one instance).
- **commons-holonic-stewardship-backlog**, new row: *stewardship grants over resources* (today devices
  only).
- **epr-rea**, candidate: *a physical-resource event export* readable by an external simulator (the
  crossover Kyberlex proposed).
- **Engagement (operator's call):** a letter to Kyberlex. "Mint pass" is this directory's step of folding a
  survey's proposals into backlog rows. It needs the cross-repo-peer blind read required
  for letters in this directory.

**The operator's aspiration, in his words** (2026-10-24). It is kept here, in team notes, because the
body may be shared with Kyberlex. In the operator's usage, "subsume" means becoming the ground others
choose because it serves them: "attractive, not restrictive", the same word he uses for the legacy money
system ("subsume it competitively, yet peacefully").

> "I'm approaching this from the foundational compute, rust, p2p, brit, rakia, lvi.. ultimately.. our own
> honest evolution is that the foundations for the Elohim Protocol's honest evolution needs to arrive at
> the same time... We're approaching the possibility of the projects computational enclosure with
> discipline. Whereas I think (just him doing most of this in Typescript) is taking the primitives for
> granted. I think the thrust of what we've been working on for several months are trying to build ..as
> close to the physical silicon as possible, the architecture FOR the self-sustaining social coding
> commons, and organs, that will make any subsequent project or story (such as Kyberlex's) born honest,
> governable, evolvable, and un-enclosable (limited only by it's own reach)... I think our discipline is
> why we will subsume the other projects.. and be the candidate for a true social-coding commons to emerge
> for all other applications."

**How the pass ran.** The operator shaped the frame in steps:
1. a translation map, not a comparison;
2. collaboration, with Elohim as enabler;
3. money is the same destination by different instruments (his reading);
4. ONE's ETI matches our boundary-settlement posture;
5. hold no mechanism as authoritative, and keep humility in our own voice;
6. procedures are records;
7. the question of "one", held symmetrically;
8. plain language first;
9. a map onto the manifesto epics;
10. the record and its economic layer (EPR and epr-rea), as discussed with Kyberlex.

A first draft ranked mechanisms and graded ONE's claims; it was rewritten under step 5.

Sources read:
- the ONE repository at `b359ce9`, with full history, never executed (`research-manifest.json`:
  `kyberlex-one`);
- the portal repository, text only (`one-commons-portal`);
- ten GitHub Discussions;
- five Internet Archive items, including podcast transcripts;
- a Julia Discourse post with no replies;
- the surge.sh mirrors;
- the chat exchange the operator supplied.

The open web turned up no third-party discussion.

The work ran as read-only Codex passes:
- ONE institutions;
- ONE in code;
- Elohim in code;
- Elohim canon;
- Kyberlex's lineage;
- external voices;
- the manifesto epics;
- a posture and citation review;
- a blind read.

Their receipts are in `genesis/local-dev/research-kyberlex-one/` (gitignored), with external copies under
`external/`. The working grades in those receipts ("TENSION", "ABSENT") are not carried into this map.

## Credit

Kyberlex, for a corpus generous enough to translate and for the conversation; Rainer Rodrigues, whose
docking proposal gave SIMONE its first test. The operator, for the frame and its corrections. Codex, for
the reading passes. Claude Opus 5.5, for synthesis.
