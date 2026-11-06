# Draft rulings R1–R4 for the artifact-store sprint (2026-11-05, pre-challenge)

Frame: Elohim Protocol is generational architecture. Rework to get a foundation right is acceptable;
a convenience that would have to be torn out is not. Integrity-class rulings are made by a Claude
model; Astra (gpt-6-astra) challenges each one before it is recorded.

## R1 — content address, and the BLAKE3 iroh-blobs store

**Evidence.** `elohim-storage/src/p2p_iroh/blob_store.rs:1-11` says the iroh-blobs `FsStore` is
"BLAKE3-keyed (iroh-blobs native); disjoint on disk from the legacy SHA256-keyed BlobStore" and that a
"Cutover (Phase 11) will replace legacy BlobStore call sites with this type uniformly. For now both
stores coexist, runtime config selects one." Every record in the protocol names bytes by CIDv1
(`BlobCid`, raw 0x55 + sha2-256; `BlobCid::verifies` dispatches on the declared codec and fails closed
on an unknown one). DNA custody entries still carry `sha256-…` strings and are slated for a batched DNA
move to CIDs (native-dev-plane spec §10.10). The dual-address router item
(`2026-08-04-dual-address-blob-router-unmanifested-blake3-400.md`) says "Do NOT resolve this ad hoc —
it touches the reach/serve contract." Radicle's lesson 14: bare SHA-1 RIDs forced a grammar change;
CIDv1 is self-describing, which is the agility Radicle lacked.

**Ruling.**
1. The protocol's byte address is **CIDv1, self-describing in codec and multihash**. The invariant is
   "every address is a CIDv1", not "every address is sha2-256". sha2-256 stays the default multihash for
   new records. **BLAKE3 (multihash 0x1e) is an admitted multihash**, so iroh-blobs' native verified
   streaming can be used on the iroh path under a legal CID (`bafkr4i…`), not under a private
   `blake3-<hex>` form.
2. Two CIDs of the same bytes are a **verified equivalence**: recorded only by a peer that hashed the
   bytes itself under both multihashes, never accepted from a peer's claim. A record names one CID; an
   equivalence is a derived index entry.
3. **One byte store per device.** The physical store may be the iroh-blobs `FsStore` (the Phase 11
   cutover direction) **only if it answers by the canonical CID asked**, through the verified
   equivalence index, and only after the per-mode F2 receipt proves it. Until that cutover,
   `IrohBlobStore` is a transport-side cache that never holds a CID the device's one store lacks and is
   never an identity. No device holds the same bytes in two stores.
4. The legacy `sha256-<hex>` and `blake3-<hex>` address forms are refused on every new surface
   (manifests, records, routes); `parse_content_address`'s acceptance of them is a read-side
   compatibility shim with a retire-when.
5. The dual-address router question is answered: a CID is servable by its own form when the device
   holds verified bytes for it; transport negotiation selects a path, never an identity.

## R2 — custody floor per reach rung

**Evidence.** Reach rungs (`elohim/epr/src/reach.rs`): Private, SelfScope, Intimate, Trusted,
Familiar, Community, Public, Commons. D6: cache bytes are a local pledge with zero DHT heads at `self`
or `intimate`; release objects carry holder sets on the DNA. Phase 3 custody: "two peers are not
necessarily two households"; failure-domain placement was the highest-priority deferred strategy.
Phase 2: a 600 s inventory freshness horizon; repair is bounded by kick limits and a 120 s sweep. The
dwelling-hub resolver already maps `peer_id → agent_cid → dwelling_hub_id`. `adoptionDiscipline` is a
constitutional artifact notarized with the release, not a per-peer preference.

**Ruling.**
1. The shape is fixed: a channel head at a given reach may be elected only when every chunk of each
   artifact's manifest closure has **≥ k committed custodians other than the publisher across ≥ d
   declared failure domains**, each counted from a holder claim bound to its authenticated sender and
   fresher than `freshnessSecs`, with a verified fetch on record; a closure that later falls below k
   shows a typed `under-custodied` state per copy and is repaired within `repairSecs` from consenting
   capacity; no peer is conscripted.
2. A **failure domain is declared, never inferred**: the device's dwelling hub (household). d counts
   distinct dwelling hubs.
3. Defaults by rung, carried in `adoptionDiscipline.custodyFloor` (new fields), overridable per
   channel by its standing body, never below the rung below:
   - Private / SelfScope: the local pledge only; k = 0, zero DHT heads (D6).
   - Intimate: k ≥ 1, d ≥ 1 (the person's second device counts).
   - Trusted / Familiar: k ≥ 2, d ≥ 2.
   - Community / Public: k ≥ 3, d ≥ 2, and RS 4+3 sharding above the shard threshold (§10.11).
   - Commons: k ≥ 5, d ≥ 3.
   - `freshnessSecs` 600 (phase 2's horizon); `repairSecs` 900.
4. Three custodians in one dwelling hub count as one toward d. The publisher's own devices never count
   toward k.

## R3 — ordering of the hardening lane (H1 holder binding, H2 BlobStore atomicity)

**Ruling.** H1 and H2 land **first** in the sprint, on their own elohim-storage write-set, as lane 0.
They do not gate Track E slices E2–E4 (the tier ALPN admits only the person's own devices, and the
eprfs tier store already writes temp + fsync + rename). They **do** gate: F2's receipt in any mode
that rides `BlobStore`; every custody count at Trusted reach and above; the release-head gate (R2).
H1 is integrity-class and gets a Claude-family adversarial review before it lands. The Stage-2
inventory signature floor ships behind H1 as a declared level with an adoption metric.

## R4 — supersede / yank / revoke

**Evidence.** §10.1: a bad record is countered by `Recomputed{matched:false}`, never deleted. Offers
and intents are withdrawn by a `Dismiss` verdict note on the CID. Custodians are blind and never asked
to unseal. The inalienable red-team spec: "P2P cannot guarantee deletion"; the protocol must state its
erasure limit plainly. `holds.rs`: another peer's placement is released only by that peer's withdrawal.

**Ruling.** Three operations, one table, cells as defaults in the constitutional
`adoptionDiscipline`; a channel's standing body may tighten any cell and may never loosen `purge`.

| | head | advertise for adoption | serve to a peer pinned by an older head | retain under existing commitments | purge | newly adoptable | still verifiable |
|---|---|---|---|---|---|---|---|
| **supersede** (a newer release elected) | moves to the new winner | no | yes | yes | no | no | yes |
| **yank** (steward withdraws a release for a defect; a `Dismiss` note on the release CID) | moves to the prior elected winner, or `none`; never by recency | no | **yes** (rollback and heal must keep working) | yes | no | no | yes |
| **revoke** (signer standing revoked, or a mishpat ruling against the content) | as yank | no | only within the reach the ruling sets; a standing revocation alone does not reduce serving of already-adopted bytes | yes, for audit | **never by the release plane** | no | yes (verifiable is not adoptable) |

Erasure is a separate subject-right act outside the release plane, stated with its limit. Revocation
and validity are distinct: a revoked artifact's history stays verifiable against the standing recorded
at publication (survey lesson 12). The standing body today has one member; that is a fact about the
network, not an authority claim, and every cell is contestable by witness under §12.10.
