# Gitoxide Upstream Alignment — Design

**Date:** 2026-04-20
**Status:** Approved (brainstorming complete, ready for implementation plan)
**Author:** Matthew Dowell + Claude Opus 4.7
**Predecessor sprints:**
- `docs/superpowers/sprint-results/2026-04-19-brit-cli-test-page.md` — built our Linux-only test infrastructure
- `docs/superpowers/specs/2026-04-19-brit-cli-test-page-design.md` — designed that infrastructure

## TL;DR

Establish the operational foundation for contributing to upstream gitoxide (`GitoxideLabs/gitoxide`) with maintainer respect, before any upstream PR ships. Master the territory first — their practices, their CI, their objectives, their taste — so when we do approach Sebastian Thiel with a PR, it serves his roadmap and costs him minimal review time.

Two structural changes, four reference artifacts, one self-imposed readiness gate.

**Structural:**
1. A `gix-main` branch on our fork that is an auto-synced mirror of `upstream/main`. Never carries brit commits.
2. An explicit **upstream-PR workflow** where every PR to gitoxide branches off `gix-main`, never `main`.

**Artifacts (deliverables of this spec's implementation):**
1. `gitoxide-house-style.md` — distilled pre-PR checklist from CONTRIBUTING/DEVELOPMENT/COLLABORATING/STABILITY
2. `gitoxide-local-ci.md` — reproducible setup for running `just ci-test` across all 4 feature matrices locally
3. `gitoxide-objectives.md` — cross-referenced map of `tasks.md` + `SHORTCOMINGS.md` + `crate-status.md` + recent merge/reject PR patterns → where Sebastian actually wants help
4. `gitoxide-first-pr-candidates.md` — 3-5 ranked potential first contributions, each justified against (3)

**Gate:** A self-imposed readiness checklist. No upstream PR ships until every item is green.

**Non-goal for this spec:** the actual first PR, the multi-platform testing strategy, any rewrite of our existing `cli-journey`/`cli-test-page`/`baseline.md` work. Those are downstream decisions, informed by this foundation.

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
- **Not** implementing `push` / `commit` / any feature work in gitoxide. Those become candidates in `gitoxide-first-pr-candidates.md`; selection happens after this spec.
- **Not** choosing which PR to ship first. The shortlist is ranked but unselected. Selection requires follow-up discussion with Sebastian (via issue or discussion, per COLLABORATING.md).

## Architecture

### Branch topology

```
origin (our fork: ethosengine/brit)
├── main             ← brit dev branch (79 commits ahead of upstream/main)
│                       carries the brit binary, brit-verify, brit-build-ref,
│                       cli-journey, cli-test-page, baseline.md — our work
│
├── gix-main         ← pristine mirror of upstream/main
│                       auto-synced via .github/workflows/sync-upstream.yml
│                       NEVER carries brit commits
│                       starting point for all upstream PRs
│
├── feat/*-brit      ← feature branches for brit-only work (off main)
└── feat/*-upstream  ← feature branches for upstream PRs (off gix-main)
```

Every upstream PR starts from `gix-main`. Diff to `upstream/main` is pristine — no brit commits leak in. When the PR is ready, push the branch to `origin`, then open the PR at `GitoxideLabs/gitoxide` (head = `ethosengine:feat/xxx-upstream`, base = `main`).

When upstream merges it, we either:
- Wait for the auto-sync to pull it into our `gix-main`, then merge `gix-main → main` to get the change into brit
- Or merge `upstream/main → main` directly

`main` continues to carry brit-specific work. The two streams of work are cleanly separated.

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

### 6. First-PR candidate shortlist — `docs/gitoxide-first-pr-candidates.md` (in brit repo, on `main`)

3-5 ranked candidates for our first upstream PR, each with:
- **What**: crisp scope (what we'd change)
- **Why it serves the maintainer**: which objective(s) from the objectives map this maps to
- **Estimated scope**: LOC, crates touched, stability tier
- **Risks**: what could go wrong, what reviewer pushback might look like
- **Prereq engagement**: should we open an issue/discussion first? (Almost always yes for anything non-trivial.)
- **Our skill fit**: do we actually have the domain to execute this cleanly, or would we be overreaching?

Ranked by (maintainer value × our fitness × reversibility / total effort).

The output is NOT a selection. It's a menu informed by (5) and constrained by our honest self-assessment.

## Sequencing

This spec is implemented in three phases. Each produces concrete artifacts.

### Phase 1 — Infrastructure (this spec session, already partial)
- [x] Create `gix-main` branch, push to origin
- [x] Install `just` locally
- [ ] Add `.github/workflows/sync-upstream.yml`, commit to `main`
- [ ] Verify `just check` runs (at minimum)
- [ ] Commit this spec to `docs/superpowers/specs/`

### Phase 2 — Study (next work session, roughly 2 days)
- [ ] Write `docs/gitoxide-house-style.md`
- [ ] Write `docs/gitoxide-local-ci.md` (after actually running `just ci-test` ourselves)
- [ ] Write `docs/gitoxide-objectives.md`

### Phase 3 — Candidate selection readiness (after Phase 2, roughly 1 day)
- [ ] Write `docs/gitoxide-first-pr-candidates.md`
- [ ] Review the candidate list against the readiness gate
- [ ] Decide: do we engage Sebastian via issue/discussion on a candidate, or do we hold and keep studying?

## Open questions — to resolve after Phase 2

These don't block Phase 1 but need answers before Phase 3 candidate selection:

1. **Do we engage Sebastian via issue or discussion first?** COLLABORATING.md suggests yes for non-trivial work. The shortlist doc should state our plan per-candidate.
2. **What's the right cadence for `main ← upstream/main` merges?** Do we merge continuously, or only when an upstream change affects brit's correctness? (Affects how fresh our main stays with upstream fixes.)
3. **Do we adopt their `tests/snapshots/` layout for OUR future tests?** Or keep our `baseline.md` single-file approach on `main`? (Decided: keep `baseline.md` on `main`; any test we'd upstream gets reshaped to `tests/snapshots/` on a feat/*-upstream branch.)
4. **Does Sebastian accept GitHub Actions sync workflows in forks?** (Our workflow is in OUR fork, not upstream — so this is a non-issue for upstream, just a note to ourselves.)

## Risks

- **Auto-sync failure goes unnoticed** — if the sync workflow breaks (credential expiry, upstream force-pushes, etc.), `gix-main` silently drifts. Mitigation: the workflow should email/notify us on failure. Low priority until we actually depend on freshness.
- **We never make it past Phase 2** — study phases can become perpetual. Mitigation: Phase 3 has a hard time budget; after it expires, we either ship a PR or explicitly decide to keep studying with a new budget.
- **Overfit to gitoxide's current state** — Sebastian's priorities change. Our objectives map is a snapshot. Mitigation: date-stamp all the reference docs; re-read `tasks.md` before selecting any candidate.
- **The gate becomes bureaucracy** — if every PR requires ticking 15 boxes, we'll avoid shipping. Mitigation: the gate is specifically for the FIRST upstream PR. Subsequent PRs can skip gate items we've already proven we reliably do (e.g., after 3 PRs we don't need to re-prove we can run `just ci-test`).

## What this spec does NOT decide

- Which PR we ship first (Phase 3 candidate selection does)
- Whether to rewrite `cli-journey` / `cli-test-page` / `baseline.md` (deferred — they're on `main`, out of the upstream-PR workstream)
- Multi-platform testing strategy (deferred — informed by what we see in gitoxide's own CI)
- How we handle `push` / `commit` feature gaps upstream (candidate in Phase 3, execution in a later spec)
- Our position on SHA-256 / reftables / partial clones (candidate in Phase 3)
