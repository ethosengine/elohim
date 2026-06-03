---
id: project-compute-commitment-first-instance-pivot
name: compute-commitment-first-instance-pivot
description: Sprint 1 (2026-05-28) shipped Mishpat::Commitment substrate primitives but abandoned Z.D (SPA deploy) as the first instance. Deploy is authorship-delegation not compute-delegation. Recommended first real instance is mutual storage replication between family-network peers.
metadata:
  type: project
cites:
  - mutual-storage-replication-dwelling-hub-design | the spec this pivot recommends — mutual storage replication as the real first compute-commitment instance | sha256:5596799dbb456bc2
---

Sprint 1 (`2026-05-28-sprint1-zd-substrate-correct-deploy.md`) ended scope-narrowed: substrate primitives shipped (T1–T7) but Z.D-as-first-instance was abandoned mid-flight.

**Why the pivot:**

The Z.D framing cast SPA bundle deploy as the first instance of the REA compute-commitment pattern per `[[project_rea_compute_commitment_primitive]]`. The design conversation surfaced that this is a miscast — deploy doesn't fit any of the three real compute-commitment use cases:

1. **Mutual storage replication between family-network peers** — bounded reciprocity ("I host N GB for you; you host M GB for me"); bounds_validator enforces symmetry; FeedbackSignals carry weight.
2. **Doorway projection compute agreements** — compute cost for projected EPR-app content flows to stewards/collectives that approved the agreement; needs a metering model on doorway first.
3. **Distributed workloads** — REA agreements for Jenkins-shape compute tasks but using EPR-components / storage commitments / p2p sccache instead of pods; needs peer-bidding + scheduling layer.

Deploy isn't mutual (no reciprocity from deploy-svc bot to operator-steward), isn't ongoing projection compute (one-shot publish), isn't distributed workload (single-CI publish). It's an **authorship-delegation** ("this bot can write content on my behalf") miscast as compute-delegation. The substrate-correct replacement for the Z.1 anti-pattern (PATCH /db/content/{slug}) is **seeder-based**: seeder creates a Content node via conductor with `contentType=spa-bundle` and the new blobHash; operator-steward's signing identity IS the authority. Separate smaller sprint.

**What stays useful (substrate primitives shipped Sprint 1):**

- `Mishpat::Commitment` DHT entry type with action discriminator (`delegates-compute`, `acknowledges-reach-change`)
- elohim DNA `EconomicEvent` action `republish-epr` + coordinator + integrity validators
- `republish_epr_validator` reference service (template for any per-instance validator using the bounds-validator-pattern)
- `put_epr` handler substrate-correct 503 when `event` is present and no `HcClient` bridge wired

**Recommended first real instance: mutual storage replication.**

Next-bootstrap step is compute agreements between peers so resiliency epics can be proven end-to-end:
- Each peer computes aggregate views: **free-storage capacity vs stewarded-compute commitments**
- Each content item computes **resiliency + delivery metrics** (active replication commitments coverage, steward-graph distance, projected fetch latency)
- `bounds_validator` runs on every replication-commitment author (capacity? scope? rate? key-rotation?)
- Standing debits via `signal_weight_registry`: `bad-custody` (peer revoked unilaterally), `rate-limit-exceeded` (over-committed and dropped), `reach-escalation-pending` (unsanctioned scope expansion)

**How to apply:**

When designing a substrate-correct flow, before picking it as the first instance of the compute-commitment pattern, ask: is this *mutual*, *ongoing-projection*, or *distributed-workload*? If none, the use case is probably an authorship-delegation (use signing identity directly) or a content-publish (use seeder/conductor for notarization), not a compute-commitment.

**Related:** `[[project_rea_compute_commitment_primitive]]` (gospel-tier shape), `[[project_bounds_validator_pattern]]` (substrate primitive landed Sprint 2), `[[project_signal_kind_extensible_protocol_class]]` (weight registry pattern), `[[feedback_a2o_narrative_is_opus_work]]` (a2o narrative requires Opus — sprint deferred its scenarios).
