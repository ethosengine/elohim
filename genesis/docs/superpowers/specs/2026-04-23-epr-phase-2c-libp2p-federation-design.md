# EPR Phase 2c — Libp2p Federation of Signed Atoms Design

**Status:** Design — authoritative for the Phase 2c implementation plan
**Date:** 2026-04-23
**Authors:** Matthew Dowell + Opus 4.7
**Parent specs:**
- `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`
- `genesis/docs/superpowers/specs/2026-04-21-elohim-epr-integrator-compatibility-contract.md`
**Companion plans:**
- Phase 1 (landed): `genesis/docs/superpowers/plans/2026-04-21-elohim-epr-codec-crate-plan.md`
- Phase 2a (planned): `genesis/docs/superpowers/plans/2026-04-22-elohim-epr-storage-foundation-plan.md`

---

## Why this document exists

Phase 1 landed the `elohim-epr` codec (signed CBOR envelope, CID, reach, coupling, validation).
Phase 2a lands storage + REST for signed atoms (`epr_atoms` + sibling tables, `/api/v1/epr/...`).
Phase 2b (deferred) reconciles the legacy `EprHead` encoder with the generalized `Envelope` and wires internal emitters through `epr_atoms`.

**Phase 2c proves the atom primitive on the wire** — before we invest in inward plumbing we prove signed-envelope semantics hold peer-to-peer: signature verification after transit, CID stability across CBOR round-trips, reach enforcement at the libp2p layer, and coexistence with the existing `/elohim/epr/1.0.0` EprHead-resolution protocol.

The learning from this phase shapes the reconciliation in Phase 2b. If we did 2b first, we'd be plumbing an internal contract whose wire-layer assumptions are unverified.

---

## 1. Sequencing rationale (recap)

Picked from four options on 2026-04-23:

1. ~~Phase 2b first (inward reconciliation)~~
2. **Phase 2c first (outward federation) — CHOSEN**
3. ~~Parallel 2b + 2c~~
4. ~~ADR-first~~

Recommendation accepted on the grounds that the atom primitive is the risky piece — the reach-at-wire-layer gate, CBOR framing, batch semantics, and cross-peer CID resolution are all unproven. Inward plumbing becomes mechanical once the outward shape is stable.

---

## 2. P2P design gate output

### Entity: `EprAtom` (carried by the new protocol)
- **Classification:** A (EPR-notarized) — the signed envelope is the protocol truth; SQLite row is a projection.
- **Address strategy:** Content-Derived (CIDv1 base32 of CBOR envelope).
- **Source of truth:** the signed envelope bytes, discoverable from any peer that holds them.
- **Anti-pattern check:** ✅ CID is identity; no UUID PK; protocol designed before any new HTTP route.

**Note on category naming:** the p2p-design-gate's Category A assumes Holochain DHT as notary. EPR atoms are notarized by ed25519 + CID outside Holochain — a parallel truth layer. "A (EPR-notarized)" makes the distinction visible.

### Entity: `/elohim/epr-atom/1.0.0` (wire protocol)
Not a data entity. Sync-message gate applies: what it carries is the `EprAtom` above. Design order per the gate:
1. Codec primitive — `elohim-epr` crate (Phase 1, done).
2. Storage projection — `epr_atoms` (Phase 2a, planned).
3. **Libp2p protocol — this spec.**
4. HTTP surface — already in Phase 2a, no new routes.

### Not-entities (skipped by the gate)
- **`FederatedEprStore`** — Rust abstraction wrapping `epr_atoms` + libp2p fetch. Design choice deferred; see §7.
- **EprHead ↔ Envelope reconciliation** — migration contract, not an entity. Phase 2b concern. This spec treats legacy EprHead as untouched.
- **`/elohim/epr/1.0.0`** — existing protocol. Coexists unchanged.

### Cross-entity constraint
Two libp2p protocols serve overlapping concerns until Phase 2b reconciles them. Honest dual-path debt — documented, bounded, and acknowledged in this spec.

---

## 3. Locked decisions

### 3.1 Protocol ID and scope

**ID:** `/elohim/epr-atom/1.0.0`

**Initial request types:**
- `FetchAtom { cid: String }` — single-atom fetch by CID
- `AnnounceAtom { envelope_bytes: Vec<u8> }` — push notification of a new atom
- `FetchBatch { cids: Vec<String> }` — batched fetch (bounded, see §3.5)

**Deferred to 2c.1:**
- Coupling / claim / supersedence fetch (graph semantics — separate learning cycle)
- Range/query fetch (no filtering; CID-addressed only)

**Rationale:** smallest surface that round-trips a signed envelope. Graph traversal adds its own failure modes.

### 3.2 Coexistence with `/elohim/epr/1.0.0`

Both protocols live in parallel for the duration of Phase 2c. The new protocol serves `Envelope`; the legacy protocol keeps serving `EprHead`. **No translation between them at the libp2p layer during 2c.** Translation is Phase 2b's responsibility.

Practical consequence: a peer that only implements `/elohim/epr-atom/1.0.0` cannot discover legacy EprHead-only content, and vice versa. Acceptable for a learning phase; loud in the spec so it isn't surprising in logs.

### 3.3 Wire framing

**CBOR body with 4-byte big-endian length prefix.**

Rationale:
- The envelope's CID is computed over CBOR bytes. MessagePack on the wire would force double-encoding (decode MsgPack, re-encode CBOR to verify CID) at every hop.
- The length-prefix convention matches existing `elohim-storage/src/p2p/epr_protocol.rs` (currently MessagePack + length prefix). Keeping the framing constant reduces cross-protocol confusion in the libp2p stack; only the payload codec differs.

**Message envelope (outer wire frame):**

```cbor
; The outer frame is NOT the signed envelope — it's the request/response carrier.
; The signed EprAtom's bytes live inside as raw CBOR.

EprAtomRequest = {
  0: "fetch" / "announce" / "fetch_batch",  ; tag
  1: payload,                                ; per-variant body
}

EprAtomResponse = {
  0: "atom" / "atom_batch" / "announced" / "not_found" / "access_denied" / "error",
  1: payload,
}
```

**Size bounds:**

| Field | Limit | Rationale |
|---|---|---|
| `MAX_REQUEST_SIZE` | 256 KB | Accommodates `FetchBatch` of ~100 CIDs; rejects abuse |
| `MAX_RESPONSE_SIZE` | 2 MB | Envelope atoms typically <4 KB; batch of 100 ~400 KB; 2 MB headroom for future payload growth |
| `MAX_BATCH_CIDS` | 128 | Bounds request fanout and response size |

### 3.4 Reach enforcement at libp2p layer

**Mirror the REST policy from Phase 2a.**

- `reach ∈ { Commons, Public }` — served to any peer, no authentication required.
- `reach ∈ { Collective, Steward, Private }` — requires an authenticated caller (libp2p PeerId → AgentPubKey mapping). Unauthorized caller receives `not_found`, **not** `access_denied`. Rationale: `access_denied` leaks atom existence.
- **PeerId → AgentPubKey mapping is stubbed in Phase 2c.** Identity integration is a Phase 2b concern. During 2c, Collective/Steward/Private atoms are effectively un-fetchable cross-peer — they're stored locally but never leave the node. Flagged loudly in the spec; no runtime surprises.

**Same gate everywhere** is the invariant. REST and libp2p return the same "visible set" to the same caller identity. Divergence would be a correctness bug.

### 3.5 Signature and CID verification on ingress

Every atom received via `AnnounceAtom` or `FetchBatch` response **must** be verified before acceptance:
1. Deserialize CBOR → `Envelope`.
2. Recompute CID over canonical CBOR bytes; reject if mismatch.
3. Verify ed25519 signature against `envelope.author`.
4. Run the same validator chain the REST ingest path uses (`elohim-epr::validation`).

An atom that fails any step is dropped; the sending peer's reputation receives a negative signal (mechanism stubbed in 2c; fleshed out when the trust layer matures).

---

## 4. Coexistence: what changes and what doesn't

| Component | Change |
|---|---|
| `elohim/epr/` codec | No change |
| `elohim/elohim-storage/src/p2p/epr_protocol.rs` | No change (legacy EprHead resolution unchanged) |
| `elohim/elohim-storage/src/epr_codec.rs` | No change |
| `/elohim/epr/1.0.0` behavior | No change |
| **New:** `elohim/elohim-storage/src/p2p/epr_atom_protocol.rs` | New file — this protocol |
| **New:** libp2p behaviour registration | Adds `epr_atom` field alongside existing `epr` in `behaviour.rs` |
| `/api/v1/epr/...` REST from Phase 2a | No change — REST and libp2p share storage, not protocol |

No migration. No feature flag. The protocol is additive. Peers that haven't upgraded simply don't negotiate the new protocol — they continue to serve EprHead via the legacy path.

---

## 5. Open questions (flagged, not decided)

These surfaced during the gate. They don't need answers to start Phase 2c, but the plan must touch them.

### 5.1 Announcement fanout policy
When a peer ingests a new atom (via REST or local authoring), how does `AnnounceAtom` reach other peers? Candidates:
- Gossipsub topic (simple, high-fanout, ordering not guaranteed).
- Direct send to K-closest peers from Kademlia (targeted, low noise, requires DHT).
- Hybrid: gossip for `Commons/Public`, direct for `Collective/Steward`.

Decide empirically during implementation. Start with direct send to a small peer set; measure before scaling.

### 5.2 Announcement dedup
Announcements are CID-keyed, but a peer may receive the same CID from multiple sources. Candidates:
- Bounded LRU of recently-seen CIDs (size TBD).
- Bloom filter sized to expected announcement rate.

Decide when announcement fanout is implemented — the dedup strategy depends on the fanout policy.

### 5.3 Integration with `kad_store.rs`
Does an announced atom register as a Kademlia provider record, or live in a separate libp2p-local index? Candidates:
- Kad provider records: leverages existing provider-discovery plumbing; couples EPR atoms to Kad's lifetime assumptions.
- Separate index: independent eviction policy; requires new peer-discovery logic.

Decide after the FetchAtom / AnnounceAtom round-trip works. Provider records are the natural extension if the Kad semantics fit.

---

## 6. Out of scope for Phase 2c

- Projector from `epr_atoms` → pillar tables (Phase 2b).
- Signal Harness migration to emit EPRs (Phase 2b).
- Write-through feature flag wiring (Phase 2b).
- Full identity integration at libp2p layer (Phase 2b).
- EprHead ↔ Envelope translation (Phase 2b).
- Coupling/claim/supersedence fetch over libp2p (Phase 2c.1).
- Range queries, filtering, pagination over libp2p (Phase 2c.1 or later).

---

## 7. `FederatedEprStore` — abstraction choice deferred

The name surfaced in brainstorming as a wrapper that unifies local `epr_atoms` access with cross-peer fetch. It's a useful concept but an implementation detail — not needed for the protocol spec.

**Deferred question:** trait on `EprService` vs. separate struct. Answer during plan authoring by reading the Phase 2a service layout.

---

## 8. Test surface

Cross-peer tests are the whole point of this phase. The plan must include:

### 8.1 Round-trip integrity (P0)
- Peer A authors atom → REST ingest → announces via libp2p → Peer B receives → signature + CID verify → SQLite projection.
- CID must be byte-identical on both sides.
- Signature must verify against the same `author` key.

### 8.2 Reach gate parity (P0)
- Same caller identity fetching the same atom via REST and via libp2p MUST see the same visibility (both succeed, or both `not_found`).
- Collective/Steward/Private atoms authored on Peer A with no shared trust context MUST return `not_found` to Peer B.

### 8.3 Batch semantics (P1)
- `FetchBatch { cids: N }` returns `N` results or `not_found` per CID.
- Exceeds `MAX_BATCH_CIDS` → protocol-level error, no partial acceptance.

### 8.4 Validation rejection (P1)
- Tamper with signature bytes → receiving peer drops atom, does not persist.
- Tamper with payload bytes → CID mismatch → drop.
- Expired atom → drop (validator chain catches).

### 8.5 Coexistence smoke (P0)
- Peer running both `/elohim/epr/1.0.0` and `/elohim/epr-atom/1.0.0` serves correctly on both. No cross-talk.

---

## 9. Integrator compatibility contract

This phase ships only one new wire shape (the `EprAtomRequest`/`EprAtomResponse` CBOR envelopes). Per the six-layer enforcement chain:

| Layer | Artifact |
|---|---|
| 1 — JSON schema | `elohim/sdk/schemas/v1/p2p/epr-atom-message.schema.json` (new) |
| 2 — Rust struct + serde_cbor | `elohim/elohim-storage/src/p2p/epr_atom_protocol.rs` |
| 3 — schema contract test | extend `elohim/elohim-storage/tests/schema_contract.rs` |
| 4 — TypeScript interface | **not needed in 2c** — no browser/Tauri client reaches libp2p directly |
| 5 — Golden vectors | `elohim/elohim-storage/tests/vectors/epr_atom_messages.json` (new) |
| 6 — Cross-language interop | **not needed in 2c** — Rust-only surface |

Layers 4 and 6 light up when a non-Rust peer (e.g., browser WASM libp2p) joins the protocol. Deferred until needed.

---

## 10. Related artifacts

| Artifact | Relationship |
|---|---|
| `elohim/epr/` | Codec primitive — consumed |
| `elohim/elohim-storage/src/p2p/epr_protocol.rs` | Legacy EprHead protocol — coexists, unchanged |
| `elohim/elohim-storage/src/p2p/epr_atom_protocol.rs` | **New** — this protocol |
| `elohim/elohim-storage/src/p2p/behaviour.rs` | Adds `epr_atom` behaviour field |
| `elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/` | Phase 2a storage (depends on it landing first) |
| `elohim/sdk/schemas/v1/p2p/epr-atom-message.schema.json` | **New** — wire contract |
| `genesis/docs/superpowers/plans/2026-04-23-epr-phase-2c-libp2p-federation-plan.md` | **Next artifact** — task-by-task implementation plan |

---

## 11. Dependency on Phase 2a

Phase 2c reads/writes `epr_atoms`. **Phase 2a must land the storage tables before 2c starts.** If 2a is still in flight when we're ready to execute 2c, two options:
- Wait (clean dependency, no throwaway work).
- Mock storage behind a trait, execute 2c against in-memory, swap when 2a lands.

Preference: wait. The mocking path risks the mock diverging from the real schema and masking bugs this phase is designed to surface.

---

## Summary

Phase 2c adds a new libp2p request-response protocol `/elohim/epr-atom/1.0.0` that carries signed CBOR-encoded `EprAtom` envelopes between peers. It coexists additively with the legacy `/elohim/epr/1.0.0` EprHead protocol — no migration, no feature flag. Reach enforcement mirrors REST. Signature and CID verification are mandatory on ingress. Identity integration is stubbed; three policy decisions (fanout, dedup, Kad integration) are flagged open and decided during implementation. Deferred to Phase 2b: EprHead↔Envelope reconciliation, projector into pillar tables, and write-through wiring.

The phase is done when two peers round-trip a signed atom with verified identity across the wire, and reach visibility matches REST.
