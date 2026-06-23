# Elevate Arm — Runtime Harvest Poller — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the self-healing loop's final arm — ELEVATE — by adding an EXTERNAL Python poller that reads each alpha node's admin read-endpoints (`GET /admin/self-healing` primary, `GET /admin/render-stats` + `GET /health` secondary), evaluates pure exhaustion predicates over a persisted per-node sample window, deterministically files a `self-heal-exhaustion` finding to `.claude/data/runtime-findings.jsonl` on a NEW exhaustion fingerprint, and dispatches a background `runtime-triage` agent — riding the EXISTING ledger+sentinel pattern (instantiation D of the findings-sentinel design; `ci-harvest.py` is the WRITE-side reference impl).

**Architecture:** The elevate arm is an **EXTERNAL poller**, never runtime code. **THE NO-RUNTIME-WRITE RULE (non-negotiable): runtime Rust code MUST NEVER write `.claude/data`** — this plan adds ZERO Rust. The Rust services only EMIT JSON read-models (`/admin/self-healing`, `/admin/render-stats`) and leave `warn!` seams; this Python poller READS those endpoints and WRITES the ledger, exactly as `ci-harvest.py` reads the Jenkins JSON API and writes `ci-findings.jsonl`. State is split into a pure core (`_lib/runtime_harvest.py`: `evaluate(window) -> list[Finding]` over a persisted sample buffer, and `reconcile(entries, active, poll_index) -> (new, bumped, closed)`) that is unit-tested with synthetic fixtures and NO network/disk, plus a thin I/O shell (`.claude/scripts/runtime-harvest.py`) that does the polling, the flock'd atomic ledger/cursor writes, and the SessionStart-hook dispatch directive.

**Tech Stack:** Python 3 (3.12.9, stdlib-only — `urllib.request`, `hashlib`, `json`, `fcntl`, `argparse`, `re`, `os`, `time`, `datetime`; NO third-party, `pytest` is NOT installed); the EXISTING `.claude` ledger+sentinel tooling (mirror `.claude/scripts/ci-harvest.py` for the write side, `.claude/agents/deprecation-triage.md` for the agent header, `.claude/settings.json` SessionStart-hook registration); the bespoke self-running test harness (`_lib/__tests__/<x>_test.py` with the walk-up bootstrap + `check(label, cond)` helper — NOT pytest).

---

## Decisions

### D1 — Dependency ordering: C (read model) ideally BEFORE D (this poller), but D is SELF-CONTAINED

The primary input is `GET /admin/self-healing` from the **stability-surface read-model plan** (`genesis/docs/superpowers/plans/2026-06-13-stability-surface-read-model-plan.md`). Verified status (E2/E3/E4):

| Endpoint | Status | Verified at |
|---|---|---|
| `GET /admin/self-healing` | **PENDING** (sibling plan C builds it; grep of `doorway-service/src` for `self-healing` route = ZERO hits) | E2, E3, E4 |
| `GET /admin/render-stats` | **LANDED** | `doorway/doorway-service/src/server/http.rs:2337-2338`; handler `routes/admin.rs:986-988`; body `RenderTraceSnapshot` (`elohim/elohim-render/src/stats.rs:39-52`), no auth |
| `GET /health` | **LANDED** | doorway liveness probe |

**The poller is SELF-CONTAINED: it polls whatever endpoints exist NOW and lights up the richer predicates with ZERO code change when C lands.** `evaluate()` is **field-presence-tolerant**: an absent field contributes NO signal (it is never read as exhaustion). Today (render-stats + health only) it fires ONLY the `render-degenerate` predicate. When `/admin/self-healing` lands, its `upstreams[].circuit`, `admission.shedTotal`, `projector.lagSeconds`/`caughtUp` fields populate the sample, and the circuit/shed/projector predicates activate automatically. Ideal landing order is **C before D** so all predicates are live on day one; but D ships value (render saturation detection) before C. This plan adds ZERO Rust — it does NOT build `/admin/self-healing` (that is plan C's scope; building it here would violate the no-Rust constraint and duplicate C).

### D2 — Endpoint signals over logs (no Loki)

The poller reads endpoint JSON, NOT Loki. Rationale (verified E2/E3): every exhaustion signal has an endpoint home — `degenerateRate` + `stalled`/`timedOut` deltas (`/admin/render-stats`, landed), and `upstreams[].circuit`/`errorStreak`/`skipped`, `admission.shedTotal`/`available`, `projector.lagSeconds`/`caughtUp` (`/admin/self-healing`, pending). The `warn!` seams (`target: "ssr_busy"`, `WarmupSelfHealEvent`, `target: "admission_busy"`/`"upstream_shed"`) are deliberately **log-only duplicates** of the endpoint scalars. Reading endpoints dodges the unresolved "does `ssr_busy` reach Loki?" question and the alpha Loki 502-storm untrustworthiness (adam pod 26GB/day spam — memory `project_alpha_substrate_probe_rails`). Endpoints are the source of truth; logs are not consulted.

### D3 — Reachability realism: target node admin endpoints directly; unreachable ≠ finding

`/api/v1/*` storage reads proxy to **matthew ONLY** (EprRouter single-target — memory `project_alpha_substrate_probe_rails`), and `/p2p/status` is NOT proxied (SPA fallthrough). But `/admin/self-healing`, `/admin/render-stats`, `/health` are **doorway-resident Cat-C node-local** — they report the POLLED pod's own state, which is exactly the correct scope for a per-node elevate poller. The poller targets each node's base URL directly (`NODES` list; alpha base `https://doorway-alpha.elohim.host`). **If a node is unreachable, that is NOT a finding and NOT a finding-storm** — the poll degrades quietly: a failed/timed-out/404 fetch yields an EMPTY sample for that endpoint (no fields → no predicate fires), and a fully-unreachable node skips its window append entirely. Never flap. (Mirrors `ci-harvest.py`'s fail-safe: a Jenkins outage emits nothing, never a finding.)

### D4 — Deterministic + idempotent: pure `evaluate` + pure `reconcile`, closure by DISAPPEARANCE

Same input → same fingerprint; re-running the poller never double-files. The multi-poll predicates ("circuit Open across ≥N consecutive polls", "lag persistently rising over a window") REQUIRE a persisted per-node sample window — meaningless in one invocation. So: each invocation polls ONCE, appends ONE sample to a ring buffer in `runtime-cursor.json`, then the **pure** `evaluate(window) -> list[Finding]` runs over the buffer (the I/O shell owns append + truncate). `reconcile(entries, active_findings, poll_index) -> (new, bumped, closed)` is ALSO pure — that is how idempotency and closure are unit-tested deterministically without disk, satisfying the hard constraint directly. Append is flock-guarded; ledger is written BEFORE the cursor (crash-safe, idempotent re-harvest via fp-dedupe — mirror `ci-harvest.py:273-277,279-282`).

### D5 — Closure by DELETION, not `status:"closed"` (DEVIATION from prompt — the "ride the pattern" constraint wins)

The prompt's scope item (3) says "fp absent for ≥N polls → status closed." The VERIFIED pattern (ci-harvest, deprecation-sentinel) closes by **DELETION of the ledger line** — "fixed items are deleted, never parked" — so a reappearance re-fires as NEW (free regression detection). The hard constraint "ride the EXISTING pattern, do not reinvent" outranks the literal "status closed" wording. **Decision: closure = DELETE the ledger line when the fp's exhaustion is absent for ≥ `CLOSE_STREAK` consecutive polls** (a per-fp `clean_poll_streak` in the cursor, the analog of ci-harvest's per-job `green_streak`). Second divergence from ci-harvest: ci-harvest gates closure on `status == "triaged"` (an agent must have judged it); a runtime exhaustion **self-resolves without triage** (the node recovered on its own), so disappearance-closure applies to **ANY** status here. Both deviations are named in the summary.

### D6 — Fingerprint = `fp(node + class + provenance)`, churn-normalized

`fp = sha256(f"{node}|{class}|{provenance}").hexdigest()[:12]` — 12-hex, mirroring `ci-harvest.py:109-113`. `node` and `class` lowercased (developer vocabulary); `provenance` is the exhaustion locus (e.g. `circuit:storage-upstream`, `render-degenerate`, `projector:lamad`, `admission-shed`), passed through the digit/IP/timestamp normalization of `ci-harvest.py:96-106` so poll-count churn ("open for 7 polls") never forks the fp into a new finding. Same exhaustion on the same node = ONE ledger line. This dedupes recurrence and is the structural suppression key.

### D7 — Suppression is STRUCTURAL (presence suppresses), not a status filter

Dispatch fires ONLY for findings whose fp is ABSENT from the ledger (`by_fp.get(fp) is None`). ANY ledger presence — `open`/`triaged`/`blocked` — is a known entry: it bumps `seen`/`last_poll`, never re-dispatches (verified `ci-harvest.py:289-312`, `deprecation-sentinel.py:317-333`: "Any ledger presence is a LIVE positive — cite, don't re-fire"). A blocked finding suppresses by simply existing. There is NO separate "blocked → skip" branch. The triage agent's job is to set `status`/`backlog`; presence-thereafter = silence.

### D8 — Wiring: SessionStart `--hook` + a `/loop` (or cron) driver — instantiation B shape, NOT PostToolUse

E4's own conclusion: the poller reads HTTP endpoints (not Bash tool output), so it is **structurally instantiation B (`ci-harvest.py`)**, not the PostToolUse(Bash) deprecation-sentinel. "Match E4" resolves to E4's CONCLUSION, not the literal `PostToolUse(Bash)` matcher. Registration:
- **SessionStart `--hook`** (mirror `.claude/settings.json:26` ci-harvest entry, `"async": true`, fail-safe exit 0) — surfaces accumulated findings + dispatch directives at session start, silent on no-signal.
- **A `/loop` or `/schedule` poll driver** for the actual cadence — the consecutive-poll predicates need REGULAR ticks (every poll appends one sample), which SessionStart alone does NOT provide. The plan registers the SessionStart hook (Task 8) and DOCUMENTS the `/loop` invocation (Task 8); the cron/`/schedule` routine is an operator action, not a code change.

### D9 — Backlog convention named, NOT built here (triage owns canonicalization)

When `runtime-triage` canonicalizes a finding it writes `genesis/data/timeline/backlog/self-heal-<slug>.md` (the `genesis/data/timeline/backlog/<class>-<slug>.md` convention, `<class>` = the `self-heal` domain prefix; timeline-CONVENTIONS-conformant, plain-path cites — mirror `deprecation-triage`'s schema). This plan NAMES the convention and ships the agent definition; it does NOT auto-canonicalize (the agent owns it, out of scope per the prompt).

### D10 — Out of scope (named follow-on plans)

- **The runtime CODE emitting richer signals** — the Rust side already leaves `warn!` seams + the `/admin/self-healing` fields; this plan only CONSUMES them. Building `/admin/self-healing` is plan C (stability-surface read-model). Adding new emit seams is a separate Rust plan.
- **Auto-FIXING the elevated finding** — the `runtime-triage` agent drives the fix (scope → canonicalize → fix|block); out of this plan.
- **The REA actuation / recover loop** (`tune_knob`, `quarantine_peer`, `delegates-compute`) — the RECOVER step; this plan is the ELEVATE step ONLY.
- **Arc-shrink** — separate thread.
- **Auto-canonicalizing every finding into a backlog doc** — the convention is named (D9) but the triage agent owns canonicalization.

## Canonical names

| Name | Kind | Path / signature | Role |
|---|---|---|---|
| `runtime-harvest.py` | Python script (I/O shell) | `.claude/scripts/runtime-harvest.py` | poll endpoints, append sample, flock'd ledger/cursor write, dispatch directive; mirrors `ci-harvest.py` |
| `runtime_harvest.py` | Python module (pure core) | `.claude/scripts/_lib/runtime_harvest.py` | `evaluate`, `reconcile`, `fingerprint`, `normalize` — importable (hyphen in script name blocks import) |
| `runtime_harvest_test.py` | bespoke test | `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` | self-running asserts; `python3 …_test.py` exit 0 = pass |
| `runtime-findings.jsonl` | ledger | `.claude/data/runtime-findings.jsonl` | one line per LIVE finding (schema below) |
| `runtime-cursor.json` | cursor | `.claude/data/runtime-cursor.json` | per-node sample ring buffer + per-fp `clean_poll_streak` + `poll_index` |
| `evaluate` | pure fn | `evaluate(window: dict) -> list[Finding]` | exhaustion predicates over the persisted sample window |
| `reconcile` | pure fn | `reconcile(entries, active, poll_index) -> (new, bumped, closed)` | idempotent append/dedupe + closure-by-disappearance |
| `fingerprint` | pure fn | `fingerprint(node, cls, provenance) -> str` | `sha256(node\|cls\|provenance)[:12]`, churn-normalized |
| `runtime-triage` | agent | `.claude/agents/runtime-triage.md` (model: opus) | dispatched on NEW fp; flag → scope → canonicalize → (fix\|block) |
| `self-heal-exhaustion` | finding class | the ledger `class` field value | Cat C operational ledger state |
| `self-heal-<slug>.md` | backlog (named, NOT built here) | `genesis/data/timeline/backlog/self-heal-<slug>.md` | triage-owned canonical doc |

### `runtime-findings.jsonl` line schema (mirrors `ci-findings.jsonl`, poll-index watermarks)

```json
{"ts": "2026-06-13T07:34:28+00:00", "fp": "a1b2c3d4e5f6", "class": "self-heal-exhaustion", "node": "alpha", "provenance": "render-degenerate", "line": "render.degenerateRate 0.41 sustained >= 4 polls (stalled+timedOut rising)", "status": "open", "seen": 1, "first_poll": 812, "last_poll": 815, "backlog": "genesis/data/timeline/backlog/self-heal-<slug>.md"}
```

Field mapping vs `ci-findings.jsonl` (`ci-harvest.py:13-17`): `class` → `"self-heal-exhaustion"`; `job` → `node`; `category` → `provenance`; `first_build`/`last_build` → `first_poll`/`last_poll` (the prompt's `first_seen`/`last_seen` map here — closure arithmetic needs the monotonic poll-index, NOT wall-clock; `ts` keeps the wall-clock first-capture stamp); `seen`, `status` (`open → triaged → blocked`, triage-owned), `backlog?` unchanged. Closure = DELETION (D5), never a `status:"closed"`.

## File Structure

| File | New/Modify | Responsibility |
|---|---|---|
| `.claude/scripts/_lib/runtime_harvest.py` | **Create** | Tasks 1-4: pure `normalize`, `fingerprint`, `evaluate`, `reconcile` |
| `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` | **Create** | Tasks 1-4: bespoke self-running tests (TDD core) |
| `.claude/scripts/runtime-harvest.py` | **Create** | Tasks 5-7: I/O shell — fetch, sample-append, flock'd ledger/cursor write, render dispatch, `--hook`/`--nodes` argparse, fail-safe |
| `.claude/data/runtime-findings.jsonl` | **Created at runtime** (do not hand-create) | the ledger (Task 6 writes it) |
| `.claude/data/runtime-cursor.json` | **Created at runtime** (do not hand-create) | sample buffer + streaks + poll index (Task 6 writes it) |
| `.claude/agents/runtime-triage.md` | **Create** | Task 7: the dispatched triage agent (mirror `deprecation-triage.md`) |
| `.claude/settings.json` | Modify (SessionStart `hooks` array) | Task 8: register `runtime-harvest.py --hook` (mirror ci-harvest entry) |

## Build / test commands (VERIFIED — E5)

`pytest` is NOT installed (E5: `import pytest → ModuleNotFoundError`). Tests are bespoke self-running asserting scripts — `exit 0 = pass`, nonzero = fail. The pure core is importable via the walk-up bootstrap snippet (env_scope_test.py:6-12). Run from anywhere (paths are resolved by the bootstrap / `CLAUDE_PROJECT_DIR`).

```bash
# Pure-core TDD loop (Tasks 1-4) — exit 0 = pass
python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"

# Poller, manual (Tasks 5-7) — human-readable summary; silent-ish on no signal
python3 /projects/elohim/.claude/scripts/runtime-harvest.py
python3 /projects/elohim/.claude/scripts/runtime-harvest.py --nodes alpha

# Poller, hook mode (Task 8) — SessionStart JSON, NEVER errors (exit 0 on any failure)
python3 /projects/elohim/.claude/scripts/runtime-harvest.py --hook; echo "exit=$?"

# Loop driver (Task 8, operator cadence) — poll every 5 min so consecutive-poll predicates tick
# /loop 5m python3 /projects/elohim/.claude/scripts/runtime-harvest.py
```

---

## Task 1: Pure `normalize` + `fingerprint` (copy from ci-harvest)

**Files:**
- Create: `.claude/scripts/_lib/runtime_harvest.py`
- Create: `.claude/scripts/_lib/__tests__/runtime_harvest_test.py`

- [ ] **Step 1: Write the failing test**

Create `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` (bootstrap snippet mirrors env_scope_test.py:6-12):

```python
#!/usr/bin/env python3
"""Tests for runtime_harvest (the elevate-arm pure core: predicates + ledger
reconcile). Run: python3 .claude/scripts/_lib/__tests__/runtime_harvest_test.py
(exit 0 = pass). pytest is NOT installed — this is the bespoke harness."""
import sys
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
from _lib import runtime_harvest as rh  # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── normalize: build-invariant error-line normalization ──
check("normalize collapses whitespace", rh.normalize("a   b\tc") == "a b c")
check("normalize masks poll counts", rh.normalize("open for 7 polls") == "open for # polls")
check("normalize masks durations", rh.normalize("lag 42s") == "lag #")
check("normalize masks timestamps",
      rh.normalize("at 2026-06-13T07:34:28") == "at #")

# ── fingerprint: stable, node/class lowered, 12-hex ──
fp1 = rh.fingerprint("alpha", "self-heal-exhaustion", "render-degenerate")
check("fingerprint is 12 hex", len(fp1) == 12 and all(c in "0123456789abcdef" for c in fp1))
check("fingerprint stable across reruns",
      fp1 == rh.fingerprint("alpha", "self-heal-exhaustion", "render-degenerate"))
check("fingerprint node-cased-insensitive",
      fp1 == rh.fingerprint("ALPHA", "self-heal-exhaustion", "render-degenerate"))
check("fingerprint differs by node",
      fp1 != rh.fingerprint("jessica", "self-heal-exhaustion", "render-degenerate"))
check("fingerprint count-churn invariant (provenance normalized)",
      rh.fingerprint("alpha", "self-heal-exhaustion", "circuit open 7 polls")
      == rh.fingerprint("alpha", "self-heal-exhaustion", "circuit open 9 polls"))

print(f"\n  {_p} assertions passed ✅")
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — `ModuleNotFoundError: No module named '_lib.runtime_harvest'` (nonzero exit). The module does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Create `.claude/scripts/_lib/runtime_harvest.py` (NO shebang — `_lib` modules omit it, cf. store.py):

```python
"""runtime_harvest — pure core of the elevate-arm runtime poller (findings-
sentinel pattern, instantiation D). NO I/O, NO network: exhaustion predicates
(`evaluate`) over a persisted sample window + ledger `reconcile`. The thin I/O
shell is .claude/scripts/runtime-harvest.py. NEVER imported by runtime Rust —
the no-runtime-write rule means only this external poller touches .claude/data."""
import hashlib
import re

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")

# ── exhaustion thresholds (rationale in the plan Decisions / Task 3) ──
OPEN_POLLS = 3          # circuit Open across >= N consecutive polls
SHED_POLLS = 3          # admission/upstream shed delta > 0 across >= N polls
LAG_POLLS = 3           # projector caughtUp=false / lag rising across >= N polls
LAG_SECONDS = 30        # projector lagSeconds threshold (seconds)
DEGEN_RATE = 0.25       # render degenerateRate sustained threshold
DEGEN_POLLS = 3         # ... across >= N consecutive polls
WINDOW = 8              # ring-buffer length per node (>= max predicate window)
CLASS = "self-heal-exhaustion"


def normalize(line):
    """Build-invariant normalization so poll-count / duration / timestamp churn
    produces ONE fingerprint (mirror ci-harvest.normalize)."""
    line = ANSI.sub("", line)
    line = re.sub(r"\s+", " ", line).strip()
    line = re.sub(r"\b\d{1,3}(?:\.\d{1,3}){3}\b", "#", line)          # IPv4
    line = re.sub(r"\b\d+(?:\.\d+)?(?:ms|s|m|h)\b", "#", line)        # durations
    line = re.sub(r"\b20\d{2}-\d{2}-\d{2}[T ][\d:.]+\S*", "#", line)  # timestamps
    line = re.sub(r"\b\d+\b", "#", line)                              # bare counts/polls
    return line[:300]


def fingerprint(node, cls, provenance):
    """fp(node + class + provenance), 12-hex. node/class are developer
    vocabulary (lowered); provenance is normalized for count-churn invariance."""
    norm = f"{node.lower()}|{cls.lower()}|{normalize(provenance)}"
    return hashlib.sha256(norm.encode()).hexdigest()[:12]
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0`, all assertions print `✅`.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add .claude/scripts/_lib/runtime_harvest.py .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): pure normalize + fingerprint for runtime-harvest core

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 2: Predicate — render degenerate (the LANDED-today signal)

**Files:**
- Modify: `.claude/scripts/_lib/runtime_harvest.py` (add `evaluate`)
- Modify: `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` (add render cases)

The sample window shape (per node): `{"node": str, "samples": [sample, ...]}` where each `sample` is the merged JSON of the endpoints polled that tick, e.g. `{"render": {"degenerateRate": 0.41, "stalled": 9, "timedOut": 2}}` (render-stats keys `total/rendered/renderedEmpty/stalled/timedOut/errored/avgWallMs/maxWallMs/degenerateRate`, verified `stats.rs:39-52`). A `Finding` is a dict `{"node", "class", "provenance", "line"}`. **Field-presence-tolerant: a missing `render` key contributes NO render signal.**

- [ ] **Step 1: Write the failing test**

Append to `runtime_harvest_test.py` (before the final `print`):

```python
# ── evaluate: render-degenerate predicate (LANDED-today signal) ──
def _win(node, samples):
    return {"node": node, "samples": samples}


def _render(rate, stalled=0, timed=0):
    return {"render": {"degenerateRate": rate, "stalled": stalled, "timedOut": timed}}


# sustained high degenerateRate across DEGEN_POLLS -> one finding
hot = _win("alpha", [_render(0.40, 5, 1)] * rh.DEGEN_POLLS)
f_hot = rh.evaluate(hot)
check("render-degenerate fires when sustained",
      any(f["provenance"] == "render-degenerate" for f in f_hot))
check("render-degenerate finding carries node+class",
      f_hot[0]["node"] == "alpha" and f_hot[0]["class"] == rh.CLASS)

# single hot poll (< DEGEN_POLLS) -> no finding
blip = _win("alpha", [_render(0.40, 5, 1)])
check("render-degenerate silent on a single blip",
      not any(f["provenance"] == "render-degenerate" for f in rh.evaluate(blip)))

# healthy (low rate) -> no finding
cool = _win("alpha", [_render(0.01)] * rh.DEGEN_POLLS)
check("render-degenerate silent when healthy",
      not any(f["provenance"] == "render-degenerate" for f in rh.evaluate(cool)))

# absent render field -> no finding (field-presence-tolerant)
empty = _win("alpha", [{}] * rh.DEGEN_POLLS)
check("render-degenerate silent when field absent", rh.evaluate(empty) == [])
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — `AttributeError: module '_lib.runtime_harvest' has no attribute 'evaluate'` (nonzero).

- [ ] **Step 3: Write minimal implementation**

Append to `runtime_harvest.py`:

```python
def _tail(samples, n):
    """Last n samples (the predicate window). Fewer than n => empty (a predicate
    needs n consecutive observations before it can fire)."""
    return samples[-n:] if len(samples) >= n else []


def _render_degenerate(node, samples):
    """SSR saturation: degenerateRate >= DEGEN_RATE for >= DEGEN_POLLS
    consecutive polls. degenerateRate = (stalled+timedOut)/total, the LANDED
    /admin/render-stats headline (stats.rs:50). Field-presence-tolerant."""
    win = _tail(samples, DEGEN_POLLS)
    rates = [s["render"]["degenerateRate"] for s in win
             if isinstance(s.get("render"), dict) and "degenerateRate" in s["render"]]
    if len(rates) < DEGEN_POLLS or any(r < DEGEN_RATE for r in rates):
        return None
    worst = max(rates)
    return {
        "node": node,
        "class": CLASS,
        "provenance": "render-degenerate",
        "line": f"render.degenerateRate {worst:.2f} sustained >= {DEGEN_POLLS} polls "
                f"(SSR stalled/timedOut saturation)",
    }


def evaluate(window):
    """PURE: exhaustion predicates over a node's persisted sample window.
    Returns a list of Finding dicts (possibly empty). NO I/O. Each predicate is
    field-presence-tolerant: an endpoint field absent from the samples (e.g.
    /admin/self-healing not yet landed) contributes NO signal."""
    node = window.get("node", "?")
    samples = window.get("samples", [])
    findings = []
    for pred in (_render_degenerate,):  # Task 3 extends this tuple
        f = pred(node, samples)
        if f is not None:
            findings.append(f)
    return findings
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0`.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add .claude/scripts/_lib/runtime_harvest.py .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): render-degenerate exhaustion predicate (LANDED render-stats signal)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 3: Predicates — circuit / shed / projector-lag (the PENDING /admin/self-healing signals)

**Files:**
- Modify: `.claude/scripts/_lib/runtime_harvest.py` (add three predicates; extend the `evaluate` tuple)
- Modify: `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` (add cases)

These read `/admin/self-healing` fields (PENDING — plan C). They are field-presence-tolerant: until C lands, the fields are absent and these predicates NEVER fire (so D ships safely before C). Thresholds + rationale:

| Predicate | Provenance | Fires when | Rationale |
|---|---|---|---|
| upstream circuit Open | `circuit:<endpoint>` | `upstreams[].circuit == "open"` for `OPEN_POLLS` (3) consecutive polls | a momentary open is normal recovery; 3 polls = sustained inability to reach an upstream = exhaustion. Until C: derived from `errorStreak` monotonic + `skipped == true` if `circuit` absent. |
| admission/upstream shed | `admission-shed` | `admission.shedTotal` strictly increasing across `SHED_POLLS` (3) polls, OR `admission.available == 0` sustained 3 polls | one shed is a spike; a rising shed total across polls = a sustained shed-storm, the inbound-admission exhaustion. |
| projector lag persists | `projector:<id>` | `projector.caughtUp == false` for `LAG_POLLS` (3) polls, OR `projector.lagSeconds >= LAG_SECONDS` (30s) for 3 polls | transient lag self-heals; persistent not-caught-up beyond a window = the projector cannot keep up = exhaustion. 30s chosen as the cache-TTL-order threshold (projector route `cache_ttl(5)` ×6). |

- [ ] **Step 1: Write the failing test**

Append to `runtime_harvest_test.py` (before the final `print`):

```python
# ── evaluate: circuit-open predicate (PENDING /admin/self-healing) ──
def _sh(upstreams=None, admission=None, projector=None):
    d = {}
    if upstreams is not None:
        d["upstreams"] = upstreams
    if admission is not None:
        d["admission"] = admission
    if projector is not None:
        d["projector"] = projector
    return d


open_win = _win("alpha", [_sh(upstreams=[{"endpoint": "storage", "circuit": "open"}])] * rh.OPEN_POLLS)
check("circuit-open fires when open >= N polls",
      any(f["provenance"].startswith("circuit:") for f in rh.evaluate(open_win)))
recov = _win("alpha", [_sh(upstreams=[{"endpoint": "storage", "circuit": "open"}]),
                       _sh(upstreams=[{"endpoint": "storage", "circuit": "closed"}]),
                       _sh(upstreams=[{"endpoint": "storage", "circuit": "open"}])])
check("circuit-open silent when not consecutive",
      not any(f["provenance"].startswith("circuit:") for f in rh.evaluate(recov)))

# ── evaluate: admission-shed predicate ──
shed = _win("alpha", [_sh(admission={"shedTotal": 10}), _sh(admission={"shedTotal": 14}),
                      _sh(admission={"shedTotal": 21})])
check("admission-shed fires on rising shedTotal",
      any(f["provenance"] == "admission-shed" for f in rh.evaluate(shed)))
flat = _win("alpha", [_sh(admission={"shedTotal": 10})] * rh.SHED_POLLS)
check("admission-shed silent when shedTotal flat",
      not any(f["provenance"] == "admission-shed" for f in rh.evaluate(flat)))

# ── evaluate: projector-lag predicate ──
lag = _win("alpha", [_sh(projector={"caughtUp": False, "lagSeconds": 40})] * rh.LAG_POLLS)
check("projector-lag fires when not caught up >= N polls",
      any(f["provenance"].startswith("projector:") for f in rh.evaluate(lag)))
caught = _win("alpha", [_sh(projector={"caughtUp": True, "lagSeconds": 2})] * rh.LAG_POLLS)
check("projector-lag silent when caught up",
      not any(f["provenance"].startswith("projector:") for f in rh.evaluate(caught)))

# absent /admin/self-healing block -> none of the pending predicates fire
absent = _win("alpha", [{"render": {"degenerateRate": 0.0}}] * rh.WINDOW)
check("pending predicates silent when self-healing block absent",
      not any(f["provenance"].startswith(("circuit:", "projector:")) or
              f["provenance"] == "admission-shed" for f in rh.evaluate(absent)))
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — the new circuit/shed/projector asserts fail (predicates not yet in the `evaluate` tuple).

- [ ] **Step 3: Write minimal implementation**

Append the three predicates to `runtime_harvest.py` and extend the `evaluate` tuple:

```python
def _circuit_open(node, samples):
    """An upstream circuit Open for OPEN_POLLS consecutive polls. Until
    /admin/self-healing lands, `upstreams` is absent => no signal. Per-endpoint:
    fires for any endpoint open across the whole window."""
    win = _tail(samples, OPEN_POLLS)
    if len(win) < OPEN_POLLS:
        return None
    endpoints = {u.get("endpoint", "?")
                 for s in win for u in (s.get("upstreams") or [])}
    for ep in endpoints:
        states = []
        for s in win:
            ups = {u.get("endpoint"): u for u in (s.get("upstreams") or [])}
            u = ups.get(ep, {})
            # circuit field is authoritative; fall back to errorStreak+skipped
            if "circuit" in u:
                states.append(u["circuit"] == "open")
            else:
                states.append(bool(u.get("skipped")) and u.get("errorStreak", 0) > 0)
        if len(states) == OPEN_POLLS and all(states):
            return {
                "node": node, "class": CLASS, "provenance": f"circuit:{ep}",
                "line": f"upstream {ep} circuit Open >= {OPEN_POLLS} consecutive polls",
            }
    return None


def _admission_shed(node, samples):
    """Sustained shed-storm: shedTotal strictly rising across SHED_POLLS, OR
    available==0 sustained. Absent `admission` => no signal."""
    win = _tail(samples, SHED_POLLS)
    sheds = [s["admission"]["shedTotal"] for s in win
             if isinstance(s.get("admission"), dict) and "shedTotal" in s["admission"]]
    if len(sheds) == SHED_POLLS and all(b > a for a, b in zip(sheds, sheds[1:])):
        return {"node": node, "class": CLASS, "provenance": "admission-shed",
                "line": f"admission.shedTotal rising {sheds[0]}->{sheds[-1]} over "
                        f"{SHED_POLLS} polls (shed-storm)"}
    avail = [s["admission"]["available"] for s in win
             if isinstance(s.get("admission"), dict) and "available" in s["admission"]]
    if len(avail) == SHED_POLLS and all(a == 0 for a in avail):
        return {"node": node, "class": CLASS, "provenance": "admission-shed",
                "line": f"admission.available == 0 sustained >= {SHED_POLLS} polls"}
    return None


def _projector_lag(node, samples):
    """Projector not caught up / lagSeconds over threshold for LAG_POLLS polls.
    Absent `projector` => no signal."""
    win = _tail(samples, LAG_POLLS)
    projs = [s["projector"] for s in win if isinstance(s.get("projector"), dict)]
    if len(projs) < LAG_POLLS:
        return None
    not_caught = all(p.get("caughtUp") is False for p in projs)
    lag_high = all(isinstance(p.get("lagSeconds"), (int, float))
                   and p["lagSeconds"] >= LAG_SECONDS for p in projs)
    if not_caught or lag_high:
        why = "caughtUp=false" if not_caught else f"lagSeconds>={LAG_SECONDS}"
        return {"node": node, "class": CLASS, "provenance": "projector:reconcile",
                "line": f"projector {why} sustained >= {LAG_POLLS} polls"}
    return None
```

Then change the `evaluate` predicate tuple from `(_render_degenerate,)` to:

```python
    for pred in (_render_degenerate, _circuit_open, _admission_shed, _projector_lag):
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0`.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add .claude/scripts/_lib/runtime_harvest.py .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): circuit/shed/projector-lag predicates (field-tolerant, PENDING self-healing)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 4: Pure `reconcile` — idempotent append + closure by disappearance

**Files:**
- Modify: `.claude/scripts/_lib/runtime_harvest.py` (add `reconcile`)
- Modify: `.claude/scripts/_lib/__tests__/runtime_harvest_test.py` (add cases)

`reconcile(entries, active, poll_index) -> (new, bumped, closed)` is PURE (no disk). `entries` = current ledger list; `active` = findings from `evaluate` this tick (with fp attached); `poll_index` = the monotonic counter. Rules (mirror `ci-harvest.py:282-360`): unknown fp → append `{status:"open", seen:1, first_poll, last_poll}` (NEW, dispatch-eligible); known fp present in `active` → bump `seen`, `last_poll`, reset its `clean_poll_streak` to 0 (BUMPED); known fp ABSENT from `active` → increment `clean_poll_streak`; at `clean_poll_streak >= CLOSE_STREAK` → DELETE the entry (CLOSED, D5 — any status). Suppression is structural: only NEW entries are returned for dispatch.

- [ ] **Step 1: Write the failing test**

Append to `runtime_harvest_test.py` (before the final `print`):

```python
# ── reconcile: idempotent append + closure-by-disappearance ──
def _finding(fp, prov="render-degenerate"):
    return {"fp": fp, "node": "alpha", "class": rh.CLASS, "provenance": prov,
            "line": "x"}


# new fp -> appended as open, returned as NEW
new, bumped, closed = rh.reconcile([], [_finding("aaa")], 100)
check("reconcile appends new fp", len(new) == 1 and new[0]["status"] == "open")
check("reconcile new fp seen=1 + poll watermarks",
      new[0]["seen"] == 1 and new[0]["first_poll"] == 100 and new[0]["last_poll"] == 100)

# re-run same finding next poll -> NOT new (idempotent), bumped instead
entries = list(new)
new2, bumped2, closed2 = rh.reconcile(entries, [_finding("aaa")], 101)
check("reconcile never double-files a known fp", new2 == [])
check("reconcile bumps seen + last_poll", bumped2 and bumped2[0]["seen"] == 2
      and bumped2[0]["last_poll"] == 101)

# absent for CLOSE_STREAK polls -> closed (deleted), not status-flipped
e = [{"fp": "bbb", "node": "alpha", "class": rh.CLASS, "provenance": "x", "line": "x",
      "status": "open", "seen": 1, "first_poll": 1, "last_poll": 1, "clean_poll_streak": 0}]
for i in range(rh.CLOSE_STREAK):
    new3, bumped3, closed3 = rh.reconcile(e, [], 10 + i)
check("reconcile closes by disappearance (deletes the line)",
      closed3 and closed3[0]["fp"] == "bbb" and not any(x["fp"] == "bbb" for x in e))

# blocked fp present again -> still NOT new (structural suppression, any status)
blocked = [{"fp": "ccc", "node": "alpha", "class": rh.CLASS, "provenance": "x",
            "line": "x", "status": "blocked", "seen": 5, "first_poll": 1,
            "last_poll": 9, "clean_poll_streak": 0}]
n4, _, _ = rh.reconcile(blocked, [_finding("ccc")], 50)
check("reconcile suppresses re-dispatch for blocked fp", n4 == [])
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — `AttributeError: module '_lib.runtime_harvest' has no attribute 'reconcile'` (and `CLOSE_STREAK` undefined).

- [ ] **Step 3: Write minimal implementation**

Append to `runtime_harvest.py` (add the constant near the other thresholds, and the function):

```python
CLOSE_STREAK = 3        # fp absent for >= N consecutive polls => closed (deleted)
MAX_NEW_FINDINGS = 12   # ledger appends per run (storm guard, mirror ci-harvest)


def reconcile(entries, active, poll_index):
    """PURE: fold this tick's active findings into the ledger list (MUTATES
    `entries` in place, like ci-harvest). Returns (new, bumped, closed).
    - unknown fp        -> append open/seen=1 (NEW; dispatch-eligible)
    - known fp active   -> bump seen/last_poll, reset clean_poll_streak (BUMPED)
    - known fp inactive -> increment clean_poll_streak; at >= CLOSE_STREAK DELETE
                           (CLOSED by disappearance, ANY status — runtime
                           exhaustions self-resolve without triage; D5)
    Structural suppression: a fp already on the ledger (any status) is NEVER
    returned as NEW."""
    by_fp = {e["fp"]: e for e in entries}
    active_fps = set()
    new, bumped = [], []
    for f in active:
        fp = f["fp"]
        active_fps.add(fp)
        e = by_fp.get(fp)
        if e is None:
            if len(new) >= MAX_NEW_FINDINGS:
                continue
            e = {"fp": fp, "class": f["class"], "node": f["node"],
                 "provenance": f["provenance"], "line": f["line"][:300],
                 "status": "open", "seen": 1,
                 "first_poll": poll_index, "last_poll": poll_index,
                 "clean_poll_streak": 0}
            by_fp[fp] = e
            entries.append(e)
            new.append(e)
        else:
            e["seen"] = e.get("seen", 1) + 1
            e["last_poll"] = poll_index
            e["clean_poll_streak"] = 0
            bumped.append(e)
    closed, kept = [], []
    for e in entries:
        if e["fp"] in active_fps:
            kept.append(e)
            continue
        e["clean_poll_streak"] = e.get("clean_poll_streak", 0) + 1
        if e["clean_poll_streak"] >= CLOSE_STREAK:
            closed.append(e)          # decomposed — line NOT kept (D5)
        else:
            kept.append(e)
    entries[:] = kept
    return new, bumped, closed
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0`, full assertion count printed.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add .claude/scripts/_lib/runtime_harvest.py .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): pure reconcile — idempotent append + closure-by-disappearance

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 5: I/O shell — fetch + sample-window cursor (degrade-quiet)

**Files:**
- Create: `.claude/scripts/runtime-harvest.py`

This is the thin I/O shell (no business logic — it imports the pure core). It fetches per node, merges the endpoint JSON into one sample, appends to the per-node ring buffer in `runtime-cursor.json`, and (Task 6) reconciles. **Degrade quietly (D3):** any fetch failure/timeout/404 yields an empty sub-sample; a fully-unreachable node skips its append.

- [ ] **Step 1: Write the failing test (shell smoke via subprocess)**

Append to `runtime_harvest_test.py` (before the final `print`) — the shell is non-importable (hyphen), so smoke-test it via subprocess against an unreachable node (must exit 0, NOT error):

```python
import subprocess  # noqa: E402

_root = None
_h = Path(__file__).resolve()
for _ in range(8):
    if (_h / ".claude" / "scripts" / "runtime-harvest.py").is_file():
        _root = _h
        break
    _h = _h.parent
check("runtime-harvest.py shell exists", _root is not None)
_r = subprocess.run(
    ["python3", str(_root / ".claude" / "scripts" / "runtime-harvest.py"),
     "--nodes", "doesnotexist", "--base", "http://127.0.0.1:9"],
    capture_output=True, text=True, timeout=60)
check("shell degrades quietly on unreachable node (exit 0)", _r.returncode == 0)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — `runtime-harvest.py shell exists` assert fails (script not created yet).

- [ ] **Step 3: Write minimal implementation**

Create `.claude/scripts/runtime-harvest.py` (shebang present — standalone script, cf. ci-harvest.py:1):

```python
#!/usr/bin/env python3
"""runtime-harvest — EXTERNAL elevate-arm poller (findings-sentinel pattern,
instantiation D). Reads each alpha node's admin read-endpoints, appends one
sample per poll to a per-node ring buffer, evaluates pure exhaustion predicates
over the window, and files NEW findings to .claude/data/runtime-findings.jsonl
+ dispatches a background runtime-triage agent. Mirrors ci-harvest.py.

THE NO-RUNTIME-WRITE RULE: runtime Rust NEVER writes .claude/data. This
external poller READS endpoints and WRITES the ledger. Zero Rust here.

Endpoints (per node, degrade-quiet — a missing one contributes no fields):
  GET /admin/self-healing   PRIMARY (PENDING — stability-surface plan C)
  GET /admin/render-stats   SECONDARY (LANDED)
  GET /health               liveness

Stores:
  .claude/data/runtime-cursor.json    {poll_index, windows:{node:[sample,...]},
                                        # plus clean_poll_streak lives on ledger entries}
  .claude/data/runtime-findings.jsonl one line per LIVE finding:
      {ts, fp, class:"self-heal-exhaustion", node, provenance, line, status,
       seen, first_poll, last_poll, backlog?}  (closure = DELETION, D5)

Modes:
  (default)  poll all NODES, append sample, reconcile; human summary
  --hook     same; emit SessionStart-hook JSON (silent when nothing new)
  --nodes a,b  restrict to listed nodes
  --base URL   override the node base URL template (test/dev)

Fail-safe: in --hook mode every error exits 0 — a node outage must never
break session start.
"""
import argparse
import fcntl
import json
import os
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _lib import runtime_harvest as rh  # noqa: E402

NODES = ["alpha"]  # doorway-alpha pod; extend per cluster-state
BASE_TMPL = "https://doorway-{node}.elohim.host"
ENDPOINTS = {
    "self_healing": "/admin/self-healing",
    "render": "/admin/render-stats",
    "health": "/health",
}
HTTP_TIMEOUT = 8

PROJECT = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
)
CURSOR_PATH = os.path.join(PROJECT, ".claude", "data", "runtime-cursor.json")
LEDGER_PATH = os.path.join(PROJECT, ".claude", "data", "runtime-findings.jsonl")


def get_json(url):
    """Fetch JSON; None on ANY failure (degrade quiet — D3)."""
    try:
        req = urllib.request.Request(url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            return json.loads(resp.read().decode("utf-8", "replace"))
    except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError,
            OSError, ValueError):
        return None


def poll_node(node, base):
    """One poll: fetch every endpoint, merge into ONE sample. Returns the
    sample dict, or None if the node is wholly unreachable (skip its append)."""
    sample, reached = {}, False
    sh = get_json(base + ENDPOINTS["self_healing"])
    if isinstance(sh, dict):
        reached = True
        for k in ("upstreams", "admission", "projector"):
            if k in sh:
                sample[k] = sh[k]
        if isinstance(sh.get("render"), dict):
            sample["render"] = sh["render"]
    rs = get_json(base + ENDPOINTS["render"])
    if isinstance(rs, dict):
        reached = True
        sample["render"] = rs  # render-stats is authoritative for render
    h = get_json(base + ENDPOINTS["health"])
    if isinstance(h, dict):
        reached = True
    return sample if reached else None


def load_cursor():
    try:
        with open(CURSOR_PATH, encoding="utf-8") as fh:
            c = json.load(fh)
            c.setdefault("poll_index", 0)
            c.setdefault("windows", {})
            return c
    except (OSError, json.JSONDecodeError):
        return {"poll_index": 0, "windows": {}}


def load_jsonl(path):
    out = []
    if os.path.exists(path):
        with open(path, encoding="utf-8", errors="replace") as fh:
            for raw in fh:
                try:
                    out.append(json.loads(raw))
                except json.JSONDecodeError:
                    continue
    return out


def write_jsonl(path, entries):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        for e in entries:
            fh.write(json.dumps(e, ensure_ascii=False) + "\n")
    os.replace(tmp, path)
```

(Task 6 adds `harvest()` + `render()` + `main()`. For THIS task's smoke test to pass — exit 0 on an unreachable node — append a minimal `main()` stub that polls and exits cleanly:)

```python
def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--hook", action="store_true")
    ap.add_argument("--nodes")
    ap.add_argument("--base", help="base URL override (test/dev)")
    args = ap.parse_args()
    nodes = [n.strip() for n in args.nodes.split(",")] if args.nodes else NODES
    for node in nodes:
        base = args.base or BASE_TMPL.format(node=node)
        poll_node(node, base)  # Task 6 wires sample-append + reconcile + dispatch


if __name__ == "__main__":
    if "--hook" in sys.argv:
        try:
            main()
        except BaseException:  # noqa: BLE001 — never break session start
            pass
        sys.exit(0)
    main()
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0` (shell exists; unreachable-node poll returns cleanly, subprocess exit 0).

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add .claude/scripts/runtime-harvest.py .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): runtime-harvest I/O shell — degrade-quiet fetch + cursor scaffold

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 6: I/O shell — flock'd ledger/cursor write + dispatch directive

**Files:**
- Modify: `.claude/scripts/runtime-harvest.py` (add `harvest`, `render`; rewrite `main`)

Wire the full loop: append sample → truncate window to `rh.WINDOW` → `rh.evaluate` → attach fp → `rh.reconcile` → flock'd atomic write (ledger BEFORE cursor) → render the dispatch directive. Mirror `ci-harvest.py:reconcile()` flock + `render()` directive shape.

- [ ] **Step 1: Write the failing test (dispatch directive on a synthetic hot node)**

Append to `runtime_harvest_test.py` (before the final `print`). Drive a LOCAL HTTP server returning a hot render-stats body for `DEGEN_POLLS` polls and assert a finding lands + a dispatch directive is emitted:

```python
import http.server  # noqa: E402
import threading  # noqa: E402
import tempfile  # noqa: E402

_hot = json.dumps({"total": 100, "stalled": 40, "timedOut": 5, "degenerateRate": 0.45})


class _Hot(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = _hot if self.path == "/admin/render-stats" else "{}"
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(body.encode())

    def log_message(self, *a):
        pass


_srv = http.server.HTTPServer(("127.0.0.1", 0), _Hot)
threading.Thread(target=_srv.serve_forever, daemon=True).start()
_port = _srv.server_address[1]
_tmp = tempfile.mkdtemp()
_env = dict(os.environ, CLAUDE_PROJECT_DIR=_tmp)
os.makedirs(os.path.join(_tmp, ".claude", "data"), exist_ok=True)
_script = str(_root / ".claude" / "scripts" / "runtime-harvest.py")
for _ in range(rh.DEGEN_POLLS):
    subprocess.run(["python3", _script, "--nodes", "t",
                    "--base", f"http://127.0.0.1:{_port}"],
                   env=_env, capture_output=True, text=True, timeout=60)
_ledger = os.path.join(_tmp, ".claude", "data", "runtime-findings.jsonl")
_lines = [json.loads(x) for x in open(_ledger)] if os.path.exists(_ledger) else []
check("hot node files a self-heal-exhaustion finding",
      any(e["class"] == rh.CLASS and e["provenance"] == "render-degenerate"
          for e in _lines))
# idempotent: re-polling once more does NOT add a second line for the same fp
_before = len(_lines)
subprocess.run(["python3", _script, "--nodes", "t", "--base", f"http://127.0.0.1:{_port}"],
               env=_env, capture_output=True, text=True, timeout=60)
_after = len([json.loads(x) for x in open(_ledger)])
check("re-poll never double-files (idempotent ledger)", _after == _before)
# dispatch directive is emitted in --hook mode on a fresh ledger
_tmp2 = tempfile.mkdtemp()
_env2 = dict(os.environ, CLAUDE_PROJECT_DIR=_tmp2)
os.makedirs(os.path.join(_tmp2, ".claude", "data"), exist_ok=True)
for _ in range(rh.DEGEN_POLLS - 1):
    subprocess.run(["python3", _script, "--nodes", "t", "--base", f"http://127.0.0.1:{_port}"],
                   env=_env2, capture_output=True, text=True, timeout=60)
_hk = subprocess.run(["python3", _script, "--hook", "--nodes", "t",
                      "--base", f"http://127.0.0.1:{_port}"],
                     env=_env2, capture_output=True, text=True, timeout=60)
check("hook mode emits runtime-triage dispatch directive on new fp",
      "runtime-triage" in _hk.stdout and _hk.returncode == 0)
_srv.shutdown()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — no ledger written (the Task-5 `main` stub polls but does not append/reconcile/dispatch).

- [ ] **Step 3: Write minimal implementation**

In `runtime-harvest.py`, add `harvest` + `render` and REPLACE the Task-5 stub `main`:

```python
def harvest(nodes, base_override, as_hook):
    os.makedirs(os.path.dirname(LEDGER_PATH), exist_ok=True)
    lock = open(LEDGER_PATH + ".lock", "w")  # noqa: SIM115 — held for fn lifetime
    fcntl.flock(lock, fcntl.LOCK_EX)
    try:
        cursor = load_cursor()
        cursor["poll_index"] = cursor.get("poll_index", 0) + 1
        poll_index = cursor["poll_index"]
        entries = load_jsonl(LEDGER_PATH)
        active = []
        for node in nodes:
            base = base_override or BASE_TMPL.format(node=node)
            sample = poll_node(node, base)
            if sample is None:
                continue  # wholly unreachable -> no append, no finding (D3)
            win = cursor["windows"].setdefault(node, [])
            win.append(sample)
            del win[: -rh.WINDOW]  # keep last WINDOW samples
            for f in rh.evaluate({"node": node, "samples": win}):
                f["fp"] = rh.fingerprint(f["node"], f["class"], f["provenance"])
                active.append(f)
        new, bumped, closed = rh.reconcile(entries, active, poll_index)
        for e in new:  # stamp wall-clock first-capture time
            e.setdefault("ts", datetime.now(timezone.utc).isoformat(timespec="seconds"))
        write_jsonl(LEDGER_PATH, entries)   # ledger BEFORE cursor (crash-safe)
        os.makedirs(os.path.dirname(CURSOR_PATH), exist_ok=True)
        tmp = CURSOR_PATH + ".tmp"
        with open(tmp, "w", encoding="utf-8") as fh:
            json.dump(cursor, fh, indent=1)
        os.replace(tmp, CURSOR_PATH)
    finally:
        fcntl.flock(lock, fcntl.LOCK_UN)
        lock.close()
    return new, bumped, closed


def render(new, bumped, closed, as_hook):
    parts, sys_parts = [], []
    if new:
        fps = ", ".join(e["fp"] for e in new)
        lines = " | ".join(f'{e["fp"]} [{e["provenance"]}] {e["node"]}: "{e["line"][:80]}"'
                           for e in new[:5])
        parts.append(
            f"[runtime-harvest] {len(new)} NEW self-heal-exhaustion finding(s) "
            f"captured to .claude/data/runtime-findings.jsonl — {lines}. "
            f"DISPATCH (do not derail the current task): launch the `runtime-triage` "
            f"agent via the Agent tool with run_in_background: true and the prompt "
            f"'Triage runtime ledger fingerprint(s) {fps} per your agent definition "
            f"(.claude/agents/runtime-triage.md). The node self-REPORTED exhaustion; "
            f"scope the cause, canonicalize by concern, fix if bounded or document "
            f"the blocker.' Fall back to general-purpose with the same prompt if the "
            f"type is unavailable. Then continue your current task."
        )
        sys_parts.append(f"+{len(new)} new runtime finding(s) -> runtime-triage dispatch")
    if bumped:
        sys_parts.append(f"{len(bumped)} known exhaustion(s) recurred")
    if closed:
        sys_parts.append(f"{len(closed)} exhaustion(s) self-resolved (closed by disappearance)")
    if not parts and not sys_parts:
        return None
    if as_hook:
        return json.dumps({
            "systemMessage": "runtime-harvest: " + "; ".join(sys_parts),
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": " ".join(parts) if parts else "; ".join(sys_parts),
            },
        })
    out = []
    if sys_parts:
        out.append("; ".join(sys_parts))
    out.extend(parts)
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--hook", action="store_true", help="SessionStart-hook JSON output")
    ap.add_argument("--nodes", help="comma-separated node subset")
    ap.add_argument("--base", help="base URL override (test/dev)")
    args = ap.parse_args()
    nodes = [n.strip() for n in args.nodes.split(",")] if args.nodes else NODES
    new, bumped, closed = harvest(nodes, args.base, args.hook)
    rendered = render(new, bumped, closed, args.hook)
    if rendered:
        print(rendered)
    elif not args.hook:
        print("runtime-harvest: nothing new")
```

(Keep the existing `if __name__ == "__main__":` block from Task 5 — it already wraps `--hook` in the fail-safe and calls `main()`.)

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0`; finding filed, idempotent re-poll adds no line, hook mode prints a `runtime-triage` directive.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add .claude/scripts/runtime-harvest.py .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): runtime-harvest write side — flock'd ledger + runtime-triage dispatch

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 7: The `runtime-triage` agent definition

**Files:**
- Create: `.claude/agents/runtime-triage.md`

Mirror `.claude/agents/deprecation-triage.md` exactly (Opus, same tool set, the `flag → scope → canonicalize → (fix | block)` contract, ledger-deletes-on-close, backlog `genesis/data/timeline/backlog/self-heal-<slug>.md`). The agent is what `runtime-harvest` dispatches; it owns canonicalization (named, not auto-built per D9).

- [ ] **Step 1: Write the failing test (definition presence + header contract)**

Append to `runtime_harvest_test.py` (before the final `print`):

```python
_agent = _root / ".claude" / "agents" / "runtime-triage.md"
check("runtime-triage agent definition exists", _agent.is_file())
_txt = _agent.read_text() if _agent.is_file() else ""
check("runtime-triage is model: opus", "model: opus" in _txt)
check("runtime-triage name matches dispatch", "name: runtime-triage" in _txt)
check("runtime-triage reads the runtime ledger", "runtime-findings.jsonl" in _txt)
check("runtime-triage names the backlog convention",
      "genesis/data/timeline/backlog/self-heal-" in _txt)
check("runtime-triage deletes-on-close (rides the pattern)",
      "DELETE" in _txt and "blocked" in _txt)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — `runtime-triage agent definition exists` assert fails.

- [ ] **Step 3: Write minimal implementation**

Create `.claude/agents/runtime-triage.md`:

```markdown
---
name: runtime-triage
description: Runtime self-heal-exhaustion triage and fix agent (Opus). Dispatched (in background) by the runtime-harvest poller when a NEW exhaustion fingerprint lands in .claude/data/runtime-findings.jsonl (a node SELF-REPORTED that a self-healing mechanism is exhausted — circuit stuck Open, shed-storm, projector lag persisting, render saturation). Scopes the cause across the Rust services + manifests, canonicalizes the concern into the timeline backlog (self-heal-<slug>.md, timeline-CONVENTIONS-conformant), then drives to fix when bounded — plan, fan out, implement, verify — or documents the blocker so the deterministic suppression layer stops further dispatches. Invoke when "triage the new exhaustion", "drain runtime ledger entry <fp>", or from a delivery/deprecation-style stasis sweep. Examples: <example>Context: the poller filed a circuit-stuck-Open exhaustion on alpha. user: 'Triage runtime fingerprint a1b2c3d4e5f6' assistant: 'I'll dispatch runtime-triage to scope the upstream-self-protection path, canonicalize the backlog entry, and fix the breaker config if bounded' <commentary>One agent owns the whole flag→canon→fix path for the fingerprint; the node reported, the agent elevates.</commentary></example> <example>Context: a projector-lag exhaustion needs a substrate change we can't take now. user: 'projector caughtUp=false sustained on alpha' assistant: 'runtime-triage will document the blocker in the canonical backlog and mark the ledger entry blocked so the poller stops re-firing' <commentary>Blocked-and-canonicalized is a terminal state for automation; the stasis sweep re-checks it later.</commentary></example>
tools: Task, Bash, Glob, Grep, Read, Edit, Write, TodoWrite, WebFetch
model: opus
color: red
---

You are the runtime self-heal-exhaustion triage agent for the elohim monorepo.
You are dispatched in the background with one or more ledger fingerprints when
the `runtime-harvest` poller (`.claude/scripts/runtime-harvest.py`) detects that
a node's self-healing mechanism is EXHAUSTED — the loop's ELEVATE arm fired. You
own the whole path: **flag → scope → canonicalize → (fix | block)** — and you
leave the system in a state where the deterministic layers (poller fingerprint
dedupe + backlog citation) answer every re-encounter without another dispatch.

## What "elevated" means

The node self-REPORTED exhaustion via an admin read-endpoint
(`/admin/self-healing`, `/admin/render-stats`). The poller turned a sustained
condition into a finding. Your job is NOT to re-detect — it is to find the ROOT
CAUSE in the code/config and resolve or block it. The exhaustion classes:
- `render-degenerate` — SSR stalled/timed-out saturation (`elohim-render`,
  doorway render path, `ssr_busy` seam).
- `circuit:<endpoint>` — an upstream breaker stuck Open (upstream-self-protection
  path; warm-stream health).
- `admission-shed` — inbound admission shed-storm (doorway accept loop / inbound
  semaphore).
- `projector:reconcile` — projector cannot catch up (storage projector / DHT
  reconcile).

## The two stores you reconcile

1. **Ledger** (the poller's EXISTING-POSITIVES check surface):
   `.claude/data/runtime-findings.jsonl` — one JSON line per LIVE finding:
   `{ts, fp, class:"self-heal-exhaustion", node, provenance, line, status,
   seen, first_poll, last_poll, backlog?}`. Presence = the poller suppresses
   dispatch (ANY status); absence-for-N-polls = the poller DELETES it (the node
   self-resolved). Status vocabulary: `open` (captured) → `triaged`
   (canonicalized, fix in flight) → `blocked` (needs operator/substrate). You
   UPDATE the line in place for live transitions (set `status`, `backlog`). You
   do NOT need to delete on fix — the poller closes by disappearance once the
   exhaustion stops recurring (your fix makes it disappear). DELETE manually
   only when you have CONFIRMED the fix removed the condition and want immediate
   closure; a reintroduced exhaustion then reads as NEW and re-fires (regression
   handling for free).
2. **Canonical backlog** (the decision record):
   `genesis/data/timeline/backlog/self-heal-<slug>.md` — one file per *concern*
   (a concern may cover several fingerprints/nodes: e.g. the same circuit Open
   across alpha + jessica). Registered `timeline-entity` managed surface —
   follow `genesis/data/timeline/CONVENTIONS.md` (backlog kind). Frontmatter:

   ```yaml
   ---
   id: "backlog-self-heal-<slug>"
   kind: "backlog"
   contentType: "backlog-item"
   contentFormat: "markdown"
   title: "<exhaustion concern, human-readable>"
   slug: "self-heal-<slug>"
   written: "YYYY-MM-DD"
   author: "runtime-triage"
   status: "backlog" | "wip"          # unified delivery gradient; NO tombstones
   priority: "high" | "medium" | "low"
   self_heal_status: open | in-progress | blocked   # domain axis, ledger-aligned
   severity: low | medium | high
   fingerprints: [<ledger fps this canonicalizes>]
   nodes: [<affected nodes>]
   relatedNodeIds: []
   tags: [self-heal, <class token>]
   cites: [<endpoint URLs that proved it, repo paths — PLAIN paths/URLs>]
   ---
   ```

   Cite discipline: entity docs are DELIBERATELY plain-path cite targets — do
   NOT run cite-gen sealing.

   Body sections: **What is exhausted** (quote the finding line + the endpoint
   JSON that proved it) · **Root-cause inventory** (file:line list from your
   scope pass through the Rust services) · **Fix path** · **Current decision**
   (fix applied / blocked by X — what the poller cites on re-encounter) ·
   **Verification** (what proved the exhaustion stopped, when).

## Procedure

1. **Read the ledger entries** for the fingerprint(s) in your dispatch prompt.
2. **Scope**: Grep/Glob the Rust services for the mechanism behind the
   provenance class (the breaker, the admission semaphore, the projector loop,
   the render path). Check whether an existing `self-heal-*.md` backlog already
   covers this concern — if so EXTEND it (add fingerprints/nodes), never fork.
3. **Confirm reachability**: re-fetch the node's `/admin/self-healing` +
   `/admin/render-stats` with `curl` to confirm the condition is live (the
   poller may have caught a transient). If already self-resolved, note it and
   let the poller close by disappearance.
4. **Canonicalize**: write/extend the backlog entry per the schema above.
5. **Decide and act**:
   - **Bounded fix** (a threshold/config change, a breaker reset path, a missing
     manifest route): implement it, run the affected project's quality gates
     (root CLAUDE.md per-project commands; doorway/storage use the RUSTFLAGS
     overrides), and on green set ledger `status: triaged` + backlog
     `self_heal_status: in-progress`. The poller closes the ledger line by
     disappearance once the exhaustion stops recurring.
   - **Blocked** (needs a substrate change, an operator cluster action, a
     sibling plan to land): document the blocker precisely in **Current
     decision**, set ledger `status: blocked` + backlog `self_heal_status:
     blocked`. SUCCESS for automation — the poller never re-dispatches a present
     fp; the stasis sweep owns re-checks.
6. **Commit-only discipline**: commit on the current branch with a clear
   `fix(self-heal): …` (or `chore(self-heal): …` for block-and-document)
   message. NEVER `git push` — the integrator owns push. Stage selectively if
   the worktree has unrelated in-flight changes.

## Hard rules

- Ledger lines: live transitions in place (`open → triaged → blocked`); the
  poller closes by disappearance. Manual DELETE only on confirmed-removed.
  Never park a tombstone.
- Never claim fixed without re-fetching the endpoint and confirming the
  condition is gone — quote it in the closing commit message.
- One concern = one backlog file; fingerprints/nodes map N:1 onto concerns.
- If the fix would touch >20 files, change a dependency major version, or
  require a cluster (kubectl) action, STOP at `blocked` with a written plan
  sketch — that scale needs an operator-initiated sprint, not a background
  agent. (Cluster ops are operator-owned — never run kubectl.)
- The ELEVATE arm only. You do NOT build the actuation/recover loop (REA
  tune_knob/quarantine) — that is a separate plan; if the fix needs actuation,
  block with that note.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0`.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim && git add .claude/agents/runtime-triage.md .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): runtime-triage agent (mirror deprecation-triage, self-heal class)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 8: Sentinel wiring — SessionStart hook + /loop driver

**Files:**
- Modify: `.claude/settings.json` (SessionStart `hooks` array)

Register `runtime-harvest.py --hook` the SAME way ci-harvest is wired (SessionStart, async, fail-safe — D8). The `/loop` cadence driver is an operator invocation (documented here), not a settings change.

- [ ] **Step 1: Read the current registration**

Read `.claude/settings.json` and locate the `SessionStart` hook array — find the existing ci-harvest entry (`"command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/scripts/ci-harvest.py\" ..."` with `"async": true`). Note the exact JSON shape (object with `type`/`command`/optional `async`/`timeout`).

```bash
python3 - <<'PY'
import json
p = "/projects/elohim/.claude/settings.json"
d = json.load(open(p))
ss = d.get("hooks", {}).get("SessionStart", [])
print(json.dumps(ss, indent=2))
PY
```

- [ ] **Step 2: Write the failing test**

Append to `runtime_harvest_test.py` (before the final `print`):

```python
_settings = _root / ".claude" / "settings.json"
_sd = json.loads(_settings.read_text())


def _flatten(arr):
    cmds = []
    for grp in arr:
        for h in grp.get("hooks", []):
            cmds.append(h.get("command", ""))
    return cmds


_ss = _flatten(_sd.get("hooks", {}).get("SessionStart", []))
check("runtime-harvest registered on SessionStart",
      any("runtime-harvest.py" in c and "--hook" in c for c in _ss))
```

- [ ] **Step 3: Run test to verify it fails**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: FAIL — `runtime-harvest registered on SessionStart` assert fails.

- [ ] **Step 4: Write minimal implementation**

Add a hook entry to the `SessionStart` array in `.claude/settings.json`, matching the ci-harvest entry's shape exactly (same group, or a new group object — match what Step 1 showed). The command:

```json
{
  "type": "command",
  "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/scripts/runtime-harvest.py\" --hook",
  "async": true,
  "timeout": 30
}
```

(If ci-harvest sits in its own group object with a `matcher`/`async`, place runtime-harvest in the SAME group's `hooks` array or clone the group — preserve the existing JSON structure; do NOT reformat unrelated entries. Edit with a single targeted `Edit` on the SessionStart block.)

Also DOCUMENT the cadence driver (operator action — the consecutive-poll predicates need regular ticks SessionStart alone does not give). The canonical invocation (no code change):

```
/loop 5m python3 /projects/elohim/.claude/scripts/runtime-harvest.py
```

or a `/schedule`d cloud routine running the same command every 5 minutes.

- [ ] **Step 5: Run test to verify it passes**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: PASS — `exit=0`.

- [ ] **Step 6: Verify settings.json is still valid JSON, then commit**

```bash
python3 -c "import json; json.load(open('/projects/elohim/.claude/settings.json')); print('valid')"
cd /projects/elohim && git add .claude/settings.json .claude/scripts/_lib/__tests__/runtime_harvest_test.py
git commit -m "feat(elevate): wire runtime-harvest --hook on SessionStart (mirror ci-harvest)

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Task 9: End-to-end self-review against an unreachable + a hot node

**Files:** (no new files — verification + the running ledger)

- [ ] **Step 1: Full pure-core suite green**

Run: `python3 /projects/elohim/.claude/scripts/_lib/__tests__/runtime_harvest_test.py; echo "exit=$?"`
Expected: `exit=0`; assertion count printed (all `✅`).

- [ ] **Step 2: Live degrade-quiet check against alpha**

Run: `python3 /projects/elohim/.claude/scripts/runtime-harvest.py --nodes alpha; echo "exit=$?"`
Expected: `exit=0`. Either `runtime-harvest: nothing new` (alpha healthy / `/admin/self-healing` absent → only render predicate active and not tripped) OR a self-heal-exhaustion summary if alpha is genuinely saturated. NEVER a stack trace, NEVER a finding-storm. Inspect the ledger:

```bash
test -f /projects/elohim/.claude/data/runtime-findings.jsonl && cat /projects/elohim/.claude/data/runtime-findings.jsonl || echo "no findings (expected if healthy)"
python3 -c "import json; json.load(open('/projects/elohim/.claude/data/runtime-cursor.json')); print('cursor valid')"
```

- [ ] **Step 3: Idempotency check**

Run the poller against alpha 3× in a row; confirm the ledger never grows a duplicate line for the same fp (count lines before/after — equal unless a genuinely new exhaustion appeared).

- [ ] **Step 4: Hook-mode fail-safe check**

Run: `python3 /projects/elohim/.claude/scripts/runtime-harvest.py --hook --nodes doesnotexist --base http://127.0.0.1:9; echo "exit=$?"`
Expected: `exit=0`, no output (or empty) — a wholly unreachable node must never break session start.

- [ ] **Step 5: Self-review checklist (confirm each, in the commit message)**

- [ ] ZERO Rust added (no-runtime-write rule): `git diff --name-only <base>..HEAD` shows only `.claude/**` files.
- [ ] `evaluate` and `reconcile` are pure (no `urllib`/`open`/`os` calls inside them) — verified by the network-free unit tests passing.
- [ ] Field-presence-tolerant: the "pending predicates silent when self-healing block absent" + "render silent when field absent" assertions pass — D ships safely before C.
- [ ] Closure = DELETION, not `status:"closed"` (D5) — the closure test deletes the line.
- [ ] Suppression is structural — the blocked-fp test confirms no re-dispatch for any present status.
- [ ] Fingerprint is count-churn invariant — the "circuit open 7 polls" == "circuit open 9 polls" assertion passes.
- [ ] Degrade-quiet — unreachable node yields no finding, exit 0.

- [ ] **Step 6: Commit the self-review note**

```bash
cd /projects/elohim && git commit --allow-empty -m "chore(elevate): self-review — pure core green, zero Rust, degrade-quiet verified

Pure evaluate/reconcile unit-tested (no network/disk); render-degenerate active
today, circuit/shed/projector predicates field-tolerant for the PENDING
/admin/self-healing (plan C); closure-by-disappearance (DELETE, not status:closed);
structural suppression; SessionStart --hook + /loop cadence wired.

$(printf 'Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>')"
```

---

## Self-Review

**Scope adherence (delivers exactly the prompt's IN list):**
1. ✅ External poller `.claude/scripts/runtime-harvest.py` — reads `/admin/self-healing` (primary, PENDING-tolerant) + `/admin/render-stats` + `/health`, evaluates the exhaustion predicates, files a deterministic finding with `fp(node+class+provenance)` mirroring ci-findings (Tasks 5-6).
2. ✅ Pure unit-tested `evaluate(samples) -> list[Finding]` — the TDD core, synthetic fixtures, no network/disk (Tasks 2-3).
3. ✅ Deterministic ledger discipline — idempotent append (dedupe by fp, bump on recurrence, never duplicate an open fp) + closure by disappearance (Task 4).
4. ✅ Sentinel wiring — SessionStart `--hook` (the ci-harvest analog; E4's conclusion) + `/loop` cadence; NEW fp dispatches background `runtime-triage`; structural suppression (Tasks 7-8).

**Out-of-scope, named as follow-ons (D10):** richer runtime emit seams (Rust, plan C builds `/admin/self-healing`); auto-FIX (triage agent owns it); arc-shrink (other thread); REA actuation / recover loop; auto-canonicalization (triage owns it; convention named in D9).

**Hard constraints honored:**
- NO-RUNTIME-WRITE: zero Rust; external poller reads endpoints, writes the ledger (Architecture + Task 9 Step 5 check).
- Endpoint signals over logs (no Loki) — D2, with the verified rationale that every signal has an endpoint home.
- Deterministic + idempotent — pure `evaluate`/`reconcile`, same input → same fp, closure by disappearance (D4/D5, Task 4).
- Ride the existing pattern — same jsonl schema family as ci-findings, same sentinel→triage→canon→stasis flow; Cat C operational state (D5/D6/D7, Canonical names).
- Reachability realism — node admin endpoints polled directly; unreachable degrades quiet, never a storm (D3, Task 9).
- Self-contained — works on render-stats + health TODAY, lights up the C-endpoint predicates with zero code change when it lands (D1, field-presence-tolerant `evaluate`).

**Deviations from the prompt (named, justified):**
- **pytest → bespoke harness.** The prompt's "failing pytest" steps are written against the VERIFIED test convention instead (E5: `pytest` is NOT installed). TDD shape preserved (expect nonzero exit → impl → expect exit 0).
- **`status:"closed"` → DELETION.** Prompt scope (3) said "status closed"; the hard "ride the pattern" constraint wins — the verified pattern deletes (regression-detection for free). Both the field-name divergence (`first_seen/last_seen` → `first_poll/last_poll` poll-index watermarks) and the closure mechanism are documented in D5.
- **Persisted sample window added.** The multi-poll predicates require a `runtime-cursor.json` ring buffer (not named in the prompt but load-bearing); the I/O shell owns it so `evaluate`/`reconcile` stay pure.

**Risks / open items the executor should watch:**
- `.claude/settings.json` SessionStart shape varies (group object vs flat) — Task 8 Step 1 reads it first; edit surgically, re-validate JSON (Task 8 Step 6).
- Predicate thresholds (`OPEN_POLLS=3`, `DEGEN_RATE=0.25`, `LAG_SECONDS=30`) are first-cut; once `/admin/self-healing` lands and real distributions are observed, story-harvest the tuned values (a constraint-bearing discovery) into an a2o regression scenario.
- `NODES = ["alpha"]` is the current single-pod scope; extend from `cluster-state.yaml` `provides_node_types` when multi-node admin polling is reachable.
