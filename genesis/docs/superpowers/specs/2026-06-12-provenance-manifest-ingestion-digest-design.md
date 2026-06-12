---
id: provenance-manifest-ingestion-digest-design
cites:
  - genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md
  - resilience-dimensions-proof-suite | D1/D2 boundary tests this digest extends with the digested-content measured case; workstream D @wip rows remain the multi-peer acceptance gate | sha256:a89f58ec4906e152 | path: genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md
---

# Provenance Manifest v1 + Ingestion Digest — Layer 2 distribution-plane design

> **⚠ SUPERSEDED (2026-06-12 evening, operator scope cut).** The historical-provenance
> half designed below — the Provenance Manifest v1 artifact, the genesis sealer CLI, the
> ingestion digest that replays declared git-era lineage, the `attestation:seed-bootstrap`
> "declared-not-witnessed" discriminator, and brit as stage-2 author — is **dropped
> entirely**. The operator's cut: **seeding = init.** When content is written to the
> network, that IS its birth; there is no declared past to digest, and brit drops off the
> critical path (history, if ever needed, grafts on later as an enrichment attestation).
> The live design is now **`2026-06-12-init-authoring-native-seeding-design.md`** — a
> conductor-bearing agent authors content through the real `create_content` front door
> (anchored, witnessed, signal-projected, stocked natively), and the real remaining work is
> the relationship design (author standing · author-steward · custody vs. projection
> replication · peers-per-reach). This file is kept for the design-history record only; do
> not implement from it.

**Date:** 2026-06-12
**Status:** SUPERSEDED by `2026-06-12-init-authoring-native-seeding-design.md` (operator scope cut, same day)
**Owner surfaces:** `elohim/sdk/schemas/v1/objects/provenance-manifest.schema.json` (new),
genesis sealer CLI (new, cite-gen-shaped), seeder digest pass (new), brit stage-2 seam
**Origin:** part 2 of `genesis/data/timeline/backlog/resilience-unmeasured-vs-zero-honest-denominators.md`
(operator design seed: "EPR CLI as git porcelain over the real CRUD gates; seeding = replaying
the lifecycle, not backfilling manifests"; routed to brit 2026-06-12)

## Problem

Bulk-seeded content arrives on the substrate **stateless**: bytes and fields, no lineage.
The receiving elohim-storage cannot know whose content it is, at what reach, or how that
reach was earned — so every seeded item renders the degenerate resilience shape. Part 1
(shipped 2026-06-12) made that honest (`distributionState: "unmeasured"`); this design
makes it *real*: seeded content carries a declared, signed, commit-graph-shaped history
the DHT can digest, and the digest replays that history through the substrate's real
write paths.

Hand-rolling per-item state across ~3,500 seed items is not viable. The state must be
**derived** by a tool (the citation-sealer pattern) and **digested** by replay — never
backfilled by writing projections directly.

## Decision: approach C — the artifact is the contract; authors are pluggable

One wire artifact (Provenance Manifest v1) + one digest pass, designed together with
brit, shipped separately:

- **Stage 1 author:** a genesis sealer CLI derives manifests from seed fixtures + git
  history. No brit dependency at runtime.
- **Stage 2 author:** brit's elohim-protocol schema emits the *same artifact* from real
  commit trailers/notes (`refs/notes/brit/`, brit phases 0+1/2a primitives). The stage-1
  golden corpus becomes brit's conformance fixtures.
- **Stage 3 (out of scope here):** live `epr push` makes manifest authoring continuous
  rather than batch. Lives on brit's roadmap.

Nothing in stage 1 is throwaway: the schema, the CID math, the digest, and the golden
fixtures all carry forward.

## P2P design gate (passed 2026-06-12)

No new DHT entry types, no new tables, no new substrate wire formats. Summary:

| Entity | Classification | Source of truth |
|---|---|---|
| ProvenanceManifest | Input artifact (not a substrate entity) | the git/brit repo |
| Seed-bootstrap attestation | Notarized (A) — existing consolidated-attestation entry type, new `attestation:seed-bootstrap` discriminator string | Holochain DHT |
| Content anchor | Notarized (A) — existing `Content` entry; digest heals the NULL `dht_anchor_hash` of bulk-seeded rows | Holochain DHT |
| Stewardship commitments | Notarized (A) — existing `Mishpat::Commitment` (custody-blob / provide) | Holochain DHT |
| `shard_manifests` | Operational (C) — existing table, authored ONLY by real shard-encode/stock | SQLite (measured reality) |

Constraints carried out of the gate:

- **Declared vs measured stays two-layer.** The manifest never declares distribution
  state. `distributionState: "measured"` flips only when bytes actually shard-encode.
- **Workstream-D junctions gate the counts.** `humans.household_id` and collective
  regions are substrate-owned with no create surface (Epic B). The digest *consumes*
  them; it does not invent a write path for them.
- **Trust model:** everything the digest writes is signed by the trusted issuer
  (genesis sealer key) — the SDK gospel's bootstrap-until-qahal-grades, made explicit
  and auditable per item.

## §1 The artifact: Provenance Manifest v1

One manifest per content item, keyed by `(contentId, contentHash)`.

**Addressing (EPR-canonical):** canonical **dag-cbor** bytes (strict canonical form,
enforced by `elohim/epr/src/cbor.rs`), CID = CIDv1, codec 0x71 (dag-cbor), multihash
sha2-256, minted via the existing `elohim-epr` cid utilities (`bafyrei…`). Seed files
carry the manifest as **dag-json** (textual twin; `serde_ipld_dagjson` already a dep).
The trusted-issuer signature is over the canonical dag-cbor bytes. The `subject`'s blob
hash stays `sha256-<hex>` because that field *refers to* the blob layer.

This matches brit's approved design exactly ("CID over DAG-CBOR canonical
serialization" for every ContentNode): stage-1 and stage-2 authors share both the
schema and the addressing math.

**Schema home:** `elohim/sdk/schemas/v1/objects/provenance-manifest.schema.json`
(protocol layer; validated + codegen'd like every other schema; brit references the
same file per its schema-driven-development principle).

**Body (declared history only):**

| Field | Content |
|---|---|
| `subject` | contentId, blob hash (`sha256-<hex>`), blob CID where present, contentType |
| `lineage` | ordered events: `authored {persona, at, gitCommit?}` → `revised […]` → `published {reach, earnedVia}` → `supersedes?` — the commit-graph summary; stage 2 backs each event with real brit trailer/note refs |
| `reach` | final reach + earning chain (genesis bootstrap: mostly `trusted-issuer-grant` rationale, with pointers to three-leg evidence where it exists) |
| `stewardship` | declared stewards: `{householdId, personaIds, action: custody-blob \| provide, scope: content:<reach>}` |
| `issuer` | trusted-issuer key id + signature (over canonical dag-cbor) |
| `manifestVersion` | `1` |

## §2 Stage-1 author: the genesis sealer CLI

A cite-gen-shaped tool (working name `provenance-seal`) living beside the seeder/cite
tooling. It **derives** manifests:

- **Inputs:** seed JSON corpus; humans/households/deployments fixtures; `git log` of
  the content sources.
- **Output:** a `provenance` block embedded in each seed item (dag-json) — travels
  through the existing pipeline unchanged; no sidecar files.
- **Signing:** trusted-issuer key — fixture key on the local stack, operator-held for
  alpha runs.
- **Idempotency:** re-runs touch only items whose content hash changed; output is
  byte-stable for unchanged inputs (asserted by golden tests).

## §3 The ingestion digest

A post-import pass driven by the seeder — replay through existing surfaces, additive to
the bulk import (NOT a seeder replacement). Per item with a valid `provenance` block
(schema + canonical form + signature + CID all verified first):

1. **Anchor.** If the content row's `dht_anchor_hash` is NULL (every bulk-seeded item
   today), heal through the existing `update_via_conductor → create_content` re-publish
   path. Kills the known bulk-seed anchor gap as a side effect.
2. **Bootstrap attestation.** Call the consolidated-attestation coordinator with kind
   `attestation:seed-bootstrap`: subject content CID, manifest CID, declared reach +
   earning rationale, issuer signature. The substrate's auditable record that this
   history was *declared by the trusted issuer*, not witnessed live.
3. **Stewardship commitments.** For each declared steward, replay through the existing
   Mishpat commitment path. Two known traps designed in: commitments reach **`active`**
   via the real conductor transition (HTTP POST inserts `proposed` — never hand-edit
   state); the commitment **CID is the entry_hash**, never the action_hash.
4. **Stock.** For items with blobs, trigger the same shard-encode + `upsert_manifest`
   step the front-door POST route already runs, against the already-uploaded blob. The
   only step that touches the distribution plane — and it does so by doing the work,
   not declaring it. (Small storage surface to invoke the existing encoder for an
   existing blob; named in the implementation plan, designed last per the gate.)

**Idempotency:** every step keys on content hash + manifest CID. Re-running is a no-op
unless the manifest changed (attestation supersedes; commitments reconcile; anchor and
stock skip when present).

**Failure isolation:** one bad manifest skips that item with a logged reason — never
aborts the corpus (the EprRouter poisoned-row lesson applied).

**Named gap (not papered):** until Epic B lands an ingestion path for the
substrate-owned junctions (`humans.household_id`, collective regions), declared stewards
light commitments and attestations, and household-count joins light only where junctions
already exist (the matthew/jessica/james fixtures).

## §4 The declared-vs-measured line

After a full digest run:

- **Lights from declared-and-attested state:** `stewardingCollectives`,
  `commitmentBackedCollectives`, reach provenance, diversity inputs.
- **Lights only from measured reality:** `distributionState: "measured"` (step 4's real
  shard-encode on the ingesting node); regional spread and live-peer numbers (actual
  peers holding actual shards).

The demo card goes from "not yet distributed" to *real small numbers* (one stewarding
node, measured), and grows as the substrate actually replicates. Part 1's
unmeasured-honesty work is what makes this incremental staging safe to ship.

## §5 Testing & error handling

- **Schema/canonical:** manifest schema-contract test (Rust ↔ JSON Schema, same harness
  as views); dag-json → dag-cbor → CID round-trip stable; non-canonical input rejected;
  signature verify/reject.
- **Sealer goldens:** fixture genesis items with known git history → expected
  manifests, byte-stable across runs. These goldens ARE brit's stage-2 conformance
  fixtures: brit's author must emit byte-identical manifests for the same declared
  history.
- **Digest (local stack):** each step idempotent (run twice → identical substrate
  state); D1/D2 boundary tests extended with a digested-content case asserting
  `measured` + nonzero declared counts; per-item failure isolation test.
- **A2O:** one scenario — *seeded content carries its history* (seed → digest →
  resilience snapshot reads measured with attested provenance); one regression row —
  unsigned/tampered manifest rejected.

## §6 Staging & the brit seam

- **Stage 1a (local stack):** sealer + digest over the demo set (elohim-host-landing +
  neighbors), end-to-end.
- **Stage 1b (local stack):** full corpus — throughput + idempotency at scale. Bulk
  import remains in place; the digest is additive.
- **Alpha runs are operator ceremonies:** trusted-issuer key is operator-held; the
  standing agent rail excludes bulk writes to shared alpha.
- **Stage 2 (brit):** brit's elohim-protocol schema maps trailers/notes → Provenance
  Manifest v1; a brit verb emits the same dag-cbor artifact from real commit history;
  the sealer becomes a fallback. Lives on brit's roadmap as the phase-2 pull-through
  ("git artifacts become protocol content") with the golden corpus as its acceptance
  gate.
- **Stage 3 (out of scope):** live `epr push` — authoring-time manifests, no batch
  sealing.
- **Sequencing rail:** digest verification on live substrate waits for Layer 1's
  custody-convergence green; until then everything proves on the local stack.

## Out of scope

- Replacing the bulk import pipeline (explicit operator decision: the digest is
  additive).
- The full brit git-porcelain verb set (`epr commit|push|log|tag|stock`) — brit roadmap.
- Epic B junction ingestion (named dependency, separate work).
- Multi-peer replication mechanics (workstream D; the matrix feature's @wip rows remain
  its acceptance gate).
