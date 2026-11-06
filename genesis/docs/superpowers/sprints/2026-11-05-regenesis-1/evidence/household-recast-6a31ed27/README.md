---
id: rg1-shem-household-recast-6a31ed27
title: "shem household recast on dev 6a31ed270 — blob-durability set"
---

# shem household recast on dev 6a31ed270 — blob-durability set

Device shem, session `617b9272-adc3-4226-9bd4-cec1ac027767`, for orchestrator `184beb01`.
Steward of record: Matthew ("yes, recast when the integrated head arrives").

## What ran

| Step | Result |
|---|---|
| Checkout | worktree `rg1-integrate` detached at `6a31ed270265a8391736cc5f24f2a38e62490d1e` (= origin/dev); brit `1c6ae5568`, rakia `f9787c9e5`, sophia `631f7f4b8`; conductor fork pin `96bcd2bee215` |
| DNA wasm | `just build` in elohim, imagodei, infrastructure, mishpat, node-registry: EXIT=0 each |
| DNA + hApp pack | fork `hc dna pack` per `HAPP_DNA_TABLE` row, `hc app pack` workdir (raw `hc dna hash`, local rustc, not a baseline): lamad `uhC0ks3F1YwYyEF0D4cMCMakGPLOsI3MWYxwg5_-5La9K-n0Vr2Ke`, imagodei `uhC0kcVJouRpPF0cHcIRiNgrN2yQEGY4GB9o_ZiCyDuJ_xTXRII--`, infrastructure `uhC0kO2CNlHaYxq2y-x01o0fUagdm0_6QP0_cIzO3HHjz_LC0b_63`, mishpat `uhC0kqJEGpjFrTaXOHHCnRtxHoMwFow9sFbSTZG5jnZIr8Sb4hAKl`, node_registry `uhC0kXlFYJP8A43rEspUGvAQOBBztMW5LZOOmKWH2tFCY5nk_4xSl` |
| Mesh binaries | `just mesh build` (storage `p2p p2p-iroh`, doorway, beacon) from the head: EXIT=0 |
| Apps | sophia UMD, app deps closure, elohim-app (browser+server) and lamad dist from the head: EXIT=0 (wasm-pack 0.13.1 installed with Matthew's approval) |
| Preflight | all ok except the expected bundle-identity skew against the old cells |
| Recast | `MESH_RESET=1 just mesh start` (MESH_DIR = canonical `household-dowell`): EXIT=0; `just mesh wait`: ready in 408 s |
| Prologue | `PROLOGUE: complete`, every MUST-SUCCEED leg green. Soft reds: `seed-operator-bindings` (OPERATOR_LOGIN_FAILED human-matthew-manager, HTTP 401 on the second invocation; the first created 1 binding) and `seed-commitments` (7 created; activation of james-son→matthew-manager got HTTP 400 "REA projection changed during authority read; deferred") |

## Lanes

| Lane | Report | Scenarios |
|---|---|---|
| `@concern:blob-durability and not @wip and not @browser-only` | `sprint-report-household-20261106T134927Z-6a31ed27` | 14: 7 passed, **1 failed**, 3 pending, 3 skipped |
| `features/content/epr-content-addressing.feature` | `…T135336Z-6a31ed27` | 2 passed |
| `features/dataplane/blob-replication.feature` | `…T135359Z-6a31ed27` | 2 passed |
| rerun `features/resilience/chaos-peer-churn.feature` | `…T135503Z-6a31ed27` | 3: 2 passed, **1 failed** (same scenario, reproduced) |

app-blob-heal-on-read ("First request to an app with locally-missing bytes heals and serves") passed
inside the tag lane.

**The red.** `chaos-peer-churn.feature:112` "Cascading peer loss degrades the protection status
honestly, step by step" refuses at its baseline step: custody seeded on matthew, jessica and james, but the
household resilience fold on matthew counts 1 intra-household copy for `chaos-ladder` ("household
formation incomplete: member agent keys not bound on the fold peer"). No peer had been killed yet. The two
sibling chaos scenarios pass. Unverified lead: the prologue's red `seed-operator-bindings` leg (operator
login 401), which may leave jessica's and james's agent keys unbound as household members on matthew.

Pending (step not implemented): the three grandma-photos scenarios that need the operator seed lever.
Skipped: the household-diversity / distribution scenarios gated by `@requires`.

`a2o-reports-listing.txt` lists `genesis/a2o/reports/` in the integrate worktree after the runs.
