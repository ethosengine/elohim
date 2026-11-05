WEAKENED — FastCDC and whole-byte identity survive, but the ruling needs an immutable algorithm specification, protected vectors, and a reconstructible, transportable RS layout before implementation.

The local Cargo registry contains `reed-solomon-erasure 6.0.0` but no FastCDC source. FastCDC API observations below rely on prior documentation knowledge, not a locally verified 5.0.0 implementation. I ran no Git, Cargo, network, or write operations.

1. **High — “v2020 + size parameters + level 1” is not yet a complete interoperability contract.**

   **Evidence:** [plan.md:97](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:97) declares a published gear table without identifying its bytes; [shelf-picks.md:11](/projects/elohim/genesis/local-dev/radicle-survey/re-genesis-1-shelf-picks.md:11) explicitly required its hash. The dependency still says version 3 at [plan.md:120](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:120), while [STATE.md:10](/projects/elohim/genesis/local-dev/regenesis-1/STATE.md:10) selects 5.0.0.

   Rust and TypeScript **can** chunk deterministically. The hazards are implementation details: v2020’s 64-bit gear state, wrapping arithmetic, two-byte scanning optimization, shifted gear/mask tables, normalization masks, and returned cut-index convention. JavaScript bitwise operations truncate Numbers to 32 bits; Numbers also cannot exactly represent arbitrary 64-bit table entries. A plausible port can therefore produce consistently wrong boundaries.

   **Fix:** Pin `=5.0.0` as the reference implementation and an immutable protocol specification independently of the dependency. Explicitly select level 1 through the documented `with_level` API, subject to source verification. Specify unsigned modulo-\(2^{64}\) arithmetic, exact tables and masks, scan start/reset, cut inclusion, maximum handling, odd tails, and EOF. Use BigInt or proven two-word arithmetic. **No seed field is defensible** if the unmodified table is fixed by the profile; optional implementation seeding must be inaccessible under that identifier.

2. **High — The proposed vector can certify a changed algorithm against its own rewritten answer.**

   **Evidence:** [plan.md:118](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:118) specifies only “10 MiB xorshift” and seed `0x454c4f48494d`. “Xorshift” does not identify a generator: state width, shift constants, update ordering, output selection, and byte order remain unspecified. [vectors.rs:465](/projects/elohim/elohim/eprfs/eprfs-core/tests/vectors.rs:465) rewrites computed expectations when `EPRFS_WRITE_VECTORS` is present, then returns successfully. The second-run stability criterion at [plan.md:234](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:234) consequently proves repeatability, not continuity.

   **Fix:** Freeze the exact generator algorithm, seed as a hexadecimal string, length **10,485,760 bytes**, input digest, chunker revision, parameters, table digest, ordered `(offset,length,CID)` results, and whole-byte CID. Distinguish the **fixture generator seed** from chunker configuration.

   Add empty, 365-byte, min/avg/max ±1, odd-length, repetitive, forced-maximum, and short-final-tail fixtures. Verify streaming with different input-buffer divisions against one-shot chunking. Existing epoch vectors must be compare-only even in regeneration mode; changed expectations require explicit review and, when behavior changes, a new profile identifier. Independently reviewed vectors matter more than two implementations sharing one mistaken expectation.

3. **High — `rs-gf8` with data/parity counts does not identify an erasure code.**

   **Evidence:** [plan.md:98](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:98) declares only those fields. The cached implementation fixes additional choices: [build.rs:11](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reed-solomon-erasure-6.0.0/build.rs:11) uses reduction constant 29, corresponding to polynomial `0x11d`; [core.rs:430](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reed-solomon-erasure-6.0.0/src/core.rs:430) constructs a systematic matrix by multiplying a Vandermonde matrix by its top-square inverse. Another legitimate GF(256) RS implementation can produce different parity.

   Storage additionally computes `ceil(byte_len/data)`, appends zero padding, and splits contiguous data shards at [sharding.rs:236](/projects/elohim/elohim/elohim-storage/src/sharding.rs:236). Reconstruction trims to original length at [sharding.rs:423](/projects/elohim/elohim/elohim-storage/src/sharding.rs:423). A regression already documents incorrect handling of the padded tail at [sharding.rs:620](/projects/elohim/elohim/elohim-storage/src/sharding.rs:620).

   **Fix:** Give the code a versioned identifier that binds field representation, polynomial, matrix construction, shard ordering, and padding/layout rules. These constants need not be repeated in every manifest if the identifier unambiguously specifies them. Original byte length and shard geometry must be present or uniquely derivable. The proposed `ShardRef.len` can supply shard size, but validators must enforce consistency. Pin parity bytes and reconstruction from every four-of-seven subset, including padded inputs.

4. **High — The 454 MB artifact does not fit the proposed composition with existing whole-object RS.**

   **Evidence:** RS currently divides the **whole object** into four data shards at [sharding.rs:236](/projects/elohim/elohim/elohim-storage/src/sharding.rs:236). A 454 MB artifact therefore produces shards exceeding 100 MiB. The actual payload ceiling is **64 MiB minus 1 KiB**, at [shard_protocol.rs:30](/projects/elohim/elohim/elohim-storage/src/p2p/shard_protocol.rs:30). FastCDC’s maximum constrains CDC chunks, not these independently generated shards.

   This defeats the shelf’s transport argument at [shelf-picks.md:15](/projects/elohim/genesis/local-dev/radicle-survey/re-genesis-1-shelf-picks.md:15). A local PUT/restart/GET receipt can pass while distributed custody remains impossible.

   **Fix:** Specify bounded RS stripes or an explicit authenticated transfer representation for large logical shards. My preference is bounded stripes: four data shards of at most 4 MiB plus three parity shards, with stripe identity and shard index declared. Apply the RS admission threshold to the artifact; do not accidentally disable RS because each CDC chunk is below 64 MiB. Verify actual shard transport and recovery of the 454 MB artifact, not merely local reconstruction.

5. **Medium — The sizes are defensible starting parameters; their claimed benefits are unproved.**

   **Evidence:** [shelf-picks.md:14](/projects/elohim/genesis/local-dev/radicle-survey/re-genesis-1-shelf-picks.md:14) correctly calls the parameters provisional, but [line 20](/projects/elohim/genesis/local-dev/radicle-survey/re-genesis-1-shelf-picks.md:20) promises that successive binaries share “most chunks.”

   A 365-byte blob should remain one 365-byte chunk, without padding. A 1 MiB wasm may gain little deduplication; larger wasm and the storage binary offer more opportunities. The nominal binary chunk count is hundreds, with roughly 1,700–1,800 possible near-minimum chunks depending on what “454 MB” denotes. The configured average is not a guaranteed measured mean.

   Linking, compression, embedded timestamps, and broad rebuild changes can destroy sharing. Four-MiB chunks also quadruple the previous maximum tier-buffer requirement: [tier_blob.rs:14](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:14).

   **Fix:** Retain these parameters experimentally. Measure chunk-size distribution, byte-weighted reuse, transferred bytes, manifest overhead, and Rust/TS CPU and memory across real successive binaries and wasm. Define empty input as zero chunks and permit a final chunk below minimum. Preserve one manifest head per artifact rather than introducing per-chunk DHT heads.

6. **High — The identity split survives, but a chunk list does not authenticate its claimed whole-byte CID.**

   **Evidence:** Existing code clearly separates artifact identity from manifest identity: [tier_blob.rs:29](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:29) names `record.output`, while [line 213](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:213) hashes the encoded manifest separately. That is two objects, not two competing identities for one object.

   However, the plan permits per-chunk verification without routine whole-byte verification at [plan.md:148](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:148). Valid chunk CIDs do not prove their concatenation matches a claimed artifact CID; reordering valid chunks illustrates the gap. Existing tier reads explicitly check the complete digest at [tier_blob.rs:325](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:325).

   Also, identical bytes cannot guarantee identical **full manifests**: `TierBlobManifest` embeds a derivation record, and DNA manifests include author and timestamps at [lib.rs:660](/projects/elohim/elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:660).

   **Fix:** Keep whole-byte identity independent of representation. Verify the whole-byte binding before accepting an imported representation; subsequent reads may reuse a trustworthy immutable local binding. Promise deterministic layout records, not byte-identical provenance envelopes. Persist the profile with every layout, including the storage index currently sketched as a bare vector.

**The ruling I would record instead**

Epoch 0 names complete artifact bytes by CIDv1 raw+BLAKE3-256. Chunking, erasure coding, provenance, and storage placement do not alter that identity. A canonical representation record names its artifact CID, byte length, explicit profile, ordered chunks, and any erasure layout; its record CID identifies that record, never substitutes for the artifact CID.

FastCDC v2020 uses min 262,144, target 1,048,576, max 4,194,304 bytes and normalization level 1. The immutable profile specifies the exact unseeded gear tables, masks, unsigned 64-bit arithmetic, scanning and cut semantics, and EOF behavior. No configurable seed is admitted. Rust fastcdc 5.0.0 is the pinned reference pending source/API verification; the TypeScript twin must reproduce the frozen conformance suite. Empty input produces no chunks; small nonempty inputs remain unpadded.

The suite fixes generator semantics, fixture seed, input length and digest, profile/table identity, boundaries, chunk CIDs, and whole-byte CID. Existing expectations cannot be silently regenerated. Streaming partitions must not change boundaries. Parameters remain experimental until representative sharing, transfer, and resource measurements exist.

RS remains 4+3, admitted by artifact size, under a versioned identifier fixing GF(256) polynomial `0x11d`, field representation, systematic matrix, index ordering, zero padding, and truncation. Encode consecutive stripes containing at most 16 MiB of original bytes; each stripe uses four equal contiguous data shards of `ceil(stripe_length/4)` bytes and three parity shards. Declare stripe order, original lengths, shard indices, lengths, and CIDs. Recover every four-of-seven combination and prove actual transport for the 454 MB artifact.

Unknown profiles are retained but not interpreted. Imported layouts require whole-artifact verification before acceptance. Deterministic layout metadata remains separate from provenance and custody observations; chunks introduce no independent DHT heads.