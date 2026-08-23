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
graduation-trigger: decompose-complete (every row in §9 landed or superseded) OR superseded-by-implementation
domain: peer-hoster dataplane (T2) × confidentiality plane (3.13) × custody/REA × doorway projection (T4, caches only)
habits: [blob-durability, dataplane-convergence, reach-enforced-everywhere]
topic: [sharding, reed-solomon, swarm, inventory-gossip, bitfield, blind-custody, key-ring, x25519, custody-commitment, witnessed-harm-limit, quarantine, iroh, libp2p, red-teamed]
cites:
  - "doorway-federated-continuity-roadmap | Lanes S and C6 this spec is the design pass for; grounding corrections and the S0 prerequisite rows flow back to it | sha256:4c661dbbb6927763 | path: genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md"
  - "private-layer-blind-custody-resiliency-floor | the gate output this spec inherits and explicitly changes (§7): ring travels with the ciphertext, no Shamir in the ring, dual-key via key-stewardship reuse; bond-decay deferred unchanged | sha256:1dd9950a41c2ff73 | path: genesis/docs/superpowers/plans/2026-08-09-private-layer-blind-custody-resiliency-floor.md"
  - "ownership-custody-inalienable-red-team-design | floor rows designed against — CSAM refusal-to-instantiate (quarantine vs refuse), guardian-excluded composition (§4.2), erasure and revocation limits stated plainly | sha256:d80fea9b7bf8843f | path: genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md"
  - genesis/research/witnessed-harm-limit-research-2026-08-09.md
  - "elohim-seam-map-concern-routing | routes confidentiality to 3.13 (encryption ≠ permission) — the warrant for a custody verdict distinct from reach and the serve-path answer | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "weave-epic-arc-design | Wave C HELD items resolved: ShardManifest field-add retired (manifest is derived), KeyEnvelope becomes the bound signed ring with epoch supersession | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md"
  - "stewardship-over-sovereignty | the no-absolute-lockout rule that makes the floor envelope mandatory and keeps cryptography an accelerator of recovery, never its gate | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - genesis/data/timeline/backlog/arch-confidentiality-plane-backlog.md
  - elohim/elohim-storage/src/p2p/blob_swarm.rs
  - elohim/elohim-storage/src/sharding.rs
  - elohim/elohim-storage/src/p2p/inventory_gossip.rs
  - elohim/elohim-storage/src/p2p/inventory_broadcaster.rs
  - elohim/elohim-storage/src/services/private_replica.rs
  - elohim/elohim-storage/src/services/sealed_against_self.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/shard_service.rs
  - elohim/sdk/schemas/v1/attestation/subtypes/key-stewardship-metadata.schema.json
  - elohim/sdk/schemas/v1/attestation/subtypes/gate-decision-metadata.schema.json
memory_anchors:
  - feedback_reach_head_replication_distinct_planes
  - project_inventory_exchange_not_byte_replication
  - feedback-cleanup-toward-p2p-dataplane-trajectory
  - feedback_local_mesh_first_cadence
  - feedback-identity-sovereignty-ontology-guard
  - project_mishpat_commitment_cid_is_entry_hash
---

# The swarm curve and blind custody — one dataplane pattern

> **Why one spec.** The operator's two 2026-08-23 asks — *shards sync faster as more shards are
> replicated* (torrent-style aggregate bandwidth) and *Adam replicates Matthew's whole household,
> love map included, and can read none of it* — are one mechanism seen from two sides. Ciphertext
> shards are shards. If the public swarm is designed right the blind replica inherits it; if the
> blind replica is designed right it becomes the continuous load that makes the swarm curve
> measurable instead of deploy-bursty. This spec fixes the pattern, answers the P2P design gate for
> every entity either side touches, inherits the witnessed-harm working consensus at birth, and
> decomposes into rows the roadmap's Lanes S and C6 claim.
>
> **Red-teamed 2026-08-23** by three independent lenses (dataplane-vs-tree, confidentiality/adversary,
> canon/gate). 36 findings; every one is dispositioned in §12. The first draft's grounding was wrong
> in six places and its blind-custody half was wrong in four load-bearing ones — the corrected
> design below is the one to build. Nothing in §11 is left for the operator.

## 1. Grounded state (tree, 2026-08-23 — including uncommitted in-flight edits)

- **Encoder.** `sharding.rs` bands: `none` (≤16 MiB, one shard), `chunked` (16–64 MiB, 1 MiB
  sequential, no parity), `rs-4-7` (>64 MiB). `rs-8-12` exists only in the DHT docstring;
  `determine_encoding` cannot return it and `create_shards` ignores the encoding string (reads
  `ShardConfig` directly). The RS band only started working through `PUT /blob` today (`8854f6de5`).
  **Shard hashes are deterministic under an agreed `ShardConfig`; the manifest as a whole is not**
  — `created_at` is `Utc::now()`, `mime_type`/`reach` are caller inputs and the four live call sites
  disagree (`http.rs:2668` request mime + `"commons"`; `db/shard_manifests.rs:69` content_format +
  row reach; `reconcile/custody.rs:811` and `p2p/mod.rs:1967` `"application/octet-stream"` +
  `"commons"`); `private_replica.rs:97-102` uses a *different* config (`rs_threshold: 0`). The
  300 s `manifest_backfill_pass` re-derives every blob in the store as `reach: "commons"`.
- **Shards are ordinary blobs; the composite is not.** `PUT` stores every shard under its own
  `sha256-` address; the RS/chunked composite is **never on disk under its own name**
  (`blob_swarm.rs:26-29`, `p2p/mod.rs:7962`). The inventory snapshot is a full replace over
  `blob_store.list_hashes()` (`db/peer_blob_inventory.rs:116`), so a composite address survives only
  until the next snapshot tick. Paging landed (`INVENTORY_GOSSIP_PAYLOAD_BUDGET = 3_500` B,
  `inventory_broadcaster.rs:39`, sized under iroh-gossip's 4 KiB cap); under budget pressure the
  broadcaster drops **hints first** and keeps addresses.
- **Swarm.** `race_fetch_with_swarm` → `fetch_shards_via_swarm`: per-shard rotated holder lists
  (`plan_shard_holders`, per-shard `lookup_hosts` already runs), bounded by `total_inflight`; each
  landed shard persists with its own serve-blob REA credit; a manifest-only holder answers
  `FetchOutcome::Manifest`. **A landed shard is servable by its own hash before the composite
  exists** (`shard_service.rs:69-80`). **But**: the event-driven `BlobInventoryDelta` at
  `p2p/mod.rs:8036-8078` fires once per *completed composite*, never per shard; a peer-supplied
  manifest is persisted (`record_generated_manifest`) before any shard lands and **the reassembled
  composite is never re-hashed against `blob_hash`** (each shard verifies only against itself);
  a pushed shard (`shard_service.rs:89-105`, `http.rs:2496`) is stored with **no manifest
  association and no authorization** — hash equality is the only gate, on both transports.
- **In flight, uncommitted in the shared worktree (another session, 2026-08-23):**
  `reconstructible_threshold` / `should_start_shard_race` / `SwarmRaceOutcome::Reconstructible`
  (pure, correct as written) and `ShardRole` / `plan_shard_placement_slots` (data shards take the
  diverse holder prefix). This spec treats them as landing.
- **DHT.** `content_store_integrity` defines `ShardManifest` and `ShardLocation`; elohim-storage
  never writes them (no caller of `register_shard_manifest`). `shard_locations` is SQLite-only.
  Custody commitments are per marker, never per shard (`salvage_commitment_author.rs:147`).
  Custody reconcile's presence check is `list_hashes().contains(composite)` — so every RS/chunked
  custody commitment reads *missing* on every sweep, forever (`reconcile/custody.rs:84-86,243`).
- **Gossip authentication is Stage 1.** `verify_structural` accepts any non-empty signature;
  `handle_inventory` applies a snapshot under the **body-claimed** `peer_id` (`gossip_dispatch.rs:327-352`);
  `custody_announce` publishes `{shard_hash, holder_agent_cid}` on a public topic.
- **Blind custody.** `services/private_replica.rs` proves encrypt → RS → sealed-DEK → reconstruct-
  then-decrypt on one host (dryoc `crypto_box_seal` of a bare 32-byte DEK — anonymous, bound to
  nothing). `sealed_against_self.rs` ships a nested 2-of-2 sealed box (mishpat quorum ⊗ subject
  imagodei). Shamir delivery exists and is gated *per release* by an on-DHT
  `attestation:recovery-approval` that the custodian verifies (`shamir_transport.rs`). There is no
  X25519 reader key bound to an `agent_cid`; `attestation:key-stewardship` exists ("a device-key
  stewarded for a human's identity", metadata `device_id`/`stewardship_class`). Attestation
  subtypes are a **closed, generated list in the integrity zome** (`ATTESTATION_KINDS`, floor 1) —
  adding one moves the DNA hash; the integrity zome does not validate metadata shape.
- **Plaintext at rest, beyond the shard path.** `PUT /blob` dual-writes the whole composite into
  the iroh `FsStore` (`http.rs:2809`), served under `iroh_blobs::ALPN` to any dialer. The doorway
  blob pantry (`routes/storage_proxy.rs:905-917`) caches any 200 body and re-serves it with **no
  reach or auth re-check** (the freshness pantry next door *does* gate, `freshness.rs:243`);
  `AppFileCacheService` persists plaintext to MongoDB. `classify_custody_authorization` has zero
  hits; the sibling `reach_authorization.rs` fails **open** on pool error at Stage 1.
- **Witnessed-harm limit.** Working consensus across the red-team spec (CSAM = refusal to
  instantiate, floor, fail-closed), the rights-floor plan, the 2026-08-09 survey and two recorded
  council positions. Position council-TBD; the architectural shape is settled enough to design against.

## 2. The pattern — one invariant chain

```
author edge ─(private reach: encrypt → Ciphertext; the ONLY plaintext point)─▶ bytes
bytes ─(ShardEncoder under the config RECORDED in the manifest)─▶ manifest        DERIVED (C); verified on reassembly
manifest + shards ─(custody-blob commitments, one per manifest × custodian)─▶ replica set   the ONLY notarized objects
holders ─(inventory: composite address + shard bitfield, substituted for shard files)─▶ peer_blob_inventory (C)
requester ─(fastest-k-of-n: data-first then scarce-first, rotated holders, per-shard credit)─▶ shards
each landed shard ─(per-shard delta, bit set)─▶ the requester is a source before it is complete
reassembly ─(sha256 == blob_hash, else refuse + demote the manifest's source)─▶ composite
```

Six rules, each load-bearing:

1. **The manifest is derived, never notarized — and derivation is verified, not trusted.** Shard
   hashes are a pure function of (bytes, encoder parameters); the manifest carries those
   parameters (`encoder: { kind, data, parity, chunk }`), so any peer can re-derive and check.
   A peer-supplied manifest is held **unverified** until the reassembled composite hashes to
   `blob_hash`; only then is it recorded. `mime_type` and `reach` are **not manifest semantics** —
   reach lives on the EPR; the manifest fields stay for wire compatibility and are never read for a
   verdict. Zero DHT writes for the public swarm; lighting `ShardManifest`/`ShardLocation` would add
   1–8 heads per blob for a fact the bytes prove.
2. **Custody commitments are the only notarized objects** — one per (manifest × custodian),
   composite root. Presence is judged manifest-aware (≥ `data` shards local ⇒ held).
3. **Inventory names shard indices at composite cost.** The broadcaster *substitutes* one composite
   entry carrying a bitfield for the shard files it folds; the entry is not droppable under budget.
4. **Completion is k-of-n, data-first then scarce-first, serve-while-incomplete.** Done at
   `data_shards` landed; races ordered data-before-parity, ascending holder count within class; a
   landed shard emits a per-shard delta with its bit set.
5. **Encryption precedes every byte write, enforced by type at the store.** `BlobStore::store`
   and the iroh `add_bytes` accept `StorableBytes::{Public(Vec<u8>), Ciphertext(Ciphertext)}`;
   private-reach bytes can only be constructed as `Ciphertext` by `private_replica::encrypt`. The
   newtype at `distribute_shards` alone closes 1 of ~15 write sites — the store is the choke point.
6. **Every byte path is a propagation plane the quarantine actuator must reach** — inventory
   (snapshot *and* retraction delta), Kad provider records, custody announce, both blob stores,
   both doorway caches.

## 3. The curve — what makes it superlinear, and what flattens it

**Model.** `k` data shards of `n`, `h` holders covering disjoint shards, uplink `u`: one requester's
throughput ≈ `min(down, min(h,k)·u)` — 4× single-source at `h ≥ 4` for `rs-4-7`, *and* the fastest
4 of 7. Network-wide, a peer that lands one shard is a source for it on its next delta, so full
replication across N peers takes ~log₂(N) rounds. RS beats torrent here: useful after one shard,
any k suffice; rarest-first is a placement concern (in flight), not an availability one.
**Today the curve is cadence-bound**, not bandwidth-bound: no per-shard delta exists, so a new
holder becomes visible only at the next ~60 s snapshot — and the composite entry it would need is
erased by that snapshot. Flatteners and cures:

| # | Flattener (tree today) | Cure | Row |
|---|---|---|---|
| F0 | A peer-supplied manifest is trusted: attacker answers the manifest race with `{blob_hash: H, shard_hashes: [theirs]}`, shards self-verify, requester reassembles attacker bytes, persists, books credit, re-advertises H | Re-hash the reassembled composite; refuse on mismatch; persist the manifest only after; demote the manifest's source | **S0-a** (prereq) |
| F0′ | A pushed shard has no manifest association — the exact peers the curve needs (partial holders) cannot fold | Push wire carries `manifest_cid` + `shard_index` (additive `#[serde(default)]`); receiver persists a `shard_locations` membership row; pushes also pass the custody verdict (§5) | **S0-b** (prereq) |
| F0″ | Custody presence is composite-blind → every RS commitment re-kicks forever | `has(hash) ∨ (manifest resolves ∧ ≥ data shards local)`, reusing `read_blob_bytes_for_manifest`'s floor | **S0-c** (prereq) |
| F1 | Inventory is N flat addresses per blob (64 for a 64 MiB `chunked`); composite never in a snapshot; hints dropped first under the 3.5 KB page budget | **Composite substitution + bitfield** (§3.1): one non-droppable entry replaces N; snapshot set stable; dedup fingerprint covers the bitfield; bit-OR merge, never `replace_into` | S1′ |
| F2 | `chunked` has no parity — every piece required, no fastest-k; and most content lands there | **Band collapse** (§10.1): `none ≤ 16 MiB`, `rs-4-7 (16 MiB, 256 MiB]`, `rs-8-12 > 256 MiB`; reads keep `chunked`; `create_shards` parses the encoding; single encode; streaming RS above 64 MiB | S5 |
| F3 | All-or-nothing completion | **in flight** (`Reconstructible`) | landing |
| F4 | No per-shard delta → cadence-bound | Per-shard delta in the `FetchOutcome::Hit` arm of `fetch_shards_via_swarm` with the bit set — **this, not the bitfield, is the superlinear enabler** | S4 |
| F5 | Rotation only; a scarce shard can wait behind abundant ones | Scarce-first *within role class, data before parity* (`shard_role` is pure); admission budget is exactly k, so class order matters | S6 (after ShardRole lands) |

Not flatteners, not to be built: a tracker, a DHT shard-location table, per-shard commitments, a
second hash namespace on the libp2p path.

### 3.1 The inventory change (one wire change, one projection change)

```rust
// p2p/inventory_gossip.rs — BlobHint, additive; old peers ignore, new peers require for folded entries
pub shards_held: Option<Vec<u8>>,   // LE bit i = shard index i of the manifest at `address`
pub encoding:    Option<String>,    // "rs-4-7" | "rs-8-12" | "chunked" | "none"
```

- **Fold = substitution.** A `LocalInventory` layer over `list_hashes()` maps shard files to their
  manifest (via the membership rows S0-b persists, and `shard_manifests` for locally sharded blobs)
  and emits **one composite entry** in their place; a holder of the composite bytes sets all bits.
  Shard addresses are no longer emitted once the fold is in; the snapshot set is therefore stable
  across shard landings and the composite entry survives every tick.
- **Non-droppable.** A folded entry loses N facts if stripped; `build_bounded_inventory_publications`
  must page it, never drop its hint (today's ordering is inverted for this case).
- **Wire address is legacy-shaped during the transition.** The hint rides the `sha256-` composite
  address the gossip plane speaks today; CIDv1 on the inventory wire is the named downstream
  migration, not extended here.
- **Projection.** `peer_blob_inventory.shard_bitfield BLOB NULL` (source of truth: local /
  operational; rebuilt from the next snapshot). Writes: `merge_shard_bitfield` bit-ORs via `UPDATE`;
  the three `replace_into` writers never touch it; `content_fingerprint` covers the bitfield so a
  bitfield-only change defeats the dedup fast-path. Rows from a bitfield claim carry
  `source = 'gossip-bitfield'` and **custody/salvage exclude that tier from honored-replica counts**
  (six consumers read `lookup_hosts` as truth; a partial holder must not count as a replica).
- **Gate on authentication.** The hint extends attacker-controllable fields into the race under a
  victim's identity (body-claimed `peer_id`, structural signature). S1′ does not ship before the
  minimum fix — bind the applied `peer_id` to the propagation source — and the named cure for
  bit-lying (`advertiser_health` demotion) waits for Stage-2 ed25519 signatures, or it punishes the
  victim of a forgery.

## 4. P2P Design Gate output

Concern-canon states use the registry's four values. Classes not listed for an entity are `n-a`
with the reason "no decision predicate, no message, no route is born on this entity" — the
predicate-bearing entities carry the full table.

### Entity: ShardManifest (derived)
- **Classification**: **Ephemeral (C)**. Reconstruction: re-run the encoder under the recorded
  parameters, or accept a peer's manifest and verify on reassembly. Lookup identity = the blob it
  describes (`bafkrei…`, raw — the `BlobToManifest` relation); the manifest's own content address,
  where one is needed, is dag-cbor (`bafyrei…`). `blob_hash` `sha256-` is legacy wire.
- **Head-plane**: zero. **Stakes**: all four; verification on reassembly is floor-protected
  (a Simulacra peer reassembling attacker bytes is still a poisoned store). **Transport**: `auto`.
- **Field changes** (SQLite + wire, no DHT): `encoder: { kind, data, parity, chunk }` recorded;
  `reach`/`mime_type` deprecated for verdicts (never read), kept for wire compatibility.

### Entity: Inventory bitfield hint / `peer_blob_inventory.shard_bitfield`
- **Classification**: **Ephemeral (C)** wire + projection. **Stakes**: stage-invariant claim; the
  shard fetch verifies it by hash.
- **Concern canon (extended `plan_shard_holders` + fold):** C0 answered (T2 dataplane) · C1 n-a
  (no election) · C2 n-a · C3 answered (a claim never blocks; a miss falls through) · C4 **partial**
  — which indices held: answered; held-but-unreadable is *not* in the hint and is answered by the
  custodian's own `readable:false` commitment projection, not by gossip · C5 **partial** — the
  claim is evidence only after the hash-verified fetch; until Stage-2 signatures, the *source* of the
  claim is unauthenticated · C6a answered (bounded by manifest length, one fold per tick) · C6b
  answered (bit-OR is idempotent) · C7 **partial** (advertise/serve symmetry is checked only by the
  race miss; demotion waits for Stage 2) · C8 answered (`blob_swarm_shard_fetched{result}`,
  `distinct_source_peers`, composite-elapsed histogram by holder count) · C9 n-a (no identity
  carried beyond peer_id, which S1′'s gate binds) · C10 answered (additive, `serde(default)`, old
  peers ignore) · C11 answered (page budget is the externally imposed bound; the folded entry is
  paged, not dropped) · C12 n-a · C13 n-a · C14 n-a. Registered in `elohim-storage/seam-registry.yaml`
  with the landing commit.

### Entity: Swarm completion + reassembly verification (`reconstructible_threshold` in flight; `verify_composite` new)
- Pure predicates. C4 answered (`Reconstructible { missing_parity }` is an honest partial) · C5
  **answered by S0-a** (reassembled bytes are evidence only after `sha256 == blob_hash`) · C6a/C6b
  answered · C7 answered (a manifest whose reassembly fails demotes its source and is not
  re-advertised) · C8 answered (`composite_refused_hash_mismatch` counter). Rest n-a.

### Entity: BlobCustodyCommitment (public and blind)
- **Classification**: **Notarized (A), existing kind** — `rea_commitments action='custody-blob'`;
  blind adds `resource_classified_as: "content:private-encrypted"` + `readable: false`.
  **Address**: agent-scoped composite `(custodian agent_cid, manifest CID, "custody-blob")`.
  **Head-plane**: composite root, one per (manifest × custodian); seed O(10), 1 yr low hundreds.
  **Stakes**: placement stage-priceable; **confidentiality floor-protected**.
  **Coordinator**: `content_store::create_rea_commitment` → `ReaCommitmentCommitted` →
  `rea_projection`. **No new route.**
- **Precondition restored from the 2026-08-09 plan**: the **capacity pledge** is a household-set
  ceiling and the floor degrades to zero on a withdrawing device; the reciprocal-custody preference
  in placement (§5) is bounded by it — no pledge, no reciprocal slot.
- **Concern canon:** C0 answered · C2 answered (a commitment never loses authority silently —
  release is a re-settlement event) · C3 answered (salvage) · C4 **answered** (a `readable:false`
  commitment projects as *held-but-unreadable* in the custodian's resilience view — the plan's
  unbound C4 closes on the commitment, not on gossip) · C5 answered (verdict from notarized
  commitments, never self-claim) · C6a/C6b answered (salvage cooldown; idempotent reconcile) ·
  C11 answered (pledge ceiling) · C12 **partial** (custodian consent = pledge; reader-set consent
  surface = the ring, §4 KeyRing — author-declared, not yet a consent *protocol*) · C13 **partial**
  (may-store/may-read is the first graduation; wards/guardians are a named non-goal, §5) ·
  C14 **unbound** (what a released custodian may retain and for how long is undefined — the
  quarantine "hold" line makes this *more* urgent; carried to §13 as a missing node). Others n-a.

### Entity: ReaderKey (X25519 key bound to an `agent_cid`) — the held decision, answered without a DNA move
- **Classification**: **Attested-Private (B2)**. The key pair is minted and held by the human's
  storage node (private, B); its public half is notarized by **reusing `attestation:key-stewardship`**
  ("a device-key stewarded for a human's identity") with metadata gaining an optional
  `reader_pk: "x25519:<base64>"` field — the integrity zome validates subtype membership, expiry
  and `supersedes_cid` revocation (floors 1/5/7), **not metadata shape**, so this is
  **DNA-hash-NEUTRAL** (schema + storage-side validator + TS codegen move together; no WASM change).
  The first draft's `attestation:reader-key` was DNA-HASH-MOVING (closed generated list) and is dropped.
- **Authority is the signature, not DHT residency.** Floor 2 (issuer authorization) is accept-all
  today, so "it's on the DHT" proves nothing. The record `{agent_cid, x25519_pk, valid_from,
  valid_until?, supersedes?}` is **signed under the agent's ed25519 key through the existing
  conductor-signing RPC** (`sign_for_agent`, `binding_mint.rs` pattern; signer-match gate) and
  verified reader-side against `agent_cid` — which *is* the ed25519 pubkey — with canonicalized
  `agent_cid` (location suffix zeroed, as `binding_proof_wire.rs` does). Why dual-key, not
  conversion: the ed25519 secret lives in lair; storage cannot derive X25519 from it.
- **Address**: agent-scoped composite `(agent_cid, "reader-key")`; rotation = new attestation with
  `supersedes_cid`. **Head-plane**: one per human + one per rotation; seed ~10, **1 yr ≈ 2×
  households × (1 + rotations/yr ≈ 1) — low hundreds; negligible against the 3.5 k content-head
  anchor.** **Stakes**: `Constitutional`, floor-protected. **Coordinator**: existing attestation
  create fn → EntryHash. **Projections**: `attestations` (`dht_anchor_hash: yes`); Automerge: no.
  **Route**: `GET /api/v1/identity/reader-key/{agent_cid}` in `build_manifest()`.
- **C9 identity-lineage (answered, it was the silent skip):** a reader re-key does **not** orphan
  rings — any current DEK holder (the author or any reader who can unseal today) re-issues the ring
  at a new epoch; it needs the DEK, not the plaintext. If no current holder is reachable the ring is
  **fail-loud unreadable for the new key**, never silently re-sealed by a custodian. Acceptance:
  "Jessica re-keys, then reads with Matthew dark" (§10).
- **Framing**: key location is a mechanical fact; holding your own reader key is not an apex tier.
  Wards/guardians: **explicit non-goal** — guardianship has no DHT entity; ring composition MUST
  support a guardian-excluded reader set and an outside-the-holon challenge (red-team spec §4.2
  req 3–4) before any guardian key is ever placed in a ward's ring; until then a guardian is not a
  reader by default.

### Entity: KeyRing (sealed DEKs — travels with the ciphertext, bound to it)
- **Classification**: **Attested-Private (B2) as bytes** — a dag-cbor, CID-addressed
  (`bafyrei…`) record held by blind custodians like any shard. **Source of truth**: the author's
  node (private); custodians hold replicas; the private EPR head attests which ring is current.
- **Shape (corrected).** `{ epoch: u32, prev_ring_cid: Option<Cid>, ciphertext_cid, envelopes:
  [sealed…], floor: sealed, sig }` where each reader envelope seals
  `{ dek, ciphertext_cid, epoch, prev_ring_cid, author_agent_cid }` (not a bare DEK — an envelope is
  therefore **untransplantable** between rings and **rollback-detectable**), `sig` is the author's
  ed25519 over canonical ring bytes (`sign_for_agent`), and envelopes are **unlabeled**: readers
  trial-decrypt (N ≤ ~10, 80 B each), so a custodian learns the reader *count*, never the roster.
  Readers reject CID mismatch, epoch regression, bad signature.
- **Recovery floor (corrected — no Shamir in the ring).** Pre-placing Shamir shares would delete
  the per-release `attestation:recovery-approval` gate and hand a quorum silent takeover (the
  IPV case). Instead every ring carries exactly one **floor envelope**: the DEK
  sealed-against-self (`sealed_against_self.rs`, mishpat quorum ⊗ subject imagodei, true nesting).
  Opening it needs governance cooperation **and** the subject's imagodei key — which is itself
  recoverable through the existing social path (`KeyStewardship` M-of-N, `recovery-approval`,
  `shamir_transport`). That is the canon's non-cryptographic completion: no absolute lockout, no
  party that can read alone. The floor envelope is **mandatory** — a ring without it is refused at
  authoring; §10.2 of the first draft ("household's choice") is withdrawn.
- **Supersession.** Ring head is a **declared dependency** in the private EPR body
  (`encryption: { scheme, ciphertext_cid, key_ring_cid, epoch }`). Adding a reader = new ring
  epoch, same DEK. **Removing a reader = DEK rotation + re-encrypt + new `ciphertext_cid`** — the
  honest price; a removed reader keeps what they could already read, and custodians keep serving
  the old ring at its old CID forever. **Stated limit:** revocation changes what is *newly*
  readable; it recalls nothing already replicated (the same plainness the erasure limit gets).
- **DEK**: random per artifact; **convergent encryption excluded** (cross-household dedup is a
  confirmation oracle). `plaintext_cid` is **deliberately not published** — the 2026-08-09 plan's
  field is dropped for the same reason.
- **Delivery path.** The ring is fetched over the custody/reach-authorized shard path, **never the
  public `/blob` namespace** — the doorway blob pantry re-serves any 200 body ungated (S0-d).
- **Where referenced**: EPR body (opaque to the DNA) → no integrity change, no manifest field-add;
  retires the weave-arc Wave C field-add and the plan's `EncryptedShardManifest` (A2) entity.
- **Concern canon:** C0 answered (3.13 confidentiality) · C2 answered (epoch monotonic) · C4
  answered (a ring is present-or-absent; unreadable-for-me is trial-decrypt failure, loud) · C5
  answered (signature) · C9 answered (above) · C10 answered (epoch + scheme tag) · C12 **partial**
  (author-declared reader set; no reader-side consent protocol) · C13 **partial** (guardian
  non-goal) · C14 **unbound** (old rings on released custodians). Others n-a.

### Entity: QuarantineVerdict (reuses `attestation:gate-decision`) / local quarantine set
- **Classification**: **Notarized (A), existing kind** — mishpat `attestation:gate-decision` with
  `gated_subject_cid = manifest CID`, `decision_outcome = block`, `gate_kind ∈ { quarantine, refuse }`,
  `rationale`, and the floor-5 `expires_at` (required for `quarantine`, **absent for `refuse`**).
  **DNA-hash-NEUTRAL** (existing subtype; `gate_kind` is metadata). **Address**: content-derived
  over the attestation. **Head-plane**: one per verdict; seed 0, 1 yr tens. **Stakes**: floor —
  `Constitutional`, never cheapens. **Coordinator**: existing attestation create → EntryHash.
  **Projections**: `attestations` (`dht_anchor_hash: yes`) → local quarantine set (C, rebuilt from
  attestations). **Route**: none new. **Issuing role**: council-gated (§6); this spec wires the
  actuator only.
- **Two verdicts, not one (corrected).** `quarantine` = reversible, expiring, **bytes held** (hold
  is a separate witnessed decision). `refuse` = terminal, no expiry, **mandatory local destruction**
  of shards + manifest + inventory rows + cache entries on every peer that sees it — the
  refusal-to-instantiate floor, which "hold" alone contradicted.
- **Concern canon:** C0 answered (floor, beneath every holon ceiling) · C2 answered (refuse is
  terminal; quarantine expires, never silently escalates) · C4 answered (retraction, not silence —
  §6) · C5 answered · C6b answered (idempotent actuation) · C8 answered (`quarantine_actuated{plane}`)
  · C12 answered (the verdict's envelope carries consent/authority provenance) · C14 **partial**
  (what a peer retains under `quarantine` is defined — bytes, unadvertised; under `refuse` nothing;
  what a *released custodian* retains is the open C14 above). Others n-a.

### Design constraints discovered
- **Encrypt-then-shard is a type at the store**, not at `distribute_shards` (rule 5). `handle_push`
  additionally needs the custody verdict (§5) — today anyone can plant bytes on a stranger's disk.
- **The manifest's `reach` is dead for verdicts** — the 300 s backfill re-stamps everything
  `commons`; the only reach that governs is the EPR's.
- **Dedup is lost under encryption.** Accepted. **Source chains stay node-local.** "Whole household"
  = blobs + Automerge docs + projections.
- **What a blind custodian learns** (named floor, not solved): manifest size, encoding, shard
  bitfield, reader *count*, access timing; and — because `custody-blob` commitments are notarized —
  **the replica map at manifest granularity is public to anyone who can read commitments**.
  Shard-level `custody_announce` is **excluded** for private-reach manifests. Adversarial custody
  (volunteer-to-hold for traffic analysis) is bounded by the pledge ceiling and placement, not
  prevented. PSI discovery is prior art, not a row.
- **Erasure cannot be guaranteed once replicated; revocation recalls nothing.** User-facing text
  says both (red-team spec §4.1).

## 5. Blind custody, composed — the Matthew ↔ Adam walk (corrected)

1. Matthew's household authors the love map (private reach). The authoring edge holds the only
   plaintext: random DEK → `encrypt` → `Ciphertext` → `ShardEncoder` (band by size) →
   `ciphertext_cid`; ring epoch 1 sealed to {Matthew, Jessica} as unlabeled reader envelopes + the
   mandatory floor envelope; ring signed; private EPR body names `ciphertext_cid`, `key_ring_cid`,
   `epoch`. **No guardian key is placed** (non-goal until guardian-excluded composition exists).
2. Placement (`PlacementStrategy`, C3 social input) picks custodians from *outside* Matthew's reach
   scope — Adam's household first under the **reciprocal-custody preference, bounded by both
   households' pledges**. `classify_custody_authorization(scope, custodian)` returns `MayStoreOnly`
   for Adam — **fail-closed on any DB/pool error** (a deliberate divergence from Stage-1
   `reach_authorization`'s fail-open; pinned by an injected-pool-error test). Commitments: one per
   (manifest × custodian), `readable: false`.
3. Custody push delivers ciphertext shards **with `manifest_cid` + `shard_index`** and the ring
   over the shard path; Adam's `handle_push` admits them **because a commitment names him**
   (otherwise refused). Adam's inventory advertises the composite with its bitfield; Adam serves
   every shard to the swarm; Adam's node reads `encoding`, size, bitfield, reader count — never
   bytes that decrypt.
4. Matthew's node goes dark. Jessica's device (reader key attested, signature-verified) resolves
   the private EPR, swarm-fetches 4 of 7 from Adam + two other custodians, **verifies the
   reassembled ciphertext hash**, fetches the ring over the authorized path, trial-decrypts her
   envelope, checks `ciphertext_cid`/epoch, decrypts. No doorway, no Matthew.
5. Jessica re-keys next month: Matthew (or Jessica herself, holding the DEK) re-issues the ring at
   epoch 2; custodians receive it like any blob; old ring stays fetchable (stated). Symmetry:
   Adam's private footprint rides Matthew's disk the same way. Reach scopes unchanged; the custody
   gate is the only new verdict on the hot path; the swarm curve applies unchanged.

**Serve-path verdict for ciphertext (the `reach-enforced-everywhere` answer).** Custody is
reach-blind by design; the boundary is serving. For ciphertext shards the serve path enforces
**quarantine + pledge/rate**, not reach — confidentiality is the encryption's job, and a serve-time
reach check would forbid the very traffic the floor depends on. For plaintext shards the existing
reach check stands. That is one line of the red habit's `first_move`, answered here.

## 6. The witnessed-harm limit — inherited at birth, not bolted on

This spec inherits the working consensus (red-team spec §4.1, rights-floor plan, survey §5, council
positions 2026-08-09) and provides the dataplane hooks it needs. Position remains **council-TBD**;
the council re-convenes on the concrete decision with then-current evidence; dated positions are
not standing votes.

**Settled for this design:** duty attaches on knowledge — custodians are never inspection agents
and are never asked to unseal; the human is the rights-bearing subject, no artifact has an
inalienable claim to discovery/replication/decryption; known-item matching (CSAM) only at an
already-plaintext edge, council-authorized, attested list provenance, structural scope boundary;
CSAM is refusal-to-instantiate; agents see verdicts, not content.

**Actuator planes (corrected — retraction, not silence; all the planes, not four):**

| Plane | Where | `quarantine` | `refuse` |
|---|---|---|---|
| Inventory | broadcaster fold + **explicit `BlobInventoryDelta.removed` retraction** (rows at rest on peers are not cleared by silence) | retract, stop advertising | retract + purge local rows |
| Kad provider records | `kad_store.rs` (persisted, republished independently) | `stop_providing` | `stop_providing` + purge |
| Custody announce | `custody_announce.rs` public topic | retraction message | retraction |
| Swarm / push / salvage | `fetch_shards_via_swarm`, `handle_push`, `salvage_pass` | not fetched, pushed, or re-placed; **bytes held** | refused + **local destruction** of shards, manifest, membership |
| Blob stores | `BlobStore` **and** the iroh `FsStore` | not served | destroyed |
| Doorway caches | blob pantry (`storage_proxy.rs`), `AppFileCacheService` (Mongo) | purge entry | purge entry |
| Readability | reader edge | **advisory only** — a check inside the reader's own binary over bytes it holds; listed for honesty, not as an actuator | n-a |
| Authoring edge | `private_replica::encrypt` (the one plaintext point) | the seam where a council-authorized known-item matcher *would* sit — `Verdict` type named, **no matcher in this spec** | — |

Every actuation carries the consensus envelope (provenance, scope, expiry/review time for
`quarantine`, reason-visible-at, un-firable counsel path) and sits at the floor — unreachable from
any per-holon ceiling. Adversarial acceptance stories (Codex request #2) ship with the rows (§10).

## 7. Changes from the 2026-08-09 blind-custody plan (explicit)

| Plan decision | This spec | Why |
|---|---|---|
| `EncryptedShardManifest` (A2 field-add: `encryption`, `plaintext_cid`, `key_envelope_ref`) | **Retired.** Manifest is derived (C), carries no encryption metadata; the EPR body carries it | the manifest is not notarized; `plaintext_cid` is a confirmation oracle |
| KeyEnvelope as author private-chain entry + notarized "envelope exists" attestation | **Ring travels with the ciphertext** (B2 bytes, bound + signed) | must be readable with the author dark |
| Recovery = Shamir shares of the DEK to quorum | **Floor envelope** (sealed-against-self) + existing `recovery-approval`/`shamir_transport` for the imagodei key | pre-placed shares bypass the per-release gate (takeover) |
| ed25519→X25519: conversion vs dual-key undecided | **Dual-key**, notarized by reusing `attestation:key-stewardship`; authority = conductor signature | lair holds the ed25519 secret; closed subtype list |
| Bond-decay lifecycle (§3 of the plan) | **Deferred, unchanged** — not in this spec's rows; the pledge ceiling and "release cannot complete before re-placement" are carried as preconditions | the harder half; needs its own pass after C6-c |
| C4 unbound (held-but-unreadable) | **answered on the commitment projection** | gossip cannot say it; the commitment can |
| C12 partial · C13 partial · C14 unbound | **carried at the same state** | no evidence to upgrade |

## 8. Transport — one race, two planes; iroh-blobs deferred on evidence

The race is **not** transport-agnostic today: `SwarmFetchParams.cmd_tx` is the libp2p command
channel and `connected` is libp2p `PeerId`s; `IrohShardProtocol` is mounted server-side
(`main.rs:3301`) but `IrohShardClient` has zero callers. **T2 is therefore a `ShardFetcher` trait**
over `{libp2p cmd_tx, IrohShardClient}` threaded through `SwarmFetchParams`, with candidates carrying
iroh NodeIds from `peer_transport_manifest` — an order of magnitude more than "add NodeIds to the
list". iroh-blobs' native downloader needs a BLAKE3 address per shard (`peer_blob_inventory.blake3_hash`
is the reserved slot); adopt it only if S3 on `dual` shows the race, not the link, is the bound.
**Serve side is live today and must be gated first**: the iroh `FsStore` receives the plaintext
composite on every `PUT` (`http.rs:2809`) — rule 5 covers it.

## 9. Decomposition — rows, tiers, disjointness

| Row | What | Tier | Write-set | State |
|---|---|---|---|---|
| F3 parity-aware completion | `Reconstructible` outcome | — | `blob_swarm.rs`, `http.rs`, `metrics.rs` | **in flight, uncommitted** |
| ShardRole placement | `ShardRole`, `plan_shard_placement_slots` | — | `sharding.rs`, `p2p/mod.rs` | **in flight, uncommitted** |
| **S0-a** verify composite | re-hash after reconstruct; manifest persisted only after; demote source; counter | Sonnet | `blob_swarm.rs` (reassembly), `blob_fetch.rs`, `db/shard_manifests.rs` | **prereq, new** |
| **S0-b** push membership | `ShardRequest::Push { manifest_cid?, shard_index? }`; `shard_locations` row on receive; encoder params in manifest | Sonnet | `shard_protocol.rs`, `shard_service.rs`, `http.rs` PUT /shard, `sharding.rs` (params) | **prereq, new** |
| **S0-c** custody presence manifest-aware | `has ∨ (manifest ∧ ≥ data local)` | Codex | `reconcile/custody.rs` | **prereq, new** |
| **S0-d** doorway blob pantry gating | `reach_is_stockable`-equivalent on the blob pantry; never stock `Authorization`-bearing or non-public bodies | Sonnet | `doorway-service/src/routes/storage_proxy.rs` | **prereq, new — also a live bug for any private blob today** (backlog `security-doorway-blob-pantry-ungated`) |
| S1′ bitfield inventory | hint fields; substitution fold; `shard_bitfield` column; bit-OR merge; fingerprint; `gossip-bitfield` source tier; `peer_id` bound to source | Sonnet | `inventory_gossip.rs`, `inventory_broadcaster.rs`, `db/peer_blob_inventory.rs` + migration, `gossip_dispatch.rs`, `blob_swarm.rs:148-158` | after S0-a/b; backlog row re-pointed |
| S4 per-shard delta | delta with bit set in the `FetchOutcome::Hit` arm | Codex | `blob_swarm.rs`, `p2p/mod.rs` | after S1′ (needs the bitfield) |
| S5 band collapse | band table in `ShardConfig`; `determine_encoding` owned; `create_shards` parses encoding; single encode; streaming RS >64 MiB; `rs-8-12` real | Sonnet | `sharding.rs`, `http.rs` PUT | new |
| S6 scarce-first | within role class, data before parity | Codex | `blob_swarm.rs` (pure fn) | after ShardRole lands |
| S3 curve measure | a2o: one RS blob, 1/2/3 holders, wall-clock falls; composite-elapsed histogram by holder count | Sonnet | `genesis/a2o/features/resilience/swarm-curve.feature`, `metrics.rs` | after S4 |
| C6-a reader key | `key-stewardship` metadata `reader_pk`; storage X25519 keystore; `sign_for_agent` record; gossip like `identity_binding_gossip`; signer-match verify; `GET` route | Opus | schema subtypes + codegen, `elohim-storage` identity, `http.rs` manifest | new; closes confidentiality #5 |
| C6-b private path | `StorableBytes` at `BlobStore::store` + iroh `add_bytes`; bound ring shape + floor envelope + signature; EPR `encryption` body; reader-edge trial-decrypt + checks; ring re-issue on re-key | Opus | `blob_store.rs`, `p2p_iroh/blob_store.rs`, `private_replica.rs` → production, EPR codec consumer | after C6-a and ShardRole commit (touches `distribute_shards`) |
| C6-c custody gate | `classify_custody_authorization` **fail-closed** (injected-pool-error test); `handle_push` admits only commitment-named pushes; `readable:false`; reciprocal preference bounded by pledge; private manifests excluded from `custody_announce` | Opus design, Sonnet impl | `reach_authorization.rs` sibling, `shard_service.rs`, `placement.rs`, `peer_selection.rs`, `custody_announce.rs` | depends on confidentiality #1 |
| C6-d quarantine actuator | local set from `gate-decision` attestations; the §6 plane table incl. retraction delta, Kad stop-providing, announce retraction, both stores, both doorway caches; `refuse` destruction | Sonnet | broadcaster, `kad_store.rs`, `custody_announce.rs`, swarm/push/salvage, `blob_store.rs`, iroh store, doorway caches | council-gated to *issue*; wiring is not |
| T2 iroh race | `ShardFetcher` trait over `{cmd_tx, IrohShardClient}` | Opus | `blob_swarm.rs`, `p2p_iroh/shard.rs` | roadmap T2, re-scoped |

Disjointness: S0-a/S1′/S4/S6 all touch `blob_swarm.rs` — one claimant at a time in the order
S0-a → S6 → S1′ → S4. C6-b waits for the in-flight `p2p/mod.rs` commit. S0-d is doorway-only.

**Sequencing.** (in-flight lands) → S0-a ∥ S0-b ∥ S0-c ∥ S0-d → S6 → S1′ → S4 → S5 → S3 (first
curve number on the 3-peer mesh) → C6-a ∥ C6-c → C6-b → C6-d → T2 → re-measure S3 on `dual`.
Prove every step on the local mesh; the fleet confirms.

## 10. Verification and habit linkage

- **Habits.** `blob-durability` (register full at 12 → new *checks*, not a new habit):
  `@concern:blob-durability` scenarios `swarm-curve` (S3), `blind-custody-reads-through-a-stranger`
  (Jessica reads with Matthew dark; Adam cannot), `reader-rekey-then-read` (C9), `composite-hash-
  mismatch-refused` (S0-a). **These land born-red under a green habit: by covenant rule 4 the first
  run that includes them flips `blob-durability` red until they pass — that is intended, and the
  one-line delta says so.** `reach-enforced-everywhere` (red): the serve-path verdict for ciphertext
  (§5) is one `first_move` line answered; its scenario (`ciphertext-serves-without-reach-but-not-
  under-quarantine`) joins that habit's check. `dataplane-convergence`: unchanged check, the
  per-shard delta rides the same convergence test.
- **Adversarial acceptance (Codex request #2):** a quarantined manifest is retracted from inventory,
  Kad and announce, not fetched/pushed/re-placed, purged from both doorway caches — *and* the
  household's identity, counsel route and recovery request still work; a `refuse` destroys locally
  and recovery still works; a blind custodian cannot enumerate the author's reader set; a forged
  inventory snapshot under a victim's `peer_id` is not applied; a pushed shard with no commitment is
  refused; a manifest whose reassembly mismatches is refused and its source demoted.
- **Metrics:** existing `blob_swarm_shard_fetched{result}`, `blob_swarm_composite_completed`,
  `distinct_source_peers`; new composite-elapsed histogram by holder count,
  `composite_refused_hash_mismatch`, `quarantine_actuated{plane}`.
- **No habit flip from this spec.** Design moves nothing; the first S3 run does.

## 11. Decisions (resolved — nothing left for the operator)

1. **Bands:** `none ≤ 16 MiB` · `rs-4-7 (16 MiB, 256 MiB]` · `rs-8-12 > 256 MiB`. A ≤16 MiB blob is
   single-source by design (one shard = one transfer; parallelism below that is noise on household
   uplinks). S3 *confirms* on the mesh and may lower the floor; it does not reopen the decision.
2. **Recovery:** the floor envelope is **mandatory** in every ring; no Shamir shares of the DEK are
   ever pre-placed; the quorum path is the existing imagodei-key social recovery. There is no
   "household opts out of recovery" — that would be the absolute lockout the canon forbids.
3. **Reader-key home:** reuse `attestation:key-stewardship` (DNA-hash-neutral) + conductor-signed
   self-certifying record; `attestation:reader-key` is not minted. If a future DNA-hash-moving
   ceremony (C5) happens anyway, promoting to a dedicated subtype is a cosmetic option, not a need.
4. **Guardians:** non-goal; guardian-excluded ring composition is a precondition, not a default.
5. **Hold vs destroy:** `quarantine` holds, `refuse` destroys; the verdict kind decides, never the peer.

## 12. Red-team verdicts folded (2026-08-23, three lenses, 36 findings)

| Lens | Finding | Disposition |
|---|---|---|
| dataplane | peer-supplied manifest never verified against composite hash | **S0-a**; rule 1 rewritten |
| dataplane | pushed shard has no manifest association | **S0-b** |
| dataplane | per-shard delta does not exist; curve is cadence-bound | §1 corrected; **S4** |
| dataplane | composite never on disk → snapshot erases it | fold = **substitution** (§3.1) |
| dataplane | `replace_into` × 3 destroys the bitfield; dedup fingerprint skips it | bit-OR `UPDATE`; fingerprint covers bitfield |
| dataplane | 3.5 KB budget drops hints first; F1 "silently stops" is stale | folded entry non-droppable; F1 rewritten |
| dataplane | manifest not deterministic; private path uses another config | encoder params recorded; verify-not-trust; `reach`/`mime` dead for verdicts |
| dataplane | custody presence composite-blind → re-kicks forever | **S0-c** |
| dataplane | `rs-8-12` unreachable; `create_shards` ignores encoding; double encode in memory; `u8` counts | S5 scope rewritten |
| dataplane | race is libp2p-bound; `IrohShardClient` has no callers | T2 = `ShardFetcher` trait (§8) |
| dataplane | scarce-first could front-load parity into the k-slot budget | within-class, data first; after ShardRole |
| dataplane | bitfield rows promote into six `lookup_hosts` consumers | `gossip-bitfield` source tier, excluded from replica counts |
| security | doorway blob pantry re-serves any 200 body ungated | **S0-d** + standalone security backlog row; ring never via `/blob` |
| security | Shamir shares in the ring = silent quorum takeover | **removed**; floor envelope instead |
| security | PUT dual-writes plaintext into the iroh store | rule 5 moved to the store; iroh `add_bytes` typed |
| security | newtype at `distribute_shards` closes 1 of ~15 write sites; `handle_push` stores anything | `StorableBytes` at `BlobStore::store`; push needs a commitment |
| security | envelope binds to nothing; anonymous; rollback; `agent_cid` suffix | bound envelope `{dek, cid, epoch, prev, author}`; ring signed; canonical `agent_cid` |
| security | `attestation:reader-key` is DNA-hash-moving; Floor 2 accept-all | reuse `key-stewardship`; authority = signature |
| security | "hold" contradicts refusal-to-instantiate | `quarantine` vs `refuse` |
| security | six propagation planes missed; silence clears nothing | §6 table rewritten with retractions |
| security | gossip unauthenticated; hints forgeable under a victim; replica map public via announce | S1′ gated on `peer_id` binding; demotion waits Stage 2; private manifests excluded from announce; map-is-public stated |
| security | readability hook is not an actuator | marked advisory |
| security | backfill re-stamps reach `commons` | reach dead on the manifest; EPR governs |
| security | no forward secrecy / reader revocation; `classify_custody_authorization` does not exist; sibling fails open | epoch + DEK rotation on removal; limit stated; fail-closed requirement + test |
| canon | reader-key DNA class false; reuse `key-stewardship` | adopted |
| canon | guardians "compose for free" contradicts red-team §4.2 | non-goal + guardian-excluded precondition; C13 partial |
| canon | immutable ring → revocation unenforceable; churn; dangling `key_ring_cid` | epoch/supersession; EPR declares head; limit stated |
| canon | no non-cryptographic recovery path | floor envelope mandatory (§11.2) |
| canon | C9 re-key orphans envelopes | answered: any DEK holder re-issues; fail loud |
| canon | ring leaks roster | unlabeled envelopes, trial decryption; count stated |
| canon | Step 4 bulk-skipped; C12/C13/C14 dropped | per-entity tables; carried |
| canon | C4 "answered" overclaimed | partial on the hint; answered on the commitment |
| canon | plan changes unnamed; pledge ceiling dropped; DEK/plaintext_cid unspecified | §7 table; ceiling restored; random DEK, convergent excluded, `plaintext_cid` dropped |
| canon | `reach-enforced-everywhere` missing; born-red scenarios under a green habit | habit added; serve-path verdict; covenant effect stated |
| canon | address forms (raw vs dag-cbor); legacy wire "not extended" | corrected (§4); transition stated (§3.1) |
| canon | QuarantineVerdict / ReaderKey entity blocks incomplete; no 1-yr count | completed |

Survived all three lenses unchanged: derived manifest / zero DHT writes; composite-root commitments;
encrypt-before-shard *ordering*; k-of-n completion; the bitfield as the one wire change; the
witnessed-harm inheritance (duty-on-knowledge, custodians never inspection agents, human-not-artifact,
CSAM at the plaintext edge only, no matcher shipped, council-TBD); frontmatter and habit-register logic.

## 13. Missing nodes (story-graph maintainer)

- **chain** swarm-curve / **between** "holder advertises composite" → "requester races shard N from a
  shard-only holder" / **missing node** "pushed shard knows its manifest; inventory substitutes one
  bitfield entry" / **state** designed (S0-b, §3.1), unbuilt.
- **chain** swarm-curve / **between** "shard N lands" → "neighbour sources shard N from me" /
  **missing node** "per-shard delta with the bit set" / **state** designed (S4), unbuilt — the
  superlinear enabler.
- **chain** swarm-integrity / **between** "shards self-verify" → "composite is the requested blob" /
  **missing node** "reassembled bytes hash to `blob_hash`" / **state** designed (S0-a), unbuilt.
- **chain** blind-custody / **between** "custodian holds the ring" → "reader decrypts with the author
  dark" / **missing node** "bound, signed, unlabeled envelopes + mandatory floor envelope, fetched off
  the authorized path" / **state** designed (§4), unbuilt (C6-b).
- **chain** blind-custody / **between** "reader re-keys" → "reader still reads" / **missing node**
  "a current DEK holder re-issues the ring at epoch+1" / **state** designed (C9), unbuilt.
- **chain** witnessed-harm / **between** "verdict notarized" → "payload stops propagating everywhere it
  went" / **missing node** "one local quarantine set actuates eight planes with retractions" /
  **state** designed (§6), unbuilt (C6-d), issuing role council-TBD.
- **chain** custody-release / **between** "custodian released" → "what it may retain" / **missing
  node** C14 witnessed residual / **state** unbound (carried from the 2026-08-09 plan; bond-decay pass).
