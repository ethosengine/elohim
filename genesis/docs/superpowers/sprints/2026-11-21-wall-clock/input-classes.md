---
id: wall-clock-input-classes
title: "Input classes: sources and buildProcess kept apart natively"
date: 2026-11-21
---

# Input classes: a step's sources and its buildProcess, kept apart natively

Written 2026-11-21 on ethosengine (session `6d68315a…`, Claude Opus 5.5), before implementation,
for the wall-clock sprint. Serves plan §9's fourth test, "the unit on offer is a step keyed on
exactly what its recipe reads", and arrival backlog row 42. The interface below is what the
two submodule branches implement. The release train (rakia publish, brit dependency bump and
release, both gitlink moves on dev, then the JS consumers) is ordered by shem and the steward
after reading this record.

## The rule

A gate is keyed on its step's sources and its own recipe. A step's `buildProcess` is an input of
the executor that runs that process, which for a pipeline file is CI. CI's dispatch keeps both
classes. A device gate asks for sources alone.

## What folds them together today (at the pinned commits)

| Site | Pin | What it does |
|---|---|---|
| `elohim/rakia/rakia-core/src/constellation.rs:187-209` (`plan_from_changes`) | rakia `f9787c9e` | Extends `source_patterns` with `build_process`, matches changed paths against the union, and records one `changedFile` reason per step (`break; // one match is enough`). Nothing says which class matched. |
| `elohim/brit/brit-cli/src/commands/fingerprint.rs` (~52-53, 87) | brit `4eb2e8cc` | The same union feeds `ContentFingerprint::from_repo_globs`. |
| `elohim/brit/brit-cli/src/commands/affected.rs` | brit `4eb2e8cc` | Prints `affected_by: Vec<rakia_core::generated_types::AffectedReason>`, the type generated from rakia's build-plan schema. |

## Finding: `buildProcess` out alone does not cure row 42

Five steps list a `buildProcess` file that their own `sources` glob also covers:

| Manifest | Step | buildProcess | Covered by |
|---|---|---|---|
| `elohim/eprfs/build-manifest.json` | `rust-build-test` | `elohim/eprfs/Jenkinsfile` | `elohim/eprfs/**` |
| `elohim/epr/build-manifest.json` | `rust-build-test` | `elohim/epr/Jenkinsfile` | its sources glob |
| `elohim/brit/build-manifest.json` | `brit-ci` | `elohim/brit/.github/workflows/ci.yml` | its sources glob |
| `elohim/rakia/build-manifest.json` | `rakia-ci` | `elohim/rakia/.github/workflows/ci.yml` | its sources glob |
| `app/elohim-library/build-manifest.json` | `build-storybook` | `app/elohim-library/Jenkinsfile` | its sources glob |

So the sources class of a step is **its sources minus its own buildProcess**. A path that a step
names as its own pipeline file belongs to that step's executor even when a broad source glob
also covers it. This needs no manifest change: the manifest already says which file is the
pipeline file. Another step that lists the same path as a source still gets it as a source (the
subtraction is per step, never global).

Row 42's delta names four layers holding the pipeline file in the device's key. This change
covers the first two natively: selection (rakia, brit affected) and the step fingerprint (brit
fingerprint). The other two, the JS project-tree material in `gate-evidence.mjs` and the bin
store's whole-tree source state in `device-runtime`, are not touched here and still hold it.

## Interface

### Schema (rakia): `elohim/rakia/schemas/v1/build-plan.schema.json`

One optional property is added to `$defs.affectedReason`:

```json
"inputClass": {
  "type": "string",
  "title": "InputClass",
  "enum": ["sources", "buildProcess", "upstream"],
  "description": "Which input class selected the step. Set only by a classified plan; absent in the legacy plan."
}
```

Generated Rust (`rakia-core/src/generated_types.rs`, regenerated with `pnpm run rakia:codegen:rs`
and checked by `rakia:codegen:rs:verify`):

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub input_class: Option<InputClass>,   // serde: "inputClass"

pub enum InputClass {
    #[serde(rename = "sources")] Sources,
    #[serde(rename = "buildProcess")] BuildProcess,
    #[serde(rename = "upstream")] Upstream,
}
```

`planVersion` stays `"1.0"`: the field is optional, and a plan that does not set it serializes to
exactly today's bytes. No manifest schema change is needed (`$defs.stepInputs` already requires
both `sources` and `buildProcess`).

### Planner (rakia-core)

- `plan_from_changes(&Constellation, &[String])` is unchanged: union, first matching path, no
  `inputClass`. Every existing caller keeps today's output.
- New: `pub enum InputSelection { Sources, All }` and
  `pub fn plan_from_changes_classified(&Constellation, &[String], InputSelection) -> Result<TopoPlan, ConstellationError>`.
  - Each class is matched separately. A directly affected step gets one `changedFile` reason
    per class that matched, in the fixed order sources, buildProcess. Each reason's `path` is the
    first matching changed path in caller order **within that class**. A path that matches both
    of one step's classes (its own pipeline file under a broad source glob) is subtracted from
    that step's sources and yields the buildProcess reason alone; test 4 pins it. Corrected
    2026-11-21 after shem's review: an earlier draft said such a path yields both reasons.
  - Sources class = the step's `sources` globs, minus paths matched by its own `buildProcess`.
  - buildProcess class = the step's `buildProcess` entries with a trailing `@functionName`
    suffix stripped before matching (only a final `@` followed by an identifier, so a path such
    as `@scope/pkg` is kept whole). The legacy planner keeps passing the suffix to the glob, as
    today.
  - `Sources` drops the buildProcess class **before** propagation is seeded, so a step selected
    only through its pipeline file selects neither itself nor its dependents.
  - Upstream: a selected step gets one `upstreamNode` reason, `inputClass: "upstream"`, for each
    immediate dependency that is selected, including when the step is also selected directly.
    The legacy planner keeps recording only the first upstream of a step it reaches only
    transitively.
- One shared rule returns a step's input patterns for a selection, so brit's fingerprint and the
  planner cannot drift: `inputs::step_input_patterns(sources, build_process, selection) ->
  InputPatterns { include, exclude }`. `All` is sources plus stripped buildProcess entries;
  `Sources` is sources, excluding the stripped buildProcess entries.
- Compatibility: `AffectedReason` gains a public field and is not `#[non_exhaustive]`, so code
  outside rakia that builds it with a struct literal stops compiling. Only rakia's own tests do
  that today. The rakia release (0.1.2) says so in its note.

### brit

- `brit build affected --inputs <sources|all>`. Flag omitted: the legacy planner and today's
  bytes. Given (either value): the classified planner, every reason carrying `inputClass`.
  `all` selects the steps today's planner selects, with provenance, with one difference: a
  `File@fn` buildProcess entry is matched as `File` (the legacy glob keeps the suffix and
  matches nothing). Shem scanned 2034 entries on dev on 2026-11-21 and found none with `@`.
  `sources` selects from sources alone.
- `brit build fingerprint --inputs <sources|all>`. Flag omitted: the legacy union and the same
  CID and JSON as today. Given: rakia's `step_input_patterns`, the rule the classified planner
  matches, so a change that selects a step under a selection also moves its key under it (a
  `File@fn` pipeline entry moves the `all` key; the legacy union's suffixed glob matches no
  file). `sources` hashes the step's sources minus its own buildProcess, so the pipeline file's
  bytes are not in the fingerprint. Corrected 2026-11-21 after shem's review: an earlier draft
  defaulted to `all` and hashed the unstripped entry. A new `ContentFingerprint::from_repo_globs_excluding(repo, commit,
  include, exclude)` in brit-graph carries the subtraction. `from_repo_globs` is unchanged.
- `brit build plan` is unchanged (legacy planner, legacy fingerprint).
- The JSON key added is `inputClass`, inside each `affected_by` entry. Its spelling comes from
  the generated type. (The surrounding brit keys stay snake_case, as today.)

### For the JS consumers (shem's lane, after release)

- `gate-oracle.mjs`: add `--inputs sources`. Its `source: ${reason.path}` label stops lying for
  a pipeline-only match. `upstreamNode` reasons keep their `upstream` field.
- `gate-evidence.mjs`: add `--inputs sources` to `build fingerprint`. This re-keys every
  reusable verdict once, at the roll.
- `gate-runner.mjs`'s path fallback (`graph-walker.mjs:104`) also matches `inputs.buildProcess`
  and needs the same rule.

## Tests promised (each repo, its own toolchain)

1. A pipeline-file-only change marks the step through buildProcess, and under `--inputs sources`
   neither selects it nor moves its fingerprint.
2. A source change does both.
3. A file that is a source of one step and the buildProcess of another is reported per step:
   `sources` on the first, `buildProcess` on the second.
4. A step whose sources glob covers its own pipeline file (the eprfs shape) is not selected by
   that file under `sources`, and its sources fingerprint is unchanged by it.
5. Upstream propagation from a buildProcess-only step selects no dependents under `sources`.
6. Default output (flag omitted) is byte-identical to today's for the same input, pinned as a
   literal string in a test.

## Record

Implemented 2026-11-21 on ethosengine. The write legs were Codex gpt-5.6-sol from this
record, and the design survey was Codex gpt-6.2-astra. The session model reviewed the work and
ran every build and test. Nothing is published, released, tagged or pinned.

| Repo | Branch (pushed to its own origin) | Head | Base |
|---|---|---|---|
| ethosengine/rakia | `ethos/input-classes` | `8a4f184a` (on `158f2365`) | the dev pin `f9787c9e` (on `run-ci/regenesis-1-schemas`, not `main`) |
| ethosengine/brit | `ethos/input-classes` | `10fa5cac` (on `3f6b0c8d`) | the dev pin `4eb2e8cc` (brit 0.3.0) |

rakia, on its CI toolchain (stable): `cargo fmt --all --check` exit 0. `cargo clippy --workspace
--all-targets -- -D warnings` exit 0. `cargo test --workspace` exit 0, 50 passed and 0 failed
at `8a4f184a` (49 at `158f2365`), including the new tests in
`rakia-core/tests/input_classes_test.rs` and `src/inputs.rs`.
`rakia:codegen:rs:verify` exit 0, and `node --test schemas/tests/*.test.mjs` passed 3 of 3.

brit at `10fa5cac`, on its pin (1.100.0), built against rakia `8a4f184a` (0.1.2) through a
non-committed `--config patch.elohim.{rakia-core,rakia-brit}.path=…` with `--offline`. For
that run the lock's two rakia entries were rewritten to the patched form, and Cargo.lock was
restored after it:

| Check | Exit | Result |
|---|---|---|
| `cargo fmt --all --check` | 0 | |
| `cargo clippy --all-targets -p brit-cli -p brit-graph -p cli-journey -- -D warnings -A unknown-lints --no-deps` | 0 | |
| `-p brit-graph` tests | 0 | 23 passed |
| `-p brit-cli` tests | 0 | 114 passed, 5 ignored |
| `-p cli-journey --test input_classes` | 0 | 6 passed |
| `--test rakia` | 0 | 8 passed |
| `--test memory_build` | 0 | 4 passed, 1 ignored |

All six promised tests are covered. Test 6 pins the flag-omitted `build affected` output, and
the rakia planner's legacy serialization, as literal strings.

Not verified, and why:

- **rakia-executor was not compiled.** Its `elohim-epr =0.1.0` comes from the Nexus `elohim`
  registry, and cargo refuses that registry without a credential provider. This device holds no
  credential, and its cache has only `elohim-epr` 0.2.0. The rakia run dropped the member from
  the workspace for that run only. The crate is not touched by this change, and rakia's GitHub
  CI builds it.
- **Two parts of brit's suite did not run.** These are the exhaustive `just clippy` feature
  matrix (`small`, `max-pure`, `lean-async`) and the inherited gitoxide QA (nextest).
- **The brit branch does not build against the registry yet.** It needs the rakia publish
  first. brit-cli requires rakia-core and rakia-brit 0.1.2, and its Cargo.lock still pins
  0.1.1, so the brit branch does not resolve `--locked` until the release bumps the lock. The
  rakia branch bumps `[workspace.package].version` to 0.1.2. On `main` that bump is what
  publishes, so merging the rakia branch is the release step.
- **Device trap, for whoever repeats this.** A leading `cargo --config … clippy` is not
  forwarded to the external cargo-clippy subcommand, which then checks the registry crate. Put
  `--config` after `clippy`.

Review. Shem read both branches independently (no build) and sent five advisories, all
applied: `strip_function_suffix` strips only a trailing `@identifier`; `fingerprint --inputs`
is optional and, when given, keys through `step_input_patterns`; brit requires rakia 0.1.2;
the both-classes claim above is corrected; the 0.1.2 note names the struct-literal break, and
rakia's CLAUDE.md documents the classified planner. Shem's `324dcb4e6` moves five recipe-read
files in `gate-storage-client-ts` and `gate-eprfs-ts` from buildProcess to sources, which
adoption needs.

Found on the way. Commit `101b7b767` (the build hook's seat count) shrank
`genesis/data/timeline/backlog/steward-device-arrival-open-items.md` from 1531 lines to about
40, and the sprint's `journal.md` by about 573 lines. Its message names neither file. Rows 16
to 62, row 42 among them, exist only at `101b7b767^`. Shem found landings 7 to 10 did the same (an append that opened
for write before reading) and repaired both files in `b83575402`.

Release train, 2026-11-21 21:50Z (steward authorized in this session: "Yes, run the release").
rakia PR #4 (`ethos/input-classes` to `main`, a fast-forward of three commits: `f9787c9e`,
`158f2365`, `8a4f184a`) ran CI, and its `test` check failed. The input-class tests pass. The
failure is `rakia-executor/tests/contract.rs:34`
(`canonical_identity_matches_the_javascript_oracle_including_null`): it expects the blake3
artifact CID `bafkr4igyfkfu6h3vwzbc3zsulgdn6okd2eh6ovuqiyouihuyxrgt7s4ude`, and the executor
emits the sha256 CID `bafkreigyfkfu6h3vwzbc3zsulgdn6okd2eh6ovuqiyouihuyxrgt7s4ude`.
`f9787c9e` ("executor kernel swap staged") changed that file. It has no CI run of its own,
and it is the commit dev pins. The swap waits on elohim-epr 0.2 (the test file's TODO). The PR
is open and unmerged, and nothing is published. Either the swap finishes, or the assertion is
staged with its ignored sibling. That decision belongs to the executor-swap lane.
