---
title: "The Record Proves — views resolve from verified bytes where the person is; the model composes, the record proves"
id: record-proves-model-composes-design
tier: spec
status: Draft
created: 2026-09-26
maintainers: Matthew Dowell + Claude Opus 5.5
license: CC-BY-4.0
class: protocol-canonical
context-tier: disclosed
steward: angular-architect
graduation-trigger: the record-proves check (client-resilience.feature "Cached app works offline", @concern:record-proves, bound on the doorway-failover habit) passes twice on the household mesh — then the claim earns its Stance, its seam-map row and its own habit (§9)
topic:
  - content-addressing
  - local-first
  - service-worker
  - elected-heads
  - local-inference
  - transclusion
informed-by:
  - app/elohim-app/src/apps-sw.ts
  - app/elohim-app/src/app/elohim/utils/raw-cid-verify.ts
  - elohim/elohim-storage/src/blob_store.rs
  - elohim/elohim-storage/src/p2p/blob_fetch.rs
  - elohim/brit/brit-epr/src/engine/meta.rs
  - elohim/elohim-facings/src/lib.rs
cites:
  - "runtime-artifacts-elected-content | §12: deploy = declare a head — the election this spec makes the only trust point | sha256:eaa2716381075140 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - genesis/plans/2026-03-30-resilient-html5-app-delivery-design.md
  - "elohim-seam-map-concern-routing | the atlas this claim graduates into: a composer / local-inference row, reader never authority (§9) | sha256:7ea7563016174974 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "values-forward | the values canon a Stance joins at graduation; Stance V.1 (never claim designed as done) bounds this spec | sha256:f4e7524522d3b811 | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "native-delivery-sprint-plan | Lane N6: the household proof that a new app build reaches every peer by election | sha256:8a08fcde0dec6869 | path: genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md"
---

# The Record Proves

> Licensed CC-BY-4.0: anyone may implement, copy or adapt this specification with attribution.
> The repository's code licenses are a separate, owed decision (§10).

## §0 — The claim

**Nothing a person sees depends on a server being up, or being honest — only on the record.**

Three parts, each of which must hold on its own:

1. **Names are elected.** Which version of a thing is current is decided by the head election
   the conductor validates in wasm, never by whichever host was last written to.
2. **Bytes prove themselves.** Every byte a client keeps is checked against its content address
   by the client itself. A server — doorway, peer, gateway — is a disk, not an authority.
3. **Views are composed where the person is.** The client holds verified bytes and the last
   version it verified, so a view can be composed from them with no server reachable at all.

And one consequence for machine readers:

4. **The model composes; the record proves.** A model — including cheap local inference on the
   person's own device — may read the record and compose views and answers from it. It is never
   the record's authority. Every claim it presents resolves to a span of the record, and what it
   adds of its own is marked as its own. The protocol works fully with no model present.

## §1 — Why this is being staked (measured 2026-09-26)

- Two federation doorways served two different versions of the same page for days, each
  answering 200. Each doorway's head moved only when a deploy wrote to that host directly
  (`genesis/a2o/features/dataplane/federation-version-convergence.feature`).
- The app pipeline spent about twelve pipeline-hours across six runs and delivered zero bundles,
  waiting for doorways to be ready to be written to (dataplane-convergence habit, DELTA 2026-09-24).
- The apps service worker exists so that a small peer which cannot afford to extract and serve
  an app can hand over raw bytes and let the client do the work. It cached those bytes — from
  peers and from the doorway — without checking them against any address.

Each is the same defect: the view depended on a server's state or honesty rather than on the record.

## §2 — The resolver chain: theory against what is built

The chain a client walks to show a person something:

`name → elected head → address of the bytes → bytes, from anyone → verified → cached → composed`

| Link | Theory | Built (2026-09-26) | Evidence |
|---|---|---|---|
| Name → elected head | Conductor-validated election; deploy = declare a head | Built for content. App bundles: native-delivery Lane N N1–N5; household proof N6 open | elected-content spec §12; `app-bundle-elected-delivery.feature` |
| Head → address of the bytes | The client reads the address from the elected head | **Not built.** The service worker takes the address from the doorway's `X-Blob-Hash` header | `apps-sw.ts` `probeCapability` |
| Bytes, from anyone | Any peer or gateway may serve; small peers serve raw blocks | Built for whole blobs (`get_blob_or_heal`, race fetch across libp2p and iroh); per-file peer delivery exists | `p2p/blob_fetch.rs`; `DeliveryPeer.serves_extracted` |
| Verified | The client hashes before it keeps | Storage verifies every received blob. The service worker verifies the whole archive (`verifyRawSha256`) before caching; single files cannot be verified yet and are served but not cached | `raw-cid-verify.ts`; `apps-sw.ts` `extractZip` |
| Cached | Only verified bytes, keyed by address; the last verified version remembered | Verified-only cache (`apps-v3`); remembering the last verified version is the next slice | `apps-sw.ts` |
| Composed | Views are folds of the record bound to stateless elements | Folds are pure and addressable (`elohim-facings`); the binding from fold to element lives inside Angular code in an opaque bundle | `elohim-facings/src/lib.rs`; `app/elohim-elements` |

**The honest trust point today is the doorway header in row two, not the election.** The bytes a
client caches are proven; the claim that they are the *current* version is only as good as the
doorway that named them. Closing row two — the client resolving the address from the elected head
(`/epr-head/{id}` already serves a dag-cbor envelope whose CID every agreeing peer mints
identically) — is what makes the election the only trust point.

## §3 — Standards at every boundary; novelty only where it is ours

Where bytes cross a boundary, this protocol uses the standards and invents nothing:

- **Leaves:** CIDv1, raw codec, sha2-256 (`bafkrei…`). SHA-256 is native to WebCrypto, so any
  browser verifies without a library. BLAKE3 stays a peer-to-peer transport detail of iroh.
- **Nodes:** IPLD (dag-cbor already names `/epr-head` envelopes and brit's `EprMeta`).
- **Streams:** CARv1 — the verifiable replacement for "ship the whole zip".
- **Serving:** the IPFS trustless-gateway contract — the server returns raw blocks or a CAR and
  the client verifies. This is the small-peer design stated as a standard.

Novelty is spent on what no standard provides: **governed names** (the head election), **reach
and standing** (who may be served what), and **facings** (audience-shaped folds of the record).

## §4 — Amendment: the record is truth; the archive is a projection

The 2026-03-30 delivery design stated: "The ZIP blob is truth." This spec amends it: **the record
is truth; the archive is one content-verified projection of it.** Nothing that consumes the zip
changes today. What changes is the direction of derivation once per-file addresses exist (§7): a
tree of per-file addresses is the version, and the zip is produced from it for the consumers that
want one archive.

## §5 — The model composes, the record proves

Cheap local inference changes who does the structuring. The web chose unstructured documents
because structure — spans, links, versions, attribution — was the author's unpaid labour. A model
on the person's own device can propose that structure for them. The rules that keep it honest:

- **Grounding.** Every claim a generated view presents resolves to a span of the record:
  `(leaf CID, byte range)`, verified by hashing the leaf.
- **Marked glue.** What the model contributes of its own is marked as the model's.
- **Zero-model floor.** Every surface works with no model present (a watch, an L0 device).
  Inference improves the edge; it never carries truth.
- **Never the authority.** A model's output is a proposal. The record, and the election over it,
  decide.
- **Attribution.** When a generated answer carries its transclusions, REA can attribute value
  back to the authors it drew on.

**Prior art.** Ted Nelson's Xanadu asked for permanent span addresses, two-way links, every
version kept, transclusion and royalties. Content addressing supplies the addresses; DHT links
supply the backlinks; elected heads name versions over permanent bytes; REA supplies attribution.
Its designs stalled on coordination and on authoring labour, and its stewardship kept the design
closed behind marks and franchises. This protocol takes the design lessons, not the enclosure:
hence the open licence on this document.

## §6 — Known holes, named

- **Doorway-named versions** (§2 row two): a replayed or stale release is one header away.
- **Service-worker update lifecycle:** `skipWaiting` + `clients.claim` means whoever serves the
  shell replaces the verifier; the verifier's own bytes are not verified by anything.
- **`/ipfs/{cid}` on the storage port has no reach gate:** it is an existence oracle for any CID.
  The doorway does not proxy it; storage ports must not be public until it is gated.
- **An address is not access control.** Anyone holding a CID can ask for it; reach is enforced by
  who agrees to serve. Private content is encrypted before hashing, or a guessable CID confirms it.
- **Single files are unverifiable** until per-file addresses exist; they are served, never cached.

## §7 — Slices, in order

1. Deterministic `version.json` (`buildTime` from the commit) — one build, one address. *(this pass)*
2. The service worker verifies the archive before caching; single files are served uncached;
   caches older than `apps-v3` are dropped. *(this pass)*
3. The client remembers the last version it verified, so a view resolves offline; the
   record-proves check goes runnable.
4. **Per-file addresses:** publish a tree root over per-file CIDs alongside the zip. Owed decision,
   leaning to brit `EprMeta` as the authoritative tree (one format for source and build output,
   and the source-to-file chain of slice 10), with UnixFS generated from it as a projection for
   IPFS gateways and verified-fetch path resolution.
5. `/ipfs/{cid}`: trustless-gateway raw responses, reach gate (`blob_serve_verdict`), heal via
   `get_blob_or_heal`.
6. CARv1 responses and a doorway proxy for `/ipfs/`.
7. The client's verified fetch pointed at the household's own peers as trustless gateways.
8. The client resolves the address from the elected head, not a doorway header.
9. **The protocol's own artifacts live on the protocol — the rakia MVP.** A project whose claim is
   "the record proves, not the server" must not take its own bytes on a registry's word. Today:

   | Artifact | External origin today |
   |---|---|
   | Conductor and storage images | Harbor (`scripts/ci/push-harbor-tracked.sh`, `build-storage-image.sh`) |
   | Deployed hApp bundle for a joining workspace | Harbor via oras, Jenkins-artifact fallback (`fetch-deployed-dna.sh`) |
   | cache-core WASM | Harbor (`app/elohim-app/scripts/build-wasm.sh`) |
   | brit crates | Nexus, behind a credential |
   | Submodule pin attestation | the upstream forge's CI check (`gate-attest.mjs`) |

   The move is the one Lane N makes for app bundles: CI publishes each artifact as a release on a
   channel (bytes to the blob plane by CID, a rakia release manifest, a staging declaration), and
   every consumer resolves the channel's head and fetches by CID from any peer, verifying before
   use. Registries remain as mirrors for tools that need them; the origin is the household's own
   dataplane. rakia adds no store: it is the manifest that gives blobs meaning.

   **MVP: the hApp bundle.** The `happ-bundle` class already exists in rakia's release-manifest
   schema. Done when a fresh workspace joining the network obtains the deployed hApp by resolving
   its channel's head and fetching the bundle by CID from a household peer, verified, with no
   Harbor or Jenkins call on the path — and a second, never-published-to peer serves it too.
   Each further artifact class follows the same shape; the registry column above empties row by
   row.
10. **brit: one chain of addresses from source to served file.** brit (EPR-applied git) already
    seals a source tree as `EprMeta`, a sorted `{path, cid}` list under a CIDv1 dag-cbor root
    (`elohim/brit/brit-epr/src/engine/meta.rs`). Using the same tree for build output joins the
    chain end to end: brit source-tree CID → rakia build (deterministic, slice 1) → output `EprMeta`
    root → release manifest → elected head. A browser verifying one file can walk back to the
    exact source that produced it. It also sharpens attestation: with deterministic builds, a
    peer's attestation can say "I rebuilt source tree X and got output root Y" — an independent
    reproduction, not only "it ran on my device" — which is what makes an earned head earned.
    Carried further, the repository's own source lives on the protocol, the forge becomes a
    mirror, and a submodule pin is attested by a brit attestation on the pinned commit rather than
    a forge's CI check. brit's crates sit behind a registry credential today, so brit is itself a
    row in slice 9's table.
11. **One client resolver.** The browser holds several partial resolvers, none of which owns
    naming and verification; the service worker (slices 2–3) is now honest, the rest are not:

    | Piece | Defect |
    |---|---|
    | `elohim-core` `Loader` (`app/elohim-elements/elohim-core/src/loader/loader.ts`) | the right shape (transport-ordered, `verifyCid` on by default) — no production caller |
    | `BlobVerificationService` (`app/lamad/src/app/services/blob-verification.service.ts`) | falls back to the SERVER as the "authoritative" verifier — inverted trust |
    | `HeliaFetchService` | default `verifiedFetch` looks to the public IPFS network, where household blocks are not announced; its HTTP fallback verifies nothing |
    | `ContentService.getContent` → `DataLoaderService` | errors become placeholders, so the IndexedDB fallback is unreachable |
    | `IndexedDBCacheService` | keyed by id/slug with a TTL, not by CID; no head records |
    | `HolochainCacheService` | no consumers |
    | `SwBridgeService.invalidateApp` | no callers |

    The move: the `Loader` (verification on) becomes the one resolver the content service, media
    and the worker share; the server-authority fallback is removed; IndexedDB is keyed by CID with
    head records (name → CID, when verified); dead pieces are deleted. The Tauri path gets the
    same contract (it runs no worker and verifies nothing today), so browser and native pass the
    same test vectors.

## §8 — P2P design gate

No slice here adds a DHT entry type. Bytes are blob-plane content addressed by CID; the per-file
tree root is blob-plane content; versions are named by the existing head election. Identity is
content-derived throughout. Nothing new is notarized.

## §9 — Graduation

When the record-proves check is green twice on the household mesh, the claim earns:
a Stance in the values canon; a composer / local-inference row in the seam map (reader, never
authority) with routing rows for "a client kept bytes it did not verify" and "a generated claim
has no grounding"; and a habit of its own in `app/elohim-app/.epr-meta/`. Until then this spec is
its only home.

## §10 — Owed decisions

- Per-file tree format (§7.4).
- Which habit carries §7.9 as a check, or whether the artifact origin earns a habit of its own.
- Repository-wide licensing: code (AGPL and Apache today, several packages undeclared) and docs.
- Whether principles carry a numbered series; two unrelated "P1"s exist today.
