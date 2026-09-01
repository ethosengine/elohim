---
id: "backlog-task-release-manifest-schema-packager"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: release-manifest schema (rakia v1) + epr-release packager — package a coordinator bundle / config EPR / storage binary as blob(s) + a validated release manifest"
slug: "task-release-manifest-schema-packager"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
claimedBy: "claude-opus-t1"
priority: "high"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-channel-ceremony-driver"
  - "backlog-task-release-adoption-controller-observe"
tags: [upgrade-propagation, rung5, release-manifest, rakia, schema, packaging, delegable]
---

**Claimable by any implementation agent. Foundation atom of the rung-5 family
(spec: `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md`
§5); no dependency on any sibling — T2/T3 consume this task's outputs.**

## Why

A release must be an EPR object on the same dataplane as content — blob bytes
plus a validated, content-addressed manifest — so all substrate primitives
(resiliency, replication, reach) come along for free. Nothing can be elected,
adopted, or attested until the artifact has this shape.

## P2P design-gate decision

Carried by the spec §5 (ReleaseManifest entity): Notarized (A) reusing the
existing `Content` entry type, DNA-hash-NEUTRAL via the `metadata_json` valve;
content-derived CID; iroh affinity for bytes; head-plane cost = channels only.
This task builds the SCHEMA and the local PACKAGER — it authors nothing to the
DHT (that is T2's ceremony driver).

## Scope

1. `elohim/rakia/schemas/v1/release-manifest.schema.json` — JSON Schema for
   the manifest body (spec §5): `kind: "release-manifest"`, `channelId`
   (`runtime:<artifact-class>:<network>:<channel-name>`), `artifactClass`
   (`coordinator-bundle | config-epr | storage-binary | happ-bundle`),
   `artifacts[] {blobCid, bytes, sha256, filename}`, `appliesTo.roles{<role>:
   {dnaHash, coordinatorWasmHashes[]}}`, `envelope {wireEpochs[],
   lineageParentCid|null, additiveOnly}`, `provenance {builderAgent,
   toolchain, buildInfo, builtFrom.gitCommit}`, `declaredReach`,
   `adoptionDiscipline {soakSecs, attestationThreshold, canaryOrder[]}`.
   Follow the conventions of the existing `elohim/rakia/schemas/v1/` files.
2. `genesis/a2o/scripts/epr-release-package.ts` (tsx) — given an artifact file
   (a repacked `.happ`, a config JSON, or the debug `elohim-storage` binary)
   plus channel/envelope args: PUT the blob(s) to a target storage peer's blob
   route, compute CIDs, read `appliesTo` from the artifact itself where
   derivable (for a `.happ`: unpack and hash roles the way
   `happ_manager.rs::bundle_dna_hashes` does — port the derivation, do not
   guess), fill provenance from `--build-info`/git, validate against the
   schema, and write the manifest JSON to stdout/file. No DHT authoring.
3. Schema-validate fixtures for all four artifact classes under
   `genesis/a2o/fixtures/` (or the a2o-conventional fixture home).

## Interface contract (consumed by T2, T3)

- The manifest JSON validated by (1) is the ONLY currency between packager,
  ceremony driver, and controller. Field names above are normative; extend
  only additively.
- The blob(s) must be fetchable by CID from the peer they were PUT to before
  the packager exits 0 (round-trip check).

## Disjointness contract

- MAY create the schema file, the packager script, fixtures, and edit this
  atom.
- MUST NOT edit Rust source, any zome, `hc-mesh.sh`, sibling task scripts, or
  author/declare anything on the DHT.

## DoD + verification

- Packager exits 0 for all four artifact classes against a running `just dev
  start` or mesh peer; emitted manifests validate against the schema; blob
  round-trip proven in the transcript.
- A deliberately envelope-broken input (wrong role hash) still packages —
  packaging is not verification (that is the controller's floor) — but the
  manifest records what the artifact actually is.

## Implementation notes (2026-09-01)

Landed in the working tree, uncommitted — the integrator commits path-limited.
`status` stays `open` until then.

**What landed**

| File | What |
|---|---|
| `elohim/rakia/schemas/v1/release-manifest.schema.json` | The ReleaseManifest schema (scope 1) |
| `genesis/a2o/scripts/epr-release-package.ts` | The packager + a `--validate` mode (scope 2) |
| `genesis/a2o/scripts/__tests__/fixtures/release-manifest-{coordinator-bundle,config-epr,storage-binary,happ-bundle,envelope-broken}.json` | Emitted fixtures, all four classes + the DoD's broken-envelope case (scope 3) |

**The `.happ` `appliesTo` derivation — the documented fallback was taken.**
`happ_manager::bundle_dna_hashes` unpacks the bundle and calls
`AppBundle::resolve_cells`, so reproducing it in TypeScript means reproducing
Holochain's `DnaDef` serialization and blake2b hashing *byte-exactly* — a second
implementation of a consensus-critical hash, in a workspace with neither a
msgpack nor a blake2b dependency, and with no way to detect its own drift.
Adding those deps would buy a number we already have from an authoritative
source. So the packager reads `appliesTo` from a running peer's
`GET /version` `passport.happ.roles` (`--applies-to-from <url>`), which reports
exactly the hashes `bundle_dna_hashes` resolves, with `--applies-to <json|@file>`
for the offline/deliberate case. `coordinatorWasmHashes` is the normative sorted
array; the passport's zome→hash map is preserved additively as `coordinatorZomes`
so no information is dropped.

**Schema is deliberately OPEN** (`additionalProperties` unset), diverging from
`build-manifest`/`build-plan`'s closed style. Those are repo-local and
single-version; a release manifest crosses the wire between mixed-version peers,
where §8.2's additive floor requires consumers to tolerate fields they do not
know. Typo-detection is recovered as a *producer-side* lint: `--strict` walks the
schema and reports every unnamed property (proven below), rather than closing the
contract and fragmenting it later — the same reasoning `build-manifest.schema.json`
already records for its extensible `buildExecutor.kind`.

**Verification (all commands from `genesis/a2o`)**

- Four classes, each exiting 0 against the already-running household mesh peer
  `http://localhost:8090` (matthew), each with the blob round-trip proven — PUT
  the bytes, GET them back by the address the store returned, compare sha256:
  - `config-epr` — 127 B, cid `bafkreigx5jdmfj3ob6b3mqfcsq5higrp2jgdxczyjpy2mtf7pwgzkqzubq`, PUT 201, GET 127 B matching
  - `coordinator-bundle` — `content_store.wasm` 13 558 512 B, cid `bafkreih55t4vyzozvjgu7b62baj5oe2oyrkshcgw4yq7oamvmo3eainmsa`, PUT 201, GET matching
  - `happ-bundle` — `elohim.happ` 10 897 977 B, cid `bafkreie7bmyi6srdkummlem5rmxh4nqfg4suwdxkkppwa5yvvyy2dcrvie`, PUT 201, GET matching
  - `storage-binary` — the live debug `elohim-storage` 381 123 176 B, cid `bafkreidybjt3ccgs6i5up43vegwm2afnstdwyvt2dgnz6tn6n4h667wsiu`, PUT 201, GET matching, 2 m 45 s
- The CID construction was cross-checked against an independent
  `hashlib`/`base32` computation — byte-identical, so the `bafkrei…` form matches
  the `Cid::new_v1(0x55, Sha2_256(bytes))` canon rather than being self-consistent
  only with itself.
- `--strict --validate scripts/__tests__/fixtures/release-manifest-*.json` →
  `5/5 manifests validate`, exit 0.
- Negative cases all refuse with exit 2: a legacy `sha256-<hex>` in `blobCid` and
  an off-vocabulary reach; a typo'd `adoptionDisciplin` (caught only by
  `--strict`, exactly the gap the open schema opens); an empty `wireEpochs` and a
  malformed channel id. An unreachable peer reports `could not reach <url>` and
  exit 2 rather than a stack trace.
- DoD second bullet: a manifest whose `appliesTo.roles.lamad` names a DNA hash no
  peer runs **packages and exits 0**
  (`release-manifest-envelope-broken.json`) — packaging is not verification.
- a2o gate legs — `pnpm run lint`, `format:check`, `typecheck` — are clean *on
  these files* (0 hits for `epr-release-package` / `release-manifest`). The legs
  themselves are pre-existing red across the package (253 lint errors, 12
  unformatted scripts including the sibling `release-ceremony.ts` /
  `release-attestation-probe.ts`, and 2 `auth.steps.ts` typecheck errors).

**Decisions worth knowing**

- *Fixture home.* Placed under `scripts/__tests__/fixtures/`, not a new
  `genesis/a2o/fixtures/`, because `genesis/a2o/.epr-meta`'s
  `new-feature-subdir-needs-meta` rule requires a new a2o subdirectory to be born
  with its own `.epr-meta`, and authoring governance is outside this task's
  write-set. The atom's "or the a2o-conventional fixture home" clause covers it.
- *Fixtures are prettier-formatted, the packager is not.* `format:check` is a leg
  of `_gate-genesis-a2o`, and prettier collapses short arrays that
  `JSON.stringify(…, 2)` expands. A regenerated fixture needs
  `prettier --write` before check-in. Everything else the packager emits is
  reproducible: two runs 20 minutes apart differed in exactly one byte-range —
  `builtFrom.gitCommit`, because HEAD moved under this shared worktree.
- *No Rust, no zome, no DHT authoring, no mesh process started/stopped.* The mesh
  was already running and was used only for content-addressed, additive blob
  PUT/GETs. Those writes live in `/tmp/elohim-local-mesh/matthew/{blobs,blobs_iroh}`,
  which grew 305 M → 1.3 G — transient mesh state, recreated on the next mesh
  start, prunable at will.
- *Sibling contract check.* `scripts/release-ceremony.ts` (T2) currently reads a
  manifest duck-typed on `channelId` and says in its own comment that T1's schema
  had not landed. It validates channel ids against `^runtime:[^:]+:[^:]+:[^:]+$`,
  a strict superset of this schema's pattern, so every manifest this packager
  emits satisfies it.

**Stations left (outside this task's write-set)**

0. **The schema lands in a submodule — two commits, not one.**
   `elohim/rakia` is a git submodule (`github.com/ethosengine/rakia.git`, no
   `update = none`), so `release-manifest.schema.json` is invisible to a
   `git status` in the parent; the parent only shows ` M elohim/rakia`. The
   integrator commits inside `elohim/rakia` first, then bumps the pointer in the
   parent alongside the a2o files. (`git` refuses to run inside that submodule in
   this container — `detected dubious ownership` — and adding the `safe.directory`
   exception is a config change this task did not make.)
1. **Rust types for the controller.** Rakia's README forbids hand-written Rust
   mirroring a schema, but adding `release-manifest.schema.json` to
   `codegen-rs.mjs`'s explicit `SCHEMAS` list needs the generator to learn scalar
   `$defs` (it emits a struct for *every* `$defs` entry, so `blobCid`/`dnaHash`/
   `reach` would become empty structs). Every *object* `$def` here is already
   lifted the way the generator wants, and `lineageParentCid` uses the supported
   `["string","null"]` nullable idiom. Note the generator is **already red on
   `dev` for an unrelated reason**: `pnpm run rakia:codegen:rs:verify` fails with
   `inline nested objects must be lifted to $defs (path: GateProject.run)` — a
   pre-existing defect in `build-manifest.schema.json`, not caused by this work,
   and it must be fixed before this schema can join the list.
2. `elohim/rakia/schemas/README.md`'s schema table does not list this schema
   (rakia doc, outside the write-set).
3. No test wires the fixtures into `pnpm test:unit`; today they are checked by
   `epr-release-package.ts --strict --validate`. A `scripts/__tests__` case is a
   one-liner for whoever owns that file.

**Story-graph nodes discovered mid-flight**

- chain: release packaging → adoption / between: *packager emits `appliesTo`* →
  *controller verifies `appliesTo`* / missing node: **the artifact self-declares
  what it applies to.** Today the binding is read from a peer that already has
  the artifact installed, which cannot work for an artifact built but not yet
  installed anywhere. Assertion: *a `.happ` or coordinator bundle can be asked,
  offline, which roles and DNA hashes it binds to, by the same code the conductor
  uses.* Probe: a Rust-side `--emit-applies-to <bundle>` on an existing binary
  (elohim-storage or a small rakia tool) whose output the packager consumes
  verbatim. State: absent; the packager's peer-passport read is the standing
  substitute.
- chain: release provenance / between: *artifact is built* → *manifest records
  `builtFrom.gitCommit`* / missing node: **the commit the artifact was actually
  built from.** The packager probes HEAD, and on this shared worktree HEAD moved
  between two runs minutes apart — so `builtFrom.gitCommit` can name a commit the
  bytes were never built from. Assertion: *`builtFrom.gitCommit` equals the commit
  the artifact's own build stamped into it.* Probe: compare
  `provenance.buildInfo.commitFull` against `builtFrom.gitCommit` and refuse a
  mismatch. State: unenforced; today the two fields can disagree silently, and on
  the local mesh `buildInfo.commitFull` is `"unknown"`, so the check has nothing
  to bite on until build-info stamping reaches the dev binary.
- chain: compatibility envelope / between: *release declares `wireEpochs`* →
  *controller checks the envelope* / missing node: **a registry of what a wire
  epoch number means.** §8.1 names the axis and `p2p/sync_state.rs`'s `epoch` is a
  per-process *boot* epoch, not a protocol wire version — different concept, same
  word. Assertion: *epoch N names an enumerated wire contract a peer can claim to
  speak.* Probe: a declared epoch list the schema can validate against. State:
  absent; the schema types `wireEpochs` as non-negative integers and says in its
  description which epoch it is *not*.
