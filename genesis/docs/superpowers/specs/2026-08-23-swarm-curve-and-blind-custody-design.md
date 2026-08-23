---
title: "The swarm curve and blind custody — one dataplane pattern for sharded blob distribution and encrypted shard custody"
id: swarm-curve-and-blind-custody-design
tier: spec
status: Draft
created: 2026-08-23
maintainers: Matthew Dowell + Claude Fable 5
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete (every row in §8 landed or superseded) OR superseded-by-implementation
domain: peer-hoster dataplane (T2) × confidentiality plane (3.13) × custody/REA
habits: [blob-durability, dataplane-convergence]
topic: [sharding, reed-solomon, swarm, inventory-gossip, bitfield, blind-custody, key-envelope, x25519, custody-commitment, witnessed-harm-limit, quarantine, iroh, libp2p]
cites:
  - "doorway-federated-continuity-roadmap | Lanes S and C6 this spec is the design pass for; two grounding corrections (shard inventory IS gossiped; parity-aware completion is in flight) flow back to it | sha256:64cef78afe379745 | path: genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md"
  - "private-layer-blind-custody-resiliency-floor | the gate output this spec inherits (0 new entry types, composite-root commitments) and the two held decisions it answers: reader key = dual-key attestation, key ring travels with the ciphertext | sha256:1dd9950a41c2ff73 | path: genesis/docs/superpowers/plans/2026-08-09-private-layer-blind-custody-resiliency-floor.md"
  - "ownership-custody-inalienable-red-team-design | the floor rows this spec designs against — CSAM refusal-to-instantiate, inalienable subject rows, the erasure limit stated plainly | sha256:d80fea9b7bf8843f | path: genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md"
  - genesis/research/witnessed-harm-limit-research-2026-08-09.md
  - "elohim-seam-map-concern-routing | routes confidentiality to 3.13 (encryption ≠ permission) — the warrant for a custody verdict distinct from reach | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "weave-epic-arc-design | where the ShardManifest field-add and KeyEnvelope were HELD (Wave C); this spec retires the field-add (manifest is derived) and places the envelopes in a CID-addressed key ring | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md"
  - genesis/data/timeline/backlog/arch-confidentiality-plane-backlog.md
  - elohim/elohim-storage/src/p2p/blob_swarm.rs
  - elohim/elohim-storage/src/sharding.rs
  - elohim/elohim-storage/src/p2p/inventory_gossip.rs
  - elohim/elohim-storage/src/services/private_replica.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
memory_anchors:
  - feedback_reach_head_replication_distinct_planes
  - project_inventory_exchange_not_byte_replication
  - feedback-cleanup-toward-p2p-dataplane-trajectory
  - feedback_local_mesh_first_cadence
  - feedback-identity-sovereignty-ontology-guard
---

# The swarm curve and blind custody — one dataplane pattern

> **Why one spec.** The operator's two 2026-08-23 asks — *shards sync faster as more shards are
> replicated* (torrent-style aggregate bandwidth) and *Adam replicates Matthew's whole household,
> love map included, and can read none of it* — are one mechanism seen from two sides. Ciphertext
> shards are shards. If the public swarm is designed right, the blind replica inherits it for free;
> if the blind replica is designed right, it becomes the continuous load that makes the swarm curve
> measurable instead of deploy-bursty. This spec is the design pass the sprints were missing: it
> fixes the pattern, answers the P2P design gate for every entity either side touches, inherits the
> witnessed-harm working consensus at birth, and decomposes into rows the roadmap's Lanes S and C6
> can claim. It supersedes the *grounding* in those lanes where the tree disagreed with them (§1).

## 1. Grounded state (tree, 2026-08-23 — including uncommitted in-flight edits)

What is true, with the two corrections the roadmap needs:

- **Encoder.** `sharding.rs` bands: `none` (≤16 MiB, one shard), `chunked` (16–64 MiB, 1 MiB
  sequential, no parity), `rs-4-7` (>64 MiB). `create_manifest` is deterministic: same bytes +
  same `ShardConfig` → same manifest. The RS band only started working through `PUT /blob` today
  (`8854f6de5`); the read path reconstructs from ≥`data_shards`.
- **Shards are ordinary blobs.** `PUT` stores every shard under its own `sha256-` address
  (`http.rs` "Store each shard"), and the inventory broadcaster's `current_hashes` walks the blob
  dir (`http.rs:4920`, `blob_store::list_hashes`). **Correction to roadmap S1:** shard-level
  inventory *is* gossiped today — as N flat addresses per blob — and `fetch_shards_via_swarm`
  already does one `lookup_hosts` per shard (`blob_swarm.rs:148-158`). What is missing is not
  population; it is *shape* (§3.1).
- **Swarm.** `race_fetch_with_swarm` → `fetch_shards_via_swarm`: each shard races its own rotated
  holder list (`plan_shard_holders`), bounded by `total_inflight`; each landed shard persists with
  its own serve-blob REA credit; a manifest-only holder answers `FetchOutcome::Manifest`. A landed
  shard publishes an event-driven `BlobInventoryDelta` (`p2p/mod.rs:8058`), so serve-while-incomplete
  exists at the composite level.
- **In flight, uncommitted in the shared worktree (another session, 2026-08-23):** `reconstructible_threshold`
  / `should_start_shard_race` / `SwarmRaceOutcome::Reconstructible` (parity-aware completion) and
  `ShardRole` / `plan_shard_placement_slots` (data-shard-first placement). **Correction to the
  backlog rows `2026-08-23-swarm-parity-aware-completion` and `…-placement-keeps-data-shards-diverse`:**
  claimed and substantially built; this spec treats them as landing, not as open design.
- **DHT.** `content_store_integrity` already defines `ShardManifest` and `ShardLocation` entry types
  and the coordinator exposes `register_shard_manifest` / `get_shard_locations` — and **elohim-storage
  never calls them** (no caller in the crate). The manifest lives in SQLite `shard_manifests` and on
  the wire. That is the correct state and §4 makes it a decision, not an accident.
- **Custody.** `custody-blob` REA commitments (`reconcile/custody.rs`) are the replica set; salvage
  self-authors successors via `PlacementStrategy` (diversity-aware default). Custody push
  (`push_shard`) is shard-granular and libp2p-only.
- **Blind custody.** `services/private_replica.rs` proves encrypt → RS-4-7 → sealed-DEK →
  reconstruct-then-decrypt on one host (dryoc `crypto_box_seal`, X25519). `sealed_against_self.rs`
  already ships a nested 2-of-2 sealed box (mishpat ⊗ imagodei keys) — the reader-key *math* is in
  the tree twice; the reader-key *substrate* (an X25519 key bound to an `agent_cid`) is not.
- **Transport.** libp2p carries the swarm and custody push. `IrohNode::fetch_blob_from` has no
  production caller; `peer_blob_inventory.blake3_hash` exists and is always `None`. iroh-blobs is
  pinned (`=0.94`) but nothing addresses by BLAKE3.
- **Witnessed-harm limit.** A working consensus exists across the red-team spec (CSAM = refusal to
  instantiate, floor, fail-closed), the rights-floor plan, the 2026-08-09 survey, and two recorded
  council positions (Fable, Codex). Position is deliberately council-TBD; the *architectural* shape is
  settled enough to design against (§6).

## 2. The pattern — one invariant chain

```
author edge ──(private reach only: encrypt-then-shard; plaintext never reaches distribute_shards)──▶ bytes
bytes ──(deterministic ShardEncoder)──▶ manifest            DERIVED (C): identity = blob CID, re-derivable by any holder
manifest + shards ──(custody-blob commitments)──▶ replica set   the ONLY notarized objects (A, existing kind, composite root)
holders ──(inventory gossip: composite address + shard BITFIELD)──▶ peer_blob_inventory (C)
requester ──(fastest-k-of-n race: scarce-first order, rotated holders, per-shard REA credit)──▶ shards
each landed shard ──(delta with updated bitfield)──▶ the requester is a source before it is complete
```

Five rules, each load-bearing:

1. **The manifest is derived, never notarized.** Anyone with the bytes can recompute it; anyone with
   it can verify every shard. Its identity *is* the blob CID. Lighting `ShardManifest`/`ShardLocation`
   on the DHT would add one head per blob (and 7× that for locations) to a head plane that already
   quiesces in hours — for a fact the bytes themselves prove. Zero DHT writes for the public swarm.
2. **Custody commitments are the only notarized objects** — one per (manifest × custodian), the
   composite-root bundling the blind-custody plan already priced. Who *should* hold is notarized;
   who *does* hold is gossip (C); what the bytes *are* is the CID.
3. **Inventory says which shards, at composite cost.** A holder advertises the composite address once
   with a bitfield of the shard indices it holds, not N shard addresses (§3.1).
4. **Completion is k-of-n, scarce-first, serve-while-incomplete.** A requester is done at
   `data_shards` landed (in flight), orders shard races by ascending holder count, and advertises
   each landed shard before the composite exists.
5. **Encryption is orthogonal and precedes sharding.** The swarm is encryption-agnostic; a blind
   custodian serves ciphertext shards at full speed. For private reach the private path is the
   *only* path: `distribute_shards` / `PUT` never see private plaintext (§5, §6).

## 3. The curve — what makes it superlinear, and the four flatteners

**Model.** With `k` data shards of `n`, `h` distinct holders covering disjoint shards, each with
uplink `u`: a single requester's throughput ≈ `min(down, min(h,k)·u)` — 4× a single source at
`h ≥ 4` for `rs-4-7`, *and* it takes the fastest 4 of up to 7, which single-source fetch cannot.
Network-wide, every peer that lands one shard is a source for that shard on its next delta — the
BitTorrent flash-crowd shape — so full replication of a blob across N peers takes ~log₂(N) rounds
instead of N single-source transfers. RS is *better* than torrent here: a peer is useful after one
shard, and any k suffice, so rarest-first is a placement concern, not an availability one.

**Flatteners, and the cure for each:**

| # | Flattener (today) | Cure | Row |
|---|---|---|---|
| F1 | Inventory is N flat `sha256-` addresses per blob: a 64 MiB `chunked` blob = 64 entries (~4.5 KB) per snapshot; receivers cannot tell a shard from a composite; large snapshots hit the gossip frame limit and silently stop advertising | **Shard bitfield hint** on the composite address (`BlobHint.shards_held`, additive/sparse); shards stop being advertised individually once every peer reads hints | S1′ |
| F2 | `chunked` band (16–64 MiB) has no parity and 1 MiB pieces — every piece is required, the fastest-k property is absent, and it is the band most content lands in | **Band collapse:** retire `chunked` for new writes; `> SINGLE_SHARD_MAX → rs-4-7`; `> 256 MiB → rs-8-12` (already named in the DHT docstring). Reads keep `chunked` for existing manifests. Parameters are measured, not asserted (S3) | S5 |
| F3 | All-or-nothing completion; parity raced alongside data | Parity-aware completion — **in flight**, lands with the uncommitted `Reconstructible` outcome | (landing) |
| F4 | Holder order is rotation only: a scarce shard can wait behind abundant ones for an inflight slot, and every holder is weighted equally | **Scarce-first ordering** in `plan_shard_holders` (ascending holder count; pure, unit-testable) — cheap now. Latency-weighted holder choice (`advertiser_health` score) is a *later*, measure-driven step, not this pass | S6 |

What is **not** a flattener and must not be built: a tracker, a DHT-resident shard location table,
per-shard custody commitments, or a second hash namespace on the libp2p path. Each re-creates, on
the head plane or the gossip plane, what the bitfield + commitments already say.

### 3.1 The bitfield hint (the one wire change)

```rust
// p2p/inventory_gossip.rs — BlobHint, additive, #[serde(default, skip_serializing_if = Option::is_none)]
pub shards_held: Option<Vec<u8>>,   // little-endian bit i = shard index i of the manifest at `address`
pub encoding:    Option<String>,    // "rs-4-7" | "rs-8-12" | "chunked" | "none" — lets a receiver size the bitfield and know k
```

`peer_blob_inventory` gains `shard_bitfield BLOB NULL` (C — reconstructable from the next snapshot;
**source of truth: local (operational)**). `fetch_shards_via_swarm` derives `per_shard_hosts` from
bitfields first and falls back to per-shard rows during the transition. A holder with the composite
bytes sets all bits; a holder with shards only sets theirs. `current_hashes` keeps listing shard
files (the blob store does not change); the broadcaster folds them under their manifest before
emitting. Old peers ignore the hint; new peers ignore nothing.

## 4. P2P Design Gate output

### Entity: ShardManifest (derived)
- **Classification**: **Ephemeral (C)** — *decision*: the existing DHT entry type stays unused by
  elohim-storage. Reconstruction: re-run `ShardEncoder::create_manifest` on the bytes, or accept a
  peer's manifest and verify every shard hash. Identity = the blob CID (`bafkrei…`, raw codec;
  `blob_hash` `sha256-` is legacy wire). **Transport affinity**: `auto`.
- **Head-plane cost**: zero — that is the point. Lighting it would be ≥1 head per blob, 8× with
  locations; the blob already has a head through its EPR.
- **Anti-pattern check**: "bare sha256 as address" — present as legacy (`blob_hash`), not extended:
  new fields are CID/bitfield shaped.

### Entity: Shard bitfield hint / `peer_blob_inventory.shard_bitfield`
- **Classification**: **Ephemeral (C)** wire + projection; reconstructed from the next snapshot.
- **Network stakes**: all four; the hint is stage-invariant (no verification cost — it is a claim
  the shard fetch itself verifies by hash, C5 evidence-not-authority).
- **Concern canon (predicate `plan_shard_holders`, extended)**: C0 answered (dataplane, T2) ·
  C4 **answered** (a holder of *some* shards is now distinguishable from a holder of the composite —
  the blind-custody plan's unbound C4 closes here) · C6a answered (bounded by manifest length) ·
  C7 **partial** (a peer that advertises bits it cannot serve is detected only by the race miss;
  `advertiser_health` demotion is the later step) · C8 answered (`blob_swarm_shard_fetched{result}`,
  `distinct_source_peers`) · others n-a/registration-time. Registered in
  `elohim-storage/seam-registry.yaml` when the code lands.

### Entity: Swarm completion predicate (`reconstructible_threshold` — in flight)
- Pure decision predicate; C6a answered (stops admitting races at k), C6b answered (idempotent
  persist per shard), C4 answered (`Reconstructible { missing_parity }` is an honest partial, not a
  `Hit`). Register with the landing commit.

### Entity: BlobCustodyCommitment (public and blind)
- **Classification**: **Notarized (A), existing kind** — `rea_commitments action='custody-blob'`,
  unchanged for public; blind adds `resource_classified_as: "content:private-encrypted"` +
  `readable: false` (as the 2026-08-09 plan). **Address**: agent-scoped composite
  `(custodian agent_cid, manifest CID, "custody-blob")`. **Head-plane**: composite root — one per
  (manifest × custodian), never per shard; a household's backup set is one manifest tree. Seed
  O(10); 1 yr low hundreds. **Stakes**: placement stage-priceable; **confidentiality
  floor-protected** (no `DEV_MODE` plaintext path, ever). **Coordinator**:
  `content_store::create_rea_commitment` → `ReaCommitmentCommitted` → `rea_projection`. **No new route.**

### Entity: ReaderKeyAttestation (the X25519 substrate — the held decision, now answered)
- **Classification**: **Notarized (A), existing kind** — a content-typed attestation on the elohim
  DNA (`attestation:reader-key`, the consolidating home that already carries
  `attestation:identity-credential`). Body: `{ agent_cid, x25519_pk, valid_from, valid_until?,
  superseded_by? }`, **signed by the agent's ed25519 key inside the zome (`sign()`)** — a genuinely
  cross-signed binding, unlike today's self-asserted transport bindings.
- **Why dual-key, not conversion**: the ed25519 secret lives in lair; elohim-storage cannot derive
  an X25519 secret from it. A storage-minted X25519 pair, bound by a conductor-signed attestation,
  is buildable now. Conversion (confidentiality backlog #5's other fork) is closed by this decision.
- **Address**: agent-scoped composite `(agent_cid, "reader-key")`; rotation = new attestation +
  `superseded_by`. **Head-plane**: one per agent (+ per rotation); ~tens at seed. **Stakes**:
  floor-protected (`Constitutional` — a binding that lets a reader open a household's DEK never
  cheapens). **DNA-hash class**: **NEUTRAL** if the attestation subtype is data (as
  `identity-credential` is); verify against `content_store_integrity` before the first commit — if
  subtype validation is integrity-side, it is MOVING and must ride C5's deploy ceremony.
- **Coordinator**: existing `create_attestation`-family fn → EntryHash. **Projection**: SQLite
  `attestations` (`dht_anchor_hash: yes`); Automerge: no. **Route**: read-only
  `GET /api/v1/identity/reader-key/{agent_cid}` declared in `build_manifest()`, `{agent_cid}` is the
  composite's agent half.
- **Framing check**: key location is a mechanical fact; holding your own reader key is not an apex
  tier. Wards/guardians: a guardian's reader key is simply another entry in the key ring (§5) —
  mediated agency composes without a new entity.

### Entity: KeyRing (sealed DEKs — travels with the replica)
- **Classification**: **Attested-Private (B2) as bytes** — a tiny CID-addressed blob
  (`crypto_box_seal` output is 80 B per reader; a household of 5 readers + 3 recovery shares ≈ 700 B)
  holding `[{ reader: agent_cid, role: reader | recovery-share, sealed: bytes }]`. It is ciphertext,
  so **blind custodians hold it exactly like any shard** — which is the decision that makes "my node
  is dark, Jessica can still read" true: the envelopes are wherever the ciphertext is, not on the
  author's (dark) source chain. The 2026-08-09 plan's "private source-chain entry delivered over the
  reach path" is superseded for this reason.
- **Address**: content-derived CID over the sealed bytes. **Transport**: `auto`.
- **Where it is referenced**: the private EPR's dag-cbor body (`encryption: { scheme:
  "xsalsa20poly1305+rs", ciphertext_cid, key_ring_cid }`). EPR content is opaque to the DNA →
  **no integrity change, no manifest field-add.** This *retires* the weave-arc Wave C "ShardManifest
  field-add" — the manifest is derived (above) and carries no encryption metadata at all.
- **Attested how**: the private EPR head is the attestation that a ring exists; readers discover
  their envelope by `agent_cid` inside it. **Recovery**: `recovery-share` entries are Shamir shares
  of the DEK sealed to quorum members' reader keys; delivery authorization is the existing
  `attestation:recovery-approval` + `shamir_transport` protocol. A quorum that can reconstruct a DEK
  is a reader-set by construction — this is declared by the household at ring time, not hidden (§9).

### Entity: QuarantineVerdict / local quarantine set
- **Classification**: the verdict is **Notarized (A), existing kind** — a Mishpat attestation
  naming a manifest CID with `{ scope: discovery | readability | propagation, expiry, reason-visible-at }`
  (the witnessed-harm consensus's coercive-action envelope). The local set is **Ephemeral (C)**,
  rebuilt from attestations. Design here is *only the actuator hooks* (§6); the verdict's issuing
  role and lists are council-gated and not designed in this spec.

### Design constraints discovered
- **Encrypt-then-shard is a type, not a convention.** For private reach, `distribute_shards` and the
  swarm accept a `Ciphertext(Vec<u8>)` newtype minted only by `private_replica::encrypt`; the plain
  `&[u8]` entry stays for public reach. The "encryption-ordering landmine" the blind-custody plan cites
  at `p2p/mod.rs:1492` is closed by the compiler, not by review.
- **Dedup is lost under encryption** (same photo in two households = two ciphertexts). Accepted.
- **Source chains stay node-local**; "whole household" = blobs + Automerge docs + projections.
- **Metadata is visible to a blind custodian** (manifest size, encoding, bitfield, household). The
  traffic-analysis floor is named, not solved; PSI-style discovery is prior art, not a row.
- **Erasure cannot be guaranteed once replicated.** User-facing text must say so (red-team spec §4.1).

## 5. Blind custody, composed — the Matthew ↔ Adam walk

1. Matthew's household authors the love map (private reach). The authoring edge holds the only
   plaintext: `encrypt` (DEK) → `Ciphertext` → `ShardEncoder` (rs-4-7) → ciphertext CID; key ring
   sealed to {Matthew, Jessica} as readers and {3 recovery members} as shares; private EPR body
   names both CIDs.
2. Placement (`PlacementStrategy`, C3 social input) picks custodians from *outside* Matthew's reach
   scope — Adam's household first under the **reciprocal-custody preference** (Adam's commitments
   already name Matthew as custodian, or a pledge pairs them). `classify_custody_authorization`
   returns `MayStoreOnly` for Adam (fail-closed on DB error). Commitments: one per (manifest ×
   custodian), `readable: false`.
3. Custody push delivers ciphertext shards + the key-ring blob to Adam over the ordinary shard
   path. Adam's inventory advertises the composite with its bitfield; Adam can serve every shard to
   the swarm; Adam's storage reads `encoding`, size, and household — never bytes that decrypt.
4. Matthew's node goes dark. Jessica's device (reader key attested) resolves the private EPR,
   swarm-fetches 4 of 7 from Adam + two other custodians, fetches the key ring, unseals her
   envelope, decrypts. No doorway, no Matthew.
5. Symmetry: Adam's private footprint rides Matthew's disk the same way. Neither side's reach
   scope changed; the custody gate is the only new verdict on the hot path. The swarm curve applies
   unchanged — ciphertext shards are shards.

## 6. The witnessed-harm limit — inherited at birth, not bolted on

This spec does not re-derive the policy; it inherits the working consensus (red-team spec §4.1,
rights-floor plan, witnessed-harm survey §5, council positions 2026-08-09) and provides exactly the
hooks that consensus needs from the dataplane. Position remains **council-TBD**; the council
re-convenes on the concrete decision with then-current evidence, and dated positions are not
standing votes.

**What the consensus settles for this design:**

- **Duty attaches on knowledge; custodians are never inspection agents.** A blind custodian is never
  asked, required, or able to unseal a replica to discover whether a duty exists. This spec's blind
  path makes that structural: custodians hold ciphertext; the only plaintext edge is the author's.
- **The human is the rights-bearing subject; no artifact has an inalienable claim to discovery,
  replication or decryption.** A prohibited payload may have those capabilities narrowed to zero
  while the human floor (identity lineage, counsel, help, challenge, recovery, permitted private
  access) stays intact.
- **Known-item matching (CSAM) only at an already-plaintext edge**, council-authorized, with attested
  list provenance and a structural scope boundary (no silent list growth). CSAM is **refusal to
  instantiate** — never a custody assignment, never a lens verdict.
- **Agents see verdicts, not content**; compelled sight is quarantined with no memory formation.

**The hooks this dataplane provides (designed here, council-gated to *use*):**

| Hook | Where | Effect when a manifest CID is quarantined |
|---|---|---|
| Discovery | `inventory_broadcaster::current_hashes` / hint fold | the composite and its shards are not advertised (C4 honest absence: silence, not a lie) |
| Propagation | `fetch_shards_via_swarm`, `push_shard`, `salvage_pass` | not fetched, not pushed, not re-placed; **existing custody bytes are not destroyed** (legal hold is a separate witnessed decision) |
| Readability | key-ring resolution at the reader edge | readers' envelopes are not unsealed while `readability` is in scope; recovery shares untouched |
| Authoring edge | `private_replica::encrypt` entry (the one plaintext point) | the seam where a council-authorized known-item matcher *would* sit — a named seam with a `Verdict` type and **no matcher in this spec** |

Every actuation carries the consensus envelope: provenance, scope, expiry/review time,
reason-visible-at, and an un-firable counsel path. None of it is reachable from a per-holon ceiling;
it sits at the floor (red-team spec reading rule). Adversarial acceptance stories (Codex request #2)
belong with the rows in §8, not after them.

## 7. Transport — one race, two planes; iroh-blobs deferred on evidence

The race is transport-agnostic by construction (`cmd_tx` + `connected`). The iroh leg is **the same
race over the mounted `IrohShardProtocol`** (roadmap T2/T3), with candidates carrying iroh NodeIds
resolved from `peer_transport_manifest`. iroh-blobs' native multi-provider downloader needs a
BLAKE3 address per shard — a second hash namespace alongside the sha256 CID (`peer_blob_inventory.blake3_hash`
is the already-reserved slot). That is a measured decision: build T2, run S3 on `dual`, and adopt the
native downloader only if the curve on iroh is bandwidth-bound by our race rather than by the link.

## 8. Decomposition — rows, tiers, disjointness

| Row | What | Tier | Tree / write-set | State |
|---|---|---|---|---|
| F3 parity-aware completion | `Reconstructible` outcome | — | `blob_swarm.rs`, `http.rs`, `metrics.rs` | **in flight, uncommitted** |
| data-shard-first placement | `ShardRole`, `plan_shard_placement_slots` | — | `sharding.rs`, `p2p/mod.rs` | **in flight, uncommitted** |
| S1′ bitfield hint | `BlobHint.shards_held/encoding`; `shard_bitfield` column; broadcaster fold; `per_shard_hosts` from bitfields | Sonnet (wire + migration), Codex-claimable once the wire shape is frozen here | `inventory_gossip.rs`, `inventory_broadcaster.rs`, `db/peer_blob_inventory.rs`, one migration, `blob_swarm.rs:148-158` | supersedes backlog `2026-08-23-shard-level-inventory-gossip` (re-pointed) |
| S5 band collapse | retire `chunked` for writes; `rs-8-12` band; reads unchanged | Sonnet | `sharding.rs` (+ `http.rs` PUT band log) | new; parameter-bearing |
| S6 scarce-first | ascending-holder-count ordering in `plan_shard_holders` | Codex | `blob_swarm.rs` (pure fn + tests) | new |
| S3 curve measure | a2o: one RS blob, 1/2/3 holders, wall-clock falls; `@concern:blob-durability`; household lane | Sonnet | `genesis/a2o/features/resilience/swarm-curve.feature` + steps | roadmap S3 |
| C6-a reader key | `attestation:reader-key` + conductor-signed binding + storage keystore + `GET` route | Opus | elohim DNA coordinator (verify NEUTRAL), `elohim-storage` identity | new; closes confidentiality #5 |
| C6-b key ring + private path | `Ciphertext` newtype; key-ring blob; private EPR `encryption` body; reader-edge unseal | Opus | `private_replica.rs` → production module, `p2p/mod.rs` distribute, EPR codec consumer | new; closes confidentiality #6 |
| C6-c custody gate | `classify_custody_authorization` fail-closed; `readable:false` commitments; reciprocal preference in placement | Opus design, Sonnet impl | `p2p/reach_authorization.rs` sibling, `reconcile/placement.rs`, `peer_selection.rs` | new; depends on confidentiality #1 |
| C6-d quarantine hooks | the four actuator checks in §6 reading a local set rebuilt from attestations | Sonnet | broadcaster, swarm, push, salvage, reader edge | new; council-gated to *issue*, not to *wire* |
| T2 iroh race | same race over `IrohShardProtocol` | Opus | `p2p_iroh/`, `blob_swarm.rs` candidates | roadmap T2 |

Disjointness: S1′ and S6 both touch `blob_swarm.rs` — S6 is a pure fn at the top of the file, S1′ the
`per_shard_hosts` derivation; sequence S6 → S1′ or give them one claimant. C6-b and the in-flight
placement edit both touch `p2p/mod.rs::distribute_shards`; C6-b waits for that commit.

**Sequencing.** (in-flight lands) → S6 → S1′ → S5 → S3 (first curve number on the 3-peer mesh) →
C6-a ∥ C6-c → C6-b → C6-d → T2 → re-measure S3 on `dual`. Prove every step on the local mesh; the
fleet confirms.

## 9. Verification and habit linkage

- **Habit:** `blob-durability` (existing; the register is full at 12, so the curve is a new *check*
  under it, not a new habit): add `@concern:blob-durability` scenarios `swarm-curve` (S3) and
  `blind-custody-reads-through-a-stranger` (Jessica reads with Matthew dark; Adam cannot). Both on
  the household lane, 3 peers, no `@requires:shem`.
- **Metrics:** existing `blob_swarm_shard_fetched{result}`, `blob_swarm_composite_completed`,
  `distinct_source_peers`; add a composite-elapsed histogram labelled by holder count so the curve
  is a Grafana line, not a log grep.
- **Adversarial acceptance (Codex request #2):** a quarantined manifest is not advertised, fetched,
  pushed or re-placed, *and* the household's identity, counsel route and recovery request still work.
- **No habit flip from this spec.** Design moves nothing green; the first S3 run does.

## 10. Decisions left to the operator (three)

1. **Band parameters** — `rs-4-7` from 16 MiB and `rs-8-12` above 256 MiB are proposals; S3 sets them.
2. **Quorum-can-read** — a recovery quorum that can reconstruct a DEK is a reader-set. This spec makes
   it *declared* (the ring lists them); whether the default ring includes recovery shares at all is
   the household's choice surfaced at authoring, and the council's question for defaults.
3. **Reader-key home** — `attestation:reader-key` on the elohim DNA (recommended, DNA-hash-neutral if
   subtype is data) versus an imagodei profile field (DNA-hash-moving). The first commit verifies the
   class before choosing.

## 11. Missing nodes (story-graph maintainer)

- **chain** swarm-curve / **between** "holder advertises composite" → "requester races shard N from a
  shard-only holder" / **missing node** "inventory names shard indices at composite cost" / **state**
  designed (§3.1), unbuilt (S1′).
- **chain** blind-custody / **between** "custodian holds sealed envelopes" → "reader decrypts with the
  author dark" / **missing node** "key ring travels with the ciphertext" / **state** designed (§4),
  unbuilt (C6-b).
- **chain** witnessed-harm / **between** "verdict notarized" → "payload stops propagating" / **missing
  node** "four actuator hooks read one local quarantine set" / **state** designed (§6), unbuilt (C6-d),
  issuing role council-TBD.
