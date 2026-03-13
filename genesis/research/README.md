# Research Index

Research lives at each module boundary alongside the code it informs. This index connects the open questions driving each area of the protocol to where the thinking lives.

The sections below are ordered by the protocol's deepest conviction: that the hardest problem in human coordination isn't computation — it's legitimacy. People don't reject distributions because the math is wrong. They reject them because they can't see *why*. The protocol's answer is to collect richer signals than money can carry, let composable pipelines do honest math, and let elohim tell the story.

This conviction is empirically grounded. Druckman & Adrian (2020) found that robot mediators produced more integrative agreements *and* higher participant satisfaction than human mediators delivering identical content — perceived neutrality outweighed human warmth. Sanfey et al. (2003) showed participants accepted unfair offers from computers at significantly higher rates than identical offers from humans, because non-intentional agents don't trigger the same sense of insult. Claure et al. (2023) demonstrated that AI-allocated resources reduced interpersonal tension even when allocations were unequal. The pattern is consistent: when an agent shows its work and has no ego in the outcome, people trust the result more than they trust each other.

**Key references:**

*AI mediation & fairness:*
- Druckman & Adrian, "Who is Best at Mediating a Social Conflict?", *Group Decision and Negotiation*, 2020
- Sanfey et al., "The Neural Basis of Economic Decision-Making in the Ultimatum Game", *Science*, 2003
- Claure et al., "The Social Consequences of Machine Allocation Behavior", *Computers in Human Behavior*, 2023
- Chugunova & Luhan, "Ruled by Robots: Preference for Algorithmic Decision Makers", *Public Choice*, 2024

*Economic ontology & accounting:*
- [REA (Resource-Event-Agent)](https://en.wikipedia.org/wiki/Resources,_events,_agents) — McCarthy's 1982 accounting ontology. Economic activity modeled as events connecting agents to resources, not as debits and credits. The protocol's recognition pipeline, stewardship allocations, and economic events all speak REA.
- [ValueFlows](https://valueflos.ws/) — Open vocabulary for distributed economic coordination, built on REA. Defines the grammar the protocol uses: agents perform processes that transform resources, tracked as events with provenance. Lynn Foster and Bob Haugen's work over a decade-plus to make care, commons stewardship, and ecological accounting legible in the same language as market exchange.
- [hREA](https://github.com/h-REA/hREA) — ValueFlows implemented on Holochain. Agent-centric REA accounting without central ledgers. The protocol's natural persistence layer for economic events — each participant holds their own source chain of economic activity, validated by peers through the DHT. Pospi's work bridging ValueFlows vocabulary to Holochain's agent-centric architecture.

---

## The Elder Problem

The protocol's most ambitious bet: AI agents that steward human flourishing rather than optimizing engagement. Elohim agents nudge (proactive guidance), play (co-exploration), and resolve (mediated conflict). The research questions are foundational — how do you give an agent constitutional constraints that feel like wisdom rather than rules? How does RAG ground an agent in a learner's actual journey? And critically: how does an elohim turn a `StageTrace` from the recognition pipeline into a story that a human trusts — "here's why your recognition looked like this, here's what's shifting, here's what you'd change"?

The explainability question isn't a nice-to-have. It's the entire value proposition. The pipeline does math. The elohim fosters trust and legitimacy.

[elohim/elohim-agent/research/](../../elohim/elohim-agent/research/)

---

## The Value Distribution Problem

How do you distribute recognition fairly across a network of stewards with different contributions, affinities, and constitutional constraints? v0 uses linear proportional allocation weighted by affinity — functional but naive. The deeper questions involve temporal decay (does a creator's share diminish as maintainers contribute?), cascading attribution through content dependency graphs, multi-swimlane distribution across different types of value, and cybernetic feedback that self-corrects concentration. This is where economics meets cybernetics meets constitutional AI.

The pipeline collects signals well beyond what a dollar conveys — contribution type, affinity, stewardship tenure, constitutional limits. The math is solvable. The hard part is making the result *legitimate*, which is why this research is inseparable from the elder problem above.

[elohim/elohim-storage/research/](../../elohim/elohim-storage/research/) — `economic-systems-research.md` (Drips, Unyt, hREA, EAE survey), `future-distribution-models.md` (six post-v0 research directions)

---

## The Edge Problem

How does a decentralized protocol meet people where they are — in browsers, behind NATs, without installing anything — without becoming the centralized chokepoint it's trying to eliminate? The doorway is the answer we're exploring: a gateway that proxies, caches, and federates, but never owns. Federation with Matrix and ActivityPub, WebRTC signaling for peer-to-peer when possible, graceful fallback when not.

[doorway/research/](../../doorway/research/)

---

## The Trust Substrate

Why Holochain and not a blockchain? Because agent-centric architecture means your data lives with you, validation is intrinsic to the data type, and there's no global consensus bottleneck. But the DHT brings its own questions — gossip protocol tuning, sharding strategies for content-heavy networks, entry validation patterns that enforce constitutional constraints without central authority. The research here is about making the substrate invisible while keeping it trustworthy.

[elohim/holochain/research/](../../elohim/holochain/research/)

---

## The Content Addressing Problem

EPR content has three tiers — Head (gossipped metadata), Document (peer-cached body), Bytes (steward-delivered shards). IPFS gives us content-addressed delivery and verified fetch, but mapping EPR tiers to IPLD primitives (DAG-CBOR encoding, CID resolution) is non-trivial. The rust-ipfs fork (tracking `dariusc93/rust-ipfs` with connexa) is the vehicle; the research is about making content-addressed delivery feel instant while remaining verifiable.

[elohim/rust-ipfs/research/](../../elohim/rust-ipfs/research/)

---

## The Networking Problem

Steward nodes don't form flat peer networks — they form trust-weighted meshes where affinity to content determines topology. libp2p gives us the protocol primitives (request-response, gossipsub, Kademlia), but the research questions are about NAT traversal at scale (tx5, sbd relay), peer discovery that respects affinity, and how the steward topology emerges from individual stewardship decisions rather than being centrally planned.

[steward/node/research/](../../steward/node/research/)

---

## The Stewardship Problem

"Take it with you" means a desktop app that runs a Holochain conductor, stores your data locally, and works offline. Tauri gives us the native shell; the hard problems are conductor embedding lifecycle, identity handoff between web and native contexts, deep link routing across cold and warm starts, and making offline-first feel seamless rather than degraded. This is where stewardship stops being a principle and becomes a UX.

[steward/device/research/](../../steward/device/research/)

---

## The Assessment Problem

How do you measure human growth without reducing it to a score? Sophia renders assessments in three modes — mastery (graded), discovery (psychometric), and reflection (open-ended). The research is about what makes each mode work: IRT for adaptive difficulty, instrument design that surfaces authentic values rather than socially desirable answers, and scoring models that inform the learning path without flattening the learner.

[sophia/research/](../../sophia/research/)
