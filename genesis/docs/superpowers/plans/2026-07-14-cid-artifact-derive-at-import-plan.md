---
id: cid-artifact-derive-at-import
title: Close the CID-Addressed-Artifact Drift Seam — Derive at Import, Don't Store
status: Draft
created: 2026-07-14
class: architecture
artifact_kind: plan
domain: D8
sprint: refinement-stability
requires_env: [household-nodes]
cites:
  - cite-fingerprint-cid-convergence | Cite Fingerprint ↔ Canonical CID Convergence | sha256:0a657c9c1b0c43e7 | path: genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md
  - genesis/docs/superpowers/specs/2026-06-27-epr-meta-kinship-lineage-reconciliation-design.md
  - deterministic-reach-archetype-floor-design | Deterministic Reach-Archetype Floor | sha256:a2ee1687a1759a0f | path: genesis/docs/superpowers/specs/2026-06-10-deterministic-reach-archetype-floor-design.md
  - semantic-computable-links-design | Semantic-Computable Links | sha256:1460bc102580ab0d | path: genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
  - genesis/seeder/src/cid-artifact.ts
  - genesis/data/lamad/content/manifesto.json
---

<!--
  intended-cites (cite-gen --seal stamps sha256 + path):
    cite-fingerprint-cid-convergence        -> genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md
    epr-meta-kinship-lineage-reconciliation -> genesis/docs/superpowers/specs/2026-06-27-epr-meta-kinship-lineage-reconciliation-design.md
    deterministic-reach-archetype-floor     -> genesis/docs/superpowers/specs/2026-06-10-deterministic-reach-archetype-floor-design.md
    semantic-computable-links               -> genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
-->

# Close the CID-Addressed-Artifact Drift Seam — Derive at Import, Don't Store

**Discovery note.** Lexical prior-art surfacing (spec-coherence-index) returned strong matches; the
semantic lens (MemPalace) was **unavailable in this session — degraded to lexical-only**. The plan is
composed from the lexical seeds below.

## The seam

A content node in `genesis/data/lamad/content/*.json` that carries a `blobHash` + a `sourcePath` `.md`
is a **CID-addressed artifact**: at seed time the source `.md` is uploaded to `/blob/<hash>` and imported
into the protocol runtime, addressed by `blobCid`. Three of its fields are **derived from the source**,
not free-authored: `content` (frontmatter-stripped body), `blobHash` (`sha256(file)`), `blobCid`
(`CIDv1-raw-sha256(file)`).

This session, `manifesto.json` silently drifted from `manifesto.md` (edited under `[skip ci]`) — all
three fields stale — producing a live `/blob/<hash>` 404. We shipped **rung 1 (remediation)**: a push-time
guard (`genesis/seeder/src/cid-artifact.ts` derivation + `cid-artifact-integrity.spec.ts` attestation +
`pnpm run sync:cid-artifacts` fix). That catches drift; it does not close the seam.

**Framing (operator).** dev is the free authoring space; pushing to the shared repo/runtime is a
*reach-earning attestation at the deterministic floor* (Values Forward Stance II.1 + III.2, dogfooded).
See [[project_reach_earned_push_deterministic_floor]]. This plan is the *lowest rung* of that floor.

## The decision — converge, don't guard

The composition changes the shape of the close. **The canonical CID derivation already exists**, specced
and golden-vector-tested, in [cite-fingerprint-cid-convergence](epr:cite-fingerprint-cid-convergence):

> `body_cid = CIDv1(raw 0x55, sha2-256(canonical_body_bytes))` — implemented in
> `elohim/eprfs/eprfs-core/src/address.rs` (`BlobCid::compute_raw`) and `elohim/brit/brit-epr/src/engine/cid.rs`.
> The cite fingerprint is the 16-hex short-form of the *same digest*. One digest, two renderings.

So rung 1's `cid-artifact.ts` **re-implemented in TypeScript** (ad-hoc base32) a derivation that is
canonical in Rust. The real close is not another guard — it is to **stop hand-storing a derivable value**
and **derive it at import from the one canonical derivation**. A CID derived from content is correct *by
construction*; the drift class disappears rather than being policed. This is the deterministic floor doing
its own enforcement (content-addressing's whole point), which is exactly what
[deterministic-reach-archetype-floor](epr:deterministic-reach-archetype-floor) names as an *earned-reach
compiler invariant*.

**Sequencing decision:** pursue **rung 3 (derive-at-import) as the seam-closer**; **do not build rung 2**
(the edit-time `.epr-meta` prevention rule) — it would guard a drift class that rung 3 *deletes*. Rung 2 is
captured as a **conditional backlog item**, to be built only if rung 3 stalls. Rung 1's guard stays as the
transitional attestation and **simplifies** at Phase 4 (from "stored == derived" to "source does not
hand-store derivable fields").

## Prior art composed (compose, don't fork)

- [cite-fingerprint-cid-convergence](epr:cite-fingerprint-cid-convergence) — **the canonical derivation home**
  (`BlobCid::compute_raw`, raw `0x55`, golden vectors). Phase 1 converges the seeder onto it.
- [epr-meta-kinship-lineage-reconciliation](epr:epr-meta-kinship-lineage-reconciliation) — the `.epr-meta`
  compose-gate mechanism rung 2 *would* use; referenced only for the deferred conditional item.
- [deterministic-reach-archetype-floor](epr:deterministic-reach-archetype-floor) — the floor framing this
  plan implements at the content-artifact layer.
- [semantic-computable-links](epr:semantic-computable-links) — the cite-gen content-addressing pattern the
  fingerprint short-form descends from.

## Scope

- **In scope:** the seeder import/build path for CID-addressed content artifacts; the shared derivation in
  `genesis/seeder/src/cid-artifact.ts`; the view schema fields for blob-backed content; the guard test.
- **Testable on:** `household-nodes` / locally (seed + `/blob` resolve). Not env-blocked.
- **Out of scope:** the `integ/dev-merge → dev` branch reconciliation (separate integrator merge); rung 2
  edit-time `.epr-meta` (deferred conditional); any change to how `/blob` *stores* bytes (doorway serve path
  is read-only here).
- **Only one CID-addressed artifact exists today** (`manifesto.json`); constitution/confession/theology are
  inline-content. The plan is class-level so future blob-backed docs inherit the floor.

## Phase 0 — p2p-design-gate + storage decision

Invoke `p2p-design-gate` for the entity "CID-addressed content artifact address production." Decide the
storage question and record the answer:

- **(a) Omit** `content`/`blobHash`/`blobCid` from the *source* seed JSON entirely — the JSON is curation +
  `sourcePath` only; the seeder derives the three at seed time. **(recommended — nothing derivable is stored,
  so nothing can drift.)**
- **(b) Keep** them as a build-generated projection (`build:data` style, "DO NOT EDIT — regenerate" marker)
  gated by a freshness check.

**Verify:** gate answers recorded in the plan; the recommended (a) confirmed against what doorway + the view
schema actually read (Phase 3 must not break on a missing field).

## Phase 1 — Converge the derivation on the canonical Rust home

The seeder must not carry a *second* CID implementation. Bind `cid-artifact.ts`'s derivation to eprfs-core's
`BlobCid::compute_raw` (raw `0x55`, sha2-256, base32) — or, at minimum, pin it to eprfs-core's **golden
vectors** in a shared cross-check test so the TS and Rust derivations cannot diverge.

- **Files:** `genesis/seeder/src/cid-artifact.ts`; a new cross-check test importing eprfs-core golden vectors.
- **Verify:** `cid-artifact.ts` `deriveBlobCid(bytes)` === eprfs-core `compute_raw(bytes)` for the golden
  vector set AND for `manifesto.md` (already confirmed this session: stored `bafkreigdzb…` decodes to the
  file's sha256). Test-first: write the cross-check, watch it pass.

## Phase 2 — Derive at import/seed

In the seeder content build/seed path, derive `content`/`blobHash`/`blobCid` from `sourcePath` using the
Phase-1 canonical derivation, per the Phase-0 storage decision. Preserve every curated field (stewards,
contributors, `linkedData`, `openGraphMetadata`).

- **Files:** the seeder content-load path (`seed.ts` / build-data equivalent); `manifesto.json` (drop or
  mark the three fields per Phase 0).
- **Verify:** seed manifesto locally → `GET /blob/<hash>` resolves (no 404); the seeded node's address
  matches the canonical derivation of the current `manifesto.md`. Edit `manifesto.md`, re-seed, confirm the
  address tracks with **no manual sync step**.

## Phase 3 — Schema + Rust↔TS boundary

Reconcile the view schema (`elohim/sdk/schemas/v1/views/`) + codegen so the derived-vs-stored distinction is
honest (a field that is *derived at seed* should not read as a hand-authored wire field). Ensure doorway's
blob-serve reads the derived address. Run the schema-contract harness.

- **Verify:** `pnpm run schema:validate` + the schema_contract test green; doorway serves the manifesto blob
  from the derived hash. p2p-design-gate boundary re-checked (snake_case never leaves Rust; no TS transform).

## Phase 4 — Simplify the guard; retire the drift class

With the fields derived, `cid-artifact-integrity.spec.ts` changes from "stored == derived" to "source JSON
does not hand-store derivable fields" (option a) or "generated projection is fresh" (option b). The drift
class is structurally gone.

- **Verify:** full genesis vitest + validate green; the guard now fails loudly if someone *re-introduces* a
  hand-stored `blobHash` into a source seed. Record the decision **not** to build rung 2, with rationale.

## Risks

- **Rust→TS binding.** Calling eprfs-core from the TS seeder may need a napi/wasm/CLI shim; the golden-vector
  cross-check (Phase 1) is the low-cost fallback that still prevents divergence without a live binding.
- **Doorway assumption.** If the blob-serve path reads a *stored* `blobHash` from the seeded row rather than
  deriving, Phase 2/3 must keep the seeded *runtime* row carrying the derived value (derive-then-store-at-seed),
  omitting it only from the *source* JSON. Phase 0 must nail this distinction.
- **Existing manifesto migration.** The already-correct `manifesto.json` (fixed this session, live on `dev`)
  must remain valid through the transition; Phase 2 is a no-op on its address if the derivation matches.

## Backlog captured (not in this plan's scope)

- **Rung 2 (conditional):** edit-time `.epr-meta` rule on `genesis/docs/content/elohim-protocol/` firing when
  a CID-artifact `sourcePath` `.md` is edited — build **only if** rung 3 (this plan) stalls. File to
  `genesis/data/timeline/backlog/`.
- **`genesis/seeder` typecheck red (pre-existing):** `wait-for-drain.ts` / `wait-for-pull.ts` missing
  `@elohim/storage-client` exports; `just gate` runs `install validate test`, not typecheck. Separate debt.
