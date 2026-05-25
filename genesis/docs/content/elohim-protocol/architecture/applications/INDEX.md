---
title: Application archetypes — battle-testing the elohim protocol on familiar patterns
tier: architecture
status: Living document
created: 2026-05-24
maintainers: Matthew Dowell + Opus 4.7
---

# Application archetypes — the protocol's proof gallery

If you're a systems architect — fluent in SQL, GraphQL, Kafka, S3, Spring Batch, Redis, double-ledger accounting, service-oriented architecture — and you're thinking *"yeah right, P2P substrate at planetary scale, sure"* — **this is the directory you want to start with**. Each file here is a battle-test of the theory: take a familiar legacy application, show exactly how it composes from the eight substrate primitives, and quantify the scaling answer per-peer and globally.

The composition is concrete. The math is concrete. The code anchors are concrete. No hand-waving.

## The eight foundational primitives are spec'd separately

The substrate primitives are defined in [`../2026-05-24-records-lifecycle-design.md`](../2026-05-24-records-lifecycle-design.md) Part A. Read it first if you want the substrate vocabulary before the applications. Read here first if you want the proofs before the foundation.

Either order works. The graph is bidirectional.

## Active subsumption targets (in build / next-up)

These are the legacy applications the elohim protocol is actively building substrate-native replacements for. Each is its own canonical-architecture spec with frontmatter bridging the epic narrative to the technical composition to the code.

| File | Replaces | Pillar | Status |
|---|---|---|---|
| [`mint-monarch-application-design.md`](./mint-monarch-application-design.md) | Mint / Monarch.app | shefa | Full draft — exemplar |
| [`khan-academy-application-design.md`](./khan-academy-application-design.md) | Khan Academy | lamad | Composition draft |
| [`google-drive-application-design.md`](./google-drive-application-design.md) | Google Drive | lamad + elohim | Composition draft |
| [`google-photos-application-design.md`](./google-photos-application-design.md) | Google Photos | lamad + elohim | Composition draft |
| [`meta-facebook-application-design.md`](./meta-facebook-application-design.md) | Meta / Facebook | imagodei + qahal | Composition draft |
| [`patreon-application-design.md`](./patreon-application-design.md) | Patreon | shefa + lamad | Composition draft |
| [`requests-offers-application-design.md`](./requests-offers-application-design.md) | Amazon — commerce side / cooperative procurement | shefa | Composition draft |
| [`aws-compute-application-design.md`](./aws-compute-application-design.md) | AWS / cloud compute marketplace | shefa + elohim | Composition draft |

## Deferred-but-coherent — see horizons/

Patterns we've thought through, that compose cleanly, but that aren't on the active path right now:

- YouTube (digital media platform) — see [`../horizons/youtube-application-design.md`](../horizons/youtube-application-design.md)
- WordPress (composed SPA) — see [`../horizons/wordpress-application-design.md`](../horizons/wordpress-application-design.md)
- Factory (industrial supply chain as collective) — see [`../horizons/factory-application-design.md`](../horizons/factory-application-design.md)
- Bank (financial institution as collective) — see [`../horizons/bank-application-design.md`](../horizons/bank-application-design.md)

## The architect's reading guide

For each application archetype, the same questions are answered concretely:

1. **What does the user see?** (the grandma test — what's the interface, what works)
2. **Primitive composition** — exactly which EPRs, Events, Resources, Observations, Commitments, Attestations, FeedbackSignals, Links carry this pattern; manifest discriminators
3. **How does one operation flow?** — end-to-end trace of a representative interaction
4. **Per-household storage footprint** — concrete numbers (MB SQL projection, GB iroh-blob working set, MB cold archive)
5. **Network bandwidth profile** — per-month per-household
6. **DHT entry impact** — per-peer entry visibility under the reach model; why it doesn't melt
7. **Render speed** — why the dashboard / feed / library doesn't hang
8. **Cross-household aggregation** — how collectives federate without data replication
9. **Where agentic intelligence carries the load** — what humans can't bear, what elohim cognition does
10. **Bridges to legacy** — how the substrate-native version coexists with the incumbent during transition; cash-out / bidirectionality

Each archetype must answer all ten. If any answer is hand-waved, the proof fails for that pattern.

## The frontmatter bridge — how to read it

Every application archetype has frontmatter declaring:

```yaml
realizes:              # ← which epic narratives this archetype gives technical form to
informed-by:           # ← which architecture specs this composition rests on
informs:               # ← what downstream code / sprint specs this constrains
```

If you're tracing FROM the human-story epic, follow `realizes:` from each application archetype back to its narrative. If you're tracing FROM the code, the archetype's code-anchors section points you back. If you're skeptical of a substrate primitive, the `informed-by:` chain takes you to that primitive's canonical spec.

## What MUST be true for this directory to land its argument

If the protocol's theory is right:

1. Each archetype's per-household footprint stays inside consumer-grade hardware (laptop / phone with modest SSD, modest RAM, modest bandwidth)
2. The 8B-user math works — DHT entry budget, peer gossip rate, cold-archive cost all stay inside known bounds
3. The same eight primitives carry all of these patterns without special-casing or extension
4. Agentic intelligence (elohim cognition) is what unlocks the patterns humans can't bear to author (care narration, inventory upkeep, observation crystallization)
5. Bridges to legacy systems let users adopt incrementally; cash-out is structural
6. No single peer or hub is a bottleneck; no central server is required for any flow

If even one of these breaks for even one archetype, the theory needs revision. The proof gallery is the place where the theory is most exposed to test.
