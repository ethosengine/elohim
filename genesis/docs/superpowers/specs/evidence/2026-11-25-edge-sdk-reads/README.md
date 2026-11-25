---
title: "What the Edge steps and the storage gate read of elohim/sdk, 2026-11-25"
id: edge-sdk-reads-evidence
status: draft
class: process-meta
date: 2026-11-25
context-tier: disclosed
steward: human:matthew
author: "Claude (Sonnet 5.5, research agent for session cacccb55), read-only evidence for backlog row 94; every claim re-read in the worktree at origin/dev 45637ca63"
serves: [push-gate-pays-once]
tags: [gate, edge, sdk, sources, evidence, proposal]
graduation-trigger: >-
  A deliberate Edge roll applies (or the steward rejects) the narrowed source list in Table 5, and this file
  becomes its appendix or is retired with it.
cites: []
---

# What the Edge steps and the storage gate read of elohim/sdk

Row 94 of `genesis/data/timeline/backlog/steward-device-arrival-open-items.md` (line 3076) records that one test
under `elohim/sdk/domains/elohim-agent/scripts/` made CI select an Edge build and made the lander run the storage gate
for about 100 minutes. This file measures what the storage build and its tests really read under `elohim/sdk/`, so a
later deliberate change can narrow the declaration. Nothing here is applied. No cargo was run and no manifest or code was
edited. CONFIRMED means I read the cited line; INFERRED means I concluded it from other lines.

## 1. Where `elohim/sdk` is declared as an input

There is no `elohim/elohim-storage/build-manifest.json`. The `elohim-storage` gate project lives in
`elohim/holochain/build-manifest.json` (CONFIRMED, lines 388-401): `dir: elohim/elohim-storage`, `steps: [cargo-build-storage]`,
recipe `just gate`. Its source list is therefore the sources of that one step. The same file holds the Edge steps.

Table 1. Edge manifest `elohim/holochain/build-manifest.json`, every step and its sdk globs (CONFIRMED).

| Step id (line) | sdk globs it declares | Note |
|---|---|---|
| cargo-build-doorway (13) | `elohim/sdk/domains/imagodei/types/**`, `elohim/sdk/domains/infrastructure/types/**` (37-38) | doorway-service path dependencies, Cargo.toml lines 187 and 190 |
| cargo-build-storage (67) | `elohim/sdk/**` (80) | the cause named in row 94 |
| build-doorway-app (141) | `elohim/sdk/storage-client-ts/**` (151), `elohim/sdk/epr-ts/**` (153) | a TypeScript build; out of scope here |
| build-agent-sdk (179) | none | sources are `elohim/elohim-agent/elohim-agent-sdk/**` |
| sdk-feature-matrix (204) | none | `crates/elohim-sdk/**`, a different "sdk" |
| build-edge-image (233) | none | sources `elohim/holochain/edgenode/**`, `elohim/holochain/*.yaml` (236-239) |
| deploy-manifests (262) | none | orchestrator manifests and CI scripts (265-274) |
| dataplane-validation (294) | none | a2o features and a script |

Finding: `build-edge-image` and `deploy-manifests` declare no sdk path. They are stale only because they `depends` on
`cargo-build-storage` (lines 251-256 and 283-287, CONFIRMED). One glob on one step produces all three stale steps in the
fleet-roll refusal. I found no project-level `watch` or `changesets` key in this manifest (INFERRED from grep of the
file for those words; no hit).

The gate project `epr-storage` (same file, line 404) is already narrow: `elohim/sdk/schemas/v1/views/epr-*.schema.json`,
`inputs/epr-*.schema.json`, `storage-client-ts/tests/epr*`, `storage-client-ts/src/generated/Epr*` (413-416) and the
`domains/lamad/types/**`, `domains/shefa/types/**` pair (439-440). It is the model for the proposal, not a target.

Other manifests that name `elohim/sdk` (CONFIRMED by `grep -rn "elohim/sdk" --include=build-manifest.json`):

| Manifest | Lines | Declared |
|---|---|---|
| `app/elohim-app/build-manifest.json` | 22, 111-114 | `elohim/sdk/**`; a gate dir `elohim/sdk` with `elohim/sdk/scripts/**` |
| `genesis/build-manifest.json` | 14-15 | `elohim/sdk/schemas/**`, `elohim/sdk/domains/**` |
| `elohim/holochain/dna/build-manifest.json` | 22, 71, 116, 151 | `domains/*/types/**`, `schemas/**` |
| `elohim/eprfs/build-manifest.json` | 30, 84, 102-112 | adapter, lamad manifest, seam-guidance files, two seam.json files |
| `elohim/epr/build-manifest.json`, `elohim/eprfs/eprfs-ts/build-manifest.json`, `elohim/sdk/storage-client-ts/build-manifest.json` | 38/111-117, 11, 10-15 | `epr-ts/**`, `storage-client-ts/**`, a few schema files |
| `elohim/placement`, `steward/device`, `bridges/cargo`, `bridges/jenkins`, `genesis/orchestrator` | various | narrow lists (enums, lamad/shefa types, `elohim-agent/adapter/**`, stasis schemas) |

Only `elohim/holochain/build-manifest.json` selects an Edge build. The others are listed for completeness; this proposal
does not touch them.

How the gate derives its closure (CONFIRMED): `genesis/orchestrator/gate-input-audit.mjs` `main` (near the end of the file)
takes the union of the gate's step `sources` as the declared closure. `readSet` (line 471) and `auditProject` (line 479)
scan only `.rs` files covered by those sources for named reads (`include_str!`, path joins, `// gate-reads:` comments) and
report any read the sources do not cover. `gate-runner.mjs` calls `readSet` at lines 826 and 1077. So the declared source
is the key; the audit can only prove a source is missing, never that one is extra.

[GAP: no recorded read list for `elohim-storage`. `grep -rn "gate-reads" elohim/elohim-storage` finds nothing (CONFIRMED),
and I found no per-project read-set file in `/projects/elohim/.eprfs/status/` (CONFIRMED by listing; I did not open
`flows.jsonl` or the stasis event files). The audit's output for this project was not run, because it is a node script I
judged outside "read-only research"; run `node genesis/orchestrator/gate-input-audit.mjs elohim-storage` to compare.]

## 2. What the storage build reads from `elohim/sdk/`

Table 2. Build-time reads (CONFIRMED unless marked).

| Mechanism | Path read | Evidence |
|---|---|---|
| Cargo path dependency | `elohim/sdk/domains/shefa/types` | `elohim/elohim-storage/Cargo.toml:93` |
| Cargo path dependency | `elohim/sdk/domains/lamad/types` (feature `json`) | `Cargo.toml:99`; also `elohim/elohim-facings/Cargo.toml:36` |
| `include_str!` | `sdk/domains/lamad/manifest/graph.json` | `src/graph_engine.rs:70` |
| `include_str!` | `sdk/domains/lamad/manifest/relationships.json` | `src/db/authored_edges.rs:157` |
| `include_str!` | `sdk/schemas/v1/manifests/bootstrap-standing-policy.json`, `bootstrap-tending-policy.json` | `src/services/bootstrap_manifests.rs:26,28` |
| `pillar_file!` embeds | `sdk/domains/{elohim,imagodei,infrastructure,lamad,mishpat,qahal,shefa}/manifest.json`, plus `lamad/manifest/observation-kinds.json` and other "referenced" files | `src/services/observation_kinds.rs:45-110` (the macro at 45-52; entries read through line ~110) |
| Run-time path, default only | `../sdk/domains/elohim/manifest.json`, overridable by `ELOHIM_MANIFEST_PATH` | `src/services/signal_weight_registry.rs:62`, `constitutional_ratio_registry.rs:83` |
| Run-time directory | `../sdk/domains`, overridable | `manifest_registry.rs:519`; `main.rs:3132` falls back to `elohim/sdk/domains` |
| ts-rs `export_to` | `elohim/sdk/storage-client-ts/src/generated/` | about 25 sites, for example `src/tally/mod.rs:34`, `src/services/routing.rs:28`, and `crates/elohim-views/src/*.rs`. These are WRITES at `cargo test export_bindings`, not reads. |

No `build.rs` exists in `elohim/elohim-storage` (CONFIRMED: `ls` fails). The workspace `elohim/Cargo.toml:31` lists
`sdk` under `exclude`, so the sdk is not a workspace member of storage.

Dockerfile `elohim/elohim-storage/Dockerfile` (CONFIRMED): lines 184-185 copy exactly the two types crates
(`elohim/sdk/domains/shefa/types`, `lamad/types`) to `/sdk/...` for the dependency stages. Line 521 (stage `check`, which
runs `cargo test --lib` at line 541) and line 611 (stage `builder`) each run `COPY elohim/sdk /sdk`, the whole tree. The
comment at 606-610 says why: the `include_str!` paths resolve to `/sdk/...`. The copy is broader than the reads. A narrower
`COPY` is a second edit this proposal does not make.

Jenkins and CI scripts (CONFIRMED): `scripts/ci/build-storage-image.sh`, `storage-input-digest.sh` and
`storage-quality-gate.sh` contain no `sdk/` path. `scripts/ci/closure-input-digest.sh` mentions only `build-agent-sdk`. In
`elohim/holochain/Jenkinsfile` the only "sdk" hits are `sdk-feature-matrix`, `build-agent-sdk` and `elohim-agent-sdk`
(lines 79, 463, 508, 604-619, 3118), none of which is `elohim/sdk`. So for `cargo-build-storage`, `build-edge-image` and
`deploy-manifests` no stage script reads `elohim/sdk` except through the Dockerfile copies above.
[GAP: `closure-input-digest.sh` computes the image closure key from the manifest sources (lines 32-65); I did not trace
whether that key also hashes Dockerfile COPY paths. Narrowing the manifest changes that key for every future build.]

## 3. What the storage tests read at run time

Table 3. Reads by `cargo test` under `elohim/elohim-storage` (CONFIRMED paths; the schema subdirectories come from the
greps named in the last column).

| Test file | sdk path read | Line |
|---|---|---|
| `tests/schema_contract.rs` | `schemas/v1/` via `schema_dir()`; subdirectories `objects`, `intents`, `manifest-payloads`, `views` (for example `views/mechanism-selection.schema.json`) | 41; 2642-6439 |
| `tests/api/schema_contract_recovery_v2.rs` | `schemas/v1/` | 19 |
| `tests/schema_contract_diesel_epr.rs` | `schemas/v1/` | 15 |
| `tests/api/qahal_http_contract.rs` | `schemas/v1/` | 66 |
| `tests/api/api_observations_stream_test.rs` | `schemas/v1/` | 125 |
| `tests/search/route.rs` | `schemas/v1/views/content-search-view.schema.json` | 478, 540 |
| `tests/api/reach_vocabulary_contract.rs` | `schemas/v1/enums/reach.schema.json` | 22 |
| `tests/graph/lamad_manifest_registration.rs` | `domains/lamad/manifest.json` | 61 |
| `tests/graph/shefa_manifest_registration.rs` | `domains/shefa/manifest.json` | 26 |
| `tests/observation/manager_resume_test.rs` | `domains/` (a directory) | 66 |
| `tests/rea/standing_extension_integration.rs` | doc comment names `domains/elohim/manifest.json` | 4 (INFERRED: a comment, not a read) |
| `src/services/observation_kinds.rs` (unit test) | `domains/` directory | 382 |
| `src/http.rs` (unit test) | `fixtures/spa-route-discrimination.vectors.json` | 21664 |
| `src/db/rea_commitments.rs` (unit test) | `fixtures/route-claims.vectors.json` | 3295 |
| `src/api/compute_grants.rs` (unit test) | `schemas/v1/commitments/delegates-compute.schema.json` | 635 |
| `src/services/content_signals.rs` (unit test) | `schemas/v1/feedback-signals/undeclared-behaviour.schema.json` | 861 |

Schema subdirectories of `schemas/v1/` named anywhere in storage `src` or `tests` (CONFIRMED by grep, counts of
mentions): `commitments`, `conductor-signing`, `dna-signals`, `enums`, `feedback-signals`, `intents`, `manifest`,
`manifests`, `manifest-payloads`, `objects`, `p2p`, `release-manifest`, `signal-intent`, `views`. The Docker `check`
stage runs `cargo test --lib` only (line 541), so the integration tests above run in the local `just gate`, not in that image stage.

## 4. Layout of `elohim/sdk/` and who reads each part

File counts are tracked files from `git ls-files` in the worktree (CONFIRMED). Verdict key: BUILD, TESTS, NOT READ.

Table 4. Immediate subdirectories and files.

| Path | Files | Verdict | Evidence |
|---|---|---|---|
| `domains/` | 160 | BUILD and TESTS, partly | broken down below |
| `schemas/` | 502 | BUILD and TESTS for `v1/` only | `schemas/v1/manifests/*` is embedded (Table 2); tests read `v1/` (Table 3). `current/`, `generated-ts/`, `generated-ttl/`, `scripts/`, `tests/`, `CLAUDE.md` NOT READ by anything I found |
| `fixtures/` | 2 | TESTS | two vector files (Table 3, `http.rs:21664`, `rea_commitments.rs:3295`) |
| `storage-client-ts/` | 792 | NOT READ (WRITE target) | ts-rs output only (Table 2) |
| `epr-ts/` | 49 | NOT READ | no storage reference found |
| `scripts/` | 6 | NOT READ | no storage reference found |
| `src/` | 15 | NOT READ | no storage reference found |
| `app-lifecycle.seam.json`, `working-version.seam.json` | 2 | NOT READ by storage | declared by the eprfs manifest (lines 111-112), not the storage step |
| `package.json`, `tsconfig.json`, `vitest.config.ts`, `README*.md`, `CLAUDE.md` | 6 | NOT READ | none found |

Table 4b. `domains/` per domain.

| Domain | Files | Verdict | Evidence |
|---|---|---|---|
| `lamad` | 59 | BUILD and TESTS | `types` crate, `manifest.json`, `manifest/graph.json`, `relationships.json`, `observation-kinds.json`. Other files in `lamad/manifest/` (attestations, content-formats, content-types, gates, observations, rendering, signals) not named: NOT READ found |
| `shefa` | 11 | BUILD and TESTS | `types` crate (`Cargo.toml:93`), `manifest.json` |
| `elohim` | 2 | BUILD and TESTS | `manifest.json` |
| `imagodei` | 8 | BUILD | `manifest.json` embedded (observation_kinds); `types` is a doorway dependency, not storage |
| `infrastructure` | 4 | BUILD | `manifest.json` embedded |
| `qahal` | 12 | BUILD | `manifest.json` embedded; its `types` crate is not a storage dependency |
| `mishpat` | 2 | BUILD | `manifest.json` embedded |
| `avodah` | 8 | TESTS at most | no embed; the directory-walking tests (`observation_kinds.rs:382`, `manager_resume_test.rs:66`) may read its `manifest.json` |
| `elohim-agent` | 52 | NOT READ, except possibly `manifest.json` | its `adapter` is an eprfs workspace member (`elohim/eprfs/Cargo.toml:16`), not a storage dependency; the Row 94 test is `scripts/brit-first.test.py` |

[GAP: the "referenced" file lists inside `EMBEDDED_PILLARS` (observation_kinds.rs, after line 110) name more files than
I enumerated; each one is under a domain already listed as BUILD. Whether `schemas/v1/` subdirectories such as `inputs/`,
`zome-inputs/`, `registries/`, `attestation/`, `agent/`, `economic-events/`, `frames/` are read by a helper I did not grep is
not established; the `schema_dir()` joins I counted are the ones that appeared literally.]

## 5. Proposal, not applied

Narrowed source lists. "Stops selecting" is what a change under that path would no longer cause. Applying this edits
`elohim/holochain/build-manifest.json`, which is itself an Edge input (it is in the Edge pipeline's own build process), so
the edit belongs with a deliberate roll carrying `[edge:roll]`. Package projections of the agent hooks live under
`.claude/` and `elohim/sdk/domains/elohim-agent/`; they are named in the last column where they fall out.

| Step or gate | Today | Proposed | Stops selecting | Proof still missing |
|---|---|---|---|---|
| `cargo-build-storage` step (line 80) and therefore the `elohim-storage` gate | `elohim/sdk/**` | `elohim/sdk/domains/*/manifest.json`, `elohim/sdk/domains/lamad/manifest/**`, `elohim/sdk/domains/lamad/types/**`, `elohim/sdk/domains/shefa/types/**`, `elohim/sdk/schemas/v1/**`, `elohim/sdk/fixtures/**` | `elohim/sdk/domains/elohim-agent/**` (scripts, tests, adapter, schemas, CLAUDE.md), `domains/*/scripts/**`, `domains/*/CLAUDE.md`, `domains/avodah|imagodei|infrastructure|qahal/types/**`, `schemas/{current,generated-ts,generated-ttl,scripts,tests}/**`, `storage-client-ts/**`, `epr-ts/**`, `scripts/**`, `src/**`, top-level seam and config files; also elohim-agent package projections that only touch those paths | see items 1-5 below |
| `build-edge-image` | none of its own; stale through `depends` | no edit | selected only if `cargo-build-storage` is | none; it follows the storage step |
| `deploy-manifests` | none of its own; stale through `depends` | no edit | same | none |
| `cargo-build-doorway` (37-38) | two `types/**` globs | no edit | n/a | not in scope |
| Dockerfile `COPY elohim/sdk /sdk` (521, 611) | whole tree | optional: copy the proposed paths only | n/a | the `include_str!` and Cargo paths must still resolve; untested |
| `epr-storage` gate | already narrow | no edit | n/a | none |

Proof still missing before anyone applies the first row:
1. [GAP] Run `node genesis/orchestrator/gate-input-audit.mjs elohim-storage` against the proposed list; it must report no
   undeclared Rust reads. I did not run it.
2. [GAP] Directory-valued reads at run time: `main.rs:3132`, `manifest_registry.rs:519`, and the two directory-walking
   tests read `../sdk/domains` whole. The proposed `domains/*/manifest.json` covers what I could see them open; if any loader
   also opens `domains/*/schemas/**` or `lamad/schemas/**`, the list is too narrow. `ELOHIM_MANIFEST_PATH` and
   any pillar-directory override take paths at run time that a source list cannot see.
3. [GAP] The `types` crates (`lamad`, `shefa`) have their own path dependencies and `Cargo.lock`; I did not trace
   whether either reaches another sdk directory. The existing `epr-storage` gate lists them under `types/**` only.
4. [GAP] The `schemas/v1/**` entry is deliberately whole, because I could not prove which of its subdirectories the
   tests read. A later pass can narrow it to the fourteen directories in Table 3's last paragraph.
5. [GAP] Other crates in the storage closure (`crates/**`, `bridges/**`, `elohim/epr/**`) were only grepped for `include_str!`
   reaching sdk, with no hits other than ts-rs writes; I did not read their tests. Likewise the `epr-storage` gate
   and the `schema-dna` and `elohim-placement` gates select on their own sdk globs and are unaffected.
6. A two-step check once applied: the edit must not drop `elohim/sdk/domains/lamad/manifest/graph.json` or the two
   bootstrap policy files, which break the compile (Dockerfile comment lines 606-610 records that failure).
7. The first push after the edit selects the Edge pipeline by design; do it in the roll, not before.
