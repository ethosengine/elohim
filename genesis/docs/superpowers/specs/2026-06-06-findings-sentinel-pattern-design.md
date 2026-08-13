---
title: "Findings Sentinel Pattern — flag → agent → canon → stasis for in-flight and remote findings"
id: findings-sentinel-pattern-design
status: Draft
class: process-meta
process_subdomain: hooks
topic: [sentinel, deprecation, security, vulnerability, ci, jenkins, harvest, flake, fingerprint, ledger, backlog, stasis, triage, museum]
cites:
  - ci-orchestrator-recurring-anti-patterns-museum | the frequency-ranked CI lessons home — instantiation B graduates recurring-trap lessons INTO it (chronicle-equivalent for the ci class), and ci-failure-triage checks its trap list before declaring novel root causes | sha256:0e325f2f174689ae | path: genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - unified-memory-loop-design | the stasis-loop discipline this pattern instantiates per finding class — one scoreboard, measure→dispatch→re-measure | sha256:99100efd20d10129 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md
  - verification-result-index-design | the system→state-store precedent for closure-by-observation — instantiation B confirms fixes by fingerprint disappearance, the same evidence-over-claim posture | sha256:8d6b292dafc4a44e | path: genesis/docs/superpowers/specs/2026-06-01-verification-result-index-design.md
informed-by: [genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md]
---

# Findings Sentinel Pattern

Design sessions: 2026-06-06 (operator-directed). One reusable architecture for
"react to X automatically without creating a dump", instantiated twice the same
day: (A) deprecation/security warnings in Bash tool output, (B) CI pipeline
results from Jenkins. Future dumps with this shape (lint-debt classes, a2o
drift, advisory feeds) COMPOSE from this spec rather than re-deriving.

## 1. The pattern (four layers + close semantics)

| Layer | Role | Anti-dump property |
|---|---|---|
| **1. Deterministic flag** | Cheap capture fingerprints findings into a JSONL ledger (`.claude/data/*.jsonl`). The ledger is the **existing-positives check surface**: presence (live statuses) suppresses dispatch; absence fires the dev. | Dedupe by normalized fingerprint; count-bearing summary lines digit-normalized (one live concern across count churn); identifier lines exact (distinct advisories stay distinct). |
| **2. Background Opus dispatch** | NEW fingerprint → directive to launch the class's triage agent (`run_in_background`) — scope → canonicalize → fix-if-bounded → else document blocker. Current task never derails. | Scale posture is goal-shaped ("largest genuine step toward stasis this run supports") with named anti-patterns (triage-as-terminal, fix-spree, mega-entry, re-scan, partial-marked-done) — never how-to. |
| **3. Canonical backlog** | One timeline-CONVENTIONS-conformant entry per CONCERN in `genesis/data/timeline/backlog/<class>-<slug>.md` (fingerprints N:1). `status:` = unified delivery gradient (`backlog`/`wip` only — no tombstones); domain axis (`deprecation_status:`/`ci_status:`) carries live state; "Current decision" is the deterministic citation line. Entity docs stay envelope-free plain-path cite targets. | Shares the managed-surface registry, delivery-status projection, /converge ranking — the same infrastructure as every other backlog item. Re-encounter of a known fingerprint cites the decision once per session; blocked-and-canonicalized NEVER re-fires agents. |
| **4. Stasis sweep** | Ceremony-pattern skill (measure → dispatch → re-measure; sibling of /memory-stasis-loop): drains open items, re-checks blocked items whose blockers may have cleared, repairs ledger↔backlog incoherence. Manual, /loop, or routine. | Stasis := ledger empty or blocked-with-valid-blocker only. |

**Close semantics (full memory decomposition):** FIXED-no-work-left items are
DELETED from ledger AND backlog — the verifying commit is the durable record;
genuinely meaningful lessons (rare) graduate to the class's history home
BEFORE deletion. Everything in a backlog has a live trajectory or it's not
there. Reintroduction reads as NEW → re-fires = regression handling for free.

## 2. Instantiation A — deprecation/security sentinel (landed 2026-06-06)

As-built record (commits `bf6e38b49`, `60ce2006b`, `a6f1d74d7`):
- Flag: `.claude/hooks/deprecation-sentinel.py` (PostToolUse:Bash) →
  `.claude/data/deprecations.jsonl`. Classes `deprecation` | `security`
  (install/audit summaries, GitHub push banners, CVE/GHSA/RUSTSEC ids);
  guard-token skip for commands that themselves mention the signal class.
- Dispatch: `.claude/agents/deprecation-triage.md` (Opus).
- Canon: `backlog/{deprecation,security}-<slug>.md`; history home =
  `timeline/chronicle/`.
- Sweep: `.claude/skills/deprecation-stasis/SKILL.md`.
- First full lifecycle proven same-day: Vitest 4 `test.poolOptions` —
  capture → dispatch → fix (`3ac89f433`) → decomposition → stasis.

## 3. Instantiation B — CI findings (this design)

CI results differ from Bash-output findings in three ways: they arrive
REMOTELY (need a fetch trigger, not a passive hook), flakes need OCCURRENCE
tracking (the same fingerprint recurring across builds IS the signal), and
closure is OBSERVED (a fix is confirmed by the fingerprint disappearing from
subsequent builds, not by a local run alone).

### 3.1 Deterministic harvester — `.claude/scripts/ci-harvest.py`

Cursor-based, idempotent, anonymous Jenkins JSON API (per
pipeline-diagnostics: Overall.Read on jenkins.ethosengine.com; authenticated
curl via devspace `JENKINS_USERNAME`/`JENKINS_TOKEN` only if ever needed).

- **Scope**: the elohim multibranch pipelines × `dev`
  (`elohim`, `elohim-edge`, `elohim-genesis`, `elohim-holochain`,
  `elohim-orchestrator`, `elohim-sophia`, `elohim-steward`,
  `elohim-storybook`) — registry constant in the script, trivially editable.
- **Cursor**: `.claude/data/ci-cursor.json` `{job: last_harvested_build}`.
  First run initializes shallow (last ~5 builds), never replays history.
- **Per unharvested completed build** with result FAILURE/UNSTABLE:
  - failed test cases via `…/<N>/testReport/api/json` (className+name+
    errorDetails);
  - non-test failures via console-tail classification against
    `.claude/data/failure-taxonomy.json` category regexes (the existing
    classifier vocabulary — reused, not re-derived).
- **Fingerprint**: sha256(job + category + identifier) — identifier is the
  test `className.name` or the normalized error line (line/col refs, hashes,
  timestamps, build numbers stripped) so the same failure across builds is
  ONE finding.
- **Ledger**: `.claude/data/ci-findings.jsonl` —
  `{ts, fp, class: "ci-failure", category, job, line, status, seen,
  first_build, last_build, backlog?}`. The harvester is the single writer of
  `seen`/`last_build` (occurrence tracking → flake evidence); triage owns
  `status`/`backlog`; green builds advance `job_green` watermarks in the
  cursor file so the sweep can confirm disappearance.
- **Urgent split** (operator decision 2026-06-06): a job whose latest
  completed build is red where the previously harvested one was green is a
  FRESH REGRESSION → injected as urgent context (now-work for the session /
  operator), NOT a triage dispatch. Recurring signatures and stale reds are
  backlog-work → ledger + dispatch directive (same goal language as
  instantiation A).
- **Triggers** (operator decision): SessionStart async hook (catch-up since
  cursor — every session starts with CI findings already captured) + a
  post-push `--wait <job>` mode (bounded poll until the dispatched build
  completes, then harvest — armed via `run_in_background` Bash right after a
  CI-triggering push). Local cron / remote routine deliberately deferred.

### 3.2 Triage owner — `.claude/agents/ci-failure-triage.md` (Opus)

Sibling of deprecation-triage; same stores discipline, same scale posture,
same decomposition-at-close, plus CI-domain specifics:
- Dispatches `ci-observer` (Haiku scan) / `ci-investigator` (Sonnet deep
  factual) as READ-ONLY sub-analysts via Task — their contracts unchanged.
- Knows the museum's recurring traps (NOT_BUILT/superseded reads as
  0-failures, `#[ignore]` is a CI no-op, host-green ≠ CI-green, webhook
  double-fire, baseline-rollback over-build) — checks the trap list BEFORE
  declaring a novel root cause.
- Cannot trigger builds (anonymous MCP; `[build:*]` tags are integrator
  push territory) → closure is two-phase: fix lands + local verification →
  ledger `status: triaged` (fix-landed-awaiting-CI); the DETERMINISTIC layer
  confirms — fingerprint absent for ≥3 subsequent harvested builds of that
  job → the sweep decomposes (deletes ledger line + backlog entry).
- History home for graduated lessons: the anti-patterns MUSEUM record (its
  frequency-ranked list is exactly this pattern's chronicle-equivalent) —
  extend the museum, never fork a second lessons doc.

### 3.3 Stasis home — the agentic-developer loop's rails (operator revision, 2026-06-06)

CI stasis does NOT live in `deprecation-stasis` (that sweep stays
instantiation-A-only). Two-part replacement:

1. **Closure is fully deterministic — it lives in the harvester.**
   Confirmation-by-disappearance (`green_streak.<job> ≥ 3` with no recurrence
   past `triaged_at_build`) deletes the ledger line; recurrence
   (`last_build > triaged_at_build`) reopens to `open`. Backlog decompose
   rides the `decompose_on_confirm` stamp the triage agent set at fix time
   (judgment made once, executed deterministically); unstamped entries are
   reported for graduate-then-decompose. No agent or ceremony owns CI
   closure.
2. **Draining lives in the agentic-developer loop as floor/ceiling rails**
   (SKILL.md §"CI findings rails"): the harvester's `recent.<job>` rolling
   windows give the **floor** — pass/unstable/fail ratios the shift must hold
   or raise (never leave touched jobs below where it found them; open
   findings on touched jobs are in-scope candidates, prioritized by `seen`);
   **ceiling** — brainstorming confidence: a finding whose resolution is
   design-shaped (low confidence after verify, architecture/substrate/
   cross-cutting) is above the ceiling — stop iterating, capture as
   needs-brainstorm (`ci_status: blocked` + the design question), route to
   /brainstorm. Stasis = riding between the rails with the ledger draining.

## 4. Testing

- Harvester: live read-only runs against jenkins.ethosengine.com (cursor
  init shallow; idempotent re-run → zero new entries; urgent-split fires on
  the currently-red elohim-genesis/dev); offline → silent exit 0.
- Fingerprint normalization: same test failure across two builds → one
  ledger entry with seen=2; distinct tests → distinct entries.
- SessionStart wiring: jq schema validation; async; generous timeout;
  failure-silent (a Jenkins outage must never break session start).

## 5. Captured follow-ups

1. **Remote routine** (/schedule) once the household cadence justifies it.
2. **Skill rename** deprecation-stasis → findings-stasis (with redirects).
3. **a2o sprint-report harvest** — the genesis pipeline's sprint-report.md
   artifact carries scenario-level findings; fold into the harvester as a
   third source once the test-level capture proves its keep.
4. **Third instantiation candidates**: lint-debt classes, a2o drift — both
   compose from §1 when they arrive.
