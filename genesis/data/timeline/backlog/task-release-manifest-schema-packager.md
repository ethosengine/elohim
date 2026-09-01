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
