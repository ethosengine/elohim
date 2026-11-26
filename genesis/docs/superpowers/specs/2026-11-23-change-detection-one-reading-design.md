---
title: "Change detection read as one thing — where it runs, where it is missing, and the reuse still to harvest"
id: change-detection-one-reading-design
status: draft
related: genesis/docs/superpowers/specs/2026-11-24-verdict-is-an-attestation-on-an-artifact-design.md
class: process-meta
date: 2026-11-23
context-tier: disclosed
steward: human:matthew
author: "Claude (session cacccb55, Fable 5.2 orchestrating); a repository census and a gate scoping by Codex, web research by a Sonnet agent, a fence proposal by the ethosengine session; prose by a scribe seat"
serves: [push-gate-pays-once]
refines: native-dev-tooling-over-peers-plan
tags: [change-detection, selection, closure-key, reuse, read-set, shadow, rust-style, withheld, fence]
graduation-trigger: >-
  A folded sentence saying the stations of section 6 are on dev with their Verify lines met and the
  per-step reading of section 4 has been read on two devices for ten landings.
cites: []
---

# Change detection read as one thing

This document collects every place in the repository that decides "did something relevant change",
names the two ways that decision failed on 2026-11-23, and turns what is missing into stations for
the habit `push-gate-pays-once`. It is a design and an inventory: no code is written and nothing here
is built. It refines the plan, `genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md`,
whose sections 8 to 10 hold its stations, its six tests and its vocabulary.

Its evidence is five campaign files on device shem under `genesis/local-dev/f4-ask-20261123/`,
untracked and not in the repository: `change-detection-census-receipt.md` (the census, a static read
of every mechanism that decides whether something changed, at dev `2081b2532`),
`rust-style-scope-receipt.md` (the scoping receipt, a static read of the Rust style gate, at dev
`e14abce95`), `change-detection-industry-notes.md` (the industry notes, web research on build and CI
systems), `ethosengine-fence-scope-design.txt` (ethosengine's proposal for narrowing the governance
fence) and `jenkins-carrier-design-receipt.md` (the Jenkins receipt, a static read of Jenkins as a
carrier, at dev `2081b2532`). This document carries what it needs from them and stands without them.
The receipts read source lines and ran nothing: no gate, test, build or timing; line references are
theirs. Where the integrator read a line or witnessed an event first-hand, the text says so.

The finding: selection, closure validity and cost are three separate instruments that do not read
each other, so a read can invalidate a gate's key without selecting the gate, and nothing compares
what ran with what had changed.

The steward asked for this on 2026-11-23: "a light research task.. about change detection.. anything
that does that role, brit diff, verdicts, what's best practice in industry.. just to make sure we
understand everywhere we have change detection, where we might be missing it.. I feel like we should
have caught that wall-clock on the rust style gate a long time ago and the fact that we didn't makes
me think we might have missed something else that's obvious". Later the same day: "let's make sure
we've got the right designs there and let's take those to our plans and specs so we pickup those
atomic wins to harvest more re-use over our artifacts".

**Terms.**

- **census**: the static read of every mechanism that decides whether something changed (section 0); nothing was run.
- **scoping receipt**: the static read of the Rust style gate alone (section 0); nothing was run.
- **gate**: a named check with a recipe, selected by changed paths, whose verdict is recorded against a closure key.
- **judge**: the shared code that decides a gate's verdict and reuse: the gate runner and gate-evidence.
- **closure key**: a hash over everything a gate's result depends on: its declared inputs, recipe, runner, compiler, the tree and the files its code reads.
- **fingerprint**: a hash over the committed files a step declares as its inputs; one part of a closure key.
- **verdict**: the signed record of a gate's result, green or red, for one closure key.
- **reuse**: accepting a verdict already held for the same closure key instead of running the gate again.
- **selection**: deciding, from the paths a change touched, which gates it runs.
- **read-set**: the files a gate's code is found or declared to read; they enter its closure key.
- **shadow**: a report that prints what a new rule would decide beside the real decision, and decides nothing.
- **landing**: a push that moves the integration branch `dev`.
- **station**: one claimable unit of work in a plan, with a State and a Verify.
- **habit**: a declared, checkable behaviour of the system that a station serves; here `push-gate-pays-once`.
- **steward**: the human who rules; `human:matthew`.
- **integrator**: the session that assembled and pushed the landings of 2026-11-23, session cacccb55 on device shem.
- **shem** and **ethosengine**: two devices of the one steward, each with its own checkout and gates.
- **sibling**: the steward's other device, whose verdicts can be reused.
- **admitted**: said of a device whose signed records this device accepts; used in no other sense here.
- **fold**: a device reading another device's signed records into its own store.
- **integrity-class**: a change to selection, a key, or which work is skipped; its `on` needs the steward's ruling and an adversarial review, its shadow does not.
- **brit**: this repository's Git-compatible tool, which also computes step fingerprints and affected steps.
- **rakia**: the build-graph tool that walks manifests.
- **epr** and **eprfs**: the native CLI and its Rust workspace.
- **CID**: content identifier, a hash that names bytes.
- **ExternalClaim**: a record this repository's bridges emit for something an outside service reported, marked unattested.
- **offer**: a declared statement of what an outside service may be observed doing; the Jenkins one claims and settles nothing.
- **grant**: a bounded, revocable delegation naming provider, recipient, scope and window.

## 1. What happened on 2026-11-23: two misses in opposite directions

### 1a. Too wide: the Rust style check repeats

The Rust style check compares a candidate tree's Rust style findings against a baseline (the merge
base with the integration branch) and refuses new findings. The scoping receipt traces where it runs.

**In a gate run.** `just gate rust-style` runs the checker with `--verify-scan`: adapter tests, a
build of the checker, then one repository comparison. The baseline is `merge-base(origin/dev, HEAD)`
and the candidate is the working tree, untracked Rust included; the governed Rust of both snapshots is
scanned. The gate runner records cycle data for the run, but reusable evidence needs a closure key,
and this gate has none (section 3). (`justfile:988`; under `genesis/orchestrator/`: `rust-style.sh:61–76`,
`rust-style/src/main.rs:86–126`, `rust-style/src/repository.rs:108–128,395–412`, `gate-runner.mjs:872–903`.)

**In a push.** A push of N commits does not cost N scans; it pays three kinds.

| Scan | Where it is called | Against which baseline | When it is paid |
|---|---|---|---|
| mandatory per-ref scan | `.husky/pre-push.bash:964–997` | the pushed tip's merge base with the remote ref | once per pushed ref (not a deletion, not a notes ref), before any changed-file filtering |
| full scan | the `rust-style` gate (`justfile:988`), selected at `.husky/pre-push.bash:1425` | `merge-base(origin/dev, HEAD)` (the receipt gives this for a direct gate run; assumed the same when a push selects the gate) | when the `rust-style` gate is itself selected |
| scoped comparison | `genesis/orchestrator/gate-runner.mjs:615–626` | the default integration baseline | for every selected cargo gate other than `rust-style`, before the reuse decision, so a reused gate pays it too |

The integrator read `gate-runner.mjs` on dev `2081b2532`: the scoped call (lines 615–626) comes
before `decideReuse` (about line 734). The census, which places it before reuse, is right; the
scoping receipt's phrase "each executed Cargo gate" is imprecise.

**In a hand-over.** A push to a `self/<name>` ref still runs the mandatory per-ref scan, against
`origin/dev`. The hand-over then skips project gates and writes an advisory hand-over record, and that
record does not satisfy the later dev landing, which scans again.
(`.husky/pre-push.bash:128–151,984–997,810–815,1632–1640`.)

**In CI.** The orchestrator pipeline's `Rust Style Gate` stage has no `when` condition. It runs the
checker's format, lint, tests and build, then one comparison of the committed head before downstream
dispatch; its output becomes stage evidence, not a closure verdict. (`genesis/orchestrator/Jenkinsfile:2746–2755`,
where the integrator read `stage('Rust Style Gate')` on dev `2081b2532`; `genesis/orchestrator/rust-style-ci.sh:8–59,70–74`.)

The receipt found the reason for the per-ref scan in the hook's own comment ("Check EACH pushed object
against its actual destination … Never substitute the dirty checkout or last ref for the pushed
tree", `.husky/pre-push.bash:982–983`), which means ref tips. It found no reason for anything finer.

One cost is recorded: a single observation of one full `rust-style` scan, 29 seconds
(`.eprfs/status/flows.jsonl:21320`, as the census cites it). It is not a per-landing cost, nor a
per-ref or scoped measurement, and the census found no recorded cost for the other always-run legs.
No instrument reported the repetition. The cost ceilings (`genesis/orchestrator/gate-cycle.mjs:131`)
report work that is expensive, not work that is unnecessary, and neither shadow report compares what
a step executed against its last green.

### 1b. Too narrow: a registry change selected no gate

Landing `cecab9d08` changed the policy registry, `.claude/epr-meta/policies.yaml`. The eprfs gate's
canon parity test (`elohim/eprfs/epr-cli/tests/canon_lift_parity.rs`) reads that registry and
declares the read (line 42), and the read audit recognizes the declaration
(`genesis/orchestrator/gate-input-audit.mjs:444`). So the read was in the gate's closure key. But the
registry is absent from the gate's selectors (`elohim/eprfs/build-manifest.json:14,69`) and `.claude/`
is filtered from selection (`.ci-ignore:13`, which the integrator read as `.claude/`), so the change
selected no gate. What followed is the integrator's first-hand account. The parity test reported the generated
projection `.claude/epr-meta/generated/canon-lift/canon-atoms.json` stale. Dev was red on it from
landing `cecab9d08` at about 21:05Z to landing `e14abce95` at 22:13Z, about 70 minutes. The sibling
device's gate, run on the next landing's commit, found it. The fix was `epr canon-lift --write`,
landed in `e14abce95` (commit `a5f8744c4` before the pick). The census confirms the source lines that
explain the miss; it did not itself verify the historical execution.

## 2. The finding: three instruments that do not read each other

- **Selection** decides which gates a change runs. A pushed change's paths are matched against each
  gate's declared inputs (`genesis/orchestrator/gate-runner.mjs:103`), widened by an oracle that asks
  brit which steps are affected and keeps one hop (`genesis/orchestrator/gate-oracle.mjs:43`), and by
  rakia's transitive graph (`elohim/rakia/rakia-core/src/constellation.rs:184`).
- **Closure validity** decides whether a verdict already held for a gate still stands. The closure key
  is a hash over the step fingerprints, manifest, recipe, runner, tree, compiler, gitlinks and the
  reads the audit found (`genesis/orchestrator/gate-evidence.mjs:367`); reuse fences refuse a held
  verdict on a red, dirty inputs, a signer from a device not admitted, or a governance change
  (`genesis/orchestrator/gate-runner.mjs:189,223`).
- **Cost** records what a run took. Cycle records and the ceilings over them
  (`genesis/orchestrator/gate-cycle.mjs:131`) say how long a gate ran.

Two consequences follow. First, a read can invalidate a key without selecting its gate: the read audit
feeds closure validity, not selection, which is exactly 1b. Second, nothing compares what ran with
what had changed: a step that runs on every push with an unchanged input closure, as in 1a, costs
seconds that cost records count and that no record calls waste. That is the census's finding from
the instruments it read (the cost ceilings, the input shadow and the recipe shadow), not a proof
that no such record exists anywhere.

## 3. The inventory

### 3.1 Detectors in front of work

Condensed from the census. **C** means the census read the line (CONFIRMED); **P** means it is the
census's inference (PLAUSIBLE). "Pays once" means the result is keyed and reusable, not merely that
it is called once; "n-a" means the mechanism decides selection or reports and holds no verdict.

| | Mechanism (path:line) | Unit | How it detects | Pays once | Blind spot |
|---|---|---|---|---|---|
| C | Selection, `genesis/orchestrator/gate-runner.mjs:103` | glob | match manifest inputs to changed paths | n-a | a read-set is not selection |
| C | Oracle, `genesis/orchestrator/gate-oracle.mjs:43` | step | brit affected, one hop | n-a | failure falls back to paths |
| C | Rakia, `elohim/rakia/rakia-core/src/constellation.rs:184` | path | transitive graph over source and buildProcess globs | n-a | gitlink-prefix hole (the arrival backlog, `genesis/data/timeline/backlog/steward-device-arrival-open-items.md`, line 728) |
| C | Fingerprint, `elohim/brit/brit-cli/src/commands/fingerprint.rs:1` | step hash | content hash of committed matching blobs | yes | declared inputs only |
| P | `brit diff` (tree diff at `elohim/brit/gitoxide-core/src/repository/diff.rs:11`) | file | tree comparison | n-a | CLI mapping unconfirmed |
| C | Verdict notes, `elohim/brit/brit-build-ref/src/build_cmd.rs:114` | closure | verified lookup of signed records | yes | not selection |
| C | Closure key, `genesis/orchestrator/gate-evidence.mjs:367` | key | hash | yes | broad trees and recipes |
| C | Reuse fences, `genesis/orchestrator/gate-runner.mjs:189` | closure or range | verification and diff | yes | conservative invalidation |
| C | Selecting-change fence, `genesis/orchestrator/gate-runner.mjs:223` | range | oracle | yes | needs the oracle |
| C | Read audit, `genesis/orchestrator/gate-input-audit.mjs:117` | path | static scan of Rust reads and `gate-reads` | n-a | missed reads (arrival backlog line 592) |
| C | Named suite, `genesis/orchestrator/suite-reads.json:3`, `suite-run.mjs:240` | hash | materialize and compare | yes | tree-side tests keep ambient dependencies |
| C | Cargo narrowing, `genesis/orchestrator/cargo-affected.mjs:33` | crate | reverse dependencies | n-a | unknowns keep the whole workspace |
| C | Task source view, `genesis/orchestrator/gate-task.mjs:227` | CID | source root and derivation | yes | source view is opt-in |
| C | Input shadow, `genesis/orchestrator/gate-runner.mjs:298` | fingerprint | all-versus-sources comparison | n-a | not the recipe's actual reads |
| C | Recipe shadow, `genesis/orchestrator/gate-recipe-key.mjs:643` | digest | compare root justfile closure | n-a | root justfile only |
| C | CI baseline, `genesis/orchestrator/Jenkinsfile:300` | range | diff from a chosen baseline | n-a | fallback can narrow history |
| C | CI graph, `genesis/orchestrator/build-graph.groovy:298`, `graph-walker.mjs:230` | step | hash and graph | yes | depends on baseline and state |
| C | Canon parity, `elohim/eprfs/epr-cli/tests/canon_lift_parity.rs:157` | digest | recompute and compare | no | the selection omission of 1b |
| C | Rust style ratchet, `genesis/orchestrator/rust-style/src/repository.rs:587` | finding, crate | compare baseline and current debt | no | repeated scans (1a) |
| C | Image closure, `elohim/holochain/Jenkinsfile:462` | key | registry lookup of a closure label | yes | registry failure reads as a miss |

The census lists more, all **C**, that this design does not change. Keyed and reusable: the binary
build view (`elohim/eprfs/bin-build-view.json:2`, which selects no gate) and the image build
(`scripts/ci/build-storage-image.sh:83`). Unkeyed, run each time selected: schema freshness
(`elohim/sdk/schemas/scripts/codegen-ts.mjs:892`), ts-rs export, mesh freshness
(`app/elohim-app/scripts/hc-mesh.sh:2624`). Reports and refusals with no verdict: the pipeline
registry, publish precheck, cites, index lag, the `.epr-meta` resolver and the device-head check. And
detectors of running state, outside the gate plane: hApp drift, the deploy hold, scope, peer rules,
content addressing, head propagation, projection reconcile and sync state.

### 3.2 Pre-push legs, in order

All rows are **C**. **U** means the leg has no changed-path selector; **P** means it matches paths;
**S** means it follows gate selection. Line numbers are in `.husky/pre-push.bash` unless named.

| Line | Leg | Mark and decision |
|---|---|---|
| 51 | submodules | U; refuses when missing |
| 991 | security paydown | U per eligible ref; compares a baseline |
| 994 | rust-style | U per eligible ref; refuses |
| 1047 | input policy | U; resolves the shadow |
| 1099, 1126 | fleet-roll, doc-cut | ref, graph or document size, and tag; refuse |
| 1176 to 1583 | twelve path-matched legs, listed below the table | P |
| 1375 | ci-ignore | filter; early exit |
| 1425 | project selection | S; manifest and oracle |
| 1445 | preflight | S |
| 1603 | compose gate | U after the early exits; evaluates the range |
| 1624 | T2 receipt | invoked unconditionally; inside, `genesis/orchestrator/scripts/t2-receipt.sh` is P for serving (:42, current-source proof) and recency (:69, warns or refuses) |
| 1667 | sweettest | S, plus destination and override |
| 1689 | resource check | capacity and pressure, not changes |
| 1896 | project gates | S; closure reuse where enrolled |
| 2007 | epr landing | dev, eprfs selection and exact HEAD |
| 2088 | Brit advisory | P; acknowledged integrity blind spot (line 774) |

The path-matched legs, all P: package projections (1176), hook tests (1216), habits projection
(1270), library harnesses (1297), the agent-only check with its focused early exit (1403), humans and
presences (1457), CID artifacts (1485), devices (1500), deployments (1530), runtime pins except on a
hand-over (1547), accounts (1569) and ci-ignore freshness (1583); several regenerate and compare.
The preflight's ordered checks (`genesis/orchestrator/gate-preflight.mjs:546`) inspect the
prerequisites of selected projects and hold no reusable verdict. Schema codegen is a selected recipe
(`justfile:923`), not an unconditional leg. For `just gate`, the preflight and the cargo-scoped
rust-style comparison run before the reuse decision (`genesis/orchestrator/gate-runner.mjs:615,734,1026`).

### 3.3 CI stages with no change test

All **C**. Some stages have no `when` at all; the rest are stages that run on any accepted trigger:
they have trigger, mode or branch conditions but no test of their relevant inputs. The census found
no recorded cost for any of these stages.

| Pipeline (path:line) | Stages with no `when`, or that run on any accepted trigger |
|---|---|
| `genesis/orchestrator/Jenkinsfile:2568` (where the unconditional group begins) | no `when`: dedup, checkout, rust-style (the stage itself at :2746); postflight health (:3276) and P2P (:3310) |
| `Jenkinsfile:1295` | any accepted trigger: checkout, version, WASM, install, unit, security, cleanup; publishing, deployment, health and E2E behind branch gates |
| `elohim/holochain/Jenkinsfile:2858` | any accepted trigger: setup, cache, quality, SDK, installer, publish, deploy, validation, cleanup; image bodies also consult closures |
| `elohim/holochain/dna/Jenkinsfile:406` | any accepted trigger: checkout, version, cache, WASM cache; hot-swap and publish branch-only |
| `genesis/Jenkinsfile:1953` | no `when`: peer mesh, blob upload, warm cache (:2135), projection sync (:2146), federation (:2158) |
| `genesis/Jenkinsfile:1511` | any accepted trigger: hosts, substrate, health, E2E; seed-parameter gates around installation, codegen, seeding and proofs |
| `sophia/Jenkinsfile:111` | any accepted trigger: checkout, install, lint, typecheck, tests, build, UMD, Nexus, archive; Sonar and npm branch-only |
| `steward/device/Jenkinsfile:130` | any accepted trigger: checkout, cache, hApp and UI fetch, Linux build; only release is conditional |

App and DNA build stages use `shouldRunStep`; in Genesis only constants and seeding do (`Jenkinsfile:1553`;
DNA :515; Genesis :1664, 1735). The census ranks five likely misses by structure, all **P**: the
cargo-scoped rust-style comparison before reuse (now read by the integrator, as 1a says); Sophia's
whole suite (`sophia/Jenkinsfile:140`); Steward's Linux build (`steward/device/Jenkinsfile:400`);
Genesis schema generation (`genesis/Jenkinsfile:1654`); the app's unit tests (`Jenkinsfile:1701`).
Live health and projection probes differ: unchanged source does not mean an unchanged runtime, so
they need a freshness policy, not a fingerprint.

### 3.4 Withheld from reuse

A gate is enrolled for reuse in `genesis/agentic/pool-policy.json:91`; the policy withholds the rest
with a written reason (lines 97 to 132, all **C**). Counting the census table, 36 gates are withheld.
Grouped by reason, in the policy's own words:

| Reason | Gates | Count |
|---|---|---|
| no fingerprint, alone | elements-codegen, epr-app-package, elohim-address, elohim-blob-store, elohim-storage-client, seam-guidance-contracts, reach-drift, domain-types, dialogue-stasis, agentic-shift, rakia-codegen, rakia-validate, cargo-coverage, pipeline-list-fresh | 14 |
| no fingerprint, with another reason | epr-storage (build-chain gaps), memory-ceremony (external tools), rust-style (baseline and adapters) | 3 |
| a moving baseline or history | elohim-app (moving baseline), cargo-bridge (history), address-profile-lint (repository and baseline) | 3 |
| workspace, generated or external outputs | elohim-library, elohim-storybook, app-lifecycle-contracts, doorway-app, eprfs-ts, storage-client-ts (recursive builds), genesis-a2o (workspace state) | 7 |
| a whole tree or context | elohim-sdk (whole Docker context), manifest-hygiene (whole tree and tooling) | 2 |
| reads not resolved or audited | host-contract, sweettest-check, schema-validate, schema-codegen, constants-sync, gherkin-prepush-lint | 6 |
| missing justfiles | dna-extern-lints | 1 |

So 17 gates name "no fingerprint", 14 of them for no other reason.

## 4. The missing reading

What is missing is one reading that puts the three instruments side by side. This section states it
as a contract, not an implementation.

For every step, meaning a gate, a pre-push leg or a CI stage, each run writes one row:

| Field | What it holds |
|---|---|
| step | the gate project, the hook leg, or the pipeline and stage |
| key | the step's closure key, or the written reason it has none |
| key changed | whether the key differs from the key of the step's last green: yes, no, or unknown when there is no key |
| outcome | ran, reused, or skipped |
| seconds | the step's wall-clock for this run |

The reading answers two questions from those rows. Which steps ran with an unchanged key? That is
re-verification paid twice, and 1a is its first instance. Which steps have no key and no reason? Those
are the steps nobody has judged, and 1a's CI stage is one of them.

The reading decides nothing. It selects no gate, refuses no reuse and changes no key, as the input
shadow decides nothing today.

Where it belongs follows the repository's rule, which has two parts: the section "What to work on,
and how you know it's done" of the root `CLAUDE.md`, which refuses a new register or ledger, and the
steward's ruling of 2026-10-22 recorded in `.claude/scripts/.epr-meta`, that the lowest-level
primitives are native (a record others interpret, and a judge that gives one answer, go in the native
crates; a script may sense and call). The rows are observations on a declared measure [GAP: the
measure id; naming it is part of building CD-reading and does not block its Verify], written through
the existing observation path (`epr flow sense`), and the reading is a native report beside the
others `epr flow report` prints. The pre-push hook and the gate runner stay thin callers.

This design proposes, and the steward may change, that the first rung writes rows only for gates and
pre-push legs on a device. A second rung gives CI stages the same row, read from Jenkins stage records
through the existing Jenkins bridge (`bridges/jenkins`); that rung is not in CD-reading.

In the industry notes this is practice 1 of the five ranked practices and audit questions 1 to 3:
per-step counts over a window, a reason and owner for each always-run step, and a report of steps
that executed while their input closure was unchanged.

## 5. What industry practice says

The table uses only what the industry notes support. **[F]** means the research agent fetched and
read the source; **[S]** means search snippets only. The ranked practices and the audit checklist are
the agent's own synthesis from those sources, and two items had no source at all.

| Practice | What it catches | Do we have it | Take as given or build natively |
|---|---|---|---|
| Per-step table of executed against key unchanged (Nx Cloud task analytics [S]; Bazel execution logs and Build Event Protocol [F]; Turborepo `--summarize` and `--dry` [F]) | always-run steps nobody chose | partly: input shadow and recipe shadow (3.1) report pieces; neither compares executed reads to the last green | the form is known; the rows read signed verdicts, so the report is native (section 4) |
| A written reason and an owner for each always-run step, enforced by a test (agent's synthesis) | always-run steps with no judgement behind them | partly: reuse withholding has reasons (3.4); always-run legs and stages have none (3.2, 3.3) | take as given |
| Hermetic inputs: an undeclared read fails loudly (Bazel sandboxing [F]) | too-narrow keys | partly: the read audit is a static scan (`genesis/orchestrator/gate-input-audit.mjs:117`) and has missed reads (arrival backlog line 592) | take as given; a sandbox that fails can fall back silently [F], so it must say so |
| Clean rebuild, or a diff of two execution logs, to expose undeclared or unstable inputs (Bazel [F]; Nix `--check` [S]; Gradle [S]) | undeclared reads, non-determinism | not found in the census | take as given |
| Lockfile and toolchain changes deliberately wide, with an explicit narrowing (Nx [F]; bazel-diff [F]; Turborepo [F]) | too-narrow selection after a toolchain or lockfile move | partly: the closure key includes the compiler and gitlinks (`genesis/orchestrator/gate-evidence.mjs:367`) | take as given |
| A full downstream stage as the safety net for selected tests (Meta [F]; Microsoft TIA [S]) | a selection that skipped a needed test | not established by the census | take as given |
| Cache writes restricted to trusted producers (cache poisoning, CVE-2025-36852 [S]) | a poisoned or forged result | yes: verdicts are signed records (`elohim/brit/brit-build-ref/src/build_cmd.rs:114`) and the reuse fences check the signer (`genesis/orchestrator/gate-runner.mjs:189`) | native (below) |
| A ratchet keyed per file and rule (eslint-formatter-ratchet, eslint-seatbelt [S]); a linter cache does not track cross-file dependencies (ESLint `--cache` [F]) | repeated whole-repository lint scans | partly: the Rust style ratchet keeps findings by file and line and aggregates per crate (`genesis/orchestrator/rust-style/src/repository.rs:33–53`), but holds no reusable verdict | the per-file form is known; the cross-file key is the agent's inference |
| Live state compared with desired state (Argo CD [F]) | drift outside builds | yes: hApp drift, deploy hold, peer rules (3.1) | take as given; ignoring a field to quiet noise can hide a real change [F] |
| Sampled re-execution of reused verdicts | a key that stopped moving | no | no source found; this design's own check (CD-key-moves) |
| Mutate one input to prove the key moves | a key that does not cover an input | partly: gate-evidence tests invalidate on some mutated inputs; full coverage not established | no source found; this design's own check (CD-key-moves) |

The research did not cover Rush, Gradle's affected selection, Launchable, Buildkite, CircleCI, Zuul,
Bors, Flux, ruff, pre-commit, lint-staged, Betterer, cargo and clippy fingerprinting (the docs
returned 404), or codegen and lockfile drift. Merge queues were researched and are not compared here.

Two places where a signed-verdict system among one steward's devices differs from what those tools
assume, in the plan's terms:

- **Who wrote the result.** Mainstream tools assume one operator controls every worker, and have no
  notion of a verdict's independence. Here what matters is who signed, whether that device's
  environment matched the closure, and whether a signed red or an interrupted run could be mistaken
  for a result. This is the plan's test 5 (the plan, section 9, test 5): settlement is a signed record
  keyed on the closure, and a sibling's result buys speed and never counts as an independent witness.
- **The environment in the key.** Mainstream tools assume a controlled pool of identical machines.
  With a few real developer devices, host state such as `PATH` and the tools in `/usr/bin` can leak
  into a result: the industry notes draw this from Bazel's remote-caching documentation, which says
  tools outside the workspace, such as a compiler in `/usr/bin`, are not tracked. The receipts record
  no incident of it; the plan's test 4 lists an inherited `EPR_BIN` and the node version among its
  too-narrow breaks. The plan's answer (section 10, test 6) is a fresh envelope for each
  carried step that inherits nothing it did not declare; until then, whatever host state a step reads
  belongs in its key or in its written reason for having none.

## 6. Stations

The plan's rule is harvest first, by seconds per landing; no seconds are measured here, so the order
is by structure. The reading first, because it sizes everything after it; then CD-read-selects,
because it is the miss that left dev red and its shadow should print first; then the stations that
remove repeated work; then the soundness station, which saves no seconds but without which a reused
verdict can be wrong; then the fence; last, Jenkins as a carrier. Integrity-class is as under Terms.

- [ ] **CD-reading — the per-step reading of section 4 exists and has been read for ten landings on two devices** (serves push-gate-pays-once)

State: nothing writes the rows. The pieces exist apart: closure keys, cycle records and ceilings,
the input shadow and the recipe shadow (section 2 and 3.1). This first rung covers gates and pre-push
legs on a device; CI stages come in the second rung of section 4, which is not in this station. The
reading decides nothing and is not integrity-class. It comes first because the per-step reading, not
the 29-second observation of 1a (which sizes nothing), sizes every later station: the always-run
registry, the rust-style saving and the no-fingerprint reuse are each a count of its rows.

Verify: on shem and on ethosengine, for ten landings each, the reading prints one row for every gate
and pre-push leg the landing ran, reused or skipped, with a key or a reason in every row, and lists
the steps that ran with an unchanged key.

- [ ] **CD-read-selects — a file a gate declares it reads selects that gate** (serves push-gate-pays-once)

State: the read audit (`genesis/orchestrator/gate-input-audit.mjs:117`) feeds the closure key, not
selection (`genesis/orchestrator/gate-runner.mjs:103`), and `.ci-ignore` filters `.claude/` before
selection. That is 1b: the canon parity test's declared read of `.claude/epr-meta/policies.yaml` keyed
eprfs and selected nothing. Shadow first: print the gates a change's read-set would add beside the
real selection. Integrity-class: it changes selection.

Verify: replayed against landing `cecab9d08`'s diff, the shadow names eprfs.

- [ ] **CD-rust-style-once — within one gate run, the Rust style comparison for one baseline and one tree is computed once and read by every cargo gate** (serves push-gate-pays-once)

State: the repeats are those of 1a. In a push, the mandatory per-ref scan runs before any
changed-file filtering (`.husky/pre-push.bash:994–1004`), a selected `rust-style` gate adds a full
scan, and every selected cargo gate other than `rust-style` adds a scoped comparison
(`genesis/orchestrator/gate-runner.mjs:615–626`); CI repeats a comparison unconditionally
(`genesis/orchestrator/rust-style-ci.sh:70`). Because the scoped comparison runs before `decideReuse`
(about line 734, as the integrator read it), a reused cargo gate still pays it, so this station
saves scans on reused gates too. The
per-ref scan compares the committed tip against its merge base with the remote, the cargo gates
against the default integration baseline; [GAP: whether these share one baseline and one tree on an
ordinary dev push, which decides how many scans this station removes; CD-reading, or a read of
`genesis/orchestrator/rust-style.sh`, should settle it first; it does not block this station's
Verify]. This station changes no key and is not integrity-class.

Keyed reuse of the comparison across runs is a separate, integrity-class change. The scoping receipt
gives a five-step shadow plan for it: crate keys with mutation tests, a cached baseline scan compared
with a fresh one, separate scan and adapter closures, shadow selection beside the real checks, and a
record in the `rust-style` habit's ledger before the gate leaves the withheld list. Its key must cover
the full semantic inputs (baseline finding multiset, ownership, effective configuration and
strictness, policy holdings, cross-boundary transfers), or same-count replaced or moved debt could pass.

Verify: one gate run that selects two or more cargo gates logs one rust-style comparison per baseline
and tree, each cargo gate's log names the comparison it read, and the findings equal today's
per-gate scans on the same commit.

- [ ] **CD-always-run — every leg that runs with no change test in front of it has a written reason, an owner and a retire-when in one registry beside the withheld list, and a test fails for a leg with neither a key nor a reason** (serves push-gate-pays-once)

State: the U legs of 3.2 (submodules, security paydown, rust-style, input policy, the compose gate,
the T2 receipt) and the stages of 3.3 have no written reason. The withheld list in
`genesis/agentic/pool-policy.json` is the precedent, a reason for every withheld gate; the registry
sits beside it, with no new store. A leg meant to run on every push, such as a live health probe,
keeps running and says why. Not integrity-class: it changes no selection and no key.

Verify: the test fails on a fixture leg with neither key nor reason, and passes on the current hook
and Jenkinsfiles once each U leg and each unconditional stage has its row.

- [ ] **CD-no-fingerprint — a gate that declares inputs and no steps gets a fingerprint over its declared inputs, so the gates withheld only for "no fingerprint" can be judged for reuse** (serves push-gate-pays-once)

State: for `rust-style` the scoping receipt shows the cause. The gate declares inputs but no steps,
and `stepFingerprints()` returns `"gate-only inputs, no step fingerprint"`, so the whole input set
has no fingerprint (`genesis/orchestrator/build-manifest.json:241–279`;
`genesis/orchestrator/gate-evidence.mjs:96–105`). By the census table, 14 gates are withheld for "no
fingerprint" alone and 3 more name it beside another reason (3.4); those 3 stay withheld for the
other reason. Whether the 14 share rust-style's cause is not known, so the station's first step is
to classify them: inputs declared with no steps, or another cause, recorded as a table in this State
paragraph once done. The fingerprint over declared inputs applies only to the gates that share the
first cause. Shadow first: print the fingerprint each would carry and leave the withholding in place.
Turning reuse on for any of them changes which work is skipped and is integrity-class.

Verify: the classification table is in this paragraph; in shadow, each gate it lists under "inputs
declared with no steps" prints the fingerprint it would carry, two runs on an unchanged tree print
the same value, and a change to one declared input moves it. Only those gates are counted.

- [ ] **CD-ci-stage-selection — the orchestrator's Rust style stage and the other unconditional stages select on their whole comparison closure, running when evidence is missing** (serves push-gate-pays-once)

State: the `Rust Style Gate` stage has no `when` (1a), and 3.3 lists the other stages with no input
test. The scoping receipt judges selecting the rust-style stage on `*.rs` alone unsound, because a
Cargo, lint-configuration or governance-only change can require a paydown or weaken strictness
without touching a Rust file; inherited strict bindings, adapters and baseline changes can also alter
the result (`genesis/orchestrator/rust-style/src/repository.rs:89–100,482–487,530–581`).
The sound selector is the complete comparison closure, with missing evidence forcing a run; live
health and projection stages need a freshness policy instead (3.3). Shadow first; `on` is integrity-class.

Verify: each orchestrator build prints, in its own console log, a shadow line per stage naming
would-run or would-skip and its reason, and replays of a documentation-only, a Cargo-only and a
configuration-only commit show would-run for the last two. The Verify reads a Jenkins log and needs
no path to a device.

- [ ] **CD-key-moves — for each enrolled gate, a test mutates one declared input and one declared read and shows the key moves; a periodic run re-executes a sample of reused verdicts and compares** (serves push-gate-pays-once)

State: `gate-evidence.test.mjs` exists and the scoping receipt's plan extends it to every
independently mutated input; whether today's tests cover every enrolled gate is not established. Nothing re-executes reused verdicts. The industry research found no source for either
check as a named practice; they are this design's own. This design proposes the sample, and the
steward may change it: one reused verdict per dev landing, the one whose last real execution is
oldest, re-executed after the landing so that it does not delay one; a disagreement is a red finding
on that gate's key. Not integrity-class: these checks change no key and skip no work.

Verify: the mutation test passes for every enrolled gate, and for each of ten dev landings the oldest
reused verdict has a fresh run's verdict recorded beside it.

- [ ] **CD-fence-scope — a governance-changing push refuses a sibling verdict only for the gates whose own judge or recipe changed** (serves push-gate-pays-once)

State: today any governance path in a push refuses every sibling verdict. The fence's path list and
function are `SIBLING_REUSE_GOVERNANCE_PATHS` and `siblingGovernanceChange` at
`genesis/orchestrator/gate-evidence.mjs:480–507`, called from `genesis/orchestrator/gate-runner.mjs:194`
(read by the integrator on dev `2081b2532`). On 2026-11-23 ethosengine measured, on the first landing's
diff, 16 selected and enrolled projects: 1 had its own judge changed, and 15 were refused only by the
whole-push fence (its note to the integrator, recorded in the sprint journal
`genesis/docs/superpowers/sprints/2026-11-23-f4-ask/journal.md`). Its proposal, in five lines:

1. Refuse a sibling verdict for gate P only when the push changes the shared judge or P's own recipe set.
2. The judge is `gate-runner.mjs`, `gate-evidence.mjs` and the JS they import, as an import closure
   computed from the destination tip; P's recipe set is its manifest entry, recipe, executor and the
   `pool-policy.json` key paths its evaluation reads.
3. Attack A1, a misclassified path (a file the judge or a recipe depends on that the rule's table
   does not know): cured by computing the import closure, treating unknown `gate-*` and `suite-*`
   paths as universal, and feeding read-sets into P's recipe set.
4. Attack A2, self-blessing (the push itself narrows the rule that judges it): cured by reading the
   classification table and the inert key list from the destination only, a change to either universal.
5. Attack A3, an inert key that is not (a policy key treated as having no effect on a gate when it
   has one) [GAP: the sibling's note arrived cut off after "a pool-policy key marked inert"; its cure
   for A3 is not in this session's record. The gap does not block the shadow; it blocks turning the
   rule on]; a further option, judge-pinned reuse, lets a sibling verdict stand when produced under
   the destination tip's judge.

Integrity-class; not built. The plan's station Fence-scope (section 8) is the same work; this entry
is a cross-reference to it.

Verify: on a governance-changing landing, a shadow line names, per gate, whether the narrowed rule
would have accepted the sibling's verdict; across ten such landings, no gate it would have accepted
differs in result from the run that was paid. If the window supplies fewer, the station stays open
and says how many it has seen; it is not closed on fewer.

- [ ] **CD-jenkins-shadow — beside a gate's real reuse decision the runner prints whether Jenkins holds a green for that exact closure, and admits nothing** (serves push-gate-pays-once)

In short: Jenkins already runs some quality checks in gate-only mode, and nothing shows a lander
whether such a run matches its own closure. This station prints that comparison and decides nothing.

State: from the Jenkins receipt, whose claims concern source and not demonstrated runtime behaviour.
What exists: the Jenkins bridge translates finished stages into process and flow events under an
ExternalClaim marked unattested (`bridges/jenkins/src/translate.rs:361,439`), and observation needs an
active offer (`bridges/jenkins/src/observe.rs:247`). The offer declares `claims: none`, `settlement:
none` and `crossesNetwork: false` (`bridges/.epr-meta/offers/jenkins-edge-pipeline.offer.md:15`), so
it claims and settles nothing. In gate-only mode the Edge pipeline makes storage and doorway quality
failures blocking (`elohim/holochain/Jenkinsfile:541`), under branch authority (`:2597`, `:3485`).
None of this is the lander's gate method or reusable gate authority.

The receipt names seven missing nodes between a posted miss and a reused Jenkins verdict:

1. Bounded exposure: one admitted device shows Jenkins only the authorized intent, project, closure and ref (P).
2. A capacity and grant check before a pod is requested; today's Edge guard checks only the trigger cause (`elohim/holochain/Jenkinsfile:2746`).
3. A signature binding method, closure, result and grant; today `agentId` is optional (`elohim/eprfs/epr-cli/src/peer.rs:1593,1623`).
4. An attributed relay by one designated admitted device, since `via` events cannot re-cross (`elohim/eprfs/eprfs-peer/src/allow.rs:118`).
5. Proof of the same method at the commit, recipe, toolchain and environment; a matching commit is not the closure (`genesis/orchestrator/gate-evidence.mjs:367`).
6. Under-grant wording: the fences stay, and a carrier's verdict is required not to print `sibling` (`gate-evidence.mjs:588`; `gate-runner.mjs:189,754`).
7. Telling outcomes apart: a completed failure is red; an interruption, refusal or `NOT_BUILT` is no verdict (`bridges/jenkins/src/translate.rs:174`).

The receipt's first slice is this read-only shadow: `bridges/jenkins/src/translate.rs` and its tests,
a small `genesis/orchestrator/gate-jenkins-shadow.mjs`, and a line beside the gate runner's real
decision that reads "Jenkins holds a green for this closure" only with closure-bound evidence,
followed by "shadow; not admitted for reuse", and otherwise says why no comparison is available. A
contract test backed by a fake is to show that a wrong closure or method, absent evidence, an abort,
a red, an unavailable service and an invalid grant change no decision and settle no miss. It adds no
durable entity (the comparison is rebuilt from existing observations) and no DNA change. Not
integrity-class in shadow; anything that lets a Jenkins verdict decide is.

Before anything decides, the steward would need three rulings, in the receipt's words: "Which
sponsored service agent may carry which gates, under whose grant, bounds and revocation rule?"; "May
its verified under-grant verdict replace local computation, while contributing zero independent
witnesses?"; and "Approve the wording `carrier: Jenkins; basis: under-grant; independence: none`?"

Verify: on one landing the line prints for a closure Jenkins built in gate-only mode, and the
landing's decisions are byte-identical with the shadow on and off. If no landing supplies such a
closure, the station stays open and says how many landings it has seen; it is not closed without one.

## 7. What this design refuses, so that a later reader does not add it

- **Skipping the Rust style check on commits that touch no Rust.** The scoping receipt judges it
  unsound: Cargo, configuration and governance-only changes can require paydown or weaken strictness
  (`genesis/orchestrator/rust-style/src/repository.rs:530–581`). Documentation-only commits are
  already handled without it.
- **A count of "bespoke" detectors as a measure.** It rewards deleting implementors, not removing
  waste. Count unresolved duplicate records, or steps with no key and no reason.
- **Raising any ceiling to make a number fit.** A ceiling that a repeated step breaks is reporting the
  repetition.
- **A new ledger or register beside the flow store.** The plan and the repository's rule (section 4)
  refuse it. The rows of section 4 are observations in the flow store, and the always-run reasons sit
  beside the existing withheld list.

These are design positions for the steward to ratify; nothing enforces them but this text and the
reviews of the stations.

### A third miss, found on 2026-11-24

The household mesh's preflight refuses a DNA's wasm as stale when the wasm's build time is older
than the date of the last commit that touched the zome's directory
(`app/elohim-app/scripts/hc-mesh.sh`, the stale-hApp check). On dev `93983b057` it refused all
five DNAs, and a rebuild found every wasm fresh: the commit it compared against added a licence
field to each `Cargo.toml`. It is the Rust style miss in mirror image: there a step ran with
nothing changed and nobody saw the cost; here a change test fired on a change that was not one,
and the cost was a household lane judged unavailable for most of a night. It belongs to the
census as a change test with no closure behind it, and to CD-reading as a step whose refusal
should name the input that moved. Integrator's first-hand read; the script's lines were not
re-read for this note.

## 8. Open questions for the steward

1. Start CD-reading and the three shadows (read-selects, no-fingerprint, CI stages) before any `on`?
2. Who owns the always-run registry of CD-always-run?
3. What warning margin should the reading print for a step whose key has not changed in N landings
   while the step still runs: what is N?
4. The three rulings CD-jenkins-shadow quotes from the Jenkins receipt (which service agent may carry
   which gates; whether its verdict may replace local computation; the wording of its line), needed
   before any Jenkins verdict decides anything.

## 9. What was not established

**Bears on the stations.**

- Costs, beyond one observation of 29 seconds for one full rust-style scan: no per-ref or scoped
  scan, no other always-run leg and no CI stage in 3.3 has a recorded cost; repetition counts and
  savings are unmeasured.
- Whether the per-ref scan and the cargo gates' scoped comparisons share a baseline and tree, and
  whether the 14 "no fingerprint" gates share rust-style's cause.
- For CD-jenkins-shadow: live offer standing, a usable service key and grant, idle-capacity
  observation and exact equality of CI and local closures (the receipt's own undetermined list).
- The `rust-style` habit's earlier ledger entries, including a prior false-green incident.

**Bears on the inventory.**

- That the receipts' different heads (`e14abce95`, `2081b2532`) do not matter: the integrator states,
  from the landing's contents and not line by line, that the commits between them are ethosengine's
  carried column and status line and touch neither the Rust style gate nor the runner's style call.
- The `cecab9d08` incident rests on the integrator's first-hand account; the census explains it from
  source lines and did not verify the historical execution.
- Marked PLAUSIBLE by the census: the mapping of `brit diff` to the tree diff it read; likely misses
  2 to 5 of 3.3, ranked by structure and not by cost (the first is now read by the integrator); that
  no universal census of work with no detector exists.
- The tools section 5 lists as not researched; any source for sampled re-execution or input mutation;
  anything beyond snippets for items marked [S]. The fence proposal's copy here is cut off in three places.
