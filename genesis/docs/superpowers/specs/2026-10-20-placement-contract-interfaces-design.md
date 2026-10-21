---
id: placement-contract-interfaces-design
title: The placement contract's interfaces — one document every implementer builds against
status: Draft
class: architecture
date: 2026-10-20
context-tier: disclosed
steward: human:matthew
author: "Claude (architect seat, Opus 5.5), synthesizing three independent architects' proposals (developer-ergonomics, the judges' winner, with grafts from performance-layering and reuse-ioc) at the orchestrator's direction; actor agent:architect@claude-opus-5-5"
serves: materialize-what-is-read
gap: plans__2026-10-20-placement-contract-architecture-plan#3
graduation-trigger: >-
  Every seam A–I below has a first instance in code that compiles against the signatures given here
  unchanged, with its contract test green in its owning gate (`just gate eprfs` for A–C and the eprfs
  half of H, the placement-core gate for D–E, `just gate elohim-storage` for the custody and serving
  adapters, the ark gate for F, `pnpm run schema:test` for G, the app and SDK gates for H). No
  implementer has had to change a signature here to get there, or the change is recorded in §6 as a
  revision with its reason.
cites:
  - "what-every-app-inherits-placement-contract-design | the design this document shapes: rulings 1-6, section 7 floor and ceiling, section 12 bytes layer, slices and decisions | sha256:fe10f9dc966c2e9a | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - "placement-contract-architecture-plan | the lanes and write-set; item I1 is this document, I2 its adversarial seam check | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "placement-eprfs-slice1-brief | the slice-1 walk API adopted as seam B, with the R6 named-absence change | sha256:37af148c22129809 | path: genesis/docs/superpowers/plans/task-placement-eprfs-slice1-brief.md"
  - "placement-interfaces-brief | the brief this document answers: required contents, rulings R1-R6 and R6a | sha256:f953a81da54d3247 | path: genesis/docs/superpowers/plans/task-placement-interfaces-brief.md"
  - elohim/eprfs/CLAUDE.md
  - elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md
  - elohim/eprfs/eprfs-core/src/storage.rs
  - elohim/eprfs/eprfs-core/src/error.rs
  - elohim/eprfs/eprfs-core/src/tree.rs
  - elohim/eprfs/eprfs-core/src/walk.rs
  - elohim/eprfs/eprfs-core/src/address.rs
  - elohim/eprfs/eprfs-core/src/composition.rs
  - elohim/eprfs/eprfs-core/src/projection.rs
  - elohim/eprfs/eprfs-core/src/awareness.rs
  - elohim/eprfs/eprfs-storage/src/lib.rs
  - elohim/elohim-storage/src/reconcile/placement.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/services/salvage_commitment_author.rs
  - elohim/elohim-storage/src/services/rea_commitment_service.rs
  - elohim/elohim-storage/src/services/peer_selection.rs
  - elohim/elohim-storage/src/services/serve_routing.rs
  - elohim/elohim-storage/src/blob_reach.rs
  - elohim/elohim-storage/src/private_reach.rs
  - elohim/elohim-storage/src/api/compute_grants.rs
  - elohim/elohim-storage/src/api/compute_tasks.rs
  - elohim/elohim-storage/.epr-meta/evidence-class-descends.habit.md
  - elohim/elohim-compute/src/actuation.rs
  - elohim/ark/core/src/lib.rs
  - elohim/ark/core/src/berth.rs
  - elohim/ark/core/src/manifest.rs
  - elohim/ark/core/src/passport.rs
  - elohim/ark/core/src/candidate.rs
  - elohim/ark/core/src/intent.rs
  - elohim/ark/core/src/rea.rs
  - elohim/epr-rea/src/model.rs
  - elohim/epr/src/reach.rs
  - elohim/epr/src/witness.rs
  - crates/seam-contracts/src/answer.rs
  - crates/seam-contracts/src/reason.rs
  - crates/seam-contracts/src/election.rs
  - crates/seam-contracts/src/harness/arbitrated.rs
  - crates/seam-contracts/src/harness/quiescent.rs
  - crates/elohim-storage-client/src/client.rs
  - elohim/sdk/schemas/v1/manifest/app-manifest.schema.json
  - elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json
  - elohim/sdk/schemas/v1/enums/closure.schema.json
  - elohim/sdk/schemas/v1/enums/reach.schema.json
  - elohim/sdk/schemas/v1/objects/answer.schema.json
  - elohim/sdk/schemas/v1/views/CONVENTIONS.md
  - elohim/sdk/epr-ts/src/index.ts
  - elohim/sdk/storage-client-ts/src/client.ts
  - app/elohim-app/src/apps-sw.ts
  - app/elohim-app/src/app/elohim/utils/raw-cid-verify.ts
  - crates/seam-contracts/src/residual.rs
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/p2p/blob_fetch.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/api/account.rs
  - elohim/elohim-storage/src/db/salvage_capacity.rs
  - elohim/elohim-storage/src/services/holds.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/compute_task.rs
  - elohim/epr-rea/src/index.rs
  - elohim/eprfs/build-manifest.json
  - elohim/eprfs/eprfs-core/src/attestation.rs
  - elohim/eprfs/eprfs-local/src/lib.rs
  - elohim/ark/core/src/exit.rs
  - elohim/sdk/schemas/scripts/codegen-ts.mjs
  - elohim/sdk/schemas/scripts/test-schema.mjs
  - genesis/a2o/scripts/lib/sut.ts
  - app/elohim-app/src/app/elohim/services/content.service.ts
  - app/lamad/src/app/renderers/iframe-renderer/iframe-renderer.component.ts
---

# The placement contract's interfaces

The design spec (`what-every-app-inherits-placement-contract-design`, "the spec", sections written
"spec §n") says what is built and why. This document says how each seam is shaped, so every later slice
fills it in without guessing. It cites the spec's reasons and does not restate them.

Rulings in force: spec §1 rulings 1–7 and spec §7; the orchestrator's R1–R7 and R6a recorded on the
plan; architecture before landing. Ruling 7 (R7) is carried by one shared type, `Unanswered<R>`, and one
status channel, both defined in §2.0 before the seams that use them. Every signature below was checked
on 2026-10-20 against the code it has to fit, and the file each type lives in is named. The adversarial
seam check (item I2, `task-placement-seam-check-report.md`) revised the signatures on the same day; §6
lists each revision by finding id (A1…I12). **New** marks a file that does not exist yet.
**Follow-on** marks a path outside this session's write-set, named and not written.

The design rule throughout is that **the wrong call cannot be written**. Where a type can prevent a
mistake, the type prevents it. Where only a check can catch it, the check is named as a rail (§1.3).

**Write-set of this item.** This item writes this document and its report, nothing else. Each piece of
code below is assigned to the lane that will write it: lane E (`elohim/eprfs/**`: the eprfs-core
modules, the new `eprfs-serve` crate and the new `@elohim/eprfs` package), lane S
(`elohim/sdk/schemas/v1/**`), or a follow-on outside the covenant (placement-core, ark-core `task.rs`,
the elohim-storage adapters, the elohim-compute feature split, the seam-contracts additions, the
`@elohim/epr` verifier, storage-client-ts, `apps-sw.ts`).

Named follow-ons that lanes E and S need but cannot write inside their own paths (G5, H15, H16):
`elohim/sdk/schemas/scripts/test-manifest-needs.mjs`; the `INTERFACE_FILES` and `GENERATED_OUTPUT_DIRS`
entries in `scripts/codegen-ts.mjs` (adding `elohim/sdk/storage-client-ts` and `elohim/eprfs/eprfs-ts`);
the discriminating tokens in `test-schema.mjs`'s `TASK_1_TO_3_SURFACE`; the `schema:test` entry in the root
`package.json`; the regenerated `schema-enums.ts` distributions; the elohim-storage `GRANT_SCOPES` equality
test; the `pnpm-workspace.yaml` entry for `elohim/eprfs/eprfs-ts`; an `eprfs-ts` gate project with a
ts-test step (a `_gate-eprfs-ts` recipe) and a gate for storage-client-ts; the elohim-app `prebuild`
ordering (`@elohim/epr`, `@elohim/eprfs`, `@elohim/storage-client` before `build:sw`) and its package
dependencies. Lane S's S5 deliverable is therefore the `v1/**` files plus a dry `--verify` naming the
follow-on diff. Until the `@elohim/epr` follow-on lands `verifyAddress`, lane E's `eprfs-ts` carries a local
raw and dag-cbor check pinned to the same vectors, so it compiles in this session.

---

## 0. Findings that shape the interfaces

Each finding changes a name or a shape below, and each was read in the code.

1. **The slice-1 walk enum is called `Placement`** (`eprfs-core/src/walk.rs`, re-exported from
   `lib.rs`). It collides with the placement contract's vocabulary. The rename to `Coverage` is decided
   (decision 12), and this document is written with it applied. Lane E applies it before E1 lands, which
   costs one identifier today (§2.B).
2. **`EprfsError` is not `#[non_exhaustive]`.** In the working tree slice 1 already marks `TreeError`
   (`tree.rs`, with the note that the next publication is `0.2.0`) and `AbsentReason`, each of its
   variants included (`walk.rs`); only `EprfsError` (`error.rs`) is unmarked. Slice 1's five new
   `TreeError` variants are a semver break for brit, which pins `eprfs-core = "0.1.2"` from the `elohim`
   registry (`brit-bridge/Cargo.toml`, `brit-cli/Cargo.toml`). The first registry publish that carries
   `walk` is therefore `0.2.0`; it marks `EprfsError` too, and in the same release removes
   `MaterializationPolicy` after one deprecated release (B9) and marks `DerivationKind` (C17). brit
   moves its pin when it adopts that release, never as a stray move. Variant-level `#[non_exhaustive]`
   means no crate outside eprfs-core can build an `AbsentReason` or a `WalkItem`; eprfs-core therefore
   exposes public constructors for every absence and walk item a remote tier must produce (B12).
3. **`BlobCid` has no `Ord`** (`address.rs`: `Debug, Clone, PartialEq, Eq, Hash`). `cid::Cid` 0.11 is
   `Copy, PartialOrd, Ord, Hash`, so `BlobCid` gains `PartialOrd, Ord` additively. Several keys below
   need it (`DerivationKey`, the `BTreeMap`s keyed by CID).
4. **`FetchPolicy` is `Clone` only and has no `Default`.** It gains `Copy` and `Default` (with
   `#[default] LocalOnly`, the safe default) additively. The slice-1 code calls `policy.clone()`, which
   keeps compiling; `BatchOptions` derives `Default` over it (A12).
5. **`elohim_epr_rea::model::AgentRef` has no `Ord`, and `elohim_epr::Reach` deliberately has none**
   (`reach.rs` comments forbid it). No type below derives `Ord` over either. Ranking tie-breaks on the
   agent string's bytes; reach compares through `Reach::openness()`.
6. **`DirectoryStorage::has_blob` reads and hashes the whole object** to answer a presence question
   (`eprfs-storage/src/lib.rs`, `has_blob` calls `read_object`). The tier chain never calls `has_blob`
   on its hot path (§2.A contract 1).
7. **`closure` already names something else in the SDK.** `enums/closure.schema.json` is the
   `open | closed` posture toward silence. The bytes closure is a `tree-view` object on the wire; the
   manifest field may keep the name `needs.closure`.
8. **The release root is bound outside the manifest**, so `needs.closure` carries paths only and the
   release record binds the root. (The SDK domain manifest is not inside the packaged bundle archive:
   `package-app.mjs` packs `--dist`, the Angular browser output, so the reason is binding, not
   self-reference; G8.) Where `closure` lives, app or release manifest, is spec §19 Q1, and this
   placement answers it only subject to the operator's ruling (§5, decision 22).
9. **ark-core's `Intent` is not an REA intent** (`intent.rs`: a write-ahead process record with
   `at_epoch_ms` and `incarnation`). No placement module imports it. ark-core's REA projection is
   `rea.rs` (`RuntimeScope::from_berth`, `as_rea_intent`), and that is the berth offer's existing home.
10. **`ReaVerb` cannot express custody** (`epr/src/witness.rs:315`: use, consume, produce, cite,
    affirm, dismiss). A notarized placement commitment therefore maps onto
    `shefa_types::CreateReaCommitmentInput` (`sdk/domains/shefa/types/src/lib.rs`), the shape salvage
    authors today, and not onto `epr_rea::model::Commitment`, until `ReaVerb` grows.
11. **Salvage self-candidacy breaks agreement today.** `build_salvage_candidates` pushes self into the
    pool before the peer's own capacity row has round-tripped. Spec §7.2 admits self only through the
    shared published record (§2.D.3).
12. **`GRANT_SCOPES` is a private const** (`api/compute_grants.rs:46`: `sweettest-feedback`,
    `hosted-cell`, `measure-stage`). Placement needs the scope list, so it gains a shared home: an SDK
    enum `enums/grant-scope.schema.json` and a closed Rust `GrantScope` (§2.D.1), with a test in
    elohim-storage that its list equals the enum, extended to the duplicated `hosted-cell` constants in
    `doorway-service/src/routes/hosted_cell.rs` and `api/hosted_custody.rs`. `grant-scope` is the
    **compute-grant** scope list, a closed subset of `delegates-compute.scope`, which stays an open
    string by design (its schema and test use `republish-epr`, `serve-url-projection`); the two are not
    one vocabulary (G10).
13. **`elohim-compute` is not pure as a crate** (it depends on chrono, dashmap and parking_lot for
    `health`, `peers`, `counters`, `report`), **but `actuation.rs` uses only serde.** ark-core already
    depends on it. placement-core takes it with `default-features = false` after a follow-on gates the
    other modules behind a default-on `runtime` feature (§1.3, R-P1).
14. **Wire view types now live in `crates/elohim-views`** (ts-rs anchored), not in elohim-storage's
    `views.rs` (`elohim-storage/CLAUDE.md`). New wire views go there.
15. **eprfs's dependency direction, read precisely.** "domain adapter → eprfs → elohim-storage" and "do
    not make elohim-storage understand working trees" (`elohim/eprfs/CLAUDE.md`) govern the eprfs
    adapter crates and working trees. eprfs-core is a pure value crate and a leaf, and elohim-storage
    depending on it (and on `eprfs-serve`) in order to serve trees as a tier, decoding nodes only to check
    membership, is within that rule in substance. It is stated as rail R-E3 and put to the operator
    (§5, decision 3).
16. **The evidence vocabulary `fixture | real` exists only as prose** in an `unwired` habit
    (`elohim-storage/.epr-meta/evidence-class-descends.habit.md`), whose declared first move is the
    schema field. The one `evidence_class` field in the SDK
    (`attestation/subtypes/challenge-support-metadata.schema.json`) means a kind of challenge evidence,
    a different thing. The probe takes the habit's two values as a new enum,
    `enums/evidence-stage.schema.json`, named `evidenceStage` on the wire to avoid the collision, and
    lands it as that habit's first move so the observation and attestation schemas take the same `$ref`
    (I10).
17. **Ruling 7 has no shared type yet.** seam-contracts' `Answer<T>` carries the state and
    `AnswerReason` the transport reason, but nothing carries when to come back or what can be done
    locally, and no status channel exists. §2.0 adds both to seam-contracts, and R-E1 admits that
    zero-dependency leaf to eprfs-core so every seam can name one type (B1, C13).
18. **`DeliveryPeer` knows a peer by libp2p `peer_id`, multiaddrs and `http_port`** (`p2p/mod.rs`), not
    by agent key, and only a cross-signed binding resolves the agent (`household_id`). Tier identity
    therefore allows a transport id where no binding resolves an agent key, and is never read as
    attribution (A11, H6).

---

## 1. Layers, crates and rails

### 1.1 The graph

```text
 L0 leaves        elohim-seam-contracts (std/alloc, zero first-party deps)   cid · multihash · serde
                         ▲                       ▲
 L1 pure cores    eprfs-core                elohim-placement-core  NEW  elohim/placement/core
  (no runtime,    BlobCid, Tree, View/walk,  Need, Posture, Candidate, admit→Eligible, Ranking,
   no I/O,        LevelWalk, TierChain,      Charter, InForce, decide()→Plan, explain, Floor<Ports>
   no clock)      ReachGate/AskPolicy,         ▲ also elohim-epr (Reach, cid) · elohim-epr-rea
                  ForwardGate, TreeView,       │ (AgentRef, atom_cid) · elohim-compute (Governor,
                  Derivation*                  │  Refusal, LimitOwner; actuation only)
                         ▲                     │
                         └──────────── elohim-ark-core (+ task.rs: TaskSpec, BerthOffer, TaskKind)
 L2 adapters      eprfs-storage   DirectoryStorage, MemoryStorage, SemaphoreGate, HttpTier (feature http)
  (I/O, async,
   transport,     eprfs-serve NEW TierEndpoint: the serving side of A and B, membership-checked;
                                  ReasonLabel faces; testing: assert_tier_contract, CountingTier
   persistence)   eprfs-local     LocalMaterializer (depth 1), restore_exact_tree, Hydrator mount (slice 3)
                  elohim-storage  BlobStoreTier, PeerTransportTier, RootReachGate, custody Ports, the
                                  PlacementStrategy shim
                  ark-supervisor  TaskRunner: materialize view → run in envelope → record derivation
                  brit-bridge     seal_git_tree / restore over any EprfsStorage
 L3 hosts         elohim-storage binary · elohim-hub · epr / brit / rakia / lvi CLIs · apps-sw.ts
 TypeScript       @elohim/epr (exists: cbor, cid; gains verifyAddress)
                     ◄── @elohim/eprfs NEW elohim/eprfs/eprfs-ts (tree codec, walk, TierChain; pure)
                           ◄── @elohim/storage-client (HttpTier, placement.status()/why())
                                 ◄── apps-sw.ts (CacheTier; host)
```

Arrows point from a crate to what it depends on. Decisions live only in L1. L2 translates rows, files,
sockets and conductor calls into L1 types and applies L1 decisions as effects. L3 composes adapters and
wires triggers. No adapter makes a placement, reach or ranking decision of its own.

### 1.2 Who owns which trait

| Crate | File | Owns |
|---|---|---|
| elohim-seam-contracts | `src/unanswered.rs` (follow-on, alloc only) | `Unanswered<R>`, `ComeBack`, `Meanwhile`, `Escalation`, `PendingStatus`, `StatusCursor` (R7) |
| eprfs-core | `src/storage.rs` | `EprfsStorage` (existing; gains `fetch_at`, `fetch_blobs`, `query_read_path`, `query_list_dir`, `query_diff`, all defaulted) |
| eprfs-core | `src/walk.rs` (slice 1, in the working tree) | `walk`, `walk_collect`, `walk_spine` (and `SpineWalk`, slice 4), `read_path`, `list_dir`, `diff_trees` (all `S: EprfsStorage + ?Sized` from slice 2), `TreePath`, `View`, `Coverage`, `Depth` |
| eprfs-core | `src/level_walk.rs` (new, slice 2) | `LevelWalk`, `walk_batched` |
| eprfs-core | `src/tier.rs` (new, slice 2) | `TierId`, `TierClass`, `TierRecord`, `TierOrder`, `ForwardGate`, `BlobSink`, `BatchItem`, `ItemAbsence`, `TierChain` |
| eprfs-core | `src/reach.rs` (new, slice 2) | `Requester`, `KeyCustody`, `ReachGate`, `AskPolicy`, `ReachRefusal` |
| eprfs-core | `src/view_decl.rs` (new, slice 2) | `TreeView` (the serializable root plus view) |
| eprfs-core | `src/derive.rs` (new, slice 4) | `MethodDescriptor`, `ProjectedRoot`, `DerivationKey`, `DerivationRecord`, `DerivationBundle`, `DerivationIndex`, `TrustPolicy`, `Derive`, `derive_or_reuse`, `project_view` |
| eprfs-core | `src/hydrate.rs`, `src/overlay.rs` (new, slices 3 and 7) | `Hydrator`, `Overlay` |
| eprfs-core | `src/probe.rs` (new, slice 1 step b) | `MaterializationReading` |
| eprfs-serve | `src/lib.rs` (new crate, eprfs workspace; depends on eprfs-core and elohim-seam-contracts) | `TierEndpoint`, `ItemAbsenceLabel`/`ReachReasonLabel` (the `ReasonLabel` faces); feature `testing`: `assert_tier_contract`, `CountingTier` |
| elohim-placement-core | `src/{kind,need,offer,eligibility,rank,adopt,decide,floor,explain,params,ceiling,manifest}.rs` (follow-on) | every floor and ceiling interface |
| elohim-seam-contracts | `src/probe.rs`, `harness/arbitrated.rs` `check_ordering` (follow-on additions) | `Reading<T>`, the list form of the arbitration harness |

### 1.3 Rails (each a test that fails a build)

| Rail | Check | Lives in |
|---|---|---|
| R-E1 eprfs-core's frozen dependency set | `boundary::frozen_dependency_set` parses `[dependencies]` of `include_str!("../Cargo.toml")` and asserts it is exactly {async-trait, bytes, serde, serde_json, serde_bytes, serde_ipld_dagcbor, thiserror, cid, multihash-codetable, elohim-seam-contracts (feature serde)}. seam-contracts is admitted (I2 revision) because ruling 7's shared type must be nameable by every seam and it is a std/alloc leaf with zero first-party dependencies, whose own boundary test keeps it one. An exact set, not a deny-list: a needle scan misses `tokio.workspace = true` and trips on the existing `tokio` dev-dependency | `eprfs-core/src/lib.rs` (new test module) |
| R-E2 no I/O, runtime or clock in eprfs-core | `boundary::no_io_in_core` scans every module, using split literals as ark-core does, for `std::fs`, `File::`, `std::net`, `std::thread`, `spawn`, `Instant::now`, `SystemTime::now`; `scan_covers_every_module` keeps the list honest. Async fns only await an `EprfsStorage`, `DerivationIndex` or `Derive` port | same |
| R-E3 direction | eprfs-storage, eprfs-local, eprfs-serve and eprfs-agent never depend on `elohim-storage`; elohim-storage may depend on `eprfs-core` and `eprfs-serve` (including eprfs-serve's `testing` feature as a dev-dependency), never `eprfs-storage` or `eprfs-local`. The tier contract harness therefore lives in eprfs-serve, and elohim-storage writes its own `ForwardGate` over tokio rather than reusing `SemaphoreGate` (A19) | `eprfs-storage/tests/boundary.rs` and elohim-storage's own boundary test |
| R-P1 placement-core's direct dependencies | exactly {elohim-epr, elohim-epr-rea (default-features off), elohim-compute (default-features off), elohim-seam-contracts (feature serde), serde, serde_ipld_dagcbor, cid, sha2, thiserror}. chrono, ts-rs and ed25519 arrive transitively through elohim-epr, as they do for ark-core; the rail governs what this crate names, R-P2 governs what it calls | `placement/core/src/lib.rs` |
| R-P2 no clock | no `Utc::now`, `SystemTime::now` or `Instant::now` in source; time arrives as `EpochSecs` | same |
| R-P3 no floats in ranking | no `f32`, `f64` or `partial_cmp` in `rank.rs` or `params.rs`; weights are integer basis points. `elohim-peer-fabric/src/score.rs` sorts floats with `partial_cmp(..).unwrap_or(Equal)`, so ties keep input order: fine for choosing where I fetch from, a defect for choosing who carries | same |
| R-P4 device facts never serialize | `compile_fail` doctest: `serde_json::to_string(&DeviceFacts { .. })` does not compile | `placement/core/src/offer.rs` |
| R-P5 ranking sees only eligible candidates | `Eligible` has a private field; `compile_fail` doctest that it cannot be built outside the crate | `placement/core/src/eligibility.rs` |
| R-P6 drafts never reach the floor | no `From<Draft> for InForce`; `compile_fail` doctest | `placement/core/src/ceiling.rs` |
| R-P7 every ranking runs the contract | a source scan pairs each `impl Ranking for` with an `assert_ranking_contract` call in the tests | `placement/core/src/rank.rs` |
| R-P8 one ranking home | after the move, elohim-storage holds no `impl PlacementStrategy for` and no `impl Ranking for` outside the shim in `reconcile/placement.rs` (test modules excepted), and no greedy-diversity loop in `src/services/`. Not `fn rank(`: `services/standing.rs`, `services/feedback_projector.rs` and `db/models.rs` hold unrelated rank functions (D19) | elohim-storage test |
| R-A1 ark-core stays pure | the existing `boundary::no_runtime_or_io_deps` keeps passing with eprfs-core and placement-core added | `ark/core/src/lib.rs` (existing) |
| R-T1 `@elohim/eprfs` stays pure | imports neither `@elohim/storage-client` nor any DOM or Angular module | `eprfs-ts` lint config |
| R-T2 the service worker caches only verified bytes | ESLint `no-restricted-syntax` forbids `cache.put` in `apps-sw.ts` outside three writers: `CacheTier.put`, the head memory writer, and `ArchiveWriter.put`, which only `extractZip` calls after `verifyAddress` has passed on the whole archive (H1). A lint cannot stop a same-origin page writing Cache Storage, so `CacheTier` also verifies every hit (H5) | `app/elohim-app` eslint |
| R-T3 one verifier per language | Rust `BlobCid::verifies`; TypeScript `verifyAddress` in `@elohim/epr`. `raw-cid-verify.ts` stays a thin raw-only wrapper (`verifyRawSha256` = codec raw or legacy address, then `verifyAddress`; `expectedSha256Hex` kept, raw only), because `content.service.ts`'s public-content guard and `raw-cid-verify.spec.ts` depend on dag-cbor failing closed there (H13) | both |
| R-G1 the needs block is placement-neutral | §2.G contract | `scripts/test-manifest-needs.mjs` |

**Which layers never do I/O:** eprfs-core, elohim-placement-core, elohim-ark-core, and the pure
`@elohim/eprfs` modules. placement-core has no async at all.

### 1.4 The CID carriers

Four Rust types carry a CID across these seams. They name the same address, a CIDv1 over sha2-256, and
differ in which crate may name them and how they serialize.

| Type | Crate | What it is | Serialized as |
|---|---|---|---|
| `cid::Cid` (0.11) | `cid` | the address itself; `Copy, Ord, Hash` | bytes in every format through its own serde. So every `Cid` field in placement-core carries `#[serde(with = "cid_serde")]`: a `bafy…` string in JSON and tag-42 bytes in DAG-CBOR (G1) |
| `BlobCid` | eprfs-core `address.rs` (existing) | a newtype over `Cid`, built only by hashing (`compute`, `compute_raw`) or parsing (`parse`), and verifiable (`verifies`) | the canonical base32 string **in every format**, including DAG-CBOR (`serde(into = "String", try_from = "String")`) |
| `BlobLink` | eprfs-core `address.rs` (existing) | the same address in a link position | a string in JSON and tag 42 in DAG-CBOR. Tree entries, `TreeView.root` and the batch route use it |
| `CidString` | placement-core `offer.rs` (new) | a CID's string form, for set members, map keys and fields that sit inside internally tagged enums (G1) | a string in every format |

Placement-core names `Cid` and `CidString` only, because R-P1 keeps eprfs-core out of its dependencies.
The conversions are these:

```rust
// BlobCid → Cid: the existing accessor, then a copy (Cid is Copy).
let cid: Cid = *blob_cid.as_cid();
// Cid → BlobCid: no infallible conversion exists, so a CID is never re-labelled without its codec
// being checked. Go through the string form:
let blob_cid = BlobCid::parse(&cid.to_string())?;
// BlobCid ↔ BlobLink: From in both directions (existing).
// Cid ↔ CidString (placement-core, new):
impl From<&Cid> for CidString { /* cid.to_string() */ }
impl CidString { pub fn to_cid(&self) -> Result<Cid, PlaceError>; pub fn as_str(&self) -> &str; }
```

The `ManifestContext.closure_key` that ark-core builds (§G) is therefore
`Some(NeedKey::TreeView(*tree_view.cid()?.as_cid()))`.

`cid_serde` is exported by `elohim_epr_rea::index::cid_serde`; placement-core uses this same
implementation for every CID field. It must not keep a copy of its own: a copy can drift from
epr-rea's encoding, and the encoding is what the CIDs are computed over. The integration pass
removed the temporary copy and made the existing module public without changing its encoding.

---

## 2. Seams

### 2.0 The unanswered type and the status channel (R7)

**Owner:** `crates/seam-contracts/src/unanswered.rs` (follow-on; alloc only, like `residual.rs`), the
purest crate every seam already depends on once R-E1 admits it (§1.3). Its wire form is
`objects/unanswered.schema.json` (lane S); the TypeScript twin is generated from that schema, never
hand-written.

```rust
// crates/seam-contracts/src/unanswered.rs
/// Ruling 7: every refusal, absence or "not carried" across seams A–I. Generic over the seam's own
/// typed reason, so each seam keeps its vocabulary (`R: ReasonLabel`); the three parts are fields, not
/// Options, so a constructor that omits one does not compile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize),
           serde(rename_all = "camelCase"))]
pub struct Unanswered<R> {
    pub why: R,
    pub come_back: ComeBack,
    pub meanwhile: Meanwhile,
    /// Set when this is a repeat ask before `come_back`: the answer escalates rather than repeats.
    pub escalation: Option<Escalation>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize),
           serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "when"))]
pub enum ComeBack {
    At { epoch_secs: u64 },          // a time a tier or gate gave
    OnEvent { event: String },       // e.g. "tier-reachable", "posture-changed", "parameters-declared"
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize),
           serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "local"))]
pub enum Meanwhile {
    UseHeld { what: String },        // what is already local, named
    RunHere,                         // the device can do it itself
    BuiltInInForce,                  // a default is answering in its place
    Nothing { because: String },     // said, never implied
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize),
           serde(rename_all = "camelCase"))]
pub struct Escalation { pub asked: u32, pub changed: String, pub who_can_be_asked: Vec<String> }
impl<R> Unanswered<R> {
    pub fn new(why: R, come_back: ComeBack, meanwhile: Meanwhile) -> Self;
    /// A repeat before `come_back`: same three parts plus what changed and who can be asked.
    pub fn escalate(self, changed: String, who_can_be_asked: Vec<String>) -> Self;
    pub fn map_why<Q>(self, f: impl FnOnce(R) -> Q) -> Unanswered<Q>;
}

/// The channel a pending answer travels on (R7): a pull cursor in a pure core, a stream adaptor at
/// the edge (a `futures::Stream` in an adapter crate, an `AsyncIterable` in TypeScript). Terminal
/// states are `Answered` and `Withdrawn`; every other status is followed by another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingStatus<T, R> {
    Queued,
    RetryingAt { epoch_secs: u64 },
    Unanswered(Unanswered<R>),       // not terminal: come_back says when it will change
    Answered(T),                     // terminal
    Withdrawn,                       // terminal: the need or request was withdrawn
}
pub trait StatusCursor {
    type Value; type Reason;
    /// `None` means no change since the last poll: a quiet channel costs nothing at rest.
    fn poll_status(&mut self, now_epoch_secs: u64) -> Option<PendingStatus<Self::Value, Self::Reason>>;
}
```

**`Answer<T>`, the state every seam reads facts through** (existing, `crates/seam-contracts/src/answer.rs`,
unchanged). `Unanswered<R>` says why a request was not met; `Answer<T>` says whether a fact the decider
needs arrived. It is used below as `Candidate.live`, `admitted` and `consent` (§D.3), the `Governor`
grant (§D.4), `Carrying.honored` (§D.6), and `SetSource::set` and `ParameterHead::declarations` (§E).

```rust
// crates/seam-contracts/src/answer.rs (existing; shown for reference)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Answer<T> {
    Present(T),   // the answer arrived and carries a value
    Absent,       // the answer arrived and says the thing does not exist: an OBSERVED absence
    Unreachable,  // no answer arrived: nothing is established in either direction
}
impl<T> Answer<T> {
    pub fn observed_absence(value: Option<T>) -> Self;   // None was observed → Absent
    pub fn from_local_get(value: Option<T>) -> Self;
    pub fn from_result<E>(value: Result<Option<T>, E>) -> Self;   // Err → Unreachable
    pub fn state(&self) -> AnswerState;                // Present | Absent | Unreachable, a ReasonLabel
    pub fn as_ref(&self) -> Answer<&T>;
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Answer<U>;
}
```

The three states are not two. Placement reads them as follows; every row is pinned by the D and E
contracts:

| Field | `Present(v)` | `Absent` | `Unreachable` |
|---|---|---|---|
| `Candidate.live` | `v == true` passes; `false` is ineligible `Unknown { input: "liveness" }` | ineligible `Unknown { input: "liveness" }` | ineligible `Unknown { input: "liveness" }`, never read as dead |
| `Candidate.admitted[receiver]` | compared with the need's reach (§D.4) | ineligible `OutsideReach` | ineligible `Unknown { input: "reach" }` |
| `Candidate.consent[receiver]` | `NotNeeded` passes; `Granted` is checked against its bounds | ineligible `NoConsent` | refuses (§D.4: "an Unreachable grant refuses"), as `Unknown { input: "consent" }` |
| `Carrying.honored` | `true` counts as a holder and an incumbent; `false` does not | not a holder (an observed loss) | not counted as a holder **and** not read as "not honored": no release or move is based on it |
| `SetSource::set` | the set, verified by `InForce::from_head` | `ParamRefusal::Unverifiable { input: "set" }` | the same; `select_in_force` keeps the last verified set or the built-in |
| `ParameterHead::declarations` | the declarations, elected after verification | the last verified set, or the built-in (§E contract 3) | the same |

The derived `serde` form is a convenience inside Rust (externally tagged, `"Absent"`, `{"Present": v}`);
the wire form of an answer is `objects/answer.schema.json`, and no type in this document puts an
`Answer` on the wire directly.

How each seam constructs it: A, `Unanswered<ItemAbsence>` per item at the sink; B,
`Unanswered<ItemAbsence>` inside the walk's absences and `TreeError::Unanswered`; C,
`Unanswered<DerivationUnanswered>`; D, `Unanswered<IneligibleReason>` and `Unanswered<NotCarriedReason>`;
E, `Unanswered<ParamRefusal>`; F, D's types for the task kind plus `PendingStatus<TaskOutcome, _>`; H, the
generated `Unanswered` in TypeScript and `Placement.watch`; I, a probe's `Unreachable` arm with its
`AnswerReason`. Contract (`seam-contracts/tests/unanswered.rs`): every seam's reason type is a
`ReasonLabel` and passes `check_reason_labels`; a cursor whose inputs did not change returns `None`; a
cursor reaches a terminal state only through `Answered` or `Withdrawn`.

### A. The storage tier and the tier chain, with reach injected

**Owner:** eprfs-core, `src/storage.rs` (trait, additive), `src/tier.rs` and `src/reach.rs` (new).
Every name of a reach value stays outside eprfs: the core sees only the verdicts of injected policies.

**A.1 The trait gains calls, all defaulted**, so `MemoryStorage`, `DirectoryStorage`, the slice-1 test
doubles, brit-bridge and eprfs-local compile unchanged.

```rust
// eprfs-core/src/storage.rs  (slice 2, additive; FetchPolicy gains Copy and Default, #[default] LocalOnly)

/// Existing, unchanged: the bytes a store returns, owned and reference-counted (`bytes::Bytes`), with
/// the CID they were asked for. A store does not promise that `bytes` verify against `cid`: the chain
/// and the default `fetch_blobs` check `h.cid == item.cid && item.cid.verifies(&h.bytes)` themselves.
#[derive(Debug, Clone, PartialEq)]
pub struct BlobHandle { pub cid: BlobCid, pub bytes: Bytes }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItem { pub cid: BlobCid, pub path: TreePath }   // the object's place under the root

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BatchOptions {
    pub policy: FetchPolicy,
    /// The rest of the requester's tier list. A forwarding tier forwards a miss only to these, in
    /// order (spec §12.3), and reports every tier it asked in `ItemAbsence::NotHeld.tried`.
    pub via: Vec<TierId>,
}

#[async_trait]
pub trait EprfsStorage: Send + Sync {
    // resolve_epr, has_blob, fetch_blob, put_blob, put_blob_verified, publish_attestation: unchanged.

    /// The identity this store stamps on what it answers (A2). A leaf store is `TierId::own()`, which
    /// is indexical and never serialized; a serving adapter returns its agent key.
    fn tier_id(&self) -> TierId { TierId::own() }

    /// One object named by its place under a root. A leaf store ignores `root`, `path` and `via`; a
    /// serving or forwarding tier uses them for its reach and membership checks. The walk's internal
    /// `fetch_checked` calls this from slice 2, so every node fetch carries its root and the options a
    /// forwarding tier needs (A8).
    async fn fetch_at(&self, root: &BlobCid, item: &BatchItem, opts: &BatchOptions) -> Result<BlobHandle> {
        let _ = root;
        self.fetch_blob(&item.cid, opts.policy).await
    }

    /// Many objects under one root (R6: `fetch_blobs(root, …)`): out of order and resumable (re-send
    /// the items the sink did not see). Each object is verified before the sink sees it; absences are
    /// named to the sink, never raised as errors. Err is reserved for the tier failing as a whole.
    async fn fetch_blobs(&self, root: &BlobCid, items: &[BatchItem], opts: &BatchOptions,
                         sink: &mut (dyn BlobSink + Send)) -> Result<BatchReport> {
        // Each requested item is answered exactly once; bytes are fetched once per CID and fanned out
        // to every item carrying that CID (A13). This body is the default as written, not a sketch;
        // `group_by_cid` is the private helper declared after the trait.
        let me = self.tier_id();
        let mut report = BatchReport::default();
        for (cid, group) in group_by_cid(items) {                // stable, first-occurrence order
            match self.fetch_at(root, &group[0], opts).await {
                Ok(h) if h.cid == *cid && cid.verifies(&h.bytes) => {
                    for item in group {
                        report.arrived += 1; report.bytes += h.bytes.len() as u64;
                        if sink.arrived(item, h.bytes.clone(), &me) == SinkControl::Stop { report.stopped = true; return Ok(report); }
                    }
                }
                Ok(_) => for item in group { report.absent += 1; sink.absent(item, ItemAbsence::integrity(&me)); },
                Err(e) => for item in group { report.absent += 1; sink.absent(item, ItemAbsence::from_error(&e, &me)); },
            }
        }
        Ok(report)
    }

    // Depth-0 queries (B5), defaulted to the free functions, which are `S: EprfsStorage + ?Sized` from
    // slice 2 so these defaults work through `dyn EprfsStorage`. A tier that links eprfs-core and can
    // answer in one round trip overrides them (HttpTier over the query routes, §A.6).
    async fn query_read_path(&self, root: &BlobCid, path: &TreePath, opts: &BatchOptions, limits: &TreeLimits)
        -> TreeResult<ReadLeaf> { crate::walk::read_path(self, root, path, opts, limits).await }
    async fn query_list_dir(&self, root: &BlobCid, path: &TreePath, opts: &BatchOptions, limits: &TreeLimits)
        -> TreeResult<DirListing> { crate::walk::list_dir(self, root, path, opts, limits).await }
    async fn query_diff(&self, old: &BlobCid, new: &BlobCid, view: &View, opts: &BatchOptions, limits: &TreeLimits)
        -> TreeResult<Vec<TreeChange>> { crate::walk::diff_trees(self, old, new, view, opts, limits).await }
}

/// Private to storage.rs. Groups items by CID: groups in the order each CID first occurs, items inside a
/// group in input order. A CID appearing at several paths is one group, so its bytes are fetched once.
fn group_by_cid(items: &[BatchItem]) -> Vec<(&BlobCid, Vec<&BatchItem>)>;

pub trait BlobSink {
    /// Called only with bytes that verified against `item.cid`. `Bytes` passes through without a copy.
    fn arrived(&mut self, item: &BatchItem, bytes: Bytes, from: &TierId) -> SinkControl;
    /// R7: every absence carries its three parts.
    fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>);
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkControl { Continue, Stop }

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BatchReport { pub arrived: usize, pub absent: usize, pub bytes: u64, pub stopped: bool }
```

New `EprfsError` variants (`src/error.rs`, in the `0.2.0` release that marks the enum
`#[non_exhaustive]`):

```rust
#[error("unanswered: {cid}")]   Unanswered { cid: BlobCid, why: Unanswered<ItemAbsence> },
#[error("root required for a fetch beyond the own store: {0:?}")]   RootRequired(BlobCid),   // A18
```

From slice 2 the walk's free functions take `opts: &BatchOptions` where slice 1 took `policy:
FetchPolicy`; `impl From<FetchPolicy> for BatchOptions` keeps every slice-1 call site a one-token change
(`&policy.into()`), recorded in §6. `tree.rs::fetch_checked` (now taking `root`, `path` and `&BatchOptions`) maps `EprfsError::Unanswered`
to the new `TreeError::Unanswered { cid, path, why }` (B2), keeps `BlobNotFound`/`BlobNotLocal` as
`TreeError::Incomplete`, and keeps every other error as `TreeError::Storage`. The walk continues on both
`Incomplete` and `Unanswered`: `Incomplete`, and `Unanswered` whose `why` is `NotHeld`, become
`AbsentReason::NotHeld { tried }`; every other `why` becomes `AbsentReason::Withheld` (absence not
established, §2.B). `read_path`, `list_dir` and `Hydrator::read` return `TreeError::Unanswered` with the
three parts instead of a `String` reason (B1).

**A.2 Tiers, absences and the order of asking.**

```rust
// eprfs-core/src/tier.rs  (new, slice 2; pure)
/// Who answered. `own` is indexical: it means "this store" to whoever holds it, so it is never
/// serialized (its Serialize refuses) and every serving adapter stamps its `self_id` instead; a
/// requester-side adapter rewrites any `own` in a remote answer to the dialled TierRecord's id (A2).
/// An agent key (uhCAk…) when a cross-signed binding resolves one; otherwise the transport id the tier
/// was dialled by. Never read as attribution: it decides where to dial and what to count, never whom to
/// credit (`elohim-storage/CLAUDE.md`'s attribution cut; A11).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TierId(TierIdForm);
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum TierIdForm { Own, Agent(String), Transport(String) }
impl TierId {
    pub fn own() -> Self; pub fn agent(key: impl Into<String>) -> Self; pub fn transport(peer_id: impl Into<String>) -> Self;
    pub fn is_own(&self) -> bool; pub fn as_str(&self) -> &str;
}
/// Own returns `Err(S::Error::custom("tier id `own` is indexical and never serialized"))`: a serde error
/// the caller handles, never a panic. `TierRecord` has public fields and no validating constructor, so an
/// `own` id in a record surfaces as that error at the wire boundary (serving adapters stamp `self_id`
/// before they serialize; contract 9 asserts no answer carries `own`).
impl Serialize for TierId { /* "agent:<key>" | "transport:<peer id>"; Own is the error above */ }
impl<'de> Deserialize<'de> for TierId { /* refuses "own" and any string without one of the two prefixes */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TierClass { Own, PersonDevice, Household, Hub, HolonHub, Pool, DoorwayPeer, Doorway }

/// Where the requester learned of a tier. Both sources are legitimate (R6a): held records (roster,
/// commitments, the person's devices, spaces) and the doorways the requester ingresses through,
/// which are plural participants. Plurality is what a person relies on, so the record keeps every
/// source that named it and the probe can count distinct doorways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "source")]
pub enum TierSource { HeldRecord { record: String }, Doorway { doorway: TierId } }

/// A dial hint, never identity: a base URL or a multiaddr.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "form")]
pub enum TierAddress { Url { base: String }, Multiaddr { addr: String } }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierRecord {
    pub id: TierId,
    pub class: TierClass,
    pub sources: Vec<TierSource>,             // non-empty by construction (A14)
    pub endpoint: Option<TierAddress>,        // where to dial (A11)
    pub standing: Option<ReachLevel>,         // the widest reach this tier is admitted to read, as
                                              // Reach::openness(), from a held record or the doorway entry;
                                              // None = unknown (A5, H7)
    pub proximity: Option<Proximity>,
}
impl TierRecord {
    /// Merges a duplicate by id: sources are unioned; a class conflict keeps the narrower class (the
    /// lower `TierClass`), and the conflict is counted.
    pub fn merge(&mut self, other: TierRecord);
}
/// No Ord derive: ProximityOrder's comparator is explicit (A15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proximity {
    pub network_rank: u8,                     // lan 0 · wan 1 · relay 2
    pub warm_roots: Vec<BlobCid>,             // roots this tier declared warm (per content, not per tier)
    pub serves_extracted: bool,
    pub rtt_ms: Option<u32>,
    pub last_seen_ms: Option<u64>,
}

/// Pure and stable. The own store is always first and is not an input: the chain holds it apart.
/// Duplicates by id merge through `TierRecord::merge`. The default `ProximityOrder` compares, in order:
/// class; warm for `root` first; network rank; measured rtt ascending, unknown last; more recent
/// last_seen first; id. It carries scorePeerForSw's factors (lan > wan > relay, per-content warmth,
/// recency, extracted serving) as an ordering, not its additive score, and that difference is stated:
/// a warm WAN peer no longer outranks a cold LAN peer of the same class unless warmth is ranked first,
/// which this comparator does (A15, H17).
pub trait TierOrder: Send + Sync { fn order(&self, root: &BlobCid, records: &mut Vec<TierRecord>); }
pub struct ProximityOrder;

/// The typed reason inside `Unanswered<ItemAbsence>`. The come-back and the meanwhile ride beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "kind")]
#[non_exhaustive]
pub enum ItemAbsence {
    NotHeld { tried: Vec<TierId> },                // observed absent at every tier listed
    Refused { by: TierId, refusal: ReachRefusal }, // reach said no, by name
    OverBudget { by: TierId, rule: String },       // never queued silently; the retry time is come_back
    Unreachable { tier: TierId },                  // no answer: absence NOT established
    MembershipUnknown { by: TierId },              // the tier could not establish the path's membership:
                                                   // NOT final, the chain advances (A3)
    Integrity { from: TierId },                    // bytes failed their address; never stored
}
impl ItemAbsence {
    /// The full mapping (A17): `EprfsError::Unanswered { why }` → why, unchanged; a corrupt object
    /// (DirectoryStorage's "existing content object is corrupt") → Integrity; Io / Storage →
    /// Unreachable { tier: at } (absence not established); only BlobNotFound / BlobNotLocal →
    /// NotHeld { [at] }. Each arrives wrapped with its come_back and meanwhile.
    pub fn from_error(e: &EprfsError, at: &TierId) -> Unanswered<ItemAbsence>;
    pub fn integrity(from: &TierId) -> Unanswered<ItemAbsence>;
}

/// Bounded upstream (spec §12.3 rule 3). A synchronous try-acquire, so the core needs no runtime.
/// eprfs-storage's `SemaphoreGate` implements it over `tokio::sync::Semaphore::try_acquire_owned`
/// plus a byte bucket; elohim-storage writes its own over tokio (R-E3). On a person's own device the
/// budget is the person's; on a shared tier the adapter reads it from the space's charter
/// (§2.D.7 `Charter::forwarding`), per requester (`on_behalf`, A4).
pub trait ForwardGate: Send + Sync {
    fn admit(&self, on_behalf: &Requester, to: &TierId, bytes_hint: Option<u64>)
        -> core::result::Result<ForwardPermit, OverBudget>;
}
pub struct ForwardPermit { _release: Box<dyn Send + Sync> }        // dropping it releases the slot
impl ForwardPermit { pub fn new(release: impl Send + Sync + 'static) -> Self; }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverBudget { pub retry_after_ms: u64, pub rule: String }
```

The `ReasonLabel` faces (A10) live in eprfs-serve, which depends on eprfs-core and on
elohim-seam-contracts: local `ItemAbsenceLabel` and `ReachReasonLabel` enums with exhaustive
`From<&ItemAbsence>` / `From<&ReachReason>`, run through `check_reason_labels`. The orphan rule forbids
implementing the foreign trait on eprfs-core's types from eprfs-serve, and the local enums avoid it.

**A.3 Reach on both sides, as injected policy.**

```rust
// eprfs-core/src/reach.rs  (new, slice 2)
/// Who asks, as the serving transport authenticated them. Built only at a transport adapter; the
/// seam registry pins the construction sites. Device facts never appear here (R6).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Requester { agent: Option<String>, custody: KeyCustody }
impl Requester {
    pub fn authenticated(agent_key: impl Into<String>, custody: KeyCustody) -> Self;
    pub fn anonymous() -> Self;
    pub fn agent(&self) -> Option<&str>;
    pub fn custody(&self) -> KeyCustody;
}
/// Spec §10.11: the actor's key-custody tag rides with every reach check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyCustody { DeviceHeld, HubHeld, DoorwayHeld, SponsoredAgent, Anonymous }

/// Every non-admission is a refusal; there is no admit-on-error path (spec §18, `http.rs:4650`).
/// To a requester outside reach, a missing object and a private one get the same refusal, naming the
/// requester's missing standing and not the object (spec §12.3 rule 2). The tier's own counters keep
/// the two apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReachRefusal { pub reason: ReachReason, pub needs_standing: Option<String> }  // opaque to eprfs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReachReason { OutsideStanding, NoRecord, NotUnderRoot, IdentityRequired, AuthorizerUnavailable }

/// Serving side. Tree nodes inherit their root's reach (spec §12.3 rule 2); an object outside every
/// root is judged by its own records. Sync: reach is read from records the tier already holds, and an
/// unknown record refuses (`NoRecord`) rather than fetching.
pub trait ReachGate: Send + Sync {
    fn admit_root(&self, who: &Requester, root: &BlobCid) -> core::result::Result<(), ReachRefusal>;
    fn admit_object(&self, who: &Requester, cid: &BlobCid) -> core::result::Result<(), ReachRefusal>;
}

/// Requester side: may this tier be ASKED for objects under this root? The read pattern is itself a
/// signal, so a private root's miss never leaves the tiers its reach admits (spec §12.3 rule 2). The
/// root's reach is the host's (the head plane records reach beside the root, slug → {root, reach});
/// eprfs passes it opaquely.
pub trait AskPolicy: Send + Sync {
    fn may_ask(&self, tier: &TierRecord, root: &BlobCid, root_reach: Option<ReachLevel>) -> bool;
}
/// `Reach::openness()` as an opaque ordinal, so eprfs-core names no reach value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReachLevel(pub u8);
```

The gate's refusal is a local diagnostic. `TierEndpoint` normalizes every denied root to
`OutsideStanding` with `needsStanding: null` before returning a batch, read, list or diff answer.
Presence returns `Missing`. An unknown root and a protected root must have the same refusal
envelope, including its come-back and meanwhile fields; a private group's standing label must
not leave with that refusal. The serving counters count refused batches, not individual gate
reasons. Detailed local diagnostics remain the host's responsibility.

**A.4 The chain.** The requester composes it with its own store first.

```rust
// eprfs-core/src/tier.rs
/// Builds a tier from a record, using `record.endpoint`. TS has the same trait (§H.1).
pub trait TierConnector: Send + Sync { fn connect(&self, record: &TierRecord) -> Option<Arc<dyn EprfsStorage>>; }

pub struct TierChain {
    own: Arc<dyn EprfsStorage>,
    outer: Vec<(TierRecord, Arc<dyn EprfsStorage>)>,
    ask: Arc<dyn AskPolicy>,
    gate: Arc<dyn ForwardGate>,
    who: Requester,                                   // the requester itself, for its own reads
    stats: ChainStats,                                // atomics per TierId actually dialled: hits, misses,
                                                      // integrity, write-back failures; never attribution
}
impl TierChain {
    pub fn new(own: Arc<dyn EprfsStorage>, records: Vec<TierRecord>, root_hint: &BlobCid, order: &dyn TierOrder,
               connect: &dyn TierConnector, ask: Arc<dyn AskPolicy>, gate: Arc<dyn ForwardGate>,
               who: Requester) -> Self;
    pub fn own(&self) -> &Arc<dyn EprfsStorage>;
    pub fn tiers(&self) -> impl Iterator<Item = &TierRecord> + '_;
    pub fn stats(&self) -> ChainStatsSnapshot;
    /// The serving-side entry (A4): forward a miss on behalf of an inbound requester. The gate is asked
    /// with `on_behalf` (a per-requester budget on a shared tier), and the outer list is intersected with
    /// `opts.via` in `via`'s order. The next tier sees this tier's own identity, so it judges reach for
    /// the forwarder; the endpoint has already judged the requester (A.5 step 1).
    pub async fn forward_blobs(&self, on_behalf: &Requester, root: &BlobCid, root_reach: Option<ReachLevel>,
                               items: &[BatchItem], opts: &BatchOptions, sink: &mut (dyn BlobSink + Send))
        -> Result<BatchReport>;
}
#[async_trait]
impl EprfsStorage for TierChain {
    // fetch_blob(cid, LocalOnly): own only. fetch_blob(cid, FetchIfMissing) with outer tiers: own, then
    //   Err(RootRequired(cid)), never a silent local-only answer and never BlobNotFound (A18).
    // has_blob: own; when own misses and outer tiers exist, `Unknown`, never `Missing` (A18).
    // fetch_at: a one-item fetch_blobs (A8). fetch_blobs: the contract below.
    // put_*, publish_attestation: to `own`.
}
```

**Contract** (`eprfs-storage/tests/tier_chain.rs`, run by eprfs-serve's `testing`-feature
`assert_tier_contract` harness against every implementor, using `CountingTier` doubles; elohim-storage
runs the same harness as a dev-dependency, R-E3):

1. **Own store first, without `has_blob`.** The chain tries `own.fetch_at(.., LocalOnly)`. A hit
   makes zero outer calls. It never calls `has_blob` (finding 6).
2. **`LocalOnly` stays local** and never waits on a tier: absences come back at once.
3. **A pathless read never leaves the device.** `TierChain::fetch_blob` and `has_blob` answer from
   `own` only, and say so (`RootRequired`, `Unknown`) rather than reporting a false absence. Outer tiers
   are reached only through `fetch_blobs` (and `fetch_at`, which is a one-item `fetch_blobs`), which
   always carry a root and `via`.
4. **Ask before asking.** A tier for which `may_ask` is false sees zero calls. (4b) With a `public`
   standing tier and an `intimate` root, `may_ask` is false; with a `household` standing tier and a
   `public` root, it is true.
5. **Strict tiering.** The chain asks the first admitted, untried outer tier through `fetch_blobs`,
   passing the remaining admitted tiers as `opts.via`. That tier forwards under its own gate through
   `forward_blobs`. The chain never re-asks a tier named in `NotHeld.tried`; it continues with the next
   admitted tier not yet tried. `Refused` and `OverBudget` are final for the item (a refusal does not
   fan out the read pattern; over budget is answered with its come-back time, not routed around).
   `Unreachable` and `MembershipUnknown` advance to the next tier.
6. **Verify at every hop.** Bytes are accepted only when `handle.cid == item.cid &&
   item.cid.verifies(&bytes)`. Bad bytes are `Integrity`, counted against the dialled `TierId`, never
   stored.
7. **Store before return.** Forwarded bytes are written with `own.put_blob_verified` before
   `sink.arrived`, so a second read is an own hit. **A failed write-back still delivers the verified
   bytes** and is counted: the read matters more than the cache.
8. **Bounded upstream.** Past `ForwardGate`, the item is `OverBudget { rule }` with
   `come_back: At(retry)`.
9. **Named failure.** `NotHeld.tried` lists every tier asked, in order, by the ids the requester
   dialled; no answer that reaches a sink contains `own` from another party.
10. **Batch equivalence.** Each requested `BatchItem` is answered exactly once (arrived or absent);
    bytes are fetched once per CID and fanned out to every item with that CID; items with unrequested
    CIDs are dropped (A13).
11. **A leaf store ignores policy.** `DirectoryStorage` and `MemoryStorage` treat `FetchIfMissing` as
    `LocalOnly`, which is what they do today. The chain is the only thing that honours the policy.
12. **Every absence carries R7's three parts.**

**A.5 The serving side.**

```rust
// eprfs-serve/src/lib.rs  (new crate in the eprfs workspace; depends on eprfs-core and elohim-seam-contracts)
pub struct TierEndpoint<G> {
    self_id: TierId,                          // its agent key; stamps every absence it answers (A2)
    store: Arc<dyn EprfsStorage>,             // the same store the upstream chain's `own` is (A4)
    gate: G, upstream: Option<TierChain>, limits: TreeLimits, counters: ServeCounters,
}
impl<G: ReachGate> TierEndpoint<G> {
    /// Refuses (panics in debug, errors in release) when `upstream`'s own store is not `store`.
    pub fn new(self_id: TierId, store: Arc<dyn EprfsStorage>, gate: G, upstream: Option<TierChain>,
               limits: TreeLimits) -> Result<Self>;
    /// 1. `gate.admit_root(who, root)`, once per batch (R1), or every item is Refused by the same name;
    /// 2. membership per item: resolve `item.path` under `root`; spine nodes this tier does not hold are
    ///    fetched through `upstream` (verified and written back), and spine nodes shared across the batch
    ///    are decoded once (A3). Its link must equal `item.cid`, else NotUnderRoot, which is said only
    ///    for a path proved absent in a held node; with no upstream and the spine not held, the item is
    ///    MembershipUnknown, which is not final for the requester's chain;
    /// 3. own store, else `upstream.forward_blobs(who, root, …, opts)` (restricted to `opts.via`),
    ///    verify, write back, serve.
    pub async fn serve_batch(&self, who: &Requester, root: &BlobCid, items: &[BatchItem], opts: &BatchOptions,
                             sink: &mut (dyn BlobSink + Send)) -> Result<BatchReport>;
    pub async fn serve_read(&self, who: &Requester, root: &BlobCid, path: &TreePath) -> TreeResult<ReadLeaf>;
    pub async fn serve_list(&self, who: &Requester, root: &BlobCid, path: &TreePath) -> TreeResult<DirListing>;
    /// Admits BOTH roots and refuses the whole diff if either fails (B6).
    pub async fn serve_diff(&self, who: &Requester, old: &BlobCid, new: &BlobCid, view: &View) -> TreeResult<Vec<TreeChange>>;
    /// `has_blob` for another party: an object reachable only through a root `who` may not read is
    /// `Missing` (spec §12.3 rule 2).
    pub async fn presence(&self, who: &Requester, root: &BlobCid, cid: &BlobCid) -> Result<BlobPresence>;
}
```

`TierEndpoint` uses a store's depth-0 query methods (`query_read_path` and the others); it is not itself
an `EprfsStorage` (B5).

Serving contract (`eprfs-serve/tests/endpoint.rs`): admit once per batch; a stranger asking under an
`intimate` root has every item refused with the same `ReachRefusal`, whether or not the object exists;
a CID not at `item.path` under `root` is `NotUnderRoot` even when the root is public; a cold forwarding
tier with an upstream resolves the spine through it and serves; a leaf also referenced by a public
content row stays servable through that row's own route, and the private tree's paths are not listed;
`serve_diff` with a public `new` and an intimate `old` refuses the stranger, not a diff; over budget names
its rule; no answer carries `own`.

**A.6 Implementors and callers.**

| Implementor | Where | Class |
|---|---|---|
| `DirectoryStorage`, `MemoryStorage` (existing) | `eprfs-storage/src/lib.rs` | Own; test |
| `HttpTier` (replaces the empty `StorageEndpoint { base_url }`) | `eprfs-storage/src/http.rs` (new, feature `http`: `reqwest` 0.12, rustls, stream). Depends on reqwest directly and returns `Bytes`, not through `elohim-storage-client`, whose `get_blob` returns `Vec<u8>` and pulls automerge and a multi-thread runtime. **Never `/blob/{cid}`** (A1): `fetch_blobs` (and `fetch_at`, a one-item batch) uses `POST /eprfs/v1/blobs` with body `{root, items:[{cid,path}], via}`, answered as a CBOR sequence (RFC 8742) of arrived items then one absent trailer, in the wire format given below the table; the depth-0 queries use `POST /eprfs/v1/read`, `/eprfs/v1/list`, `/eprfs/v1/diff` (B5). It rewrites any remote `own` to its record's id. Authenticator (A7): through a doorway, the doorway-validated `X-Agent-Cid` gives `KeyCustody::DoorwayHeld`, accepted only when the request arrived via that doorway; direct to a household or hub, the request is signed by the requester's agent key over `(root, item CIDs, nonce)`, giving `DeviceHeld` or `HubHeld`; otherwise `Requester::anonymous()`. `HttpTier` therefore takes a signer | Household · Hub · Pool · DoorwayPeer · Doorway |
| `BlobStoreTier` | elohim-storage `src/eprfs_tier/blob_store_tier.rs` (follow-on) over `BlobStore`. Preconditions: the BlobStore accepts dag-cbor `bafyrei…` objects and the sha256-to-iroh alias holds (spec §12.3 rule 5, §17 slice 2); tree objects it stores are held off the `/blob/{cid}` route (A1) | Own (peer) |
| `PeerTransportTier` | elohim-storage (follow-on). **One instance per peer** (A6): one TierRecord, one agent key (or transport id), resolved to a libp2p peer through `peer_transport_manifest`. It speaks a new request-response protocol `/elohim/eprfs-blobs/1` carrying `{root, items, via}` and answered by the peer's `TierEndpoint`. It does not wrap `race_fetch`, whose hash-only command, concurrent fan-out over several peers, `Vec<u8>` result and bare `Miss` would bypass `AskPolicy` per tier, strict order and membership; racing, where wanted, is the chain's ordering | Household by key |
| `RootReachGate: ReachGate` | elohim-storage (follow-on). Generalizes `blob_reach::blob_serve_verdict` from content rows to root records and composes `private_reach::private_serve_verdict` | serving |
| `ReachStandingAsk: AskPolicy` | host. Asks a tier only if `openness(root reach) >= openness(tier standing)` (A5: a tier admitted no wider than the root may be asked); the tier's standing comes from `TierRecord.standing`, defaulting by class when unknown (`Own`, `PersonDevice`, `Household`: household; every other class: public). With the root's reach unknown, only `Own`, `PersonDevice` and `Household` | requesting |
| `SemaphoreGate: ForwardGate` | `eprfs-storage/src/budget.rs` (new); elohim-storage writes its own (R-E3) | both |
| `TierEndpoint` host | elohim-storage serves `POST /eprfs/v1/blobs` and the query routes through it (follow-on); the hub hosts the same endpoint (elohim-hub) | — |

**The batch route's wire format** (`POST /eprfs/v1/blobs`; `HttpTier`, the route writer over
`TierEndpoint::serve_batch`, and `PeerTransportTier`'s `/elohim/eprfs-blobs/1` frames all use it). Every
data item is one definite-length CBOR map with text-string keys, encoded as canonical DAG-CBOR like the
rest of eprfs. A CID is tag 42 over a byte string (the `BlobLink` form). A path is an array of byte
strings, one per `TreePath` component; the root path is the empty array. Bytes are a major-type-2 byte
string.

- **Request** (`Content-Type: application/cbor`): one map `{"root": cid, "items": [{"cid": cid,
  "path": path}, …], "via": [text, …]}`, where `via` holds `TierId`s in their serialized form.
- **Response** (`Content-Type: application/cbor-seq`, RFC 8742: data items written back to back with no
  header and no separators) has two parts.
  1. Zero or more **arrived** items, `{"cid": cid, "paths": [path, …], "bytes": bstr}`, one per distinct
     CID. `paths` lists every requested path that these bytes answer, so a CID requested at two paths is
     sent once (contract 10). `HttpTier` calls `sink.arrived` once per `(cid, path)` after it has
     verified the bytes.
  2. Exactly one **absent trailer**, `{"absent": [{"cid": cid, "path": path, "unanswered": u}, …]}`, always
     last and always written, with an empty array when nothing was absent. `u` is `Unanswered<ItemAbsence>`
     in the field and tag names of `objects/unanswered.schema.json` and `objects/item-absence.schema.json`.
- **Telling the parts apart.** The parser reads one data item at a time. A map with the key `"bytes"` is
  an arrived item. A map with the key `"absent"` is the trailer, and the stream ends after it; anything
  after the trailer is a protocol error. A stream that ends **without** a trailer means the tier failed as
  a whole partway through: every requested item not yet answered becomes `Unreachable { tier }`, so the
  walk reads it as `Withheld`, never as `NotHeld` (B3).

A response with one item arrived and one absent, in CBOR diagnostic notation:

```text
{"cid": 42(h'00017112…'), "paths": [[h'61', h'62']], "bytes": h'…'}
{"absent": [{"cid": 42(h'00015512…'), "path": [h'61', h'63'],
             "unanswered": {"why": {"kind": "not-held", "tried": ["agent:uhCAk…"]},
                            "comeBack": {"when": "on-event", "event": "tier-reachable"},
                            "meanwhile": {"local": "nothing", "because": "no tier holds it"},
                            "escalation": null}}]}
```

Error framing beyond a missing trailer is still open (§6).

`tier_records(held, doorways) -> Vec<TierRecord>` builds entries from `DeliveryPeer`: `TierId::agent`
when an active cross-signed binding resolves the agent key, else `TierId::transport(peer_id)`;
`endpoint` from multiaddrs and `httpPort`; `standing` from the peer's declared `commitments` reach tags;
`proximity.warm_roots` from its `warm:<hash>` capabilities.

**Callers:** the walk and its queries, eprfs-local's `LocalMaterializer` once it materializes by root
(its pathless `materialize_entry` under `MaterializationPolicy::FetchMissing` gets `RootRequired` from a
chain, so the projection-manifest materializer is not a chain caller until it does, A18), brit-bridge
(already generic over `S: EprfsStorage`), the ark task runner, and every hub, whose own
`TierEndpoint.upstream` is a `TierChain` over the same store (that is how a household absorbs repeats).

**Extension: a new tier** is one `impl EprfsStorage`, usually `fetch_blobs` and `tier_id`. It neither
verifies (the chain and the endpoint do) nor decides reach (it is served through a `TierEndpoint`). A
new kind of carrier is a new `TierClass` variant, which is a spec change. A new transport is an adapter
behind the trait (spec §12.3 rule 6) with its authenticator named. The tier list is never asked of the
network as a policy; the host builds it from what it holds and what its doorways supplied.

**Worked example.** Matthew's phone reads `index.html` of a household app whose release root R is
declared `intimate`. Records give the chain `[Jessica's laptop (Household, held record, standing
household), hub H (Hub, held record, standing household), two delivery peers named by doorways W1 and W2
(DoorwayPeer, source Doorway, standing public), W1 itself (Doorway, standing public)]`.
`ReachStandingAsk` keeps the laptop and H (intimate is no wider than household) and drops W1 and the
doorway-named peers for R (intimate is narrower than their public standing); for a `public` root it would
keep all of them, ordered by proximity. The own cache misses one leaf. The chain sends a one-item
`fetch_blobs` to the laptop with `via = [H]`. The laptop does not hold the leaf; its endpoint admits the
phone's agent, checks membership from its held spine, and calls `forward_blobs(phone, R, …)`, which asks
H; H hits from its own store; the laptop verifies, writes back and serves; the chain verifies again,
writes to the phone and returns. Counters: own miss 1, laptop forward 1, H hit 1. A stranger connecting
to H is served through `TierEndpoint`; `serve_batch(stranger, R, …)` refuses every item with
`OutsideStanding` before any byte is read, the same answer it gives for a CID H does not hold, and
`/blob/{cid}` does not serve H's tree objects at all.

**Preconditions (spec §17 slice 2):** the sha256-to-iroh alias; the BlobStore keyed for dag-cbor
objects; the two reach defects of spec §18; and a third (A1): `/blob/{cid}` serves any blob no content
row references to an anonymous caller (`blob_reach.rs`, `refs.is_empty()` → `Serve`), so it must refuse
an object held only as a member of a tree whose root is not public, or `BlobStoreTier` keeps tree objects
off that route, leaving `TierEndpoint` the only door to tree bytes. The route's `Authorize` arm
(`http.rs`, above `community`) fails closed for every reader today, which is why `HttpTier` never uses it.

### B. View, walk and query

The slice-1 brief's §1 API is adopted as given, with the R6 change, and it is built in the working
tree's `eprfs-core/src/walk.rs`: `TreePath` (`root`, `from_components`, `from_slash_bytes`,
`components`, `is_root`, `join`, `starts_with`, `byte_len`; it already derives `Ord`), `View` (`whole`,
`new(ProjectionPath…)`, `from_tree_paths`, `paths`, `is_empty`, `placement`), `walk`,
`TreeWalk::{next, stats}`, `walk_collect`, `WalkItem::{Node, Boundary, Absent}`, `WalkedNode`,
`WalkStats { nodes_fetched, nodes_reused, node_bytes, entries_visited }`, `WalkedView::leaves_in_view`,
`read_path` → `ReadLeaf`, `list_dir` → `DirListing`, `diff_trees` → `Vec<TreeChange>`, `LeafKind`, and the
five `TreeError` variants (`PathNotFound`, `NotADirectory`, `NotALeaf`, `ExternalBoundary`,
`InvalidView`).

Every type that seam B adopts from slice 1, with its fields as the working tree defines them. Each is
`#[non_exhaustive]` where marked, so a crate outside eprfs-core reads the fields but builds values only
through the public constructors (B12). The field names here are the code's today; the `placement` fields
rename to `coverage` as shown in the refinements below.

| Type | File | Derives | Shape |
|---|---|---|---|
| `TreeLimits` | `tree.rs` | `Debug, Clone, Copy, PartialEq, Eq` (`Default`: the values shown) | `{ max_node_bytes: usize = 4 MiB, max_entries_per_node: usize = 100_000, max_depth: usize = 128, max_total_entries: usize = 1_000_000, max_total_bytes: usize = 1 GiB, max_path_bytes: usize = 4096, max_pending_entries: usize = 4096 }` |
| `TreeEntry`, `TreeEntryKind` | `tree.rs` | `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`, `deny_unknown_fields` | `TreeEntry { name: Vec<u8>, kind: TreeEntryKind }`; `TreeEntryKind` (kebab-case) is `File { blob: BlobLink, executable: bool }`, `Directory { tree: BlobLink }`, `Symlink { target: BlobLink }` and `External { kind: String, reference: Vec<u8> }`, mirrored by §H.1's `TreeEntryKind` |
| `BlobPresence` | `storage.rs` | `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`, kebab-case | `enum { Local, Remote, Missing, Unknown }` |
| `WalkedNode` | `walk.rs`, non-exhaustive | `Debug, Clone, PartialEq, Eq` | `{ path: TreePath, cid: BlobCid, node: TreeNode, placement: Placement /* Spine or InView */, entry_placements: Vec<Placement> /* parallel to node.entries */ }` |
| `WalkItem` | `walk.rs`, non-exhaustive, variants too | `Debug, Clone, PartialEq, Eq` | `Node(WalkedNode)`, `Boundary { path: TreePath, kind: String, reference: Vec<u8>, placement: Placement }`, `Absent { view_path: TreePath, reason: AbsentReason }` (slice 2 adds `unanswered`, below) |
| `WalkStats` | `walk.rs`, non-exhaustive | `Debug, Default, Clone, PartialEq, Eq` | `{ nodes_fetched: usize, nodes_reused: usize, node_bytes: usize, entries_visited: usize }` |
| `WalkedView` | `walk.rs`, non-exhaustive | `Debug, Clone` | `{ root: BlobCid, nodes: Vec<WalkedNode>, boundaries: Vec<WalkItem>, absent: Vec<WalkItem>, stats: WalkStats }`, with `leaves_in_view()` |
| `LeafKind` | `walk.rs` | `Debug, Clone, Copy, PartialEq, Eq` | `enum { File { executable: bool }, Symlink }` |
| `ReadLeaf` | `walk.rs`, non-exhaustive | `Debug, Clone, PartialEq, Eq` | `{ path: TreePath, cid: BlobCid, kind: LeafKind, bytes: Bytes }` |
| `DirListing` | `walk.rs`, non-exhaustive | `Debug, Clone, PartialEq, Eq` | `{ path: TreePath, cid: BlobCid, entries: Vec<TreeEntry> }` (no child is fetched) |
| `TreeChange` | `walk.rs`, non-exhaustive | `Debug, Clone, PartialEq, Eq` | `{ path: TreePath, old: Option<TreeEntryKind>, new: Option<TreeEntryKind> }` |
| `SpineWalk` | `walk.rs`, **new in slice 4** (§C) | `Debug, Clone, PartialEq, Eq`, non-exhaustive | `{ root: BlobCid, nodes: Vec<WalkedNode>, absent: Vec<WalkItem>, stats: WalkStats }`: `nodes` holds every Spine node in pre-order, and the InView entries appear only as entries of those nodes, never fetched. `absent` holds only `NotFound` and `ThroughLeaf`, because `walk_spine` returns `TreeError::Unanswered` for a `NotHeld` or `Withheld` absence on the spine. An External on the spine stays as an entry of its node |

Slice 1's free functions take `(storage: &S, root, view or path, policy: FetchPolicy, limits:
&TreeLimits)` with `S: EprfsStorage`. From slice 2 they take `opts: &BatchOptions` and `S: EprfsStorage +
?Sized` (§A.1).

Refinements (findings 1–2 and the seam check). These are **decided**, and every signature in this
document is written with them applied. Lane E confirms them by building them. A seat that cannot build
one as written records a §6 revision with its reason before it lands, as the graduation trigger
requires; confirming one is not an open question.

```rust
// Rename of `Placement`, free before E1 lands. The fields rename with it (B11): View::placement →
// View::coverage, WalkedNode.placement → coverage, WalkedNode.entry_placements → entry_coverage,
// WalkItem::Boundary.placement → coverage; eprfs-storage/tests/walk_directory_storage.rs moves too.
pub enum Coverage { InView, Spine, Outside }

#[non_exhaustive]                                   // already, variants included
pub enum AbsentReason {
    NotFound { missing_at: TreePath },                         // the name is absent in a held node
    ThroughLeaf { leaf: TreePath },
    NotHeld { missing_at: TreePath, cid: BlobCid, tried: Vec<TierId> },   // R6: observed absent; `tried` is
                                                               //   empty for a leaf store (B2)
    // slice 2:
    Withheld { missing_at: TreePath, cid: BlobCid, why: ItemAbsence },    // refused, over budget, unreachable
                                                               //   or membership unknown: absence NOT
                                                               //   established; the walk continues
}
// WalkItem::Absent { view_path, reason: AbsentReason, unanswered: Option<Unanswered<ItemAbsence>> }:
// NotHeld and Withheld carry the R7 three parts (when to come back, what is local meanwhile);
// NotFound and ThroughLeaf are observed answers about a held tree and carry None.
// TreeError gains (slice 2):
//   Unanswered { cid: BlobCid, path: TreePath, why: Unanswered<ItemAbsence> }
// which read_path, list_dir and Hydrator::read return for a path whose nodes are not held, in place of
// Incomplete { reason: String } (B1).
// Public constructors (B12), because variant-level #[non_exhaustive] stops a remote tier building these:
impl AbsentReason { pub fn not_found(..) -> Self; pub fn through_leaf(..) -> Self;
                    pub fn not_held(..) -> Self; pub fn withheld(..) -> Self; }
impl WalkItem     { pub fn node(..) -> Self; pub fn boundary(..) -> Self; pub fn absent(..) -> Self; }
```

**Contract.** The slice-1 brief's tests 1–29 with the R6 replacement bind every `walk` implementation,
including a remote tier answering on the server side (built through the public constructors). In
addition, from slice 2:

- (30) a walk over a `TierChain` whose own store holds only the root fetches every node from the
  outer tier exactly once under `FetchIfMissing`;
- (31) under `LocalOnly` the same walk yields `NotHeld` per missing subtree and makes zero outer calls;
- (32) the depth-0 query defaults equal the free functions on the 10,000-entry fixture, including
  through `&dyn EprfsStorage`;
- (33) a walk whose tier answers `Refused` or `Unreachable` yields `Withheld` and continues; nothing
  that is not observed absent reads as `NotHeld`.

**B.1 The level-batched walk** (`eprfs-core/src/level_walk.rs`, slice 2). It is a sans-I/O state
machine with the same view matching, `Budget` and verification as `TreeWalk`, and no storage reference.

```rust
pub struct LevelWalk<'a> { /* per-frontier view tails, Budget<'a>, node cache, absences */ }
pub enum LevelOutcome { Next, Done }
impl<'a> LevelWalk<'a> {
    pub fn start(root: &BlobCid, view: &'a View, limits: &'a TreeLimits) -> TreeResult<Self>;
    /// This level's frontier, deduplicated, yielded in chunks of at most `max_pending_entries`, so one
    /// level may take several `fetch_blobs` calls and a level wider than 4096 directories does not
    /// refuse where the depth-first walk passes (B13).
    pub fn wanted(&self) -> &[BatchItem];
    pub fn offer(&mut self, item: &BatchItem, bytes: &Bytes) -> TreeResult<()>; // verify vs the link, decode, expand
    /// Integrity, and a storage failure of the own store, are errors, as in walk.rs (tests 10 and 29);
    /// only NotHeld, Refused, OverBudget, Unreachable and MembershipUnknown stay absences (B4).
    pub fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>) -> TreeResult<()>;
    /// An item never offered (the sink returned Stop, a batch was cut short, the tier failed as a whole)
    /// becomes `Withheld { Unreachable }`, never `NotHeld` (B3).
    pub fn advance(&mut self) -> TreeResult<LevelOutcome>;
    pub fn finish(self) -> WalkedView;                                         // nodes re-sorted to pre-order
}
/// One `fetch_blobs(root, …)` per tree level chunk, over any tier. It re-sends the items the sink did not
/// see once, as the resumable contract allows, before `advance`.
pub async fn walk_batched<S: EprfsStorage + ?Sized>(storage: &S, root: &BlobCid, view: &View,
    opts: &BatchOptions, limits: &TreeLimits) -> TreeResult<WalkedView>;
```

Contract: **level equivalence** (`walk_batched` and `walk_collect` give equal node sets, absences,
boundaries and errors on every view of the slice-1 suite, tampered and corrupt cases included);
**level batching** (over a counting tier, one `fetch_blobs` per level chunk: for the headline view, 3
calls whose `wanted()` sets total the 7 spine nodes); **order invariance** (the same result whatever
order `offer` is called in within a level); a test at a level of exactly `max_pending_entries + 1`
directories.

**B.2 Depth 0 on the server side.** Slice 2 walks on the client side, level by level, which needs no
tree understanding in a tier. A tier that links eprfs-core may answer one question in one round trip by
overriding `EprfsStorage::query_read_path`, `query_list_dir` and `query_diff` (§A.1). They are methods
of the tier trait, not a separate `TreeQuery: Sized` trait, so `TierConnector`'s `Arc<dyn EprfsStorage>`
and `TierChain.outer` keep the override and the chain can call it (B5). The walk's free functions relax
to `S: EprfsStorage + ?Sized` (as `walk_batched` is) so the defaults compile through `dyn` (C8).
`HttpTier` overrides them over `POST /eprfs/v1/{read,list,diff}`; `TierEndpoint` answers those routes by
calling its store's query methods after admitting the root (both roots for a diff).

The answers need a wire form (B5): `ReadLeaf`, `DirListing`, `TreeChange` and `TreePath` gain public
constructors and the two-form serde of `BlobLink` (DAG-CBOR byte-string components; JSON slash-joined
UTF-8, a non-UTF-8 name refusing with `InvalidView`).

`diff_trees` reads two roots (B6): `DiffFrame` and `diff_side` carry their own side's root, and each node
fetch goes through `fetch_at(side_root, …)`. A requester admitted to `new` but not `old` is refused the
whole diff (§A.5 `serve_diff`), so `TreeChange.old` never leaks a narrower root's names.

**B.3 The serializable carrier of a root and a view** (`eprfs-core/src/view_decl.rs`, slice 2). This
is the Rust twin of the SDK `tree-view` object (§2.G).

```rust
/// Canonical form (DAG-CBOR): tag-42 root and byte-string path components, so non-UTF-8 names survive.
/// Human-readable form (JSON): CID string and slash-joined UTF-8 paths; a non-UTF-8 path refuses with
/// TreeError::InvalidView. The same two-form rule as BlobLink. The root path is spelled "" in JSON and
/// as the empty component list in DAG-CBOR, so View::whole() is [""].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeView { root: BlobLink, paths: Vec<TreePath> }
impl TreeView {
    pub fn new(root: BlobCid, view: &View) -> Self;            // normalized: sorted component-wise, an antichain
    pub fn root(&self) -> &BlobCid;
    pub fn view(&self) -> View;
    pub fn cid(&self) -> TreeResult<BlobCid>;   // dag-cbor CID of the canonical form: the NeedKey of R1
}
impl Serialize for TreeView { /* is_human_readable branch */ }
/// Refuses (TreeError::InvalidView) a path list that is empty, not sorted component-wise (not by the
/// slash-joined string: '-' 0x2d sorts before '/' 0x2f), or not an antichain (a path covered by
/// another), as decode_tree refuses noncanonical bytes. So cid() is a function of the View (B7).
impl<'de> Deserialize<'de> for TreeView { /* same, then the canonical check */ }
```

A shared vector (in `fixtures/tree-vectors.json`) pins `["a", "a/b"]` as refused on decode and as
normalizing to `["a"]` through `TreeView::new`, with the same `cid()` in Rust and TypeScript.

**B.4 Depth and the later slices** (signatures fixed now).

```rust
// eprfs-core/src/walk.rs
/// No Ord: depth does not order offline capability (B14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Depth { Query, Sparse, OnRead, Kept }   // spec §12.2; Kept is read from the residency pin, never written by eprfs
impl Depth {
    /// Spec §12.2's table: Kept satisfies every requirement; Sparse (the whole view offline) and OnRead
    /// (only what was read) are not comparable to each other; each satisfies itself and Query.
    pub fn satisfies_offline(self, required: Depth) -> bool;
}

// eprfs-core/src/hydrate.rs  (slice 3: on-read depth)
/// `&self` reads: the spine cache sits behind a std `RwLock` (R-E2 forbids threads and clocks, not
/// locks), so concurrent reads share fetched spine nodes and a read waiting on a remote tier does not
/// block an own-store hit (B8).
pub struct Hydrator<S: ?Sized> { /* root, limits, spine cache: RwLock<…>, storage */ }
impl<S: EprfsStorage + ?Sized> Hydrator<S> {
    pub fn new(storage: Arc<S>, root: BlobCid, limits: TreeLimits) -> TreeResult<Self>;
    pub async fn read(&self, path: &TreePath) -> TreeResult<ReadLeaf>;   // fetched on first read, verified
    pub async fn list(&self, path: &TreePath) -> TreeResult<DirListing>;
    pub fn read_set(&self) -> View;   // the view discovered by reading (spec §12.2)
}

// eprfs-core/src/overlay.rs  (slices 3 and 7) — on the writing device only; deliberately not Serialize (R6)
/// Put takes any entry kind, so an overlay can move a gitlink (External) or graft a directory by CID (B10).
pub enum OverlayChange { Put { kind: TreeEntryKind }, Remove }
pub struct Overlay { base: BlobCid, changes: BTreeMap<TreePath, OverlayChange> }
impl Overlay {
    pub fn over(base: BlobCid) -> Self;
    pub fn put(&mut self, path: TreePath, kind: TreeEntryKind) -> TreeResult<()>;
    /// Erases every recorded change under `path`, then records Remove; a later put under `path`
    /// re-creates its spine. The map therefore never needs the order of operations.
    pub fn remove(&mut self, path: TreePath);
    pub fn is_clean(&self) -> bool;
    /// Re-encodes only the changed paths' spine and puts each new node verified; dropping costs nothing.
    pub async fn seal<S: EprfsStorage + ?Sized>(&self, storage: &S, limits: &TreeLimits) -> TreeResult<BlobCid>;
}
```

Overlay contract: `put(a/b/f)`, `remove(a)`, `put(a/c)` seals to a tree whose `a` holds only `c`;
an External put round-trips its reference bytes.

`Depth` and `FetchPolicy` together replace `MaterializationPolicy` (`projection.rs`: `LocalOnly`,
`FetchMissing`, `Sparse`) as the materializer's input, by this mapping (B9), which is not a rename:
`LocalOnly` → (`FetchPolicy::LocalOnly`, an absence is an error); `FetchMissing` → `FetchIfMissing`;
`Sparse` → `LocalOnly` with each absence written as a sparse marker (`BytePresence::Remote`). The old
`Sparse` is **not** `Depth::Sparse` (the view's bytes are here). `MaterializationPolicy` stays,
`#[deprecated]`, for one release; eprfs-local's public `materialize(…, policy)` and eprfs-agent's
`round_trip.rs` move with it, and both are in finding 2's `0.2.0` break list. `BytePresence`
(`awareness.rs`) stays the per-path projection, where `Pinned` is depth 3 and `Hydrating` is depth 2 in
flight.

**Implementors and callers.** The walk functions are generic over any `S: EprfsStorage + ?Sized`, so the
same function walks a `DirectoryStorage` in slice 1 and a `TierChain` or a `dyn` tier in slice 2.
Callers: eprfs-local's `LocalMaterializer` at depth 1 (`walk_batched`, then `fetch_blobs(root,
leaves_in_view)`), brit checkout (slice 7), the ark task runner (§F), lvi (depth 2), and the slice-1
example probe.

**Worked example.** The slice-1 headline: a 10,000-file tree under `{a1/b2/f003, a4/b4/f099, a9/b0}`
fetches 7 of 111 nodes and no leaf locally. Through `walk_batched` over an `HttpTier` it is 3 round
trips for the spine ({root}, {a1, a4, a9}, {a1/b2, a4/b4, a9/b0}) and one `fetch_blobs` for the leaves
in view: 4 round trips against 109 one at a time.

### C. Derivation by identity

**Owner:** eprfs-core, `src/composition.rs` (the record's edge, additive) and `src/derive.rs` (new,
slice 4). The record is evidence, never authority (spec §12.5).

```rust
// eprfs-core/src/derive.rs
/// The canonical DAG-CBOR descriptor of everything that changes the output except the input bytes
/// (C4, C12, C15). Its CID is the method CID. `view` is the method's declared input set (for a gate, the
/// step's `inputs.sources`), so the key cannot be computed under another view and a change inside any
/// declared input moves P.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MethodDescriptor {
    pub name: String,                     // e.g. "brit-gate", "a2o-measure", "recall-fold"
    pub tools: BTreeMap<String, String>,  // tool → version
    pub flags: Vec<String>,
    pub params: Option<BlobCid>,          // the parameter set the method runs with, if any
    pub exec: Option<BlobCid>,            // CID of the ChildSpec (artifact, argv, env), when it runs a process
    pub step: Option<String>,             // brit step_name
    pub embedder: Option<String>,         // recall's embedder
    pub view: Vec<TreePath>,              // the declared input paths (no root), normalized as TreeView's
    pub non_path_inputs: BTreeMap<String, String>,   // declared identities with no path (dnaHash, A2O_SUT_*)
    pub deterministic: bool,              // bound to the method CID; a record's flag must equal it
}
impl MethodDescriptor { pub fn cid(&self) -> Result<BlobCid>; pub fn view(&self) -> View; }

/// Built only by `project_view` (C14): a caller cannot key on an unprojected root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectedRoot(BlobCid);
impl ProjectedRoot { pub fn cid(&self) -> &BlobCid; }
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "form", content = "cid")]
pub enum KeyInput { Projected(ProjectedRoot), Raw(BlobCid) }   // Raw: a wasm CID, never a tree root

/// (method CID, input). The input is the VIEW-PROJECTED root of what the method reads, so a change
/// outside the declared view never invalidates the record. It is the storage key, never the reach key:
/// reach is judged on the input root (C1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DerivationKey { pub method: BlobCid, pub input: KeyInput }

/// What the WRITER observed (C3). Agreement is not a recorded basis: the reader computes it from distinct
/// authenticated records with equal outputs and judges independence from its own roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "basis")]
#[non_exhaustive]
pub enum DerivationBasis {
    Computed,                                            // by `computed_by`; "here" is the reader's comparison
    UnderGrant { grant: String },                        // carried for its requester under that grant
    Recomputed { checked: BlobCid, against: String, matched: bool },   // the other record's output and
                                                         //   its computed_by: a mismatch names what it refutes (C10)
    #[serde(other)] Unknown,                             // a newer basis; TrustPolicy never accepts it (C11)
}

/// Authenticates `computed_by` (C3, C16): a signature by that agent over the record's canonical bytes,
/// or, once notarized, the attestation's reference. `EprRef` is eprfs-core's existing slug identity
/// (`address.rs`: `pub struct EprRef(String)`, `serde(transparent)`, with `new` and `as_str`), the type
/// `publish_attestation` already returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "witness")]
pub enum Witness { Signature { sig: String }, Attestation { epr: EprRef } }

/// No deny_unknown_fields: records travel between peers on different versions (C11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationRecord {
    pub key: DerivationKey,
    pub output: BlobCid,
    pub computed_by: String,          // agent key; the `composedBy` role
    pub basis: DerivationBasis,
    pub deterministic: bool,          // must equal the descriptor's; the index rejects a mismatch (C15)
    pub witness: Witness,
}
impl DerivationRecord {
    /// Projects onto the existing edge: source = input, derived = output, attributed_to = computed_by,
    /// relation = DerivationKind::Derivation (new unit variant, "derivation", landed in 0.2.0 with
    /// DerivationKind marked #[non_exhaustive], C17), metadata = {method}.
    pub fn to_edge(&self) -> DerivationEdge;
    /// A Notarized (A) attestation through the existing publish_attestation path, with a new
    /// AttestationKind::Derivation. Only for a record over an adopted head; a draft's record refuses
    /// (spec §14: drafts' derivations stay C and do not attest) (C7).
    pub fn to_attestation(&self, adopted: &AdoptedHead) -> Result<AttestationDraft>;
}
/// A declared head and the root it adopted; drafts have none. Plain data, built by the caller from the
/// head plane (slug → {root, reach}); `head` is the head's slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptedHead { pub head: String, pub root: BlobCid }

/// Spec §14: one corpus digest per adopted head per method (C7). Named by its CID, reached through the
/// head, which is not cached; the bundle itself is immutable once named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationBundle { pub head: String, pub root: BlobCid, pub method: BlobCid, pub records: Vec<DerivationRecord> }
impl DerivationBundle {
    /// Decodes record by record: an undecodable record is skipped and counted, never failing the bundle (C11).
    pub fn decode_lenient(bytes: &[u8]) -> Result<(Self, usize)>;
}

pub enum DerivationLookup { Found(Vec<DerivationRecord>), NotRecorded }
/// R7's typed reason for seam C (C13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "reason")]
#[non_exhaustive]
pub enum DerivationUnanswered {
    NotRecorded, IndexUnreachable, Refused { refusal: ReachRefusal }, InputIncomplete { at: String },
    OutputNotFetchable, Mismatch { outputs: Vec<BlobCid> }, NotCarried,   // capacity did not admit a run here
}

#[async_trait]
pub trait DerivationIndex: Send + Sync {
    /// Carries what reach is judged on (C1). A serving index runs admit_root(who, at.root), recomputes P
    /// with project_view over its own nodes (a membership check, as TierEndpoint does for paths), then
    /// answers the records keyed (method, P). On the requester side each index is gated with
    /// `AskPolicy::may_ask(tier, at.root, …)`, so P, a hash of private content, never reaches an index
    /// the root's reach does not admit.
    async fn lookup(&self, who: &Requester, at: &TreeView, method: &MethodDescriptor)
        -> core::result::Result<DerivationLookup, Unanswered<DerivationUnanswered>>;
    /// One fetch answers K methods on one input (contract 6).
    async fn lookup_many(&self, who: &Requester, at: &TreeView, methods: &[MethodDescriptor])
        -> core::result::Result<BTreeMap<BlobCid, DerivationLookup>, Unanswered<DerivationUnanswered>>;
    /// Idempotent on (key, output, computed_by). Records sit side by side and never overwrite. Refuses an
    /// unauthenticated record and one whose `deterministic` differs from its descriptor.
    async fn record(&self, record: &DerivationRecord) -> Result<()>;
}

/// The reader's judgement, never stored in the record.
pub trait TrustPolicy: Send + Sync {
    /// `records` are authenticated. `roster` is the reader's own view of failure domains.
    fn accept<'r>(&self, key: &DerivationKey, records: &'r [DerivationRecord], me: &str,
                  roster: &dyn DomainRoster) -> core::result::Result<&'r DerivationRecord, Unanswered<DerivationUnanswered>>;
}
pub trait DomainRoster: Send + Sync { fn domain_of(&self, agent: &str) -> Option<String>; }

#[async_trait]
pub trait Derive: Send + Sync {
    fn descriptor(&self) -> &MethodDescriptor;
    /// `prior`: the accepted record and output for a parent root, when the caller names one, so a
    /// history-shaped method costs only its change (spec §12.5; C9). `input_store` is the store the
    /// method reads `input`'s root and paths from; `output_store` is the store it writes the derived
    /// output's objects to. The two may be the same store. The returned CID is the output root in
    /// `output_store`.
    async fn derive(&self, input: &TreeView, input_store: &dyn EprfsStorage, output_store: &dyn EprfsStorage,
                    prior: Option<&Derived>) -> Result<BlobCid>;
}

pub struct Derived { pub output: BlobCid, pub record: DerivationRecord, pub reused: bool }
pub enum DeriveOutcome { Derived(Derived), NotDerived(Unanswered<DerivationUnanswered>) }

/// Look up; if an accepted record's output is fetchable, fetch it; else, if the device's capacity admits
/// it, compute, record and return; else NotDerived with R7's three parts (spec §12.5: recompute only
/// when capacity admits; the task is kept, named as not carried). The view is the descriptor's.
/// The output is fetched as `fetch_at(input_root, BatchItem::derived(output, method))` through a
/// root-carrying tier (C2): a derivation output inherits its input root's reach, and a serving tier
/// admits it through the record (it holds an accepted record keyed (method, P) under that root with that
/// output). `BatchItem::derived` uses a reserved path form, `TreePath::derived(method)`, whose component
/// contains NUL and so can never name a tree entry (`valid_name` refuses NUL).
pub async fn derive_or_reuse(index: &dyn DerivationIndex, trust: &dyn TrustPolicy, method: &dyn Derive,
    input_root: &BlobCid, storage: &dyn EprfsStorage, who: &Requester, me: &str, roster: &dyn DomainRoster,
    parents: &[BlobCid], capacity_admits: bool) -> Result<DeriveOutcome>;

/// A walk that stops at the links of InView entries: it fetches spine nodes only, refusing only on an
/// absence ON the spine (C5). The derivation key's input never needs an InView directory's subtree.
pub async fn walk_spine<S: EprfsStorage + ?Sized>(storage: &S, root: &BlobCid, view: &View,
    opts: &BatchOptions, limits: &TreeLimits) -> TreeResult<SpineWalk>;
/// The view-projected input root. Rules (C6), pinned by P vectors in fixtures/tree-vectors.json that the
/// Rust and TypeScript suites both reproduce:
/// - an InView directory is referenced by its EXISTING link (no re-encode); an InView file or symlink
///   entry is kept as is;
/// - a Spine node is re-encoded keeping only entries on or in the view;
/// - an External entry in a kept node is kept verbatim, whether reached at Spine or InView;
/// - a ThroughLeaf leaf is kept (the view path stops at it);
/// - a NotFound view path contributes nothing;
/// - a spine directory with no surviving entries is dropped from its parent.
/// Zero leaf fetches. A NotHeld or Withheld absence on the spine refuses with TreeError::Unanswered: a key
/// over partly held input means nothing. Returns the root and the new spine nodes for the caller to store.
pub fn project_view(spine: &SpineWalk, limits: &TreeLimits) -> TreeResult<(ProjectedRoot, Vec<(BlobCid, Bytes)>)>;
```

**Contract** (`eprfs-core/tests/derive.rs`):

1. The same key and a deterministic method give the same output CID on two stores; a second peer
   fetches the output through a `TierEndpoint` under the input root. That is spec §16's "two peers
   share one derived output".
2. `record` is idempotent, and `lookup` never returns a record for another key; an unauthenticated
   record, or one whose `deterministic` differs from its descriptor, is refused.
3. `derive_or_reuse` computes nothing when a record is accepted and its output arrives. **A record
   whose output bytes cannot be fetched is a miss**: the record is evidence, not bytes. With capacity
   not admitting a run, the outcome is `NotDerived` with its three parts.
4. A non-deterministic method's records are reusable only as the reader's own `Computed` or as
   `UnderGrant`; the reader never computes agreement over them.
5. `walk_spine` plus `project_view` fetch only spine nodes (a counting store reads exactly the spine) and
   no leaf, are name-preserving and order-invariant, are unchanged by any change outside the view, keep
   an `InView` directory's CID unchanged inside it, and refuse only on a spine absence.
6. One `lookup_many` answers K methods on one input with one fetch per adopted bundle.
7. A lookup by a requester outside the input root's reach is refused, for every view, and a household
   member is answered for a partial view (the derived-by-identity scenarios at :46-51 and :77-83).
8. Conflicting outputs for a deterministic key are never accepted by `HouseholdFirst`; they surface as
   `Mismatch`, and a `Recomputed` record names the `(computed_by, output)` it refutes.
9. A record or bundle with an unknown field or basis decodes; the unknown basis is never accepted.
10. Given a prior over a root that differs by one leaf, a history-shaped method fetches only that
    leaf's bytes.

**Implementors.** Index: a local sidecar beside the store in eprfs-local (new); brit's evidence index
through an adapter in brit; elohim-storage when records travel through the build-attestation path
(spec §14). `Derive`: light methods (the lexical fold, blame) in their own crates; heavy methods run as
tasks (§F). `TrustPolicy`: the consumer, such as brit's gate evidence; the default `HouseholdFirst`
accepts the reader's own `Computed`, then `UnderGrant` by a carrier in the reader's household, then
equal outputs from authenticated records in at least 2 domains the reader's roster judges independent,
and otherwise recomputes or answers `NotDerived`.

**First instances, each re-keyed.** brit `BuildAttestationContentNode` (`manifest_cid` plus `step_name`
→ the descriptor, `inputs_hash` → the projected input root, `output_cid` → output, `agent_id` →
computed_by, `signature` → `Witness::Signature`); a failed attestation (`success: false`) is not
projected into a record, so a failed build's output is never reused (C16); brit gate verdicts, found
today by an inputs hash in git notes (`brit-epr/src/elohim/refs.rs`), become a `GitNotesIndex:
DerivationIndex`, the verdict output encoding pass or fail; epr-cli's recall `store_dir(root,
measure_cid, embedder)`, keyed by method only, gains its input root, with the embedder in the
descriptor; rakia outputs; compiled wasm (method = compiler and version, input = `KeyInput::Raw(wasm
CID)`; whether a conductor may run it stays the runtime's question, spec §13).

**Extension: a new method** is a `MethodDescriptor` plus a `Derive`. Mark it deterministic only if a
recompute can spot-check it. The core never interprets a method.

**Worked example.** A brit gate for `elohim/eprfs` (method M, whose descriptor's view is the step's
declared `inputs.sources` in `elohim/eprfs/build-manifest.json`: `elohim/eprfs`, `elohim/epr`,
`elohim/epr-rea`, `elohim/epr-index`, `elohim/Cargo.toml`, `.cargo/config.toml`, `bridges/did/…`,
`crates/consent-grant`, and the rest of that list) ran at commit C1. Commit C2 changes only `genesis/`.
`walk_spine` plus `project_view` of C2 under M's view give the same P as C1, at the cost of the spine
nodes down to those paths and no leaf. `lookup(me, C2 under M's view, M)` returns C1's record,
`HouseholdFirst` accepts it as `UnderGrant` by Jessica's berth, and the verdict V arrives through the
chain under C2's root. The gate does not re-run. A commit that changes only `elohim/epr` moves P, and
the gate runs.

### D. The placement floor

**Owner:** a new crate, `elohim-placement-core` at `elohim/placement/core/` (library
`placement_core`), a member of `elohim/Cargo.toml` beside `ark/core` (follow-on). Dependencies are
R-P1's exact set. It is pure functions over a snapshot, in the shape of ark-core's `lifecycle::step`:
inputs go in, a plan comes out, an adapter applies it. Salvage is generalized, not replaced (spec §17
slice 5).

**D.1 Kinds: a closed list, because grant scopes are a closed list.** One `decide` pass covers every
kind, so capacity is charged across kinds in the charter's serve order (§D.7).

```rust
// placement-core/src/kind.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum KindId { CustodyBlob, CustodyRoot, LiveSession, Relay, WakeIngress, HubRuntime, Inference, IndexFold, Task }

/// Mirrors enums/grant-scope.schema.json and elohim-storage's GRANT_SCOPES (a test asserts equality).
/// The compute-grant scopes only; delegates-compute's own `scope` stays an open string (finding 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum GrantScope { SweettestFeedback, HostedCell, MeasureStage }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "action")]
pub enum MishpatAction { CustodyBlob, ReplicatesContent, DelegatesCompute { scope: GrantScope } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Charge { pub bytes: u64, pub memory_bytes: u64, pub cpu_millis: u32, pub pids: u32 }

pub trait PlacementKind: Send + Sync {
    fn id(&self) -> KindId;
    fn action(&self, need: &Need) -> MishpatAction;
    /// The scope consumed when the need touches another person (R3); None: a declared posture suffices.
    fn scope(&self, need: &Need) -> Option<GrantScope>;
    /// Whether the receiver may carry its own need. Custody kinds: false (a copy on the receiver is not
    /// a second holder). Task: true, so the requester's own envelope is the first candidate (F5).
    fn receiver_may_carry(&self) -> bool;
    /// What adoption charges against the carrier's remaining offer, or the kind's own refusal. It can
    /// only refuse: the floor's universal checks run first and a kind cannot widen them.
    fn fits(&self, need: &Need, candidate: &Candidate, offer: &OfferTerms, remaining: &Capacity)
        -> Result<Charge, IneligibleReason>;
    /// The method used when no parameter set for this kind is in force.
    fn default_method(&self) -> MethodId;
}
pub struct KindRegistry(BTreeMap<KindId, Box<dyn PlacementKind>>);
impl KindRegistry {
    pub fn new() -> Self;
    pub fn with(self, kind: Box<dyn PlacementKind>) -> Self;
    pub fn get(&self, id: KindId) -> Option<&dyn PlacementKind>;
}
/// MethodId → Ranking (D7). "This peer does not run the in-force method" means the MethodId is not here.
pub struct MethodRegistry(BTreeMap<MethodId, Box<dyn Ranking>>);
impl MethodRegistry {
    pub fn builtin() -> Self;                      // XorDistance, DiversityAware, NearestBytes, WeightedCapacity
    pub fn with(self, r: Box<dyn Ranking>) -> Self;
    pub fn get(&self, m: MethodId) -> Option<&dyn Ranking>;
}
```

**Extension: a new placement kind** (relay, live-session host, wake ingress) is four coupled steps
landed together: (1) a `KindId` variant and its entry in `enums/placement-kind.schema.json`; (2) a
`PlacementKind` implementation in the crate that owns the carrier (custody in elohim-storage, task in
ark-core, relay in its own design); (3) if it spends compute, a `GrantScope` variant, its
`enums/grant-scope.schema.json` entry and its `GRANT_SCOPES` consumer (`compute_grants.rs:46`), a
coordinator change that moves no DNA hash; (4) a probe (§I).

**D.2 Needs.** A need is an intent to receive (spec §7.1).

```rust
// placement-core/src/need.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "form", content = "id")]
pub enum NeedKey {
    Blob(String),      // legacy custody marker ("sha256-<hex>"), until the custody rows migrate
    TreeView(Cid),     // TreeView::cid(): R1, per tree root and view, never per blob
    Task(Cid),         // TaskSpec CID (§F)
    Session(Cid),
}
impl NeedKey {
    /// Ranking bytes. Blob → the marker's UTF-8 bytes, so the XOR key is byte-identical to
    /// placement.rs::hash_into_keyspace (pinned by golden test); the others → the CID's bytes.
    pub fn rank_bytes(&self) -> Vec<u8>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Demand { pub holders: u16, pub domains: u16 }   // domains <= holders; resolved by the charter

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    pub always_on: bool,
    pub bytes: Option<u64>,               // for a task: disk_bytes plus the materialized view's size (F16)
    pub memory_bytes: Option<u64>, pub cpu_millis: Option<u32>, pub pids: Option<u32>,   // pids: D20, F16
    pub isolation: Option<Isolation>,     // the weakest isolation the need accepts (F6)
    pub carriers: Option<BTreeSet<AgentRefKey>>,   // a narrowing every peer's admit sees (F14); None: any
}
/// placement-core's own copy of ark's isolation ladder (placement-core does not depend on ark-core);
/// ark-core maps passport::EffectiveTier onto it 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Isolation { None, Intrinsic, Delegated, Bounded, Enforced }
/// An agent key as an ordered string, for sets (AgentRef has no Ord, finding 5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AgentRefKey(String);

/// Deliberately no Device variant: a device's own needs exist only as PrivateNeed.
/// A struct, not an internally tagged enum, so the CID field's human-readable branch stays honest in
/// both formats (G1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expresser { pub by: ExpresserKind, pub id: String /* manifest/charter CID string, or agent key */ }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpresserKind { App, Person, Charter, Agent }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum NeedClass { Care, Member, Agent, Commons }   // the charter's serve-order classes

/// Kind-specific terms inside the need's canonical encoding (F4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "terms")]
pub enum NeedTerms {
    None,
    Task { #[serde(with = "cid_serde")] input_root: Cid, scope: GrantScope, base_root: Option<CidString> },
}

/// Every CID field carries `#[serde(with = "cid_serde")]` (epr-rea's helper, branching on
/// is_human_readable): a bafy… string in JSON, bytes in DAG-CBOR (G1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Need {
    pub kind: KindId,
    pub key: NeedKey,
    pub demand: Demand,
    pub requires: Requirements,
    pub terms: NeedTerms,
    pub reach: Reach,             // the reach it was expressed at; it crosses no wider (spec §7.2)
    pub receiver: AgentRef,       // carried for whom; one receiver per Need
    pub expressed_by: Expresser,
    pub class: NeedClass,
    #[serde(with = "cid_serde")]
    pub in_scope_of: Cid,         // the space whose membrane it was expressed in (VF in_scope_of)
}
impl Need { pub fn id(&self) -> Result<Cid, PlaceError /* §E */>; }   // atom_cid of the canonical encoding; the
                                                             // identity explain and Carrying key on (D10)

/// Device-membrane needs ("keep this offline", "spare my battery", a budget). Not Serialize (R6).
pub struct PrivateNeed { pub key: NeedKey, pub depth: Depth, pub budget_bytes: Option<u64> }

/// Implemented by each source of needs: the manifest reader (§G), a person's keep intents, a spool.
pub trait NeedSource { fn needs(&self, scope: &Cid) -> Vec<Need>; }
```

(`Depth` is `eprfs_core::Depth`'s variants mirrored here as `placement_core::Depth`, because
placement-core does not depend on eprfs-core; `ark-core` maps the two 1:1. `Cid` derives `Ord`, so the
CID-bearing `NeedKey` keeps `Ord`; its serde uses `cid_serde` too.)

**Legacy custody mapping** (D10, E12), stated here and pinned by contract 13. The custody `Ports`
builds a `Need` from a legacy custody row as: `reach` = the widest reach of the content rows that
reference the blob, or `Public` when none does (so today's notarized behaviour is kept); `in_scope_of` =
a named legacy scope CID (`MethodId::named("elohim:placement:legacy-custody-scope@1")`'s CID); `receiver`
= the smallest receiver among the rows for that blob (deterministic, where today the first row of an
unordered load wins); one `Need` per (kind, key). A legacy commitment row becomes a `Carrying` with
`under_params` = `InForce::builtin(CustodyBlob).ranking_cid()` (XorDistance), one fixed method for every peer, so legacy
holders are incumbents under the built-in set and the first pass moves nothing. The
`SALVAGE_DIVERSITY_PLACEMENT` knob is retired before the floor reads legacy rows; a peer whose knob still
differs is observe-only for the custody kinds. Legacy provider rows written under the transport id
(`self_read_cid`) are normalized to `Snapshot.me` through `Snapshot.me_aliases`.

**D.3 Postures, offers and candidates.** An offer is an intent to provide (spec §7.1). Device facts
never leave the device; only their effect crosses, as a narrowed, deferred or withdrawn offer.

```rust
// placement-core/src/offer.rs
pub type EpochSecs = u64;

/// Public: written when it changes, with hysteresis, never on a timer (spec §7.1). Its liveness is
/// read by the observer, never assumed from the posture (D3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Posture {
    pub carrier: AgentRef,
    pub offers: BTreeMap<KindId, OfferTerms>,
    pub reach: Reach,                       // how far the posture is visible
    pub written_at: EpochSecs,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferTerms {
    pub spare_bytes: Option<u64>, pub compute: Option<Capacity>,
    /// Roots kept at depth 3, each listed only at a reach no narrower than the posture's (F8).
    pub holds: Vec<CidString>,
    pub reach_ceiling: Option<Reach>,
    pub scopes: BTreeSet<GrantScope>,       // F6: what a task offer will spend
    pub isolation: Option<Isolation>,       // F6
    /// Own-household only and deferred to a charging or unmetered window (D11): eligible for its own
    /// household's needs, charged as a lag, never as an ineligibility.
    pub deferred: bool,
    pub household_only: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capacity { pub bytes: Option<u64>, pub memory_bytes: Option<u64>, pub cpu_millis: Option<u32>, pub pids: Option<u32> }

/// The device's own facts. NOT Serialize (rail R-P4). Its only way off the device is `posture()`.
pub struct DeviceFacts {
    pub on_battery: bool, pub metered: bool, pub always_on: bool,
    pub can_steward: bool, pub can_infer: bool, pub budget_bytes: u64,
}
pub struct PersonChoices { pub will_carry: BTreeSet<KindId>, pub lift_battery_default: bool, pub budget_bytes: Option<u64> }
impl DeviceFacts {
    /// Carrying for another household or space is off by default on battery, on a metered link or when
    /// not always on; the person's own choice may lift it (spec §7.2). Holding within the device's own
    /// household is not withdrawn for battery: the custody offer stays, `household_only` and `deferred`,
    /// so a two-phone household with no hub still holds its own bytes on both phones (spec §7.2, D11).
    /// Peers see only the presence, deferral or absence of an offer.
    pub fn posture(&self, me: AgentRef, choices: &PersonChoices, reach: Reach, now: EpochSecs) -> Posture;
}

/// What an OBSERVER resolves about a posture from records it holds, never self-report (the
/// `build_salvage_candidates` household join generalized). An unknown fact is Unknown, not a default.
/// Admission and consent are per receiver, because one snapshot holds needs from many receivers (D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub posture: Posture,
    pub domain: FailureDomain,
    /// Presence or inventory freshness within the floor's freshness horizon (D3); Unknown or Absent is
    /// ineligible (`Unknown { input: "liveness" }`).
    pub live: Answer<bool>,
    pub admitted: BTreeMap<AgentRefKey, Answer<Reach>>,   // receiver → narrowest circle admitted, relative to it
    pub consent: BTreeMap<AgentRefKey, Answer<Consent>>,  // receiver → R3: a grant is consent and eligibility
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FailureDomain(String);
impl FailureDomain { pub fn of(household: Option<&str>, agent: &AgentRef) -> Self; }   // "hh:<id>" | "agent:<key>"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "consent")]
pub enum Consent {
    NotNeeded,
    /// The grant's own bounds, as compute_grants.rs enforces them (D2): recipient, the validity window
    /// (valid_from <= now < valid_until) and its epr_scope, checked against NeedKey::Task.
    Granted { grant: String, scope: GrantScope, recipient: AgentRefKey, valid_from: EpochSecs,
              valid_until: EpochSecs, epr_scope: EprScope },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "scope")]
pub enum EprScope { All, Tasks { cids: BTreeSet<CidString> } }
/// A CID in its string form, for sets and maps on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CidString(String);
```

**Guarded self-candidacy (R4).** Self is a candidate only through its own `Posture` as it arrived from
the same shared source every peer reads. The snapshot builder never appends self. A peer that cannot yet
see its own posture does not adopt (spec §7.2, concern C1). A stale posture of a carrier that went
silent is ineligible through `live` (D3), so a dead carrier ranked into the prefix does not hold the
need under-held: the next pass ranks without it. The remaining window is a carrier whose own posture
has not round-tripped; it can over-fill, which the drain rule absorbs, and it cannot under-fill only
because liveness is observed, not because postures are assumed current.

**D.4 Eligibility comes first, takes no parameters, and is never ranked away.**

```rust
// placement-core/src/eligibility.rs
pub struct Eligible<'a> { candidate: &'a Candidate, offer: &'a OfferTerms, charge: Charge }   // private (R-P5)
impl<'a> Eligible<'a> {
    pub fn candidate(&self) -> &'a Candidate;
    pub fn agent(&self) -> &'a AgentRef;
    pub fn offer(&self) -> &'a OfferTerms;
    pub fn charge(&self) -> Charge;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ineligible { pub agent: AgentRef, pub why: Unanswered<IneligibleReason> }   // R7 (D6)

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "reason")]
#[non_exhaustive]
pub enum IneligibleReason {
    NoPosture,
    Unknown { input: &'static str },                       // a fact the floor does not hold (spec §7.2)
    OutsideReach { need: Reach, admitted: Reach },
    OverCapacity,
    NoConsent, ConsentExpired,
    IsReceiver,                                            // only for a kind whose receiver may not carry
    NotNamed,                                              // outside Requirements::carriers
    CrossHouseholdBelowPublic,                             // D13, until per-space cells exist
    Kind { label: &'static str },                          // the kind's own refusal
}
/// The Copy face for ReasonLabel (dashboards and the habit register enumerate the same labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IneligibleKind { NoPosture, Unknown, OutsideReach, OverCapacity, NoConsent, ConsentExpired, IsReceiver, NotNamed, CrossHouseholdBelowPublic, Kind }
impl IneligibleReason {
    pub fn kind(&self) -> IneligibleKind;
    pub fn refusal(&self) -> elohim_compute::Refusal;     // code GateRefused(label); limit owner named per reason
    /// The come-back event and the meanwhile for each reason (posture-changed, grant-issued,
    /// capacity-freed; "run here" or "nothing").
    pub fn unanswered(self, now: EpochSecs) -> Unanswered<IneligibleReason>;
}

/// The typed face. Order is fixed and matches `Governor::decide`'s authorize → gate → render (D17):
/// consent for this receiver (scope, recipient, window, epr_scope; an Unreachable grant refuses) →
/// posture offers the kind, and is live → every input known → named carriers → receiver rule → reach for
/// this receiver (admitted.openness() <= need.reach.openness()) → kind.fits(offer, remaining). No
/// parameter set is an argument: the ceiling cannot reach it.
pub fn admit<'a>(kind: &dyn PlacementKind, need: &Need, candidate: &'a Candidate, remaining: &Capacity,
                 now: EpochSecs) -> Result<Eligible<'a>, Ineligible>;

/// The elevate face over the same stage functions, so a refusal reaches the elevate sink in the shape
/// every other Governor uses (elohim-compute/src/actuation.rs). It overrides `decide` to call `admit`, so
/// both faces yield the same first refusal; a contract test asserts equal IneligibleKind.
pub struct Eligibility<'k, 'a> { pub kind: &'k dyn PlacementKind, _for: core::marker::PhantomData<&'a ()> }
impl<'k, 'a> Eligibility<'k, 'a> { pub fn new(kind: &'k dyn PlacementKind) -> Self; }
pub struct Candidacy<'a> { pub need: &'a Need, pub candidate: &'a Candidate }
impl<'k, 'a> elohim_compute::Governor for Eligibility<'k, 'a> {
    type Request = Candidacy<'a>;
    type Grant = Answer<Consent>;
    type Context = (Capacity, EpochSecs);
    type Effect = Eligible<'a>;
    // authorize(req, grant, now_epoch_s) · gate(req, ctx) · render(req, ctx); decide = admit
}
```

`LimitOwner` mapping: `NoConsent` and `ConsentExpired` → `Commitment`; `NoPosture` and the battery
withdrawal → `SelfLimit`; `OutsideReach` → the space, which today has no variant. The additive
`LimitOwner::Charter` is proposed to elohim-compute (follow-on); `LimitOwner` is not
`#[non_exhaustive]`, so the proposal greps for exhaustive matches first.

**D.5 Ranking, generalizing `PlacementStrategy`.**

```rust
// placement-core/src/rank.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MethodId(#[serde(with = "cid_serde")] Cid);
impl MethodId {
    /// CIDv1, codec raw (0x55), sha2-256 over the UTF-8 bytes of `spec`: the construction of
    /// `BlobCid::compute_raw`. A name is `<namespace>:<area>:<name>@<version>`, matching
    /// `^[a-z0-9-]+(:[a-z0-9-]+)+@[1-9][0-9]*$`, e.g. "elohim:placement:xor-distance@1". The `elohim`
    /// namespace is reserved for built-ins. A method whose behaviour changes takes a new version, and so
    /// a new MethodId. `named` hashes whatever it is given; a unit test checks every built-in name
    /// against the grammar, including the legacy custody scope (§D.2).
    pub fn named(spec: &str) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Factor { Distance, DomainSpread, SpareCapacity, NearBytes, AlwaysOn }   // no reach or consent factor can exist
/// Basis points (rail R-P3); every weight is > 0 (E6).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Weights(BTreeMap<Factor, i32>);

/// The rank key: incumbency first (scoped to the parameter set in force, §D.7), then the method's
/// score, then the smaller agent key (today's XOR tie-break, kept byte for byte), then the smaller
/// failure domain (today's final tie-break, placement.rs:177-205; D12).
pub type RankKey<'a, S> = seam_contracts::ArbitrationKey<bool, S, (core::cmp::Reverse<&'a str>, core::cmp::Reverse<&'a FailureDomain>)>;

/// What a ranking may read about the need (D8, F4): the key, the receiver's domain and the task's input
/// root, resolved in Scratch.
pub struct RankInput<'a> { pub need: &'a Need, pub receiver_domain: Option<&'a FailureDomain>, pub input_root: Option<&'a Cid> }

/// Object-safe, so MethodRegistry can hold `Box<dyn Ranking>`. A method's score type is internal to it;
/// the order it emits is built from `RankKey` values.
pub trait Ranking: Send + Sync {
    fn method(&self) -> MethodId;
    /// The factors this method reads; `within_band` refuses a weight on any other (E2).
    fn factors(&self) -> &'static [Factor];
    /// A TOTAL order over `eligible`, best first, as indices into `eligible`. Pure; reads only the input,
    /// the candidates, the weights and the scratch's precomputed agent keys. `out` is cleared first.
    fn rank(&self, input: &RankInput<'_>, eligible: &[Eligible<'_>], incumbents: &[bool], weights: &Weights,
            scratch: &Scratch, out: &mut Vec<usize>);
}
/// Reused across passes: precomputed sha256(agent) per candidate (once per snapshot, not once per need),
/// index buffers. Capacity is retained.
#[derive(Default)] pub struct Scratch { /* agent_keys: HashMap<String, [u8; 32]>, buffers */ }
impl Scratch {
    /// sha256 of the agent key's UTF-8 bytes, as placement.rs::hash_into_keyspace computes it. `decide`
    /// fills it for every candidate in the snapshot before any `rank` call; None only for an agent it
    /// was not given, and a method that sees None computes the same digest itself.
    pub fn agent_key(&self, agent: &str) -> Option<&[u8; 32]>;
}

pub struct XorDistance;      // = XorDistanceStrategy, moved, byte-identical; factors: [] (reads no weight)
pub struct DiversityAware;   // = DiversityAwarePlacementStrategy, moved; factors: []; its greedy emits a full order
pub struct NearestBytes;     // task kind; factors: [NearBytes, Distance] (see §F)
pub struct WeightedCapacity; // custody tuning's method; factors: [Distance, SpareCapacity, AlwaysOn] (E2)

/// The one diversity pass, as today's placement.rs:207-237: repeated passes, each taking at most one
/// candidate per not-yet-used failure domain in rank order (a candidate with no domain is its own),
/// until `count` are taken or the pool is exhausted (D15). With no domain data it equals rank order.
pub fn diversify(ordered: &[usize], domains: &[&FailureDomain], count: usize, out: &mut Vec<usize>);

// How a Ranking turns RankKey into its output: build one key per eligible index, sort the indices
// GREATEST key first (ArbitrationKey's "greater wins": an incumbent `true` beats `false`, then the higher
// score, then the Reverse-wrapped smaller agent and smaller domain), and write them into `out`. The
// score type S is the method's own; XorDistance's is Reverse<[u8; 32]>, so a smaller XOR distance ranks
// higher. That is today's ascending (distance, agent_cid, domain) sort, read with greatest first.
impl Ranking for XorDistance {
    fn method(&self) -> MethodId { MethodId::named("elohim:placement:xor-distance@1") }
    fn factors(&self) -> &'static [Factor] { &[] }
    fn rank(&self, input: &RankInput<'_>, eligible: &[Eligible<'_>], incumbents: &[bool], _weights: &Weights,
            scratch: &Scratch, out: &mut Vec<usize>) {
        let need = sha256(&input.need.key.rank_bytes());                       // once per need
        let keys: Vec<RankKey<'_, core::cmp::Reverse<[u8; 32]>>> = (0..eligible.len()).map(|i| {
            let agent = eligible[i].agent().0.as_str();                        // AgentRef(pub String)
            let ak = scratch.agent_key(agent).copied().unwrap_or_else(|| sha256(agent.as_bytes()));
            ArbitrationKey {
                tier: incumbents[i],
                clock: core::cmp::Reverse(xor(&need, &ak)),
                tiebreak: (core::cmp::Reverse(agent), core::cmp::Reverse(&eligible[i].candidate().domain)),
            }
        }).collect();
        out.clear();
        out.extend(0..eligible.len());
        out.sort_by(|&a, &b| keys[b].cmp(&keys[a]));   // best first; total, because carriers are unique
    }
}
// `sha256` and `xor` are rank.rs's private helpers, the ones placement.rs uses today. `decide` then
// truncates `out` to the chosen prefix and, where the charter asks for domains, passes it through
// `diversify`. A method never truncates or diversifies by itself.

#[cfg(feature = "testing")]
pub mod contract {
    /// Binding on every Ranking: determinism; total order; no duplicates; output ⊆ input; empty → empty;
    /// agreement under any permutation of the input; weights reorder and never add or remove; a weight on
    /// a factor not in `factors()` changes nothing. Built on seam_contracts' ArbitrationBudget/
    /// ArbitrationReport; the list form is proposed upstream as `harness::arbitrated::check_ordering`,
    /// because `check_arbitration` returns one winner.
    pub fn assert_ranking_contract(r: &dyn Ranking);
}
```

**Compatibility, by golden test** (`rank::tests::matches_storage_strategy_vectors`): for every vector
in `reconcile/placement.rs`'s tests, `XorDistanceStrategy.rank(m, c, n)` equals `XorDistance`'s order
over `NeedKey::Blob(m)` with all candidates eligible, no incumbents and default weights, truncated to
`n`; the same for `DiversityAwarePlacementStrategy` and `DiversityAware`. `Snapshot.candidates` is unique
by carrier: the adapter dedupes duplicate-carrier candidates keeping the smallest `FailureDomain`, so the
dedup vectors (`placement.rs:294-303`, `413-422`) and the conflicting-duplicate regression
(`placement.rs:522-543`) hold over an index-based output (D12). One added vector pins multi-pass
diversity: A{a1,a2,a3}, B{b1,b2}, XOR order a1,a2,a3,b1,b2, n = 4 gives {a1, b1, a2, b2} (D15).
elohim-storage keeps `PlacementStrategy` as a thin shim over these, so no existing placement moves.

**D.6 Adoption: self-adopted, idempotent, make-before-break (R3, R4).**

```rust
// placement-core/src/adopt.rs
/// R1: long-lived. Renewed by nothing periodic: it lapses only on an observed carrier loss or a
/// revokes-commitment (spec §7.2). `until` is set only when the need itself is bounded (a task, a session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lease { pub from: EpochSecs, pub until: Option<EpochSecs> }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Period { pub start: EpochSecs, pub len_secs: u64 }      // one settlement per commitment per period

/// R2: below public, placement is household-local and not notarized. Derived, never chosen. The plane
/// names its owner: a carrier writes into the RECEIVER's household plane (D13), over the household
/// records transport, so the receiver's floor reads it as honored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "plane")]
pub enum RecordPlane { Notarized, HouseholdLocal { household: String } }
impl RecordPlane { pub fn for_need(need: &Need, receiver_household: Option<&str>) -> Self; }   // openness < Public → HouseholdLocal

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentDraft {
    pub id: String, pub need: CidString, pub kind: KindId, pub action: MishpatAction,
    pub provider: AgentRef,            // always the deciding peer: no peer assigns work to another
    pub receiver: AgentRef, pub key: NeedKey, pub in_scope_of: CidString,
    pub lease: Lease, pub plane: RecordPlane,
    pub method: MethodId, pub params: CidString /* the set's CID */, pub ranking: CidString /* its ranking_cid */,
    pub inputs_digest: [u8; 32],       // spec §7.4: what remains to explain it
    pub replaces: Option<String>,      // make-before-break successor of this commitment
}
/// CustodyBlob: byte-identical to rea_commitment_service::deterministic_custody_id(provider, receiver,
/// marker) ("custody-blob-" + 16 hex). Others: "<kind>-" + hex16(sha256(kind|provider|receiver|key)).
pub fn commitment_id(kind: KindId, provider: &AgentRef, receiver: &AgentRef, key: &NeedKey) -> String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carrying {
    pub commitment: String, pub need: Cid /* Need::id, D10 */, pub key: NeedKey, pub kind: KindId,
    pub carrier: AgentRef, pub receiver: AgentRef,
    pub lease: Lease, pub under_params: Cid /* the ranking_cid it was adopted under */,
    pub honored: Answer<bool>,         // claimed until a holding probe exists; Unreachable is not "not honored"
}
/// Only ever my own commitment (D18): another carrier's loss is a Carrying with honored Absent, never a
/// Release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release { pub commitment: String, pub provider: AgentRef, pub reason: ReleaseReason }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseReason { OverHeld, Superseded { by: String }, NeedWithdrawn }
/// Evidence class is required (I9): only `RealEvidence`, which only a `Real` reading constructs, can
/// settle; a fixture observation never reaches a settling draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FulfilmentDraft { pub commitment: String, pub period: Period, pub provider: AgentRef, pub quantity: u64,
                             pub unit: &'static str, pub evidence: RealEvidence }
/// The field is private, so no struct literal outside placement-core compiles (the `compile_fail`
/// doctest). `from_readings` is the only constructor, and it checks at run time: it refuses an empty
/// list and any reading whose `evidence` is not `EvidenceStage::Real` (a unit test passes it a Fixture
/// reading). It derives what `FulfilmentDraft` and `Plan` derive, so both still compile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealEvidence { readings: Vec<CidString> }   // the dag-cbor CID of each reading's canonical encoding
impl RealEvidence {
    pub fn from_readings<T: Serialize>(readings: &[seam_contracts::probe::Reading<T>]) -> Result<Self, PlaceError>;
    pub fn readings(&self) -> &[CidString];
}
/// The CARRIER's record of what it carried, once per commitment per period (spec §8 gap 1, R1).
pub fn settle(c: &Carrying, period: Period, quantity: u64, evidence: RealEvidence) -> FulfilmentDraft;
```

A notarized draft maps onto `shefa_types::CreateReaCommitmentInput` (finding 10), as
`SalvageCommitmentAuthor` builds it today; a household-local draft becomes a signed record in the
receiver's household plane, read by that household's own floor and probe. A cross-household carrier of a
below-public need is admitted only once that transport exists; until then it is ineligible
`CrossHouseholdBelowPublic` (D13).

**D.7 The charter, the snapshot and `decide`.**

```rust
// placement-core/src/decide.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LossDomain { Device, Household, Region }

/// The space's constitutional input (one of R5's three authorities). Not tunable by a parameter set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Charter {
    pub id: Option<Cid>,                          // None: no charter (every household today)
    pub demand: BTreeMap<LossDomain, Demand>,     // loss → holders and domains (spec §19 Q2 owner)
    pub serve_order: Vec<NeedClass>,              // scarcity order; empty: the built-in order, said so
    pub holding_floor: Demand,
    pub forwarding: Option<ForwardingRule>,       // a shared tier's appropriation rule (spec §12.3 rule 3)
    pub genesis_params: BTreeMap<KindId, Cid>,    // per kind, the sets the charter itself ratifies (§E; E7)
    pub tuning_rule: RatificationRule,            // how a tuning is ratified here (§E; E8)
    pub moves_per_event: (u16, u16),              // the charter-declared range for max_moves_per_event (E6)
    pub freshness_horizon_secs: u64,              // liveness cut-off for postures and inventory (D3, D4)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardingRule { pub max_in_flight: u16, pub bytes_per_window: u64, pub window_secs: u64, pub clause: String }
impl Charter { pub fn resolve(&self, loss: LossDomain) -> Demand; }   // never below holding_floor

/// One Floor per peer across every scope it is a member of (D9): capacity, which a Posture offers
/// without a scope, is charged once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub me: AgentRef, pub me_aliases: Vec<String>,  // legacy transport ids of me (D5)
    pub now: EpochSecs, pub period: Period,
    pub charters: BTreeMap<CidString, Charter>,     // per scope
    pub needs: Vec<Need>,
    pub candidates: Vec<Candidate>,                 // every posture held, unique by carrier, resolved by the observer
    pub carrying: Vec<Carrying>,                    // observed commitments, mine pending included, and their honored answers
    pub params: BTreeMap<(CidString, KindId), InForce>,   // §E, keyed by (scope, kind) (E15); absent → InForce::builtin
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Plan { pub adopt: Vec<CommitmentDraft>, pub release: Vec<Release>, pub settle: Vec<FulfilmentDraft>,
                  pub unserved: Vec<NotCarried>, pub observe_only: Vec<(Cid /* need id */, MethodId)> }

/// Computes the WHOLE deterministic assignment, then returns `me`'s slice, so the agreement test is
/// checkable directly. Needs are taken in (charter serve-order position of class, need id); capacity
/// is charged across kinds and scopes in that order within the pass. For each need:
/// - scope: a need is ranked under the set of (need.in_scope_of, kind); a need whose scope has no
///   charter here is observe-only;
/// - eligibility: every candidate through `admit`; refusals are kept, never ranked away;
/// - incumbency: a carrier is incumbent iff its commitment is honored and `under_params` equals the
///   `ranking_cid` of the set now in force for that (scope, kind) (E5). Incumbents rank first, so offer churn moves nothing
///   ("heals fill, never move"), while a newly ratified set, which resets incumbency, re-places;
/// - fill (D1): adopt iff (honored < demand.holders OR honored distinct domains < demand.domains) and
///   me ∈ the diversified chosen prefix, and me holds no live commitment for this need already, pending
///   or unhonored (D5: no re-draft every pass);
/// - move (only after a ratified set changed): adopt the newly chosen holder with `replaces` set; the
///   superseded commitment is released only once its successor reads honored Present(true), and at
///   most `InForce::max_moves_per_event` moves happen per event (concern C6a);
/// - drain (D1): release my commitment only when releasing keeps both honored >= holders and honored
///   distinct domains >= domains, and I am the lowest-ranked honored carrier whose release keeps both;
/// - observe: a need whose in-force MethodId is not in `methods` adopts nothing (concern C10);
/// - scarcity: an unmet need is NotCarried by name, with R7's three parts, never dropped.
pub fn decide(s: &Snapshot, kinds: &KindRegistry, methods: &MethodRegistry, scratch: &mut Scratch) -> Plan;

/// Device membrane: residency and the device's own Posture. Its inputs never leave the device.
pub struct PrivateSnapshot { pub facts: DeviceFacts, pub choices: PersonChoices, pub needs: Vec<PrivateNeed>,
                             pub me: AgentRef, pub reach: Reach /* D20: the posture's */, pub now: EpochSecs,
                             pub params: Option<DeviceParams> }
/// A person-ratified tuning of their own device's floor; device state, not Serialize, never published
/// (spec §7.3; E16).
pub struct DeviceParams { pub weights: Weights }
pub struct PrivatePlan { pub posture: Posture, pub keep: Vec<NeedKey>, pub deferred: Vec<NeedKey>,
                         pub refusals: Vec<(KindId, Unanswered<IneligibleReason>)> }
pub fn decide_private(s: &PrivateSnapshot) -> PrivatePlan;
```

**D.8 The reconcile loop and its ports.**

```rust
// placement-core/src/floor.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    PostureChanged(AgentRef), MembershipChanged, NeedExpressed(Cid), NeedWithdrawn(Cid), Miss(Cid),
    CommitmentObserved(String), ParametersDeclared { scope: Cid, kind: KindId, set: Cid },
    /// Clock-born (D4): emitted by the custody Ports adapter when a carrier's inventory or presence
    /// crosses the charter's freshness horizon, at most once per carrier per horizon. A carrier that goes
    /// silent produces no event of its own, so loss must be observed by the passage of time.
    CarrierLost(AgentRef),
    /// Clock-born: one per period per peer; it settles and never re-places.
    PeriodClosed(Period),
}
/// A skip test only (D16): a trigger whose affected set is empty skips the pass; every pass that runs is
/// whole, so capacity is charged the same way as a full pass.
pub fn affected(trigger: &Trigger, s: &Snapshot) -> bool;

#[derive(Debug, thiserror::Error)] #[error("port: {0}")] pub struct PortError(pub String);

/// Sync, like salvage_pass; async adapters bridge with block_in_place exactly as SalvageCommitmentAuthor
/// does today. Implemented outside this crate.
pub trait Ports {
    /// One read pass into a reused snapshot. One grouped query per table, never one per provider per blob.
    /// It calls `select_in_force` (§E) for each (scope, kind).
    fn snapshot(&self, scopes: &[Cid], now: EpochSecs, into: &mut Snapshot) -> Result<(), PortError>;
    fn author(&self, draft: &CommitmentDraft) -> Result<(), PortError>;   // idempotent by id
    fn release(&self, release: &Release) -> Result<(), PortError>;
    fn settle(&self, f: &FulfilmentDraft) -> Result<(), PortError>;
}
pub struct Floor<P: Ports> { ports: P, kinds: KindRegistry, methods: MethodRegistry, scopes: Vec<Cid>, snapshot: Snapshot, scratch: Scratch }
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PassOutcome { pub adopted: u32, pub released: u32, pub settled: u32, pub unserved: u32, pub observed_only: u32 }
impl<P: Ports> Floor<P> {
    pub fn new(ports: P, kinds: KindRegistry, methods: MethodRegistry, scopes: Vec<Cid>) -> Self;
    /// Event-driven: no polling timer. The adapter coalesces triggers under the posture hysteresis into one call.
    pub fn on(&mut self, trigger: &Trigger, now: EpochSecs) -> Result<PassOutcome, PortError>;
}
```

**D.9 The explanation, recomputed rather than stored (spec §7.4).** Each commitment carries method,
parameter-set CID and inputs digest. Asking why re-runs the pure ranking for one need over the inputs
the peer holds; once those are gone, the three fields are what remains, and the answer says so.

```rust
// placement-core/src/explain.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub need: CidString, pub key: NeedKey, pub kind: KindId,
    pub method: MethodId, pub parameters: RatificationRef,
    pub fallback: Option<ParamFallback>,          // which fallback applied, when one did (E10)
    pub inputs_digest: String,                    // 64 hex characters on the wire (G13)
    pub own_refusals: Vec<Ineligible>,            // the asker's own devices, listed
    pub others_refused: BTreeMap<String, u32>,    // other members' refusals, COUNTED by label, never listed
    pub eligible: u32,
    pub chosen: Vec<AgentRef>,                    // named only at the reach their posture declared
    pub outcome: Outcome,
    pub digest_matches_commitment: Option<bool>,  // None: no commitment to compare
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "outcome")]
pub enum Outcome { Carried, Held, NotCarried { why: Unanswered<NotCarriedReason> }, InputsGone { why: Unanswered<NotCarriedReason> } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "reason")]
pub enum NotCarriedReason { NoEligible, DomainShort { have: u16, need: u16 }, ServedFirst { class: NeedClass }, NoPriorityChosen, MethodNotRun, InputsGone }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotCarried { pub need: CidString, pub key: NeedKey, pub receiver: AgentRef, pub why: Unanswered<NotCarriedReason> }

pub fn explain(s: &Snapshot, kinds: &KindRegistry, methods: &MethodRegistry, need: &Cid, asker: &AgentRef,
               committed: Option<[u8; 32]>) -> Option<Explanation>;
```

**Contract for D** (`placement/core/tests/floor_contract.rs`, with `Ports` doubles; `Snapshot`,
`InForce` and `Plan` derive `Debug, Clone, PartialEq, Eq` so the seam-contracts harnesses compile, D14):

1. **Agreement.** Snapshots differing only in input order give identical plans for the same `me`.
   Simulating `decide` for every candidate as `me`, the union of adoptions equals the global chosen set.
2. **Self only.** Every draft's provider is `me`, and every release names a commitment whose provider is
   `me` (D18).
3. **Idempotent.** `seam_contracts::harness::check_quiescence(initial, decide, apply)` holds: a replay
   authors nothing, and a pending own commitment is not re-drafted.
4. **Make-before-break.** No plan releases a superseded commitment whose successor is not honored
   `Present(true)`; honored never falls below `min(holders, honored_before)` across a re-ranking; at most
   `max_moves_per_event` moves per event.
5. **Hysteresis.** Under an unchanged parameter set, an honored incumbent is never moved because a
   better-ranked candidate appears, with one carve-out: a domain-gap fill followed by the drain of an
   over-held domain, which keeps both holder and domain counts throughout (D1).
6. **Revert.** Declaring a new head whose value is an earlier set's CID gives, once the moves settle,
   the placement that set's ranking gives over the current inputs.
7. **Every need appears once**, as `Carried`, `Held` or `NotCarried`.
8. **No `IneligibleReason` is reachable through any parameter set** (property test over generated sets).
9. **R2.** Reach below public gives `HouseholdLocal { household }` naming the receiver's household.
10. **No ceiling needed.** With no `InForce` held, placement converges under the built-in set.
11. **Idle is free.** With unchanged inputs a pass writes nothing; `PeriodClosed` only settles;
    `CarrierLost` fires at most once per carrier per horizon.
12. **Explainable.** `explain` over the adoption snapshot returns `digest_matches_commitment == Some(true)`,
    and lists no other member's refusal.
13. **Legacy behaviour kept.** Named behaviours preserved through the custody `Ports` and the legacy
    mapping above: no existing placement moves on the first pass; a self already committed but not yet
    honored is not re-drafted; a transport-id provider row counts as `me`; a dead carrier is re-placed
    after the horizon. Tests rewritten rather than kept: the `StaticStrategy` stub tests in
    `reconcile/custody.rs` (decide takes no injected strategy) and the self-append tests in
    `services/salvage_commitment_author.rs` (they now seed self's capacity row).
14. **A dead carrier does not hold a need.** A candidate whose posture is past the horizon is
    `Unknown { input: "liveness" }`, and the need re-fills.
15. **Capacity is charged once across scopes** for a carrier in two spaces.
16. **Every refusal carries R7's three parts.**

**Who implements and who calls.**

- Custody kinds (`CustodyBlob`, `CustodyRoot`): `elohim-storage/src/reconcile/custody_floor.rs`
  (follow-on) implements `Ports` over `rea_commitments`, `peer_blob_inventory` (read once per pass),
  `salvage_capacity` (the custody posture source, its `last_seen_at` the liveness input), the `humans`
  household join and `SalvageCommitmentAuthor`, and emits `CarrierLost` at the horizon. `salvage_pass`
  becomes the custody kind's driver; `reconcile_pass` still moves the bytes ("salvage authors intent;
  reconcile moves bytes"), now through a `TierChain`.
- Task kind: ark-supervisor (§F). Device membrane: the device's own storage process calls
  `decide_private`.
- Callers of `Floor::on`: the reconcile task in elohim-storage's `main.rs`. Today's
  `salvage_recheck_seconds` ticker stops being a placement trigger; the freshness horizon replaces it as
  the one bounded clock for loss.

**Worked example.** A household document tree has demand `{2, 2}` from the charter's `Household` loss.

- The phone is on battery, so its posture offers no custody for other households; for its own household
  it offers custody `household_only` and `deferred`, so it is eligible and holds in the charging window.
- The laptop and the hub are both `hh:A`, admitted `intimate`.

`decide` on either peer chooses `[hub, laptop]` (the phone, deferred, ranks after them); honored 2 meets
`holders: 2` but honored domains 1 is below `domains: 2`, so the outcome for the second domain is
`NotCarried { DomainShort { have: 1, need: 2 } }` with `come_back: OnEvent("posture-changed")`; both
adopt (`HouseholdLocal { household: hh:A }`), and both explanations are identical. When the
grandparents' household `hh:B` posts a posture, it is ineligible `CrossHouseholdBelowPublic` for this
intimate tree until the cross-household household-records transport exists. For a `public` document
tree with the same demand, `hh:B` fills the domain gap (fill fires on the domain count), adopts
`Notarized`, and once it reads honored the laptop, the lowest-ranked holder whose release keeps two
holders and two domains, drains. The hub and `hh:B` are untouched.

### E. The ceiling: draft, ratification reference, parameter-set head (interfaces only)

**Owner:** `placement/core/src/params.rs` and `src/ceiling.rs`. The ceiling changes the floor's inputs,
never its outcome (spec §7.3). R5's three authorities each live in exactly one type: the carrier's
`Posture`, the space's `Charter` (demand and domains, serve order, holding floor, forwarding rule, the
tuning rule and the move range), and the parameter set (`InForce`). Serve order, loss resolution and
household diversity are the charter's and are not in a parameter set.

```rust
// placement-core/src/params.rs
/// Scope-free, so the built-in set has one CID everywhere and ships with the floor (E1). Its CID is over
/// {schema, kind, method, weights, max_moves_per_event} only; where it is in force is the head's fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterSet {
    /// The version of this struct's canonical encoding: 1 for the fields below, and bumped only when a
    /// field is added, removed or reinterpreted. It is not a protocol version. It is part of `cid()`
    /// and not of `ranking_cid()`, so re-encoding the same ranking under a new schema keeps incumbency.
    /// A floor refuses a value it does not know, as `ParamRefusal::Unverifiable { input: "schema" }`.
    pub schema: u32,
    pub kind: KindId,
    pub method: MethodId,
    pub weights: Weights,                // every factor the method reads, each > 0 (E2, E6)
    pub max_moves_per_event: u16,        // C6a; within the charter's declared range (E6)
}
impl ParameterSet {
    pub fn cid(&self) -> Result<Cid, PlaceError>;          // atom_cid of the canonical encoding (E17)
    /// The incumbency key (E5): over (kind, method, weights) only, so republishing a set at another reach
    /// or move cap does not reset incumbency.
    pub fn ranking_cid(&self) -> Result<Cid, PlaceError>;
}
#[derive(Debug, thiserror::Error)] #[error("{0}")] pub struct PlaceError(pub String);

/// The space's head (E14): one per space, as the design gate's (declarer, space, `placement-parameters`)
/// head identity, naming one set per kind. The scope and the predecessor live here, not in the set (E1, E5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeadDeclaration {
    #[serde(with = "cid_serde")] pub scope: Cid,
    pub sets: BTreeMap<KindId, CidString>,
    pub reach: Reach,                    // published at (who may see it); in force for the space's members
    pub predecessor: Option<CidString>,  // the head declaration this one follows
    pub ratification: RatificationRef,
    pub ratified_at: EpochSecs,          // the ratification's notarized timestamp (spec C2; E4)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatificationRef { pub by: RatificationKind, pub id: String /* record hash or charter CID */,
                             pub rule: Option<CidString>, pub touches: Vec<AgentRefKey> }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RatificationKind {
    /// The floor's built-in sets, content-addressed and shipped with the floor (spec §7.3). Nobody put
    /// them in force; they run when nothing has been. Explanations say "no tuning chosen".
    BuiltIn,
    /// Sets the space's charter itself names (Charter::genesis_params); the charter's adoption ratified them.
    Charter,
    /// A tuning ratified under the charter's tuning_rule: id = the ratifying act's hash; rule = the clause
    /// for `Clause`; touches = whose consent `TouchedConsent` requires (E8).
    Ratified,
}
/// A struct with a kind field rather than an internally tagged enum carrying CIDs, so its JSON and
/// DAG-CBOR forms are both honest (G1).

/// How a space ratifies a tuning (E8). The household default is each touched member's consent (the
/// design gate); a charter may name a clause and require drafts from several lineages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatificationRule { TouchedConsent, Clause { cid: Cid, min_lineages: u16 } }

/// The ONLY parameter form the floor accepts. Fields private; the CID is computed at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InForce { set: ParameterSet, cid: Cid, ranking_cid: Cid, ratification: RatificationRef }
impl InForce {
    /// The built-in set for a kind: CustodyBlob → XorDistance (the legacy method, reading no weights, so
    /// no tuning is in band under it); CustodyRoot and the other custody-shaped kinds →
    /// WeightedCapacity with every factor it reads present and non-zero; Task → NearestBytes (E2).
    pub fn builtin(kind: KindId) -> Self;
    /// Verifies BuiltIn against `builtin(set.kind).cid` and Charter against `charter.id` and
    /// `charter.genesis_params[set.kind]` IN CORE; only Ratified is handed to `check` (E9). Refuses a
    /// set whose kind is not `kind` or whose head names another scope (E15).
    pub fn from_head(scope: &Cid, kind: KindId, set: ParameterSet, head: &HeadDeclaration,
                     check: &dyn RatificationCheck, charter: &Charter) -> Result<Self, Unanswered<ParamRefusal>>;
    pub fn set(&self) -> &ParameterSet;
    pub fn cid(&self) -> &Cid;
    pub fn ranking_cid(&self) -> &Cid;
    pub fn ratification(&self) -> &RatificationRef;
    pub fn max_moves_per_event(&self) -> u16;
}
/// R7's typed reason for the ceiling (E10); every refusal arrives as Unanswered<ParamRefusal>, with
/// come_back OnEvent("parameters-declared") or the missing record's arrival, and meanwhile
/// BuiltInInForce or UseHeld { the last verified set }.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "reason")]
#[non_exhaustive]
pub enum ParamRefusal { Mismatch, Refused, Unverifiable { input: String }, UnknownFactor, UnknownMethod, Band { rule: String } }
/// Which fallback an explanation reports (E10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamFallback { BuiltIn, LastVerified }

/// Pure (E3, E13): filter the declarations with `check` FIRST, then elect among the verified ones only,
/// with `seam_contracts::select_arbitrated_winner` over `ArbitrationKey { tier: (), clock: ratified_at,
/// tiebreak: head CID }` (E4: one tier; newest ratification wins, so a charter amendment naming a new
/// genesis set displaces an older tuning, and a later ratified tuning displaces it in turn). An
/// unverifiable newer declaration never shadows an older verified one.
pub fn select_in_force(scope: &Cid, kind: KindId, decls: &[HeadDeclaration], sets: &dyn SetSource,
    check: &dyn RatificationCheck, charter: &Charter, last_verified: Option<&InForce>)
    -> (InForce, Option<Unanswered<ParamRefusal>>);
pub trait SetSource { fn set(&self, cid: &Cid) -> Answer<ParameterSet>; }

// placement-core/src/ceiling.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PressureKind { NotCarried, UnderHeld, Churn, UnevenCost, MemberConcern }
/// Counts only; a key is admitted only for the drafter's own person's need at or wider than the draft's
/// reach (E11). Fields private; built by `Pressure::count` or `Pressure::own_need`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pressure { kind: PressureKind, key: Option<NeedKey>, count: u32 }
impl Pressure {
    pub fn count(kind: PressureKind, count: u32) -> Self;
    pub fn own_need(kind: PressureKind, need: &Need, drafter: &AgentRef, reach: Reach) -> Result<Self, PlaceError>;
}
/// Drafted by a member's elohim, named by its layered identity; never by a space-elohim (R5). It carries
/// the space's reach, so it travels to `touches` only within it (E11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft { pub proposed: ParameterSet, pub against: CidString, pub scope: CidString, pub reach: Reach,
                   pub drafted_by: AgentRef, pub lineage: String, pub pressures: Vec<Pressure>, pub touches: Vec<AgentRef> }
/// A tuning may only reweight (E2, E5, E6): same kind and method; the same weight keys, every key a
/// factor `method.factors()` lists, every weight > 0; max_moves_per_event inside the charter's range. A
/// revert (proposed equals a set this space verified before) is exempt from the band and is checked
/// against that set. Whether a set leaves some member's carrier unchosen for every need (an exclusion)
/// is not detectable here; it is a ratification question.
pub fn within_band(head: &ParameterSet, proposed: &ParameterSet, method: &dyn Ranking, charter: &Charter)
    -> Result<(), Unanswered<ParamRefusal>>;

/// Implemented by the adapter that reads the space's declared head: EVERY candidate declaration it holds
/// (E3), so election happens after verification, in core.
pub trait ParameterHead: Send + Sync { fn declarations(&self, scope: &Cid) -> Answer<Vec<HeadDeclaration>>; }
/// Implemented by the adapter for `Ratified` only: do the ratifying act's signers satisfy the charter's
/// tuning_rule for these touches? Re-derived by every floor (concern C5).
pub trait RatificationCheck: Send + Sync {
    fn verify(&self, r: &RatificationRef, charter: &Charter) -> Result<(), Unanswered<ParamRefusal>>;
}
/// Implemented by a member's elohim. Nothing in placement-core calls a model.
pub trait TuningSink: Send + Sync { fn propose(&self, draft: &Draft) -> Result<String, PortError>; }
```

**Contract** (`placement/core/tests/ceiling_contract.rs`, with doubles; E13).

1. No conversion from `Draft` to `InForce` exists (rail R-P6), and a lenient `RatificationCheck` cannot
   admit a draft as `BuiltIn` or `Charter`: core checks those itself.
2. `admit` takes no parameters, so a tuning cannot widen eligibility.
3. An unratified head is never in force: `select_in_force` over declarations whose newest fails `verify`
   elects the newest verified one; `Absent` or `Unreachable` from `ParameterHead` leaves the last set this
   floor verified, or the built-in one, and the explanation names which (`ParamFallback`).
4. A revert is a new declaration whose value is an earlier set's CID, ratified the same way; the head
   never moves backward (spec §7.3, concern C2). By D contract 6 it restores that set's placement.
5. Every peer reading the same declarations computes the same `InForce` and the same plan.
6. `within_band` refuses a weight on a factor the method does not read, a removed or added key, a weight
   ≤ 0, and a move cap outside the charter's range.
7. A set republished at another reach keeps its `ranking_cid`, so no incumbent moves.

**Implementors.** `ParameterHead`, `SetSource` and `RatificationCheck`: elohim-storage, reading
household-local head records below public (R2) and EPR head declarations at collective or commons reach
(no new entry type); the `Ports::snapshot` adapter calls `select_in_force` for each (scope, kind).
`TuningSink`: the elohim's drafting surface (a later, unnumbered slice).

**Worked example.** Explanations show the hub carrying 90% of the household's custody
(`Pressure::count(UnevenCost, …)`). Matthew's elohim drafts `{SpareCapacity: +2000}` (a reweight of a
factor `WeightedCapacity` reads, every other key kept and non-zero) against the built-in set for
`CustodyRoot`, at the household's reach, naming the two members it touches. The household has no charter,
so its tuning rule is the default `TouchedConsent`: both touched members consent, and the ratification
record lists them in `touches`. The household declares a head naming that set for `CustodyRoot`. Every
peer's `Trigger::ParametersDeclared` elects it (verified, newest), which changes `ranking_cid` and so
resets incumbency for that kind: the laptop's free space ranks it above the hub for some roots, it adopts
with `replaces`, and the hub releases each one only after the laptop reads honored, at most
`max_moves_per_event` per event. Reverting is a new declaration naming the built-in set's CID.

### F. A task placed on a berth

**Owner:** `ark/core/src/task.rs` (new, follow-on). ark-core gains `eprfs-core` (path dependency into
the eprfs workspace, as elohim-storage already takes `ark/core`) and `placement-core`; both pass R-A1.

```rust
// ark-core/src/task.rs
/// Spec §12.6: (method, input root, view, envelope). The device names the task and never ships inputs.
/// No requester-written scope (F1): the scope is derived one way from the method, as
/// `compute_tasks::required_scope` does today, so granted authority never comes from the requester's
/// own bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskSpec {
    pub method: MethodDescriptor,                 // §C; its `exec` is the CID of (child, envelope) (F2)
    pub input: TreeView,                          // what it reads: root + the descriptor's view (§B.3)
    pub base_root: Option<BlobCid>,               // a root the requester believes a berth holds, so the
                                                  //   delta is fetched through the tier chain (F9)
    pub child: TaskChild,                         // F12: no restart policy to get wrong
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<RuntimeEnvelope>,        // existing six-field contract (manifest.rs)
    pub outputs: Vec<String>,                     // output path prefixes, relative to `{output}` (F13)
}
/// The task-shaped subset of ChildSpec: artifact (ArtifactRef::Pinned), argv, env, env_scrub, stdin,
/// readiness. It has no restart policy; it converts to a ChildSpec with `Restart::Temporary` only (F12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskChild { /* artifact, argv, env, env_scrub, stdin, readiness */ }
impl TaskSpec {
    /// Refuses when `method.exec` is not the CID of (child, envelope), when `input`'s paths are not the
    /// descriptor's view, or when the method's derived scope is none (F2).
    pub fn new(method: MethodDescriptor, input_root: BlobCid, base_root: Option<BlobCid>, child: TaskChild,
               envelope: Option<RuntimeEnvelope>, outputs: Vec<String>) -> Result<Self, ManifestError>;
    pub fn cid(&self) -> Result<Cid, ManifestError>;      // canonical dag-cbor; an absent envelope moves nothing
    pub fn need_key(&self) -> Result<NeedKey, ManifestError>;   // NeedKey::Task(self.cid()?)
    /// The scope, one way from the method (required_scope moved into placement-core) (F1).
    pub fn scope(&self) -> Option<GrantScope>;
    /// (method CID, projected input): the method CID already binds the program that runs (F2).
    pub fn derivation_key(&self, projected: &ProjectedRoot) -> Result<DerivationKey, ManifestError>;
    /// kind Task, demand {1,1}, Requirements from envelope.bound (pids included; bytes = disk_bytes plus
    /// the materialized view's size) and the envelope's isolation, NeedTerms::Task { input_root, scope,
    /// base_root }. Consent is the snapshot builder's to resolve per candidate (F5): NotNeeded for the
    /// requester's own envelope, Granted from the requester's delegates-compute grant otherwise.
    /// `carriers` narrows every peer's admit (`--on`, F14).
    pub fn as_need(&self, requester: AgentRef, space: Cid, reach: Reach, class: NeedClass,
                   carriers: Option<BTreeSet<AgentRefKey>>) -> Result<Need, ManifestError>;
}

/// The public projection of a local Berth. Built only by `from_berth`, which copies no data_root, port,
/// artifact path or passphrase (R6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BerthOffer {
    pub blade: AgentRef, pub berth: CidString /* CandidateContent.berth */, pub manifest: String, pub incarnation: u64,
    pub isolation: Option<Isolation>,     // None for a berth with no process yet
    pub capacity: Capacity,               // None fields when the manifest declares no envelope
    pub holds: Vec<(CidString, Reach)>,   // sorted, at most 64 roots kept at depth 3, each with its own reach (F8)
    pub scopes: BTreeSet<GrantScope>,
}
impl BerthOffer {
    /// capacity = From<&envelope.bound> (None fields without an envelope); isolation = the weakest
    /// `effective_tier` across the passport's processes, None for an empty passport (F7).
    pub fn from_berth(berth: &Berth, berth_cid: Cid, blade: AgentRef, manifest: &RuntimeManifest,
                      passport: &Passport, holds: Vec<(Cid, Reach)>, scopes: BTreeSet<GrantScope>) -> Self;
    /// Offers KindId::Task with scopes and isolation in its OfferTerms, and lists only the holds whose own
    /// reach is no narrower than the posture's (F8): no posture names a root below its own reach.
    pub fn posture(&self, reach: Reach, written_at: EpochSecs) -> Posture;
}
impl From<EffectiveTier> for Isolation { /* 1:1 onto placement_core::Isolation */ }
impl From<&ResourceQuota> for Capacity { /* memory_bytes, cpu_millis, pids; disk_bytes → bytes */ }

/// impl PlacementKind: receiver_may_carry = true; refuses `Kind { label: "scope" }` when the offer's
/// scopes lack the need's derived scope, `Kind { label: "isolation" }` when the offer's isolation is
/// weaker than the need's, and `OverCapacity` when capacity < the need's requirements (pids included);
/// default method = NearestBytes.
pub struct TaskKind;
/// Settles onto the existing provider-authored `rakia-compute-complete-v1` EconomicEvent (F11), not a
/// parallel record.
pub struct TaskOutcome { pub task: Cid, pub exit: ExitClass, pub output: Option<BlobCid>,
                         pub record: Option<DerivationRecord>, pub completion_event: String }
```

`NearestBytes` ranks (F9): berths whose `holds` include the need's `input_root` first; then berths whose
`holds` include its `base_root` (the delta is then a few subtrees fetched through the berth's tier
chain); then the receiver's domain; then XOR. Root equality alone would never prefer a remote berth for
a fresh edit, since an uncommitted edit's root exists only on the requester's device.

**Flow, in order; each step short-circuits the ones after it.**

1. The requester runs `walk_spine` over the input under the descriptor's view, then `project_view`,
   giving P.
2. `DerivationIndex::lookup(me, input, method)`. An accepted record whose output arrives: fetch the
   outputs only, raise no need, stop.
3. Otherwise the device membrane decides first whether to run it here (`decide_private`); if not,
   `as_need`. The requester's own envelope is the first candidate (`receiver_may_carry`); when the floor
   finds it ineligible by capacity, each berth's floor admits and ranks with `NearestBytes`.
4. The chosen berth self-adopts by accepting the request: the request is **provider-less** (a
   coordinator change to `content_store/src/compute_task.rs`, which moves no DNA hash), the first
   accepting berth's `accept` event is the commitment, and a second accept for the same request is
   refused by the zome (first-acceptance-wins), so the task has one commitment even though two berths may
   both decide to adopt (F10). The grant is R3's consent.
5. Its ark materializes the view at depth 1 or 2 from its own `TierChain`, runs `child` in the envelope
   with `{input}` (the materialized root's directory) and `{output}` (`{data_root}/out/<task cid>`) as
   template keys the TaskRunner resolves beside the berth's own (F13), and seals the outputs under
   `{output}` to a root.
6. On a clean exit (`ExitClass` success) it records the `DerivationRecord` and authors the existing
   `rakia-compute-complete-v1` event that fulfils the request (F11); on any other exit it authors the
   completion or decline with the exit class and records no derivation, so a failed run never
   short-circuits a later request. The receipt's `status`, `exitCode` and `observedTests` ride on that
   event.
7. The requester runs `fetch_blobs(input_root, derived output items, …)` over the output leaves only.
8. The requester withdraws the need (`Trigger::NeedWithdrawn`); the `TaskSpec` stays only as the key of
   the derivation record, so a completed task is never re-adopted when its lease ends (F15).
9. With no berth reachable and the device ineligible, the task is kept and shown as not carried,
   through a `PendingStatus` cursor with R7's three parts (`come_back: OnEvent("berth-offered")`,
   `meanwhile: RunHere`), and runs when a berth arrives or the person runs it here.

**Contract.** A derivation hit raises no need; the `TaskSpec` CID is unmoved by an absent envelope (the
rule of ark-core's `live_manifest_cid_is_unmoved_by_absent_envelope`); `TaskSpec::new` refuses a child
or envelope whose CID is not the descriptor's `exec`; two specs with the same method CID run the same
program; no `BerthOffer` built by `from_berth` contains a local path, and no posture lists a root below
its own reach; a berth without the derived scope is `Kind { "scope" }`; with no matching grant every
remote berth is `NoConsent`; the requester's own envelope is never `IsReceiver`; `NearestBytes` puts a
berth holding the input root above one holding only the base root, and that above one holding neither,
whatever the XOR order; a single-holder task is a top-1 choice, so `check_arbitration` applies; the
requester's counted fetches equal the output leaves; one task request yields one commitment; a failed
run records no derivation; a completed task is never re-adopted; `--on` narrows every peer's admit
(`NotNamed`).

**First instances.** `just measure` (`genesis/agentic/compute/measure.sh`) is the first task kind;
rakia's executor is the second. `compute-task.schema.json`'s chunk payloads give way to `input`, since
inputs are named, not shipped; `compute-receipt.schema.json` maps onto the completion event's metadata
(status, exit code, observed tests) plus `TaskOutcome`; `compute_tasks.rs::required_scope` moves into
placement-core as the one-way mapping from the method onto the scope; `ArtifactRef::Pinned` is the raw
CID through `elohim_epr::cid::raw_cid_from_sha256_hex`; ark-core's `rea.rs` projection is where the
berth offer's REA intent is written.

**Worked example.** `just measure genesis/a2o/features/compute/x.feature --on jessica`. The method is
the a2o-stage descriptor, marked `deterministic: false` (a measure reads the live household), whose view
is the system under test's path set from `genesis/a2o/scripts/lib/sut.ts` (`elohim/elohim-storage`,
`doorway/doorway-service`, the renderer, the conductor and DNA sources, the fixtures file) plus the
feature and the a2o scripts, and whose `non_path_inputs` carry the declared identities with no path
(`dnaHash` from `E2E_DNA_HASH`, the `A2O_SUT_*` identities); the input root is the monorepo root,
including the dirty digest of an uncommitted edit. P is unchanged since yesterday (no SUT edit), so the
record jessica's berth wrote under grant is found and the report fetched: reusing an `UnderGrant` record
for an unchanged SUT is what `just measure` asks for by default, and `--fresh` refuses reuse. After an
edit to elohim-storage P moves. `--on jessica` sets `requires.carriers = {jessica}`, so every peer's
admit refuses any other carrier. Her berth holds yesterday's root, which the requester names as
`base_root`; it adopts by accepting, fetches the changed subtrees through its tier chain, materializes
and runs. The requester fetches a report tree of three files and withdraws the need.

### G. Manifest needs and the SDK schemas

Lane S writes these files; the shapes are fixed here. Each is born under `elohim/sdk/schemas/v1/` with
"Source of truth:" in its description, camelCase, `additionalProperties: false`, an `epr:schema:…` `$id`,
enums by `$ref`, and the answer envelope wherever an absence has two provenances
(`views/CONVENTIONS.md`).

**CID encoding, per field (G1).** Every CID in an S3 or S4 schema is a string (`^bafy…`, CIDv1, with
`^bafyrei` for dag-cbor objects). `cid::Cid` 0.11's own serde writes bytes (`serialize_newtype_struct`)
and reads only bytes or a sequence, so on the Rust side every CID field carries
`#[serde(with = "cid_serde")]` (`epr-rea/src/index.rs`'s helper, which branches on
`is_human_readable`), and no CID sits inside an internally tagged enum, whose serializer reports
human-readable whatever it wraps and would re-address canonical bytes: `Expresser` and
`RatificationRef` are therefore structs with a kind field (§D.2, §E), and CIDs in sets and maps are
`CidString`. A contract vector pins it: a JSON fixture deserializes, re-encodes to DAG-CBOR, and gives the
same `Need::id` as one built in Rust.

| Item | File | `$id` | Rust mirror |
|---|---|---|---|
| R7 | `objects/unanswered.schema.json` | `epr:schema:object:unanswered` | `seam_contracts::Unanswered<R>` (per-seam reason by `$ref`) |
| S1 tree view | `objects/tree-view.schema.json` | `epr:schema:object:tree-view` | `eprfs_core::TreeView`: `{ root: string (CIDv1 dag-cbor, ^bafyrei), paths: string[] }`, `minItems: 1`, each path matching `^$\|^[^/\u0000]+(/[^/\u0000]+)*$` with `.` and `..` refused per component; `""` is the root, so `View::whole()` is `[""]` (G9). The canonical form is the normalized view (sorted component-wise, an antichain), and a shared vector shows `["a","a/b"]` refused on decode and normalizing to `["a"]` with the same `TreeView::cid`. `$defs.ViewPaths` = the paths alone |
| S2 needs | `manifest/app-manifest.schema.json`, one optional `needs` property, its shapes nested under `$defs.ManifestNeeds.$defs` (G15) | — | `placement_core::manifest::ManifestNeeds` |
| S3 need | `intents/placement-need.schema.json` | `epr:schema:intent:placement-need` | `Need` |
| S3 berth offer | `intents/berth-offer.schema.json` | `epr:schema:intent:berth-offer` | `BerthOffer` |
| S3 posture | `intents/posture.schema.json` | `epr:schema:intent:posture` | `Posture` |
| S3 tier record and absence | `objects/tier-record.schema.json`, `objects/item-absence.schema.json` (H2) | `epr:schema:object:tier-record`, `…:item-absence` | `TierRecord`, `ItemAbsence` |
| S4 parameter set | `objects/placement-parameter-set.schema.json`, and `objects/placement-parameter-head.schema.json` with its `ratification` | `epr:schema:object:placement-parameter-set`, `…:placement-parameter-head` | `ParameterSet`, `HeadDeclaration`, `RatificationRef` |
| enums | `enums/placement-kind`, `grant-scope`, `loss-domain`, `need-class`, `depth`, `isolation`, `placement-factor` (G12), `evidence-stage` (I10), `disclosure` (I5) `.schema.json`, each with `_codegen` (TypeScript only, no zome) and never `_dna`, so no constant reaches `content_store_integrity` and no DNA hash moves (G11) | `epr:schema:enum:…` | the Rust enums above |
| views | `views/placement-explanation-view.schema.json` (Category C) | `epr:schema:view:placement-explanation` (G13) | `Explanation`, serialized with `inputsDigest` as a 64-character hex string and CIDs as strings |
| probe | `objects/probe-reading.schema.json` (§I) | `epr:schema:object:probe-reading` | future: one monomorphic wire struct per served promise in `crates/elohim-views` (e.g. `MaterializationReadingView`), with `answer` as the narrowed answer envelope, to be checked by a schema_contract test when that promise is served; `seam_contracts::probe::Reading<T>` stays internal (G6, I8) |

`ParameterSet.weights` is `propertyNames: { $ref: placement-factor }` with integer values `≥ 1`. Any
member S3 or S4 adds to `NeedClass`, `Factor` or another enum lands in the Rust enum in the same change
(G12).

**The needs block.** Every need's shape is fixed now in `$defs.ManifestNeeds.$defs`, and **each field
becomes declarable only when its first reader lands** (spec §17 slice 9, §20). Until then
`needs.properties` does not reference it, and `additionalProperties: false` makes it undeclarable, so no
manifest can declare a need nothing reads. S2 therefore lands the `needs` object and the `$defs`; the
`properties` entries arrive with their readers, in the spec's slice-9 order:

| Field | `$defs` shape | First reader (lands with) |
|---|---|---|
| `closure` | `$ref: ../objects/tree-view.schema.json#/$defs/ViewPaths`, `title: "StartSet"`; refuses a `root` key | residency pin-to-bytes node |
| `runtimes` | `[{ channel: string }]` | the same |
| `worksOffline` | `enum: ["sparse", "on-read", "kept"]` (`query`, which holds no bytes, cannot mean "works offline", so it is refused; G7); absent with a declared `closure` means `kept` | the same |
| `channels` | `[string]` | head following |
| `loss` | `{ <contentType>: $ref loss-domain }`: a per-content-type demand rule, not a need (G3) | the custody-floor need source |
| `signing` | `{ present: [string], delegable: [string] }` | the signing design (spec §19 Q4) |
| `compute` | `[{ scope: $ref grant-scope, bounds: { ratePerHour: integer } }]`: a standing grant request, not a task need; `ratePerHour` maps 1:1 onto delegates-compute's `rate_per_hour` bound (G3, G10) | the task need source (§F) |
| `projectable` | `{ reach: $ref reach (required), retentionFloorDays: integer ≥ 1 }` | the projection reader |
| `indexed` | `[{ contentType, measure }]`, `measure` = the `IndexMeasure` declaration CID (dag-cbor, `^bafyrei`, `IndexMeasure::cid()`) or a `PinnedRef` `id@version`, never a placement `MethodId` (G14) | the search fold |
| `passing`, `notify` | per their own designs | the live-session and wake designs |
| `witness`, `outward` | per their own designs | their designs |

The shapes of `loss`, `signing` and `closure` assume the operator answers spec §19 Q1, Q2 and Q4 as
"the app declares" (§5, decision 22). If the operator rules that the space or charter supplies them,
those shapes move to the charter or a space policy and never enter `needs.properties`.

**Contract** (`scripts/test-manifest-needs.mjs`, new, a named follow-on; run by `pnpm run schema:test`):

- **non-regression** (G4): no existing manifest validates today (`test-schema.mjs` records the eight
  pillar manifests' pre-existing drift), so the check is that no pillar manifest gains an error whose
  `schemaPath` falls under `/properties/needs` or `$defs/ManifestNeeds`; the discriminating tokens
  (`ManifestNeeds` and the `$defs` names, not a bare `closure`, which already matches gate closure) join
  `TASK_1_TO_3_SURFACE`, and the current error count is recorded as a baseline that must not grow;
  `needs` is optional;
- **placement neutrality** (G15): a recursive check over `$defs.ManifestNeeds` and every `$def` reachable
  from it by `$ref` (not the manifest's whole root `$defs`, where `peerReviewedBy` and `graph.nodes`
  live) refuses any property name matching `/(device|host|hub|doorway|peer|node|url)/i` and any
  `format: uri` (spec §6);
- `closure` refuses a `root` key; `projectable.reach` accepts only the eight SDK reach values;
- the JSON → `Need` → DAG-CBOR → `Need::id` round trip is stable across serde passes (G1);
- each enum's values equal the Rust variants, checked in Rust by `placement/core/tests/schema_contract.rs`
  reading the enum JSON with `include_str!` (no hand-written golden list in node; G11); `grant-scope`
  also equals `GRANT_SCOPES` and the doorway and hosted-custody constants (an elohim-storage test).

**Who reads the needs** (G2, G3). placement-core cannot name `eprfs_core::TreeView` (R-P1), so the caller
computes the closure's key:

```rust
// placement/core/src/manifest.rs
/// Hand-written, not generated: schema codegen emits TypeScript only, and the Rust mirror is pinned to
/// `$defs.ManifestNeeds` by `placement/core/tests/schema_contract.rs`, which reads the schema with
/// `include_str!`. It holds only the fields placement reads, and each one lands with its reader, as the
/// schema's `properties` entry does. There is no `deny_unknown_fields`: `channels`, `signing`, `passing`
/// and the rest belong to other readers, and the schema, not this struct, refuses fields nothing declares.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestNeeds {
    pub closure: Option<Vec<String>>,                  // ViewPaths, slash-joined; "" is the root
    pub works_offline: Option<WorksOffline>,           // absent with a declared closure means Kept
    pub loss: Option<BTreeMap<String, LossDomain>>,    // content type → loss domain
    pub compute: Option<Vec<ComputeNeed>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorksOffline { Sparse, OnRead, Kept }         // `query` is refused by the schema (G7)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeNeed { pub scope: GrantScope, pub bounds: ComputeBounds }
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeBounds { pub rate_per_hour: Option<u32> }

pub struct ManifestContext {
    pub manifest: Cid,                      // the manifest's CID; every real manifest id is a slug, so the
                                            //   reader takes the CID, never the id field
    pub release_root: Cid,                  // bound by the release record (finding 8)
    pub closure_key: Option<NeedKey>,       // NeedKey::TreeView(*tree_view.cid()?.as_cid()) (§1.4), computed
                                            //   by ark-core, which links both crates and maps Depth 1:1
    pub receiver: AgentRef, pub scope: Cid, pub reach: Reach, pub class: NeedClass,
}
pub struct ManifestNeedsOut {
    pub private: Vec<PrivateNeed>,          // closure + worksOffline → a PrivateNeed at the declared depth
    pub loss_rules: BTreeMap<String, LossDomain>,   // content type → loss; the custody need source applies
                                            //   charter.resolve to each content root it holds
    pub grant_requests: Vec<GrantRequest>,  // compute → standing delegates-compute requests; a Task need
                                            //   comes only from a TaskSpec
}
pub struct GrantRequest { pub scope: GrantScope, pub rate_per_hour: Option<u32> }
pub fn needs_from_manifest(n: &ManifestNeeds, ctx: &ManifestContext) -> Result<ManifestNeedsOut, PlaceError>;
```

under a schema-contract test (`placement/core/tests/schema_contract.rs`, the pattern of
`elohim-storage/tests/schema_contract.rs`). `channels` → head following, which is not placement.

**Codegen.** The new objects join `INTERFACE_FILES` in `scripts/codegen-ts.mjs` and the enums reach every
distributed `schema-enums.ts`; `elohim/sdk/storage-client-ts` and `elohim/eprfs/eprfs-ts` join
`GENERATED_OUTPUT_DIRS` (both named follow-ons, H16). These are schema-first objects, so no ts-rs; the
probe's future per-promise wire views go in `crates/elohim-views` (finding 14), pinned to the schema by its schema_contract
test.

**Worked example.** When the residency reader lands, the lamad manifest gains
`needs: { closure: [""], worksOffline: "kept" }`: the release root is the Angular browser output
(`package-angular.mjs`'s `bundle.dist`), so the whole bundle is the root path `""`, not `dist/browser`.
The release record binds the root; residency pins the closure at depth 3. When the custody need source
lands, `loss: { "concept": "household" }` (a real lamad content type) resolves through the space's
charter to `{holders: 2, domains: 2}` for each concept root the source holds, and joining a co-op whose
charter answers `{3, 3}` changes the placement without changing the app.

### H. The TypeScript SDK surface, and the service worker as a tier

**H.1 `@elohim/eprfs`** (new, `elohim/eprfs/eprfs-ts/`, lane E; pure, rail R-T1). It depends on
`@elohim/epr` (`encodeCanonical`, `decodeCanonical`, and `verifyAddress`, which `@elohim/epr` gains
beside `verifyCid` and `verifyBlobHash`: raw `0x55` and dag-cbor `0x71` CIDv1 sha2-256; a legacy
`sha256-<hex>` or `sha256:<hex>` accepted for reads only; follow-on, with a local check pinned to the same
vectors until it lands) and `multiformats`. Byte compatibility with Rust is pinned by one vector file,
`elohim/eprfs/fixtures/tree-vectors.json` (new), read by both the Rust and the vitest suites: the frozen
tag-42 bytes, the non-UTF-8 name case of `tree.rs`, the walk fixtures, the `TreeView` normalization and
`project_view` P vectors, and one serialized `TierRecord` and one of each `ItemAbsence` and `Unanswered`
(H2).

The wire-crossing types (`Unanswered`, `ItemAbsence`, `TierRecord`, `TierSource`) are **generated** from
lane S's schemas (`objects/unanswered`, `item-absence`, `tier-record`) into `eprfs-ts/src/generated/`, never
hand-written, and the Rust side serializes camelCase fields inside tagged enums (`rename_all_fields`), so
TypeScript transforms nothing (H2). The shapes below are what that generation yields.

```ts
// eprfs-ts/src/tree.ts
export type TreeEntryKind =
  | { file: { blob: CID; executable: boolean } } | { directory: { tree: CID } }
  | { symlink: { target: CID } } | { external: { kind: string; reference: Uint8Array } };
export interface TreeEntry { name: Uint8Array; kind: TreeEntryKind }
export interface TreeNode { version: 1; entries: readonly TreeEntry[] }
export interface TreeLimits { maxNodeBytes: number; maxEntriesPerNode: number; maxDepth: number;
  maxTotalEntries: number; maxTotalBytes: number; maxPathBytes: number; maxPendingEntries: number }
/** Decodes tree.rs's canonical bytes and re-encodes them, comparing byte for byte, as decode_tree does. */
export function decodeTree(bytes: Uint8Array, limits?: Partial<TreeLimits>): TreeNode;

// eprfs-ts/src/generated/ (from the schemas) — absences are values, never throws, so an unread path is
// named and never hangs. R7: every one carries why, comeBack and meanwhile.
export type ItemAbsence =
  | { kind: 'not-held'; tried: readonly string[] }
  | { kind: 'refused'; by: string; refusal: { reason: ReachReason; needsStanding: string | null } }
  | { kind: 'over-budget'; by: string; rule: string }
  | { kind: 'unreachable'; tier: string }
  | { kind: 'membership-unknown'; by: string }
  | { kind: 'integrity'; from: string };
export type ComeBack = { when: 'at'; epochSecs: number } | { when: 'on-event'; event: string };
export type Meanwhile = { local: 'use-held'; what: string } | { local: 'run-here' }
  | { local: 'built-in-in-force' } | { local: 'nothing'; because: string };
export interface Unanswered<R> { why: R; comeBack: ComeBack; meanwhile: Meanwhile;
  escalation: { asked: number; changed: string; whoCanBeAsked: string[] } | null }

export type TierClass = 'own' | 'person-device' | 'household' | 'hub' | 'holon-hub' | 'pool' | 'doorway-peer' | 'doorway';
export type TierSource = { source: 'held-record'; record: string } | { source: 'doorway'; doorway: string };
export type TierAddress = { form: 'url'; base: string } | { form: 'multiaddr'; addr: string };
export interface TierRecord { id: string; class: TierClass; sources: TierSource[]; endpoint: TierAddress | null;
  standing: number | null;                                            // Reach::openness(); null = unknown
  proximity: { networkRank: 0 | 1 | 2; warmRoots: string[]; servesExtracted: boolean;
               rttMs: number | null; lastSeenMs: number | null } | null }

// eprfs-ts/src/walk.ts (hand-written, not generated: walk results do not cross the wire)
/** The walk's own absences beside the tier ones. Paths are byte components, as everywhere in eprfs-ts
 *  (H14), so a non-UTF-8 name survives in every variant. The mapping onto Rust is the table below. */
export type NamedAbsence =
  | { kind: 'absent'; cid: string; path: readonly Uint8Array[]; unanswered: Unanswered<ItemAbsence> }
  | { kind: 'not-found'; path: readonly Uint8Array[] }
  | { kind: 'through-leaf'; leaf: readonly Uint8Array[] }
  | { kind: 'invalid-view'; path: readonly Uint8Array[] };            // a non-UTF-8 name, never decoded lossily

// eprfs-ts/src/tier.ts
export interface BatchItem { cid: CID; path: readonly Uint8Array[] }  // byte components, as TreePath (H14)
export interface Tier {
  readonly record: TierRecord;
  fetchAt(root: CID, item: BatchItem, via: readonly string[], signal?: AbortSignal): Promise<Uint8Array | NamedAbsence>;
  fetchBlobs(root: CID, items: readonly BatchItem[], via: readonly string[],
             sink: (item: BatchItem, r: Uint8Array | NamedAbsence) => void, signal?: AbortSignal): Promise<void>;
}
export interface WritableTier extends Tier { put(cid: CID, bytes: Uint8Array): Promise<void> }
/** Rust's TierConnector, so records the page holds become tiers (H8). */
export interface TierConnector { connect(record: TierRecord): Tier | null }
export interface ChainPolicy {
  order(records: TierRecord[], root: CID): TierRecord[];   // default: proximityOrder (§A.2's comparator, H17)
  mayAsk(tier: TierRecord, root: CID, rootReach: number | null): boolean;   // unknown root reach: only own,
                                                           //   person-device and household (H7)
  admit(to: string): { release(): void } | { retryAfterMs: number; rule: string };
}
export interface ChainStatsSnapshot { perTier: Record<string, { asked: number; hit: number; integrity: number }> }
/** Seam A's contract, ported to vitest on the same vectors. Own tier first; verifies before returning or writing back. */
export class TierChain implements Tier {
  constructor(own: WritableTier, records: readonly TierRecord[], connector: TierConnector, policy: ChainPolicy);
  readonly record: TierRecord;   // { id: 'chain', ... }
  fetchAt(root: CID, item: BatchItem, via: readonly string[], signal?: AbortSignal): Promise<Uint8Array | NamedAbsence>;
  fetchBlobs(root: CID, items: readonly BatchItem[], via: readonly string[],
             sink: (item: BatchItem, r: Uint8Array | NamedAbsence) => void, signal?: AbortSignal): Promise<void>;
  stats(): ChainStatsSnapshot;                             // feeds the browser half of the probe (I11)
}

// eprfs-ts/src/walk.ts — paths are byte components, as Rust's TreePath; `pathString` is a display helper
export class View { static whole(): View; static of(paths: readonly (readonly Uint8Array[])[]): View;
  static ofStrings(paths: readonly string[]): View;
  coverage(path: readonly Uint8Array[]): 'in-view' | 'spine' | 'outside' }
export function pathString(path: readonly Uint8Array[]): string | null;   // null for a non-UTF-8 name
export type WalkItem =
  | { node: { path: readonly Uint8Array[]; cid: CID; node: TreeNode; coverage: 'in-view' | 'spine' } }
  | { boundary: { path: readonly Uint8Array[]; kind: string } }
  | { absent: { viewPath: readonly Uint8Array[]; reason: NamedAbsence } };
export interface WalkStats { nodesFetched: number; nodesReused: number; nodeBytes: number; entriesVisited: number }
/** One fetchBlobs per tree level chunk, as walk_batched; `stats` settles when the iteration ends. */
export function walkBatched(tier: Tier, root: CID, view: View, limits?: Partial<TreeLimits>)
  : { items: AsyncIterable<WalkItem>; stats: Promise<WalkStats> };
export function readPath(tier: Tier, root: CID, path: readonly Uint8Array[], limits?: Partial<TreeLimits>)
  : Promise<Uint8Array | NamedAbsence>;
export function diffTrees(tier: Tier, oldRoot: CID, newRoot: CID, view: View)
  : Promise<{ path: readonly Uint8Array[]; old?: TreeEntryKind; new?: TreeEntryKind }[]>;
export interface MaterializationReading { /* generated from the probe schema, §I */ }
```

**`NamedAbsence` and Rust's walk absences.** Walk items do not cross the wire: each language walks on its
own side over the batch route. What crosses is the batch route's absent trailer (`Unanswered<ItemAbsence>`,
generated in both languages from one schema) and the shared vectors. The two walks keep different
shapes, because Rust raises errors where TypeScript returns values (decision 11: TypeScript tiers never
throw). The walk vectors in `tree-vectors.json` are compared through this mapping, which is the only
place the two shapes meet:

| Rust (`eprfs-core`) | TypeScript (`NamedAbsence`) |
|---|---|
| `WalkItem::Absent { reason: NotHeld { missing_at, cid, tried }, unanswered: Some(u) }` | `{ kind: 'absent', cid, path: missing_at, unanswered: u }`, with `u.why.kind === 'not-held'` and `u.why.tried` equal to `tried` |
| `WalkItem::Absent { reason: Withheld { missing_at, cid, why }, unanswered: Some(u) }` | `{ kind: 'absent', cid, path: missing_at, unanswered: u }`, with `u.why` equal to `why` (any kind but `not-held`) |
| `WalkItem::Absent { reason: NotFound { missing_at }, unanswered: None }` | `{ kind: 'not-found', path: missing_at }` |
| `WalkItem::Absent { reason: ThroughLeaf { leaf }, unanswered: None }` | `{ kind: 'through-leaf', leaf }` |
| `Err(TreeError::InvalidView)`, raised where a path must take its string form (TreeView's JSON form, a query answer's JSON form) and a component is not UTF-8. A Rust walk over byte paths never raises it | `{ kind: 'invalid-view', path }`, returned where the TypeScript side needs a string: a service-worker URL path, `pathString`, the JSON forms |

The Rust `view_path` and the TypeScript `viewPath` beside each item are the same path.

**H.2 `@elohim/storage-client` additions** (follow-on).

```ts
// storage-client-ts/src/tiers/http-tier.ts
export class HttpTier implements Tier {
  constructor(record: TierRecord, opts: { signer?: AgentSigner; timeoutMs?: number });   // dials record.endpoint
  // fetchAt → a one-item POST /eprfs/v1/blobs (never /blob/{cid}; H3); fetchBlobs → POST /eprfs/v1/blobs.
  // Rewrites any remote 'own' to record.id. Verification stays in TierChain.
}
export class HttpConnector implements TierConnector { constructor(opts: { signer?: AgentSigner }); }

// storage-client-ts/src/placement.ts — the whole app-author runtime surface
/** Typed: the manifest field plus its sub-key, mapped to a NeedKey (H12). */
export type NeedRef = { field: 'closure' } | { field: 'loss'; contentType: string } | { field: 'compute'; scope: string }
  | { field: 'projectable' } | { field: 'indexed'; contentType: string };
export type NeedStatus =
  | { need: NeedRef; state: 'present' }
  | { need: NeedRef; state: 'absent'; unanswered: Unanswered<NotCarriedReason> }
  | { need: NeedRef; state: 'unreachable'; reason: AnswerReason };   // never reads as met (I11)
export type PendingStatus<T, R> =
  | { status: 'queued' } | { status: 'retrying-at'; epochSecs: number }
  | { status: 'unanswered'; unanswered: Unanswered<R> }
  | { status: 'answered'; value: T } | { status: 'withdrawn' };       // the last two are terminal
export interface Placement {
  status(): Promise<readonly NeedStatus[]>;
  /** R7's channel: reports until answered or withdrawn; a quiet need yields nothing. */
  watch(need: NeedRef): AsyncIterable<PendingStatus<NeedStatus, NotCarriedReason>>;
  why(need: NeedRef): Promise<PlacementExplanation>;
}
/** Network needs explain through the storage route; device-private needs (residency) are answered by the
 *  device's own tier (in the browser, the worker) and carry no device facts (H12). */
export type PlacementExplanation =
  | { plane: 'network'; view: PlacementExplanationView }   // generated from views/placement-explanation-view
  | { plane: 'device'; kept: boolean; deferred: boolean; unanswered: Unanswered<string> | null };
export function placementFor(appId: string, client: StorageClient, device: DevicePlacement): Placement;
export interface DevicePlacement { status(need: NeedRef): Promise<NeedStatus> }   // the worker's port
```

An app author declares needs in the manifest and can ask what state each is in and why, and watch it.
No function takes a device, host, hub or URL, so an app cannot place anything. `unreachable` (the
unmeasured state) never reads as met (spec §9). A residency need is a `PrivateNeed`, never `Serialize`
and never off the device (R6), so the remote `StorageClient` never reports it. `StorageClient.getBlob`
stays for blob-plane reads and is not a tier: it returns unverified bytes.

**H.3 The service worker as the browser's own tier** (`app/elohim-app/src/apps-sw.ts`, follow-on).

```ts
class CacheTier implements WritableTier {
  /** Its own cache name, `eprfs-objects`, outside activate's `apps-*` purge: CID-keyed bytes that verify
   *  never need a version bump (H19). */
  constructor(cache: Cache, origin: string);
  readonly record: TierRecord;                       // { id: 'own', class: 'own', ... }
  /** cache.match(`${origin}/_eprfs/${cid}`), then verifyAddress on EVERY hit: Cache Storage is shared by
   *  the origin and html5-app iframes run allow-same-origin, so a page can write a forged entry. A
   *  mismatch is integrity, the entry is deleted, and it is never served (H5). */
  fetchAt(root: CID, item: BatchItem, via: readonly string[]): Promise<Uint8Array | NamedAbsence>;
  put(cid: CID, bytes: Uint8Array): Promise<void>;   // with rememberHead and ArchiveWriter.put, the only cache.put (R-T2)
  // fetchBlobs: per item over fetchAt
}
/** The zip path's writer for bundles that are not yet trees: callable only from extractZip, after
 *  verifyAddress passed on the whole archive (H1). */
class ArchiveWriter { put(prefix: string, path: string, response: Response): Promise<void> }
```

| Today in `apps-sw.ts` | Becomes |
|---|---|
| `cache.match` by bundle hash plus path | `CacheTier`, keyed by leaf CID (`/_eprfs/<cid>`), immutable, untouched by `invalidate`, in its own cache. Depth 3 requires `navigator.storage.persist()` granted; a non-persistent grant reports the need as unreachable (not kept), never as met (H19) |
| `recallHead`, `headKey`, `_heads/{slug}` | the head plane, split (H10): `slug → { root, reach, residentView }`. The recalled-miss 404 ("not in bundle") is answered only for a path inside `residentView`; any other path goes through `readPath` and is fetched or answered `not-held` with the tiers tried. The root comes from an `X-Tree-Root` header on `_capability` beside `X-Blob-Hash` (the archive's raw hash), verified by walking it from the release record, and is recorded with its reach so `mayAsk` has it (H7, H10). Installed, head adopted and resident remain three facts (spec §10.8) |
| `getDeliveryPeers` asking the doorway's `/api/v1/peers/delivery` (cached 30 s) | **kept, by design (R6a)**: each doorway the browser ingresses through supplies `TierRecord`s with a doorway source, built by §A.6's `tier_records` rules (agent key when a binding resolves it, else the transport id; `endpoint`; `standing` from the peer's `commitments` reach tags; `warmRoots`). `DeliveryPeer` gains an `agentKey` field resolved through the cross-signed binding that `householdId` already uses (H6). One origin is one doorway, so the worker's own reading counts one doorway per origin; the other doorways come from the person's held doorway list (`.well-known/elohim-auth` per doorway), fetched cross-origin with CORS, the peer cache keyed per doorway (H18). Added beside them: records the page holds (household roster, hub commitments), sent over the existing `apps-sw` BroadcastChannel as `{ type: 'tiers', records, endpoints }`, the endpoints kept out of the records so they stay placement-neutral, and connected by `HttpConnector` (H8) |
| `scorePeerForSw` | `ChainPolicy.order`'s default, §A.2's comparator: class, per-root warmth, network rank, rtt, recency. It no longer drops cold or non-extracting peers as `tryDeliveryPeers` does (`servesExtracted && warm`); they are ordered after warm ones and before the doorway (H17) |
| `tryDeliveryPeers` ("served but not cached") | household and doorway-named `HttpTier`s. Each leaf is a raw CID named inside a verified tree node, so peer bytes verify and are cached. That closes spec §15's "per-file digests" missing node. A household `HttpTier` needs a transport a secure context may use (H11): HTTPS under a name the hub holds, a doorway relaying by key, or WebTransport. `http://<lan-ip>:<httpPort>` is mixed content from an https origin and is used only on loopback and dev origins until one of those exists |
| navigation (network-first `fetch(event.request)`) and `probeCapability` (HEAD `_capability` whenever the in-memory `deliveryCache` is empty) | for a kept app whose closure is resident, the head is read from the head plane before any probe and the navigation is served from `CacheTier` first (H9) |
| `fetchIndividual`, `fetchViaZip`, `extractZip` | kept for bundles that are not yet trees, writing through `ArchiveWriter`; a bundle tree is produced by `package-angular` (follow-on) |
| `verifyRawSha256` (`raw-cid-verify.ts`) | kept as a raw-only wrapper over `verifyAddress` (rail R-T3), with `expectedSha256Hex`; `content.service.ts`'s public-content guard and `raw-cid-verify.spec.ts`'s dag-cbor fail-closed test are unchanged (H13) |

A bundle published as an eprfs tree is served by `readPath(new TierChain(cacheTier, records, connector,
policy), bundleRoot, filePath)`; install (depth 3) is `walkBatched` over the closure's view and one
`fetchBlobs` over its leaves, streamed into `CacheTier`.

**Contract.** `decodeTree` passes the shared vectors; `verifyAddress` agrees with `BlobCid::verifies` on
raw and dag-cbor vectors; the generated wire types decode the shared `TierRecord`, `ItemAbsence` and
`Unanswered` vectors the Rust side wrote; `TierChain` passes Seam A's contract items on the same vectors;
the worker caches only bytes that verified, and an entry written by a page under `/_eprfs/<cid>` with
wrong bytes is integrity and never served; an `HttpTier` never requests a URL without the root;
offline, a kept app's index and closure open from `CacheTier` with no network round trip awaited past
one failed attempt (H9); an unread path answers a 404 naming the path, the tiers tried, when to come back
and what is local, without waiting on any network timeout (under `LocalOnly`, or offline, it settles
after only the own tier's cache lookups; a fake-timer assertion shows no timer elapses; H20); a
non-UTF-8 name yields `invalid-view` and is never decoded lossily (H14); a household peer's single file
is cached after verification and served offline afterwards, tested on an https origin so mixed content
cannot pass it (H11); a worker update that bumps `CACHE_NAME` leaves `CacheTier`'s entries in place.

**Worked example.** Offline, the person opens a kept app. `CacheTier` holds the tree nodes and the
closure's leaves, so it opens. An unread help page returns
`{ kind: 'absent', unanswered: { why: { kind: 'not-held', tried: [] }, comeBack: { when: 'on-event',
event: 'tier-reachable' }, meanwhile: { local: 'use-held', what: 'the app closure' } } }`, which the shell
shows by name. Online again, the doorway-supplied delivery peer and the household hub both appear in the
list; on a dev origin, or once the hub serves HTTPS under its name, the hub is nearer and serves the page,
which is verified and cached.

### I. The probe shape

**Owner:** the implemented generic type in `crates/seam-contracts/src/probe.rs` (alloc only, like
`residual.rs`), internal to Rust; the wire envelope `objects/probe-reading.schema.json` (lane S), whose
`answer` `$ref`s `objects/answer.schema.json` with `value` narrowed, so the envelope's three-branch
partition (Present takes no reason, Absent only `observed_absent`, Unreachable always a reason) is kept
(I1); future monomorphic wire structs per served promise in `crates/elohim-views` (I8), which this
interface pass does not implement; and each crate's own reading struct.

```rust
// crates/seam-contracts/src/probe.rs
/// The answer and its reason in one value, so Present-with-reason or Unreachable-without-one cannot be
/// written (I1). `unreachable` asserts `reason.state() == AnswerState::Unreachable`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum ProbeAnswer<T> { Present(T), Absent, Unreachable(AnswerReason) }
impl<T> ProbeAnswer<T> {
    pub fn present(v: T) -> Self; pub fn absent() -> Self;
    pub fn unreachable(reason: AnswerReason) -> Result<Self, AnswerReason>;   // refuses ObservedAbsent
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
pub struct Reading<T> {
    promise: String,          // spec §9 promise kind, e.g. "bytes-layer-reads-only-what-is-read@1"
    concern: String,          // the @concern: tag joining it to its a2o scenario and habit
    subject: String,          // a root CID, a need key, an app id
    answer: ProbeAnswer<T>,   // never Present(healthy) when nothing was read
    evidence: EvidenceStage,
    disclosure: Disclosure,
    observed_at: String,      // caller-supplied RFC 3339, as residual.rs: no new clock (I12)
}
// Reading::new requires every field. Read-only getters expose them. The type serializes but
// does not deserialize: an existing reading cannot be relabelled by mutation, struct update or
// wire reconstruction. A producer remains responsible for the provenance of a new reading.
/// The habit evidence-class-descends's two values, landed as `enums/evidence-stage.schema.json` (I10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum EvidenceStage { Fixture, Real }
/// Who may read a reading, in existing vocabulary (I5): the holder alone, the parties by role (spec §9's
/// rows: carrier and carried-for, grantor, custodians, the leaver and each holder, settlement), or anyone
/// at a reach, compared through Reach::openness() and carried here as its ordinal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize),
           serde(rename_all = "kebab-case", rename_all_fields = "camelCase", tag = "disclosure"))]
pub enum Disclosure { Holder, Parties { roles: Vec<PartyRole> }, AtReach { openness: u8 } }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum PartyRole { Carrier, CarriedFor, Grantor, Custodian, Leaver, Holder, Settlement }
/// `read` returns None when the probe has nothing to read yet: "not measured" stays outside `Reading`,
/// as LocalResolve does, so no false Unreachable reason is coined and AnswerReason (published 0.1.0, not
/// #[non_exhaustive]) needs no new variant (I2). The caller renders None as unmeasured.
pub trait Probe { type Value; fn promise(&self) -> &'static str;
                  fn read(&self, observed_at: &str) -> Option<Reading<Self::Value>>; }
/// The only writers a Holder reading may reach (I7): the `.eprfs/status` file writer, loopback-only admin
/// routes, loopback-scraped gauges (as `services/holds.rs` and its `elohim_node_holds_*` gauges are), and
/// a worker-local store in TypeScript.
pub trait HolderSink { fn write_holder(&mut self, promise: &str, bytes: &[u8]); }
/// The holder's explicit choice to let a Holder reading's aggregate numbers leave, e.g. into a habit
/// DELTA. Constructed only by a holder-facing command; it carries counts, never paths or keys.
pub struct Declassified { pub promise: String, pub counts: Vec<(String, u64)>, pub chosen_by: String }
```

```rust
// eprfs-core/src/probe.rs (lane E) — pure data. Disclosure: Holder.
/// Slice 1 builds it from WalkStats and the counting-wrapper witness alone; the per-tier map arrives in
/// slice 2 with ChainStats (I4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterializationReading {
    pub root: BlobCid, pub view: Vec<String>,
    pub nodes_fetched: usize, pub nodes_reused: usize, pub node_bytes: usize, pub entries_visited: usize,
    pub leaves_fetched: usize, pub leaf_bytes: u64, pub absent: usize, pub boundary_count: usize,
    pub witness_nodes_fetched: Option<usize>,      // the independent CountingTier witness (treeNodesInStore)
    pub baseline_nodes_fetched: Option<usize>,     // the --baseline load_verified_tree count (B0)
    pub tiers: Option<BTreeMap<String, (u64, u64)>>,   // slice 2: tier → (asked, hit)
}
pub struct WitnessCounts { pub nodes_fetched: usize, pub tree_nodes_in_store: usize }
impl MaterializationReading {
    pub fn from_walk(walked: &WalkedView, view: &View, leaves: (usize, u64), witness: Option<&WitnessCounts>,
                     baseline: Option<usize>) -> Self;
    /// Slice 2.
    pub fn with_tiers(self, tiers: &ChainStatsSnapshot) -> Self;
}

// placement-core/src/probe.rs — Disclosure: AtReach(the space's reach) above public; for a below-public
// need, Parties within the household (R2). Comparable values are digests, never carrier names (I6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgreementReading { pub need: CidString, pub inputs_digest: String /* hex */, pub chosen_set: CidString,
                              pub honored: u16, pub wanted: u16 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamsAgreementReading { pub scope: CidString, pub head: CidString, pub in_force: BTreeMap<KindId, CidString> }
```

`chosen_set` is `atom_cid` of the sorted chosen agent keys; carrier names stay with `explain()`, at the
reach each carrier declared (spec §7.4).

| Promise | Value | Built from |
|---|---|---|
| The bytes layer reads only what is read (`materialize-what-is-read`) | `MaterializationReading` | `WalkStats`, a `CountingTier` witness, the B0 baseline; `ChainStats` from slice 2; the TypeScript `walkBatched` stats and `TierChain.stats()` for the browser half |
| Placement converges with no coordinator | `AgreementReading`, compared across two peers | the explanation inputs digest and the chosen-set CID |
| One parameter set in force per space | `ParamsAgreementReading` | `select_in_force` over `ParameterHead`'s declarations |
| Reach on every path | refused-by-name counts per `ReachReasonLabel` | `TierEndpoint` counters |
| Compute carried | a completion event per commitment per period | the `rakia-compute-complete-v1` events and `FulfilmentDraft` records |
| Plural doorways | distinct doorways supplying entries | `TierRecord.sources` (one per origin in a browser; the rest from the held doorway list) |

**Contract.** A probe with nothing to read returns `None`, rendered unmeasured, never
`Present(healthy)` (spec §9, CONVENTIONS rule 11), and an `Unreachable` answer always carries a real
transport reason; a `Holder` reading reaches only a `HolderSink` writer, asserted by a test that no
`Holder` reading is passed to any other writer, and its numbers leave only as a `Declassified`; `evidence`
is required, and a `Fixture` reading cannot construct the `RealEvidence` a settling `FulfilmentDraft`
needs (§D.6: a `compile_fail` doctest that no struct literal builds one, and a unit test that
`from_readings` refuses a Fixture reading); every refusal a probe counts is a `ReasonLabel`
(`check_reason_labels`), through eprfs-serve's local label enums for eprfs's reasons (§A.2).

**Worked example.** The slice-1 step (b) run over the brit-sealed monorepo prints the plain JSON of
`MaterializationReading`: `{ root, view, nodesFetched, nodesReused, nodeBytes, entriesVisited,
leavesFetched, leafBytes, absent, boundaryCount: <the number of gitlink boundaries>,
witnessNodesFetched, baselineNodesFetched }`. The `Reading<_>` envelope wraps it only once the
seam-contracts follow-on lands; until then the holder declassifies the counts into the habit's checks and
DELTA by hand, and the habit compares them with the B0 baseline. After slice 1 the habit is red, not
green, because no remote tier exists yet.

---

## 3. Reuse map

### 3.1 Existing code that becomes an instance of a seam

| Existing | File | Becomes |
|---|---|---|
| `EprfsStorage` | eprfs-core `storage.rs` | the one tier trait (A), with `tier_id`, `fetch_at`, `fetch_blobs` and the three depth-0 query methods added, all defaulted |
| `MemoryStorage`, `DirectoryStorage` | eprfs-storage `lib.rs` | the own tier; the shared test double |
| `StorageEndpoint { base_url }` | eprfs-storage `lib.rs` | `HttpTier` (the empty placeholder retires) |
| `fetch_checked`, `fetch_node`, `decode_tree`, `valid_name`, `Budget` | eprfs-core `tree.rs` | the one verification and budget path for walk, `LevelWalk`, chain, endpoint, hydrator and `project_view` |
| `load_verified_tree`, `restore_exact_tree` | eprfs-core, eprfs-local | kept for exact restore; can later be `walk_collect(View::whole())` plus `fetch_blobs` |
| `TreeLimits`, `ProjectionPath`, `BlobLink` two-form serde | eprfs-core | the budget everywhere; `View::new`'s input; the pattern of `TreeView` |
| `DerivationEdge`, `CompositionGraph` | eprfs-core `composition.rs` | the projection of `DerivationRecord` (new unit variant `Derivation`) |
| `MaterializationPolicy`, `BytePresence` | `projection.rs`, `awareness.rs` | `(Depth, FetchPolicy)` as input by §B.4's mapping (old `Sparse` = `LocalOnly` plus sparse markers, not `Depth::Sparse`), deprecated for one release; `BytePresence` kept as the per-path projection |
| brit `BuildAttestationContentNode`, gate verdicts in git notes | brit-epr | the first `DerivationRecord` kinds; `GitNotesIndex` |
| recall `store_dir(root, measure_cid, embedder)` | epr-cli `flow/memory/recall/index.rs` | `DerivationKey` (descriptor with measure and embedder, projected input root); its incremental re-fold becomes `Derive`'s `prior` |
| `seal_git_tree`, `SealedGitTree` | brit-bridge | the producer of roots the walk reads |
| `PlacementStrategy`, `XorDistanceStrategy`, `DiversityAwarePlacementStrategy`, `hash_into_keyspace` | `reconcile/placement.rs` | `Ranking`, `XorDistance`, `DiversityAware`, `NeedKey::rank_bytes`, moved with golden tests; storage keeps a shim |
| `PlacementCandidate` | same | `Candidate` (posture plus observer-resolved facts) |
| `salvage_pass`, `SalvageConfig`, `SalvageOutcome`, `CommitmentAuthor` | `reconcile/custody.rs` | the custody kinds' driver over `decide`; `Ports::author`; `SalvageConfig.enabled` → the custody posture |
| `LocalBlobStore`, `BlobStoreSnapshot`, `FetchKicker`, `reconcile_pass` provider branch | `reconcile/custody.rs` | snapshot holdings; the byte mover, then a `TierChain` fetch |
| `SalvageCommitmentAuthor`, `run_salvage_pass`, `resolve_self_agent_cid` | `services/salvage_commitment_author.rs` | the notarized `Ports::author` (its `block_in_place` bridge kept); the driver; `Snapshot.me` |
| `build_salvage_candidates` household join | same | the observer's `Candidate` resolution, without the self-append |
| `deterministic_custody_id` | `services/rea_commitment_service.rs` | `commitment_id(CustodyBlob, …)`, byte-identical |
| `salvage_capacity` rows, `capacity_pledge_author` | elohim-storage | the custody `Posture` source |
| `PeerSelection`, `SelectionOutcome::Short { gap_kind }` | `services/peer_selection.rs` | the ingest phase of `CustodyRoot`; `gap_kind` → `NotCarriedReason` |
| `custody_rotation` successor pledge | elohim-storage | `ReleaseReason::Superseded` |
| `blob_serve_verdict`, `private_serve_verdict` | `blob_reach.rs`, `private_reach.rs` | `RootReachGate`, generalized to roots and composed |
| `race_fetch` | `p2p/blob_fetch.rs` | **not** a tier: it stays the custody byte mover's racing fetch; `PeerTransportTier` speaks `/elohim/eprfs-blobs/1` to one peer's `TierEndpoint` (A6) |
| `GRANT_SCOPES`, `grant_scope`, `required_scope`, compute-task and compute-receipt schemas | `api/compute_grants.rs`, `api/compute_tasks.rs` | `GrantScope` (one shared list of compute-grant scopes); `required_scope` moves into placement-core as the one-way method → scope mapping; the task kind's consent and wire; the receipt rides on the completion event |
| requester-side `compute-fulfilled` (`rea_observed_compute.rs`) | elohim-storage | stays the requester's view of the carrier's provider-authored `rakia-compute-complete-v1` event |
| `compute_task.rs` accept / complete / decline events | `content_store` coordinator zome | the task kind's commitment (the first accept, provider-less request, first-acceptance-wins) and its settlement; a coordinator change, no DNA hash move (F10, F11) |
| `acquisition_pins` (`closure_rule_json`, `head_ref`) | elohim-storage | the device's kept set (depth 3); the source of `PrivateNeed` |
| `Governor`, `Refusal`, `LimitOwner` | `elohim-compute/src/actuation.rs` | eligibility's elevate face and refusal vocabulary |
| `Answer`, `AnswerReason`, `ReasonLabel`, `ArbitrationKey`, `select_arbitrated_winner`, `check_arbitration`, `check_quiescence` | `crates/seam-contracts` | probe answers and transport reasons, labels (eprfs's through eprfs-serve's local label enums), the rank key, head choice over verified declarations, the ranking and idempotence contracts; `Unanswered` and `PendingStatus` (§2.0) are added beside them |
| `Reach::openness`, `raw_cid_from_sha256_hex`, `compute_cid` | `elohim-epr` | reach comparison (no fourth vocabulary); the sha256 alias; need, set and task CIDs |
| `AgentRef`, `atom_cid` | `elohim-epr-rea` | identity; explanation, need and set CIDs |
| ark `Berth`, `Passport`/`EffectiveTier`, `RuntimeEnvelope`, `ResourceQuota`, `ChildSpec`, `CandidateContent.berth`, `rea.rs` | ark-core | `BerthOffer::from_berth`, `Isolation`, `TaskSpec` fields, `Capacity`, the berth identity, the REA intent home |
| `apps-sw.ts` `recallHead`, `getDeliveryPeers`, `scorePeerForSw`, the BroadcastChannel | app | the head plane; doorway-supplied tier records; the default order; the tier-records message |
| `@elohim/epr` cbor and cid; `verifyRawSha256` | epr-ts; app utils | the codec behind `@elohim/eprfs`; a raw-only wrapper over `verifyAddress` |
| `DeliveryPeer` | `p2p/mod.rs` (ts-rs) | the doorway source of `TierRecord`s, gaining `agentKey` |
| `cid_serde` | `epr-rea/src/index.rs` | the CID encoding of every placement and schema-mirrored CID field (G1) |

Not instances, by name: epr-cli `flow/placement.rs` (the *document* placement ledger) and ark-core
`Intent`/`IntentAction` (a write-ahead process record).

### 3.2 Parallel mechanisms each seam retires

| Retired | File | Replaced by | Retire when |
|---|---|---|---|
| The ingest selector's own household-first greedy | `services/peer_selection.rs` | `rank` + `diversify` + `NotCarriedReason` | ingest expresses needs to the floor |
| `select_diverse`'s household pass, for choosing carriers | `elohim-peer-fabric/src/score.rs` | `diversify` (placement); `TierOrder` (where to fetch from) | serve routing feeds a `TierOrder` |
| The `serve_routing` load → fold → select adapter | `services/serve_routing.rs` | `tier_records` (which admits only peers a held record or doorway names for the content) plus `ProximityOrder: TierOrder` (which orders by per-root warmth); the holder filter is the record builder's, not the order's | the read path goes through `TierChain` |
| The salvage ticker (`salvage_recheck_seconds`) and `custody_sweep_seconds` as placement triggers | `reconcile/custody.rs` | `Trigger`; `CarrierLost` at the charter's freshness horizon for loss; `PeriodClosed` for settlement only | the floor driver is wired |
| The salvage self-append | `build_salvage_candidates` | guarded self-candidacy (§D.3) | the custody `Ports` lands |
| The per-provider, per-blob honored count | `salvage_pass` | one grouped inventory query per pass | same |
| `SALVAGE_TARGET_REPLICAS` | config | `Charter::resolve(loss)` | the first charter-backed need |
| The `diversity_placement` knob in `select_placement_strategy` (`SALVAGE_DIVERSITY_PLACEMENT`) | elohim-storage | `Charter`'s `Demand.domains` (diversity is the charter's, not a parameter) and one fixed built-in method for legacy rows | retired before the floor reads legacy rows; a peer whose knob differs is observe-only (E12) |
| Per-blob placement granularity | custody rows | `NeedKey::TreeView` (R1) | the custody migration; `NeedKey::Blob` exists until then |
| Uncached peer bytes in the worker | `apps-sw.ts` `tryDeliveryPeers` | verified `HttpTier` reads through `TierChain` | the worker on `TierChain` |
| `verifyRawSha256` as its own verifier | `raw-cid-verify.ts` | a raw-only wrapper over `verifyAddress` (its fail-closed behaviour on dag-cbor kept) | the worker imports `@elohim/epr` |
| `MaterializationPolicy` as the materializer's input | `projection.rs` | `(Depth, FetchPolicy)`, by §B.4's mapping | eprfs-local materializes by view, after one deprecated release |
| `LocalMaterializer::fetch_verified_blob`'s second verify | eprfs-local `lib.rs` | `fetch_blobs`' in-place verify | same |
| Inline `TestStorage` doubles, brit-bridge `MemoryStorage`, eprfs-local `FakeStorage` | `tree.rs`, brit-bridge tests, eprfs-local tests | eprfs-storage `testing`: `MemoryStorage`, `CountingTier` | slice 1 moves `tree.rs`'s; the others on their next edit |
| The doorway's locality-ladder reach function | `doorway-service/src/cache/access_control.rs:44` | `Reach::openness` on the SDK enum | its owner `reach-enforced-everywhere`, before slice 2 |

The doorway's delivery-peer list is **not** retired (R6a). It becomes one of two sources of tier records.

---

## 4. Performance the shape guarantees

**Bounded memory.**

- A walk holds its frontier (≤ `max_pending_entries`) and fetched node bytes charged to
  `max_total_bytes`; its node cache is inside that charge. `LevelWalk` holds one level chunk of at most
  `max_pending_entries` at a time (B13).
- `fetch_blobs` streams into a sink; no batch is buffered whole. The batch route writes a CBOR sequence
  item by item.
- The chain's peak is `max_in_flight × max_blob_bytes`. `DirectoryStorage` defaults to 64 MiB per blob,
  so a phone's gate is sized at 2 in flight of 16 MiB; large blobs ride the chunked BlobStore path.
- Service-worker install streams leaves into Cache Storage, where `extractZip` holds the archive plus
  every expanded entry (about 2–3× the bundle).
- `decide` is O(needs × k log k) over k eligible candidates per need, a whole pass whenever `affected`
  does not skip it, into a reused `Snapshot` and `Scratch`. Explanations are recomputed, so they cost nothing at
  rest.

**Zero-copy bytes.**

- `Bytes` is reference-counted from the tier through `verifies` (which borrows) to the sink and the
  write-back; the write-back clone is O(1). `HttpTier` splits frames off a `BytesMut`; this is why it
  depends on reqwest directly and not on `elohim-storage-client`'s `Vec<u8>`.
- Rankings return indices into `eligible`; today's `rank` allocates a `String` per chosen candidate.
- In TypeScript the `Uint8Array` goes into `new Response(bytes)` without a copy.
- What remains: `decode_tree` builds an owned `TreeNode` and re-encodes for the canonical check. At
  spine scale this is nothing; a borrowed `TreeNodeRef<'a>` is the later, non-breaking optimization and
  must keep the canonical check.

**CID short-circuits.**

- `diff_trees` skips equal subtree CIDs, and a walk reuses a node reached twice.
- The chain answers from the own store before any network, and never via `has_blob` (finding 6).
- `project_view` keys a derivation so commits outside the view reuse it; a task looks up before it
  places.
- An app upgrade fetches only the changed spine and leaves.
- An unchanged parameter-set CID with an unchanged inputs digest means no re-decide.

**Fewest round trips.**

- A level-batched walk costs depth + 1 round trips, then one `fetch_blobs` for the leaves.
- One reach check per root per batch, never per blob (R1); the membership check shares the batch's
  spine nodes.
- `DerivationBundle` is one fetch per (adopted head, method); `lookup_many` asks for K methods on one
  input at once.
- The depth-0 query methods answer in one round trip where a tier overrides them.
- The floor's snapshot is one grouped query per table, retiring salvage's N+1 count.

**Hashing once.** Today `XorDistanceStrategy::rank` hashes every candidate's agent key for every blob
(1,000 blobs × 30 candidates = 30,000 sha256 calls per pass); `Scratch` hashes each once per snapshot.
The own hop verifies twice (`DirectoryStorage::read_object`, then `fetch_checked`): kept for now and
measured in E5; if it matters, an adapter hint the core honours only for the own store, added only when
a measurement asks.

**Blocking I/O.** `DirectoryStorage` calls blocking `std::fs` inside `async fn`. A `TierChain` on a
multi-thread runtime and every serving adapter wrap own-store reads in `tokio::task::spawn_blocking`
(eprfs-storage adds tokio's `rt` feature; eprfs-core never does).

**Where caches sit.**

| Cache | Where | Key | Invalidation |
|---|---|---|---|
| Verified objects | each party's own store (`DirectoryStorage`, `BlobStoreTier`, `CacheTier`) | CID | never; residency lets go |
| Forwarded misses | the household hub's own store | CID | never; it absorbs repeats, one miss per object per household upstream |
| Decoded tree nodes | `TierEndpoint`'s bounded LRU | CID | never needed (immutable) |
| Reach verdicts | per request; optionally an LRU per (requester, root) in the tier | (agent, root) | a membership or reach-change event, never across one |
| Ranking keys | `Scratch` | agent | rebuilt per snapshot |
| Derivations | `DerivationIndex` | bundle CID, reached through the adopted head | never (an immutable bundle once named); the head is not cached; trust chosen at read |
| Heads (name → CID) | not cached | — | only `recallHead`'s named offline fallback |
| Negative answers | not cached past one request | — | — |

**Idle is free.** The floor runs on triggers. Leases are not renewed on a timer. Two clock-born
triggers remain, both bounded: `PeriodClosed`, one per period per peer, which settles; and
`CarrierLost`, at most one per carrier per freshness horizon, because a carrier that goes silent produces
no event of its own (D4). A household at rest with every posture declared and every carrier live makes no
placement calls, which `idle-is-free` checks. A quiet R7 status cursor returns `None` and costs nothing.

---

## 5. Resolutions of the judges' conflicts, and decisions for the operator

Each line is the resolution this document adopts. Those marked **operator** need a ruling before the
seat that builds them lands.

1. **Doorways (R6a).** A doorway is a legitimate, plural source of tier records (`TierSource::Doorway`),
   the worker keeps asking each doorway it ingresses through for delivery peers, and records the
   requester holds are added beside them. No proposal's retirement of `/api/v1/peers/delivery` survives.
2. **`fetch_blobs`.** `fetch_blobs(root, items: &[BatchItem { cid, path }], opts: &BatchOptions { policy,
   via }, sink)`: the root first (R6), per-item paths so a serving tier can check membership from its own
   nodes (spec §12.3), and `via` for strict forwarding. The rootless form is dropped; a pathless
   `fetch_blob` on a chain is own-only.
3. **Tier chain home and budget.** `TierChain` in eprfs-core with an injected synchronous `ForwardGate`;
   `SemaphoreGate` in eprfs-storage; over budget names its rule; a shared tier's budget comes from the
   charter, per requester (`TierChain::forward_blobs`). **Ruled by the operator, 2026-10-20:** elohim-storage
   may depend on eprfs-core and eprfs-serve to serve trees by CID and check membership; it never
   interprets a working tree (finding 15).
4. **Reach inside eprfs.** Opaque: `ReachGate` (serving) and `AskPolicy` (requesting) are traits whose
   implementations live in hosts; eprfs-core never names a reach value (it carries `ReachLevel`, an
   ordinal) and keeps an exact frozen dependency set, which now admits `elohim-seam-contracts` for R7's
   shared type (revised by the seam check: ruling 7 requires one type every seam names, and the
   zero-dependency leaf is the purest home). Out-of-reach answers are the same refusal for missing and
   private.
5. **Kinds.** A closed `KindId` with a `PlacementKind` registry and one heterogeneous `decide` pass, so
   capacity is charged across kinds in the charter's serve order.
6. **Hysteresis and revert.** Incumbency is scoped to the parameter set in force; offer churn under one
   set moves nothing, a newly declared ratified set re-places make-before-break with at most
   `max_moves_per_event` moves per event, and a revert is a new declaration of the earlier set's CID.
   **Ruled by the operator, 2026-10-20:** agreed as the reconciliation of spec §7.2's hysteresis with
   §7.3's revert story, with `max_moves_per_event` declared in the parameter set inside the charter's range.
7. **Three authorities.** Serve order, loss → holders and domains (household diversity), holding floor,
   the forwarding rule, the tuning rule and the move range are the charter's; method, weights and
   `max_moves_per_event` (inside the charter's range) are the parameter set's; the offer is the
   carrier's.
8. **The built-in set.** `RatificationKind::BuiltIn` names the floor's content-addressed default, one
   per kind, scope-free so it has one CID everywhere, which nobody put in force; a charter may name its
   own genesis set per kind (`Charter`); every tuning is `Ratified` under the charter's tuning rule, a
   household's being each touched member's consent.
   **Ruled by the operator, 2026-10-20:** R5's "every parameter set in force is ratified" governs the
   sets a space puts in force; the built-in floor default is content-addressed code, exempt, so a
   household with no charter still has placement.
9. **placement-core's dependencies.** R-P1's exact set, following ark-core's precedent. elohim-compute is
   taken with `default-features = false` after a follow-on gates its runtime modules behind a
   default-on `runtime` feature (finding 13). placement-core does not depend on eprfs-core; it uses
   `cid::Cid`.
10. **Crate path:** `elohim/placement/core`, a member of `elohim/Cargo.toml`, like `ark/core`.
11. **TypeScript homes.** `@elohim/eprfs` at `elohim/eprfs/eprfs-ts` (pure; inside lane E's write-set);
    the one verifier `verifyAddress` in `@elohim/epr`; `HttpTier` and the placement surface in
    `@elohim/storage-client`; shared vectors at `elohim/eprfs/fixtures/tree-vectors.json`. Tiers return
    named absences, never throw.
12. **Slice-1 names.** `Coverage` for `Placement`, its fields renamed with it; `AbsentReason::NotHeld`
    gains `tried`; `Withheld` as slice 2's variant for absence not established; public constructors for
    absences and walk items. Decided; **lane E** applies them before E1 lands, and a change is a §6
    revision with its reason.
13. **Server-side walk.** Client-side level walking by default (`LevelWalk`); the depth-0 query methods on
    `EprfsStorage` for one-round-trip answers where a tier overrides them, with the walk functions relaxed
    to `?Sized` so they work through `dyn`.
14. **Self-candidacy.** Only through the peer's own posture as received from the shared source; no
    pending-self offer.
15. **Schemas.** `objects/tree-view`; the needs block's shapes fixed in `$defs`, each field declarable
    only with its reader. **Operator:** plan item S2 reads "the manifest needs block of spec §6", while
    spec §17 slice 9 lands each field with its reader; this document follows the spec, so S2 ships the
    object and its definitions with no declarable field until a reader lands.
16. **Probe.** `seam_contracts::probe::Reading<T>` with a `ProbeAnswer` that pairs the reason with its
    state; `evidence-stage` (`fixture | real`) as a new enum, the habit's first move; `Disclosure` (holder,
    parties by role, at a reach) in existing vocabulary; unmeasured is a probe returning `None`, outside
    `Reading`, not an `Unreachable` with an invented reason.
17. **Compile defects in the winner, fixed.** `MishpatAction::DelegatesCompute { scope: GrantScope }`
    (a closed enum, not `&'static str` under `Deserialize`); `NeedKey` and other `Ord` derives avoid
    `AgentRef` and `Reach`; `BlobCid` and `FetchPolicy` gain the derives they need; `TaskSpec::cid` and
    `need_key` return `Result`, as `RuntimeManifest::cid` does.
18. **Leases.** No renewal timer and no `Renew` action (spec §7.2); `PeriodClosed` only settles, and
    `CarrierLost` at the freshness horizon is the one other clock-born trigger.
19. **Explanations.** Recomputed from (method, parameter set, inputs digest), which every commitment
    carries; other members' refusals counted, never listed (spec §7.4).
20. **Versioning.** `0.2.0` at the first registry publish carrying `walk`: `TreeError` and
    `AbsentReason` are already marked in the working tree; `EprfsError` and `DerivationKind` are marked in
    the same release, and `MaterializationPolicy` is deprecated there.
21. **Write-set.** One statement, at the top of this document: this item writes the document and its
    report; each piece of code is assigned to lane E, lane S or a named follow-on, and the follow-ons the
    lanes need outside their paths are listed there.
22. **Manifest-declared shapes.** **Ruled by the operator, 2026-10-20:** the app declares `closure`, `loss` and `signing` as needs and the
   space resolves each; rakia and brit may mirror `closure` into the release manifest. (Original framing:
   the `$defs` for `loss` and `signing` assumed spec §19 Q2 and Q4 answered "the app declares", and
   `closure` in the app manifest answered Q1 the same way; the ruling confirms all three.)
23. **Ruling 7.** One generic `Unanswered<R>` (why, come back, meanwhile, escalation) and one
    `PendingStatus` channel with `Answered` and `Withdrawn` as terminal states, in seam-contracts, used by
    every seam with its own `ReasonLabel` reason.
24. **The task's commitment.** The compute request becomes provider-less and the first accepting berth's
    accept event is the commitment (first-acceptance-wins), a coordinator change that moves no DNA hash,
    so a berth self-adopts (R3) and a task still runs once. **Operator:** confirm the coordinator change
    to `content_store/src/compute_task.rs`.
25. **Cross-household below public.** Until the household-records transport lets a carrier write into
    the receiver's household plane, a cross-household carrier of a below-public need is ineligible
    (`CrossHouseholdBelowPublic`), not silently unreadable.

## 6. Not decided here, and revisions

- The numbers: period length, forwarding budgets, retention floors, `max_moves_per_event` defaults
  (spec §20).
- The batch route's error framing beyond a missing trailer: the arrived items, the trailer and the
  missing-trailer rule are fixed in §A.6, while a typed error item for a tier that fails partway
  through is not.
- The relay, wake and live-session kinds' `fits` rules, which belong to their own designs.
- The `NeedClass` and `Factor` vocabularies beyond the members named here, settled by S3 and S4, each
  new member landing in the Rust enum in the same change.
- Prefetch policy for depth 2.
- `WeightedCapacity`'s default weights, and the freshness horizon's default.
- **Tie-break salts** (design spec §7.3 names "ranking weights and tie-break salts"): dropped. The XOR
  tie-break over the agent key is kept byte for byte so the golden vectors and today's placements hold;
  a salt would re-place every legacy commitment. A salt can be added later as a new `Factor`-free
  parameter under a new method id (E18).
- **Device-scope tuning** is carried by the non-`Serialize` `DeviceParams` on `PrivateSnapshot`; how a
  person ratifies it on their own device is the drafting surface's design (E16).
- The cross-household household-records transport (decision 25).

**Revisions.** A seat that must change a signature records it here, with its reason, before it lands.
The adversarial seam check (item I2, 2026-10-20) revised the document before any seat built against it;
each id below is a finding in `task-placement-seam-check-report.md`, which records the evidence and the
alternative chosen where the finding offered two.

- §2.0, R-E1, decision 4, 23 — R7 (B1, C13, D6, E10, H4): `Unanswered<R>`, `ComeBack`, `Meanwhile`,
  `Escalation`, `PendingStatus`, `StatusCursor` in seam-contracts; eprfs-core admits seam-contracts.
- §A.1 — A8: `fetch_at` takes `&BatchOptions`, and the chain's `fetch_at` is a one-item batch; A12:
  `FetchPolicy: Copy + Default`; A13: each item answered once, bytes fanned out per CID; A17: the full
  `from_error` mapping; A18: `RootRequired`, chain `has_blob` → `Unknown`; B5/C8: the depth-0 query
  methods on `EprfsStorage`, walk functions `?Sized`, `TreeQuery` removed; slice-1 free functions take
  `&BatchOptions` (`From<FetchPolicy>`).
- §A.2 — A2: `TierId::own` never serializes; `tier_id()`; A11/H6: `TierId::transport`,
  `TierRecord.endpoint`, stats never attribution; A14: `sources: Vec`; A5/H7: `standing`; A15/H17:
  `Proximity` without `Ord`, explicit comparator, `order(root, …)`, per-root warmth; A16/H2:
  `rename_all_fields = "camelCase"` on tagged enums; A3: `MembershipUnknown`; A10/I3: `ReasonLabel` faces
  as local enums in eprfs-serve; A4: `ForwardGate::admit(on_behalf, …)`.
- §A.3 — A5: `AskPolicy::may_ask(tier, root, root_reach)`, `ReachLevel`.
- §A.4 — A4: `TierChain::forward_blobs(on_behalf, …)`, `TierChain::own`.
- §A.5 — A2/A4: `TierEndpoint { self_id, store: Arc<dyn EprfsStorage>, … }` over the chain's own store;
  A3: membership resolves the spine through upstream; B6: `serve_diff` admits both roots.
- §A.6 — A1/H3: `HttpTier` never uses `/blob/{cid}`; the third precondition; A6: `PeerTransportTier` per
  peer over `/elohim/eprfs-blobs/1`, not `race_fetch`; A7: authenticators per transport, `HttpTier` takes
  a signer; A5: the ask inequality corrected (contract 4b); A19/R-E3: harness and `CountingTier` in
  eprfs-serve `testing`, elohim-storage's own `ForwardGate`.
- §B — B2/A9: `TreeError::Unanswered`, `AbsentReason::NotHeld.tried`, `Withheld`; B3: un-offered →
  `Withheld { Unreachable }`; B4: `LevelWalk::absent` returns `TreeResult`; B6: `diff_side` carries its
  root; B7: `TreeView` decode refuses non-canonical paths, `""` root; B8: `Hydrator` reads take `&self`;
  B9: `MaterializationPolicy` mapping, deprecated a release; B10: `OverlayChange::Put { kind:
  TreeEntryKind }`, `remove` semantics; B11: the field renames; B12: finding 2 corrected, public
  constructors; B13: level chunks of `max_pending_entries`; B14: `Depth` without `Ord`,
  `satisfies_offline`.
- §C — C1: `lookup(who, at, method)` judged on the input root; C2: outputs inherit the input root's
  reach, fetched as `BatchItem::derived`; C3: `Agreed` removed, `Witness`, reader-computed agreement with
  a `DomainRoster`; C4/C12/C15: `MethodDescriptor` (params, exec, step, embedder, declared view,
  non-path inputs, deterministic); C5: `walk_spine`; C6: P construction rules and vectors; C7: bundles per
  (adopted head, method), `lookup_many`, `to_attestation`; C9: `prior`; C10: `Recomputed { checked,
  against, matched }`; C11: no `deny_unknown_fields` on records, `#[serde(other)] Unknown`, lenient
  bundle decode; C14: `ProjectedRoot`, `KeyInput`; C16: brit `success` and `signature`; C17: `Derive` in
  R-E2, `DerivationKind` in the 0.2.0 batch.
- §D — D1: fill and drain count domains, contract 5's carve-out; D2/F5: per-receiver `admitted` and
  `consent`, `Consent::Granted { recipient, valid_from, epr_scope }`; D3: `Candidate.live`; D4:
  `CarrierLost` clock-born at the horizon; D5: pending-own guard, `me_aliases`, contract 13 restated;
  D7: `MethodRegistry`; D8/F4: `RankInput`, `NeedTerms`; D9: one `Floor` across scopes; D10/E12: the legacy
  mapping, `Need::id`-keyed explain and `Carrying`; D11: deferred own-household offers; D12: domain
  tie-break, snapshot unique by carrier; D13: `RecordPlane::HouseholdLocal { household }`,
  `CrossHouseholdBelowPublic`; D14: derives; D15: multi-pass `diversify`; D16: `affected` as a skip test;
  D17: `admit` in `Governor` order; D18: `Release.provider`; D19: R-P8; D20/F16: `PrivateSnapshot.reach`,
  `Requirements.pids`; F6: `OfferTerms.scopes`/`isolation`, `Requirements.isolation`; F14:
  `Requirements.carriers`; G1: CID fields via `cid_serde`, `Expresser` as a struct.
- §E — E1: scope off the set; E2: `Ranking::factors`, weighted built-in for `CustodyRoot`; E3/E13:
  `ParameterHead::declarations`, pure `select_in_force`, `ceiling_contract.rs`; E4: single tier, clock =
  `ratified_at`; E5: predecessor on the head, `ranking_cid` for incumbency, revert exempt from the band;
  E6: `diversify` removed, weights > 0, move range from the charter; E7: `genesis_params` per kind; E8:
  `tuning_rule`, `touches`; E9: core verifies `BuiltIn` and `Charter`; E11: `Draft.reach`, private
  `Pressure`; E14: one head per space naming a set per kind; E15: params keyed by (scope, kind); E17:
  fallible CIDs, stored in `InForce`.
- §F — F1: no requester scope; F2: the method binds `exec`; F3: SUT view, non-deterministic measure; F7:
  `from_berth` takes the manifest; F8: per-hold reach; F9: `base_root`; F10: provider-less request,
  first-acceptance-wins; F11: settle on the completion event, `ExitClass`; F12: `TaskChild`; F13:
  `{input}`/`{output}`; F15: withdraw after outcome.
- §G — G2/G3: `ManifestContext`, `ManifestNeedsOut`; G4: non-regression; G5: follow-ons named; G6/I8:
  monomorphic wire views; G7: `worksOffline` enum; G8: the worked example; G9: the S1 path pattern; G10:
  grant-scope as a subset; G11: `_codegen`, Rust-side equality; G12: `placement-factor`; G13: the view
  `$id`; G14: `indexed.measure`; G15: the neutrality scan's scope; G16: decision 22.
- §H — H1: `ArchiveWriter`; H5: `CacheTier` verifies every hit; H8: TS `TierConnector`; H9: the offline
  contract; H10: the head plane split, `X-Tree-Root`; H11: secure-context transport; H12: typed
  `NeedRef`, device-plane explanations; H13: the raw-only wrapper; H14: byte paths; H15/H16: gates and
  codegen follow-ons; H18: one doorway per origin; H19: `CacheTier`'s own cache, persistence; H20: no
  network timeout awaited.
- §I — I1: `ProbeAnswer`; I2: `Probe::read` returns `Option`; I4: slice-1 reading fields; I5:
  `Disclosure`; I6: digest-only `AgreementReading`; I7: `HolderSink`, `Declassified`; I9: `RealEvidence`;
  I10: `evidence-stage`; I11: TS stats and answer-state `NeedStatus`; I12: `observed_at`.
- **Cold read** (2026-10-20, a reader new to the document; recorded in the seam-check report under
  "Cold read"). These additions define what the signatures above already used. §2.0: `Answer<T>` shown as
  seam-contracts defines it, with how placement reads each state. §1.4: the CID carriers (`Cid`,
  `BlobCid`, `BlobLink`, `CidString`), their conversions, `CidString::{from, to_cid, as_str}`, and the
  note that `cid_serde` is private in epr-rea today. §A.1: `BlobHandle` (existing) shown, and
  `group_by_cid` declared as a private helper, so the default `fetch_blobs` body is normative. §A.2: the
  `own` serialization error is a serde error, never a panic. §A.6: the batch route's wire format. The
  arrived item carries `paths`, so one CID requested at two paths is sent once, and a stream without its
  trailer reads as `Unreachable`. §B: the table of adopted slice-1 types with their fields, and
  `SpineWalk` defined; the refinements are decided, not open. §C: `EprRef` named, `AdoptedHead` derives,
  and `Derive::derive`'s `from`/`into` renamed to `input_store`/`output_store`. §D.5: the `MethodId::named`
  grammar and hash, `Scratch::agent_key`, and an `XorDistance::rank` skeleton connecting `RankKey` to
  `rank`'s output. §D.6: `RealEvidence` gets derives and the `from_readings` constructor; the
  compile-time and run-time halves of I9's check are told apart. §E: `ParameterSet.schema` explained.
  §G: the `ManifestNeeds` Rust mirror. §H.1: `NamedAbsence` uses byte paths and lives in `walk.ts`, not
  `generated/`, with its mapping onto Rust's `WalkItem::Absent` and `TreeError::InvalidView`. §I:
  `ProbeAnswer` gains the serde derive, which `Reading<T>` needs in order to compile.


### Interface implementation reconciliation (2026-10-21)

The C1–C6 reports record the implementation pass against these interfaces. The revisions below
consolidate their current choices, including integration fixes; they supersede earlier signature
sketches and withdrawn alternatives above. The reports retain the per-round evidence. This is the
interface and first-adapter boundary. It does not declare the later runtime seams complete or turn
`materialize-what-is-read` green. Final gate receipts belong to the reports, not to this revision list.

| Seam | Implemented revision and reason |
|---|---|
| C1: answer and reading construction (§2.0, §I) | `ProbeAnswer::Unreachable` carries the checked `UnreachableReason` newtype, which rejects `ObservedAbsent` in constructors and deserialization while preserving the reason's wire string. Every `Reading<T>` field is private, with getters and a seven-argument constructor; `Reading` is `Serialize` only. Compile-fail tests pin answer/evidence/disclosure immutability and prevent clone-and-struct-update relabeling. These are Rust construction constraints, not proof that a producer's observations are true. |
| C1: disclosure (§I) | `Reading::route` yields private holder/parties/shared wrappers. `SharedReading::deliver` checks the reading's reach, the sink's declared audience and the 1–8 openness range before constructing the borrowed `Admitted` value accepted by `SharedSink`. `HolderSink` receives a `HolderReading`; explicit `declassify` creates a private-field, serialization-only `Declassified` record. A dishonest sink or producer remains outside this type guarantee. Per-promise HTTP wire views in `crates/elohim-views` remain future work. |
| C1: common contracts (§2.0) | `Unanswered<R>` does not require `ReasonLabel` until a label is requested. Escalation starts with the initial ask and saturates. The executable `StatusSource`/`Changes` cursor and its terminal predicate cover change-only delivery and terminal stop; the cursor harness detects silence after a change. Ordering harnesses check ranking contracts without inventing a common reason vocabulary. |
| C2: reusable values (§1.3–1.4, §D) | `elohim-compute` has a default-on `runtime` feature; placement takes the pure subset. CID serde has one public home, `elohim_epr_rea::index::cid_serde`; the temporary placement copy is removed. `NeedKey` decoding accepts either map order without losing DAG-CBOR links. `NeedTerms::Task.input_root` uses validated `CidString` because its internally tagged enum cannot preserve the native CID serializer. Its JSON spelling stays the same, but its DAG-CBOR string representation revises the original identity sketch. |
| C2: admission and accounting (§D) | Receiver household is optional input to admission and an unknown household refuses; scope derives from the registered kind and must agree with task terms. Offer reach ceiling, scope and isolation are enforced. Capacity uses the component-wise maximum of declared demand and `Kind::fits` charge, net of reservations on that carrier across kinds and scopes. Before serving any class, pending reservations also call `fits`; an unresolved charge holds the offered room. Reservations are keyed once per carrier and commitment identity, repeated expressions do not double-charge, and an honored incumbent is credited its own footprint. Duplicate live rows count one carrier, while dead rows are neither honored nor released. |
| C2: unread inputs and settlement (§D.6) | `NeedSource::needs` returns `Answer<Vec<Need>>`; `Snapshot.needs_read` and `Plan.unencodable` preserve failed reads instead of interpreting them as withdrawal. `RealEvidence::from_readings` binds a commitment, period, real evidence, promise, subject, present answer and observation time; settlement checks the same commitment/period and supported quantity. `fulfilment_promise` currently recognizes only custody kinds (`holding-is-honored@1`); other kinds still need promise mappings, while tasks use the separate completion-event contract. The immutable `Reading` getters are the only consumer access. `Ports::evidence` defaults to none and the floor settles at period close; `decide` does not fabricate evidence. Provider attribution remains the runtime adapter's responsibility. |
| C2: parameter authority (§E) | `InForce` retains verified scope and exposes `binds(scope, kind)`; built-in fallback is scope-free. `RatificationCheck` verifies a head against its charter. Ratified and charter declarations compete in one clock-ordered tier (`ratified_at` and `adopted_at`); the intermediate ratified-over-charter ordering was withdrawn. Explain records the parameter-set CID and last-verified fallback. Draft/pressure inputs decode as untrusted inputs and validate own needs before constructing private checked values. Numeric defaults and placeholder weighted-capacity weights remain unratified. |
| C2: rankings and identity (§D–E) | `NearestBytes` is lexicographic over input/base holdings, receiver domain, distance and XOR; it has no weighted factors. `WeightedCapacity` retains a weighted score. Diversity ordering preserves all incumbents ahead of non-incumbents while diversifying within each tier. Commitments are matched by kind, key and receiver; `Carrying.in_scope_of` records scope, and the historical `need` identity is retained. Scratch state is bounded by the snapshot and carryings are indexed once per pass. |
| C3: executable task values (§F) | `TaskSpec` and `TaskOutcome` have private fields, getters and checked decoding. The execution identity includes normalized output prefixes as well as child and envelope; empty outputs preserve the earlier identity. Pinned artifact/child validation rejects mismatched artifact identities, passphrase input, non-scrubbed environments and unsafe paths. `TaskKind::with_spec` takes a checked spec and optional measured view bytes; `as_need` also receives measured view bytes. A spec alone cannot invent capacity or grant scope. The current runtime envelope has no isolation field; no isolation promise is inferred from it. |
| C3: berth and outcome records (§F) | `BerthOffer` retains a private, normalized, at-most-64 hold list, narrowing duplicate-root reach before truncation. The REA projection emits one `Use` intent per declared capacity unit. A successful outcome records a derivation only for zero exit with the expected output; failed logs may be retained without a successful derivation. Depth conversion uses adapter functions rather than cross-crate orphan implementations. |
| C4: tier composition (§A) | `TierChain` takes `Arc<dyn TierOrder>` and orders per request; no constructor root hint survives. Locally held `BatchOptions.root_reach` is not a wire field. Forwarding receives bounded `OnBehalf` context, inherited budget and normalized `via`; `RootReachGate::root_level` can supply held reach to the serving adapter. `TierEndpoint::new` is fallible in every build and refuses its own tier id among upstreams. Read, list, diff and presence share the gate. |
| C4: refusal and error truth (§A–B) | Gate-specific root denial reasons stay local. Every caller-facing root denial normalizes to `OutsideStanding` with no `needs_standing`; presence returns `Missing`. A regression compares the entire `Unanswered` envelope for private and unknown roots and verifies refusal before storage reads. The implementation returns before constructing an upstream target. A locally verified integrity failure or local storage fault dominates a remote refusal; a remote peer's claimed integrity failure is only unreachable evidence. `EprfsError::Corrupt(BlobCid)` is typed and unanswered errors are boxed. No per-reason counter privacy or cryptographic unobservability claim is implied. |
| C4: membership and bounded work (§A–B) | Core owns `check_membership`, `Membership` answer mapping and `ReachLevel::narrower`; serving reuses them. Level walks reject unoffered entries and retain typed absence/unanswered states. Retained frame/path bytes and pending entries are bounded; write-back has a separate bounded byte queue flushed before the batch returns. The request-item ceiling is local, not negotiated. `SemaphoreGate` bounds per-requester concurrency and requester cardinality and prunes idle state; the proposed byte bucket is not implemented. Unknown body size remains unknown. `OffloadedDirectory` moves blocking I/O off the async executor; a decoded-node LRU awaits measurement. |
| C4: derivation boundary (§C) | Method/key/projected-root/witness records and lenient bundle decoding are implemented interfaces. The derived tree-path form is binary CBOR only and is not admitted by a serving tier yet. Slice-4 `walk_spine`, `project_view`, `derive_or_reuse`, projected-root vectors, the derive contract and `HouseholdFirst` remain follow-ons. The materialization probe in `eprfs-core/src/probe.rs` is also a future runtime seam. |
| C5: storage adapters (§1.3, §A, §D) | `PlacementStrategy` delegates ranking through legacy custody values while preserving the existing salvage tests unchanged. `BlobStoreTier` holds tree-addressed bytes beneath a dedicated `eprfs-tier` store, separate from public blob serving; raw and DAG-CBOR addresses are verified on writes and reads. `RootReachGate` reads injected locally held reach records; no database provider or remote route is installed by this pass. R-E3 allows the approved core/serve dependencies and forbids adapter back-edges. R-P8 scans production code, including declarations after test modules; dependency scans include Cargo's legacy underscore table aliases. |
| C5: honest local absence (§A.1) | `BlobStoreTier::has_blob` uses the existing fallible `BlobStore::size`: held is local, only not-found is missing, and other I/O errors propagate. The shared `BlobStore::get` metadata path now falls back to compute payloads only for not-found; other metadata errors stay errors. An ENOTDIR regression covers presence, fetch and batch classification as unreachable rather than not-held. Successful reads and the legacy public serving layout are unchanged. |
| C6: browser address and transport (§H) | `@elohim/epr` owns strict address verification for supported sha2-256 raw/DAG-CBOR CIDs and legacy SHA-256 blob hashes. `HttpTier` passes raw arrivals through chain verification; `CacheTier` verifies both stored writes and read hits. Batch CBOR sequence framing is bounded and incremental, trusts a trailer only at clean EOF, and invalidates it on trailing bytes. Timeout/item limits and cancellation are explicit. Signer headers and signed request shape remain a proposed transport contract until the serving route implements them. Cache-body read failures yield named absence and do not abort later batch items. Both former extra verifiers now delegate to the common verifier, and the R-T3 gate declares the source trees it scans. |
| C6: placement client (§H–I) | `DevicePlacement.declared()` distinguishes an undeclared need from a missing status; worker failures reject rather than looking like an empty declaration set. Watches terminate only on answered/present or withdrawn, deduplicate the reading rather than its changing retry time, and respect bounded retry timing. Proposed placement/explanation URLs are client interfaces; there is no deployed endpoint implied by their existence. The workspace/public package surface exports the client and HTTP tier through its barrel and framework-free `./placement` and `./tiers/http-tier` subpaths. Schema codegen selects per-interface output directories; `AnswerReason` and `PlacementExplanationView` are emitted into `storage-client-ts/src/generated/placement/`, with writing and verification using the same output map, rather than importing a gitignored cache. The app gate separately checks the worker with strict TypeScript and WebWorker globals: the Angular entry graph excludes it, while its bundle and unit runner only transpile it. Package, worker type-check and focused unit evidence belongs to the C6 report and does not establish live worker routing. |

**Still open before runtime graduation.** These are explicit boundaries of this pass, not errors that
an interface-only green gate can settle:

- **Reach after inclusion.** A CID admitted through one root can also occur under a narrower root.
  The operator must resolve membership-by-inclusion versus per-object restriction before a remote tier
  ships. `admit_object` is not enforcement today. The private/unknown refusal normalization above closes
  a separate existence oracle; it does not answer this inclusion policy.
- **Publication and licensing.** eprfs is at 0.2.0 and `eprfs-serve` declares `publish = false`. The dependent
  seam-contracts release and its AGPL-3.0 versus the eprfs workspace's Apache-2.0 declaration need a
  release/licensing decision. The current publisher's fixed crate list also needs that dependency
  order. Local gates do not prove registry availability or authorize a release.
- **Runtime acceptance and completion.** First-acceptance-wins in the coordinator, TaskRunner
  materialization/lookup/input substitution/output sealing/completion, registered production kinds,
  actual carrier evidence attribution and a completion cursor remain named follow-ons. The first
  accepting berth has not been wired merely by building `TaskSpec` and `TaskOutcome`.
- **Runtime serving and the worker.** A Rust remote tier/transport route and signer agreement are
  pending, as are the worker's live cache/head-plane/root-header/resident-view paths, persistence at
  kept depth, tier broadcast/order selection and delivery-peer routing through the common chain. Pure
  cache, connector and placement-client tests do not establish household reachability or offline app
  closure. No materialization or per-promise serving probe is claimed here.
- **Identity and tuning follow-ons.** Commitment ids still omit scope, and fulfillment identity still
  uses period start without period length; changing either requires a deliberate identity migration.
  A receiver without a posture still needs an authoritative household source. The unratified defaults,
  freshness horizons and device-scope tuning above remain design decisions.
