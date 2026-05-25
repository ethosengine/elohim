---
title: Horizons — Coherent application patterns awaiting active subsumption
tier: architecture
status: Living document
created: 2026-05-24
---

# Horizons — patterns we've thought through, but aren't actively building right now

This directory holds **application pattern designs that are architecturally coherent on the elohim substrate, but are NOT on the active subsumption path.** They are preserved so the architectural thinking isn't lost — and so when the time comes to build them, the substrate-shape is already known and we don't re-derive.

The active subsumption targets (where we are putting real implementation effort right now) live in [`records-lifecycle-design.md`](../2026-05-24-records-lifecycle-design.md) Part B. Those are:

- **Khan Academy** (lamad learning platform)
- **Google Drive** (file store + collaboration)
- **Google Photos** (media library)
- **Mint / Monarch** (personal finance — shefa)
- **Meta / Facebook** (social graph + feed)
- **Patreon** (creator monetization)
- **Amazon Requests & Offers** (cooperative commerce)
- **Amazon AWS** (peer-native compute marketplace)

The horizons in this directory are coherent additions to that set, but each is held off because either (a) the legacy incumbent is less load-bearing to displace right now, (b) the substrate primitives need more maturity before this pattern is buildable end-to-end, or (c) it's a high-stakes/high-regulation pattern that benefits from the active subsumptions proving the substrate first.

## Current horizons

- [`youtube-application-design.md`](./youtube-application-design.md) — digital media platform; asymmetric author/viewer; massive blob storage. Subsumed-by-implication once Patreon + Photos prove the substrate at media scale.
- [`wordpress-application-design.md`](./wordpress-application-design.md) — composed SPA / personal site; reach-gated discovery; doorway web2 projection.
- [`factory-application-design.md`](./factory-application-design.md) — industrial supply chain as a collective EPR; multi-party coordination; sensor-graduation Events. Requires industrial trust networks before substrate-native operation is feasible.
- [`bank-application-design.md`](./bank-application-design.md) — financial institution as a collective EPR; banking becomes an REA dashboard with token-minting authority. Regulatory complexity makes this a late move; bridges (Plaid / Stripe / banking APIs) provide parallel operation in the meantime.

## When a horizon graduates

A horizon graduates from this directory to active Part B status in the records-lifecycle spec when:

1. The substrate primitives it composes are all stable and shipping
2. Either an operator wants to build it, or an active subsumption target (Khan/Drive/Photos/Monarch/Meta/Patreon/R&O/AWS) has matured to where this becomes the natural next step
3. A bridge exists or is straightforward to write for whatever legacy interop is needed during the transition

At graduation: the horizon spec moves out of this directory into `architecture/`, gets renumbered into Part B of records-lifecycle (or its own canonical spec if it's substantial enough), and the implementation plan gets drafted.

## What NOT to put here

- Sprint-shape specs (those live in `genesis/docs/superpowers/specs/`)
- Half-finished active work (finish it in Part B of records-lifecycle)
- Speculative ideas without primitive-composition (those belong in brainstorm notes, not architecture)
- Patterns that re-derive primitives we already have (those should become amendments to existing canonical specs)

A horizon doc should answer: "If we built this today on the substrate, here's exactly how the eight primitives compose, and why we are NOT building it today."
