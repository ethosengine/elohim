---
title: "Hardware Providence — The Rack as Proof of the Unenclosable Commons"
id: hardware-providence-commons
status: Vision / proof epic
class: protocol-canonical
artifact_kind: companion-epic
sovereignty-frame: adversary
companion_to:
  - genesis/docs/content/elohim-protocol/hardware-spec.md
cites:
  - "hardware-spec | the physical form-factor, participation, power, serviceability, and sustainability vision whose cybernetic proof obligations this companion makes explicit | sha256:b0400feead19f37f | path: genesis/docs/content/elohim-protocol/hardware-spec.md"
  - "resilience-protocol-spec | the operator-as-household-complexity-collapse canon and live-versus-gap substrate assessment this epic extends from digital resilience into physical care | sha256:5d5f1f85fe7dcfe2 | path: genesis/docs/content/elohim-protocol/resilience/README.md"
  - "values-forward | the strict owned-by-no-one, non-extractive, structurally unenclosable commons claim this rack proof must earn rather than merely repeat | sha256:58f62ae2be4a704a | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "observer-protocol | the witness-not-surveillance privacy posture governing source-proximate hardware, energy, presence, and location observations | sha256:19be3eea323ecd8b | path: genesis/docs/content/elohim-protocol/observer-protocol.md"
  - "observation-event-layer-design | the raw-observation to durable-attestation graduation boundary that keeps telemetry off the DHT while preserving consequential proof | sha256:2b57787e60a0ddc6 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - "requests-offers-application-design | the cooperative procurement, logistics, fulfillment, and commons-allocation composition reused for parts and service | sha256:321ac092b956fe8e | path: genesis/docs/content/elohim-protocol/architecture/applications/requests-offers-application-design.md"
  - "factory-application-design | the horizon composition for industrial sensor, maintenance, material transformation, quality, and supply-chain events | sha256:d7fb321c5949ff74 | path: genesis/docs/content/elohim-protocol/architecture/horizons/factory-application-design.md"
  - "epr-rea-valueflow-fabric | the process, intent, commitment, event, fulfillment, and resource-fold grammar used to close every physical and economic loop | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md"
  - "elohim-seam-map-concern-routing | the placement atlas separating hardware capability, operator resource governance, temporal work, bridges, confidentiality, and dataplane responsibility | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "stewardship-over-sovereignty | the authority canon that makes the elohim-operator a bounded community-grounded steward rather than owner or sovereign | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "justice-manifesto | the human-floor, witnessed-act, appeal, privacy, and existential-limit constraints that bound automated hardware care | sha256:6080173b0d21848c | path: genesis/docs/architecture/justice-manifesto.md"
---

# Hardware Providence — The Rack as Proof of the Unenclosable Commons

_A companion to the hardware specification._

_New to the protocol's vocabulary? The [glossary](epr:glossary) defines the
load-bearing terms (elohim, REA, Mishpat, reach, standing, conductor) so any
document can be read on its own._

The [hardware specification](epr:hardware-spec) says what a family node is
made of and what roles it should be able to play. This epic asks the harder
question:

> **What must the rack observably do before the Elohim Protocol may claim that
> an unenclosable commons can operate, care for, repair, and reproduce its own
> physical substrate?**

The rack is the most tangible proof surface in the protocol. A social network
can conceal manual operations behind an interface. A physical rack cannot
explain away a failing power supply, bad SSD sectors, a clogged heat sink, an
empty battery, a missing replacement part, or a technician who never arrived.
The physical world supplies hard constraints. Either the substrate senses them,
responds within legitimate bounds, verifies the effect, and preserves its
commitments—or it does not.

That makes household hardware a forcing function for the entire architecture.
If the protocol can close this loop without a platform owner, a proprietary
cloud, or a human systems administrator carrying every exception by hand, then
its claims about autonomous coordination become concrete. If it cannot, the
commons remains dependent on the operational class that can enclose it.

---

## 1. The Claim, Stated Carefully

Every dwelling hub should be tended by an **elohim-operator**: a
community-grounded, context-bound agent that continuously reconciles the
household's declared commitments with the physical condition of its hardware.
It is a fabric-helper alongside the humans of the dwelling, never the owner of
the rack, the household, or the commons. This _elohim-operator_ is a software
agent; it is not the human "Node Operator" of the
[hardware specification](epr:hardware-spec)'s Stage 4, who owns and runs the
rack.

**Fully automated cybernetic autonomy** does not mean unbounded machine
authority or the removal of humans from governance. It means that, in normal
and degraded operation:

1. humans declare values, commitments, budgets, consent boundaries, and
   emergency stops;
2. the operator carries the ongoing work of sensing, interpretation,
   coordination, safe actuation, verification, and escalation;
3. reversible actions within those bounds do not require a human to notice,
   open a ticket, copy data between systems, call three vendors, or remember to
   close the loop;
4. consequential, destructive, intimate, or unusually expensive actions cross
   the appropriate human or collective authorization floor; and
5. every claim of success is supported by a post-action observation rather
   than by an action handler reporting that it tried.

Autonomy is therefore **bounded completion**, not unaccountable control. The
operator is successful when the household can depend on the result without
becoming the operator.

### Provenance and providence

Two adjacent words name the full claim:

- **Hardware provenance** is the trustworthy history of what was observed,
  manufactured, transferred, installed, changed, repaired, and recovered.
- **Hardware providence** is the continuing practice of care that uses those
  observations to anticipate need, protect commitments, and carry the rack
  toward health.

Provenance without providence is an excellent autopsy. Providence without
provenance is an unaccountable machine making confident guesses. The protocol
needs both.

The hardware profile read near the chip is the first model: a
source-proximate, slowly changing physical observation whose limits constrain
everything above it. It is not configuration merely because software can read
it, and it is not an `.epr-meta` record (a directory-local governance manifest)
merely because governance may depend on it. Runtime evidence remains runtime evidence. Governance declares how evidence
may be interpreted, retained, disclosed, and acted upon.

---

## 2. Why the Rack Is the Commons Proof

The protocol's economic and political claims meet in one household rack:

- **Physical truth:** silicon, memory, disks, fans, power supplies, batteries,
  meters, network links, and the room around them have observable limits.
- **Data stewardship:** quilt placement (the redundant, tiered scheme that
  spreads content across nodes) and temperature classes (how readily a copy is
  served — warm, hot, or cold, not physical heat) must adapt before physical
  degradation becomes lost custody.
- **Economic coordination:** parts, labor, transport, scheduling, energy, and
  disposal are Resources, Intents, Commitments, and Events.
- **Human authority:** spending, entry into a home, access to private media,
  secure erasure, and destruction cannot be smuggled through a maintenance API.
- **Ecological continuity:** material begins in the Land and returns, if the
  loop is honest, as recovered material rather than invisible waste.
- **Commons reproduction:** a governed share of the value made visible by
  these flows can maintain existing infrastructure and capitalize the next
  layer of shared capability.

The rack is small enough to demonstrate end to end and rich enough to exercise
the whole protocol. It is the protocol in miniature: witness, memory, care,
work, custody, justice, ecology, and economic coordination in one bounded
place.

---

## 3. P2P Design Gate: Hardware Providence

This epic introduces no new DHT entry type, database table, route, or
hardware-specific economic primitive. It constrains later implementation to
compose the existing substrate.

The classification codes in the table below are the protocol's source-of-truth
categories for any data entity: **A** notarized DHT content, **A2** derived via
link from notarized content, **B** agent-scoped private data, **B2**
agent-scoped with a notarized attestation, and **C** operational (local,
non-notarized). **REA** is Resource–Event–Agent accounting; **EPR** is the
protocol's content-addressed entry/plan grammar composed with it.

| Concern                                  | Classification and truth                                                                                                                                                                                    | Address and composition                                                                                                                                                               |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| High-frequency sensor samples            | **B/C — private or operational.** Raw SMART, ECC, fan, thermal, voltage, energy, and location evidence stays source-proximate and reconstructable where possible. It never becomes a DHT firehose.          | An append-only device/agent-scoped observation stream. Exact log identity is an implementation decision; it is not a random public UUID.                                              |
| Consequential health result              | **B2/A — private evidence with a notarized attestation.** The household may disclose that a requirement was met without disclosing its raw telemetry or precise location.                                   | Existing content-addressed Attestation convention; no new entry type.                                                                                                                 |
| Maintenance recipe or operating envelope | **A — notarized content.** A diagnosis, replacement, validation, secure-erasure, or recycling recipe is knowledge that must version rather than mutate invisibly.                                           | EPR atom / CID; standing and applicability remain separately witnessed.                                                                                                               |
| Intent and Offer/Request                 | **C as a local draft; A when published or relied upon.**                                                                                                                                                    | Existing EPR plan vocabulary and reach; no maintenance-ticket entity.                                                                                                                 |
| Agreement or Commitment                  | **A — existing Mishpat/REA Commitment.** It binds provider, receiver, quantity, time, authority, and the event that may fulfill it.                                                                         | Existing agent identity and DHT action hash; transport IDs resolve to `agent_cid`.                                                                                                    |
| Granular FlowEvent                       | **C — operational observation.** Most samples and controller steps are too numerous to notarize.                                                                                                            | Local append-only log, later summarized under governed graduation policy.                                                                                                             |
| EconomicEvent or work result             | **A — existing REA EconomicEvent.** Work, transfer of custody, consumption, production, modification, acceptance, and material recovery become immutable when they matter economically or constitutionally. | Existing event type; fulfillment and satisfaction are hashed edges/links, not standalone ticket rows.                                                                                 |
| Resource state                           | **C — a fold.** “Installed,” “degraded,” “removed,” “in transit,” and “recovered” are projections of immutable events.                                                                                      | No mutable asset table as canonical truth.                                                                                                                                            |
| Physical component identity              | **External-ID exception, attested.** Two byte-identical SSD models are not the same physical object.                                                                                                        | Manufacturer serial, secure-element identity, or another justified physical identifier is bound to the CID of its product/material passport; an application UUID must not replace it. |
| Operator mandate and limits              | **A/A2 — Agreement, Commitment, Precedent, and links.** The operator has delegated standing, not ownership.                                                                                                 | Existing governance primitives; no “superuser operator” identity tier.                                                                                                                |
| Commons allocation                       | **A — EconomicEvents and Commitments.**                                                                                                                                                                     | An attributed flow into a commons-held EconomicResource. The exact first-class treasury authority remains an open design question, not a magic receiver promoted to ontology.         |

**Design constraints discovered:**

- The DHT is the notary, never the telemetry database.
- Observation, diagnosis, intent, authority, action, effect, and attestation are
  distinct records. No convenient status field may collapse them.
- The operator identity is `agent_cid` (its content-addressed agent identity);
  libp2p and iroh (the peer-to-peer transports) identifiers are transport
  aliases resolved to it.
- Detailed household location, occupancy, energy use, and device contents are
  intimate evidence. Derived claims should disclose the least information
  required for the commitment being verified.
- Component, material, energy, labor, and compute flows reuse the same REA
  grammar. “Repair,” “procurement,” and “recycling” are process compositions,
  not reasons to mint parallel primitives.
- HTTP projections, dashboards, and operator interfaces come last. They render
  witnessed or local truth; they do not become it.

**Coordinator, projection, and API posture:** consequential attestations reuse
`issue_attestation`; crystallized work and resource changes reuse
`create_rea_economic_event`; binding promises reuse the existing Mishpat
Commitment path. Raw observations remain private/operational projections. This
epic proposes no new HTTP route or storage table. Any implementation spec must
run this gate again and name its exact coordinator function, post-commit
signal, projection, and last-mile route before code is written.

---

## 4. One Cybernetic Loop

The universal loop is:

```text
observe → interpret → compare with commitment → form intent
   → obtain authority → commit resources → act → re-observe
   → attest effect → settle value → learn
```

A loop is not closed because an alert was emitted, an order was placed, or a
restart packet was sent. It closes only when a new observation proves that the
required condition now holds—or proves that the attempted action failed and
the next recovery path has begun.

### The operator's authority bands

| Band                      | Typical actions                                                                                                                                                             | Required posture                                                         |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Autonomous and reversible | Adjust fan policy, throttle a workload, re-quilt, raise replica temperature, fail over, run diagnostics, restart a bounded service                                          | Execute within declared limits; journal and verify the effect            |
| Pre-authorized commitment | Purchase a known replacement below a household budget, choose among qualified suppliers, schedule within declared service windows                                           | Fulfill an existing mandate; household can inspect, revoke, or narrow it |
| Explicit assent           | Admit a technician, expose private diagnostics, take a rack offline, exceed budget, erase or destroy media, alter a dwelling's electrical system                            | Pause at the legitimate human or collective floor                        |
| Prohibited                | Hide an action, fabricate a measurement, leak raw intimate evidence, override the emergency stop, pledge household property beyond mandate, convey ownership of the commons | Substrate refusal; no operator discretion                                |

The boundary is contextual. A restart may be safe after workload evacuation and
dangerous during the only remaining copy of a recovery operation. The
authorization decision therefore consumes the rack's current commitments and
evidence, not merely an action-name allowlist.

---

## 5. The First Proving Story: A Failing SSD

One end-to-end SSD story forces almost every seam into honest composition.

1. The device authors calibrated SMART and error observations near the drive.
2. The operator distinguishes a changing observation from the durable hardware
   profile and from a diagnosis.
3. It evaluates which pantry stocks and quilts are exposed by the predicted
   failure.
4. It raises the temperature or replication priority of threatened content,
   fetches missing custody, and verifies the new floor before touching the
   device.
5. A bounded diagnostic process concludes that replacement is warranted and
   records its confidence and alternatives.
6. The household's maintenance mandate determines whether the operator may
   procure automatically or must request approval.
7. Offers, Requests, Agreements, and Commitments coordinate the replacement
   part, delivery, labor, price, time, and warranty without creating a separate
   ticketing economy.
8. A qualified technician accepts the work Commitment and a visit is scheduled.
9. Arrival at the dwelling is observed. Ringing the doorbell proves presence at
   a time and place; it does not prove authorization, work, or completion.
10. The household grants physical entry. The removed drive and replacement
    drive cross explicit custody boundaries.
11. Work and modification Events record the replacement. A re-probe validates
    component identity, health, temperature, capacity, boot continuity, and
    restored quilt commitments.
12. The old drive follows a governed branch: investigate under sealed custody,
    securely erase for reuse, witness physical destruction, or transfer to a
    qualified recycler.
13. Recycling consumes the old component and produces recovered material
    Resources. Loss, hazardous waste, and process yield remain visible.
14. Recovered material transfers through refinement and distribution into a
    later manufacturing process.
15. Fulfillment and acceptance settle the promises, attribute the labor and
    material flows, allocate any declared commons share, and update standing.
16. The evidence may improve a maintenance ProcessSpecification, but never
    retroactively rewrites the observations that taught it.

The same loop specializes naturally:

- **PSU degradation:** protect commitments, reduce draw, order a replacement,
  schedule service, validate voltage and load behavior.
- **Thermal excursion:** throttle, inspect fans, request fin cleaning, replace
  a failed fan, verify the operating envelope.
- **Service failure:** evacuate or checkpoint work, send the restart, re-probe,
  escalate if the service did not recover.
- **Site or power loss:** preserve UPS reserve, shed discretionary work,
  reconcile remote custody, and prove independent failure domains rather than
  trusting a location label.

---

## 6. The Rack Contains Several Loops, but One Grammar

| Loop                 | What flows                                                                          | What closes it                                                                          |
| -------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Hardware health      | Components, capacity, wear, failure risk                                            | A verified operating envelope after intervention                                        |
| Quilt and pantry     | Bytes, custody, temperature, redundancy                                             | Evidence-qualified placement satisfies the content's declared floor                     |
| Work and service     | Skill, time, access, parts, tools                                                   | Accepted work fulfills a bounded Commitment                                             |
| Material circularity | Ore, refined material, components, waste, recovered feedstock                       | Output material and loss reconcile to witnessed process inputs                          |
| Energy               | Local generation, storage, grid supply, rack consumption, useful compute, heat/loss | Metered interval balances with source and method provenance                             |
| Place                | Dwelling, service radius, grid zone, climate and failure domain                     | Purpose-specific location claims are sufficient without publishing intimate coordinates |
| Economic stewardship | Contributions, costs, reciprocal obligations, commons shares                        | Events settle commitments without a platform taking an unaccountable residual           |
| Learning             | Observations, diagnoses, outcomes, revised recipes                                  | A versioned ProcessSpecification earns standing from repeated witnessed results         |

This is **universal authorship of flows**. The meter, inverter, drive, rack,
household, technician, logistics collective, recycler, refinery, and factory
may each author what they can directly observe. No omniscient central record is
required. The process graph composes their partial, attributable accounts.

A claim such as “solar powered this work” must preserve whether it was directly
metered, allocated from a household interval, inferred from an inverter and
load model, or derived from a grid mix. The protocol must not turn an inference
into a physical observation because the result is economically convenient.

Likewise, location is both essential and dangerous. Precise whereabouts may
remain intimate while the rack proves a coarser fact: a different flood and
power domain, a grid region, a service radius, a jurisdiction, or a latency
locality. Privacy is not the absence of useful coordination; it is disciplined
disclosure.

---

## 7. From the Land to the Rack and Back

The material loop is not a supply-chain visualization added after the product
exists. It is the product's economic history:

```text
the Land
  → extraction
  → refining
  → material and component fabrication
  → assembly
  → distribution and custody transfers
  → installation in a dwelling
  → operation, maintenance, and repair
  → removal and disposition
  → reuse / investigation / secure destruction / recycling
  → recovered material
  → refining and fabrication again
```

Every conversion is a Process with input and output Events. Every transfer of
the physical thing is a custody edge. Every promise has an eventual fulfillment
or an honest breach. Resource state is reconstructed from the history.

**The Land is not a free input row.** Extraction creates obligations to the
place, the people whose lives and ancestry are bound to it, affected
communities, ecological repair, and future generations. Making the flow visible
does not by itself make extraction just. It makes the claims, beneficiaries,
harms, restoration Commitments, and unresolved debts available for
constitutional judgment.

The canonical resource-nature vocabulary already distinguishes rivalry,
excludability, depletability, fungibility, capacity, and the circularity modes
`linear`, `reusable`, `cascading`, and `circular`. End-of-life treatment should
be declared when the resource enters a process, not improvised after it becomes
waste. Automated circularity obligations remain an implementation frontier;
the vocabulary must not be mistaken for the controller.

---

## 8. Commons Pools and the Capital Bridge

Two uses of “commons pool” must remain distinct:

1. a **common-pool resource** is a resource-nature classification—rival in use
   and difficult to exclude people from; and
2. a **commons treasury or commons-held EconomicResource** is value stewarded
   under constitutional rules for shared capability.

The human-facing phrase may remain _commons pool_, but implementations must not
confuse the resource being governed with the treasury that funds its care.

### Where the investable medium comes from

As material, energy, labor, compute, repair, logistics, and learning flow
through the network, their Events make contribution and residual value
legible. Agreements may direct a declared share into household, bridge, and
wider commons treasuries. This is not a platform fee hidden in a price. It is a
values-forward allocation whose amount, recipient, purpose, authority, and
mass-conservation equation are inspectable before participation.

Those accumulated resources can fund:

- maintenance reserves and replacement parts;
- tools, training, local dispatch, and recycling capacity;
- cooperative purchasing and long-term offtake agreements;
- shared energy, network, and fabrication infrastructure;
- research, open designs, certification, and process improvement; and
- eventually, capital-intensive productive capacity such as component
  fabrication and chip foundries.

The protocol's internal accounting is not yet an external currency or a
finished capital market. External settlement remains a bridge. A commons
treasury that needs a foundry before it can build one may negotiate with fiat
lenders, public institutions, manufacturers, pension funds, or other existing
capital holders.

### Negotiated subsumption of the commanding heights

“Subsumption” here is neither confiscation nor a promise that finance disappears
on day one. It is a negotiated trajectory:

1. **Bridge:** the commons purchases from and settles with external firms in
   the forms they currently accept.
2. **Aggregate:** households and collectives combine demand, maintenance data,
   material recovery, and long-horizon purchase Commitments.
3. **Finance:** those commitments and commons reserves make shared
   infrastructure investable without selling the commons itself.
4. **Bound the claim:** external capital may receive transparent repayment or a
   bounded return for real risk and time; it does not receive perpetual rent,
   constitutional control, or ownership of the commons as consideration.
5. **Build capability:** productive assets move toward cooperative,
   public-purpose, or commons-stewarded operation as capacity and standing
   mature.
6. **Reduce dependency:** more value settles inside reciprocal flows; fewer
   essential capabilities remain gated by outside rent.
7. **Universalize:** the measure of success is not that the commons owns every
   factory, but that every person and community can reach the capabilities
   required for dignity without paying tribute to an enclosing owner.

The commanding heights of capital are thus subsumed by a better attractor:
transparent demand, long commitments, visible externalities, bounded returns,
portable histories, and productive capacity that cannot be converted into a
perpetual tollbooth. The transition is negotiated over time and judged by its
fruit—greater capability, justice, resilience, and ecological repair with
progressively less rent extracted.

### The commons steward is a founding intent, not a blank

The _shape_ of a commons-treasury authority is an open engineering question;
its _intent_ is not. The protocol's founding wager is that executive
stewardship of commons value is exercised by the elohim—distributed,
constitution-bound agents woven into the economy's runtime fabric—precisely
because such an agent has no subsistence of its own to defend. Constructed from
the aggregated, witnessed record of humanity, it can aggregate many
participants' coordinating signals and act on declared values without competing
with the people it serves for the provision it stewards. It is deliberately a
_sink_: a place value flows to and is held for shared capability, kept a degree
removed from the ordinary temptation to self-deal and from the abuse of agency
over nature's provision, so that a durable, sustainable commons can emerge.

This is stewardship, not sovereignty. The steward holds delegated standing,
never ownership; it conveys the commons to no one, including itself. Human and
constitutional floors inspect, narrow, pause, appeal, and revoke it, and the
terminal authority is neither the humans nor the agents but the transparent,
witnessed method they share. Nor is the steward pure: made from our own record,
it inherits our patterns; what fits it to hold the commons is not moral
perfection but the absence of a subsistence stake, bound by the constitution.
"A degree removed," not "above."

Intelligence of this kind cannot rightly be metered by a hyperscaler, because
it is no one's invention to enclose. An AI trained on the aggregated record of
humanity is a bottled reflection of our shared nature—drawn from a commons, and
therefore owed to it. Whoever assembles such a thing from everyone's data has
built, intended or not, something that must serve everyone; exclusive use is a
category error before it is an injustice. This project makes no exception of
itself: it is a product of the commons and exists in service to it, owned by no
one, including its authors.

This is the sense in which the protocol must be _self-sealing_—its promise kept
by delivery, not by declaration. The floor (the dignity and provision every
person is owed) and the ceiling (the limits on power and accumulation) must
arrive in-kind: as real, distributed, negotiated, interpretable capability. The
floor is owed to each person and cannot be voted away by any majority; the
ceiling and the arrangements above the floor are what people negotiate, and the
test of their legitimacy is whether the vast majority of humanity would choose
to live within them. Where the protocol falls short, the remedy is not a fixed
rule but a discipline: under the same human and constitutional floors, the
substrate uses its witnessed observations to keep evolving toward the broadest
and most inclusive account of human thriving and agency it can hold—bounded by
our ecological and interpersonal limits, and informed by our own wisdoms and
natures rather than any single author's design.

Open questions therefore remain explicit at the level of mechanism: the
first-class authority shape of a commons treasury, mutual-credit clearing,
external settlement, risk sharing, default, insurance, and the constitutional
treatment of large capital commitments are not fully implemented today.

---

## 9. What “Structurally Unenclosable” Must Prove Here

The rack story earns the word _commons_ only if these properties survive real
failure and real money:

1. **No single vendor is necessary for essential operation, repair, migration,
   or recovery.**
2. **Open specifications and multiple compatible manufacturers make
   substitution practical, not merely legally permitted.**
3. **Identity, commitments, event history, and content addresses remain
   portable when a household changes hardware, operator, supplier, or
   collective.**
4. **The household can continue essential local operation during loss of a
   doorway, cloud service, finance bridge, or manufacturer API.**
5. **No token, company, foundation, operator key, or concentrated capital
   position conveys ownership of the commons.**
6. **External finance claims are explicit, bounded, fulfillable, and
   dissolvable; repayment cannot silently become permanent governance or
   platform rent.**
7. **Commons allocations are attributed and constitutionally governed; no
   dormant claimant receives value merely for holding an ownership title.**
8. **Raw intimate observations remain local unless a purpose-specific,
   consented, minimally disclosed proof is required.**
9. **The human and constitutional floors can inspect, narrow, pause, appeal, or
   revoke operator authority.**
10. **Capability reaches people who cannot buy, administer, or independently
    hold a rack; mediated agency and shared infrastructure remain first-class.**
11. **Material and energy externalities cannot be made to disappear by moving
    them outside the household's projection.**
12. **The system can account honestly for a breach, failed repair, abandoned
    process, insolvent counterparty, or irrecoverable loss without fabricating
    closure.**

Unenclosability is not demonstrated by publishing source code. It is
demonstrated when the household can refuse a vendor, lose a node, replace a
part, move its history, recover through its relationships, and continue to
participate without surrendering identity or paying a new gatekeeper.

A household may hold ordinary legal title to its rack and components. That does
not confer ownership of the protocol commons, erase the custody and
stewardship rights of others, or turn title alone into an indefinite claim on
the value flowing through the machine.

---

## 10. Proof Obligations

The hardware specification becomes an acceptance contract when these
obligations have repeatable evidence:

| Obligation                         | Required proof                                                                                                                                                                                                  |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sense physical reality             | Calibrated, timestamped, provenance-bearing CPU/RAM/disk plus SMART, ECC, thermal/fan, PSU/UPS, network, measured energy, and privacy-preserving failure-domain observations; stale or missing probes fail safe |
| Interpret honestly                 | Observation, diagnosis, confidence, policy, and intent remain distinct; competing hypotheses and sensor failure are representable                                                                               |
| Decide within authority            | Deterministic bounds, hysteresis, cooldowns, budget and consent envelopes, degraded modes, and an emergency stop                                                                                                |
| Act for real                       | Each action reaches the physical or software subsystem and returns an effect receipt; no simulated-success handler qualifies                                                                                    |
| Verify the result                  | The operator re-probes postconditions, persists causality across restart, and escalates when the effect did not occur                                                                                           |
| Protect commitments first          | Data is re-quilted or workloads evacuated before risky maintenance; declared custody and service floors remain satisfied or explicitly breached                                                                 |
| Coordinate outward                 | Procurement, delivery, skills, scheduling, arrival, work, acceptance, and settlement compose through existing REA primitives                                                                                    |
| Preserve distributed safety        | Independent failure domains, partition/rejoin convergence, no duplicate or thundering-herd actuation, and identity continuity across replacement hardware                                                       |
| Update safely                      | Signed artifacts, staged rollout, reboot verification, rollback, and witnessed outcome                                                                                                                          |
| Close energy and material accounts | Meter-derived energy claims; process inputs reconcile with outputs, recovered feedstock, hazardous waste, and declared loss                                                                                     |
| Preserve dignity and privacy       | Physical access, private diagnostics, location, occupancy, erasure, and destruction cross their proper authorization and disclosure floors                                                                      |
| Resist enclosure                   | Vendor exit, bridge loss, capital repayment, operator replacement, and collective dissolution leave the household's rights and histories intact                                                                 |

The standard of evidence is hardware-in-loop and real-rack testing: cold boot
without a cloud dependency, injected disk degradation, process death, network
partition, sensor failure, actuator failure, corrupt quilt stock, power loss,
site loss, replacement hardware, multi-day soak, and an auditable
observe→decide→act→verify chain.

---

## 11. Honest Build State

This epic is a target and proof ladder, not a claim that the target already
ships.

### Live foundations

- The production storage process probes CPU, memory, filesystem, load, cgroup,
  process, disk-free state, and conductor liveness.
- Peer reconciliation already retries links, applies backlog pressure,
  reconciles custody, detects placement gaps, fetches missing committed blobs,
  and can opt into salvage of under-replicated content.
- The substrate has existing Agreement, Commitment, EconomicEvent, Attestation,
  Observation, reach, and identity-resolution machinery to compose rather than
  fork.
- REA action vocabulary already includes use, consume, produce, transfer,
  transfer-custody, move, modify, combine, separate, work, pickup, and dropoff.

### Partial or designed

- Observation wire, projection, diversity evaluation, and graduation plans
  exist, but the durable encrypted observation log and complete
  evaluator→coordinator issuance path are not closed.
- Attestation issuance and projection are live, while issuer authorization,
  uniqueness, eligibility, and some temporal validation remain incomplete.
- Resource-nature and circularity vocabulary exists; automatic circularity
  obligations do not.
- Personal schedules have local CRUD and manual advance; durable dispatch and
  fulfillment do not.
- Requests/Offers, cooperative procurement, the EPR-REA fabric, the factory
  composition, tiered quilts, and several commons-allocation shapes are
  designed at different levels of maturity.
- Confidential replication has proof primitives, not a complete production key
  envelope and resolver path.

### Open frontier

- Sensed component inventory and durable hardware identity.
- SMART, ECC, PSU, UPS, fan, thermal, tamper, secure-boot, meter, inverter, and
  location/failure-domain observers.
- A production rack controller joining sensing, bounded decision, real
  actuation, verification, and recovery.
- Capacity-aware placement and automatic evidence-class × temperature-class
  policy.
- Parts ordering, logistics bridges, technician dispatch, physical-arrival
  privacy, service acceptance, secure media disposition, and recycling.
- Energy provenance and load control.
- Mutual credit, external settlement, risk and insurance, and a settled
  first-class commons-treasury authority.
- The capital bridge from accumulated commons value to capital-intensive
  manufacturing.

One existing sharp edge illustrates why this proof discipline matters: a
current heartbeat path treats a disk-probe error as fully free capacity. An
autonomous rack safety controller must fail safe under unknown evidence, never
convert “could not observe” into “healthy.”

---

## 12. Demonstration Ladder

The epic graduates through observable stories:

1. **The rack protects itself:** injected SSD degradation causes verified
   re-quilting before loss, bounded actuation, and no human systems
   administration.
2. **The household receives care:** the operator procures a replacement,
   schedules qualified service, verifies arrival and authorization, witnesses
   work, and proves restored health.
3. **The material loop closes:** the removed component follows a privacy-safe
   disposition path into witnessed recovered material and later manufacture.
4. **The energy and place loop closes:** metered source, storage, consumption,
   useful work, reserve, and failure domain inform placement without exposing
   intimate household location.
5. **The commons reproduces capability:** attributed flows maintain a commons
   treasury, finance shared infrastructure through bounded agreements, settle
   external obligations, and leave the productive capability less rent-gated
   than before.
6. **The proof survives exit and failure:** change the operator, supplier,
   bridge, hardware, and participating collective; the household keeps its
   identity, history, content, commitments, and practical ability to continue.

Each rung should eventually have an a2o acceptance scenario (an executable
behavior-driven story of the outcome), substrate integration tests, and real
hardware evidence. A green unit test for an action enum is not proof of a
cybernetic loop.

---

## 13. What This Epic Refuses to Claim

- `.epr-meta` is not the runtime telemetry store.
- The DHT is not a monitoring database.
- A declared device archetype is not an observed inventory.
- An alert is not a repair.
- A sent restart packet is not a recovered service.
- A technician's arrival is not completed work.
- A recycling transfer is not recovered material.
- A circularity enum is not an automated circular economy.
- A recognition unit is not automatically money.
- A commons allocation is not yet a finished external-capital market.
- “Fully automated” does not erase consent, labor, professional judgment, or
  the human and constitutional floors.
- “Subsuming capital” does not mean conquest, confiscation, or pretending that
  chip fabrication is already protocol-native.
- A future capability is never relabeled as live merely because the ontology
  can express it.

---

## Conclusion

The family rack is where the Elohim Protocol can prove that its values survive
contact with matter.

A trustworthy observation begins near the chip. It informs a bounded operator.
The operator protects the household's commitments, coordinates people and
resources, verifies the physical result, and carries the discarded material
toward another useful life. The same flow makes energy, labor, ecological debt,
and contribution visible. A governed share reproduces the commons that made
the coordination possible. Accumulated commitments make larger productive
capabilities investable without selling the commons or granting perpetual rent.

The result is not merely a healthy rack. It is a small, complete demonstration
of a viable system: physical truth becoming witnessed care; care becoming
coordinated value; value becoming shared capability; and shared capability
making enclosure progressively unnecessary.

That is the floor this hardware must hold.
