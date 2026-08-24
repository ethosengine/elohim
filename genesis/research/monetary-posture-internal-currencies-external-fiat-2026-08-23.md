---
title: "The Monetary Posture — currencies as Mishpat policy inward, selective legibility and liability absorption outward"
id: monetary-posture-internal-currencies-external-fiat-2026-08-23
status: Capture
date: 2026-08-23
sovereignty-frame: bridge-legibility
---

> **Frame note.** This paper's sovereignty language sits in the **bridge-legibility** frame: it
> concerns how a participant is made legible to *external* sovereigns — tax authorities, financial
> regulators, municipalities — while head, ceiling, and authority continue to flow from the commons
> governance pool, never from the external reader. Where it quotes "monetary sovereignty" it is
> naming a state's frame in order to reason about the bridge to it, not adopting an apex.
> Elohim's identity floor is *imago dei*, backstopped by community: *"self-sovereignty is a floor of
> dignity, not a ceiling of authority"* (Stance IV.1); *"there is no self-sovereign apex that answers
> to nothing"* (Stance II.4). Canon anchor: `genesis/docs/architecture/stewardship-over-sovereignty.md`.
> The one place this paper argues *against* a sovereignty ontology outright is §3.2, where the
> crypto-monetary position — that a currency escapes a state by having no issuer — is refused on the
> same grounds its sibling refuses structural illegibility.

**Grading legend** (this directory carries two; this is `.epr-meta`'s): ✅ verified-in-source this
pass · ◐ single-source or recalled-canonical, not re-verified this pass · ⚠ verify at source before
load-bearing. Repository file:line pointers verified against
`fix/doorway-breaker-trial-theft-and-apps-extraction-herd` on **2026-08-23**. The token plane in
particular moves week to week — §5 records where a prior audit has already gone stale.

# The Monetary Posture

**Sibling to [Succession Without Conquest](epr:succession)** — which graduated into canon as a
manifesto companion, with its grading and build-state audit kept in the
[evidence bridge](epr:succession-without-conquest-mutualist-lineage-2026-08-23) beside this paper —
which argues that the mutualist tradition was priced out rather than refuted, and that succession runs
on two unilateral tracks — liability absorption at institutional scale, and the negotiated personal
equilibrium at the scale of a life. This paper takes the same argument down to the two questions that
document deliberately deferred: **what is this protocol's posture toward currency, inward?** and
**what is its posture toward state fiat, outward?**

It exists because the manifesto opens with a promissory note it never discharges. Part I's economic
indictment is titled *"The Poverty of Currency"* and names five defects — currencies carry no values,
have no natural limits, fail to reward care, concentrate rather than circulate, are blind to
externalities — then says: *"Modern computation could enable currencies that decay, that carry
values, that reward pro-social behavior."* ✅ Nothing in the corpus then says what the protocol's
actual posture toward currency **is**. That gap has been filled, badly and repeatedly, by artifacts
that contradict each other and contradict canon (§5).

**For a reader arriving cold**, the vocabulary is the same as the sibling paper's: a **witnessed
event** is one concrete act, attested by its participants and notarized on the shared tamper-evident
substrate; an **elohim** is a per-community AI agent that reads the substrate; a **council** (qahal)
is the deliberative body that decides — *the council steers, the detector reports*. **Mishpat** is
the justice pillar, where justice is restored capability and **punishment is not a category**.
**REA** (Resource–Event–Agent) is the accounting model. **Reach** is earned audience, gated by
**standing**, and is never a balance that can be spent — it is a graph-derived view with no stored
score, so it cannot be transferred or pledged. An **EPR** is the protocol's addressable unit of
content-plus-context; `governs_epr` names the scope a policy applies to. A **Commitment** (capital C)
is a notarized record of a bounded, revocable obligation or delegation of authority. The **Stances**
are thirteen numbered positions in `values-forward.md` recording what this project has settled; they
are cited here as binding. `epr:` links are content-addressed document IDs that resolve to sibling
documents in this directory.

**A few further terms recur from the wider corpus**, and are defined here rather than left to a
sibling, because this paper leans on all of them. A **lens** is a named school's reading of a rule — a
deterministic `rule` plus a declared `telos`, governing a scope, forkable and supersedable — and it is
the mechanism on which a currency is declared (§2.1). The **deterministic floor** and the **elohim
ceiling** are the two decision layers: the floor is mechanical, runs with no AI and no network, and is
identical for everyone; the ceiling reads a particular situation in context and proposes — *never a
computed payout at the ceiling, never judgment at the floor*. **Friction-gradient limitarianism** is
the position that accumulation past a threshold should meet steadily rising friction rather than a
hard cap, so a limit is a gradient a person negotiates rather than a wall that rewards evasion. A
**Category-B projection** is data the substrate holds but does not notarize — a local view, not a
DHT-anchored fact — and an **Operational-C shape** is a quantity recomputed on read from its
underlying events rather than stored as an authoritative balance; both are borrowed from the
[Hypha survey](epr:hypha-dao-autonomous-collectives-cross-pollination-2026-06-24), and both matter
here because a stored balance is exactly the shape N4 refuses. The **Amplified set** is the corpus's
class of detectors judged worth raising to standing attention rather than left as passive measures.
An **EAE** is an *Elohim Autonomous Entity*, the protocol's model of a firm-like collective, and
**middot** is the measure primitive its detectors compose with.

**And for a reader arriving from the protocol side rather than the monetary side**, the historical
and technical vocabulary this paper leans on, in one place:

- **WIR** — *Wirtschaftsring*, a Swiss business-to-business mutual-credit network founded 1934 and
  still operating; members clear trade in a unit held at parity with the franc, usually settling
  split (part francs, part WIR).
- **Trueque** — the Argentine barter network that scaled to millions during the 2001 crisis and
  collapsed, principally because units were issued on account creation rather than on settled trade.
- **Sardex** — a Sardinian mutual-credit network (2009–) whose distinguishing feature is human
  brokers who match trades and assess credit lines.
- **Fureai Kippu** — a Japanese time-banking system in which hours of elder care earn credits
  transferable to a relative in another region.
- **Chiemgauer** — a Bavarian regional currency (2003–) carrying demurrage at 2% per quarter.
- **C3** — *Commercial Credit Circuit*, a mutual-credit design backed by insured invoices, deployed
  in Uruguay, Brazil and Honduras.
- **LETS** — *Local Exchange Trading System*, the most numerous category of household-scale
  mutual-credit experiments (UK/Canada/Australia, 1980s–). Structurally the closest analogue to what
  §2 proposes, and the most instructive: LETS schemes rarely collapse dramatically, they **fade** —
  through participant fatigue, thin supply on the offer side, and the administrative burden falling
  on a few unpaid organisers. ◐
- **demurrage** — a carrying cost on holding money, so that currency is not a costless store of
  value. Gesell's *Freigeld*; its land counterpart is *Freiland*.
- **seigniorage** — the gain accruing to whoever issues money: the difference between a unit's
  exchange value and its cost of issue.
- **numéraire** — the single unit everything else is priced against.
- **chartalism** — the position that a currency is demanded because obligations to the state are
  denominated in it; "taxes drive money."
- **the Cantillon effect** — that new money enters an economy at particular points, so whoever is
  nearest the issuance spends before prices adjust.
- **prosumidor** — the Trueque norm requiring members to both produce and consume.
- **CDFI / CUSO** — a US Community Development Financial Institution; a Credit Union Service
  Organization. **MSB** — Money Services Business, a US regulatory classification. **CBDC** — central
  bank digital currency. **OFAC** — the US sanctions authority.
- **waqf** — the Islamic endowment: property permanently dedicated to a purpose, with no owner,
  administered by a trustee. A thousand years of case law on entities that outlive their founders.

---

## 0. Both postures, stated first

**Inward.** The protocol has no currency, and will issue no unit for anyone to hold. But the reason is not that a currency
layer is missing — it is that **a currency is a configurable property of the substrate itself**, and
the configuring belongs to a community rather than to the protocol. Every part a currency is made of
already exists as machinery: an issuance trigger is an event creating a resource; a credit limit is
the bounds validator that already meters what a commitment permits; a scope boundary is a scope; a
carrying cost is a decay rule over a stock; clearing is a fold. What the protocol supplies is the
**policy surface on which a community declares its own settings** — and that surface already ships,
as the `author-lens` Commitment (§2.1). A currency here is a *Mishpat lens*: a named school's reading, with a
deterministic `rule` and a declared `telos`, governing a scope, forkable, supersedable, and plural by
construction. Constitutional bounds arrive as lenses with `role: floor` or `role: ceiling`. **Zero new
DHT entry types; zero new commitment actions.** The protocol supplies the grammar and the detectors;
the community supplies the currency; the council decides.

**Outward.** The posture is **selective legibility plus liability absorption**, and it explicitly
refuses both available naive positions — that the network escapes the state by being illegible, and
that it defeats the state by making fiat superfluous. The tax gate is not a wall to route around; it
is **the bridge**, and canon already calls money exactly that (Stance I.4: *"Money and private
property are the bridge"*). ✅ The network's move is to be *more* legible than the incumbent can be on
its own about the things a state legitimately needs to know, and to have *no capturable centre* on
the things it does not. What the AI-mediated commons absorbs is not the state's authority but its
**concerns** — measurement, risk detection, dispute resolution, needs assessment, public-good
maintenance — offered as better evidence than the state can collect alone, and offered freely.

**Between them sits the tension every monetary authority carries and none can name.** Money's
medium-of-exchange and store-of-value functions pull against each other structurally; a central bank
holds them in permanent adversarial tension at the apex, with one instrument, in a technical register
that forbids saying whose interest is being served this quarter. **We do not resolve that tension — we
decompose it**, giving each function its own lens, its own `telos`, and its own declared Mishpat limit,
of which the sharpest is: **preservation is legitimate; accretion is not** (§2.2).

**The two postures are one refusal seen from two sides.** Inward, no protocol-level unit *that anyone
holds*, because a holdable unit is an ownership surface and an ownership surface is a capture surface.
Outward, no protocol-level counterparty, because a counterparty is a chokepoint.

**But "empty" is the wrong word for it, and an earlier pass of this paper used it wrongly.** What the
protocol ships is not an absence — it is an elaborate *grammar*: a lens mechanism with a validator, a
bounds validator, supersession, decay over stocks, and REA accounting, with no unit, no issuer, no
treasury, and no exchange rate. And beneath the grammar sits the record itself — un-denominated,
recorded before anything has been priced, and outliving every denomination read over it. That record,
not the absence of a unit, is where this paper locates capture resistance: a denomination here can
always be re-read, and never becomes the only memory. **§2.9 makes that argument, states what it does
*not* reach — one medium can still become the one everyone uses — and is where a plural-currency
facilitator answers for itself.** **The protocol holds no unit and issues nothing
today; it is not, and must not claim to be, permanently incapable of issuance.**

**What this paper does *not* do, deliberately.** It does not propose a numéraire, and it does not
resolve settlement semantics. The standing operator decision is that the substrate stays *agnostic to
the measure applied*, and settlement is **deliberately uninterpreted** under the design constraint
*record facts, defer valuation* — keeping provenance immutable so any future constitutional
settlement story remains retroactively computable. ✅ That refusal is the design, not a gap. But §2.7
records the mirror failure, which is live in our tree today: **refusing to author a rate while
shipping a default one.**

---

## 1. What canon already settles, so this paper does not re-litigate it

Six positions are already decided. A monetary posture that reopens any of them is not a posture but a
proposal to change canon.

**1. The issuance of money is common inheritance.** Stance I.4 lists what belongs to everyone —
non-reproducible, positional value — and names *"the issuance of currency and credit"* explicitly,
alongside natural monopolies, network effects, open protocols, and knowledge built on prior
knowledge. The sorting test is reproducibility. The refusable-in-advance clause is blunt: *"the
unearned increment — the rent of land, the toll of a network, the enclosure of shared knowledge,
**the private issuance of money** — is common inheritance, and it graduates to a commons owned by no
one."* ✅

**Consequence, and it is the strongest single constraint in this paper:** seigniorage is not the
protocol's to take, and it is not a community's to privatise either. Any circuit whose issuance
accrues a gain must return that gain to the floor on a declared schedule, or it has enclosed the
commons at exactly the point canon named.

**2. Not a blockchain, and not a token.** Stance I.2, with its reasoning intact: *"A global chain
re-centralizes what it claims to distribute: one canonical ledger, one consensus everyone must join,
one asset whose price becomes the system's true objective function… REA/ValueFlows lets us build
**many currencies-as-remembering** — care, stewardship, ecological restoration made visible — rather
than one coin that flattens them all into price."* And: *"There is no coin to buy… and there will not
be one, by design."* ✅

**3. Money is a technology of memory.** `values-in-the-machine.md`: *"Money began as a way to
**remember obligation** across more relationships than a single mind could hold… Wealth is congealed
obligation."* The plural term of art in canon is **current-sees** — *"we can build many current-sees,
each shaped to make a particular kind of flow visible and worth tending."* The barter-origin story is
explicitly rejected. ✅ Stance III.1 puts it as a design instruction: *"Money was always a technology
for remembering obligation; we are returning it to that purpose rather than bolting kindness onto an
attention machine."* ✅

**4. Shefa is not a currency system.** The subject-home gospel says it in one line:
*"Shefa is value accounting — tracking what was produced, consumed, transferred, and by whom. It is
not a currency system."* (`elohim/sdk/domains/shefa/CLAUDE.md:24`) ✅

**5. Enforcement is by participation, never coercion.** Stance IV.2: *"the protocol owns no violence
to impose, so the only consequence of refusing the limits is the narrowing of one's own reach…
**To not respect the limits is to limit your own reach.**"* ✅ This is the credit-default answer, and
the sibling paper supplies its economic warrant.

**6. Return is graduation, never seizure.** Stance I.4: *"This is graduation, not confiscation: a
negotiated schedule, never a seizure."* And between labour and the common inheritance sits *"not a
wall but a **negotiated gradient of subsidy** that respects each person's unique and unequal
capacities."* ✅ Note what canon does **not** contain: the word *taxation* appears nowhere in the
eight spine documents, and Stance I.4 names and declines UBI ("redistributes *after* the fact") and
limitarianism-as-cap ("caps the top but leaves the floor to charity") by name. The protocol does not
have a tax. It has a practice.

---

## 2. The internal posture — currencies as Mishpat policy

### 2.1 A currency is a configuration, declared as a lens — and the lens already ships

**You do not build a currency on this substrate; you set one.** The mechanism does not need inventing. The shipped, mechanical meaning of a "Mishpat policy" is a
**lens**: a `Mishpat::Commitment` with `action: "author-lens"`, whose entire concept lives in
`payload_json`. ✅ Adding one was DNA-hash-neutral — zero integrity-struct change, coordinator
hot-swap via `update_coordinators`.

A lens payload requires **six fields**, validated at
`elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs:557-583`:

| Field | Meaning | What a currency circuit puts here |
|---|---|---|
| `action` | `author-lens` | — |
| `governs_epr` | the scope this lens governs | the collective / place / corpus the circuit serves |
| `school` | whose reading this is | `gesellian-demurrage`, `wir-clearing`, `care-hours`, `prosumidor-strict` |
| `role` | `lens` \| `floor` \| `ceiling` | `lens` for a community circuit; `floor`/`ceiling` for constitutional bounds |
| `rule` | an object — the deterministic predicate, *"the teeth"* | issuance trigger, decay, credit limit, scope boundary |
| `telos` | an object — what it steers toward | what this circuit is *for*, in the community's own words |

`role` being a closed enum of `{lens, floor, ceiling}` is the whole architecture of this paper in
three words. **A plain lens is one school's reading of what a currency should do here. A floor or a
ceiling is a constitutional bound.** Plurality lives at `lens`; the never-rules live at `floor` and
`ceiling`; and because both are the same entry shape, a community can read its own bounds in the same
market where it reads its options.

The projection ships end-to-end: a `lenses` SQLite table (cid PK = Commitment `entry_hash`,
`governs_epr`, `school`, `role`, `rule_json`, `telos_json`, `version_parent`, `revoked_at`,
`dht_anchor_hash`) served over `GET /api/v1/epr/:scope/lens-market`, and **a NULL `dht_anchor_hash`
fail-closes the lens out of the market** — an unnotarized policy cannot govern. ✅

Three properties follow for free, and each is a monetary property the tradition had to build
institutions to get:

- **Supersession instead of amendment.** Commitments are immutable by validation — *"create a new
  Commitment to supersede"* ✅ — and lenses carry `version_parent`. A currency's rule change is a new
  lens citing its parent, so the circuit's monetary history is a readable chain rather than a mutable
  setting. The WIR dropped demurrage in 1948; under this shape that would be a superseding lens with
  an author, a date, and a parent, not a silent parameter edit. **And the substance of that change
  deserves more than an illustration, because it cuts against §2.2.** The WIR's most successful
  decades came *after* it dropped the Gesellian carry cost, settling into a clearing network whose
  discipline came from credit assessment rather than from decay — a real datum against treating
  demurrage as the central instrument. Stodder's counter-cyclicality result, the strongest empirical
  finding the WIR has produced, is a finding about a *post-demurrage* WIR. ◐ Read honestly, the WIR
  supports **plural, declared, supersedable monetary rules** — which is this paper's claim — and is at
  best neutral on demurrage itself, which is why §2.3 carries it as one knob among five rather than as
  the answer.
- **Revocation, not deletion.** `revoked_at` is a column; a withdrawn policy remains legible.
- **Plurality without forking the network.** Multiple lenses may govern the same scope from different
  schools. That is the whole design of the lens market, and it is why "which currency does this
  community use" is not a question the protocol answers.

**The gap, stated because it is the load-bearing one.** `binds-policy` — the declaration of *which*
lens governs an EPR (pin / latest / range, one live binding per `(epr_scope, school)`) — is fully
designed and **not shipped**: the coordinator's dispatch has no `binds-policy` arm, so such a
commitment is hard-rejected as "unhandled action." ✅ Lenses can be *authored* and *read* today;
nothing yet declares one *binding*. A currency you can propose but not enact is not yet a currency.
This is §6's first take.

### 2.2 The three functions, and the tension nobody at the apex is allowed to name

This is the section a monetary posture is obliged to write, and the one most such documents skip.

**The tension.** Money classically does three jobs — **medium of exchange**, **store of value**, and
**unit of account** — and the first two pull against each other structurally, not accidentally. A
perfect store of value is a *defective* medium of exchange, because a thing that holds its value at
no cost can wait indefinitely — and that waiting is what the sibling paper calls **the strike**: the
structural power of a costless store of value to withdraw from investment and simply hold, which
manufactures the very scarcity that restores its own yield, with nobody needing to conspire (sibling
§2.1). A perfect medium of exchange
is a poor store of value, because everything that forces circulation — a carry cost, a stock limit,
inflation — degrades holding. **Gesell's sharpest single observation is that money's zero cost
of carry gives it an unearned advantage over every good it is exchanged against**, and demurrage is
not an attack on thrift but the removal of that advantage. ◐

**How the incumbent handles it: adversarially, at the apex, and unnamed.** The Federal Reserve's dual
mandate — maximum employment and price stability — *is* this tension, carried by a single institution
with essentially one instrument. Price stability serves the store-of-value function: it protects
creditors, savers, bondholders, and the real value of existing claims. Maximum employment serves the
medium-of-exchange function: circulation, velocity, real activity, trade that clears. One policy rate
must serve both, so the institution is permanently trading one constituency against another **while
describing the trade as a technical optimisation.** The Phillips curve is that conflict wearing a lab
coat.

Two consequences follow, and the corpus should say both plainly.

First, **the trade has a direction.** Resolving the tension toward price stability is resolving it
toward existing holders — toward creditors over debtors, toward assets over wages, toward the old
over the young. That is not a conspiracy and does not require anyone's bad faith; it is what happens
when an unnamed tension is adjudicated by an institution whose mandate is written in the vocabulary
of one side. It is also, from the monetary direction, exactly the generational transfer the sibling
paper's §4.6 arrives at from the housing direction. ⚠

Second, **the tension is held at the apex where it cannot be deliberated.** A central bank cannot say
"this quarter we are choosing savers over workers" without becoming the political actor it is
structured to deny being — which the Fed's own repeated line, that it has blunt tools and Congress
holds the rest, concedes honestly. So the most consequential values conflict in the monetary system
is resolved continuously, by an unelected body, in a technical register, with no surface on which the
choice can be named as a choice.

**Our posture: do not resolve the tension — decompose it, and declare each function's limit.**

The forced fusion of the three functions into one asset is a historical artifact of *physical* money:
one object in a pocket had to do every job. Once money is a record — and canon is explicit that
*"money began as a way to remember obligation"* — the fusion is optional. Stance I.2 says the same
thing from the other end: *"many currencies-as-remembering… rather than one coin that flattens them
all into price."* ✅ **The flattening *is* the forced fusion.**

So each function becomes a lens with its own `telos` and its own declared limit:

| Function | `telos` | The Mishpat limit | The failure if unlimited |
|---|---|---|---|
| **Medium of exchange** | that trade clears; circulation | must not become a store of value — a carry cost or a stock limit is legitimate here and nowhere else | hoarding → the strike → manufactured scarcity |
| **Store of value** | that a person may carry value across time | **may preserve; may not accrete as a claim on others' future labour** | rent — the unearned increment |
| **Unit of account** | commensuration, for a stated purpose | must declare its scope and never silently become universal | the accidental numéraire (§2.7); care flattened into price |

**The store-of-value limit is the sharp one, and it needs saying in one line.**

> **Preservation is legitimate. Accretion is not.**

A person may hold value across time — for old age, for a child, for a house, for a hard year. That is
prudence, and the protocol has no quarrel with it whatsoever; a posture that treated saving as
suspect would fail every household it exists to serve. What is not legitimate is a store of value
that **grows by virtue of being held**, because growth without work is a claim on the future produce
of others — the unearned increment, which Stance I.4 has already classified as common inheritance. ✅

This is George's produce-of-labour / unearned-increment line applied to **time** rather than to
**place**, and it is the cut the whole lineage was reaching for. The distinction is not *saving good,
saving bad*. It is **saving versus rent** — and the tradition always had the cut. What it lacked was
an instrument fine enough to draw it per person, which is §2.8's subject and the sibling paper's §4.5.

**Where the floor/ceiling structure does the work.** A widow's reserve and a speculator's position can
be the same number, and no rule distinguishes them.

- **Deterministic floor (mechanical):** a circuit may declare a carry cost, a stock limit, an issuance
  trigger. These are rules, they hold with no AI and no network, and they are the same for everyone.
- **Elohim ceiling (discerning):** whether *this* holding, in *this* life, is prudence or accumulation
  is irreducibly contextual. **Never a computed payout at the ceiling, never judgment at the floor.** ✅

**Why this is available to us and not to the Fed.** Not because we are
cleverer. Because we are not forced to fuse. A central bank administers one currency for one polity
and must therefore resolve the tension with one instrument — a genuine structural constraint, not a
failure of nerve. A plural-currency substrate can let a circulation instrument be a circulation
instrument and a preservation instrument be a preservation instrument, with different rules,
different scopes, and different councils. **The tension is not resolved; it is decomposed**, which is
what Lietaer's yin/yang typology was reaching for (◐) and what monetary monoculture makes impossible
by construction.

And the thing that changes when it is decomposed is not efficiency. It is that **the conflict becomes
deliberable.** A community that declares a demurrage on its clearing unit and no accretion on its
preservation instrument has made a values choice, in public, with an author, a date, and a
supersession path — the thing an apex institution structurally cannot do.

**Three honest limits.**

1. **Decomposition opens an arbitrage seam**, and it is Keynes' critique of Gesell arriving inside our
   own design: if the circulation unit decays and the preservation instrument does not, value flows to
   the preservation instrument. The answer is not to block the flow — it is that **if preserved value
   cannot earn (the accretion limit) and cannot buy standing (N3), the flight to it is harmless.** It
   is just saving. Gesell needed Freiland because in his world the escape hatch *did* earn; ours is
   closed by the limit rather than by a second instrument.

   **The obvious rebuttal deserves a direct answer: if the preservation instrument transfers, it *is*
   a medium of exchange, and the decomposition collapses into one asset with two names.** The answer
   is that under N6 it does not transfer as a rail. A preservation holding is a claim against the
   circuit that issued it, and turning it back into purchasing power is a *settlement* — a redemption
   against that circuit, or a bilateral agreement with another — not an endorsement to a third party
   at a market price. That is the difference between a bank deposit and a banknote. It is also a
   genuine cost to the holder, and this paper should not pretend otherwise: the instrument is **less
   liquid on purpose**, and the illiquidity is precisely what keeps it from becoming the thing it was
   decomposed away from. Whether households will accept that trade is an open empirical question, and
   it is the sharpest one this posture faces. ⚠
2. **A preservation instrument that cannot accrete still concentrates**, slowly, through inheritance
   and unequal capacity to save. That is a real residue, and it is where friction-gradient
   limitarianism and the negotiated equilibrium of §2.8 do the remaining work — not the monetary
   design.
3. **None of this ships.** §5 applies in full: there is no unit of account, no issuance policy, and no
   conservation invariant anywhere in the tree.

### 2.3 The five knobs, expressed as `rule` and `telos`

Every currency in Lietaer's corpus differs on the same five parameters. Written as a lens `rule`,
they become a declared, diffable, supersedable object rather than institutional folklore:

| Knob | The question | Historical settings |
|---|---|---|
| **Issuance trigger** | what event creates a unit? | WIR: a member's purchase creates a matched debit/credit. Fureai Kippu: an hour of care. Trueque: **account creation** — the failure. Chiemgauer: fiat purchase at par. |
| **Backing / redemption** | what, if anything, is it claimable against? | WIR: nothing — it clears. C3: an insured invoice. Chiemgauer: euros at 95%. Care-hours: nothing. |
| **Carry cost** | does holding it cost? | Gesell/Wörgl: ~1%/month. Chiemgauer: 2%/quarter. WIR: dropped 1948. Most: none. |
| **Scope boundary** | who may hold and transfer it, and does it leave? | WIR: Swiss members. Sardex: Sardinian businesses. Fureai Kippu: transferable to a parent in another region — *the interesting case*. |
| **Clearing + credit limit** | how do books close, and who decides how far negative you may go? | WIR: a bank underwrites. Sardex: brokers assess. Trueque: nobody — the failure. |

A sixth, which the tradition treats as an afterthought and this protocol cannot: **the consequence of
default**. Canon fixes it (Stance IV.2 — the narrowing of one's own reach) and forbids the punitive
register. The circuit's lens declares *how graduated*, never *whether punitive*.

**Do not read this table as a menu the protocol ships.** It is the shape a community's declaration
takes. The protocol's contribution is that the declaration is legible, notarized, plural, and
supersedable — which is exactly what no complementary currency in the historical record had.

### 2.4 The constitutional floor: the never-rules

These are the ones that arrive as lenses with `role: floor` / `role: ceiling`, and the register is
**prohibition**, not redirect. Per the corpus's own handling classes, prohibition is right only for
substrate invariants — everything else is a context-relative policy that gets a redirect.

**N1 — No protocol-level unit that is held as property.** The protocol issues no unit that any person
or circuit holds, accumulates, transfers at a price, or pledges. Every *circulating* unit is a
community's own liability under its own lens. *(Stance I.2; and structurally, a holdable
protocol-level unit would be a protocol-level counterparty, which is the chokepoint §3 refuses.)*

> **What N1 does not say.** It does not say the commons may never issue. Issuance *against the
> commons, for the commons* — bounded, mandated, non-accumulable, reabsorbed on completion — is a
> different object from a unit held as property, and refusing it permanently would forfeit the very
> inheritance §1.1 says belongs to everyone. That capacity is held **unexercised and
> precondition-gated**, not forbidden, and its form would be a **policy lane over a commons pool**
> rather than a unit anyone carries. §2.9.

**N2 — No protocol seigniorage, and no private seigniorage either.** Issuance gain is common
inheritance (§1.1) and returns to the floor on a declared schedule. A circuit that retains its own
issuance gain has enclosed the thing canon names.

**N3 — Reach is never purchasable.** No unit, balance, or circuit position may confer standing,
governance weight, or audience. This is the anti-plutocracy invariant and it is what the
[Hypha survey](epr:hypha-dao-autonomous-collectives-cross-pollination-2026-06-24) rejected in
another project's design: a transferable token that is *also* a voting multiplier *"re-introduces the
exact capital→voice coupling Hypha says it rejects."* ✅ Corollary, from the same survey and binding
on any interop: **a bridge MUST strip transferable-token and voting-power-multiplier semantics at the
seam** — it projects agreements and agents, never token-weighted governance.

**N4 — Standing is not collateral and cannot be spent.** Standing is a graph-derived view with no
stored score, computed differently by different evaluators. ✅ A design that treats it as a quantity
which clears, transfers, or collateralises has imported the social-credit shape the architecture
refuses. Do **not** mint a `VoiceBalance` / `StandingToken` entry — *"that makes standing a bank-like
ledger."* ✅

**N5 — Issuance is a function of settled, verified reciprocity, never of account creation.** This is
trap **A5** — the Trueque trap — adopted verbatim from
[the trap detectors](epr:comparative-political-economy-trap-detectors-2026-08-07) rather than
re-derived. Its full redirect, which every circuit lens must satisfy: ✅
> issuance authority separable from issuance beneficiary · per-unit provenance verifiable at trade
> time at O(1) cost to the counterparty · a live public aggregate of units outstanding · stock limits
> or demurrage against hoarding · supply-side thinness monitored as a ratio.

Its signature — *any minting, credit, or reputation grant triggered by onboarding; growth velocity
celebrated as success; units outstanding not publicly aggregated* — is a detector, not a rule, and it
points at us.

**N6 — No convertibility guarantee, and no single fungible rail.** The protocol never promises that
one circuit's units convert to another's. Conversion, where it happens, is a bilateral agreement
between two communities, expressed as an agreement between them and legible as such.

**N7 — The care layer is never converted.** §2.7.

**N8 — No published default allocation.** Carried from the sibling paper's §8.2: *"the protocol
refuses to author an allocation but has not learned to refuse a default, and a fallback nobody chose
is harder to contest than one somebody did."* ✅ A shipped percentage table is a default nobody chose.

### 2.5 Plurality is the capture resistance

Lietaer's structural claim is that **monetary monoculture is the fragility** and a diverse monetary
ecosystem is resilient (◐). Read as a security property rather than an economic one, it is
**half** of this protocol's answer to monetary capture: *there is no unit everyone must hold, so no
single medium's capture would carry the network.* The other half — the half that keeps this from
being an argument from mere absence — is §2.9's: beneath every denomination sits an un-denominated
record that outlives it, so no medium ever becomes the only memory. **Plurality means no medium is
load-bearing; the record beneath is what stops one from becoming so.**

Three mechanisms carry it, all shipped or near-shipped:

1. **Schools.** The `school` field means two communities can run incompatible monetary philosophies
   over the same substrate without either being wrong. The protocol takes no view on Gesellian
   demurrage versus WIR clearing versus care-hours; those are readings, and the market shows them
   side by side.
2. **Forkability with a readable history.** Supersession chains mean a community that dislikes its
   circuit's direction authors a competing lens rather than fighting for control of a parameter.
3. **The `prosumidor` rule**, stolen from the Trueque as the corpus already recorded: **membership
   requires both producing and consuming** — *"the only norm that kept issuance tethered to
   output."* ✅ A holder who only ever draws is the failure signature, and it is queryable.

**And the hazard, which is ours and named as trap A6 — *ownerless is not uncaptured*.** An ownerless
system still has interests, written by whoever authors the elohim's weights, curates its corpus, or
controls its update path; *"a living-room elohim that phones home for its values is a datacenter with
a nicer address."* ✅ For a monetary posture this is the sharpest possible warning: **the capture
surface for a plural-currency network is not the ledger, it is the model that advises every council
about its rate.** A thousand currencies advised by one un-forked elohim is monoculture wearing a
diversity costume. Waqf — the Islamic endowment: property permanently dedicated to a purpose, with no
owner, administered by a trustee — is the thousand-year case law, and the safe form is
*elohim-as-clerk of a self-executing, community-amendable constitution — never lord.*

### 2.6 The Cantillon detector, pointed at ourselves

The source conversation's sharpest economic observation was about fiat: new money enters at specific
points, and whoever is nearest the issuance spends at pre-inflation prices. The Cantillon effect is
not a property of fiat; it is a property of **any** issuance.

So it is a detector, in the corpus's three-part shape, and per the Amplified-set discipline it points
at us:

- **Signature:** units issued, joined against the identity and standing of the first recipient, and
  the lag between issuance and the recipient's first settled reciprocal event.
- **Naive optimizer:** a circuit maximising velocity or participation issues earliest to whoever is
  most active — which is whoever already had the most capacity.
- **Redirect:** issuance concentration is not evidence that early recipients are more productive; it
  is evidence about **who was positioned to receive**, and it is the same measurement that A5's
  issuance-vs-settled-reciprocity ratio wants.
- **Council-facing output:** a standing panel — *"issuance in this circuit is concentrating at the
  top decile of prior standing; here is the distribution, here is the trend, here is the
  evidence."* The council steers. The detector reports. *(Standing caveat, stated here because it is
  load-bearing for everything downstream: ratification is currently first-quorum-wins and therefore
  single-write-capturable — §5. Every "the council decides" sentence in this paper is contingent on
  that fix.)*

This is the internal analogue of the thing the paper's external half asks a central bank to be honest
about, and running it on ourselves is the price of asking.

### 2.7 Two layers, and the one that is never converted

**This is not a new proposal.** The two-layer denomination was already written in §10 of the trap
detectors, and this paper extends rather than re-derives it: ✅

> a physically-denominated **compute-credit layer** that *may* trade across communities (FLOPs and
> kWh are legitimately commensurable) over the locally-denominated **care-recognition layer** the
> substrate never converts — *trade the physics, keep the meaning local* — plus a local-capacity
> sinking-fund split on every served call, so provider interest and village graduation align rather
> than oppose.

What this paper adds is why the rule is a **constitutional floor (N7) rather than a preference**, and
it is Stance III.1's reasoning: a system reshapes the world to produce what it measures. The moment a
care hour has an exchange rate against a compute credit, care becomes a priced input and the
protocol has rebuilt the thing the manifesto's Part I indicts — *"a dollar spent on weapons is
identical to one spent on medicine."* The commensuration *is* the harm. Physics may be commensurable
because a kWh is a kWh in any village; an hour with a dying parent is not.

The sibling paper's liability-absorption ledger sits exactly on this seam and is the reason the rule
needs stating precisely. That ledger **does** denominate care in fiat — for a municipal reader, at a
moment of presentation. The rule that keeps it honest: **denomination outward, for a named external
reader, never written back into the substrate's own record.** ✅ Otherwise the absorption ledger
resolves by the back door the valuation question the design deliberately leaves open.

**The live defect this rule already has.** We have an accidental numéraire in the tree today: `μ`
(mean balance) at `concentration_service.rs:47` **has no locality column**, so two communities on one
storage node share one relational scale nobody set. ✅ The corpus names the close: add `collective_cid`
to the scope key of `concentration_snapshots` / `responsibility_demand_configs` / manifests, and give
`ExchangeRateView` the `CollabAgreement` shape — bilateral, steward-counter-attested, negotiated —
**deleting the `"algorithm"` source variant**, because an algorithmically-derived exchange rate is
precisely the authored-by-nobody default that N8 refuses. This is a defect repair, not a feature.

### 2.8 What the internal posture is *for*: the personal equilibrium, denominated

The sibling paper's §4.5 argues that a person can limit their take from rent to what they would be
owed by the common inheritance in a Georgist/Gesellian end-state — reached in high-context counsel
under *deterministic floor, elohim ceiling*, with the invariant **never a computed payout at the
ceiling, never judgment at the floor**. ✅

The monetary consequence is worth stating plainly, because it is the place where a reader will most
expect a number and must not be given one:

**The equilibrium is a discernment, not a denomination.** It is not a figure the substrate computes
and applies; that would be a computed payout at the ceiling, and it would also be the single
universal instrument §2.7-of-the-sibling shows George and Gesell were forced to settle for. What the
substrate supplies is *evidence for the conversation*: what this person holds, what part of it is
reproducible produce and what part is positional increment, what the local floor currently costs to
hold, what neighbours lack. The person decides; the council holds what the person cannot hold alone;
the elohim shows its work and can be wrong.

**What the substrate may record is the commitment, not the calculation.** Once a person reaches their
own equilibrium, the thing that belongs on the substrate is an ordinary Commitment — a declared,
bounded, revocable, witnessed undertaking, in exactly the form the protocol already uses for every
other kind of authority. That is the **completed gift** the EAE survey was reaching for, in the one
shape that both survives Stance II.4 (witnessed, appealable, revocable) and needs no untouchability
to hold.

### 2.9 The generational answer: a substrate already full of currency, held in abstract

The posture so far answers *how* a community declares a currency. It has not answered the question a
plural-currency facilitator is actually obliged to answer:

> **How does a network that hosts many currencies avoid being captured by them?**

The obvious answer — *there is nothing to capture, because there is no unit* — is too thin. It
answers by absence, and an absence is not a design. It is also not quite true: the grammar ships
(§2.1), and a grammar is a thing that can be captured. The real answer is not that the
protocol holds no currency. It is that **it holds nothing else.**

**Start from what a currency actually is.** The definition *Rethinking Money* works from — recalled
here rather than re-verified at the page this pass — is not the token but the flow: a currency is
**the information that moves between parties in an exchange**: who gave what to whom, when, under
what agreement, and what was recognised in return. The unit is a
carrier for that information, and a lossy one. ◐ Read that way, **REA is already currency, kept in
abstract**: resource, event, agent — the same facts, recorded before anything has been priced.

Beer's line, which this paper already takes as its frame, is the same observation from the other
side: money *"attenuates the full state space of human contribution down to a few priced channels,
and care is variety that falls outside the channels"* — our reading of *Designing Freedom*, not a
verbatim Beer sentence. ✅ **A price is a denomination that has discarded almost everything it knew.**

**And the discarding is not a defect — which is where Hayek has to be met, not waved past.** "The Use
of Knowledge in Society" is precisely the argument that a price's compression *is* its power: a
one-dimensional signal that lets strangers coordinate without any of them holding the whole picture,
and no richer record can substitute for it. That argument is untouched here, and this paper does not
claim otherwise. **The answer is that the record is not competing with the price at the price's own
job.** Prices coordinate; records remember. The sibling paper states the same boundary as a refusal —
*no calculation is being attempted*: computation stays local, aggregation is voluntary, and the
global layer holds no allocative authority. What the un-denominated record is for is not
out-computing the signal but keeping what the signal dropped **available for judgment** — which is a
different function, on a different timescale, answering to a council rather than to a market.

So the protocol is not empty of currency. **It is full of currency, held un-attenuated** — and that
reframes the capture question rather than dodging it. But it reframes only half of it, and the half
it does not reach must be said here rather than saved for the limits section.

**Where capture actually happens.** A medium captures a network when it becomes *the only memory*.
Once the priced channel is the sole surviving record, whatever the price ignored is not merely
undervalued — it is **invisible**, and there is no remaining vantage from which anyone could notice.
That is how enclosure works monetarily. Not by owning the ledger; by being the only thing the ledger
remembers.

**The answer, then, is a layering rather than a refusal.** The un-denominated record is primary and
permanent. Every denomination over it is a lens — scoped, authored, dated, forkable, supersedable,
revocable — and therefore a *reading* of a record that outlives it. Three properties follow, and they
are the whole of the capture resistance:

1. **No currency is ever the record.** It is a projection of one.
2. **Any currency is re-readable.** The same events, under a different lens, yield a different
   denomination — without rewriting a single fact.
3. **What a currency ignored is still there.** Care that fell outside the priced channels remains in
   the substrate as witnessed events, available to the next lens that has a use for it.

The design constraint §0 already states — *record facts, defer valuation*, keeping provenance
immutable so any future settlement story stays retroactively computable ✅ — **is this property
written as an engineering rule.** The substrate never forgets what the currency abstracted away.

**But that defeats only one of the two capture modes, and the distinction is the honest core of this
section.**

- **Memory capture** — a medium becomes the sole surviving record, so what it ignored becomes
  unthinkable rather than merely unpriced. **The architecture does prevent this.** The record is
  primary, the denominations are readings, and a reading cannot erase what it declined to read.
- **Functional capture** — one denomination becomes the one everyone actually uses: the lens
  merchants accept, the unit the bridge institution settles in, the rail a newcomer has no realistic
  choice but to join. **The architecture does not prevent this**, and no property of the record
  does. A railway track outlives any particular train; a railway monopoly is still a monopoly.

Plurality of *possible readings* is not plurality of *live circuits*, and only the second is an
answer to functional capture. What would actually bear on it is thinner and more demanding than the
elegant claim: live alternative circuits with real volume, a merchant side that accepts more than
one, exit that is affordable in practice rather than in principle, and a detector watching
concentration of settlement — **which does not exist** (§8, limit 8). Until it does, this section's
claim should be read at exactly its true strength: **the substrate makes capture reversible and
visible; it does not make it impossible.**

**What the elohim is for at this layer.** Not to pick the rate. Its work is to read the
un-attenuated record and hold in view the thing a denomination structurally cannot carry — the deeper
responsibility, and human flourishing as the actual object — while a council decides the monetary
questions as the occasion presents them:

- how value should **flow**, and at what velocity;
- **when a store of value is appropriate**, and when it has become a strike in waiting (§2.2);
- what **friction accumulation should meet**, at what threshold, and on whom it falls;
- how **redistribution** happens, and on whose declared schedule;
- how **issuance** is warranted, and against what evidence;
- how much **liquidity** is enough, and what it costs to supply;
- how the circuit **meets the legacy economy** at the ramps, and what it owes there (§3).

*The council steers; the detector reports.* The elohim's advantage here is not judgment — it is
**variety**: it can hold the whole state space at once, which is precisely the faculty a single
priced channel gave up.

**And the design is therefore occasional, not universal.** A settled community with thick reciprocity
and a struggling one inside a polycrisis do not need the same monetary design, and forcing them onto
one is the monoculture failure (§2.5) arriving as compassion. What the substrate supports instead is
**policy lanes over commons pools**:

- A **policy lane** is a scoped, declared monetary regime — a lens or a bound set of them — governing
  one purpose for one scope, with a `telos` that says what it is for, a supersession path, and no
  claim on anything outside its scope. Lanes coexist; they do not have to agree; a community may run
  several at once for different purposes, and the unit-of-account limit (§2.2) is what keeps one from
  silently becoming all of them.
- A **commons pool** is the counterpart on the resource side: a stock held for a declared purpose,
  governed by a lane, belonging to no participant, and answerable to the council that declared it —
  the waqf shape, which is the thousand-year case law for exactly this (§2.5).

**Which is where issuance stops being an exception and becomes an instance — of reach.** Some public
goods are too large for any community to underwrite: a fusion or fission programme, industrial scaling
for an experimental technology whose unit cost only falls at volume (solar is the worked historical
case), universal provision such as healthcare infrastructure or a basic income, a continental grid.
They share a shape — enormous coordinated commitment, sustained over decades, no near-term return to
any contributor, and no possibility of the participants clearing it among themselves. **Mutual credit
cannot fund them**, because mutual credit is settled reciprocity between parties who trade with each
other, and nobody trades with a reactor. **Philanthropy cannot fund them at scale**, because it is a
claim on already-accumulated wealth and inherits the distribution it means to correct.

**But "superproject" is the wrong noun, and it hid the actual structure.** These are not a special
category of thing requiring a special new power. **They are ordinary Mishpat policies whose reach has
grown** — and this corpus already has the primitives for reach. A currency *is* a lens (§2.1); a lens
governs a declared scope; and how far a policy carries beyond the people who authored it is exactly
what **reach** names. So the governing question is never "may the commons issue?" It is the question
the protocol already asks continuously of everything else: **has this policy earned the reach it is
claiming?**

Three existing primitives therefore do the work a bespoke gate would have done badly:

- **Reach is earned and standing-gated, and is never purchasable (N3).** A policy cannot buy its way
  to the scope at which it could mobilise. That is the anti-plutocracy invariant already load-bearing
  everywhere else in this paper, applied to monetary policy without modification.
- **Scope is declared, not assumed.** `governs_epr` names what a policy governs; the unit-of-account
  limit (§2.2) forbids it silently becoming universal. A policy that has outgrown its declared scope
  is detectable as such.
- **Supersession and revocation are the ordinary path** (§2.1). A mandate at any scale is amendable
  and revocable by the mechanism every other Commitment uses. ✅

**And at sufficient scale, such a policy is a tax. The corpus should say the word.** A Mishpat policy
that has earned enough standing, scope and reach to mobilise resources across a whole layer is
functionally a power of taxation, whatever else it is called. The connotation is bad and the evasion
would be worse: declining to name it would be the identical move this paper convicts the apex of in
§2.2 — a values choice carried in a technical register precisely so it need not be defended as a
choice. Name it, and it becomes contestable.

**Subsidiarity is the gate, and it runs in the opposite direction from a state.** Canon is already
explicit — *maximum local autonomy within constitutional bounds; decisions made at the lowest
appropriate layer* ✅ — over an eleven-rung geographic ladder from **individual** through
neighbourhood, community, district, municipality, county, province, nation and continental to
**global**, with functional layers (workplace, educational, ecological/bioregional, cultural,
**industry_sector**, affinity) running in parallel. ✅ A state taxes first and devolves reluctantly.
Here the burden of proof runs **upward**: a tax-like policy is admissible at a given rung only where
every lower rung is *demonstrably insufficient for the task* — not merely inconvenient, not merely
slower. Subsidiarity is not a preference for smallness; it is an evidentiary standard that a policy
must clear before its reach is legitimate.

**The worked case, because the standard is meaningless without one.** Replacing the oil-burning
engines of the global container fleet — the reactors, and the shipbuilding and refit supply chain to
carry it — so that the carbon emissions of integrated global logistics end rather than relocate. No
household, municipality, or single nation can do this, and the reason is structural rather than
financial: **the emitting system is the supply chain itself**, which is why the policy sits on
`industry_sector` crossed with `global`, and why no lower rung can be shown sufficient. That
demonstration — not the size of the number — is what would earn the reach.

**The failure modes of monetary policy at scale need their own weight, because the reach primitives
were not designed for this.** Reach was built to govern *audience*; mobilisation is a different load,
and four hazards do not come for free with it:

| # | Hazard at scale | Where it bites | State today |
|---|---|---|---|
| H1 | **Seigniorage concentration** — whoever sits nearest issuance spends before anything adjusts (the Cantillon effect, §2.6) | The detector exists as a design; the council reading it is capturable | **Open** (§5) |
| H2 | **The ratchet** — a mandate that never completes and so never reabsorbs, becoming a permanent claim | Non-accretion depends on reabsorption actually happening | **Not designed** |
| H3 | **Measurement capture** — whoever defines the need defines the issuance; the evidence base becomes the prize | The un-attenuated record is the defence, and it is also the target | **No detector** |
| H4 | **No exit at global reach** — a policy at the top rung has no outside for those inside it, which is precisely the property that makes state taxation coercive rather than voluntary | Appeal and arm's length carry the entire load once exit is gone | **Unresolved** |
| H5 | **Deferred-loss attribution** — the signal that triggers reward settles *before* the signal that reveals cost, so whoever authored an exposure is promoted out of its consequence and a successor inherits the loss | Standing granted on unsettled outcomes; the selection effect compounds each cycle | **No primitive** (§2.10) |

**H4 is the one that should stop anybody claiming this is solved.** Every reassurance elsewhere in
this paper leans somewhere on plurality and forkability — *if you dislike this circuit, author
another*. At global reach that consolation is gone by construction, and what remains is the appeal,
the arm's length, and the council's own honesty. Which is exactly the set §5 says is currently
capturable. **A tax power on capturable governance is the worst object in this paper**, and naming it
is the only responsible thing this section can do about it today.

**What the arm's length is for.** The danger is not issuance; it is *proximity*. An issuance power
sitting inside the valueflows it funds is the Cantillon effect with extra steps, and it is how commons
of this kind have historically been enclosed. The answering design is separation: the body
deliberating a mandate must be structurally distinct from the flows receiving it, so that **no
beneficiary sits on the body that authorises**. Councils deliberate at arm's length from the human
valueflows; the elohim advise and show their work; the flows stay in the ordinary REA plane where
every other event lives. The arm's length is not ceremony — **it is the whole of the anti-enclosure
argument.**

**One observation about symmetry, since the objection will be reflexive.** The wealthy already author
their own jurisdictions: offshore shelters are private constitutional design, purpose-built, and
nobody asks whether they have earned the reach. A commons declaring a policy lane in the open, with a
named author, a declared scope, a supersession path and a public aggregate, is not a stranger thing
than what capital does routinely and quietly. That is an argument about **symmetry, not permission** —
it establishes that the question is live, not that the answer is yes. And it is also a warning: the
same evasion is the predictable response to any policy that reaches, which is §2.10's subject.

**And the limit on all of it.** None of the above is a design, and this section must not be read as
having produced one. H1–H4 are unresolved, the governance the whole structure rests on is capturable
(§5), and nothing here has been tried. It is the argument that the *question* stays open and that
**reach, standing, scope and subsidiarity are the right instruments to hold it** — not a claim to
have built any of them for this purpose. A protocol that refuses to name a capacity it cannot yet
hold ends up either pretending it never wanted it, or reaching for it the first time a council faces
something big and finds no rule there. Naming it, locating it in primitives it already has, and
leaving it unexercised is the third option. ⚠

### 2.10 The membrane is a values surface: provenance and dynamic pricing at the bridge

§3 concedes that the enforcement surface was never the issuer — **the ramps, the holders and the
merchants are.** That is stated there as a vulnerability. It is also the opportunity, and the corpus
should treat it as a foundational feature rather than a compliance afterthought.

**Every payment crossing the bridge carries a provenance question, and the network is entitled to
ask it.** A state asks a narrow version of this already, for narrow reasons: sanctions, laundering,
tax. The network's version is wider and differently motivated — not *is this legal?* but **what was
extracted, from whom, to produce this?** The un-attenuated record (§2.9) is what makes the wider
question answerable at all, because provenance is exactly the kind of context a price discards. This
is the **Values Scanner** applied at the membrane rather than to a household budget: the same organ,
pointed at the boundary.

**And the corollary is pricing.** If entry is priced dynamically against that provenance, then the
burden of arbitrage at the edge — the gap between what an external actor can extract elsewhere and
what they can realise here — **funds the commons instead of accruing to whoever is fastest at the
boundary.** Wealth that exits an extractive position into the network pays for the exit, and the
payment lands in the communities the network touches, in every corner of the globe that transacts
with it. That is the redistributive counterpart to §2.9's mobilisation, and it is the more important
of the two, because it needs no mandate and no issuance — only a boundary that declines to be neutral.

**The worked case that makes the inversion legible — and it is already half-built.** In conventional
property-and-casualty cover, the rate per unit of exposure *falls* as the exposure base grows, and two
quite different things are bundled into that discount. One is a genuine efficiency: administering a
single large policy costs less per insured dollar than administering a thousand small ones, and
passing that saving on is honest. The other is **bargaining position** — a holder large enough to move
an insurer's book can negotiate a rate below what its own liabilities would actuarially justify, and
frequently does, sometimes below the level at which the policy is profitable to write. ◐ **The second
is not a discount for being less risky. It is a discount for being large, and the pool's smaller
members fund it.**

That is the Cantillon effect wearing an actuary's coat (§2.6): whoever sits nearest the pricing power
gets terms before the pool adjusts. It is the unearned increment in George's precise sense — a return
to position rather than to produce. And in the monetary register it is what N3 forbids outright, since
a negotiated below-actuarial rate is **the purchase of position by scale**, which is reach being
bought.

**The repair is this paper's own move, arriving in a second domain: decompose rather than fuse.**
Conventional insurance prices with one instrument and therefore cannot separate the efficiency from
the leverage — the identical forced fusion §2.2 identifies in money's three functions, in a different
market. On an un-attenuated record the two come apart, because administrative cost and actual
exposure are separate recorded facts rather than one negotiated number: the **administrative economy
of scale is produce** and should be passed on in full; the **positional discount is rent** and should
not exist.

**And here the corpus is further along than anywhere else in this paper.** `MemberRiskProfile` ships
in the elohim DNA's integrity zome and prices risk on *"actual behavioral observation instead of
proxies like credit scores"* — care-maintenance, community-connectedness, and claims history, carried
by Observer attestations with an evidence trail and a trend direction
(`content_store_integrity/src/lib.rs:1501`). ✅ **That is the §2.9 thesis instantiated**: the
un-attenuated record doing actuarial work a price could not, because the context is exactly what a
premium discards. `CoveragePolicy` carries a `coverage_level` and a `governed_at` Qahal
(`:1563`) over a five-rung `GOVERNANCE_LEVELS` ladder — individual, household, community, network,
constitutional (`:2120`) ✅ — which is **subsidiarity shipped**, not merely argued. And decisively:
there is **no exposure-base, portfolio-size, or total-insured field anywhere in the entry type.** ✅
Scale is not an input, so the conventional discount has nothing to attach to. The inversion is
structural rather than enforced.

**The self-conviction this section owes, because the same read produces it.** `RISK_TIERS` ends in
**`"uninsurable"` — *"Too risky for coverage"*** (`:1538`). ✅ A pool that can price anyone honestly
and still exclude them has reproduced the sorting it was built to answer, and it fails Part IX's test
— *those who have been at the bottom of every system* — from the inside. Mishpat has no punishment
category and justice is restored capability (§1); an uninsurable tier is neither. Whether that tier
is a floor-layer responsibility (the pool prices, the commons covers) or a defect in the entry type
is **an open canon question, and this paper does not settle it — it names it.** ⚠

**And the failure mode the hazard table misses comes from inside the industry.** An underwriter books
a large policy — half a million dollars of annual premium, say — priced far beneath what the exposure
warrants. The premium is recognised *now*. The losses arrive later: a serious auto claim, a
workers'-compensation claim that runs for years. By then the book has been handed on, and the person
who wrote it has already been promoted on the only signal that had settled — **volume booked,
business landed.** The successor inherits the million-dollar losses. ◐ *(operator account, first-hand,
P&C underwriting.)*

Three things follow, and the third is the one that matters.

1. **The reward and the consequence have different addresses.** The gain attaches to the author at
   the moment of writing; the loss attaches to whoever holds the book when it settles.
2. **The mispricing is invisible precisely when it is being rewarded**, because the one channel being
   read — premium written — is the one channel structurally incapable of showing it. That is Beer's
   attenuation exactly (§2.9): a person priced on a few channels, with the variety that would have
   contradicted the reading discarded as a matter of course.
3. **So it is a selection effect, not a run of bad luck.** Each cycle promotes the people whose
   success came from writing business the book could not carry. Over enough cycles the firm's
   leadership is disproportionately composed of them, and the behaviour is not surviving *despite*
   the mechanism — it is surviving *because of* it. **The trap is evolutionary, and the organisation
   is its own selective pressure.**

**The regulatory reading is the part this paper cannot afford to skip.** The entire state
rate-regulation apparatus exists to prevent this specific failure: it obliges insurers to collect
*enough* to cover expected losses, and in many jurisdictions legally mandates a minimum margin. It is
the strongest available instance of *an apex institution with the correct mandate, the legal
authority, and the actuarial data in hand.* **And insurers still destroy themselves on competitive
cycles.** ◐ That is not an argument that regulation is worthless — it is evidence that **a rule at the
apex cannot repair a measurement failure at the edge**, which is §2.2's decomposition argument
arriving from the empirical side rather than the theoretical one.

**In this corpus's own vocabulary the trap already has a name: it is N5.** *Issuance is a function of
settled, verified reciprocity, never of account creation.* N5 was derived from the Trueque, where
units were minted on account creation rather than on settled trade. The underwriting case is the
identical defect in a non-monetary register: **standing issued on booking rather than on settlement,
and the promotion is the mint.** Put in the reach vocabulary of §2.9 it is sharper still — reach was
*granted* on a signal that had not yet settled, which is not earning at all. N5's own redirect already
demands the repair in the monetary case: *lag-until-first-settled-reciprocity*, and a live public
aggregate. Nothing applies it to standing.

**The self-conviction, because the same trap is fully available to us.** `MemberRiskProfile` carries
`historical_claims_rate`, a `risk_trend_direction`, and a `next_assessment_due` ✅ — every one of
which scores the **member**. Nothing in the tree scores the **judgment of whoever set a price**, and
nothing joins a pricing decision to the losses that settle after its author has moved to another
scope. A protocol that priced risk on witnessed behaviour while granting standing on volume would
have rebuilt the trap exactly, with better data and a cleaner conscience. **The un-attenuated record
is what makes the repair possible — the join is permanent and the author is named — but no primitive
performs it.** ⚠ This is the most concrete thing H3 and H5 point at, and it belongs in the
Measure family as a row rather than sitting here as a paragraph.

**Three constraints keep this from becoming something else.**

1. **It is a price, not a punishment.** Mishpat has no punishment category; justice is restored
   capability (§1). A provenance-sensitive entry price is a term of trade a counterparty can read in
   advance and decline, not a sanction imposed after the fact.
2. **It must not become a border.** A membrane that prices entry so steeply that only the already-
   compliant can cross has rebuilt the exclusion it was meant to answer, and would fail Part IX's
   test — *those who have been at the bottom of every system* — from the wrong direction.
3. **The scanner is capturable in exactly the way §2.5 warns.** Whoever authors the valuation
   authors the toll. Trap A6 applies with full force: **a single un-forked elohim pricing the
   world's entry is the monoculture failure at the point of maximum leverage.**

**State of it: nothing here ships.** There is no provenance field on a bridge payment, no dynamic
pricing anywhere in the tree, and the bridge institution itself (§4) is a design. This is a research
position recorded so it can be argued with, and it is the strongest single candidate for the next
pass on this paper. ⚠

**What all of §2.9–2.10 is actually for: soft power, and the honest limit of it.** Neither section
arms the network. They describe the instrument by which the sibling paper's *succession without
conquest* actually operates — **the commons projecting soft power**: influence that works through
what others want access to, and through the terms on which access is granted. A tax-like policy at
earned reach and a provenance-priced membrane are both instruments of **negotiation**, not
compulsion. The network cannot levy on anyone who has not joined, cannot seize, and — Stance IV.2,
already binding on everything in this paper — *"owns no violence to impose."* ✅ What it has is a
function worth having, and terms.

That is how subsumption happens peacefully rather than by collapse. An external power — a
municipality first, then an industry, eventually something larger — negotiates with the network
because the network is *delivering a function it needs*, and the terms of that negotiation are where
the values are actually enacted. The sibling's §6 is the municipal instance of exactly this move;
§2.9 and §2.10 are its general monetary form. It is also why §3.2 refuses obsolescence-by-
hyperinflation: a strategy whose end-state is the incumbent's collapse forfeits the negotiation that
was the whole mechanism.

**And the limit, which is not small.** Soft power is non-coercive exactly while joining remains
optional — and **the more successful the network, the less optional it becomes.** That is functional
capture (§2.9) arriving as *success* rather than as attack: "you can always not join" ages into "you
can always not use the internet," and it ages faster the better the thing works. Nothing in this
paper prevents that, and no monetary rule can, because the mechanism is adoption rather than
compulsion. What stands between soft power and ordinary leverage is not architecture but the
sibling's three refusals in §6.5, quoted rather than paraphrased because the wording is the
commitment: **no dependency** (*"the network must be able to say so truthfully"*, whose named failure
is *"the network discovers it can threaten to stop"*), **no conditionality** (*"no absorption is
offered contingent on policy, forbearance, or recognition"*, whose failure is that *"the first use as
leverage retro-taints every prior offer"*), and **no exit tax** (*"any person and any institution may
walk away at no cost"*, whose failure is *"coercion in the language of attraction"*). ✅ Those are
load-bearing commitments rather than decorative ones, and the sibling is explicit that if succession
requires violating any of them, *"the correct response is to stop rather than to proceed with a
better story."* ✅

**No exit tax bears directly on §2.10, and the distinction is fine enough to state.** Pricing *entry*
against provenance is not an *exit* tax — nobody is charged for leaving, and the third refusal is
about departure. But the failure mode it names, *coercion in the language of attraction*, is exactly
the risk a priced membrane runs, which is why the second constraint below is not optional.

### 2.11 What §2 composes into — and the line it must not cross

Stated at full strength, because the claim is the operator's and deserves to be met rather than
softened: **the protocol occupies, in effect, the role of a maximally honest and disciplined
underwriting, banking, and investment system at scale** — because an assessor can sit at nearly every
transaction, externality and risk, and price it against what is actually known rather than against a
proxy. Wisdom available at the scale of everything the network can see.

**The three institutions are one function, separated by an accident of cost.** Underwriting prices a
claim about the future given a counterparty. So does lending; so does investing. They differ in tempo
and instrument, not in kind — each is *an assessment of a future claim under incomplete information
about a party.* They became separate professions because the **assessor was expensive**, and the way
an expensive assessor is made affordable at volume is **standardisation**: actuarial tables, credit
scores, ratings, covenants, sector betas. Every one of those is a proxy substituted for knowledge
that could not be afforded per case — which is Beer's attenuation again (§2.9), and the reason a
credit score can be simultaneously predictive in aggregate and unjust in the particular. The stated
design of `MemberRiskProfile` — behavioural observation *"instead of proxies like credit scores"* ✅
— is that substitution being refused in the one domain where the corpus has actually built something.

**And the addition no incumbent can make is the externality.** Conventional underwriting prices risk
*to the insurer*; it is structurally incapable of pricing harm to a third party, because there is no
counterparty to bill and no channel in the instrument to carry it. This is precisely the fifth defect
the manifesto named and never discharged — currencies *"are blind to externalities"* ✅ — and it is
why this paper exists at all (§0). An assessor per externality is not a new financial product. **It is
the discharge of that promissory note**, and it is available here for exactly one reason: the
un-attenuated record holds the context an externality lives in, which a price had already thrown away.

**Now the line, and it is the most important sentence in §2.** *This is not a global price, and it
must never become one.* The sibling states the boundary in terms this section inherits without
amendment: the calculation problem is not solved, **no calculation is being attempted** — *"computation
stays local, aggregation is voluntary and anonymised, and the global layer holds no allocative
authority… requisite variety and polycentric governance, not a planning board. Any design in this
project that starts allocating from an aggregate has crossed that line and should be read as a
defect."* ✅

So the defensible claim is narrower than the words "at global scale" naturally suggest, and the
difference is the whole of it:

| | Refused | Held |
|---|---|---|
| **What is universal** | one assessment, computed centrally, applied everywhere | the *availability* of a local assessment, everywhere |
| **What it produces** | a price the network asserts | a reading a party can act on, argue with, or ignore |
| **Where it lives** | an aggregate with allocative authority | the scope that authored it, plural and forkable |
| **Hayek** | answered by out-computing him | untouched — no calculation attempted (§2.9) |

**An assessor everywhere is requisite variety. One assessor for everywhere is the planning board.**
They are separated by a single property — whether the assessments aggregate into an authority — and
nothing but discipline keeps them apart, because the machinery is identical either way.

**And "maximally honest" is not a property of assessment at all.** This is what the underwriting trap
(§2.10) actually taught. The underwriters who destroyed their books were not badly informed; the
assessment was not the failure. **The failure was in what the assessment was rewarded for**, and
cheapening the assessor does nothing about that. Honesty is a property of the **standing layer** —
of whether reach can be earned on a signal that has not settled — and that layer is the one §5 says is
currently capturable. An assessor at every transaction, sitting on a reward mechanism that pays out
before the loss arrives, would industrialise the trap rather than repair it. **The order of operations
matters: fix H5 and the ratification defect first, or the scale is an amplifier.**

**Three further costs, so the claim is not read as free.**

1. **Assessment is compute, and compute is power.** An elohim at every transaction has an energy bill,
   and §5.7–5.8 of the sibling is this project's own argument about who pays such bills and what they
   do to a community. "Nearly every transaction" is a quantity with a watt figure attached, and nobody
   here has produced it. ⚠
2. **The record becomes the prize** (H3). When everything is priced from the record, capturing the
   record is the highest-return attack in the system, and it scales with the claim.
3. **"Visible to the network" is doing enormous work.** Whatever is not visible is unpriced, so the
   boundary of visibility becomes the boundary of justice — the selective-legibility cost of §3.4
   turned inward, with per-fold anonymity still unsolved (§8, limit 4). A system that prices only
   what it can see will systematically under-price whatever stays outside it, and the people outside
   it are rarely the powerful.

**State of it.** None of this ships. There is no underwriting of externalities anywhere in the tree,
mutual credit has zero implementation, and the one genuinely-built instance — the insurance mutual's
behavioural risk profile — scores members rather than judgment and still carries an `uninsurable`
tier. This section is the composition §2 points at, recorded so it can be argued with and so the line
it must not cross is written down **before** anyone is tempted to cross it. ⚠

---

## 3. The external posture — toward state fiat

### 3.1 Canon holds two registers, and both must be held

A paper that presents only one misrepresents the corpus. ✅

- **Compliance.** The constitution's National Layer: *"Compliance with national law unless it violates
  global layer"* and — the operative phrase for this whole paper — *"Support for legal reform toward
  flourishing, **not circumvention**."*
- **Subordination.** The manifesto: nation-states are *"near-sovereign yet subject to the protocol
  too… they can be corrected, restructured, replaced, or dissolved by combination of external
  consensus and internal consent to the global values network."* And the confession: *"The ecological
  layer outranks the nation, so that creation's limits hold veto over national sovereignty."* ◐

These are not in contradiction once the timescale is right. The compliance register governs **conduct
now**; the subordination register states **what the protocol holds to be true about legitimacy**, on
the same footing as any tradition that holds a law above the law. The posture below is the compliance
register worked out operationally, and it takes the *"not circumvention"* clause as binding.

### 3.2 Two naive positions, both refused

**Naive position one: illegibility.** The network escapes the state by having no capturable centre.
Refused at length in the sibling paper — by unincorporated-association doctrine (the participants
become the entity, jointly and severally), by Ostrom's eighth design principle (recognition is a
success condition, not a compromise), by Scott read in full, and by our own trap A6. The crypto-
monetary version of this position — *a currency escapes a state by having no issuer* — fails
identically and for the same reason: the issuer was never the enforcement surface. **The ramps, the
holders, and the merchants are.**

**Naive position two: obsolescence by hyperinflation.** The conversation this work derives from ends
here: a hyper-mutual network absorbs real activity, the fiat tax base collapses, the treasury
monetises, fiat hyperinflates, and the real economy is fine because it has moved. Refused on three
grounds.

1. **It is not peaceful, and the conversation concedes it.** Its own final section names what
   actually happens: the state mandates fiat-only payment for energy and utilities, criminalises
   alternative clearing, and shifts to collecting in kind. A theory whose end-state is the state
   seizing physical infrastructure has not avoided confrontation; it has scheduled one.
2. **The harm lands on the wrong people.** Hyperinflation destroys the savings of everyone *outside*
   the network — pensioners, the unbanked, the un-networked. Against Part IX's test ("those who have
   been at the bottom of every system") this is disqualifying. And against Part IV-B's emission
   posture it is the precise inversion: a maximal negative externality onto non-participants, from a
   protocol whose measure of success is positive externality onto exactly those people.
3. **It mistakes which institution is under pressure.** Municipal and national incentives run
   opposite ways, and the mechanism is worth stating here rather than deferring to the sibling. A
   municipality is a *cost centre carrying a mandate it cannot fund*: it must deliver water, roads,
   policing, emergency response and pensions out of a base it does not control, and its budget
   process rewards anything that removes a line item. A treasury is the inverse — its mandate **is**
   the base, and anything that shrinks taxable activity attacks the instrument it exists to defend.
   So absorption is strong at the municipal level and weak at the national one, not because
   municipalities are braver but because shedding a liability scores as a win in one ledger and a
   loss in the other. A strategy that assumes the treasury collapses is betting the project on the
   leg that does not bear weight. **Neither paper has tested this on a real municipality** (§8, limit
   3), and the sibling asserts the reception without precedent — so this is a reasoned incentive
   claim, not an observed one. ◐

### 3.3 Take chartalism seriously: the tax gate is the bridge

The gate is real. A state's currency is demanded because obligations are denominated in it, and no
amount of protocol design removes that. Every honest actor in the complementary-currency record
worked *with* it rather than against it: WIR settles **split** — some fraction francs, the rest WIR —
precisely so members can meet fiat obligations. ◐

The instructive precedent runs further, and it inverts the whole framing. In the **C3** model
(Commercial Credit Circuit, developed by STRO and deployed in Uruguay, Brazil and Honduras), the unit
is backed by insured invoices — and the Uruguayan government **accepted C3 units in payment of
taxes**. ⚠ *(This specific claim should be verified at source before it becomes load-bearing; the
mechanism's shape is ◐ and the tax-acceptance detail is the part to check.)* If it holds, it is the
single most important datum in this paper's external half: **a mutual-credit unit becomes robust
exactly when a tax authority accepts it.** The gate is not the enemy of complementary currency. It is
the thing that, once opened even slightly, makes one durable.

So the posture is not "escape the gate." It is:

> **Make every participant's fiat position continuously computable and honestly reported, so that the
> single largest enforcement pretext against the network never has a factual basis — and so that the
> ladder in §3.5 has somewhere to start.**

A network whose members are visibly, boringly tax-compliant is a network a revenue authority has no
reason to attack and, eventually, a reason to work with. This is *"support for legal reform toward
flourishing, not circumvention"* discharged operationally.

**One thing the protocol must not do, per §1.6.** It must not build a tax. Canon has no taxation
mechanism, declines UBI-as-transfer and limitarianism-as-cap by name, and returns the common
inheritance by negotiated graduation. Fiat tax obligations are **participants' own, under their own
jurisdictions**, and the network's contribution is evidentiary — making a member's position legible
*to that member and their accountant* — never custodial and never a withholding.

**How a fiat position is computed when the circuit has no exchange rate.** This is the practical crux
of the whole external posture, and the first draft of this paper asserted the posture without
answering it. N6 refuses a convertibility guarantee, so there is no protocol rate to read. The answer
is that **a taxable event already carries its own denomination, and the network's job is to preserve
it rather than to derive it**:

1. **Most circuit trades have a fiat leg, and it is the leg that prices them.** WIR's split
   settlement is the model: a member paying 70% francs and 30% WIR has stated a price in francs for
   the whole transaction. The fiat-denominated value is a *fact about the trade*, recorded at the
   moment it happens, not a rate applied afterwards.
2. **Where a trade is wholly in-circuit, the parties declare the fiat value at the time**, exactly as
   a barter exchange must under existing tax law, and that declaration is a witnessed, counter-signed
   attestation like any other. Both sides sign the number. That is a stronger record than a
   self-assessment on a form, and it is the same primitive the substrate already uses.
3. **Care-layer events are not priced at all** (N7), and this is where the posture must be precise
   rather than convenient: gift and mutual-aid activity has never been a taxable exchange, and
   nothing here changes that. The line between an in-circuit *trade* and a *gift* is drawn by the
   parties, declared, and legible — which is more than the cash economy offers.
4. **The bridge institution (§4) holds the fiat leg**, so where an obligation is actually settled in
   state money, an ordinary regulated entity is the one doing it, keeping ordinary records.

What the network therefore supplies is not a valuation engine but a **continuously-current position
statement**: here are your declared fiat-denominated events, here is the counterparty attestation on
each, here is your running position, ready for your accountant. **The protocol never computes a rate
it was not given, and never asserts a value the parties did not declare** — which keeps §0's
*record facts, defer valuation* intact at exactly the point where it would be most tempting to break
it.

*This is a design answer, not a shipped one.* Nothing in §5's build state implements a declared-value
attestation or a position statement, and no revenue authority has been shown either.

### 3.4 Selective legibility, stated as a rule

> **The network is maximally legible where a state's concerns are legitimate, and structurally
> uncapturable where its instincts are acquisitive. The first is a choice about evidence; the second
> is a property of having no centre.**

Legible, by design: aggregate economic activity and its fiat-denominated valuation at the boundary;
the identity and jurisdiction of the legal persons who participate; provenance of units within a
circuit (A5 requires per-unit provenance verifiable at trade time anyway); the absorbed-liability
ledger (sibling §5.2); the circuit's own rules, since a lens is a public notarized artifact.

Not available, because there is nothing to hand over: a protocol treasury; a global ledger of all
participants; a key that freezes a circuit; a party who can be ordered to alter another party's
records. None of these is withheld — **none exists.** The distinction matters legally and morally:
this is not obstruction, it is architecture, and it is the same architecture that prevents *us* from
doing those things.

**The honest cost, which this paper will not hide.** Our own corpus already convicted us here:
*"witnessing-as-legibility is the move Scott warns about, executed with better intentions and no
exemption,"* and Trap 5's corrected form is asymmetric in both tails — Aadhaar's error correlated
with the served population; the Trueque died because *nobody could see* how many créditos existed;
and *"local authorship is not a defense against legibility harm. It changes who administers it."* ✅
Choosing legibility is choosing a set of harms whose mitigation is unfinished work.

### 3.5 The escalation ladder

Five rungs, each with a precedent, each reachable without the one above it. Nothing here requires a
threshold, a critical mass, or anyone's defeat.

| Rung | What happens | Precedent | What it needs from us |
|---|---|---|---|
| **1. Fiat-legible circuits** | A circuit runs; members settle split; every member's fiat position is continuously computable and honestly filed | WIR split payment ◐ | §2.1's `binds-policy` gap closed; a valuation surface at the boundary |
| **2. A bridge institution** | A co-op, CDFI, credit union, or state public bank joins as a legal person and gives the circuit a fiat settlement leg | credit-union CUSO structures; the Bank of North Dakota ◐ | The legal-person layer as architecture (sibling §6.4); **no single bridge load-bearing** |
| **3. Local acceptance** | A municipality accepts circuit units for some fees, or denominates a program in them | C3 Uruguay ⚠; Bristol Pound council-tax acceptance ⚠ | A circuit with a clean issuance record and a public aggregate outstanding (N5) |
| **4. Liability absorption** | The network presents an audited account of public obligation it has taken on; the municipality books the relief (see below) | none — this is the new instrument | Sibling §5.2, plus the three refusals of §5.5 |
| **5. Federated administration** | Bioregional councils carry functions municipalities have ceded, with the municipality as a partner rather than a predecessor | Ostrom polycentricity ✅; Bauwens' partner state ⚠ | Everything above, plus governance capacity we do not have |

**Rung 4, stated here so this paper does not depend on its sibling for its own ladder.** The network
already records the witnessed events that constitute absorbed public function — care hours delivered
to an elder who would otherwise be a home-care case, a dispute resolved restoratively that would
otherwise be a docket entry, maintenance performed on a shared asset. Those are aggregated,
denominated in the municipality's own budgeting unit, and presented as a claim that says *here is the
obligation you no longer carry, with the evidence attached* — not *pay us*. The incentive is fiscal:
municipal obligations grow faster than the tax base, and a council needs a defensible line item, not
a conversion to mutualism. Three refusals keep it from becoming leverage — **no dependency** (it must
stay reversible on the municipality's side), **no conditionality** (never offered in exchange for
forbearance), **no exit tax** (anyone may walk away at no cost). The denomination is **outward only**:
it is translation for a named external reader at a moment of presentation, and it is never written
back into the substrate's own record, which is what keeps §0's *record facts, defer valuation* intact.
The full argument is [the canon companion](epr:succession) §6.

**The ladder is not a roadmap and rung 5 is not a promise.** Most circuits will live at rungs 1–2
indefinitely, and that is a complete outcome, not a failure to progress. Rung 4 is where the sibling
paper's contribution sits and where the refusals bind hardest: no dependency, no conditionality, no
exit tax.

### 3.6 What the AI-mediated commons actually absorbs

"Absorbing statecraft concerns" is the phrase to be most careful with, so here it is concretely.
**Not** the state's authority, its mandate, or its monopoly on force. Its *concerns* — the problems it
is charged with and is measurably failing to solve — met by producing **better evidence than the state
can collect on its own**, and offering it freely:

- **Measurement.** Real-time local economic accounts built from witnessed events rather than from
  lagging surveys — the thing Jerven's critique says national statistics cannot deliver at local
  resolution. ◐
- **Risk detection.** The detector library, pointed at the local economy: issuance concentration
  (§2.6), supply-side thinness (N5), circuit fragility.
- **Needs assessment.** What a place actually lacks, from the events themselves rather than from
  eligibility forms — with §7's warning attached, because "the assessment instrument becomes the
  care" is a named structural trap.
- **Dispute resolution.** Restorative processes that never reach a docket, with the record to show it.
- **Public-good maintenance.** The absorbed-liability ledger.

**And the honest counter-argument, which is the strongest one against this whole paper.** Every item
above is also a description of a surveillance apparatus with better manners, and *"an elohim that
phones home for its values is a datacenter with a nicer address."* The only structural answers we have
are: computation stays local; aggregation is voluntary and anonymised; the global layer holds no
allocative authority; the elohim is forkable and its corpus is local; and the detector reports while
the council steers. Those are real, and they are not sufficient by themselves — **per-fold anonymity
remains an unsolved design constraint, not a footnote.** ✅

### 3.7 The threat model, named

| Vector | Live precedent | Our exposure | Posture |
|---|---|---|---|
| Unincorporated-association / joint-and-several liability | Ooki DAO; *Sarcuni v. bZx* ◐ | Households personally liable for network conduct | The legal-person layer; no protocol-level actor to be construed as the association |
| Money-transmitter / MSB classification | routine state enforcement ◐ | A circuit that custodies or transmits value | The protocol custodies nothing; a bridge institution is licensed for what it does |
| Securities | the EAE epic's own "future value securitization" and 5% revenue shares to neighbours (sibling §6.3) ◐ | **Live and unnamed in our published content** | Name it; do not build it; operator action |
| Sanctions / OFAC | Tornado Cash ◐ | A circuit that becomes a value-transfer path | Legibility at the boundary; named legal persons |
| Payment-rail deplatforming | routine | A bridge institution's account closed | **No single bridge load-bearing** — the rung-2 invariant |
| Chokepoint seizure (hosting, domains, RPC) | routine | The doorway projection layer, T4 | Real, and mitigated by the substrate's own topology rather than by legal posture |

**One refusal, stated plainly because a paper about escaping monetary chokepoints invites the
question.** Nothing here is designed to evade sanctions, launder value, or conceal transfers, and the
architecture is actively hostile to those uses: witnessed events with attested participants and
per-unit provenance are a *stronger* evidentiary record than cash. A network built to make care
visible cannot coherently offer opacity for value transfer, and would not be worth building if it
did. That is not a compliance posture adopted for safety; it follows from the primitive.

---

## 4. Where the two postures meet: the bridge institution

The inward and outward halves join at one pattern, and naming it is most of this paper's practical
content.

A **bridge institution** is an ordinary legal person — a co-op, a CDFI, a credit-union CUSO, a
community land trust, a state public bank — that participates in a circuit *and* holds a fiat
relationship. It is where the lien gets filed (sibling §7's one-shot defector), where the fiat
settlement leg lives, where the tax filing happens, and where a licence is held if a licence is
needed.

Four invariants:

1. **No single bridge is load-bearing.** If any one institution's failure or seizure would end a
   circuit, the circuit has re-created the chokepoint it exists to avoid. Design for swap-out from
   the first day.
2. **The bridge holds fiat, never the circuit's governance.** It is a member, not an authority. It
   gets no lens-authorship privilege, no ratification weight, and no reach it did not earn — N3.
3. **The bridge is legible; the protocol is empty.** All the compliance surface lives at the bridge,
   which is *built for it*. This is what makes distributed legal legibility cheap rather than
   burdensome — the households do not each become financial institutions.
4. **The bridge cannot convert the care layer.** N7 binds it too. A bridge that offered care-hour
   redemption in fiat would perform exactly the commensuration §2.7 forbids.

The master-account question from the source conversation lands here. The realistic path is not the
Fed building a WIR; it is a **state public bank or a CDFI winning direct settlement access**, and a
circuit connecting through it. That is a sibling-paper Watch item, and it is the single external
development that would most change this ladder.

---

## 5. Build-state adjudication

House rule: verdicts adjudicate against **current build-state** with file:line, never the durable
spec. Verified 2026-08-23. This is the section that keeps the paper from being a proposal about a
system that does not exist.

**What ships and is right.** The `author-lens` Commitment with its six-field validator and closed
`{lens, floor, ceiling}` role enum; the `lenses` table and `lens-market` route, fail-closed on a NULL
`dht_anchor_hash`; commitment immutability with supersession; `cid == entry_hash` as commitment
identity; the seven-check bounds validator (found, not-revoked, in-window, scope match, reach ceiling
on the 8-level order, sliding-60-minute rate window, rotation TTL) — which is a **credit-limit
enforcement engine that already exists**, currently metering authority rather than value. ✅

**What does not ship, and blocks the posture.**

- **`binds-policy` is absent** from the coordinator dispatch — a lens can be authored and read but not
  declared binding. ✅ This is the gap between "a currency can be described" and "a currency can be
  enacted."
- **The lens market's ranking signals are dormant.** `lens_selections` (affinity) and `lens_verdicts`
  (contention) have no production writer; the migration says so itself, and the folds degrade to
  affinity=0 / contention=0. ✅ Plurality without selection signal is a catalogue, not a market.
- **`genesis/a2o/features/qahal/plural-mishpat-lenses.feature` is 6 scenarios, all `@wip`.** ✅
- **Mutual credit is not built.** A repo-wide grep for `mutual_credit`, `credit_limit`,
  `MutualCredit`, `creditLimit` returns **zero hits**, and canon says so plainly. ✅ Any sentence of
  the form "the protocol's mutual-credit system does X" is describing an open design question as a
  feature.
- **`medium_of_exchange_id`** — the ValueFlows column where a currency would attach — exists and is
  written `None` at every production call site; the DHT Commitment entry does not carry the field at
  all. The only non-null value anywhere is a test fixture. ✅
- **hREA/ValueFlows interop does not ship.** The hREA DNA role is commented out of `happ.yaml` and
  absent from the built bundle; `bridges/valueflows` is ~1,200 LOC of M1 stub returning 503 with
  hardcoded `agent-fixture-provider` fixtures. ✅

**The live defects a monetary posture inherits.** Three, and the first two are security:

1. **Unauthenticated mint.** `POST /api/v1/token/discernment-mint` accepts a caller-supplied
   `agent_id` and `amount` with no authorization check (`api/token.rs:346-367`). Found independently
   by two of five red-team agents on 2026-08-07 and **re-verified still open 2026-08-23**. ✅
2. **Single-write-capturable ratification.** A percentage-threshold governance action carries no `"m"`,
   so `derive_status` computes `approve >= 0`, and `eligibility_predicate` is `None` at every write
   site — first-quorum-wins, permanently. *"This must be fixed before any claim about
   council-ratified parameters is defensible."* ✅ **Every "the council decides" sentence in this
   paper is contingent on that fix.**
3. **The accidental numéraire.** `μ` with no locality column (§2.7). ✅

**One correction to a prior audit, offered because the corpus asks for it.** The 2026-08-07 red team
found *"supply is monotone by wiring / decay has zero callers."* That has gone stale: `apply_decay`
now has a live HTTP caller (`api/token.rs:112,122` → `handle_apply_decay:374` →
`TokenDecayService::apply_decay:381`). ✅ The tree moved, exactly as that audit warned it would.

**The disposition of the token plane**, which the sibling paper raises and this one must settle. The
Rust token plane ships — balances, mints, transfers, decay — but is **never DHT-anchored**
(`dht_anchor_hash: None` at every write site), declares itself a Category-B projection, has **zero
consumers repo-wide**, and stores balances as `Float`. ✅ Against it, `elohim/elohim-token/README.md`
declares a *"default economic rail… single fungible medium of exchange"* and `src/lib.rs` is a
*"Settlement Bridge Interface… to any settlement chain."*

**Adjudication.** The crate README and the settlement bridge are **superseded by Stance I.2** and
should be retired: a single fungible rail is the one-asset-as-objective-function the stance names, and
a chain settlement bridge re-creates the ownership surface Stance I.1 refuses. The **decay kernel and
the per-layer unit machinery survive** — but as the Hypha survey's Operational-C shape:
**recomputed on read from notarized contribution events, never a stored bank-like balance.** ✅ Today's
SQLite `token_balances` is exactly the stored balance that survey warned against, and the corpus owes
an argument for why a projection-layer balance is not that ledger. This paper's position: a projection
is legitimate only while it is *derivable* — the moment it is authoritative for anything, it is the
ledger, and the null `dht_anchor_hash` means no peer could check it anyway.

**And the structural finding that gates everything above.** There is **no economy habit** among the
twelve, and **zero `@concern:` tags** in `genesis/a2o/features/shefa/` or `/qahal/` — so economics and
governance have no habit-bound proof by construction. All 21 shefa scenarios are `@wip`. ✅ Under this
repo's covenant, none of this paper's takes is schedulable as first-class delivery until a red habit
exists with a runnable check. **The first deliverable is the red, not the spec.**

---

## 6. Verdicts

*(take / study / leave / watch. "Take" means re-implement properly in our architecture.)*

**Take**

1. **Close `binds-policy`** — the coordinator dispatch arm that lets an authored lens actually govern
   an EPR. Nothing in the internal posture is enactable without it, and it is designed already.
2. **The eight never-rules (N1–N8) as `role: floor` / `role: ceiling` lenses**, authored in the same
   market as the policies they bound, so a community reads its own constitution where it reads its
   options.
3. **The three-function decomposition** (§2.2): medium of exchange, store of value, and unit of
   account as *separate* lenses with separate `telos` values and separate declared limits — carrying
   **preservation is legitimate, accretion is not** as the store-of-value limit, and the
   floor/ceiling split (a carry cost is a rule; whether a holding is prudence or accumulation is a
   discernment).
4. **The five-knob lens `rule` schema** (§2.3) as the declaration shape for a circuit, with the
   default-consequence field fixed to the graduated, non-punitive register of Stance IV.2.
5. **The Cantillon detector** (§2.6), in the three-part detector shape, in the Amplified set —
   pointed at ourselves.
6. **Repair the accidental numéraire** — `collective_cid` on the concentration scope key,
   `ExchangeRateView` given the `CollabAgreement` shape, `"algorithm"` source variant deleted (§2.7).
   A defect repair, not a feature.
7. **The bridge-institution pattern with its four invariants** (§4), most of all *no single bridge
   load-bearing*.
8. **Selective legibility as the stated external posture** (§3.4), replacing both illegibility and
   obsolescence-by-hyperinflation.

**Study**

9. **The C3 tax-acceptance mechanism** — the invoice-insurance backing and the tax-receivability
   claim. If it verifies, it is the most important precedent for rung 3 and deserves its own reading.
10. **Fureai Kippu's cross-region transfer** — the one historical case of a care unit deliberately
   crossing a locality boundary, and therefore the one real test of N7's boundary.
11. **Sardex's brokerage economics** — what a human broker actually costs per member, as the
    denominator for the sibling paper's falsification instrument.

**Leave**

12. **The single fungible default rail and its settlement-chain bridge** (§5). Superseded by Stance
    I.2; retire the crate's README claims.
13. **Obsolescence by hyperinflation** (§3.2). Refused on harm-distribution and mechanism grounds.
14. **A protocol-level unit, treasury, or convertibility guarantee** (N1, N6). Not deferred — refused.
15. **A stored, authoritative balance for standing or recognition** (N4). Operational-C recompute-on-
    read, or nothing.

**Watch**

16. **Master-account access for non-traditional institutions** — the single external development that
    most changes §3.5's ladder.
17. **Municipal acceptance of complementary units** for fees or program denomination — rung 3 becoming
    live anywhere.
18. **Stablecoin and CBDC regulation** — because it is where "what is a money transmitter" is
    currently being redefined, and the definition will land on circuits like ours whether or not we
    resemble the thing being regulated.

---

## 7. Outputs — the mint pass

*(Vocabulary, for a cold reader: to **mint** here is to promote a finding into a **cluster row** — one
line in a backlog cluster file citing this paper's `epr:` slug. It has nothing to do with minting
token units, a collision this paper is obliged to disambiguate. The convention is declared in this
directory's `.epr-meta`.)*

Both comparative political-economy documents deliberately minted zero rows, deferring on schema-fit
grounds; their §10 audit retired that precondition, so the successor was expected to close it. **This
paper closes §9 graduation item 1** (the amplified set A1–A5 as Measure-family rows) **for A5**.

*(Row 11 below is the exception: it is **proposed on 2026-08-24 and not yet written** into the
cluster file. It is listed so the mint pass has a mechanical input, not so the table can be read as
complete.)*

**MINTED 2026-08-23** — live, not proposed: `commons-holonic-stewardship-backlog` **rows 21–23**
(`binds-policy`, currency-as-Mishpat-lens with the eight never-rules, the three-function
decomposition) · `measure-family-borrows-backlog` **row 21** (the Cantillon detector) ·
`arch-workspace-discipline-backlog` **row 14** (the legal-person / bridge-institution layer) · and
`2026-08-23-concentration-mu-accidental-numeraire.md` standalone.

| Take | Destination | Row shape |
|---|---|---|
| 1 — `binds-policy` | [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) | The coordinator dispatch arm + one-live-binding-per-`(epr_scope, school)` rule. Designed in the lens-version-DAG spec; **DNA-hash-neutral** (coordinator hot-swap, integrity's per-action table ends `_ => None`). Blocks every other monetary row. |
| 2, 4 — never-rules as floor/ceiling lenses; the five-knob `rule` schema | [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) | A currency circuit as `author-lens` with `role` ∈ {lens, floor, ceiling}. **p2p-design-gate: zero new DHT entry types, zero new commitment actions.** Carries N1–N8 verbatim. |
| 3 — the three-function decomposition | [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) | Three `role`-typed lenses rather than one currency: MoE (`telos`: circulation; carry cost/stock limit legitimate), SoV (`telos`: preservation across time; **accretion refused** — growth-by-holding is unearned increment and therefore common inheritance per Stance I.4), UoA (`telos`: commensuration for a *stated* scope). Gate: the accretion limit must be stated as a `role: ceiling` lens, and the prudence-vs-accumulation judgment must be **excluded** from the mechanical floor. Sibling of Take 2's never-rules; **zero new entry types.** |
| 5 — the Cantillon detector | [measure-family-borrows-backlog](epr:measure-family-borrows-backlog) | A Measure family composing with [middot](epr:middot-measure-primitive-design): issuance joined to first-recipient standing and to lag-until-first-settled-reciprocity. Sibling of A5's issuance-vs-settled-reciprocity ratio; **closes trap-detectors §9 item 1 for A5.** |
| **11 — decision-quality attribution (H5)** ⚠ *proposed 2026-08-24, NOT yet minted* | [measure-family-borrows-backlog](epr:measure-family-borrows-backlog) | A Measure family joining a **pricing or underwriting decision** to the losses that settle after its author has changed scope, so standing cannot be earned on a signal that has not settled. N5's *lag-until-first-settled-reciprocity* redirect, applied to **standing** rather than to units. Sibling of Take 5's Cantillon detector; closes the gap §2.10 names against `MemberRiskProfile`, which scores members and not judgment. |
| 10 — Fureai Kippu cross-region transfer | [measure-family-borrows-backlog](epr:measure-family-borrows-backlog) | The N7 boundary test as a study row, not a borrow: does a care unit that crosses a locality retain its meaning, and what did the Japanese case actually do about it? |
| 7 — the bridge institution | [arch-workspace-discipline-backlog](epr:arch-workspace-discipline-backlog) | Joins the sibling's legal-person row and the two open licensing decisions (rows 2, 9). Carries the four invariants, most of all *no single bridge load-bearing*. |

**Standalone, not folded** (operationally-atomic per `CLUSTERS.md`), each cross-linked to this paper
rather than to a cluster:

- **Unauthenticated `discernment-mint`** (`api/token.rs:346-367`) — re-verified open 2026-08-23.
- **Single-write-capturable ratification** (`governance_action_tally.rs:151-161`,
  `responsibility_demand_configs.rs:143`) — `approve >= 0` with `eligibility_predicate: None`. Gates
  every council claim in both papers.
- **The accidental numéraire** (`concentration_service.rs:47`) — Take 6.
- **The `"uninsurable"` risk tier** (`content_store_integrity/src/lib.rs:1538`) — §2.10. A shipped
  entry type that can price anyone honestly and still exclude them, in a corpus whose justice is
  restored capability and whose test is *those who have been at the bottom of every system*. Named,
  not settled: it is either a floor-layer responsibility (the pool prices, the commons covers) or a
  defect in the entry type, and that is a canon decision rather than a research take.

**Not minted — held for the operator.** The disposition of `elohim/elohim-token` (crate README and
settlement-bridge claims vs Stance I.2, §5) and the retirement of the stale token/blockchain sections
in `shefa.md` and `economic_coordination/epic.md`. These are content decisions on published or
canonical surfaces, not research takes.

**Deliberately not minted — §2.9.** The commons-issuance position mints **nothing**, and that is the
point of it: §2.9 licenses keeping a question open, not building an instrument, and P1–P5 are all
unmet. Minting a build row here would convert a refusal-to-foreclose into a roadmap item, which is
the precise error the section exists to avoid. What it *does* carry into the operator's hands is a
canon question — **whether the corpus's monetary framing should say "never issues" at all**, given
that Stance I.4 makes issuance a common inheritance. That is an operator decision on a canonical
surface, and it is recorded here rather than acted on. The one thing already actioned is editorial:
this paper and its sibling no longer assert permanent incapacity.

**One priority note that is not a row.** Per §5, economics has no habit and no `@concern` tag. Every
row above is un-schedulable as first-class delivery until an economy habit exists with a runnable
check. The natural first red, and the cheapest: **`binds-policy` enacts a lens** — a scenario that
authors a lens, binds it, and shows an economic event refused for violating its `rule`. That single
check would bind the whole internal posture to evidence.

**Takes 8, 9, 11 and all three Watch items die honestly in this prose**, per the cluster discipline.

---

## 8. Method note, credit, and honest limits

**Provenance.** Same as the sibling paper: an operator conversation (Gemini, August 2026, at repo root
as `Proudhon, Marx, and Capital's Threat.txt`) running from Gesell's foreword through Proudhon's
mutual banking, the WIR, Fed credit policy and moral suasion, master accounts, DAO liability, and an
"emergent state by obsolescence." The operator's question that generated *this* paper specifically:
*what is the protocol's internal posture toward currencies as Mishpat policy — diverse as the network
is, and capture-resistant — versus its external posture toward state fiat?* §2 and §3 are that
question answered in its own terms.

**Four operator steering notes (2026-08-23, and one 2026-08-24)** shaped this paper, and each is
recorded because the argument in it is the operator's.

1. **The personal equilibrium.** The Donut's floor/ceiling and friction-gradient limitarianism are
   *our arguments brought to George and Gesell*, not inheritance from them; a capitalist can live in
   right relationship by limiting rent-take to what they would be owed by nature in the
   Georgist/Gesellian end-state; that equilibrium is determinable only through a high-friction,
   high-context negotiation, which is exactly what elohim councils make affordable; and therefore
   *walking humbly* with respect to what one holds is present-tense practice, not abstraction. §2.8
   is that note's monetary consequence — **the equilibrium is a discernment, never a denomination** —
   and it is why N8 exists.
2. **The medium-of-exchange / store-of-value tension**, which generated all of §2.2: that a monetary
   posture must *know, acknowledge, and integrate* the Mishpat limits of each function, rather than
   leaving them ignored and held in adversarial tension at the apex, which is what every central-bank
   and treasury treatment of fiat does. The decomposition, the per-function limit table, and
   **preservation is legitimate, accretion is not** are that note worked out. The observation that the
   Fed's dual mandate *is* the tension carried by one instrument is the operator's; the Keynes-arbitrage
   limit and the floor/ceiling split are the author's additions to keep it honest.
3. **Which abundance.** Material abundance is the end-state genuinely threatening to capital (Gesell
   on Proudhon), but the tradition never asked *what kind* — and most efforts toward abundance have
   ignored our social capacities to lift the natural floor. That note lives principally in the sibling
   paper's §4.6; its monetary consequence here is §2.2's unit-of-account limit, because a unit that
   cannot distinguish abundance-in-calories from abundance-in-housing is precisely the flattening
   Stance I.2 refuses.

4. **The underwriting trap (2026-08-24), from first-hand P&C practice.** That the scale discount is
   only half the story: the deeper defect is *temporal*. Revenue settles before loss, so the
   underwriter who prices a book into the ground is promoted on volume and a successor inherits the
   claims — which makes systemic mispricing an **evolutionary trap** rather than a lapse, since each
   cycle selects for it and staffs the executive from among its winners. Paired with the observation
   that state rate regulation exists to prevent exactly this, mandates minimum adequacy, and *still*
   fails on competitive cycles. §2.10's second half and hazard **H5** are that note worked out; the
   identification of it as **N5 in a non-monetary register** — standing issued on booking rather than
   settlement — is the author's addition.

**Method.** Eight parallel context-isolated readers over the canon spine, the EAE tree, the mishpat
DNA, shefa/REA, compute/trust/reach, the research corpus, the framing guards, and current build state,
each returning graded claims with file:line evidence. That pass materially changed this paper: it
supplied the `author-lens` mechanism (§2.1) that replaced an invented design; it produced the
build-state audit in §5 including the `binds-policy` gap; and it caught that **the two-layer
denomination was already written** in the trap detectors' §10, which this paper now extends and
credits rather than re-deriving as novel (§2.7).

**Grading.** Legend at the head. Everything attributed to this repository is ✅, verified on
`fix/doorway-breaker-trial-theft-and-apps-extraction-herd` on 2026-08-23 — with the standing caveat
that the token plane moves weekly, demonstrated by the `apply_decay` correction in §5. Historical
monetary claims (WIR split settlement and the 1948 demurrage drop, Chiemgauer's parameters, Sardex's
brokerage, Bank of North Dakota, Ooki/bZx posture) are ◐. **The C3 tax-acceptance claim is ⚠ and is
the one external fact this paper would most like verified**, because §3.3 leans on it and Take 8
exists to check it. The Bristol Pound council-tax detail is likewise ⚠. Lietaer's monetary-monoculture
thesis is ◐ and its efficiency–resilience coordinate is excluded entirely.

**Honest limits.**

1. **The internal posture is a grammar, not a currency.** Nothing here demonstrates that a community
   can actually run a circuit on this substrate. `binds-policy` is missing, the lens market's ranking
   signals are dormant, mutual credit has zero implementation, and the whole shefa scenario set is
   `@wip`. This is a design posture adjudicated against a build state that does not yet test it.
2. **Every council claim is contingent on a security fix.** First-quorum-wins ratification (§5) makes
   "the council decides" indefensible as written until repaired. That is not a caveat; it is a
   precondition.
3. **The external posture is untested by contact.** No revenue authority, regulator, or municipality
   has ever been shown any of this. Rung 3 has a precedent elsewhere and none of ours. A posture that
   has never met an adversary is a hypothesis about an adversary.
4. **Selective legibility chooses harms.** §3.4 states the cost rather than resolving it, and
   per-fold anonymity remains an unsolved design constraint.
5. **The register on non-participants.** §3.2's refusal of hyperinflation rests on harm to people
   outside the network. That same standard should be turned on rung 4: an absorption ledger changes a
   municipality's budget, and municipal budgets are how non-participants eat. The refusals in the
   sibling's §5.5 are load-bearing here, not decorative.
6. **The precedents are business-to-business; the protocol is not.** WIR and Sardex — the two cases
   this paper leans on hardest — are B2B networks where credit assessment is institutional, trade
   relationships recur, and a defaulting member has a legal identity and attachable assets. This
   protocol's vocabulary is households, elder care, and neighbours. Household-scale complementary
   currencies have a markedly worse survival record than B2B ones, and nothing here closes that gap;
   Chiemgauer is the household-scale case in the corpus and is far simpler than anything §2 proposes.
   Whether B2B mutual-credit dynamics transfer to household scale is **open, and is the largest
   unexamined assumption in §2**. ⚠ Worse, the household-scale literature carries failure modes the
   five knobs (§2.3) do not model and the never-rules do not touch: **LETS fade rather than fail** —
   participant fatigue, thin offer-side supply, administrative burden concentrating on a few unpaid
   organisers — and **governance capture by the active minority**. Neither appears anywhere in §2's
   design surface. A serious next pass on this paper is a LETS study row, not another B2B precedent.
7. **The mobilisation capacity is an argument, not a design.** §2.9 establishes that a permanent
   refusal to issue is inconsistent with this paper's own commitments, and locates the question in
   primitives the corpus already has — reach, standing, scope, and subsidiarity — rather than in a
   bespoke gate. It does not design the instrument, and hazards **H1–H5 are all open**, H4 (no exit
   at global reach) and H5 (deferred-loss attribution) most seriously. Nothing in §2.9 licenses
   building anything; it licenses keeping the question open and refusing to foreclose it by accident.
8. **The capture answer is structural, not demonstrated.** §2.9 argues that an un-denominated record
   under every denomination is what prevents a hosted medium from becoming the only memory. That
   holds *if* the record stays richer than the lenses reading it — which is an empirical property of
   a running network, not a theorem. A substrate whose events are, in practice, only ever recorded
   because some circuit wanted to price them would have the attenuation back, one layer down, and
   nothing here would detect it. **That detector does not exist**, and it is the most important thing
   §2.9 leaves unbuilt. ⚠
9. **§2.11's composition is the paper's largest claim and its least built.** That the protocol
   occupies the role of underwriting, banking and investment at scale is a *composition* of things
   argued in §2.9–2.10, not a thing anyone has stood up. Its central discipline — an assessment
   available everywhere is not one assessment for everywhere — is protected by nothing but intent,
   since the machinery is identical on both sides of that line. And its honesty depends entirely on
   the standing layer (H5) and the ratification defect (§5), both open. **Scale applied before those
   are fixed is an amplifier, not a repair**, and this paper would rather say so than be quoted as
   having promised the composition. ⚠

### Works referred to

Cited by argument rather than by page; graded in the legend at the head of this paper.

*Monetary and political economy* — Silvio Gesell, *Die natürliche Wirtschaftsordnung* (1916; tr. *The
Natural Economic Order*). Henry George, *Progress and Poverty* (1879). Pierre-Joseph Proudhon,
*Idée générale de la révolution au XIXe siècle* (1851) and the 1849–50 Bastiat exchange on free
credit. J. M. Keynes, *The General Theory* (1936), ch. 23 on Gesell. Bernard Lietaer, *The Future of
Money* (2001); Lietaer, Arnsperger, Goerner & Brunnhuber, *Money and Sustainability: The Missing
Link* (2012); Lietaer & Dunne, *Rethinking Money* (2013). James Stodder on WIR counter-cyclicality
(*Journal of Economic Behavior & Organization*, 2009). Kate Raworth, *Doughnut Economics* (2017).
Ingrid Robeyns, *Limitarianism* (2024).

*Institutions and coordination* — Ronald Coase, "The Nature of the Firm" (1937). Yochai Benkler,
"Coase's Penguin" (2002). Friedrich Hayek, "The Use of Knowledge in Society" (1945). Elinor Ostrom,
*Governing the Commons* (1990). Milgrom, North & Weingast, "The Role of Institutions in the Revival
of Trade" (1990). James C. Scott, *Seeing Like a State* (1998). Morten Jerven, *Poor Numbers* (2013).
Stafford Beer, *Designing Freedom* (1973). Michel Bauwens, Vasilis Kostakis & Alex Pazaitis, *Peer to
Peer: The Commons Manifesto* (2019).

*Legal precedent referred to in §3.7* — CFTC v. Ooki DAO (2022–23) and *Sarcuni v. bZx DAO*, on
unincorporated-association liability for distributed organisations; the Tornado Cash actions, on
developer exposure for publishing infrastructure.

---

**Credit.** [The trap detectors](epr:comparative-political-economy-trap-detectors-2026-08-07) supply
A5 verbatim as N5, traps A6 and A7, the detector shape, the two-layer denomination this paper extends,
the accidental-numéraire finding, and both security findings — the monetary half of this paper is
substantially that document's §10 carried forward.
[The reading program](epr:comparative-political-economy-reading-program-2026-08-07) supplies the
legibility discipline of §3.4 and the *prosumidor* rule.
[Playnet](epr:playnet-free-association-cross-pollination-2026-08-05) supplies the unit-agnostic
steering that keeps §2 from proposing a numéraire, and the closure property that a future circuit's
conservation test should adopt. [Hypha](epr:hypha-dao-autonomous-collectives-cross-pollination-2026-06-24)
supplies N3's capital→voice refusal, the lossy-bridge invariant, and the Operational-C shape that
decides the token plane's disposition in §5.
[Beer](epr:beer-designing-freedom-elohim-critique-2026-06-04) supplies the frame beneath the whole
paper: money *"attenuates the full state space of human contribution down to a few priced channels,
and care is variety that falls outside the channels"* — the answer to which is *"not a better
complaint channel. It is a bigger regulatory language."*
