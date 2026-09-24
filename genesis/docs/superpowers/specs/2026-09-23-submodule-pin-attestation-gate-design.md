---
title: A submodule pin move is gated by its own attestation and re-gates its direct consumers — rung 3 of the component ladder
id: submodule-pin-attestation-gate-design
status: Draft
class: architecture
serves: pin-attestation
date: 2026-09-23
context-tier: disclosed
steward: agent:orchestrator@claude-fable-5-1
graduation-trigger: the four landing commits in §8 are on dev, the `pin-attestation` habit atom carries a delta for each, one real push has printed an oracle-diff that was only the intended propagation, and the oracle flip (§8 commit 4) has held for two pushes that moved a pin; then the rung-1 question (a locally minted attestation per gate run) is opened as its own design
cites:
  - elohim/rakia/schemas/v1/build-manifest.schema.json
  - elohim/rakia/docs/specs/2026-04-12-rakia-design.md
  - elohim/rakia/docs/plans/build-attestation-integration.md
  - elohim/rakia/docs/plans/stage-2-canopy.md
  - genesis/orchestrator/gate-runner.mjs
  - genesis/orchestrator/graph-walker.mjs
  - genesis/orchestrator/run-local-gate.sh
  - genesis/orchestrator/gate-cycle.mjs
  - "verification-as-memoized-derivation-guidestar | Law III: reading is free, trusting is a choice; the attested gate degrades to claimed offline and never blocks the floor | sha256:b4a0a2e087e67c12 | path: genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md"
  - "evidence-ladder-push-left | admissibility rule and the T0-T4 ladder the pin-attestation observation sits beside | sha256:ac39aeb003dada60 | path: genesis/docs/superpowers/specs/2026-08-10-evidence-ladder-push-left-design.md"
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
  - genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md
  - genesis/data/timeline/backlog/cargo-test-memory-shed-storage-gate.md
---

# A submodule pin move is gated by its own attestation and re-gates its direct consumers

## 1. The incident that priced the problem

On 2026-09-23 a push from a local machine set up for brit work moved the `sophia` submodule
pointer. The pre-push gate ran sophia's full local suite, because sophia's manifest step lists
`sophia/**` as its sources and the gitlink path `sophia` matches that glob under picomatch. The
suite failed once for want of sophia's `node_modules` on a machine that had no reason to carry
them. Moving the `elohim/brit` pointer, by contrast, runs nothing: brit has no manifest, so a
red brit pin is indistinguishable from a green one at the monorepo boundary.

Both are the same defect. The gate models a submodule as a directory whose files changed, when
what changed is a **pin**: a reference to a commit that another repository has already built,
tested, and attested. Re-running the component's suite from the monorepo is the wrong proof at the
wrong altitude, and running nothing is no proof at all.

## 2. The ladder this is one rung of

The operator's course-set (2026-09-23) names a ladder a component climbs:

1. **Component-local.** Change the component in its own checkout; its own gate builds and tests it
   locally; a local attestation is minted.
2. **Component-peer.** Push the component; peers validate it between themselves and mint
   attestations. Today the only peer is the component's GitHub CI, one attester.
3. **Consumer-local.** Someone sees a validated version, bumps the pin in the monorepo, and the local
   gate runs the **consumers** of the pin, not the component's suite again.
4. **Consumer-peer.** Push the monorepo; peers validate the integration. Today that is Jenkins.
5. **Propagation.** As a version accumulates attestations from diverse peers its reach rises and
   upgrade propagation accelerates across the network. Proven for runtime artifacts on the household
   mesh (release channels, election, canary soak, promote, revert); not yet for a tool or crate.

This spec is **rung 3 only.** It is chosen first by the debt-snowball rule of the
upgrade-propagation arc: the smallest atomic discipline that the rest depends on. Rung 1 (a signed
local attestation per gate run) is the graduation target, not part of this design.

## 3. Ground truth (measured 2026-09-23)

| fact | evidence |
|---|---|
| A submodule pin move appears in every changed-file list as the bare gitlink path (`sophia`, `elohim/brit`). | `git diff --name-only` semantics; the just `gate` recipe and the pre-push hook both feed this list to `gate-runner.mjs --changed-file-list`. |
| picomatch matches `sophia/**` against `sophia`; rakia's globset does not. | `node -e` probe (true) vs `rakia affected --repo . --files sophia` → `affected: []`. The two oracles already disagree on the one case this spec is about. |
| The JS walker does not propagate staleness through `depends`, by design ("propagation is a Jenkins concern"). rakia-core's `plan_from_changes` propagates deeply. | `graph-walker.mjs` Phase 3 comment; `constellation.rs` Phase 2. A sophia source file reaches `elohim:build-angular` → `elohim:build-site-image` → `elohim-genesis:seed-content` in four hops. |
| The `rakia` binary (brit-cli) is installed on this host and runs `affected`, `plan`, `baseline`, `fingerprint`, `graph-*`. The pre-push hook does not call it. | `~/.cargo/bin/rakia`; the rakia design spec §4 open question 4 (switch or shadow) was never answered. |
| The rakia-codegen gate is red at the pinned rakia schema. | `pnpm run rakia:codegen:rs:verify` → `inline nested objects must be lifted to $defs (path: GateProject.run)`. The generated `GateProject` has no `run`; the schema's typed-gate commit never regenerated. |
| The pinned schema refuses `GateProject.run.cargo.env`; the storage cap lives in `pool-policy.json` as a stopgap. | backlog `cargo-test-memory-shed-storage-gate` 2026-09-08 amendment; `gate-runner.mjs gateChildEnv` merge rule. |
| An authenticated GitHub CLI on this host reads the check runs at every current pin: brit `Tests pass`, rakia `test`, sophia `build`, all `success`. | `gh api repos/<owner>/<repo>/commits/<sha>/check-runs` at b86c5104d, 13cb31ddd, cb1092ef4. |
| Monorepo consumers of brit and rakia are steps, not crates: the edge image bakes `brit-verify` and `compute-executor`; the storage build reads the rakia release-manifest schema in a mirror test; the orchestrator's `rakia-validate` and `rakia-codegen` gates read the rakia schema. No monorepo crate path-depends on a brit or rakia crate. | `scripts/ci/build-compute-worker.sh`, `release_adoption/verify.rs:1563`, `genesis/orchestrator/build-manifest.json`, Cargo.toml grep. |
| sophia's manifest lives in the sophia repo and uses monorepo-relative paths. | sophia commits `9bc2f9eef`, `94bb52ef5`. This is the precedent for brit and rakia. |

## 4. The design

### 4.1 Schema step (rakia)

One bump to `elohim/rakia/schemas/v1/build-manifest.schema.json`, one `codegen-rs` regen, one pin
bump in the monorepo.

- Lift the inline `gateProject.run` and `run.cargo` objects into `$defs` as `gateRun` and
  `gateCargo`. This is what the codegen already demands.
- Add `env` to `gateCargo`: an object whose property names match `^[A-Z][A-Z0-9_]*$` and whose values
  are strings. The mirror shape is already in `genesis/orchestrator/manifest.schema.json`.
- Widen `gateRun.kind` to `just | root-just | attested`.
- Add `gateRun.attestation`, lifted as `$defs.gateAttestation`, with exactly three required fields:
  `provider` (enum, one value today: `github-checks`), `repo` (`^[\w.-]+/[\w.-]+$`), `check` (the
  check-run name, non-empty string). A `oneOf` binds it: kind `attested` requires `attestation` and
  forbids `recipe`; the other kinds require `recipe` and forbid `attestation`.
- No thresholds, no signer lists, no reputation, no second provider.

The rakia-codegen and rakia-validate gates prove the step: codegen fresh, every manifest in the tree
validates against the widened schema.

### 4.2 Component manifests

Each component declares itself, in its own repository, with monorepo-relative paths (sophia's
precedent). **The pin is the input:** the step's `sources` list the gitlink path itself alongside the
tree glob, so both oracles match a pin move without either matcher special-casing gitlinks.

| component | pipeline | step | sources | gate project | attestation |
|---|---|---|---|---|---|
| brit | `elohim-brit` | `brit-ci` | `elohim/brit`, `elohim/brit/**` | `brit` · `dir: elohim/brit` · `attested` | `github-checks` · `ethosengine/brit` · `Tests pass` |
| rakia | `elohim-rakia` | `rakia-ci` | `elohim/rakia`, `elohim/rakia/**` | `rakia` · `dir: elohim/rakia` · `attested` | `github-checks` · `ethosengine/rakia` · `test` |
| sophia | `elohim-sophia` (exists) | `build-sophia-umd` (exists) | add `sophia` | `sophia` flips `just gate` → `attested` | `github-checks` · `ethosengine/sophia` · `build` |

brit and rakia manifests declare `manualOnly: true` and omit `jenkinsPath` (the schema does not
require it), so the orchestrator never dispatches a Jenkins job for them; the `buildProcess` names
the component's own `ci.yml`. sophia keeps its Jenkins pipeline declaration unchanged; only its **local** gate changes.

### 4.3 Consumer edges

Consumer edges are declared on **steps**, because steps are the only thing rakia propagates through.

| consumer step | adds `depends` | why |
|---|---|---|
| `elohim-edge:cargo-build-storage` | `elohim-rakia:rakia-ci` | already lists `elohim/rakia/schemas/**`; the release-manifest mirror test reads it |
| `elohim-edge:build-edge-image` | `elohim-brit:brit-ci`, `elohim-rakia:rakia-ci` | bakes `brit-verify` and `compute-executor` |
| `elohim:build-angular` | (already `elohim-sophia:build-sophia-umd`) | no change |

Gate-only projects (those with `inputs`, invisible to step propagation) gain the gitlink path
directly: `rakia-validate` and `rakia-codegen` add `elohim/rakia` to `inputs.sources`. A rakia pin
move changes the schema every manifest validates against; that is a direct input, not a propagated
one.

The storage cap (`CARGO_BUILD_JOBS: "1"`) moves from `pool-policy.json cargo_env_overrides` into
`elohim-storage`'s `run.cargo.env`, closing the stopgap the runner's merge rule was written for.

### 4.4 Selection: rakia as the oracle, depth one, shadow first

`gate-runner.mjs projectsForChanges` asks `rakia affected --repo <root> --files <list>` for the
affected step set and keeps the existing Phase 4 mapping from stale steps to gate projects. Gate
projects with `inputs` are still matched in JS as today.

- **Binary resolution.** `RAKIA_BIN` if set, else `rakia` on PATH. Absent: fall back to the picomatch
  path-only walk and print `[gate] rakia unavailable — path-only selection`. The floor completes
  without it (memoized-derivation Law III).
- **Depth one.** A gate project is selected when one of its steps changed directly, or when one of
  its steps `depends` directly on a step that changed directly. rakia's `AffectedReason.upstream`
  names the immediate upstream, so depth one is a filter over rakia's output, not a second graph
  walk. Deeper propagation remains CI's concern. Each selected project's `reasons` carry the
  upstream step name so the operator sees why a consumer ran.
- **Shadow mode first.** The first landing computes both selections and prints one
  `[gate] oracle-diff: +<projects> -<projects>` line whenever they differ, without changing which
  gates run. The flip to rakia as sole oracle is a separate commit (§8 commit 4), after at least one
  real push has printed a diff that was only the intended propagation. This answers the rakia design
  spec's open question 4: shadow, then switch.

### 4.5 The attested gate

`gate-runner.mjs` dispatches a project whose `run.kind` is `attested` to a new module,
`genesis/orchestrator/gate-attest.mjs`, and never to `run-local-gate.sh` (which keeps refusing
unknown kinds). Provider, repo and check come from the registry entry; nothing travels through
argv or the environment. Keeping the read in Node lets the four outcomes be pinned by `node:test`
with a fake `gh` and reuse the gate-cycle observation path.

1. Resolve the pinned commit: `git rev-parse HEAD:<dir>`. A path that is not a gitlink is a manifest
   error (exit 2), not a gate failure.
2. If the submodule worktree is dirty, print `attested: worktree dirty — the committed pin is what is
   read; the component's own gate governs the worktree`. This never changes the outcome.
3. Read `gh api repos/<repo>/commits/<sha>/check-runs?per_page=100` and select the named check
   (latest run if several).
4. Outcome:

| read | conclusion | result |
|---|---|---|
| ok | `success` | **pass** · `attested: <repo>@<sha12> <check> success` |
| ok | `failure`, `cancelled`, `timed_out`, `action_required`, or check absent at that SHA | **refuse** (exit 1) · names check and SHA. A pin without its attestation is not a green pin. |
| ok | `in_progress` / `queued` | **refuse** (exit 1) · `attested: <check> not yet concluded` |
| failed (no `gh`, not authenticated, offline, rate-limited) | — | **pass** · `attested: claimed — <reason>`. Reading is free, trusting is a choice; the floor never blocks on the network. |

### 4.6 Evidence: the `pin-attestation@1` measure

Every attested read is recorded through the gate-cycle path as an observation on a new measure
declared in `.claude/epr-meta/measures.yaml` beside `gate-cycle-seconds@1`:

```
id: pin-attestation · version: 1 · family: dev-cycle · unit: reads
procedure: run-local-gate.sh attested branch — `epr flow note --kind observation
  --measure pin-attestation@1 --subject <dir> --value 1
  --env provider=<p> --env check=<c> --env conclusion=<success|failure|absent|unreachable>
  --env tier=<witnessed|claimed>`
default-authority: observation · status: active
```

`tier=witnessed` on a successful read of any conclusion; `tier=claimed` when the read itself failed.
No ceiling or reader is added in this spec. This is the seed rung 1 grows from: the observation is
already shaped like an attestation, missing only a signer.

## 5. The habit this serves

No existing habit's invariant covers this behaviour, and a spec that serves no habit belongs in
`held/`. So it declares one where the behaviour lives:

`genesis/orchestrator/.epr-meta/pin-attestation.habit.md`, born **red**, `active: false` (the
two-active fence holds).

- **invariant:** a submodule pin move is gated by the pinned commit's own attestation and re-gates its
  direct consumers; the component's suite is never re-run from the monorepo.
- **checks:** `a2o @concern:pin-attestation`
  (`genesis/a2o/features/devflow/pin-attestation.feature`); `pin-attestation@1` observations present
  for every pin move in the push range.
- **retire-when:** attestations are read from the dataplane rather than a forge — rung 2 landing
  makes `github-checks` one provider among peers and this habit describes a product, not a practice.

## 6. Proof

| layer | rail | cases |
|---|---|---|
| unit, orchestrator | the existing `*.test.mjs` beside `gate-runner.mjs` | gitlink path selects the attested project and its depth-one dependents; a two-hop dependent is not selected; absent `rakia` falls back and prints the line; shadow mode reports a diff without changing selection; `attestation` travels as `GATE_ATTESTATION`, never argv |
| unit, gate script | a fake `gh` on PATH inside a throwaway git fixture with a gitlink | success passes; failure refuses; absent check refuses; unreachable read passes as `claimed`; dirty worktree line printed, outcome unchanged |
| story | `genesis/a2o/features/devflow/pin-attestation.feature`, `@concern:pin-attestation`, default profile, fake `gh` | the four outcomes above plus one propagation scenario (a brit pin move selects the edge-image consumer and nothing deeper) |
| schema | rakia-core's fixture runner (validates the real monorepo manifests when present); `rakia-codegen`, `rakia-validate` gates | codegen fresh; every manifest validates; an `attested` project without `attestation` is refused; a `just` project with `attestation` is refused |

## 7. Non-goals

Stated so nobody widens them mid-flight:

- no attestation provider beyond GitHub checks;
- no thresholds, diversity, or builder reputation (canopy);
- no signed local attestation per gate run (rung 1);
- no DHT publication of attestations (rakia Stage 2);
- no running a submodule's suite from the monorepo under any condition;
- no propagation deeper than one hop in the local gate;
- no change to what Jenkins dispatches (`walkGraph` Phase 5 and `build-graph.groovy` are untouched).

## 8. Landing order

Four commits, each behind its own gate, in dependency order. Pushes from this host are SSH-only and
long gates can drop the session: run the gate to ALL CLEAR first, then push with the hook bypassed.

| # | repo(s) | change | gate |
|---|---|---|---|
| 1 | rakia; monorepo | schema lift, `attested`, `cargo.env`, regen; pin bump | `rakia-codegen`, `rakia-validate` |
| 2 | brit, sophia, rakia; monorepo | three manifests; three pin bumps; consumer `depends`; orchestrator `inputs`; storage cap into its manifest | `rakia-validate`, orchestrator tests |
| 3 | monorepo | rakia oracle in shadow mode; attested branch; measure; habit atom; feature file | orchestrator gate, a2o lint |
| 4 | monorepo | flip the oracle; habit delta; register re-projected | orchestrator gate; one real pin-moving push |

## 9. Open questions

1. **sophia's check name.** sophia's aggregate job is `build`; if its workflow gains a `tests-pass`
   style rollup, the manifest's `check` field is the one place to change.
2. **`gh` on CI runners.** The attested branch only runs in the local gate; CI keeps its own
   detection. If a CI runner ever executes `just gate`, the read degrades to `claimed` there, which is
   honest but noisy — a `GATE_ATTESTATION_SKIP=1` escape is deliberately not added until that day.
3. **Which direct consumers are worth a local gate.** Depth one selects `elohim-storage` on a rakia
   pin move, which is a ~33-minute gate today. That is the correct answer to "what does this pin
   touch"; the storage crate decomposition spec is where the cost is being paid down, not here.
