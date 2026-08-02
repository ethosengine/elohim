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
  mock-store contract test. **The identity head is a four-way answer**
  (`IdentityHeadAnswer`), not an `Option`: `Declared` populates explicit
  `controller` entries (self / steward-set / community-recovery quorum — DID 1.1
  Group Control) plus a chain-root lineage alias; `NeverDeclared` assembles the
  implicit-self document (the phase-1 shape, unchanged, and the default for a
  store that never implements the method); `Revoked` assembles a **deactivated**
  document — `didDocumentMetadata.deactivated = true`, no key material, no
  relationships, no services, no transport ids, lineage aliases only;
  `Unresolvable` **fails closed** (no document). Implementor rule: `NeverDeclared`
  is a positive claim that you looked and found nothing — a full-arc local `get`
  miss is `Unresolvable`, because it is a fact about gossip, not about existence.
  A `Declared` head with an EMPTY controller set is refused, not emitted:
  an absent `controller` reads as implicit self-control, which is not what an
  empty declaration says.
- **`did:web`** — feature-gated (`web-resolver`, off by default). URL derivation
  + resolution over an injected `DidWebFetch` trait; no HTTP client in the
  common build. **Security (the response is evidence, not authority):** this is
  the one method here that is not self-certifying — the document is handed over
  by a host, and a successful fetch establishes only that the host answered. So
  the parsed document's own `id` is re-derived against the requested DID
  (`verify_resolved_subject`) and a mismatch is a typed refusal
  (`SubjectMismatch` → `invalidDidDocument`), never a pass-through; otherwise any
  host that can answer for one domain can return another domain's document and
  have it reported as a successful resolution of *that* DID. The comparison is
  byte-exact because this crate normalizes nothing — if normalization is ever
  added it must run on BOTH sides through one function, or the test measures its
  own mirror. **Security (SSRF):** `derive_did_web_url` rejects (never
  sanitizes) any segment that, after percent-decoding, carries an
  authority/path-confusion character (`@` `/` `\` `?` `#` or control) — this
  closes the `%40`-userinfo host-confusion class. It does **not** make the
  resolved host safe: `DidWebFetch` implementations MUST additionally validate
  the resolved URL's actual host against their egress policy (block loopback /
  link-local / private / cloud-metadata ranges) before connecting. The crate
  owns confusion-rejection; the fetcher owns network policy.

## Decision-point registry (seam-concern architecture)

`seam-registry.yaml` at this workspace root is the **source of truth** for this
bridge's decision surface — every pure decision predicate, verdict fn, boundary
answer type, the concern classes (C0–C14) each answers, and the contract tests
that pin it. It conforms to
`elohim/sdk/schemas/v1/manifest/seam-registry.schema.json`; the decision-point
census (`python3 .claude/scripts/memory-kit/placement-audit.py --epr-meta`) and
the concern × seam matrix are derived read-models over it, never hand-authored.

**Birth rule:** a NEW predicate, verdict fn, or boundary answer type here is born
registered — add its row in the same change that adds the symbol, cite a real
passing test, and record `unbound`/`partial`/`n-a` honestly rather than claiming
coverage the code does not have.

## What phase 1 is NOT (named follow-ons, deliberately deferred)

- No DHT *writes* for identity lineage. The bridge READS the head through
  `ElohimIdentityStore::identity_head` and projects it; minting, rotating and
  revoking heads belong to the Wave-B `binds-identity` declaration in the
  substrate, not here.
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
