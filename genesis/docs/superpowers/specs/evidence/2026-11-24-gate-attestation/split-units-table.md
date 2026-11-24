---
title: "Table 2: which native gates could be split into finer artifacts first"
id: gate-attestation-evidence-split-units
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device), read-only evidence for integrator shem; Table 1 cells gathered by a read-only search agent and spot-checked by hand"
serves: [push-gate-pays-once]
tags: [gate, artifact, split, per-crate, evidence]
graduation-trigger: >-
  The steward rules on the graded split policy and the first split lands with its own per-unit verdict key.
cites: []
---

# Table 2: which native gates could be split into finer artifacts first

Read-only. Base: dev `c4bea4ded`. For integrator shem, who takes it to the steward with Table 1
(`tree-scope-table.md`) and a graded policy proposal. Nothing here is built until the steward
rules.

Seconds are this device's gate-cost ledger (`gate-cost.mjs` readCycleUnion/costOf, window 20):
mean [p90], with runs and fails. There are no per-step seconds on record anywhere. `gate-steps.sh`
prints step names but no times. The per-step reading (`gate-step-reading@1`, cd/reading, landing in
land/next) is the first thing that will measure them. Every "where the time goes" below is
therefore an unmeasured expectation until that reading has a few pushes.

## What `epr artifact key` keys today

One **binary**: `describe --project P --bin B [--features F] --rustflags R --sources
MANIFEST#STEP [--crate DIR]` gives the method, and `key` gives its derivation key at HEAD over
the view of the crate's sources (exit 3 when the view is dirty). Its callers:

- eprfs `epr` and `bin-launch`, after the eprfs gate (`genesis/orchestrator/eprfs-produce.sh:41`,
  `bins=(epr bin-launch)`).
- The mesh builds of `elohim-storage` (features `p2p p2p-iroh`), `doorway` and the beacon
  (`hc-mesh.sh:2075`, `just mesh build`).

It keys **no test result, no clippy result and no library build**. No gate reads it as an input or
consumes a keyed output in place of running its recipe.

## What stops any gate from consuming a finer unit today

- **One recipe, one process, one verdict.** Every one of these gates is `gate-steps.sh` running
  fmt, lint and test (plus extras) in one shell. The exit status is the first red. Steps record
  nothing separately.
- **One evidence key per project.** `gate-evidence.mjs` keys a verdict on the project's whole
  closure (manifest, steps, recipe file, runner, the project dir tree, audited reads, rustc,
  gitlinks, …). Reuse (`GATE_EVIDENCE=reuse`) is all or nothing per project. There is no per-step
  or per-package verdict to look up.
- **eprfs already selects packages but cannot record them.** `GATE_NARROW` computes the reverse
  dependencies of the changed packages (`gate-runner.mjs:629-660`) and passes them as
  `GATE_CARGO_PACKAGES` to `_gate-eprfs` under `on`. Nothing in `gate-evidence.mjs` or the
  recording path names the narrowing (no `narrow`/`GATE_CARGO_PACKAGES` there). So under
  `GATE_NARROW=on` a narrowed run would appear to be recorded as the green of the **whole**
  project closure. Default `shadow` runs the full workspace, so today nothing is mis-recorded.
  **Turning `on` without a per-package key would be an integrity hole.** I read this from the
  code and did not run it; it is the first thing to confirm.

## The table, ranked by recorded seconds

| # | Gate | Seconds mean [p90] (runs, fails) | Shape | Finer unit that could stand alone | Already keyed by `epr artifact key`? | What prevents consuming it |
|---|---|---|---|---|---|---|
| 1 | **elohim-storage** | 1359 [1669] (10, 2 red) | One crate (no workspace members). Steps: fmt, migration-hygiene, image-includes, clippy (default features), clippy-iroh (`p2p p2p-iroh`), test (`cargo test`: lib plus 19 integration test files). `CARGO_BUILD_JOBS=1`. | Per step × feature set: clippy-default, clippy-iroh and test are three compiles of the same source under different features. Per test target: the lib tests and each `tests/*.rs` binary. One crate means any `src/` change moves every unit, so the gain is on test-only, migration-only and doc-only changes, and on not re-linking both feature sets. | Partly. The mesh binary `--bin elohim-storage --features "p2p p2p-iroh"` is keyed (`hc-mesh.sh`). No test or clippy result is. | One recipe and one verdict. clippy-iroh and test share no recorded artifact, and the gate's `dev` slot build is not the keyed mesh build (different profile and features). |
| 2 | **eprfs** | 1210 [1565] (10, 3 red) | 13 crates. Steps: fmt, precheck, lint (workspace), bins (`epr`, `bin-launch`), ark (`elohim-ark` bin), test (workspace, `--skip live_corpus_`). | **Per-crate test and clippy result**, keyed by the crate's own closure plus its path dependencies. This is the clearest win: the reverse-dependency selection already exists (`GATE_NARROW`). The bins step's outputs are a second unit. | Yes for the bins: `epr` and `bin-launch` are described, keyed and put after the gate (`eprfs-produce.sh`). No per-crate test or clippy result is. | The evidence store keys the whole project, so a narrowed run has nowhere honest to record (see above). The bins are keyed **after** the gate, and nothing looks the key up **before** building them. |
| 3 | **steward-node** | 376 [502] (10, 0 red) | One crate. fmt, clippy, test. | Per step only (clippy vs test). | No (no mesh or release bin is keyed). | One recipe and one verdict. Small gain: one crate, three steps. |
| 4 | eprfs-live-corpus | 117 [218] (7, 0 red) | Already split out of eprfs: the `live_corpus_*` legs, keyed to their own project by `// gate-reads(eprfs-live-corpus): …`. | **Precedent, not a candidate.** It shows the split pattern working: a doc commit no longer moves the eprfs key. | n/a | n/a |
| 5 | **elohim-ark** | 78 [96] (10, 2 red) | 3 crates of the `elohim` workspace (`ark/core`, `ark/supervisor`, `ark/cli`). fmt, lint, test, doc test. | Per crate (core vs supervisor vs cli). The `ark` binary is also built inside the eprfs gate's `ark` step, so it is paid twice. | No. | One recipe and one verdict. The ark bin built by eprfs is not keyed, so neither gate can reuse the other's. |
| 6 | **jenkins-bridge** | 75 [102] (10, 0 red) | One crate. fmt, lint, test. | Per step only. | No. | One recipe and one verdict. Gain is negligible. |
| 7 | **cargo-bridge** | 55 [102] (10, 0 red) | One crate. fmt, lint, test. | Per step only. | No. | One recipe and one verdict. Gain is negligible. |

## Reading it

- **Two gates are 79% of the seconds:** storage (1359) and eprfs (1210) together are 2569 s of
  the 3268 s summed means across these seven rows. Steward-node is a distant third. The ark and bridge
  rows are not worth a split on cost alone.
- **eprfs is first to split by structure.** It has 13 crates, a working reverse-dependency
  selector, and bins already keyed. What is missing is one thing: a **per-package verdict key**
  in the evidence store, i.e. the package's closure, not the project's. With that, `GATE_NARROW=on`
  becomes safe and its saving becomes real. Without it, `on` must stay off.
- **storage is first by seconds but split differently.** It is one crate, so per-crate does
  nothing. Its candidate units are per step and per feature set (two clippy compiles and a test
  compile of the same source), plus per test binary for changes under `tests/`. Whether the
  feature-set split pays depends on the step seconds the reading has not measured yet.
- **One cheap structural fix spans two rows.** The ark binary is built by both the eprfs and
  elohim-ark gates and keyed by neither. Keying it (as `epr` and `bin-launch` already are) and
  having the second builder look the key up first would pay it once.

## Integrity notes for the policy proposal

- A finer verdict must key the finer closure: the crate's own sources, its path dependencies, its
  features, flags and toolchain. It must never key the project's, or a narrowed run's green
  stands for code it never ran. This is the `GATE_NARROW=on` hole above.
- A per-step verdict needs the step's own inputs. For example, `migration-hygiene` reads
  `migrations/` and its script, and that is not `src/`. That needs the same audited-reads
  discipline Table 1 describes. Otherwise the step's key falls back to the project tree.
- Reusing a keyed binary in a gate in place of building it is consumption, and it is held to the
  same admission rules as sibling evidence (D5).
