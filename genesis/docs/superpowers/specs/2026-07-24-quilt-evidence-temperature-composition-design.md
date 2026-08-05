---
title: "Quilt/Pantry Ontology and Evidence-Qualified Placement Composition"
id: quilt-evidence-temperature-composition-design
tier: spec
status: Draft
created: 2026-07-24
maintainers: Matthew Dowell + Codex
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete OR evidence-qualified-content-declared-floor-shipped
topic:
  - quilt
  - pantry
  - custody
  - resilience
  - temperature
  - placement
  - evidence
  - reed-solomon
  - cid
  - confidentiality
informed-by:
  - genesis/graphos/vocabulary.md
  - genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md
  - genesis/data/timeline/backlog/peer-hoster-async-sync-readiness-assessment.md
  - elohim/sdk/schemas/v1/manifest/app-manifest.schema.json
cites:
  - tiered-quilt-stewardship-design | Tiered Quilt Stewardship | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - durability-topology-felt-resilience | 2026-05-29-durability-topology-felt-resilience | sha256:935b1dd7d8121267 | path: genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md
  - cite-fingerprint-cid-convergence | Cite Fingerprint ↔ Canonical CID Convergence | sha256:0a657c9c1b0c43e7 | path: genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md
  - attestation-consolidation-design | Attestation Consolidation | sha256:220c0a2a68c2a805 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
---

# Quilt/Pantry Ontology and Evidence-Qualified Placement Composition

> **One-line:** preserve the quilt as the durable S3-shaped byte plane and cache-core as the
> Redis-shaped operational plane, while giving the placement controller a precise product of
> content-declared resilience, custody temperature, evidence quality, RS recoverability,
> fault-domain diversity, identity coherence, and confidentiality readiness.

## 1. Decision

The convergence ledger and the tiered pantry compose, but they do not collapse.

- The **ledger plane** says what this observer can truthfully claim about custody.
- The **quilt plane** says which byte units can reconstruct the content.
- The **pantry plane** says which peers undertake stewardship and at what service class.
- The **content declaration** says how much loss the household intends the content to survive.
- The **confidentiality plane** says which byte representation may leave the authoring trust
  boundary.
- The **facing** explains achieved protection relative to the declared floor without upgrading
  claims into facts.

Placement is therefore a vector verdict, not one scalar `tier`:

```text
PlacementVerdict =
  resilience intent
  × custody class
  × evidence class and freshness
  × RS(N,K) decodability
  × social/physical fault-domain diversity
  × holder identity resolution
  × confidentiality readiness
```

The evidence ladder being built by convergence is a prerequisite to tier-aware placement. It is
not itself a storage tier.

This design is deliberately implementation-orthogonal to the active Rust convergence write-set.
It changes no runtime schema, route, wire message, manifest, or seed item. It names the contract
those later changes must satisfy.

## 2. Grounded seam map

### Included seams

| Concern | Owning seam | Current reality |
|---|---|---|
| Quilt bytes, shard distribution, reconstruction, custody observation | Peer-hoster dataplane (T2/T3) | RS encode/reconstruct and local shard projections exist; evidence-qualified tier placement does not |
| Named policy profiles and content-type defaults | App-manifest/domain + SDK grammar | Schema and referential-integrity gate exist; Lamad declares no policies |
| Capacity pressure, admission, pledge ceiling | Resource governance | Capacity/placement metrics exist; no `TierController` consumes them |
| Floors and witnessed claims | DHT notary | Existing Commitment and consolidated Attestation primitives are available; no new entry type is needed |
| Restricted-reach bytes | Confidentiality | Seal primitives are partial; encryption-before-placement remains a hard ordering constraint |
| Browser/operator legibility | Doorway projection + facings | Felt floor slots and pantry metrics exist; they still operate on undeclared/default floors |

### Explicit exclusions and misroutes

- The DHT notarizes commitments, attestations, manifests, and lineage. It never stores bulk
  content bytes.
- Doorway cache, sessions, indexes, counters, and pub/sub are Category C operational state. They
  are not pantry custody, even when they are hot.
- Hub-internal blade/PVC placement is not Track-2 quilt placement. A pantry is a peer role, not a
  Kubernetes topology.
- Reach answers **who may read**. Resilience answers **how durably it should survive**.
- Hardware capability and product packaging do not declare content value.
- `weave` remains reserved for Holochain Moss. Storage redundancy is a `quilt`.

## 3. P2P design gate

No new DHT entry type is proposed. Lamad remains below its entry-type ceiling, and this design
does not spend that headroom.

### Entity: ResilienceRequirement

- **Classification:** Notarized (A), reusing the existing Mishpat `Commitment`.
- **Justification:** the household/author declaration changes the denominator against which the
  network claims protection. Silent mutation would make the protocol lie.
- **Content Address Strategy:** agent-scoped composite:
  `(declaring agent_cid, subject content CID, action="requires-resilience")`.
- **Address Justification:** the declaration is an agent's stance toward immutable content, not
  a second identity for the content.
- **Source of Truth:** Mishpat DHT.
- **Coordinator Zome:** `mishpat::create_commitment`.
- **Storage Projection:** `mishpat_commitments` (`dht_anchor_hash`: yes).
- **HTTP Route:** none proposed. An eventual SDK/HTTP affordance must call the coordinator and
  remain a thin projection edge.
- **Anti-Pattern Check:** no UUID, no REST-first source of truth, no reach-derived durability,
  and no positive `sovereign` apex label.

### Entity: CustodyCommitment

- **Classification:** Notarized (A), reusing the existing REA `Commitment`.
- **Justification:** accepting stewardship is a network-witnessed promise whose breach and
  fulfillment must remain auditable after one projection disappears.
- **Content Address Strategy:** agent-scoped composite:
  `(provider agent_cid, subject blob/quilt CID, action="custody-quilt")`.
- **Address Justification:** two stewards accepting the same quilt create distinct obligations.
- **Source of Truth:** Elohim DHT.
- **Coordinator Zome:** `content_store::create_rea_commitment`.
- **Storage Projection:** `rea_commitments` (`dht_anchor_hash`: yes).
- **HTTP Route:** none proposed; later stock negotiation is coordinator-first.
- **Anti-Pattern Check:** reuses the existing Commitment entry and canonical `agent_cid`; no
  holder is identified by libp2p PeerId or iroh NodeId.

### Entity: CustodyObservation

- **Classification:** Operational (C).
- **Justification:** self-held checks, verified draws, push acknowledgements, losses, and received
  announcements are observer-relative, expiring facts reconstructable from byte probes and
  protocol exchanges.
- **Content Address Strategy:** agent-scoped composite:
  `(observer agent_cid, shard CID, holder agent_cid, observation kind)`.
- **Address Justification:** the observer and holder are both semantically load-bearing. The same
  holder claim has different evidence at different observers.
- **Source of Truth:** local observation log/projection.
- **Coordinator Zome:** none.
- **Storage Projection:** `shard_locations` today (`dht_anchor_hash`: no); a normalized
  observation projection is a later migration candidate.
- **HTTP Route:** no mutation route. Existing resilience reads fold the projection.
- **Reconstruction Strategy:** enumerate local bytes, replay successful push/verify outcomes, and
  re-consume authenticated custody announcements.
- **Anti-Pattern Check:** peer-announced evidence never overwrites locally witnessed evidence;
  received source labels never upgrade the receiver's evidence class.

### Entity: HoldingsAttestation

- **Classification:** Notarized (A), reusing the consolidated Content-attestation convention.
- **Justification:** when policy needs a network-witnessed custody statement beyond expiring
  operational observations, the signed statement is notarized while its underlying probe log
  remains local.
- **Content Address Strategy:** content-derived CID.
- **Address Justification:** changing subject, holder, observation window, or evidence summary
  creates a new attestation.
- **Source of Truth:** Elohim DHT.
- **Coordinator Zome:** `content_store::issue_attestation`.
- **Storage Projection:** unified `attestations` (`dht_anchor_hash`: yes).
- **HTTP Route:** existing attestation projection routes only; no new route in this design.
- **Anti-Pattern Check:** new attestation kind, if needed, is a manifest discriminator rather
  than a new entry type.

### Entity: QuiltPolicyDeclaration

- **Classification:** Derived (A2) inside the content-addressed app manifest; not a standalone
  entity.
- **Justification:** a named policy has no meaning outside the manifest/domain vocabulary that
  defines and references it.
- **Content Address Strategy:** content-derived; it inherits the app-manifest CID and version.
- **Source of Truth:** the notarized app-manifest EPR.
- **Coordinator/Projection/Route:** existing manifest publication, projection, and codegen path.
- **Anti-Pattern Check:** policy names are vocabulary references, never foreign keys pretending
  to be content identity.

### Design constraints discovered

1. A remote peer saying its local status was `self-held` is still only `peer-announced` evidence
   at the receiver. Evidence strength never transits by copying a label.
2. A push acknowledgement proves acceptance at one moment. It does not prove continuing
   possession, disk durability, fault-domain independence, or plaintext confidentiality.
3. `drawn` cannot be both “no stewardship commitment” and the strongest custody floor.
4. RS decodability and household diversity are different denominators.
5. Any new content/blob/shard address is CIDv1-first. Bare `sha256-<hex>` remains a legacy input
   and a valid byte digest, never a new canonical address.

## 4. Canonical ontology

### 4.1 Storage nouns and verbs

| Term | Canonical meaning | Not this |
|---|---|---|
| `quilt` | Reed-Solomon distribution of one byte unit into N shards, any K sufficient to reconstruct | monolithic blob, governance lattice, Moss weave |
| `pantry` | Peer-tended stewardship role holding quilts/shards for households | bucket, database, one physical disk |
| `stock` | Enter or replenish stewardship in a pantry | HTTP upload as a wire rename |
| `draw` | Retrieve, remotely fetch, or reconstruct bytes for a caller | proof of ongoing custody |
| `shard` | One content-addressed piece of a quilt | replica count |
| `RS(N,K)` | N total shards; any K reconstruct; N-K is the loss budget | the live `rs-4-7` label unless its order is explicitly decoded |
| `re-quilt` | Restore the declared N/K/fault-domain posture after loss | merely detect or report a placement gap |

Existing externally legible route/type names such as `/blob` and `BlobStore` may remain during
migration. New semantic identifiers use the pantry vocabulary.

### 4.2 The axes that must not be called one `tier`

| Axis | Question | Provisional vocabulary | Authority |
|---|---|---|---|
| Resilience intent | How irreplaceable is this content and what failure should it survive? | `vault`, `keepsake`, `standard`, `ephemeral` | author/household Commitment |
| Custody service class | What stewardship/service posture has a holder promised? | `none`, `shelved`, `stocked`, `stocked-warm` | custody Commitment + observed fulfillment |
| Working-set state | Is a transient caller copy present? | `absent`, `drawn` | local cache |
| Physical storage class | Where are bytes realized? | RAM/SSD/disk/cellar/external archive, driver-specific | holder capability/driver |
| Evidence class | Why does this observer believe the holder has the shard? | §5 | local observation or attestation |
| Redundancy profile | How many independently placed shards reconstruct? | `RS(N,K)` | shard manifest |
| Reach | Who may read the content? | canonical reach vocabulary | content EPR/governance |
| Confidentiality readiness | Are bytes sealed before leaving authorized readers? | sealed/unsealed + key-envelope state | encryption plane |

`vault` is the high-value label. `sovereign` is not a positive apex: stewardship is
community-grounded, and a person may exercise rights through supported/mediated agency.

### 4.3 Temperature correction

The current manifest schema and tiered-quilt draft declare:

```text
shelved < stocked < stocked-warm < drawn
```

That is a valid **latency/hotness narrative**, but not a custody-floor order: `drawn` is explicitly
a transient working copy with no stewardship commitment, while `shelved` may be the slowest and
most physically durable copy.

The target model separates them:

```text
Custody commitment: none < shelved < stocked < stocked-warm
Working-set state:  absent | drawn
```

A draw may produce a transient caller copy while the source remains shelved, stocked, or
stocked-warm. No schema rename lands in this design, but future schema work should prefer
`defaultCustodyFloor` over overloaded `defaultTierFloor`, and must remove `drawn` from values
that purport to guarantee custody.

### 4.4 A floor is a tuple, not a replica count

The content-relative resilience floor has this semantic shape:

```text
ResilienceFloor {
  value_class
  min_distinct_households
  min_fault_domains
  min_custody_class
  rs_profile: RS(N,K) | unquilted
  accepted_evidence_policy
  retention_horizon
  confidentiality_requirement
}
```

This is vocabulary, not a proposed wire schema. It makes several existing truths explicit:

- `wantsHouseholds` is social/failure-domain diversity.
- `K` is the count of surviving distinct shards needed to decode.
- `N` is the desired total shard population.
- One household holding several shards does not create several household fault domains.
- Mean placement-gap coverage is not automatically RS decodability.
- Content smaller than the current RS threshold is not truthfully “quilted” merely because the
  facing wants seven holders.

## 5. Evidence ontology

### 5.1 Evidence is observer-relative

| Evidence class | Observer directly witnessed | Strength | What it still does not prove |
|---|---|---|---|
| `direct-possession` | local bytes verify against the shard CID | confirmed here | another fault domain holds it; future retention |
| `verified-draw` | an independent draw returned CID-valid bytes from the named holder | confirmed remote at time T | continuing retention after T |
| `accepted-push` | a push to the named holder returned protocol acknowledgement | witnessed acceptance | durable write, present possession after T |
| `peer-asserted` | a peer announced that a holder has bytes | claimed | authenticated authorship until signature verification lands; possession |
| `not-observed` | nothing current | unknown | anything |
| `observed-lost` | a local probe witnessed absence/corruption | negative evidence | permanent unrecoverability of the quilt |

`seeded` is provenance, not evidence strength. `verified` describes a verification act but must
name who verified whom. The current free-form `shard_locations.status` field mixes provenance,
observation method, polarity, and lifecycle; the normalized ontology keeps them separate.

### 5.2 The no-laundering law

Evidence is classified at ingestion from the receiver's perspective:

```text
announcer's source_status = self-held
receiver's evidence_class = peer-asserted
```

The source status may remain as attributed provenance, but it never changes the receiver's
evidence class. A signature authenticates who made a claim; it does not turn the claim into a
byte probe.

This preserves today's monotone rule—peer announcements never overwrite local evidence—while
making the reason computable rather than relying on free-form strings.

### 5.3 Freshness and polarity are mandatory

Every evidence record must be interpreted with:

- observer `agent_cid`;
- holder `agent_cid`;
- shard CID;
- evidence class;
- positive or negative polarity;
- `observed_at`;
- expiry/validity horizon selected by policy;
- optional attributed source evidence;
- verification/authentication state.

No universal TTL is chosen here. Higher-value floors may require shorter verification horizons
even when their retention horizon is longer. An expired observation becomes unknown, never
negative.

## 6. Evidence × resilience × custody composition

### 6.1 Three non-flattened ledgers

Every resilience fold reports a vector:

```text
confirmed = direct-possession + fresh verified-draw
witnessed = fresh accepted-push not yet independently verified
claimed   = authenticated peer-asserted, or structurally accepted legacy announcement
```

The buckets are disjoint from the observer's perspective. Facings may summarize them but must not
sum them into an unlabeled “holders” number.

### 6.2 Default qualification policy

This table is the starting policy for planning and facings, not a hard-coded protocol constant.
The author-governed resilience requirement may tighten it but never accept evidence weaker than
the implementation can honestly produce.

| Resilience intent | May satisfy hard floor | May guide recruiting/heal | Must remain labeled only |
|---|---|---|---|
| `vault` | fresh confirmed evidence across required fault domains | witnessed acceptance | peer claims |
| `keepsake` | confirmed evidence; bounded witnessed acceptance may count only with an explicit policy | witnessed acceptance | peer claims |
| `standard` | confirmed + fresh witnessed acceptance | peer claims | unauthenticated/structural-only claims |
| `ephemeral` | a verified reconstructable commons/archive source may satisfy the pointer posture | witnessed/claimed sources may guide re-fetch | no personal-custody implication |

Until Stage-2 signature verification lands, a custody announcement's non-empty signature is only
structural validation. It must not satisfy an authenticated-claim requirement.

### 6.3 Custody class adds a proof obligation

Evidence of bytes is not automatically evidence of a promised custody class:

- `accepted-push` without a realized-class receipt proves neither `stocked` nor `shelved`.
- `verified-draw` can verify availability and latency at one time, not physical medium.
- a `stocked-warm` claim needs both durable custody evidence and draw-latency evidence;
- a `shelved` claim needs durable-retention evidence, not fast draw;
- driver/physical-medium claims remain capability/attestation facts, never guessed from latency.

The placement controller therefore evaluates:

```text
holder qualifies =
  identity resolves to agent_cid
  AND pledge backs requested custody class
  AND confidentiality eligibility passes
  AND evidence meets the resilience policy
  AND evidence is fresh
```

It then evaluates the quilt as a whole:

```text
floor satisfied =
  at least K qualifying distinct shard indexes are decodable
  AND desired N/redundancy posture is met
  AND distinct-household and fault-domain minima are met
  AND no active negative evidence invalidates those counts
```

Weak claims are still useful: they widen discovery and prioritize verification. They do not
quietly turn amber into green.

## 7. Placement and repair composition

The future controller order is:

1. Resolve the subject to canonical content/blob/shard CIDs.
2. Resolve holder transport identities through `agent_cid`; never raw-compare identity
   namespaces.
3. If reach is restricted, require the sealed custody representation and valid key-envelope
   posture before candidate selection.
4. Read the content's resilience requirement; mark whether it is explicitly declared or a
   content-type/archetype default.
5. Resolve the named quilt policy for access dynamics and draw QoS.
6. Filter candidates by pledge/capability and confidentiality eligibility.
7. Select candidates for social, geographic, power, and hardware failure diversity.
8. Stock or re-quilt.
9. Record observer-honest evidence from the actual outcome.
10. Fold confirmed/witnessed/claimed coverage separately.
11. Emit a placement gap whenever any tuple leg is short.
12. Render the human facing against the declared floor, preserving unknown and weak evidence.

Repair consumes the same verdict. Detection without a replacement-holder consumer is not
re-quilting.

## 8. Confidentiality ordering

For restricted-reach content, plaintext must never be distributed first and “made private”
afterward. The safe order is:

```text
author plaintext
  → seal for the authorized reader set
  → compute CID of the sealed byte representation
  → quilt the sealed bytes
  → place sealed shards with custodians
  → distribute key envelopes separately
```

Consequences:

- Pantry custodians may hold shards they cannot read.
- Shard CIDs address ciphertext shards; integrity remains locally verifiable.
- Reach changes may rotate/re-wrap key envelopes without pretending old ciphertext placement was
  confidential if it was not.
- A placement engine must fail closed when confidentiality readiness is unknown.
- Encryption readiness is a candidate filter before evidence/temperature weighting, not another
  temperature class.

## 9. Authoring readiness: why the seed-corpus pass waits

The manifest substrate is partially ready:

- `app-manifest.schema.json` defines named `vocabulary.quiltPolicies`, a default, and per-content
  type references.
- Referential integrity is enforced by the manifest loader.
- Qahal declares a real `household` policy, proving the schema path.
- Lamad declares no quilt policies.
- Seed content exposes no canonical per-item resilience requirement.
- The felt surface always receives `standard, tierDeclared=false`.
- No runtime TierController consumes the manifest declaration.

Therefore adding `tier` fields to `genesis/data/lamad/content/*.json` now would create
declaration-shaped data with no authoritative schema, importer, DHT home, or placement consumer.
That is not harmless preparation.

The safe authoring sequence is:

1. Ratify the separate vocabularies in §4.
2. Declare Lamad **content-type quilt policies** in its manifest for access/custody dynamics.
3. Land the `requires-resilience` author-truth path and its projection.
4. Add a governed seed/import representation for explicit per-item overrides.
5. Only then classify the seed corpus and verify `tierDeclared=true` in the felt facing.

## 10. CID-first conformance inventory

CID support exists at several edges, but the byte plane still normalizes a CID to a bare
`sha256-<hex>` address early. The migration must keep SHA-256 as the digest inside CIDv1 while
stopping the bare digest from competing as a public identity.

### Address-minting and storage surfaces

| Surface | Current legacy behavior |
|---|---|
| `elohim-storage/src/blob_store.rs:109-194,401-436` | computes CID and SHA, keys/returns bare SHA, parses CID back down to bare digest |
| `elohim-storage/src/sharding.rs:32-58,147-238` | manifests and every shard mint `blob_hash`/`shard_hashes`; only whole blob has optional CID |
| `elohim-storage/src/http.rs:1950-2248,2392-2513` | PUT/GET accepts some CID inputs but responses, manifests, and inventory normalize to bare SHA |
| `genesis/seeder/src/storage-client.ts:37-49,94-100,199-224` | direct seeder client computes and transports hash-first manifests |
| `genesis/seeder/src/blob-manager.ts:148-152,216-232,331-335,403-432` | normalizes, validates, and writes `blobHash` |
| `genesis/seeder/src/seed.ts:811-820,1240-1244,1504` | sparse content and linkage persist bare SHA |
| `scripts/ci/stage-spa-blob.sh:123,154-162,208` | deployment artifact producer mints and publishes bare SHA |
| `doorway-service/src/routes/seed.rs:120-223,259-282,331-336` | requires `X-Blob-Hash`; ignores the already-sent `X-Blob-Cid` |

### Wire and projection cutover surfaces

| Surface | Current compatibility boundary |
|---|---|
| `p2p/inventory_gossip.rs:27-50,106-134,220-229` | `BlobAddress` accepts exactly `sha256-<64 lowercase hex>`; CID is rejected |
| `p2p/custody_announce.rs:53-85,145-158` | transports `shard_hash`; structurally widenable |
| `p2p/blob_protocol.rs:40,56-63` and `p2p/shard_protocol.rs:21-61` | hash-named requests and dual hash/CID content records |
| `p2p/epr_protocol.rs:46-51` | delivery query transports `blob_hash` |
| `p2p/mod.rs:359` | capability example embeds address and warmth in `warm:sha256-abc` string grammar |
| `db/models.rs:2758-2823,3201-3238` | shard manifest/location/inventory keys are hash-named |
| `content_store_integrity/src/lib.rs:480-527` | `blob_cid` is conceptually a storage address, but SDK/tests permit bare SHA in it |
| `content_store_integrity/src/lib.rs:632-706` | DHT ShardManifest/Location transport bare hash addresses |
| `elohim-views/src/infrastructure.rs:2204-2230` | `PutBlobResponse.blob_hash` remains required |
| `sdk/schemas/v1/views/element-registry-view.schema.json:31-39` | a field named `cid` is constrained to `^sha256-` |
| Doorway blob/app/cache/SSR routes | accept or preserve hash-first keys; custom blob handler normalizes CID to SHA |
| Seeder/storage client + Angular resolvers | dual-read compatibility exists, but helpers continue producing/normalizing bare SHA |

### Canon/document contradictions to reconcile first

- `genesis/graphos/vocabulary.md` still presents `sha256-<hex>` as the positive shard address.
- `protocol-specification.md:1088-1094` says components MUST produce it, while `:1210` says new
  code should not.
- `EDGE-ARCHITECTURE.md`, `P2P-DATAPLANE.md`, and Holochain `ARCHITECTURE.md` still present
  SHA-addressed storage as the destination rather than legacy reality.

### Legitimate SHA-256 uses that remain

- comparing fetched/stored bytes for integrity;
- extracting/comparing the multihash digest inside a CID;
- local filesystem, ETag, cache, or dedup keys that are never dereferenced as protocol
  addresses;
- cite fingerprints, build checks, Nix hashes, Git object IDs, and diagnostic fingerprints.

These should be typed/named as digests or dedup keys. A field named `cid` must never contain a
bare SHA value.

### Compatibility-safe migration order

1. Reconcile canon and define a typed raw-byte CID plus a distinct SHA digest/dedup type. Add a
   contract test forbidding `sha256-` in any `*cid` field.
2. Make producers CID-primary while retaining deprecated hash aliases: BlobStore, blob/shard
   responses, seeders, and deployment artifact scripts.
3. Make the doorway seed route consume and verify `X-Blob-Cid`; derive the legacy digest
   internally.
4. Correct DHT/app-manifest semantics: real CID in `blob_cid`; version shard manifests to carry
   blob and shard CIDs.
5. Keep a local `CID → SHA dedup key → transport alias` projection so filesystem layout and byte
   verification need not churn with public identity.
6. Version P2P inventory first—the hard boundary—then custody, blob, shard, and delivery
   protocols. Dual-decode and dual-publish through a bounded mixed-version window.
7. Preserve CID end-to-end through doorway cache/app/SSR and generated clients. Legacy inputs
   remain read-only adapters.
8. Stop emitting bare addresses only after telemetry shows no legacy peers/rows; remove legacy
   parsers last.

## 11. Observability and human-facing contract

The existing metrics compose with this model:

- custodian free/used/stewarded bytes describe pantry capacity;
- placement-gap count describes requested-vs-achieved work;
- RS coverage describes a present aggregate, not yet evidence-qualified decodability;
- custody announce counters describe claim convergence;
- heal/custody counters describe controller activity.

Required future additions should preserve the vector:

```text
coverage_shards{evidence="confirmed|witnessed|claimed"}
coverage_fault_domains{evidence="confirmed|witnessed|claimed"}
floor_shortfall{dimension="rs_k|rs_n|household|region|custody|evidence|confidentiality"}
evidence_expired_total{class}
custody_class_observed{class}
```

The human facing follows the same honesty law:

- `Protected` requires the declared floor using qualifying evidence.
- `Watching` may show a witnessed-but-not-yet-verified holder.
- `Claimed by another pantry` is useful, but never phrased as confirmed backup.
- `Not yet seen` remains distinct from zero.
- `tierDeclared=false` remains visible to the machine and non-assertive in the UI.

## 12. Graduation increments

This design decomposes into independently landable increments:

1. **Vocabulary normalization:** amend `genesis/graphos/vocabulary.md`; separate resilience,
   custody, working-set, evidence, and RS axes; correct CID language.
2. **Canon reconciliation:** remove the protocol-spec bare-SHA contradiction and publish the
   CID migration ledger.
3. **Evidence view:** introduce a typed evidence vocabulary/fold without changing placement
   behavior; preserve observer, source, polarity, and freshness.
4. **Lamad policy declaration:** add named content-type quilt policies after vocabulary
   ratification; no seed-item overrides yet.
5. **Resilience author truth:** land `requires-resilience` through the existing Commitment
   coordinator and projection.
6. **Evidence-qualified facing:** render confirmed/witnessed/claimed coverage separately before
   allowing it to drive placement.
7. **CID producer/wire migration:** follow §10 ordering.
8. **Tier controller:** consume declared intent, policy dynamics, capabilities, evidence, and
   confidentiality readiness.
9. **Seed authoring:** classify corpus items only after the explicit override path is real.

## 13. Acceptance invariants

1. No public field named `cid` contains `sha256-<hex>`.
2. No received peer label upgrades the receiver's evidence class.
3. No evidence bucket is counted twice.
4. No expired observation is treated as negative; it becomes unknown.
5. No `peer-asserted` claim alone satisfies a vault floor.
6. No content is called RS-protected unless K distinct shard indexes are reconstructable.
7. No household count is inferred from shard count.
8. No `drawn` working copy is treated as a custody commitment.
9. No reach value derives resilience intent.
10. No restricted-reach plaintext enters placement before sealing.
11. No transport identity is compared directly with `agent_cid`.
12. No cache-core operational state receives a quilt custody floor.
13. No new DHT entry type is added for policy, evidence, or holdings.
14. No seed tier declaration lands before its schema, importer, DHT home, and consumer exist.
15. Two doorways may differ in what they have locally witnessed, but they must label the
    evidence so neither presents a weaker claim as stronger truth.
