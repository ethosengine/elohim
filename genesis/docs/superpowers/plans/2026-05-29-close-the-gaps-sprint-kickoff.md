---
status: Draft
cites:
  - 2026-05-29-light-up-the-topology-sprint-kickoff.md   # the related doc this derives from
---

# Sprint Kickoff — Close the Gaps: Make the Topology *Felt* (Producers · Convergence · Surface)

> **Kickoff prompt** for the sprint following the prioritizer end-state epic (2026-05-29).
> Ready to paste into `/shift` or a planning session. Doctrine unchanged: gates/determinism
> in Rust, elohim `.ts` sense-and-respond, **no new DHT entry types** except behind a
> `p2p-design-gate`, doorways stay thin, **let the CI pipeline + a2o do end-to-end verification**
> (story → spec → working code; the pipeline shakes out integration).

## Where we are (what the last epic landed)

The prioritizer end-state epic shipped working code on the deterministic floor (`e6300665c` → `21cb7e8b3`, lib suite 1300 green):

- **Reads (Epic B):** `PeerCapacityView` total/pledges/held readers — LIVE.
- **Wire (Wave 1):** `BlobInventorySnapshot`/`Delta` carry optional `BlobHint` (recipient_hub_id, epr_kind, size, tier) — additive, backward-compatible.
- **Hub identity (Wave 2):** `collectives.collective_cid`+`slug`, `collective_participations.member_cid`; `hub_resolver` (`agent→hub`, CID-canonical / slug-alias); `hub_capacity_service` real device members + governance-layer classification; imagodei DNA emits `Collective`/`Membership` post-commit signals; storage projects them.
- **Prioritizer (Wave 3):** broadcaster populates hints, receive arm scores advertised blobs against active `replicates-dwelling` commitments, HIGH-priority blobs feed a bounded fetch. **Commitments can now shape what peers cache.**

**The core truth of this sprint:** the substrate is *correct but starved.* The bars, the prioritizer, and hub aggregation are all wired and right — but they read **zero** in production because the **producers don't exist yet** and some representations haven't converged. This sprint makes it *felt*: real pledged-vs-held data flowing through the live readers and prioritizer, then surfaced to a person.

## Gaps, ordered by leverage

### Gap 1 — PRODUCERS (the keystone: activates everything already built) · *highest leverage*
**Outcome:** the pledged bar shows real numbers and the prioritizer actually fetches, because commitments and capacity samples exist.
- **`replicates-dwelling` commitment writer** — nothing creates these commitments today, so the prioritizer matches zero and the pledged bar is dark. Needs the pledge-creation flow: who pledges storage to which hub, the coordinator that writes the `Mishpat::Commitment` (action `replicates-dwelling`, the `ReplicatesDwellingPayload` in `metadata_json`), validated by the existing `replicates_dwelling_validator`. This is the REA dwelling-hub replication flow (see `project_dwelling_hub_replication_pattern`). **Likely needs a brief `p2p-design-gate` confirmation** (it's the existing Commitment entry — no new type — but a new coordinator + creation surface).
- **`infrastructure:system-sample` emitter (+ observation gossip)** — nothing emits per-node capacity samples, so remote-peer `total_raw_bytes` reads 0 (local works via fs-probe). Needs a periodic probe (`system_metrics.rs` fns exist) → write an `observations` row (observer_cid = local peer) → gossip via the observation plane so peers see each other's capacity. **Scout the observation-gossip plane** (`OBSERVATION_LOG_PROTOCOL_ID` / `IROH_OBSERVATION_ALPN`, `elohim/observations/<kind>` topics) before scoping the cross-peer half.
- **Doctrine:** producers are deterministic substrate (probe → observation; pledge → commitment). No discernment here — that's the elohim ceiling later.

### Gap 2 — CONVERGENCE & CORRECTNESS POLISH (finish what the last epic deferred)
- **T6 — representation convergence:** align live-data reads (`humans.household_id` / `stewarded_nodes.household_id`) onto the canonical Collective CID via the slug↔CID alias, and converge seed commitments' `recipient_dwelling_hub_id` onto the CID where a DHT anchor exists (so the prioritizer hint↔commitment match is representation-stable once live Memberships project).
- **T3 `node→agent` pledge mapping:** hub-member pledge aggregation reads 0 because members are device node-ids but pledges key on agent CID (`KNOWN LIMITATION` in `hub_capacity_service.rs`). Map device → agent via `peer_identity_bindings`/`humans.agent_pub_key` so hub pledges aggregate.

### Gap 3 — SURFACE (make the live data visible) · *Epic A from the prior kickoff* · *high felt-impact*
**Outcome:** a person *finds and reads* the topology without knowing URLs.
- **Built:** cluster/peers pages, per-EPR ResilienceView Network tab, progressive ●◐○ icons, ambient connection chip; the readers now produce real held + local-total + hub data.
- **Gap:** top-level posture view (`ResilienceSnapshotView`/`NetworkPostureView`/`TopologyOverviewView` wire types exist, no consuming component); free/used is `<dl>` text not a clickable bar; peer/device cards aren't links (no drill-down). Pure Angular wiring on live wire-types (`angular-architect` lane) — no new substrate.

### Gap 4 — THE REMAINING EPICS (from the original light-up-the-topology kickoff)
- **Epic C — doorway Role-2 resolver** (peer-hosted EPR-apps): the big "new internet" lift; **own `p2p-design-gate`**.
- **Epic D — account-management surface** + recovery UX + post-recovery key rotation (imagodei M5; `angular-architect` + `rust-architect`).
- **Epic E — `<elohim-context-menu>` integration** into `EprLinkComponent` (primitive + stories built; zero app consumers).
- **Epic F — delivery stats** (bytes-served / who-pulled; the `blob-served` observation aggregation + endpoint; toll economics is a separate v1 decision).
- **Epic G — wisdom-input shape** (grow `wisdom.rs` to see placement-gaps/resilience/inventory; sense-and-respond only; sequence after producers).

### Gap 5 — DEPLOY & VERIFY (operator-owned + pipeline)
- **Alpha DNA forced-reinstall** for the new imagodei `Collective`/`Membership` signals (DNA-hash drift; all same-namespace peers reinstall or DHT-partition — see `project_dna_changes_dont_redeploy_without_forced_reinstall`).
- **CI sweettest + a2o** shake out the prioritizer epic end-to-end (DNA signal emit, multi-peer fetch round-trip, the `MemberKind`/`MembershipRole` wire contract, the felt experience).

## Suggested sequencing
1. **Gap 1 producers** (keystone — lights up the pledged bar + the prioritizer with real data). Commitment writer first (activates the prioritizer), then system-sample emitter (+ gossip after a plane scout).
2. **Gap 2 convergence/polish** (T6 + T3 pledge mapping) — makes the now-fed readers fully correct.
3. **Gap 3 Epic A surface** (`angular-architect`, parallelizable) — makes it felt/visible.
4. **Gap 5 deploy + CI/a2o** — verify the whole chain end-to-end (continuous, the pipeline's job).
5. **Gap 4 remaining epics** (C resolver = own gate; D/E/F/G) by felt-value.

## Story-first framing
Each gap should hang off a learner/steward scenario. The felt north-star: *a steward sees their household's real free-vs-pledged-vs-held storage, watches a peer's commitment actually pull a blob to where it belongs, and never types a URL.* Find/write the a2o scenario in `genesis/a2o/features/` (shefa storage-stewardship + elohim topology), implement to make it pass, let the pipeline run it.

## Grounding cross-refs
Prioritizer epic plan: `2026-05-29-prioritizer-end-state-wire-hub-fetch.md`. Prior kickoff: `2026-05-29-light-up-the-topology-sprint-kickoff.md`. Memory: `project_hub_identity_cid_canonical_slug_alias`, `project_storage_tiering_placement_intelligence`, `project_dwelling_hub_replication_pattern`, `project_rea_compute_commitment_primitive`, `project_substrate_floor_elohim_ceiling`, `project_dna_changes_dont_redeploy_without_forced_reinstall`, `project_placement_signals_are_shefa_inputs`.
