---
title: "Gate step-source narrowing: what a dependency's tests cost a dependent gate, measured, and the change that would stop it"
id: gate-step-source-narrowing-design
status: draft
class: process-meta
date: 2026-11-26
context-tier: disclosed
steward: human:matthew
author: "Claude (Fable 5.2), a read-only agent dispatched by the integrator session cacccb55 on shem; no cargo, no gate and no push was run"
serves: [push-gate-pays-once]
refines: verdict-is-an-attestation-on-an-artifact-design
tags: [gate, closure-key, step-sources, read-set, ci-dispatch, narrowing, shadow, evidence]
graduation-trigger: >-
  The steward rules on section 8 (the option, its first shadow step and what is not done), and the
  shadow of section 8.2 has printed beside the real decision for twenty landings on two devices, or
  the steward rules that the measured yield of section 5 does not justify building it.
cites: []
---

# Gate step-source narrowing

This document asks the steward of record, `human:matthew`, to rule on one question: which change,
if any, should stop a large cargo gate from re-running when a commit touches only a dependency
crate's tests, benches, examples or documents. It gives the measured frequency of such commits on
`dev` over thirty days, a recommendation, and the first shadow step. Nothing in it is built.

The short answer has two halves, and they pull in different directions.

- The mechanism is real and I reproduced it. A commit that changes only
  `elohim/eprfs/device-runtime/tests/boundary.rs` moves the key of `elohim-storage`,
  `steward-node`, `elohim-ark` and `jenkins-bridge` in two fields, `steps` and `reads`. With the
  read side narrowed, `steps` still moves (section 2).
- It is rare. In thirty days, 4 of 1504 commits were of this kind for `elohim-storage`, and none
  of the 170 landing ranges of this device were. Three other causes moved the same keys more often
  on edits the gate's verdict cannot depend on (section 6).

So the recommendation is the gate-key-only exclusion (option b), shipped as a shadow first, with
the turn-on held until the shadow shows it pays for its own one-time re-key. Section 8 says what I
would not do.

`cites:` is left as the empty list the neighbouring specs carry. No fingerprint was written by
hand; `epr flow cites` has not been run on this file.

## 1. How this was measured

Base: `origin/dev` at `294716a1f`, read on shem on 2026-11-26. Everything was read from git objects
and from this device's ledgers.

- Gate projects, step sources and audited reads: `loadGateRegistry`
  (`genesis/orchestrator/pipeline-registry.mjs:213-252`) and `scan`
  (`genesis/orchestrator/gate-input-audit.mjs:556-558`) called from a script, over the tracked files
  at the base.
- Real keys: `gateKeyMaterials` (`genesis/orchestrator/gate-evidence.mjs:680-839`) at pairs of
  commits, comparing the material field by field. It spawns `brit build fingerprint`, git and
  `rustc -vV`, and writes nothing.
- History: every non-merge commit reachable from `origin/dev` with a commit date from 2026-10-27
  to 2026-11-25, 1504 commits (90 merges left out). Git fallback: `brit log` has no `--since`,
  `--name-only` or `--format`.
- Landings: the 171 `landed` lines of this device's `.git/landing/settle.log` (2026-11-04 to
  2026-11-25), giving 170 ranges between consecutive landed tips, each read as a net diff.
- Seconds: `costOf` (`genesis/orchestrator/gate-cost.mjs:91-109`) over the `gate-cycle-seconds@1`
  rows of this device's flow sidecars (2508 rows), which is the last ten green full runs per
  project; and the per-step readings in `.eprfs/status/readings/gate-step.jsonl` (2530 rows, from
  2026-11-24).
- Cargo's dependency edges were read from the `Cargo.toml` files with a script. `cargo metadata`
  was not run, because the task forbade cargo.

The scripts and their outputs stay on this device under
`genesis/local-dev/step-source-narrowing/` (untracked).

A matcher check: the script matches globs with picomatch (`dot: true`). `brit build fingerprint`
reports `input_count` 1205 for eprfs's step, 1219 for jenkins-bridge's and 375 for elohim-ark's; the
script counts 1205, 1219 and 375 source files. For storage, steward and doorway brit counts 6, 1
and 5 more than the script (3175, 1920, 689 against 3169, 1919, 684). I did not trace those to
individual files; the steps' `buildProcess` entries and gitlinks are the likely difference.

## 2. What a gate's key is made of, and where `sources` enters it

The key is the sha256 of the closure material (`gate-evidence.mjs:400-421`). The step `sources`
globs of the nearest `build-manifest.json` enter it in four places.

1. `steps`: one `brit build fingerprint <manifest> --step <s> --commit <c>` per step
   (`gate-evidence.mjs:104-133`), a content fingerprint of every committed blob the globs match.
2. `reads`: the input audit scans every `.rs` file the globs cover
   (`gate-input-audit.mjs:518`), including the integration tests of dependency crates, and each
   path those files read is keyed by object id (`gate-evidence.mjs:765-769`). A read the audit
   cannot name keys the whole commit tree.
3. `gitlinks`: submodule pins under a source glob (`gate-evidence.mjs:293-305`).
4. The dirty check before reuse covers the source prefixes (`gate-evidence.mjs:447-451`).

The same globs select the gate locally (`graph-walker.mjs:343-372`, `projectsFromStale`
`:130-146`) and dispatch CI (section 7).

A gate project that declares its own `inputs` and no `steps` has no step fingerprint and so no key
(`gate-evidence.mjs:105-107`); `pool-policy.json:101-114` withholds those from reuse. Nothing in
this document changes them.

### The reproduction

Commit `7ba27c3f4` (2026-11-11) changes one file, `elohim/eprfs/device-runtime/tests/boundary.rs`.
The real key function, at its parent and at it:

| gate | real key | fields that moved | key with the dependency-tests read variant | fields that moved |
|---|---|---|---|---|
| elohim-storage | moved | `reads`, `steps` | moved | `steps` |
| steward-node | moved | `reads`, `steps` | moved | `steps` |
| elohim-ark | moved | `reads`, `steps` | moved | `steps` |
| jenkins-bridge | moved | `reads`, `steps` | moved | `steps` |

In every row the read that moved is `elohim/eprfs/device-runtime`, the whole-crate row from
`device-runtime/tests/manifest_store.rs:103` and `:106`. This confirms the 2026-11-25 finding:
narrowing the audited reads alone does not hold these keys, because the step fingerprint moves too.
I did not run the fifth gate, `cargo-bridge`; its manifest's sources are the same list as
jenkins-bridge's with its own directory first (`bridges/cargo/build-manifest.json:18`,
`bridges/jenkins/build-manifest.json:18`).

The read shadow on dev (`READ_SHADOW_VARIANTS`, `gate-runner.mjs:339-344`) cannot show this saving
for the same reason. It writes a row only when a variant's movement differs from the real key's
(`gate-runner.mjs:382-383`), and a step-fingerprint move makes every variant move. This device's
readings ledger holds 2530 rows and no `read-shadow` row.

## 3. Cargo's facts: what of a dependency enters a dependent

From Cargo's documented behaviour; not re-derived by running cargo here.

Enters a dependent's build and tests:

- the dependency's library target: the files rustc reads from its `src/` module tree, including
  any `#[path]` module and any `include!`, `include_str!` or `include_bytes!` target wherever it
  lives;
- its `build.rs` and whatever that script reads;
- its `Cargo.toml`, the workspace `Cargo.toml` it inherits from, and the dependent's own
  `Cargo.lock`;
- `.cargo/config.toml` files above the build.

Does not enter:

- the dependency's `tests/**`, `benches/**` and `examples/**` targets: Cargo builds them only for
  the packages a command selects, and a path dependency is not selected by `cargo test` in the
  dependent;
- the dependency's `#[cfg(test)]` code: `cfg(test)` is set only for the crate under test;
- its dev-dependencies;
- its Markdown and other files no target includes.

Three facts in this repository qualify that list.

- **A build script with no `rerun-if-changed` re-runs on any change in its package.** None of the
  first-party dependency crates is in that state: `elohim/elohim-chrome-asset/build.rs:21-23` and
  `elohim/eprfs/epr-cli/build.rs:5` both declare their inputs. `doorway/doorway-service/build.rs`
  and `steward/device/src-tauri` have build scripts and are gate roots, not dependencies.
- **`cargo fmt --all` formats path dependencies, every target of them.** Three gates run it:
  `bridges/cargo/justfile:4`, `bridges/jenkins/justfile:4`, `bridges/k8s/justfile:4`. For those
  three, a dependency's `tests/*.rs` is a real input of the `fmt` step. The other cargo gates run
  `cargo fmt --check` on their own package or name packages with `-p`
  (`justfile:680-699`, `elohim/elohim-storage/justfile:134`, `steward/node/justfile:43`).
- **A test can read a file by path at run time.** That is not a Cargo edge at all; it is what the
  input audit exists for.

## 4. Question 1: each cargo gate's sources, and what they cover that cannot affect it

There are 34 gate projects with a `run.cargo` block. 23 borrow manifest steps and have a key; 11
declare their own `inputs` and have none.

"Own" below means the crates whose tests the gate's recipe runs. I read that from each recipe: a
gate's `dir`, except `elohim-epr` (`-p elohim-epr -p elohim-epr-rea -p elohim-epr-index`,
`justfile:685-688`), `elohim-placement` (`elohim-placement-core` and `elohim-compute`,
`elohim/placement/justfile:4-8`) and the eprfs pair (`--workspace`, `justfile:732-746`).
"Dependency t/b/e" counts the tracked files under `tests/`, `benches/` or `examples/` of any crate
that is not the gate's own. "Docs" counts `*.md`, `docs/**` and `.epr-meta/**` in those crates.

### Gates with a key

| gate | manifest | globs | files matched | dependency t/b/e | docs | whole-crate globs that carry them |
|---|---|---|---|---|---|---|
| elohim-storage | `elohim/holochain/build-manifest.json` (`cargo-build-storage`) | 46 | 3169 | 145 | 20 | `crates/**` (:91, 44 files), `bridges/**` (:92, 24), `elohim/epr/**` (26), `elohim/elohim-render/**` (13), `elohim/epr-index/**` (8), `elohim/epr-rea/**` (7), `elohim/eprfs/device-runtime/**` (:102, 6), `elohim/ark/core/**` (5), `crates/elohim-address/**` (5) and eight more with 1 to 4 |
| steward-node | `steward/device/build-manifest.json` (`cargo-build-steward`) | 42 | 1919 | 325 | 51 | `elohim/elohim-storage/**` (:22, 203 files: storage is a dependency here), `crates/**` (:21, 44), `elohim/epr/**` (26), `elohim/elohim-render/**` (13), `elohim/eprfs/device-runtime/**` (:38, 6) and eleven more |
| eprfs, eprfs-live-corpus | `elohim/eprfs/build-manifest.json` (`rust-build-test`) | 33 | 1205 | 95 | 13 | `elohim/epr/**` (26), `crates/elohim-host-contract/**` (26), `elohim/epr-index/**` (8), `elohim/epr-rea/**` (7), `elohim/ark/core/**` (5), `elohim/ark/supervisor/**` (5), `crates/elohim-address/**` (5) and five more. `elohim/eprfs/**` (:15) is its own workspace and stays whole |
| cargo-bridge | `bridges/cargo/build-manifest.json` | 27 | 1211 | 520 | 15 | `elohim/eprfs/**` (:18, 425 files: the whole eprfs workspace is a dependency here), `elohim/epr/**` (26), `crates/elohim-host-contract/**` (26) and ten more |
| jenkins-bridge | `bridges/jenkins/build-manifest.json` | 27 | 1219 | 520 | 15 | the same list as cargo-bridge (:18) |
| elohim-ark | `elohim/ark/build-manifest.json` (`gate-ark`) | 21 | 375 | 87 | 11 | `elohim/epr/**` (26), `crates/elohim-host-contract/**` (26), `elohim/epr-rea/**` (7), `elohim/eprfs/device-runtime/**` (:19, 6), `crates/elohim-address/**` (5) and six more |
| doorway | `elohim/holochain/build-manifest.json` (`cargo-build-doorway`) | 29 | 684 | 61 | 13 | `elohim/epr/**` (26), `elohim/elohim-render/**` (13), `elohim/elohim-agent/**` (9), `crates/elohim-address/**` (5) and five more |
| k8s-bridge | `bridges/k8s/build-manifest.json` | 15 | 229 | 52 | 6 | `elohim/epr/**` (26), `elohim/epr-rea/**` (7), `elohim/ark/core/**` (5), `crates/elohim-address/**` (5) and three more |
| elohim-placement | `elohim/placement/build-manifest.json` | 10 | 219 | 40 | 6 | `elohim/epr/**` (26), `elohim/epr-rea/**` (7), `crates/elohim-address/**` (5), `crates/seam-contracts/**` (2) |
| elohim-sdk | `elohim/holochain/build-manifest.json` (`sdk-feature-matrix`) | 10 | 178 | 37 | 7 | `elohim/epr/**` (26), `crates/elohim-address/**` (5) and four more |
| host-contract | `crates/elohim-host-contract/build-manifest.json` | 4 | 105 | 31 | 4 | `elohim/epr/**` (26), `crates/elohim-address/**` (5) |
| browser-bridge, workspace-bridge | `bridges/browser`, `bridges/workspace` | 7 | 104 | 14 | 4 | `crates/elohim-address/**` (5), `crates/elohim-sdk/**` (3) and four more |
| elohim-epr | `elohim/epr/build-manifest.json` | 6 | 118 | 5 | 0 | `crates/elohim-address/**` (5) |
| elohim-compute, elohim-live-session, hc-dbtool, rust-style-checker, app-lifecycle-contracts | their own | 1 to 12 | 8 to 68 | 0 | 0 | none: their sources name only their own crate and plain files |
| orchestrator-tree, sweettest-check, manifest-hygiene | | | | | | keyed on the whole commit tree (`UNAUDITED_RECIPE_PROJECTS`, `gate-input-audit.mjs:30`; `dir: "."`, `gate-evidence.mjs:721-723`). No glob change narrows them |

Of these, `cargo-bridge`, `host-contract`, `elohim-sdk`, `app-lifecycle-contracts`,
`sweettest-check` and `manifest-hygiene` are withheld from reuse for other reasons
(`genesis/agentic/pool-policy.json:104-116`), so no key change saves them a run today.

### Gates with no key

`elohim-address`, `elohim-blob-store`, `elohim-storage-client`, `seam-contracts`, `consent-grant`,
`elohim-error`, `elohim-settings`, `epr-storage`, `memory-ceremony`, `domain-types` and
`rust-style` declare gate `inputs`. Their globs cover dependency tests too (`epr-storage` 96 files,
`elohim-blob-store` 38, `consent-grant` 31), and it does not matter: they run whenever they are
selected.

### Two things the table shows beyond tests

- `elohim-storage` declares `crates/**` and `bridges/**`. Its `Cargo.toml` depends on two of the
  bridges (`did`, `valueflows`) and on six crates under `crates/`. The globs also cover
  `bridges/cargo`, `bridges/jenkins`, `bridges/k8s`, `bridges/browser` and `bridges/workspace`,
  which storage does not build. Their test files are scanned for reads, and that is how
  `genesis/orchestrator/data/deployments.json` (read by `bridges/k8s/tests/contracts.rs`) came to
  move storage's key 7 times in thirty days.
- `elohim-storage`'s step sources name `elohim/elohim-storage/src/**`, `benches/**` and
  `migrations/**` and not `tests/**` (`elohim/holochain/build-manifest.json`, the storage step). Its
  own integration tests are keyed by the project tree, and their reads outside the crate are not
  audited for the storage gate at all. They are covered today only because `elohim/sdk/**` is a
  source. A design that narrows `elohim/sdk/**` has to audit those tests first.

### Every exception found: a dependent that does read a dependency's tests or fixtures

Found by listing audit rows whose reader and target are different crates and whose target lies
under `tests/`, `benches/`, `examples/` or a `fixtures` directory, and by searching first-party
Rust for `#[path]` and include macros.

| # | reader | target | kind | what it means for an exclusion |
|---|---|---|---|---|
| 1 | `elohim/eprfs/epr-cli/src/artifact/extract_tests.rs:162` | `crates/elohim-host-contract/tests/vectors/ethosengine-epr-on-synthetic-v2.json` | run-time read in a unit test; Cargo edge at `epr-cli/Cargo.toml:89` | keyed as a named read for eprfs and the bridges. Must stay keyed |
| 2 | the same file, `:459-461` | the `v2` and `v3` vectors in the same directory | `format!` over a loop value | the audit records `elohim/eprfs/epr-cli`, which does not contain the target. Today the `v3` file is keyed for eprfs only because `crates/elohim-host-contract/**` is a step source. **Excluding that crate's `tests/**` from the step part would leave `v3` unkeyed for the gate that runs this test.** This is the one place I found where the exclusion would produce a stale green as the code stands |
| 3 | `elohim/ark/cli/src/declared_trust.rs:422` | `elohim/eprfs/epr-cli/tests/fixtures/declared-trust-vectors.json` | `concat!` path with a `// gate-reads:` line | keyed as a named read for elohim-ark. `ark/cli/Cargo.toml` has no dependency on epr-cli, so this is a read with no Cargo edge (row 6 of `evidence/2026-11-24-gate-attestation/key-closure-gaps.md`) |
| 4 | `elohim/eprfs/epr-cli/tests/canon_lift_parity.rs:49` | `elohim/epr/tests/vectors/concern_canon_row_c0.json` | ancestor search with a `// gate-reads:` line | keyed as a named read for eprfs |
| 5 | `elohim/eprfs/epr-cli/src/artifact/host.rs:242` and `src/artifact/host/native_libraries/tests.rs:7` | `epr-cli/tests/support/artifact_elf.rs`, `toolchain_pin.rs` | `#[path]` into the same crate's `tests/`, under `#[cfg(test)]` | same crate, so eprfs's project tree keys it. It shows that a `tests/` location does not mean "not compiled into the library's test build". The audit's variant already keeps `#[path]`-reached files (`gate-input-audit.mjs:524-535`) |
| 6 | `bridges/cargo/justfile:4`, `bridges/jenkins/justfile:4`, `bridges/k8s/justfile:4` | every target of every path dependency | `cargo fmt --all -- --check` | the `fmt` step of these three gates reads dependency tests. An exclusion is unsound for them until the step formats the bridge's own package only |
| 7 | `elohim/eprfs/eprfs-serve/src/lib.rs:31-32` | `src/testing.rs` behind the `testing` feature | shared test support compiled into the library | it lives under `src/`, so a `tests/**` exclusion does not touch it. Listed so that no later rule excludes "testing" code by name |

Not found: a `#[path]`, `include!`, `include_str!` or `include_bytes!` in one crate that names a
file under another crate's `tests/`, `benches/` or `examples/`. That is a search of the tree at
one commit, not a proof about later commits.

## 5. Question 2: how often it happened, and the seconds

### Recorded seconds

`gate-cycle-seconds@1`, this device, last ten green full runs (`gate-cost.mjs:97-104`):

| gate | mean s | p90 s | max s | runs |
|---|---|---|---|---|
| elohim-storage | 4579 | 8060 | 10319 | 10 |
| eprfs | 1807 | 2386 | 3432 | 10 |
| steward-node | 1089 | 1902 | 2442 | 10 |
| doorway | 633 | 896 | 1489 | 10 |
| k8s-bridge | 152 | 313 | 313 | 6 |
| jenkins-bridge | 143 | 238 | 243 | 10 |
| elohim-ark | 163 | 196 | 247 | 10 |

The task statement gives about 1700 s as storage's p90. This ledger says 8060 s. The per-step
readings since 2026-11-24 agree with the ledger: storage ran in 5807, 5872 and 7091 s, steward-node
in 1184 and 1205 s. I use the ledger. A verdict that stands in costs 13 to 51 s in the same
readings (storage 44 and 51, steward-node 41, eprfs 36 and 43, jenkins-bridge 27, elohim-ark 13).
`berth impact` on a device-runtime test path printed `no nearby live intent observed` and no
seconds, because no session had declared those paths.

### Counts

252 of the 1504 commits touch some crate's `tests/`, `benches/` or `examples/`. 25 touch nothing
else.

The question asks for commits that touch only non-entering paths of a crate another gate declares.
Per gate, that is: the commit matches the gate's sources, and every file of it that matches is a
dependency crate's tests, benches or examples.

| gate | commits matching its sources | of those, dependency t/b/e only | still held by a read that stays | net commits | landing ranges matching its sources | of those, dependency t/b/e only |
|---|---|---|---|---|---|---|
| elohim-storage | 265 | 4 | 0 | 4 | 37 | 0 |
| steward-node | 234 | 10 | 7 | 3 | 33 | 0 |
| eprfs | 392 | 1 | 0 | 1 | 74 | 0 |
| jenkins-bridge | 388 | 29 | 24 | 5 | 74 | 4 (all 4 held by a read that stays) |
| cargo-bridge | 388 | 29 | 24 | 5 | 74 | 4 (the same) |
| k8s-bridge | 64 | 2 | 0 | 2 | 13 | 0 |
| elohim-ark | 134 | 1 | 0 | 1 | 33 | 0 |
| doorway, elohim-placement, elohim-epr, elohim-sdk, host-contract, browser-bridge, workspace-bridge | 23 to 81 | 0 | | 0 | 3 to 15 | 0 |

"Still held by a read that stays" means a file of the commit is covered by a read that a file
outside the excluded set makes. Two rows do nearly all of it:

- `epr-cli/src/artifact/extract_tests.rs:461` records the whole `elohim/eprfs/epr-cli` crate. It
  holds 24 of the bridges' 29 commits and all 4 of their landing ranges.
- `elohim-storage/src/boundary.rs:115` and `:177` and
  `src/services/constitutional_ratio_registry.rs:84` record the whole `elohim/elohim-storage`
  crate. They hold 7 of steward-node's 10 commits.

The four storage commits are `7ba27c3f4`, `1d2916dca`, `e5d200cec` and `493a71fd9`.

For the docs class (a dependency's `*.md`, `docs/**`, `.epr-meta/**`), adding it to the exclusion
changes the counts by 0 to 3 commits per gate and by no landing range.

### The estimate

Seconds saved per held event = the gate's run minus a stand-in. At p90: storage 8060 − 51 = 8009,
steward-node 1861, eprfs 2343, jenkins-bridge 211, k8s-bridge 283, elohim-ark 183.

| reading | what it assumes | storage | steward | eprfs | jenkins | k8s | ark | total |
|---|---|---|---|---|---|---|---|---|
| landing ranges of this device, 22 days | each landing is one gated push; the key is compared with the previous landing's | 0 | 0 | 0 | 0 | 0 | 0 | **0 s** |
| commits, 30 days, p90 | every such commit was gated alone, on a device that already held a green for its parent | 4 × 8009 = 32036 | 3 × 1861 = 5583 | 2343 | 5 × 211 = 1055 | 2 × 283 = 566 | 183 | **41766 s** |
| commits, 30 days, mean | the same, at mean seconds | 4 × 4528 = 18112 | 3 × 1048 = 3144 | 1764 | 5 × 116 = 580 | 2 × 122 = 244 | 150 | **23994 s** |

So the honest range for thirty days on one device is **0 to about 42,000 s**, and the history
points at the low end. The commit reading is an upper bound, not an expectation: these commits
normally land in a range with a `src/` change that moves the key anyway, which is why no landing
range qualified.

The one measured occurrence is the 2026-11-25 event itself, and it is not in this count: the
tests-only device-runtime change on branch `agentic/read-narrow` (`7871d02f`) is not on dev. At the
task's figures it cost storage 7134 s and steward-node 1211 s, 8345 s for one event. One or two such
events a month would be 8,000 to 17,000 s. That is my best single estimate, and it rests on one
observation.

### What the estimate leaves out

- **A re-gate after the base moved.** A lander gates a head, another device lands, the lander
  rebases and gates again. The key holds only if the other device's commits touched nothing in the
  closure. That is the case narrowing serves best and the history cannot count it: it needs the
  pairs of keys a device computed for the same work before and after a rebase. As a rate, 11 of
  1504 single-commit deltas (0.7%) and 0 of 170 landing-range deltas would newly hold storage's
  key when both the step part and the read part are narrowed.
- **`just gate` runs on a branch between landings**, which the settle log does not record.
- **The other device.** Landings by ethosengine are inside this device's ranges and are not counted
  as gated pushes of their own.
- **Selection.** I counted a gate as selected when a changed file matches its sources or lies in
  its `dir`. The real selector also asks `rakia affected` and keeps one hop
  (`gate-runner.mjs:108-131`).
- **One audit for thirty days.** The read sets and manifests are those of `294716a1f`, applied to
  every commit in the window.
- **CI seconds.** Section 7.
- **The one-time re-key** (section 8.4), which is a cost on the other side.

## 6. What moved the big keys instead

Because the yield above is small, I attributed every key-moving commit of the three large gates to
its cause. "Sole cause" means the commit touched nothing else in the closure.

| gate | key-moving commits | touch the gate's own tree | sole cause, commits (landing ranges) |
|---|---|---|---|
| elohim-storage | 302 | 112 | `elohim/sdk/**` 27 (3); `elohim/eprfs/device-runtime` outside `tests/` 26 (5); the whole-crate read `elohim/elohim-storage` 10 (2); `elohim/epr` 7; `elohim/Cargo.lock` 7 (2); the read of `genesis/orchestrator/data/deployments.json` 7; `genesis/agentic/bin/pool-lib.sh` 5 (2); several causes at once 76 (15) |
| steward-node | 275 | 25 | `elohim/elohim-storage` outside `tests/` 72 (5); `elohim/eprfs/device-runtime` outside `tests/` 26 (6); reads of `elohim/sdk/domains` 9 and `elohim/sdk/schemas/v1` 7 (1); `elohim/elohim-storage/tests` 6; `pool-lib.sh` 5 (2); several 89 (17) |
| eprfs | 500 | 328 | the read of `genesis/agentic/bin/berth` 28 (7); the root `justfile` 13 (1); the read of `genesis/agentic/pool-policy.json` 12 (4); the read of `elohim/sdk/schemas/v1` 9; several 47 (5) |

Three of these are edits the gate's verdict cannot depend on, and each is more frequent than the
tests-only case.

1. **A habit atom inside the storage crate still re-keys the storage gate.** `projectTree` drops
   habit atoms from the project tree (`gate-evidence.mjs:141-167`, landed 2026-11-09 in
   `0452dc85e`). The three whole-crate read rows named above put the crate's plain tree object back
   into `reads`. Verified with the real key function: commit `78d79ccf9` (2026-11-16) changes
   `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md` and nothing else in storage's
   closure, and storage's key moved in the field `reads`, at the path `elohim/elohim-storage`. 11
   commits in thirty days touch a storage habit atom and no other file under the crate. A habit
   atom alone does not select the storage gate, so the cost arrives at the next selection or the
   next re-gate after a moved base.
2. **Test files of crates storage does not build are scanned for reads**, through `crates/**` and
   `bridges/**` (section 4). The `deployments.json` row is the measured case: 7 commits.
3. **`elohim/sdk/**` is 1535 files.** Storage's Rust reads the schemas and domain manifests under
   it. I did not measure how many of the 27 commits touched only files storage does not read (the
   TypeScript client sources, for example). That measurement is the next one worth making.

The device-runtime rows outside `tests/` are the case the device-runtime reads table already
treats: narrowing below `src/**` is sound only under its conditions A to D
(`evidence/2026-11-24-gate-attestation/device-runtime-reads-table.md`), and I do not propose it.

## 7. Question 3: the constraint, and the options

### How `sources` drives CI dispatch

- `genesis/orchestrator/build-graph.groovy:234-247` (`checkSourceChanges`): a step is stale when
  any changed file that is not a governance path matches any of its `sources`, by the glob matcher
  at `:186-217`.
- `:360-375` (`propagateStaleness`): staleness spreads along `depends` to a fixed point.
- `:848-852`: the stale steps that are not manual-only are grouped by pipeline, and those pipelines
  are dispatched.
- `genesis/orchestrator/graph-walker.mjs:284-289` (`walkGraphCi`) is the JavaScript copy of that
  rule. The fleet-roll guard and the preview read it.
- `pipeline-registry.mjs` exposes pipeline metadata and the gate registry (`:147`, `:213`). It does
  not read `sources` except through `loadGateRegistry`'s step names (`:232`).
- `genesis/orchestrator/build-inputs-closure.test.mjs:1-50` holds the other direction: every Rust
  build unit's sources must cover its whole path-dependency closure. Partial coverage of a crate
  directory counts, so a manifest that names `X/src/**` and `X/Cargo.toml` passes it.

The storage step is where this matters. In `elohim/holochain/build-manifest.json`,
`build-edge-image` depends on `cargo-build-storage` and `cargo-build-doorway`, and
`deploy-manifests` depends on `build-edge-image`. So a change to
`elohim/eprfs/device-runtime/tests/**` (source at `:102`) makes the storage build stale, then the
edge image, then the deploy step: an edit to a dependency's test file selects an Edge build and a
fleet roll, which the local fleet-roll check then refuses without `[edge:roll]`.

### Would narrowing a source glob stop a pipeline that needs the change?

For the Edge pipeline, no: its storage build compiles the dependency's library and runs storage's
own checks (`elohim/elohim-storage/Dockerfile:540-541`, `cargo fmt --check` and
`cargo test --lib`). It does not build or run a dependency's integration tests.

For the dependency's own tests, it depends on whether another pipeline owns the crate. The eprfs
manifest keeps `elohim/eprfs/**` (`elohim/eprfs/build-manifest.json:15`), so a device-runtime test
edit would still dispatch `elohim-eprfs`. Several dependency crates have no gate project and, as
far as I read, no step that runs their tests: `bridges/did/did-bridge`, `crates/doorway-client`,
`elohim/elohim-render`, `elohim/elohim-chrome-asset`, `elohim/elohim-facings`,
`elohim/elohim-peer-fabric`, `elohim/elohim-cache-core`, `elohim/elohim-agent/gate-client`,
`doorway/relay-addr-beacon`. For those, narrowing the dependents' globs would mean a tests-only
edit dispatches nothing. Today it dispatches a pipeline that does not run those tests either, so
no check is lost, but the appearance of one is. I did not read every Jenkinsfile stage to confirm
that no CI stage runs a dependency's tests.

### The options

| | (a) narrow the manifests' `sources` | (b) leave `sources`; add a gate-key-only exclusion | (c) derive the step part from Cargo's graph |
|---|---|---|---|
| What changes | each whole-crate glob for a dependency becomes `X/src/**`, `X/Cargo.toml`, `X/build.rs`, plus each file the dependent really reads | the key's step part is computed over the sources minus an excluded set; `sources` and CI dispatch are untouched | the key's step part is the set of files Cargo says the gate's packages are built from |
| Who must change | this repository's manifests (about 15 steps); CI's behaviour changes with them; the fleet-roll guard's prediction changes with them | this repository (`gate-evidence.mjs`); rakia's schema if the exclusion is declared in a manifest (`stepInputs` and `gateProject` are `additionalProperties: false`, `elohim/rakia/schemas/v1/build-manifest.schema.json:118-142` and `:253-280`), with a pin move; brit only if the step part must stay a brit fingerprint | this repository (`crate-keys.mjs` and `cargo-affected.mjs` already call `cargo metadata` for eprfs); nothing in rakia or CI |
| Moves every key once | every gate whose step is edited: the manifest blob and the step fingerprint both change. The storage manifest is shared by doorway, elohim-sdk, elohim-storage and others, so one edit re-keys all of them | only gates for which something is excluded, if the narrowed digest is used only then (the precedent is `projectTree`, `gate-evidence.mjs:157-167`): 15 gates as the tree stands | every cargo gate |
| Changes CI dispatch | yes. A tests-only dependency edit no longer makes the storage build stale, so no Edge build and no roll is selected for it. That is a gain, and it is also a change to what rolls the fleet, which is the steward's | no | no |
| Cost of a wrong exclusion | a stale green locally and a pipeline not dispatched in CI. Both fail silently and together | a stale green locally. CI still dispatches on the full globs, so CI is an independent check of the same change | a stale green locally; CI still dispatches |
| What would catch it | `build-inputs-closure.test.mjs` for a missing dependency; nothing for a missing fixture read | the audit's named reads stay in the key, so a fixture a kept file reads is still keyed; a sampled full run (section 8.3); CI's verdict for the same commit | the seven open rows of `key-closure-gaps.md`; a sampled full run |
| Can ship as a shadow | no. A manifest edit is live for CI at once | yes, on the existing read-shadow rows, with no manifest, rakia or brit change (section 8.2) | yes; `crate-keys.mjs` is one already, and it decides nothing until its seven gaps close |
| Main risk | hand-maintained lists that drift: a new `include_str!` of a file outside `src/` needs a manifest edit that nothing forces | the rule must be exactly Cargo's rule, and the exceptions of section 4 must hold as the code changes | different lockfiles and feature sets per gate (rows B and C of the device-runtime table) mean one `cargo metadata` per gate workspace; recipe steps that are not cargo are outside it |

## 8. Question 4: the recommendation

### 8.1 The option

**Option (b), with the excluded set derived by one rule and not written by hand.**

The rule: for a gate, the excluded set is every file under `tests/`, `benches/` or `examples/` of
a crate that is (i) reached by the gate's sources, (ii) not one the gate's recipe tests, and (iii)
not reached by a `#[path]` from a file that stays. The read side uses the same set: a file in it is
not scanned for reads. A path that a file outside the set reads stays in `reads`, wherever it
lives, so a fixture under a dependency's `tests/` that the gate's own code reads is still keyed.

A gate opts in. The opt-in is the only thing a manifest would carry, for example a boolean on the
gate project, and that is the only rakia schema change. Until the steward wants it declared, the
shadow needs no manifest field.

Why not (a): it changes what dispatches the Edge pipeline, on evidence of 4 commits and no landing
range in thirty days. The gain there is real (fewer rolls selected) and deserves its own ruling
with its own evidence, not a ride on a local-gate change. Why not (c): it is the per-crate key
under another name, and that work has seven open closure gaps.

### 8.2 The first shadow step

Add one variant to the read shadow that already runs after each recorded gate
(`gate-runner.mjs:339-344`, `:1427-1468`). Under it:

- the audit is run without the excluded files (the existing `excludeDependencyTests` path,
  `gate-input-audit.mjs:524-548`, widened from `tests/` to the three directories);
- the `steps` field is replaced by a digest of the `<mode> <type> <oid>\t<path>` lines of the files
  the sources match at the commit, minus the excluded set, computed the same way at the base and
  at the commit.

The shadow compares movement, not key values, so it does not need brit to produce the narrowed
fingerprint. It writes the rows it already writes (`<project>:read-shadow:<variant>`,
`gate-runner.mjs:1135-1158`) into `.eprfs/status/readings/gate-step.jsonl`, inside the same budget
(`GATE_READ_SHADOW_BUDGET_MS`, default 15000 per push), and it decides nothing.

One thing to settle with the unlanded branch `agentic/read-narrow` first: its commit `3fa72045`
removes the "fix 1" and "both" variants and cites
`evidence/2026-11-25-manifest-dir-joins/README.md`, which finds an unconditional exclusion of a
dependency's `tests/**` unsound. That evidence file is in the `astra-docs` worktree and not on dev.
Its counterexample is exception 1 of section 4: content under a dependency's `tests/` that a kept
file reads. The rule above keeps that content, because it excludes files from the step part and
from the scan, and leaves `reads` as the kept files produce it. The two are compatible, and the shadow variant should land
on whichever side of that branch dev ends up.

### 8.3 The evidence that would justify turning it on

1. **The precondition, before any shadow row is believed.** Exception 2 of section 4 is closed at
   the source, and the audit's rule for a join it cannot read changes: a `CARGO_MANIFEST_DIR` join
   with a `..` segment and a non-literal part is unresolved (the whole tree) unless a
   `// gate-reads:` line names its targets. Recording the crate directory for it is not a superset
   of the truth. Today the step sources hide that; with the step part narrowed, nothing would.
2. **Shadow rows on two devices for twenty landings**, each held row checked by hand: every changed
   file of the range is in the excluded set for that gate, and the gate that owns each such crate
   ran, and was able to refuse the push, on the same push.
3. **A sampled full run.** When the narrowed key would have stood in and the full key moved, run
   the gate anyway for one in five such cases and record both outcomes. A red there is a caught
   stale green and ends the trial for that gate. This is the periodic full-key check, and it is
   also what would keep running after turn-on.
4. **A mutation test per exception row** of section 4, driven through the real key function: edit
   the fixture or the support file, expect the narrowed key to move.
5. **For the three bridges**, the `fmt` step first formats the bridge's own package only, or they
   stay opted out.
6. **The arithmetic.** The held seconds in the shadow, per device per month, exceed the one-time
   re-key of section 8.4. On the thirty days measured here they do not.

### 8.4 What turning it on would cost once

Each of the 15 gates with something excluded gets a new key form, so its first run after turn-on
is a real run. For the ones enrolled in reuse, at p90: storage 8060, eprfs 2386, steward-node 1902,
doorway 896, elohim-epr 327, k8s-bridge 313, jenkins-bridge 238, browser-bridge 359 (one recorded
run), elohim-ark 196, elohim-placement 127, eprfs-live-corpus 98, workspace-bridge 89 (one run).
That is about 15,000 s per device. Folding it into a landing that already moves every cargo key (a
`pool-lib.sh`, `gate-steps.sh` or toolchain change) makes it cost nothing extra.

### 8.5 What I would not do

- Not edit the manifests' `sources` for this. It changes CI dispatch and the fleet-roll prediction.
- Not exclude a dependency's Markdown or `.epr-meta` in the same step. The yield is 0 to 3 commits
  per gate, and CI's governance rule (`build-graph.groovy:220-231`) is a different rule from the
  local fingerprint's; aligning them is its own question.
- Not narrow below a dependency's `src/**`.
- Not exclude by directory name for anything a kept file reads.
- Not enrol `cargo-bridge`, `jenkins-bridge` or `k8s-bridge` while their `fmt` step is
  `cargo fmt --all`.
- Not turn it on from this document's numbers. They say the shadow is cheap and the turn-on is not
  yet earned.

### 8.6 What I would do first instead, on the same evidence

These are source edits and one audit rule. Each is integrity-class for the steward, and each is
smaller than the exclusion.

1. Narrow the three whole-crate rows of elohim-storage (`src/boundary.rs:115`, `:177`,
   `src/services/constitutional_ratio_registry.rs:84`). That closes the habit-atom re-key verified
   in section 6 (10 commits and 2 landing ranges as sole cause in the window, at 8060 s p90 when it
   costs a run) and frees 7 of steward-node's 10 tests-only commits. The registry row is the one
   literal `format!` that `agentic/read-narrow` already resolves under its knob.
2. Fix `epr-cli/src/artifact/extract_tests.rs:459-461` to two literal paths. It removes the
   whole-crate row that holds the bridges' keys and closes the one stale-green hazard found.
3. Measure `elohim/sdk/**` for storage (section 6, item 3).

### 8.7 The six joins, confirmed or corrected from the code

I did not have the read-narrow agent's wording for each, only the list. Each row says what the code
does, what the audit records today, and whether a source fix would change a key.

| site | what the code does | what the audit records today | verdict |
|---|---|---|---|
| `elohim/eprfs/epr-cli/src/artifact/extract_tests.rs:461` | the path is built at `:459` by `format!` over `for level in ["v2", "v3"]`, joined to `CARGO_MANIFEST_DIR` at `:461`; the targets are two files under `crates/elohim-host-contract/tests/vectors/` | `elohim/eprfs/epr-cli`, the whole crate, for cargo-bridge, jenkins-bridge, eprfs, eprfs-live-corpus and orchestrator-tree | **Confirmed fixable, and more urgent than a cost fix.** The record does not contain the real targets. Two literal paths fix it. It holds 24 of 29 tests-only commits for the bridges |
| `elohim/eprfs/device-runtime/src/bin_store/tests.rs:3051` | `base` is the literal directory `../../sdk/schemas/v1/views`; `:3055` and `:3059` join `format!("{name}.schema.json")` over an inline array of ten literal names and `["bin-release", "bin-shadows"]` | the directory `elohim/sdk/schemas/v1/views` (148 files) for eight gates | **Confirmed fixable; the record is sound, only wide.** A literal list of about ten files would narrow eight keys from 148 files to ten. It is a unit test of device-runtime, so only eprfs runs it |
| `elohim/eprfs/epr-cli/tests/governance_parity_vectors.rs:88` | `registry_path(name)` joins a parameter to the literal directory `../../sdk/schemas/v1/registries`; its two callers pass literals (`:94`, `:102`) | the directory (7 files) | **Confirmed fixable, small.** Two constants would name two of seven files. For the bridges this file is a dependency's integration test, which the exclusion would drop entirely |
| `elohim/eprfs/epr-cli/tests/view.rs:25` | `fixture(name)` copies the directory `tests/fixtures/views/<name>` recursively; callers pass `"orchard"` and `"workshop"`, the only two directories there | `elohim/eprfs/epr-cli/tests/fixtures/views` | **Corrected: nothing to gain.** The read is a directory walk and both subdirectories are used, so the record is exact. It is inside eprfs's own tree, which the project tree keys anyway |
| `elohim/elohim-storage/src/services/observation_kinds.rs:49` | the macro `pillar_file!` wraps `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../sdk/domains/", $rel))`; eight calls pass literals (`:63-96`) | `elohim/sdk/domains` (at `:48`) for elohim-storage and steward-node | **Corrected: fixable in the source, with no effect on a key as things stand.** Writing the eight `include_str!` calls out would name eight files, but `:382` of the same file and `src/services/manifest_registry.rs:519` read the directory `elohim/sdk/domains` as a real walk, and storage's sources carry `elohim/sdk/**` whole. This is a compile-time read of the library, so it must stay keyed for steward-node as well |
| `elohim/elohim-storage/tests/schema_contract_diesel_epr.rs:14` | `load_schema(relpath)` joins a parameter to `../sdk/schemas/v1`; its two callers pass the same literal, `views/epr-envelope-view.schema.json` (`:47`, `:103`) | for steward-node, the directory `elohim/sdk/schemas/v1`. For elohim-storage the file is not scanned at all, because storage's sources do not name its own `tests/**` | **Corrected: fixable, with no effect on a key.** `tests/schema_contract.rs`, `tests/api/qahal_http_contract.rs`, `tests/api/schema_contract_recovery_v2.rs` and `tests/search/route.rs` read the same directory as real walks, so steward-node keeps it. For steward-node the lever is the exclusion (storage is its dependency), not this join |

Sites the list does not have and the measurement says matter more: the three whole-crate rows of
elohim-storage (section 8.6), and `crates/seam-contracts/src/canon.rs:442` and `:449`, an ancestor
search that records the whole `crates/seam-contracts` crate for fifteen gates. The seam-contracts
row is in a file that stays under any exclusion. I did not measure what it costs.

## 9. What I did not measure

- No cargo command, no gate and no push was run. Cargo's behaviour in section 3, including
  `cargo fmt --all`, is from its documentation.
- The dependency closures come from parsing `Cargo.toml` files, not from `cargo metadata`. Features
  and target filters were not resolved.
- The read shadow's own rows: there are none on this device, so nothing here is its measurement.
- Whether brit's fingerprint and the script's matcher agree file for file. The counts agree
  exactly for three steps and differ by 1 to 6 for three others (section 1).
- ethosengine's ledgers and landings.
- What CI seconds an Edge dispatch for a tests-only edit costs, and how often the fleet-roll check
  refused one.
- Whether any Jenkins stage runs a dependency crate's tests.
- The narrowed key itself was not computed by the real key function, because no code was
  changed. The "held" counts are a path model of it; the "moved" facts in sections 2 and 6 are
  from the real function.
