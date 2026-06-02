# Holochain DNA — Protocol Integrity Layer

This directory contains the integrity and coordinator zomes that form the
Elohim Protocol's distributed truth layer. Every entry type here is a
**notary anchor** — a cryptographic proof that something happened, committed
to a distributed hash table that no single party controls.

## Why This Layer Exists

The question that determines what goes here:

**If this capability were centralized, could a handful of people extract
rent from everyone who uses it?**

If yes, it MUST be notarized here. If someone could become "the bank" by
controlling economic events, or "the credential authority" by controlling
attestations, or "the governance board" by controlling consent records —
that capability must live on distributed infrastructure where no one can
capture it.

The DHT has real limits: ~100 entry types per DNA, writes are slow and
expensive, queries require link traversal (no SQL). So the DHT is the
**notary**, not the database. It holds the minimal proof:

- **Who** — agent public key (unforgeable)
- **What** — content-addressed hash of the full record
- **When** — DHT timestamp (non-repudiable)

The full queryable data lives in elohim-storage (SQLite), projected from
post-commit signals with a `dht_anchor_hash` column that links back to the
DHT entry. Storage is the fast index. DHT is the truth. If they disagree,
the DHT wins.

## Entry Type Categories

### Notarized Protocol Primitives (SDK boundary)
These MUST be on the DHT because centralization = rent extraction:

- **Economic**: Agreement, Commitment, EconomicEvent, EconomicResource
  (if centralized, someone becomes the bank)
- **Identity**: Human, Agent, HumanRelationship, Attestation
  (if centralized, someone becomes the identity provider)
- **Content**: Content, LearningPath, ContentAttestation
  (if centralized, someone becomes the content landlord)
- **Infrastructure**: NodeRegistration, DoorwayRegistration
  (self-registration, not centrally assigned)

### Operational Entries (DHT-appropriate but not SDK primitives)
- Heartbeats, shard assignments, health attestations
- These support the network but aren't capabilities a human interacts with

## Pattern: Post-Commit Projection

```
create_entry(EntryTypes::Commitment(c))  →  source chain + DHT gossip
post_commit signal                        →  ProjectionSignal::ReaCommitmentCommitted
elohim-storage receives signal            →  upsert into SQLite with dht_anchor_hash
```

Clients write to the conductor. Storage listens and indexes. Never write
to storage directly for notarized types (legacy code may still do this —
migrate toward conductor-first).

## Coordinator Watch-outs

- **Read DHT entries back with `record.entry().to_app_option::<T>()`, never `Entry::try_into() -> SerializedBytes`.** An `Entry::try_into()` SerializedBytes round-trip serializes the `Entry::App(...)` *variant tag* into the bytes instead of unwrapping the inner app entry; on readback it fails with `Deserialize error: missing field '<first_field>'` even when the fixture and integrity structs have identical shape — the shape is fine, the *envelope* is wrong. This is a distinct failure mode from `serde_json::Value` at the zome-call boundary (that one is `feedback_serde_json_value_breaks_zome_boundary`; this is the `Entry` variant envelope on DHT readback). Sibling DNAs (mishpat, imagodei) use the correct `to_app_option` pattern. (Full lesson: memory entry `feedback_dht_readback_use_to_app_option`.)
- **`#[ignore]` on a sweettest is a CI NO-OP — it does NOT silence the test in CI.** The DNA sweettest stage invokes `cargo nextest run --release --run-ignored all`, which deliberately runs every `#[ignore]`-marked test (sweettests carry `#[ignore]` so *local* runs skip them; CI overrides to run them all). Adding `#[ignore]` to quarantine a broken sweettest does nothing in CI — the test still runs and still fails, costing a full ~75-min holochain build cycle. To actually remove a sweettest from the CI run you must DELETE it or change the Jenkinsfile invocation, not annotate it. (Full lesson: memory entry `feedback_sweettest_ignore_is_ci_noop`; the `--run-ignored all` invocation lives in `elohim/holochain/dna/Jenkinsfile`.)

## Build

```bash
just check   # Fast type-check (wasm32-unknown-unknown)
just build   # Full WASM build
just pack    # Build + pack DNA
```

RUSTFLAGS is set in the justfile. Don't override it.
