# Sweettest as the DNA-Level Integration Test Layer

**Status:** Approved 2026-04-22. Ready for implementation plan.

**Context:** Wave 1 landed a sweettest scaffolding crate (`elohim/holochain/tests/sweettest/`) with 6 thin tests, all of them `#[ignore]`'d without packed DNA artifacts. That proved the framework compiles and runs in our Che devcontainer and in Jenkins Nix builds, but it did not prove testable value. This spec defines what sweettest is *for*, what it covers, and how we enforce it so the value is predictable at development time rather than discovered in Jenkins.

## Problem

Today we have two test layers: Rust unit tests inside each DNA workspace, and A2O (cucumber-js + Playwright) against deployed doorway + storage + packaged DNAs. There is nothing between "my zome compiles" and "A2O failed three hours after I pushed." Bugs that live in that gap — validation regressions, extern signature drift, cross-agent propagation breakage — are caught only after deploy, if at all.

Jenkins is expensive. Catching DNA-level regressions *before* the pipeline runs is a first-order design goal.

## Non-goals

- **Replacing A2O.** A2O is black-box acceptance against deployed infrastructure. Sweettest is white-box integration against in-process conductors. They live at different layers and do not compete.
- **Gherkin-driving sweettest.** Rust cucumber over sweettest was considered and parked. Sweettest stays Rust-native for now; revisit if a future Wave needs shared vocabulary.
- **Testing third-party DNAs.** Sweettest verifies *our* DNAs. Interop against Moss / holo-host / external apps is a separate future concern.
- **Replacing zome-level unit tests.** Pure logic stays in each DNA's `cargo test`. Sweettest is the integration layer.

## Test-layer positioning

Sweettest is the missing middle layer.

| Layer | Surface | Typical duration | Runner | Target |
|---|---|---|---|---|
| **Unit** (`cargo test` in zome crates) | Pure functions, HDI types, validation logic in isolation | ms | `cargo test` in each DNA workspace | Logic correctness |
| **Sweettest** (this work) | In-process conductor, real DHT, real validation, cross-agent | s–min | `cargo test` in `tests/sweettest/` | DNA integration — zome externs, validation rules, propagation |
| **A2O** (`genesis/a2o/`) | Deployed doorway + storage + packaged DNAs + browser | s–min per scenario | `cucumber-js` | Deployed-system acceptance |

The quality bar is **Requests & Offers parity** (hApp-specific sweettest suite widely used in the Holochain community as the canonical integration-test reference).

## Coverage model — hybrid

### Floor (mechanical, enforced by sync-rule)

Every `#[hdk_extern]` in a coordinator zome has at least one sweettest that exercises it with realistic inputs and checks the happy-path result. Every `must_*` validation check in an integrity zome has at least one rejection test that constructs an invalid action and asserts validation rejects it.

Target volume at full coverage: roughly 40-60 tests across the 5 DNAs. Exact count tracks the extern + validation-rule inventory.

### Focal points (narrative, authored deliberately)

Each Wave names a small set (2-4) of cross-agent, propagation, or failure-mode scenarios that matter to the Wave's story. Focal points are authored alongside the Wave's feature work, not as a retroactive backfill.

Examples (Wave 1):
- "Bootstrap-steward identity persists across conductor restart."
- "Non-steward agent's bootstrap-only action is rejected at validation time on a second agent's conductor."

### Test-file layout

One file per DNA: `tests/sweettest/src/tests/<dna>.rs`. Tests within a file are grouped by module:

```rust
mod extern_coverage { /* per-extern happy-path tests */ }
mod validation_rejection { /* per-must_* rejection tests */ }
mod wave1_scenarios { /* named focal-point scenarios */ }
mod wave2_scenarios { /* added as waves progress */ }
```

No new test-binary targets per extern. No explosion of files. Test names describe behavior, not mechanism (`bootstrap_steward_identity_survives_restart`, not `test_restart_1`).

## Enforcement — sync-rule + husky check

The user directive is a *noisy* husky: catch issues before Jenkins, not after. Two complementary pre-push gates on zome-code changes.

### Gate 1 — sync-rule nudge (matches existing pattern)

Add a new sync relationship alongside the existing `model-sync`, `a2o-sync`, `testid-sync`:

```
zome-sweettest-sync:
  description: Zome source changes should have matching sweettest updates.
  patterns:
    - elohim/holochain/dna/<dna>/zomes/**/src/*.rs
      → elohim/holochain/tests/sweettest/src/tests/<dna>.rs
```

**Behavior:** pre-push hook detects a zome file touched without its matching sweettest file touched, emits a nudge:

```
zome-sweettest-sync: you changed elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs
  consider updating elohim/holochain/tests/sweettest/src/tests/imagodei.rs
```

Informs rather than blocks — same policy as other sync rules. Invisible when zome files aren't touched.

### Gate 2 — `cargo check` on sweettest (compile-time drift catcher)

Register sweettest in `elohim/holochain/dna/build-manifest.json` as a new graph-walker project with a `cargo-check` executor gated on the same zome-change patterns as the sync rule:

```json
{
  "sweettest-check": {
    "description": "Compile-check sweettest against current zome sources — catches extern-signature drift before Jenkins.",
    "inputs": {
      "sources": [
        "elohim/holochain/dna/**/zomes/**/src/*.rs",
        "elohim/holochain/tests/sweettest/**"
      ],
      "buildProcess": []
    },
    "outputs": { "artifacts": [], "verify": null },
    "depends": [],
    "executor": { "stage": "Sweettest Compile Check", "function": null }
  }
}
```

Husky runs this gate when zome source or sweettest source is touched. Runtime: 30-90s warm, up to 8 min cold. The run cost is scoped to zome-change pushes, so TS-only or docs-only pushes are unaffected.

**Outcome:** the `DnaBundle::read_from_file` class of bug (extern signature / API drift) becomes a push-time failure, not a Jenkins-time failure.

### Cadence

Tests are authored alongside zome changes. TDD-first is preferred; bare-minimum is "same commit as the zome change it covers." Wave close-out does a coverage-floor check and escalates any gap remaining.

## Jenkins stage — full integration run

The stage added to `elohim/holochain/dna/Jenkinsfile` in Wave 1 post-work, `DNA Integration (bootstrap-steward)`, stays. It runs after `Build DNA` because sweettest needs packed `.dna` artifacts to exercise real install paths.

**Settings (confirmed):**
- `nix develop` shell for toolchain — avoids the Che-specific `BINDGEN_EXTRA_CLANG_ARGS` workaround
- `CARGO_BUILD_JOBS=2` — caps compile parallelism; holds the ~6-8 GB RAM link-phase spike
- `--test-threads=1` — serializes conductor spin-up; peak runtime RAM ~1 GB
- `--release` — slower compile, ~30% less runtime RAM
- `--include-ignored` — DNAs are packed upstream, tests that gated on artifact presence now run
- `timeout(time: 30, unit: 'MINUTES')` — hung conductor does not burn the agent
- Output filter: summary-only on pass, full panic context + archived log on fail
- Post-step temp cleanup of `sweet_conductor_*` dirs (panics can skip Drop)

**Rename:** drop the `(bootstrap-steward)` qualifier once focal-point scenarios extend past Wave 1.

## Resource envelope

### Jenkins agent pod (K8s)

| Setting | Value | Rationale |
|---|---|---|
| `requests.memory` | 6 Gi | Warm-build baseline |
| `limits.memory` | 10 Gi | Cold-link peak + headroom |
| `requests.cpu` | 2 | Baseline schedulable |
| `limits.cpu` | 4 | `CARGO_BUILD_JOBS=2` + tokio runtime stays under |
| Ephemeral storage | 10 Gi | `target/` + conductor temp dirs |
| `target/` PVC | recommended | Cache between builds; cold → warm compile saves ~6-8 min per run |

### Che devcontainer (ethosengine/che-devworkspaces)

Validated empirically 2026-04-22: 8m 35s full cold compile, tests discover and self-ignore cleanly. Image includes `cmake`, `clang-libs`, `zlib-devel`, and a patched `datachannel-sys/libdatachannel/CMakeLists.txt` injecting `find_package(ZLIB REQUIRED)` before `find_package(OpenSSL)`. Sets `ENV BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/20/include"` so bindgen finds the clang resource dir despite no `clang` driver binary being present.

Contributors working outside the devcontainer (not using Che, not using Nix) need to set `BINDGEN_EXTRA_CLANG_ARGS` manually and install cmake + clang-libs + zlib-devel. Document this in `tests/sweettest/README.md`.

## Failure modes this explicitly protects against

- **Zome extern signature change without test update** — caught at sweettest compile (Gate 2).
- **Integrity rule added without rejection test** — caught by sync-rule nudge (Gate 1), escalated at Wave close if coverage gap persists.
- **Cross-agent propagation broken by a refactor** — caught at runtime via focal-point two-agent scenarios.
- **"Works in Che, breaks in Jenkins"** — Che runs the same compile + test path, so divergence surfaces at push time.
- **Validation bypass via direct entry construction** — caught by rejection tests in `validation_rejection` module.

## Rollout plan

Paying down the Wave 1 thin tests is a recurring Wave deliverable, not a single sprint. The sync-rule enforces authoring alongside zome changes; Wave close-out verifies the floor.

- **Wave 1 (done):** scaffolding, bootstrap-steward floor (all 5 DNAs), one cross-agent propagation test (imagodei).
- **Wave 2:** extern coverage floor for `imagodei` (identity coordinator). Target: ~15 tests in `imagodei.rs`. Add 2-4 Wave 2 focal scenarios.
- **Wave 3:** extern + validation-rejection floor for `mishpat` + `lamad`. Add Wave 3 focal scenarios as the integrity rules land.
- **Wave 4:** `node_registry` + `infrastructure` extern floors. By end of Wave 4, per-extern floor is at ~100% across all 5 DNAs.

## Out-of-scope for this design

- **Coverage tooling / metrics dashboard.** We're not building a sweettest coverage reporter. The floor is verified by Wave close-out review, not a metric.
- **Flaky-test quarantine.** If test flakiness emerges, addressed ad-hoc per-test with `#[ignore]` + issue reference. No framework.
- **Parallel test execution in Jenkins.** Hold at `--test-threads=1` until pod memory limits are proven tolerant of >1 concurrent conductor. Revisit only on explicit need.

## References

- **Implementation basis:** `elohim/holochain/tests/sweettest/` (Wave 1 scaffolding)
- **Sibling fast-gate:** `elohim/holochain/tests/manifest-hygiene/` (pre-push schema contract test, 0.01s, no Holochain deps)
- **Jenkins stage:** `elohim/holochain/dna/Jenkinsfile` → `DNA Integration (bootstrap-steward)`
- **Graph walker:** `genesis/orchestrator/graph-walker.mjs`
- **Sync-rule prior art:** `model-sync`, `a2o-sync`, `testid-sync`, `doorway-sync` in `.claude/` sync registry
- **Che devcontainer:** `https://github.com/ethosengine/che-devworkspaces/blob/main/containers/rust-dev/Dockerfile`
- **Quality reference:** Requests & Offers hApp sweettest suite (lightningrodlabs)
- **Wave 1 plan §7:** `genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md`

## Decision record

- Approach **A** (Rust-native sweettest) chosen over **B** (cucumber-rs Gherkin) and **C** (sweettest as A2O backend). Rationale: A is lowest-friction path to R&O parity; B's shared-vocabulary benefit is deferred to a future Wave if needed; C is redundant because real multi-node deployments already drive A2O's peer scenarios.
- Coverage model **hybrid** (mechanical floor + named focal points) chosen over pure per-extern or pure per-scenario. Rationale: floor enforces defensive coverage via sync-rule; focal points keep Wave narratives legible.
- Enforcement **noisy-husky** (sync-rule nudge + compile-check gate) over silent-Jenkins-only. Rationale: Jenkins is expensive; catching issues at push time is a first-order design goal.
