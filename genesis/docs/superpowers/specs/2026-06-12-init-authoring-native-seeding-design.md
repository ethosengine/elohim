---
id: init-authoring-native-seeding-design
cites:
  - genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md
  - resilience-dimensions-proof-suite | the D1/D2 boundary tests this spec extends with an authored+stocked measured case; its @wip rows are the multi-peer acceptance gate | sha256:a89f58ec4906e152 | path: genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md
  - tiered-quilt-stewardship-design | the custody-quilt / RS(N,K) replication canon this rides toward; replica_target_for(reach) and custody-blob commitments are the resiliency-replication primitives borrowed here | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
---

# Init-authoring: seeding = init through the front door

**Date:** 2026-06-12 (evening)
**Status:** Approved (operator scope cut — supersedes the historical-provenance design)
**Supersedes:** `2026-06-12-provenance-manifest-ingestion-digest-design.md` (commit 761eee27b)
**Owner surfaces:** seeder front-door author loop (Che join-alpha; later a ci-steward
conductor service); the existing `create_content` → shard-encode → `upsert_manifest`
path in elohim-storage; agent-key custody ceremony (operator)

## The scope cut (why this replaces the prior spec)

The prior spec modeled **declared history**: a Provenance Manifest v1 artifact (canonical
dag-cbor, trusted-issuer-signed, derived from git logs by a sealer CLI) plus an ingestion
digest that replayed that declared lineage through coordinators, with brit as stage-2
author.

The operator cut that entire half. **Seeding = init.** When content is written to the
network, that *is* its birth. There is no git-era lineage to declare, no manifest artifact,
no sealer CLI, no "declared-not-witnessed" attestation discriminator, and brit drops off
the critical path completely. (If historical provenance ever matters, it can be grafted
later as an enrichment attestation — explicitly out of scope here.)

What replaces it is smaller and truer: a conductor-bearing agent authors content through
the **real CRUD front door**, so seeded content is birthed exactly the way a user's content
is. No replay machinery, no new wire format. The remaining design is not "how do we declare
the past" but **the relationship design** the operator named: who has standing to author,
who the author-steward is, who replicates for custody (resiliency) vs. who replicates for
serving (projection), and how many peers do each per reach class.

## Core principle: init through the front door

A conductor-bearing agent authors content via the **existing** create path
(`create_content` / `POST /db/content`). The substrate does the rest natively:

- **Anchored** — the EPR Head publish drain loop (`p2p/mod.rs::drain_publish_queue`) is the
  sole writer of `p2p_published_at`; new content becomes externally visible within one
  drain interval. The bulk-seed anchor gap (NULL `dht_anchor_hash`) does not arise because
  the content took the front door, not a direct DB write.
- **Witnessed** — DHT validation runs on author; reach is earned at authoring through the
  real reach gate.
- **Signal-projected** — post-commit signals project to elohim-storage natively; no
  hand-written projection rows.
- **Stocked** — for blob-bearing content the front door **already** runs shard-encode +
  `upsert_manifest` (`http.rs` POST `/db/content`, the `tokio::spawn` after
  `services.content.create`). That manifest row is exactly what flips
  `distributionState` from `unmeasured` to `measured` (the unmeasured≠zero honesty work
  that landed on the wire today).

**Zero new DHT entry types. Zero new tables. Zero replay machinery.** Everything below
reuses substrate that already exists.

## The one mechanical piece: stock-on-author for already-uploaded blobs

The stock step is almost entirely already built. The front-door POST encodes + upserts the
manifest *when the create carries `blobHash`*. The one gap: when a blob was uploaded in a
separate step (the common seed shape — upload bytes, then author the content row referencing
them), nothing re-encodes. So the single new surface is a **stock trigger for an
already-uploaded blob**: invoke the *same* `ShardEncoder::create_manifest` +
`upsert_manifest` against an existing blob hash. Same code path the POST already runs; it is
"stock this blob now" rather than a new encoder.

- The authoring/stocking node authors **its own** manifest — same route the front-door POST
  runs, on the same node. No cross-node manifest authorship, no fan-out.
- `distributionState` flips to `measured` **only** because real bytes shard-encoded on a
  real node. It is never declared. (See the honesty line below.)
- The stock surface is the only thing this spec adds to storage; it is named in the
  implementation plan, designed last per the gate.

## P2P design gate (passed 2026-06-12)

No new DHT entry types, no new tables, no new wire formats. Every entity maps to existing
substrate.

| Entity | Classification | Source of truth | Coordinator | Address |
|---|---|---|---|---|
| Content (init'd) | Notarized (A) — existing `Content` entry; authored via front door so `dht_anchor_hash` is set by the publish drain, never NULL | Holochain DHT | `create_content` | CID (content-derived) |
| Author/seed standing | Notarized (A) — existing `Mishpat::Commitment`, `delegates-compute` action; provider→recipient bounded standing to emit author/seed events | Holochain DHT | `create_rea_commitment` (elohim DNA `content_store`) | CID = entry_hash |
| Custody (resiliency) | Notarized (A) — existing `Commitment`, `custody-blob` action; a peer collective commits to hold the author's blobs | Holochain DHT | `create_rea_commitment` | CID = entry_hash |
| Content-reach backing | Notarized (A) — existing `Commitment`, `provide` action, `content:<reach>` scope; the D3 join's input | Holochain DHT | `create_rea_commitment` | CID = entry_hash |
| `shard_manifests` row | Operational (C) — measured reality, authored ONLY by real shard-encode/stock | SQLite | (n/a — `upsert_manifest`) | content_id + h_app_id |
| Projection cache (serving) | Operational (C) — doorway blob-tier cache; rebuildable from custody peers; NEVER a custodian | SQLite/pantry (doorway-local) | (n/a) | blob hash |

Constraints carried out of the gate:

- **`Commitment` CID is the entry_hash**, never the action_hash (the bounds-gate/fetch key;
  returning action_hash silently breaks every bounds check yet passes per-task tests).
- **Commitments reach `active` via the real conductor transition.** An HTTP POST inserts
  `proposed`; never hand-edit state. The D3 join counts only `active`.
- **Workstream-D junctions stay substrate-owned.** `humans.household_id` and collective
  regions have no HTTP create surface (Epic B). Init *consumes* them; it does not invent a
  write path. Name the gap; do not paper it.

## The relationship design (the heart of the spec)

The operator's open questions, answered through the gate, the REA compute-commitment
primitive, and the tiered-quilt canon. Where the answer is genuinely open it is marked
**OPEN** with its owner — not designed here.

### a. Compute relationships — who has standing to author/seed

Modeled as the gospel-tier REA primitive: a `Mishpat::Commitment` with the
`delegates-compute` action, provider→recipient, bounded, revocable, auditable. This
displaces X-API-Key-style admin grants — standing is checkable (walk `bounded_by` to the
Commitment), revocation is real, the authority chain is itself notarized, reciprocity is
observable.

- **Local stack (stage A):** the local conductor's own agent authors as itself — it owns
  the content it births; no delegation needed (provider = recipient).
- **Che join-alpha (stage B):** the devspace conductor is a *recipient*. The operator
  steward is the *provider*. The `delegates-compute` commitment scopes the devspace agent
  to author/seed events bounded by `{ reach ceiling, EPR scope, rate, key-rotation TTL }` —
  the deploy-authority instance row of the primitive, reused verbatim for seeding.
- **ci-steward (stage C):** the long-lived ci-steward conductor service is a recipient
  under the same primitive (one durable agent, operator-held key custody). Ephemeral build
  pods talk *to* it; they never embed conductors (fresh-key-per-run is key-churn poison;
  source chains are per-agent). Concurrent builds serialize on the one source chain —
  acceptable at seeding cadence.

### b. Author-steward identity for init'd genesis content — **DECIDED (operator, 2026-06-12)**

Authorship-stewardship is **per-corpus, routed to real personas and their collectives** —
not one synthetic genesis identity:

| Corpus / collective | Author-steward | Fixture anchor (existing `genesis/data/collectives/collectives.json` ids where they map) |
|---|---|---|
| **Genesis content (the protocol corpus)** | **adam** | adam's collectives: `couple-adam-eve`, `family-eden` / `household-eden` |
| Dowell family collective (matthew, jessica, james) | matthew (author) | `family-dowell` / `household-dowell` |
| EthosEngine collective | matthew (author) | `org-ethosengine` |
| Adam–Eve family collective | (exists as its own collective) | `couple-adam-eve` |
| Extended-family collective — gertrude + matthew, jessica, james + the Seattle-area couple | (membership per operator statement) | closest existing fixture: `neighborhood-extended` / `household-extended` ("Extended Network") — **mapping to confirm**; note the Seattle members give this collective a real cross-region member set (the D5 regional-distribution dimension's first honest test bed) |
| Church collective — matthew, jessica, james + one more member (operator: "someone else, can't remember") | matthew (author) | `community-local-church` (Valley Community Church); fixtures suggest the fourth member is `pete-pastor` — **to confirm** |
| **FCT corpus** (`paths/foundations-christian-technology`) | matthew (author); routes WITH the church collective | content→collective routing example: a corpus whose steward is NOT the genesis default |

Implications locked by this decision:
- The digest/authoring flow needs a **per-corpus steward routing input** (default: adam for
  genesis content; explicit overrides like FCT→church). One small mapping, not a new entity —
  it resolves to which agent authors the content + which collective's junction rows light.
- adam is a **live alpha conductor** (shem node) — the genesis author-steward is a real peer
  whose authored content is immediately custody-eligible by the M/J/J household mesh
  (cross-household custody between eden and dowell collectives is the first real D1 ladder).
- The junction rows themselves (`humans.household_id`, collective membership, regions for the
  Seattle members) remain **Epic B ingestion work** — this decision names exactly which rows
  to create, which was the gating unknown.

### b.1 The inferred collective graph (story-derived — the stories ARE the membership table)

`genesis/data/collectives/collectives.json` declares collective *identities* but lists no
members — deliberately. Membership and the relationships between collectives are carried by
the canonical stories (`genesis/data/stories/`, operator 2026-06-12: "there are collectives
that can be inferred from these stories, that's why they're so rich"). The connected graph,
with story evidence:

| Collective | Members (inferred) | Evidence |
|---|---|---|
| `household-dowell` | matthew + jessica (co-stewards), james (stewardee) | every story's `characters:`; james-and-the-spoke (stewarded-device-sync) |
| `household-gertrude` | gertrude (her always-on home-nuc hub on shem) | gertrude-holds-the-share; her hub *is* the share's resting place |
| **Extended-family collective** = the reciprocal-backup/intimate-circle web | gertrude + matthew + jessica + james + the Seattle-area couple | the recovery web IS the collective: household-dowell ↔ household-gertrude hold each other's shares (both counterparty stories); gertrude's five share-holders include matthew + jessica; matthew's custodians are "Jessica, Adam, and Abby" (`recovery-shamir-optional.feature:14`) — Abby is plausibly one of the Seattle couple (**no `human-abby` fixture exists — names the missing Seattle-couple fixture work**) |
| `community-local-church` | matthew, jessica, james + **pete-pastor (CONFIRMED by evidence)** | pete-pastor's presence is *established by* the FCT content (`establishingContentIds: ["foundations-christian-technology"]`, `affinityDomains: ["fct","scripture"]`) — the fourth member and the FCT→church routing corroborate each other |
| `community-homeschool-coop` | terrance (collective-steward), jessica (member/proposer), james + sarah (students) | the-coop-decides (ranked-choice curriculum; collective-as-actor) |
| `couple-adam-eve` / `household-eden` | adam + eve (Eden Valley) | humans fixtures (core-family); adam is ALSO matthew's recovery custodian — a real cross-collective tie between eden and dowell |

Two load-bearing consequences:

1. **The inferred edges are commitment edges, not just membership rows.** Reciprocal
   backup-stewardship = recovery-share custody = the same `Commitment` substrate this spec
   replicates content with (the compute-commitment primitive's "recovery quorum"
   instantiation). Epic B ingestion should therefore derive BOTH the membership junctions
   AND the inter-collective custody relationships from the same story-declared graph — the
   stories are the seed manifest for the social layer, the way git was almost the seed
   manifest for content history.
2. **Geography is already in the fixtures**: gertrude (Sunset Acres) ≠ adam/eve (Eden
   Valley) ≠ the Seattle couple — three regions across the extended-family web, lighting D5
   honestly the moment the junctions exist.

Residual fixture gaps this names: the Seattle-couple humans (incl. resolving "Abby"), the
extended-family collective row (closest existing: `neighborhood-extended`/`household-extended`
— or a new explicit `family-extended` row), and member lists formalized wherever Epic B
decides junctions live.

### c. Resiliency replication (custody) — who holds the author's blobs

Custody is a `Commitment` with the `custody-blob` action: a peer collective commits to
*hold* the author's blobs. This is the `external_committed` input the D8 triptych already
sums (custody-blob rows whose provider is one of my bound peers). Replica targets are
reach-driven — the existing `replica_target_for(ReachClass)` map is prior art:

| ReachClass | custody replica target |
|---|---|
| Private | 2 |
| Intimate | 4 |
| Household | 6 |
| Neighborhood | 8 |
| Collective | 10 |
| Community | 12 |
| District | 14 |
| Public | 16 |

These are structural floors (operator-tunable later). The tiered-quilt `custody-quilt`
floor + RS(N,K) shard math is the deeper mechanism this rides toward; init'd content enters
at the simplest rung — one stewarding node holds the manifest it authored, and custody peers
accrue from there as real `custody-blob` commitments land. **The custody peers replicate by
actually holding bytes; the count grows only as the substrate replicates.**

### d. Projection/distribution (serving) — kept strictly distinct from custody

Serving is the doorway projection-cache plane: a CDN edge that stocks the blob-tier cache
on a clean 200 and serves subsequent reads locally. Per doorway gospel:

- **Doorway is a projection, never a custodian.** It does not fan out, does not iterate
  peers, does not decide which peer holds bytes. If a routed peer lacks the blob, that is a
  substrate replication gap to fix in elohim-storage's P2P layer — never a doorway fix.
- **Inventory exchange ≠ byte replication.** Gossip metadata is not custody; only the
  custody peers (c) actually hold the bytes.
- A doorway cache entry is Operational (C), rebuildable from the custody peers; it carries
  no `tier_floor`, no custody commitment.

Custody (c) is *who survives a flood*; projection (d) is *who serves the read fast*. They
are different peers playing different roles and must never blur.

### e. How many peers per role, per reach class — and what's measurable TODAY

| Role | Count rule | Measurable today on M/J/J household mesh? |
|---|---|---|
| Author-steward | 1 (the authoring node) | **Yes** — matthew/jessica/james each author their own content + commitments natively on their own conductors |
| Custody (resiliency) | `replica_target_for(reach)` (2–16) | **Partial** — household reach target = 6; the M/J/J mesh can demonstrate a small real custody set (3 conductors). Full targets and cross-region breadth wait on `@requires:shem`. |
| Projection (serving) | unbounded; doorway replicas + federated CDN | **Yes** — doorway-alpha already caches + serves; not custody, so no reach target |
| Content-reach backing (`provide`) | distinct households via D3 join | **OPEN/@wip** — provide rows exist only in test_util today (Epic B); M/J/J can author real provide commitments once the junction (b) is named |

What is honestly demonstrable today is a *small real custody set on the household mesh*:
matthew, jessica, and james are live fixture agents with their own conductors who can each
author their own `custody-blob` / `provide` commitments natively. That is a genuine
multi-peer mesh (doorway-alpha `/health` peerCount 2 — the household floor), not a
single-node demo. Breadth (full reach targets, cross-region diversity) is `@requires:shem`.

## The honesty line (preserved)

- **`distributionState` flips to `measured` ONLY via real shard-encode on a real node** —
  the stock step doing the work, never declaring it.
- **Counts grow only as the substrate actually replicates.** One stocking node = one
  measured stewarding node; custody and provide counts rise only as real commitments land
  and real bytes replicate.
- Today's unmeasured≠zero landing is the safety net: any gap renders as a distinct "not yet
  distributed" state, never a fake at-risk verdict, while the relationships fill in
  incrementally.

## Staging

- **Stage A — local stack, end-to-end.** One demo item: author it through the front door
  (with a blob) on the local stack; stock fires; the resilience snapshot reads `measured`
  with nonzero stewarding count. Proves init=birth produces a measured card with real (small)
  numbers, no replay.
- **Stage B — Che join-alpha, one item into alpha.** `NETWORK_PROFILE=join-alpha pnpm run
  hc:start` joins the devspace conductor to the alpha DHT via the deployed doorway's
  bootstrap+signal (now speaking kitsune2, the enabling substrate landed today) with
  DNA-parity fetch (`fetch-deployed-dna.sh`). Author **one** item under a `delegates-compute`
  commitment. **Writes to alpha are an operator ceremony** — the standing agent rails exclude
  bulk writes on shared alpha; one-item authoring is the bounded, operator-gated act.
- **Stage C — ci-steward service design.** Identity, agent-key custody ceremony, and the
  `delegates-compute` compute commitments for a long-lived conductor service that ephemeral
  build pods talk to. Design only here; the service itself is its own sprint.
- **Sequencing rail:** live-substrate verification of the relationships sequences *after*
  the proof-suite's Layer-1 custody-convergence reads green; until then everything proves on
  the local stack and the household mesh.

## Testing

- **Layer-1 extension (D1/D2 boundary tests, `tests/household_resilience.rs`):** add an
  **authored+stocked** case — author a content row with a blob, run the stock step, assert
  `distributionState: "measured"` AND nonzero stewarding/peer counts. This is the positive
  twin of the existing degenerate `unmeasured` row.
- **A2O:** one scenario — *seeded content is authored content*
  (`genesis/a2o/features/resilience/seeded-content-is-authored-content.feature`): author one
  item through the front door → snapshot reads `measured` with a real steward, identical in
  shape to a user-authored item. Household floor by default (M/J/J); `@requires:shem` only
  for breadth rows.
- **Per-item failure isolation preserved:** one bad item skips with a logged reason, never
  aborts the corpus (the EprRouter poisoned-row lesson).

## Out of scope

- Historical/git-era provenance of any kind (the cut half). If it ever matters, it grafts
  on later as an enrichment attestation — a separate, future decision.
- brit on the critical path (dropped entirely; brit Phase 2 "git artifacts become protocol
  content" remains on brit's own roadmap, unconnected to seeding).
- The ci-steward service *implementation* (stage C designs it; building it is its own sprint).
- Epic B junction ingestion (named dependency — the author-steward `household_id` row and
  `provide` rows outside test_util).
- Multi-peer replication *mechanics* and full reach-target convergence (tiered-quilt /
  workstream D; the resilience-dimensions `@wip` rows remain the acceptance gate).
