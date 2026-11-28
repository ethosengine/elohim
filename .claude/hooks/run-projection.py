#!/usr/bin/env python3
"""run-projection.py — the per-turn run-plane projection emitter (read-only, fail-open).

Station 2 of the governed-discovery plan (2026-09-11,
genesis/docs/superpowers/sdd/2026-09-11-governed-discovery-stations-0-3-plan/task-2.2-brief.md)
retires every bespoke derivation this hook used to carry — the habits.yaml line-scan, the saga
import, the flows.jsonl tail scan, the commitments-stock cache, all of it — and replaces them with
ONE call: the `simple` lens of `epr flow memory recall open --purpose bootstrap`, the SAME
bootstrapping head the SessionStart headline projects at `minimal`
(`.claude/hooks/load-project-context.py`). The two hooks are now thin renderers of one native
recall session, declared in `.claude/hooks/.epr-meta` (rule
`bootstrapping-head-is-recall-open`): a hook renders the head at a lens and never derives a
second orientation.

THE PROJECTION CONTRACT — what survives the rewrite:

  * It is injected PER TURN so it cannot drift deeper into context as tokens accumulate (Arize's
    PlanMessage, the run-plane design's survey §1.5 — the corpus's only per-call injection
    precedent). Plain stdout on UserPromptSubmit reaches model context directly — no wrapper
    (`pickup-semantic-surfacing.py:205-206`).
  * THE BINARY IS THE CACHE. The old cache (`.claude/data/run-projection-cache.json`, keyed on
    habits/flows/saga mtimes) existed because this hook used to re-derive state from three files
    on every miss. There is nothing left to cache: one `epr flow memory recall open` call, under a
    measured 6-second budget, replaces the whole derivation — so `cache_key`/`read_cache`/
    `write_cache` are DELETED rather than kept beside a call that no longer needs them.
  * Honest absence, never a fallback renderer: a missing binary, a non-zero exit, or a run past
    the budget all print exactly one line, `bootstrap: skipped — <reason>` — never the retired
    habits/saga/stocks derivation re-grown as a silent substitute.
  * Binary resolution and session-id derivation both come from `_observation.py`
    (`resolve_bin`, `bootstrap_session_id`) — the SAME shared module `load-project-context.py`
    uses, imported DEFENSIVELY (try/except, not a hard top-level `from _observation import ...`)
    so a missing or broken copy of that module degrades this hook to the `bootstrap: skipped`
    line rather than a Python traceback. Fix round 1 (2026-09-11 review): the session-id formula
    used to be duplicated verbatim in both hook files — one helper now, imported by both.

DELIVERY — corrected in fix round 1 (2026-09-11 review): the two registered events do NOT
receive the same treatment.

  * `--event prompt` (UserPromptSubmit, synchronous): plain stdout reaches model context
    directly — no wrapper (`pickup-semantic-surfacing.py:205-206`). This is where the run-plane
    block actually lands, and the only path this hook does real work on.
  * `--event session` (SessionStart, registered `async: true` in `.claude/settings.json`): an
    ASYNC hook's stdout is not read by anything landing in the turn — Claude Code has already
    rendered the SessionStart context by the time an async hook exits, unlike a SYNCHRONOUS
    SessionStart hook (`load-project-context.py`) whose `hookSpecificOutput.additionalContext`
    JSON wrapper IS the documented landing path. A prior revision of this docstring claimed
    "durability-guard.py already relies on exactly this for its SessionStart advisory" as
    justification for plain-stdout-lands-at-SessionStart in general; that claim does not hold
    for an ASYNC entry specifically (durability-guard.py is registered synchronous), so this
    hook now treats `--event session` as a NO-OP (nothing derived, nothing printed) rather than
    spend a subprocess call whose output is not relied upon to land anywhere. Any orientation
    that must land at SessionStart belongs in `load-project-context.py`'s synchronous path.
  * No `--event` flag at all (bare invocation) behaves like `prompt` — the derivation is cheap
    and this is the shape the task's own test drives directly.

RETIRE_WHEN — updated for the new head. The scaffold this hook still is (a thin per-turn
shell-out) is exactly the class that is load-bearing on one model tier and dead weight on the
next, so the exit is part of shipping it, not a footnote:
"""
from __future__ import annotations

import json
import math
import os
import signal
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

HOOKS = Path(__file__).resolve().parent

RETIRE_WHEN = (
    "when a paired run shows the block adds nothing: two consecutive model-tier landings in "
    "which a session run WITHOUT this hook holds the same orientation `epr flow memory recall "
    "open --purpose bootstrap` supplies as a session run WITH it — or earlier, when the harness "
    "itself injects that head's output per turn natively, making this hook a second renderer of "
    "the orientation it exists only to project (the bootstrapping head is declared in "
    "`.claude/hooks/.epr-meta`, rule `bootstrapping-head-is-recall-open`)"
)

RUN_PLANE_LINES = 6      # the block's hard cap (brief: "first 6 lines of the rendering")
TIMEOUT_S = 6             # the measured budget this hook's binary call must fit inside
BACKOFF_INITIAL_S = 60
BACKOFF_MAX_S = 15 * 60
LEDGER_MAX_BYTES = 256 * 1024


def project_dir() -> Path:
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env:
        return Path(env)
    return Path(__file__).resolve().parents[2]


def read_payload() -> dict:
    try:
        return json.loads(sys.stdin.read() or "{}")
    except (json.JSONDecodeError, OSError, ValueError):
        return {}


def state_dir(root: Path) -> Path:
    return root / ".eprfs/status/run-projection"


def _read_breaker(root: Path) -> dict | None:
    """Corrupt or unreadable state is absence: the hook makes the measured call."""
    try:
        state = json.loads((state_dir(root) / "breaker.json").read_text())
        required = {"overrunAt", "overrunAtEpoch", "elapsedSeconds", "loadAverage1m",
                    "consecutiveOverruns", "backoffSeconds"}
        if not isinstance(state, dict) or not required <= state.keys():
            return None
        consecutive = state.get("consecutiveOverruns")
        numbers = [state.get(key) for key in
                   ("overrunAtEpoch", "elapsedSeconds", "backoffSeconds")]
        load = state.get("loadAverage1m")
        if (not isinstance(state["overrunAt"], str)
                or not all(isinstance(value, (int, float)) and not isinstance(value, bool)
                           and math.isfinite(value) for value in numbers)
                or not isinstance(consecutive, int) or isinstance(consecutive, bool)
                or not 1 <= consecutive <= 64
                or numbers[0] < 0 or numbers[1] < 0
                or numbers[2] != min(BACKOFF_INITIAL_S * (2 ** (consecutive - 1)),
                                     BACKOFF_MAX_S)
                or (load is not None and (not isinstance(load, (int, float))
                                          or isinstance(load, bool) or not math.isfinite(load)))):
            return None
        return state
    except (OSError, ValueError, TypeError):
        return None


def _write_json_atomic(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, separators=(",", ":")) + "\n")
    temporary.replace(path)


def _append_overrun(root: Path, row: dict) -> None:
    directory = state_dir(root)
    directory.mkdir(parents=True, exist_ok=True)
    ledger = directory / "overruns.jsonl"
    rotated = directory / "overruns.1.jsonl"
    raw = json.dumps(row, separators=(",", ":")) + "\n"
    try:
        if ledger.exists() and ledger.stat().st_size + len(raw.encode()) > LEDGER_MAX_BYTES:
            rotated.unlink(missing_ok=True)
            ledger.replace(rotated)
        with ledger.open("a") as stream:
            stream.write(raw)
    except OSError:
        pass


def _record_overrun(root: Path, elapsed: float, now: float) -> None:
    previous = _read_breaker(root)
    consecutive = min(int(previous["consecutiveOverruns"]) + 1, 64) if previous else 1
    backoff = min(BACKOFF_INITIAL_S * (2 ** (consecutive - 1)), BACKOFF_MAX_S)
    try:
        load = os.getloadavg()[0]
    except OSError:
        load = None
    row = {
        "overrunAt": datetime.fromtimestamp(now, timezone.utc).isoformat().replace("+00:00", "Z"),
        "overrunAtEpoch": now,
        "elapsedSeconds": round(elapsed, 3),
        "loadAverage1m": load,
        "consecutiveOverruns": consecutive,
        "backoffSeconds": backoff,
    }
    try:
        _write_json_atomic(state_dir(root) / "breaker.json", row)
    except OSError:
        pass
    _append_overrun(root, row)


def _reset_breaker(root: Path) -> None:
    try:
        (state_dir(root) / "breaker.json").unlink(missing_ok=True)
    except OSError:
        pass


def _breaker_skip(root: Path, now: float) -> str | None:
    state = _read_breaker(root)
    if state is None:
        return None
    try:
        overrun_at = float(state["overrunAtEpoch"])
        retry_at = overrun_at + float(state["backoffSeconds"])
        # A wall clock set back before the overrun would hold the breaker open for as long as the
        # jump: read that as expired, and make the call.
        if now >= retry_at or now < overrun_at:
            return None
        retry = datetime.fromtimestamp(retry_at, timezone.utc).isoformat().replace("+00:00", "Z")
        return ("bootstrap: skipped — breaker open after overrun at "
                f'{state["overrunAt"]} ({float(state["elapsedSeconds"]):.3f}s); retry at {retry}')
    except (OverflowError, OSError, ValueError, TypeError):
        return None


def _observation_module():
    """The hooks' shared emitter, imported DEFENSIVELY — the same try/except pattern
    `load-project-context.py`'s `_observation_module()` uses. A missing or broken
    `.claude/hooks/_observation.py` degrades this hook to the `bootstrap: skipped` line
    (fix round 1, 2026-09-11 review: this used to be a hard top-level
    `from _observation import resolve_bin` that would raise ImportError at load time)."""
    sys.path.insert(0, str(HOOKS))
    try:
        import _observation
        return _observation
    except Exception:
        return None


def run_plane_lines(root: Path, payload: dict) -> list[str]:
    """The `simple` lens of `epr flow memory recall open --purpose bootstrap` — the per-turn
    half of the one bootstrapping head. No second orientation is derived here: no habits.yaml
    line-scan, no saga import, no flows.jsonl tail scan (all retired; see the module docstring).

    Honest absence on any failure: a missing `_observation` module, a missing binary, a
    non-zero exit, or a run past the budget all return a single `bootstrap: skipped —
    <reason>` line.
    """
    import subprocess
    now = time.time()
    skipped = _breaker_skip(root, now)
    if skipped:
        return [skipped]
    obs = _observation_module()
    binary = obs.resolve_bin() if obs else None
    if not binary:
        return ["bootstrap: skipped — no epr binary resolved ($EPR_BIN or PATH)"]
    session = obs.bootstrap_session_id(root, payload)
    started = time.monotonic()
    try:
        # Own process group, killed whole at the budget: `subprocess.run(timeout=)` kills only
        # the child and then waits for pipe EOF, which a grandchild still holding stdout delays
        # past the harness's 8 s hook timeout (measured 2026-10-30 at load 54: 9.6–11 s per
        # prompt, every turn cancelled and shown to the steward as a red error).
        proc = subprocess.Popen(
            [binary, "flow", "memory", "recall", "open",
             "--purpose", "bootstrap", "--lens", "simple",
             "--session", session, "--root", str(root)],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True,
        )
        try:
            stdout, stderr = proc.communicate(timeout=TIMEOUT_S)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except OSError:
                pass
            try:
                # Bounded: a grandchild that left the group could hold the pipe open, which is why
                # the killed call is not waited on unboundedly.
                proc.communicate(timeout=0.5)
            except Exception:  # noqa: BLE001 — reaping is best effort; the skip line still prints
                pass
            _record_overrun(root, time.monotonic() - started, time.time())
            return [f"bootstrap: skipped — recall open exceeded the {TIMEOUT_S}s budget"]
    except Exception as exc:  # noqa: BLE001 — a per-turn hook must never block or crash a turn
        return [f"bootstrap: skipped — {exc}"]
    _reset_breaker(root)
    if proc.returncode != 0:
        reason_lines = (stderr or stdout or "").strip().splitlines()
        reason = reason_lines[0] if reason_lines else f"exit {proc.returncode}"
        return [f"bootstrap: skipped — {reason}"]
    lines = stdout.splitlines()
    if not lines:
        return ["bootstrap: skipped — empty rendering"]
    return lines[:RUN_PLANE_LINES]


def main() -> int:
    event = sys.argv[sys.argv.index("--event") + 1] if "--event" in sys.argv else "prompt"
    payload = read_payload()  # drained so the writing end never blocks on a full pipe
    try:
        if event == "session":
            # Registered `async: true` — nothing reads this event's stdout, so a real
            # derivation here would be a subprocess call paid for nothing. See the module
            # docstring's DELIVERY section (fix round 1, 2026-09-11 review).
            return 0
        root = project_dir()
        lines = run_plane_lines(root, payload)
        print("\n".join(lines))
    except Exception:  # noqa: BLE001 — a projection must never block a turn or a session start
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
