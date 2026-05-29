# Durability, Topology & Felt Resilience — the Origin Half

> **Status:** Vision / design note (companion to `2026-05-29-epr-reachability-economics.md`).
> Captures the durability/persistence axis of the model — how P2P subsumes web2's
> *reliability* (cloud uptime, institutional protection, account recovery) — and grounds the
> "light-up-the-topology" felt surface in what **already exists in-repo** (mapped by a 6-agent
> grounding sweep, 2026-05-29) vs the "it just works" endgame. Most is built; the gap is
> connective tissue, committed-accounting, and joining the elohim ceiling to the floor.

## The thesis, in one line

Where the reachability note covered the **edge** (thin doorways, anycast, content-addressed
reach), this covers the **origin** — a self-healing, contract-backed, *verifiable* durability
substrate. Together: a content network with no center. The durability story is the answer to
"how does P2P become as reliable as the cloud — *just as good if not better*."

## Three axes, one gradient (recap + extension)

- **Reach** = permission (who may *see*).
- **Delivery** = economics (who pays to *move* it now).
- **Persistence** = durability (who guarantees it *survives*) ← this note.

The cloud/bank/platform bundle all three into one faceless provider — that bundle is the
chokepoint. The substrate unbundles them into governable, **verifiable** contract types. And
all three — plus **recovery** (who *restores*) — graduate along the *same* intimacy gradient:
**self → dwelling → collective → commons.** Your people are simultaneously who you share
with, who holds your bytes, and who helps you recover. That unification is why durability is
*grandma-legible*: it has **names**, where the cloud is faceless.

## Asserted → Attested → Ambient

- **Cloud = asserted.** "Eleven nines" is an SLA PDF you cannot verify, behind a single point
  of policy (deplatform / price / breach).
- **Substrate = attested.** N independent replicas held by people with skin in the game; the
  durability contract is *notarized and auditable* (you can verify your data is held, not
  trust a promise); a `reciprocity-imbalance` signal fires the moment a holder lapses.
  **Verifiable durability beats claimed durability.**
- **Endgame = ambient.** As the network matures, the attestation *recedes* into "it just
  works": a progressive icon next to a title, a free/used bar you *could* click but rarely
  need to, with the elohim collapsing the operational complexity of balancing it — **on top
  of** a deterministic floor, never replacing it.

Honest caveat: cloud reliability is *capital-intensive* (datacenters); substrate reliability
is *social* (enough capable peers who care). It degrades gracefully (lose replicas → RS-recode
→ re-replicate) but depends on **replication breadth** — the long-tail/boutique case is the
weak spot (same frontier as reach). "Better" holds *when breadth is sufficient*; the tiered
contracts + RS quilt + demand-driven replication exist to make that the default.

## The three web2 reliability functions, each subsumed

- **Cloud uptime/durability** → tiered replication contracts (`replicates-dwelling/-collective/
  -commons`) + RS-coded quilt + demand re-replication. Verifiable, not claimed.
- **Institutional protection** (insurance/banking = *pooled* loss protection) → the
  collective/commons replication tier **is** pooled durability risk; the dissolution principle
  finishes it — insurance becomes a sensemaking collective whose function is a
  replication+reciprocity primitive, with the commons-elohim co-steward holding the commons
  interest. Protection without an extractive institution.
- **Account recovery** → graduated recovery authority + socially-derived security. Better:
  no single seizable custodian, consent-graduated, survives any single party vanishing.

## Grounding: what already EXISTS (built) vs the endgame

The felt-durability surface is **not aspirational — it is substantially live.** The progressive
disclosure spine (icon → free/used bar → per-EPR drill-down) is real today:

| Layer | Built today | Where | State |
|---|---|---|---|
| **Progressive icon** | `●/◐/○` (≥3 / 1–2 / 0 stewards; green/yellow/none) next to an EPR title, hover "Stewards: N · Status" | `EprRelationshipCardComponent` (elohim-app) ← `ResilienceService.getContentResilience()` | **live** |
| **Ambient "is the network up"** | header chip, live peerCount + steward/hosted/offline mode, 30s poll | `ConnectionIndicatorComponent` ← `/p2p/status` \| `/health` | **live** |
| **Free/used/stewarded** | per-device byte triptych | `DeviceTileComponent` (rendered as `<dl>`, not yet a clickable bar) | **live (data) / stub (bar UI)** |
| **Cluster page** | device count/online/sleeping, storage X-of-Y, hosting reciprocity, freshness | `MyClusterComponent` `/shefa/cluster` ← `cluster_view.rs:51` federated aggregate | **live (no nav link)** |
| **Peer topology** | peer-household edges, reciprocation count, sole-replica resilience-cliff warnings | `PeerTopologyComponent` `/shefa/peers` ← `peer_topology` aggregate | **live (no nav link, raw json drill-down)** |
| **Per-EPR drill-down** | full `ResilienceView`: encoding/fault-tolerance, shards-with-locations, distinct peers, steward allocations, committed bytes, per-shard map | lamad content-viewer **"Network" tab** ← `GET /api/v1/resilience/{id}` (`api/resilience.rs:89-188`, deterministic from shard_manifests + shard_locations + stewardship_allocations) | **live** |
| **Per-content badge in lists** | "protectionStatus — N households" | `ResourceExplorerComponent` | **live** |
| **Top-level posture** | `ResilienceSnapshotView`/`NetworkPostureView`/`TopologyOverviewView` wire types exist | generated types, **no consuming component** | **absent** |

## The deterministic floor (real, judgment-free) — what the elohim sit on

The floor cleanly separates **gates** from **allocation**. Gates are real, pure, and
judgment-free: the 7-check `bounds_validator` (total-order reach rank, first-failure trail),
the donut clamp (DNA floor/ceiling consts + residual-collective so tiers sum to 100,
`constitutional_ratio_registry`), inventory structural verification (canonical sha256), the
content-addressed replication state machine (`replication.rs` discover/retry/caught_up),
`placement_gaps.rs` (gaps as a projection), `household_resilience.rs` (protection_status
per-request). `cluster_view.rs` reconstructs free/used (Category C, `system_metrics`) +
stewarded (Category A, `rea_commitments`) per-request, explicitly *not persisted* — and the UI
degrades permissively (a failed stewardship lookup never blanks the view). **This is the
"strong deterministic footing" required before any elohim judgment is layered on.**

## Three gaps to "it just works" (the runway)

1. **Connective tissue / discoverability.** The surfaces exist but aren't woven in:
   `/shefa/cluster` and `/shefa/peers` have **no nav links** (URL-only); EPR mentions don't
   link to the content-viewer Network tab; the free/used "bar" is text not a clickable bar;
   the top-level posture view (`ResilienceSnapshotView` et al.) has no consuming component.
   *This is mostly wiring, not new substrate.*
2. **Committed / constitutional accounting.** Observed reality is live; **committed** reality
   is stubbed. `peer_capacity_service` stub readers make the free/used capacity bar read zero;
   the donut/pledge accounting and `replication_prioritizer` (currently **dead code**, not
   wired into `drain_gap_queue`/the inventory subscriber) are Sprint-3 stubs. Closing these
   turns the bar from "observed bytes" into "pledged-vs-held with the storage-premium visible."
3. **Joining the elohim ceiling to the floor.** Both halves exist — the deterministic floor
   *and* a live LLM wisdom gate (`Phase::DevContext` vs `ElohimActive` observed from real call
   outcomes) — but they are **not joined for topology**: `wisdom.rs:28`'s input shape accepts
   only constitution/framing/event_summary, so no agent can yet reason over placement-gaps,
   resilience snapshots, or inventory advertisements. Until that input shape grows + the
   prioritizer is wired, the balancing intelligence is backend-deterministic only — no agent
   "collapses the operational complexity" yet. **Doctrine holds: gates/determinism in Rust;
   the elohim .ts is sense-and-respond, never the evaluator.**

## Recovery (the third face of the gradient)

Graduated-recovery doctrine is canon (intimate quorum → community → governance → global elohim
witness; Shamir optional hardening), and the imagodei DNA already carries `KeyRotation` + the
five `RecoveryAuthority` layers; the M5 `account`-pillar path is partly live. The note-worthy
property: the people who **hold** your data (persistence) and the people who **restore** your
access (recovery) are the *same* intimacy tiers — which is what lets "institutional protection,
just as good if not better" be something a grandmother *trusts* rather than is told.

## Why this matters now

The EPR-app delivery sprint lights the *reach* path (named front doors). This note is the
*persistence* companion: the durability surface is already 70% built and sitting on a solid
deterministic floor — the highest-leverage next epics are (1) the connective-tissue/posture UI,
(2) the committed-accounting readers + wiring the prioritizer, (3) the wisdom-input shape that
lets the elohim finally collapse topology complexity. None of it requires new DHT entry types;
it is wiring, readers, and one input-schema growth on a substrate that already attests.

## Grounding cross-refs
- Companion: `2026-05-29-epr-reachability-economics.md` (reach + delivery axes).
- Memory: `project_substrate_floor_elohim_ceiling`, `project_placement_signals_are_shefa_inputs`,
  `project_elohim_agent_sense_respond_architecture`, `project_dwelling_hub_replication_pattern`,
  `project_household_is_resilience_unit`, `project_graduated_recovery_authority`,
  `project_recovery_grandma_standard`, `project_trust_as_efficiency_signal`,
  `project_node_metrics_vs_hub_aggregation_boundary`.
