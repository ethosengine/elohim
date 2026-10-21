---
id: placement-seam-check-report
title: Adversarial seam check of the placement-contract interfaces — reviser's report
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#4
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-seam-check-brief | Adversarial seam check of the placement-contract interfaces against the code that must fit them | sha256:7fcd988908eaaf44 | path: genesis/docs/superpowers/plans/task-placement-seam-check-brief.md"
  - "placement-interfaces-brief | The composed interfaces of the placement contract | sha256:f953a81da54d3247 | path: genesis/docs/superpowers/plans/task-placement-interfaces-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:c0a34ce3ff16f2d1 | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
---

# Adversarial seam check — reviser's report

Seat: the reviser for I2 (gap `#4`). Input: the seam check's 155 findings across seams A–I. Each finding
was checked against the code it cites, then applied to
`genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md` or refused with a
reason. The spec's §6 records every applied revision by the ids below.

## Outcome

- **Applied: 155. Refused: 0.** Every finding's evidence held in the working tree. Line numbers had
  drifted by a few lines in `walk.rs`, `tree.rs` and `p2p/mod.rs`, but the code each finding cited said
  what the finding claimed. Where a finding offered two fixes, the one chosen is named below.
- **Gate evidence:** `epr flow cites verify genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md`
  → `✅ all cites content-addressed + resolvable`, `EXIT=0` (run twice: after the revisions, and again
  after 18 code paths the revisions now rely on were added to `cites:`). `epr flow context` on the spec
  prints an ambiguous gate (`just gate constants-sync | domain-types | … | schema-validate`). No code
  or schema changed, so cites verify is the gate this document-only seat runs.
- **Evidence spot-checks run this session** (each matched the finding):
  - `blob_reach.rs` `refs.is_empty() → Serve`, and `http.rs`'s fail-closed `Authorize` arm;
  - `DeliveryPeer { peer_id, multiaddrs, http_port, household_id }`, and `race_fetch(blob_hash,
    candidates, …, parallelism)` with `FetchOutcome::Hit { bytes: Vec<u8> }`;
  - `extract_agent_cid_explicit` reads `X-Agent-Cid` verbatim;
  - `fetch_checked` maps every error other than not-found to `Storage`, and the walk continues only on
    `Incomplete`;
  - `TreeError` and `AbsentReason` are already `#[non_exhaustive]` and `EprfsError` is not;
    `FetchPolicy` has no `Default`; `TreePath` already derives `Ord`;
  - `placement.rs`'s failure-domain tie-break, dedup and multi-pass greedy;
  - salvage's `self_read_cid`/`self_write_cid` and `salvage_capacity::list_fresh`'s freshness cut-off;
  - the `compute_task.rs` zome is provider-addressed; `GRANT_SCOPES` and its one-way `required_scope`;
  - ark `Berth` has no quota, `effective_tier` is per process, `ChildPolicy` defaults to
    `Restart::Permanent`, `ResourceQuota.pids` exists, and `ExitClass` exists;
  - cid 0.11's serde writes bytes and visits only bytes or a sequence; epr-rea's `cid_serde` exists;
  - all eight domain manifest ids are slugs, and `test-schema.mjs` records that the eight pillar
    manifests already fail;
  - `codegen-ts.mjs` skips enums with neither `_dna` nor `_codegen`, and `storage-client-ts` is not in
    `GENERATED_OUTPUT_DIRS`;
  - `apps-sw.ts` lines 61, 94, 145, 161, 187, 233, 268, 341 and 482;
  - the iframe renderer's `allow-same-origin`;
  - `raw-cid-verify.ts` fails closed on dag-cbor, and `content.service.ts` uses `expectedSha256Hex`;
  - `eprfs-ts` is not in `pnpm-workspace.yaml`;
  - `delegates-compute.scope` is an open string with `rate_per_hour`;
  - brit `BuildAttestation` has `success` and `signature`;
  - `sut.ts` lists the SUT parts and `dnaHash`;
  - the eprfs gate's declared `inputs.sources` reach beyond `elohim/eprfs`;
  - the evidence-class habit is `unwired`;
  - the design spec's ruling 7, §12.3 "bare CID … refused", §12.5 "history-shaped", and the §7.2
    two-phone household;
  - R7 in `task-placement-interfaces-brief.md`, which the spec omitted.

## The central revision

Ruling 7 (R7) had no type in the spec, and fourteen findings depended on one (B1, C13, D6, E10, H4,
and the absence findings in A, B, I). The spec now opens §2 with §2.0, which adds to seam-contracts:

- `Unanswered<R> { why, come_back, meanwhile, escalation }`;
- a `PendingStatus` channel, with a pull `StatusCursor` in the pure cores and stream adaptors at the
  edges. Its terminal states are `Answered` and `Withdrawn`.

R-E1 admits `elohim-seam-contracts` to eprfs-core, which reverses decision 4. The finding offered two
homes for the type. The zero-dependency leaf was chosen over eprfs-core because the ruling asks for "the
purest crate every seam already depends on". Decision 4 and finding 17 record the reversal.

## Findings, applied or refused

The alternative chosen is given where a finding offered two.

**A. Storage tier and tier chain**

- A1 applied: `HttpTier` never uses `/blob/{cid}`; `fetch_at` is a one-item `POST /eprfs/v1/blobs`. A
  third slice-2 precondition is added: `/blob/{cid}` refuses tree-only objects, or `BlobStoreTier`
  keeps them off that route.
- A2 applied: `TierId::own` never serializes. `EprfsStorage::tier_id()` and `TierEndpoint.self_id`
  stamp absences, and requester adapters rewrite a remote `own`.
- A3 applied, both fixes: membership resolves the spine through `upstream`, and
  `ItemAbsence::MembershipUnknown` (not final) covers a tier with no upstream. `NotUnderRoot` is said
  only when a held node proves the path absent.
- A4 applied: `TierChain::forward_blobs(on_behalf, …)` intersects the outer tiers with `via` and passes
  `on_behalf` to `ForwardGate::admit`. `TierEndpoint.store` is the chain's `Arc` store.
- A5 applied: the inequality is now `openness(root reach) >= openness(tier standing)`. Standing comes
  from `TierRecord.standing` (a `ReachLevel`), or from the class default. Contract 4b added.
- A6 applied: `PeerTransportTier` is one instance per peer over a new `/elohim/eprfs-blobs/1`. It no
  longer wraps `race_fetch`.
- A7 applied: authenticators are named per transport (doorway header gives `DoorwayHeld`; a signed
  direct request gives `DeviceHeld`/`HubHeld`; otherwise anonymous). `HttpTier` takes a signer.
- A8 applied: `fetch_at` takes `&BatchOptions`; the chain reaches outer tiers only by batch.
- A9 applied, merged with B2: `TreeError::Unanswered`, `AbsentReason::NotHeld.tried` and `Withheld`.
- A10 applied, option (a): local `ItemAbsenceLabel` and `ReachReasonLabel` enums in eprfs-serve, which
  takes seam-contracts. The reuse-map row is corrected.
- A11 applied: `TierId::transport`, `TierRecord.endpoint`, and `ChainStats` keyed by the dialled id and
  never read as attribution.
- A12 applied: `FetchPolicy` gains `Default` (`#[default] LocalOnly`), recorded in finding 4.
- A13 applied: contract 10 now says each item is answered once and bytes are fanned out per CID.
- A14 applied: `sources: Vec<TierSource>`, plus `TierRecord::merge` with a rule for class conflicts.
- A15 applied: `Proximity` loses `Ord`; the comparator is explicit; `order(root, …)`; `warm_roots`.
  The serve_routing row now puts the holder filter in `tier_records`.
- A16 applied: `rename_all_fields = "camelCase"` on every tagged enum.
- A17 applied: the full `from_error` mapping.
- A18 applied: chain `has_blob` answers `Unknown`; a new `RootRequired` error; the projection-manifest
  materializer is no longer listed as a chain caller.
- A19 applied: the harness and `CountingTier` move to eprfs-serve `testing`; elohim-storage writes its
  own `ForwardGate`; R-E3 allows eprfs-serve's `testing` feature as a dev-dependency.

**B. View, walk and query**

- B1 applied: §2.0, R-E1, and line 81 now cite ruling 7 and R7. `AbsentReason` gains R7 fields, and
  `TreeError::Unanswered` replaces the `String` reason.
- B2 applied: `TreeError::Unanswered`, a mapping in `fetch_checked`, the walk matching both variants,
  and `NotHeld.tried`.
- B3 applied: an un-offered item becomes `Withheld { Unreachable }`; unseen items are re-sent once.
- B4 applied: `LevelWalk::absent` returns `TreeResult<()>`; integrity and own-store failures are errors.
- B5 applied, first option: the depth-0 queries become defaulted `EprfsStorage` methods and `TreeQuery`
  is removed. Walk functions become `?Sized`. Constructors and two-form serde are added. Routes are
  `/eprfs/v1/{read,list,diff}`. The "`TierEndpoint` does" claim is corrected.
- B6 applied: `diff_side` carries its side's root; `serve_diff` admits both roots; a test is added.
- B7 applied: `TreeView` decode refuses a path list that is not canonical; the root is `""`; a shared
  vector is added.
- B8 applied: `Hydrator` reads take `&self`, with the spine cache behind a std `RwLock`.
- B9 applied: the `MaterializationPolicy` mapping is stated; deprecated one release; added to the
  0.2.0 break list.
- B10 applied: `OverlayChange::Put { kind: TreeEntryKind }`; `remove` erases changes under the path;
  a contract test is added.
- B11 applied: field renames (`coverage`, `entry_coverage`) and the eprfs-storage test site.
- B12 applied: finding 2 is corrected (only `EprfsError` remains); public constructors are added.
- B13 applied: `wanted()` yields chunks of at most `max_pending_entries`; an edge test is added.
- B14 applied: `Depth` loses `Ord`; `satisfies_offline` is added.

**C. Derivation by identity**

- C1 applied: `lookup(who, at: &TreeView, method)`. The serving index admits the input root and
  recomputes P; the requester gates each index with `may_ask`. P is the storage key, never the reach
  key.
- C2 applied: an output inherits its input root's reach and is fetched as
  `fetch_at(input_root, BatchItem::derived(..))`. The reserved path contains NUL, which `valid_name`
  refuses. Contract 1 adds a second-peer fetch.
- C3 applied: `Agreed` is removed and the reader computes agreement with a `DomainRoster`.
  `ComputedHere` becomes `Computed`. A `Witness` field is added. Mismatches surface as `Mismatch`.
- C4 applied, merged with C12/C15: `MethodDescriptor` binds params, exec, step, embedder, the declared
  view, non-path inputs and `deterministic`. `params` leaves the record.
- C5 applied: `walk_spine`; contract 5 counts spine nodes.
- C6 applied: P construction rules are pinned, with P vectors in `tree-vectors.json`.
- C7 applied: bundles per (adopted head, method); the misquote is fixed; `lookup_many`; the cache key
  is the bundle CID via the head; `to_attestation` with `AttestationKind::Derivation` only over an
  adopted head.
- C8 applied, merged with B5: walk functions are `?Sized`.
- C9 applied: `prior: Option<&Derived>`, `parents`, and contract 10.
- C10 applied: `Recomputed { checked, against, matched }`.
- C11 applied: no `deny_unknown_fields` on records; `#[non_exhaustive]` with `#[serde(other)] Unknown`;
  `decode_lenient`.
- C12 applied: the view sits in the descriptor; the worked example uses the gate's declared
  `inputs.sources`; a commit to `elohim/epr` moves P.
- C13 applied: `DerivationUnanswered` and `DeriveOutcome::NotDerived`, with capacity gating.
- C14 applied: `ProjectedRoot`, built only by `project_view`, and `KeyInput { Projected | Raw }`.
- C15 applied: `deterministic` is in the descriptor; the index rejects a mismatch.
- C16 applied: failed attestations are not projected; `signature` maps to `Witness::Signature`.
- C17 applied: `Derive` is named in R-E2; `DerivationKind` joins the 0.2.0 batch.

**D. Placement floor**

- D1 applied: fill fires on the holder count or the domain count; drain keeps both counts; contract 5
  gains a carve-out; the worked example is rewritten.
- D2 applied: per-receiver `admitted`/`consent` maps; `Consent::Granted` gains recipient, valid_from
  and epr_scope (`EprScope`).
- D3 applied: `Candidate.live` against the charter's freshness horizon; the over-fill claim is
  corrected; contract 14 added.
- D4 applied: `CarrierLost` is the second clock-born trigger, bounded per carrier per horizon. §4,
  §3.2 and decision 18 are amended.
- D5 applied: a guard against re-drafting a pending own commitment; `me_aliases`; contract 13 is
  restated as named behaviours, and the tests to rewrite are listed.
- D6 applied: `Ineligible`, `NotCarried`, `Outcome` and `PrivatePlan.refusals` all carry
  `Unanswered`.
- D7 applied: `MethodRegistry`, passed to `decide` and `explain`; observe means "not in the registry".
- D8 applied, merged with F4: `RankInput { need, receiver_domain, input_root }`.
- D9 applied, first option: one `Floor` per peer across scopes; `Snapshot.charters` per scope;
  contract 15.
- D10 applied: the legacy custody mapping is stated; `explain` and `Carrying` are keyed by `Need::id`.
- D11 applied: own-household offers stay `household_only` and `deferred` on battery; the worked example
  is rewritten.
- D12 applied, first option: the snapshot is unique by carrier (smallest domain), and the domain is
  also the final tie-break in `RankKey`.
- D13 applied, both parts: `HouseholdLocal { household }` names the receiver's plane, and a
  cross-household carrier of a below-public need is ineligible (`CrossHouseholdBelowPublic`) until the
  transport exists (decision 25). The worked example uses a public tree for the cross-household fill.
- D14 applied: derives on `Snapshot`, `InForce` and `Plan`.
- D15 applied: multi-pass `diversify`, with the A/B vector.
- D16 applied: `affected` is a skip test.
- D17 applied: `admit` is reordered to consent-first, and `Eligibility` overrides `decide`; an
  equal-kind test is added.
- D18 applied: `Release.provider`; contract 2 is extended; `ReleaseReason::CarrierLost` is removed.
- D19 applied: R-P8 now scans for `impl PlacementStrategy for` and `impl Ranking for`.
- D20 applied: `PrivateSnapshot.reach`, `Requirements.pids`.

**E. Ceiling**

- E1 applied: the scope leaves `ParameterSet` and goes on `HeadDeclaration`; the built-in CID is
  scope-free.
- E2 applied, first option: `Ranking::factors()`. `within_band` refuses a weight on a factor the
  method does not read. The `CustodyRoot` built-in is `WeightedCapacity`; `CustodyBlob` stays
  `XorDistance`, with no tuning in band. The worked example is rewritten.
- E3 applied: `ParameterHead::declarations`, and a pure `select_in_force` that verifies before it
  elects; contract 3 is reworded.
- E4 applied: one tier `()` with clock = `ratified_at`, so the newest ratification wins. This states
  what displaces a tuning.
- E5 applied: the predecessor moves to the head; a revert is exempt from the band; incumbency uses
  `ranking_cid`.
- E6 applied: `diversify` is removed (domains are the charter's); weights must be > 0; the move cap is
  bounded by the charter's range; exclusion is a ratification question.
- E7 applied: `genesis_params: BTreeMap<KindId, Cid>`.
- E8 applied: `Charter.tuning_rule` (`TouchedConsent`, or `Clause { cid, min_lineages }`) and
  `RatificationRef.touches`.
- E9 applied: `from_head` verifies `BuiltIn` and `Charter` in core; contract 1 is extended.
- E10 applied: `ParamRefusal` inside `Unanswered`; `Explanation.fallback`.
- E11 applied: `Draft.reach` and `scope`; `Pressure` fields are private, with checked constructors.
- E12 applied, merged with D10: the legacy `under_params` is the built-in `XorDistance` ranking CID;
  the knob is retired, or the peer is observe-only.
- E13 applied: `select_in_force`, called by `Ports::snapshot`; `ceiling_contract.rs`.
- E14 applied, first option: one head per space naming a set per kind, matching the design gate's
  head identity and cost estimate.
- E15 applied: params keyed by (scope, kind); `from_head(scope, kind, …)` refuses a mismatch.
- E16 applied: `DeviceParams` on `PrivateSnapshot`; §6 records that the ratification surface is
  deferred.
- E17 applied: `ParameterSet::cid` returns `Result`; `InForce` stores its CIDs.
- E18 applied, second option: salts are dropped, with the reason recorded in §6.

**F. Task on a berth**

- F1 applied: no `scope` field; `TaskSpec::scope()` is derived one way (`required_scope` moves into
  placement-core).
- F2 applied: `MethodDescriptor.exec` binds child and envelope; `TaskSpec::new` refuses a mismatch.
- F3 applied: the SUT view and non-path inputs; the measure is non-deterministic; reuse under grant is
  the default and `--fresh` refuses it.
- F4 applied, merged with D8: `NeedTerms::Task`.
- F5 applied, first option: `PlacementKind::receiver_may_carry`; consent resolution moves to the
  snapshot builder.
- F6 applied: `OfferTerms.scopes`/`isolation`, `Requirements.isolation`, and placement-core's own
  `Isolation`.
- F7 applied: `from_berth` takes the `RuntimeManifest`; isolation is the weakest tier; empty means
  `None`.
- F8 applied: `holds` carry a per-root reach; `posture` filters on it; a contract item is added.
- F9 applied: `base_root`; `NearestBytes` ranks input-root holders, then base-root holders; the worked
  example is rewritten.
- F10 applied, first option: a provider-less request with first-acceptance-wins as a coordinator
  change (decision 24, operator). The contract is "one commitment per task request".
- F11 applied: settlement goes on `rakia-compute-complete-v1`; the "closes gap 1" claim is dropped for
  tasks; `ExitClass`; a derivation is recorded only on a clean exit.
- F12 applied: `TaskChild` has no restart policy and converts only to `Temporary`.
- F13 applied: `{input}` and `{output}` template keys.
- F14 applied, first option: `Requirements.carriers`, refused as `NotNamed`.
- F15 applied: flow step 8 withdraws the need; a contract item is added.
- F16 applied, merged with D20: `pids`, plus a `bytes` rule.

**G. Manifest needs and SDK schemas**

- G1 applied: the per-field CID rule via `cid_serde`; `Expresser` and `RatificationRef` become
  structs; `CidString` in sets; a round-trip vector.
- G2 applied: `ManifestContext` takes the release root and a caller-computed `closure_key`, so
  placement-core never names `TreeView`.
- G3 applied: `loss` gives demand rules; `compute` gives standing grant requests; the manifest CID is
  an input.
- G4 applied: the contract is restated as non-regression, with discriminating tokens and a baseline
  count.
- G5 applied: follow-ons are named in the write-set paragraph; S5 is narrowed. Concern below.
- G6 applied, merged with I8: monomorphic views in elohim-views; `Reading<T>` stays internal.
- G7 applied: `worksOffline` is `sparse | on-read | kept`; `query` is refused; absent means `kept`;
  the reader maps the declared depth.
- G8 applied: closure `[""]`, content type `concept`; finding 8 is restated.
- G9 applied: the path pattern accepts `""`, `minItems: 1`, and a normalization vector.
- G10 applied: grant-scope is the compute-grant subset; `ratePerHour` maps onto `rate_per_hour`; the
  equality test is extended.
- G11 applied: `_codegen` is required and `_dna` forbidden; the Rust-side equality test reads the JSON.
- G12 applied: the `placement-factor` enum, with the same-change rule.
- G13 applied: `epr:schema:view:placement-explanation`; a hex digest.
- G14 applied: `indexed.measure` is the IndexMeasure CID or a `PinnedRef`.
- G15 applied: shapes nest under `$defs.ManifestNeeds`; the scan covers what is `$ref`-reachable from
  them.
- G16 applied: decision 22 (operator).

**H. TypeScript SDK and service worker**

- H1 applied: the `ArchiveWriter` writer; R-T2 is revised.
- H2 applied: tier-record and item-absence schemas; generated TS; `rename_all_fields`; shared vectors.
- H3 applied, merged with A1.
- H4 applied: generated `Unanswered`; `Placement.watch`; the 404 contract asserts all three parts.
- H5 applied: `CacheTier` verifies every hit; a forged entry is integrity and is deleted.
- H6 applied: `DeliveryPeer.agentKey`, else `TierId::transport`.
- H7 applied: `standing`; `mayAsk(tier, root, rootReach)`; the head plane records reach.
- H8 applied: a TS `TierConnector`, `HttpConnector`, and an endpoint list on the BroadcastChannel.
- H9 applied, second option: new rows serve kept apps cache-first and read the head before any probe;
  the contract is restated.
- H10 applied: the head plane split (`residentView`); `X-Tree-Root`.
- H11 applied: secure-context transports are named; household HTTP stays loopback/dev only until one
  exists; the test runs on an https origin.
- H12 applied: a typed `NeedRef`; a split network/device explanation; no remote report of
  `PrivateNeed`.
- H13 applied: the raw-only wrapper is kept; both legacy separators are accepted.
- H14 applied, first option: byte-component paths, `pathString`, and `invalid-view`.
- H15 applied: workspace, gates and prebuild follow-ons are named; there is a local verifier until
  `verifyAddress` lands.
- H16 applied: storage-client-ts and eprfs-ts join `GENERATED_OUTPUT_DIRS` as a follow-on.
- H17 applied, merged with A15: `order(records, root)`; per-root warmth and recency; the dropped
  `servesExtracted && warm` filter is stated.
- H18 applied: one doorway per origin; other doorways come from the held doorway list over CORS; the
  cache is keyed per doorway.
- H19 applied: `CacheTier` has its own cache outside the purge; depth 3 needs a persistent grant.
- H20 applied: "no network timeout awaited", with a fake-timer assertion.

**I. Probe shape**

- I1 applied: `ProbeAnswer<T>` pairs each reason with its state; the wire `$ref`s the answer envelope.
- I2 applied, option (b): `Probe::read` returns `Option`. No `AnswerReason` variant is added, which
  avoids a break in published 0.1.0.
- I3 applied, merged with A10.
- I4 applied: slice-1 fields (`boundary_count`, witness, baseline, optional tiers); `with_tiers` in
  slice 2; the worked example prints plain JSON.
- I5 applied: `Disclosure { Holder, Parties { roles }, AtReach { openness } }`.
- I6 applied: `chosen_set` CID; derives; household disclosure below public.
- I7 applied: `HolderSink` and `Declassified`.
- I8 applied, merged with G6.
- I9 applied: `FulfilmentDraft.evidence: RealEvidence`, constructible only from a `Real` reading; a
  `compile_fail` test.
- I10 applied: `enums/evidence-stage`, as the habit's first move; the field is named `evidenceStage`.
- I11 applied: answer-state `NeedStatus`; TS `stats()` and `walkBatched` stats.
- I12 applied: `observed_at`, an RFC 3339 string, as `residual.rs`.

## Concerns

1. **No commit.** The covenant's declared purpose says "no landing", and the main tree is shared, with
   many uncommitted paths from other sessions. The spec revision and this report are left uncommitted,
   as the sibling I1 report did (`commits: []`). A landing seat should stage exactly these two paths.
2. **Write-set floor (G5, H15, H16).** Lanes E and S cannot finish S2, S5, R-G1 or the eprfs-ts gate
   inside `elohim/eprfs/**` and `elohim/sdk/schemas/v1/**`. The spec now names each follow-on path. A
   write-set floor for those paths seems warranted; none is held, and this seat cannot grant one.
3. **Four new operator decisions.** Decision 22 covers the manifest-declared shapes (Q1, Q2, Q4).
   Decision 24 covers the provider-less compute request coordinator change. The R-E1 reversal admits
   seam-contracts (decision 4, revised). The `CustodyRoot` built-in becomes a weighted method (E2).
   Decisions 3, 6, 8 and 15 remain open.
4. **The document is now about 3,000 lines** (from 2,001), and seams A and D carry most of the growth.
   No signature here was compiled. The graduation trigger still requires each first instance to compile
   against these signatures unchanged, or to record a revision in §6.
5. The cite-seal hook reported two un-sealed cite issues on the spec. Both predate this seat; `cites
   verify` passes. `epr flow cites seal` was not run, because it rewrites path-cites into envelopes,
   and that is the steward's call on a Draft spec.

## Cold read

A cold reader new to the document read it after the seam check and returned 16 findings. This pass
applied the 15 of class correctness or interpretability and left the one preference finding (splitting
§6's revision list into per-seam headings). Every type shown as existing was checked against the working
tree on 2026-10-20 (`eprfs-core/src/{storage,address,tree,walk}.rs`, `seam-contracts/src/{answer,election}.rs`,
`epr-rea/src/{model,index}.rs`, `reconcile/placement.rs`). The spec's §6 carries the same list as a
"Cold read" revision entry.

- **Defined what was used but not shown.** `Answer<T>` (§2.0) is shown as seam-contracts defines it, with
  a table of how placement reads Present, Absent and Unreachable for each field that holds one.
  `BlobHandle` (§A.1) is the existing `{ cid: BlobCid, bytes: Bytes }`. §B adds a table of the adopted
  slice-1 types with their fields (`TreeLimits`, `TreeEntry`, `BlobPresence`, `WalkedNode`, `WalkItem`,
  `WalkStats`, `WalkedView`, `LeafKind`, `ReadLeaf`, `DirListing`, `TreeChange`). `SpineWalk` was new and
  undefined, and is now defined. `EprRef` is eprfs-core's existing slug type, and `AdoptedHead` gets
  derives. The `ManifestNeeds` Rust mirror is hand-written and pinned by `schema_contract.rs`, with no
  `deny_unknown_fields`.
- **CID carriers** (new §1.4). `Cid`, `BlobCid`, `BlobLink` and `CidString` are tabled with how each
  serializes. `BlobCid` is a string in DAG-CBOR too; `BlobLink` is tag 42. The conversions are listed
  (`*blob.as_cid()`, `BlobCid::parse`, the new `CidString::{from, to_cid, as_str}`), and with them
  `ManifestContext.closure_key`'s exact expression.
- **Wire format** (§A.6). The batch response is a cbor-seq. It carries arrived items
  `{cid, paths, bytes}`, one per distinct CID, then exactly one `{absent: […]}` trailer, with a
  diagnostic-notation example. A stream that ends without its trailer reads as `Unreachable`, so the walk
  sees `Withheld`, never `NotHeld`. The arrived item now carries `paths`, so membership-checked fan-out
  is not left to the client to guess. That is a shape change to the earlier one-line `{cid, bytes}`.
  §6's open item narrows to a typed mid-stream error item.
- **Rust and TypeScript absences** (§H.1). `NamedAbsence` moved out of `generated/` into `walk.ts`. Its
  paths are byte components, as H14 already required: the string-typed `path` and `leaf` could not carry a
  non-UTF-8 name. A table maps each variant onto `WalkItem::Absent` or `TreeError::InvalidView`.
- **Made the sketch normative.** `group_by_cid` is declared as a private helper, and the default
  `fetch_blobs` body is the default as written.
- **Ranking** (§D.5). `MethodId::named` has a stated grammar and hash (raw CID over the UTF-8 name).
  `Scratch::agent_key` is the accessor `rank` needs. An `XorDistance::rank` skeleton shows the `RankKey`
  values sorted greatest-first into `out`, which reproduces today's ascending (distance, agent, domain)
  sort.
- **Smaller clarifications.** `ParameterSet.schema` is the encoding version: 1, in `cid` and not in
  `ranking_cid`, and an unknown value is refused. `TierId::Own` returns a serde error and never panics.
  `Derive::derive`'s `from`/`into` are renamed `input_store`/`output_store`. The slice-1 refinements are
  stated as decided, so a change goes through §6, which keeps the graduation trigger's "unchanged" honest.
- **Correctness defects found while applying them.** (1) `FulfilmentDraft` and `Plan` derive
  `Clone, PartialEq, Eq, Serialize` over a `RealEvidence` that derived nothing, so they would not
  compile. `RealEvidence` now derives them and has the one constructor `from_readings`. The I9 check is
  split into the part a `compile_fail` doctest can prove (no struct literal) and the part only a run-time
  test can prove (a Fixture reading is refused). (2) `Reading<T>` derives serde over a `ProbeAnswer<T>`
  that had none, so `ProbeAnswer` gains the same `cfg_attr`. (3) `cid_serde`, which the spec cites as
  epr-rea's helper for every placement CID field, is a private `mod` in `epr-rea/src/index.rs`, so
  placement-core cannot name it. §1.4 now says so and names the follow-on: epr-rea makes it `pub`, and
  placement-core keeps no copy of its own.

`epr flow cites verify` on the spec passes. Nothing is committed, and no signature was compiled.
