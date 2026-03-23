# Distributed Trust & Reach — Four-Layer Content Access Model

**Date:** 2026-03-23
**Status:** Approved
**Scope:** Architectural design for how content reaches humans through distributed trust, earned attestations, and stewardship compute — without centralized hosting or gatekeeping.

## The Problem the Internet Has

On the current internet, content access has two states: visible or not visible. Paywall or free. The platform decides who sees what, driven by engagement algorithms and advertising economics. Hosting costs scale with popularity — the more people your content reaches, the more you pay. Distribution is a zero-sum competition for attention.

## What the Elohim Protocol Replaces It With

Content access is graduated, earned, and distributed. Reach is not a gate — it's a radius of discoverability sustained by community trust. Hosting costs are zero for creators because the people who care about the content ARE the infrastructure. The protocol makes the natural bonds of mutual aid — sharing what you love, teaching what you know, caring for what matters — legible and durable.

## Four Layers of Content Access

### Layer 1: Discovery (Reach — Ambient)

*Can this content exist in my world?*

Reach determines whether content is discoverable to a given peer. It's not checked per-request — it's ambient in the connection context. When two peers establish a P2P connection, they negotiate their trust context once:

- Agent identity (pubkey, verified)
- Collective memberships (CIDs of participation entries, verified against DHT)
- Relationships with stewards (CIDs of relationship entries, verified against DHT)

This establishes an **ambient reach ceiling** for the connection. All content at or below that ceiling is discoverable — EPR Heads flow freely. The peer sees titles, descriptions, path structure. The content exists in their world.

**Reach levels** (protocol-notarized, immutable):
- `commons` / `public` — no context needed, content reaches everyone
- `community` — collective membership with consented participation
- `familiar` — shared collective with a content steward
- `trusted` — direct relationship with a steward at trusted intimacy
- `intimate` — mutual intimate relationship with a steward
- `self` / `private` — you are the creator

**Earning reach**: Content starts with minimal reach. It earns higher reach through attestations — author-verified, steward-approved, community-endorsed, peer-reviewed, governance-ratified, safety-reviewed. The elohim ratify the governance decision. Reach is earned, not declared.

### Layer 2: Consumption (Attestation Gates — Per-Content)

*Have I earned access to this specific content?*

Once content is discoverable (Layer 1), the body may require specific attestations. A learning path step might require mastery in the prerequisite concept. An advanced module might require completion of the foundation path. This is per-content gating, not ambient context.

The gate check is local: does the requesting agent hold an attestation CID that satisfies the content's requirements? The attestation lives on the DHT — the CID is the credential. No token issuance, no bearer tokens. The DHT is the authority.

Revocation propagates via DHT gossip (200-2000ms). Immediate revocation for safety concerns is a future capability via governance signal protocol.

### Layer 3: Participation (Active Engagement)

*Can I interact with this content beyond reading?*

Different from consumption. Can I take the assessment? Submit a reflection? Leave a response? Contribute a review?

A commons article might be readable by everyone but only community members can comment. An assessment might be viewable but only attested learners can submit responses. The sophia layer (assessment engine) gates active engagement based on identity mode and attestation state.

### Layer 4: Replication (Stewardship Compute — The Infrastructure)

*By caring about this content, I become part of its infrastructure.*

This is where the economic model inverts. On the current internet, the creator pays for hosting. On the elohim protocol, the people who care about the content provide the hosting through stewardship compute.

**The podcast example:**

A podcaster creates 400 episodes. They have:
- **400 stewards** — people who care enough to commit stewardship compute. 20% of their node's capacity is pledged across family, friends, faith community, and interests. The podcaster gets a slice of each steward's budget. These stewards store shards (Reed-Solomon 4+3), serve EPR Heads, respond to shard fetch requests, and validate attestations. Their nodes ARE the CDN.
- **20,000 regular listeners** — consume via EPR resolution. Discover episodes through DHT, fetch from whichever steward is closest, listen. Recognition flows back to the creator proportional to steward ratios.
- **90,000 unique visitors** — web2 visitors through a doorway. Commons reach means the doorway serves without auth. The doorway resolves from stewards via P2P if not cached locally.

**Creator's hosting cost: zero.** The 400 stewards provide the compute from hardware they already own. The stewardship isn't a subscription or a donation — it's the natural allocation of care. The same way your bookshelf fills with books you love, your node fills with content you steward.

**The natural progression:**
1. You discover content. It reaches you because of your community context.
2. You consume it. Your node caches it naturally.
3. You allocate stewardship compute. Your budget reflects what you care about.
4. You earn mastery through sustained engagement.
5. Mastery → attestation. You can now vouch for the content's quality.
6. Attestation → peer review / curation. You contribute to the content's trust.
7. If the original creator moves on, stewardship passes to those who earned it through sustained care. (`contribution_type: "inherited"`)

This isn't patronage. It's not a transaction. It's the natural deepening of relationship with knowledge, made durable by the protocol.

## Trust Negotiation Between Unknown Peers

When Peer A requests content from Peer B, and Peer B has never seen Peer A:

### Per-Connection: Ambient Context Establishment

On P2P connection (via libp2p identify exchange), peers present:
1. **Agent pubkey** — cryptographic identity, verifiable
2. **Collective membership CIDs** — content-addressed participation entries on the DHT
3. **Relationship CIDs** — content-addressed relationship entries linking to stewards

Peer B verifies these CIDs against the DHT (one-time lookup, cached). This establishes the ambient reach ceiling for the connection. All EPR Heads within that ceiling flow without per-request checks.

### Per-Content-Fetch: Attestation Verification

When Peer A fetches content bytes (Tier 3), the serving peer checks:
1. Content reach ≤ ambient reach ceiling → proceed
2. Content has attestation requirements → check if Peer A holds the required attestation CIDs
3. Attestation CIDs resolved against DHT → verify `status: "active"`, not expired, agent matches

### CID-Based Credentials (Not Bearer Tokens)

All trust credentials are DHT entries addressed by CID:
- Attestation CID → resolves to full attestation (agent, type, reach_granted, status, grantor, evidence)
- Participation CID → resolves to collective membership (collective, intimacy_level, consent_state)
- Relationship CID → resolves to human relationship (parties, intimacy, consent, custody)

Revocation: governance process sets `status: "revoked"` on the DHT entry. Gossip propagates (200-2000ms). Serving peers recheck cached CIDs periodically.

No bearer tokens. No revocation lists. The DHT is the single source of truth.

## The Elohim's Role

The elohim (AI agents) aren't a gate at any single layer. They permeate all four:

- **Discovery**: They ratify reach decisions. Content earns commons reach through governance processes that the elohim mediate.
- **Consumption**: They verify attestation chains. Are the prerequisites legitimate? Is the mastery claim genuine?
- **Participation**: They mediate engagement. Is this assessment appropriate for this learner's level?
- **Replication**: They govern stewardship. Are allocations fair? Is the distribution just? They tell honest stories from pipeline traces.

The elohim are the nervous system of the protocol, not a gatekeeper. They create conditions where trust emerges naturally, not through enforcement.

## What's Implemented vs. What's Next

### Already Built (This Session)
- EPR Head publication + resolution (Tier 1 discovery)
- Shard fetch for content bytes (Tier 3 delivery)
- SQLite persistence (local caching = Layer 4 replication)
- Commons/public bypass at storage layer (Layer 1 for web2 visitors)
- Server-side reach authorization with agent_pubkey (Layer 1 P2P)
- Recognition on delivery via EconomicEvent (steward attribution)
- Startup EPR Head publication (existing content becomes discoverable)

### Next Sprint: Per-Connection Trust Context
- Ambient reach negotiation on P2P connection establishment
- CID-based credential presentation (attestation + membership CIDs)
- DHT verification of presented CIDs
- Cached trust context with TTL

### Future: Full Four-Layer Stack
- Per-content attestation gates (Layer 2)
- Participation gating via sophia integration (Layer 3)
- Stewardship compute budgeting and allocation (Layer 4)
- Elohim governance mediation across all layers
- Immediate revocation via governance signal protocol
- Stewarded account modality (children, seniors, those rebuilding trust)
- Per-connection ambient trust negotiation with CID verification

## Foundational Assumption: The Full Spectrum of Humanity

The protocol does not assume good actors. It assumes the full range of human expression — including sociopaths, social dominators, conspiracy theorists, the easily manipulated, those with criminal history, and every other challenge to collective trust. They are all here. They all have nodes. They all have dignity.

The system is resilient not through exclusion but through three structural properties:

### 1. Harm Cannot Scale

On the extractive web, a charismatic manipulator reaches millions because algorithms reward engagement regardless of truth. On the elohim protocol, reach is earned through community endorsement. Deception might fool an immediate circle, but reaching the district requires evidence reviewed by independent people. Reaching the city requires verification across multiple communities. The cost of deception scales exponentially with reach. Conspiracy theories cannot go viral — they can only travel as far as people who examine them and endorse them. Without an audience to monetize, the economics of exploitation collapse.

### 2. The Good-Faith Majority Has Coordination Tools

The central failure of current platforms: the majority is atomized. Each person fights trolls alone. The motivated minority exploits this atomization — it's the collective action problem that breaks every online community.

The elohim protocol gives the majority solidarity: challenges and appeals (qahal governance), consent-based decision-making, ranked-choice governance for proportional representation, and elohim-mediated deliberation that carries human interests into the process so the majority doesn't need to all show up simultaneously. The elohim make quorum irrelevant — they ensure the community's voice is heard even when individuals are busy living their lives.

### 3. Dignity Is Structural, Not Conditional

Every human on the protocol has a node, has local reach, can contribute to their immediate community, and can earn trust through demonstrated care. The system never says "you are not allowed." It says "your reach reflects what you have earned." That rule is the same for Maria contributing to her neighborhood, Emma earning district reach for her science fair, Grandma Rose maintaining her community connections, and a person rebuilding trust after incarceration.

The protocol does not punish permanently. It provides the same path to everyone: contribute, earn endorsement, expand reach. The stewarded account modality (children, seniors, those with care providers) adds protective context — not restriction but supported participation. The steward configures the context, the elohim watches the steward, and earned reach is the graduation mechanism for everyone.

Power over the protocol lives with the majority that operates in good faith. The constitutional layer — immutable principles that no actor can override — ensures that even governance itself cannot be captured. Communication is sacred. Attention is commons. Data is sovereign. Reach must be earned. These are mathematical constraints, not policies that can be lobbied away.

## The Three-Pillar Coupling: Scale Power, Scale Responsibility

On the extractive web, information, value, and governance are decoupled. Information scales without accountability (platforms profit from extremism). Value accumulates without obligation (billionaires with no stewardship). Governance gets captured by value (lobbying buys the rules). The decoupling IS the capture mechanism. Whoever can accumulate one dimension without the other two wins — and everyone else loses.

The EPR protocol's three-pillar coupling is the forcing function that makes this impossible.

Every piece of content on the protocol carries all three pillars simultaneously:
- **Lamad** (information): what it teaches, who it reaches, what knowledge it carries
- **Shefa** (value): who stewards it, how recognition distributes, what compute sustains it
- **Qahal** (governance): what reach it has earned, what constitutional authority applies, what attestations validate it

These are mathematically coupled — you cannot have one without the others:

**Information without governance is propaganda.** Content cannot reach beyond its local context without community endorsement, peer review, and governance ratification. The attestation chain IS the coupling. A podcast episode reaches commons not because the creator declared it but because 80% of the attestation types were earned through independent community processes.

**Value without responsibility is extraction.** Recognition accumulates for stewards only because they maintain content that the community endorsed and that serves learners. The stewardship allocation IS the coupling. Your 400 stewards aren't paying a subscription — they're lending compute from hardware they own because they care. Their stewardship commitment is inseparable from the content's availability.

**Governance without contribution is tyranny.** Participation in governance decisions requires earned attestations through demonstrated contribution. You cannot vote on content reach without having contributed content yourself. You cannot challenge a stewardship allocation without having stewardship experience. The attestation gate IS the coupling. Power scales only with demonstrated care.

The primitive signals of value — recognition events, stewardship allocations, affinity scores — are structurally coupled to the primitive signals of responsibility — attestation requirements, governance participation obligations, community endorsement thresholds. As one scales, the others must scale with it. A podcaster with 90,000 unique visitors has proportionally more governance oversight, more stewardship infrastructure, and more attestation requirements than a family sharing recipes within their household. The coupling is automatic, not enforced — it emerges from the structure of the EPR itself.

This is what the three-pillar EPR Head encodes. Not three independent metadata fields, but a single indivisible context: this content carries knowledge AND value AND governance simultaneously. You cannot resolve the knowledge without acknowledging the value. You cannot consume the value without operating within the governance. The coupling is the protocol's immune system against capture at every scale.
