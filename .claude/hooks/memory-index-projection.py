#!/usr/bin/env python3
"""PostToolUse `Edit|Write` transport for the memory index — the native verb, and nothing else.

Station four of the memory-kit replacement (genesis/docs/superpowers/plans/
2026-09-10-memory-kit-replacement-finish.md): `.claude/memory/MEMORY.md` becomes
`epr flow memory project --index` under a DECLARED byte bound instead of
`memory-index-projector.py`'s three hard-coded constants.

    epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md

The two legs are proven byte-identical on this corpus (sha256
8ee2e07eac63f45594cf18b6ab5f996594c5c25b0e511fd20bba198a627a83dd, 98 rows, 23,993 bytes), so
this hook is a ROUTER, not a second projector: it never renders a row itself.

Three things it does that a bare command line in settings.json cannot, and each is why it exists:

1. **Probe before spending.** Same discipline as `_observation.py`: resolve the binary
   ($EPR_BIN, the gate target, PATH), try the native verb ONCE under a hard timeout, and cache
   the verdict on disk keyed by the binary's path+mtime+size. A rebuilt binary re-probes; an
   unchanged one never spawns the trial again; a missing binary costs zero subprocesses.

2. **A LATENCY budget, not just a capability probe.** Re-measured 2026-09-11 (station six
   round (a)) on the installed binary: the native projection takes **1.0s**, not the ~119s
   recorded 2026-09-10. The probe is kept because the verdict is a measurement, not a belief —
   a slower tree or a debug binary re-probes and the hook stands down with no edit here.
   `MEMORY_INDEX_NATIVE=1` forces the projection; `=0` disables this hook for a session.

3. **A freshness guard — now a REFUSAL, not a fallback.** The native index projects
   CONTRIBUTIONS, not a directory scan (native report §3: a file with no attributed observation
   does not get to add a row to what every session loads). A memory entry written seconds ago
   has no contribution yet. The projection is therefore rendered to a scratch path first and
   INSTALLED ONLY IF it still carries a row for the entry that was just edited; otherwise the
   index is left exactly as it stands and the advisory names the import. Without this, writing
   a new entry would silently REMOVE its own row from the index.

   The kit's directory-scan projector used to hold this ground; it was deleted at station six
   round (b) (2026-09-11) and NOT replaced, deliberately. Re-rendering the index from a
   directory scan only looked like a save: it wrote rows the contribution plane does not carry,
   which the very next native run removed again — a row that appears and disappears teaches
   nothing. `epr flow memory import .claude/memory --session <id>` takes ~8s on this corpus,
   over the 10s PostToolUse budget once the projection is added, so it stays an explicit act.
   Leaving the index untouched and SAYING SO is the honest state; the row lands on the import.

Fail-open by contract: any error, timeout or missing tool exits 0 having changed nothing, and
nothing is ever printed except a hook `additionalContext` advisory.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HOOKS = Path(__file__).resolve().parent
sys.path.insert(0, str(HOOKS))
from _observation import resolve_bin  # noqa: E402  (shared binary resolution, one owner)

INDEX_REL = ".claude/memory/MEMORY.md"
BUDGET_MEASURE = "memory-index-bytes@1"

# Rendered here first; installed only after the freshness guard passes. Under
# `.eprfs/status/` and NOT under `status/memory/`, so it stays ignored (the contributions store
# below that path is deliberately tracked — see the .gitignore ladder).
SCRATCH_REL = ".eprfs/status/.memory-index-probe.md"

# Seconds. The PostToolUse timeout is 10, so the router leaves itself room to finish and still
# hand back an advisory. `MEMORY_INDEX_BUDGET_SECONDS` widens it — that is the seam the tests use
# to exercise the install path against the REAL binary, and the seam an operator uses to take one
# deliberate slow native run; it is never widened in settings.json.
def _budget() -> int:
    try:
        return max(1, int(os.environ.get("MEMORY_INDEX_BUDGET_SECONDS", "8")))
    except ValueError:
        return 8

_probe_memo: dict | None = None


def reset_cache() -> None:
    """Drop the in-process probe memo (tests re-probe against a different stub)."""
    global _probe_memo
    _probe_memo = None


def _repo() -> Path:
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env and Path(env).is_dir():
        return Path(env).resolve()
    return HOOKS.parent.parent


def _binary_key(path: str) -> str:
    try:
        st = os.stat(path)
        return f"{path}:{int(st.st_mtime)}:{st.st_size}"
    except OSError:
        return f"{path}:?:?"


def _cache_path(key: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9]+", "-", key).strip("-")[-120:]
    return os.path.join(tempfile.gettempdir(), f"claude-epr-memindex-probe-{slug}.json")


def _read_cache(key: str):
    try:
        with open(_cache_path(key), encoding="utf-8") as fh:
            blob = json.load(fh)
        if isinstance(blob, dict) and blob.get("key") == key:
            return bool(blob.get("usable")), blob.get("reason") or ""
    except (OSError, json.JSONDecodeError, ValueError):
        pass
    return None


def _write_cache(key: str, usable: bool, reason: str) -> None:
    try:
        with open(_cache_path(key), "w", encoding="utf-8") as fh:
            json.dump({"key": key, "usable": usable, "reason": reason}, fh)
    except OSError:
        pass  # a lost cache costs one re-probe, nothing else


def _trial(binary: str, root: Path) -> tuple[bool, str]:
    """One timed trial of the native verb. True only if it BOTH works and fits the budget.

    No `--out`: the trial must not write the index. It still appends the unloaded-row fold the
    native leg owns, which dedupes by content — a drained index witnesses its zero once.
    """
    argv = [binary, "flow", "memory", "project", "--index",
            "--budget", BUDGET_MEASURE, "--json", "--root", str(root)]
    started = time.monotonic()
    try:
        r = subprocess.run(argv, capture_output=True, text=True, timeout=_budget())
    except subprocess.TimeoutExpired:
        return False, f"native projection exceeded the {_budget()}s hook budget"
    except (OSError, subprocess.SubprocessError) as exc:
        return False, f"native projection could not run ({type(exc).__name__})"
    if r.returncode != 0:
        detail = (r.stderr or r.stdout or "").strip().splitlines()
        return False, detail[0] if detail else f"native projection exited {r.returncode}"
    return True, f"native projection ready in {time.monotonic() - started:.1f}s"


def native_route(root: Path) -> tuple[str | None, str]:
    """(binary, reason) when the native leg is usable right now, else (None, reason)."""
    global _probe_memo
    forced = os.environ.get("MEMORY_INDEX_NATIVE")
    binary = resolve_bin()
    if not binary:
        return None, "no `epr` binary ($EPR_BIN, the gate target, or PATH)"
    if forced == "0":
        return None, "MEMORY_INDEX_NATIVE=0 stands this hook down"
    if forced == "1":
        return binary, "MEMORY_INDEX_NATIVE=1 pins the native projection"
    key = _binary_key(binary)
    if _probe_memo and _probe_memo.get("key") == key:
        usable, reason = _probe_memo["usable"], _probe_memo["reason"]
    else:
        cached = _read_cache(key)
        if cached is None:
            usable, reason = _trial(binary, root)
            _write_cache(key, usable, reason)
        else:
            usable, reason = cached
        _probe_memo = {"key": key, "usable": usable, "reason": reason}
    return (binary if usable else None), reason


def project_native(binary: str, root: Path, out_rel: str) -> dict | None:
    """Render the native index to `out_rel`. Returns the report payload, or None on any failure."""
    argv = [binary, "flow", "memory", "project", "--index",
            "--budget", BUDGET_MEASURE, "--out", out_rel, "--json", "--root", str(root)]
    try:
        r = subprocess.run(argv, capture_output=True, text=True, timeout=_budget())
    except (OSError, subprocess.SubprocessError):
        return None
    if r.returncode != 0:
        return None
    try:
        return json.loads(r.stdout)
    except (json.JSONDecodeError, ValueError):
        return None


def advise(messages: list[str]) -> None:
    if messages:
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": "\n".join(messages)}}))


def hook(payload_text: str) -> int:
    try:
        payload = json.loads(payload_text)
    except (json.JSONDecodeError, ValueError):
        return 0
    edited = (payload.get("tool_input", {}) or {}).get("file_path") or ""
    if not edited:
        return 0
    root = _repo()
    mem = (root / ".claude" / "memory").resolve()
    try:
        target = Path(edited).resolve()
        if target.parent != mem or target.suffix != ".md":
            return 0
    except OSError:
        return 0

    # A hand-edit of the index itself is never a projection trigger — it is the thing the
    # projection overwrites. Say so once and stop; re-projecting here would discard the edit
    # before its author could read this.
    if target.name == "MEMORY.md":
        advise(["[memory-index] MEMORY.md is a PROJECTION of the contribution plane, not a "
                "source: `epr flow memory project --index` overwrites it. Edit the entry file "
                "under .claude/memory/, then `epr flow memory import .claude/memory "
                "--session <id>` and re-project."])
        return 0

    binary, reason = native_route(root)
    if binary is None:
        return 0

    report = project_native(binary, root, SCRATCH_REL)
    scratch = root / SCRATCH_REL
    if report is None or not scratch.is_file():
        advise([f"[memory-index] native projection unavailable this run ({reason}); "
                f"the index is unchanged."])
        return 0

    rendered = scratch.read_text(encoding="utf-8")
    # Freshness guard: the native index projects CONTRIBUTIONS. An entry written moments ago has
    # none, and installing this projection would delete its own row.
    if f"]({target.name})" not in rendered:
        try:
            scratch.unlink()
        except OSError:
            pass
        advise([f"[memory-index] {target.name} is not yet a contribution, so the projection "
                f"carries no row for it — the index is left UNCHANGED rather than installing a "
                f"render that would drop this entry's own row. Import it with: "
                f"epr flow memory import .claude/memory --session <your-session>"])
        return 0

    index = root / INDEX_REL
    msgs: list[str] = []
    if not index.is_file() or index.read_text(encoding="utf-8") != rendered:
        os.replace(scratch, index)
    else:
        try:
            scratch.unlink()
        except OSError:
            pass
    budget = report.get("budget") or {}
    if budget.get("state") and budget["state"] != "ok":
        msgs.append(
            f"[memory-index] {report.get('bytes')}B against {budget.get('bound', BUDGET_MEASURE)} "
            f"(soft {budget.get('soft')} / hard {budget.get('hard')}: {budget['state']}) — "
            "consolidation is population work: fold related entries under an umbrella or graduate "
            "durable knowledge to its managed home.")
    unloaded = report.get("unloadedRows") or []
    if unloaded:
        msgs.append(f"[memory-index] {len(unloaded)} row(s) past the harness load cap — "
                    "they cost tokens to write and no session can read them.")
    advise(msgs)
    return 0


def main() -> int:
    if "--hook" in sys.argv:
        try:
            return hook(sys.stdin.read())
        except Exception:  # noqa: BLE001 — hooks are fail-open by contract
            return 0
    # Manual probe: which leg would this hook take, and why.
    root = _repo()
    binary, reason = native_route(root)
    print(json.dumps({"route": "native" if binary else "stand-down",
                      "binary": binary or resolve_bin(), "reason": reason}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
