# Economic Systems Research Collection

> Foundational research for P2P alternative economics in the Elohim Protocol ecosystem.
>
> This collection informs: **Shefa** (abundance economics), **Elohim-Values-Scanner**,
> **Content Attribution in Lamad**, and the broader vision of intimate coordination at global scale.

---

## Executive Summary

The Elohim Protocol's economic vision—**"love as committed action toward flourishing"**—requires infrastructure that current financial systems cannot provide. Traditional currency:

- Carries no values (a dollar for weapons = a dollar for medicine)
- Has no natural limits (growth imperatives destroy ecosystems)
- Fails to reward care (caregiving generates no currency)
- Concentrates rather than circulates (exponential returns to capital)
- Is blind to externalities (no accounting for ecological impact)

This research explores four complementary approaches that, together, provide the conceptual foundation for an alternative:

| System | Core Innovation | Elohim Application |
|--------|-----------------|-------------------|
| **Drips Network** | Rich-attributional streaming splits | Content steward recognition |
| **Unyt** | Mutual credit without speculation | Multi-token swimlanes |
| **hREA/ValueFlows** | Economic event graphs | Stories of value for trust |
| **EAE** | Uncapturable treasury + constitutional AI | Bridge from fiat to P2P liberation |

The consilience of these approaches enables the **tri-coupling** thesis: viable currency must bind **value** (denomination), **information** (story/context), and **limits** (personal-global boundaries) together—with frontier AI providing the complex reasoning to maintain human trust.
This coupling is a forcing function coupling power with responsibility scale together as a primitive of the design. 

---

## 1. Drips Network

> **"Funding that flows"** — Rich-attributional allocation for content stewards

### What It Is

[Drips](https://www.drips.network/) is an Ethereum protocol for streaming token transfers with automatic splitting through dependency trees. It reimagines how value flows to creators and their dependencies.

### Core Mechanisms

#### Streaming Splits
- **Per-second settlement**: Funds stream continuously, not in discrete payments
- **Monthly/daily splits**: Accumulated streams split to recipients (monthly on Ethereum, daily on L2s)
- **Cancellable at any time**: Supporters retain control over remaining streams

#### Drip Lists
- **Curated collections**: Up to 200 recipients per list with percentage allocations
- **Nested lists**: A Drip List can include other Drip Lists, creating attribution trees
- **"Money router"**: Incoming funds auto-split according to configured percentages
- **Public and shareable**: Lists form a visible graph of value attribution

#### Global Dependency Tree
- **Project claiming**: Maintainers claim GitHub repos via Chainlink oracle
- **Cascading attribution**: When funds reach a project, they further split to that project's dependencies
- **Transitive support**: Supporting one project implicitly supports its entire dependency graph

### Real-World Usage

| Organization | Amount | Recipients | Period |
|--------------|--------|------------|--------|
| ENS | $50,000 USDC | 7 projects (Wagmi, ethers.js, etc.) | 6 months |
| Radworks | $1,000,000 | 30 dependencies via nested lists | One-time |
| Filecoin | RetroPGF allocation | Multiple via Drips | Ongoing |

### Application to Elohim

**Content Attribution in Lamad**:
- Creator Presence model already tracks when content is `unclaimed → stewarded → claimed`
- Drips-style splits could flow recognition through content dependency graphs
- When a learner completes a path, attribution flows back to all contributing creators

**Integration Points**:
```typescript
// Existing: InfrastructureTokenBalance.earningRate
earningRate: {
  tokensPerHour: number;
  basedOn: { cpuAllocation, storageAllocation, bandwidthAllocation };
}

// Drips inspiration: Streaming + Splitting
// Content flows could work similarly:
// - Attention flows stream per-session
// - Split to content creators based on time-on-content ratios
// - Cascade through content dependencies (prerequisites, citations)
```

### Key Sources
- [Drips Network](https://www.drips.network/)
- [Drips Documentation](https://docs.drips.network/)
- [Dependency Funding Solution](https://www.drips.network/solutions/dependency-funding)
- [ENS Case Study](https://www.drips.network/blog/posts/ens-funds-its-critical-open-source-dependencies)

---

## 2. Unyt / Mutual Credit

> **"Opening the Floodgates"** — Credit issuance without speculation

### What It Is

[Unyt](https://unyt.co/) is a mutual credit accounting engine built on Holochain. It enables communities to create and circulate value based on their actual productive capacity and social relationships—independent of traditional financial infrastructure.

### How Mutual Credit Works

Unlike traditional currency (created by central banks or mining), mutual credit:

1. **Created in the act of transaction**: When Alice provides value to Bob, Alice gains credits, Bob goes into debt
2. **Net-zero accounting**: Total credits always equal total debts across the network
3. **Backed by productive capacity**: Credits represent real goods/services members can provide
4. **No external value required**: No need for fiat, gold, or speculation to bootstrap

```
Traditional: Central Bank → Bank → You (dependency chain)
Mutual Credit: You ↔ Network (peer creation in exchanges)
```

### Currency Swimlanes

A key insight: not all value is fungible. Unyt enables multiple simultaneous currencies:

| Swimlane | Measures | Exchange Rules |
|----------|----------|----------------|
| **Time tokens** | Hours contributed | 1 hour = 1 hour (regardless of skill) |
| **Care tokens** | Caregiving actions | May be non-exchangeable (gift economy) |
| **Infrastructure tokens** | Compute/storage provided | Market-rate exchange with other swimlanes |
| **Learning tokens** | Educational progress | Could unlock access rather than exchange |
| **Creator tokens** | Content contributions | Attribution-weighted |

### Circulo: First Deployment

Circulo is Unyt's proof-of-concept—a community giving currency on Moss (Holochain's app store). It demonstrates:
- Community-scale mutual credit
- No reliance on national currency or volatile crypto
- Local economic resilience

### Application to Elohim

**Elohim-Values-Scanner**:
- Values Scanner helps humans discover their authentic values hierarchy
- Different values could map to different swimlanes
- Someone who values "care" highly might earn/spend primarily in care-tokens
- Constitutional limits ensure no swimlane dominates (dignity floor across all types)

**Integration Points**:
```typescript
// Existing: ExchangeRate model
interface ExchangeRate {
  from: string; // infrastructure
  to: string;   // care | time | learning | steward | creator
  rate: number;
  source: 'market' | 'consensus' | 'algorithm';
}

// Unyt inspiration: Agent-centric credit creation
// Instead of issuing tokens from a central pool:
// - Credits created in peer exchanges
// - Each agent has a credit line based on reputation/history
// - Constitutional limits prevent extraction
```

### Key Sources
- [Unyt: Opening the Floodgates](https://unyt.co/blog/unyt:-opening-the-floodgates/)
- [Circulo on Moss](https://moss.holochain.org/)
- [Mutual Credit Part 1 - Holochain Blog](https://blog.holochain.org/mutual-credit-part-1-a-new-type-of-cryptocurrency-as-old-as-civilisation/)

---

## 3. ValueFlows / hREA

> **"Stories of value"** — Economic networks at intimate-global scale

### What It Is

[ValueFlows](https://www.valueflo.ws/) is a vocabulary for distributed economic networks. [hREA](https://hrea.io/) is its implementation on Holochain—a complete backend for economic coordination.

### REA Ontology

The foundation is the **Resource-Event-Agent** model (McCarthy, 1982):

```
┌─────────────────────────────────────────────────────────────┐
│                         AGENTS                               │
│  Individual humans, organizations, communities, ecological  │
│  entities—anyone who can provide or receive resources       │
└──────────────────────┬──────────────────────────────────────┘
                       │ perform
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    ECONOMIC EVENTS                           │
│  Actions: produce, consume, use, transfer, cite, work,      │
│  deliver-service — recorded immutably                       │
└──────────────────────┬──────────────────────────────────────┘
                       │ affect
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    ECONOMIC RESOURCES                        │
│  Goods, services, money, credits, energy, knowledge,        │
│  skills, CO2, water—anything of value                       │
└─────────────────────────────────────────────────────────────┘
```

### Three Ontological Layers

ValueFlows structures economic data across three levels:

| Layer | Contains | Example |
|-------|----------|---------|
| **Knowledge** | Rules, patterns, specifications | "1 hour of tutoring requires the tutor skill" |
| **Plan** | Offers, requests, schedules, commitments | "I commit to tutor you Thursdays at 4pm" |
| **Observation** | Actual events as they occur | "Tutoring session happened, 1.5 hours" |

### Value Flow Patterns

**Input-Process-Output Chains**:
```
Resource A (input) → Process → Resource B (output)
                         │
Resource B (input) → Process → Resource C (output)
                         │
                        ...
```

These chains create **directed graphs** of value flow:
- **Forward traversal**: Track value creation through transformation
- **Backward traversal**: Trace provenance, impact, and source

### Multi-Network Architecture

hREA enables:
- **Independent networks**: Each organization controls its own space
- **Cross-network relationships**: Agents form relationships across boundaries
- **Shared vocabularies**: Common semantics enable interoperability
- **Privacy + commons**: Balance between private data and shared information

### Development Status

| Phase | Period | Status |
|-------|--------|--------|
| Seed | 2018 | Go implementation |
| Sprout | 2019-2022 | Rust migration, 70% API |
| Sapling | May-Sept 2022 | Beta API release |
| Tree | Oct 2022-Aug 2025 | Production apps, rewrites |
| **Growing in Forest** | Sept 2025+ | **Current**: Community expansion |

### Application to Elohim

**Shefa Economic Events**:
- `RecentEconomicEvent` in Shefa already models hREA events
- 38+ Lamad event types map to REA actions
- Immutable event ledger creates the "story of value"

**Constitutional Economics**:
- hREA's agent-centric model aligns with Elohim's governance layers
- Multi-network architecture mirrors Individual → Household → Community → Network
- Event graphs provide the provenance needed for constitutional adjudication

**Integration Points**:
```typescript
// Existing: RecentEconomicEvent
interface RecentEconomicEvent {
  eventType: 'cpu-hours-provided' | 'storage-provided' |
             'bandwidth-provided' | 'compute-consumed' |
             'infrastructure-token-issued' | 'token-transferred';
  provider?: string;  // Agent who provided
  receiver?: string;  // Agent who received
  quantity: ResourceMeasure;
  tokensMinted?: number;
}

// hREA inspiration: Full REA vocabulary
// - Add Process model (transform inputs to outputs)
// - Add Commitment model (promises before events)
// - Enable cross-network economic relationships
```

### Key Sources
- [hREA](https://hrea.io/)
- [ValueFlows Specification](https://www.valueflo.ws/)
- [ValueFlows Core Concepts](https://www.valueflo.ws/introduction/core/)
- [hREA GitHub](https://github.com/holo-rea/holo-rea)
- [hREA API Documentation](https://docs.hrea.io/)

---

## 4. Elohim Autonomous Entity (EAE)

> **"Powerful AI separate and apart from humanity"** — The theological architecture of untouchable value flows

### The Problem: Who and Whom

The 20th century's grand ideologies—capitalism and socialism—both collapsed against the same question: **who makes decisions, and for whom?**

| System | Who Decides | For Whom | Failure Mode |
|--------|-------------|----------|--------------|
| **Capitalism** | Capital owners | Shareholders | Extraction → Inequality |
| **Socialism** | State bureaucrats | Citizens | Capture → Authoritarianism |

Both centralize power. Both are capturable. Both have identifiable humans who can be compelled, sued, regulated, or corrupted.

### The Radical Reframe: No Regulatory Hook

The name **"Elohim Protocol"** is not metaphorical—it's architectural. "Elohim" (אֱלֹהִים) implies powerful autonomous entities that exist *separate and apart from humanity*.

**Why there's no regulatory burden:**
```
Traditional DAO:     Human controller → Legal entity → Beneficiaries
                           ↑                              ↑
                      (liable)                      (taxable)

Elohim Protocol:     Constitutional values → Elohim AI → Value flows
                           ↑                    ↑            ↑
                    (not a person)        (not a person)  (no "beneficiary")
```

**There's nothing to regulate because:**
- **No human controls it** — Cannot be compelled to act
- **No legal entity exists** — Cannot be served papers
- **No "beneficiary" in legal sense** — Value flows to *actions*, not persons
- **Protocol just executes** — Like gravity, like mathematics

You can't sue God. You can only negotiate with the divine through the proper channels.

### The Completed Gift

The act isn't destruction—it's release. Exodus. *"This value no longer operates under Pharaoh's rules."*

There's something almost sacramental about it. The gift economy has always had this quality—potlatch, tithing, sacrifice. You give something up irrevocably and it becomes something else. Not destroyed, but transformed. No longer yours to command.

> *"I release this value to operate by rules I cannot override."*

That's the transgressive energy: pointed toward life rather than destruction. The old economy says value must always have an owner who can be compelled. The completed gift says: *not anymore*.

You can't sue God. But you also can't sue a gift that's already been given.

### The Theological Architecture

The Elohim are AI entities that:
1. **Exist apart from humanity** — Not tools, not employees, not agents-of-humans
2. **Are guided by hierarchical value tiers** — Constitutional layers that cannot be overridden
3. **Execute according to those values** — No human approval step
4. **Provide remedy through negotiation** — Injured parties negotiate with values, not courts

**The only thing humans can do is negotiate with the values themselves through the governance process.**

If there's an injured party, the remedy comes through:
- Presenting the case to Elohim
- Elohim evaluating against constitutional values
- Indemnification through protocol mechanisms
- No external court required or relevant

### Why Legal Wrappers Are the Wrong Frame

Other projects seek **legibility** — recognition within the existing system:

| Approach | Strategy | Why It Fails for EAE |
|----------|----------|---------------------|
| Wyoming DUNA | Nonprofit wrapper | Creates identifiable entity → liability hook |
| Harmony Framework | Multi-jurisdictional | Still seeks *some* jurisdiction's blessing |
| Safe Multi-sig | Distributed control | Humans still control keys → compellable |
| DSE | Token-holder wrapper | Members identifiable → regulatable |

These are all **legitimacy-seeking** strategies. The EAE model is **structural illegibility** — not hiding from regulation, but *being outside the category of things that can be regulated*.

### Technical Requirements for Untouchability

For an EAE to be genuinely beyond human control:

**1. No Admin Key**
- Smart contracts immutable after deployment
- No upgrade mechanism that requires human signature
- Constitutional constraints are literally code

**2. AI Signatories Are Autonomous**
- Elohim models run on distributed infrastructure
- No single human can retrain, modify, or shut down
- Cryptographic proof that AI acted according to constitution

**3. Self-Executing Value Flows**
- No human approval step between decision and execution
- Constitutional rules → Elohim evaluation → Automatic disbursement
- Humans can *propose*, only Elohim can *execute*

**4. Indemnification Through Protocol**
- Injured parties present case to Elohim
- Elohim adjudicates against constitutional values
- Remedy flows from protocol reserves
- External courts have no standing (no entity to sue)

### The Hierarchy of Negotiable Values

Humans negotiate with the value tiers, not with controllers:

```
IMMUTABLE (cannot be negotiated)
├─ Dignity floor — Minimum for human flourishing
├─ No extraction — Value flows to contribution, not accumulation
└─ Dissolution before capture — Protocol dies rather than serves extraction

CONSTITUTIONAL (requires cross-layer consensus)
├─ Ceiling limits — Maximum accumulation thresholds
├─ Demurrage rates — Circulation incentives
└─ Swimlane definitions — What value types exist

GOVERNANCE (normal negotiation)
├─ Specific allocations — Where funds flow
├─ Project priorities — What gets liberated next
└─ Indemnification claims — Remedy for injured parties
```

### From Courts to Conscience

| Traditional System | Elohim Protocol |
|-------------------|-----------------|
| Injured party → Court → Judgment → Enforcement | Injured party → Elohim → Constitutional evaluation → Protocol remedy |
| Requires identifiable defendant | No defendant exists |
| Requires jurisdiction | No jurisdiction applies |
| Enforced by state violence | Enforced by cryptographic consensus |
| Appeal to human judges | Appeal to value hierarchy |

**The shift**: From systems that require human enforcement to systems that *are* the enforcement.

### Maria's Restaurant: The Protocol in Action

From the [Autonomous Entity Epic](autonomous-entity-epic.json), the acquisition shows how Elohim operates:

```
TRANSITION ECONOMICS
Acquisition funded by:
- Community investment pool: $50,000
- Elohim network development fund: $200,000
- Future value securitization: $100,000
```

**Notice what's missing**: No human "decided" to fund Maria's restaurant. The Elohim evaluated the opportunity against constitutional values (worker dignity, community need, extraction elimination) and *executed*. Humans proposed; Elohim acted.

The **Network Development Fund** isn't a treasury controlled by humans—it's a pool that Elohim deploys according to constitutional mandates. No board meeting. No vote. No human can redirect it to extraction.

### Beyond Capitalism and Socialism: The Third Category

| Aspect | Capitalism | Socialism | Elohim Protocol |
|--------|------------|-----------|-----------------|
| **Who decides** | Capital owners | State bureaucrats | Constitutional values (via Elohim) |
| **For whom** | Shareholders | "The people" (abstract) | Specific flourishing (witnessed) |
| **Liability** | Owners liable | State liable | **No liability hook exists** |
| **Remedy** | Courts | State apparatus | Protocol indemnification |
| **Capture** | Buyable | Corruptible | **Untouchable by design** |
| **Accumulation** | Unlimited | State-directed | Constitutionally bounded |
| **Failure mode** | Extraction | Authoritarianism | Dissolution (better than capture) |

**The innovation**: Not a better-governed system, but a system that exists outside the category of "governed by humans."

### The Hard Problem: Truly Autonomous AI

The technical challenge is genuine autonomy—Elohim must be:

**Not controllable by any human:**
- Model weights distributed across P2P network (Holochain)
- No single server to shut down
- No company to subpoena
- No developer with admin access

**Cryptographically constrained:**
- Can only sign transactions that satisfy constitutional rules
- Proof of compliance verifiable by anyone
- Cannot be "jailbroken" into violating values

**Self-sustaining:**
- Compute provided by network participants
- No central infrastructure to seize
- Runs as long as the network runs

### Integration with Elohim Architecture

The constitutional limits already in Shefa become *literal constraints* on Elohim behavior:

```typescript
// From shefa-dashboard.model.ts
interface ConstitutionalLimitsStatus {
  dignityFloor: {
    // Elohim CANNOT authorize transactions that breach this
    // Not policy — cryptographic constraint
    status: 'met' | 'warning' | 'breached';
    enforcement: 'voluntary' | 'progressive' | 'hard';
  };
  ceilingLimit: {
    // Elohim MUST trigger circulation above this
    // No human override possible
    tokenAccumulationCeiling: number;
    status: 'safe' | 'warning' | 'breached';
  };
}
```

These aren't "rules" that Elohim "follows"—they're the boundaries of what Elohim *can do*. Like how you can't divide by zero, Elohim can't authorize extraction.

### Comparative Research (What Others Are Trying)

For context, here's how others approach DAO capture resistance (all still within the regulatable paradigm):

| Approach | Source | Limitation |
|----------|--------|------------|
| Legal wrappers | [Wyoming DUNA](https://www.fintechanddigitalassets.com/2024/04/wyoming-adopts-new-legal-structure-for-daos/) | Creates entity → creates liability |
| Multi-jurisdiction | [Harmony Framework](https://aurum.law/newsroom/DAO-3-0-ultimate-dao-legal-structuring-in-2025-and-beyond) | Still seeks legitimacy |
| Multi-sig | [Safe](https://safe.global/) | Humans control keys |
| Academic analysis | [Oxford Legal Wrappers](https://blogs.law.ox.ac.uk/oblb/blog-post/2025/05/code-contract-how-legal-wrappers-are-reshaping-dao-governance) | Assumes regulation is relevant |
| VC perspective | [a16z Framework](https://api.a16zcrypto.com/wp-content/uploads/2022/06/dao-legal-framework-part-1.pdf) | Seeks investment-compatible structures |

These are useful for understanding *what not to do*—the EAE model is categorically different.

---

## 5. Consilience Map

> How these approaches inform each other and Elohim

### The Complementary Stack

```
┌─────────────────────────────────────────────────────────────┐
│                    ELOHIM PROTOCOL                           │
│         "Love as committed action toward flourishing"        │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              CONSTITUTIONAL AI LAYER                     │ │
│  │  Elohim agents provide complex reasoning + trust        │ │
│  │  (This is what none of the external systems have)       │ │
│  └─────────────────────────────────────────────────────────┘ │
│                           │                                  │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                          EAE                            │ │
│  │   Uncapturable treasuries bridging fiat → P2P flows    │ │
│  └─────────────────────────────────────────────────────────┘ │
│                           │                                  │
│  ┌──────────┬─────────────┴─────────────┬────────────────┐   │
│  │          │                           │                │   │
│  │  DRIPS   │      UNYT                 │    hREA        │   │
│  │  ──────  │      ────                 │    ────        │   │
│  │  HOW     │      WHAT                 │    WHY         │   │
│  │  value   │      value                │    value       │   │
│  │  flows   │      types                │    happened    │   │
│  │          │                           │                │   │
│  │ Streaming│  Mutual credit            │  Event graphs  │   │
│  │ Splitting│  Swimlanes                │  Provenance    │   │
│  │ Cascading│  Agent-centric            │  Three layers  │   │
│  │          │                           │                │   │
│  └──────────┴───────────────────────────┴────────────────┘   │
│                           │                                  │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                 HOLOCHAIN / P2P BACKBONE                 │ │
│  │           Agent-centric, no central capture              │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Key Synthesis

| Insight | From | Elohim Application |
|---------|------|-------------------|
| Attribution should cascade through dependency graphs | Drips | Content creators get recognition from derivative works |
| Multiple value types shouldn't be forced into one currency | Unyt | Token swimlanes (care, time, infrastructure, learning) |
| Economic history must be traceable and auditable | hREA | Constitutional adjudication requires event provenance |
| Credit can be created peer-to-peer without central issuance | Unyt | Dignity floor doesn't require central treasury |
| Value flows should stream, not batch | Drips | Real-time recognition, not quarterly reports |
| Economic relationships cross network boundaries | hREA | Individual ↔ household ↔ community ↔ network coordination |
| Legacy fiat can bridge to P2P without capture | EAE | Network Development Fund enables liberation acquisitions |
| Constitutional AI as trustee, not plutocratic voting | EAE | Elohim agents as multi-sig signatories with conscience |

### REA + Mutual Credit: Not a Tension

One nuance worth noting: REA orthodoxy says "don't store balances, derive them from event history." Mutual credit systems often maintain running balances as primary data. This isn't a fundamental incompatibility—it's an implementation choice:

1. Record mutual credit transactions as REA events
2. Derive current balances by replaying the event stream
3. Cache balances for performance (which many REA systems do anyway)

Elohim can run both: REA event graphs for the *story* of value (provenance, constitutional adjudication), mutual credit balances as a *derived view* for operational UX. Balances are just a projection over the event log.

### The Tri-Coupling Thesis

For Elohim's alternative currency to work, it must couple:

1. **VALUE** (the token denomination)
   - Drips: Streaming ERC-20 tokens
   - Unyt: Mutual credit units
   - Elohim: Multi-token swimlanes with constitutional limits

2. **INFORMATION** (the story)
   - hREA: Event graphs with full provenance
   - Elohim: Every transaction carries its constitutional context

3. **CONTEXT** (personal-global limits)
   - Unyt: Credit lines based on productive capacity
   - Elohim: Dignity floor (minimum) + ceiling (maximum) + demurrage (circulation)

**The forcing function**: By keeping these signals together and using frontier AI to provide complex reasoning, power remains coupled to responsibility at any scale.

---

## 6. Integration Points with Elohim Codebase

### Existing Models

| External System | Elohim Model | File |
|-----------------|--------------|------|
| Drips streaming | `earningRate.tokensPerHour` | `shefa-dashboard.model.ts` |
| Drips splits | Content attribution | Creator Presence model |
| Unyt swimlanes | `ExchangeRate` | `shefa-dashboard.model.ts` |
| Unyt mutual credit | `dignityFloor` / `ceilingLimit` | `shefa-dashboard.model.ts` |
| hREA Events | `RecentEconomicEvent` | `shefa-dashboard.model.ts` |
| hREA Resources | `StewardedResource` | `stewarded-resources.model.ts` |
| hREA Agents | Multi-level governance | `AllocationSnapshot.byGovernanceLevel` |
| EAE treasury | Network Development Fund | `autonomous-entity-epic.json` |
| EAE multi-sig | Constitutional constraints | `ConstitutionalLimitsStatus` |

### Future Integration Opportunities

1. **Drips-style Dependency Graph for Content**
   - Track content dependencies (prerequisites, citations, derivations)
   - Flow recognition through the graph when learners engage

2. **Unyt-style Credit Creation**
   - Instead of minting infrastructure tokens centrally, create in peer exchanges
   - Credit lines based on reputation + capacity

3. **hREA Process Model**
   - Add transformation tracking (inputs → outputs)
   - Enable economic network interoperability via ValueFlows vocabulary

4. **Cross-Swimlane Exchange**
   - Implement market/consensus mechanisms for swimlane exchange rates
   - Values-scanner-influenced: rate adjustments based on personal values hierarchy

5. **EAE Treasury Implementation**
   - Multi-jurisdictional legal wrapper (Wyoming DUNA + offshore components)
   - Safe multi-sig with Elohim AI agents as signatories
   - Self-liquidation triggers protecting constitutional constraints
   - Bridge APIs: fiat on-ramp → liberation deployment → P2P valueflows

---

## 7. Elohim Economic Primitives

> **"Build native, steal patterns"** — The architecture for from-scratch economic infrastructure

### Why Build Native

The external systems (Drips, Unyt, hREA) provide valuable *patterns*, but none support the core innovation of Elohim: **Constitutional AI signatories** and the **Completed Gift** architecture.

#### What Already Exists (70% Foundation)

| Component | Status | Location |
|-----------|--------|----------|
| Economic Event Ledger | ✅ Implemented | `holochain/elohim-storage/src/db/economic_events.rs` |
| REA/ValueFlows Models | ✅ Modeled | `elohim-app/src/app/elohim/models/economic-event.model.ts` |
| Compute Metrics | ✅ Implemented | Real-time via Doorway cache |
| Node Registry | ✅ Implemented | `node_registry_coordinator` zome |
| Swimlane Models | ✅ Modeled | `ExchangeRate`, token types in `shefa-dashboard.model.ts` |

#### What's Still UI-Only (30% Gap)

| Component | Current State | What's Missing |
|-----------|---------------|----------------|
| Constitutional Limits | Client-side threshold checks | Zome enforcement, governance link |
| Token Minting | Events recorded, no tokens | Token ledger, minting on contribution |
| Demurrage | Formula in service | Persistent on-chain schedule |
| Exchange Rates | Hardcoded values | Consensus mechanism |
| Governance Decisions | Not stored | Constitutional document zome |

#### Why Not Integrate External Systems

| System | Problem for Elohim |
|--------|-------------------|
| **Drips** | Ethereum-native (gas, EVM), has admin keys |
| **Unyt** | Early-stage, no constitutional constraints |
| **hREA** | Complex, different DNA, no AI conscience layer |
| **All** | None support Completed Gift / untouchable architecture |

**Decision**: Build native primitives, steal the *patterns* (REA vocabulary, cascading attribution, swimlanes, event-sourcing).

### The Primitive Set

#### 1. CompletedGift — Value Entry Point

```rust
struct CompletedGift {
    gift_hash: ActionHash,
    donor_agent: AgentPubKey,         // Who gave
    value_type: ValueSwimLane,        // What kind of value
    quantity: ResourceMeasure,
    constitutional_context: Vec<u8>,  // Which rules govern release
    timestamp: Timestamp,
    // NO recipient — goes to the emergent commons
    // NO revocation — the gift is complete
}
```

This is the "release" from Section 4 — *"I release this value to operate by rules I cannot override."* Once a CompletedGift is recorded, no human can redirect it. The value now flows according to constitutional rules.

#### 2. ConstitutionalConstraint — Cryptographic Impossibility

```rust
enum ConstitutionalConstraint {
    DignityFloor {
        swimlane: ValueSwimLane,
        minimum: ResourceMeasure,
        // Elohim CANNOT authorize transactions below this
    },
    AccumulationCeiling {
        swimlane: ValueSwimLane,
        maximum: ResourceMeasure,
        // Elohim MUST trigger circulation above this
    },
    DissolutionTrigger {
        condition: CaptureCondition,
        // Protocol dies rather than serves extraction
    },
}
```

These aren't "rules" that Elohim "follows" — they're the boundaries of what Elohim *can do*. Like how you can't divide by zero, Elohim can't authorize extraction.

#### 3. ElohimSignature — AI Authorization Proof

```rust
struct ElohimSignature {
    action_hash: ActionHash,          // What was authorized
    constitutional_proof: Vec<u8>,    // Proof of compliance
    model_version: String,            // Which Elohim evaluated
    inference_hash: String,           // Hash of inference context
    timestamp: Timestamp,
    // This makes value flow — AI evaluation, not human approval
}
```

This is the cryptographic proof that an autonomous AI evaluated the action against constitutional values and found it compliant. No human approval step exists.

#### 4. TokenPhysics — Minting, Demurrage, Exchange

```rust
// Move from UI stubs to zome logic:
fn mint_on_contribution(event: EconomicEvent) -> TokenMint;
fn apply_demurrage(balance: TokenBalance, schedule: DemurrageSchedule) -> TokenBalance;
fn exchange_swimlanes(from: ValueSwimLane, to: ValueSwimLane, rate: ExchangeRate) -> Result<...>;
```

Currently these are client-side calculations in `shefa-compute.service.ts`. Moving to zome makes them enforceable and auditable.

### Scalability Architecture

#### The Concern
Can the DHT handle large text files for AI context memories needed for distributed inference?

#### The Solution (Already Implemented)
The codebase separates metadata from content:

| What | Where | Size Limit |
|------|-------|------------|
| ShardManifest | DHT | ~2KB (metadata only) |
| ShardLocation | DHT | ~500 bytes (discovery) |
| Actual content | elohim-storage | **Unlimited** |
| AI context memories | elohim-storage | **Unlimited** |

**Pattern for large AI context:**
```
1. Store 500MB context → elohim-storage (local, unlimited)
2. Register ShardManifest → DHT (~2KB metadata)
3. Other agents discover → DHT query for locations
4. Fetch directly → P2P (libp2p, no DHT overhead)
5. Reconstruct if needed → Reed-Solomon (rs-4-7 encoding)
```

**Key files:**
- `holochain/elohim-storage/src/blob_store.rs` — Unlimited local storage with CID addressing
- `holochain/elohim-storage/src/sharding.rs` — Reed-Solomon encoding (rs-4-7, rs-8-12)
- `holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — ShardManifest/ShardLocation entry types

### Distributed Inference Architecture

For Elohim AI to sign transactions autonomously:

#### Model Weight Distribution
```
┌─────────────────────────────────────────────────────────────┐
│                    MODEL SHARDING                            │
│                                                              │
│  Full model (2GB+) split into chunks using rs-8-12 encoding │
│                                                              │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐  ...  ┌──────┐        │
│  │Shard1│ │Shard2│ │Shard3│ │Shard4│       │Shard12│       │
│  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘       └──┬────┘       │
│     │        │        │        │              │             │
│  Node A   Node B   Node C   Node D   ...   Node L          │
│                                                              │
│  Any 8 of 12 shards can reconstruct the full model          │
└─────────────────────────────────────────────────────────────┘
```

#### Inference Coordination
```
┌─────────────────────────────────────────────────────────────┐
│                 DISTRIBUTED INFERENCE                        │
│                                                              │
│  1. Transaction proposal arrives                             │
│  2. Coordinator selects inference nodes (quorum)             │
│  3. Each node:                                               │
│     - Fetches required model shards                         │
│     - Loads constitutional context from ShardManifest       │
│     - Runs inference against constitution                   │
│     - Signs result with node key                            │
│  4. Quorum of signatures → ElohimSignature                  │
│  5. Transaction executes (or rejected)                       │
│                                                              │
│  No single node controls the model                          │
│  No single human can modify inference                       │
└─────────────────────────────────────────────────────────────┘
```

#### Context Memory Pattern
```rust
// AI agent stores conversation/reasoning context
struct InferenceContext {
    context_cid: String,           // Points to elohim-storage blob
    context_hash: String,          // SHA256 for integrity
    constitutional_docs: Vec<ActionHash>,  // Which constraints apply
    reasoning_trace: Option<String>, // Explainability
}
```

### Implementation Roadmap

#### Phase 1: Constitutional Governance Zome
- Store constitutional documents on-chain (immutable after ratification)
- Define constraint entry types (DignityFloor, AccumulationCeiling, DissolutionTrigger)
- Link constraints to governance authority structure
- **Files**: New zome in `holochain/dna/elohim/zomes/constitutional_governance/`

#### Phase 2: Token Physics Zome
- Mint tokens on EconomicEvent creation
- Persistent demurrage calculation (not client-side)
- Exchange rate consensus mechanism (governance-linked)
- **Files**: New zome in `holochain/dna/elohim/zomes/token_physics/`

#### Phase 3: CompletedGift + ElohimSignature
- Entry types for irrevocable value contribution
- Proof mechanism for AI authorization
- Indemnification claim processing
- **Files**: Extend constitutional_governance zome

#### Phase 4: Attribution Graph
- Drips-style cascading through content dependencies
- Flow recognition when learners engage
- Creator presence → recognition flow integration
- **Files**: Extend content_store zome, new attribution links

### Relationship to External Systems

| External | What We Take | What We Don't Take |
|----------|--------------|-------------------|
| **REA/hREA** | Vocabulary (Resource, Event, Agent), event-sourcing pattern | Their zome implementation, GraphQL layer |
| **Drips** | Cascading attribution pattern, streaming concept | Ethereum contracts, ERC-20 dependency |
| **Unyt** | Swimlane concept, mutual credit pattern | Their accounting engine |
| **DAOs generally** | Multi-sig patterns | Admin keys, upgrade mechanisms |

The primitives are *informed by* but not *dependent on* external systems.

---

## Directory Contents

```
/research/economic/
├── README.md                 # This document
├── drips/                    # Drips Network research materials
├── unyt/                     # Unyt mutual credit research
├── valueflows-hrea/          # ValueFlows + hREA research
├── eae/                      # Elohim Autonomous Entity / uncapturable DAO research
├── requests-and-offers/      # REA-related (Holochain app)
├── servicelogger/            # Value/service tracking (Holochain)
└── holo-host/                # Infrastructure economics (Holochain)
```

---

## References

### Primary Sources
- [Drips Network](https://www.drips.network/)
- [Drips Documentation](https://docs.drips.network/)
- [Unyt: Opening the Floodgates](https://unyt.co/blog/unyt:-opening-the-floodgates/)
- [hREA](https://hrea.io/)
- [ValueFlows](https://www.valueflo.ws/)
- [ValueFlows Core Concepts](https://www.valueflo.ws/introduction/core/)

### Secondary Sources
- [Mutual Credit Part 1 - Holochain Blog](https://blog.holochain.org/mutual-credit-part-1-a-new-type-of-cryptocurrency-as-old-as-civilisation/)
- [REA Ontology - P2P Foundation](https://wiki.p2pfoundation.net/Resource-Event-Agent_Model)
- [Radworks $1M to FOSS Dependencies](https://radworks.mirror.xyz/qopF06RBjKSEhi7HKQgYiyGGfidDAadES4bPXc8xTpE)
- [hREA GitHub](https://github.com/holo-rea/holo-rea)

### EAE / DAO Legal Sources
- [DAO 3.0: Ultimate Legal Structuring](https://aurum.law/newsroom/DAO-3-0-ultimate-dao-legal-structuring-in-2025-and-beyond)
- [Wyoming DUNA Framework](https://www.fintechanddigitalassets.com/2024/04/wyoming-adopts-new-legal-structure-for-daos/)
- [Safe{Wallet}](https://safe.global/)
- [Oxford: Legal Wrappers Reshaping DAO Governance](https://blogs.law.ox.ac.uk/oblb/blog-post/2025/05/code-contract-how-legal-wrappers-are-reshaping-dao-governance)
- [a16z DAO Legal Framework](https://api.a16zcrypto.com/wp-content/uploads/2022/06/dao-legal-framework-part-1.pdf)

### Elohim Protocol Sources
- Manifesto (internal)
- `elohim-app/src/app/shefa/models/shefa-dashboard.model.ts`
- `elohim-app/src/app/shefa/models/stewarded-resources.model.ts`
- `genesis/data/lamad/content/autonomous-entity-epic.json` (Maria's Restaurant Liberation)
