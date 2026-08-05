---
name: project-seam-concern-architecture-landed
title: Seam-concern architecture P0-P4 landed 2026-08-02
description: "Canon+crate+registries+census+cascade+matrix+birth-rule+residual channel live (13 commits, unpushed); P5 held; graduation = supersession, never in-place edit"
metadata: 
  node_type: memory
  type: project
  originSessionId: 9e9e69ab-dae2-42a4-8cb8-cc022008f6ee
  modified: 2026-08-02T19:56:55.737Z
---

The seam-concern-contract architecture plan (genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md, status Active) was implemented P0–P4 on 2026-08-02 in 13 commits (573e134e4..93ef3da04 on feat/angular22-node24, committed NOT pushed — integrator pushes). What now exists and where:

- **Concern canon**: `.claude/epr-meta/policies.yaml` (enforcement rows; C2@2 + C6a@2 with validators) + `.claude/epr-meta/concerns.yaml` (Precedent rows; two-home rule in its header). Pins via `epr-meta-pin.py --registry`. A class graduates by supersession lineage, never in-place edit.
- **`crates/seam-contracts`** (published `elohim-seam-contracts`): `Answer<T>`, `ReasonLabel`, `Arbitrated`/`Quiescent`/`Liveness` harnesses, `canon.rs` PolicyPins (deprecation cascade), `residual.rs` ResidualWitness (C14). Leaf crate, wasm-clean; pre-push gate exists but NO CI pipeline coverage yet (ledgered).
- **Decision-point registries**: `seam-registry.yaml` in elohim-storage, content_store zome, doorway-service, steward/node (schema: `elohim/sdk/schemas/v1/manifest/seam-registry.schema.json`); census + cascade + concern×seam matrix + seeded forecast + calibration ledger all surface in `placement-audit.py --epr-meta`; standalone runner `.claude/scripts/seam-audit.py`.
- **Birth rule**: p2p-design-gate Step 4 (package-rooted) + seam-birth-rule injects in doorway/steward/seam-contracts `.epr-meta`.
- **Storage liveness regression**: `elohim/elohim-storage/src/liveness_contract.rs` FAILs the pre-wave-5 and 2026-07-11 historical predicate sets, PASSes live.

Residuals: P5 held (brit residency, wider adoption; did-bridge first). P3.3 zome pull-forward not taken (scope decision). HIGH ledgered finding: elohim-sdk Native-mode silent write loss. Cross-runtime validator parity gap dormant. Forecast row 1 (doorway should_serve_response fail-open, zero call sites) is the top pre-seeded collision for reach-enforcement work.
