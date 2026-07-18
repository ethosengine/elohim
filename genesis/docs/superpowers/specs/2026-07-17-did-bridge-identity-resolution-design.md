---
title: "DID Bridge — W3C DID 1.1 as elohim's identity projection standard (bridges/did)"
id: did-bridge-identity-resolution
tier: spec
status: Draft
created: 2026-07-17
maintainers: Matthew Dowell + Claude Fable 5
class: protocol-canonical
topic: [did, identity, resolution, bridge, interop, did-key, did-elohim, did-web, atproto, resolver, alsoKnownAs, lineage]
context-tier: disclosed
sovereignty-frame: descriptive
steward: rust-architect
graduation-trigger: decompose-complete OR bridges/did phase-1 DoD green
domain: D2
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
cites:
  - wave3-valueflows-hrea-interop-design | the canonical bridge-crate pattern (own workspace, types/bridge/tests split, runtime-consumes-library) this crate COMPOSES from | sha256:c8d903ad73f0284d | path: genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md
  - context-governed-binding | the resolver shape DidResolver instantiates — interface=anchor, binding=context-scoped, resolution=negotiated — making DID resolution an instance of the protocol pattern, not bespoke | sha256:92bdc62351d683c0 | path: genesis/docs/superpowers/specs/2026-07-12-context-governed-binding-design.md
  - coherent-transport-identity-resolver-design | the falsified prior design this DESIGNS AROUND — its agent_cid/transport-id namespace problem is re-homed as alsoKnownAs in the assembled DID document instead of an AgentPeerBinding entity | sha256:63117b359cfa3891 | path: genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md
  - frame-witness-primitive-architecture | the witnessed-primitive architecture the phase-2 identity head will need when self-asserted binding graduates to witnessed binding | sha256:9acf41622029875e | path: genesis/docs/superpowers/specs/2026-07-15-frame-witness-primitive-architecture-design.md
  - epr-resolution-provider-design | the doorway resolution-contract prior art the universal-resolver route composes with (head-first previews, typed degradation, manifest-declared route claims) | sha256:bc1f1cbcae739c4a | path: genesis/docs/superpowers/specs/2026-07-02-epr-resolution-provider-design.md
  - genesis/data/timeline/backlog/eprfs-ipfs-analog-dataplane-sdk-surface.md
---

# DID Bridge — W3C DID 1.1 as elohim's identity projection standard

## 1. Purpose and operator direction

Give the protocol a **standards-legible identity surface** — W3C DID 1.1 — as a
bridge crate, valued even though identity is not DID-native here: for peer-facing
clarity ("this is how you resolve one of us"), interop quality, a foundation for
the atproto bridge (which must speak `did:plc`/`did:web` regardless), and
IoC-style consistency in how identity is handled and resolved. The DID document
becomes the one assembled, resolvable identity artifact the substrate currently
lacks — a **projection of substrate truth, never truth itself** (P1).

DID-the-mechanism is adopted; SSI-the-ideology is not. DID 1.1 structurally
supports subject ≠ controller, multiple controllers, and Group Control — the
imago-dei ontology (community/guardian backstop, graduated mediated agency) maps
onto the spec's own data model. The p2p-design-gate's identity-ontology guard
applies to every naming choice in this design: no surface may frame
"self-sovereign" as the apex tier.

## 2. Prior art — Dyne.org W3C-DID (did:dyne), borrowed and inverted

Design cues only — AGPL-3.0 code, Node/RESTroom runtime; nothing is consumed.

| Dyne feature | Verdict | Rationale |
|---|---|---|
| Governance domains in the identifier (`did:dyne:<domain>` with `.A` admin keyrings) | Borrow concept, **invert polarity** | Theirs: registrar-approved primary identity (admin grants the DID). Ours: primary DID is self-certifying (key-derived, granted by no one); collective-governed **alias** namespaces come later as `alsoKnownAs`, encoding community context without registrar capture. |
| No keys on server; client-side signing | Already ours | Conductor holds keys; doorway never sees them. |
| Universal-resolver / universal-registrar endpoint contracts | Borrow directly | One standard route and the whole DID ecosystem (incl. atproto tooling) can resolve us. |
| Git-as-registry, filesystem storage | Reject | The DHT with witnessed validation is a strictly stronger verifiable data registry. |
| "Share DIDs p2p using IPFS" | **Analog, deferred** | Our analog is the p2p dataplane as an IPFS-like SDK surface (working name: eprfs byte-plane). Captured separately — see cites: eprfs-ipfs-analog backlog item. DID documents are small JSON projections; when the dataplane SDK surface exists, they ride it like any content. |

## 3. Design

### 3.1 Crate layout — `bridges/did/` (own workspace, per bridges/CLAUDE.md)

```
bridges/did/
  Cargo.toml          # workspace: resolver=2, Apache-2.0, mirrors valueflows
  did-types/          # DID 1.1 data model: DidDocument, VerificationMethod (Multikey ed25519),
                      #   the five verification relationships, Service, alsoKnownAs, controller.
                      #   serde camelCase wire-faithful. No I/O.
  did-bridge/         # DidResolver trait + method impls: did:key (offline), did:elohim
                      #   (projection assembly), did:web (fetch, feature-gated).
                      #   Doorway/storage mount points.
  did-tests/          # spec-conformance fixtures (W3C test vectors where available),
                      #   codec round-trips, resolver contract tests.
```

### 3.2 The `DidResolver` trait — the IoC seam

One trait; DID methods plug in as implementations. Deliberately the same
resolver shape as context-governed binding (interface = anchor, binding =
context-scoped, resolution = negotiated): identity resolution becomes an
instance of the protocol's own resolution pattern, not a bespoke path. The
future atproto bridge plugs `did:plc` into this trait instead of inventing
its own resolution.

```rust
#[async_trait]
pub trait DidResolver {
    fn method(&self) -> &'static str;               // "key" | "elohim" | "web" | ...
    async fn resolve(&self, did: &Did) -> Result<DidResolutionResult, DidResolutionError>;
}
```

`DidResolutionResult` carries the document + resolution metadata per DID 1.1
(`didResolutionMetadata`, `didDocumentMetadata`) so error semantics
(`notFound`, `invalidDid`, `methodNotSupported`) are standard, not invented.

### 3.3 `did:key` codec — every agent standards-legible for free

`AgentPubKey` ↔ `did:key` is mechanical: a holo agent hash is 39 bytes =
3-byte prefix (`0x84 0x20 0x24`) + 32-byte ed25519 core + 4-byte DHT loc.
The codec extracts the 32-byte core and emits
`did:key:z…` (multicodec `0xed01` + base58btc, per the did:key method spec);
the reverse re-derives the full holo hash (loc bytes recomputed). Round-trip
property-tested. No substrate change; the whole fleet gains DIDs on day one.

### 3.4 `did:elohim:<agent_cid>` — self-certifying method, projection-assembled

- **Syntax:** method-specific-id IS the agent_cid (`uhCAk…`). Create is free and
  decentralized — the gate's identity rule (content/key-derived, no registrar).
- **Resolution assembles, never stores** (P1): verification method from the
  agent key (Multikey); `authentication`/`assertionMethod` reference it;
  profile service entry from the humans projection when present; doorway
  endpoints as `service` entries; **transport ids (libp2p PeerId, iroh NodeId)
  as `alsoKnownAs`**. That last line gives the agent_cid ↔ transport-id
  namespace mismatch (the falsified transport-identity-resolver's problem) a
  standards-shaped home: one resolution surface names all of an agent's
  identifiers, instead of a bespoke `AgentPeerBinding` entity.
- Assembly runs in elohim-storage (it owns the joins); the crate defines the
  assembly contract so storage conforms rather than invents.

### 3.4a Method naming — why `did:elohim` (settled 2026-07-17, operator-affirmed)

A DID method names the **verifiable data registry you resolve against** — the network,
not the subject's domain and not a resolution authority (`did:plc` = PLC, `did:sov` =
Sovrin). Ours is the elohim protocol substrate, so the method is `did:elohim`.
Alternatives considered and rejected:

- **`did:imagodei`** — wrong layer twice: the agent key is substrate-level (one key
  signs across every DNA, not imagodei-scoped), and not every DID subject is imago dei
  (service agents, nodes, collectives) — stamping the human ontology on non-human
  subjects would dilute the very framing the ontology guard protects. The pillar's
  ownership lives where it belongs: the humans projection feeds assembly, and the
  phase-2 identity head homes in the imagodei DNA.
- **`did:epr`** — wrong plane: EPR is the record/content grammar (data plane); agent
  identity is the control plane. A general "resolve any EPR head" method would misuse
  the DID standard for content resolution (that is CID/eprfs territory) and re-conflate
  the agent_cid/transport-id/content-CID namespaces. Externally, EPR reads as
  Electronic Patient Record — a hostile collision for a W3C method registration.
- **`did:epi`** — an acronym no external peer can expand tells them nothing at the
  moment of resolution, defeating the peer-facing-clarity goal; collision-prone.

The method name inherits the network's already-public name (elohim.host, the Elohim
Protocol) — a deliberate consistency the operator affirmed. Settled now because the
cost asymmetry is maximal: today a rename is one constant; after external peers
resolve us it is a migration with an `alsoKnownAs` compatibility tail.

### 3.5 Doorway surfaces (web2 face; consumes did-bridge like a web2 bridge)

- `GET /1.0/identifiers/{did}` — universal-resolver-compatible resolution.
- `GET /.well-known/did.json` — the doorway's own `did:web` document (this is
  the atproto foundation: atproto handles resolve via did:web/did:plc).
- Both are new 8080 routes: match arm **+** `is_service_path` **+** unit test
  (the `/auth/portal` shadow trap).

## 4. p2p-design-gate accounting

1. **Entity class:** phase 1 introduces **no new DHT entry types**. The DID
   document is class-C operational **projection** (assembled per-request);
   `did:key`/`did:elohim` identifiers are content-derived from the agent key.
2. **Existing entry types:** none consumed beyond what resolution reads
   (agent keys, humans projection, doorway registrations).
3. **Identity:** content-derived (key-derived) — no slugs, no UUIDs.
4. **Coordinator/signal:** none in phase 1 (read-only projection). The phase-2
   identity head (below) creates DHT state and MUST route the full gate before
   design.

## 5. Named follow-ons (deferred deliberately, not scope)

- **Identity head + agent-key lineage** — the deep open question: a
  DHT-registered identity head whose controllers are self + community-recovery
  quorum (DID Group Control semantics), rotation-as-update, lineage as a DAG of
  heads. The assembled DID document of §3.4 is the dry-run spec of what that
  head must contain; the method upgrades in place (documents gain real
  `controller` entries and update history) without breaking `did:elohim`.
  Feeds roadmap rung 2 (Grandma recovery = community-backstopped continuity).
- **`did:plc`** — arrives with the atproto bridge, plugging `DidResolver`.
- **eprfs dataplane DID sharing** — the IPFS-analog SDK surface (cited backlog
  item); DID documents ride it as ordinary content once it exists.

## 6. Definition of done (phase 1)

- `bridges/did` workspace: `cargo build && cargo nextest run && cargo clippy
  -- -D warnings && cargo fmt --check` green (RUSTFLAGS="", pooled target dir).
- `did:key` codec round-trips real fleet agent keys (fixture from alpha).
- `DidResolver` contract tests: did:key resolves offline; did:elohim assembly
  contract satisfied by a mock store; standard error metadata on unknown
  method/did.
- Doorway routes land as a separate leg once the crate is green (distinct
  write-set), with `is_service_path` unit test.
