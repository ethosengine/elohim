---
title: AWS — substrate-native peer-native compute marketplace
tier: architecture
status: Composition draft (primitives mapped; full walkthrough pending)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: shefa (primary — compute as economic resource), elohim (substrate + provider identity), imagodei (compute provider as agent), qahal (compute-provider collectives / clusters)
realizes:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (compute as commodity; peer-provided infrastructure)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md (Tier-1 / Tier-3 nodes; the topology compute providers sit in)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md (VF semantics for compute economic flows)
  - (planned: computation-attestation-graduated-rigor spec — Witness/Audit/Proof/Confirmation tiers for verifying compute results)
informs:
  - app/elohim-app/src/app/shefa/ + elohim pillar for compute-marketplace surfaces
  - elohim/elohim-storage/src/services/ (compute-allocation services)
  - elohim/sdk/domains/shefa/manifest.json (resource_classified_as: compute-cycle, gpu-hour, storage-tb-month)
defers:
  - Specific verification protocol per workload class (per the compute-attestation spec)
  - Cross-provider scheduling / load-balancer logic (application layer)
  - Cryptographic computation proofs for confidential workloads (zk-proof bridge spec)
---

## The grandma test

A small-business owner or AI researcher opens the app. They see: compute capacity available across the peer mesh (CPU, GPU, storage, by location and hardware class), jobs they've submitted, billing for completed work. A compute provider sees: their own capacity declarations, jobs accepted, throughput, earnings. AWS-shape — but the providers are individuals and cooperatives running real hardware, not a hyperscaler; the buyer pays the provider directly; the substrate's own AI inference, sweettest jobs, and graduation evaluators are the first paying customers.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Compute provider | imagodei Human + provider-EPR (`content_type: "compute-provider"`) | reach=`commons-attested` (so buyers can find them) |
| Capacity declaration | Commitment (`action: "provide-compute"`) | provider's `hardware_class` + `availability_window` + `pricing` |
| Compute resource | Resource (`resource_classified_as: "compute-cycle"` / `"gpu-hour"` / etc.) | per-hardware-class shape; balance = remaining capacity in window |
| Job request | Commitment (`action: "request-compute"`) from consumer | hardware requirements + budget + deadline |
| Match | Event (`action: "matched-compute"`) | elohim-mediated; links request to a provider's capacity slot |
| Job execution | Event (`action: "executed-compute"`) | with bytes-of-output, elapsed-time, cost as quantities; observation_refs to telemetry |
| Verification | Attestation (`content_type: "attestation:computation"`) | proof_evidence.class per the compute-attestation gradient (Witness/Audit/Proof/Confirmation) |
| Bill | derived view over consumer's consumed-compute Events | mass-balanced REA accounting |
| Payout | Event (`action: "transfer"`) | consumer → provider, in currency-USD Resource or community currency |
| Marketplace | Collective EPR (`content_type: "compute-marketplace"`) | aggregates provider declarations + buyer demand |
| Compute-cooperative | Collective EPR (`content_type: "compute-cooperative"`) | many small providers pooling capacity; shares revenue across members |

## Stress points the substrate handles

- **Real-time capacity matching**: compute-Resource entries are independently gossiped at `community` reach (small entry, frequent updates as capacity consumes); matching = elohim-mediated based on hardware class + locality + standing
- **Trust without central platform**: providers earn standing via verified-fulfillment Attestations; consumers earn standing via payment-fulfillment Attestations; no platform-owned trust rating
- **Verification of paid-for work**: per the computation-attestation gradient — operations vary from cheap-trust (witness) to expensive-replay (audit) to cryptographic (proof) to multi-party-confirmation; manifest declares verification class per workload type
- **Substrate's own compute use**: AI inference, sweettest test jobs, graduation evaluators, vision-elohim image processing — these are the **first paying customers** of the marketplace before external workloads arrive. The substrate eats its own compute, generating internal demand that bootstraps the supplier side.
- **Geographic locality + privacy**: data-sensitive jobs match to providers in the requester's reach scope only; cross-jurisdictional flow requires explicit reach-grant

## Scale answer

- Per-provider: thousands of capacity-Resource updates per month × 500 B ≈ ~MB SQL; capacity declarations are short-lived
- Per-consumer: hundreds of job-Commitments per month + matching Events
- Marketplace-hub: aggregates provider declarations within the cooperative; federated query for cross-cooperative matching
- 100M households × small compute capacity each = massive aggregate; per-peer footprint stays small because matching is reach-scoped

## Bridges to legacy

- **bridges/aws/** (consumer-side) — wrap AWS API as a substrate-native provider with stewardship-elohim signing; substrate-native consumers can use AWS compute through this bridge during the transition
- **bridges/gcp/** , **bridges/azure/** — same pattern for Google Cloud and Azure
- **bridges/runpod/** , **bridges/vast/** — already-decentralized GPU markets become substrate-native by speaking the marketplace primitive
- **Cash-out**: providers can export their capacity history + revenue Events; consumers can export their job-history + spend Events; portable to legacy cloud billing

## Where the substrate has a real advantage

- **Aggregating long-tail compute**: 100M households each contributing ~hours/week of idle compute = exaflops of aggregate capacity that no hyperscaler can match for cost-per-cycle once you exclude their margin
- **Locality + privacy**: peer-mesh matching keeps sensitive data on local providers; sovereignty preserved
- **Cooperative ownership**: compute-cooperatives mean small providers participate in markets they'd be excluded from individually
- **AI compute specifically**: as AI inference moves to the edge (per the project_intelligence_zero_marginal_cost_inevitable memory), substrate-native compute marketplaces become the natural distribution surface

## Code anchors

| Surface | Path |
|---|---|
| Compute-marketplace surfaces | `app/elohim-app/src/app/shefa/` |
| Compute-resource entries | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` |
| Compute-attestation pattern | (planned graduation from `2026-05-01-computation-attestation-graduated-rigor-design.md`) |
| Provider capacity tracking | `elohim/elohim-storage/src/services/` (planned) |

*Full draft pending.*
