---
title: "Gate and attestation evidence, 2026-11-24"
id: gate-attestation-evidence-index
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device), read-only evidence for integrator shem; Table 1 cells gathered by a read-only search agent and spot-checked by hand"
serves: [push-gate-pays-once]
tags: [gate, attestation, evidence, index]
graduation-trigger: >-
  The design section that cites these tables is accepted by the steward, and these files become its
  appendix or are retired with it.
cites: []
---

# Gate and attestation evidence, 2026-11-24

Read-only evidence gathered on the ethosengine device for the integrator (session cacccb55) during
the native-dev-plane sprint, for the steward's ruling on gate-key and attestation policy. Nothing
here is built or decided; each table states its own base commit and method.

| File | What it answers | Base |
|---|---|---|
| [attestation-properties-table.md](attestation-properties-table.md) | Table 1: for gate verdicts, artifact records, household ValidationAttestations and Jenkins claims, which of closure, toolchain, platform, signer standing, independent-execution count, method version, later enrichment and location each records today (file:line or absent) | dev d74bc6707 |
| [tree-scope-table.md](tree-scope-table.md) | The gates whose closure key covers the whole commit tree, the reads the input audit cannot name, and what a declaration would narrow (ranked by recorded seconds) | dev at the roll-2a review |
| [split-units-table.md](split-units-table.md) | Table 2: which native gates could be split into finer artifacts first, whether `epr artifact key` already keys the unit, and what blocks consuming it (ranked by recorded seconds) | dev c4bea4ded |
| [release-acceptance-design-note.md](release-acceptance-design-note.md) | An unreviewed proposal: what the store checks today when it accepts a release closure from a sibling, and what acceptance without a source rebuild would require of an attestation | dev 9f272d343 |
| [v3-to-fold.tsv](v3-to-fold.tsv) | Raw key moves between two adjacent commits (roll-2a v3 and the batched fold): per gate project, whether its key moved and which material field moved it | roll-2a-v2 vs fold-on-v3 |

Seconds are this device's gate-cost ledger (`genesis/orchestrator/gate-cost.mjs`), mean and p90 over
the last 20 runs. There were no per-step seconds before the per-step reading (lane 2, landed at
9f272d343). The scripts that produced the key-move and tree-scope tables stay on the device under
`genesis/local-dev/cd-key-moves/`.
