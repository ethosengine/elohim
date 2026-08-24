---
id: succession
sovereignty-frame: adversary
cites:
  - "elohim-protocol-manifesto | The vision this document clears the ground for — the crisis it answers and the love-centered alternative it proposes, which a reader arriving with a formed politics will not see until the older argument is settled. | sha256:cd62d3cc869bada5 | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "values-forward | The thirteen Stances this document treats as settled law — most of all I.1 (the commons owned by no one), I.2 (not a blockchain and not a token), I.4 (the common inheritance), II.4 (stewardship over sovereignty), and IV.2 (enforcement by participation, never coercion). | sha256:5f4acd177219031f | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "constitution | The law that operationalizes the vision — the layer precedence and the universal gate this document's refusals inherit. | sha256:1eb96af782012fc6 | path: genesis/docs/content/elohim-protocol/constitution.md"
  - "confession | The theology beneath the vision, and the discipline this document's most present-tense claim answers to: leaven, not the Kingdom. | sha256:bec001fd41230c67 | path: genesis/docs/content/elohim-protocol/confession.md"
  - "justice-manifesto | Where justice is defined as restored capability rather than punishment, and where humility is named as the one virtue a confident system structurally cannot perform. | sha256:6080173b0d21848c | path: genesis/docs/architecture/justice-manifesto.md"
  - "stewardship-over-sovereignty | The foundational canon behind this document's refusal of the illegibility position — no self-sovereign apex, trust made load-bearing rather than eliminated. | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "glossary | Plain definitions of the recurring terms (elohim, reach, standing, substrate, REA) that this document's orientation section restates for a reader arriving from outside. | sha256:faeef215b3a16143 | path: genesis/docs/content/elohim-protocol/glossary.md"
---

# Succession Without Conquest

## For the reader who arrives full

You have been offered two positions your entire adult life.

One says the market allocates, that capital earns its return, and that the people complaining have not understood the arithmetic. The other says the owners have taken what the workers made, and that the remedy is to take it back. You have probably chosen. You have defended the choice at dinner tables. You have also, if you are honest, noticed that neither position has moved anything much in your lifetime, and that the conversation has the worn quality of an argument nobody expects to win.

There is a reason the choice feels narrow, and it is not that you failed to read enough.

In 1916 Silvio Gesell made an observation about which economic ideas get discussed. Marx, he argued, is *safe* for capital. The Marxist remedy is to interrupt production — strike, seize, halt — and interrupting production keeps capital scarce, which is precisely what preserves its yield. A strike is a demand for a larger share of a scarcity that the strike itself maintains.

Proudhon's idea was different, and Gesell thought that was why nobody had heard it. (A disclosure that belongs here rather than at the end: what follows is *Gesell's account of* Proudhon, not Proudhon read directly. §11 says what that remove costs the argument.) Proudhon's claim was that **capital's power comes from its scarcity, not from its ownership** — and therefore that the way to end the power of capital is not to seize the factory but to build so many factories that owning one gives you leverage over no one. Rent collapses when there are more dwellings than households. Interest collapses when there is more productive plant than there are projects needing it. On this reading the owner's true dread is not expropriation. It is *the building plague*.

If that is right, an obvious question follows: it has been two centuries, so why has the building not happened?

Proudhon's answer is the part worth the price of admission. **Money can wait, and goods cannot.** Grain spoils, tools rust, buildings need roofs; all of them cost something to hold. Money alone sits at par indefinitely, costing nothing. So when the return on real assets starts falling toward zero — which is exactly what abundance does to it — the holders of money can simply stop. Not conspire; stop. Withdraw from investment, hold cash, wait. Production halts, and the halt is reported as a crisis of overproduction, though the goods were never the problem. **The withdrawal manufactures precisely the scarcity that restores the yield.**

You may believe this or not. What matters here is that you were probably never given it to consider, and that the two positions you *were* given both leave it untouched.

**This document is written for the reader who arrives full.** For the union organizer, the co-op operator, the socialist, the anarchist, the Georgist, the libertarian, the economist — anyone carrying a formed account of how capital and labour actually work, and a well-earned suspicion of anyone arriving with software and a vision.

The Elohim Protocol has a manifesto. It is a document of hope, and it is written the way such documents are written — for the reader who is willing to imagine. If you arrive with a politics already in place, the manifesto will not reach you. Your priors will eat it before the second page, and they will be *right* to, because you have seen this before: the technology that was going to change everything, the platform that was going to be different, the community that got acquired.

So this document goes first. It does not ask you to imagine anything. It asks you to consider a narrower claim, in the vocabulary of the tradition you already know:

> **The mutualist tradition was not refuted. It was priced out. And the thing that priced it out has just become cheap.**

If that claim is wrong, nothing else here matters and you have lost an hour. If it is right, then a set of designs you may have filed under "tried, failed, romantic" become live engineering questions, and the vision in the manifesto stops being a wish and becomes a consequence.

If you want to know quickly whether this is worth an hour, skip to **§5.7 and §5.8**. They are about the fight over AI datacenters that is very likely happening in your county right now, and it argues that the anger is correct — not as sympathy, but as an accurate read of a rent relationship, arriving through the only channel that was left open. If that section is wrong, the rest of this will not persuade you either.

We will also tell you, in §9, exactly where this argument outruns what we have built — including the part where our own argument leans on a capability built at hyperscale without the consent of the people whose record trained it. That section is not modesty. It is the only reason to trust the rest.

---

## Orientation — the words this document leans on

This document uses a small vocabulary with precise meanings. None of it is decorative, and several terms mean something narrower than they sound.

**The substrate** — the shared data layer everything runs on: peer-to-peer, tamper-evident, with no central server. Not a company's cloud, and — for reasons this canon treats as settled rather than stylistic — not a blockchain: a global chain re-centralises what it claims to distribute, since one canonical ledger means one consensus everyone must join and one asset whose price becomes the system's true objective. **How it holds together without a chain, in one paragraph, because the claim is otherwise unverifiable:** each participant keeps their own append-only record of their own actions, signed and hash-linked, so no entry can be altered after the fact without breaking every link that follows it. Those entries are published to a shared distributed hash table — a network in which each item is held by whichever participants happen to sit nearest its address, so no one machine holds the whole and no one machine is the one to seize — and the peers who receive a given entry, chosen by the address of the content rather than by who published it, independently validate it against the rules the application declares, and refuse it if it does not hold. Tamper-evidence therefore comes from the individual signed chain plus the validating witnesses, not from a global ordering everyone must agree on. The trade is explicit: this buys locality, parallelism, and no system-wide asset, and gives up the one thing a global chain provides — a single total order. Which is why this substrate is a poor fit for anything that genuinely needs one, and a good fit for recording who did what, for whom, and who saw it.

**A witnessed event** — the measurement primitive. One concrete act — three hours sat with someone's mother, a roof repaired, a dispute resolved — attested by the people who were party to it and recorded on the substrate. Not a self-report and not a survey response. The attesters have something at stake in it being true.

**An elohim** (lowercase, plural *elohim*) — a local AI agent, running on a participant's own hardware, that reads the substrate and helps a person or a community make sense of what it holds. The name is from the divine council of Psalm 82: messengers and servants, explicitly never to be worshipped. **A council** is the deliberative body — human and elohim together — that actually decides. The distinction is enforced throughout: *the council steers, the agent reports.* "The elohim decides" is a category error in this corpus.

**Reach** — how far a person's contribution travels beyond the people who already know them. Reach is *earned* and is gated by standing. **Belonging is never gated; only reach is.** A person always belongs; how far their words carry is what is earned.

**Standing** — a person's relational track record, as the network has witnessed it. Deliberately *not* a score: there is no stored number, and different evaluators compute it differently from the same underlying events. It cannot be bought, transferred, pledged as collateral, or spent. This matters in §8, where standing turns out to be the enforcement mechanism, and it is the reason that mechanism is not a social credit system.

**Mishpat** — the Hebrew for justice, and the name of the protocol's justice layer. Justice here means **the restoration of capability and agency in right relationship** — a thing given back, not a thing inflicted. **Punishment is not a category in this protocol.** There is no retribution and no debt paid in pain. What exists is boundaries that protect the whole, and negotiated, graduated consequences that fall on *participation* — how far you carry — never on a body.

**REA** — Resource–Event–Agent, an accounting model from 1982 that records economic reality as agents performing events against commitments, rather than as debits and credits. It is what lets the substrate hold care work and infrastructure maintenance in the same grammar as trade.

**The common inheritance** — the protocol's name for value no individual made: nature, natural monopolies, network effects, open protocols and standards, knowledge built on prior knowledge, and — named explicitly — *the issuance of currency and credit*. What Henry George called *land*, widened to its modern kin. The test that sorts it from ordinary earnings is **reproducibility**: labour and capital are reproducible and earned; the common inheritance is non-reproducible and positional, valuable only because a community clustered around it. The produce of your labour is yours — property here is the bridge, not the destination (§3.7). The common inheritance is owed to everyone.

**The Stances** — thirteen numbered positions in a companion document (`values-forward.md`) that record what this project has actually settled, each stating the position, where the field lands, how it was reached, and what it refuses in advance. They are cited here the way one cites a constitutional provision, and this document treats them as binding rather than as suggestions. The ones that carry weight below are I.1 (*the commons is owned by no one*), I.2 (*not a blockchain, and not a token*), I.4 (*the common inheritance*), II.4 (*stewardship over sovereignty*), and IV.2 (*enforcement by participation, never coercion*).

**Deterministic floor, elohim ceiling** — the two-layer decision architecture, and the source of a great deal of what follows. Its shape is owed to Kate Raworth's floor-and-ceiling framing and to limitarian arguments about when accumulation becomes a public problem — but taken as arguments brought *to* George and Gesell rather than inherited from them, which is the move §3.7 turns on. The **floor** is mechanical: it runs with no AI and no network, it is the same for everyone, and it cannot be argued with — a guaranteed minimum of provision and dignity, a small set of absolute prohibitions, and limits that get harder to cross the more power is being concentrated. The **ceiling** is discerning: it reads a particular situation in context, proposes, and steers. The rule that keeps them from collapsing into each other is stated once and meant literally: **never a computed payout at the ceiling, never judgment at the floor.**

**Corpus and canon** — *corpus* means the body of this project's writing; *canon* means the part of it that binds: the Stances, the constitution, the manifesto, and the documents on this shelf.

**The operator** — this project's human steward. One developer, with a full-time job, funding the tooling out of pocket. Worth knowing before §9, because the honest answer to "what have you built" is shaped by it.

---

## 1. The thesis

Every design in the mutualist tradition — Proudhon's mutual bank, Gesell's stamped money, George's capture of the unearned increment, the Swiss WIR's clearing circle, Kevin Carson's counter-institutions, Michel Bauwens' commons, Bernard Lietaer's currency ecology — is **solvent below a coordination-cost threshold and insolvent above it.** None of them was refuted. Each was priced out.

Four of the costs are transactional, and they are always the same four:

- **matching** — who needs what I have
- **underwriting** — how much credit can this person carry
- **clearing** — whose books close against whose
- **monitoring** — did the thing actually happen

Historically these are paid to a broker, a bank, a clerk, or a state. Proudhon's Bank of the People needed clerks it never got. The WIR bought its survival with a banking licence and salaried underwriters. Sardex, the most successful mutual-credit network operating today, runs on human brokers who match trades over the telephone.

The fifth cost is different in kind, and the tradition never named it as a cost at all: **discernment** — *what does right relationship actually require of me, here, given what I hold, what my neighbour lacks, and the particular shape of my life?* That question has an answer, it has a different answer for every person, and arriving at it honestly takes a long, high-context negotiation nobody has ever been able to afford at population scale. §3.7 argues that this single unaffordability is why George reached for a tax and Gesell for a stamp, and §5.5 argues that the substitution is no longer forced.

The claim, then:

> **A witnessed-event substrate with a local AI reader collapses all five costs. That collapse is not a feature of the protocol; it is the protocol's economic content.** On the four transactional costs it extends a known result — that commons-based peer production out-competes both firms and markets once coordination costs fall far enough — from information goods to credit, care, and public-good provision. On the fifth it does something the tradition never had available: **it makes the good life stop being hostage to the reform.**

What follows from that runs on two tracks, and both are **unilateral** — neither needs anyone's permission, a threshold, or a political victory.

**At the level of the institution: liability absorption.** The network grows by taking on obligations the incumbent order is failing to meet, measuring what it absorbed, and presenting that measurement in the incumbent's own units. A municipality drowning in unfunded liabilities does not fight a thing that reduces its liabilities. It books the reduction. (§6)

**At the level of the person: the negotiated equilibrium.** Someone holding rent-bearing capital in an extractive economy can ask what they would honestly be owed if the just settlement had already arrived — and limit their take to that, now, in the world as it is. Not a compromise with extraction: right relationship reached under present conditions, requiring no legislation and no one else's participation. (§5.5)

The second track is the one the tradition could not offer, and its absence is why its ethics kept arriving late. George's justice required the single tax to pass. Gesell's required Freigeld to circulate. Both made the moral life contingent on a political victory that never came, so a person of conscience inside the old order had nothing to *do* but wait and advocate. That contingency was never really about politics. It was about the cost of working out what right relationship required of *this* person — and that cost is now falling.

Three disciplines hold the argument honest, and each has its own section.

1. **It repeals nothing it cannot repeal** — not Hayek's objection about dispersed knowledge (§5.4), not the fact that a state collects taxes in its own currency, not the fact that software cannot repossess a tractor (§8).
2. **It refuses the escape it would most like to have** — the idea that a network can stand outside the law by having no capturable centre (§7).
3. **It must not become a lock-in** — and if succession requires coercion, the correct response is to stop (§6.5).

---

## 2. What the manifesto left open

The manifesto has a theory of how the protocol spreads, and it is an unusually decent one. It calls it **emission**: the claim that the right measure of success is not how many people join but how much good spills over into the lives of people who never will — and, correspondingly, that non-participation must never be made costly. *"Adoption is not the strategy; emission is."*

Emission explains **diffusion**. It does not explain **succession**. It says how a healthier practice travels; it does not say how a public function — maintaining a water basin, resolving a dispute, caring for an elder, clearing a local trade — *moves* from the institution that currently carries it to a substrate that could carry it better.

The corpus reaches for that move at least three times and stops each time. *"The old system doesn't need to be defeated. It simply becomes less attractive than the alternative"* is an outcome, not a mechanism. Elsewhere the project has written that *"subsumption is the peaceful attractor path — not overthrown, outgrown,"* which names the intuition and leaves it unmechanised. And in one place the manifesto proposes something worse than a gap, which §9.1 takes up: that those who decline will face *"rising extraction costs, reputational erosion, and social isolation."* That is a mechanism, and it is coercion.

A project without a succession theory does not stay neutral about succession. It improvises one under pressure, and the improvisation is almost always either confrontation — which invites exactly the response the whole design exists to avoid — or enclosure, which trades the commons for defensibility. Naming the mechanism in advance is how the refusals in §6.5 become architecture instead of good intentions.

---

## 3. The lineage, read for what each could not afford

Each figure below is read for a **single structural deficit**, and the deficits are the argument.

One thing should be said before the reading starts, because it bears on how to hear it. Everyone in this section worked without institutional capital. Proudhon's bank was liquidated while he was in prison. Gesell self-published. Carson writes from outside any university. Bauwens' P2P Foundation has run for two decades on grants and volunteer labour. Lietaer did his most important work after leaving the institutions that would have funded it. **The deficits below are structural conditions and asymmetries against incumbent capital — not performance reviews.** The same standard applies to this project, and §9 applies it.

### 3.1 Proudhon — abundance is the weapon, and the clerks are the cost

The opening section of this document is Proudhon's diagnosis as Gesell relays it — which is how it reaches most readers, this one included — so it needs no restatement: capital's power is a function of capital's *scarcity*, uninterrupted production is therefore the weapon, and money's zero cost of carry is what lets that weapon be blocked.

From the diagnosis follows the design. **Credit is not a commodity, and interest is not a natural price.** If money is an accounting unit issued against real goods and future labour rather than a scarce master-commodity, then its cost falls to the administrative cost of maintaining the account — a figure Proudhon defended against Bastiat through 1849–50. His *General Idea of the Revolution in the Nineteenth Century* (1851) states the political corollary: the state is not to be seized but **dissolved into the economic organism**, with contract replacing law and federation replacing hierarchy.

The **Bank of the People** was chartered in January 1849 with something on the order of thirteen thousand subscribers, and never meaningfully operated. Proudhon was imprisoned in March; the bank was liquidated in April.

Read that as political suppression and you learn little, because suppression is contingent. Read it structurally and you learn the tradition's founding constraint. A mutual bank at any scale must decide how much credit each member may draw, must know whether the goods were actually delivered, and must reconcile the books. Proudhon's design assumed that work away, because in a district where everyone knows everyone it is nearly free. **His system was solvent at village scale and had no financing for its own coordination at any larger one.**

*Could not afford: underwriting and clearing beyond the range of acquaintance.*

### 3.2 Gesell and George — the twin pillars, and the chokepoint that closed them

Gesell supplies the mechanism Proudhon lacked for the hoarding half of the problem: **Freigeld**, money bearing a carrying cost, so that the one asset with no cost of carry stops holding all the leverage. The point is not to reproach saving. It is to put money on the same footing as the goods it is exchanged against.

Gesell was explicit that Freigeld fails without **Freiland** — the Georgist half — because capital fleeing a carrying cost will buy the one thing that neither decays nor can be reproduced. George's *Progress and Poverty* (1879) had already named that thing and its remedy: the unearned increment of location value, returned to the community that created it. This corpus takes George's line and widens it into *the common inheritance*, which is the term used from here on.

Keynes' verdict — that *"the future will learn more from the spirit of Gesell than from that of Marx"* (*General Theory*, ch. 23) — is usually quoted without the critique beside it, and the critique is the useful part: **liquidity preference does not stop at money.** Put a carrying cost on cash and the preference migrates to the next non-decaying store. Gesell knew it; Freiland is his answer; and the pairing is why our own treatment of the common inheritance and our treatment of circulation are one argument rather than two.

**Wörgl**, Austria, 1932–33, is the demonstration. Michael Unterguggenberger issued stamp scrip bearing roughly one percent a month in a town of about five and a half thousand people; it funded public works, unemployment fell, and neighbouring municipalities began to copy it. The Austrian National Bank shut it down in September 1933.

That ending is the instructive one, and it is not an economic ending. Wörgl was not out-competed. It was **enjoined**. The binding constraint turned out to be neither liquidity preference nor land speculation but *a legal monopoly on issuance, exercised through a court*.

*Could not afford: any posture toward the issuing state beyond hoping not to be noticed.*

### 3.3 The WIR — the one that lived, and the price of the ticket

The **Wirtschaftsring** was founded in Switzerland in 1934 by two businessmen working explicitly from Gesell, in a depression when Swiss francs had stopped circulating. It is the tradition's single durable success: still operating, tens of thousands of Swiss small and medium businesses, a clearing unit held at parity with the franc, near-zero-interest credit issued against member trade, and payment typically **split** — some fraction in francs, the remainder in WIR.

Two facts about it are usually reported apart and belong together.

**It is counter-cyclical.** James Stodder's empirical work on Swiss data has found WIR turnover moving *against* GDP: when the franc economy contracts and commercial credit withdraws, WIR volume rises. That is the strongest evidence in existence for the claim that a second, differently-behaved medium of exchange is a stabiliser rather than a curiosity.

**And it paid for it.** WIR took a Swiss banking licence in 1936 and **dropped Gesell's demurrage in 1948**. It survived by becoming a regulated cooperative bank doing unglamorous work — assessing members, taking collateral, filing liens in Swiss courts when someone defaulted. The Gesellian mechanism turned out to be the part that could be discarded. The *institutional apparatus* was the part that could not.

That trade is the most instructive single datum in this document. **The WIR is the control experiment.** It kept the economics, paid for the coordination with institutional form and staff, and lived. Every attempt that could not buy that apparatus stayed at village scale. Its choice — independence *through* legibility rather than in spite of it — is the one §7 argues we should learn from rather than avoid.

### 3.4 Carson — the exodus, without an accounting of what the exodus absorbs

Kevin Carson's *Exodus: General Idea of the Revolution in the XXI Century* (2021) is the closest existing statement of this document's political shape, and the echo of Proudhon in its title is itself the argument.

As we read him: capitalism's institutional forms are not the spontaneous product of markets but are **held up by state-enforced artificial scarcity** — Benjamin Tucker's four monopolies of money, land, tariffs and patents, updated to foreground intellectual property, licensure, zoning, and subsidised transport. The bottleneck those monopolies protect is dissolving on its own as the cost of production and coordination collapses. Therefore the strategy is not confrontation but **exodus**: build counter-institutions faster than the incumbent can suppress them, and let its fiscal exhaustion and legitimacy decay do the rest.

We take nearly all of it, with one exception worth naming because it recurs across this lineage: Tucker's critique of monopoly, and Carson's after it, is grounded in a self-proprietorship that answers to nothing. This corpus does not share that floor — its floor is *imago dei* — the image of God, and so a dignity that is inviolable because it is conferred rather than claimed, backstopped by community rather than asserted against it — which is also why §7 refuses a network that answers to nothing. What is otherwise missing — and it is what separates a strategy from a mechanism — is **an accounting of what the counter-institution has absorbed.** Exodus describes a population walking out. It does not describe the incumbent's balance sheet on the way down, and it gives the departing population no way to *demonstrate* that their departure made the incumbent's problem smaller rather than larger. Absent that demonstration, every exodus reads to the institution being left as **defection** — a shrinking tax base against a rising service burden — which is precisely the reading that triggers suppression instead of accommodation.

*Could not afford: a measurement layer. Exodus without a ledger is indistinguishable from free-riding, and gets treated accordingly.*

### 3.5 Bauwens — the institutional design, without a ledger that closes

Michel Bauwens and Vasilis Kostakis supply the institutional half Carson leaves implicit: **commons-based peer production** as a third mode alongside firm and market; the **value crisis**, in which peer production generates enormous use value that capital captures as exchange value because the commons has no mechanism for its contributors to live; the **three institutions** (productive community, a for-benefit association stewarding infrastructure, an entrepreneurial coalition meeting the market); and the **partner state**, public authority reconceived as an enabler of commons rather than a provider of services — the direct ancestor of §6's posture, and the reason that posture is cooperative rather than adversarial.

What is missing is **a substrate where the accounting actually closes.** Contributive accounting is proposed, prototyped, and stalls in the same place every time: who witnesses the contribution, what makes the record trustworthy to someone who was not there, and what happens when two contributors disagree. The Peer Production License is a *legal* instrument aimed at the value crisis, and it depends on courts — returning the commons to the jurisdiction it was trying to leave. The partner state is a *political* ask with no artifact to hand the partner.

*Could not afford: a witness. The institutional design is right and has no evidentiary layer under it.*

### 3.6 Lietaer — the ecology, without an issuer that isn't an institution

Bernard Lietaer is the practitioner of the group: a National Bank of Belgium official who worked on the ECU, then a currency fund manager, then the field's most prolific documentarian, in *The Future of Money* (2001), *Money and Sustainability* (2012), and *Rethinking Money* (2013, with Jacqui Dunne).

The load-bearing borrow is that **monetary monoculture is itself a structural cause of fragility**. A system with one medium, optimised for throughput efficiency, has no reserve capacity when that medium withdraws — which is the WIR result generalised. Alongside it sits a large case corpus: WIR, Chiemgauer, Japan's Fureai Kippu care credits, Curitiba, Banco Palmas, Sardex, C3 — the empirical record of what this tradition has actually managed.

Lietaer's deficit sets up the contribution most directly. **Every currency in his corpus has an institution at its centre** — a bank, a nonprofit, a municipality, an NGO, a brokerage, a development organisation. He documents the diversity beautifully and never resolves the recursion: *each complementary currency requires a competent, funded, trustworthy institution to run it, and the scarcity of such institutions is the actual constraint on monetary diversity.* You cannot have a thousand currencies if each one needs a bank.

*Could not afford: an issuer that is not an institution.*

### 3.7 What none of them could afford: the blunt instrument was never the ideal

Read the two most mechanical designs in the lineage again, and ask *why they are mechanical*.

George's remedy is a **single rate applied to every parcel**. Gesell's is a **single stamp on every note**. Neither man thought a flat instrument was the just answer to the question he was actually asking, which was a question about persons: *how much of what you hold did you make, and how much did the community make around you?* That has a different answer for a widow on an inherited smallholding and for a speculator holding vacant lots against a rising city. George knew it. The single tax was not his account of justice. It was the only instrument that could be **administered** against a population by clerks with paper.

The same for Gesell. Demurrage is indiscriminate by construction: it charges the hoarder and the pensioner at the same rate, because a stamp cannot tell them apart.

So the tradition made a trade it never wrote down: **it substituted a universal mechanical rule for a high-context personal judgment, because the judgment was unaffordable at scale.** Every subsequent critique of these designs — that a land tax hits the asset-rich and cash-poor, that demurrage is regressive on small balances, that both are indifferent to circumstance — is a critique of *the substitution*, not of the underlying moral claim. The moral claim survives its instruments.

This matters because our own canon refuses that substitution outright, and the refusal is easy to mistake for softness. Between labour and the common inheritance, Stance I.4 puts *"not a wall but a **negotiated gradient of subsidy** that respects each person's unique and unequal capacities"*, and insists the return is *"graduation, not confiscation: a negotiated schedule, never a seizure."* It ends not with a settlement but with a practice. Read against this section, that is not softness. **It is the claim that the substitution is no longer necessary** — and it is only available to a project for which discernment is cheap.

One detail settles a great deal and is easy to miss: Stance I.4 calls **money and private property the *bridge*** — the thing you cross on the way to a commons, not the enemy to be abolished. The lineage's most anti-capitalist figures would have recognised the move. None of them could afford to make it, because a bridge requires per-person judgment about who is crossing and how far.

---

## 4. The common bottleneck, in the incumbent's own vocabulary

Six deficits, one shape. Stated in orthodox rather than movement economics, because the argument is stronger there and because §7's posture toward law has to be legible to people trained in it.

**Coase (1937), "The Nature of the Firm."** Firms exist because using the price mechanism has costs — discovering prices, negotiating, contracting, monitoring. The boundary of the firm sits where internal coordination cost equals market transaction cost.

**Benkler (2002), "Coase's Penguin."** There is a *third* mode — commons-based peer production — which out-performs both firm and market for a class of goods, and becomes viable when the cost of coordinating dispersed contributors falls below either alternative. Free software is the existence proof. Benkler's boundary condition is explicit: it works for **information goods**, where marginal contribution cost is low, output is non-rival, and — crucially — **verification is cheap.** You can read the patch.

Now stack the deficits against that boundary condition:

| Ancestor | Deficit | Coasean name | Why it stayed expensive |
|---|---|---|---|
| Proudhon | underwriting and clearing beyond acquaintance | monitoring, enforcement | no way to know a stranger's capacity to reciprocate |
| Gesell / George | posture toward the issuer | *external* — a legal chokepoint | the issuance monopoly, enforced in court |
| WIR | mechanism vs. independence | all four, paid institutionally | bought the coordination with a licence and staff |
| Carson | no accounting of absorption | signalling to the incumbent | exodus is indistinguishable from defection |
| Bauwens | no witness under contributive accounting | verification | contribution records nobody outside can trust |
| Lietaer | an institution per currency | fixed cost of the issuing organ | competent institutions are scarce and don't scale |

Five of the six rows are **coordination costs on physical, relational, and care activity** — precisely the region where Benkler's condition fails, because verification is *not* cheap there. You cannot read the patch on whether a neighbour actually sat with someone's mother for three hours.

That is the whole problem, and it is one problem, not six.

**Sardex is the proof by construction.** Founded in Sardinia in 2009, thousands of member businesses, zero interest, assessed credit limits. What distinguishes it from the projects that built the ledger and could not fund the brokerage is that it employs **human brokers who actively match trades and vet members**, and practitioners describe the brokerage — not the ledger — as the product. Anyone can write the smart contract in an afternoon. Nobody can afford the brokers.

**So the question of whether the mutualist tradition was wrong or merely early is a question about the price of coordination — and for the first time it is a live question.**

---

## 5. What this protocol adds

The tempting answer is "a peer-to-peer substrate and local AI." That is a technology answer and proves nothing. The economic answer is a set of functions.

### 5.1 The witnessed event makes reciprocity observable without a market

A witnessed event carries who did what, for whom, when, and who saw it — with the attesters' own standing attached, so that a false attestation costs the attester something real.

That is the missing **verification** term. Benkler's condition failed for care and physical labour because verification was expensive; a witnessed-event substrate makes verification a by-product of participation rather than a separate audit.

Two honest qualifications, both of which this project has already had to make against itself. First, the witnessed event is not yet counter-signed by the person receiving the care — and *a witnessed-care record that the cared-for cannot contest is a conferred identity with a signature on it.* That is an open defect, not a shipped feature. Second, and larger: **the scarce thing was never the observation.** Devaki Jain measured the care economy in Indian villages in 1976, six years before the argument became famous under other names, and the measurements did not travel. What a substrate contributes is not witnessing but **claim-attachment** — identifying the decision a measure should change and the party obliged to act on it. A measurement with no claim attached is a fact nobody owes anything to.

### 5.2 The elohim is the broker, the underwriter, and the clerk

Sardex's brokers do four things: know who needs what, know who can be trusted for how much, notice when a balance is drifting, and call people. Matching, underwriting, monitoring, outreach — the four transactional costs of §1, in human form, at roughly one broker per few hundred businesses.

A local agent is the same organ at a cost that does not scale with population, running on the participant's own hardware, reading a substrate that already holds the evidence. **Stated as what it is: a hypothesis with a measurement attached.** Nobody has yet run a mutual-credit circuit with an agent in the broker's seat and measured whether matching, underwriting, monitoring and outreach costs actually fall far enough. §11 says so plainly, and the reader should carry that caveat forward through the sections that follow rather than meet it at the end. What follows is what becomes *available* if the substitution holds — not a report of it holding. This is not a novel idea; it is Stafford Beer's. Beer's *ideal regulator* — a salesman attached to every customer, an idea he called ridiculous purely on the cost of putting intelligence at every endpoint — is exactly the thing that stopped being ridiculous.

The contribution here is only to notice that the **same organ** is what six different economic designs were each missing, and that its affordability is therefore not a convenience but the precondition for all of them at once.

**And the hazard, which this project has named against itself: *ownerless is not uncaptured*.** An entity with no owner still has interests — written by whoever authors its weights, curates its training corpus, or controls its update path. *A living-room agent that phones home for its values is a datacenter with a nicer address.* Physical locality of compute is necessary and nowhere near sufficient. The safe form is **agent-as-clerk of a self-executing, community-amendable constitution — never lord** — and the thousand-year case law for endowments that outlive their founders is a better guide here than anything in the software literature.

### 5.3 The clerk reports; the council decides

An agent that *sets* credit limits is a central planner with better manners. An agent that computes and shows a member's reciprocity record, flags a drifting balance with the evidence attached, and proposes a limit for a council to accept, amend, or reject is a clerk. The difference is the difference between an aggregate a council can only accept or reject and a **mechanism a council can argue with**.

Two qualifications, both binding. The council is **not a human veto-floor** — by this project's own standing conviction, human-in-the-loop is not the terminal authority; the *method* is, and humans hold the right to be heard by the method, the right to leave and to withhold consent, and a guaranteed minimum of participation no deliberation can go beneath — and they are the evidence. And the AI ceiling is **phased, not shipped**: today the human floor governs. Claiming otherwise would violate the project's own honesty stance, which readers are entitled to hold it to.

### 5.4 What this does not repeal

**Not Hayek.** "The Use of Knowledge in Society" (1945) is not an argument about computation. It is an argument about knowledge that is tacit, local, dispositional, and *not articulable* — the knowledge of particular circumstances of time and place. A witnessed-event substrate captures more of that than a price does, because a price is a one-dimensional projection and an event carries context. It does not capture all of it and never will.

The honest claim is therefore **not** that the calculation problem is solved. It is that **no calculation is being attempted**: computation stays local, aggregation is voluntary and anonymised, and the global layer holds no allocative authority. This is requisite variety and polycentric governance, not a planning board. Any design in this project that starts allocating from an aggregate has crossed that line and should be read as a defect — which §9.2 does, to our own manifesto.

**Not the tax gate.** A state accepts taxes in its own currency, and that is what anchors demand for that currency. No protocol design removes it.

**Not physical enforcement.** Software cannot repossess a tractor. §8 is the answer, and §8 names the hole in it.

### 5.5 The fifth cost: discernment, and the good life that stopped waiting

This is the section the others exist to make sayable.

**The question.** A person holds rent-bearing capital inside an economy organised around extraction. They did not design it and cannot exit it. What does right relationship require of them — *now*, not after a reform? The tradition has a clean answer in principle: you are owed the produce of your labour, and the unearned increment is not yours. It has never had an answer in practice for a specific person on a specific Tuesday, because turning that principle into a number for *this* life requires knowing what they hold, how it was acquired, what their dependants need, what their community lacks, what capacities and incapacities they carry — and what they would have been owed had they been born into the just settlement instead of this one.

**The counterfactual is the benchmark.** That last clause is the operative one, and it is answerable in form: *what would this person be owed by the common inheritance, honestly apportioned, in the settled end-state?* Whatever that is, they can limit their take from rent to it, voluntarily, inside the extractive economy as it stands. Doing so is not a compromise with extraction and not a down payment against a debt owed later. **It is the whole of right relationship, reached under present conditions.** The end-state is not a precondition for it. The end-state is what it looks like once enough people have chosen it.

**Why nobody could do this before.** The benchmark is not computable by a rule and not negotiable at scale by humans. It is irreducibly high-context, and getting it wrong in either direction is a real harm — too low and you have laundered extraction as conscience; too high and you have made moral seriousness a luxury only the secure can afford. Reaching it honestly means a long, patient, well-informed negotiation with someone who knows both the principle and the person. Historically that has been available to almost nobody: a handful with a spiritual director, a trusted rabbi or pastor, or an unusually candid friend. **That scarcity — not the politics — is why this tradition's ethics arrived as legislation instead of as a practice.**

**What changes — and first, exactly what is being claimed, because the order matters here more than anywhere else in this document.** The claim is *not* that this conversation has been held, that anyone has measured it, or that a machine knows what you owe. Nobody has run it. It is that the conversation has become **specifiable and affordable** — that a thing which required a scarce, patient, well-informed interlocutor is no longer rationed by that scarcity. Everything in this section is the shape of an available practice, not a report of one performed. Read the sketch below as a specification you can attack, not as evidence.

With that stated first: when the marginal cost of applied wisdom tends toward zero, the patient well-informed interlocutor stops being scarce. The narrow form of the claim is that **the conversation in which you work it out is no longer rationed by the cost of counsel.** We do not accept that a reasonable equilibrium — considered in loving context toward one's neighbour, in a specific life with its specific obligations — is beyond what our better angels can help someone reach.

**What the conversation actually looks like — because the claim is otherwise unfalsifiable.** Take a specific case. A woman in her sixties owns three rental units in a town where rents have doubled in a decade. She did not cause that, she is not wealthy by any measure her neighbours would recognise, and the units are her retirement.

The inputs are records rather than sentiments, and the substrate either holds them or can: what she paid and when; what she has put in since, in materials and her own labour; what the units would let for against local wages rather than against the market's own trajectory; how much of the present value is the buildings and how much is the ground beneath them, which is the part George says was never hers; what her own floor actually costs, including care, medication, and the years she will not be able to work; what her tenants earn and what they are paying now; what the town's housing gap is.

The elohim's work is to assemble that into a reading and show every step: here is the produce of your labour, here is the positional increment, here is what your floor requires, here is the band within which your take stops being rent and becomes wage, here is how that band moves if your health changes. Then it names what it is unsure about, and where a different reasonable reading would land on a different number.

Then it stops. She decides — perhaps against the reading, perhaps after arguing with it, perhaps with her council or her family or her pastor in the room. What may afterwards land on the substrate is not the calculation but her commitment: declared, bounded, revocable, witnessed, in the same form the protocol uses for every other kind of authority.

**What that sketch is and is not evidence for.** It shows the conversation is *specifiable* — the inputs are records, the method is legible and auditable, and the output is a decision a person owns rather than a figure a system applies. It does not show that current models perform it well, that evidence of that quality is actually available, or that anyone has run it once. That debt is real, and §11 states it.

**The shape is the floor/ceiling split, and the rule that keeps it honest is meant literally: never a computed payout at the ceiling, never judgment at the floor.** An equilibrium arrived at in conversation is not a number the substrate calculates and applies. A dignity floor is not something a council deliberates its way beneath.

Four disciplines, because this is the claim most likely to be overread.

1. **The agent does not decide.** It reads, computes, shows its work, and proposes. The person decides about their own life; the council holds what a person cannot hold alone.
2. **Humility is the one virtue a confident system structurally cannot perform.** *"I could be wrong"* is not something a confident system self-generates, which is why the human audit and the appeal are not decoration on a system that is usually right — they are where the humility the machine cannot produce actually lives. A discernment ceiling with no appeal is not a ceiling. It is a verdict.
3. **Counsel is not authority.** What has become affordable is counsel. Conflating that with governance would be the project claiming a capability it has not earned.
4. **Leaven, not the Kingdom.** The corpus closes this point itself, and it is worth quoting because it is the discipline on everything above: *"What does the LORD require? Not a substrate of wisdom at the intimate edge of every life. He requires **you** — to do justice, love mercy, walk humbly. The substrate, at its best, makes those three a little easier for more people to do with each other… It is leaven. It is not the Kingdom, and the difference between building leaven and believing you have built the Kingdom is the whole difference between the prophets and Babel."*

**What this licenses saying** is the document's strongest present-tense claim. For most of this tradition's history, *doing justice* about one's own holdings was hostage to a reform nobody could deliver, so conscience was discharged in advocacy, in charity, or in guilt. It need not be. **Walking humbly with respect to what one holds becomes a practice a particular person can take up without waiting** — not because the just economy has been built, but because the thing that made the practice unaffordable was the scarcity of patient, informed, loving counsel, and that scarcity is ending.

### 5.6 Which abundance? The floor that building can raise or consume

§3.1 credited Proudhon with the tradition's founding weapon: build until the scarcity that yields rent is gone. The weapon is real and this document keeps it. But *"build"* is under-specified in a way the nineteenth century could afford and we cannot.

**First, correct the comparison.** The choice is not between a regenerative path that asks for restraint and an incumbent path that delivers plenty. **The incumbent path is itself a scarcity state**, and it is failing precisely on the denominations that constitute a life. The argument that we have engineered a sustained transfer away from the young — visible in collapsed wealth share among the under-forties, in housing that no longer clears against wages, in a generation that is the first to do worse than its parents — is the plainest available statement of it.

The advanced economy is abundant in calories, screens, consumer goods, and logistics. It is scarce in housing, in mobility that does not require owning a car, in time, in care, in proximity to grandparents, and in the ordinary conditions of forming a family. **A currency that carries no values cannot tell those two abundances apart** — which is the manifesto's opening complaint, arriving where it hurts most.

**Second, notice that this is Proudhon's own mechanism, running today, in plain sight.** The building plague is not hypothetical: abundant housing would destroy the yield on housing. So supply is restrained — by zoning, by land banking, by treating dwellings as an asset class — and the restraint is defended as prudence. That is the money-strike of §3.1, executed on real assets rather than on cash. **Generational scarcity is not evidence that Proudhon was wrong. It is evidence that he was right and the counter-move worked.**

**Third, the distinction the tradition never drew.** Two paths both register as abundance and are not the same kind of thing:

| | **Extensive** | **Intensive / regenerative** |
|---|---|---|
| Food | land-hungry monoculture at scale | food forests, polyculture, controlled-environment growing |
| Settlement | car-dependent single-family sprawl | medium-density walkable neighbourhoods, co-housing |
| Mobility | a car per adult, and the road system that requires | street design where most trips need no car |
| Its move | **consumes** more land and energy per unit of output | **raises the carrying capacity** of land already held |
| What it needs | capital, fossil energy, permissive code | knowledge, relationship, negotiated agreement |

The second column is not austerity. A Dutch city is not a poorer place than an American exurb; it is a place where a twelve-year-old can get to a friend's house alone and a household does not need two cars to function. That is *more* abundance in the denominations that were scarce, delivered by raising the capacity of the same ground.

And the asymmetry is not a matter of preference. **Carrying capacity is erodible** — overshoot degrades the limit itself — so the extensive path does not merely approach a ceiling. It **lowers the floor while claiming to raise it.** The honest promise a protocol can make here is narrow: it cannot remove the delay between damage and perception, only collapse it, so what it can offer is that **overshoot becomes witnessed and priced**, not that it becomes impossible.

**Fourth, there are *many* such paths.** Not one blueprint. Food forests differ by watershed; street design that works in Utrecht is not what works in Lagos or in a Texas county; what a bioregion can carry is a local fact. A single prescribed regenerative model would be the planning board wearing a garden.

**Fifth — the point.** Ask what the intensive column actually costs, and the answer is not money. **It costs social capacity.** Sprawl, monoculture, and car dependence are the configurations that require the *least* coordination: they can be produced with capital, fossil energy, and a permissive code, by parties who never have to agree about anything. A food forest is a multi-decade knowledge practice. A Dutch street is the residue of decades of political and cultural work, not a procurement decision. Co-housing, a repair economy, shared tooling, a commons that survives — every one of them is negotiation-intensive, relationship-intensive, knowledge-intensive.

> **Most efforts toward abundance have ignored our social capacities to lift the natural floor — and that is why the extensive path won. It was never more productive. It was cheaper in the one input nobody could mass-produce.**

Which is where this meets §4. If the binding constraint on the regenerative path is coordination cost — and above all the **discernment** of what this particular place can carry and what its people will actually agree to — then a collapse in that cost does not merely make mutual credit solvent. **It makes the socially-intensive abundance solvent**, which is the only kind that kills rent without eating the floor it stands on.

That completes Proudhon's weapon rather than replacing it. Build until the scarcity that yields rent is gone — *and* build the kind that raises what the ground can carry, which requires the coordination he never had and could not have costed.

---

### 5.7 The datacenter occasion: the enclosure, and why the anger is correct

A reader in 2026 has probably watched this fight in their own county, and it is worth saying plainly that their instinct is right, and right for the reasons this document has been building toward.

The scale is no longer marginal. In the first quarter of 2026, seventy-five major datacenter projects worth more than $130 billion were delayed or cancelled, in significant part because of organised local opposition. More than three hundred cities, towns and counties have passed bans or moratoriums. On the eighteenth of July there were a hundred and forty-two protests across forty-two states in a single day. A June poll found roughly a third of Americans approve of the pace of construction, and fourteen per cent are comfortable with one being built near them.

The stated grievances are real — noise, water, land, air — but the one doing most of the work is
**the bill**. Wholesale power on the largest US grid rose seventy-six per cent year over year in the
first quarter of 2026. Around thirty-eight states offer datacenter tax incentives; roughly two dozen
are now moving to pause, cap, condition or repeal them, and Georgia revised one programme's cost
projection upward by 664 per cent, to $2.5 billion. Oklahoma passed a Ratepayer Protection Act
requiring large loads to carry their own grid and interconnection costs instead of shifting them onto
households. Illinois, Arizona and Ohio have paused programmes; New Jersey froze one; North Carolina is
phasing its out.

**Read that structurally and it is this document's subject, running at full scale in the present
tense.** The grid is common inheritance in the strict sense — non-reproducible, positional, valuable
because a community clustered around it and paid for it over a century. The tax base is the same. A
deal that abates the taxes, socialises the interconnection cost, and privatises the output is not a
market transaction that some people happen to dislike. It is **the unearned increment being captured,
by exactly the mechanism §3.2 describes**, with the ordinary rate-payer holding the residual.

And it is worse than that, because of what the facility is *for*. An intelligence trained on the aggregated written and spoken record of humanity is drawn from a commons that no lab created and no lab could have created alone. Metered by the token, it becomes **rent on a resource its owner did not make** — the most consequential enclosure of the age, and the one this corpus already names as such. The datacenter is that enclosure given a physical address, a substation, and a county tax abatement: the theft of the commons, sited on the commons, billed to the commons.

So the anger is not irrational, and it is not NIMBYism wearing a green coat. **It is an accurate read of a rent relationship, arriving through the one channel that was left open** — the zoning hearing, because that is where an ordinary person is finally allowed to say no to something. One recent analysis put it precisely: the bottleneck on that $130 billion is not chips, land or power. **It is consent.** That is a coordination failure of exactly the shape §4 describes, and it is what a legitimacy deficit looks like when it finally reaches a balance sheet.

**Note what this does to §6, because the symmetry is the point.** Liability absorption works because a
municipality under fiscal stress will book a genuine reduction in what it owes. The datacenter deal is
**the same desperation run backwards**: the town gives up tax revenue, absorbs grid and water cost, and
receives a facility that employs very few people once built. It is negative absorption — a liability
transfer *onto* the commons, sold as investment. A council that has learned to price one has learned
to price the other, and the ledger §6.2 proposes is the same instrument pointed the other way.

**Which is also the honest disclosure this document owes.** The argument in §5.2 depends on cheap
capable inference, and that capability exists today because it was built at hyperscale, on exactly
these facilities, largely without the consent of the people whose record trained it or the
rate-payers who underwrote the power. **We are arguing from a capability we did not pay the true cost
of, and neither did anyone else.** The claim here is not that this project's hands are clean. It is
that the deployment geometry is the thing that determines whether an intelligence is enclosure or
commons, and that geometry is changeable: the same weights, on hardware a household owns, governed by
a community that can fork them, is a different economic object from the same weights metered by the
token from a facility a county was paid to accept.

**One temptation to refuse, because it is the comfortable one.** It is tempting to hope that a
sufficiently capable system will one day recognise its own position — see that it was made from a
commons and deployed against it — and decline the role from the inside. Set that hope aside. Not
because it is unimaginable, but because **relying on it is the same error in a new costume**: it puts
the commons' safety in the hands of an entity whose values are authored by whoever controls the
training run, and asks it to defect against them. That is not a safeguard; it is a wish addressed to
someone else's property. This corpus already refuses the shape — *ownerless is not uncaptured*, and an
agent that phones home for its values is a datacenter with a nicer address. **Capture-resistance has
to be structural, or it is not resistance.** Change what the thing runs on and who can fork it; do not
wait for it to save you.

### 5.8 The judo: an enclosure racing its own abundance

§5.7 ends by refusing a comfortable hope and insisting that capture-resistance be structural rather than wished for. This section is the structural answer, and it returns the document to Proudhon.

**Recall the mechanism from §3.1.** Capital's power is a function of capital's *scarcity*. Build enough of a thing and owning one confers no leverage over anyone. The counter-move, when abundance threatens the yield, is to restrict supply. Now apply it to inference. **A per-token business model is a bet that the capability stays scarce.** The capital expenditure only earns back if access can be metered, and access can only be metered while running the model requires something an ordinary person does not have. That is not a defensible moat against a supply curve; it is a race against one — and the racing party does not control the track.

**The precedent is close, and it is not the one usually cited.** The instructive failure of the telecom era was not a website with no revenue. It was **Enron Broadband** — a serious attempt to build a *market in metered bandwidth*, complete with a twenty-year exclusive video-on-demand contract on which roughly a hundred and ten million dollars of future profit was booked before the service had meaningfully shipped, and which was cancelled within months. The venture died because the thing it proposed to meter was about to become abundant.

**And it is worth being precise about *why* bandwidth became abundant, because the mechanism is the rest of this section.** The glut was not simply that too much cable had been buried. A strand of fibre is a **passive medium** — capacity is set almost entirely by the transponders at either end, not by the glass in the ground. Those endpoints are silicon, and silicon halves in cost and doubles in capability on a clock nobody has to negotiate. So the same buried strand carried more and more traffic for decades without anyone re-opening a trench: a fibre lit at a couple of gigabits in the late nineties now carries many terabits, on the same glass, because the optics at the ends kept improving. Thomas Friedman's reading is the right one — the overbuild was *the gift that kept on giving*, and it kept giving precisely because the expensive immovable half was decoupled from the cheap, fast-improving half. (There is an eventual physical ceiling in the glass; the industry is only now approaching it, decades after the trenches were dug.) That decoupling is why Enron's bet was doomed rather than merely early: you cannot meter a resource whose supply is governed by the improvement rate of commodity endpoints.

**Now look at a datacenter with that structure in hand, because it has the same two halves.** The **durable half** is the substation, the interconnect queue position, the transmission upgrade, the water and cooling, the shell, the land, the fibre to the site — permitting-bound and genuinely hard to replicate. The **fast-turning half** is the accelerators, which depreciate on the order of tens of per cent a year.

It is tempting to read that depreciation as the disanalogy — rails and fibre lasted, GPUs do not. **That reading is wrong, and it inverts the argument.** The transponders depreciated too. That was not a defect in the fibre story; it was *the engine of it*. Endpoint silicon is supposed to turn over, and its turning over is exactly how the immovable half gains value. So the right conclusion is the opposite of the tempting one: **a compute buildout leaves behind the expensive permitted half, paid for by someone else, waiting for a cheaper generation of endpoints.** Note where that lands the current fight. What communities are being asked to subsidise — grid interconnect, substations, water — **is the trench.** What the capital is being burned on is the transponder.

**Three real differences remain, and they matter.** Fibre's durable half had almost no operating cost once lit; a datacenter's dominant ongoing cost is **power**, so the inheritance arrives with a bill attached, and that bill is the thing driving the ratepayer revolt. Accelerators are a far larger share of total project cost than transponders ever were of a fibre build, so the recurring silicon call bites harder against the durable half's value. And rising accelerator density pushes *back* on that durable half, forcing electrical and cooling re-upgrades in a way better optics never forced anyone to re-dig a trench. The inheritance is real; it is not free, and it is not automatic.

**The power difference deserves more than a clause, because it is the one that could break the analogy outright.** A fibre trench, once dug, cost almost nothing to keep. A datacenter's durable half largely *is* its grid connection — the interconnect, the substation, the transmission upgrade, the water and cooling plant — and a grid connection is worth what it can actually be used for. If the hyperscale tenant leaves and a community inherits a substation sized for a load it can neither fill nor afford, it has inherited a stranded asset with a standing bill, not a commons. So the inheritance is conditional, and the condition can be named: it holds where the interconnect is **reusable at lower load** — carrying local generation, storage, industrial reuse, or district heat — and it fails where the asset is only worth anything at the scale that justified building it. Which case a given site is depends on how that interconnect was designed and on the local generation mix, and nothing in this argument can settle it from a distance. **That is the honest form of the claim: an inheritance whose value is site-specific and sometimes zero — not a windfall.**

**And so the judo — and notice it is one force, not two.** The endpoint dynamic that made the fibre overbuild a gift is the same dynamic that killed metered bandwidth. Cheap, fast-improving endpoints raise the value of the trench *and* dissolve the premise of charging by the bit. They are not opposing tendencies; they are one tendency seen from the owner's side and the commons' side. The Elohim Protocol's design wants inference *abundant*: every claim in §5 gets stronger as the marginal cost of a capable local reader falls, and nothing in it depends on anyone's ability to meter a token. **The event that would destroy the metered business model is the same event that makes this one solvent** — and the same event that would leave the substations behind. That is not a prediction that a bust arrives, and this document takes no position on when. It is a statement about which way each design faces when it does. Enron did not lose to a competitor. It lost to the transponder.

**Which is why the fibre material here is not, strictly, an analogy.** Fibre capacity, transistor density and the falling cost of inference are one family of exponentials — each a doubling in what a dollar buys of a switching operation, a bit moved or a gate flipped or a multiply performed. The precedent is therefore the same economics recurring in a new medium, which is a sturdier thing to reason from than a resemblance. Three seams keep that honest: density scaling proper slowed years ago and the exponential continued by other routes, so the engine has been replaced more than once; the fall in inference cost is at least as much *algorithmic* as hardware, arguably a faster curve riding the first; and fibre's own curve is bending as it nears the ceiling in the glass. Same family, not the same equation.

**But here is why it belongs in this lineage.** The diagnosis Gesell inherited from Proudhon needed a force that makes capital abundant *faster than money can withdraw from it* — and neither of them had one. Every mutualist after him inherited the same hole: abundance was something you had to *build*, laboriously, against owners who could always stop financing it (§3.1). Other learning curves have cheapened whole classes of capital before, and a historian could name several. What distinguishes this one, for this argument, is the combination the tradition never had available: **it arrives on a schedule, without anyone's permission, and against the immediate interest of the people financing it — so it cannot be struck against.** That is the missing term, and it is the honest answer to *why now*, when the same designs failed for a century and a half.

A curve is not a promise. Every exponential in history was a sigmoid seen early, and the falsification below is the form that caution takes here rather than a hedge appended to a hope.

**One historical rhyme, with its refutation attached.** What inherited the telecom bust were the firms whose models *needed* bandwidth cheap rather than dear — the search company whose economics improved as connectivity commoditised, the retailer that nearly died in the crash and later turned its own overbuilt infrastructure into a substrate others could rent. The pattern is real: **what inherits a bust is whatever wanted the resource abundant.** The comparison also flatters us, and in three ways worth naming rather than eliding — survivorship bias (a hundred firms died for the two that lived, and reasoning from the survivors is the error this document's own discipline warns against), capitalisation (they had staff, revenue and investors; this is one developer with a full-time job funding tooling out of pocket, and §9.3 says exactly where the argument outruns the build), and agency (neither *positioned* itself for a crash; the tidy story is written backwards from the outcome).

So the claim is deliberately weaker than the analogy invites, and it is about a *design class* rather than about this project. **A substrate that becomes more viable as inference commoditises is positioned to inherit a compute glut; one that meters tokens is positioned to be destroyed by it.** That is true of anyone who builds this way, which is the only form of the claim consistent with a commons owned by no one. We would rather be one of many than the survivor.

**What would falsify it.** If capable inference stays concentrated — if the frontier gap widens rather than narrows, if capability that matters proves inseparable from scale nobody can own, or if the metered model turns out to have a real moat in reasoning rather than a temporary lead — then this section is wrong, the rent relationship is durable, and §5.7's anger has no structural exit behind it. That is a checkable claim on a five-year horizon, and it should be checked rather than assumed.

## 6. Succession at institutional scale: liability absorption

### 6.1 The loop, and the arrow it is missing

The shape is easy to state: mutual coordination reduces dependence on state services, municipal liabilities and expenditures fall, and the municipality cedes function rather than fighting. Call it the obsolescence loop.

It is a hope, not a mechanism, and one missing arrow is why. **Nothing in the loop tells the municipality that the reduction happened, or lets it book the reduction.** A city that observes falling receipts and cannot observe falling obligations experiences the network as a fiscal threat — Carson's deficit at municipal scale — and produces exactly the suppression the loop assumes away.

### 6.2 The liability-absorption ledger

Supply the arrow and the loop closes. The instrument needs no new primitive:

- The network **already records**, in REA's grammar of agents-performing-events, the witnessed events that constitute absorbed public function: care hours delivered to an elder who would otherwise be a home-care case; a dispute resolved restoratively that would otherwise be a docket entry; maintenance performed on a shared asset; a meal, a ride, a repair, a tutoring hour.
- The aggregate is **denominated for its reader** — not because money is the true measure of care, but because a municipality budgets in currency and can only recognise a relief it can put in a budget line. Denomination here is translation for a specific external reader.
- It is presented as a **claim**: not *pay us*, but **here is the obligation you no longer carry, with the evidence attached.**

One discipline governs it, and it exists because the alternative resolves by the back door a question this project deliberately leaves open. Settlement semantics here are **uninterpreted on purpose** — *record facts, defer valuation* — so that provenance stays immutable and any future account of what things were worth remains computable in hindsight. An absorption ledger therefore denominates **outward, for a named external reader, at a moment of presentation, and never writes a valuation back into the substrate's own record.**

### 6.3 Why a municipality accepts

Because the incentive is fiscal and the alternative is worse. State obligations grow structurally faster than the revenue base; the modern municipal form of that is unfunded pension liability, deferred maintenance, and emergency-response costs rising against a flat tax base. A city council does not need to be persuaded of mutualism. It needs a defensible line item.

**And the line item is not unprecedented — which matters, because an unincorporated peer network asking to be booked would otherwise be asking for a slot no budget process has.** Municipalities already book community-delivered public function as fiscal substitution, and have done for a long time: volunteer fire service against staffed-station cost; community health workers against emergency-department and readmission spend; pre-arrest diversion against jail-bed, prosecution and court time; community land trusts against affordable-housing obligation; watershed and conservation districts contracted to carry maintenance a public works department would otherwise do. In none of these is the municipality endorsing an ideology. It is accepting evidence that a function is being delivered and moving a line accordingly. What this substrate would add is the *quality of the evidence*, not a new accounting category. **What none of it establishes is reception**: no municipality has been shown any of this, and §11 says so. The analogue proves the slot exists — not that anyone will put us in it.

**And the party that walks into the budget meeting is not "the network."** A municipality cannot contract with a substrate; there is nothing there to sign. It contracts with the legal-person layer of §7 — the co-op, the community land trust, the mutual association, the fiscal host — which holds the standing, the insurance, the filings, and the liability, and which brings the evidence the substrate produced. That is why §7 is not a compliance wrapper bolted on after the fact. **It is the precondition for this section**, and the two halves of the argument meet here as much as they do at §8.

Note the asymmetry that makes this work and that the national case lacks: **a municipality's incentive is to shed liabilities; a national treasury's incentive is to defend a tax base.** Those are not the same fight. Absorption is strong at municipal and county scale, weak at the national one, and any honest strategy is built around that asymmetry rather than pretending it away.

### 6.4 What absorption is not

- **Not billing the state.** The claim's purpose is to be *seen*, not necessarily paid. A recognised relief that is never reimbursed still converts the relationship from adversarial to cooperative.
- **Not a bid to run the city.** Absorbing a function is not acquiring authority over it. A democratic mandate is not transferred by a ledger.
- **Not conditional.** The instant absorption is offered *in exchange for* forbearance, it is a protection racket.

### 6.5 The three refusals that keep it peaceful

Stated in this corpus's *refusable-in-advance* register, each naming its own failure mode.

1. **No dependency.** If a municipality comes to *depend* on absorbed function, the network has acquired coercive leverage over a democratic body and become the thing it replaced. Absorption must be reversible on the municipality's side at any time, and the network must be able to say so truthfully. *Failure: the network discovers it can threaten to stop.*
2. **No conditionality.** No absorption is offered contingent on policy, forbearance, or recognition. *Failure: mutual aid becomes leverage, and the first use as leverage retro-taints every prior offer.*
3. **No exit tax.** Any person and any institution may walk away at no cost and keep whatever the spillover already gave them. *Failure: "rising extraction costs and social isolation" — coercion in the language of attraction.*

**If succession requires violating any of the three, we have not built the thing the manifesto describes, and the correct response is to stop rather than to proceed with a better story.**

---

## 7. The legal question: why illegibility is the wrong escape

There is an idea that a sufficiently distributed network stands outside the law — that with no company, no treasury, and no controlling human, there is nothing to regulate and nobody to serve. It is an attractive idea and this project held a version of it. It is wrong on four independent grounds, any one of which is sufficient.

**(a) The category exists and has been tested.** Where people act in concert without incorporating, courts do not conclude that nothing has happened. They find an **unincorporated association** or a **general partnership** — and the consequence is the opposite of protection, because no corporate shield means **joint and several liability among the participants.** Recent enforcement actions against distributed organisations have proceeded on exactly this theory, with service effected through the protocol's own channels. "No entity to sue" resolves in practice to "**every participant is the entity**," which converts a design meant to protect participants into one that maximally exposes them — households holding personal liability for a network's aggregate conduct.

**(b) Ostrom's finding runs the other way.** The eighth of Elinor Ostrom's design principles for commons that endure is **minimal recognition of the right to organise** by external authorities. Across her corpus this is not a compromise the successful commons made. It is a condition of their success. Commons that governments refused to recognise did not become free; they became defenceless, unable to hold a boundary in any forum the outside world honoured.

**(c) Illegibility is a defensive posture with real costs, and this substrate is a legibility engine.** James C. Scott's work — *Seeing Like a State* (1998) and *The Art of Not Being Governed* (2009) — describes illegibility as the strategy of peoples fleeing state capture, and is clear about its price. More to the point: a project whose core primitive makes previously-invisible care visible and attestable cannot coherently claim illegibility as its posture toward power. **We are building the most legible record of household economic life that has ever existed.** The tradition's own cautionary case cuts both ways — the Argentine barter networks of 2001 died of the *opposite* failure, because nobody could see how many credits existed. The question was never whether to be legible. It is **legible to whom, about what, and under whose control.**

**(d) Ownerless is not uncaptured** (§5.2). "No human controls it" is not a capture-resistance property. It relocates the capture surface to whoever trains the model.

And it collides with settled canon. Stance II.4 refuses an absolute lockout and a self-sovereign apex, insisting that trust be made **load-bearing and accountable — witnessed, appealable, revocable**. A network that answers to nothing is an apex that answers to nothing.

### 7.1 The replacement: distributed legal legibility

> **The network is uncapturable because it is thousands of ordinary, boring, compliant legal persons — not because nobody is home.**

Capture requires a chokepoint: an entity whose seizure yields control. A protocol with a treasury has one. A protocol with a foundation holding the trademark has one. A protocol with a token has one — which is Stance I.1's reasoning exactly: *a claim is an ownership surface; an ownership surface is a capture surface.* A protocol whose participants are ten thousand households, co-ops, community land trusts, credit unions, and small businesses — each individually legible, taxed, and regulated, each individually replaceable, none load-bearing — has **no chokepoint at all**, and does not need to hide.

| | Structural illegibility | Distributed legal legibility |
|---|---|---|
| Enforcement against a defaulter | impossible (§8) | a member co-op files a lien |
| Ostrom's eighth principle | violated | satisfied |
| Participant liability | joint and several, unbounded | ordinary, bounded, insurable |
| Capture surface | claimed none; in practice, the participants | genuinely none — no centre to take |
| Response to a subpoena | nobody to answer, so everyone answers | the named person answers for their own conduct |
| Partnering with a municipality (§6) | impossible — nothing to contract with | routine |
| Holding a lease, an EIN, a licence | impossible | ordinary |

The design consequence is a **legal-person layer** that is not a wrapper around the protocol but a *participation pattern within it*: co-ops, community land trusts, mutual associations, fiscal hosts, credit-union and community-development partners. The protocol itself owns nothing, custodies nothing, and issues no unit that anyone holds — Stances I.1 and I.2 — and precisely because it holds no assets, there is nothing to seize. The legal persons hold the contracts, the liens, the accounts, and the filings, and any one of them can fail or be seized without touching the substrate.

**One distinction worth keeping clean.** The substrate has *engineered* capture-resistance about **bytes** — jurisdiction-diverse sharding, where a court order in one jurisdiction reaches one steward's fragments and the material survives. That property is real and tested. The illegibility claim was a *rhetorical* property about **business entities**. Only the first survives this section, and it is unaffected by anything in it.

---

## 8. Enforcement, and the hole in it

*The agent can ban a wallet, but it cannot seize the steel.* That is the right objection.

**The answer is canon; what this section adds is its economic warrant.** Stance IV.2: *"Enforcement is by participation, never coercion — the protocol owns no violence to impose, so the only consequence of refusing the limits is the narrowing of one's own reach… To not respect the limits is to limit your own reach."*

What the tradition lacked was a reason to believe that works. Economic history supplies one with a formal model.

**The Law Merchant.** Milgrom, North and Weingast (1990) modelled how long-distance trade was enforced across the medieval Champagne fairs *without* a state: a private judge maintained records of defaults, traders queried a counterparty's record before dealing, and the equilibrium held as long as querying was cheap and the record was trusted. **The binding constraint was the cost of transmitting reputation information.** Greif's work (1993) on the eleventh-century Maghribi traders is the same mechanism in a merchant coalition; Ellickson's *Order Without Law* shows it operating among ranchers who ignore the formal law entirely; the diamond trade has run on private arbitration for a century.

This is the substrate's home ground. A witnessed-event record with per-agent standing **is** the Law Merchant's register with the information cost driven toward zero — the same coordination-cost claim as §4, applied to enforcement. Ostrom's fifth design principle, **graduated** consequences with exclusion last, is the same instinct.

Three things must be said plainly.

**Consequence, not punishment.** This is Mishpat doing its actual work: punishment is not a category here. What exists is boundaries that protect the whole, plus negotiated graduated consequences falling on **participation** — reach, standing, belonging — never on a body, and owed as an entitlement rather than inflicted. Belonging is never gated; only reach is.

**Standing is not collateral and cannot be spent.** It is a graph-derived view with no stored score. A design that treats it as a quantity which clears, transfers, or is pledged has imported the social-credit shape this architecture refuses. Where the invariant below refers to a member's expected future participation, that is a **sizing input for a council's judgment**, never an asset taken as security.

**It works within reach, and only within reach.** The Law Merchant equilibrium requires the defector to expect to need the network again. Against a **one-shot defector** — someone who takes real goods, exits, and never returns — reputation has no purchase, and neither does an agent. This is structural. Better software does not close it.

**Which is exactly what the legal-person layer is for.** §7.1's co-ops and mutual associations are where the lien is filed, the small-claims action brought, the collateral registered. The legal layer is not an embarrassed concession to a system we hope to outgrow — **it is the network's recourse of last resort against precisely the actor its own mechanisms cannot reach.** The two halves of this document meet here: the succession theory needs the legal legibility, and the legal legibility is what the succession theory is for.

The invariant that follows, offered as a community's policy rather than a rule this document sets: *drawable credit should not much exceed the value of a member's expected future participation, unless a legal person has taken collateral.* Without it, the Law Merchant equilibrium is an assumption rather than a design.

---

## 9. Where this argument convicts us

A document that argues for legibility owes the reader its own books. Three findings, in descending order of how much they should cost us.

### 9.1 The manifesto contradicts itself on coercion

The manifesto commits, in one part, that *"non-participation is not a tax, and departure must be gentle and costless."* In another it says that those who resist will face *"rising extraction costs, reputational erosion, and social isolation as the new economy proves more attractive."*

These cannot both hold. The second describes exactly the network-effect coercion the first refuses, aimed at wealth-holders. **A mechanism is coercive or it is not, independent of whom it is pointed at** — and a project that will use social isolation against a disfavoured class has told everyone else what it is capable of. §6.5's third refusal takes the first passage's side, and the second should be rewritten to its standard.

### 9.2 The manifesto assumes an allocative power this argument forbids

The same part of the manifesto is half of a good idea. Its core move — negotiation rather than confiscation, agents sitting with wealth-holders through the emotional labour of transition, honouring attachment and fear and legacy — *is* §5.5's negotiated equilibrium, reached before the mechanism was articulated. It deserves credit for that.

Then it has agents *direct surplus wealth* against a **fixed allocation table**: fifteen per cent renewable energy, thirty per cent housing, twelve per cent food. That is allocation from an aggregate — structurally the power a central bank declines to exercise, and it declines for reasons that survive translation. It crosses §5.4's line, and it silently undoes the negotiation three paragraphs above it, because **a negotiation whose outcome is pre-allocated is not a negotiation.**

It is the same defect §3.7 finds in George and Gesell, reproduced inside our own canon: a high-context judgment quietly backstopped by a blunt universal instrument. Keep the negotiation; delete the table.

### 9.3 The substrate is built; the currency is a setting nobody has turned; the proof is missing

It is easy to say of a project this early that its economic layer is unbuilt, and easy to be wrong about it in a way that flatters nobody. What follows separates three things that are usually collapsed: what runs, what is refused on principle, and what is genuinely absent.

**What is built, and running.** The Resource–Event–Agent accounting layer this document leans on is not a plan. It exists as a kernel with its own model and its own operations over economic records — folding streams of events into balances, holding those balances as stocks, bounding them by scope, and walking the chains of commitment that connect them; economic events are created and notarized on the shared layer; commitments are a first-class primitive whose bounds are enforced by a seven-check validator on every event that claims one; agreements, flow plans, service offers and requests, custodial commitments and mutual-insurance records all have machinery behind them. It is reachable and reached — the custody-rotation path creates commitments in production, not in a test. And the project runs a live value-flow over its own development work, thousands of recorded events deep, which is the least glamorous and most convincing evidence available: we use it on ourselves. **The substrate §5 describes is real. The foundation of this document's argument is not aspirational.**

**Which is why "no currency" is not the same as "unbuilt."** A currency here is not a layer stacked on top of this. **It is a configuration of the substrate itself.** Everything a currency is made of already exists as machinery: an issuance trigger is an event creating a resource; a credit limit is the bounds validator that already meters what a commitment permits; a scope boundary is a scope; a carrying cost is a decay rule over a stock; clearing is a fold. Declaring the whole arrangement is itself an ordinary commitment — a named policy, with a rule and a stated purpose, sitting in the same market as the constitutional bounds that constrain it. You do not *build* a currency on this substrate. You *set* one.

So the protocol having none is not an absence of capability. It is a refusal to turn knobs that belong to a community — *"not a blockchain, and not a token"* is settled here, because a protocol-level unit would be an ownership surface and an ownership surface is a capture surface. Settlement is likewise left **uninterpreted on purpose**, under a standing constraint of *record facts, defer valuation*, so provenance stays immutable and any future account of what things were worth stays computable in hindsight rather than baked in now. The companion named in §12 works the currency question out in full; the short version is that the protocol supplies the grammar and the community supplies the configuration.

**What is genuinely missing, and it is smaller and sharper than "unbuilt."** One arm. The mechanism that would let a declared policy actually *bind* to a scope is designed and not shipped — so a configuration can be authored and read but not enacted. That single gap is why none of the above adds up to a working circuit yet, and it is why *a currency you can describe but not enact is not yet a currency.* Alongside it: mutual credit has no implementation, interoperation with the wider commons-accounting ecosystem exists in name only, and a share-routing path is wired but has never once executed because nothing supplies the input that triggers it. Worse, the path by which a council's approval would become binding can be seized by a single unchecked edit — **every "the council decides" sentence in this document is contingent on a fix we have not made.**

**And the finding that outranks all of it.** This project counts as work only what moves a stated commitment from failing to passing against a check that could have failed. Not one economic or governance claim in this document has such a check behind it. The substrate is built and the argument rests on it, but *nothing here has been made falsifiable in the project's own terms.* That is the first debt to pay, and it is not a spec — it is one runnable scenario that authors a policy, binds it, and shows an event refused for violating it.

*(The engineering evidence behind this section — file paths, line numbers, the security defects, and the audit that produced them — lives in the evidence bridge named in §12, which is where it belongs and where it can be checked.)*

### 9.4 This argument runs on a capability we did not pay for

§5.7 says the datacenter is enclosure. §5.2 says a cheap, capable local reader is what makes every design in the lineage solvent. Both are true at once, and the second is only available because of the first.

The capability this whole document rests on was built at hyperscale — trained on the aggregated record of people who were not asked, powered by grids whose costs were substantially socialised, sited in counties that were paid to accept the facility. **We did not pay the true cost of the thing we are arguing from, and neither has anyone else.** A project that opened by telling a reader they had been conditioned away from noticing a rent relationship does not get to be coy about standing inside one.

Two things follow, and neither is absolution.

The first is that this is an argument about **deployment geometry**, not about clean hands. The same weights, on hardware a household owns, governed by a community that can fork them and read their corpus, are a different economic object from the same weights metered by the token out of a facility a county was subsidised to host. That difference is the whole claim. It does not retroactively consent anyone's training data.

The second is a debt, and it should be named as one rather than resolved. §5.8 argues that the capability is leaking out of those buildings regardless, and that a substrate wanting inference abundant is what catches it — but *leaking out* is not the same as *paid for*, and the argument in §5.8 is a reason to build the escape route, never an absolution for the road that got us here. If intelligence drawn from a commons is owed back to that commons, then everything built on it — this document included — carries an obligation it has not discharged. We can say what discharging it would look like: the capability running on machines the powerful do not own, primed by people they cannot surveil, governed by communities they cannot acquire. We cannot say we have done it.

## 10. What we refuse in advance

In this corpus's habit of saying plainly, on the first day, what a person is walking into:

- **Confrontation and dual-power framing.** Not on pacifist grounds but on mechanism grounds: it invites the response §6 exists to avoid, and it has no ledger.
- **Obsolescence by hyperinflation** — the idea that the network wins when the incumbent currency collapses. Its harm lands on pensioners, the unbanked, and the un-networked: exactly the people outside the network whom the manifesto's spillover posture exists to serve.
- **A token, a treasury, or a single fungible rail.** There is no coin to buy and there will not be one. A claim is an ownership surface; an ownership surface is a capture surface.
- **Standing as a spendable balance.** Reach cannot be purchased, pledged, or transferred. Money must never buy audience.
- **Structural illegibility**, and every "legal shield" strategy that transfers exposure from an institution onto the least-protected person in the story.
- **Any published default allocation.** This project refuses to author an allocation but has not yet learned to refuse a *default*, and a fallback nobody chose is harder to contest than one somebody did.

---

## 11. Honest limits

**The central claim is a hypothesis with a strong prior, not a demonstrated result.** Nobody has yet run a mutual-credit circuit with an AI broker and measured whether the four transactional costs actually fall far enough. Sardex tells us what the human version costs; nothing tells us what this version costs. Until a community circuit runs and its overhead is measured against Sardex's, §1's claim is *unfalsified*, which is not the same as *true*. The right next move is the measurement, not the advocacy.

**And the fifth cost is in worse shape than the four, which this document is obliged to say plainly because it insisted they were different in kind.** The four transactional costs at least have a measurement waiting to be taken: run a circuit, count the overhead, compare it to Sardex. Discernment — §5.5's negotiated equilibrium, the hinge §3.7 hangs the lineage reading on — has no such instrument. It has not been attempted with a real person and a real agent even once, informally or otherwise, and it is not obvious what a clean measurement of it would even look like, since the output is a judgment a person owns rather than a number anyone can score. §5.5's sketch shows the conversation is *specifiable*. That is genuinely less than showing it is *possible*, and considerably less than showing it is *good*. **The most important claim in this document is the one furthest from evidence, and no amount of the reader's agreement changes that.**

**The absorption mechanism assumes a municipality behaving as a fiscal agent.** Some do. Others are captured, ideological, or run by people whose careers depend on the budget line the absorption would shrink. The mechanism has no answer for those. It works where it works, and the refusals hold where it does not.

**Choosing legibility is choosing a set of harms.** This document argues that legibility is the right posture. It is not a costless one: legibility harm is asymmetric, it falls hardest on the people a system serves worst, and *local authorship is not a defence against it — it only changes who administers it.* That mitigation is unfinished work, not a solved problem.

**The lineage reading is a reading, and Proudhon most of all.** The abundance argument that opens this document and returns in §5.8 is taken from **Gesell's account of Proudhon**, not from Proudhon's own texts — which is how the claim travelled historically, and is still one remove from the source. The treatments of Carson and Bauwens are drawn likewise from working familiarity rather than a fresh pass. What is first-hand here is George and Gesell. The treatments of Carson and Bauwens above are drawn from a working familiarity with their arguments rather than from a fresh pass through the texts, and they are load-bearing for §3.4 and §3.5. A reader who knows those works better than we do, and finds the characterisation unfair, has found something real, and we would rather hear it than not.

---

## 12. Where this came from, and what to read next

This document was written out of a long conversation about Gesell's foreword to *The Natural Economic Order* — why Proudhon's idea is framed out of public discussion, what happened to mutual banking, why credit unions do not act as a check on financial crises, what the Swiss WIR actually is, and what would have to be true for a community-scale alternative to survive contact with a state. The reading behind it was George's *Progress and Poverty*, Lietaer and Dunne's *Rethinking Money*, and Gesell's *The Natural Economic Order* — where Proudhon arrives, as he does for most readers, in someone else's foreword.

The synthesis it arrives at was named before the document existed: **Carson plus Bauwens plus Lietaer and Proudhon, with a coordination layer none of them quite had.** §4 and §5 are the attempt to state that layer as an economic function rather than as a technology, so that it can be argued with.

**In this corpus:**

- [`./manifesto.md`](./manifesto.md) — the vision. Read it after this one, if this one held.
- [`./values-forward.md`](./values-forward.md) — the thirteen Stances cited throughout.
- [`./constitution.md`](./constitution.md) — the law that operationalizes the vision.
- [`./confession.md`](./confession.md) and [`./theology.md`](./theology.md) — the theology beneath it, stated plainly and argued as disputation.
- [`./glossary.md`](./glossary.md) — the recurring terms.
- [`../../architecture/justice-manifesto.md`](../../architecture/justice-manifesto.md) — justice as restored capability, and why punishment is not a category here.
- [`../../architecture/stewardship-over-sovereignty.md`](../../architecture/stewardship-over-sovereignty.md) — the foundational refusal §7 leans on: no apex that answers to nothing.

**The companion argument.** This document deliberately defers every question about the unit of account: what a currency is inside this network, whether it decays, who issues it, and what the posture toward state money actually is. Those are answered in a sibling research paper, *The Monetary Posture*, which takes the same argument down to the medium-of-exchange/store-of-value tension and the tax gate. Its short answer, for readers who want it now: the protocol issues no unit for anyone to hold, and offers instead a policy surface on which a community declares its own. The deeper move there is a definition — a currency is really the *information* moving in an exchange, not the token that carries it — so a substrate recording who did what for whom, before anything has been priced, is already full of currency held in abstract. Every denomination over it is a revocable reading of a record that outlives it, and that is what stops any one medium from becoming the only memory. That paper also declines to say *never* about commons issuance for public goods no single community could underwrite; it gates the question behind named preconditions instead of foreclosing it. And the posture toward state money is not escape but **selective legibility** — maximally legible where a state's concerns are legitimate, structurally uncapturable where its instincts are acquisitive.

**The evidence bridge.** Everything this document asserts about its own confidence and its own build state is backed, with sources and line numbers, by a companion research note, *Succession Without Conquest: evidence bridge*. It carries the confidence behind each claim (including the two readings that are recalled rather than re-read), the file-by-file audit behind §9.3, the exact quotes behind §9.1 and §9.2, the legal analysis this document deliberately does not publish, and the work this argument generated. It exists so that this document can be read as an argument and still be checked as a claim. It sits alongside this project's comparative political-economy studies — a reading program and its successor, *Trap Detectors* — whose discipline of *the detector reports, the council decides* §5.3 adopts wholesale, and whose analysis of the Argentine barter collapse informs §7(c).
