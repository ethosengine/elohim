---
name: project-epr-flow-valueflow-projection
title: epr flow — developer valueflow projected from the repo
description: "epr flow project/walk/status + seal/reseal/hold over recipes.yaml + .eprfs sidecars — dev valueflow projected from the filesystem; walk is seal-aware."
metadata: 
  node_type: memory
  title: epr flow — developer valueflow projected from the repo
  type: project
  originSessionId: 89a16d25-7c1e-4c41-ab07-855e74cd03df
  modified: 2026-07-21T14:39:11.765Z
---

The developer valueflow is projectable from the repository itself (landed 2026-07-18,
commits 2e7f8c8dd spec · 6f9103a14 crate · 8cb9d89da CLI). Rails:

- `elohim/epr-rea` (elohim workspace): REA/VF atoms (ProcessSpec/Intent/Commitment/FlowEvent/
  Process, dag-cbor CIDs), folds (resource_state, fulfillment ratio), FlowStore
  (Memory + `.eprfs/status/flows.jsonl` sidecar, CID-verified lines), FlowWalk
  (walk_back lineage / walk_forward unfulfilled frontier). Deps kept to epr codec +
  serde so bridges/valueflows (isolated workspace) can path-dep it.
- `epr flow project|walk|status` (elohim/eprfs/epr-cli): derives records from the repo —
  specs/plans → Process (inputs = sealed cites, scope = refines target) + git-provenanced
  Produce events; gap-items → Intents (+ Commitments when CLAIMED); a2o features →
  scenario Commitments. Idempotent (CID dedup). Recipe = `.claude/epr-meta/recipes.yaml`
  (elohim-dev-pipeline@1; stages+edges hashed, paths: are binding-plane, excluded).
- Rust body-CID short fingerprints == sealed cite `sha256:hex16` (dynamic parity test
  over real cites) — the cite↔CID convergence holds in Rust via eprfs-core BlobCid.
- `.eprfs/` is gitignored operational state; rebuild = re-run project.

Slice-1 sealed contract edges landed 2026-07-21 (spec 2026-07-21-sealed-contract-edges-governor-frontier-design):
`DepEdge` on the FlowStore (governor: compiler/codegen/schema-contract/test = Governed, never
stale; cite-seal = full-CIDv1 seal, staleness DERIVED never stored; Held has valid_from) +
one-graph edge index (doc cites + sidecar) + `epr flow seal|reseal|hold` (path-confined,
two-phase reseal, stale-gated). First real-repo run: 278 sealed · 79 stale · 4 dangling —
the triage backlog is now enumerable. Gaps #4-#11 (governor policies, file-leave hook,
edge-findings ledger, push-gate, triage tranches, Attestation graduation) still OPEN in the spec.

**Why it matters:** "change A → walk the valueflow to the finish" is now a mechanical
traversal (frontier = open downstream commitments), not a judgment call. Granularity is a
governed per-scope policy, never a crate constant (spec §2.4).

Next: slice 5 (FlowStore over diesel rails) is HARD-GATED on the REA action-vocabulary
reconciliation (ReaVerb ≠ storage actions ≠ schema enum — no fourth enum). a2o suite:
genesis/a2o/features/devflow/developer-valueflow-projection.feature (6 @wip).
Related: [[project_rea_compute_commitment_primitive]], [[project_reach_earned_push_deterministic_floor]].
