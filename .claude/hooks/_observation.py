#!/usr/bin/env python3
"""Structured-observation emitter for the drift-signal hooks.

Station one of the memory-kit replacement (genesis/docs/superpowers/plans/
2026-09-10-memory-kit-replacement-finish.md). Each drift-signal hook used to keep a private JSON
accumulator under `.claude/memory-kit/`; the replacement shape is:

    bound declared in .claude/epr-meta/{measures,policies}.yaml
      -> outcome witnessed by `epr flow note --kind observation --measure <id@version>`
      -> history as folds in the flows sidecar
      -> headline as a projection (`epr flow report --headline`).

**The JSON accumulators are gone (station six round (b), 2026-09-11).** Each hook now writes ONE
thing — the fold — and every accumulated number that used to come from a private JSON file is
derived from the fold plane by the native report:

    cleanup-pressure-ceiling@1       derive: distinct-subjects-since-reset over the five drift measures
    placement-drift-due-ceiling@1    derive: distinct-subjects-since-reset
    map-currency-drift-ceiling@1     derive: distinct-subjects-since-reset
    mempalace-surfaces-changed@1     derive: files-newer-than (reads the tree, not a fold)
    sovereignty-landings-ceiling@1   derive: count-since-reset  (read back by `bound_count` below)
    memkit-report-tier-mb@1          status: superseded — the tier it measured no longer exists

`_HEADLINE_BRIDGE` is EMPTY and `bridge_headline` is GONE: every headline slot has a native
producer, so bridging a kit reading would double a value that is already derived. An emit that
fails now costs the fold, not just a duplicate — which is why `emit` is still fail-open and
silent, and why a hook must never depend on its return value to do its own job.

Nothing is printed on either path: these hooks are best-effort and must never block or narrate.

Probe cost discipline: the hooks run on EVERY Edit|Write under a tight budget, so the probe
result is cached in-process AND on disk, keyed by the binary's path+mtime+size. A rebuilt binary
re-probes; an unchanged one never spawns a subprocess again. A missing binary costs zero
subprocesses.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

__all__ = ["resolve_bin", "available", "emit", "reset_cache", "bound_count",
           "bootstrap_session_id"]

# Resolution order is the one the plan fixes: $EPR_BIN, the gate target, then PATH.
_GATE_TARGET_BIN = "/tmp/eprfs-gate-target/debug/epr"

_PROBE_TIMEOUT = 4  # seconds; a slow/hung binary must degrade to the JSON fallback
_EMIT_TIMEOUT = 5

_probe_memo: dict | None = None  # in-process cache: {"bin":…, "key":…, "available":bool}


def reset_cache() -> None:
    """Drop the in-process probe memo (tests re-probe with a different stub)."""
    global _probe_memo
    _probe_memo = None


def resolve_bin() -> str | None:
    """The `epr` binary this hook should talk to, or None when there is none."""
    env = os.environ.get("EPR_BIN")
    if env and os.path.isfile(env) and os.access(env, os.X_OK):
        return env
    if os.path.isfile(_GATE_TARGET_BIN) and os.access(_GATE_TARGET_BIN, os.X_OK):
        return _GATE_TARGET_BIN
    return shutil.which("epr")


def bootstrap_session_id(project_dir, payload: dict) -> str:
    """The one session label the bootstrap view carries for this session — shared by
    `load-project-context.py` (SessionStart) and `run-projection.py` (UserPromptSubmit) so the
    SessionStart `open` and every later per-turn `open` address the SAME `epr flow memory recall`
    session and its continuation state accumulates, rather than each hook opening its own.

    Prefers the harness payload's own `session_id`. Absent that, falls back to a short hash of
    the project dir + today's date — deterministic across both hooks for the same tree on the
    same day, which is the best available continuity without a harness-supplied id. `project_dir`
    accepts anything `str()`-able (a plain string or a `Path`) so both hooks' native project-dir
    types pass through unchanged.

    This was previously duplicated verbatim in both hook files (the `bootstrapping-head-is-
    recall-open` rule's whole point is exactly one derivation of the bootstrapping head — a
    duplicated formula is the same drift class one file over). One helper, imported by both.
    """
    sid = (payload or {}).get('session_id')
    if sid:
        return str(sid)[:64]
    import hashlib
    from datetime import date
    basis = f"{project_dir}:{date.today().isoformat()}"
    return hashlib.sha256(basis.encode()).hexdigest()[:16]


def _binary_key(path: str) -> str:
    try:
        st = os.stat(path)
        return f"{path}:{int(st.st_mtime)}:{st.st_size}"
    except OSError:
        return f"{path}:?:?"


def _disk_cache_path(key: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9]+", "-", key).strip("-")[-120:]
    return os.path.join(tempfile.gettempdir(), f"claude-epr-measure-probe-{slug}.json")


def _read_disk_cache(key: str):
    try:
        with open(_disk_cache_path(key), encoding="utf-8") as fh:
            blob = json.load(fh)
        if isinstance(blob, dict) and blob.get("key") == key:
            return bool(blob.get("available"))
    except (OSError, json.JSONDecodeError, ValueError):
        pass
    return None


def _write_disk_cache(key: str, value: bool) -> None:
    try:
        with open(_disk_cache_path(key), "w", encoding="utf-8") as fh:
            json.dump({"key": key, "available": value}, fh)
    except OSError:
        pass  # the cache is an optimisation; losing it only costs a re-probe


def _run(argv: list[str], timeout: int) -> str:
    try:
        r = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
        return (r.stdout or "") + (r.stderr or "")
    except (OSError, subprocess.SubprocessError):
        return ""


def _probe(binary: str) -> bool:
    """True when this binary's `flow note` accepts `--measure`.

    `epr flow note --help` is the documented probe. Today it answers with an argument
    error rather than a usage dump, so a miss falls through to the bare `epr flow` usage
    (which prints the full `note` signature) before concluding the verb is absent.
    """
    if "--measure" in _run([binary, "flow", "note", "--help"], _PROBE_TIMEOUT):
        return True
    return "--measure" in _run([binary, "flow"], _PROBE_TIMEOUT)


def available() -> bool:
    """Whether structured observations can be appended natively right now."""
    global _probe_memo
    binary = resolve_bin()
    if not binary:
        _probe_memo = {"bin": None, "key": None, "available": False}
        return False
    key = _binary_key(binary)
    if _probe_memo and _probe_memo.get("key") == key:
        return bool(_probe_memo["available"])
    cached = _read_disk_cache(key)
    if cached is None:
        cached = _probe(binary)
        _write_disk_cache(key, cached)
    _probe_memo = {"bin": binary, "key": key, "available": cached}
    return cached


# Measures whose ZERO is a BULK CLEAR of the whole accumulator rather than one subject healing.
#
# The two shapes look identical at the call site and are not the same event.
# `placement-drift-signal.py:167-170` pops ONE re-opened doc — a per-subject heal, and the native
# report retires that subject by reading its latest fold value. `map-drift-signal.py:143-147`
# empties `store["changed"]` ENTIRELY when MAP.md is refreshed, so a per-subject zero on MAP.md
# would leave every other accumulated seed counted forever. The fold-plane equivalent of emptying a
# collection is the reset measure the registry already declares, so that path is routed to it.
#
# `cleanup-pressure-reset@1` has the same shape and no automated producer: it is the cleanup-CYCLE
# stamp, appended by whoever finishes a cleanup pass, and it replaces `cleanup-pressure.py --reset`:
#   epr flow note --kind observation --measure cleanup-pressure-reset@1 --subject . --value 1
_BULK_CLEAR_ON_ZERO = {
    "map-currency-drift@1": "map-currency-drift-reset@1",
}


def emit(measure: str, subject: str, value, *, reason: str,
         env: dict | None = None, root: str | None = None) -> bool:
    """Append one structured observation. Returns True only when it landed.

    Fail-open by construction: a missing binary, an absent verb, a non-zero exit or a
    timeout all return False so the caller keeps its JSON fallback. Never prints.
    """
    if not available():
        return False
    binary = resolve_bin()
    if not binary:
        return False
    # Measure routing: a bulk clear is the reset measure on the repository, never a zero on one
    # subject (see _BULK_CLEAR_ON_ZERO).
    try:
        is_clear = float(value) == 0.0
    except (TypeError, ValueError):
        is_clear = False
    if is_clear and measure in _BULK_CLEAR_ON_ZERO:
        measure, subject, value = _BULK_CLEAR_ON_ZERO[measure], ".", 1
    # NO `--on`: the native verb REFUSES it alongside `--measure` ("a structured observation's
    # subject IS its target" — elohim/eprfs/epr-cli/src/flow/mod.rs run_observation).
    argv = [binary, "flow", "note",
            "--kind", "observation",
            "--measure", measure,
            "--subject", subject,
            "--value", str(value),
            "--reason", reason]
    for k, v in (env or {}).items():
        argv += ["--env", f"{k}={v}"]
    if root:
        argv += ["--root", root]
    try:
        r = subprocess.run(argv, capture_output=True, text=True, timeout=_EMIT_TIMEOUT)
        return r.returncode == 0
    except (OSError, subprocess.SubprocessError):
        return False


# ── native bound reader ─────────────────────────────────────────────────────────────────
# The inverse direction of `emit`: a hook that needs the ACCUMULATED value of a bound it has
# just folded onto reads it back from the report rather than keeping its own tally.
#
# This replaces the last private accumulator — `sovereignty-guard-signal.py` escalated on a
# `landings` count it summed into a private JSON file under the deleted kit. Only that
# hook calls this, and only on the rare path where a landing actually fired, so the extra
# subprocess is paid per LANDING rather than per edit. Fail-open: a miss returns None and the
# caller says so in its own message instead of reporting a number it did not measure.


def bound_count(bound_id: str, *, root: str, timeout: int = 10):
    """Folds contributing to a derived bound since its last reset, or None.

    `bound_id` is the ceiling row's id WITHOUT the version suffix, as `--bound` takes it.
    """
    if not available():
        return None
    binary = resolve_bin()
    if not binary:
        return None
    try:
        r = subprocess.run([binary, "flow", "report", "--bound", bound_id,
                            "--json", "--root", root],
                           capture_output=True, text=True, timeout=timeout, cwd=root)
        if r.returncode != 0:
            return None
        payload = json.loads(r.stdout)
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError, ValueError):
        return None
    for recipe in payload.get("recipes") or []:
        for outcome in recipe.get("outcomes") or []:
            if outcome.get("bound", "").split("@")[0] == bound_id:
                folds = outcome.get("contributingFolds")
                if isinstance(folds, int):
                    return folds
    return None


if __name__ == "__main__":  # tiny manual probe: `python3 _observation.py`
    print(json.dumps({"bin": resolve_bin(), "measure_verb": available()}), file=sys.stderr)
