# Gitoxide Upstream Alignment — Design

**Date:** 2026-04-20
**Status:** Approved, amended same-day with `gix-brit` playground reframe
**Author:** Matthew Dowell + Claude Opus 4.7
**Predecessor sprints:**
- `docs/superpowers/sprint-results/2026-04-19-brit-cli-test-page.md` — built our Linux-only test infrastructure
- `docs/superpowers/specs/2026-04-19-brit-cli-test-page-design.md` — designed that infrastructure

**Amendment note (2026-04-20, second brainstorm):** The original spec had a Phase 3 deliverable of a "first-PR candidate shortlist" — a speculative, pre-selected menu of candidates to bring to Sebastian. In a follow-up brainstorm the same day we concluded this framing was timid and passive. The shortlist was deleted. Replaced with `gix-brit`: a dedicated gitoxide-contribution playground branch where we hack on gitoxide with upstream discipline, and first-PR candidates emerge naturally from real work rather than being speculatively chosen.

## TL;DR

Establish the operational foundation for contributing to upstream gitoxide (`GitoxideLabs/gitoxide`) with maintainer respect. Do it by BUILDING, not studying: set up a dedicated contribution-playground branch (`gix-brit`) where we work on gitoxide in upstream shape for our own needs. First upstream PRs emerge as natural byproducts of that work — commits we made for ourselves that turn out to be valuable enough to cherry-pick upstream.

Three structural changes, three reference artifacts, one self-imposed readiness gate.

**Structural:**
1. A `gix-main` branch on our fork that is an auto-synced mirror of `upstream/main`. Never carries brit commits.
2. A `gix-brit` branch, off `gix-main`, as our active **contribution playground**. Pure gitoxide tree. We hack on gix freely, but commit-at-commit we hold ourselves to upstream conventions (conventional commits, AI trailer, thiserror, no unwrap, tests-first). Diverges from `gix-main` as we work; rebased periodically for freshness.
3. An explicit **upstream-PR workflow** where every PR to `GitoxideLabs/gitoxide` is a `feat/*-upstream` branch off fresh `gix-main`, cherry-picked from `gix-brit` when a commit or chain proves upstream-worthy.

**Artifacts (deliverables of this spec's implementation):**
1. `gitoxide-house-style.md` — distilled pre-PR checklist from CONTRIBUTING/DEVELOPMENT/COLLABORATING/STABILITY
2. `gitoxide-local-ci.md` — reproducible setup for running `just ci-test` across all 4 feature matrices locally
3. `gitoxide-objectives.md` — cross-referenced map of `tasks.md` + `SHORTCOMINGS.md` + `crate-status.md` + recent merge/reject PR patterns → shapes our SENSE of what's valuable but does NOT pre-pick candidates

**Gate:** A self-imposed readiness checklist. No upstream PR ships until every item is green. Applies at the **feat/*-upstream cherry-pick moment**, not to raw gix-brit commits.

**Non-goal for this spec:** the actual first PR, the multi-platform testing strategy, any rewrite of our existing `cli-journey`/`cli-test-page`/`baseline.md` work. Those are downstream decisions, informed by this foundation.

**Explicitly dissolved:** The speculative "first-PR candidate shortlist" deliverable. We don't need to guess what Sebastian wants — we build what WE need in upstream shape, and the upstream-valuable subset surfaces itself.

## Problem

We merged upstream/gitoxide into our brit fork today and discovered our current test infrastructure (`cli-journey`, `cli-test-page`, `baseline.md`) is largely a reinvention of the Rust ecosystem's gold-standard CLI testing stack (`trycmd` + `insta`), done less robustly and Linux-only. We don't know:

- What gitoxide's practices actually require of a pristine PR
- Whether our CI can green the same way theirs does (4 feature matrices, multi-platform, fuzz, wasm, etc.)
- Which of our novel ideas (MockRemote, HTML-rendered CLI captures) are genuinely upstream-contributable vs. fork-only
- What Sebastian's current priorities are — where help would serve his roadmap vs. where a PR would cost review time without value
- What a "pristine PR" even feels like from the reviewer's side of the loop

Shipping upstream PRs without that foundation costs the maintainer time we haven't earned. Gitoxide is a single-maintainer-driven project; our standard has to be "don't ship until we're confident we're delivering serious value to his roadmap, not ours."

The deeper constraint: **gitoxide's COLLABORATING.md is explicit that "for crates you do not own, for major or architectural changes please open a discussion, an issue or a PR to allow participation and don't merge until there is agreement."** That's not just etiquette — it's the maintainer saying "engage me first, don't surprise me." Any contribution workflow we adopt has to respect that.

## Non-Goals

To prevent scope creep:

- **Not** shipping a first PR. The spec produces the conditions for a first PR; the first PR itself is a later decision.
- **Not** designing the multi-platform test strategy. That's a downstream concern once we see how gitoxide's own testing handles Windows/macOS.
- **Not** archiving or rewriting our existing brit test infrastructure. `main` carries what it carries; `gix-main` is a separate pristine workspace. Cleanup of `main` is a future decision.
- **Not** implementing `push` / `commit` / any feature work in gitoxide as this spec's scope. Those become natural gix-brit work items when we decide to hack on them.
- **Not** choosing which PR to ship first. First PR emerges from real gix-brit work — not pre-selected from a speculative shortlist.
- **Not** porting brit's existing main-branch work (brit-cli, cli-journey, cli-test-page, baseline.md, sibling-path deps to rakia) into gix-brit. gix-brit starts from pristine gix-main and stays pristine-gitoxide-shape.

## Architecture

### Branch topology

```
origin (our fork: ethosengine/brit)
├── main             ← brit-the-product (unchanged, 79 commits ahead of upstream/main)
│                       carries brit binary, brit-verify, brit-build-ref,
│                       cli-journey, cli-test-page, baseline.md, rakia sibling deps
│                       - dev continues here for brit product features
│                       - periodically merges from gix-main to pull upstream fixes
│                       - ci.yml fails here due to sibling-path deps — acceptable
│                         known issue, deferred to Phase 2 Local-CI-Runbook
│
├── gix-main         ← pristine mirror of upstream/main
│                       auto-synced via .github/workflows/sync-upstream.yml
│                       NEVER carries brit commits
│                       starting point for gix-brit AND for all upstream PRs
│
├── gix-brit         ← gitoxide contribution playground, off gix-main
│                       - pure gitoxide tree, no brit customizations
│                       - we hack freely (YOLO on problems we want to solve)
│                       - but each commit holds to upstream conventions:
│                         conventional commits, AI trailer, thiserror, no unwrap,
│                         tests accompany code, etc.
│                       - diverges from gix-main as we work
│                       - **rebased** on gix-main periodically for freshness
│                         (force-push to origin/gix-brit is OK — solo branch)
│                       - commits are NOT automatically upstreamed
│
├── feat/*-brit      ← feature branches for brit product work (off main)
└── feat/*-upstream  ← feature branches for upstream PRs
                        branched off fresh gix-main, NOT off gix-brit
                        individual commits cherry-picked from gix-brit
                        polished, readiness-gated, then PR'd upstream
```

### Two concerns, cleanly separated

| Concern | Home branch | Character of work |
|---|---|---|
| Brit-the-product development | `main` | Build brit binary, verify, build-ref, use gix as library, dogfood |
| Gitoxide improvement work | `gix-brit` | Hack on gitoxide; YOLO + discipline; no PR pressure |
| Actual upstream PR submission | `feat/*-upstream` | Cherry-picked from gix-brit, polished, PR'd |

### How first PRs emerge naturally

1. We work on `gix-brit` — on whatever gix problem we want to solve, in upstream shape
2. Each commit has conventional message, AI trailer, tests, no unwrap, thiserror errors
3. Over time, gix-brit accumulates improvements
4. **When a commit or chain proves valuable upstream** (we hit it multiple times, or we know other gitoxide users would want it), we:
   - Branch `feat/<topic>-upstream` off current `gix-main` (pristine)
   - `git cherry-pick <commit(s)>` from `gix-brit`
   - Polish: address any conflicts from gix-main freshness, fill gaps
   - Apply the readiness gate to the final set
   - Push to our fork, open PR at `GitoxideLabs/gitoxide`
5. When upstream merges, the change lands in `upstream/main` → `gix-main` auto-syncs → we rebase `gix-brit` on gix-main → our commit now exists "above" upstream's version (usually becomes a no-op and drops during rebase)

### Why not port main's work into gix-brit?

- **main's brit-cli has sibling-path deps** to `../rakia/rakia-brit` and `../rakia/rakia-core` that break in any tree that doesn't have rakia as a sibling. gix-brit needs to stay self-contained.
- **main's cli-journey and cli-test-page** are tech debt (reinventing trycmd+insta poorly). We don't want to carry that into our contribution surface.
- **main's baseline.md** is a brit-product artifact, not upstream-shape.
- **main as the brit product is FINE.** It ships what it ships. Leaving it alone is the right move.

Eventually, when we want brit features to benefit from our gix-brit improvements, they flow naturally: gix-brit commits → upstream PRs → upstream/main → gix-main → `main` via periodic merge. The round-trip is slow but clean.

### Sync workflow

A GitHub Actions workflow on `main` that runs:
- On schedule (every 6 hours)
- On `workflow_dispatch` (manual trigger)

Does:
1. Fetch `upstream/main`
2. Checkout `gix-main`, fast-forward-only merge from `upstream/main`
3. Push `gix-main` back to `origin`

Fails safely if fast-forward is impossible (meaning `gix-main` has drifted — which should never happen but we want to detect it).

### Local dev environment

Install `just` (cargo install just) as the task runner. Gitoxide's entire local CI workflow runs through `just`:

- `just check` — compile across all 4 feature sets
- `just clippy` — clippy across all 4 feature sets
- `just unit-tests` — all unit tests
- `just journey-tests` — shell-based integration tests
- `just doc` — doc build
- `just check-mode` — verify file modes are correct (they have scripts that die on CRLF, etc.)
- `just test` — all of the above
- `just ci-test` — CI's version (what CI actually runs)

We mirror this locally. Our readiness gate requires `just ci-test` to be green before we push any upstream PR branch.

## Practices we inherit from upstream

Distilled from CONTRIBUTING.md / DEVELOPMENT.md / COLLABORATING.md / STABILITY.md:

### Commit messages — load-bearing, not cosmetic

- `cargo smart-release` reads commit messages to auto-generate changelogs and pick per-crate version bumps
- **Use conventional commits ONLY for user-visible changes:** `feat:`, `fix:`, `change!:`, `remove!:`, `rename!:`
- **Bare messages for everything else:** no `chore:`, no `refactor:` prefixes — those "don't affect users of the API"
- **Breaking changes MUST split into two commits** so `smart-release` attributes the break to the right crate:
  1. First: the break itself, minimal (`change!: rename Foo to Bar`)
  2. Second: adapters (`adapt to changes in <crate>`)

### Commit history

- **No squashing.** "Track everything" approach. More commits is fine.
- Feature branches + PRs are fine, merged with merge commits (not rebased/linearized)
- Stacked Git is endorsed for organizing commits by topic
- AI disclosure via `Co-authored-by: <agent-identity>` trailer in commits (or PR comment)

### Code style

- **test-first** — write tests before/alongside code
- **never unwrap()** — not even in tests. Use `expect("why")` if necessary, with the `why` explaining the invariant
- **thiserror everywhere** for error chains
- **use git itself as reference implementation** — "run the same test against git whenever feasible"
- **use libgit2 fixtures** where appropriate

### Trunk-based, `main` never broken

- Short-lived PRs preferred over long-lived branches
- `main must never be broken or show warnings` — run `just test check-size` before pushing
- If `main` breaks on CI and you know the cause, fix or revert immediately
- If you don't know the cause, open a PR to invite collaborators (used as sync primitive)

### Collaboration etiquette

- **For crates you do not own**, major/architectural changes require a **discussion, issue, or PR before merge** — the maintainer wants to see it before you ship
- Minor changes you can just make
- For crates you own, ship freely

**This is the core of "respect the maintainer" — before writing code for anything non-trivial, open the conversation.**

### Testing — their stack

- Shell-based journey tests: `tests/journey/*.sh` sourced into `tests/journey.sh`
- Primitives in `tests/utilities.sh` (`expect_run`, `expect_snapshot`, `sandbox`, `title`, `step`, `with`, `when`)
- Snapshots in `tests/snapshots/{panic-behaviour,plumbing,porcelain}/`
- Internal Rust tool `tests/it/` (`internal-tools` crate) for generating fixture data
- Multi-platform CI matrix: `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-latest`, `windows-latest`, `windows-2025`, `windows-11-arm`, plus a 32-bit Windows job
- Windows platform divergence handled via allowlist: `etc/test-fixtures-windows-expected-failures-see-issue-1358.txt`

### Stability tiers

Crates live in three stability tiers (per `STABILITY.md`). Tier determines how often breaking changes can land. When contributing, respect the target crate's tier.

## Readiness gate — before any upstream PR ships

A commit is ready to propose upstream ONLY when ALL of the following are true:

- [ ] Branch is off `gix-main` (verified: `git merge-base HEAD gix-main == gix-main tip` or trivially close)
- [ ] `just ci-test` green locally across all 4 feature matrices:
  - [ ] `cargo check --workspace`
  - [ ] `cargo check --no-default-features --features small`
  - [ ] `cargo check --no-default-features --features max-pure`
  - [ ] `cargo check --no-default-features --features lean-async --tests`
- [ ] `just clippy` green across all 4 feature matrices (no warnings)
- [ ] `just journey-tests` green (shell journey tests pass)
- [ ] `just doc` green (doc build succeeds, no broken intra-doc links)
- [ ] `just check-mode` green (file modes correct, no CRLF where LF expected)
- [ ] Commits follow their message style:
  - [ ] Conventional prefixes ONLY on user-visible changes
  - [ ] Breaking changes split into break + adapt commits
  - [ ] `Co-authored-by: Claude <claude@anthropic.com>` (or equivalent) trailer on AI-assisted commits
- [ ] No `.unwrap()` introduced; `.expect("why")` with reasoning where needed
- [ ] Error types use `thiserror` where a new error is introduced
- [ ] Target crate's stability tier respected (per STABILITY.md)
- [ ] Change maps to an explicit entry in one of: `tasks.md`, `SHORTCOMINGS.md`, `crate-status.md`, or an existing open GitHub issue
- [ ] **For non-trivial changes**: a GitHub discussion or issue opened FIRST, and the maintainer has had an opportunity to weigh in (per COLLABORATING.md)
- [ ] PR body references the issue/discussion, explains the *why* not just the *what*, and flags any expected-failures/platform caveats

Only when every box is checked does the PR get submitted.

## Deliverables

This spec is complete when the following artifacts exist and are committed:

### 1. `gix-main` branch — ALREADY SHIPPED ✓

Created from `upstream/main`, pushed to `origin/gix-main` as part of this spec's brainstorming session.

### 2. Sync workflow — ALREADY SHIPPED in this spec's implementation

`.github/workflows/sync-upstream.yml` on `main` branch. Runs every 6 hours + on-demand. Fast-forwards `gix-main` from `upstream/main`.

### 3. House-style reference — `docs/gitoxide-house-style.md` (in brit repo, on `main`)

Distilled pre-PR checklist. One source of truth for our team, derived from upstream docs. Structured as:
- Commit message rules (with examples)
- Code style rules (no unwrap, thiserror, etc.)
- Stability tier cheat sheet
- Testing conventions
- AI disclosure format

Implementation: read the four root-level upstream docs; distill into a checklist; commit.

### 4. Local CI runbook — `docs/gitoxide-local-ci.md` (in brit repo, on `main`)

Reproducible setup steps + the `just` command reference for our local equivalent of CI. Includes:
- Install `just` (cargo install just --locked)
- Run `just --list` to see all recipes
- What `just ci-test` actually runs and how long it takes
- Known environmental gotchas (RUSTFLAGS, Nix, etc. — we've hit these before)
- How to run individual journey tests (`just journey-tests`)
- How to run specific feature-matrix checks

Implementation: actually run `just ci-test` on our tree; document what we hit.

### 5. Maintainer objectives map — `docs/gitoxide-objectives.md` (in brit repo, on `main`)

A synthesis of:
- `tasks.md` (upstream) — explicit roadmap
- `SHORTCOMINGS.md` (upstream) — known gaps
- `crate-status.md` (upstream) — feature-parity tracking
- Last 10-20 merged PRs — what Sebastian accepts, what he pushes back on, what he rewrites before merging
- Open discussions/issues tagged `help wanted` or similar

Produces a **ranked understanding of where help would actually help**. Not a candidate list — a map of his objective landscape so WE can propose candidates with informed context.

Implementation: read the upstream docs; scan recent PRs via `gh api`; write synthesis.

### 6. `gix-brit` branch — ALREADY SHIPPED ✓

Created from `gix-main`, pushed to `origin/gix-brit` as part of this spec's amendment. Our contribution playground. Discipline rules:

- **Commits on gix-brit are NOT auto-PR'd upstream** — they're ours to use, keep, discard, rewrite
- **Each commit holds to upstream conventions** regardless: conventional commits (for user-visible), AI trailer on AI-assisted work, thiserror errors, no unwrap, tests-first where practical
- **We stay pristine-gitoxide-shape** — no brit customizations, no sibling-path-deps, no brit-specific renames
- **Rebase, don't merge, when pulling from gix-main** — keeps cherry-picks for feat/*-upstream branches clean and linear
- **Force-push to origin/gix-brit is OK** — solo branch, no collaborators to break

## Sequencing

This spec is implemented in three phases. Each produces concrete artifacts.

### Phase 1 — Infrastructure (complete)
- [x] Create `gix-main` branch, push to origin
- [x] Install `just` locally
- [x] Add `.github/workflows/sync-upstream.yml`, commit to `main`
- [x] Verify `just check` runs (at minimum)
- [x] Commit this spec to `docs/superpowers/specs/`
- [x] Enable GitHub Actions on fork
- [x] Verify `sync-gix-main` workflow runs successfully
- [x] Create `gix-brit` branch, push to origin (spec amendment work)

### Phase 2 — Study (next work session, roughly 2 days)
- [ ] Write `docs/gitoxide-house-style.md`
- [ ] Write `docs/gitoxide-local-ci.md` (after actually running `just ci-test` ourselves)
- [ ] Write `docs/gitoxide-objectives.md`

### Phase 3 — First gix-brit work session (after Phase 2, open-ended)

No pre-selected candidate. We open gix-brit, decide what problem we want to solve today, and we start solving it. The discipline is at the commit level, not the candidate-selection level.

- [ ] First working session on gix-brit — any gix problem we find interesting
- [ ] Set up a lightweight tracking habit: as we hit things worth noting (shortcomings we'd like to address, infra gaps, Windows issues, etc.), we add them to a simple `gix-brit-notes.md` scratch doc — not a prioritized candidate list, just a reminder of threads we'd want to pick up
- [ ] When a gix-brit commit or chain proves upstream-worthy (by our own judgment, informed by the objectives map), cherry-pick to `feat/<topic>-upstream` off gix-main, polish, apply readiness gate, open PR

## Readiness-gate application timing

The ten-item readiness gate applies **at the feat/*-upstream cherry-pick moment**, not to gix-brit commits themselves. Specifically:

- gix-brit commits aim to be upstream-shape but aren't required to pass the full gate
- When we branch `feat/<topic>-upstream` off gix-main and cherry-pick, THAT's when the full gate runs
- Typical flow at cherry-pick time:
  1. `git checkout gix-main && git pull`
  2. `git checkout -b feat/<topic>-upstream`
  3. `git cherry-pick <commits from gix-brit>`
  4. Resolve any freshness conflicts; tidy the chain
  5. Run the full gate locally (`just ci-test`, clippy all matrices, doc, check-mode)
  6. Push; open PR

The gate is protection for Sebastian at the interface, not bureaucracy on our internal work.

## Open questions — to resolve during/after Phase 2

These don't block current work but need answers eventually:

1. **What's the rebase cadence for gix-brit on gix-main?** Once a week? Whenever conflict emerges? Probably "whenever we remember and it's convenient" — solo branch, low stakes.
2. **How do we handle the ci.yml-red-on-main issue?** (Sibling-path deps to rakia.) Options: remove brit-cli from workspace in CI, git-submodule rakia into brit, relocate rakia-brit/rakia-core into brit. Phase 2 Local-CI-Runbook is where this gets decided and potentially fixed.
3. **Do we adopt trycmd+insta for future testing work on main?** (Recognized tech debt in cli-journey/cli-test-page.) Not blocking; probably answered when we next touch that code.
4. **Do we maintain a gix-brit-notes.md scratch doc?** My lean: yes, lightweight. Just a bullet list of "things we noticed on gix-brit that might be worth upstreaming someday." Not a prioritized shortlist.

## Risks

- **Auto-sync failure goes unnoticed** — if the sync workflow breaks (credential expiry, upstream force-pushes, etc.), `gix-main` silently drifts. Mitigation: the workflow should email/notify us on failure. Low priority until we actually depend on freshness.
- **We never make it past Phase 2** — study phases can become perpetual. Mitigation: Phase 2 deliverables are explicit; after they're done, gix-brit work begins whether or not the study feels "complete".
- **gix-brit drifts so far from gix-main that cherry-picks become painful** — Mitigation: rebase gix-brit on gix-main at least weekly during active work; if a specific commit won't rebase cleanly, that's a signal to either upstream it sooner or accept it as gix-brit-only.
- **We build upstream-valuable work on gix-brit but never actually submit it** — the "emerges naturally" model could produce stagnation. Mitigation: periodic (monthly?) review of gix-brit commits, ask "has any chain here become upstream-worthy?" and cherry-pick the yes-answers.
- **Overfit to gitoxide's current state** — Sebastian's priorities change. Our objectives map is a snapshot. Mitigation: date-stamp the reference docs; re-read `tasks.md` before cherry-picking any candidate to a feat/*-upstream branch.
- **The gate becomes bureaucracy** — if every PR requires ticking 15 boxes, we'll avoid shipping. Mitigation: the gate is explicitly for upstream submission, not for gix-brit work. First PR's gate is strict; subsequent PRs can skip gate items we've proven reliably (after 3 PRs we don't re-prove we can run `just ci-test`).

## What this spec does NOT decide

- Which PR we ship first — emerges from gix-brit work, not pre-selected
- Whether to rewrite `cli-journey` / `cli-test-page` / `baseline.md` (deferred — they're on `main`, out of the upstream-PR workstream)
- Multi-platform testing strategy (deferred — informed by what we see in gitoxide's own CI and what we build in gix-brit)
- How we handle `push` / `commit` feature gaps upstream — these become natural gix-brit candidates if we need them for brit product, or just stay upstream-backlog items if we don't
- Our position on SHA-256 / reftables / partial clones (these may become gix-brit topics if we choose)
