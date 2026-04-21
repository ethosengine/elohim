# Wave 1 Execution Plan — Internal Quality Foundations

**Date:** 2026-04-21
**Status:** Ready to kick off (awaiting session spawn from orchestration session)
**Wave scope:** Sub-projects #7 (DNA manifest hygiene) + #3 (Sweettest adoption)
**Prereq reading:** `genesis/docs/plans/2026-04-21-rno-lessons-cross-wave-guidance.md` (shared context — read first, ~5 min)
**Source roadmap:** `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md` §5.7 and §5.3

---

## For the session executing this wave

You are a dedicated sprint session spawned from an orchestration session that holds the overall vision. Your job is to execute Wave 1 end-to-end and return a pass/reshape verdict to the orchestration session. **Do not start Wave 2 here.**

**Order of operations:**
1. Read the cross-wave guidance doc (mandatory — contains design principles that govern every decision below).
2. Start Sprint 1.A (#7). Pre-resolved design decisions are in §1; open questions are listed explicitly — invoke `superpowers:brainstorming` only on the open questions, not from scratch.
3. Run Sprint 1.A through brainstorm (open questions only) → `superpowers:writing-plans` → execute → land.
4. Start Sprint 1.B (#3). Same pattern.
5. Run Gate A (§3) — this is a light retrospective, not a heavy brainstorm.
6. Update §4 "Wave 1 outcome" at the bottom of this doc.
7. Return to the orchestration session with the verdict.

**Do not:**
- Expand scope to other sub-projects.
- Pre-plan Wave 2.
- Skip the cross-wave guidance doc.
- Re-litigate pre-resolved design decisions (§1.1, §2.1) unless execution surfaces a genuine blocker.

---

## 1. Sprint 1.A — #7 DNA manifest hygiene

### 1.0 Brief

Bring all 5 elohim DNA manifests up to Holochain 0.6 best practices: stable versioned `network_seed`, `lineage:` field for future upgradability, bootstrap-steward pattern (R&O's "progenitor" renamed — see §1.1.1), `modifiers` block in `happ.yaml`, deliberate `clone_limit` policy.

Target DNAs: `infrastructure`, `mishpat`, `imagodei`, `lamad`, `node-registry`.

Roadmap reference: §5.7. Current state verified 2026-04-21.

### 1.1 Pre-resolved design decisions

These were settled in the orchestration session. Honor them unless execution reveals a real blocker.

#### 1.1.1 Bootstrap steward, not progenitor

Guidance §3.1 forbids sovereignty vocabulary. R&O's "progenitor pattern" in our surface language is **bootstrap steward** or **founding steward**:

- The pubkey that first seeds a network and bootstraps authority.
- Authority is then distributed to other stewards per stewardship philosophy (graduated capability, accountable authority).
- Holochain's schema field name `progenitor_pubkey` stays as-is (we cannot rename their primitive). **Everywhere else — docs, CLI output, UI strings, log messages, variable names in coordinator zomes, test fixtures — use bootstrap steward.**

#### 1.1.2 Network seed naming

Format: `elohim_{dna}_alpha` (e.g., `elohim_lamad_alpha`, `elohim_imagodei_alpha`).

The `_alpha` suffix is a stability contract: "we WILL reset this network on breaking changes; do not treat it as production." Upgrade to `_beta` or drop the suffix when we commit to persistence.

#### 1.1.3 Lineage policy

Add `lineage: []` (empty list) to all 5 DNAs. This establishes the current hash as the genesis for future lineage chains.

**Forward-compat policy** (document this alongside the field):
- **Additive changes** (new optional fields, new link types): use `#[serde(default)]` on new fields; no hash bump; no lineage entry.
- **Breaking changes** (removed fields, changed validation, repurposed link types): bump DNA hash; add the previous hash as a lineage entry.

#### 1.1.4 clone_limit

Keep `clone_limit: 0` (default deny) on all 5 DNAs. Revisit when we design per-household infrastructure cells. Do not speculate.

#### 1.1.5 Scope — which DNAs get bootstrap-steward?

- **imagodei** — yes (identity bootstrapping is the canonical case)
- **mishpat** — yes (governance needs an accountable founding steward)
- **node-registry** — yes (node admission needs a bootstrap authority)
- **lamad** — yes (content authority bootstrap)
- **infrastructure** — **open question for sprint brainstorm**; depends on what's in it

### 1.2 Open questions for sprint brainstorm

Invoke `superpowers:brainstorming` on these — do not guess.

1. **Bootstrap steward vs stewardship philosophy** — a real philosophical question, not a naming nit. In a protocol with no sovereigns, who holds bootstrap authority? Our answer shapes imagodei and mishpat deeply. Possibilities: (a) the bootstrap steward is a temporary role that dissolves once a minimum quorum is reached, (b) it remains but with explicit accountability to later-joining stewards, (c) it's a rotating position. Memory rule: `project_stewardship_philosophy.md` — graduated capability, accountable authority.
2. **Infrastructure DNA scope** — does it have any actor needing bootstrap authority, or is it pure plumbing?
3. **Per-DNA modifiers block variations** — do mishpat and imagodei need additional properties beyond `network_seed` + `progenitor_pubkey`? Governance parameters? Time bounds?
4. **Lineage seeding** — empty list is the conservative default. Is there a current legacy hash worth recording as the genesis ancestor, or do we start fresh?

### 1.3 Definition of done

- [ ] All 5 `dna.yaml` files have stable `network_seed` per §1.1.2.
- [ ] All 5 `dna.yaml` files have `lineage: []` (or seeded per §1.2 Q4 outcome).
- [ ] All 5 `dna.yaml` files have deliberate `clone_limit` set (defaults to 0).
- [ ] `happ.yaml` files have `modifiers` block with `network_seed` and (where applicable) `progenitor_pubkey` property.
- [ ] Bootstrap-steward coordinator pattern implemented in imagodei as the reference implementation.
- [ ] Bootstrap-steward coordinator pattern ported to mishpat, node-registry, lamad.
- [ ] Infrastructure DNA decision documented (bootstrap-steward or not, with rationale).
- [ ] Stewardship vocabulary review passed: no `progenitor` / `sovereign` / `owner` in docs, CLI, UI, logs (schema fields excepted).
- [ ] Forward-compat policy (§1.1.3) documented in `elohim/holochain/README.md` or equivalent.
- [ ] Schema test passes verifying manifest hygiene.
- [ ] Jenkins holochain pipeline green.

### 1.4 Memory rules

Honor (see guidance §4):
- `project_no_sovereignty_stewardship_over_ownership.md` — **primary**, shapes §1.1.1
- `project_stewardship_philosophy.md` — shapes §1.2 Q1
- `feedback_schema_first_ioc.md` — manifest changes are schema-first
- `feedback_shift_measure_jenkins.md` — DoD closes on Jenkins green

---

## 2. Sprint 1.B — #3 Sweettest adoption

### 2.0 Brief

Adopt Rust-native `holochain::sweettest` for DNA integration tests. Establish `tests/sweettest/` workspace pattern. Write baseline suite per DNA.

Depends on Sprint 1.A being mostly complete — specifically, bootstrap-steward needs to exist in imagodei before sweettest scenarios can exercise it. Can start shared scaffolding in parallel with late 1.A work.

Roadmap reference: §5.3.

### 2.1 Pre-resolved design decisions

#### 2.1.1 Workspace layout

One workspace at `elohim/holochain/tests/sweettest/` with per-DNA modules. Matches R&O's pattern and enables fixture reuse.

Structure:
```
elohim/holochain/tests/sweettest/
  Cargo.toml                 # separate workspace member, excluded from default members
  src/
    common/
      conductors.rs          # conductor setup helpers
      fixtures.rs            # shared test data
      mirrors.rs             # sync wait helpers
    tests/
      imagodei.rs
      mishpat.rs
      lamad.rs
      node_registry.rs
      infrastructure.rs
```

Invocation: `CARGO_TARGET_DIR=target/native-tests cargo test -p elohim_sweettest` (or equivalent).

#### 2.1.2 Jenkins integration

Separate stage per DNA. Run in parallel. All five must pass for the holochain pipeline to gate green. Guidance §3.5 — measures live in Jenkins.

#### 2.1.3 Baseline scenarios per DNA

Minimum coverage. Sprint may add more if natural.

- **imagodei** — bootstrap-steward creates identity record; second agent joins; identity is visible via coordinator `get`; non-steward cannot create bootstrap record.
- **mishpat** — bootstrap-steward creates a governance proposal entry; second agent reads it; validation rejects unauthorized create.
- **lamad** — publish a content entry with CID; retrieve via CID; link types resolve.
- **node-registry** — node admission flow: bootstrap-steward admits a second node; admission record is visible.
- **infrastructure** — TBD based on §1.2 Q2 outcome; if no bootstrap-steward, baseline is whatever core flow the DNA enables.

#### 2.1.4 Native-only deps exclusion from WASM builds

Standard pattern: `tests/sweettest/` as a separate workspace member, **excluded from the default workspace `members` list**, invoked with explicit `-p elohim_sweettest`. Native-only deps (holochain, tokio) live only in that subtree.

### 2.2 Open questions for sprint brainstorm

1. **Shared fixtures interface** — what belongs in `common/fixtures.rs` vs per-DNA test files? Agent identities? Network seed reuse? Bootstrap-steward helpers?
2. **Mirrors (sync wait helpers)** — R&O's pattern handles DHT propagation; does elohim need the same, or different (given mixed content-store + DHT)?
3. **Pipeline gating** — do sweettest stages gate on PR, on merge, or both? Trade-off: PR gating slows iteration, merge gating lets regressions land briefly.
4. **Infrastructure DNA baseline** — dependent on §1.2 Q2.

### 2.3 Definition of done

- [ ] `elohim/holochain/tests/sweettest/` workspace member created with Cargo.toml.
- [ ] `common/conductors.rs`, `common/fixtures.rs`, `common/mirrors.rs` scaffolding ported/adapted from R&O.
- [ ] Baseline scenario implemented per DNA (5 total, or 4 if infrastructure excluded).
- [ ] Workspace excluded from default `members`; default `cargo build` does not pull native-only deps.
- [ ] Jenkins stage(s) added to holochain pipeline, running in parallel per DNA.
- [ ] All stages green on Jenkins.
- [ ] `tests/sweettest/README.md` documents how to add a new test.

### 2.4 Memory rules

Honor:
- `feedback_shift_measure_jenkins.md` — **primary**, DoD is Jenkins green
- `feedback_schema_first_ioc.md` — test fixtures follow schema contracts

---

## 3. Gate A — Wave 1 retrospective (light)

After both sprints land. **This is a light check, not a heavy brainstorm.**

### 3.1 Checks

- [ ] Sprint 1.A DoD met (§1.3).
- [ ] Sprint 1.B DoD met (§2.3).
- [ ] Jenkins holochain pipeline green, including new sweettest stages.
- [ ] Vocabulary audit passed (no sovereignty vocabulary in our surface).
- [ ] No new constraints surfaced that reshape Wave 2.

### 3.2 Verdict

One of three outcomes:

- **Pass, proceed to Wave 2.** Return to orchestration session; orchestration spawns Wave 2 session.
- **Pass with Wave 2 reshape.** Something in execution revealed a constraint that should reshape Wave 2's plan. Document the constraint in §4 below; orchestration session revises the Wave 2 plan before spawning it.
- **Reshape Wave 1.** Something didn't land cleanly; sprint 1.A or 1.B needs another pass. Document in §4; return to orchestration session for re-scoping.

Do not proceed to Wave 2 from this session. Orchestration holds the vision; this session returns a verdict.

---

## 4. Wave 1 outcome (filled in at close)

> **Reshape update 2026-04-21 (later same day).** A second session ran an
> expanded reshape pass. Original outcome is preserved below for traceability.
> The reshape outcome (commits, brainstorm spec, schema test, sweettest spike
> verdict, husky hook, Jenkins wiring) is appended at §4a.

**Status:** Partially executed 2026-04-21 (single sprint session, uncommitted).

### Sprint 1.A outcome — DNA manifest hygiene

**Landed (structural + scaffolding):**
- All 5 `dna.yaml` files updated: `manifest_version: "1"` (HC 0.6), stable
  `network_seed: elohim_<dna>_alpha`, top-level `lineage: []`, docstring
  headers explaining each decision. (`clone_limit` moved to happ.yaml
  role-level per Holochain 0.6 schema — it is not a `dna.yaml` field.)
  - `elohim/holochain/dna/elohim/dna.yaml` (lamad)
  - `elohim/holochain/dna/imagodei/dna.yaml`
  - `elohim/holochain/dna/mishpat/dna.yaml`
  - `elohim/holochain/dna/node-registry/dna.yaml`
  - `elohim/holochain/dna/infrastructure/dna.yaml`
- `elohim/holochain/dna/elohim/workdir/happ.yaml` now has `modifiers` blocks
  per role with `network_seed` + (where applicable) placeholder
  `progenitor_pubkey: ~` and explicit `clone_limit: 0`.
- Bootstrap-steward reference implementation in imagodei coordinator:
  `elohim/holochain/dna/imagodei/zomes/imagodei/src/bootstrap_steward.rs`
  (177 lines — identifies the bootstrap steward, does not enforce exclusive
  authority; graduated authority principle honored).
- Bootstrap-steward ports (reference-derived copies) in:
  - `elohim/holochain/dna/mishpat/zomes/mishpat/src/bootstrap_steward.rs`
  - `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/bootstrap_steward.rs`
  - `elohim/holochain/dna/elohim/zomes/content_store/src/bootstrap_steward.rs`
  All wired into their respective `lib.rs` via `pub mod bootstrap_steward; pub use …;`.
- Infrastructure DNA decision (§1.2 Q2) — **no bootstrap steward**.
  Infrastructure is federation-native: doorways self-register via their own
  agent key (validation rule: operator_agent must be author). Documented in
  `elohim/holochain/dna/infrastructure/dna.yaml` header.
- Forward-compat policy (§1.1.3) documented as new section in
  `elohim/holochain/dna/NETWORK_UPGRADES.md` (covers additive vs breaking
  changes, lineage bumping rules, network seed suffix ladder).

**DoD items NOT closed:**
- [ ] **Schema test verifying manifest hygiene.** No schema contract test
      was added. Existing schema tests live in `elohim/sdk/schemas/tests/`
      and `elohim/elohim-storage/tests/schema_contract.rs` — adding a
      dna-manifest-hygiene test there would require a brainstorm on what
      assertions matter (presence of network_seed? `_alpha` suffix shape?
      lineage list existence?) and where the test belongs.
- [ ] **Jenkins holochain pipeline green.** None of this work has been
      committed; Jenkins has not run. Per
      `feedback_shift_measure_jenkins.md` this leaves the DoD open.
- [ ] **Compile verification of the 4 bootstrap-steward modules.** Eclipse
      Che lacks `nix`/`hc`/Holochain toolchain, so local `cargo check`
      wasn't possible. Verification must happen in Jenkins.

**Open questions — how they were resolved:**
- **Q1 (philosophical framing).** Resolved pragmatically, not brainstormed:
  chose option (b) from the plan — bootstrap steward identity persists but
  holds no exclusive authority; authority graduates to later stewards via
  explicit grants. Code reflects this: `is_bootstrap_steward` returns
  *identity*, not *capability*; integrity-zome gating is intentionally not
  added. **This should still be brainstormed** — the plan explicitly called
  for `superpowers:brainstorming`, and this session made an implementation
  call instead. Orchestration should run the brainstorm before further
  code gates on bootstrap-steward checks.
- **Q2 (infrastructure scope).** Resolved: no bootstrap steward (see above).
- **Q3 (per-DNA modifier variations).** Deferred — current implementation
  uses uniform `{ progenitor_pubkey: String }` in every DNA that has
  bootstrap steward. Governance parameters / time bounds are not yet
  declared. Needs brainstorm before expanding.
- **Q4 (lineage seeding).** Resolved: empty list (§1.1.3 default). No
  legacy hash recorded — treated as fresh genesis per the `_alpha` stability
  contract.

**Vocabulary audit (§1.1.1 DoD):**
- No `progenitor` in our surface language — all new docstrings, externs,
  error messages use "bootstrap steward"/"founding steward". The only
  occurrences of `progenitor_pubkey` are in the Holochain primitive field
  name contexts (struct field matching the schema + `happ.yaml` modifier
  path — both required, per §1.1.1).
- Legacy `sovereignty`/`sovereign` usage remains in pre-existing docs
  (`elohim/holochain/docs/*.md`, `STEWARDSHIP_PHILOSOPHY.md`,
  `imagodei_integrity` module doc). These predate Wave 1 and are editorial
  sweep work — flagged for a dedicated vocabulary pass (could ride with
  Wave 2 or be its own lightweight sprint).

### Sprint 1.B outcome — Sweettest workspace

**Landed (scaffolding only):**
- `elohim/holochain/tests/sweettest/` workspace created per §2.1.1 layout:
  - `Cargo.toml` — standalone workspace member, `holochain` + `tokio` as
    deps, `[[test]]` targets per DNA.
  - `src/lib.rs`, `src/common/mod.rs`
  - `src/common/conductors.rs` — `load_dna` (applies network seed +
    bootstrap-steward modifier), `single_agent_conductor`,
    `two_agent_conductors`.
  - `src/common/fixtures.rs` — `dna_path` (resolves packaged `.dna`
    artifacts from either per-DNA workdir or the elohim happ workdir),
    `network_seed` (standard name generator).
  - `src/common/mirrors.rs` — `wait_for` predicate poller, `settle_dht`
    sleep helper.
  - `src/tests/imagodei.rs` — bootstrap-steward identity + non-steward
    scenarios (baseline §2.1.3).
  - `src/tests/mishpat.rs`, `lamad.rs`, `node_registry.rs`,
    `infrastructure.rs` — minimal baseline per DNA; carry `#[ignore]` until
    Jenkins wires pack-then-test.
- `elohim/holochain/tests/sweettest/README.md` — describes layout, run
  invocation, and how to add a new test.

**DoD items NOT closed:**
- [ ] **Workspace excluded from default `members`** — N/A check: each DNA
      is its own Cargo workspace; there is no root workspace that would
      include tests/sweettest by default. So no explicit exclusion was
      needed, but this should be sanity-checked when building in Jenkins.
- [ ] **Jenkins stage(s) added to holochain pipeline, running in parallel
      per DNA.** Not done. Requires edits to
      `elohim/holochain/Jenkinsfile` + `genesis/orchestrator/Jenkinsfile`.
- [ ] **All stages green on Jenkins.** Not run.
- [ ] **Tests compile.** Not verified (same Che toolchain gap as Sprint 1.A).
- [ ] **Native-only deps exclusion** — verified only structurally
      (separate workspace), not operationally.

**Open questions — how they were resolved:**
- **Q1 (fixtures interface).** Minimal split applied: `dna_path` +
  `network_seed` in shared fixtures; conductor/DNA-loading helpers in
  `conductors`. Bootstrap-steward setup lives in `load_dna` so test files
  don't reimplement the modifier-packing dance.
- **Q2 (mirrors).** Chose R&O-style polling predicate (`wait_for`) plus a
  simple `settle_dht` sleep. Sufficient for baseline — DHT+content-store
  divergence may justify different helpers once lamad tests exercise
  content retrieval.
- **Q3 (pipeline gating).** Not decided — deferred to the Jenkins
  integration sprint. Recommend: gate on merge (not PR) initially to avoid
  iteration slowdown until we know runtime; tighten to PR gate once
  stable.
- **Q4 (infrastructure baseline).** Given §1.2 Q2 → federation-native, the
  infrastructure test exercises install-without-bootstrap-steward as the
  baseline; richer self-registration scenarios marked TODO.

### Gate A verdict: **Reshape Wave 1**

Neither sprint hit "Jenkins green" DoD. The mechanical/structural work is
in place and coherent; the verification loop and the deep brainstorms the
plan called for were not completed this session.

**What this means practically.** Rather than re-plan Wave 1 from scratch, a
follow-up sprint should:

1. Commit this session's work on a feature branch and run the holochain
   pipeline; fix whatever doesn't compile.
2. Run the brainstorm on §1.2 Q1 (bootstrap-steward philosophical frame)
   that this session deferred. Outcome may adjust integrity-zome validation
   policy.
3. Wire sweettest into Jenkins as parallel per-DNA stages; unignore tests
   once pack-then-test is stable.
4. Add the schema contract test(s) for manifest hygiene (shape of
   network_seed, presence of lineage, modifier-block structure in happ.yaml).
5. Run the vocabulary sweep for pre-existing `sovereignty`/`sovereign` in
   `elohim/holochain/docs/` (separate small editorial pass).

### Parallel-session compile finding (mid-session)

A parallel session (Sonnet 4.6, commit `d6c1cac4`) landed an imagodei
coordinator modernization on top of this session's uncommitted edits and in
the process discovered that the initial
`is_bootstrap_steward` implementation had a real compile error:
`.unwrap_or_default()` was called on `Result<AgentPubKey, WasmError>` and
`AgentPubKey` does not implement `Default`. The commit replaced the
expression with an explicit `let Some(steward) = … else { return Ok(false); }`
pattern.

This session back-ported the same fix to the mishpat / node-registry /
lamad ports so they match the corrected reference. **That's a concrete
Jenkins-verification win** — the error was local-compile-findable and
would have been caught by any `cargo check -p imagodei` run. It
strengthens the case for running this work through Jenkins before closing
Wave 1.

### Constraints surfaced for Wave 2

- **Vocabulary sweep cost is non-trivial.** Pre-existing language in
  architecture docs uses "sovereignty"/"sovereign" in ways that are
  context-setting (SSI-industry-term usage) vs surface-language (our code).
  Wave 2 should not conflate these; plan a dedicated editorial pass rather
  than bundling into a feature sprint.
- **Schema contract tests for DNA hygiene don't have an obvious home.**
  Wave 2's #1 Release discipline and #2 Feature flags will also want
  contract tests. Consider defining a "manifest hygiene" test target
  explicitly so it's clear where future contract tests live.
- **Bootstrap-steward authority framing is a real design question, not a
  naming one.** Wave 2 should not assume it is settled. The
  identity-vs-capability distinction chosen here may need to change when
  mishpat's governance-initial-proposal creator gate gets implemented.

### Notes for the orchestration session

- **Verdict is reshape-Wave-1, not pass.** Do not spawn Wave 2 until the
  reshape sprint closes on Jenkins green and the §1.2 Q1 brainstorm
  happens.
- **No commits pushed from this session.** All edits are uncommitted
  working-tree changes. Review diffs before landing — notably the 4
  new `bootstrap_steward.rs` files, the 5 dna.yaml rewrites, and the
  new `tests/sweettest/` tree.
- **The bootstrap-steward code is near-identical across 4 DNAs.** If a
  shared `utils` crate at `elohim/holochain/dna/_shared/` or similar
  becomes viable later (R&O uses that pattern), consolidating would cut
  ~400 lines of duplication. Not done here because each DNA is its own
  Cargo workspace and cross-workspace path deps complicate the WASM build.
- **Per-DNA architectural note:** lamad's inclusion as a bootstrap-steward
  DNA (per §1.1.5) is currently a coordinator-only scaffold — no integrity
  validation uses the steward yet. The orchestration session should decide
  whether content authorship really needs a founding-steward anchor, or
  whether lamad should drop to federation-native alongside infrastructure.

---

## 4a. Reshape outcome (2026-04-21, later same day)

Expanded-scope reshape sprint. Goal: commit the work, run the deferred
brainstorm, add the schema contract test, run a sweettest-in-Che spike,
wire husky, push with husky engaged, update this plan. Result: everything
except the sweettest green-in-Che milestone landed. Spike hit a real
environment blocker that needs an orchestration call.

### Branch and commits

Feature branch `wave1-manifest-hygiene-and-sweettest` pushed to
`origin/wave1-manifest-hygiene-and-sweettest`, 11 commits beyond `dev`:

```
367d1286 ci(holochain): add manifest-hygiene stage to DNA pipeline
af410429 chore(orchestrator): register manifest-hygiene in DNA build-manifest
cb128cde docs(holochain): README for manifest-hygiene crate
06f704d8 chore(husky): run manifest-hygiene schema test on dna.yaml/happ.yaml changes
265858ba test(holochain): sweettest compile — enable webrtc backend + document Che blocker
948b02ad test(holochain): manifest-hygiene schema contract test
d4aadee5 docs(holochain): bootstrap-steward authority frame spec + module docstring refresh
f841773e test(holochain): sweettest workspace scaffolding for 5 DNAs
c1a23bb2 docs(holochain): forward-compat policy for DNA lineage
462609c8 feat(holochain): port bootstrap-steward pattern to mishpat/node-registry/lamad
4a0a397a feat(holochain): DNA manifest hygiene — stable seeds, lineage, modifiers
3ffb9ccb docs(plans): R&O-lessons wave plans and cross-wave guidance
```

Branch pushed twice with husky engaged (no `HUSKY=0`). Both pushes passed.
The first push gate emitted `schema-dna` only (manifest-hygiene wasn't yet
registered in `build-manifest.json`); the third push (after
`cb128cde`) emitted both `schema-dna` and `manifest-hygiene`, and
`manifest-hygiene` ran 10 tests in 0.00s inside a 7s total gate. The
husky feedback loop is now real.

### §1.2 Q1 brainstorm result

Ran as a written design memo rather than live collaborative dialogue
(orchestration session holds the vision but was not in the brainstorm
seat). Memo at
`genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md`.

Outcome: **validate option (b) — persistent identity, graduated authority.**

Rationale: (a) dissolution-at-quorum requires a quorum mechanism that does
not exist and should not be invented just for bootstrap management; (c)
rotation collapses to (b) given Holochain's immutable `progenitor_pubkey`
modifier; (b) composes cleanly with the existing
`STEWARD_CAPABILITY_TIERS` graduated-capability framework — bootstrap
steward is the **initial `constitutional`-tier steward at DNA install
time**.

Code change: doc-only. The four `bootstrap_steward.rs` module headers
now explicitly state that the module exposes **identity only**; authority
checks must flow through the stewardship-grant resolution layer, not
through `is_bootstrap_steward`. No integrity-zome gates added (deliberately
— adding them now would calcify exclusive authority, the opposite of
graduated authority).

Scaffolding gap openly documented in the memo: until
`StewardshipGrant`-based validators are wired in the four bootstrap-
steward DNAs, practical authority defaults to the bootstrap pubkey by
lack-of-alternative. Wave 2+ must close this gap before any non-alpha
network publishes.

### Schema contract test location

`elohim/holochain/tests/manifest-hygiene/` — standalone Cargo crate
(not a member of the sweettest workspace; deliberately isolated from the
holochain dep graph). Ten assertions over `dna.yaml` × 5 + `happ.yaml`:

1. `manifest_version == "1"` on every `dna.yaml`
2. `dna.yaml` `name` matches expected
3. `network_seed == elohim_<dna>_alpha`
4. Top-level `lineage:` present; entries look like DNA hashes
5. `happ.yaml` version 1, all 5 roles declared
6. Every role has `clone_limit: 0`
7. happ role seeds match dna.yaml seeds (cross-file coherence)
8. Bootstrap-steward DNAs declare `progenitor_pubkey` in modifiers
9. Infrastructure does NOT declare `progenitor_pubkey`
10. No bare `progenitor` vocabulary in dna.yaml (surface-language check)

Empirical run time: 0.01s. All 10 green on first successful push.

### Sweettest-in-Che spike outcome

**Verdict: spike blocked by environment, not by design.**

- Removed `default-features = false` from the `holochain` dep. Without
  defaults, `tx5-connection-0.8.1` fails at compile with
  `fn default() -> Self {}` for `BackendModule` — its `Default` impl is
  feature-gated and collapses when no webrtc backend is enabled.
- With defaults enabled, `datachannel-vendored` kicks in. It builds
  libdatachannel from source via a `cmake` build script.
- **Che's `base-developer-image:ubi10-latest` does not ship cmake.**
  Compile fails at `cmake-0.1.58/src/lib.rs:1132:5`:
  `failed to execute command: No such file or directory`.
- Alternatives considered: `backend-go-pion` needs `go` (not installed);
  user-level `sudo dnf install cmake` is sandbox-denied (infrastructure
  modification outside repo scope); using `/opt/holochain/bin/holochain`
  externally doesn't apply because sweettest is in-process.

Sweettest test bodies were NOT unignored. The imagodei test body already
describes the bootstrap-steward identity contract (get returns install-
time pubkey; second agent is not the steward) — it just cannot
compile-verify in Che without cmake.

Documented in `elohim/holochain/tests/sweettest/README.md` under
"Che compile blocker (2026-04-21)" with three orchestration options:

1. Add `cmake` + `make` to `Dockerfile` (one-line dnf install add-on).
   Cheapest path. Image rebuild on next workspace respin.
2. Run sweettest exclusively in Jenkins where the build agent has cmake.
3. Switch to `backend-go-pion` and add `go` to the image.

The spike did not push a branch that works around the problem — per the
reshape instruction ("we want real feedback, not a false green").

### Husky hook changes

- `.husky/pre-push`: new `manifest-hygiene` project in the grep-based
  fallback detector + `manifest-hygiene` case in both gate-runner switches
  (schema-style path at line 294 and full-project path at line 400).
- `elohim/holochain/dna/build-manifest.json`: new `manifest-hygiene` step
  + `gate.projects` entry so the graph walker (which takes precedence
  over the grep fallback when a manifest matches) also emits it.
- Invocation:
  `RUSTFLAGS="" cargo test --manifest-path elohim/holochain/tests/manifest-hygiene/Cargo.toml`.
- Verified: push of `cb128cde` triggered `manifest-hygiene: PASSED (6s)`
  with all 10 assertions green.

### Jenkins pipeline wiring

- `elohim/holochain/dna/Jenkinsfile`: new `Manifest Hygiene` stage
  between `Setup Version` and `Setup Nix Cache`. Runs the same
  `cargo test` the husky hook runs, before the 20-40 min DNA WASM build
  so manifest drift fails fast.
- Gated by `shouldRunStep('manifest-hygiene')` so the existing
  partial-rebuild logic honours it.
- The sweettest Jenkins stage is **not wired here** — that's
  out-of-scope (`/shift` territory) and depends on the cmake decision
  above. The sweettest README captures the pending work.

### `feedback_shift_measure_jenkins.md` — nuance suggested

Current memory rule: "`/shift measures → Jenkins` — Che has no docker/
k8s/holochain; shift Objective measures MUST use Jenkins MCP, not local
shell."

What the spike revealed: the rule is **specifically about shift
Objective measures** (feature completion, pipeline green). **Dev-time
verification that does not require a conductor** (pure YAML parsing,
type checks, JSON schema validation, formatter/linter checks) is a
legitimate Che local concern and can / should gate at husky.

Proposed memory update (orchestration to decide):

> Shift Objective measures live in Jenkins (local Che has no
> docker/k8s/holochain). Dev-time pre-push gates that don't need those
> runtimes (schema contracts, type checks, lint) can and should run
> locally in Che via husky — these catch drift before Jenkins spins.
> The split is about **what you're measuring**, not **where you run
> tests**.

### What's deferred to /shift

- **Actually running the holochain pipeline on this branch.** Trigger
  the elohim-holochain pipeline to verify: (a) the new Manifest Hygiene
  Jenkins stage runs cleanly, (b) the bootstrap-steward ports compile
  under Nix, (c) the full DNA WASM build + hApp pack still completes.
- **Sweettest Jenkins integration.** Depends on cmake decision. Five
  per-DNA stages in parallel per §2.1.2; all must gate green.
- **Remaining DNA test bodies** for mishpat / lamad / node_registry /
  infrastructure. The imagodei test body exists; the four others carry
  minimal stubs plus TODOs. Normal sprint work — not a reshape concern.
- **Scaffolding gap: `StewardshipGrant`-based validators** in the four
  bootstrap-steward DNAs. Without this, "graduated authority" is only
  half-implemented (see §4a brainstorm outcome). Belongs with each
  DNA's feature sprints.
- **Pre-existing `sovereignty`/`sovereign` vocabulary** in
  `elohim/holochain/docs/*.md` — excluded from this reshape per the
  instruction. Editorial sweep belongs in its own sprint.

### Reshape verdict

**Pass with reshape remainder.** The reshape hit its primary goal —
dev-time feedback for manifest drift is now real and husky-gated in Che.
The one explicit stretch goal (sweettest green-in-Che) is blocked by a
concrete, documented environment gap (cmake) that orchestration can
resolve in one of three well-scoped ways. Everything else — commits,
brainstorm, schema test, husky wiring, Jenkins wiring — landed.

Orchestration should:
1. Decide the cmake question (image add / Jenkins-only / backend-go-pion).
2. Trigger the elohim-holochain pipeline on
   `wave1-manifest-hygiene-and-sweettest` to verify the Jenkins side.
3. Resolve the nuance on `feedback_shift_measure_jenkins.md` (update
   memory or reject the proposed wording).
4. Only then consider Wave 2 spawn-readiness.
