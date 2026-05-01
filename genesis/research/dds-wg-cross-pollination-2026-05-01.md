---
title: DDS-WG Cross-Pollination
status: Capture
date: 2026-05-01
---

# DDS-WG Cross-Pollination — May 2026

The [Decentralized Deliberation Standard](https://github.com/dds-wg/dds) (DDS) is an open protocol for verifiable deliberation, drafted by Nicolas Gimenez ([ZKorum](https://www.zkorum.com), [Agora Citizen Network](https://www.agoracitizen.network)) and the dds-wg working group. It is built on AT Protocol for transport, Arweave / Filecoin / Logos for archival, and Ethereum for verification. The spec is a working draft as of early 2026; it is the most coherent treatment of public deliberation infrastructure we have seen from outside our own design tradition.

We surveyed it this week. The engagement sharpened design questions we had been holding loose, and produced two new specs in our own repository.

## What we lifted

The **integrity-vs-correctness split**. DDS distinguishes "result-commitment hash" (proof that the result has not been modified since publication) from "zkML proof" (proof that the computation was executed faithfully). This is a clean framing we adopted into our `ComputationAttestation` primitive, where the *Witness* and *Audit* stations carry integrity claims and the *Proof* and *Confirmation* stations carry correctness claims.

**Asymmetric verification**. Expensive to prove, cheap to verify. The cost asymmetry is what makes opt-in proof tractable: the protocol does not pre-pay compute for unrequested rigor. Default is *Witness*; rigor escalates on demand.

**The Habermas Machine recognition**. The DDS author has been vocal in [OpenCivic](https://opencivic.network) about admiring DeepMind's Habermas Machine (Tessler et al., *Science* 2024) and Habermas's own intellectual legacy. Surveying that admiration triggered a reframe for us: the protocol's existing community-deliberation + elohim-agent-as-counsel + governance-validator loop is *already* a Habermas-Machine-shaped artifact — but pluralistic (per-evaluator standing), constitutionally bounded (floors that resist majority override), and graduated (capability scales with stewardship). We did not need to build a Habermas Machine; we needed to recognize what we already had and articulate the contract that makes it auditable. See [`habermas-machine-2024.md`](habermas-machine-2024.md) and [`habermas-legacy.md`](habermas-legacy.md).

**The four design tensions framing** (Ownership/Convenience, Discoverability/Durability, Provable/Economical Computation, Autonomy/Interoperability). Clean enough to reuse the *form* in our own architectural docs.

**The two-paths anonymity framing**. DDS is honest about not mixing pseudonymity and strong anonymity in a single application. We are not adopting that bifurcation (graduated reach + per-evaluator standing is our cut), but engaged it seriously because it is more disciplined than most spec language on the topic.

## Where our paths diverge

- Peer-native DHT + libp2p substrate (Holochain) instead of federated PDS + Firehose. Different bargain on data availability vs. server authority.
- Chain-agnostic `SettlementBridge` with content-addressed provenance hash instead of Ethereum result-commitment as source of truth. We do not reject ETH as one possible bridge target — just refuse to make it the spine.
- Reach as an authoring-time concern instead of implicit Firehose global broadcast.
- Constitutionally-notarized floors that resist majority override, instead of three flat access modes.
- AT Protocol interop at the doorway projection layer instead of lexicons as protocol primitives. Doorway is *our* federation / web2-projection layer; AT Proto is one optional flavor among several an operator can choose.

## Outputs

The survey produced four committed artifacts:

- [`genesis/docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md`](../docs/superpowers/specs/2026-05-01-computation-attestation-graduated-rigor-design.md) — peer-native attestation primitive
- [`genesis/docs/superpowers/specs/2026-05-01-atproto-lexicon-projection-doorway-design.md`](../docs/superpowers/specs/2026-05-01-atproto-lexicon-projection-doorway-design.md) — doorway federation sibling
- [`habermas-machine-2024.md`](habermas-machine-2024.md) and [`habermas-legacy.md`](habermas-legacy.md) — Habermas reference markers
- [`README.md`](README.md) — new **The Deliberation Problem** and **The Archival Problem** sections

A cross-pollination issue was opened at the dds-wg repository as a thank-you and invitation to engage further: *[issue link added after posting]*.

## Credit

Nicolas Gimenez (ZKorum, Agora Citizen Network) and the dds-wg working group authored the DDS spec we engaged with. Their work made this engagement worthwhile.
