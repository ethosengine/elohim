---
title: "Sealed Edges Slice 1 — the Seal-Aware Walk (edge records, one-graph index, seal/reseal/hold)"
id: sealed-edges-slice1-seal-aware-walk
tier: plan
status: Draft
created: 2026-07-21
maintainers: Matthew Dowell + Claude Fable 5
class: process-meta
process_subdomain: doc-lifecycle
sprint: unranked-new (born from the 2026-07-21 spec; ranks at next roadmap refresh)
requires_env: []
topic: [sealed-edges, edge-record, edge-index, seal, reseal, hold, governor, epr-cli, epr-rea, sidecar, walk, frontier, cidv1]
refines:
  - genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md
cites:
  - sealed-contract-edges-governor-frontier | the spec this plan implements — slice 1 of its gap-items (#1 edge record, #2 one-graph index, #3 seal/reseal/hold verbs) | sha256:ace1788fa44a293f | path: genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md
  - cite-fingerprint-cid-convergence | binds the verdict logic — sidecar edges compare full CIDv1, doc envelopes compare the short-form rendering of the same digest; Rust stays the single encoder | sha256:0a657c9c1b0c43e7 | path: genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md
  - epr-rea-valueflow-fabric | the parent fabric — FlowRecord/FlowStore/walk this slice extends with the Edge record and seal-aware Frontier | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - elohim/epr-rea/src/store.rs
  - elohim/epr-rea/src/walk.rs
  - elohim/eprfs/epr-cli/src/flow/mod.rs
  - elohim/eprfs/epr-cli/src/flow/walk.rs
---

# Sealed Edges Slice 1 — the Seal-Aware Walk

> **Goal:** make the walk seal-aware end-to-end — sidecar edge records (gap #1), the one-graph
> edge index over doc cites + sidecar records (gap #2), and `epr flow seal|reseal|hold` (gap #3)
> — with a single end-to-end proof: *seal an edge, mutate the upstream, walk forward, see the
> stale frontier; reseal or hold, see it clear.* No hook, no gate, no policy registry, no
> triage in this slice — everything else composes on top of a walk that already understands
> seals. Covers gap-items #1–#3 of
> `specs__2026-07-21-sealed-contract-edges-governor-frontier-design`; gaps #4–#11 remain
> tracked there as later rungs.

## Context (verified in-tree 2026-07-21)

- `elohim/epr-rea` (nested single-crate workspace): `FlowRecord` enum + `FlowStore` trait with
  `MemoryFlowStore` and `SidecarFlowStore` (append-only dag-cbor lines, CID'd, deduped by CID);
  `walk.rs` has `FlowWalk::{walk_back → Lineage, walk_forward → Frontier}`.
- `elohim/eprfs/epr-cli` `flow` family: `project` / `walk` / `status` wired in `main.rs`;
  `flow/mod.rs` already ports the Python cite oracle to Rust — `strip_frontmatter`,
  `parse_frontmatter`, canonical-body CID (`BlobCid`), and the cite **envelope parser**
  (`slug | desc | fingerprint [| status:] [| path:]`, mod.rs:343).
- `eprfs-core::BlobCid`: `compute_raw` (raw 0x55 → `bafkrei…`) + `short_fingerprint` pinned to
  the Python oracle (convergence spec). Rust IS the single CID encoder — no new encoder appears
  in this slice, and Python remains decode-only.
- Native crates: `RUSTFLAGS=""`, `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/<fam>/crates/{dev}`.

**Design bounds from the spec (binding):** an edge carries exactly one conformance mechanism —
`governor: cite-seal` seals a CID and can go stale; any other governor is a citation of the
stronger system and NEVER enters the stale set. Doc `cites:` envelopes stay untouched (short-form
fingerprints, tool-managed by cite-gen); sidecar records seal full CIDv1. Staleness is derived,
never stored as truth; `held` carries `reason` + `valid_from` (+ optional `superseded_by`).

## Task 1 — `DepEdge` record in `elohim-epr-rea` (gap #1)

**Files:** `elohim/epr-rea/src/model.rs`, `store.rs`, `lib.rs` (re-exports), unit tests in-file.

**Source of truth (per the spec's p2p-design-gate output):** the `.eprfs/status/` sidecar is the
floor truth for edge records at this slice (local, append-only, B2-shaped observation — no DHT
writes); doc-plane edges' truth stays the doc's own `cites:` bytes. The edge *index* and every
verdict are derived/operational (C) — reconstructable from sidecar + tree, never authoritative.
Graduation (push → Attestation, held → `Mishpat::Commitment`) is gap #11, out of this slice.

1. Add payload struct + enums (VF-adjacent naming, dag-cbor canonical, CID via `atom_cid`):
   ```rust
   // Source of truth: .eprfs/status/ sidecar (local observation floor, append-only; B2 —
   // graduates to the existing Attestation entry type at push, gap #11). Never a DHT write here.
   pub struct DepEdge {
       pub from: String,                 // repo-relative path of the downstream artifact
       pub to: String,                   // repo-relative path (v1 floor; slugs resolve via index)
       pub desc: Option<String>,         // the announcement — why this edge exists
       pub governor: Governor,
       pub sealed_cid: Option<Cid>,      // full CIDv1 raw of upstream body at conformance; None unless CiteSeal
       pub sealed_by: AgentRef,
       pub sealed_at: i64,               // git/appended timestamp, no wall-clock in lib
       pub status: Option<EdgeStatus>,   // None = healthy claim at seal time
   }
   pub enum Governor { Compiler(String), Codegen(String), SchemaContract(String), Test(String), CiteSeal }
   pub enum EdgeStatus { Held { reason: String, valid_from: i64, superseded_by: Option<Cid> } }
   ```
   Note: `Stale` is deliberately NOT an `EdgeStatus` variant — staleness is derived (§2 of the
   spec), only `Held` is a declared state. Invariant enforced in a constructor/validate fn:
   `sealed_cid.is_some() ⇔ governor == CiteSeal`.
2. Add `FlowRecord::Edge(DepEdge)` variant; `SidecarFlowStore` needs no changes beyond the enum
   (append/dedupe/CID machinery is generic over `FlowRecord`). Add `FlowStore::edges()` default
   method (filter-map, latest-per-`(from,to)` wins by `sealed_at` then file order — the
   append-only reseal/hold semantics).
3. Dedup identity: `edge_fp(from, to) = sha256(from|"|"|to)[:12]` helper (an index key, never an
   address — matches the spec's gate output).

**Verify:** `cargo test` in `elohim/epr-rea` — round-trip (append Edge → records → edges),
latest-wins on reseal, invariant rejection (sealed_cid on a Compiler edge), CID stability
golden. `cargo fmt --check && cargo clippy -- -D warnings`.

## Task 2 — the one-graph edge index + seal-aware walk (gap #2)

**Files:** new `elohim/eprfs/epr-cli/src/flow/edges.rs`; edits to `flow/walk.rs`, `flow/mod.rs`
(re-export only).

1. `EdgeIndex::build(root, store)` merges two sources into one `Vec<IndexedEdge>`:
   - **Doc plane:** walk the doc corpus the projection already scans; for each doc, parse
     `cites:` envelopes (existing parser). Each envelope → an `IndexedEdge { governor: CiteSeal,
     sealed_short: fingerprint, … }`. Resolve `path:` locator first, slug via the projection's
     doc set second. Envelope `status: held…` maps to `Held`.
   - **Sidecar plane:** `store.edges()` (Task 1), latest-per-(from,to).
2. Verdict derivation per edge (pure fn, unit-tested):
   - governor ≠ CiteSeal → `Governed(mechanism)` — never stale.
   - `Held{..}` → `Held` (carry reason).
   - else recompute upstream canonical-body CID: sidecar edges compare full CID; doc envelopes
     compare `short_fingerprint` (one digest, two renderings — same verdict logic, two
     comparisons). Mismatch → `Stale`, match → `Ok`, unresolvable target → `Dangling`.
3. Seal-aware walk: `flow walk <path>` gains an **Edges** section in both directions —
   incoming (who depends on me: forward edges from the index where `to == path`) and outgoing
   (what I depend on) — each line `verdict · governor · desc`. `FrontierView` gains
   `stale_edges: Vec<EdgeView>`; `flow status` gains the one-line totals
   `edges: N sealed · M governed · S stale · H held · D dangling`.

**Verify:** unit tests on verdict derivation (fixture temp-tree: doc with envelope cite, sidecar
edge, drift the upstream, assert Stale; held stays Held while stale-by-CID). Parity guard: for a
doc envelope, the Rust verdict on an UNCHANGED target must be `Ok` for every sealed cite in
`genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md`
itself (the dogfood corpus check, mirroring the existing cite-parity approach).

## Task 3 — `epr flow seal | reseal | hold` (gap #3)

**Files:** new `elohim/eprfs/epr-cli/src/flow/seal.rs`; `flow/mod.rs` run() dispatch.

1. `epr flow seal <file> --on <upstream> [--governor compiler:<unit>|codegen:<pipeline>|schema-contract:<test>|test:<id>|cite-seal] [--desc <hint>]`
   — default governor `cite-seal`; computes upstream canonical-body CID (Rust encoder),
   appends `FlowRecord::Edge`. Prints the sealed line (mirror cite-gen's output UX). Refuses
   `--governor cite-seal` on a missing upstream (Dangling at birth is an error); non-seal
   governors record no CID.
2. `epr flow reseal <file> [--on <upstream>] [--all-stale]` — the deliberate re-bless (mirrors
   `cite-gen --refresh`): recompute + append superseding record(s) for this file's stale
   outgoing edges; `--all-stale` without `--on` iterates them. Never auto-blesses from the walk
   or a hook — reseal is always an explicit act.
3. `epr flow hold <file> --on <upstream> --reason <text> [--valid-from <iso>] ` — append the
   `Held` record (valid_from defaults to the git head timestamp source the flow family already
   uses; no wall-clock).
4. Wire into `flow::run` + help text; `--json` output for all three (agents consume this).

**Verify:** CLI integration test in `elohim/eprfs/epr-cli/tests/` on a temp fixture repo:
seal → status shows 1 sealed; mutate upstream → status shows 1 stale and walk-forward from the
upstream lists the downstream in `stale_edges`; reseal → 0 stale; separate edge held → stays
held, excluded from stale. Governed edge (compiler) never appears stale under mutation.

## Task 4 — end-to-end proof + gates (the slice's definition of done)

1. The Task-3 integration test IS the a2o-shaped proof (temp-repo fixture; no live env —
   `requires_env: []` holds). Additionally run the three verbs against THIS repo once and
   commit nothing: `epr flow status` before/after a scratch seal in `/tmp` — smoke only.
2. Gates, per touched tree: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
   in `elohim/epr-rea` AND the `elohim/eprfs` workspace (pool slots, `RUSTFLAGS=""`). Existing
   parity pins (`cite_parity`, convergence goldens) must stay green untouched.
3. Update the spec's gap-item states: #1–#3 flip OPEN→CLAIMED on landing (verification via the
   tests above earns done; the ledger keeps them CLAIMED until independently verified).

## Non-goals (compose later, tracked as gaps #4–#11)

Governor policies + auto-derivation (#4), file-leave hook (#5), edge-findings ledger +
escalation (#6), push-gate leg (#7), genesis-stage/prose/recipe triage (#8–#9), scoreboard
wiring (#10), attestation/Mishpat graduation (#11). Slug identity for code artifacts and
rename-propagation stay on the spec's open questions — v1 edge targets are paths.

## Tasks (decompose targets)

- [x] Task 1: `DepEdge` + `Governor` + `EdgeStatus::Held` + `FlowRecord::Edge` + `edges()` latest-wins + `edge_fp` + invariant, with unit tests (elohim/epr-rea)
- [x] Task 2: `EdgeIndex::build` (doc envelopes + sidecar, one graph) + verdict derivation (Ok/Stale/Held/Governed/Dangling) + seal-aware `flow walk`/`flow status` surfaces + dogfood parity check (epr-cli)
- [x] Task 3: `epr flow seal/reseal/hold` verbs + `--json` + run() wiring, with CLI integration test on a temp fixture repo (epr-cli)
- [x] Task 4: end-to-end stale→reseal/hold proof green + fmt/clippy/test gates on epr-rea + eprfs workspace + flip gap-items #1–#3 to CLAIMED
