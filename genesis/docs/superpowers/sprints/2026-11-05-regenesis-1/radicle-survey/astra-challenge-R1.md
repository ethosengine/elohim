**REFUTED — R1 preserves useful CID agility, but prematurely admits a second protocol identity without defining compatibility, agreement, retention, or authorization across aliases.**

1. **High — The code evidence does not establish multihash agility.**

   The `IrohBlobStore` quotation is accurate: [blob_store.rs:1](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/blob_store.rs:1) documents separate BLAKE3 storage and a future cutover. That comment establishes intent, not readiness.

   Other claims need correction. `BlobStore::parse_content_address` does **not** accept `blake3-<hex>`. Worse, it accepts a parsed CID, discards its codec and multihash identifier, and returns its digest as presumed SHA-256: [blob_store.rs:226](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:226). `get_by_address` then constructs a `sha256-…` lookup. This is algorithm confusion, not safe compatibility.

   The tier’s complete-output verifier explicitly computes SHA-256 and requires multihash `0x12`: [tier_blob.rs:281](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:281), [tier_blob.rs:365](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:365). “Every record” being raw-codec is also overbroad: method descriptors use DAG-CBOR ([derive.rs:59](/projects/elohim/elohim/eprfs/eprfs-core/src/derive.rs:59)).

   The actual `BlobCid::verifies` definition remains unverified because `address.rs` was outside the permitted context; its caller’s documented contract supports SHA-256, not BLAKE3.

   **Fix:** distinguish CID parsing, permitted identity profiles, and implemented verification. Audit every digest-extraction boundary before admitting another algorithm.

2. **High — “BLAKE3 is admitted” contradicts the current release contract and lacks a rollout rule.**

   Rakia’s [`blobCid` definition](/projects/elohim/elohim/rakia/schemas/v1/release-manifest.schema.json:91) explicitly requires raw-codec, SHA-256 CIDv1 through both its description and `^bafkrei…` pattern. Artifacts separately require `sha256`, expressly a non-addressing checksum. BLAKE3 admission therefore requires a schema and reader transition; it is not merely exercising existing flexibility.

   D7 requires understood semantics before decisions and holds unknown epochs ([native-dev-plane design:410](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:410)). R1 supplies neither a compatible-reader floor nor a capability transition.

   **Fix:** retain self-describing CIDv1, but specify one canonical emission profile per object class and protocol epoch. Admit future algorithms through an explicit migration. Updating the schema regex alone is insufficient; both `blobCid` and `sha256` must verify against the same bytes.

3. **Medium — Bao is valuable, but does not require making BLAKE3 an interchangeable publication identity.**

   Installed iroh-blobs documentation describes requests by BLAKE3 hash and byte ranges ([README:5](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iroh-blobs-0.94.0/README.md:5)). Bao authenticates streamed ranges against the expected BLAKE3 root, allowing partial retrieval and corruption detection before receiving the whole object. Its decoder always validates; sender-side validation is separately selectable ([bao-tree:35](/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bao-tree-0.15.1/src/lib.rs:35)).

   Per-chunk SHA-256 CIDs instead authenticate complete chunks against a verified manifest. They support chunk-level resume, independent reuse and deduplication across artifacts, but do not provide subchunk proofs against the whole artifact’s ordinary SHA-256 digest. Conversely, Bao alone supplies neither those standalone chunk identities nor the manifest’s declared chunk composition, RS layout, or governance.

   Crucially, a valid Bao stream proves correspondence to **its BLAKE3 root**, not to an independently supplied SHA-256 CID.

   **Fix:** use Bao as transport verification, retaining canonical-CID verification before acceptance. A locally established dual-hash binding can accelerate subsequent transfers; a remote alias assertion cannot substitute for that verification.

4. **High — Byte equivalence is insufficiently separated from record identity and agreement.**

   R1 correctly requires local hashing before recording equivalence. But “a record names one CID” leaves producers free to choose different output identities for identical deterministic builds.

   [`DerivationRecord`](/projects/elohim/elohim/eprfs/eprfs-core/src/derive.rs:139) describes agreement through equal outputs; D5 graduates a converged **(derivation, output CID)** ([design:342](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:342)). Two honest producers choosing different hashes can therefore fragment convergence. Teaching agreement to consult equivalence makes a supposedly derived storage index relevant to trust decisions.

   Codec distinctions also matter: identical bytes addressed as raw and DAG-CBOR do not authorize interchangeable interpretation. Nor may an artifact CID be substituted for its manifest CID ([tier_blob.rs:27](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:27)).

   **Fix:** canonicalize output identity at production. Equivalence may locate payloads, but must not rewrite signed records, merge principals, transfer reach, or manufacture independent agreement.

5. **High — “Answers by canonical CID” is necessary but insufficient for an FsStore cutover.**

   R1’s interim position contradicts itself: a transport cache that holds bytes also present in the primary store is a second copy, while the same paragraph prohibits two stores holding the same bytes. Plan §7.2 explicitly rejects a second physical copy ([plan:266](/projects/elohim/genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md:266)).

   More seriously, FsStore’s wrapper retains blobs through tags; `forget(hash)` deletes **every tag** naming that hash, enabling collection ([iroh wrapper:53](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/blob_store.rs:53)). The footprint specification instead protects running, pinned, served, carried and arriving bytes, including concurrent arrivals ([device-footprint:115](/projects/elohim/genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md:115)).

   **Fix:** one payload owner, reconstructible local indexes, and retention evaluated across every reference to that payload. FsStore is sound behind this contract; it becomes a second authoritative identity if its hash or tags independently decide custody or deletion. F2 transfer success alone cannot prove that contract.

6. **High — R1.5 turns possession into serving authority.**

   “A CID is servable … when the device holds verified bytes” omits the controlling conditions. §10.12 requires serving to enforce task scope and object reach ([design:574](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:574)). The inalienable specification says custody confers no authority ([red-team design:79](/projects/elohim/genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md:79)).

   Blind custody requires precision: ciphertext serving enforces quarantine and pledge/rate constraints, whereas plaintext retains reach checks ([swarm-curve:407](/projects/elohim/genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:407)). Neither is unconditional serving.

   **Fix:** separate address resolution from authorization. Hash aliases inherit no permissions. This also preserves R4’s restricted-serving cases. I found no inherent no-conscription violation in hash agility itself.

7. **Medium — The generational cost is semantic migration, not changing a prefix.**

   D6 explicitly places legacy DNA custody identities in a batched migration ([design:559](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:559)). Wrapping an existing SHA-256 digest in CIDv1 preserves its digest; converting to BLAKE3 requires reading the bytes. Existing custody reconciliation uses string identities and SHA-256 normalization ([custody.rs:55](/projects/elohim/elohim/elohim-storage/src/reconcile/custody.rs:55)).

   Over ten years and thousands of peers, optional hashes impose repeated compatibility work across inventory, placement, retention, release verification and derivation agreement. Rebuilding aliases requires reading retained payloads; missing payloads cannot be rehashed. Signed historical records cannot simply be rewritten.

   **Three concrete failure scenarios:** an older peer rejects a BLAKE3 release while upgraded peers elect it; withdrawing a cache alias drops FsStore tags protecting bytes still promised under a SHA-256 custody commitment; an unauthorized carrier obtains private plaintext through an alternate CID when serving checks possession alone.

   **Alternative:** one canonical hash profile now, with CID-encoded agility reserved for governed migration. This avoids perpetual dual-identity semantics while retaining an escape from cryptographic obsolescence.

The ruling I would record instead

The protocol uses CIDv1 with explicit codec, multihash and digest length. Each object class and protocol epoch declares its accepted and canonical emission profiles. Artifact bytes and chunks currently use raw/SHA-256; structured records use their declared structured codec. BLAKE3 is an allowed internal transport/storage hash, not yet an interchangeable identity for release artifacts or derivation outputs.

Future hash admission requires a governed compatibility transition covering readers, schemas, verification, custody, retention and deterministic output agreement. Unknown profiles produce typed unsupported outcomes. Historical signed records remain unchanged.

Bao may authenticate transfers against BLAKE3 roots. Acceptance additionally verifies the requested canonical CID. Locally verified mappings between hashes are rebuildable indexes, never authority for permissions, standing or record equivalence.

Each device has one authoritative payload store. Transport indexes and outboards may reference its bytes without maintaining an independent persistent payload copy. FsStore may implement that store only after demonstrating canonical verification, atomic recovery, alias-index reconstruction, daemon-independent local operation, scoped serving and retention across all live references. Until then, pure-iroh artifact delivery waits or uses a demonstrated shared-payload implementation.

New protocol surfaces reject legacy address spellings. Compatibility readers validate algorithms explicitly; they never reinterpret arbitrary CID digests as SHA-256.

DNA custody migration preserves existing obligations and historical references. Rakia retains its current `blobCid` and `sha256` contract until a coordinated transition; both must describe the same artifact. `DerivationRecord.output` remains the canonical complete-artifact CID, distinct from chunk and manifest CIDs.

Possession and byte integrity permit lookup, not disclosure. Every serve applies the governing plaintext reach/task rules or ciphertext quarantine/pledge rules. Custody remains voluntary, and hash equivalence grants no additional rights.
