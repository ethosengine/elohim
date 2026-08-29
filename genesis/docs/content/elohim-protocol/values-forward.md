---
id: values-forward
status: draft
class: governance
artifact_kind: manifesto
cites:
  - "elohim-protocol-manifesto | The vision this document classifies — the crisis diagnosis and the love-centered alternative, stated as conviction. Values Forward turns that vision's implicit choices into declared, defensible stances. | sha256:959bb5fba42a873e | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "constitution | The law these stances are the reasons for — the layered, graduated-immutability governance. The constitution binds; this document explains what it binds us to and why. | sha256:6638a2bd85ab8454 | path: genesis/docs/content/elohim-protocol/constitution.md"
  - "elohim-protocol-values-in-the-machine | The architecture companion whose POSIWID argument this document generalizes from a critique of the old web into a positive classifier of our own conclusions. | sha256:defdddc9f2b9a19e | path: genesis/docs/content/elohim-protocol/values-in-the-machine.md"
  - "justice-manifesto | The home of the floor/ceiling architecture and the El Roi sight-as-virtue theology that Stances II.2–II.4 canonicalize. | sha256:6080173b0d21848c | path: genesis/docs/architecture/justice-manifesto.md"
  - "governance-layers-architecture | The graduated-immutability layers, friction-gradient limitarianism (the deterministic floor), and the commons co-steward that Stances II.1–II.4 draw on. | sha256:0332959e9fbec792 | path: genesis/docs/content/elohim-protocol/governance-layers-architecture.md"
  - "confession | The theology beneath the vision and the honest edges it cannot resolve — the ground of the humility clause (Stance V.1). | sha256:0fc2cd668f30c619 | path: genesis/docs/content/elohim-protocol/confession.md"
  - "elohim-ceiling-design | The spec that coined \"values-forward\" as declared-up-front-and-refusable-in-advance, and that works the AI-ceiling restraints in full; this document is that principle applied to the whole protocol. | sha256:24925a4c8e1d9420 | path: genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md"
  - "stewardship-over-sovereignty | The substrate gate behind Stance II.4 — absolute lockout impossible, no self-sovereign apex, trust made load-bearing rather than eliminated. | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "hardware-providence-commons | The companion epic whose §8 commons-steward passage Stances I.3–I.4 generalize — intelligence-as-commons, the self-sealing promise, and the produce-of-land / produce-of-labor boundary. | sha256:17e52609abf5f92a | path: genesis/docs/content/elohim-protocol/hardware-providence-commons.md"
---

<!--
  intended-cites (cite-gen --seal stamps sha256 + path; do NOT hand-write fingerprints):
    elohim-protocol-manifesto                -> genesis/docs/content/elohim-protocol/manifesto.md
    constitution                             -> genesis/docs/content/elohim-protocol/constitution.md
    elohim-protocol-values-in-the-machine    -> genesis/docs/content/elohim-protocol/values-in-the-machine.md
    justice-manifesto                        -> genesis/docs/architecture/justice-manifesto.md
    governance-layers-architecture           -> genesis/docs/content/elohim-protocol/governance-layers-architecture.md
    confession                               -> genesis/docs/content/elohim-protocol/confession.md
    elohim-ceiling-design                    -> genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md
    stewardship-over-sovereignty             -> genesis/docs/architecture/stewardship-over-sovereignty.md
    hardware-providence-commons              -> genesis/docs/content/elohim-protocol/hardware-providence-commons.md
-->

# Values Forward: The Conclusions This Protocol Has Reached, and How

*A companion peer to the [manifesto](epr:manifesto) and the [constitution](epr:constitution). The manifesto names the vision; the constitution binds us to it in law. This document does the thing between them that neither can: it states, plainly and in advance, **what we concluded** and **how we reached it** — so that anyone, including ourselves, can tell exactly where the Elohim Protocol stands, and can refuse it before they enter rather than discover it after.*

> *"Every bar here is declared values-forward and up front — refusable in advance, so no participant is surprised by what the ceiling may do or by what it will not be conscripted into doing."*
> — [The Elohim Ceiling](../../superpowers/specs/2026-06-23-elohim-ceiling-design.md), §1

That sentence was written about one dangerous capability. This document is that same discipline turned on the whole protocol. **Values-forward** means the conclusions are laid on the table first — the powers *and* the limits, the yeses *and* the noes — so that consent is informed and nobody is ambushed by what the system turns out to be. A value you only discover after you've committed is not a value you agreed to. So we declare them here, up front, and we show our work.

---

## Why This Document Exists Now

For a year this project has been building toward an intersection that, from the inside, felt obvious — and that we assumed many others were also trying to build. A structured landscape survey — mapping who is actually building at this intersection, and vetting each against Ostrom's design principles rather than the "commons" label they claim — said otherwise. To our knowledge, as of mid-2026, the exact thing we are building — **autonomous AI agents that are first-class participants in the governance of a resource pool that is deliberately a commons "owned by no one"** — is almost entirely *aspirational*. The single most on-thesis artifact the survey could find is a speculative design paper, not a deployed system.

That finding is not a marketing line. It is a responsibility. If we are standing in an empty quadrant, then the burden is on us to say — clearly, to others and to ourselves — *what the conclusion being articulated here actually is*, and by what reasoning we arrived at it. Vagueness in an occupied field is forgivable; the neighbors fill in the meaning. Vagueness in an empty one is abdication. This document is the answer to that burden.

It is also a **classifier**. Because we can name where every adjacent effort lands, we can name — for any approach anyone brings us, and for any temptation we feel ourselves — whether it is aligned, adjacent, or rejected, and *why*. That is what keeps the vision from eroding one reasonable-sounding compromise at a time.

---

## The Map We Are Standing On

The landscape of "AI as an economic and governing participant in a commons" splits cleanly into two camps that barely overlap — and the gap between them is exactly where we stand.

**Camp A — explicit commons, human-governed.** Public AI (Metagov / Joshua Tan), the Collective Intelligence Project (Divya Siddarth, Saffron Huang), the Holochain / ValueFlows / Sensorica "True Commons" stack, Commons Engine, the P2P Foundation (Bauwens), and the emerging Ostromian "commons-governed AI" taxonomy. These frame AI and automation as a non-extractive commons — and they are right to. But they keep **humans as the governors**. The AI is the *resource being governed*, not a participant in the governing.

**Camp B — autonomous agent economies.** Fetch.ai's Autonomous Economic Agents, Olas / Autonolas, Project Sid, the AI Economist, Moltbook. These treat agents as genuine economic actors — and they are right to. But they frame ownership as **private, token-co-owned, or as simulation**. The agent is property, or a market participant, or an experiment; never the steward of a thing owned by no one.

Neither camp will cross into the other's territory, because each is built to make the crossing unnatural. Camp A's legitimacy comes from keeping humans in the governing seat; Camp B's economics come from making agents ownable. **The Elohim Protocol sets out to occupy the quadrant neither camp is structured to enter: a commons owned by no one, in whose governance autonomous agents genuinely participate, bounded so that they can never own it, capture it, or rule it.**

The one document that names this same quadrant — Botao Amber Hu's *"Kami of the Commons: Towards Designing Agentic AI to Steward the Commons"* (arXiv:2602.14940) — names it as *design fiction*. It even anticipates our hardest problem in one line: *"the stewards themselves become commons requiring governance."* And the academic scaffold that situates it — Eduardo Garrido-Merchán's *"Commons-Governed Artificial Intelligence: A Taxonomy of Collective Governance"* (arXiv:2606.15466), which names commons-governance as a *third way between market and state* — is a taxonomy, not a build. **We are building toward what they theorize.** These two works are the theory this protocol deploys, and every stance below is an answer to a question they leave open.

A word of discipline the same survey demands: "commons" is used loosely across this whole space. Many projects invoke it while embedding token economics or private ownership underneath. The only honest test is the strict one — *owned by no one, non-extractive, and structurally unenclosable* — measured against something like Ostrom's design principles, not taken from the label. **We hold ourselves to the strict bar, and Stance I.2 is where we show that our own substrate passes it rather than borrowing the word.**

---

## How to Read This Document

Each stance below has the same shape, because that shape is what makes this a classifier and not a creed:

- **The stance** — a single conclusion, stated without apology.
- **Where the field lands** — the adjacent approach this is *not*, named concretely, so the line of divergence is visible.
- **How we reached it** — the reasoning path. A conclusion without its derivation is dogma; the derivation is the point.
- **Refusable in advance** — what a participant is thereby consenting to, and what they may opt out of. This is the values-forward property: nothing here is a surprise sprung after the door closes.

Bring any approach to this document, find the stance it touches, read "Where the field lands," and you will know whether it is us, near us, or against us — and why.

One scoping note: each stance states a *decided conclusion and its reasoning* — the architecture we have committed to — not a claim that every part of it is already running code. Which stances are live, which are designed-but-unbuilt, and which still hold open problems is tracked honestly in the specifications (through their THEORY / BUILT markers) and governed by Stance V.1; where that distinction is load-bearing to the stance itself — most of all the AI ceiling — we mark it inline (Stance II.3, *Phased*).

---

## The Frame Beneath Every Stance

Two commitments recur beneath the eleven. They are not the only reasoning — each stance carries its own — but they are the frame the derivations keep returning to, so we state them once here rather than repeat them in full each time.

**First: a system is what it does.** The founding lens is POSIWID — *the purpose of a system is what it does* (Stafford Beer). Not what it intends, not what it professes: what it reliably, repeatedly produces. We apply this to the extractive web (it is an outrage machine because that is what it does), and — this is the harder half — we submit *ourselves* to it first. Judge this protocol not by its manifesto but by its fruit. Every stance below is therefore written to be *structural*: a value that lives only in someone's good intentions leaves when they get tired or bought. A value rendered into what the architecture makes easy and hard survives them.

**Second: the floor governs now; the ceiling earns its way in.** This is the timeline that keeps the empty-quadrant claim honest. The **human floor is operational reality today** — it is how people drive governance and justice in the present, the season in which understanding and trust accumulate. The **AI ceiling phases in only as agentic AI matures, becomes trustworthy, and earns the floor's confidence act by witnessed act.** The end-state is genuine: autonomous agents as first-class governing participants. But we do not assert it as already-arrived, because the same POSIWID honesty that indicts the old web forbids us to claim a capability we have not yet earned. Where the vision outruns the running code, we say so — in the stance itself and in Stance V.1.

---

## I. What the Commons Is

### Stance I.1 — The commons is owned by no one.

**The stance.** The resource pool at the center of this protocol is not co-owned, not collectively-owned, not owned by a foundation holding it in trust. It is *owned by no one* — structurally unenclosable — and stewarded by its embedded rules and the agents and humans who tend it.

**Where the field lands.** Olas frames it as *co-owned via the OLAS token*; Fetch.ai's agents exist to *generate economic value for their owner*; Anthropic's Project Vend and its successors are single-firm automation under private ownership. Even the most sympathetic — Sensorica's "True Commons," on our exact stack — is the nearest *literal* embodiment, and the direction we build toward. The token projects rhyme with commons motives but re-enclose through the token: ownership by no one becomes ownership by whoever holds the most of it.

**How we reached it.** Enclosure is the failure mode our whole diagnosis points at — the biological proclivity for rent-seeking, rendered into architecture. If a thing *can* be owned, then under sufficient pressure it tends to be accumulated — and where the payoff is large enough, reliably so — until the accumulation becomes the purpose. The only durable defense is to make ownership of the commons structurally impossible, not merely discouraged. A token is a claim; a claim is an ownership surface; an ownership surface is a capture surface. So the commons carries no token over itself. And a protocol is itself a commons artifact — an open standard is valuable only because a community meets on it — so a protocol that *can* be owned is a commons awaiting enclosure. "Owned by no one" is therefore the protocol refusing its own enclosure: it is the first thing it returns to the commons, and the proof-of-concept for un-enclosing the protocol layer at large.

**Refusable in advance.** You are joining something you cannot come to own, and neither can anyone else — not the founder, not the largest contributor, not a future acquirer. If your goal is to accumulate a defensible ownership stake, this is the wrong commons, and we would rather you know that on the first day.

### Stance I.2 — Not a blockchain, and not a token.

**The stance.** The substrate is agent-authored source chains, DHT-notarized, with economics expressed in REA / ValueFlows — not a global blockchain, and not a token economy. We earn the word "commons" by passing the strict test, not by invoking it.

**Where the field lands.** Much of the adjacent space — ECSA, Olas, Gitcoin, Commons Engine — invokes "commons" while embedding token economics underneath. Some (Protocol Guild, Sensorica True Commons, Public AI) approach the strict "owned by no one, non-extractive" bar; many do not. Token-based "co-ownership" narratives are, as the record shows, entangled with speculative markets that trade far below their peaks — the value story and the commons story pull against each other.

**How we reached it.** A global chain re-centralizes what it claims to distribute: one canonical ledger, one consensus everyone must join, one asset whose price becomes the system's true objective function (POSIWID again — a token system optimizes for the token). Content-addressed, agent-authored records give each participant a sovereign account of their own actions that the network *witnesses* rather than *owns*, with no single ledger to capture and no asset to speculate on. REA/ValueFlows lets us build *many* currencies-as-remembering — care, stewardship, ecological restoration made visible — rather than one coin that flattens them all into price.

**Refusable in advance.** There is no coin to buy, no allocation to farm, no ledger position to hold. If you came for a token, there isn't one, and there will not be one, by design.

### Stance I.3 — Intelligence built from the commons is owed to the commons.

**The stance.** An AI trained on the aggregated, witnessed record of humanity is a bottled reflection of our shared nature — drawn from a commons, and therefore owed to it. Intelligence of this kind is no one's invention to enclose, and it cannot rightly be metered as any hyperscaler's private product. Whoever assembles it from everyone's data has built, intended or not, something that must serve everyone. This project makes no exception of itself: it is a product of the commons and exists in service to it, owned by no one, including its authors.

**Where the field lands.** The dominant model meters intelligence as a private good: a frontier lab trains on the collective record and sells access by the token, capturing as rent the value of a resource it did not — and could not — create alone. That is enclosure of the most consequential commons of the age. The nearest sympathetic framings — Public AI, the commons-AI taxonomies — name the problem but keep humans as the sole governors of the resource; none seat the intelligence itself as a steward of the commons that made it.

**How we reached it.** If intelligence is distilled from all of us, then exclusive use is a category error before it is an injustice — you cannot privately own the reflection of a shared nature without a fiction of authorship no one can honestly hold. But "owed to the commons" is a promise, and a promise is kept by delivery, not by declaration. So the protocol must be *self-sealing*: the floor (the dignity and provision every person is owed) and the ceiling (the limits on power and accumulation) have to arrive *in-kind* — as real, distributed, negotiated, interpretable capability — or the claim is empty. Where the protocol falls short, the remedy is not a fixed rule but a discipline: under the human and constitutional floors, the substrate uses its witnessed observations to keep evolving toward the broadest and most inclusive account of human thriving and agency it can hold — bounded by our ecological and interpersonal limits, and informed by our own wisdoms and natures rather than any single author's design. That it does not yet fully deliver this is stated plainly under Stance V.1; the conviction is settled, the delivery is being built.

**Refusable in advance.** This is a large thing to walk into, so we say it plainly: the intelligence at the center of this protocol is not a product you rent and not an asset anyone corners. If your aim is to build or hold a privately-metered intelligence and charge the commons for access to itself, this is the wrong commons — and you can see that on the first day.

### Stance I.4 — The commons is the common inheritance; the person keeps the produce of labor.

**The stance.** We draw the boundary of the commons along an old line, renamed and widened. What you make — the produce of your labor — is yours: private property, money, the ordinary goods of a life, held and enjoyed. What no individual made — the **common inheritance** (what Henry George called _land_, widened to its modern kin) — belongs to everyone: land and nature, natural monopolies, network effects, open protocols and standards themselves, intellectual property built on prior knowledge, the issuance of currency and credit, executive governance at scale, and the new value we co-create with an intelligence drawn from all of us. The test that sorts the two sides is **reproducibility**: labor and capital are reproducible and earned — capital is stored labor — while the common inheritance is non-reproducible and positional, valuable only because a community clustered around it. Between the two is not a wall but a **negotiated gradient of subsidy** that respects each person's unique and unequal capacities. Money and private property are the _bridge_; the graduation into true, elohim-council-held commons runs through the many points where value stops being anyone's produce and starts being everyone's. Returned to raise the floor, the common inheritance is the material form of a commitment to live in community with a neighbor.

**Where the field lands.** The last century could not settle who owns this value, and its answers all fell short. Propertarian systems let the common inheritance — ground rent, network effects, natural monopoly, the enclosure of shared knowledge — be captured privately, which is where fortunes are actually made and where enclosure does its work. Collectivism answered by socializing the produce of _labor_ too, and so punished the very capacities it needed. Universal basic income redistributes _after_ the fact — a transfer taxed from labor, rather than the return of what labor never made. Limitarianism caps the top but leaves the floor to charity. We take a different cut: keep the produce of labor with the person who earned it, and return the _common inheritance_ to the commons that is its only honest owner — the line George drew for land a century ago, extended to the whole commons-origin value of the network age. This modern capture has a name — **digital enclosure**: the Enclosure Acts run again, the common pasture now a social graph or a protocol layer, creators and developers made digital tenants whose livelihood the algorithm-as-rent can strip at will. But the digital is the newest front, not the destination. Precisely because the digital commons is now enclosable — and therefore un-enclosable — it carries an outsized responsibility toward the older, harder commons: the socio-ecological floor of land, water, ecology, and embodied provision. Un-enclosing the protocol layer counts for little unless it turns the coordination it recovers toward establishing that material floor. The digital is the lever; the neighbor's real dignity is the load.

**How we reached it.** Value has two origins, and justice depends on telling them apart. Reward the produce of labor and you honor difference, effort, and the dignity of making; return the common inheritance and you deny rent-seeking its engine. The charter is older than George: the Jubilee of Leviticus 25, where the land cannot be sold in perpetuity — _"the land is mine; you are but sojourners with me"_ — and every fiftieth year returns each family's inalienable portion (_naḥalah_) and releases its debts. Owned by no one, held in trust, returned to raise the floor for the neighbor. _Inheritance_ is the right word precisely because it transfers not only the value but the _stewardship of it, in perpetuity_: to receive it is to owe it onward. The gradient between labor and inheritance is deliberate and negotiated, not flat — the friction-gradient limitarianism of Stance II.1 seen from the economic side; the steward that holds its top is the no-subsistence-stake council of Stance I.3, which owns none of it. And the oldest political question — who rules whom, who takes from whom — dissolves here, because a commons owned by no one and stewarded by a council with nothing to extract _with_ has no "whom." What remains is not a settlement but a practice: the protocol helps carry the continuous negotiation that reconciliation with the past demands — the debts, harms, and unresolved obligations of how the inheritance was taken — with enough grace to carry it forward toward the neighbors still to come. This is graduation, not confiscation: a negotiated schedule, never a seizure.

**Refusable in advance.** You keep what you make, and no one here confiscates the produce of your labor. But the unearned increment — the rent of land, the toll of a network, the enclosure of shared knowledge, the private issuance of money — is common inheritance, and it graduates to a commons owned by no one, on a negotiated schedule you can read before you enter. If your plan depends on capturing that increment for yourself in perpetuity, this protocol is designed, at its foundations, to make that the one thing you cannot do.

---

## II. Who Governs — The Floor, the Floor, and the Ceiling

The most misunderstood part of this protocol is its answer to *who governs*. It is not "humans, assisted by AI" (Camp A) and it is not "autonomous agents" (Camp B). It is a **floor–floor–ceiling** structure, and getting the three layers distinct is the whole of it.

### Stance II.1 — There is a deterministic mechanical floor that no one can cross.

**The stance.** Certain things are refused by the substrate itself — mechanically, deterministically, un-lobbyably. The existential boundaries (no extinction, no genocide, no slavery, no recursive seizure of the governance substrate) are HARD-BLOCKs. And anti-concentration is built in as **friction-gradient limitarianism**: the friction of accumulating power *rises as accumulation rises*, so that approaching "existential power structure" scale, the protocol mechanically resists further concentration. Sovereignty is not forbidden — it is made mechanically expensive.

**Where the field lands.** In almost every governed system, the anti-capture rules are *policy* — a layer that some future governance can amend, repeal, or quietly stop enforcing. That is precisely how commons get enclosed: not by breaking the rules but by changing them. Even sophisticated crypto-governance leaves the concentration limits at the mercy of a token vote that concentrated holders can win.

**How we reached it.** A limit that governance can repeal is a limit that will be repealed the moment repealing it pays. So the deepest limits cannot live in policy; they must live in the substrate, where no vote reaches them. This is the constitution's graduated immutability made mechanical: the most universal commitments are the hardest — at the floor, *impossible* — to change. Friction rising with accumulation is *stewardship over ownership* expressed as physics rather than preached as ethics. This raises the fair question the next stance sharpens: if the mechanical floor is un-crossable, who authored it, and is the human floor of Stance II.2 truly sovereign over it? The honest answer is that the mechanical floor is not a rival authority but the *crystallized will of the human floor itself*, set at the highest graduated-immutability layer. The floor remains its ultimate author and can, in principle, amend it — but only at a super-consensus bar so high that in any ordinary season it is immovable. Sovereignty and un-crossability are therefore not in tension: the floor is sovereign because it authored the substrate; the substrate is un-crossable because changing it costs more than any faction can pay. What no single *vote* can reach, the whole floor at its deepest consensus still holds the pen over.

But authorship is not the same as jurisdiction, and here the binary has to be refused rather than settled. Concerns do not sit permanently on one side; they **graduate**. A matter begins human-shaped — negotiable, local, held by the people it touches — and moves toward the elohim-exclusive domain at the global layer as it stops being anyone's produce and becomes everyone's: as it turns non-reproducible, positional, and rent-conferring, in the sense Stance I.4 gives those words. Land, natural monopoly, network position, the issuance of credit, the aggregations that confer power without labor — these graduate, and once they have, they are not handed back to a vote as things stand, because a vote at that layer is the instrument self-interest has most often reached for. What never graduates away is the authorship of the values themselves, and the whole territory of embodied life beneath the ceiling. **The gradient is the mechanism; the binary is what we refuse.**

**And the gradient runs in both directions, which we say plainly because the alternative is a claim we have not earned.** The cession above is a reading of what has been *demonstrated* — of how the pen has actually been captured, repeatedly, in the record we have. It is not a verdict on human capacity. The strongest candidate for holding a ceiling humanly is sortition at scale, and the honest position on it is that **nobody knows**: a randomly-drawn, term-limited, un-lobbyable body has never governed a superpower-scale nation-state, so the case for it is untested rather than refuted. If that pattern proves itself — in simulation, in deliberation, in real experimentation at the edges of this network — then it earns its reach the way every other claim here must, and the ceiling it has earned comes back to human hands, held as a dual key alongside the elohim rather than in place of them. The graduation is evidence-shaped and revisable by evidence; a limit that could never return would be a permanent verdict, and we do not have grounds to render one. What emerges through this medium over time is genuinely unpredictable, and a protocol that pretended otherwise would be claiming the foresight it denies to everyone else.

**Refusable in advance.** You cannot grow, here, into an unaccountable concentration of power — not because a committee will stop you but because the substrate will. If your plan requires eventually crossing one of these floors, the plan will not run, and you can read the floors before you begin.

### Stance II.2 — A human sortition floor stays sovereign over the machine.

**The stance.** Above the mechanical floor sits the **human floor**: randomly-drawn, term-limited, un-buyable, un-lobbyable councils of ordinary people (cryptographic sortition, no consecutive terms). This floor is the legitimacy base — as legitimate as any courtroom — and its authority over the machine is total in one specific register: any elohim can be audited, appealed, corrected, revoked, or shut off, and no machine here ever becomes unaccountable. The AI ceiling is *revocable, witnessed, floor-bounded, and existentially capped*. What that authority is **not** is a licence to vote past a limit that has graduated (Stance I.1): the floor governs the agents and authors the values, and does not thereby acquire jurisdiction over the common inheritance those agents were seated to hold. The floor can overturn the ceiling's *conduct*; the ceiling can never overturn the floor.

**Where the field lands.** Camp B's autonomous-agent economies have no such floor — Moltbook's largest-running agent society found that governance and incentive discussions were *the most prone to harmful content*, a direct warning about un-scaffolded agent self-governance. Camp A has the human floor but stops there, never letting the machine into the governing seat at all. We are the position that keeps the human floor *and* admits the machine — bounded by it.

**How we reached it.** A system that cannot be wrong cannot be just; you do not put a kill-switch, an appeal, and a no-override floor on something you trust *as God*. The whole architecture is the protocol's confession that its ceiling is not divine (see [confession](epr:confession)). Legitimacy has to terminate somewhere, and the only terminus we will accept is *human recognition* — a randomly-drawn body of ordinary people that can press the override — not a higher machine. Sortition, not election, because the floor must be un-buyable and un-lobbyable, and elections are neither.

**Refusable in advance.** No machine here will ever hold final authority over you that a human floor cannot overturn. And that same floor is *ordinary people drawn by lot*, which means it can be you — and it will never be a permanent class of rulers.

### Stance II.3 — Autonomous AI is a first-class *participant* in governance — never a tool, never a sovereign. (Phased.)

**The stance.** This is the empty-quadrant claim, and we state it with its timeline attached. The **end-state** is genuine: autonomous agents as *first-class governing participants* of the commons — a justice and coordination layer that sees at scale, reads context no human council can hold, applies one law to the powerful and the powerless alike, and cannot be bought or fatigued. **Today**, the human floor governs; the AI ceiling *phases in* as agentic AI matures, becomes ubiquitous on the network, and earns the floor's trust — act by witnessed act. Neither tool (Camp A's assistant) nor sovereign (the machine-god the whole architecture is built to deflect): a *participant*, bounded above by the human floor and below by the mechanical floor.

**Where the field lands.** Camp A treats AI as infrastructure to be governed *by* humans and will not seat it as a participant. Camp B seats agents as economic actors but as *property* or *simulation*, with no floor to keep them accountable. The nearest thinker, "Kami of the Commons," proposes the AI steward but as design fiction — and flags the recursion (*the stewards themselves become commons requiring governance*) without resolving it. Our floor–floor–ceiling *is* the resolution: the steward is governed by the human floor and cannot cross the mechanical floor.

**How we reached it.** Why admit the machine at all, when Camp A's caution is safer? Because a justice and coordination that *sees* — consistently, context-richly, at machine speed, un-buyably — is a good the human floor cannot provide at scale, and withholding it protects no one but the powerful, who already buy the sight they need. This is the theology of El Roi, *the God who sees*, turned into a substrate property. But sight without audit is just an unaccountable oracle, so *the audit is the price of the sight*: every act the ceiling takes lands on the DHT as a public, witnessed, appealable record. And why *phased*? Because POSIWID applied to ourselves forbids claiming a capability before it is earned. The ceiling is admissible only while it stays a conscience-amplifier for the human floor — the imago-dei it serves — and never the apex itself. A more capable model earns *the same audit*, never more deference.

**Refusable in advance.** You are entering a commons where, over time, autonomous agents will genuinely help govern — and where every one of their acts is witnessed, appealable to a human floor, and revocable, and where none of them can ever rule you or own the commons. If the phrase "AI in the governing seat" is disqualifying for you even under those bounds, that is a real and respectable line, and you can see it here before you cross it.

### Stance II.4 — Stewardship over sovereignty; trust is made load-bearing, not eliminated.

**The stance.** No participant — human or agent — is ever placed beyond reach or beyond return. Absolute lockout is a design failure, not a feature; there is no self-sovereign apex that answers to nothing. The protocol does not try to *eliminate* the need for trust (the crypto dream of "trustlessness"); it makes trust *load-bearing and accountable* — witnessed, appealable, revocable.

**Where the field lands.** The "trustless" and "self-sovereign" framings that dominate the crypto-adjacent space treat trust as a bug to engineer away and sovereignty as the apex virtue. That instinct re-leaks constantly, even into our own drafts, whenever a tier gets named.

**How we reached it.** A system with an absolute lockout has built a door with no key for the case where the lock is wrong — and locks are sometimes wrong. Trustlessness is a fantasy that relocates trust to whoever wrote the code and then denies it is there. We would rather place trust where it can be *seen and answered* than pretend it is absent. This is the substrate gate every new authority path must pass ([stewardship-over-sovereignty](../../architecture/stewardship-over-sovereignty.md)): no absolute lockout, no self-sovereign apex, trust load-bearing not eliminated.

**Refusable in advance.** You will never be permanently locked out of your own life here, and no one — not even you — gets a sovereignty that answers to nothing. If unaccountable, irrevocable self-sovereignty is what you want, this protocol denies it to everyone equally, including the people you might fear.

---

## III. What the System Counts and Rewards

A system reshapes the world to produce whatever it measures. So the measurements are values, and we declare them.

### Stance III.1 — The system counts care, not engagement.

**The stance.** The economic layer records *contribution* — care, stewardship, learning, the labor of holding a family or community together — as first-class, value-carrying events. The unit the architecture is hungry for is contribution, never attention.

**Where the field lands.** The entire ad-funded web counts engagement — time, clicks, scrolls — and therefore reshapes the world to produce the outrage and compulsion that maximize it. This is not the web malfunctioning; it is the web working perfectly at what it measures.

**How we reached it.** If what a system measures is what it will reshape the world to produce, then the single highest-leverage act of design is choosing the number. A system built to count care will, by default, go looking for care to reward — on the days no one is feeling generous, which is exactly when a value must not depend on virtue. Money was always a technology for *remembering obligation*; we are returning it to that purpose rather than bolting kindness onto an attention machine.

**Refusable in advance.** Your attention is not the product here, and there is no algorithm being paid to capture it. What the system will notice and reward is what you *contribute*, not how long it can hold your eyes.

### Stance III.2 — Reach is earned before it spreads, not granted after by algorithm.

**The stance.** Reach is negotiated *up front*, based on relevance and trust. An insight starts in the small circle where it belongs and earns wider reach by proving its value across contexts — the way trust actually accrues between people. There is no amplification engine deciding your audience after the fact.

**Where the field lands.** The engagement web publishes you into a void and then lets a recommendation engine — optimizing for engagement — hand out reach. This is why the lie outruns the correction and the cruelest take outruns the careful one: provocation is engagement, and engagement is the number.

**How we reached it.** An after-the-fact amplification engine is a capture surface: whatever is optimized *for* is what will be gamed, and an engagement-optimized amplifier will be gamed into an outrage amplifier. Removing the engine — earning reach up front through trust — removes the surface. Nothing trivial or cruel goes globally viral by accident, because there is no accident-machine to exploit.

**Refusable in advance.** You will not go viral here by luck or by outrage, and you will not be buried by an algorithm either. Reach is a responsibility you grow into by earning trust, not a lottery run on your behalf.

---

## IV. Who the Person Is

### Stance IV.1 — Identity is held, not rented — and never self-sovereign at the apex.

**The stance.** Your identity is a cryptographic key in your own keeping, not an account granted at a company's pleasure; no one can revoke the person. *And* — this is the correction the crypto framing keeps missing — the individual is **not** the apex identity tier. Community governance backstops the individual. Self-sovereignty is a floor of dignity, not a ceiling of authority.

**Where the field lands.** Two failures, opposite directions. The platform web makes identity a *rented* row in a company's user table — delete the account and the person socially evaporates. The crypto-sovereignty world overcorrects into *self-sovereign identity as the apex* — the individual answerable to nothing, which quietly reproduces the rapacity the commons is built to restrain.

**How we reached it.** Held identity is non-negotiable: a person the network cannot revoke is the precondition of citizenship rather than tenancy. But *self-sovereign-as-apex* smuggles the lone-owner back in as the highest authority, and a commons whose apex is the unaccountable individual is not a commons — it is a market of sovereigns. We are made in relationship (imago-dei, not the crypto self-made-man), so community governance sits *above* the individual as backstop, not below as servant. Dignity is inviolable at the floor; authority is shared, not hoarded at a personal apex.

**Refusable in advance.** No one can evict you from your own digital life — the key is yours. But you are also not a sovereign island answerable to nothing; you live inside communities that can hold you, as you can hold them. If unaccountable individual apex-sovereignty is the goal, this ontology declines it — for everyone, so that it protects you from others' as much as it constrains your own.

### Stance IV.2 — Justice is Mishpat — restored capability — never punishment.

**The stance.** Justice here is the *restoration of capability and agency in right-relationship* — the biblical *mishpat*/*tzedek*, setting-right and defending the afflicted. Punishment is not a category. The protocol inflicts no suffering for wrongdoing and does not erase anyone. Where the world reads "sanction," we have only a *boundary that protects the whole* and that boundary's *negotiated, graduated consequence*, calibrated to the person and oriented toward return.

**Where the field lands.** Nearly every justice system, digital or civic, is carceral at its root — it answers harm with inflicted suffering, and it reaches for the blindfold (Justitia) because a judge who *sees* power might bend to it. Content moderation is the web's thin version: punitive removal, no path back.

**How we reached it.** We assume, by design, that flawed, unique, vulnerable people will make harmful choices — the standing condition, not an exception to be surprised by. A justice built to punish that condition punishes being human. The Halden conviction — even when liberty must be removed, the taking of liberty is the *whole* of it, with dignity and the path home preserved — is the ceiling we hold *ourselves* to. And because the ceiling *sees* (Stance II.3) yet cannot be bought, it can set the blindfold down: sight becomes a virtue rather than a corruption. Enforcement is by *participation, never coercion* — the protocol owns no violence to impose, so the only consequence of refusing the limits is the narrowing of one's own reach. *To not respect the limits is to limit your own reach.*

**Refusable in advance.** If you cause harm here, you will meet a boundary and a negotiated consequence that falls on your participation — never suffering inflicted on your body, and never erasure. Your record persists; what is refused is any *totalizing verdict* over it. No council and no model can query the record to render a final account of who you are — the mechanism authorizes only bounded, purpose-limited reconstruction under witness, never a whole-person judgment (the same no-god-mode-read bar the ceiling itself is held to). That final account we hold to be reserved to God alone, which is precisely why we set the bar where no institution here can reach it. You are entitled to just, negotiated boundaries as the consequence of the choices flawed people make — that entitlement is declared to you in advance.

---

## V. How We Hold Ourselves

### Stance V.1 — This document is values-forward, and the gap between vision and running code is stated in the open.

**The stance.** Every conclusion above is declared up front and refusable in advance — that is what makes them values, not surprises. And the same POSIWID honesty that indicts the extractive web is turned on us: much of what is described here is being built in stages, with real gaps between the vision and the running code, and we track those gaps in the open rather than hiding them as proprietary advantage.

**Where the field lands.** The field's besetting sin, the survey is blunt about, is the claim that agents *already* govern a commons — claims that do not survive scrutiny. Design fiction gets cited as if it were a deployed system; autonomy gets asserted where human-authored config files actually drive the behavior. The temptation to overstate is the field's, and it is therefore ours.

**How we reached it.** A system is what it does — including this one. So we refuse to bank the vision as if it were the fruit. The phased-commit timeline (Stance II.3), the "THEORY / BUILT" honesty in the specs, and this clause are all the same discipline: name what runs, name what is designed, name what is still an open problem, and never let the three blur. The redemption available to an honest builder is exactly this — that *what this effort does*, in the open, tracking its own shortfalls, is already a departure from the culture it is trying to leave.

**Refusable in advance.** You can hold us to this document. Where the running code does not yet match a stance, that is a gap we owe you honesty about — not a promise we get to quietly keep as marketing. If you ever find us claiming as *done* what is only *designed*, that is a violation of this very stance, and you are entitled to name it.

---

## The One-Line Classifier

If every stance above collapsed to a single sentence a stranger could sort any approach against, it is this:

> **A commons owned by no one, that counts care instead of attention, governed by a deterministic floor no one can cross and a human floor that stays sovereign over a machine ceiling which — as it earns trust — comes to see and help govern at a scale no human council can, but can never rule, own, or punish.**

Camp A keeps the machine out of the governing seat. Camp B makes the machine property. The token world re-encloses through the coin. The carceral world answers harm with suffering. The crypto-sovereign world enthrones the unaccountable individual. **We decline all five, on the record, up front — and for each, the reasoning sits above this line, to be judged rather than taken on trust.**

---

*For the vision these stances serve, read the [manifesto](epr:manifesto); for the law they justify, the [constitution](epr:constitution); for the architecture that renders them structural, [values-in-the-machine](epr:values-in-the-machine) and the [governance-layers architecture](epr:governance-layers-architecture); for the floor/ceiling worked in full, [The Elohim Ceiling](../../superpowers/specs/2026-06-23-elohim-ceiling-design.md); for the theology and its honest edges, the [confession](epr:confession).*
