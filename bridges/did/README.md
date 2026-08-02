# bridges/did — W3C DID 1.1 identity bridge

A bridge crate giving the Elohim Protocol a **standards-legible identity
surface**: W3C [DID 1.1](https://www.w3.org/TR/did-1.1/). A DID document is a
*projection of substrate truth, never truth itself* (P1) — assembled per
request, never stored.

DID-the-mechanism is adopted; SSI-the-ideology is not. The document model
(subject ≠ controller, multiple controllers, Group Control) maps onto the
imago-dei ontology; no surface frames "self-sovereign" as an apex tier.

## Crates

- **`did-types`** — the DID 1.1 data model (`DidDocument`, `VerificationMethod`,
  the five verification relationships, `Service`, `Controller`, `alsoKnownAs`).
  Wire-faithful serde, no I/O.
- **`did-bridge`** — the `DidResolver` trait (the IoC seam), a `MethodRegistry`,
  and method impls: `did:key` (offline codec), `did:elohim` (projection-assembly
  contract), `did:web` (feature-gated `web-resolver`).
- **`did-tests`** — registry routing, error-metadata semantics, did:key vectors
  (real Holochain agent keys, incl. holo_hash's reference test vector, plus a
  published-pubkey vector), did:elohim assembly.

## Methods

| Method | Resolution | Notes |
|--------|-----------|-------|
| `did:key` | offline | every `AgentPubKey` gets a DID for free via the codec |
| `did:elohim:<agent_cid>` | assembled | from an `ElohimIdentityStore` (implemented by elohim-storage) |
| `did:web` | fetched | feature `web-resolver`; caller injects a `DidWebFetch` |

`did:key` and `did:elohim` are self-certifying — the document is derived from the
key, so its subject cannot be asserted at us. `did:web` is not: the document
arrives from a host, so the resolver re-derives the document's `id` against the
requested DID and refuses a mismatch (`SubjectMismatch` → `invalidDidDocument`).
A revoked `did:elohim` identity resolves to a **deactivated** document
(`didDocumentMetadata.deactivated = true`) carrying no key, service or transport
material; an identity head the store cannot determine fails resolution closed
rather than degrading to an implicitly self-controlled document.

## Usage

```rust
use did_bridge::{DidKeyResolver, MethodRegistry};
use did_types::Did;

let registry = MethodRegistry::new().with(Box::new(DidKeyResolver::new()));
let did = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr")?;
let result = registry.resolve(&did).await?;
let document = result.did_document.expect("resolved");
```

The `did:key` codec is also usable standalone — `AgentPubKey` (`uhCAk…`) ↔
`did:key` (`did:key:z6Mk…`), with the 4 DHT location bytes recomputed on the
reverse path exactly as `holo_hash` (blake2b-128 XOR-fold), cross-checked
against the `holo_hash` reference implementation's own test vectors.

## Build / test

This is its own Cargo workspace (native build — **not** WASM). Override the
ambient WASM getrandom flag and use a pooled target dir:

```bash
cd bridges/did
export RUSTFLAGS=""                                   # ambient sets a WASM flag that breaks native
export CARGO_TARGET_DIR="$(cargo-pool key | sed -n 's/^slot_dev=//p')"
cargo build
cargo test                    # cargo nextest run where nextest is installed
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Reference

- Spec: `genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md`
- Bridge pattern: `bridges/CLAUDE.md`
