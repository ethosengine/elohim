REFUTED — scheme agility survives, but the proposed brit compatibility rule, signed transcript, and consumer handling are insufficient to build safely as written.

1. **High — “Absent scheme ⇒ 0” is safe only within a precisely frozen legacy format.**

   Stripping a field does not inherently forge an Ed25519 signature. It becomes a downgrade when the future signature—or its extractable Ed25519 component—authenticates bytes that the legacy verifier also accepts. The plan does not specify protection against that overlap (`genesis/local-dev/regenesis-1/plan.md:113`, `:132`). Today brit dispatches directly to Ed25519 using `agent_id` (`elohim/brit/brit-epr/src/engine/signing.rs:174`, `:183`).

   **Required downgrade test:** freeze a valid historical note as a positive control. Create a test-only future hybrid profile containing an Ed25519 component. Verify that an unsupported reader returns HELD. Then independently remove `scheme`, replace it with zero, remove `key`, and extract the Ed25519 component into the legacy `signature` field. Every transformed future record must remain non-admissible, while the historical fixture still verifies. Repeat through `read_verified`, not merely the cryptographic helper. A regression variant signing both profiles over the same legacy preimage should expose the vulnerability.

   **Fix:** restrict default-zero interpretation to frozen legacy notes; require explicit scheme and epoch on new records and cryptographically bind them.

2. **High — The stated brit signing surface is wrong, and defaulting can invalidate retained notes.**

   Brit does **not** currently sign canonical JSON with the signature field removed. `ContentNode::canonical_bytes` emits DAG-CBOR (`elohim/brit/brit-epr/src/engine/content_node.rs:15`). `without_signature` leaves `"signature": ""` in the object (`elohim/brit/brit-epr/src/elohim/attestation/build.rs:44`). Verification reconstructs and serializes that object (`elohim/brit/brit-epr/src/engine/signing.rs:175`); the “canonical JSON” comment is stale.

   Consequently, ordinary serde defaulting followed by serialization introduces `scheme: 0` into old signing bytes. Adding absent `key` as null has the same problem. Changing to JSON, removing the signature member, or introducing domain separation also breaks historical verification.

   **Fix:** preserve the historical serializer exactly, including field absence and empty signature. Pin historical preimage, signature and CID fixtures. New records should use D7’s exact-byte body framing; D7 explicitly forbids re-encoding before verification (`genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:413`). This is an explicit legacy exception, not evidence that old notes satisfy the new framing.

3. **High — HELD is a proposed label, not yet an end-to-end authority boundary.**

   The concrete consumers requiring changes are:

   - **Reuse:** brit’s `refs::admit` converts every verifier error into an unverifiable string (`elohim/brit/brit-epr/src/elohim/refs.rs:98`). `build_cmd::find` emits only `scan.verified` (`elohim/brit/brit-build-ref/src/build_cmd.rs:127`), which feeds `BritIndex::find` (`elohim/ark/cli/src/brit_index.rs:403`). This presently excludes failures, but loses the HELD distinction.
   - **Head folding and acknowledgements:** `Announcer::judge` maps errors to `Forged`; successful classifications can advance acknowledgements (`elohim/eprfs/eprfs-peer/src/announce.rs:148`, `:157`). HELD must reach neither path.
   - **Custody:** gossip applies inventory deltas to the live projection (`elohim/elohim-storage/src/p2p/gossip_dispatch.rs:537`); `salvage_pass` counts fresh inventory rows toward honored replicas (`elohim/elohim-storage/src/reconcile/custody.rs:507`). Custody announcements independently project holder claims (`elohim/elohim-storage/src/p2p/gossip_dispatch.rs:865`). Holding an envelope in those tables would confer custody significance.
   - **Snapshots:** missing proof becomes `UnprovenSigner`, and `is_accepted` accepts both provenance classes (`elohim/elohim-storage/src/trust/snapshot.rs:507`, `:707`). An unsupported attached proof must never be converted into missing proof.
   - **Consent and standing:** `Ask::signed_from` and carrier response validation depend on boolean proof checks (`crates/consent-grant/src/pending.rs:237`; `elohim/elohim-storage/src/services/device_carrier.rs:1230`). Roster standing separately requires verifying signatures (`elohim/epr-rea/src/participants.rs:871`); changing envelopes does not automatically update it.

   **Fix:** propagate typed HELD outcomes and keep bounded opaque retention outside authoritative projections. Test zero changes to custody counts, reuse candidates, acknowledgements, consent state and standing. Wider-reach election is explicitly deferred (`plan.md:29`); its HELD safety cannot honestly be certified from current implementation.

4. **High — Scheme and key identify cryptography; they do not define or authenticate meaning.**

   `check(domain, body)` does not specify whether scheme and key participate in the signed transcript (`plan.md:113`). The proposed inventory signs “body incl. author_peer,” although `epoch`, `holder` and `level` sit outside its listed delta body (`plan.md:130`). Unless protected elsewhere, changing `Hints` to `Full` could change interpretation without invalidating the signature.

   Existing snapshot and carrier verification derive the expected key from the claimed signer (`elohim/elohim-storage/src/trust/snapshot.rs:736`; `crates/consent-grant/src/carrier.rs:118`). Replacing those checks with verification against an envelope-supplied key would permit signer substitution.

   **Fix:** specify an unambiguous transcript binding object type/domain, object epoch, scheme, key and every consequential field. Bind the key to the expected signer independently. Domain and epoch need not be duplicated inside `SignatureEnvelope`; they must be mandatory, authenticated context. Preserve snapshot layout versioning separately from signature-scheme numbering (`snapshot.rs:317`).

5. **Low — The did:key equality survives, with a terminology correction and verification limit.**

   The local codec explicitly encodes `ed 01 || public_key`, where `ed 01` is the unsigned-varint encoding of `0xed` (`bridges/did/did-bridge/src/codec.rs:25`). Its DID and multikey constructors differ exactly by the `did:key:` prefix (`:108`, `:117`). `DeviceKey::did_key` uses that constructor (`elohim/eprfs/epr-cli/src/device_key.rs:307`), while `ChannelHead.device` stores the full DID (`elohim/eprfs/eprfs-peer/src/head.rs:29`).

   Thus `z6Mk…` equals the **method-specific identifier**, not the full DID or verification-method URI.

   **Fix:** compare decoded canonical multikey bytes, enforcing minimal varint encoding and scheme-specific lengths. Do not implement this as a prefix check. The codec cites the did:key specification (`codec.rs:8`), but I found no local specification copy; independent external-spec verification remains unperformed under your no-network constraint.

6. **High — A hybrid profile fits the envelope, but does not eliminate identity and transport changes.**

   A scheme can define one composite-key encoding and one composite-signature encoding inside the existing vectors. It must specify component ordering, parameters, framing, and **both signatures required**. An unordered list or “either verifies” is not equivalent.

   However, brit’s rule that the reader **always** uses raw-32-byte `agent_id` cannot represent a composite verifying key (`plan.md:132`). Snapshot and carrier signer derivation likewise extracts Ed25519 keys. Head key equality binds identity to the entire did:key multikey: adding a second key changes that identifier. Finally, heads have a hard 4,096-byte limit (`elohim/eprfs/eprfs-peer/src/head.rs:20`, `:149`); larger hybrid profiles require transport work.

   **Fix:** freeze raw-key identity handling to legacy epoch zero. Define new-record signer binding independently of legacy `agent_id`, and document future key authorization, roster changes and frame limits. Promise extensible envelopes, not migration-free identities.

7. **High — “Kept heads are unaffected” contradicts the proposed breaking change.**

   The plan changes `signature: String` to `sig: SignatureEnvelope` and moves the signing domain to v2 (`plan.md:131`), but says preserved heads gain fields additively (`:156`). Persisted `HeadTable.own` contains an actual `ChannelHead` (`elohim/eprfs/eprfs-peer/src/head_table.rs:114`). Loading rejects incompatible JSON and deliberately refuses reset because sequence continuity matters (`:237`).

   **Fix:** define the preserved-state boundary explicitly. Retain old signed heads as historical evidence and preserve sequence/acknowledgement state while issuing fresh v2 heads. Never reinterpret a v1 signature under v2 or silently reset the sequence. Require a restart fixture containing an existing `heads.json`.

8. **Medium — The kernel and schema proposals disagree on wire shape.**

   The kernel proposes `{signer, envelope}` (`plan.md:114`); the schema task proposes flat `{signer, scheme, key, sig}` (`:133`). Neither fixes multikey serialization or whether `sig: Vec<u8>` becomes a CBOR byte string. The current signature explicitly uses `serde_bytes` (`elohim/epr/src/signature.rs:17`).

   **Fix:** choose one exact JSON/CBOR shape, preserve byte-string encoding explicitly, and pin cross-language vectors. Schema validation must admit bounded unknown schemes far enough to return HELD rather than reject them as malformed Ed25519.

**The ruling I would record instead**

Signature agility is adopted through `SignatureEnvelope { scheme: u16, key: Multikey, sig: bytes }`. Scheme codes identify immutable, fully specified signature profiles. Scheme zero is strict Ed25519 with canonical multicodec `0xed`, a 32-byte public key and a 64-byte signature. A known scheme with incompatible key material is Invalid.

Every new signed surface declares its object epoch and exact signing transcript. The transcript unambiguously binds domain/object type, epoch, scheme, canonical key and exact body bytes. Domain and epoch may live outside the envelope; neither may be inferred from attacker-controlled defaults. All authority-bearing inventory metadata is protected. The envelope key must match independently established signer identity or authorization.

Structurally admissible unsupported schemes are HELD without asserting authenticity. Unsupported object epochs are also HELD. HELD bytes may be retained under bounded resource policy, but confer no custody, reuse, election, acknowledgement, consent or standing effect. Consumers preserve this distinction explicitly.

Absent scheme means zero only in the frozen historical brit format. Its DAG-CBOR preimage, empty signature member and field-presence semantics remain unchanged. New notes use an explicit versioned format with mandatory scheme and key. Historical notes are neither rewritten nor silently upgraded.

Channel heads use the v2 domain and canonical multikey equality with the device DID’s method-specific identifier. Preserved peer state retains sequence continuity; historical v1 signatures remain v1 evidence.

Hybrid profiles may use canonical composite keys and signatures within this envelope, requiring every component. Their admission separately specifies identity authorization and transport limits. Acceptance requires downgrade, historical-byte, signer-substitution, HELD noninterference, cross-language encoding and preserved-state restart tests.