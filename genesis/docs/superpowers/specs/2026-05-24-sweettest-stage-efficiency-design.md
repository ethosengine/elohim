# Sweettest Stage Efficiency — Design

**Version:** 0.1
**Status:** Draft (post-brainstorm, pre-implementation)
**Last updated:** 2026-05-24
**Owner:** Matthew (operator decisions); spec drafted in brainstorm session 2026-05-24
**Related:** `elohim/holochain/dna/Jenkinsfile` (stage source of truth); RCA #1225 (inline in Jenkinsfile, no separate doc); memory `project_pre_dispatch_hard_fail_post_dispatch_unstable`, `feedback_understand_orchestrator_substrate_before_changes`

---

## 1. Why this spec exists

The DNA Integration (bootstrap-steward) stage at `elohim/holochain/dna/Jenkinsfile:525-641` is the dev-velocity tax on every Holochain-touching push. Build #1268 hit the 90-min timeout at 83 min; the most recent passing run on 2026-05-24 benchmarked at **1h 14m** — clearing the ceiling by ~16 min, healthy enough not to fail but pressuring every iteration. The stage serializes 18 test binaries — each a real Holochain conductor + kitsune2 p2p + SQLite + keystore — and runs the full suite even when a push touches only one DNA.

The capability bar this spec underwrites: **a Holochain-touching push gates on minutes of CI, not hours, when only one DNA changed.** The full-suite path remains available for `dev`-branch consolidation, tagged pushes, and any push that touches shared scaffolding — selectivity does not become a regression vector.

### What this spec subsumes (and what it does not)

This spec **absorbs**:
- Restoration of the `sweettest-target-cache` PVC mount (disabled at commit `f0cac18c8` after build #1212 broke `hc dna pack`)
- Orchestrator changeset propagation to the elohim-holochain pipeline (the orchestrator gates pipeline dispatch correctly today but does not pass the changeset downstream, blocking per-DNA decisions)
- Per-DNA nextest filtering via in-source `@dna-scope` markers, harvested at filter-build time, preserving the existing four-test quarantine
- Time-boxed re-attempt of sccache for sweettest (disabled per RCA #1225; recurred 2026-05-19 at `a92d91c2b` after brief re-enable at `0b4055851`)

This spec **does not** address:
- Per-DNA Jenkinsfile pipelines (CLAUDE.md forward-references `mishpat/Jenkinsfile`, but the artifact doesn't exist; that's an architectural shift owed its own sprint)
- Compile-once / run-distributed via `cargo nextest archive` (architectural; deferred)
- The four already-quarantined tests (`epr_2b_batch_a_full_loop`, `create_and_list_succeeds`, `refresh_ttl_appends_timestamp`, `cross_agent_get_returns_none`) — they stay quarantined and their RCAs are out of scope
- sccache pipeline-wide adoption beyond sweettest (sccache is currently only wired into the DNA pipeline; broadening is its own work)

---

## 2. State of play (ground truth, 2026-05-24)

### 2.1 The stage

`elohim/holochain/dna/Jenkinsfile:525-641` runs a single `cargo nextest run` invocation against the standalone workspace at `elohim/holochain/tests/sweettest/` (18 `[[test]]` declarations in `Cargo.toml:16-86`). Discipline is set by:

| Setting | Value | Cost |
|---|---|---|
| `timeout` | 90 minutes (line 536) | Active ceiling |
| `--release` | true | Slower compile, ~30% less runtime RAM |
| `--test-threads=1` | true | Serialized — each test holds full conductor + SQLite + keystore |
| `CARGO_BUILD_JOBS=2` | true (line 600) | OOM cap — holochain link spikes 6-8 GB |
| `RUSTC_WRAPPER` | `unset` (line 591) | sccache off per RCA #1225 |
| `CARGO_TARGET_DIR` | unset (default `./target`) | PVC mount disabled at lines 133-141 |

The quarantine expression (line 637-641) excludes four tests by name. Any selectivity work must **compose with** this exclusion, not replace it.

### 2.2 The orchestrator gate

`genesis/orchestrator/orchestrator-strategy.mjs:32` declares `elohim-holochain` with `changePatterns: ['elohim/holochain/dna/', 'elohim/elohim-cache-core/', 'elohim/holochain/rna/', 'VERSION']`. The pipeline is **not** unconditionally dispatched — App-only or doorway-only pushes do not gate on sweettest today. Good news: that lever is already pulled.

But `genesis/orchestrator/Jenkinsfile:725-742` passes only `FORCE_BUILD`, `FORCE_DEPLOY`, `DEPLOY_ONLY` to the downstream pipeline. The changeset itself is **not threaded through** — so any per-DNA decision-making downstream needs an explicit new parameter.

### 2.3 The PVC

`genesis/manifests/nix-cache-pvc.yaml:77` provisions `sweettest-target-cache-holochain` (20Gi, `openebs-jiva-csi-default`, `jenkins` ns). The PVC is **still alive in cluster state**; only the pod's `volumeMount` is commented out. Commit `f0cac18c8` (build #1212) disabled the mount after setting `CARGO_TARGET_DIR=/cargo-target` **pod-wide** broke `hc dna pack` — pack hard-codes `./target` for `content_store.wasm` lookup, and the env var redirected the artifact out of pack's search path.

### 2.4 The sccache history

RCA #1225 lives inline at `elohim/holochain/dna/Jenkinsfile:586-590`: 32 of 1857 rustc invocations hit ENOENT on sccache build-script subprocess executables. Commit `efbac2938` (2026-05-09) first disabled. Brief re-enable at `0b4055851` (2026-05-14). Re-disabled at `a92d91c2b` (2026-05-19) after the same ENOENT class recurred — 32/1857 again. The comment in the Jenkinsfile reads "Re-enable when tiered-quilt substrate hardens that path." This is **misleading**: the bug is sccache 0.15.x's build-script cache-hit serialization, not a substrate-storage problem. Tiered-quilt landed 2026-05-11 (per memory `project_tiered_quilt_spec_landed_2026_05_11`); the bug remains.

### 2.5 The test surface

16 of 18 binaries declare DNA scope unambiguously via `const DNA: &str = "..."` or a single zome import. The two ambiguous cases:
- `recovery_m3` — uses generic `load_dna()`, no static binding to a DNA in the file header
- `qahal_collab_t0_test` — uses the imagodei DNA at runtime but is conceptually qahal (and qahal-as-DNA doesn't exist yet; qahal is a coordinator inside imagodei)

These two need operator decisions before commit. The other 16 follow from the table in §3.3.

---

## 3. Design

Four waves, each independently shippable, each with its own rollback path. None depends on a later wave being merged.

### 3.1 Wave 1 — Scoped `CARGO_TARGET_DIR`, PVC mount restored

Re-enable the `sweettest-target-cache` volumeMount at `elohim/holochain/dna/Jenkinsfile:90-92`. Scope `CARGO_TARGET_DIR=/cargo-target` to the sweettest `sh` block only — **never** pod-wide, **never** in the stage `environment {}`. DNA-pack stages continue to use `./target` exactly as today.

```groovy
sh '''
  cd elohim/holochain/tests/sweettest
  CARGO_TARGET_DIR=/cargo-target cargo nextest run --release ...
'''
```

A one-line comment names the discipline so the next person doesn't lift the env var back to pod scope.

**Why this works:** the original break (#1212) was specifically that the pod-wide env redirected `hc dna pack`'s search. Pack runs in a different `sh` block in a different stage; scoping the env var to the cargo invocation leaves pack's environment untouched. The PVC itself was never the problem.

**Payoff:** compile drops from ~30 min (cold) → ~10 min (warm second consecutive build). Stage total ~74 → ~54 min.

**Rollback:** revert the two-line scoping. PVC stays provisioned for the eventual next attempt.

### 3.2 Wave 2 — `CHANGED_PATHS` passthrough from orchestrator

Add a `stringParam(name: 'CHANGED_PATHS', value: changedPaths.join('\n'))` to the `elohim-holochain` build at `genesis/orchestrator/Jenkinsfile:725-742`. Declare the parameter in the downstream Jenkinsfile's `parameters {}` block. Empty string → full-suite fallback (preserves today's behavior for manual / `FORCE_BUILD` / tag-pushed builds, which have no inherent changeset).

Extend `genesis/orchestrator/test/orchestrator-strategy.test.mjs` to assert the parameter is included in the dispatch payload — this is the "drift test" the memory `feedback_understand_orchestrator_substrate_before_changes` instructs to read before changing orchestrator substrate.

**Why this works:** the orchestrator already computes the changeset to do its path-matching. Threading it through is one parameter, no new computation. Downstream gets the signal without inheriting the path-matching logic — the gate stays in the orchestrator, the filter lives in the DNA Jenkinsfile.

**Payoff:** none on its own. Infrastructure for Wave 3.

**Rollback:** remove the parameter; downstream ignores absent/empty values.

### 3.3 Wave 3 — `@dna-scope` markers + nextest binary filter

Each of the 18 binaries in `elohim/holochain/tests/sweettest/src/tests/` gets a top-of-file marker:

```rust
//! @dna-scope: imagodei
// or for cross-DNA:
//! @dna-scope: imagodei, lamad
```

Tests without a marker **default to always-run** (safety). A pre-push check fails if any file under `src/tests/` lacks the marker — prevents drift.

**Starting mappings:**

| Binary | Scope |
|---|---|
| `imagodei`, `imagodei_peer_binding`, `imagodei_sign_for_agent`, `portal_host_crud`, `submit_specialist_revocation`, `attention_tending` | `imagodei` |
| `mishpat` | `mishpat` |
| `lamad`, `manifest`, `feedback_signal`, `attestation_coordinator`, `stake_class_gate`, `epr_phase_2b_batch_a_e2e` | `lamad` |
| `node_registry` | `node_registry` |
| `infrastructure` | `infrastructure` |
| `recovery_m3` | **operator decision** — uses `load_dna()` generically |
| `recovery_m4` | `imagodei` (per file comment) |
| `qahal_collab_t0_test` | **operator decision** — uses imagodei DNA but is conceptually qahal |

A ~30-line shell harvester runs in the Jenkinsfile before the cargo invocation:

1. Parse `params.CHANGED_PATHS` for paths under `elohim/holochain/dna/<dna>/**` → set of changed DNAs
2. `grep '^//! @dna-scope:' src/tests/*.rs` → binary → DNAs map
3. Intersect: binaries whose scope set overlaps the changed-DNAs set
4. Compose nextest expression:
   ```
   -E "(binary(b1) | binary(b2) | ...) and not (test(=epr_2b_batch_a_full_loop) | test(=create_and_list_succeeds) | test(=refresh_ttl_appends_timestamp) | test(=cross_agent_get_returns_none))"
   ```
5. **Fallback to full suite** if:
   - `CHANGED_PATHS` is empty (manual / force / tag)
   - Any changed path is outside `dna/<dna>/**` (e.g., `elohim-cache-core/`, `rna/`, `VERSION`, the sweettest harness, any `Cargo.*`)
   - No markers matched (defensive — would mean every test binary lacks a scope tag)

**Why this works:** the in-source marker is grep-discoverable from outside the Rust compiler, so the harvester is a five-line shell snippet rather than a parser. The default-to-always-run posture means a forgotten marker creates redundant work, never a missed failure. The pre-push check makes "forgot to add a marker" a local-feedback signal, not a production-CI surprise.

**Payoff:** single-DNA push (the common case — current push touched only imagodei) drops from ~44 min of test wall-time to ~5-10 min. Stage total: **~15-20 min**.

**Rollback:** drop the harvester; cargo invocation reverts to today's full-suite expression. Markers stay in source as harmless documentation.

### 3.4 Wave 4 — sccache RCA-driven decision (revised post-investigation 2026-05-24)

**The RCA changed the framing.** Investigation of Jenkins build #1225 (the post-`0b4055851` run with stats capture) revealed:

- The actual cargo error is `could not execute process \`sccache rustc ...\` (never executed) → No such file or directory (os error 2)`. **The binary that fails to spawn is `sccache` itself, not `build-script-build`.** Build scripts are a red herring — the same spawn-ENOENT hits regular library compiles like `hashbrown` (lib).
- Failure rate: 32 of 1857 rustc invocations (~1.7%) on #1225, *after* 1548 prior invocations of the exact same `sccache` binary succeeded in the same build. The binary is on PATH; spawn intermittently fails at the syscall level.
- Cache stats are healthy: 0 write errors, 0 cache read errors, 85.52% overall hit rate. The substrate (S3/MinIO via tiered-quilt) is not the blocker.
- Matches upstream sccache **issue #2023** (closed 2024 — "Intermittent CI failures `error: No such file or directory (os error 2)`"; symptom class recurs) and **issue #2687** (open Apr 28 2026 — different exact mechanism but same ENOENT-under-load family).
- The diagnostic plumbing in `0b4055851` did not actually capture daemon output — the archived `dna-integration-sccache-daemon.log` from #1225 is **0 bytes**, because `SCCACHE_LOG`/`SCCACHE_LOG_FILE` in sccache 0.15 are client-side env vars, not daemon-side. The original investigation's data was thinner than it appeared.

Full RCA memorialized at `.claude/memory/feedback_sccache_spawn_enoent_rca.md`.

**Wave 4 becomes a three-step decision, not a single attempt:**

**4a — Comment correction (always do, ~5 min work).**
Update the Jenkinsfile comment at lines 586-590. The current text ("Re-enable when tiered-quilt substrate hardens that path") is misleading — tiered-quilt is irrelevant. Replace with a pointer to the RCA memory entry and the upstream issue numbers. Even if no other Wave 4 work happens, future operators stop being misdirected.

**4b — sccache 0.14.0 downgrade A/B (low risk, ~1 day with observation).**
sccache 0.15.0 (released 2026-04-30) is the latest. The bug may have intensified in 0.15. Pin sccache 0.14.0 (released 2026-02-09) in the agent image, re-enable `RUSTC_WRAPPER=sccache` for sweettest, watch 3 consecutive `dev` builds. If clean → keep 0.14.0 pinned. If recurs → revert and accept that sccache cannot be relied on for sweettest in any current version.

**4c — Upstream issue filing (always do, ~1 hour work).**
File a sccache GitHub issue with the build #1225 reproducer signature: stats artifact (1857 executions, 32 spawn failures), the verbatim cargo error block, the Holochain dep tree shape. Reference #2023 and #2687. This is the "right road" fix — Mozilla can't address what they can't reproduce, and our environment is a useful repro.

**Decision logic after 4b:**
- 0.14.0 clean → land it, stage drops to ~10-15 min single-DNA / ~50 min full-suite.
- 0.14.0 recurs → sccache stays disabled for sweettest. The target-cache PVC (Wave 1) is the cache mechanism for sweettest. Stage stays at ~15-20 min single-DNA / ~54 min full-suite (still a big win vs today's ~74 min). Other DNA stages keep their working sccache wiring.

**Corrected recurrence classifier** (use if 4b lands and you want an auto-flag for future regressions):

```groovy
post { failure {
    script {
        def hits = sh(returnStdout: true, script: '''
            grep -c "could not execute process .sccache rustc" "$WORKSPACE/elohim/holochain/tests/sweettest/dna-integration.log" || true
        ''').trim().toInteger()
        if (hits >= 3) {
            currentBuild.description = "sccache spawn ENOENT recurred — see feedback_sccache_spawn_enoent_rca + upstream #2023"
        }
    }
} }
```

The earlier proposed pattern `ENOENT.*build-script` would NOT have matched the real signature — the literal string "build-script" doesn't appear in the failing line.

**Rollback (if 4b lands then regresses):** single git revert of the sccache re-enable. Comment correction (4a) and upstream issue (4c) stay landed regardless.

---

## 4. Expected wall-time matrix

| State | Compile | Tests | Total | Notes |
|---|---|---|---|---|
| Today (warm baseline) | ~30 min | ~44 min | **~74 min** | 2026-05-24 benchmark |
| After W1 (PVC) | ~10 min | ~44 min | ~54 min | Full suite, warm |
| After W1+W2 | ~10 min | ~44 min | ~54 min | Infra; same wall-time |
| After W1+W2+W3 (single-DNA push) | ~10 min | ~5-10 min | **~15-20 min** | Common case |
| After W1+W2+W3 (full-suite push) | ~10 min | ~44 min | ~54 min | Shared paths / dev / manual |
| After all (single-DNA, sccache OK) | ~3-5 min | ~5-10 min | **~10-15 min** | Best case |
| After all (full, sccache OK) | ~3-5 min | ~44 min | ~50 min | |

---

## 5. Verification

End-to-end, post-merge to `dev`:

1. **Single-DNA push:** one-line change to `elohim/holochain/dna/imagodei/zomes/coordinator/imagodei/src/lib.rs`. Expect stage ~10-20 min; nextest log shows only imagodei-scoped binaries ran.
2. **Shared-path push:** change to `elohim/holochain/tests/sweettest/Cargo.toml`. Expect full suite.
3. **Out-of-set push:** change to `genesis/orchestrator/orchestrator-strategy.mjs`. Expect `elohim-holochain` not dispatched at all (orchestrator-level gate, unchanged from today).
4. **3-build sccache watch:** three consecutive `dev` builds, no ENOENT. If recurrence → revert W4 only.

---

## 6. Risk surface

| Wave | Risk | Mitigation |
|---|---|---|
| W1 | A code path we don't yet see also assumes `./target` and breaks under PVC mount | Scoped env var keeps the surface narrow; first build will surface it |
| W2 | `CHANGED_PATHS` exceeds Jenkins parameter length limit on large changesets | Newline-delimited string is compact; if a changeset is large enough to overflow, treating it as full-suite is acceptable behavior |
| W3 | A test silently changes which DNA it exercises and the marker stays stale | Pre-push smoke check catches missing markers; stale-but-present markers caught by code review on the test file diff |
| W3 | Cross-DNA test misclassified, regression slips through | Default-to-always-run on missing markers; multi-DNA `@dna-scope` syntax for explicit cross-DNA |
| W4 | ENOENT recurs and the classifier doesn't fire (different error class) | 3-build observation window catches non-classified failures too; revert remains a single commit |
| W4 | sccache 0.16+ has its own regressions | Pin to a tested version; rollback path is single-commit revert |

---

## 7. Execution sequencing (for `/shift` Objective)

Each wave is a standalone commit / PR (per memory `feedback_subagent_dep_conflict_supervision` — `dev` accepts local merges; PR review at `dev → main`). Suggested order:

1. **W1 first** — pure infra, no source touch, biggest single-step payoff. Land alone. Watch one or two builds for stability.
2. **W2 + W3 together** — W2 is meaningless without W3; W3 is meaningless without W2. Land them as separate commits in one PR (or two stacked PRs).
3. **W4 last** — riskiest. Land only after Waves 1-3 have settled, so a sccache regression doesn't co-mingle with selectivity bugs in debugging.

Total estimated effort: ~1-2 days of focused work for W1+W2+W3; W4's risk envelope means budget another 1-3 days of observation across the 3-build window.

---

## 8. Open questions for the operator before kickoff

1. **`recovery_m3` scope.** Uses generic `load_dna()`. Options: `recovery` (introduces a new logical scope name), `*` (always-run sentinel), or read the test body to determine the dominant DNA. Operator decision needed before W3 commits.
2. **`qahal_collab_t0_test` scope.** Conceptually qahal but currently exercises imagodei DNA. Recommend `imagodei` until qahal DNA splits out (the qahal architecture vision spec is independent of this work). Operator confirms or overrides.
3. **Pre-push check location.** `.husky/pre-push` is the natural home, but the file is shared across the whole repo. Alternative: a check inside the orchestrator `manifest-hygiene` stage (CI-side). Operator preference between local-feedback (husky) and CI-side enforcement (manifest-hygiene).
4. **sccache version pin location.** The Jenkins agent image is built where? (Likely a Dockerfile under `.k8s/` or the orchestrator's agent template.) Operator points the way before W4.

---

## 9. Provenance

- Brainstorm session 2026-05-24 with operator
- Phase 1 exploration of `elohim/holochain/dna/Jenkinsfile`, `genesis/orchestrator/orchestrator-strategy.mjs`, `genesis/manifests/nix-cache-pvc.yaml`, and `elohim/holochain/tests/sweettest/src/tests/*.rs` (18 files)
- Git history for `f0cac18c8`, `ff648597f`, `efbac2938`, `0b4055851`, `a92d91c2b`
- Memory entries: `feedback_understand_orchestrator_substrate_before_changes`, `project_tiered_quilt_spec_landed_2026_05_11`, `feedback_subagent_dep_conflict_supervision`, `project_pre_dispatch_hard_fail_post_dispatch_unstable`
- Shift journal `2026-05-11T02-24-fix-sccache-unbound-on-elohim-holochain.journal.md` (sccache recurrence)
