# bridges/did — Local Guidance

The W3C **DID 1.1** identity bridge for the Elohim Protocol. A standards-legible
identity surface — one resolvable DID document per agent — assembled as a
*projection of substrate truth, never truth itself* (P1). Consumed later by
`doorway-service` (web2 face: universal-resolver route + the doorway's own
`did:web` document) and by `elohim-storage` (which implements the
`did:elohim` assembly contract).

## Workspace structure

- `did-types/` — the DID 1.1 data model. No I/O, no async. Standalone so tooling
  can depend on the identity wire shapes without the resolver stack.
- `did-bridge/` — the `DidResolver` trait (IoC seam), `MethodRegistry`, and the
  `did:key` / `did:elohim` / `did:web` method impls + the agent-key codec.
- `did-tests/` — integration/conformance tests (registry routing, error
  metadata, did:key vectors, did:elohim assembly against a mock store).

## What phase 1 is

- **Read-only projection.** No new DHT entry types; no coordinator/signal. The
  DID document is class-C operational, assembled per request.
- **`did:key`** — offline. `AgentPubKey` (`uhCAk…`, 3-byte prefix `0x842024` +
  32-byte ed25519 core + 4-byte DHT loc) ↔ `did:key:z6Mk…` (multicodec `0xed01`
  + base58btc of the core). The reverse path recomputes the loc bytes exactly as
  `holo_hash` (blake2b-128, XOR-folded 16→4) — cross-checked against the
  `holo_hash` reference implementation's own test vectors, so the round-trip is
  byte-identical. `blake2b_simd` is used
  directly rather than depending on `holo_hash` (keeps the bridge off the
  Holochain tree and avoids a duplicate-`holo_hash` type conflict when
  `elohim-storage` later consumes this crate).
- **`did:elohim:<agent_cid>`** — self-certifying. `ElohimResolver` assembles the
  document per spec §3.4: Multikey from the agent key; `authentication` +
  `assertionMethod` reference it; transport ids (libp2p PeerId, iroh NodeId) as
  `alsoKnownAs`; profile + doorway `service` entries. `ElohimIdentityStore` is
  the contract `elohim-storage` implements; this crate ships the trait + a
  mock-store contract test. Controller is implicit (subject controls) in
  phase 1 — explicit self + community-recovery-quorum controllers arrive with
  the phase-2 identity head.
- **`did:web`** — feature-gated (`web-resolver`, off by default). URL derivation
  + resolution over an injected `DidWebFetch` trait; no HTTP client in the
  common build. **Security (SSRF):** `derive_did_web_url` rejects (never
  sanitizes) any segment that, after percent-decoding, carries an
  authority/path-confusion character (`@` `/` `\` `?` `#` or control) — this
  closes the `%40`-userinfo host-confusion class. It does **not** make the
  resolved host safe: `DidWebFetch` implementations MUST additionally validate
  the resolved URL's actual host against their egress policy (block loopback /
  link-local / private / cloud-metadata ranges) before connecting. The crate
  owns confusion-rejection; the fetcher owns network policy.

## What phase 1 is NOT (named follow-ons, deliberately deferred)

- No DHT identity head / agent-key lineage (phase 2 — routes the full
  p2p-design-gate before design).
- No `did:plc` (arrives with the atproto bridge, plugging `DidResolver`).
- No doorway routes yet (distinct write-set: `GET /1.0/identifiers/{did}` and
  `GET /.well-known/did.json`, each needing a match arm **+** `is_service_path`
  **+** a unit test — the `/auth/portal` shadow trap).

## DID version

Target is DID **1.1**. Assembled/resolved documents emit
`https://www.w3.org/ns/did/v1.1` as the base `@context` (`DID_CONTEXT_V1_1`).
Deserialization accepts the legacy 1.0 context (`DID_CONTEXT_V1`) and preserves
whatever came in on round-trip (wire fidelity — never silently upgrade).

## Build / test

Native build — override the ambient WASM getrandom flag and use a pooled target
dir (the disk-guard hook denies native cargo without `CARGO_TARGET_DIR`):

```bash
cd bridges/did
export RUSTFLAGS=""
export CARGO_TARGET_DIR="$(cargo-pool key | sed -n 's/^slot_dev=//p')"
cargo build
cargo test          # cargo nextest run where nextest is installed
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

**Conformance gate:** wire shapes are checked against the hand-derived W3C DID
1.1 schema (`schemas/did-document-1.1.schema.json`) by
`did-tests/tests/did11_conformance.rs` — every document the crate emits, plus
the fidelity fixtures, must validate; run it directly with
`cargo test -p did-tests --test did11_conformance`. The co-located `.epr-meta`
injects this reminder when you edit a DID wire shape.

## Reference docs

- Spec: `genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md`
- Bridge pattern: `bridges/CLAUDE.md`
