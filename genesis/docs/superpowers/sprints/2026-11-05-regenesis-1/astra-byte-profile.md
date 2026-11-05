WEAKENED — raw+BLAKE3 survives as the canonical address for newly produced native payloads, but the proposed validator, SHA-2 exclusions, representation binding, and store transition need changes before implementation.

1. **High — The prefix is header-determined; the proposed validator is insufficient.**

   [plan.md:101](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:101) accepts length 59, prefix `bafkr4i`, and the lowercase base32 alphabet; [plan.md:129](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:129) carries that check into DNA integrity.

   The binary encoding is `01 55 1e 20 || digest[32]`: 36 bytes, or 288 bits. Base32 needs 58 symbols, plus the `b` multibase marker: **59 characters is correct**. However, `afkr4i` covers only the first **30 header bits**, leaving two digest-length bits unchecked.

   I verified locally that encoding `01 55 1e 21 || zero[32]` produces a 59-character string starting `bafkr4i` that passes the proposed guard while declaring **33 digest bytes and supplying 32**. Lengths 34 and 35 also pass. Nonzero trailing padding bits pass too.

   **Fix:** decode, require CIDv1/raw/0x1e/32, reject trailing bytes, and require canonical re-encoding equality. An exact specialized check must additionally constrain character 8 to `[a-h]` and the final character to `[aeimquy4]`. Test malformed encodings, not merely 10,000 successful hashes.

2. **High — OCI and cites are not the only SHA-2 boundaries.**

   [cid.rs:30](/projects/elohim/elohim/epr/src/cid.rs:30) explicitly identifies Cargo `.crate` checksums and npm/pnpm tarball integrity as external byte identities. Constructors preserve SHA-256, SHA-384, and SHA-512 at [cid.rs:102](/projects/elohim/elohim/epr/src/cid.rs:102). These are not merely comments about hypothetical consumers: dependency records carry those addresses in `to_resource` at [model.rs:489](/projects/elohim/elohim/epr-rea/src/model.rs:489).

   A second, actionable omission is **ark executable artifacts**: [task.rs:234](/projects/elohim/elohim/ark/core/src/task.rs:234) requires a SHA-256 pin and rejects a supplied CID unless it equals the raw SHA-256 CID. BLAKE3 artifacts cannot pass that contract unchanged.

   **Fix:** distinguish native payload identity from externally supplied checksum evidence. Preserve external digests and verify them on ingestion; assign BLAKE3 after obtaining bytes. A checksum alone cannot yield their BLAKE3 address. Explicitly include ark’s artifact type, validators, and consumers in the clean break. None of this requires retaining SHA-2 as the canonical identity of newly produced native blobs.

3. **Low — Direct iroh key interoperability survives, with a precise boundary.**

   The installed dependency defines `Hash` as a BLAKE3 hash, computes it with ordinary `blake3::hash`, and exposes construction from exactly 32 digest bytes: [iroh-blobs hash.rs:17](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iroh-blobs-0.94.0/src/hash.rs:17), [hash.rs:32](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iroh-blobs-0.94.0/src/hash.rs:32). The wrapper imports the supplied payload directly at [blob_store.rs:85](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/blob_store.rs:85).

   Therefore, for an admitted raw+BLAKE3-256 CID, `Hash::from_bytes(cid_digest)` is the correct key. No equivalence table or second content identity is necessary. Do **not** call `Hash::new(cid_digest)`: that hashes the digest again.

   **Qualification:** this proves key compatibility, not availability. Holding every FastCDC chunk does not automatically materialize the whole-file object in FsStore. Nor does this mapping apply to retained SHA-2 records. Test empty, small, multi-chunk, and streamed payloads; transfer raw payloads rather than substituting a collection’s hash.

4. **High — “One authoritative store” survives; a persistent transport cache does not satisfy it.**

   [rulings-final.md:33](/projects/elohim/genesis/local-dev/radicle-survey/rulings-final.md:33) prohibits a second persistent payload copy. Yet [plan.md:130](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:130) retains FsStore as a “transport cache.” The existing path literally duplicates `data` into it at [http.rs:4273](/projects/elohim/elohim/elohim-storage/src/http.rs:4273), and imports receive persistent tags.

   Retention is also unresolved: [blob_store.rs:67](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/blob_store.rs:67) deletes **every tag naming a hash**, irrespective of which obligation owns each tag. Two commitments sharing bytes cannot safely use that operation as individual release.

   **Fix:** require demonstrated shared payload backing, or keep persistent FsStore delivery disabled until it becomes the sole authority. Receipts must cover independent reference release, crash recovery, restart, local operation, authorization, quarantine, and collection. Replace the now-unnecessary byte alias-index receipt with direct CID/digest round-trip evidence. Renaming duplication “cache” is not an implementation of the ruling.

5. **High — Valid chunks do not authenticate their ordering under the whole-file CID.**

   [plan.md:130](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:130) specifies `index/<cid>` containing a DAG-CBOR chunk list, while [plan.md:148](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:148) limits normal reads to per-chunk verification.

   Replace that index with a reordered list of valid, equal-length chunks: each chunk still verifies, lengths still agree, but the reconstructed bytes no longer match the requested whole-file CID. A filename and structural existence check do not cryptographically bind that list.

   Existing tier streaming provides stronger evidence: it verifies the manifest at [tier_blob.rs:240](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:240), then rejects a complete-output digest mismatch at [tier_blob.rs:325](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:325).

   **Fix:** authenticate the exact ordered manifest and its verified association with the whole-file CID, or recompute the complete digest before declaring successful delivery. For verified incremental delivery, use proofs anchored to the requested root. Add index substitution and reordering tests alongside corrupt-chunk tests.

6. **Medium — Thresholds can remain configuration only if representation is explicitly noncanonical.**

   [sharding.rs:173](/projects/elohim/elohim/elohim-storage/src/sharding.rs:173) selects none/chunked/RS from local thresholds. Meanwhile [re-genesis-1-shelf-picks.md:13](/projects/elohim/genesis/local-dev/radicle-survey/re-genesis-1-shelf-picks.md:13) promises byte-identical manifests from identical builds.

   Two producers with different RS thresholds can emit different shard manifests for identical bytes. That changes representation identity and custody obligations, although **the whole-byte BLAKE3 CID remains unchanged**. There is also an immediate inconsistency: [plan.md:130](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:130) says inline “when one chunk”; the declared maximum chunk is 4 MiB, while [plan.md:148](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:148) retains a 16 MiB inline threshold.

   **Fix:** separate physical inline layout, deterministic content chunking, and custody encoding. Readers use recorded parameters, never current thresholds. Permit separately identified representations under one payload CID; otherwise pin representation-selection policy in the profile. Changing configuration must not overwrite a manifest or silently reinterpret an existing commitment.

7. **Medium — Three positive vectors are not enough justification for permanent mirrors.**

   [plan.md:89](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:89) accepts three constructors; [plan.md:132](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:132) gives brit three vendored rows. The dependency objection is real: [eprfs-core/Cargo.toml:20](/projects/elohim/elohim/eprfs/eprfs-core/Cargo.toml:20) deliberately avoids the full codec’s crypto and ts-rs dependencies.

   But matching successful hashes does not align admission, canonicalization, or rejection behavior. A concrete surviving hazard is [address.rs:91](/projects/elohim/elohim/eprfs/eprfs-core/src/address.rs:91): `short_fingerprint()` labels **any digest** `sha256:`. Adding `compute_bytes()` makes that misleading operation available on BLAKE3 identities.

   **Fix:** extract a small dependency-neutral addressing kernel for Rust, retaining wrappers rather than duplicated policy. Keep independent TypeScript implementations under shared positive **and negative** conformance vectors. Guard citation projection by profile. Separate `verify_cid_any`’s digest verification from byte-surface admission; add streaming construction so large binaries do not require whole-file allocation merely to use the kernel.

**The ruling I would record instead**

Epoch 0 newly produced native payloads—including content, SPA/SSR bundles, artifacts, complete binaries, chunks, and erasure shards—use CIDv1/raw (`0x55`)/unkeyed BLAKE3-256 (`0x1e`, 32 bytes). Payload size does not select a hash. Structured records retain DAG-CBOR/SHA-256; citation fingerprints retain their existing SHA-256 construction; Holochain entry hashes remain unchanged.

External ecosystem checksums, including OCI, Cargo and npm integrity, remain algorithm-labelled evidence. They are not interchangeable native output identities. Native ingestion verifies applicable external evidence and computes the canonical BLAKE3 identity from bytes. Ark’s native artifact contract crosses with the other producers and consumers.

Canonical byte strings require complete structural validation and canonical encoding. Generic digest verification never grants admission or authorization. Citation short forms reject incompatible profiles.

An admitted byte CID maps directly to an iroh raw-blob key through its digest, without an alias table. Whole-payload, chunk, collection, and record identities remain distinct.

Each device has one authoritative payload store. FsStore may occupy that role only after durability, recovery, scoped-serving, quarantine, and obligation-aware retention receipts pass. A persistent duplicate transport cache does not qualify.

Chunk lists must be authenticated and bound to their complete payload identity. Delivery verifies that binding and the required bytes. Storage thresholds may vary without changing payload identity; representations retain explicit parameters and independent identities wherever their manifests differ.

Rust consumers share a minimal addressing implementation. Cross-language implementations share adversarial conformance vectors. The clean break removes legacy native address spellings; it does not erase external checksum semantics or the expressly preserved historical evidence.