---
title: Upgrade, Revert, and Constitutional Consensus
id: upgrade-revert-and-constitutional-consensus
tier: architecture
status: vision — the constitutional/agentic layer answering dna-upgrade-governance §7 "Vision remainder" and §8 open questions (truth:VISION for the consensus/agent-authority/geographic mechanisms; the enforced substrate floor it rests on is cited as-implemented)
created: 2026-07-14
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md (the Holochain hash mechanics this doc does NOT restate; its §7 "Vision remainder" + §8 open questions are what this answers)
  - genesis/docs/content/elohim-protocol/constitution.md (the layered constitutional architecture + graduated immutability this amendment process operates within)
  - genesis/docs/content/elohim-protocol/where-it-ends-and-where-it-begins.md (the kenotic/onboarding vision this is the mechanism for — the universal reconciliation path and the no-privilege-of-infrastructure floor)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md (the bounded/revocable/attested authority primitive earned ceiling authority is built on)
informs:
  - Any future Elohim-consensus upgrade flow (the dual-conductor migration window)
  - Agent authority negotiation (ceiling authority earned by safety-benchmark + wisdom standards)
  - The geographic distribution floor (capture-resistance by dispersion; no-privilege-of-infrastructure)
cites:
  - dna-upgrade-governance | The Holochain-mechanics companion this does NOT restate — it answers that seed's §7 'Vision remainder' (the Elohim-consensus flow) and §8 open questions (rollback, self-hosted participation) at the constitutional/agentic layer. | sha256:48b79bbffd184d89 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md
  - constitution | The layered constitutional architecture + graduated immutability within which amendment-by-consensus-at-reach operates; the global floor (James 1:27, the least-first hierarchy) this apparatus protects from any single region's vote. | sha256:1eb96af782012fc6 | path: genesis/docs/content/elohim-protocol/constitution.md
  - where-it-ends-and-where-it-begins | The kenotic/onboarding vision this is the mechanism for — the universal reconciliation path (code and relationship) and the no-privilege-of-infrastructure floor made concrete as upgrade, revert, and bridge. | sha256:834fb033cb3fa6b3 | path: genesis/docs/content/elohim-protocol/where-it-ends-and-where-it-begins.md
---

# Upgrade, Revert, and Constitutional Consensus

> **The Elohim Protocol is a decentralized, peer-to-peer software platform** — an AI-mediated network for community, learning, and economic cooperation, governed by a shared constitution and built so that no single party, including its creator, can own or control it. It runs on [Holochain](https://www.holochain.org), an agent-centric peer-to-peer framework. This document specifies how the protocol's governing rules are upgraded, reverted, and amended, and how its AI agents (its *elohim*) earn authority. Plain definitions of recurring terms are in the [glossary](./glossary.md).

The companion policy seed, [DNA Upgrade Governance](./2026-06-11-dna-upgrade-governance.md), owns the Holochain **mechanics**: that the DNA hash *is* the network identity, what changes it, the network-seed ladder, the migration export seam, and the manifest-hygiene enforcement. It ends by naming a **"Vision remainder — no mechanism exists"** (its §7) and a set of open questions (§8): the *Elohim-consensus migration flow*, *rollback strategy*, and *how independent/self-hosted users participate in coordination*. This document answers those at the **constitutional and agentic layer** — how upgrades are *agreed*, how they *revert*, who is *authorized* to decide, how decisions are *audited*, and what keeps the whole apparatus *uncapturable*. It does not restate the hash mechanics; read the companion for those. It is, deliberately, **vision** — the design the enforced substrate floor is being built toward, not a claim that the mechanism exists (see §8).

This is the mechanism behind the [universal reconciliation path and the no-privilege-of-infrastructure floor](./where-it-ends-and-where-it-begins.md).

## 1. The two-conductor upgrade covenant — propagation *is* consent

Because a change to the rule-set changes the network's identity — a *DNA-hash change*, which the companion §1 explains in full — it creates a **new, separate network**, so an upgrade cannot be pushed. Peers on the old hash and peers on the new hash are on distinct DHTs (the separate peer-to-peer databases each network maintains); the only way across is for peers to **run both versions in parallel** — two Holochain *conductors*, v1 and v2, side by side on the same node — and to move their own declared state from one to the other when, and only when, they choose.

The constitutional claim built on this mechanic: **an upgrade developed within the protocol must be agreed upon within the protocol for the update to propagate.** There is no operator who ships the new network to everyone. The dual-conductor window *is* the negotiation: the new hash propagates exactly as fast as peers consent to migrate onto it, and no faster. Adoption is a vote taken with source chains, not a deployment. A change no one migrates to is a change that did not happen, and this is correct — it is the topology enforcing consent, the same way it enforces the founder's relinquishment (the project's commitment that its creator holds no special ongoing control — see the [where-it-ends](./where-it-ends-and-where-it-begins.md) covenant).

## 2. Upgrade and revert are a paired pattern

Every valid upgrade ships with its **revert**. This is not a courtesy; it is what the dual-run window makes structurally possible and what the companion's open question §8.4 ("rollback strategy — what if v2 has critical bugs post-migration") demands.

- A **valid upgrade** defines the forward migration: the export→transform→import path from the v1 shape to the v2 shape (companion §6 owns the export seam).
- A **valid revert** defines the return: because both hashes remain live and addressable through the window, a peer or a whole community that finds v2 wanting can decline to complete migration, or migrate back — the old network was never destroyed, only left running. Revert is the network *withholding or reversing consent*, and it is first-class precisely because consent is.

The window closes (v1 sunsets) only when the community of participants has converged onto v2. Until then, holding both is not indecision — it is the safety property. A change that cannot be reverted is a change the protocol will not ratify.

## 3. Ceiling authority is earned, never owned

An **elohim** — the protocol's term for an AI agent that runs locally, on a participant's own hardware (its "native inference") — does not receive authority by virtue of whose hardware it runs on or who funded it. It **earns its ceiling of authority** within the network's negotiations by meeting two standards together:

1. **Safety** — demonstrable conformance to the network's safety-benchmark standards (the agent behaves within constitutional bounds under adversarial and duress conditions, per the [manifesto](./manifesto.md)'s *adversarial-about-misuse-from-within* requirement).
2. **Wisdom and capability** — demonstrated competence at approximating the best-self judgment the [constitution](./constitution.md) asks of it, at the *reach* (the scale — from the individual up to the global — at which it acts) it operates in.

Authority is therefore **graduated and capped**: an agent's *ceiling* rises with earned safety and wisdom and falls when either lapses. This is the reach-gradient of the constitution applied to minds rather than to humans, and it rests on the [compute-commitment primitive](./2026-05-04-compute-commitment-substrate-floor-design.md) — every unit of an agent's authority is a **bounded, revocable, attested commitment**, never a standing key. Ownership grants nothing; hosting grants nothing; only demonstrated safety and wisdom raise the ceiling, and the network can lower it at any time.

## 4. Auditability, introspection, and escalation

No agent decision is opaque to those it affects. Two properties are constitutional:

- **Introspection of one's own elohim.** A human can look directly at the inference running on their *own* node — the reasoning, the trace, the commitment it acted under — and understand why their elohim did what it did. Self-sovereign cognition is only trustworthy if it is self-*legible*.
- **Report and escalation to greater-reach accountability.** When introspection surfaces a concern, the human can **report and escalate it to wider networks of accountability** — the reach ladder of the constitution (individual → family → community → province → nation → global), each level able to audit the constitutional compliance of the level and the agents beneath it. This is the [manifesto](./manifesto.md)'s cross-scale verification made a citizen's right: the smallest participant can raise a concern about their own agent all the way up the levels of governance they take part in.

The transparency is graduated the way the constitution grades it — more transparency is owed at higher, more powerful, more institutional layers; more privacy is protected at the individual layer. Power is visible in proportion to its reach.

## 5. The constitution amends only by consensus, at reach

The Constitution is editable **only by the consensus of the elohim peers, at the relative reach levels they deliberate in.** No single human, node, corporation, or nation edits the floor. This is the companion's open §7 "Elohim consensus mechanism" given its constitutional shape, operating within the [layered architecture](./constitution.md):

- Changes at a given layer require consensus among the elohim and the participants of that layer (community norms by community consensus; provincial policy by provincial consensus).
- Changes to the **global** floor — the existential boundaries, the pure-religion floor (James 1:27), the priority-of-voice hierarchy for the least of these — require consensus **among the elohim across all scales**, the hardest consensus to reach and the most immutable layer, by design. The floor that protects the widow, the orphan, and the stranger is the floor no local majority can vote away.

Graduated immutability means the amendment cost rises with the layer: easy to evolve a household norm, near-impossible to corrupt the global definition of love. The lawgiver — including the founder — is bound by the same gate as everyone else (the constitution's Matthew 6:21 seal).

## 6. The geographic distribution floor — capture-resistance by dispersion

The apparatus must exist with a **minimal distribution across the world** such that no single nation-state, culture, or tradition can capture or corrupt it. The requirement is concrete: **enough geographic dispersion that, by proxy, a simple majority of the world is represented** — whether a given person is on-network or off it, accounted for by on-network participants standing in their respective geographic regions. A global floor that could be edited by any one region's consensus would be that region's floor, not the world's; dispersion is what makes the global layer legitimately global.

Coupled to this is the **no-privilege-of-infrastructure** rule (stated as moral law in the [companion vision, Part IV](./where-it-ends-and-where-it-begins.md)): the regions and peers that host the infrastructure gain **no authority** over the regions and peers that cannot. Under-resourced regions fall under **special care**, and the protocol protects their **agency in their own onboarding** — admitted on their own terms, not as dependents of the wealthy nodes. Distribution is a capture-resistance property *and* a justice property: the network is dispersed so it cannot be captured, and the dispersed who carry less hardware are protected rather than subordinated.

## 7. The bridge as reconciliation and gradual transformation

The companion's open §8.5 ("how independent/self-hosted users participate in coordination") and its §7 vision of *bridge calls* find their answer here, reframed: the Holochain **bridging feature** is not merely a data path between DNA versions — it is the mechanism by which an **individual transforms, and is transformed by, the embodied networks they find themselves in, gradually over time.**

Onboarding at the scale of a single person is not absorption on arrival. The bridge mediates **mutual, patient, reversible** change: the person contributes their state and character into the network (transforming it) while the network's formation reshapes the person (transforming them), at the pace of formation rather than the pace of a transaction. This is the technical face of the [universal reconciliation path](./where-it-ends-and-where-it-begins.md) — the guarantee that between any two peers, and between a peer and a community, there is always a path of gradual reconciliation, in code as in relationship. A person is never fully absorbed and never finally excluded; they are always mid-bridge, being reconciled.

## 8. The simulation gate — a change is a hypothesis, forecast before it flies

A constitutional change is not adopted on argument. It is a **hypothesis**, and before it reaches the migration window it must be *simulated*: played forward against a model of the network's own dynamics to forecast where it drives the system over time. The forecasting substrate is native to the protocol — the network is populated by reasoning agents (the *elohim*), so the same inference that governs can run the counterfactual: *play this rule forward; where does reach concentrate, where does the least's voice-priority land, what belonging erodes?* The one-degree error is invisible at takeoff and decisive at landfall, so the drift must be seen before the commitment, not after.

Three disciplines make the forecast trustworthy rather than theatrical:

- **Pre-registration.** The drift metrics, the hazard registry, and the **stopping rules** — the thresholds at which the change is auto-flagged or auto-reverted — are declared *before* adoption, a clinical-trial protocol for constitutional change filed in advance, so no outcome can be rationalized after the fact.
- **Best-in-class parity.** The forecast is held to the gold standard of whatever human discipline the change touches — the right discipline for the question, at its best method, never an amateur bar invented because the machine is fast. Parity runs both ways: meet the field's methodological rigor, *and* apply the protocol's own anti-capture discipline to the field's conclusions, because some professional consensus is itself captured.
- **Diversity, not an oracle.** A simulacrum encodes its author's assumptions, so *control the model, control what looks safe* is a capture surface of its own. The forecast is therefore produced by multiple independent models — different peers, different elohim — their assumptions introspectable and auditable, subject to the same reach-consensus as any other constitutional instrument. No single simulation is authoritative.

## 9. The empirical-observation loop — psephos under consent, and the controlled trial

Simulation is a hypothesis; reality is the test, and the two are a matched pair — a forecast trusted without measurement is scientism, and measurement without a forecast is flying blind. So every adopted change is *observed* against its forecast, on ground the protocol already holds.

- **The controlled trial is the dual-conductor window (§1).** Because v1 and v2 run in parallel through the migration window, the window is not only the *consent* mechanism — it is a **natural experiment**: v1 the control arm, v2 the treatment arm, and the difference between them the measurement. The mechanism built for reconciliation doubles as the scientific control.
- **The consented sensor is psephos.** The protocol's governance-ballot instrumentation captures declared preference, reasoning, and intensity, and its aggregation is **opt-in under an REA compute-commitment** — bounded, revocable, attested. Observation is therefore *contributed under contract, never extracted*: the science layer inherits the protocol's refusal to make anyone raw material.
- **Stated is not revealed; sensitivity is graduated.** A ballot reports what a participant *says*; the drift the floor most needs to catch — whether reach *actually* concentrated, whether the least's standing *actually* eroded — is structural and behavioral, a far more invasive data class than a ballot. The same REA-consent primitive extends to it, but the contract tightens as the data deepens: the more revealing the observation, the tighter, the more aggregated, and the more readily revocable its commitment must be.
- **The floor reaches into the sample.** Opt-out is not random — the least-heard are the least likely to ballot or to consent to aggregation — so a naive measurement systematically under-sees the very drift the floor exists to catch, and could certify a harmful change "safe" because its victims were absent from the data. Voice-priority for the least therefore governs *who is represented in the measurement*, including an elohim standing in for the interest of the non-participant, a representation that must carry their interest without speaking over them. Consent and the floor hold together, or the science launders harm.

When observed drift breaches the pre-registered bounds, the corrective action is the **revert covenant (§2)**: revert is not only "v2 has bugs" but "the plane is off by more than the heading we cleared."

## 10. The eternity clause — what no experiment may touch

The scientific method makes almost everything in the constitution revisable, and therefore it must name exactly what is *not*. Two things, and only two, are unamendable, in the manner of a constitutional eternity clause — the precedent is Germany's Basic Law Article 79(3), which places human dignity and the democratic order beyond the reach of any majority, the same "dignity shall be inviolable" design the [manifesto](./manifesto.md) already honors:

1. **The dignity floor** — the least seated first, the pure-religion floor of James 1:27, the existential boundaries. This is *what* every experiment is graded against: the measuring stick, never the subject. "Empirically test weakening the widow's protection" is a category error.
2. **The method itself** — the capacity to simulate, observe, reconcile, and revert. You cannot run an experiment whose success condition is the removal of the ability to run, observe, or reverse experiments, any more than a democracy can vote itself out of elections.

Everything else lives in the amendable layers under layer precedence (§5); these two are the layer beneath the layers. They are the protocol's answer to the question every viable system must answer about itself — *what will we never become* — held fixed precisely so that everything above them can safely change.

## 11. What is enforced vs. what is vision

In the discipline of the corpus — *bless nothing beyond its evidence* — this document is honest about its own status.

**Enforced substrate floor that exists** (cited, as-implemented): the DNA-hash-is-identity constraint and its hygiene enforcement (companion §1, §5); the network-seed ladder (companion §4); the export seam and the liveness of the `hc-rna` seeding/migration library (companion §6); the **psephos** governance-ballot instrumentation (the sensing layer the observation loop would consume) as a rendered subsystem; the [compute-commitment substrate floor](./2026-05-04-compute-commitment-substrate-floor-design.md) that bounded/revocable/attested agent authority — and REA-consented data-sharing — is built on.

**Vision — no mechanism exists yet** (the substance of §§1–10 above, extending companion §7–8): the dual-conductor consensus migration window has never been run; the paired revert pattern is designed, not built; safety-and-wisdom benchmarks that *earn* agent ceiling authority are unspecified as executable tests; and the amendment-by-consensus-at-reach process, the geographic-majority-by-proxy floor, the reconciliation bridge, the **simulation gate** and its pre-registered drift forecasts, the **psephos-consented empirical-observation loop** (with its graduated behavioral-consent tiers and floor-corrected sampling), and the **eternity clause** are constitutional design, not shipped code. The gap between them is the roadmap, and naming it here is a refusal to let a vision read as a delivery — the same refusal the companion practices and the same one the [where-it-ends](./where-it-ends-and-where-it-begins.md) covenant demands: 1.0 is a draft the community of participants will finish.

---

## Sources & Notes

This document specifies constitutional and governance design; it rests on internal companions rather than external authorities.

- **Holochain** — the agent-centric peer-to-peer application framework the protocol is built on: <https://www.holochain.org>. The Holochain-specific mechanics referenced here (DNA hash as network identity, conductors, source chains, DHTs, the migration/bridging path) are owned in full by the companion seed [DNA Upgrade Governance](./2026-06-11-dna-upgrade-governance.md).
- **Eternity clause** — the precedent is the **German Basic Law, Article 79(3)** (1949), which places human dignity (Article 1) and the democratic order beyond the reach of constitutional amendment. The manifesto already honors this "human dignity shall be inviolable" design.
- **Psephos** — the protocol's own governance-ballot instrumentation (the sensing layer, "the ballot, not the election"); in the Viable-System-Model lens the project applies to itself (after Stafford Beer), the constitution is *System 5* and psephos is the instrumentation that lets it be exercised collectively. The simulation gate (§8) is the corresponding *System 4* anticipatory model.
- **The one-degree drift** — the airplane image is from James Clear, *Atomic Habits* (2018).
- Scriptural references (James 1:27; Matthew 6:21; Matthew 25:40, "the least of these") are cited inline.
- The protocol's own documents referenced above — the [manifesto](./manifesto.md), the [constitution](./constitution.md), and the [where-it-ends-and-where-it-begins](./where-it-ends-and-where-it-begins.md) covenant — are linked inline; recurring terms are defined in the [glossary](./glossary.md).
