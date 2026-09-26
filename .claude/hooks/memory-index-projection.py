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

3. **A freshness guard — a REFUSAL, not a fallback.** The native index projects
   CONTRIBUTIONS, not a directory scan (native report §3: a file with no attributed observation
   does not get to add a row to what every session loads). A memory entry written seconds ago
   has no contribution yet. The projection is therefore rendered to a scratch path first and
   INSTALLED ONLY IF it drops no row the index carries today, save the rows named below; otherwise
   the index is left exactly as it stands. Without this, writing a new entry would silently REMOVE
   its own row from the index.

   The kit's directory-scan projector used to hold this ground; it was deleted at station six
   round (b) (2026-09-11) and NOT replaced, deliberately. Re-rendering the index from a
   directory scan only looked like a save: it wrote rows the contribution plane does not carry,
   which the very next native run removed again — a row that appears and disappears teaches
   nothing.

4. **The harness imports — agents never do (ruling 2026-09-23,
   `feedback_harness_witnesses_agents_dont_narrate`).** An entry with no contribution used to
   end in an advisory asking the agent to run `epr flow memory import` by hand. Now the hook
   DISPATCHES that import itself, DETACHED from the PostToolUse call (it takes ~8s, over the
   budget). It never parses the entry to decide who wrote it: at the edit moment it appends a
   harness WRITE WITNESS (`.eprfs/status/memory-writes.jsonl`: path, sha256 of the exact bytes,
   its own session, the instant it saw them) and asks `epr flow memory attribution`, the one
   authority, for the rest. The native verb reads the entry through the one frontmatter parser
   (its `originSessionId` governs; `modified` is its write instant), else the witness of its
   exact bytes, and authors it by the claim that writer held AT that instant — a claim made
   later never takes it, and an ambiguous write time (no `modified`, no matching witness; live
   mtime drifts) is never guessed. Workers serialize on one flock (two quick edits never race),
   then project and install under the guard above. Every step is one JSON line in a bounded log
   (`.eprfs/status/memory-import.log.jsonl`), never stdout.

5. **The backfill.** Entries written before (4) existed, by sessions now closed, are imported
   under the claim their writer held when each was written, exactly as (4) decides. It needs no
   harness registration: every worker runs a BOUNDED sweep of the owed orphans under the same
   lock, so they are picked up the next time anyone edits memory. `--backfill` runs the same
   sweep unbounded, for a SessionStart entry to call later with no code change. An orphan the
   native verb cannot attribute is NOT imported under anyone: it is reported by name as
   `unattributable — awaiting its author or the standing human's claim` with the verb's reason,
   and its row is the one row the install guard lets the projection leave out.

6. **Steward of record (operator ruling).** An unattributable entry often records the operator's
   own ruling. The advisory names the ONE line the standing human would run —
   `epr flow memory import <entry.md>… --session S --steward-of-record` — for the entries the
   native verb says no agent author could possibly have written (no origin session; one that
   never claimed; or a write PROVEN by `modified`/a witness to predate its first claim), and
   never runs it. The authorship is recorded as steward of record, never as written by them.

Fail-open by contract: any error, timeout or missing tool exits 0 having changed nothing, and
nothing is ever printed except a hook `additionalContext` advisory.
"""

from __future__ import annotations

import datetime
import fcntl
import hashlib
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


def project_native(binary: str, root: Path, out_rel: str, timeout: int | None = None
                   ) -> dict | None:
    """Render the native index to `out_rel`. Returns the report payload, or None on any failure.

    `timeout` defaults to the hook budget; the detached worker and the backfill pass their own,
    since they run outside any hook's clock.
    """
    argv = [binary, "flow", "memory", "project", "--index",
            "--budget", BUDGET_MEASURE, "--out", out_rel, "--json", "--root", str(root)]
    try:
        r = subprocess.run(argv, capture_output=True, text=True, timeout=timeout or _budget())
    except (OSError, subprocess.SubprocessError):
        return None
    if r.returncode != 0:
        return None
    try:
        return json.loads(r.stdout)
    except (json.JSONDecodeError, ValueError):
        return None


# ── the contribution plane, read through the ONE authority ──────────────────────────────────────
#
# This hook never parses an entry's frontmatter to decide who wrote it or when. It records what
# the HARNESS itself observed — a write witness of the entry's exact bytes, its own session, the
# instant it saw the edit — and asks `epr flow memory attribution` for everything else: the
# entry's `originSessionId`/`modified` through the one frontmatter parser, the witness, the claim
# the writer held at that instant, and whether the entry is attributable, importable, or
# admissible for steward of record. Two parsers once disagreed; there is one now.

MEMORY_REL = ".claude/memory"
ACTORS_REL = ".eprfs/status/actors.jsonl"
WITNESS_REL = ".eprfs/status/memory-writes.jsonl"
LOG_REL = ".eprfs/status/memory-import.log.jsonl"
LOCK_REL = ".eprfs/status/.memory-import.lock"
# The log keeps its last LOG_KEEP lines once past LOG_MAX: bounded, never a growing ledger.
LOG_MAX = 400
LOG_KEEP = 200
# Witnesses are evidence an import may still need, so they are kept longer — still bounded.
WITNESS_MAX = 4000
WITNESS_KEEP = 2000
# Seconds. The worker runs outside any hook budget; these bound a wedged native verb, nothing else.
IMPORT_TIMEOUT = 180
LOCK_WAIT = 300

UNATTRIBUTABLE = "unattributable — awaiting its author or the standing human's claim"


def _now() -> str:
    return datetime.datetime.now(datetime.timezone.utc).isoformat(
        timespec="milliseconds").replace("+00:00", "Z")


def _append_bounded(path: Path, line: str, max_lines: int, keep: int) -> None:
    """Append one line; once past `max_lines`, keep the last `keep`. Never raises."""
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "a", encoding="utf-8") as fh:
            fh.write(line + "\n")
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        if len(lines) > max_lines:
            tmp = path.with_suffix(".trim")
            tmp.write_text("".join(lines[-keep:]), encoding="utf-8")
            os.replace(tmp, path)
    except OSError:
        pass


def log(root: Path, **record) -> None:
    """One JSON line in the bounded import log."""
    _append_bounded(root / LOG_REL, json.dumps({"at": _now(), **record}, sort_keys=True),
                    LOG_MAX, LOG_KEEP)


def witness(root: Path, target: Path, session: str | None) -> str | None:
    """The harness's write witness, appended AT the edit moment: `{path, sha256, session,
    observedAt}` of the entry's exact bytes. The harness's own record — not an agent narrating —
    and the only write time the import trusts after the entry's own `modified`: a live mtime
    drifts forward (a checkout, a re-save) and could make a LATER claim look current."""
    if not session:
        return None
    try:
        data = target.read_bytes()
    except OSError:
        return None
    observed = _now()
    _append_bounded(root / WITNESS_REL, json.dumps({
        "path": f"{MEMORY_REL}/{target.name}",
        "sha256": hashlib.sha256(data).hexdigest(),
        "session": session,
        "observedAt": observed,
    }, sort_keys=True), WITNESS_MAX, WITNESS_KEEP)
    return observed


def attribution(binary: str, root: Path, timeout: int | None = None) -> list[dict] | None:
    """`epr flow memory attribution .claude/memory` — per entry: its writer, write instant, the
    claim that authors it, and whether it is attributable/importable/admissible for steward of
    record. None when the verb is unavailable (fail-open: nothing is decided without it)."""
    argv = [binary, "flow", "memory", "attribution", MEMORY_REL, "--json", "--root", str(root)]
    try:
        r = subprocess.run(argv, capture_output=True, text=True,
                           timeout=timeout or IMPORT_TIMEOUT)
        if r.returncode != 0:
            return None
        return list(json.loads(r.stdout).get("entries") or [])
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError, ValueError,
            AttributeError):
        return None


def orphans(report: list[dict]) -> list[dict]:
    """Entries with no contribution request at all."""
    return [e for e in report if not e.get("contributedBy")]


def unattributable(report: list[dict]) -> list[dict]:
    """The unattributable orphans, each with the native verb's own reason. The ONE derivation:
    the advisory, the backfill and the parity test all read this, from the same report."""
    return [{"entry": e["entry"], "originSession": e.get("session"),
             "reason": f"{UNATTRIBUTABLE} ({e.get('reason')})",
             "stewardOfRecordAdmissible": bool(e.get("stewardOfRecordAdmissible")),
             "stewardOfRecordReason": e.get("stewardOfRecordReason")}
            for e in orphans(report) if not e.get("attributable")]


def opted(report: list[dict]) -> set[str]:
    return {e["entry"] for e in report if e.get("indexed") is False}


# ── the lock ────────────────────────────────────────────────────────────────────────────────────

class ImportLock:
    """One flock for every harness import and install: two quick edits queue, never race."""

    def __init__(self, root: Path, wait: float = LOCK_WAIT):
        self.path, self.wait, self.fh = root / LOCK_REL, wait, None

    def __enter__(self) -> "ImportLock":
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.fh = open(self.path, "a+")
        deadline = time.monotonic() + self.wait
        while True:
            try:
                fcntl.flock(self.fh, fcntl.LOCK_EX | fcntl.LOCK_NB)
                return self
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    self.fh.close()
                    raise TimeoutError(f"import lock held past {self.wait}s")
                time.sleep(0.2)

    def __exit__(self, *exc) -> None:
        try:
            fcntl.flock(self.fh, fcntl.LOCK_UN)
        finally:
            self.fh.close()


# ── the native verbs the worker and the backfill drive ──────────────────────────────────────────

def import_entries(binary: str, root: Path, names: list[str]) -> tuple[bool, str]:
    """`epr flow memory import <entry.md>…` — the ENTRY form, with NO caller assertions: the
    native verb derives each entry's writer and write instant itself and authors it by the claim
    that writer held then, never a later one."""
    argv = [binary, "flow", "memory", "import",
            *[f"{MEMORY_REL}/{n}" for n in names], "--json", "--root", str(root)]
    try:
        r = subprocess.run(argv, capture_output=True, text=True, timeout=IMPORT_TIMEOUT)
    except subprocess.TimeoutExpired:
        return False, f"import exceeded {IMPORT_TIMEOUT}s"
    except (OSError, subprocess.SubprocessError) as exc:
        return False, f"import could not run ({type(exc).__name__})"
    if r.returncode != 0:
        detail = (r.stderr or r.stdout or "").strip().splitlines()
        return False, detail[-1] if detail else f"import exited {r.returncode}"
    try:
        counts = json.loads(r.stdout).get("counts") or {}
        return True, json.dumps(counts, sort_keys=True)
    except (json.JSONDecodeError, ValueError, AttributeError):
        return True, "imported"


_ROW_RE = re.compile(r"\]\(([^)\s]+\.md)\)")


def rows(text: str) -> set[str]:
    return set(_ROW_RE.findall(text))


def install(binary: str, root: Path, excused: set[str], must_carry: str | None = None,
            timeout: int | None = None, opted_out: set[str] | None = None
            ) -> tuple[str, dict | None]:
    """Project to scratch; install only if no row is lost but the EXCUSED ones.

    Excused: the named unattributable entries, entries that no longer exist, and entries that opt
    out (`index: false`, as the native report reads them). Any other lost row means the plane is
    missing something it should hold, and installing would hide it — the index is left as it
    stands and the outcome says which. Returns (outcome, report).
    """
    opted_out = opted_out or set()
    report = project_native(binary, root, SCRATCH_REL, timeout)
    scratch = root / SCRATCH_REL
    if report is None or not scratch.is_file():
        return "projection-failed", None
    rendered = scratch.read_text(encoding="utf-8")
    index = root / INDEX_REL
    live = index.read_text(encoding="utf-8") if index.is_file() else ""
    mem = root / MEMORY_REL
    lost = sorted(n for n in rows(live) - rows(rendered)
                  if n not in excused and (mem / n).is_file() and n not in opted_out)
    if must_carry and f"]({must_carry})" not in rendered and must_carry not in opted_out:
        lost = sorted(set(lost) | {must_carry})
    if lost:
        try:
            scratch.unlink()
        except OSError:
            pass
        return "refused-would-drop:" + ",".join(lost), report
    if live != rendered:
        os.replace(scratch, index)
        return "installed", report
    try:
        scratch.unlink()
    except OSError:
        pass
    return "unchanged", report


def _binary_or_log(root: Path, mode: str) -> str | None:
    binary = resolve_bin()
    if not binary:
        log(root, mode=mode, outcome="no-binary")
    return binary


# The orphan sweep a worker runs after its own import is BOUNDED: at most this many entries in one
# import run, so one edit never pays for a whole backlog. What it leaves, the next memory edit (or
# `--backfill`) picks up.
SWEEP_MAX_IMPORTS = 6


def owed(report: list[dict], skip: str | None = None) -> list[str]:
    """Orphans the native verb says are importable (a claim authored them), minus `skip`."""
    return [e["entry"] for e in orphans(report) if e.get("importable") and e["entry"] != skip]


def _install_from(binary: str, root: Path, must_carry: str | None) -> tuple[str, list[dict]]:
    report = attribution(binary, root) or []
    unattr = unattributable(report)
    outcome, _ = install(binary, root, {u["entry"] for u in unattr}, must_carry=must_carry,
                         timeout=IMPORT_TIMEOUT, opted_out=opted(report))
    return outcome, unattr


def worker(root: Path, session: str | None, name: str) -> int:
    """The detached leg of one PostToolUse dispatch: import the edited entry if the native verb
    says it is importable, then a bounded sweep of the other owed orphans, then install.
    Fail-open: every failure is a log line."""
    binary = _binary_or_log(root, "worker")
    if not binary:
        return 0
    try:
        with ImportLock(root):
            report = attribution(binary, root)
            if report is None:
                log(root, mode="worker", session=session, entries=[name],
                    outcome="attribution-unavailable")
                return 0
            entry = next((e for e in report if e.get("entry") == name), None)
            if entry and entry.get("importable"):
                ok, detail = import_entries(binary, root, [name])
                log(root, mode="worker", session=entry.get("session"), entries=[name],
                    asOf=entry.get("writtenAt"), basis=entry.get("writtenBasis"),
                    outcome="imported" if ok else "import-failed", detail=detail)
                if not ok:
                    return 0  # a failed import changes nothing: no sweep, no install
            elif entry and not entry.get("contributedBy"):
                log(root, mode="worker", session=entry.get("session"), entries=[name],
                    outcome="unattributable", detail=entry.get("reason"))
            sweep = owed(report, skip=name)[:SWEEP_MAX_IMPORTS]
            if sweep:
                ok, detail = import_entries(binary, root, sweep)
                log(root, mode="worker-sweep", entries=sweep,
                    outcome="imported" if ok else "import-failed", detail=detail)
            carry = name if entry and (entry.get("contributedBy") or entry.get("importable")) \
                else None
            outcome, _ = _install_from(binary, root, carry)
            log(root, mode="worker", session=session, entries=[name], outcome=outcome)
    except Exception as exc:  # noqa: BLE001 — fail-open, recorded
        log(root, mode="worker", session=session, entries=[name],
            outcome="error", detail=f"{type(exc).__name__}: {exc}")
    return 0


def backfill(root: Path) -> dict:
    """Import every owed orphan (the native verb decides who authored each); name the rest.
    Unbounded: the `--backfill` CLI mode (a SessionStart leg can call it later unchanged)."""
    result: dict = {"imported": [], "failed": [], "unattributable": [], "install": None,
                    "stewardOfRecordCommand": None}
    binary = _binary_or_log(root, "backfill")
    if not binary:
        result["install"] = "no-binary"
        return result
    try:
        with ImportLock(root):
            report = attribution(binary, root)
            if report is None:
                result["install"] = "attribution-unavailable"
                return result
            by = {e["entry"]: e for e in report}
            names = owed(report)
            if names:
                ok, detail = import_entries(binary, root, names)
                rows_ = [{"entry": n, "session": by[n].get("session"), "claim": by[n].get("claim"),
                          "asOf": by[n].get("writtenAt"), "basis": by[n].get("writtenBasis")}
                         for n in names]
                result["imported" if ok else "failed"] = rows_
                log(root, mode="backfill", entries=names,
                    outcome="imported" if ok else "import-failed", detail=detail)
            outcome, unattr = _install_from(binary, root, None)
            result["unattributable"] = unattr
            # The advisory names the exact line the standing human would run — never runs it.
            result["stewardOfRecordCommand"] = steward_command(root, unattr)
            result["install"] = outcome
            log(root, mode="backfill", outcome=outcome,
                unattributable=[u["entry"] for u in unattr],
                stewardOfRecordCommand=result["stewardOfRecordCommand"])
    except Exception as exc:  # noqa: BLE001 — fail-open, recorded
        result["install"] = f"error: {type(exc).__name__}: {exc}"
        log(root, mode="backfill", outcome="error", detail=result["install"])
    return result


def dispatch(root: Path, session: str | None, name: str) -> bool:
    """Start the worker DETACHED: its own session, no inherited pipes, so the hook returns now.
    The worker is told nothing about attribution: the witness already recorded what the harness
    saw, and the native verb reads it."""
    try:
        subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "--import-worker",
             *(["--session", session] if session else []), "--entry", name],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            cwd=str(root), env={**os.environ, "CLAUDE_PROJECT_DIR": str(root)},
            start_new_session=True, close_fds=True)
        return True
    except (OSError, subprocess.SubprocessError) as exc:
        log(root, mode="dispatch", session=session, entries=[name],
            outcome="spawn-failed", detail=type(exc).__name__)
        return False


def advise(messages: list[str]) -> None:
    if messages:
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": "\n".join(messages)}}))


STEWARD_SESSION = "steward-of-record"


def standing_human(root: Path) -> str:
    """The human this device's roster witnessed (the latest `witness` subject), or a placeholder
    the operator fills — never a guess."""
    found = "human:<handle>"
    try:
        with open(root / ACTORS_REL, encoding="utf-8") as fh:
            for line in fh:
                try:
                    rec = json.loads(line).get("record") or {}
                except (json.JSONDecodeError, ValueError, AttributeError):
                    continue
                if rec.get("kind") == "witness" and str(rec.get("subject", "")).startswith(
                        "human:"):
                    found = rec["subject"]
    except OSError:
        pass
    return found


def steward_command(root: Path, items: list[dict]) -> str | None:
    """The ONE line the standing human runs to stand, as steward of record, for the entries the
    native verb says NO agent author could have written (operator ruling). Entries whose author
    could still be witnessed are never listed; the verb re-checks every condition anyway."""
    admissible = [i for i in items if i.get("stewardOfRecordAdmissible")]
    if not admissible:
        return None
    files = " ".join(f"{MEMORY_REL}/{i['entry']}" for i in admissible)
    return (f"epr actor claim --as {standing_human(root)} --session {STEWARD_SESSION} && "
            f"epr flow memory import {files} --session {STEWARD_SESSION} --steward-of-record")


def _unattributable_line(items: list[dict], root: Path | None = None) -> str | None:
    if not items:
        return None
    line = (f"[memory-index] {len(items)} entr{'y' if len(items) == 1 else 'ies'} left out of the "
            f"index as {UNATTRIBUTABLE}: " + ", ".join(i["entry"] for i in items))
    command = steward_command(root, items) if root else None
    if command:
        line += f". The standing human may stand for the admissible ones as steward of record: " \
                f"{command}"
    return line


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
                "under .claude/memory/; the harness imports it and re-projects."])
        return 0

    if os.environ.get("MEMORY_INDEX_NATIVE") == "0":
        return 0
    binary = resolve_bin()
    if not binary:
        return 0

    # (4) The harness's own record of this edit, FIRST and synchronously: the exact bytes, this
    # session, the instant it saw them. Nothing here reads the entry's frontmatter.
    session = payload.get("session_id")
    witness(root, target, session)

    report = attribution(binary, root, timeout=_budget())
    if report is None:
        return 0
    entry = next((e for e in report if e.get("entry") == target.name), None)
    if entry is None:
        return 0
    orphan = not entry.get("contributedBy")
    # The harness imports it — detached — when the native verb says a claim authored it; and any
    # other owed orphans ride along on the same worker, so they are picked up the next time anyone
    # edits memory. With nothing to import, no worker is started for it.
    if entry.get("importable") or owed(report, skip=target.name):
        if dispatch(root, session, target.name):
            log(root, mode="dispatch", session=session, entries=[target.name],
                outcome="dispatched")
    if orphan:
        if entry.get("importable"):
            advise([f"[memory-index] {target.name} is not yet a contribution: the harness is "
                    f"importing it as {entry.get('claim')} (session {entry.get('session')}, "
                    f"written {entry.get('writtenAt')} by its {entry.get('writtenBasis')}) and "
                    "will re-project the index when that lands. Nothing to run."])
        else:
            log(root, mode="dispatch", session=session, entries=[target.name],
                outcome="unattributable", detail=entry.get("reason"))
            items = unattributable([entry])
            command = steward_command(root, items)
            advise([f"[memory-index] {target.name} is {UNATTRIBUTABLE} ({entry.get('reason')}); "
                    "it is not imported under anyone, and the index is left UNCHANGED."
                    + (" If it records the operator's own ruling, the standing human may stand "
                       f"for it as steward of record: {command}" if command else "")])
        return 0
    if entry.get("importable"):
        return 0  # its author's own edit: the worker re-imports and re-projects

    # Contributed already, by someone else: project and install under the guard, as before —
    # inside the hook's budget, so only when the probe says the native leg fits it.
    binary, reason = native_route(root)
    if binary is None:
        return 0
    unattr = unattributable(report)
    # The same lock the workers hold, waited on briefly: a worker mid-import will re-project when
    # it lands, so a busy lock is a reason to stand aside, never to install beside it.
    try:
        with ImportLock(root, wait=1.0):
            outcome, proj = install(binary, root, {u["entry"] for u in unattr},
                                    must_carry=target.name, opted_out=opted(report))
    except TimeoutError:
        log(root, mode="hook", entries=[target.name], outcome="deferred-to-worker")
        return 0
    if proj is None:
        advise([f"[memory-index] native projection unavailable this run ({reason}); "
                f"the index is unchanged."])
        return 0
    msgs: list[str] = []
    if outcome.startswith("refused-would-drop:"):
        msgs.append(f"[memory-index] the projection would drop row(s) for "
                    f"{outcome.split(':', 1)[1]} — the index is left UNCHANGED rather than "
                    "installing a render that loses them.")
    budget = proj.get("budget") or {}
    if budget.get("state") and budget["state"] != "ok":
        msgs.append(
            f"[memory-index] {proj.get('bytes')}B against {budget.get('bound', BUDGET_MEASURE)} "
            f"(soft {budget.get('soft')} / hard {budget.get('hard')}: {budget['state']}) — "
            "consolidation is population work: fold related entries under an umbrella or graduate "
            "durable knowledge to its managed home.")
    unloaded = proj.get("unloadedRows") or []
    if unloaded:
        msgs.append(f"[memory-index] {len(unloaded)} row(s) past the harness load cap — "
                    "they cost tokens to write and no session can read them.")
    line = _unattributable_line(unattr, root)
    if line:
        msgs.append(line)
    advise(msgs)
    return 0


def _arg(name: str) -> str | None:
    try:
        return sys.argv[sys.argv.index(name) + 1]
    except (ValueError, IndexError):
        return None


def main() -> int:
    if "--hook" in sys.argv:
        try:
            return hook(sys.stdin.read())
        except Exception:  # noqa: BLE001 — hooks are fail-open by contract
            return 0
    if "--import-worker" in sys.argv:
        entry = _arg("--entry")
        session = _arg("--session") if "--session" in sys.argv else None
        if entry:
            try:
                return worker(_repo(), session, entry)
            except Exception:  # noqa: BLE001
                return 0
        return 0
    if "--backfill" in sys.argv:
        try:
            result = backfill(_repo())
        except Exception:  # noqa: BLE001 — a SessionStart leg is fail-open too
            return 0
        if "--json" in sys.argv:
            print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    # Manual probe: which leg would this hook take, and why.
    root = _repo()
    binary, reason = native_route(root)
    report = attribution(binary, root) if binary else None
    print(json.dumps({"route": "native" if binary else "stand-down",
                      "binary": binary or resolve_bin(), "reason": reason,
                      "unattributable": unattributable(report or [])}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
