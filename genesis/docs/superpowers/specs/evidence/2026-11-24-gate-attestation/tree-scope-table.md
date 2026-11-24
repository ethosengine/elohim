---
title: "Table 1a: the gates whose key covers the whole commit tree, and why"
id: gate-attestation-evidence-tree-scope
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device), read-only evidence for integrator shem; Table 1 cells gathered by a read-only search agent and spot-checked by hand"
serves: [push-gate-pays-once]
tags: [gate, closure-key, tree-scope, evidence]
graduation-trigger: >-
  The steward rules on narrowing these keys, or the gate-input audit names every read and no gate keys the whole tree.
cites: []
---

# Tree-scope gates: why each keys the whole tree, and what a declaration would cover (2026-11-24)

Read-only, computed at 6f8dd6cd5 (fold head) with gate-evidence.mjs / gate-input-audit.mjs; seconds are
this device's (ethosengine) gate-cost ledger, mean of the last <=10 green runs (p90 in brackets).
No key or declaration was changed. Narrowing a key is integrity-class: for the steward.

Two independent whole-tree inputs exist:
- reads scope "tree": readSetAt returns null for every non-cargo gate (the audit scans .rs only;
  gate-input-audit.mjs scan(): `if (!file.endsWith('.rs') ...) continue`), and for a cargo gate on
  UNAUDITED_RECIPE_PROJECTS or with an unresolved read.
- dirTree: a gate whose dir is "." keys the whole commit tree as its project tree, whatever its reads.
`// gate-reads:` declarations are parsed only from Rust sources, so a non-Rust gate cannot declare a
read that way; its existing route is a suite read list (SUITE_READ_LISTS, today only orchestrator:
genesis/orchestrator/suite-reads.json), which also runs the suite in a view of those reads.

| rank | gate | s mean [p90] | kind | dir | why tree | declarable? | key would then cover |
|---|---|---|---|---|---|---|---|
| 1 | orchestrator-tree | 150 [304] | cargo | genesis/orchestrator | on UNAUDITED_RECIPE_PROJECTS (231 Rust reads named, 0 unresolved); its recipe runs non-Rust legs (lint-jenkinsfiles) | yes, after auditing the recipe's non-Rust reads (Jenkinsfiles etc.) and removing it from the list | genesis/orchestrator tree + the 231 named reads + the declared Jenkinsfile set |
| 2 | elohim-app | 127 [142] | just:gate | app/elohim-app | non-cargo: no audit | not by gate-reads; by a suite read list + a view run | app/elohim-app tree + listed reads (lamad workspace, sophia element dist, libraries, pnpm lock/workspace) |
| 3 | sweettest-check | 87 [157] | cargo | . | 1 unresolved: elohim/holochain/tests/sweettest/src/tests/happ_lineage_migration.rs:314 (CARGO_MANIFEST_DIR/../../dna/node-registry) | yes: `// gate-reads: elohim/holochain/dna/node-registry` (a genuine walk) | reads scope would be paths (10 reads) BUT dir "." keeps dirTree = whole tree: also needs dir narrowed to elohim/holochain/tests/sweettest (manifest change) |
| 4 | genesis-a2o | 83 [84] | root-just | genesis/a2o | non-cargo | suite list | genesis/a2o tree + listed reads |
| 5 | elohim-storybook | 48 [54] | root-just | app/elohim-library | non-cargo | suite list | app/elohim-library tree + listed reads |
| 6 | elohim-library | 30 [31] | root-just | app/elohim-library | non-cargo | suite list | as above |
| 7 | doorway-app | 26 [26] | just:gate | doorway/doorway-app | non-cargo | suite list | its tree + listed reads |
| 8 | genesis | 22 [26] | just:gate | genesis/seeder | non-cargo | suite list | genesis/seeder tree + listed reads |
| 9 | dna-extern-lints | 13 [72] | root-just | . | non-cargo AND dir "." | suite list AND a narrowed dir | the DNA zome trees it lints |
| 10 | eprfs-ts | 12 [15] | just:gate | elohim/eprfs/eprfs-ts | non-cargo | suite list | its tree + listed reads |
| 11 | storage-client-ts | 11 [12] | just:gate | elohim/sdk/storage-client-ts | non-cargo | suite list | its tree + generated bindings |
| 12 | address-profile-lint | 8 [8] | root-just | . | non-cargo AND dir "." | suite list AND a narrowed dir | — |
| 13 | constants-sync | 6 [6] | root-just | . | non-cargo AND dir "." | suite list AND a narrowed dir | — |
| 14 | schema-codegen | 5 [6] | root-just | . | non-cargo AND dir "." | suite list AND dir elohim/sdk/schemas | schemas + generated outputs |
| 15 | epr-ts | 4 [4] | root-just | elohim/sdk/epr-ts | non-cargo | suite list | its tree |
| 16 | schema-validate | 2 [3] | root-just | . | non-cargo AND dir "." | as schema-codegen | — |
| 17 | gherkin-prepush-lint | 2 [2] | root-just | genesis/a2o | non-cargo | suite list | genesis/a2o features |
| 18 | schema-dna | 1 [2] | root-just | . | non-cargo AND dir "." | as schema-codegen | — |
