---
id: placement-c5-report
title: "C5 report — elohim-storage adapters: the ranking shim, the blob-store tier, the root reach gate and rail R-P8"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#28
actor: agent:implementer@codex
commits: []
habits: [materialize-what-is-read]
cites:
  - "placement-c5-brief | C5 brief | sha256:8c975eefd4dd4d37 | path: genesis/docs/superpowers/plans/task-placement-c5-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:de0208f9b6e426db | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
---

Resumed the interrupted C5 implementation at base `6cc1a17d5`. The preceding report was an
unverified draft; this report records the completed adapter work and its final-source checks.
No staging, commits, push, publication or mesh restart was performed. The household identities
remain stopped and preserved.

## Delivered

- `src/reconcile/placement.rs`: `PlacementStrategy` is a compatibility shim over placement-core's
  rankings, using admitted legacy custody values. Its 21 existing tests are unchanged from the
  base and pass. Runtime salvage ordering remains under those existing contracts.
- `src/eprfs_tier/{mod,blob_store_tier,root_reach_gate}.rs`: an `EprfsStorage` adapter holds
  verified raw/DAG-CBOR bytes in a dedicated `eprfs-tier` store, separate from the public blob
  route. `RootReachGate<R: ReachRecords>` derives reach from injected local authority and fails
  closed when that authority cannot be read. It creates no records or attestations.
- `src/boundary.rs`: R-P8 keeps rankings in placement-core, with the existing ingest diversity
  pass explicitly tolerated only until ingest expresses floor needs. R-E3 allows the approved
  core/serve dependencies and refuses adapter back-edges. The source scan preserves production
  declarations after test modules; the manifest scan handles renamed, target-specific and
  legacy underscore dependency tables.
- `src/api/compute_grants.rs`: a parity test pins the existing grant-scope strings to the shared
  placement vocabulary.
- `Cargo.toml`, `Cargo.lock`, `Dockerfile`, `src/lib.rs`: dependency/module wiring and image build
  inputs for the new pure core and adapter contract harness.
- `src/blob_store.rs`: the minimal shared prerequisite correction preserves metadata I/O errors
  in `BlobStore::get`; only `NotFound` enters the existing compute-payload fallback. This changes
  error classification, not successful reads, stored identities or the public serving layout.

## Tests and gate evidence

Required gate: `env ELOHIM_SESSION_ID=codex-placement-c5 just gate elohim-storage`.
**Final gate evidence: `EXIT=0`.**

The gate runs migration hygiene, the Docker include-target rail, formatting, default clippy,
iroh all-target clippy and the full Cargo test suite. The final run passed 5,728 tests with zero failures and 59 existing ignores across 18 result blocks:
4,792 library tests, 2 binary tests, 933 integration tests and 1 doctest.
Receipt: `genesis/local-dev/placement-codex-handoff/c5-gate-3.log`, ending in `EXIT=0`.

There are 22 new C5 tests: six BlobStoreTier tests, ten RootReachGate tests, five boundary tests
and one grant-scope parity test. They cover the leaf contract, raw/DAG-CBOR verification,
isolated storage, reach admission and unreadable authority, ranking/dependency rails, and
scope vocabulary. The existing 21 placement tests are byte-identical to the base.

`an_unreadable_store_is_not_a_missing_blob` checks missing and held bytes, then replaces the
blob directory with a file to produce ENOTDIR even as root. Presence and fetch preserve the
storage error; batch delivery reports unreachable rather than not-held. This covers the new
adapter and the shared `get` metadata correction together.

Evidence history:

- `c5-gate-1.log`: `EXIT=1` before build because the sandbox could not write the declared shared
  Cargo pool's `.gate-owner`; the native gate was retried through authorized execution.
- `c5-gate-2.log`: `EXIT=0`, but superseded for final-source acceptance. Its test compiler began
  at 02:19:57 UTC, before the last `get`/fetch assertions were saved at 02:22:28 UTC.
- `c5-gate-3.log`: `EXIT=0`, final-source rerun, supervised by the root agent after the child's escalation
  request was interrupted before launch. The test compiler invocation is 03:04:10 UTC, after
  every final functional C5 edit. `c5-source-times.log` and `c5-source-times-final.log` preserve
  the two audits. No C5 source changed during this final run.

The Cargo berth is `codex-placement-c5`, held exclusively for storage and renewed as needed.
The root agent coordinates its release/handoff after the actual final gate exit; the following native gates remain serialized.
The Docker include-target rail checks eleven external include targets; no image build or remote
serving probe is claimed. Existing ts-rs attribute warnings do not represent test failures.

## Revisions absorbed by interfaces spec §6

1. RootReachGate's specific refusal reasons are local diagnostics. C4's serving endpoint now
   normalizes denied-root answers; a local gate's detailed reason is not an external privacy
   guarantee by itself.
2. R-E3 permits `eprfs-serve` as approved by the operator. The current adapter uses its testing
   feature as a dev dependency, but the rail does not impose an extra dev-only restriction.
3. R-P8 excludes each formatted test module, not all source following the first test module.
   Dependency-table aliases are normalized before applying the same direction rule.
4. Tier presence uses fallible `BlobStore::size`, not boolean `exists`, to distinguish genuine
   absence from an unreadable store. `BlobStore::get` now preserves the same distinction.
5. The `eprfs-tier` store and injected reach-record interface are first adapters. They do not
   install a database provider, authenticated batch route or remote transport.

Story-graph station: `materialize-what-is-read / between disk metadata probe → named per-item
absence / missing node: ENOTDIR/I/O errors remain unreachable through BlobStore::get and tier
presence/fetch/batch; only genuine NotFound becomes not-held / probe:
an_unreadable_store_is_not_a_missing_blob / current state: final-source gate green (`EXIT=0`)`.

## Concerns and scope

The interface/adapter work is complete with the final gate evidence above. The remaining concerns belong to the explicitly named follow-ons: runtime reach-record wiring and
remote serving, the operator's membership-by-inclusion versus per-object restriction decision,
and dependency publication/licensing before release. No live placement floor, derivation runtime,
worker routing or household offline-closure acceptance is claimed. The materialization habit
remains red. The final gate took 36 minutes and left the existing gate-cycle finding `984ccc2c18d8` open.

Independent source review by the root and Rust integration seats found the honest-absence and
rail loopholes described above; these are fixed. Their subsequent review found no further C5
source blocker. Scoped commits remain empty because the brief prohibits staging and commits.
