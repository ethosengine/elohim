# Wave 2 Execution Plan — Credibility & Safe-Rollout

**Date:** 2026-04-21
**Status:** Ready to kick off (awaiting Gate A pass + session spawn from orchestration)
**Wave scope:** Sub-projects #1 (Release discipline) + #2 (Feature flags)
**Prereq reading:**
- `genesis/docs/plans/2026-04-21-rno-lessons-cross-wave-guidance.md` (shared context — mandatory)
- `genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md` §4 (Wave 1 outcome — check for reshape constraints)

**Source roadmap:** `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md` §5.1 and §5.2

---

## For the session executing this wave

You are a dedicated sprint session spawned from an orchestration session that holds the overall vision. Your job is to execute Wave 2 end-to-end and return a pass/reshape verdict to the orchestration session. **Do not start Wave 3.**

Wave 3 is #4 hREA alignment — a multi-week XL project that requires a **heavy Gate B brainstorm from scratch** before any implementation. That brainstorm happens in the orchestration session, not here.

**Order of operations:**
1. Read the cross-wave guidance doc (mandatory).
2. Read Wave 1 outcome §4 — if there are "Constraints surfaced for Wave 2," honor them before starting.
3. Start Sprint 2.A (#1). Pre-resolved design decisions in §1; open questions in §1.2 — brainstorm only on the open questions.
4. Run Sprint 2.A through brainstorm (open questions only) → `superpowers:writing-plans` → execute → land.
5. Start Sprint 2.B (#2). Same pattern.
6. Run Gate B prep (§3) — this is **not** the heavy brainstorm itself (that's in orchestration); it's collecting the inputs the orchestration brainstorm will need.
7. Update §4 "Wave 2 outcome."
8. Return to the orchestration session with the verdict and Gate B prep package.

**Do not:**
- Attempt Wave 3 work (hREA / VF-GraphQL).
- Invoke `superpowers:brainstorming` on Wave 3 — that's orchestration's job at Gate B.
- Make release decisions that lock in Wave 3 assumptions (e.g., don't version-pin to VF vocabulary before #4 is designed).

---

## 1. Sprint 2.A — #1 Release discipline

### 1.0 Brief

Establish semantic versioning, CHANGELOG, and `/release` slash command for elohim's shippable artifacts. Migrate to `main = release-only, dev = default` branch policy. Ship the first release (backfilled to current state) as the initial signal.

Roadmap reference: §5.1.

### 1.1 Pre-resolved design decisions

#### 1.1.1 Single root CHANGELOG, sectioned by component

One `CHANGELOG.md` at repo root. Section by component: `elohim-app`, `elohim-storage`, `steward`, `elohim-agent`, `DNAs (infrastructure/mishpat/imagodei/lamad/node-registry)`, `sophia (submodule)`, `Infrastructure/DX`.

Rationale: the protocol is stewarded as a whole; components are authored within it. Fragmenting changelogs fragments the story. R&O uses this pattern successfully.

Format: Keep-a-Changelog with emoji section headers per R&O convention (✨ Features, 🐛 Bug Fixes, ♻️ Refactor, 📝 Docs, 🏗️ Infra, 💥 Breaking).

#### 1.1.2 Semver scope

- **Elohim protocol version** (the umbrella) bumps MAJOR on any DNA hash change. This is a real compatibility break at the protocol layer.
- **Component versions** (elohim-app, elohim-storage, steward, etc.) move independently within a protocol major version. Each follows standard semver.
- CHANGELOG records both: a protocol-version heading at the top of each release, with component version bumps listed inside.

#### 1.1.3 Pre-release identifiers

- `-alpha.N` before first stable release within a protocol major
- `-rc.N` for release candidates
- **No betas.** Keep it simple.

#### 1.1.4 /release command ↔ Jenkins split

- `/release` (slash command) **prepares** the artifact: finalize CHANGELOG, bump versions, create annotated git tag, push tag.
- **Jenkins** (on tag push) **builds and publishes**: runs the full release pipeline, produces artifacts, publishes to distribution channels.
- Zero manual build steps. The human hits `/release` and reviews Jenkins output.

#### 1.1.5 Branch policy migration

- Add `main` as a new protected branch, initially identical to `dev`.
- `dev` remains the default branch for PRs and everyday work.
- `main` only advances on release — Jenkins fast-forwards `main` to a tagged commit on `dev`.
- Migrate in-flight branches: nothing changes; everyone keeps branching from `dev`.
- First release is the first `main` advance.

### 1.2 Open questions for sprint brainstorm

1. **Backfill version number** — what is v0.1.0? Current tip of `dev`, or do we need to stabilize something first? If Wave 1 just landed, the post-Wave-1 commit is the natural v0.1.0.
2. **Sophia submodule versioning** — it has its own lifecycle. Do we pin a sophia version per elohim release, or let it float? (Pin recommended; surface in CHANGELOG.)
3. **DNA hash bump cadence** — if Wave 1 #7 did the lineage work, what counts as a DNA hash change going forward? Need explicit policy so MAJOR bumps aren't accidental.
4. **CHANGELOG authoring workflow** — conventional-commits-style automation, or manual per-PR entries in an Unreleased section? Small team argument for manual; automation argument for scale.
5. **What's the release artifact distribution** — tag + GitHub release notes suffice for now, or do we need a publishing target (npm, cargo, Docker registry)? Defer most of this; enumerate what the first release actually ships.

### 1.3 Definition of done

- [ ] `CHANGELOG.md` at repo root, sectioned per §1.1.1, backfilled with current state as v0.1.0 (or alpha).
- [ ] `RELEASE_CHECKLIST.md` documents the step-by-step release process.
- [ ] `.claude/commands/release.md` implements the `/release` slash command.
- [ ] Semver policy documented in `CLAUDE.md` (or root doc), covering §1.1.2 and §1.1.3.
- [ ] `main` branch exists, protected, advanced from `dev` at the first release tag.
- [ ] First release tag (v0.1.0 or v0.1.0-alpha.1) pushed and Jenkins published.
- [ ] Branch protection rules updated.
- [ ] CLAUDE.md updated with branch policy section.

### 1.4 Memory rules

Honor:
- `feedback_shift_measure_jenkins.md` — release build closes on Jenkins green
- `project_no_sovereignty_stewardship_over_ownership.md` — vocabulary review on CHANGELOG authoring (no "owner" language in release notes)

---

## 2. Sprint 2.B — #2 Feature flags

### 2.0 Brief

Replace ad-hoc flag checks with an atomic feature-flag system that cleanly separates **declared flags** (intent, config) from **observed state** (reality, derived). Establish the distinction in code and in docs so future flag work doesn't re-conflate them.

Roadmap reference: §5.2.

### 2.1 Pre-resolved design decisions

#### 2.1.1 Flags vs state — rigorous separation

Guidance §3.2 is the governing principle. Restated for this sprint:

- **Flags** (this sprint's scope): declared intent, build-time env vars or config. Safe, stateless, TS-side. Examples: `MOCK_BUTTONS_ENABLED`, `PEERS_DISPLAY_ENABLED`, `EXPERIMENTAL_QUIZ_SOUND`.
- **State** (out of scope for this sprint): derived from observation of real signals. Lives in elohim-agent / state machines. Never a boolean toggle. Examples: `Phase::ElohimActive`, `Peer::Healthy`.

This sprint builds the **FeatureFlagsService** for flags only. It does not touch state machines. Anyone later proposing a "state flag" gets pointed at guidance §3.2.

#### 2.1.2 Service placement

Shared `FeatureFlagsService` lives in `elohim-library` (`app/elohim-library/projects/elohim-service/src/services/feature-flags.service.ts` or similar). Each shell overrides the flag source:

- **Web (elohim-app)** — flags from Vite `import.meta.env`.
- **Tauri (steward/device)** — flags from Tauri config or IPC at bootstrap.

Override mechanism: DI token for the flag-source adapter; shell provides its own implementation.

#### 2.1.3 Naming convention

- `ELOHIM_FLAG_<SCOPE>_<NAME>` — uppercase, snake case, scoped.
- Scopes correspond to pillars: `LAMAD`, `IMAGODEI`, `SHEFA`, `QAHAL`, `ELOHIM` (cross-cutting), `DEV` (developer-only toggles).
- Example: `ELOHIM_FLAG_LAMAD_EXPERIMENTAL_QUIZ_SOUND`, `ELOHIM_FLAG_DEV_MOCK_PEERS`.

#### 2.1.4 Runtime toggle UI

Defer. Build-time is sufficient for v1. If steward-operators need runtime toggles later, design that as a separate mini-project (it's a steward-admin concern, not a feature-flag concern).

#### 2.1.5 Migration target — quiz-sound

R&O's example migration was env-mode → atomic flags. Elohim's equivalent: `app/elohim-app/src/app/lamad/quiz-engine/services/quiz-sound.service.ts` has ad-hoc flag checks. Migrate it as the reference example.

### 2.2 Open questions for sprint brainstorm

1. **Flag registry format** — TypeScript enum, JSON file, or TS const object? Consider discoverability (dev reads one file to see all flags) and type safety (compile-time checking).
2. **Unknown-flag behavior** — read of an undeclared flag: throw, warn, or silently return false? (Recommendation: throw in dev, false with warning in prod. Confirm.)
3. **Tauri flag source** — does steward read from `tauri.conf.json`, a separate `flags.json` in app config dir, or an IPC call at startup? Pros/cons of each.
4. **Flag documentation location** — inline JSDoc on each flag, or a separate `FEATURE_FLAGS.md` catalog, or both?
5. **Sweep target list** — besides `quiz-sound.service.ts`, what other code has ad-hoc flag checks that should migrate in this sprint? Run a grep.

### 2.3 Definition of done

- [ ] `FeatureFlagsService` interface + default implementation in elohim-library.
- [ ] Web shell (elohim-app) provides Vite env-var adapter.
- [ ] Tauri shell (steward/device) provides its adapter per §2.2 Q3 outcome.
- [ ] `quiz-sound.service.ts` migrated to use `FeatureFlagsService`; ad-hoc checks removed.
- [ ] Any other ad-hoc flag sites found in sweep (§2.2 Q5) migrated.
- [ ] Flag registry per §2.1.3 naming convention, documented per §2.2 Q4 outcome.
- [ ] `CLAUDE.md` gains a section explaining the **flags vs state** distinction with pointers to guidance §3.2.
- [ ] Unit tests for FeatureFlagsService (default behavior, override, unknown-flag handling).
- [ ] No code mentions "flag" for anything that is actually observed state.

### 2.4 Memory rules

Honor:
- `project_elohim_active_observed_not_flagged.md` — **primary**, the whole sprint's governing rule
- `project_elohim_agent_sense_respond_architecture.md` — flags are TS-side only; gates stay in Rust
- `feedback_schema_first_ioc.md` — flag registry is a contract; document it schema-first

---

## 3. Gate B prep (heavy Gate B runs in orchestration)

This section **collects inputs** for the heavy Gate B brainstorm that happens in the orchestration session. The sprint session does not run the Gate B brainstorm itself.

### 3.1 Light Wave 2 retrospective checks

- [ ] Sprint 2.A DoD met (§1.3).
- [ ] Sprint 2.B DoD met (§2.3).
- [ ] First release published on Jenkins.
- [ ] Vocabulary audit passed (no sovereignty language in CHANGELOG, flag names, docs).
- [ ] Flags-vs-state distinction survives review (nothing mislabeled).

### 3.2 Gate B inputs to collect for orchestration

The orchestration session's Gate B brainstorm is on sub-project #4 (hREA / VF-GraphQL alignment). Collect and surface to orchestration:

1. **Wave 1/2 quality baseline** — what does "elohim is worthy of being graduated into" look like post-Wave-2? Specifically: DNA hygiene status, test coverage level, release credibility.
2. **Graph substrate status** — is the parent session's graph substrate landed / stable / still in flight? This is the prerequisite for path (b) VF-GraphQL views.
3. **Release constraints on Wave 3** — anything in §1 that locks future assumptions about versioning that would affect hREA alignment? (E.g., if we pin a VF vocabulary version, that becomes a semver commitment.)
4. **Flag-system constraints on Wave 3** — any flags created in §2 that presume hREA work? (There should be none; flag this if there are.)
5. **VF team coordination status** — has Lynn Foster / Bob Haugen / VF team been engaged? If not, orchestration should open that conversation before Gate B brainstorm.
6. **Impedance questions surfaced during Wave 1/2** — anything in #7 or sweettest work that touched shefa-domain types and revealed VF-alignment questions? Surface them.

Write these into §4 below as the Gate B prep package.

---

## 4. Wave 2 outcome (filled in at close)

**Status:** Not yet started.

**Sprint 2.A outcome:** _TBD — commit links, release tag, any deferred work._

**Sprint 2.B outcome:** _TBD._

**Gate B prep package for orchestration:**

1. Wave 1/2 quality baseline: _TBD_
2. Graph substrate status: _TBD_
3. Release constraints on Wave 3: _TBD_
4. Flag-system constraints on Wave 3: _TBD_
5. VF team coordination status: _TBD_
6. Impedance questions surfaced: _TBD_

**Verdict:** _TBD (pass-to-Gate-B-brainstorm / reshape-wave-2)._

**Notes for orchestration session:** _TBD._
