# HANDOFF — two live workstreams

1. **Context-coverage / memory-stasis system** (this session) — committed `3e493694d`, on `origin/sprint` only, **dev-merge pending**. The open design question: **converge `/memory-ceremony` vs the new memory-stasis workflow vs `/memory-kit`.**
2. **Routing/projection shakeout + deploy** (prior session, STILL BLOCKED on Harbor EIO) — preserved below in Part B.

_Last updated: 2026-06-01 · Author: Claude Opus · Branch: `sprint/cross-pillar-cleanup`_

---

# PART A — Context-coverage / memory-stasis system (NEW, the active handoff)

## Goal

Stop the spec/plan/memory pile from being a write-only dumping ground. Build a **deterministic "context
coverage" discipline** — code-coverage, applied to the memory/doc graph — so every artifact is instantly
auditable for *position + state*, the surface trends to a tunable **stasis** target, and `/brainstorm`+`/plan`
compose from canon instead of re-speccing. Then **converge** the three overlapping memory cadences into one.

## Current Progress (verified against the repo)

### Branch / commit state — READ FIRST (the can't-see-it facts)
- Checkpoint **`3e493694d`** `feat(memory-kit): context-coverage system … [skip ci]` — **48 files**, on
  **`origin/sprint/cross-pillar-cleanup` ONLY**. Verified: `git branch -r --contains 3e493694d` → only
  `origin/sprint/cross-pillar-cleanup`.
- **DEV-MERGE PENDING.** `dev` is checked out + **dirty** (submodule churn + `Cargo.lock`) + behind
  `origin/dev` in the **`/projects/elohim-worktrees/qahal-m1`** worktree — could not merge from the main
  worktree. Operator is running the merge there (the 3 commands in Next-Steps #1). Until then the work is
  NOT on `dev`.
- Regenerable coverage-outputs are **gitignored** (`.claude/memory-kit/.gitignore`: `state-ledger.json`,
  `spec-coherence-index.json`, `gap-items/`). The **config** (`context-coverage.yaml`) and **ratchet floor**
  (`context-coverage-baseline.json`) ARE tracked.
- `[skip ci]` on the commit + `sprint/*` not orchestrator-indexed = no CI fired.

### What landed (all verified present + runnable)
The system is the code-coverage toolchain, pointed at context:

| Role | Artifact |
|---|---|
| **instrument + report** | `.claude/scripts/memory-kit/placement-audit.py` — `--ledger` (per-file budget: position+state+next-action), `--coverage` (un-captured/un-reviewed backlog), `--stasis` (composite score vs benchmark), `--headline` (the SessionStart line), `--focus` (testable surface from cluster-state), `--json` |
| **config (tunable, with methodology)** | `.claude/memory-kit/context-coverage.yaml` — `margin`/budgets/dimension-weights/exclusions, **each with a `why:` methodology block**. The tooling reads it; tune here, not in code. |
| **the gate (ratchet)** | `.claude/scripts/memory-kit/context-ratchet.py` — must-not-decrease + ratchets the baseline up. Baseline **0.468**. |
| **decompose → gap-items** | `decompose.py <doc>` → bounded, cited gap-items (OPEN=implement, CLAIMED=verify). Checkboxes lie → a checked box is CLAIMED, never done. |
| **prior-art index** | `spec-coherence-index.py [--query]` — token-overlap "have we spec'd this?" for the brainstorm canon-check. |
| **state-machine gates** | `state-machine-gen.py` → `genesis/docs/_state/{blockers,regression,unverified,needs-triage}/CLAUDE.md` pressure-dir gates (meant to be EMPTY). |
| **contract** | `genesis/docs/PLACEMENT.md` — homes (CANONICAL/HISTORY/ACTIVE), lifecycle, the 4 verification states, env-scope, feedback graph, the stasis contract. |
| **env reality** | `genesis/manifests/cluster-state.yaml` — declares node/cluster/registry availability; `--focus` scopes off it (BLOCKED-BY-ENV ≠ regression). |
| **history home** | `genesis/docs/content/elohim-protocol/history/` (sibling of `architecture/`) + `INDEX.md` + the worked record `2026-06-01-dht-is-a-notary-not-a-byte-store.md` (bidirectional links). |
| **wrappers** | `.claude/commands/{brainstorm,plan}.md` — pre (compose-from-canon + focus + budget) → skill → post (land auditable + decompose). |
| **the loop** | `.claude/workflows/memory-stasis-loop.js` — measures `--stasis`+`--coverage`, dispatches the equipped agent for the lowest dimension, re-measures, until `at_stasis && uncaptured==0`. Loop length ∝ real backlog. |
| **agents re-equipped** | `.claude/agents/{librarian,historian,cartographer,storyteller}.md` — stasis mandates ("drive your slice to stasis; how is your judgment"); **cartographer got read-only mempalace** (the gap it self-diagnosed). |
| **the link rule** | memory `project_link_is_path_plus_explainer` — a link = a **path + 1-2 sentence plain-text explainer**; bare paths don't count. |

### The STASIS model (the benchmark)
- **Composite stasis score = context coverage %**, benchmark **1.0**, ±**15%** band → **at stasis when ≥ 0.85**.
  Current: **`0.468 / 1.000`** (`placement-audit.py --stasis`; verified). Dimensions weighted per the manifest.
- 7 MEASURED dims (capture 66%, status 41%, well-formed 21%, **memory-cite 4.4% ← biggest lever**, CLAUDE.md
  right-sized 71%, history-bidirectional 67%, MEMORY.md-budget 100%) + 2 HARD gates (dumps=0, pressure-dirs=0).
- **3 UNMEASURED dims** (honest — can't claim full stasis until wired): **code→story/archetype/scenario/CLAUDE.md/doc
  traceability** (path+explainer), gap→test-infra tagging, CLAUDE.md recursive coverage.

## THE OPEN CONVERGENCE QUESTION (what the operator wants decided next)
There are now **three overlapping things tending the same surface with the same four agents**, no shared scoreboard:
1. **`/memory-ceremony`** — four-agent substrate-currency ceremony → **gospel-tier rewrites** (judgment-heavy, periodic).
2. **`/memory-kit` (hygiene-sweep)** — librarian-solo **deterministic** tools + hooks (byte budgets, dedup, drift, cites).
3. **NEW: context-coverage + `memory-stasis-loop`** — deterministic measurement (the stasis score) + manifest + ratchet + the loop that dispatches the same four agents to drain toward the score.

**Proposed convergence (for the next session to ratify):** make **context coverage the unifying scoreboard**.
The stasis score + dimensions are the common goal; `/memory-kit` raises the *deterministic* dimensions (status,
capture, budgets, dedup, links), `/memory-ceremony` raises the *judgment* dimensions (traceability quality,
gospel coherence); **`memory-stasis-loop` is the orchestrator** — measure `--stasis`, pick the lowest dimension,
dispatch the right modality (kit=deterministic drain, ceremony=judgment drain), re-measure, until in-band; the
ratchet enforces, the manifest tunes. i.e. ceremony + kit become **phases the loop invokes**, not competing cadences.
**Sub-questions to resolve:** does the loop replace or wrap the ad-hoc cadences? how do ceremony "gospel rewrites"
map to the traceability/coherence dimensions? is the ceremony's substrate-currency-audit subsumed by `--stasis`?

## What Worked
- **The code-coverage analogy as the unlock.** Reframing the whole thing as "context coverage" made the missing
  pieces obvious (the ratchet, the manifest, exclusions) and gave one number (the score) to drive.
- **The system self-tests + self-heals.** A hardening workflow (`wk4y260rq`) found **28 confirmed bugs** (11/11 tools
  green, convergence proven). The e2e brainstorm sim caught a real `decompose` bug. Adding the `.claude/memory-kit/`
  gate raised the score 0.464→0.468 and the **ratchet auto-captured the gain** — the discipline working on its author.
- **Born-auditable, +0 debt.** The `/brainstorm` e2e (`…/specs/2026-06-01-verification-result-index-design.md`) landed
  with status+cites → ACTIVE+linked, not a no-status orphan; and it's now prior-art for the next brainstorm (dedup at source).
- **Manifest-driven tuning, proven.** `margin: 0.15→0.40` moved the gate `≥0.85→≥0.60` with zero code change.
- **Deterministic-first, agents-for-residue.** Decomposing all 145 specs/plans was a cheap script sweep (captured 95,
  surfaced **3,238 OPEN + 183 CLAIMED** gaps — the plan-truthing externality); only 49 prose specs need an agent.

## What Didn't Work / known issues
- **dev-merge could not run from the main worktree** — `dev` is locked to the dirty `qahal-m1` worktree. Don't force it.
- **`decompose.py` REQ_HEADING bug (NOT fixed):** `\b(requirement|task|gate|component…)\b` misses the *plural* headings
  (`## Requirements`, `## Tasks`) — so prose specs with a Requirements section wrongly fall to "needs AGENT
  decomposition." Fix: `(?i)(requirement|acceptance|task|gate|component|deliverable|criteri|goal)s?\b`.
- **3 stasis dimensions still UNMEASURED** (see above) — full stasis can't be claimed until they're wired.
- The iroh "landed" claims were a lie: `ci-investigator` graded **1/12 gates verified** (tests never run in CI; soaks
  never ran; alpha cluster crashlooping). The iroh delivery-master is stamped with the truth; it stays HOT.

## Next Steps (ordered)
1. **Operator: finish the dev-merge** in the qahal-m1 worktree (PENDING):
   ```bash
   cd /projects/elohim-worktrees/qahal-m1
   git fetch origin && git merge origin/dev
   git merge sprint/cross-pillar-cleanup          # docs/tooling only — disjoint from your submodule churn
   HUSKY=0 git push origin dev                     # commit carries [skip ci]
   ```
2. **DECIDE the convergence** (the section above) — `/memory-ceremony` vs `memory-stasis-loop` vs `/memory-kit`. This is
   a design decision, not code; resolve before more building so the loop is the agreed orchestrator.
3. **Drive the score 0.468 → 0.85** — run the `memory-stasis-loop` workflow (or dispatch the librarian) at the biggest
   lever first: **memory-cite 4.4%** (link the ~236 unlinked `.claude/memory/*.md` to a system, or forget them). Re-run
   `context-ratchet.py` after to lock each gain.
4. **Wire the 3 unmeasured dims** — start with **code traceability** (path+explainer links, the original ask + the
   marquee dimension), then gap→test tagging, then CLAUDE.md recursive coverage.
5. **Build the verification-result index** — the spec is written:
   `genesis/docs/superpowers/specs/2026-06-01-verification-result-index-design.md` (5 OPEN gaps). It closes the loop's
   back half (CLAIMED→done auto-resolves via ci-investigator), so the score can reach DONE not just CLAIMED.
6. **Fix the `decompose.py` plural-heading bug** (one regex, above).

## Key references (Part A)
- Run it: `python3 .claude/scripts/memory-kit/placement-audit.py --stasis` (the score) · `--ledger` (the budget) · `--coverage` (the backlog).
- The contract: `genesis/docs/PLACEMENT.md`. The toolkit gate: `.claude/scripts/memory-kit/CLAUDE.md`. The data-dir gate: `.claude/memory-kit/CLAUDE.md`.
- Hardening workflow result (28 bug fixes): task `wk4y260rq` (transcript under the session's workflow dir).

---

# PART B — Routing/projection shakeout + deploy (PRIOR session — STILL BLOCKED, preserved)

**Status: routing fix `37c822d1c` is on `origin/dev` (merge `d439f6667`); deploy BLOCKED ~14h on Harbor registry
storage EIO; render-validation pending.** Unrelated to Part A but still open.

- **Deploy blocker (operator/cluster):** orchestrator builds #1118/#1119 ABORTED — `ErrImagePull` HTTP 500 on
  `harbor.ethosengine.com/ethosengine/ci-builder:latest`; reproduced as registry storage-driver **EIO** reading the
  manifest link (`{"DriverName":"filesystem","Op":"open",…,"Err":5}`). **Fix:** heal the Harbor registry PV, or
  rebuild+re-push `ci-builder` to write a fresh `latest` manifest; then deploy re-runs on the next dev push / `[build:edge]`
  / a Jenkins Replay of `elohim-orchestrator/dev`. `alpha.elohim.host/auth/portal` is still 404 (fix on dev, not deployed).
- **The fix `37c822d1c`** (`doorway/doorway-service/src/server/http.rs`): single `is_auth_owned_path()` predicate gates
  both the `/auth` dispatch guard and `is_service_path` (un-shadows the seeded `imagodei-portal`); `derive_app_subpath`
  extracted; pooled `ssr_http_client` (R20). Verified: `shakeout_` 15/15, doorway lib 524/524, clippy/fmt clean.
- **Open follow-ups:** ① render-validate `/auth/portal` on alpha once deployed (`cd genesis/a2o && pnpm look <url>` vs
  `reports/look/auth-portal-before/`); if it reaches the EPR router but 404s the bundle → **R21** (bundle not staged,
  `Jenkinsfile:1109-1112`). ② Land **R1** warm-cache fast path (note at
  `genesis/docs/architecture/framework-cleanup/2026-05-31-R1-warm-cache-fast-path-implementation-note.md`; 3 invariants).
  ③ Manifesto 403 → genesis reseed `RESET_STORAGE=true`. ④ Apex/`/dashboard`/L1-image-bake — operator-owned.
- Memory: `project_sprint_branch_not_orchestrator_indexed` (no CI on `sprint/*`).
