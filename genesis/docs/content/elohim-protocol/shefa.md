---
title: Shefa — Sacred Economic Infrastructure for Human Flourishing
id: shefa-economic-infrastructure
tier: vision
status: vision
created: ~2025 (pre-implementation whitepaper)
class: protocol-canonical
pillar: shefa
provenance: relocated 2026-06-26 from genesis/docs/Shefa_Economic_Infrastructure_Whitepaper.md; body revised 2026-09 for coherence with the later economic canon (succession.md, the value_scanner epic, the shefa domain guide)
cites:
  - shefa-domain-gospel | CLAUDE | sha256:ca13d4bc043c03cb | path: elohim/sdk/domains/shefa/CLAUDE.md
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md
---

# Shefa: Sacred Economic Infrastructure for Human Flourishing

### How to read this paper

This paper was drafted in 2025, before the protocol was implemented, and has since been revised to agree with positions the protocol has settled; the repository [README](../../../../README.md) says which parts of it run. The protocol has no coin, no chain of its own and no proof of stake: shared commitments are notarized on the Holochain DHT, and money outside the network is reached through a settlement bridge (§3.2, §3.5). The "tokens" in these pages are readings over one record of economic events, not a currency ([The Tokens Come Last](value_scanner/epic.md#the-tokens-come-last) in the Value Scanner epic; the [shefa domain guide](../../../../elohim/sdk/domains/shefa/CLAUDE.md)). Commons value is held in trust by a council the community seats at arm's length, and the protocol's AI agents advise it without holding any of it (§2.2; [Succession Without Conquest](succession.md)). Identity is witnessed socially and with consent, without biometric or location surveillance (§5). Limits are a dignity floor beneath and a limit on accumulation above, both declared in advance, with friction rising toward the limit and no confiscation (§4; [Mishpat](glossary.md), the protocol's justice principle).

## Abstract

This whitepaper introduces Shefa (שֶׁפַע, Hebrew: "abundance, overflow, flow"), the economic pillar of the Elohim Protocol, designed so that value flows to contribution rather than extraction. It builds on REA (Resource-Event-Agent) accounting and gives identity, community, learning and work an economic footing: the protocol's [`imagodei`](imagodei.md), `qahal`, [`lamad`](lamad.md) and `avodah` pillars.

Value is attributed to the people who create it through the REA events that record their work, and is held in trust for them until they claim it with an identity the people who know them have attested. A council the community seats holds that trust, with the community's elohim, the protocol's AI agents, as witnesses and counsel. The aim is divine fairness in economic coordination: wealth cannot accumulate without prerequisite responsibility, and the network's value accrues to those who create it rather than those who control the infrastructure.

## 1. Introduction

### 1.1 The Problem of Extraction

Current economic systems have a capture problem: network effects accrue to whoever controls the infrastructure, not to the people who create the value. Facebook captures the value of relationships, and Uber the value of coordinating drivers and riders. Even cooperatives struggle, because administrative friction makes fair distribution harder than extraction.

The result is that care work, creation and contribution go unseen. A nurse's midnight compassion, a parent's developmental attention, a repairer's environmental stewardship: each creates real value, and that value is carried away from the people who created it toward whoever sits at the point of extraction.

### 1.2 Etymology and Theological Foundation

Shefa (שֶׁפַע) carries particular weight in Jewish mystical tradition. In Kabbalah, shefa represents divine abundance flowing down through creation, filling vessels prepared to receive. Current extractive systems *block* shefa: they capture and hoard rather than letting value flow to where it's needed.

The Elohim Protocol's economic layer doesn't create abundance; it removes the dams that prevent natural flow.

## 2. Core Principles

### 2.1 Value-Aware Accounting

Traditional currency is value-blind: a dollar from exploitation equals a dollar from care. A price records that something changed hands and forgets how it got there, so extraction and contribution look the same on the books.

Shefa keeps its accounts in REA form instead, an accounting model that records economic life as agents taking part in events that use and produce resources. Each act is written down as an economic event: what was used, what was produced, who did it, for whom, and who saw it. The record carries its origin with it, so extraction is visible and accountable. The people who saw the act sign attestations to it (§5.1), and their signatures make it a witnessed event rather than a self-report ([Succession Without Conquest](succession.md), Orientation).

A **token reading** is a view taken over that record along one dimension: care given, hours contributed, skills learned. Whoever reads the record computes the reading; it is not stored in the record, and it cannot be spent. A household or a community decides which readings to take (§7), and what can be spent is a balance in a unit of exchange a community declares for itself (§7.2). In the shefa domain guide's words, Shefa is value accounting, not a currency system.

### 2.2 Stewardship Over Ownership

Shefa encodes a fundamental shift from ownership to stewardship. Value is attributed to the people who created it and held in trust until they claim it, within limits negotiated and declared in advance (§4). Accumulation requires demonstrated capacity for responsibility, and no agent, human or AI, can extract more than their attributed share.

A council holds that value in trust for one community. In this protocol a council is the deliberative body that decides: people the community seats, with the community's elohim as witnesses and counsel ([Succession Without Conquest](succession.md), Orientation). It sits at arm's length from what it holds, owning none of it and having nothing to extract with ([Values Forward](values-forward.md), Stance I.4), and it cannot deliberate its way beneath the dignity floor or past the other limits in §4. How a community seats its commons council is one of the open questions at the end of this paper.

An elohim (lowercase; the plural is the same word) is one of the protocol's AI agents, running locally on participants' own hardware and acting for a person and their community under the constitution's values ([glossary](glossary.md)). The elohim keep the accounts, help people gather attestations and give counsel. None of the value is theirs to hold, and the protocol's human councils can audit, correct or shut off any of them (Stance II.2).

Stewardship is a relation a person earns through sustained care of a resource or a body of content. Standing, a person's relational track record as the network has witnessed it, decays when it goes untended, so no one becomes a dormant landlord.

### 2.3 Beer's Cybernetic Governance

Stafford Beer's Viable System Model holds that an organization governs itself well through recursive feedback rather than central control: each level must have enough regulatory variety to match the complexity it faces (Ashby's Law of Requisite Variety). Beer tested the idea in [Project Cybersyn](https://en.wikipedia.org/wiki/Project_Cybersyn), a real-time economic coordination system with factory-level participation, designed for Salvador Allende's Chile. It ran for two years and was destroyed in the 1973 coup. Beer's own reading was that it fell because it depended on a single government and a single building; the [Economic Coordination epic](economic_coordination/epic.md) tells that story at length.

Shefa keeps Beer's principle and drops the dependency. Value the old system hides is recorded where the people it concerns can see it, and recognition of contribution is the feedback signal. The bounds in §4 exist to make sure it reaches the people who create value, so that the network can regulate itself in the way Beer described in *[Designing Freedom](https://en.wikipedia.org/wiki/Designing_Freedom)* (1973). Because the record lives on the devices of the people who keep it, there is no central computer to be destroyed in a coup and no single government to capture. Worker participation is the foundation: the people who do the work record it and are the first to see it.

## 3. Technical Architecture

### 3.1 The Layered Stack

| Layer | Carried by | What it does | Scope |
|-------|------------|--------------|-------|
| Witness | The people present, or the [Observer](observer-protocol.md), a consent-bound witnessing protocol (below) | Sees an act, with consent, and turns it into an REA event the witnesses attest | The device |
| Ledger | REA events on Holochain, in the ValueFlows vocabulary | Agent-centric accounting: each person's own signed record, with shared commitments notarized on the DHT | The network, with no global consensus |
| Settlement | A settlement bridge to external rails | Exchanges value held in the network for money outside it (§3.5) | The boundary with the outside economy |
| Stewardship | A council the community seats, advised by its elohim | Holds network value in trust for attributed contributors (§2.2) | The community |

ValueFlows is an open vocabulary for REA. hREA, its Holochain implementation, is a protocol this one interoperates with through its ValueFlows bridge.

The Observer is a protocol, not a product: a camera or microphone in a home or workplace, with a physical switch beside it that the people there can turn off, and rules for what may be kept. It streams only to a node they control, where an elohim turns what it sees into a structured REA event, and it preserves visual data only when specific harm patterns are detected ([The Elohim Observer](observer-protocol.md)). Its adoption path starts with the kitchen: one camera, one switch.

### 3.2 What Stays With the Person and What Is Shared

Most of economic life needs no one else's agreement to be recorded. A person's care for their household, their learning, their contributions to their community and their own data are kept on their own source chain (the signed, append-only record each participant holds of their own actions), validated by their network and shown to others as they choose.

Some things have to be shared: network value held in trust, commitments between people, and attestations of identity. These are notarized on the Holochain DHT, an agent-centric distributed hash table with no global consensus. The peers who receive an entry check it against the rules the application declares and refuse it if it does not hold ([Succession Without Conquest](succession.md), Orientation). Exchange with money outside the network goes through a settlement bridge (§3.5).

### 3.3 The Flow Mechanics

```
A person acts
     ↓
Witnessed, with consent (by the people present, or by the Observer)
     ↓
REA event recorded on the person's own source chain,
with the witnesses' signed attestations
     ↓
Pattern recognition (the community's elohim read the record)
     ↓
Value attributed to the contributors who created it
     ↓
Held in trust by the community's council
     ↓
Claimable by attributed contributors
(subject to attested identity and the bounds in §4)
```

Where what is held in trust is money from outside the network, it sits in accounts held on outside payment rails by a legal person, such as a cooperative or a fiscal host, because the protocol itself owns and holds nothing ([Succession Without Conquest](succession.md), §7.1).

### 3.4 An Example: One Care Event

This example comes from the [Economic Coordination epic](economic_coordination/epic.md), which follows the Okonkwo family in Lagos. Adaeze Okonkwo is a nurse on the night shift. In the small hours she spends 47 minutes bringing a critical patient to a stable condition while two junior nurses watch and learn. Adaeze records the act on her own source chain, and the two nurses sign attestations that they saw it. The record looks like this (identifiers are illustrative):

```json
{
  "event": "care_event_example",
  "recorded_by": "adaeze_okonkwo",
  "resources_consumed": {
    "agent_time": "47_minutes",
    "skill_application": "emergency_triage_advanced",
    "emotional_labor": "high_intensity"
  },
  "resources_created": {
    "patient_stability": "critical_to_stable",
    "family_peace": "anxiety_to_hope",
    "junior_learning": "technique_witnessed"
  },
  "agents": {
    "primary": "adaeze_okonkwo",
    "supported": "patient_placeholder",
    "witnessed": ["nurse_fatima", "nurse_blessing"],
    "benefited": ["family_member_1", "family_member_2"]
  },
  "attestations": [
    { "by": "nurse_fatima", "claim": "emergency_stabilization_witnessed" },
    { "by": "nurse_blessing", "claim": "emergency_stabilization_witnessed" }
  ]
}
```

Nothing in the record is a price. Whoever reads it computes a reading, so the same event can carry different ones: her community's care reading might count it as 23, while a reader costing it in naira might put it at ₦78,000. [Succession Without Conquest](succession.md) (§9.3) states the rule as "record facts, defer valuation," and the weighting that turns 47 minutes of emergency care into a care reading of 23 is the community's to declare.

The patient, critically ill, could not consent to the record and appears here only as a placeholder. Succession Without Conquest names the hazard in that: a record of care that the person cared for cannot contest is "a conferred identity with a signature on it" (§5.1).

### 3.5 What Adaeze Receives

The event is credited to Adaeze: it adds to her witnessed record and her standing, supports a skill attestation in emergency triage, and records what the junior nurses learned as her teaching. The protocol calls that credit recognition, and the value it attributes to her accumulates in trust until she claims it (§5).

The record also lets her show an employer work her payslip never recorded. In the epic, that visibility wins her a raise, and later she joins a community health cooperative that pays her a share of its income. If her community runs a unit of exchange of its own, such as a mutual-credit circle (§7.2), she can trade in it with the others who accept it. Where such a unit meets money outside the network, such as naira in a bank account, the exchange goes through a settlement bridge: a connection between value held in the network and money on outside payment rails.

## 4. Bounds on Value

Shefa is designed to hold value flows within constitutional constraints, which take their authority from the protocol's [constitution](constitution.md): the plain-language charter of values every elohim reads and is bound by. Succession Without Conquest holds such rules at two levels, which it calls the floor and the ceiling. The floor is mechanical: it runs with no AI and no network, it is the same for everyone, and it cannot be argued with. The ceiling reads a situation in context, and there the community's council, advised by its elohim, decides. The rule that keeps them apart is "never a computed payout at the ceiling, never judgment at the floor" ([Succession Without Conquest](succession.md), Orientation). Both bounds in this section, the dignity floor beneath and the limit on accumulation above, are declared in advance and belong to the mechanical floor.

### 4.1 Four Priorities, in Order

Four priorities govern how value flows.

#### First: Existential Minimums (the Dignity Floor)

Basic needs come first. The dignity floor is the minimum of provision and dignity below which no one may fall. Provision flows first to those below it, care labor always receives recognition, and what the floor protects cannot be extracted under any circumstances. The protocol calls the standard it holds itself to here self-sealing: the promise of a floor is kept only when the provision actually arrives ([glossary](glossary.md)).

#### Second: Contribution Recognition (Attribution)

Value flows to the people who created it, and derivative work shares recognition with its sources. When a contributor is offline or has not yet joined, what is attributed to them waits in trust. A work whose author has not joined is represented by a creator presence, established and stewarded by the elohim until it is claimed, which a community can vouch for ([manifesto](manifesto.md), Part IV-B). What it holds passes only to someone who can show, through the attestation in §5, that they are that author.

#### Third: Community Circulation

Balances held for exchange are meant to move, and commons pools serve the community's needs before any one person's surplus. Demurrage on hoarding returns idle value to the commons (constraint 3, below). Beyond the floor, accumulation meets friction that rises as accumulation rises, up to a declared limit. What a person holds is never seized; where value has to return to the commons, it returns on a negotiated schedule the person can read before they begin. This is how Mishpat treats limits: as declared bounds with a graduated consequence ([Values Forward](values-forward.md), Stances I.4 and II.1).

#### Fourth: Network Development

Part of the commons goes to helping the next community set itself up and join. Work that maintains shared infrastructure and keeps the network coordinated is credited like any other contribution, and a community may fund work on the protocol itself from its commons pool.

### 4.2 The Five Constraints

The design encodes five inviolable constraints, and they are the spine of Shefa.

1. **No accumulation without responsibility.** Claiming network value requires demonstrating stewardship capacity: a witnessed record of care for something over time, such as resources tended, shared work kept going, or content curated after mastering it to the level of applying what one knows.
2. **Graduated claiming.** Small claims are easy; large claims require more attestation.
3. **Demurrage on hoarding.** Value that sits uncirculated decays back to the commons, while value in active use is renewed. §7.1 says which readings it applies to and why some do not decay.
4. **Attribution persistence.** Contributions are remembered even when the contributor is offline, and what is attributed to them is held in trust until they claim it.
5. **Anti-capture.** No agent, human or AI, can extract more than their attributed share. Council members are bound by this like anyone else.

## 5. Identity Attestation

### 5.1 Why Claiming Needs Attested Identity

Value held in trust for someone is only safe if the person who claims it is that someone. Without a way to tell, one person could pose as a thousand contributors (a Sybil attack) or claim another person's share, and the system would collapse into fraud; attested identity closes the economic loop. Shefa answers the question the way people always have, by asking the people who know you. An attestation is a signed witness claim by one person about another. Its history is append-only, so an attester who changes their mind issues a new attestation that supersedes the old one. Claiming needs two kinds: attestations of identity, which say who you are, and attestations of skill and stewardship, which show the capacity constraint 1 asks for.

### 5.2 Who Witnesses, and What

Identity in Shefa is witnessed socially, and only with the person's consent. Three kinds of witness count. People who have met you attest that you are a real person in their lives, not an account in a bot farm. The people you live and work with attest to who you are. Your contributions, witnessed by others over months and years, show that you are the same person who made them. Each attestation carries the attester's own standing, so if one is shown to be false, the consequence falls on the attester. Under Mishpat that consequence is limited to their participation and does not extend to their body: it narrows their reach (how far their contributions travel).

None of this is a biometric scan or a location trace. You choose who may attest for you and what each attestation may reveal. The community's elohim keep the record and help you gather it, but identity rests on the word of the people who know you.

### 5.3 The Claiming Flow

A contributor who wants to claim value attributed to them brings the attestations they choose to share: people who have known them for years, and the witnessed record of their work. The claim is checked first for identity, then against the responsibility threshold: stewardship capacity in proportion to the claim's size (§4.2). Only then is it paid out from what is held in trust, up to the amount attributed to the claimant. The community's elohim gather the attestations and report what they show; they do not decide. No identity score is involved. Standing is a relational shape, never a single number.

### 5.4 What This Raises the Cost Of

Social attestation raises the cost of each of these attacks and ties every false word to someone's standing.

- Sybil attacks: to pass as a thousand people, one person needs a thousand histories of people who know them. A ring of invented identities that vouch only for one another has no one outside the ring to vouch for it.
- Identity theft: claiming another person's share means persuading that person's relationships to attest falsely, and each of them stakes their own standing on it.
- Ghost attribution: a claim made for a contributor who does not exist still needs a person the community has witnessed.
- Capture through scale: a billionaire's shell identities each need the same history of people who know them as anyone else's would, so wealth alone does not multiply a voice.

## 6. Integration Architecture

### 6.1 Relationship to the Other Pillars

Shefa is the economic substrate the other pillars share. The shefa domain guide declares how each one couples to it.

| Pillar | What it brings to Shefa |
|--------|-------------------------|
| [`imagodei`](imagodei.md) (identity) | Identity anchors attribution: value is credited to a person, and claiming it requires attested identity. A person gains economic agency once their identity is established in the network. |
| `qahal` (community and governance) | The community's trust networks attest to identity and govern collective stewardship pools. Stewards who have earned it gain governance standing here. |
| [`lamad`](lamad.md) (learning) | Learning generates value. Mastery at the level of applying what one knows makes a person eligible to steward content, and skill attestations open new ways to contribute. |
| `avodah` (work) | Work is where learning becomes action. The conversation between lamad (wisdom) and avodah (action) is where Shefa value is created. |

The [Observer](observer-protocol.md) is not a pillar. It is one of the witnesses that turn acts into the REA events Shefa reads (§3.1).

### 6.2 The Unified Graph

These pillars share one relational graph: the same people, relationships and records, read for different purposes. Identity reaches each of the others as witnessed, attested identity, and each of them feeds the record Shefa reads.

```
                      imagodei
                    (Who you are)
                          │
           identity witnessed and attested
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               │               ▼
        qahal             │             lamad ◄────► avodah
      (Who you            │         (What you're    (What you
       are with)          │           becoming)        do)
          │               │               │
          └───────────────┼───────────────┘
                          │
                          ▼
                        shefa
                  (How value flows)
```

## 7. Token Readings

### 7.1 Many Readings of One Record

Shefa reads the one record along several dimensions, each with its own meaning. Six examples follow. A household, a school, a library or a garden can declare a reading of its own over the same record, and it will interoperate with the others because the record is shared. The last two columns describe what a community might do if it declared a unit of exchange based on that reading.

| Reading | Taken from | A balance in it might pay for | Demurrage (illustrative) |
|---------|------------|-------------------------------|--------------------------|
| Care | Witnessed caregiving | Services, goods | Medium |
| Time | Hours contributed to the community | Coordination, services | Low |
| Learning | Skills developed and taught | Education, mentorship | None |
| Steward | Environmental and resource protection | Sustainable goods, restoration | Low |
| Creator | Work that helps others | The right to build on a work | None |
| Infrastructure | Network maintenance | Protocol services | High |

Demurrage (constraint 3) falls on balances and leaves the record of who did what untouched. It returns a balance left idle to the commons while one that keeps moving is renewed. The [Value Scanner epic](value_scanner/epic.md) traces the idea to Silvio Gesell's stamped money. Learning and Creator carry none, because what they read is attribution itself, a skill someone developed and taught or a work others build on, and attribution persists (constraint 4). The rates are illustrative: each community declares its own and says why.

### 7.2 Exchange

Exchange within the network is recorded as REA events: a transfer from one person to another for goods or services, a voluntary return to a commons pool, or a claim on what is held in trust (§5.3). Exchange across community boundaries is recorded the same way and is visible to both communities.

What moves depends on what a community has declared. A community that wants a unit of exchange declares it as a setting over the same record: a mutual-credit circle, in which members extend credit to one another directly. As Succession Without Conquest puts it (§9.3), the protocol supplies the grammar and the community supplies the configuration. The protocol counts the issuance of money among the common inheritance, the value no individual made, and holds it behind preconditions; until they are met, the protocol mints nothing ([Values Forward](values-forward.md), Stance I.4; [The Tokens Come Last](value_scanner/epic.md#the-tokens-come-last)).

## 8. Network Effects That Serve Participants

Platforms capture network effects for their shareholders. Shefa is designed to turn the same effect toward the people who produce it. More participants make more value visible as REA events. More visible value means more contribution credited to the people who did the work. The shared record also does the bookkeeping whose cost made fair distribution harder than extraction (§1.1), so a cooperative or collective can share its income according to what each member did, and a workable alternative to extraction draws more participants. If the loop holds, care becomes economically visible, creators receive the value held for them, and communities coordinate without central control.

## 9. Boundaries and Requirements

For developers: Shefa owns little of its own. The core types for economic life (economic events, agreements, commitments and resources) belong to the protocol as a whole. Shefa declares what each economic act signals, what metadata describes stewardship and exchange, what the protocol should watch for in a community's economic health, and how each act couples to the other pillars; its one record type of its own is the stewardship context attached to a resource. The vocabulary, schemas and coupling rules are specified in the [shefa domain guide](../../../../elohim/sdk/domains/shefa/CLAUDE.md), the developer notes kept with the code.

## 10. Security Considerations

### 10.1 Attack Vectors and Defenses

| Attack | Defense |
|--------|---------|
| Sybil (fake identities) | Social attestation: each identity needs its own history of people who know it (§5.4) |
| Value inflation | Constitutional generation limits |
| Extraction capture | No one can extract more than their attributed share (§4.2, constraint 5) |
| Collusion | Transparent flows enable detection. Selective disclosure lets witnesses and auditors see what a check needs without everyone's details being published, so detecting collusion does not require surveillance (§10.2) |
| State coercion | The record lives on participants' own devices, with no central store to seize. Money held in trust sits in accounts a legal person holds on outside rails (§3.3); a seizure reaches that legal person's accounts, not the record or any other community's holdings |

### 10.2 Privacy Preservation

REA events are designed to be encrypted and disclosed selectively. A person decides who may see which details of their record, and can show a witness or an auditor what a check needs and no more. The community sees aggregate patterns without seeing whose they were, which the Value Scanner epic calls community patterns without surveillance. The Observer is designed to process what it sees locally and to preserve visual data only when specific harm patterns are detected ([The Elohim Observer](observer-protocol.md)). Each person stewards their own record and chooses what to disclose from it.

## 11. One Person's Path

Follow Adaeze from her first night on the network. She passes through two of the protocol's four stages of participation ([Progressive Stewardship](../../../../README.md#progressive-stewardship)), and moving up keeps her identity, record and standing.

At first, her work becomes visible. She joins as a hosted participant: a doorway, one of many replaceable web entrances to the network, holds her keys and runs her instance of the protocol for her. She begins recording what she does, sees the REA record of her care for the first time, and finds the other nurses nearby who are doing the same.

Visibility becomes recognition. Witnessed nights turn into skill attestations, her teaching is credited to her, and she begins to exchange within the network. By now she is an app steward, running the software on her own device and holding her own keys.

Recognition becomes something she can rely on. She joins a community health cooperative, part of her income arrives through it and through her community's own unit of exchange, and she depends less on a hospital whose administrators and suppliers, she says in the epic, take most of what her care produces. How far and how fast that goes is for her and her community to decide.

## 12. Conclusion

The old economy is blind—it sees transactions, not transformation; counts money, not meaning; measures GDP, not flourishing. Shefa sees the nurse's midnight compassion, the repairer's environmental stewardship, the student's peer tutoring, the coder's open-source contribution, the parent's invisible labor, the elder's accumulated wisdom.

The network holds your value in trust. When you show up—embodied, witnessed, attested—you can claim what's yours.

### Open questions

- Who seats the commons council, and how, is not settled.
- How a large claim is checked is not specified.
- The unit a reading is measured in is left to each community and not yet named.
- Who operates a settlement bridge is not decided.
- The preconditions for issuance are not stated.
