---
name: Memory lifecycle as protocol design — comet shape + deliberate forgetting
description: Memory should be comet-shaped (99% recent head, long dwindling tail, small memorialized core); deliberate forgetting is a first-class action; same lifecycle applies to Claude memory, elohim-agent memory, DHT entries, EPR gossip — household compute is bounded so intentional decay is required for sustainability; what we build to complement Anthropic Dreams IS the protocol's data hygiene prototype
type: project
originSessionId: 10d85ef0-1979-4311-97e9-c2c209de48e2
---
The comet shape: memory has a bright dense head (99% — recent artifact, hot, fully present), a long dwindling tail (compacted, distilled, lossy but referenceable), and a small memorialized core (manifesto-tier — never forgotten, structurally anchored). Most data lives in the tail, fading; only what *earns* memorialization stays in searing memory.

**Deliberate forgetting is a first-class action.** The interesting design question is not "what to remember" but "what is ok to forget" — and the answer must be principled, not accidental. Letting things fade ungraciously is the same failure mode as remembering everything forever; both are bugs.

**This is one problem at multiple scales:**
- Claude/agent memory (MEMORY.md, auto-memory, Anthropic Dreams)
- elohim-agent research (autonomous agent's working memory + long-term memory)
- DHT entries (notarized facts that accumulate)
- EPR artifacts and gossip (graph edges/links proliferating across the network)
- Content nodes / lamad recognition records / shefa events
- Scenarios / spec corpus / git history

The compute footprint problem: the household network is bounded. Every link/event/EPR/notarization that stays "hot" forever costs compute on every replica, every search, every query. Long-term, unbounded retention is structurally unsustainable for household-scale infrastructure. **Intentional decay is the sustainability loop.**

**Connects to existing project principles:**
- *DHT vs libp2p scoping* — DHT is expensive; only put narrow integrity-load on it. Same insight: only memorialize what *earns* the cost.
- *Trust is an efficiency signal* — trustworthy/repeat-validated data costs less to distribute. Hot-tier earns its place by proving principle-shaped through repetition.
- *Reach is earned at authoring* — and analogously, *durability is earned through repeat reference*. Memorialization is the asymptote of repeated citation.
- *Stewardship philosophy / graduated authority* — what gets memorialized, compacted, or forgotten is a stewardship decision; different tiers need different authority (operator for personal memory, qahal for collective memorialization, individual author for own EPR decay policy).
- *Ungrudging service* — forgetting is not dishonor. The gift flowed; the trace can fade. Memory hygiene is opposite of grudge-holding.
- *Household horizontal scaling* — adding blades is the resilience pattern, but each blade still has bounded compute. Decay is what keeps the asymptote livable.

**What we build to complement Anthropic Dreams (the local /dream skill) IS the protocol's data lifecycle prototype.** The principles we encode in /dream — promotion criteria, compaction rules, validity intervals, forgetting policy — are the same primitives that need to govern EPR gossip, DHT entry decay, content node archival.

**Practical today → protocol governable tomorrow.** /dream is the dev-loop prototype; the principles graduate into elohim-agent specification (autonomous memory hygiene) and into the protocol substrate (EPR/DHT lifecycle policy). Same circularity/sustainability loop at every scale.

**Lifecycle primitives (the protocol-generalizable schema):**
- *promote* — episodic → semantic, or semantic → manifesto, when earning criteria are met
- *compact* — distill one entry to its essence + pointer to git/source; lossy single-entry consolidation
- *merge* — fuse N entries about the same concept into a new head c ≈ 0.8(a+b); lossy multi-entry consolidation with mandatory lineage edges to predecessors; predecessors' validity intervals close. THE most graph-shaped operation; prevents unbounded growth without requiring forgetting
- *close-interval* — mark superseded with end-date (Zep-style); preserves trajectory; structurally distinct from delete
- *memorialize* — anchor to manifesto-tier, never forget; the asymptote of repeated citation
- *forget* — release fully, with audit trail of what was forgotten and why; first-class action

**How to apply:**
- Every long-lived data type in the protocol should declare its lifecycle: hot-tier criteria, decay function, merge candidacy, memorialization threshold, forgetting policy. Don't accept "store forever by default."
- /dream v1 should encode generalizable lifecycle primitives, not Claude-Code-specific ones — proposal types map cleanly onto EPR / DHT entry / content node lifecycles.
- Merge in EPR context: two related EPRs gossiping the same subject coalesce into a new EPR head with lineage edges; originals close their interval; compute footprint shrinks while trajectory is preserved.
- This deserves its own spec under elohim-agent research: a memory-lifecycle / data-decay specification that /dream prototypes and that the protocol substrate adopts.
- Forgetting and merge policy is governance — different scales need different authorities (personal/operator, collective/qahal, author/own-EPR). Don't centralize.
- Validity intervals preserve trajectory even when current value supersedes — "remembered as of <date>, superseded <date>" is structurally different from delete.

**Sources:** brainstorm 2026-05-10; this insight extends and load-bears the agentic-context-graph model and Anthropic Dreams beta documentation.
