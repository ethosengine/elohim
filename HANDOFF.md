# HANDOFF — Routing/projection shakeout landed on sprint; awaits operator dev-merge to deploy + validate

_Last updated: 2026-06-01 · Author: Claude Opus (overnight agentic /shift) · Branch: `sprint/cross-pillar-cleanup`_

---

## Goal

Shake out the EPR-app **routing + projection-cache** defects so the deployed surfaces serve
correctly and fast, then **visually validate** them — so we can start grinding down
`@elohim-visually-validated` a2o scenarios in the genesis pipeline. This handoff covers the
routing-shakeout shift that ran overnight and what the operator must do to land + validate it.

---

## Current Progress (verified against the repo)

### Branch / deploy state — READ THIS FIRST (the can't-see-it facts)
- **Local HEAD = `origin/sprint/cross-pillar-cleanup` = `68e45e144`** (everything is pushed; tree clean).
- **`origin/dev` = `52e3f2d1b`** (unchanged). **The routing fix `37c822d1c` is NOT on `dev`.**
  Verified: `git merge-base --is-ancestor 37c822d1c origin/dev` → false.
- **⇒ The fix is NOT deployed to alpha.** `https://alpha.elohim.host/auth/portal` is still **404**
  until the operator merges sprint→dev. The branch is **20 commits ahead of `origin/dev`**.

### The routing shakeout shift — DONE (numeric target met + locally verified)
Sprint result: `.claude/shifts/2026-05-31T03-16-doorway-routing-projection-shakeout.sprint-result.md`
(journal alongside it; both gitignored).

Commits on the branch (newest→oldest, all pushed):
```
68e45e144 docs(memory): sprint/* not orchestrator-indexed + /auth/portal routing-shadow fix
6b69bf98a docs(framework-cleanup): R1 warm-cache fast-path ready-to-land note (operator-attended)
37c822d1c fix(doorway): un-shadow /auth/portal from the EPR router + pool the EPR proxy client  ← THE FIX
c42c90787 test(doorway): freeze routing-shakeout oracle (shakeout_* — 15 tests, 4/15 red baseline)
b5658340a chore(session): checkpoint handoff + memories + plan edits before routing shakeout shift
501e96dcd … 14 prior "browser-feedback tooling (L1+L2)" commits (pnpm look + visual gate) …
```

**What `37c822d1c` changed** (all in `doorway/doorway-service/src/server/http.rs`):
1. **`/auth/portal` 404 (P1) — FIXED.** The `/auth` dispatch guard was a catch-all
   `path.starts_with("/auth")` → 404 *"Auth endpoint not found"* **before** the EPR router ran,
   shadowing the seeded `imagodei-portal` projection. `is_service_path` also blanket-listed `"/auth"`.
   Fix: a single `is_auth_owned_path()` predicate (exact-match the **20** real `handle_auth_request`
   arms, query-stripped) now gates **both** the dispatch guard (`~line 1494`) **and** `is_service_path`
   (`~line 1096`). Unowned `/auth/*` now falls through to the EPR router.
2. **`derive_app_subpath()` extracted** from `dispatch_to_projected_epr` (behavior-identical; pinned
   under test) — the reusable projection-`url_path`→storage-sub-path derivation.
3. **R20 perf — DONE.** `dispatch_to_projected_epr` now uses the pooled `state.ssr_http_client`
   instead of building a throwaway `reqwest::Client` per request.

**Verification (re-run by Opus, not trusted from the subagent):** frozen oracle `shakeout_` **15/15**;
full doorway lib **524/524**; `clippy -D warnings` clean; `fmt --check` clean; two consecutive clean
local measures. The oracle is `#[cfg(test)] mod shakeout_tests` in `http.rs` — 15 pure-function tests.

**Visual baseline captured** (gate was ON): `genesis/a2o/reports/look/{auth-portal-before,
lamad-before,landing-before}/` — `/auth/portal` `capture.json` confirms the 404. The **after-render**
needs the fix deployed (= dev-merge), so the visual gate's `validatedRegressed==0` leg could not close
overnight — it's teed up for the operator.

### Routing/loading bugs (the backlog) — current status
| Route | Status |
|---|---|
| `alpha/auth/portal` 404 | **code-fixed** (`37c822d1c`); awaits dev-merge → deploy → render-check. **Caveat R21:** the imagodei-portal *bundle* may not be staged (`Jenkinsfile:1109-1112` stages only landing+lamad-spa) — if so, routing is correct and the remaining gap is build+stage. |
| `doorway-alpha/dashboard` 404 | NOT a doorway routing bug — *"File not found in app:"* is emitted by **elohim-storage** (`elohim/elohim-storage/src/http.rs:4567`); the doorway-app bundle isn't staged/projected (R21-class). |
| `elohim.host` (apex) 302 / `/lamad` 404 | operator-gated — apex EPR router empty (adam cells `CellDisabled`, R10 / O1 / O2). |
| `alpha/db/content/manifesto` 403 | needs `RESET_STORAGE=true` genesis reseed (insert-or-skip keeps stale `community` grade). Code already merged on dev. |

---

## What Worked

- **TDD-oracle-as-judge for the shift.** Froze 15 `shakeout_*` pure-function tests in `http.rs` FIRST
  (4/15 red baseline), then had a subagent implement to green. Self-contained measure
  (`cargo test --lib shakeout_`) runs natively in Che — no Holochain stack needed. doorway builds with
  `RUSTFLAGS=""` + `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/doorway__doorway-service/dev`.
- **Subagent dev sprint + independent Opus verification.** rust-architect (Sonnet) implemented; Opus
  re-ran every gate (don't trust the report). Caught two false alarms that weren't real failures
  (see below).
- **`is_auth_owned_path` as the single source of truth** for "auth owns this path" — used by both the
  dispatch guard and `is_service_path` — is the clean fix that respects doorway's "no per-domain proxy"
  discipline (unknown `/auth/*` falls through to the dynamic EPR router).
- **`pnpm look` for the visual baseline** (`cd genesis/a2o && pnpm look <url> --out <slug>`). `pnpm a2o:setup`
  (Chromium) already done this session. `capture.json` carries the console 404 — fast confirmation.
- **Reading the actual Jenkinsfile deploy gate before pushing** (`elohim/holochain/Jenkinsfile:1650`):
  proved a `sprint/*` push does NOT deploy to alpha (gate needs `dev`|`feat-.+`|`claude/.+` or
  `FORCE_*`) — so pushing for CI is safe.
- **`HUSKY=0 git push` foreground** — the pre-push gate would fail on pre-existing errors; CLAUDE.md
  sanctions the bypass, CI is the real gate.

## What Didn't Work (don't repeat)

- **Expecting a CI fresh-trigger from a `sprint/*` push.** `elohim-orchestrator` indexes only
  `{PR-*, dev}` — **NOT `sprint/*`** (config is in Jenkins multibranch UI XML, not the repo;
  ci-investigator high-confidence). The push delivers the webhook, but the orchestrator never runs;
  `elohim-edge`/`elohim-genesis` create build #1 then self-skip (*"PIPELINE SKIPPED — USE
  ORCHESTRATOR"*, `NOT_BUILT`). **`[build:edge]`/`[build:*]` tags are inert** on `sprint/*` (the
  orchestrator parses them and never sees the branch). CI for this work runs on the **dev-merge** (or a
  manual `UserIdCause` build). Saved to memory `project_sprint_branch_not_orchestrator_indexed`.
- **A persisted `cd` from a previous bash call** (`cd genesis/a2o` for `pnpm look`) made later
  repo-relative paths resolve from the wrong CWD (empty `git diff`/measure — looked like lost work but
  wasn't). Use absolute paths or `cd /projects/elohim &&`.
- **sccache transient null-byte corruption** from a subagent's parallel cargo runs zeroed a measure
  mid-shift; it self-healed. If the measure returns empty, suspect sccache and re-run (or
  `RUSTC_WRAPPER=""`).
- **Hygiene:** an env-presence check using `${JENKINS_TOKEN:-…}` expanded the token into terminal
  output (not committed / not sent externally). Use `${VAR:+set}` only — never `${VAR:-…}` — to probe a
  secret's presence.
- **Pushing a `claude/*` branch to force CI** — tempting (it DOES build+deploy), but the edge deploy
  restarts the *whole* alpha edge node (conductor+storage+doorway) — heavy cluster-domain action,
  operator territory. Don't.

---

## Next Steps (ordered)

### 1. Operator: merge `sprint/cross-pillar-cleanup` → `dev` (the gate for everything below)
This is the only path to CI + alpha deploy for the fix (sprint/* is not orchestrator-indexed). The
branch is 20 commits ahead of dev (14 browser-feedback tooling + the routing fix + docs/memory). The
merge runs `elohim-orchestrator/dev` → dispatches `elohim-edge` (expect doorway `shakeout_` 15 + lib
~524 green in the Docker `check` target) → **deploys to alpha**. *(Agent did not merge — sprint→dev is
operator-driven.)* Use the `git commit-tree` plumbing merge if the stale `.git/index.stash.*.lock`
reappears (do NOT touch the real operator stashes).

### 2. Render-validate `/auth/portal` on alpha (closes the visual gate)
`cd genesis/a2o && pnpm look https://alpha.elohim.host/auth/portal --out auth-portal-after`
→ compare to `reports/look/auth-portal-before/` (baseline 404). Expect the imagodei-portal projection.
**If it now reaches the EPR router but storage 404s the bundle → that's R21** (bundle not staged), the
routing fix is correct, and the remaining work is the build+stage step — not this code.

### 3. Land R1 — the warm-cache fast path (operator-attended, <30 min)
Ready-to-land note: `genesis/docs/architecture/framework-cleanup/2026-05-31-R1-warm-cache-fast-path-implementation-note.md`.
~40 LOC (`serve_app_file` extraction so `/` + `/lamad` hit the doorway MongoDB cache). **Three
must-keep invariants:** (a) `cache_enabled` admin-bypass check, (b) `x-epr-router: dispatched`
re-injection, (c) **slug-index parity** — validate on alpha via `/admin/cache/stats` cold/warm
(`X-Cache: MISS`→`HIT`). Deliberately not auto-landed (the slug-parity assumption needs one eyeball).
Then **R7** (`warm_stream` → `load_slug_index()`) and **R9** (EprRouter boot race) — both
`doorway/doorway-service/src/**`, code-fixable next shift.

### 4. Manifesto 403 (one operator action)
Genesis with **`RESET_STORAGE=true`** (parameterized build), then
`curl https://alpha.elohim.host/db/content/manifesto` → expect 200; `reach-commons.feature` `@regression` green.

### 5. Operator-owned follow-ups (flagged, not agent-fixable)
- **Apex (`elohim.host`)** — adam `enable_app` on `lamad`+`imagodei` cells + conductor-data PVC durability (R10/O1/O2).
- **`/dashboard`** — stage/project the doorway-app operator bundle (R21-class).
- **L1 image-bake** — fold Chromium into the che image so `pnpm a2o:setup` isn't per-workspace.
- **pnpm v10** — migrate the `pnpm` field in `package.json` to `pnpm-workspace.yaml` (deprecation warns).

---

## Key references
- Branch: `sprint/cross-pillar-cleanup` (HEAD `68e45e144` = origin); `origin/dev` `52e3f2d1b` (fix NOT merged/deployed).
- The fix: `37c822d1c` in `doorway/doorway-service/src/server/http.rs` (`is_auth_owned_path`, `derive_app_subpath`, pooled `ssr_http_client`); oracle in `#[cfg(test)] mod shakeout_tests`.
- Measure: `RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/doorway__doorway-service/dev cargo test --manifest-path doorway/doorway-service/Cargo.toml --lib shakeout_` → 15.
- Sprint result + journal: `.claude/shifts/2026-05-31T03-16-doorway-routing-projection-shakeout.{sprint-result,journal}.md` (gitignored).
- Reliability backlog: `genesis/docs/architecture/framework-cleanup/2026-05-30-reliability-backlog.md` (R1–R22).
- Deploy gate proof: `elohim/holochain/Jenkinsfile:1650` (alpha deploy: `dev`|`feat-`|`claude/`|`FORCE_*`).
- Memory: `project_sprint_branch_not_orchestrator_indexed` (why no CI on sprint/*).
