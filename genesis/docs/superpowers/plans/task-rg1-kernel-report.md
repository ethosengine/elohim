---
id: rg1-kernel-report
title: "Re-genesis 1, T0 — the profile kernel: elohim-address, elohim-epr 0.2.0 profile + chunker + signature envelope, consent-grant carrier"
status: landed
landed_commit: 77c83e07b
landed_on_dev: "dev b663aa65f..9f9fef877 (L1, 2026-11-06 01:45Z)"
verified_by: "orchestrator 184beb01 (claude-fable-5-1), post-L2 verification pass 2026-11-06 on the integrated tree 29a62b5c0/428752cb7 — genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/logs/verify-gate-elohim-epr.log, verify-gate-elohim-address.log, verify-gate-consent-grant.log, verify-gate-epr-ts.log, verify-gate-seam-contracts.log; rust-style-crossing-dc01152d8..29a62b5c0.log PASS (every EXIT=0; summary verify-pass-summary.log). Hook gates on the L2c push attempt (push-L2c-1.log): gherkin-prepush-lint, genesis, elohim-ark, genesis-a2o, elohim-storybook, elohim-app, rust-style all GREEN."
concerns: "EPR signing transcript changed (EPR wire break); signature.schema.json and the epr-ts verifier taken by T0 (overlap with T5.2/T5.4); storage consumers of Signature/CarrierProof/Dropped must adapt (T2); Erasure variant spelled RsGf8x11dV1; chunk_bytes panics on invalid params (try_chunk_bytes added); domain prefix-freeness is a registry rule, not a byte rule; commit attribution UNMEASURED (subagent session envelope bound to another checkout)"
class: protocol-canonical
gap: regenesis-1#T0
actor: agent:kernel-implementer@claude-opus-5-5
habits: []
commits: [7c9a9e6f7, 77c83e07b]
cites:
  - "native-dev-tooling-over-peers-plan | §7 rulings R1–R4 that re-genesis 1 amends; the kernel implements the amended R1 byte profile | sha256:a894a6f45fc92215 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
---

Branch `regenesis/kernel`, worktree `/projects/elohim/.worktrees/rg1-kernel`, base `1051d0957`.
Brief: `genesis/local-dev/regenesis-1/briefs/T0-kernel.md` (governed by plan §2.5 and the
design-gate record `genesis/local-dev/regenesis-1/design-gate.md`).

## What landed

**K1 — `crates/elohim-address`** (new, nested workspace root, wasm32-safe). Deps: `cid 0.11`
(alloc), `multihash-codetable 0.2 {sha2, blake3}`, `data-encoding` (base32), `thiserror`; `serde`
behind a feature; `blake3`/`sha2` as dev-only oracles. Full-decode byte-CID parser, three
constructors (bytes / citation / record), fail-closed verifier, sha2-only short fingerprint.
Vectors `crates/elohim-address/vectors/address-vectors.json`: positive rows derived outside the
crate (coreutils `sha256sum`, `@noble/hashes/blake3` — `b3sum` is not installed here — and a
hand-written Python CIDv1 encoder); 14 negative rows, each naming the failed check. Lockfile
boundary test (no tokio / ts-rs / serde_json / ipld-core / first-party crates; at most 72
packages, 58 today); the vectors are read with a test-local JSON reader because a `serde_json`
dev-dependency would enter the lockfile. New gate project `elohim-address` in
`elohim/epr/build-manifest.json` (+ recipe `_gate-elohim-address`: test, clippy, fmt, wasm32).

**K2 — `elohim-epr` 0.2.0.**
- `src/profile.rs`: byte/record profiles, the `fastcdc-2020` chunker with its immutable
  specification in the module doc (tables by digest, u64 wrapping, cut/EOF rules), the
  `rs-gf8-0x11d-v1` erasure declaration, `chunk_bytes`, `try_chunk_bytes`, `chunk_stream`.
  `fastcdc = "=5.0.0"` (crates.io fetch). Table digest `3bd9b879…8001` derived independently
  (Python parse of the crate's table literals, BLAKE3 by noble).
- `tests/chunker_vectors.rs` plus a `chunker` section appended to
  `elohim/eprfs/fixtures/tree-vectors.json` (additive only; eprfs-core's vector test clones the
  file, so the section passes through it untouched). 13 fixtures (10 MiB xorshift64* main,
  empty, 365 B, min±1, avg±1, max±1, odd, repetitive, forced-max, short-tail), each agreeing
  three ways: the pinned vectors, the fastcdc reference, and an independent re-implementation of
  the specification text. Streaming at 1 KiB / 7 KiB / 1 MiB+1 reads equals one-shot.
  `REGENERATE_VECTORS=1` only adds absent fixtures; it refuses to change a pinned one or the
  section's declaration.
- `src/signature.rs` recast: `SignatureScheme`, `Multikey`, `SignatureEnvelope`,
  `SignatureCheck`, `signing_transcript`; `Signature { signer, envelope }`. EPR envelopes now sign
  under the transcript (domain `elohim:epr:v1:`, epoch 0); `Epr::verify_with_key` returns
  `EprError::SignatureHeld(scheme)` for unknown schemes. ts-rs regenerated `Signature.ts` and
  the new `SignatureEnvelope.ts`; the epr-ts verifier twin follows; `signed_eprs.json`
  regenerated (now with `transcript_hex`); `signature.schema.json` recast.
- `src/cid.rs`: `compute_cid` / `compute_raw_cid` delegate to the kernel (outputs unchanged,
  pinned by `tests/profile_vectors.rs`); re-exports `compute_bytes_cid`, `verify_cid_any`,
  `ByteCid`, `Verify`, `AddressError`. `pub use elohim_address as address;` at the crate root.
- `crates/consent-grant`: `CarrierProof { signer, envelope }`, domain bumped to
  `elohim:device-carrier:v2:`, typed `proof_check`, `statement_transcript`,
  `CarrierProof::{ed25519, sign}`, `Ask::signature_from`, `Dropped::HeldScheme(u16)`; the
  expected key is still derived from the claimed signer and compared with the envelope key.
- `scripts/ci/cargo-publish-epr.sh` publishes `elohim-address`, then `elohim-epr`.

## Interfaces (downstream tracks read this block)

```rust
// crates/elohim-address  (crate elohim_address 0.1.0; re-exported as elohim_epr::address)
pub const BYTE_CODEC: u64 = 0x55;
pub const BYTE_MULTIHASH: u64 = 0x1e;
pub const BYTE_DIGEST_LEN: usize = 32;
pub const BYTE_CID_PREFIX: &str = "bafkr4i";
pub const BYTE_CID_LEN: usize = 59;
pub const BYTE_CID_BYTES: usize = 36;
pub const RECORD_CODEC: u64 = 0x71;
pub const SHA2_256: u64 = 0x12;
pub const RECORD_CID_PREFIX: &str = "bafyrei";
pub const CITE_CODEC: u64 = 0x55;
pub const CITE_CID_PREFIX: &str = "bafkrei";
pub enum AddressError { Length { len: usize }, Multibase, Alphabet, Padding, Base32,
    DecodedLength { len: usize }, Version { version: u64 }, Codec { codec: u64 },
    Multihash { multihash: u64 }, DigestLength { len: u64 }, NonCanonical, NotSha2 { multihash: u64 } }
impl AddressError { pub fn check(&self) -> &'static str }
    // "length" | "multibase" | "alphabet" | "padding" | "base32" | "decoded-length" | "version"
    // | "codec" | "multihash" | "digest-length" | "non-canonical" | "not-sha2"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)] pub struct ByteCid(/* Cid */);
impl ByteCid {
    pub fn parse(text: &str) -> Result<ByteCid, AddressError>;
    pub fn from_digest(digest: [u8; 32]) -> ByteCid;
    pub fn as_cid(&self) -> &Cid;
    pub fn digest(&self) -> &[u8; 32];          // iroh key: Hash::from_bytes(*digest)
    pub fn to_bytes(&self) -> Vec<u8>;
}
impl Display for ByteCid; impl FromStr for ByteCid /* Err = AddressError */;
impl TryFrom<Cid> for ByteCid /* Error = AddressError */; impl From<ByteCid> for Cid;
#[cfg(feature = "serde")] impl Serialize + Deserialize for ByteCid;   // string form, parsed on read
pub fn compute_bytes_cid(bytes: &[u8]) -> ByteCid;           // raw 0x55 + BLAKE3-256
pub fn compute_raw_sha2_cid(bytes: &[u8]) -> Cid;            // citations only (bafkrei…)
pub fn compute_record_cid(canonical_dag_cbor: &[u8]) -> Cid; // dag-cbor 0x71 + sha2-256
pub enum Verify { Verified, Mismatch, UnsupportedProfile { codec: u64, multihash: u64, len: usize } }
pub fn verify_cid_any(claimed: &Cid, bytes: &[u8]) -> Verify;
pub fn short_fingerprint(cid: &Cid) -> Result<String, AddressError>; // "sha256:<16 hex>", sha2 only

// elohim_epr::cid (0.2.0) — existing functions unchanged in output
pub fn compute_cid(canonical_bytes: &[u8]) -> Cid;      // = compute_record_cid
pub fn compute_raw_cid(bytes: &[u8]) -> Cid;            // = compute_raw_sha2_cid
pub use elohim_address::{compute_bytes_cid, verify_cid_any, AddressError, ByteCid, Verify};

// elohim_epr::profile
pub use elohim_address::{BYTE_CID_LEN, BYTE_CID_PREFIX, BYTE_CODEC, BYTE_DIGEST_LEN, BYTE_MULTIHASH,
    CITE_CID_PREFIX, RECORD_CID_PREFIX, RECORD_CODEC, SHA2_256};
pub const BYTE_PROFILE_EPOCH: u16 = 0;
pub const RECORD_PROFILE_EPOCH: u16 = 0;
pub const CHUNKER_FASTCDC_2020: &str = "fastcdc-2020";
pub const ERASURE_NONE: &str = "none";
pub const ERASURE_RS_GF8_0X11D_V1: &str = "rs-gf8-0x11d-v1";
pub const FASTCDC_2020_TABLE_DIGEST: &str = "3bd9b87949825cd9b2a592d199dc15d485812848535c9b1d2ff6979baf7a8001";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(tag = "id")]
pub enum Chunker { #[serde(rename = "fastcdc-2020")] Fastcdc2020 { min: u32, avg: u32, max: u32, level: u8 } }
impl Chunker { pub fn id(&self) -> &'static str; pub fn validate(&self) -> Result<(), ProfileError>; }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] #[serde(tag = "id")]
pub enum Erasure { #[serde(rename = "none")] None,
                   #[serde(rename = "rs-gf8-0x11d-v1")] RsGf8x11dV1 { data: u8, parity: u8 } }
impl Erasure { pub fn id(&self) -> &'static str; pub fn validate(&self) -> Result<(), ProfileError>; }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ByteProfile { pub epoch: u16, pub codec: u64, pub multihash: u64, pub chunker: Chunker, pub erasure: Erasure }
impl ByteProfile { pub fn validate(&self) -> Result<(), ProfileError>; }
pub const BYTE_PROFILE_0: ByteProfile; // {0, 0x55, 0x1e, Fastcdc2020{262144, 1048576, 4194304, 1}, RsGf8x11dV1{4, 3}}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordProfile { pub epoch: u16, pub codec: u64, pub multihash: u64 }
pub const RECORD_PROFILE_0: RecordProfile; // {0, 0x71, 0x12}
pub enum ProfileError { UnknownEpoch(u16), AddressTriple { codec: u64, multihash: u64 },
    ChunkerParameters(Chunker), ErasureParameters(Erasure) }
pub fn fastcdc_2020_table_bytes() -> Vec<u8>;
pub fn fastcdc_2020_table_digest() -> String;
pub fn chunk_bytes(profile: &ByteProfile, bytes: &[u8]) -> Vec<(usize, usize)>;   // (offset, len); panics on invalid params
pub fn try_chunk_bytes(profile: &ByteProfile, bytes: &[u8]) -> Result<Vec<(usize, usize)>, ProfileError>;
pub struct StreamChunk { pub offset: u64, pub data: Vec<u8> }
pub struct ChunkStream<R: Read>; // impl Iterator<Item = std::io::Result<StreamChunk>>
pub fn chunk_stream<R: Read>(profile: &ByteProfile, reader: R) -> Result<ChunkStream<R>, ProfileError>;

// elohim_epr::signature (crate root re-exports Multikey, Signature, SignatureCheck,
// SignatureEnvelope, SignatureScheme)
pub const ED25519_PUB_MULTICODEC: u64 = 0xed;
pub const ED25519_MULTIKEY_LEN: usize = 34;
pub const ED25519_SIGNATURE_LEN: usize = 64;
#[repr(u16)] pub enum SignatureScheme { Ed25519 = 0 }
impl SignatureScheme { pub fn code(self) -> u16; pub fn from_code(code: u16) -> Option<Self>; }
pub enum MultikeyError { Empty, Varint, NonMinimalVarint, MissingKey, Text }
#[derive(Debug, Clone, Eq)] pub struct Multikey(/* varint(codec) ‖ key */); // PartialEq + Hash by decoded (codec, key)
impl Multikey {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Multikey, MultikeyError>;  // minimal varint required
    pub fn ed25519(public_key: &[u8; 32]) -> Multikey;
    pub fn parse(text: &str) -> Result<Multikey, MultikeyError>;           // "z" + base58btc
    pub fn codec(&self) -> u64;
    pub fn key_bytes(&self) -> &[u8];
    pub fn as_bytes(&self) -> &[u8];
    pub fn ed25519_key(&self) -> Option<[u8; 32]>;
}
impl Display for Multikey;   // "z…" base58btc == did:key method-specific identifier
pub fn signing_transcript(domain: &[u8], epoch: u16, scheme: u16, key: &[u8], body: &[u8]) -> Vec<u8>;
    // = domain ‖ u16be(epoch) ‖ u16be(scheme) ‖ key ‖ body
pub enum SignatureCheck { Verified, Invalid, Held { scheme: u16 } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SignatureEnvelope { pub scheme: u16,
    #[serde(with = "serde_bytes")] pub key: Vec<u8>,     // canonical multikey bytes
    #[serde(with = "serde_bytes")] pub sig: Vec<u8> }
impl SignatureEnvelope {
    pub fn ed25519(vk: &[u8; 32], sig: [u8; 64]) -> Self;
    pub fn sign_ed25519(kp: &AgentKeypair, domain: &[u8], epoch: u16, body: &[u8]) -> Self;
    pub fn transcript(&self, domain: &[u8], epoch: u16, body: &[u8]) -> Vec<u8>;
    pub fn multikey(&self) -> Result<Multikey, MultikeyError>;
    pub fn check(&self, domain: &[u8], epoch: u16, body: &[u8]) -> SignatureCheck;
    pub fn check_for(&self, expected: &Multikey, domain: &[u8], epoch: u16, body: &[u8]) -> SignatureCheck;
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Signature { pub signer: Cid, pub envelope: SignatureEnvelope }
impl Signature { pub fn new(signer: Cid, envelope: SignatureEnvelope) -> Self; }
// removed: Signature::{ed25519, ed25519_checked}; fields `algorithm`, `signature`

// elohim_epr::epr
pub const EPR_SIGNATURE_DOMAIN: &[u8] = b"elohim:epr:v1:";
pub const EPR_SIGNATURE_EPOCH: u16 = 0;
pub fn epr_signing_transcript(public_key: &[u8; 32], canonical: &[u8]) -> Vec<u8>; // for keystore signers
// EprError gains SignatureHeld(u16)

// consent_grant (crates/consent-grant)
pub const CARRIER_DOMAIN: &str = "elohim:device-carrier:v2:";   // was v1
pub const CARRIER_EPOCH: u16 = 0;
pub struct CarrierProof { pub signer: String, pub envelope: SignatureEnvelope } // was { signer, signature: String }
impl CarrierProof {
    pub fn ed25519(signer: String, public_key: &[u8; 32], sig: [u8; 64]) -> Self;
    pub fn sign(key: &AgentKeypair, signer: String, kind: CarrierKind, state: &str,
                transport_id: &str, challenge: &str) -> Option<Self>;
}
pub fn statement(kind: CarrierKind, state: &str, transport_id: &str, challenge: &str) -> Option<Vec<u8>>; // domain no longer inside
pub fn statement_transcript(public_key: &[u8; 32], kind: CarrierKind, state: &str,
                            transport_id: &str, challenge: &str) -> Option<Vec<u8>>;
pub fn proof_check(proof: &CarrierProof, signer: &str, kind: CarrierKind, state: &str,
                   transport_id: &str, challenge: &str) -> SignatureCheck;
pub fn proof_holds(/* signature unchanged */) -> bool;   // proof_check(..) == Verified
impl Ask { pub fn signature_from(&self, transport_id: &str) -> elohim_epr::SignatureCheck; } // signed_from unchanged
pub enum Dropped { /* … */ HeldScheme(u16) }  // code "ask_signature_scheme_held"
```

```ts
// @elohim/epr (elohim/sdk/epr-ts)
export const EPR_SIGNATURE_DOMAIN = 'elohim:epr:v1:';
export const EPR_SIGNATURE_EPOCH = 0;
export function signingTranscript(domain: string, epoch: number, scheme: number, key: Uint8Array, body: Uint8Array): Uint8Array;
export function ed25519Multikey(publicKey: Uint8Array): Uint8Array;
export const ED25519_PUB_MULTICODEC = 0xed;
export type VerifyError =
  | { kind: 'cid-mismatch'; message: string }
  | { kind: 'scheme-held'; message: string; scheme: number }   // replaces 'algorithm-unsupported'
  | { kind: 'signature-invalid'; message: string };
export type { SignatureEnvelope };   // generated by ts-rs
// verifyEd25519 is now RFC 8032 strict (zip215: false), the twin of ed25519-dalek verify_strict
```

Wire shape (JSON; CBOR carries `key`/`sig` as byte strings):
`{ "signer": [cid bytes], "envelope": { "scheme": 0, "key": [237, 1, …32 bytes], "sig": [… 64 bytes] } }`.

## Gate evidence

Logs live in the worktree's (gitignored) campaign directory `/projects/elohim/.worktrees/rg1-kernel/genesis/local-dev/regenesis-1/`; each ends in an `EXIT=` line.

| Gate | Result | Log |
|---|---|---|
| `just gate elohim-epr` (fmt, clippy -D warnings, tests: elohim-epr, epr-rea, epr-index; 429 tests) | EXIT=0 | `gate-T0-elohim-epr.log` |
| `just gate epr-ts` (tsc build + vitest, 45 tests) | EXIT=0 | `gate-T0-epr-ts.log` |
| `just gate consent-grant` (122 tests, clippy, fmt) | EXIT=0 | `gate-T0-consent-grant.log` |
| `just gate seam-contracts` (incl. wasm32 build) | EXIT=0 | `gate-T0-seam-contracts.log` |
| `just gate elohim-address` (new: tests, clippy, fmt, wasm32) | EXIT=0 | `gate-T0-elohim-address.log` |
| `just gate rust-style` (RS-GOV-04: every touched crate debt 0 → 0) | EXIT=0 | `gate-T0-rust-style.log` |
| `cargo build --target wasm32-unknown-unknown -p elohim-address` (no default features, and `--features serde`) | EXIT=0 / EXIT=0 | `build-T0-wasm32-elohim-address.log` |

Also compiled (`cargo check --all-targets`): elohim-epr-rea, elohim-epr-index, elohim-ark-core,
elohim-ark-supervisor, elohim-ark, elohim-placement-core. TDD reds observed: the elohim-address
tests failed to compile before the crate existed; `chunker_vector_is_stable` failed on the absent
section before the one `REGENERATE_VECTORS=1` write.

## Concerns and deviations

1. **The EPR signing transcript changed (wire break, by design).** `Signature` could not become
   `{ signer, envelope }` while EPRs kept signing the bare canonical bytes without two meanings of
   `envelope.sig`. EPR envelopes now sign `elohim:epr:v1: ‖ 0000 ‖ 0000 ‖ (ed01 ‖ pk) ‖
   canonical`. No existing signed object was re-encoded: `tests/vectors/signed_eprs.json` is a
   generated test fixture (regenerated), and stored EPRs are household/alpha state the plan
   recasts. If any persisted EPR must survive, say so before T2 lands.
2. **Overlap taken from T5.** `elohim/epr/tests/schema_contract.rs` sits inside the T0 gate and
   validates `elohim/sdk/schemas/v1/objects/signature.schema.json`, so T0 recast that schema
   (T5.2's schema half; `elohim-views/src/epr.rs` `EprSignatureView` is still T5's). Likewise the
   epr-ts gate verifies the Rust vectors, so T0 updated `epr-ts/src/{epr,proof,index}.ts`, added
   `src/signature.ts`, and regenerated `Signature.ts`/`SignatureEnvelope.ts` (part of T5.4).
   T5 should rebase onto `regenesis/kernel` rather than redo them.
3. **Downstream compile breaks (expected, owned elsewhere).** elohim-storage uses
   `Signature::ed25519`, `.algorithm`, `.signature`, `CarrierProof { signature }`, the old
   carrier statement bytes (conductor-signed: it must now sign `statement_transcript`) and an
   exhaustive `match` on `Dropped` (needs `HeldScheme`) — `api/epr.rs:619-641`,
   `api/signal_emit.rs:279,316` (use `epr_signing_transcript`), `services/epr_service.rs`,
   `p2p/epr_atom_protocol.rs:294-300`, `trust/snapshot_source.rs`, `services/device_carrier.rs`
   (`proof_by`, `sign_carrier`, `:1173`), tests `epr_atom_federation_integration.rs`. That is T2.
   `elohim/sdk/storage-client-ts/tests/epr-types.test.ts:120` reads `proof.algorithm` (T5/T2).
   ark-core's `candidate.rs` size bounds were the only break inside the elohim workspace; I
   changed them to the envelope's `sig`/`key` lengths (compile fix, not ark's typed HELD,
   which stays with T4/T11).
4. **`Erasure::RsGf8x11dV1`, not `RsGf8_0x11d_v1`.** The brief's spelling trips rustc's
   `non_camel_case_types`, which would need a new `allow` (RS-GOV-04). The wire id is exactly
   `rs-gf8-0x11d-v1`.
5. **`chunk_bytes` keeps the brief's signature and panics on parameters outside fastcdc's
   ranges** (fastcdc only `debug_assert`s them; release builds would misbehave). Wire-read
   profiles go through `try_chunk_bytes` / `ByteProfile::validate` / `chunk_stream`, which
   refuse instead. Added `chunk_stream` (streaming, `StreamCDC`) for the store's `store_stream`.
6. **Transcript framing.** The transcript is the brief's exact byte rule. Scheme 0 fixes the
   key length (34), so key/body cannot slide; the domain is verifier-supplied and not
   length-prefixed, so domain prefix-freeness is a registry rule (every domain is
   `elohim:<object>:v<n>:`), not something the bytes enforce. A future scheme with
   variable-length keys should length-prefix them inside its own key encoding.
7. **Carrier domain bumped to `elohim:device-carrier:v2:`** (C12: the signed bytes changed);
   `statement()` no longer embeds the domain — it enters through the transcript.
8. **epr-ts verifies RFC 8032 strict** (`zip215: false`) so the TS twin refuses what
   ed25519-dalek's `verify_strict` refuses.
9. **Gate toolchain gap.** The pinned 1.96.1 toolchain carries no `wasm32-unknown-unknown` std
   here, so `_gate-seam-contracts`'s wasm leg failed before this change too. Both wasm legs now
   drop `RUSTUP_TOOLCHAIN` (stable 1.99.1 builds them), as the LSP's `env_wasm_roots` does.
   Installing the target for 1.96.1 needs a rustup download (no network beyond the cargo
   registry was granted).
10. **CI coverage of elohim-address.** The epr Jenkinsfile runs only the elohim workspace, so
    the kernel's own tests run in the local gate (`elohim-address`), not in CI;
    `crates/elohim-address/**` was added to the rust-build-test step sources so a kernel change
    dispatches the epr pipeline. A CI stage for the crate is a follow-up.
11. **Attribution UNMEASURED.** `epr session open --model claude-opus-5-5` refused
    (`session-conflict`: the runtime session `184beb01…` is bound to another checkout), and
    `epr actor claim` refused (`envelope-checkout-mismatch`); `berth moor … --task
    regenesis-1-T0` succeeded. Commits carry `Co-Authored-By: Claude Opus 5.5` and the hook
    printed `UNMEASURED`. Subagents of one runtime session cannot open their own envelope from
    a worktree today.
12. **Brit gap.** `brit status`/`log`/`commit` fail in this worktree with "The object database
    has too few index slots, current=32, needed=18"; every repository operation used a recorded
    Git fallback.
13. **Not done here (by the brief or by ownership):** no change to eprfs-core / brit / storage /
    doorway / DNA; no ts-rs or schema for `ByteProfile` (T5 decides its wire home); the TS
    chunker twin (BigInt) is not written; `Signature`'s `elohim-views` mirror is T5's.
