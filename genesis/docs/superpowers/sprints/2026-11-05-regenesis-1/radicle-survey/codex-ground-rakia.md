# Rakia and the landing-plane lineage

✅ The lineage now separates three planes: elected heads and attestations are control/truth; artifact bytes live on the p2p data plane; projections and indexes remain reconstructable views. Rakia owns the declarative constellation and custody naming—not another store. `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md:533-547` `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:246-260`

## A. Why / guidestars

- ✅ “When only one person can build the thing, power concentrates through technical obscurity.” `elohim/rakia/docs/specs/2026-04-12-rakia-design.md:11-15`
- ✅ “Peers are stars. The build graph … is the constellation.” `elohim/rakia/docs/specs/2026-04-12-rakia-design.md:17-21`
- ✅ “CI doesn’t own governance.” `elohim/rakia/docs/specs/2026-04-12-rakia-design.md:105-107`
- ✅ “Which release applies is a DECLARED head, never recency.” `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:237-244`
- ✅ “CI publishes, it does not deliver.” `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:499-504`
- ✅ “Peers add; they never gate.” `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:128-133`
- ✅ “Reach is the build’s only trust scale.” `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:128-136`
- ✅ “rakia names, the tier serves.” `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:246-259`
- ✅ “Value flows back to every steward.” `genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md:675-687`
- ✅ The governing test is: if forge and external runners disappear, two workspaces must still integrate and build; a returning peer may add capacity, reach, or independence, but not become a prerequisite. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:98-121`

## B. Primitives and vocabulary

- ✅ **Artifact** — immutable bytes identified by CID. A release artifact’s home schema is `ReleaseManifest.$defs.artifact`: `blobCid`, byte length, SHA-256 equality check, filename, and optional role/app/kind. Local derivation artifacts use `TierBlobManifest`, which embeds the `DerivationRecord` and chunk CIDs. `elohim/rakia/schemas/v1/release-manifest.schema.json:91-155` `elohim/eprfs/eprfs-storage/src/tier_blob.rs:19-45`
- ✅ **Closure** — the exact method and declared input view that determine reusable work. Its native key is `DerivationKey { method, input }`; it is an index key, not the artifact address. `elohim/eprfs/eprfs-core/src/derive.rs:30-75,113-137` `genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md:70-78`
- ✅ **Derivation record** — `DerivationRecord { key, output, computed_by, basis, deterministic, witness }`: evidence that a method/input produced an output CID. Records sit side by side; readers decide trust. `elohim/eprfs/eprfs-core/src/derive.rs:164-218,337-383`
- ✅ **Channel** — a stable `Content` identity whose versions are release manifests. The convention is `runtime:<artifact-class>:<network>:<channel-name>`. `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:123-152` `elohim/rakia/schemas/v1/release-manifest.schema.json:80-89`
- ✅ **Head** — the release version elected for a channel, not the newest timestamp. Its live home is `CanonicalElectionOutput.winner_target`; the manifest itself contains no `head` field. `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:6408-6450` `elohim/rakia/schemas/v1/release-manifest.schema.json:5-18`
- ✅ **Election** — the DHT’s authority-gated choice among declared channel versions; earned declarations outrank staging, and absence remains absence. `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:90-106,348-361`
- ✅ **Custody** — a pledge to hold exact bytes at the address named by the artifact/shard manifest. A content row may resolve that address but is not custody identity. `genesis/docs/superpowers/specs/2026-07-31-custody-blob-canonical-artifact-address-design.md:20-40,62-77`
- ✅ **Reach** — the protocol-owned openness scale `private → … → commons`; for builds it is the only graduation scale. `elohim/epr/src/reach.rs:1-51`
- ✅ **Publication** — turning a local CID into a `Content` version plus a staging declaration on its channel. Publication changes reach; it does not copy or deliver bytes. `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:536-550,624-639`
- ✅ **Attestation** — signed evidence binding inputs and an output CID to an actor. Brit’s current type is `BuildAttestationContentNode`; wider derivation reach eventually uses a separately governed graduation act. `elohim/brit/brit-epr/src/elohim/attestation/build.rs:5-29` `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:342-365`
- ◐ **Offer** — the designed byte-handoff record naming exact oid, base, and carrier (`bundle:<cid>` primarily). It has no current `epr peer offer` command. `genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md:304-320` `elohim/eprfs/epr-cli/src/peer.rs:88-132`
- ✅ **Landing intent** — now a content-addressed `FlowRecord::Intent{Produce}` over change, tree, base, class, closures, and optional predecessor; raise/verify/land/withdraw/dismiss are separate observations. `elohim/eprfs/epr-cli/src/flow/intend.rs:1-62,199-253,818-849`

✅ `ReleaseManifest`’s top-level fields are `kind`, optional `manifestVersion`, `channelId`, `artifactClass`, `artifacts`, `appliesTo`, `envelope`, `provenance`, `declaredReach`, `adoptionDiscipline`, and optional `notes`. `head` is external elected state; `channelId` is the stable identity; `artifacts[]` names the bytes; “soak” is `adoptionDiscipline.soakSecs`; “rollout” is represented by ordered `canaryOrder`, where an empty list means no declared ordering. `elohim/rakia/schemas/v1/release-manifest.schema.json:5-77,371-410`

## C. Wired versus spec-only

| Primitive/surface | State | Evidence |
|---|---|---|
| Rakia manifest types and constellation | ✅ Wired | Generated manifest types, qualified dependency resolution, cycle checks, affected-path propagation. `elohim/rakia/rakia-core/src/generated_types.rs:21-76,162-201` `elohim/rakia/rakia-core/src/constellation.rs:77-143,185-235` |
| Rakia/Brit change detection | ✅ Wired | Gix tree diff and ref resolution. `elohim/rakia/rakia-brit/src/changes.rs:42-138` |
| April artifact-keyed upstream schema/registry | ◐ Spec-only | Current schema still uses step-name dependencies and artifact-name arrays; April’s `inputs.upstream`, `outputs.primary`, and registry trait did not land. `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md:67-201,329-338` |
| Release manifest | ◐ Schema-only in Rakia | Explicitly excluded from Rakia codegen. `elohim/rakia/CLAUDE.md:46-54` |
| Release election/adoption controller | ✅ Wired | Controller resolves elected heads, fetches/verifies CID bytes and routes verified releases through apply vehicles. `elohim/elohim-storage/src/services/release_adoption/mod.rs:1-83` |
| `rakia-executor` | ✅ Wired, but not a build-output registry | It is a bounded peer-stage runner. Its receipt emits CIDs for logs/witnesses and has no `output_cid`. `elohim/rakia/rakia-executor/src/main.rs:11-94` `elohim/rakia/rakia-executor/src/runtime.rs:255-315` `elohim/sdk/schemas/v1/compute-receipt.schema.json:206-295` |
| Derivation types and ports | ◐ Partial | Record/key/index traits exist; `project_view`, `derive_or_reuse`, a concrete `DerivationIndex`, and signed cross-peer integration remain follow-ons. `elohim/eprfs/eprfs-core/src/derive.rs:1-13,177-218,337-425` |
| Local tier artifact custody | ✅ Wired | One-MiB CID chunks, manifest, full verification, typed misses, daemon-independent put/read. `elohim/eprfs/eprfs-storage/src/tier_blob.rs:1-45,177-337` |
| Local ValueFlows accounting | ✅ Wired | Put emits `Produce`; draw emits `Use`, with true output CID and typed outcome. `elohim/eprfs/eprfs-peer/src/local_blob.rs:16-74,76-137` |
| Scoped `serve-blob` and two-peer fetch | ◐ Not wired | Slice 1 advertises and serves nothing; unauthorized-CID refusal remains future work. `elohim/eprfs/eprfs-storage/seam-registry.yaml:109-155` `elohim/eprfs/eprfs-peer/seam-registry.yaml:625-700` |
| Offer/bundle carrier | ◐ Spec-only | Carrier decision is resolved, but no peer verb or protocol exists. `genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md:304-320,817-825` |
| Landing-intent atom/projection | ✅ Wired | `epr flow intend` and `epr peer intents` exist. `elohim/eprfs/epr-cli/src/flow/intend.rs:29-75,199-253` `elohim/eprfs/epr-cli/src/peer.rs:103-128` |

## D. Roadmap and exact placement of artifact storage

✅ The September release ladder recorded rungs 1–4 landed; elected runtime artifacts were rung 5 and remained Draft. `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:52-70`

✅ The November baseline says single-device reuse works—35m32s became a 22s evidence hit—while live sibling reuse remained unmeasured. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:604-616`

> ✅ “The prize: two workspaces build once between them. A closure computed on one peer is reused, or its bytes fetched, by the other.” `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:591-596`

✅ Section 11 starts five tracks together: A projected roots; B the correct Brit oracle; C signed `DerivationRecord`s; D honest unknown/failure states; E the tier blob store. Track E is explicitly “local tier blob put … then A11’s tolerant reader; then a `serve-blob` mount with scope enforcement.” `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:618-631`

✅ The joins are ordered F1r sibling verdict reuse; F2 sibling artifact fetch; F3 landing CAS; F4 carried work/source materialization; F5 unified tool-release/ValueFlows plane; F6 multi-principal graduation. F2 needs A+C+E+F1r and proves the 454 MB storage-binary case. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:634-646`

✅ Source has advanced beyond the November snapshot: Track E’s local, daemon-down slice and its `Produce`/`Use` events are now present. ◐ The next artifact-storage boundary is therefore not “add a Rakia store”; it is scoped network serving on the existing eprfs-peer tier, followed by F2’s two-device fetch receipt. `elohim/eprfs/eprfs-peer/seam-registry.yaml:625-700`

## E. Open decisions for the next sprint

- ◐ Define and implement the concrete Rakia `DerivationIndex` adapter without turning it into storage; records must remain side-by-side and fetchability must remain part of a hit. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:173-190,246-259`
- ◐ Finish Track E’s wire boundary: tolerant mixed-version decoding, scoped `serve-blob`, reach checks, and the unauthorized-CID adversarial test. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:572-586,626`
- ◐ Decide the manifest vocabulary for dependency, supersession, and channel pin relationships. Current `ReleaseManifest` has artifact descriptors and a single `lineageParentCid`, not a general constellation-edge model. `elohim/rakia/schemas/v1/release-manifest.schema.json:111-155,273-302`
- ◐ Reconcile raw CID custody with existing DNA custody records that still use `sha256-…`; the migration and reconciliation cost remain open. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:559-573,683`
- ◐ Resolve signer-to-`agent_cid` and standing before any cross-principal reach graduation; F6 is explicitly blocked on that redraft. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:328-365,688-690`
- ◐ Reconcile release schema drift: the September spec requires `envelope.requires`, but the schema has no such field; tool/doorway release classes also need a settled representation. `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:581-600,712-730` `elohim/rakia/schemas/v1/release-manifest.schema.json:34-44,273-302`

## F. Contradictions and drift

- ✅ **Executor ownership:** April makes Rakia the build/deploy executor and registry client; November rules that Ark is the only executor and Rakia owns naming/planning only. `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md:40-65` `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:246-268`
- ✅ **Transport:** old composition proposes `/rakia/claim|dispatch|result` over storage’s libp2p swarm; the newer design assigns steward-space carry and blobs to eprfs-peer’s iroh tier. `elohim/rakia/docs/composition.md:62-79` `genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md:70-83,279-281`
- ✅ **Schema:** April requires artifact-keyed `(kind, repo)` dependencies and output CIDs; live schemas remain step-name and artifact-name based. `elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md:133-163` `elohim/rakia/schemas/v1/build-manifest.schema.json:86-165`
- ✅ **`output_cid`:** Brit attestations have it; `rakia-executor` receipts do not. The executor only CIDs its inputs and emitted logs/witness. `elohim/brit/brit-epr/src/elohim/attestation/build.rs:5-29` `elohim/sdk/schemas/v1/compute-receipt.schema.json:274-295`
- ✅ **Head overload:** `BuildPlan.head` means the current Git commit, while release head means an elected `Content` version; the steward-space landing head is now a fork-preserving set of tips. These are three distinct concepts sharing one word. `elohim/rakia/rakia-core/src/generated_types.rs:225-290` `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:481-508`
- ✅ **Two stores are intentional only after the November clarification:** self/intimate derivation caches live in the eprfs tier with zero DHT heads; released/community objects use elohim-storage custody entries and blob transport. Rakia is neither store. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:532-581`
- ✅ **Documentation drift is acknowledged in the newest spec:** Rakia’s `CLAUDE.md`, README verbs, and executor description are stale. `genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:648-650`

◐ The sprint-level conclusion is: finish the p2p serving seam around the already-landed local tier blob substrate, then prove F2. Adding storage inside Rakia would reverse the newest ownership ruling and recreate the duplicate registry/store the lineage has spent three specs removing.