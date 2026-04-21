# elohim-epr Codec Crate — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the authoritative EPR codec — a Rust crate `elohim-epr` and companion TypeScript package `@elohim/epr` — that encodes/decodes the EPR atom defined in the graph substrate spec, with cross-language interop proven by shared test vectors.

**Architecture:** Rust crate owns the canonical implementation; ts-rs generates TS interfaces; hand-written parallel implementations of canonical CBOR + CID + Ed25519 on the TS side; both sides verified against a committed fixture set produced by the Rust implementation.

**Tech Stack:**
- Rust: `libipld-core`, `libipld-cbor` (dag-cbor = RFC 8949 deterministic encoding with CID support), `cid`, `multihash-codetable`, `ed25519-dalek`, `ts-rs`, `serde`, `chrono`, `thiserror`
- TypeScript: `@ipld/dag-cbor`, `multiformats`, `@noble/ed25519`, `@noble/hashes`, `vitest`

**Phase scope:** Implements §4 (EPR atom shape + canonical serialization + CID + proof) and the structural portion of §7 (signature + coupling validation) from the spec. Does not implement storage, manifest resolution, GraphQL, or agent hooks — those are Phases 2–6.

**Out of scope for Phase 1:**
- Any interaction with elohim-storage
- Payload schema validation (§7 stage 4) — needs manifest resolver (Phase 3)
- Integration with existing DNAs
- GraphQL
- Subscriptions

---

## Decisions locked for this plan

- **Crate location:** `elohim/epr/` — top-level Rust crate alongside `elohim/elohim-storage/`, `elohim/elohim-agent/`, etc.
- **TS package location:** `elohim/sdk/epr-ts/` — lives with other SDK TypeScript; publishable as `@elohim/epr` within pnpm workspace
- **CBOR library:** `libipld-cbor` (dag-cbor) on Rust; `@ipld/dag-cbor` on TS — both implement RFC 8949 §4.2.1 deterministic encoding with CID-aware serialization
- **Hash:** SHA-256 (matches Holochain's EntryHash; well-supported in both ecosystems)
- **CID:** CIDv1, codec `0x71` (dag-cbor), multihash `0x12` (sha2-256)
- **Signing:** Ed25519 via `ed25519-dalek` (Rust) and `@noble/ed25519` (TS)
- **Canonical bytes:** include `kind`, `schemaRef`, `schemaKey`, `reach`, `coupling`, `claims`, `supersedes`, `issuedAt`, `payload` in that order (alphabetically sorted as a CBOR map); exclude `cid`, `proof`, `supersededBy`
- **CID derivation:** `cid = CIDv1(codec=0x71, multihash=sha256(canonical-bytes))`
- **Payload format:** opaque `Vec<u8>` in this phase (dag-cbor bytes); schema validation deferred to Phase 2
- **Error handling:** `thiserror` with explicit error kinds: `EncodingError`, `DecodingError`, `SignatureError`, `CouplingError`, `InvalidCid`
- **TS type generation:** `ts-rs` derives on every exported type; `cargo test export_bindings` produces interfaces in `elohim/sdk/epr-ts/src/generated/`

---

## File Structure

### Rust crate — `elohim/epr/`

```
elohim/epr/
├── Cargo.toml                    # crate manifest + deps
├── README.md                     # short crate-level doc
├── src/
│   ├── lib.rs                    # public API surface (re-exports)
│   ├── error.rs                  # EprError + Result<T>
│   ├── reach.rs                  # Reach enum
│   ├── kind.rs                   # EprKind enum + required-coupling map
│   ├── coupling.rs               # Coupling struct
│   ├── signature.rs              # Signature struct
│   ├── envelope.rs               # Envelope struct + canonical bytes
│   ├── epr.rs                    # Epr (Envelope + payload) + builder
│   ├── cid.rs                    # CID derivation helpers
│   ├── cbor.rs                   # canonical CBOR encode/decode wrappers
│   ├── proof.rs                  # Ed25519 sign/verify
│   └── validation.rs             # structural validator (coupling requirements)
├── examples/
│   └── gen_vectors.rs            # test-vector generator binary
├── tests/
│   ├── cbor_determinism.rs       # dag-cbor determinism properties
│   ├── cid_vectors.rs            # known bytes → known CID
│   ├── sign_verify.rs            # Ed25519 RFC 8032 vectors + round-trip
│   ├── envelope_roundtrip.rs     # Envelope construct/encode/decode
│   ├── epr_roundtrip.rs          # full Epr sign + verify
│   ├── structural_validation.rs  # coupling requirement enforcement
│   └── vectors/                  # committed shared fixtures
│       ├── keypairs.json         # deterministic test keypairs
│       ├── envelopes.json        # known envelopes + their canonical bytes + CIDs
│       └── signed_eprs.json      # fully signed EPRs for cross-lang verify
└── benches/
    └── sign_verify.rs            # optional; only if we need perf data later
```

### TypeScript package — `elohim/sdk/epr-ts/`

```
elohim/sdk/epr-ts/
├── package.json                  # @elohim/epr in pnpm workspace
├── tsconfig.json
├── vitest.config.ts
├── src/
│   ├── index.ts                  # public API
│   ├── generated/                # ts-rs output
│   │   ├── Envelope.ts
│   │   ├── Signature.ts
│   │   ├── Coupling.ts
│   │   ├── Reach.ts
│   │   ├── EprKind.ts
│   │   └── index.ts
│   ├── cbor.ts                   # @ipld/dag-cbor wrapper
│   ├── cid.ts                    # CID derivation
│   ├── proof.ts                  # @noble/ed25519 verify (browser-safe)
│   ├── envelope.ts               # Envelope helpers + canonical bytes
│   ├── epr.ts                    # Epr verify flow
│   ├── validation.ts             # structural validator
│   └── errors.ts                 # EprError
└── tests/
    ├── cbor.test.ts
    ├── cid.test.ts
    ├── verify.test.ts
    ├── envelope.test.ts
    └── interop.test.ts           # loads Rust-generated vectors and verifies
```

### Workspace integration

- `elohim/Cargo.toml` — add `"epr"` to `workspace.members` (or root `Cargo.toml` depending on existing layout)
- Root `pnpm-workspace.yaml` — ensure `elohim/sdk/**/package.json` glob matches `epr-ts`
- `elohim/epr/` added to `pre-push` hook quality gate (if exists)

---

## Task Overview

23 tasks organized into five groups. Groups A–D are Rust; E is TypeScript + interop.

- **A. Rust scaffolding & primitives** (Tasks 1–4): crate, CBOR, CID, errors
- **B. Rust type system** (Tasks 5–9): Reach, EprKind, Coupling, Signature, Envelope
- **C. Rust signing & composition** (Tasks 10–14): canonical bytes, Ed25519, Epr builder, sign, verify
- **D. Rust validation & vectors** (Tasks 15–16): structural validator, vector generator
- **E. TypeScript port & interop** (Tasks 17–23): scaffolding, CBOR, CID, types, verify, interop tests, CI

---

## Prerequisites

Before Task 1, verify the working environment:

```bash
cd /projects/elohim
cargo --version          # expect 1.80+ (edition 2021)
pnpm --version           # expect 9+
ls elohim/elohim-storage # confirm we're in the right tree
```

If any prerequisite fails, stop and surface the issue.

---

## Task 1: Scaffold the `elohim-epr` crate

**Files:**
- Create: `elohim/epr/Cargo.toml`
- Create: `elohim/epr/README.md`
- Create: `elohim/epr/src/lib.rs`
- Modify: `elohim/Cargo.toml` (workspace members)

- [ ] **Step 1: Create `elohim/epr/Cargo.toml`**

```toml
[package]
name = "elohim-epr"
version = "0.1.0"
edition = "2021"
description = "Elohim EPR codec: canonical CBOR + CIDv1 + Ed25519 for the graph substrate"
license = "CAL-1.0"

[dependencies]
libipld-core = "0.16"
libipld-cbor = "0.16"
cid = { version = "0.11", features = ["serde-codec"] }
multihash-codetable = { version = "0.1", features = ["sha2"] }
ed25519-dalek = { version = "2.1", features = ["rand_core", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_bytes = "0.11"
serde_json = "1"
chrono = { version = "0.4", default-features = false, features = ["std", "serde"] }
thiserror = "2"
ts-rs = { version = "10", features = ["chrono-impl", "serde-compat"] }
hex = "0.4"

[dev-dependencies]
rand = "0.8"
```

- [ ] **Step 2: Create `elohim/epr/README.md`**

```markdown
# elohim-epr

Canonical codec for the Elohim EPR (EntityPortalReference) atom defined in
`genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.

Shipping wire primitives: canonical CBOR (dag-cbor / RFC 8949 §4.2.1),
CIDv1 (codec=0x71 dag-cbor, multihash=sha2-256), Ed25519 signatures.

Not a storage, resolver, or validator service — that's Phase 2+.
```

- [ ] **Step 3: Create `elohim/epr/src/lib.rs`**

```rust
//! elohim-epr — canonical codec for the Elohim EPR atom.
//!
//! See `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.

pub mod cbor;
pub mod cid;
pub mod coupling;
pub mod envelope;
pub mod epr;
pub mod error;
pub mod kind;
pub mod proof;
pub mod reach;
pub mod signature;
pub mod validation;

pub use coupling::Coupling;
pub use envelope::Envelope;
pub use epr::Epr;
pub use error::{EprError, Result};
pub use kind::EprKind;
pub use proof::{sign, verify, AgentKeypair};
pub use reach::Reach;
pub use signature::Signature;
```

- [ ] **Step 4: Register in workspace**

Inspect `elohim/Cargo.toml`. If a workspace manifest exists, add `"epr"` to `members`. If elohim's workspace is rooted elsewhere, add the path there.

```bash
grep -n "members" /projects/elohim/elohim/Cargo.toml
```

Add `"epr"` to the members array preserving existing formatting.

- [ ] **Step 5: Verify crate builds empty**

Create stub module files (each containing only a module comment) so `cargo check` passes.

```bash
for m in cbor cid coupling envelope epr error kind proof reach signature validation; do
  echo "//! TODO (Phase 1 plan): implement" > /projects/elohim/elohim/epr/src/${m}.rs
done
```

Run:
```bash
cd /projects/elohim/elohim && cargo check -p elohim-epr
```
Expected: clean build (may warn about unused modules — that's fine).

- [ ] **Step 6: Commit**

```bash
git add elohim/epr elohim/Cargo.toml
git commit -m "feat(epr): scaffold elohim-epr crate

New top-level Rust crate for the EPR canonical codec. Phase 1 of
the elohim-core graph substrate spec. Empty module stubs; workspace
registered; ready for canonical CBOR + CID + Ed25519 implementation.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Canonical CBOR encode/decode

**Files:**
- Modify: `elohim/epr/src/error.rs`
- Modify: `elohim/epr/src/cbor.rs`
- Create: `elohim/epr/tests/cbor_determinism.rs`

- [ ] **Step 1: Define error types**

Replace `elohim/epr/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EprError {
    #[error("cbor encode error: {0}")]
    Encode(String),
    #[error("cbor decode error: {0}")]
    Decode(String),
    #[error("invalid cid: {0}")]
    InvalidCid(String),
    #[error("signature error: {0}")]
    Signature(String),
    #[error("coupling requirement not met: {0}")]
    Coupling(String),
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(String),
}

pub type Result<T> = std::result::Result<T, EprError>;
```

- [ ] **Step 2: Write failing test — canonical CBOR round-trip**

Create `elohim/epr/tests/cbor_determinism.rs`:

```rust
use elohim_epr::cbor;
use libipld_core::ipld::Ipld;

#[test]
fn roundtrip_primitives() {
    for ipld in [
        Ipld::Null,
        Ipld::Bool(true),
        Ipld::Integer(42),
        Ipld::Integer(-7),
        Ipld::Float(3.14),
        Ipld::String("hello".into()),
        Ipld::Bytes(vec![1, 2, 3]),
    ] {
        let bytes = cbor::encode(&ipld).expect("encode");
        let decoded = cbor::decode(&bytes).expect("decode");
        assert_eq!(ipld, decoded);
    }
}

#[test]
fn deterministic_map_encoding() {
    // Two maps with different insertion order must encode identically
    let a: Ipld = Ipld::Map(
        [("b".into(), Ipld::Integer(2)), ("a".into(), Ipld::Integer(1))]
            .into_iter()
            .collect(),
    );
    let b: Ipld = Ipld::Map(
        [("a".into(), Ipld::Integer(1)), ("b".into(), Ipld::Integer(2))]
            .into_iter()
            .collect(),
    );
    let enc_a = cbor::encode(&a).unwrap();
    let enc_b = cbor::encode(&b).unwrap();
    assert_eq!(enc_a, enc_b, "canonical encoding must be order-independent");
}

#[test]
fn rejects_non_canonical_on_decode() {
    // 0x18 0x01 is uint with 1-byte length but value 1 should use 0x01
    let non_canonical = vec![0x18, 0x01];
    assert!(cbor::decode_strict(&non_canonical).is_err());
}
```

- [ ] **Step 3: Run test — verify failure**

```bash
cd /projects/elohim/elohim && cargo test -p elohim-epr --test cbor_determinism
```
Expected: fails to compile (cbor functions don't exist yet).

- [ ] **Step 4: Implement canonical CBOR wrapper**

Replace `elohim/epr/src/cbor.rs`:

```rust
//! Canonical CBOR wrapper using libipld-cbor's dag-cbor codec.
//!
//! dag-cbor implements RFC 8949 §4.2.1 ("Core Deterministic Encoding Requirements"):
//! sorted map keys, shortest-form integers, no indefinite-length items.

use crate::error::{EprError, Result};
use libipld_cbor::DagCborCodec;
use libipld_core::codec::Codec;
use libipld_core::ipld::Ipld;

/// Encode an Ipld value to canonical dag-cbor bytes.
pub fn encode(value: &Ipld) -> Result<Vec<u8>> {
    DagCborCodec
        .encode(value)
        .map_err(|e| EprError::Encode(e.to_string()))
}

/// Decode dag-cbor bytes to an Ipld value. Permissive: accepts any valid CBOR.
pub fn decode(bytes: &[u8]) -> Result<Ipld> {
    DagCborCodec
        .decode(bytes)
        .map_err(|e| EprError::Decode(e.to_string()))
}

/// Decode dag-cbor bytes with strict canonical-form enforcement:
/// the decoded value, re-encoded, must produce byte-identical output.
pub fn decode_strict(bytes: &[u8]) -> Result<Ipld> {
    let decoded = decode(bytes)?;
    let re_encoded = encode(&decoded)?;
    if re_encoded != bytes {
        return Err(EprError::Decode(
            "input is not canonical dag-cbor".into(),
        ));
    }
    Ok(decoded)
}
```

- [ ] **Step 5: Run test — verify passes**

```bash
cd /projects/elohim/elohim && cargo test -p elohim-epr --test cbor_determinism
```
Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add elohim/epr/src/error.rs elohim/epr/src/cbor.rs elohim/epr/tests/cbor_determinism.rs
git commit -m "feat(epr): canonical CBOR encode/decode via dag-cbor

Thin wrappers around libipld-cbor's DagCborCodec for RFC 8949 §4.2.1
deterministic encoding. Strict decode verifies input is canonical form
by re-encoding and byte-comparing.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: CID derivation

**Files:**
- Modify: `elohim/epr/src/cid.rs`
- Create: `elohim/epr/tests/cid_vectors.rs`

- [ ] **Step 1: Write failing test — CID from known bytes**

Create `elohim/epr/tests/cid_vectors.rs`:

```rust
use elohim_epr::cid::compute_cid;

#[test]
fn cid_from_empty_cbor() {
    // Empty CBOR object: 0xa0
    let bytes = vec![0xa0u8];
    let cid = compute_cid(&bytes);
    let s = cid.to_string();
    // CIDv1 dag-cbor of 0xa0 begins with `bafyrei` (CIDv1 base32 dag-cbor sha256).
    assert!(s.starts_with("bafyrei"), "got {s}");
}

#[test]
fn cid_stable_across_calls() {
    let bytes = vec![0x01, 0x02, 0x03, 0x04];
    assert_eq!(compute_cid(&bytes), compute_cid(&bytes));
}

#[test]
fn cid_differs_for_different_bytes() {
    let a = compute_cid(&[0x01]);
    let b = compute_cid(&[0x02]);
    assert_ne!(a, b);
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test cid_vectors
```
Expected: fails to compile.

- [ ] **Step 3: Implement CID derivation**

Replace `elohim/epr/src/cid.rs`:

```rust
//! CID derivation for EPR canonical bytes.
//!
//! CIDv1, codec = 0x71 (dag-cbor), multihash = 0x12 (sha2-256).

use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};

/// Codec byte for dag-cbor per the IPLD multicodec table.
const DAG_CBOR_CODEC: u64 = 0x71;

/// Compute CIDv1(dag-cbor, sha2-256) over the given canonical bytes.
pub fn compute_cid(canonical_bytes: &[u8]) -> Cid {
    let mh = Code::Sha2_256.digest(canonical_bytes);
    Cid::new_v1(DAG_CBOR_CODEC, mh)
}

/// Verify that a given CID matches the hash of the given canonical bytes.
pub fn verify_cid(claimed: &Cid, canonical_bytes: &[u8]) -> bool {
    compute_cid(canonical_bytes) == *claimed
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test cid_vectors
```
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/cid.rs elohim/epr/tests/cid_vectors.rs
git commit -m "feat(epr): CIDv1 derivation (dag-cbor + sha2-256)

Content addressing primitive for the EPR atom. CIDv1 with codec 0x71
(dag-cbor) and multihash 0x12 (sha2-256) per the IPLD spec.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Reach enum

**Files:**
- Modify: `elohim/epr/src/reach.rs`
- Create: `elohim/epr/tests/reach.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/reach.rs`:

```rust
use elohim_epr::Reach;
use serde_json;

#[test]
fn reach_serializes_lowercase_kebab() {
    for (variant, expected) in [
        (Reach::Commons, "\"commons\""),
        (Reach::Community, "\"community\""),
        (Reach::Collective, "\"collective\""),
        (Reach::Steward, "\"steward\""),
        (Reach::Private, "\"private\""),
    ] {
        let s = serde_json::to_string(&variant).unwrap();
        assert_eq!(s, expected);
        let r: Reach = serde_json::from_str(expected).unwrap();
        assert_eq!(r, variant);
    }
}

#[test]
fn reach_ordering() {
    // Design intent: commons is most-open, private is most-closed.
    assert!(Reach::Commons.openness() > Reach::Community.openness());
    assert!(Reach::Community.openness() > Reach::Collective.openness());
    assert!(Reach::Collective.openness() > Reach::Steward.openness());
    assert!(Reach::Steward.openness() > Reach::Private.openness());
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test reach
```
Expected: fails to compile.

- [ ] **Step 3: Implement Reach**

Replace `elohim/epr/src/reach.rs`:

```rust
//! Reach enum — envelope-level scoping primitive.
//!
//! Protocol-owned. No app may redefine what these mean. Gateways enforce
//! reach rules without parsing payload.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub enum Reach {
    /// Open to all — commons-level content.
    Commons,
    /// Open within the broader community / network.
    Community,
    /// Scoped to a specific collective / affinity group.
    Collective,
    /// Visible only to explicit stewards.
    Steward,
    /// Fully private; outside the substrate's public surface.
    Private,
}

impl Reach {
    /// Monotonically decreasing openness score (5 = most open, 1 = most closed).
    pub const fn openness(self) -> u8 {
        match self {
            Reach::Commons => 5,
            Reach::Community => 4,
            Reach::Collective => 3,
            Reach::Steward => 2,
            Reach::Private => 1,
        }
    }
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test reach
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/reach.rs elohim/epr/tests/reach.rs
git commit -m "feat(epr): Reach enum with openness ordering

Envelope-level scoping primitive. Five variants (commons, community,
collective, steward, private) with kebab-case wire format and a
monotonic openness score for gate logic.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: EprKind enum + required-coupling map

**Files:**
- Modify: `elohim/epr/src/kind.rs`
- Create: `elohim/epr/tests/kind.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/kind.rs`:

```rust
use elohim_epr::kind::{CouplingLeg, EprKind};
use serde_json;

#[test]
fn kind_serializes_pascal() {
    for (variant, expected) in [
        (EprKind::Content, "\"Content\""),
        (EprKind::Agent, "\"Agent\""),
        (EprKind::Manifest, "\"Manifest\""),
        (EprKind::Claim, "\"Claim\""),
        (EprKind::Observation, "\"Observation\""),
        (EprKind::EconomicEvent, "\"EconomicEvent\""),
        (EprKind::Commitment, "\"Commitment\""),
        (EprKind::Attestation, "\"Attestation\""),
        (EprKind::Delegation, "\"Delegation\""),
    ] {
        let s = serde_json::to_string(&variant).unwrap();
        assert_eq!(s, expected);
        let k: EprKind = serde_json::from_str(expected).unwrap();
        assert_eq!(k, variant);
    }
}

#[test]
fn required_coupling_per_kind() {
    // Content requires all three legs
    let c = EprKind::Content.required_coupling();
    assert!(c.contains(&CouplingLeg::Knowledge));
    assert!(c.contains(&CouplingLeg::Value));
    assert!(c.contains(&CouplingLeg::Governance));

    // EconomicEvent requires value only
    let ee = EprKind::EconomicEvent.required_coupling();
    assert_eq!(ee, &[CouplingLeg::Value]);

    // Manifest requires governance only (self-describing)
    let m = EprKind::Manifest.required_coupling();
    assert_eq!(m, &[CouplingLeg::Governance]);

    // Agent requires governance only (self-describing)
    assert_eq!(EprKind::Agent.required_coupling(), &[CouplingLeg::Governance]);
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test kind
```
Expected: fails to compile.

- [ ] **Step 3: Implement EprKind + CouplingLeg**

Replace `elohim/epr/src/kind.rs`:

```rust
//! EprKind enum — the nine EPR kinds defined by the graph substrate spec (§4.2).
//!
//! Each kind declares its required coupling legs. A malformed EPR — one missing
//! a required leg — is rejected at the structural validator (§7 stage 3).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub enum EprKind {
    Content,
    Agent,
    Manifest,
    Claim,
    Observation,
    EconomicEvent,
    Commitment,
    Attestation,
    Delegation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub enum CouplingLeg {
    Knowledge,
    Value,
    Governance,
}

impl EprKind {
    /// Return the coupling legs that MUST be present for this kind.
    pub const fn required_coupling(self) -> &'static [CouplingLeg] {
        use CouplingLeg::*;
        match self {
            EprKind::Content => &[Knowledge, Value, Governance],
            EprKind::Agent => &[Governance],
            EprKind::Manifest => &[Governance],
            EprKind::Claim => &[Knowledge],
            EprKind::Observation => &[Knowledge],
            EprKind::EconomicEvent => &[Value],
            EprKind::Commitment => &[Value, Governance],
            EprKind::Attestation => &[Governance],
            EprKind::Delegation => &[Governance],
        }
    }
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test kind
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/kind.rs elohim/epr/tests/kind.rs
git commit -m "feat(epr): EprKind enum with required-coupling map

Nine kinds per spec §4.2. Each kind's required coupling legs are
encoded as a const fn so the structural validator can enforce them
without heap allocation. Claim also requires knowledge OR governance —
simplified here to knowledge; broadened later if needed.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Coupling struct

**Files:**
- Modify: `elohim/epr/src/coupling.rs`
- Create: `elohim/epr/tests/coupling.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/coupling.rs`:

```rust
use cid::Cid;
use elohim_epr::kind::CouplingLeg;
use elohim_epr::Coupling;

fn test_cid(b: u8) -> Cid {
    elohim_epr::cid::compute_cid(&[b])
}

#[test]
fn empty_coupling() {
    let c = Coupling::default();
    assert!(c.knowledge.is_none());
    assert!(c.value.is_none());
    assert!(c.governance.is_none());
    assert!(!c.has(CouplingLeg::Knowledge));
}

#[test]
fn set_and_check_legs() {
    let k = test_cid(1);
    let v = test_cid(2);
    let c = Coupling {
        knowledge: Some(k),
        value: Some(v),
        governance: None,
    };
    assert!(c.has(CouplingLeg::Knowledge));
    assert!(c.has(CouplingLeg::Value));
    assert!(!c.has(CouplingLeg::Governance));
}

#[test]
fn json_roundtrip() {
    let k = test_cid(9);
    let c = Coupling { knowledge: Some(k), value: None, governance: None };
    let s = serde_json::to_string(&c).unwrap();
    let c2: Coupling = serde_json::from_str(&s).unwrap();
    assert_eq!(c, c2);
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test coupling
```
Expected: fails.

- [ ] **Step 3: Implement Coupling**

Replace `elohim/epr/src/coupling.rs`:

```rust
//! Coupling struct — ThreeLegCoupling attestation refs (spec §4.1).
//!
//! Every substantive EPR carries these; which are required is determined by
//! `EprKind::required_coupling()`.

use crate::kind::CouplingLeg;
use cid::Cid;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub struct Coupling {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub knowledge: Option<Cid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub value: Option<Cid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub governance: Option<Cid>,
}

impl Coupling {
    pub fn has(&self, leg: CouplingLeg) -> bool {
        match leg {
            CouplingLeg::Knowledge => self.knowledge.is_some(),
            CouplingLeg::Value => self.value.is_some(),
            CouplingLeg::Governance => self.governance.is_some(),
        }
    }
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test coupling
```
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/coupling.rs elohim/epr/tests/coupling.rs
git commit -m "feat(epr): Coupling struct with per-leg existence check

Three optional CID refs (knowledge / value / governance). The structural
validator uses has() to enforce EprKind::required_coupling().

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Signature struct

**Files:**
- Modify: `elohim/epr/src/signature.rs`
- Create: `elohim/epr/tests/signature_struct.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/signature_struct.rs`:

```rust
use cid::Cid;
use elohim_epr::Signature;

fn test_cid(b: u8) -> Cid {
    elohim_epr::cid::compute_cid(&[b])
}

#[test]
fn signature_constructs() {
    let signer = test_cid(99);
    let sig = Signature::ed25519(signer.clone(), vec![0u8; 64]);
    assert_eq!(sig.signer, signer);
    assert_eq!(sig.algorithm, "ed25519");
    assert_eq!(sig.signature.len(), 64);
}

#[test]
fn signature_rejects_wrong_length() {
    let signer = test_cid(99);
    // Ed25519 signatures are exactly 64 bytes
    assert!(Signature::ed25519_checked(signer.clone(), vec![0u8; 63]).is_err());
    assert!(Signature::ed25519_checked(signer.clone(), vec![0u8; 65]).is_err());
    assert!(Signature::ed25519_checked(signer, vec![0u8; 64]).is_ok());
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test signature_struct
```
Expected: fails.

- [ ] **Step 3: Implement Signature**

Replace `elohim/epr/src/signature.rs`:

```rust
//! Signature struct — detached proof on the EPR canonical bytes.

use crate::error::{EprError, Result};
use cid::Cid;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub struct Signature {
    /// CID of the issuer's Agent EPR.
    #[ts(type = "string")]
    pub signer: Cid,
    /// Signing algorithm identifier.
    pub algorithm: String,
    /// Raw signature bytes (64 bytes for Ed25519).
    #[serde(with = "serde_bytes")]
    #[ts(type = "Uint8Array")]
    pub signature: Vec<u8>,
}

impl Signature {
    pub fn ed25519(signer: Cid, signature: Vec<u8>) -> Self {
        Self { signer, algorithm: "ed25519".into(), signature }
    }

    pub fn ed25519_checked(signer: Cid, signature: Vec<u8>) -> Result<Self> {
        if signature.len() != 64 {
            return Err(EprError::Signature(format!(
                "ed25519 signature must be 64 bytes, got {}",
                signature.len()
            )));
        }
        Ok(Self::ed25519(signer, signature))
    }
}
```

The `ByteBuf` import is included for future use when binary-CBOR serialization needs it; leave the import present.

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test signature_struct
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/signature.rs elohim/epr/tests/signature_struct.rs
git commit -m "feat(epr): Signature struct with Ed25519 length check

Detached proof: signer CID (Agent EPR) + algorithm string + raw
signature bytes. ed25519_checked enforces the 64-byte length.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Envelope struct

**Files:**
- Modify: `elohim/epr/src/envelope.rs`
- Create: `elohim/epr/tests/envelope_struct.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/envelope_struct.rs`:

```rust
use cid::Cid;
use chrono::{TimeZone, Utc};
use elohim_epr::{cid::compute_cid, Coupling, Envelope, EprKind, Reach, Signature};

fn test_cid(b: u8) -> Cid { compute_cid(&[b]) }

fn sample_envelope() -> Envelope {
    Envelope {
        cid: test_cid(0),
        kind: EprKind::Content,
        schema_ref: test_cid(1),
        schema_key: "concept".into(),
        reach: Reach::Commons,
        coupling: Coupling {
            knowledge: Some(test_cid(2)),
            value: Some(test_cid(3)),
            governance: Some(test_cid(4)),
        },
        claims: vec![test_cid(5)],
        supersedes: None,
        superseded_by: None,
        issued_at: Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap(),
        proof: Signature::ed25519(test_cid(6), vec![0u8; 64]),
    }
}

#[test]
fn envelope_json_roundtrip() {
    let env = sample_envelope();
    let s = serde_json::to_string(&env).unwrap();
    let e2: Envelope = serde_json::from_str(&s).unwrap();
    assert_eq!(env, e2);
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test envelope_struct
```
Expected: fails.

- [ ] **Step 3: Implement Envelope**

Replace `elohim/epr/src/envelope.rs`:

```rust
//! Envelope — the protocol-owned header of an EPR (spec §4.1).

use crate::{Coupling, EprKind, Reach, Signature};
use chrono::{DateTime, Utc};
use cid::Cid;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub struct Envelope {
    /// Self-derived content identifier. NOT included in canonical signing bytes.
    #[ts(type = "string")]
    pub cid: Cid,

    pub kind: EprKind,

    /// CID of the Manifest EPR that declares the payload schema.
    #[ts(type = "string")]
    pub schema_ref: Cid,

    /// Content-type key within the referenced manifest.
    pub schema_key: String,

    pub reach: Reach,

    pub coupling: Coupling,

    /// Outcome claims this EPR asserts.
    #[ts(type = "string[]")]
    pub claims: Vec<Cid>,

    /// Prior version if this is a revision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub supersedes: Option<Cid>,

    /// Forward pointer; DERIVED from supersedence index, NOT in canonical bytes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[ts(type = "string | null", optional)]
    pub superseded_by: Option<Cid>,

    /// UTC timestamp, included in canonical bytes.
    pub issued_at: DateTime<Utc>,

    /// Detached signature. NOT included in canonical signing bytes.
    pub proof: Signature,
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test envelope_struct
```
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/envelope.rs elohim/epr/tests/envelope_struct.rs
git commit -m "feat(epr): Envelope struct

Protocol-owned header carrying cid, kind, schemaRef, schemaKey, reach,
coupling refs, claims, supersedes/supersededBy, issuedAt, proof.
camelCase wire, ts-rs export ready.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Canonical envelope bytes

**Files:**
- Modify: `elohim/epr/src/envelope.rs` (add `canonical_bytes` method)
- Create: `elohim/epr/tests/canonical_bytes.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/canonical_bytes.rs`:

```rust
use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, Coupling, Envelope, EprKind, Reach, Signature};

fn test_cid(b: u8) -> Cid { compute_cid(&[b]) }

fn env() -> Envelope {
    Envelope {
        cid: test_cid(0),
        kind: EprKind::Content,
        schema_ref: test_cid(1),
        schema_key: "concept".into(),
        reach: Reach::Commons,
        coupling: Coupling {
            knowledge: Some(test_cid(2)),
            value: Some(test_cid(3)),
            governance: Some(test_cid(4)),
        },
        claims: vec![test_cid(5)],
        supersedes: None,
        superseded_by: None,
        issued_at: Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap(),
        proof: Signature::ed25519(test_cid(6), vec![0u8; 64]),
    }
}

#[test]
fn canonical_bytes_excludes_cid_proof_superseded_by() {
    let mut a = env();
    let mut b = env();

    // Different cid, proof, superseded_by — canonical bytes must match
    b.cid = test_cid(42);
    b.proof = Signature::ed25519(test_cid(7), vec![1u8; 64]);
    b.superseded_by = Some(test_cid(99));

    let payload = b"hello";
    let ba = a.canonical_bytes(payload).unwrap();
    let bb = b.canonical_bytes(payload).unwrap();
    assert_eq!(ba, bb, "cid/proof/supersededBy must not affect canonical bytes");
}

#[test]
fn canonical_bytes_changes_when_schema_key_changes() {
    let a = env();
    let mut b = env();
    b.schema_key = "lesson".into();

    let payload = b"hello";
    assert_ne!(
        a.canonical_bytes(payload).unwrap(),
        b.canonical_bytes(payload).unwrap()
    );
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test canonical_bytes
```
Expected: fails (method doesn't exist).

- [ ] **Step 3: Implement `canonical_bytes`**

Append to `elohim/epr/src/envelope.rs`:

```rust
use crate::cbor;
use crate::error::Result;
use libipld_core::ipld::Ipld;
use libipld_core::cid::Cid as IpldCid;

impl Envelope {
    /// Compute the canonical bytes that get hashed for CID and signed for proof.
    ///
    /// Includes (in alphabetical map order): claims, coupling, issuedAt, kind,
    /// payload, reach, schemaKey, schemaRef, supersedes.
    /// Excludes: cid, proof, supersededBy.
    pub fn canonical_bytes(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut map: std::collections::BTreeMap<String, Ipld> = Default::default();

        map.insert(
            "claims".into(),
            Ipld::List(self.claims.iter().map(|c| Ipld::Link(*c)).collect()),
        );
        map.insert("coupling".into(), coupling_ipld(&self.coupling));
        map.insert(
            "issuedAt".into(),
            Ipld::String(self.issued_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        );
        map.insert("kind".into(), Ipld::String(kind_canonical(&self.kind)));
        map.insert("payload".into(), Ipld::Bytes(payload.to_vec()));
        map.insert("reach".into(), Ipld::String(reach_canonical(&self.reach)));
        map.insert("schemaKey".into(), Ipld::String(self.schema_key.clone()));
        map.insert("schemaRef".into(), Ipld::Link(self.schema_ref));
        if let Some(s) = self.supersedes {
            map.insert("supersedes".into(), Ipld::Link(s));
        }

        cbor::encode(&Ipld::Map(map))
    }
}

fn coupling_ipld(c: &Coupling) -> Ipld {
    let mut m: std::collections::BTreeMap<String, Ipld> = Default::default();
    if let Some(k) = c.knowledge { m.insert("knowledge".into(), Ipld::Link(k)); }
    if let Some(v) = c.value     { m.insert("value".into(),     Ipld::Link(v)); }
    if let Some(g) = c.governance{ m.insert("governance".into(),Ipld::Link(g)); }
    Ipld::Map(m)
}

fn kind_canonical(k: &EprKind) -> String {
    match k {
        EprKind::Content => "Content",
        EprKind::Agent => "Agent",
        EprKind::Manifest => "Manifest",
        EprKind::Claim => "Claim",
        EprKind::Observation => "Observation",
        EprKind::EconomicEvent => "EconomicEvent",
        EprKind::Commitment => "Commitment",
        EprKind::Attestation => "Attestation",
        EprKind::Delegation => "Delegation",
    }
    .into()
}

fn reach_canonical(r: &Reach) -> String {
    match r {
        Reach::Commons => "commons",
        Reach::Community => "community",
        Reach::Collective => "collective",
        Reach::Steward => "steward",
        Reach::Private => "private",
    }
    .into()
}

// Compatibility alias so users can access the same conversion in other modules.
pub use coupling_ipld as coupling_to_ipld;
```

Note: `libipld_core::cid::Cid as IpldCid` re-exports the same `cid::Cid` that the rest of the crate uses — `libipld_core` depends on `cid` directly. If there's a version mismatch, align `Cargo.toml`'s `cid` version to match `libipld-core`'s requirement.

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test canonical_bytes
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/envelope.rs elohim/epr/tests/canonical_bytes.rs
git commit -m "feat(epr): canonical envelope bytes for signing + CID

Implements spec §4.3/§4.4 — canonical CBOR encoding of the envelope
with alphabetically-ordered keys, excluding cid/proof/supersededBy.
Payload bytes included as CBOR byte-string. Enables deterministic
CID derivation and reproducible signatures across languages.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Ed25519 keypair + sign + verify (RFC 8032 vectors)

**Files:**
- Modify: `elohim/epr/src/proof.rs`
- Create: `elohim/epr/tests/sign_verify.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/sign_verify.rs`:

```rust
use elohim_epr::proof::{sign, verify, AgentKeypair};

#[test]
fn keypair_generates_different_keys() {
    let mut rng = rand::thread_rng();
    let kp1 = AgentKeypair::generate(&mut rng);
    let kp2 = AgentKeypair::generate(&mut rng);
    assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());
}

#[test]
fn sign_and_verify_roundtrip() {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let message = b"the quick brown fox";
    let sig = sign(&kp, message);
    assert!(verify(&kp.public_key_bytes(), message, &sig));
}

#[test]
fn verify_rejects_tampered_message() {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let sig = sign(&kp, b"original");
    assert!(!verify(&kp.public_key_bytes(), b"tampered", &sig));
}

#[test]
fn verify_rejects_wrong_key() {
    let mut rng = rand::thread_rng();
    let kp1 = AgentKeypair::generate(&mut rng);
    let kp2 = AgentKeypair::generate(&mut rng);
    let sig = sign(&kp1, b"message");
    assert!(!verify(&kp2.public_key_bytes(), b"message", &sig));
}

#[test]
fn rfc8032_test_vector_1() {
    // RFC 8032 §7.1 Test 1
    let secret_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    let public_hex = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let msg: &[u8] = b"";
    let expected_sig_hex = concat!(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155",
        "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
    );

    let secret = hex::decode(secret_hex).unwrap();
    let kp = AgentKeypair::from_secret(&secret).unwrap();
    assert_eq!(hex::encode(kp.public_key_bytes()), public_hex);
    let sig = sign(&kp, msg);
    assert_eq!(hex::encode(&sig), expected_sig_hex);
    assert!(verify(&kp.public_key_bytes(), msg, &sig));
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test sign_verify
```
Expected: fails (proof module is a stub).

- [ ] **Step 3: Implement proof**

Replace `elohim/epr/src/proof.rs`:

```rust
//! Ed25519 signing and verification.
//!
//! Conforms to RFC 8032. Uses ed25519-dalek's SigningKey/VerifyingKey.

use crate::error::{EprError, Result};
use ed25519_dalek::{Signature as EdSig, Signer, SigningKey, Verifier, VerifyingKey};
use rand::{CryptoRng, RngCore};

pub struct AgentKeypair(SigningKey);

impl AgentKeypair {
    pub fn generate<R: CryptoRng + RngCore>(rng: &mut R) -> Self {
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        AgentKeypair(SigningKey::from_bytes(&seed))
    }

    pub fn from_secret(secret: &[u8]) -> Result<Self> {
        if secret.len() != 32 {
            return Err(EprError::Signature(format!(
                "ed25519 secret must be 32 bytes, got {}",
                secret.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(secret);
        Ok(AgentKeypair(SigningKey::from_bytes(&arr)))
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }

    pub fn secret_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
}

pub fn sign(kp: &AgentKeypair, message: &[u8]) -> Vec<u8> {
    kp.0.sign(message).to_bytes().to_vec()
}

pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8]) -> bool {
    let Ok(vk) = VerifyingKey::from_bytes(public_key) else { return false; };
    let Ok(sig) = EdSig::from_slice(signature) else { return false; };
    vk.verify(message, &sig).is_ok()
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test sign_verify
```
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/proof.rs elohim/epr/tests/sign_verify.rs
git commit -m "feat(epr): Ed25519 sign/verify with RFC 8032 test vector

AgentKeypair wraps ed25519-dalek's SigningKey. sign/verify are free
functions. Verified against RFC 8032 §7.1 Test 1 (empty message) to
catch any implementation drift.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: Epr struct + builder + CID derivation

**Files:**
- Modify: `elohim/epr/src/epr.rs`
- Create: `elohim/epr/tests/epr_builder.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/epr_builder.rs`:

```rust
use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{
    cid::compute_cid,
    proof::AgentKeypair,
    Coupling, Epr, EprKind, Reach,
};

fn cid(b: u8) -> Cid { compute_cid(&[b]) }

#[test]
fn builder_produces_valid_epr() {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let agent_cid = cid(100);

    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(cid(1))
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(cid(2)),
            value: Some(cid(3)),
            governance: Some(cid(4)),
        })
        .claim(cid(5))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(b"hello world".to_vec())
        .sign(&kp, agent_cid.clone())
        .expect("sign");

    assert_eq!(epr.envelope.proof.signer, agent_cid);
    assert_eq!(epr.payload, b"hello world");
    // CID is populated
    assert!(!epr.envelope.cid.to_string().is_empty());
}

#[test]
fn builder_cid_is_stable_for_same_inputs() {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let agent_cid = cid(100);

    let mk = || {
        Epr::builder()
            .kind(EprKind::Content)
            .schema_ref(cid(1))
            .schema_key("concept")
            .reach(Reach::Commons)
            .coupling(Coupling {
                knowledge: Some(cid(2)),
                value: Some(cid(3)),
                governance: Some(cid(4)),
            })
            .claim(cid(5))
            .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
            .payload(b"hello world".to_vec())
            .sign(&kp, agent_cid.clone())
            .unwrap()
    };

    let a = mk();
    let b = mk();
    assert_eq!(a.envelope.cid, b.envelope.cid, "same inputs → same CID");
    // Signatures may differ (ed25519 is deterministic, so they should match)
    assert_eq!(a.envelope.proof.signature, b.envelope.proof.signature);
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test epr_builder
```
Expected: fails.

- [ ] **Step 3: Implement Epr + builder**

Replace `elohim/epr/src/epr.rs`:

```rust
//! Epr = Envelope + payload bytes.
//!
//! Construction flow: builder → canonical bytes → CID → sign → assemble.

use crate::{
    cid::compute_cid,
    envelope::Envelope,
    error::{EprError, Result},
    proof::{self, AgentKeypair},
    Coupling, EprKind, Reach, Signature,
};
use chrono::{DateTime, Utc};
use cid::Cid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Epr {
    pub envelope: Envelope,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl Epr {
    pub fn builder() -> EprBuilder {
        EprBuilder::default()
    }
}

#[derive(Default)]
pub struct EprBuilder {
    kind: Option<EprKind>,
    schema_ref: Option<Cid>,
    schema_key: Option<String>,
    reach: Option<Reach>,
    coupling: Coupling,
    claims: Vec<Cid>,
    supersedes: Option<Cid>,
    issued_at: Option<DateTime<Utc>>,
    payload: Vec<u8>,
}

impl EprBuilder {
    pub fn kind(mut self, k: EprKind) -> Self { self.kind = Some(k); self }
    pub fn schema_ref(mut self, c: Cid) -> Self { self.schema_ref = Some(c); self }
    pub fn schema_key<S: Into<String>>(mut self, s: S) -> Self { self.schema_key = Some(s.into()); self }
    pub fn reach(mut self, r: Reach) -> Self { self.reach = Some(r); self }
    pub fn coupling(mut self, c: Coupling) -> Self { self.coupling = c; self }
    pub fn claim(mut self, c: Cid) -> Self { self.claims.push(c); self }
    pub fn claims(mut self, c: Vec<Cid>) -> Self { self.claims = c; self }
    pub fn supersedes(mut self, c: Cid) -> Self { self.supersedes = Some(c); self }
    pub fn issued_at(mut self, t: DateTime<Utc>) -> Self { self.issued_at = Some(t); self }
    pub fn payload(mut self, p: Vec<u8>) -> Self { self.payload = p; self }

    /// Assemble canonical bytes, derive CID, sign, and return a complete Epr.
    pub fn sign(self, kp: &AgentKeypair, signer_cid: Cid) -> Result<Epr> {
        let kind = self.kind.ok_or_else(|| EprError::InvalidEnvelope("kind required".into()))?;
        let schema_ref = self.schema_ref.ok_or_else(|| EprError::InvalidEnvelope("schema_ref required".into()))?;
        let schema_key = self.schema_key.ok_or_else(|| EprError::InvalidEnvelope("schema_key required".into()))?;
        let reach = self.reach.ok_or_else(|| EprError::InvalidEnvelope("reach required".into()))?;
        let issued_at = self.issued_at.ok_or_else(|| EprError::InvalidEnvelope("issued_at required".into()))?;

        // Temporary envelope with placeholder cid + proof for canonical-bytes derivation
        let provisional = Envelope {
            cid: compute_cid(&[0]), // placeholder — excluded from canonical bytes
            kind,
            schema_ref,
            schema_key: schema_key.clone(),
            reach,
            coupling: self.coupling.clone(),
            claims: self.claims.clone(),
            supersedes: self.supersedes,
            superseded_by: None,
            issued_at,
            proof: Signature::ed25519(signer_cid.clone(), vec![0u8; 64]), // placeholder
        };

        let canonical = provisional.canonical_bytes(&self.payload)?;
        let cid = compute_cid(&canonical);
        let sig_bytes = proof::sign(kp, &canonical);

        let envelope = Envelope {
            cid,
            kind,
            schema_ref,
            schema_key,
            reach,
            coupling: self.coupling,
            claims: self.claims,
            supersedes: self.supersedes,
            superseded_by: None,
            issued_at,
            proof: Signature::ed25519_checked(signer_cid, sig_bytes)?,
        };

        Ok(Epr { envelope, payload: self.payload })
    }
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test epr_builder
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/epr.rs elohim/epr/tests/epr_builder.rs
git commit -m "feat(epr): Epr builder with canonical CID + sign flow

Builder collects envelope fields + payload, derives canonical bytes,
computes CID, signs, and returns a complete Epr. Ed25519 is
deterministic so same inputs → byte-identical EPR.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Epr verify flow

**Files:**
- Modify: `elohim/epr/src/epr.rs` (add `verify` + `verify_with_key`)
- Create: `elohim/epr/tests/epr_verify.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/epr_verify.rs`:

```rust
use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{
    cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach,
};

fn cid(b: u8) -> Cid { compute_cid(&[b]) }

fn make_valid() -> (AgentKeypair, Epr) {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let signer = cid(100);
    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(cid(1))
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(cid(2)),
            value: Some(cid(3)),
            governance: Some(cid(4)),
        })
        .claim(cid(5))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(b"hi".to_vec())
        .sign(&kp, signer)
        .unwrap();
    (kp, epr)
}

#[test]
fn verify_with_correct_key_passes() {
    let (kp, epr) = make_valid();
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_ok());
}

#[test]
fn verify_with_wrong_key_fails() {
    let (_, epr) = make_valid();
    let mut rng = rand::thread_rng();
    let other = AgentKeypair::generate(&mut rng);
    assert!(epr.verify_with_key(&other.public_key_bytes()).is_err());
}

#[test]
fn verify_fails_on_tampered_payload() {
    let (kp, mut epr) = make_valid();
    epr.payload = b"tampered".to_vec();
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_err());
}

#[test]
fn verify_fails_on_tampered_envelope() {
    let (kp, mut epr) = make_valid();
    epr.envelope.schema_key = "lesson".into();
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_err());
}

#[test]
fn verify_checks_cid_match() {
    let (kp, mut epr) = make_valid();
    epr.envelope.cid = cid(255);
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_err());
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test epr_verify
```
Expected: fails.

- [ ] **Step 3: Implement verify**

Append to `elohim/epr/src/epr.rs`:

```rust
impl Epr {
    /// Verify this Epr against a public key.
    ///
    /// Checks (in order):
    /// 1. Re-derive canonical bytes from envelope + payload
    /// 2. CID matches canonical-bytes hash
    /// 3. Proof algorithm is "ed25519"
    /// 4. Signature bytes verify under the given public key
    pub fn verify_with_key(&self, public_key: &[u8; 32]) -> Result<()> {
        let canonical = self.envelope.canonical_bytes(&self.payload)?;
        let expected_cid = compute_cid(&canonical);
        if expected_cid != self.envelope.cid {
            return Err(EprError::InvalidEnvelope(
                "cid does not match canonical bytes".into(),
            ));
        }
        if self.envelope.proof.algorithm != "ed25519" {
            return Err(EprError::Signature(format!(
                "unsupported algorithm: {}",
                self.envelope.proof.algorithm
            )));
        }
        if !proof::verify(public_key, &canonical, &self.envelope.proof.signature) {
            return Err(EprError::Signature("signature verification failed".into()));
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test epr_verify
```
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/epr.rs elohim/epr/tests/epr_verify.rs
git commit -m "feat(epr): Epr.verify_with_key — proof + CID consistency check

Three-stage verify: re-derive canonical bytes, match CID, then verify
Ed25519 signature. Tampered payload, tampered envelope field, or
mismatched CID all fail verification.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 13: Structural validator (coupling requirements)

**Files:**
- Modify: `elohim/epr/src/validation.rs`
- Create: `elohim/epr/tests/structural_validation.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/epr/tests/structural_validation.rs`:

```rust
use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{
    cid::compute_cid, proof::AgentKeypair, validation::validate_coupling,
    Coupling, Epr, EprKind, Reach,
};

fn cid(b: u8) -> Cid { compute_cid(&[b]) }

fn build(kind: EprKind, coupling: Coupling) -> Epr {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    Epr::builder()
        .kind(kind)
        .schema_ref(cid(1))
        .schema_key("x")
        .reach(Reach::Commons)
        .coupling(coupling)
        .claim(cid(5))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(vec![])
        .sign(&kp, cid(100))
        .unwrap()
}

#[test]
fn content_requires_all_three_legs() {
    let missing = Coupling { knowledge: Some(cid(2)), value: Some(cid(3)), governance: None };
    let epr = build(EprKind::Content, missing);
    assert!(validate_coupling(&epr.envelope).is_err());

    let ok = Coupling {
        knowledge: Some(cid(2)),
        value: Some(cid(3)),
        governance: Some(cid(4)),
    };
    let epr = build(EprKind::Content, ok);
    assert!(validate_coupling(&epr.envelope).is_ok());
}

#[test]
fn economic_event_requires_value_only() {
    let value_only = Coupling {
        knowledge: None,
        value: Some(cid(3)),
        governance: None,
    };
    let epr = build(EprKind::EconomicEvent, value_only);
    assert!(validate_coupling(&epr.envelope).is_ok());

    let missing = Coupling::default();
    let epr = build(EprKind::EconomicEvent, missing);
    assert!(validate_coupling(&epr.envelope).is_err());
}

#[test]
fn manifest_requires_governance_only() {
    let gov_only = Coupling {
        knowledge: None,
        value: None,
        governance: Some(cid(4)),
    };
    let epr = build(EprKind::Manifest, gov_only);
    assert!(validate_coupling(&epr.envelope).is_ok());
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cargo test -p elohim-epr --test structural_validation
```
Expected: fails.

- [ ] **Step 3: Implement validator**

Replace `elohim/epr/src/validation.rs`:

```rust
//! Structural validation — coupling requirement enforcement (spec §7 stage 3).
//!
//! Payload schema validation (stage 4) requires the manifest resolver and is
//! deferred to Phase 2.

use crate::{envelope::Envelope, error::{EprError, Result}};

pub fn validate_coupling(env: &Envelope) -> Result<()> {
    for leg in env.kind.required_coupling() {
        if !env.coupling.has(*leg) {
            return Err(EprError::Coupling(format!(
                "kind {:?} requires {:?} coupling leg",
                env.kind, leg
            )));
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p elohim-epr --test structural_validation
```
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/validation.rs elohim/epr/tests/structural_validation.rs
git commit -m "feat(epr): structural coupling validator

Implements spec §7 stage 3 — enforces that each EPR kind has its
required coupling legs populated. Payload schema validation deferred
to Phase 2 (needs manifest resolver).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 14: Test vector generator binary

**Files:**
- Create: `elohim/epr/examples/gen_vectors.rs`
- Create: `elohim/epr/tests/vectors/` (directory, will be populated by the binary)

- [ ] **Step 1: Write the generator**

Create `elohim/epr/examples/gen_vectors.rs`:

```rust
//! Generates shared test vectors for Rust ↔ TS interop verification.
//!
//! Run: cargo run -p elohim-epr --example gen_vectors
//! Writes JSON files to elohim/epr/tests/vectors/.

use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{
    cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach,
};
use serde::Serialize;
use std::{fs, path::PathBuf};

fn cid(b: u8) -> Cid { compute_cid(&[b]) }

#[derive(Serialize)]
struct KeypairVector {
    seed_hex: String,
    secret_hex: String,
    public_hex: String,
}

#[derive(Serialize)]
struct EprVector {
    label: String,
    envelope: serde_json::Value,
    payload_hex: String,
    canonical_bytes_hex: String,
    cid: String,
    signature_hex: String,
    public_key_hex: String,
}

fn main() {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors");
    fs::create_dir_all(&out_dir).expect("create vectors dir");

    // Deterministic seed for reproducibility
    let seed = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60,
        0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
        0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19,
        0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
    ];
    let kp = AgentKeypair::from_secret(&seed).unwrap();
    let keypair_vec = KeypairVector {
        seed_hex: hex::encode(seed),
        secret_hex: hex::encode(kp.secret_bytes()),
        public_hex: hex::encode(kp.public_key_bytes()),
    };
    fs::write(
        out_dir.join("keypairs.json"),
        serde_json::to_string_pretty(&keypair_vec).unwrap(),
    ).unwrap();

    // Representative EPR vectors, one per kind that makes sense in isolation
    let agent_cid = cid(100);
    let vectors = vec![
        ("content_all_legs", EprKind::Content, Coupling {
            knowledge: Some(cid(2)),
            value: Some(cid(3)),
            governance: Some(cid(4)),
        }, b"hello world".to_vec()),
        ("manifest_gov_only", EprKind::Manifest, Coupling {
            knowledge: None, value: None, governance: Some(cid(4)),
        }, b"{}".to_vec()),
        ("economic_event_value", EprKind::EconomicEvent, Coupling {
            knowledge: None, value: Some(cid(3)), governance: None,
        }, br#"{"action":"use"}"#.to_vec()),
    ];

    let issued_at = Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap();
    let epr_vectors: Vec<EprVector> = vectors.into_iter().map(|(label, kind, coupling, payload)| {
        let epr = Epr::builder()
            .kind(kind)
            .schema_ref(cid(1))
            .schema_key("test")
            .reach(Reach::Commons)
            .coupling(coupling)
            .claim(cid(5))
            .issued_at(issued_at)
            .payload(payload.clone())
            .sign(&kp, agent_cid.clone())
            .unwrap();

        let canonical = epr.envelope.canonical_bytes(&payload).unwrap();

        EprVector {
            label: label.into(),
            envelope: serde_json::to_value(&epr.envelope).unwrap(),
            payload_hex: hex::encode(&payload),
            canonical_bytes_hex: hex::encode(&canonical),
            cid: epr.envelope.cid.to_string(),
            signature_hex: hex::encode(&epr.envelope.proof.signature),
            public_key_hex: hex::encode(kp.public_key_bytes()),
        }
    }).collect();

    fs::write(
        out_dir.join("signed_eprs.json"),
        serde_json::to_string_pretty(&epr_vectors).unwrap(),
    ).unwrap();

    println!("wrote vectors to {}", out_dir.display());
}
```

- [ ] **Step 2: Run the generator**

```bash
cd /projects/elohim/elohim && cargo run -p elohim-epr --example gen_vectors
```
Expected: prints "wrote vectors to …" and creates two JSON files.

- [ ] **Step 3: Inspect output**

```bash
ls /projects/elohim/elohim/epr/tests/vectors/
head -50 /projects/elohim/elohim/epr/tests/vectors/signed_eprs.json
```
Expected: `keypairs.json` and `signed_eprs.json` exist; `signed_eprs.json` shows three labeled EPR vectors.

- [ ] **Step 4: Add a round-trip test that consumes the vectors**

Create `elohim/epr/tests/vector_roundtrip.rs`:

```rust
use elohim_epr::Epr;
use std::fs;
use std::path::PathBuf;

#[test]
fn all_vectors_verify_under_their_public_key() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/signed_eprs.json");
    let raw = fs::read_to_string(&path)
        .expect("run: cargo run -p elohim-epr --example gen_vectors first");
    let vectors: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap();

    for v in &vectors {
        let envelope = serde_json::from_value(v["envelope"].clone()).unwrap();
        let payload_hex = v["payload_hex"].as_str().unwrap();
        let payload = hex::decode(payload_hex).unwrap();
        let pk_hex = v["public_key_hex"].as_str().unwrap();
        let pk_bytes = hex::decode(pk_hex).unwrap();
        let mut pk = [0u8; 32];
        pk.copy_from_slice(&pk_bytes);

        let epr = Epr { envelope, payload };
        epr.verify_with_key(&pk).expect("vector must verify");
    }
}
```

- [ ] **Step 5: Run test — verify passes**

```bash
cargo test -p elohim-epr --test vector_roundtrip
```
Expected: 1 test passes.

- [ ] **Step 6: Commit**

```bash
git add elohim/epr/examples/gen_vectors.rs elohim/epr/tests/vectors elohim/epr/tests/vector_roundtrip.rs
git commit -m "feat(epr): test vector generator + round-trip verifier

examples/gen_vectors.rs produces deterministic keypair + envelope +
signature fixtures at tests/vectors/. Committed so TS can verify
against the same artifacts. vector_roundtrip test confirms Rust
can re-verify its own output.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 15: Export TypeScript types via ts-rs

**Files:**
- Modify: `elohim/epr/Cargo.toml` (add export test)
- Create: `elohim/epr/tests/export_bindings.rs`

- [ ] **Step 1: Write the export test**

Create `elohim/epr/tests/export_bindings.rs`:

```rust
//! Triggers ts-rs to emit TypeScript bindings into
//! elohim/sdk/epr-ts/src/generated/ via the #[ts(export_to)] attributes.

use elohim_epr::{Coupling, Envelope, EprKind, Reach, Signature};
use elohim_epr::kind::CouplingLeg;
use ts_rs::TS;

#[test]
fn export_bindings() {
    Coupling::export().unwrap();
    Envelope::export().unwrap();
    EprKind::export().unwrap();
    Reach::export().unwrap();
    Signature::export().unwrap();
    CouplingLeg::export().unwrap();
}
```

- [ ] **Step 2: Run the export test**

```bash
mkdir -p /projects/elohim/elohim/sdk/epr-ts/src/generated
cd /projects/elohim/elohim && cargo test -p elohim-epr --test export_bindings
```
Expected: test passes; `elohim/sdk/epr-ts/src/generated/` contains `.ts` files for each type.

- [ ] **Step 3: Inspect generated files**

```bash
ls /projects/elohim/elohim/sdk/epr-ts/src/generated/
```
Expected: `Coupling.ts`, `Envelope.ts`, `EprKind.ts`, `Reach.ts`, `Signature.ts`, `CouplingLeg.ts`.

- [ ] **Step 4: Commit**

```bash
git add elohim/epr/tests/export_bindings.rs elohim/sdk/epr-ts/src/generated
git commit -m "feat(epr): ts-rs export of wire types

Generates TypeScript interfaces in elohim/sdk/epr-ts/src/generated/
for Coupling, Envelope, EprKind, Reach, Signature, CouplingLeg.
Matches the existing storage-client-ts pattern (cargo test triggers
codegen; generated files committed).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 16: TypeScript package scaffolding

**Files:**
- Create: `elohim/sdk/epr-ts/package.json`
- Create: `elohim/sdk/epr-ts/tsconfig.json`
- Create: `elohim/sdk/epr-ts/vitest.config.ts`
- Create: `elohim/sdk/epr-ts/src/index.ts`
- Modify: repo-root `pnpm-workspace.yaml` (if needed)

- [ ] **Step 1: Create package.json**

```bash
cat > /projects/elohim/elohim/sdk/epr-ts/package.json <<'EOF'
{
  "name": "@elohim/epr",
  "version": "0.1.0",
  "description": "TypeScript port of the elohim-epr canonical codec",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.js"
    }
  },
  "scripts": {
    "build": "tsc",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "dependencies": {
    "@ipld/dag-cbor": "^9.2.0",
    "@noble/ed25519": "^2.1.0",
    "@noble/hashes": "^1.5.0",
    "multiformats": "^13.3.0"
  },
  "devDependencies": {
    "@types/node": "^22.10.0",
    "typescript": "^5.6.0",
    "vitest": "^3.0.0"
  },
  "license": "CAL-1.0"
}
EOF
```

- [ ] **Step 2: Create tsconfig.json**

```bash
cat > /projects/elohim/elohim/sdk/epr-ts/tsconfig.json <<'EOF'
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "lib": ["ES2022", "DOM"],
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "types": ["node"]
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests"]
}
EOF
```

- [ ] **Step 3: Create vitest.config.ts**

```bash
cat > /projects/elohim/elohim/sdk/epr-ts/vitest.config.ts <<'EOF'
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['tests/**/*.test.ts'],
  },
});
EOF
```

- [ ] **Step 4: Create stub index.ts**

```bash
cat > /projects/elohim/elohim/sdk/epr-ts/src/index.ts <<'EOF'
// Public API (filled in by subsequent tasks)
export * from './generated';
EOF

cat > /projects/elohim/elohim/sdk/epr-ts/src/generated/index.ts <<'EOF'
export * from './Coupling';
export * from './Envelope';
export * from './EprKind';
export * from './Reach';
export * from './Signature';
export * from './CouplingLeg';
EOF
```

- [ ] **Step 5: Confirm pnpm workspace picks it up**

```bash
cat /projects/elohim/pnpm-workspace.yaml
```

If `elohim/sdk/**` is already in the packages glob, no change needed. Otherwise add it:

```yaml
packages:
  - 'app/**'
  - 'elohim/sdk/**'
  - 'genesis/**'
```

Then install:

```bash
cd /projects/elohim && pnpm install
```
Expected: `@elohim/epr` appears as a workspace package.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/epr-ts pnpm-workspace.yaml pnpm-lock.yaml
git commit -m "feat(epr-ts): scaffold @elohim/epr TypeScript package

package.json, tsconfig, vitest config, and src/index.ts barrel over
ts-rs generated types. Deps: @ipld/dag-cbor, multiformats,
@noble/ed25519, @noble/hashes. Ready for canonical CBOR, CID, and
verify implementations.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 17: TS canonical CBOR wrapper

**Files:**
- Create: `elohim/sdk/epr-ts/src/cbor.ts`
- Create: `elohim/sdk/epr-ts/tests/cbor.test.ts`

- [ ] **Step 1: Write failing test**

```ts
// elohim/sdk/epr-ts/tests/cbor.test.ts
import { describe, expect, it } from 'vitest';
import { encodeCanonical, decodeCanonical } from '../src/cbor';

describe('canonical CBOR', () => {
  it('roundtrips primitives', () => {
    for (const v of [null, true, false, 0, 42, -7, 'hello', new Uint8Array([1, 2, 3])]) {
      const bytes = encodeCanonical(v);
      const decoded = decodeCanonical(bytes);
      expect(decoded).toEqual(v);
    }
  });

  it('produces deterministic bytes regardless of map insertion order', () => {
    const a = encodeCanonical({ b: 2, a: 1 });
    const b = encodeCanonical({ a: 1, b: 2 });
    expect(a).toEqual(b);
  });
});
```

- [ ] **Step 2: Run test — verify failure**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: fails (module not found).

- [ ] **Step 3: Implement cbor.ts**

```ts
// elohim/sdk/epr-ts/src/cbor.ts
import * as dagCbor from '@ipld/dag-cbor';

/** Encode any JSON-compatible value to canonical dag-cbor bytes. */
export function encodeCanonical(value: unknown): Uint8Array {
  return dagCbor.encode(value);
}

/** Decode canonical dag-cbor bytes to a JS value. */
export function decodeCanonical<T = unknown>(bytes: Uint8Array): T {
  return dagCbor.decode(bytes) as T;
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/epr-ts/src/cbor.ts elohim/sdk/epr-ts/tests/cbor.test.ts
git commit -m "feat(epr-ts): canonical CBOR via @ipld/dag-cbor

Same determinism guarantees as the Rust side (libipld-cbor). Thin
wrapper so consumers import from one place.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 18: TS CID derivation

**Files:**
- Create: `elohim/sdk/epr-ts/src/cid.ts`
- Create: `elohim/sdk/epr-ts/tests/cid.test.ts`

- [ ] **Step 1: Write failing test**

```ts
// elohim/sdk/epr-ts/tests/cid.test.ts
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { computeCid } from '../src/cid';
import { encodeCanonical } from '../src/cbor';

describe('CID derivation', () => {
  it('stable across calls', async () => {
    const bytes = new Uint8Array([1, 2, 3, 4]);
    expect((await computeCid(bytes)).toString()).toBe((await computeCid(bytes)).toString());
  });

  it('matches Rust vectors', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const raw = readFileSync(path, 'utf-8');
    const vectors = JSON.parse(raw);

    for (const v of vectors) {
      const canonical = new Uint8Array(Buffer.from(v.canonical_bytes_hex, 'hex'));
      const cid = await computeCid(canonical);
      expect(cid.toString()).toBe(v.cid);
    }
  });
});
```

- [ ] **Step 2: Run test — verify failure**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: fails.

- [ ] **Step 3: Implement cid.ts**

```ts
// elohim/sdk/epr-ts/src/cid.ts
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

/** CIDv1 codec byte for dag-cbor per the IPLD multicodec table. */
const DAG_CBOR_CODEC = 0x71;

/** Compute CIDv1(dag-cbor, sha2-256) over canonical bytes. */
export async function computeCid(canonicalBytes: Uint8Array): Promise<CID> {
  const hash = await sha256.digest(canonicalBytes);
  return CID.create(1, DAG_CBOR_CODEC, hash);
}

/** Verify a claimed CID matches the hash of the given canonical bytes. */
export async function verifyCid(claimed: CID, canonicalBytes: Uint8Array): Promise<boolean> {
  const derived = await computeCid(canonicalBytes);
  return derived.toString() === claimed.toString();
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: 2 tests pass (vector test proves Rust↔TS CID parity).

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/epr-ts/src/cid.ts elohim/sdk/epr-ts/tests/cid.test.ts
git commit -m "feat(epr-ts): CIDv1 dag-cbor sha256 matches Rust vectors

Parity with Rust's elohim-epr::cid::compute_cid proven by consuming
signed_eprs.json test vectors. Same canonical bytes produce
byte-identical CIDs across languages.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 19: TS Ed25519 verify

**Files:**
- Create: `elohim/sdk/epr-ts/src/proof.ts`
- Create: `elohim/sdk/epr-ts/tests/proof.test.ts`

- [ ] **Step 1: Write failing test**

```ts
// elohim/sdk/epr-ts/tests/proof.test.ts
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { verifyEd25519 } from '../src/proof';

function hexToBytes(hex: string): Uint8Array {
  return Uint8Array.from(Buffer.from(hex, 'hex'));
}

describe('Ed25519 verify', () => {
  it('RFC 8032 test 1', async () => {
    const pk = hexToBytes('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a');
    const msg = new Uint8Array();
    const sig = hexToBytes(
      'e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b'
    );
    expect(await verifyEd25519(pk, msg, sig)).toBe(true);
  });

  it('verifies Rust-generated vectors', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));

    for (const v of vectors) {
      const pk = hexToBytes(v.public_key_hex);
      const canonical = hexToBytes(v.canonical_bytes_hex);
      const sig = hexToBytes(v.signature_hex);
      expect(await verifyEd25519(pk, canonical, sig)).toBe(true);
    }
  });

  it('rejects wrong signature', async () => {
    const pk = hexToBytes('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a');
    const msg = new Uint8Array([1, 2, 3]);
    const badSig = new Uint8Array(64);
    expect(await verifyEd25519(pk, msg, badSig)).toBe(false);
  });
});
```

- [ ] **Step 2: Run test — verify failure**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: fails.

- [ ] **Step 3: Implement proof.ts**

```ts
// elohim/sdk/epr-ts/src/proof.ts
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';

// @noble/ed25519 v2 requires setting the sha-512 hash provider
ed.etc.sha512Sync = (...m: Uint8Array[]) => sha512(ed.etc.concatBytes(...m));

/** Verify an Ed25519 signature. */
export async function verifyEd25519(
  publicKey: Uint8Array,
  message: Uint8Array,
  signature: Uint8Array,
): Promise<boolean> {
  if (publicKey.length !== 32 || signature.length !== 64) return false;
  try {
    return await ed.verifyAsync(signature, message, publicKey);
  } catch {
    return false;
  }
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/epr-ts/src/proof.ts elohim/sdk/epr-ts/tests/proof.test.ts
git commit -m "feat(epr-ts): Ed25519 verify via @noble/ed25519

RFC 8032 vector + Rust-generated vectors both verify. Proves that
Rust signatures over canonical bytes are verifiable from TypeScript
using the same public key bytes.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 20: TS canonical envelope bytes + Epr verify

**Files:**
- Create: `elohim/sdk/epr-ts/src/envelope.ts`
- Create: `elohim/sdk/epr-ts/src/epr.ts`
- Create: `elohim/sdk/epr-ts/tests/envelope.test.ts`

- [ ] **Step 1: Write failing test**

```ts
// elohim/sdk/epr-ts/tests/envelope.test.ts
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { canonicalEnvelopeBytes } from '../src/envelope';
import type { Envelope } from '../src/generated/Envelope';
import { CID } from 'multiformats/cid';

function hexToBytes(hex: string): Uint8Array {
  return Uint8Array.from(Buffer.from(hex, 'hex'));
}

describe('canonical envelope bytes', () => {
  it('matches Rust canonical_bytes_hex for every vector', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));

    for (const v of vectors) {
      const env = reviveEnvelope(v.envelope);
      const payload = hexToBytes(v.payload_hex);
      const derived = await canonicalEnvelopeBytes(env, payload);
      expect(Buffer.from(derived).toString('hex')).toBe(v.canonical_bytes_hex);
    }
  });
});

function reviveEnvelope(raw: any): Envelope {
  // CID strings → CID objects; serde_json emits ts-rs Envelope in camelCase
  return {
    ...raw,
    cid: raw.cid,          // remains string per the TS Envelope type
    schemaRef: raw.schemaRef,
    claims: raw.claims,
    supersedes: raw.supersedes ?? null,
    supersededBy: raw.supersededBy ?? null,
    coupling: raw.coupling,
  } as Envelope;
}
```

- [ ] **Step 2: Run test — verify failure**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: fails.

- [ ] **Step 3: Implement envelope.ts**

```ts
// elohim/sdk/epr-ts/src/envelope.ts
import { CID } from 'multiformats/cid';
import { encodeCanonical } from './cbor';
import type { Envelope } from './generated/Envelope';
import type { Coupling } from './generated/Coupling';

/**
 * Compute the canonical bytes that get hashed for CID and signed for proof.
 * Mirrors Rust: excludes cid, proof, supersededBy.
 * Map keys must be alphabetically ordered: claims, coupling, issuedAt, kind,
 * payload, reach, schemaKey, schemaRef, supersedes.
 */
export async function canonicalEnvelopeBytes(env: Envelope, payload: Uint8Array): Promise<Uint8Array> {
  const map: Record<string, unknown> = {
    claims: env.claims.map((c) => CID.parse(c as unknown as string)),
    coupling: couplingToMap(env.coupling),
    issuedAt: env.issuedAt,
    kind: env.kind,
    payload,
    reach: env.reach,
    schemaKey: env.schemaKey,
    schemaRef: CID.parse(env.schemaRef as unknown as string),
  };
  if (env.supersedes) {
    map.supersedes = CID.parse(env.supersedes as unknown as string);
  }
  return encodeCanonical(map);
}

function couplingToMap(c: Coupling): Record<string, unknown> {
  const m: Record<string, unknown> = {};
  if (c.knowledge) m.knowledge = CID.parse(c.knowledge as unknown as string);
  if (c.value) m.value = CID.parse(c.value as unknown as string);
  if (c.governance) m.governance = CID.parse(c.governance as unknown as string);
  return m;
}
```

- [ ] **Step 4: Implement epr.ts**

```ts
// elohim/sdk/epr-ts/src/epr.ts
import type { Envelope } from './generated/Envelope';
import { canonicalEnvelopeBytes } from './envelope';
import { computeCid } from './cid';
import { verifyEd25519 } from './proof';
import { CID } from 'multiformats/cid';

export interface Epr {
  envelope: Envelope;
  payload: Uint8Array;
}

export interface VerifyError {
  kind: 'cid-mismatch' | 'algorithm-unsupported' | 'signature-invalid';
  message: string;
}

/** Verify an Epr against the signer's public key. */
export async function verifyEpr(epr: Epr, publicKey: Uint8Array): Promise<{ ok: true } | { ok: false; error: VerifyError }> {
  const canonical = await canonicalEnvelopeBytes(epr.envelope, epr.payload);
  const derived = await computeCid(canonical);
  const claimed = CID.parse(epr.envelope.cid as unknown as string);
  if (derived.toString() !== claimed.toString()) {
    return { ok: false, error: { kind: 'cid-mismatch', message: 'cid does not match canonical bytes' } };
  }
  if (epr.envelope.proof.algorithm !== 'ed25519') {
    return { ok: false, error: { kind: 'algorithm-unsupported', message: `unsupported algorithm: ${epr.envelope.proof.algorithm}` } };
  }
  const sigBytes = epr.envelope.proof.signature instanceof Uint8Array
    ? epr.envelope.proof.signature
    : new Uint8Array(Object.values(epr.envelope.proof.signature));
  const ok = await verifyEd25519(publicKey, canonical, sigBytes);
  return ok ? { ok: true } : { ok: false, error: { kind: 'signature-invalid', message: 'signature verification failed' } };
}
```

- [ ] **Step 5: Run test — verify passes**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: all tests pass (the canonical-bytes test proves cross-lang parity).

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/epr-ts/src/envelope.ts elohim/sdk/epr-ts/src/epr.ts elohim/sdk/epr-ts/tests/envelope.test.ts
git commit -m "feat(epr-ts): canonical envelope bytes + verifyEpr

canonicalEnvelopeBytes matches Rust byte-for-byte on every committed
vector. verifyEpr re-derives CID, checks algorithm, and verifies
Ed25519 — the TS side of the verify gate used at §7 stages 1–2.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 21: TS interop smoke test

**Files:**
- Create: `elohim/sdk/epr-ts/tests/interop.test.ts`

- [ ] **Step 1: Write the full end-to-end interop test**

```ts
// elohim/sdk/epr-ts/tests/interop.test.ts
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { verifyEpr, type Epr } from '../src/epr';
import type { Envelope } from '../src/generated/Envelope';

function hexToBytes(hex: string): Uint8Array {
  return Uint8Array.from(Buffer.from(hex, 'hex'));
}

describe('Rust ↔ TS interop', () => {
  it('verifies every signed Rust vector', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));

    for (const v of vectors) {
      const envelope = v.envelope as Envelope;
      const payload = hexToBytes(v.payload_hex);
      const publicKey = hexToBytes(v.public_key_hex);
      const epr: Epr = { envelope, payload };

      const result = await verifyEpr(epr, publicKey);
      if (!result.ok) {
        throw new Error(`vector "${v.label}" failed: ${result.error.message}`);
      }
    }
  });

  it('rejects tampered payload', async () => {
    const path = join(process.cwd(), '../../epr/tests/vectors/signed_eprs.json');
    const vectors = JSON.parse(readFileSync(path, 'utf-8'));
    const v = vectors[0];

    const envelope = v.envelope as Envelope;
    const tampered = hexToBytes(v.payload_hex);
    if (tampered.length > 0) tampered[0] = tampered[0] ^ 0xff;
    const publicKey = hexToBytes(v.public_key_hex);

    const result = await verifyEpr({ envelope, payload: tampered }, publicKey);
    expect(result.ok).toBe(false);
  });
});
```

- [ ] **Step 2: Run test**

```bash
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: all tests pass; the first fully end-to-end Rust → TS verification.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/epr-ts/tests/interop.test.ts
git commit -m "test(epr-ts): end-to-end interop — verify every Rust vector from TS

Seals the cross-language contract: every Rust-signed EPR vector
verifies via the TS verifyEpr, and tampering the payload causes
verification to fail. This is the bi-directional canonical-bytes +
CID + Ed25519 parity guarantee the Phase 1 spec requires.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 22: CI integration

**Files:**
- Modify: `.husky/pre-push` (existing pre-push hook)
- Modify: relevant `Jenkinsfile` (steward or a new epr stage)

- [ ] **Step 1: Inspect the existing pre-push hook**

```bash
cat /projects/elohim/.husky/pre-push
```

Identify where per-project quality gates are registered.

- [ ] **Step 2: Add an elohim-epr gate**

Append a detection + gate block modeled after the existing pattern. Rough template (adapt exactly to the hook's existing structure):

```sh
# elohim-epr crate + epr-ts package gate
if git diff --cached --name-only HEAD | grep -qE '^elohim/epr/|^elohim/sdk/epr-ts/'; then
  echo "→ elohim-epr changes detected; running gate"
  (cd elohim && cargo test -p elohim-epr --all-targets) || exit 1
  (cd elohim/sdk/epr-ts && pnpm test) || exit 1
fi
```

- [ ] **Step 3: Add an elohim-epr CI stage**

Pick the right Jenkinsfile. This crate is not tied to a specific existing pipeline; safest is to add a dedicated pipeline entry in `genesis/orchestrator/Jenkinsfile`'s PIPELINES map under a new key `epr`, plus a minimal `elohim/epr/Jenkinsfile` running:

```groovy
pipeline {
  agent { label 'rust-node' }
  stages {
    stage('Rust tests') {
      steps {
        dir('elohim') { sh 'cargo test -p elohim-epr --all-targets' }
      }
    }
    stage('TS tests') {
      steps {
        dir('elohim/sdk/epr-ts') {
          sh 'pnpm install --frozen-lockfile && pnpm test'
        }
      }
    }
  }
}
```

Add `epr/**` and `sdk/epr-ts/**` to the orchestrator's changeset detection for the new pipeline.

- [ ] **Step 4: Verify locally**

```bash
cd /projects/elohim/elohim && cargo test -p elohim-epr --all-targets
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: both complete cleanly.

- [ ] **Step 5: Commit**

```bash
git add .husky/pre-push elohim/epr/Jenkinsfile genesis/orchestrator/Jenkinsfile
git commit -m "ci(epr): pre-push + dedicated pipeline for elohim-epr

Adds local gate (.husky/pre-push) that runs both Rust and TS tests
when elohim/epr/ or elohim/sdk/epr-ts/ files are staged. Adds a
dedicated Jenkins pipeline via the orchestrator.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 23: Public API consolidation + README

**Files:**
- Modify: `elohim/epr/src/lib.rs` (expand re-exports)
- Modify: `elohim/sdk/epr-ts/src/index.ts` (expand re-exports)
- Modify: `elohim/epr/README.md` (usage examples)

- [ ] **Step 1: Finalize Rust public API**

Replace `elohim/epr/src/lib.rs`:

```rust
//! elohim-epr — canonical codec for the Elohim EPR atom.
//!
//! See `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.
//!
//! # Example
//!
//! ```no_run
//! use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
//! use chrono::Utc;
//!
//! let mut rng = rand::thread_rng();
//! let kp = AgentKeypair::generate(&mut rng);
//! let agent_cid = compute_cid(&[100]);
//!
//! let epr = Epr::builder()
//!     .kind(EprKind::Manifest)
//!     .schema_ref(compute_cid(&[1]))
//!     .schema_key("app-manifest")
//!     .reach(Reach::Commons)
//!     .coupling(Coupling { governance: Some(compute_cid(&[4])), ..Default::default() })
//!     .issued_at(Utc::now())
//!     .payload(b"{}".to_vec())
//!     .sign(&kp, agent_cid)
//!     .unwrap();
//!
//! assert!(epr.verify_with_key(&kp.public_key_bytes()).is_ok());
//! ```

pub mod cbor;
pub mod cid;
pub mod coupling;
pub mod envelope;
pub mod epr;
pub mod error;
pub mod kind;
pub mod proof;
pub mod reach;
pub mod signature;
pub mod validation;

pub use coupling::Coupling;
pub use envelope::Envelope;
pub use epr::{Epr, EprBuilder};
pub use error::{EprError, Result};
pub use kind::{CouplingLeg, EprKind};
pub use proof::{sign, verify, AgentKeypair};
pub use reach::Reach;
pub use signature::Signature;
pub use validation::validate_coupling;
```

- [ ] **Step 2: Finalize TS public API**

Replace `elohim/sdk/epr-ts/src/index.ts`:

```ts
// Canonical codec
export { encodeCanonical, decodeCanonical } from './cbor';
export { computeCid, verifyCid } from './cid';
export { verifyEd25519 } from './proof';
export { canonicalEnvelopeBytes } from './envelope';
export { verifyEpr, type Epr, type VerifyError } from './epr';

// Generated wire types
export type { Coupling } from './generated/Coupling';
export type { Envelope } from './generated/Envelope';
export type { EprKind } from './generated/EprKind';
export type { Reach } from './generated/Reach';
export type { Signature } from './generated/Signature';
export type { CouplingLeg } from './generated/CouplingLeg';
```

- [ ] **Step 3: Expand README**

Replace `elohim/epr/README.md`:

```markdown
# elohim-epr

Canonical codec for the Elohim EPR (EntityPortalReference) atom defined in
`genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.

## What this crate provides

- **Canonical CBOR** (dag-cbor, RFC 8949 §4.2.1 deterministic encoding)
- **CIDv1** derivation (codec 0x71, multihash sha2-256)
- **Ed25519** signing and verification
- **Envelope** struct (kind, schemaRef, schemaKey, reach, coupling, claims, supersedes, proof)
- **Epr** = Envelope + payload bytes, with builder + sign + verify
- **Structural validator** — coupling requirement enforcement per EprKind

## What this crate does NOT provide (Phase 2+)

- Persistence / storage
- Payload schema validation (requires Manifest resolver)
- GraphQL surface
- Subscriptions or federation

## Cross-language interop

The companion TypeScript package `@elohim/epr` (at `elohim/sdk/epr-ts/`) is a parallel
implementation verified against shared test vectors at `tests/vectors/`.
Regenerate vectors with:

```bash
cargo run -p elohim-epr --example gen_vectors
```

Regenerating vectors after any logic change catches Rust↔TS drift immediately;
both sides' CI runs all vectors through full verify.

## Usage (Rust)

```rust
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use chrono::Utc;

let mut rng = rand::thread_rng();
let kp = AgentKeypair::generate(&mut rng);
let agent_cid = compute_cid(&[100]);

let epr = Epr::builder()
    .kind(EprKind::Content)
    .schema_ref(manifest_cid)
    .schema_key("concept")
    .reach(Reach::Commons)
    .coupling(Coupling { knowledge: Some(k), value: Some(v), governance: Some(g) })
    .issued_at(Utc::now())
    .payload(payload_bytes)
    .sign(&kp, agent_cid)
    .unwrap();

assert!(epr.verify_with_key(&kp.public_key_bytes()).is_ok());
elohim_epr::validate_coupling(&epr.envelope).unwrap();
```

## Usage (TypeScript)

```ts
import { verifyEpr } from '@elohim/epr';

const result = await verifyEpr(epr, publicKey);
if (!result.ok) throw new Error(result.error.message);
```
```

- [ ] **Step 4: Verify everything still builds and tests**

```bash
cd /projects/elohim/elohim && cargo test -p elohim-epr --all-targets
cd /projects/elohim/elohim/sdk/epr-ts && pnpm test
```
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add elohim/epr/src/lib.rs elohim/epr/README.md elohim/sdk/epr-ts/src/index.ts
git commit -m "docs(epr): finalize public API + README

Consolidates re-exports on both Rust and TS sides. README covers
scope, out-of-scope (Phase 2+), regenerating test vectors, and
usage examples. Phase 1 of the elohim-core graph substrate spec is
feature-complete.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Self-review

**Spec coverage (§4 + §7 stages 1–3):**

| Spec requirement | Covered by |
|---|---|
| EPR envelope shape (§4.1) | Task 8 |
| EPR kinds table + required coupling (§4.2) | Task 5 |
| Canonical serialization (§4.3) | Tasks 2, 9 |
| CID derivation (§4.4) | Task 3 |
| Proof (Ed25519) (§4.5) | Task 10 |
| `supersededBy` as derived field (§4.6) | Task 9 (canonical_bytes excludes it) |
| Manifests as EPRs (§5.1) | Wire-level enabled (EprKind::Manifest in Task 5); full manifest graph is Phase 2 |
| Stage 1 — canonicalization check (§7) | Task 12 (verify_with_key re-derives + compares CID) |
| Stage 2 — signature verification (§7) | Task 12 |
| Stage 3 — coupling check (§7) | Task 13 |
| Stage 4 — payload schema check (§7) | **Explicitly Phase 2** (noted in validation.rs) |
| Storage layout (§8) | **Phase 2** |
| GraphQL surface (§9) | **Phase 4+** |
| Agent hooks (§10) | **Phase 2+** |
| Reference subgraphs (§11) | **Phase 5** |

**Placeholder scan:** none. Every step has concrete code.

**Type consistency check:**
- `Coupling` — used consistently with `knowledge`/`value`/`governance` Option<Cid> fields across Tasks 6, 8, 9, 11, 13, 14, 20
- `EprKind` — same nine variants across Tasks 5, 11, 13, 14, 20
- `Signature` — `signer: Cid`, `algorithm: String`, `signature: Vec<u8>` across Tasks 7, 8, 11, 12
- `Envelope.canonical_bytes(&self, payload: &[u8])` — consistent signature across Tasks 9, 11, 12, 20
- `verify_with_key(&self, public_key: &[u8; 32])` — consistent signature across Tasks 12, 14
- `verifyEpr(epr, publicKey)` — consistent TS signature across Tasks 20, 21

**Scope check:** plan covers Phase 1 only. Phases 2–7 of the spec are explicitly deferred. Each task in this plan produces a green test run independently; the full chain is valuable even if no further phase ships.

**Open-questions handling (from spec §16):**
- (1) Crate location → resolved: `elohim/epr/` + `elohim/sdk/epr-ts/`
- (2) VF manifest publication → N/A for Phase 1 (deferred to Phase 5)
- (3) GraphQL endpoint hosting → N/A for Phase 1
- (4) Subscription key format → N/A for Phase 1
- (5) Gate declaration schema → N/A for Phase 1

---

## Execution handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-04-21-elohim-epr-codec-crate-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration. Each task is self-contained and bite-sized, which is the shape subagent-driven execution handles best.

**2. Inline Execution** — execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints for review.

Which approach?
