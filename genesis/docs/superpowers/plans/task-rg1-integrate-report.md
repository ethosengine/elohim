---
title: "Re-genesis 1 L2 report: the integrated stack (kernel → eprfs → schemas → storage → doorway → app → seeder → steward → shem-evidence)"
id: task-rg1-integrate-report
status: claimed-not-verified
landed_commit: "2bdfb5186 + this report commit (regenesis/integrate, local, unpushed)"
verified_by:
actor: agent:l2-integrator@claude-opus-5-5
cites: []
concerns:
  - "every command ran on the host's default stable toolchain (rustc 1.99.1) in a dedicated slot family (/projects/.cargo-target-pool/family/rg1-integrate/…), not the gate's pinned 1.96.1 or the shared sprint slot (STATE 22:25Z: shared sprint-family slots produced a serde_core metadata red across worktrees); the full gate pass after L3 runs the pinned toolchain"
  - "a2o is only partly crossed: the T8→T9 items named in the brief are done, the rest of genesis/a2o (~37 step/script files reading blobHash/serverBlobHash or sending X-Blob-Hash) is T9's; death-witness.steps.ts needs a design ruling (see What is still red)"
  - "commit attribution UNMEASURED on every L2 commit (no epr session in a worktree; recorded gap)"
---

# L2 report

Branch `regenesis/integrate` (worktree `/projects/elohim/.worktrees/rg1-integrate`), local, unpushed.
Logs: `genesis/local-dev/regenesis-1/l2-*.log` in that worktree (gitignored; each ends `EXIT=`).

## Stack (base `regenesis/kernel` 9f9fef877)

| Track | Source | Stacked as |
|---|---|---|
| T3 eprfs | `regenesis/eprfs` a3315d6da (+ report d73e1d9e0) | 0fe60b253 … a3315d6da (fast-forward); report 584366acd |
| T5 schemas | d382a8efe, 010a9e859 | 334a9e81b, f25117ecf (+ fix-up 1d86f1f1a) |
| T2 storage | e6adb9b43..bc0edbd64 | ce7f645e6 … f05258ac3 (fb2386513 dropped: empty, already applied by T5) |
| T6 doorway | bc0edbd64..a1a5d5603 | bec1d7d5c … 94879c7c5 |
| T7 app | bc0edbd64..cbe451c01 | 9a62582d2, bc3c33066, 00007df6f |
| T8 seeder | e489b0150, 6bca817e0 | af8b4c60e, 4fe2d3da6 |
| T10/T11 steward+ark | bc0edbd64..7e49bf5e1 | ecad4f51b, 0e482431d, 7477f4169, 3ec10bd25 |
| hub + shem-evidence | a31d94403, 09d69007d, 368aaa3ac | b087df44a, b1e99e726, 0c3fae462 (465285852/76dd9a040 skipped: radicle docs already on the kernel as 54332c45e) |
| L2 fix-ups | this report | 59ab14214 … 2bdfb5186 (below) |

The rakia gitlink is T5's f9787c9e58 (fetched into this worktree's rakia from rg1-schemas; still unpushed upstream, so pin attestation will refuse until it reaches a rakia run-ci branch). brit gitlink 1c6ae5568 (kernel).

## Conflicts and resolutions

- T5 onto the kernel (T5 was cut from the pre-review kernel ce870481a): `crates/elohim-views/src/{epr,shared}.rs`, `views/{bin-release,epr-envelope-view}.schema.json`, generated `EprSignatureView.ts`, `storage-client-ts/tests/epr-types.test.ts` took T5's side (identical to what T2 adopted; the envelope shape). `epr-ts/src/address.ts` and `tests/address.test.ts` took the kernel's post-Astra version (the stricter, reviewed verifier; L1 checklist step 1). `epr-ts/src/index.ts` merged by hand: the kernel's address/proof exports (`verifyEd25519Strict`, code constants) plus T5's slimmed blob-hash exports. 010a9e859's test change dropped for the same reason (report kept).
- bin-release.schema.json: the resolution above lost T3's pattern edit; restored T3's stricter version (1d86f1f1a), per the orchestrator's T3 note.
- T2 fb2386513 (T5 adoption) became empty and was dropped; no other conflicts in T2, T6, T7, T8, T10/T11 or the hub/shem commits.
- Steward re-stack: shem-evidence had been stacked before T10/T11 arrived; the branch was rebuilt (reset --keep to 4fe2d3da6, steward cherry-picked, the rest replayed) so the order is seeder → steward → shem-evidence. Tree diff old-head→new-head = the steward commits only.

## Fix-ups (one commit per seam)

| Commit | Seam (source) |
|---|---|
| 1d86f1f1a | bin-release keeps T3 patterns through the T5 stack (orchestrator T3 note) |
| 59ab14214 | packager `X-Blob-Cid` → `X-Content-Address`; doorway seed answers `cid`, storage `blobCid` (ruling 18:22Z; T5/T6 reports) |
| e62c4fd9c | `element-registry-view.schema.json` cid → `^bafkr4i[a-z2-7]{52}$`; schema_contract sha256 fixtures re-minted from label bytes (T2 report) |
| 47e3f81af | `hc-mesh.sh:5485` comment; `hc-mesh-prologue.sh` read/PATCH `serverBlobHash` → `serverBlobCid`; `hc-mesh-recovery.sh` (+ test) `blobHash` → `blobCid` (T8 lint; storage wire) |
| 8b678e055 | `seed-landing-bundle.mjs` runnable: `DRY_RUN=1`, `BUNDLE_DIR` (T8) |
| 17f50f72a | a2o: `delivery.steps.ts` pattern `^bafkr4i…{52}$` + capability probe `X-Content-Address`; `mesh/household-chaos.steps.ts` byte-CID admission, store path `blobs/blobs/<cid[7..11]>/<cid>`, size via peer read, custody PUT header; `doorway/native-epr-projection.steps.ts` blobCid; placeholder CIDs in content-addressing, client-resilience, spa-bundle-delivery, web2-absorption, native-epr-projection features (T8→T9) |
| 6515ddbc7 | elohim-render `BundleSource::resolve_blob_cid/resolve_server_blob_cid` (T6); `materialize_server_bundle` verified `sha256-<hex>` — every byte-CID SSR materialize would have failed — now `ByteCid::parse` + `compute_bytes_cid`; storage image rewrites elohim-address path for elohim-render and eprfs-core, doorway image for doorway-service (missing since T6/T3); `cargo-build-storage` watches `eprfs/fixtures/tree-vectors.json` (storage-build-inputs test red since 4720931e1) |
| 44a8b99a8 | storage-client-ts `DoorwayClient.seedBlob` sent `X-Blob-Hash` → `X-Content-Address` (`cid`); `StorageClient.putBlob` PUT `/blob/` with no address → mints the byte CID; `BlobResult.hash` dropped; a2o `substrate-verify.ts` minted `sha256-` → byte CID, T8 state keys; selftest follows |
| 4ef777fb9, 8acb56d06 | relocks: elohim, elohim-storage, doorway, steward/node, bridges/{cargo,jenkins,k8s} (eprfs-core's multihash-codetable → elohim-address edge and eprfs-local/storage version bumps from T3 were never relocked; elohim-render's new dep) |
| ada281e3d | a2o `steward-publish.ts` + plan: seeder builders renamed `blobCid`/`thumbnailCid` (a2o tsc was EXIT=2) |
| abd66bdb1 | source-state `closureCid` (device-runtime `source.rs:265,285`) and placement `MethodId::named` (`placement/core/src/rank.rs`) still minted sha2 raw CIDs while T5's schemas require `bafkr4i`; `genesis/orchestrator/gate-source-state.mjs:38` admitted only `bafkrei` |
| 6ec41da11 | `elohim-chrome-asset` omni-element(.js,-v1.js) read `blobHash/serverBlobHash` (build marker always empty) |
| 3eec969ae | eprfs_tier acceptance: on the stacked tree 2/3 passed; the third returned NotHeld because the store quarantines a payload on its first failed read; the tier adapter now answers `Corrupt` for a quarantined miss (eprfs-core untouched) |
| 9a43208d8 | T1 drift (e): seeder `register_shard_manifest` payload (`seed-production.ts:553`) and `ShardManifest` (`storage-client.ts:37`) recast to the DNA/storage striped shape (`sharding.rs` ShardManifest) mapped from `PutBlobResponseView` |
| 2bdfb5186 | `elohim-storage/.epr-meta/peer-serving.seam.json` named the deleted `http_blob_router.rs`/`ChooseInputs`; epr-cli `seam_guidance` 8/9 red |

T1 drift items (a)–(d), (f) were already clean on the stacked tree: `node_registry_api.rs:30` is `content_cid`; `sdk/domains/infrastructure/types/src/lib.rs:162,199,227,242` are `content_cid`; `import_handler.rs:112` is `blob_cid`; `sdk/domains/lamad/types` has no `content_hash`/`blob_hash`; no `"blob_hash"`/`"shard_hash"`/`"blob_manifest"` anchor strings in storage or seeder src. ContentSuccession: no Rust/TS consumer outside the attestation schema (`content-succession-metadata.schema.json`, `lamad/manifest/attestations.json` have no `content_hash`). No code matches link-type indices numerically (grep of storage/seeder/doorway). Not verified against the DNA itself (regenesis/dna was not on origin when checked).

Regenerated, never hand-edited: `pnpm run schema:codegen:ts` EXIT=0 with no diff (T5's commit already carried every copy, incl. doorway-app/sdk/seeder); storage `cargo test export_bindings` EXIT=0 with no diff (`Content.ts`/`ContentWithTags.ts` are the snake_case DB row: `blob_cid`/`server_blob_cid`, no `blob_hash`). `lamad:codegen` not needed (manifest untouched).

Codemod receipt: the tracked receipt holds only seeder rows; the live hub receipt holds storage rows (7 fixture-sha256 + 5 fixture-bafkrei swaps) and no doorway rows. Re-mint rows derived from the commit diffs (a lower bound, same-hunk pairs only): 15 storage(T2-B) + 3 doorway(T6) in `genesis/local-dev/regenesis-1/l2-codemod-receipt-additions.jsonl` (worktree) for the orchestrator to append to the hub copy.

## Compile/test ladder (host stable rustc 1.99.1, slots `/projects/.cargo-target-pool/family/rg1-integrate/*`)

| Step | Command | EXIT | Log |
|---|---|---|---|
| 1 | `elohim`: `cargo check --workspace` | 0 | l2-1-elohim-check.log |
| 1b | `elohim`: `cargo check --locked --workspace` | 0 | l2-4-elohim-locked.log |
| 2 | storage `cargo check --all-targets --features p2p-iroh` (jobs 1) | 0 | l2-2-storage-check.log |
| 3 | doorway `RUSTFLAGS="" cargo check --all-targets` | 0 | l2-3-doorway-check.log |
| 4 | crates/elohim-address, consent-grant, elohim-views, seam-contracts `cargo check --all-targets` | 0,0,0,0 | l2-4-crates-*.log |
| 4 | bridges/cargo, jenkins, k8s `cargo check --locked --all-targets` | 0,0,0 | l2-4-bridges-*.log |
| 4 | steward/node `cargo check --locked --all-targets` | 0 | l2-4-steward-node.log |
| 4 | storage, doorway `cargo metadata --locked` | 0,0 | l2-4-{storage,doorway}-locked.log |
| 5 | root `CI=true pnpm install --frozen-lockfile` | 0 | l2-pnpm-install.log |
| 5 | `pnpm run schema:codegen:ts` (no diff) | 0 | l2-schema-codegen-ts.log |
| 5 | elohim-app `tsc --noEmit -p tsconfig.app.json` | 2 (TS6059 rootDir only, 200; zero other) | l2-tsc-elohim-app.log |
| 5 | lamad `tsc --noEmit -p tsconfig.app.json` | 2 (TS6059 only, 226) | l2-tsc-lamad.log |
| 5 | genesis/a2o `tsc --noEmit` | 0 (was 2 before ada281e3d) | l2-tsc-a2o.log |
| 5 | genesis/seeder `tsc --noEmit` | 0 | l2-tsc-seeder.log |
| 5 | epr-ts `pnpm test` | 0 (65) | l2-epr-ts-test.log |
| 5 | storage-client-ts `pnpm test` | 0 (135) | l2-storage-client-test.log |
| 6 | storage `cargo test --lib` | 101 (4914 pass / 1 fail: eprfs_tier batch) → fixed 3eec969ae; `--lib eprfs_tier` 0 (16/16) | l2-6-storage-test-lib.log, l2-6-storage-eprfs-tier.log |
| 6 | storage `cargo test --test '*'` (16 targets) | 0 (schema_contract 247, api 181, views 141 …) | l2-6-storage-test-integration.log |
| 5 | storage `cargo test export_bindings` (no diff) | 0 | l2-5-storage-export-bindings.log |
| 6 | doorway `cargo test --lib` | 0 (2002) | l2-6-doorway-test-lib.log |
| 6 | eprfs `RUSTFLAGS="" cargo test --workspace --no-fail-fast` | 101: only epr-cli `seam_guidance` (8/9) → fixed 2bdfb5186, rerun 0 (9/9); 135 other suites ok | l2-6-eprfs-test.log, l2-6-eprfs-seam-guidance.log |
| 6 | elohim-placement-core, elohim-ark-core, elohim-render `cargo test` | 0 (23 suites) | l2-6-elohim-render-placement-ark.log |
| 6 | elohim-app vitest | 0 (238 files / 4721) after building elohim-core/imagodei/qahal dists (first run: 14 files failed to load, unbuilt elements) | l2-6-app-vitest.log, l2-build-elements.log |
| 6 | lamad vitest | 0 (111 / 2808) | l2-6-lamad-vitest.log |
| 6 | seeder vitest | 0 (739) | l2-seeder-vitest.log |
| — | a2o `substrate-verify:selftest` | 0 | l2-a2o-substrate-selftest.log |
| — | a2o `steward-publish-plan` test | 0 | l2-a2o-steward-plan-test.log |
| — | a2o `gherkin-prepush-lint` (303 files) | 0 | l2-gherkin-prepush-lint.log |
| — | elohim-chrome-asset `frame-attributes.test.mjs` | 0 (40) | l2-chrome-asset-node-tests.log |
| — | orchestrator storage-build-inputs + build-inputs-closure | 1 → 0 after 6515ddbc7 | l2-orchestrator-*.log |
| — | seed-landing-bundle `node --check` + DRY_RUN | 0 | l2-seed-landing-bundle.log |

Not run here: `just gate` for any project (operator steer: one full pass after L3), clippy/fmt lanes, rust-style, the steward/node and ark test suites (T10/T11 report EXIT=0 on their branch; L2 ran `cargo check --locked` for steward and ark-core tests only), WTR element suites, mesh/household lanes.

## Stale gospel (managed surfaces — not edited; for `epr flow cites`)

- `elohim/elohim-storage/CLAUDE.md:152` — "Wire-level identifiers (`/blob/{hash}`, `BlobStore`, `sha256-{hex}`) keep their existing names".
- `elohim/elohim-storage/CLAUDE.md:154` — Addressing canon: blob bytes are "`bafkrei…` (raw codec, `Cid::new_v1(0x55, Sha2_256(bytes))`)" and the "legacy `sha256-<hex>` blob marker still in use on the live `/blob/<hash>` path"; the migration "downstream arc" is done.
- `doorway/CLAUDE.md:162` — "Wire-level identifiers (HTTP `/blob/{hash}`, `BlobStore`, `sha256-{hex}`) keep their existing names".
- `doorway/CLAUDE.md:164` — Addressing canon: "`bafkrei…` (raw codec over `Sha2_256(bytes)`, see `src/routes/blob.rs`) for blobs"; "bare `sha256-<hex>` blob marker on the live `/blob/<hash>` path is legacy/in-migration".
- `app/elohim-app/CLAUDE.md:86-87` — sparse/blob example `"contentBody": "sha256-abc123..."`, `"blobCid": "sha256-abc123..."`; `:95` `startsWith('sha256:') || startsWith('sha256-')` detection snippet; `:155` `curl …/blob/sha256-abc123...`; `:163` "Content shows `sha256:...`".
- `app/CLAUDE.md:19-20` — ContentNode "whose `blobHash` … `serverBlobHash`"; `:106` "PATCHes `serverBlobHash`".
- `.claude/skills/p2p-design-gate/SKILL.md:203,211,234` (and its projections under `.epr-meta/elohim/projections/{claude,codex,antigravity}/…`, `.codex/skills`, `.agents/skills`) — raw blob bytes = "`bafkrei…` `Cid::new_v1(0x55 raw, Sha2_256(bytes))`"; `sha256-<hex>` "legacy / in-migration"; `transport_affinity` lives in `http_blob_router.rs` (deleted).
- `.claude/agents/rust-architect.md:73` (and projections) — "`http_blob_router` is the backend selector … `IrohThenLibp2p` local-SHA256 libp2p fallback" (module deleted, affinity column dropped).
- `app/lamad/.epr-meta:7` — "browser bundle = blobHash, SSR server bundle = serverBlobHash".

## Address-profile lint census (`scripts/ci/address-profile-lint.sh`, l2-7-address-lint.log)

1044 violations / 185 files on the bare stack → **925 / 170** after L2 (R1 sha256-address 7, R3 bafkrei 96, R4 hash-identifier 822). Classified:
- `genesis/a2o` 367 (58 files) — legacy-left-for-T9: step/script wire reads of `blobHash`/`serverBlobHash` and `X-Blob-Hash` sends (top: `steps/dataplane/epr-app-deliverability.steps.ts` 41, `steps/ui/delivery.steps.ts` 23, `steps/dataplane.steps.ts` 21, `steps/mesh/spa-bundle-delivery.steps.ts` 15, `steps/drain/delivery-capabilities.steps.ts` 13); feature prose evidence lines (`dataplane/{blob-replication,federation-deploy,served-shell-boots}.feature` observed `sha256-…` values) = prose/history.
- `elohim/elohim-storage` 361 — 355 R4: local identifiers/comments named `blob_hash` holding byte CIDs, historical migrations (`migrations/2026-01-08…`, the new migration's `down.sql`) = legacy naming/record; 5 R3 (`dag_store.rs:209`, `tests/epr/epr_atom_protocol_unit.rs:21`, `tests/p2p/p2p_command_view_federate.rs:70`, `tests/schema_contract.rs:7182,7287`) = record/cite fixtures; 1 R1 `release_adoption/verify.rs:2026` = negative fixture.
- `elohim/holochain` 70 — legacy-left-for-T1 (L3).
- `elohim/eprfs` 31 — cite (`compute_raw` fingerprints, `RAW_HELLO_CID`, recall method CIDs) and compare-only epoch-1 tree vectors = cite/record (plan §3 KEEP).
- `elohim/brit` 18 — legacy-left-for-T4.
- `elohim/elohim-cache-core` 17 + `app/elohim-library/.../content-resolver.ts:305,692,705` — unassigned: an internal `blob_hash` JSON contract between cache-core's WASM resolver and the library (values are whatever the projection carries). Functional naming debt, no wire break.
- `.epr-meta/**` 18 — record class (keep). `elohim/epr` 4, `crates/elohim-address` 1, `elohim/sdk` 10, `elohim/rakia` 1, `app/*` spec fixtures 12, `genesis/agentic/compute/cid-executor.test.mjs` 2, `elohim/ark/core/tests/candidate_contract.rs:14` — sha2 citation/record vectors and negatives (cite/record).
- `elohim/elohim-chrome-asset/tests` — none left after 6ec41da11; `elohim/elohim-facings` 3 (`shard_hash` test fixtures), `elohim/elohim-render/tests/node_builtin_shim.rs:187` (comment), `doorway/doorway-service/{seam-registry.yaml:2732,5342, src/routes/blob.rs:722}` (prose) = prose/legacy naming. `steward/node` 1 = T10 residue.

## What is still red / open

- `genesis/a2o/steps/mesh/death-witness.steps.ts:461-700` — needs a design ruling, not a rename: it identifies one witness's bytes across the ark witness CID (dag-cbor + sha2), the pantry CID and the custody marker by comparing *digests*; under the byte profile the pantry/custody address is BLAKE3 of the bytes and the witness record CID is sha2 of the record bytes, so no digest is shared. The station must compare bytes (or the byte CID of the witness bytes), and depends on T11 ark's witness addressing. Left for T9/T11.
- The rest of a2o (367 lint hits above) is T9's.
- `rakia` gitlink f9787c9e58 is local only (T5); pin attestation refuses until it reaches an upstream run-ci branch.
- Host-toolchain caveat: every Rust EXIT above is rustc 1.99.1 (host stable); the gates pin 1.96.1. T6 reported 1.99.1 clippy findings that 1.96.1 does not raise.
- Seeder `isByteCid` (`genesis/seeder/src/byte-cid.ts`) and household-chaos/delivery step admission are regex/prefix + header checks, not the kernel's full decode (plan §2.5.2); acceptable for a seeder/test harness, recorded.
- The DNA (`regenesis/dna`) was not on origin when checked; the seeder manifest payload and T1 (a)–(f) were verified against storage's `sharding.rs` and the orchestrator's list, not the zome.
- Commit attribution UNMEASURED on every L2 commit.

