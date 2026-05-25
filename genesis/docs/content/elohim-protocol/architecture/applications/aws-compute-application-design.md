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

A small-business owner opens the app on her laptop and sees the compute marketplace: available providers organized by hardware class (CPU cores, GPU-hours, region), her two submitted jobs (one completed, one running), and a running bill she understands. Her neighborhood's computing collective — twelve households pooling idle GPU time from gaming rigs and workstations — shows up as a single provider with aggregated capacity and a shared revenue dashboard. She doesn't know or care that there is no AWS. A compute provider in that collective sees their node's declared capacity window, which jobs they've been matched to, their share of earnings, and their fulfillment Attestation track record. The substrate's own elohim-agents — running AI inference for graduation evaluators, sweettest CI jobs, and vision-elohim image processing — are the first buyers in the marketplace, generating demand before any external workload arrives. This is AWS-shape; the providers are people and cooperatives, not a hyperscaler; the buyer pays the provider directly; there is no platform margin.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Compute provider | imagodei Human + provider-EPR (`content_type: "compute-provider"`) | reach=`commons-attested` so buyers can find them |
| Capacity declaration | Commitment (`action: "provide-compute"`) | `hardware_class` + `availability_window` + `pricing_per_cycle` + `locality_region` |
| Compute resource pool | Resource (`resource_classified_as: "stewarded-compute"`) | balance = remaining capacity in window; derived over provision/consume Events |
| GPU-hour unit | Resource (`resource_classified_as: "gpu-hour"`) | per-hardware-class shape; tracked alongside CPU cycles and storage separately |
| Job request | Commitment (`action: "request-compute"`) | consumer's hardware requirements + budget ceiling + deadline + locality constraint |
| Match | Event (`action: "matched-compute"`) | compute-stewardship-elohim–mediated; links request Commitment to provider capacity slot |
| Job execution | Event (`action: "executed-compute"`) | bytes-of-output, elapsed-time, resource-cycles-consumed as quantities; `observation_refs` to telemetry iroh-blob |
| Verification | Attestation (`content_type: "attestation:computation"`) | `proof_evidence.class` per the compute-attestation gradient (Witness / Audit / Proof / Confirmation); manifest-declared per workload class |
| Bill | Derived SQL view over consumer's consumed-compute Events | mass-balanced REA accounting; no separate invoice object needed |
| Payout | Event (`action: "transfer"`) | consumer → provider's Resource (or cooperative's commons Resource); staggered per D.17 |
| Compute cooperative | Collective EPR (`content_type: "compute-cooperative"`) | many small providers pooling capacity; shares revenue via Allocation Events from the collective's Commons |
| Cooperative membership | Membership EPR + REA Agreement | each provider's share proportion; fulfillment + revenue tracked per member via Events |
| Marketplace collective | Collective EPR (`content_type: "compute-marketplace"`) | aggregates provider declarations and buyer demand at community or commons reach |
| Provider reputation | `attestation:computation` history + FeedbackSignal (`signal_class: "compute"`) | `compute_standing` derived separately from care-class standing per D.18 |
| Locality grant | Event (`action: "grant-reach"`) | required before cross-jurisdictional data-sensitive job dispatch per D.9 |

Eleven primitives, ~8 discriminator values, no special-casing for compute marketplaces.

## How one job flows

A researcher submits a GPU inference job from her household elohim-node:

```
1. She authors Commitment(action="request-compute",
       resource_classified_as_json='["gpu-hour"]',
       resource_quantity_value=4.0,
       metadata_json={"hardware_class":"gpu-l40s","locality_region":"us-west","budget_ceiling":12.00},
       deadline=now+2h)
   → DHT write (~2 KB); gossips to compute-marketplace collective at community reach

2. compute-stewardship-elohim reads pending request Commitments
   matching hardware_class + locality + budget ceiling
   against active provide-compute Commitments in reach scope
   → Selects provider (cooperative node in Portland with 4 L40S GPU-hours committed)
   → Authors Event(action="matched-compute",
         provider=researcher-commitment-cid, receiver=provider-commitment-cid)
   → Both parties' SQL projections update: job state → in-progress

3. Provider's node receives job bytes via iroh-blob pull (content-addressed by job CID)
   → Executes inference
   → Emits Observation(kind="shefa:compute-cycle-consumed") telemetry
     into local iroh-blob log at ~1 per second (never DHT)

4. Job completes. compute-stewardship-elohim on provider's node authors:
   Event(action="executed-compute",
         resource_quantity_value=3.87,  // actual GPU-hours consumed
         resource_quantity_unit="gpu-hour",
         observation_refs=["iroh://provider-cid@telemetry-log-cid#0-1247"])
   → DHT write; links to the matched-compute Event

5. Verification per D.16 multi-oracle confirmation:
   - For this workload class (manifest-declared: "inference", default_proof_class: "witness")
   - Provider's compute-stewardship-elohim issues:
     Attestation(content_type="attestation:computation",
                 proof_evidence_class="witness",
                 confirmer_signatures=[provider_sig])
   - If FeedbackSignal::Correction rate on this provider exceeds threshold →
     next attestation escalates to "audit" (Merkle-rooted inputs, re-executable)
   → Attestation DHT write; `compute_standing` projection updates

6. Payment: consumer's shefa-elohim authors
   Event(action="transfer", provider=researcher, receiver=provider-cooperative-commons,
         resource_quantity_value=11.61,  // 3.87 × $3/hr rate
         fee_splits=[{commons_cid: global_commons, amount: 0.06}])
   → Mass-conservation check at integrity zome
   → Cooperative's Allocation Events distribute revenue across member providers

7. Local SQL projection updates; researcher's billing view refreshes; <200 ms end-to-end dashboard
```

Provider standing accumulates from fulfilled Commitments + computation Attestations not subsequently Corrected. No platform rate card. No margin extraction beyond the manifest-declared Global Commons fee split.

## Storage footprint per household

| Item | Count | Size | Total |
|---|---|---|---|
| Provider-EPR + cooperative Membership | ~5 | 5 KB | 25 KB |
| Capacity Commitments (rolling 90-day window) | ~50 | 2 KB | 100 KB |
| Job-request Commitments (per consumer) | ~200/year | 2 KB | 400 KB |
| Execution Events (completed jobs) | ~1,000/year | 2 KB | 2 MB |
| Computation Attestations | ~1,000/year | 3 KB | 3 MB |
| Transfer Events (billing/payout) | ~2,000/year | 1.5 KB | 3 MB |
| SQL projection (local capacity + billing views) | — | — | ~10 MB |
| Telemetry iroh-blob logs (hot, 7-day operational) | ~100 jobs/week × 7 days | ~50 KB/job | ~35 MB |
| Telemetry cold-archive (quilt after 7 days) | ~1,000 jobs/year | erasure-coded | ~20 MB residue |

**Total: ~73 MB per active compute contributor node.** Fits on a household blade or a dedicated steward node. Telemetry observation data never reaches DHT; only graduated Events and Attestations do.

## Network bandwidth profile

- Job submission and matching (Commitment + Event gossip): ~10 KB DHT write per job × ~1,000 jobs/year ≈ 10 MB/year DHT writes
- Job bytes transit: iroh-blob pull (point-to-point, not gossiped); typical inference job 50 MB–10 GB — network cost is job-payload dominated, not substrate-protocol dominated
- Telemetry observation cursors: ~200 bytes/tick × 1 tick/second per running job × 4 average concurrent jobs ≈ 800 bytes/sec when busy, zero when idle
- Attestation gossip: ~3 KB per job × ~1,000 jobs/year ≈ 3 MB/year
- **Per household compute contributor: <50 MB/month substrate protocol overhead** (excluding job payload bytes, which are point-to-point and not protocol gossip)

Compared to S3/Lambda API overhead: substrate protocol overhead is lower than AWS SDK HTTP round-trips at similar job volume. The differentiator is job payload transit — iroh-blob direct is faster than multi-hop cloud routing for same-region peers.

## DHT entry impact

- Each job lifecycle produces: 1 request Commitment + 1 matched Event + 1 executed Event + 1 computation Attestation + 1 payment Event = **5 DHT entries per job**
- At 1,000 jobs/year/node: ~5,000 DHT entries/year, well within the ~3,000 active-entries-per-peer budget (cold entries shelve to quilt)
- At 100M households × 100 compute jobs/year (conservative estimate of participation): 50B DHT entries/year globally — but: reach-scoped. Provider-consumer Commitments are visible only to their parties and their compute-marketplace collective. Only commons-attested providers (Tier 3 public nodes, cooperative collectives) federate broadly
- Per-peer DHT visibility: ~10k–50k entries depending on cooperative membership and marketplace scope — well inside the degradation threshold

## Why the marketplace renders fast

- **Capacity discovery**: `SELECT c.* FROM rea_commitments c WHERE c.action = 'provide-compute' AND c.state = 'accepted' AND json_extract(c.resource_classified_as_json, '$.hardware_class') = ? AND c.due > now() ORDER BY c.due ASC` — indexed on `action`, `state`, and `due`; sub-millisecond locally
- **Consumer billing view**: `SELECT SUM(e.resource_quantity_value), e.resource_quantity_unit FROM economic_events e WHERE e.action = 'executed-compute' AND e.provider = :consumer_cid AND e.has_point_in_time BETWEEN :month_start AND :month_end GROUP BY e.resource_quantity_unit` — local SQL, milliseconds
- **Provider earnings**: same shape inverted: `e.receiver = :provider_cid AND e.action = 'transfer'` — instant
- **Reputation query**: `SELECT compute_standing FROM standing_scores WHERE agent_cid = :provider_cid` — single-row read after D.18 per-class standing is wired; per D.14 60s staleness SLA, this is always fresh
- **Cooperative revenue split**: `SELECT m.agent_cid, SUM(e.resource_quantity_value * m.share_proportion) FROM economic_events e JOIN memberships m ON m.collective_cid = e.receiver WHERE e.action = 'transfer' AND e.has_point_in_time >= :period_start GROUP BY m.agent_cid` — local SQL after federated projection sync

**Dashboard loads instantly because it reads local SQL** — the same pattern as Mint reading Postgres. The match step is the only computation-intensive operation and it runs as an elohim background task, not at render time.

## The long-tail math

The differentiator versus hyperscalers is the aggregate of idle compute that hyperscalers cannot access.

Back-of-envelope:
- 100M households in the substrate → conservatively 30M own compute hardware worth provisioning (desktops, gaming rigs, NAS units, dedicated mini-PCs)
- Average idle capacity per contributing node: ~2 CPU cores + 0.5 GPU TFLOPS available ~8 hours/day
- **Total aggregate: 60M cores × 8h = 480M core-hours/day; 15M GPU-TFLOPS × 8h = 120M TFLOP-hours/day**
- At $0.05/core-hour (half AWS on-demand price, no margin): ~$24M/day market value available to contributors
- AWS, GCP, Azure cannot aggregate this capacity: they do not own this hardware. Their marginal cost for same throughput is capital-expenditure to build datacenters, plus land, plus power, plus margin. The substrate's marginal cost for adding a contributor node is zero — the hardware already exists in the household.

At AI inference specifically: the cost asymmetry widens. A household L40S GPU running inference at $2/GPU-hour (substrate rate) competes with AWS p4d at $32/GPU-hour (on-demand). The substrate's per-cycle cost is 94% lower before accounting for cooperative ownership eliminating the platform margin entirely.

The substrate does not claim parity with AWS on single-job throughput or datacenter-class reliability. It claims exaflop aggregate at cost-per-cycle hyperscalers cannot match, delivered to consumers who have reason to prefer the cooperative provenance.

## Dissolution in practice

- Provider goes offline for 30 days → `Commitment(action="provide-compute")` reaches its `due` without fulfillment → state transitions to `breached`; BreachScanner authors `attestation:computation-breach` (`signal_class: "compute"`) → provider's `compute_standing` decrements independently of care-class standing (D.18 isolation)
- Provider retires node → authors `Event(action="dispose", subject=capacity-resource-cid)` → `lifecycle_state` transitions to `closed`; pending job-request Commitments matched against this provider are re-matched by compute-stewardship-elohim to available alternatives
- Consumer's long-running job contract expires → Commitment state transitions to `fulfilled` or `cancelled`; history remains queryable; no orphaned entries
- Compute cooperative dissolves → `Event(action="close-organization", subject=cooperative-epr-cid)` → collective EPR closes; member Memberships transition to `closed`; revenue balance in Collective Commons gets allocated via final Allocation Events authored under quorum governance

Cold-archive (shelved): old execution Events older than 2 years move to quilt via `Commitment(action="custody-quilt")`; telemetry iroh-blob logs are retained only if cited in active Attestations; otherwise pruned per 7-day operational retention class.

## Where agentic intelligence carries the load

- **compute-stewardship-elohim** (`D.6` specialization — `elohim/elohim-storage/src/services/inference_router.rs`): the marketplace matching logic that makes "request + match + verify" tractable without a central scheduler. It watches pending `request-compute` Commitments, evaluates available `provide-compute` Commitments by hardware_class + locality + compute_standing + pricing, and authors `matched-compute` Events. Without this agent, every job match is a manual negotiation. With it, matching is sub-second and preference-respecting.
- **compute-stewardship-elohim** (verification role): authors `attestation:computation` at the appropriate `proof_evidence.class` per the four-signal gradient from `2026-05-01-computation-attestation-graduated-rigor-design.md` — stakes, spread, consensus-deficit (FeedbackSignal::Correction rate from consumers), and provability ceiling. For high-standing providers at witness class, this is automatic. For contested or high-value jobs, escalation to audit or proof is triggered and narrated to the consumer.
- **Substrate's own compute demand** (bootstrapping role): the graduation evaluator (`elohim/elohim-storage/src/services/graduation_evaluator.rs`) runs AI inference for domain-elohim cognition; sweettest CI jobs run on the compute mesh before external workloads arrive; vision-elohim image processing runs `attestation:auto-tag` and `attestation:face-cluster` workloads. These internal demands activate the supply side — providers earn standing and build track records before external buyers arrive.
- **bridge-stewardship-elohim** (legacy interop): for consumers using `bridges/aws/`, `bridges/gcp/`, or `bridges/azure/`, the bridge-stewardship-elohim signs Observations from the hyperscaler's API (job completion webhooks, billing events) and graduates them into substrate-native Events under the provider-bridge's EPR. Consumers can use legacy cloud through the substrate billing surface during the transition.

The value-prop unlock: compute matching + verification + billing across 100M heterogeneous nodes is impossible for humans to orchestrate manually. Elohim agents handle the continuous matching, monitoring, and attestation — humans see a dashboard that feels like AWS but whose providers are their neighbors.

## What the cooperative view shows

A compute cooperative's collective dashboard federates across its members without replicating data:

- **Render path**: dashboard query → cooperative's collective-hub elohim-node → parallel libp2p RPC to each member's local SQL projection → aggregated results returned
- **What each member contributes**: their local `provide-compute` Commitment history, `executed-compute` Event aggregates, and earnings Events — visible to the collective because Memberships set reach=`collective`
- **What the hub holds**: only collective-level Events and Commitments (the cooperative's own provide-compute declarations, revenue Allocation Events, Membership records) — not member-household private data
- **Revenue split display**: `SELECT m.agent_cid, SUM(revenue) FROM cooperative_revenue_events ... GROUP BY m.agent_cid` — hub aggregates from federated member projections; no data replication

If a member revokes Membership, their data stops flowing to the hub immediately — structural cash-out. The cooperative continues with the remaining members; no central database to scrub.

## Bridges (legacy interop / cash-out)

- **bridges/aws/** (planned) — consumer-side: wraps AWS EC2/Lambda/SageMaker API as a substrate-native provider EPR with stewardship-elohim signature; substrate consumers use AWS compute through the substrate billing surface during transition; bridge-stewardship-elohim graduates AWS billing webhooks into `executed-compute` Events under the bridge's provider EPR. Cash-out: AWS compute costs are paid from consumer's currency Resource via transfer Event to the bridge-collective's Commons, which forwards to AWS.
- **bridges/gcp/** and **bridges/azure/** (planned) — same pattern for Google Cloud and Azure; each gets its own bridge-collective EPR and stewardship-elohim specialization
- **bridges/runpod/** and **bridges/vast/** (planned) — already-decentralized GPU markets become substrate-native by speaking the marketplace primitive: RunPod/Vast.ai's job API maps 1:1 to `provide-compute` / `matched-compute` / `executed-compute` Event vocabulary; their providers inherit substrate standing via Attestation bridging; substrate consumers can reach RunPod/Vast providers through the same compute-marketplace collective surface without knowing they're using a third-party platform
- **Cash-out (provider side)**: providers export their `executed-compute` Event history and earnings to standard billing formats (CSV, JSON invoice); the iroh-blob telemetry logs are the audit trail. Cash-out is structural — the history stays on the substrate; the export is read-only.
- **Cash-out (consumer side)**: consumers export job history and spend Events; bridge-billed jobs have `observation_refs` pointing to hyperscaler invoice webhook logs for legacy compliance audit

Bidirectional legibility: a consumer can join with AWS credit history backfilled; a provider can leave with their fulfillment track record intact.

## Code anchors

| Surface | Path |
|---|---|
| Compute marketplace Angular surfaces | `app/elohim-app/src/app/shefa/` |
| Inference routing / compute-stewardship-elohim | `elohim/elohim-storage/src/services/inference_router.rs` |
| Inference engine service | `elohim/elohim-storage/src/services/inference_engine.rs` |
| System metrics (compute capacity probes) | `elohim/elohim-storage/src/services/system_metrics.rs` |
| Device capacity tracking | `elohim/elohim-storage/src/services/device_capacity.rs` |
| REA commitment service (provide/request-compute) | `elohim/elohim-storage/src/services/rea_commitment_service.rs` |
| Economic event service (matched/executed-compute) | `elohim/elohim-storage/src/services/economic_event_service.rs` |
| Standing projection (compute_standing) | `elohim/elohim-storage/src/services/standing_projector.rs` |
| Content store integrity (entry types, REA_ACTIONS) | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` |
| Attestation validator (Floor 8 + D.16 confirmer chain) | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs` |
| Attestation kinds whitelist | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs` |
| Shefa pillar manifest | `elohim/sdk/domains/shefa/manifest.json` |
| View schemas | `elohim/sdk/schemas/v1/views/` (commitment-view, economic-event-view, attestation-view) |
| bridges/aws/ (planned) | `bridges/aws/` |
| bridges/gcp/ (planned) | `bridges/gcp/` |
| bridges/azure/ (planned) | `bridges/azure/` |
| bridges/runpod/ (planned) | `bridges/runpod/` |
| bridges/vast/ (planned) | `bridges/vast/` |

## What this proves about the substrate

A skeptical systems architect should walk away from this archetype able to say:

- The Kafka-like event flow (REA Events with mass-conservation) can represent a real-time compute marketplace — capacity declaration, matching, execution, billing, payout — without a central scheduler, exchange, or billing platform; the substrate's primitives are the billing infrastructure
- The computation-attestation gradient (Witness / Audit / Proof / Confirmation) is a concrete verification mechanism tied to manifest-declared workload classes, not hand-waved "trust the provider"; escalation to audit is automatic when FeedbackSignal::Correction rates trip the threshold
- The care-class / compute-class isolation (D.18 per-class `StandingScore`) prevents a bad compute provider from accumulating social-trust damage and vice versa; this is a substrate-invariant, not a convention, enforced at the `signal_class` field in the integrity zome
- The long-tail math is concrete: 100M households with idle compute hardware at substrate-native rates ($0.05/core-hour) represents ~$24M/day of aggregate compute capacity that no hyperscaler can match on cost-per-cycle because they do not own the hardware
- The bridge-and-cash-out pattern (bridges/aws/, bridges/runpod/, bridges/vast/) means the substrate is not asking anyone to abandon their existing compute supply chain; the protocol subsumes incumbents by offering a better deal to contributors while remaining legible to consumers who currently use AWS
