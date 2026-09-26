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
   budget), under the session that wrote the entry: its frontmatter `originSessionId`, else the
   hook input's `session_id`. The worker uses import's ENTRY form (`import <entry.md> --session
   S --as-of T`), which imports only the named entry, authored by the claim S held AT T — the
   instant the entry was written (frontmatter `modified`, else the mtime the hook saw, captured
   synchronously before the worker exists). A claim S registered LATER (a subagent persona in
   the same session) never takes the entry; no claim yet at T means no import. An edit by S never
   sweeps up entries other sessions wrote, and never borrows an author. Workers serialize on one
   flock (two quick edits never race), then project and install under the guard above. Every
   step is one JSON line in a bounded log (`.eprfs/status/memory-import.log.jsonl`), never
   stdout. An entry whose writer held no claim when it was written is named UNATTRIBUTABLE.

5. **The backfill.** Entries written before (4) existed, by sessions now closed, are imported
   under the claim their `originSessionId` held when each was written. It needs no harness
   registration: every worker runs a BOUNDED sweep of the owed orphans under the same lock, so
   they are picked up the next time anyone edits memory. `--backfill` runs the same sweep
   unbounded, for a SessionStart entry to call later with no code change. An entry with no
   origin, or whose origin had not claimed when it was written, is NOT imported under anyone:
   it is reported by name as `unattributable — awaiting its author or the standing human's
   claim`, and its row is the one row the install guard lets the projection leave out.
   `unattributable()` below is the one derivation of that set; the hook advisory, the backfill
   report and the parity test all read it. Claims order by `recordedAt` (the instant `epr actor
   claim` recorded them), falling back to `claimedAt` only for legacy claims.

6. **Steward of record (operator ruling).** An unattributable entry often records the operator's
   own ruling. The advisory names the ONE line the standing human would run —
   `epr flow memory import <entry.md>… --session S --steward-of-record` — and never runs it: the
   native verb admits only an active, non-fixture human Steward, and only entries no witnessed
   agent authored, and records the authorship as steward of record, never as written by them.

Fail-open by contract: any error, timeout or missing tool exits 0 having changed nothing, and
nothing is ever printed except a hook `additionalContext` advisory.
"""

from __future__ import annotations

import datetime
import fcntl
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


# ── the contribution plane, read the way import reads it ─────────────────────────────────────────

MEMORY_REL = ".claude/memory"
CONTRIBUTIONS_REL = ".eprfs/status/memory/contributions"
ACTORS_REL = ".eprfs/status/actors.jsonl"
LOG_REL = ".eprfs/status/memory-import.log.jsonl"
LOCK_REL = ".eprfs/status/.memory-import.lock"
# The log keeps its last LOG_KEEP lines once past LOG_MAX: bounded, never a growing ledger.
LOG_MAX = 400
LOG_KEEP = 200
# Seconds. The worker runs outside any hook budget; these bound a wedged native verb, nothing else.
IMPORT_TIMEOUT = 180
LOCK_WAIT = 300

UNATTRIBUTABLE = "unattributable — awaiting its author or the standing human's claim"

_ORIGIN_RE = re.compile(r"^\s*originSessionId:\s*[\"']?([^\"'\s]+)", re.M)
_INDEX_RE = re.compile(r"^index:\s*[\"']?false[\"']?\s*$", re.M | re.I)


def _frontmatter(path: Path) -> str:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return ""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return ""
    for i, line in enumerate(lines[1:], start=1):
        if line.strip() == "---":
            return "\n".join(lines[1:i])
    return ""


def origin_session(path: Path) -> str | None:
    """The session the harness recorded as having written this entry, or None."""
    m = _ORIGIN_RE.search(_frontmatter(path))
    return m.group(1) if m else None


def opted_out(path: Path) -> bool:
    """`index: false` — the entry is contributed but never carries a row."""
    return bool(_INDEX_RE.search(_frontmatter(path)))


_MODIFIED_RE = re.compile(r"^\s*modified:\s*[\"']?([^\"'\s]+)", re.M)


def _instant(value: str | None) -> datetime.datetime | None:
    """An RFC 3339 instant in UTC, or None. Compared as time, never as text."""
    if not value:
        return None
    try:
        t = datetime.datetime.fromisoformat(value.strip().replace("Z", "+00:00"))
    except ValueError:
        return None
    if t.tzinfo is None:
        return None
    return t.astimezone(datetime.timezone.utc)


def _rfc3339(t: datetime.datetime) -> str:
    return t.astimezone(datetime.timezone.utc).isoformat(timespec="milliseconds").replace(
        "+00:00", "Z")


def written_at(path: Path) -> str | None:
    """When the entry was written: its frontmatter `modified`, else the file's mtime NOW.

    The hook calls this synchronously, before its worker is detached, so the mtime is the one the
    edit left — not whatever a later write leaves by the time the worker runs.
    """
    m = _MODIFIED_RE.search(_frontmatter(path))
    if m and _instant(m.group(1)):
        return m.group(1)
    try:
        return _rfc3339(datetime.datetime.fromtimestamp(path.stat().st_mtime,
                                                        datetime.timezone.utc))
    except OSError:
        return None


def claim_history(root: Path) -> dict[str, list[tuple[int, datetime.datetime | None, str]]]:
    """Session -> its claims in APPEND order: (position, ordering instant, claimed identity).

    The ordering instant is `recordedAt` — when the claim was recorded — and only for a LEGACY
    claim that predates that field, `claimedAt` (the HEAD time, which ties across claims made
    against one tree): exactly `ActorClaim::ordering_instant`.
    """
    out: dict[str, list[tuple[int, datetime.datetime | None, str]]] = {}
    try:
        with open(root / ACTORS_REL, encoding="utf-8") as fh:
            for i, line in enumerate(fh):
                try:
                    rec = json.loads(line).get("record") or {}
                except (json.JSONDecodeError, ValueError, AttributeError):
                    continue
                if rec.get("kind") == "claim" and rec.get("session") and rec.get("claimed"):
                    instant = rec.get("recordedAt") or rec.get("claimedAt")
                    out.setdefault(rec["session"], []).append(
                        (i, _instant(instant), rec["claimed"]))
    except OSError:
        pass
    return out


def claim_as_of(history: dict, session: str | None, at: str | None
                ) -> tuple[int, str] | None:
    """The claim current in `session` as of `at` — the LAST appended with claimedAt <= at —
    exactly as `current_for_at` resolves it. None when no claim existed yet: never a later one.
    """
    when = _instant(at)
    if not session or when is None:
        return None
    found = None
    for pos, claimed_at, claimed in history.get(session, []):
        if claimed_at is not None and claimed_at <= when:
            found = (pos, claimed)
    return found


def contribution_author(root: Path, name: str) -> str | None:
    """The author of the entry's contribution request, or None when it has none.

    Import names the request `<contributions>/<stem>.json`; that is the only shape looked up.
    """
    try:
        blob = json.loads((root / CONTRIBUTIONS_REL / f"{Path(name).stem}.json")
                          .read_text(encoding="utf-8"))
        return str(blob.get("author") or "") or None
    except (OSError, json.JSONDecodeError, ValueError, AttributeError):
        return None


def entry_names(root: Path) -> list[str]:
    try:
        return sorted((p.name for p in (root / MEMORY_REL).glob("*.md")
                       if p.is_file() and p.name != "MEMORY.md"), key=str.lower)
    except OSError:
        return []


def author_session(root: Path, name: str, fallback: str | None = None) -> str | None:
    """Who wrote the entry: its recorded origin session, else the session the hook saw write it."""
    return origin_session(root / MEMORY_REL / name) or fallback


def orphans(root: Path) -> list[str]:
    """Entries with no contribution request at all."""
    return [n for n in entry_names(root) if contribution_author(root, n) is None]


def attributable(root: Path, history: dict) -> dict[str, tuple[str, str, int, str]]:
    """Orphan -> (origin session, written-at, claim position, claimed identity) for every orphan
    whose origin session HAD claimed by the time the entry was written."""
    out = {}
    for name in orphans(root):
        path = root / MEMORY_REL / name
        origin, at = origin_session(path), written_at(path)
        claim = claim_as_of(history, origin, at)
        if claim:
            out[name] = (origin, at, claim[0], claim[1])
    return out


def unattributable(root: Path, history: dict | None = None) -> list[dict]:
    """The ONE derivation of the unattributable set: orphans no claim current AT THEIR WRITING
    can author. A claim the session registered only later never counts.

    An orphan whose origin session had claimed is NOT here — it is owed an import (the backfill's
    job), and a parity check that excused it would be hiding a real mismatch.
    """
    history = claim_history(root) if history is None else history
    out = []
    for name in orphans(root):
        path = root / MEMORY_REL / name
        origin, at = origin_session(path), written_at(path)
        if claim_as_of(history, origin, at):
            continue
        if not origin:
            why = "no originSessionId in its frontmatter"
        elif not history.get(origin):
            why = f"origin session {origin} registered no actor claim"
        else:
            first = min((c for _, c, _ in history[origin] if c), default=None)
            why = (f"origin session {origin} had registered no actor claim when it was written "
                   f"({at}; its first claim is dated {_rfc3339(first) if first else 'undated'})")
        out.append({"entry": name, "originSession": origin, "writtenAt": at,
                    "reason": f"{UNATTRIBUTABLE} ({why})"})
    return out


# ── the bounded log, the lock ───────────────────────────────────────────────────────────────────

def log(root: Path, **record) -> None:
    """One JSON line; trimmed to the last LOG_KEEP lines once past LOG_MAX. Never raises."""
    record = {"at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
              **record}
    path = root / LOG_REL
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "a", encoding="utf-8") as fh:
            fh.write(json.dumps(record, sort_keys=True) + "\n")
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        if len(lines) > LOG_MAX:
            tmp = path.with_suffix(".trim")
            tmp.write_text("".join(lines[-LOG_KEEP:]), encoding="utf-8")
            os.replace(tmp, path)
    except OSError:
        pass


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

def import_entries(binary: str, root: Path, session: str, names: list[str],
                   as_of: str) -> tuple[bool, str]:
    """`epr flow memory import <entry.md>… --session S --as-of T` — the ENTRY form: only these,
    authored by the claim S held AT T (when they were written), never a later one."""
    argv = [binary, "flow", "memory", "import",
            *[f"{MEMORY_REL}/{n}" for n in names],
            "--session", session, "--as-of", as_of, "--json", "--root", str(root)]
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
            timeout: int | None = None) -> tuple[str, dict | None]:
    """Project to scratch; install only if no row is lost but the EXCUSED ones.

    Excused: the named unattributable entries, entries that no longer exist, and entries that opt
    out (`index: false`). Any other lost row means the plane is missing something it should hold,
    and installing would hide it — the index is left as it stands and the outcome says which.
    Returns (outcome, report).
    """
    report = project_native(binary, root, SCRATCH_REL, timeout)
    scratch = root / SCRATCH_REL
    if report is None or not scratch.is_file():
        return "projection-failed", None
    rendered = scratch.read_text(encoding="utf-8")
    index = root / INDEX_REL
    live = index.read_text(encoding="utf-8") if index.is_file() else ""
    mem = root / MEMORY_REL
    lost = sorted(n for n in rows(live) - rows(rendered)
                  if n not in excused and (mem / n).is_file() and not opted_out(mem / n))
    if must_carry and f"]({must_carry})" not in rendered and not opted_out(mem / must_carry):
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


# The orphan sweep a worker runs after its own import is BOUNDED: at most this many import runs
# and this many seconds, so one edit never pays for a whole backlog. What it leaves, the next
# memory edit (or `--backfill`) picks up.
SWEEP_MAX_IMPORTS = 6
SWEEP_BUDGET_SECONDS = 90


def sweep_orphans(binary: str, root: Path, mode: str, max_imports: int | None = None,
                  budget_s: float | None = None) -> dict:
    """Import each orphan under the claim its origin session held WHEN IT WAS WRITTEN. Called
    with the import lock held. Entries resolving to the same claim go in one import, dated by the
    latest of their write times — which, by construction, still resolves to that same claim."""
    result: dict = {"imported": [], "failed": [], "deferred": []}
    history = claim_history(root)
    groups: dict[tuple[str, int], dict] = {}
    for name, (origin, at, pos, claimed) in sorted(attributable(root, history).items()):
        g = groups.setdefault((origin, pos), {"claim": claimed, "entries": [], "asOf": at})
        g["entries"].append(name)
        if _instant(at) > _instant(g["asOf"]):
            g["asOf"] = at
    started, runs = time.monotonic(), 0
    for (session, _), g in sorted(groups.items()):
        if (max_imports is not None and runs >= max_imports) or (
                budget_s is not None and time.monotonic() - started > budget_s):
            result["deferred"].extend(g["entries"])
            continue
        runs += 1
        ok, detail = import_entries(binary, root, session, g["entries"], g["asOf"])
        result["imported" if ok else "failed"].append({"session": session, **g, "detail": detail})
        log(root, mode=mode, session=session, entries=g["entries"], asOf=g["asOf"],
            outcome="imported" if ok else "import-failed", detail=detail)
    if result["deferred"]:
        log(root, mode=mode, outcome="sweep-deferred", entries=result["deferred"])
    return result


def worker(root: Path, session: str | None, name: str, as_of: str) -> int:
    """The detached leg of one PostToolUse dispatch: import the edited entry as of its writing,
    then a bounded sweep of any other orphans, then install. Fail-open: every failure is logged."""
    binary = _binary_or_log(root, "worker")
    if not binary:
        return 0
    try:
        with ImportLock(root):
            claim = claim_as_of(claim_history(root), session, as_of)
            author = contribution_author(root, name)
            if session and claim and (author is None or author == claim[1]):
                ok, detail = import_entries(binary, root, session, [name], as_of)
                log(root, mode="worker", session=session, entries=[name], asOf=as_of,
                    outcome="imported" if ok else "import-failed", detail=detail)
                if not ok:
                    return 0  # a failed import changes nothing: no sweep, no install
            try:
                sweep_orphans(binary, root, "worker-sweep", SWEEP_MAX_IMPORTS,
                              SWEEP_BUDGET_SECONDS)
            except Exception as exc:  # noqa: BLE001 — the sweep is fail-open on its own
                log(root, mode="worker-sweep", outcome="error",
                    detail=f"{type(exc).__name__}: {exc}")
            excused = {u["entry"] for u in unattributable(root)}
            carry = name if contribution_author(root, name) else None
            outcome, _ = install(binary, root, excused, must_carry=carry,
                                 timeout=IMPORT_TIMEOUT)
            log(root, mode="worker", session=session, entries=[name], outcome=outcome)
    except Exception as exc:  # noqa: BLE001 — fail-open, recorded
        log(root, mode="worker", session=session, entries=[name],
            outcome="error", detail=f"{type(exc).__name__}: {exc}")
    return 0


def backfill(root: Path) -> dict:
    """Import every orphan under the claim its origin held when it was written; name the rest.
    Unbounded: the `--backfill` CLI mode (a SessionStart leg can call it later unchanged)."""
    result: dict = {"imported": [], "failed": [], "unattributable": [], "install": None}
    binary = _binary_or_log(root, "backfill")
    if not binary:
        result["install"] = "no-binary"
        return result
    try:
        with ImportLock(root):
            swept = sweep_orphans(binary, root, "backfill")
            result["imported"], result["failed"] = swept["imported"], swept["failed"]
            result["unattributable"] = unattributable(root)
            # The advisory names the exact line the standing human would run — never runs it.
            result["stewardOfRecordCommand"] = steward_command(root, result["unattributable"])
            excused = {u["entry"] for u in result["unattributable"]}
            outcome, _ = install(binary, root, excused, timeout=IMPORT_TIMEOUT)
            result["install"] = outcome
            log(root, mode="backfill", outcome=outcome,
                unattributable=[u["entry"] for u in result["unattributable"]],
                stewardOfRecordCommand=result["stewardOfRecordCommand"])
    except Exception as exc:  # noqa: BLE001 — fail-open, recorded
        result["install"] = f"error: {type(exc).__name__}: {exc}"
        log(root, mode="backfill", outcome="error", detail=result["install"])
    return result


def dispatch(root: Path, session: str | None, name: str, as_of: str) -> bool:
    """Start the worker DETACHED: its own session, no inherited pipes, so the hook returns now.
    `as_of` is captured by the hook BEFORE this — the worker never re-reads the write time.
    With no `session` the worker imports nothing for `name`: it only sweeps the other orphans."""
    try:
        subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "--import-worker",
             *(["--session", session] if session else []),
             "--entry", name, "--as-of", as_of],
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
    """The ONE line the standing human runs to stand for these entries as steward of record
    (operator ruling). The native verb re-checks every condition: it admits only an active,
    non-fixture human Steward, and only entries no witnessed agent authored."""
    if not items:
        return None
    files = " ".join(f"{MEMORY_REL}/{i['entry']}" for i in items)
    return (f"epr actor claim --as {standing_human(root)} --session {STEWARD_SESSION} && "
            f"epr flow memory import {files} --session {STEWARD_SESSION} --steward-of-record")


def _unattributable_line(items: list[dict], root: Path | None = None) -> str | None:
    if not items:
        return None
    line = (f"[memory-index] {len(items)} entr{'y' if len(items) == 1 else 'ies'} left out of the "
            f"index as {UNATTRIBUTABLE}: " + ", ".join(i["entry"] for i in items))
    command = steward_command(root, items) if root else None
    if command:
        line += f". The standing human may stand for them as steward of record: {command}"
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

    if os.environ.get("MEMORY_INDEX_NATIVE") == "0" or not resolve_bin():
        return 0

    # (4) The entry has no contribution — or has one authored by this very writer, whose edit the
    # plane should carry: the HARNESS imports it, detached, under the claim the writing session
    # held WHEN IT WROTE THE ENTRY. The write time is captured here, synchronously, before any
    # worker exists: a later write must not move it, and a claim made later must not take it.
    history = claim_history(root)
    writer = author_session(root, target.name, payload.get("session_id"))
    written = written_at(target) or _rfc3339(datetime.datetime.now(datetime.timezone.utc))
    claim = claim_as_of(history, writer, written)
    author = contribution_author(root, target.name)
    # Other orphans ride along: any worker sweeps them (bounded), so they are picked up the next
    # time anyone edits memory. With nothing owed, no sweep-only worker is started.
    owed = bool(attributable(root, history))
    if author is None or (claim and author == claim[1]):
        if not claim:
            why = ("no session is recorded as its writer" if not writer
                   else f"session {writer} had registered no actor claim when it was written "
                        f"({written})")
            log(root, mode="dispatch", session=writer, entries=[target.name], asOf=written,
                outcome="unattributable", detail=why)
            if owed:
                dispatch(root, None, target.name, written)
            if author is None:
                advise([f"[memory-index] {target.name} is {UNATTRIBUTABLE} ({why}); it is not "
                        "imported under anyone else, and the index is left UNCHANGED. If it "
                        "records the operator's own ruling, the standing human may stand for it "
                        "as steward of record: "
                        + steward_command(root, [{"entry": target.name}])])
            return 0
        if dispatch(root, writer, target.name, written):
            log(root, mode="dispatch", session=writer, entries=[target.name], asOf=written,
                outcome="dispatched")
            if author is None:
                advise([f"[memory-index] {target.name} is not yet a contribution: the harness "
                        f"is importing it under session {writer} as {claim[1]} (its claim as "
                        f"of {written}) and will re-project the index when that lands. "
                        "Nothing to run."])
        return 0
    if owed:
        dispatch(root, None, target.name, written)

    # Contributed already, by someone else: project and install under the guard, as before —
    # inside the hook's budget, so only when the probe says the native leg fits it.
    binary, reason = native_route(root)
    if binary is None:
        return 0
    unattr = unattributable(root, history)
    # The same lock the workers hold, waited on briefly: a worker mid-import will re-project when
    # it lands, so a busy lock is a reason to stand aside, never to install beside it.
    try:
        with ImportLock(root, wait=1.0):
            outcome, report = install(binary, root, {u["entry"] for u in unattr},
                                      must_carry=target.name)
    except TimeoutError:
        log(root, mode="hook", entries=[target.name], outcome="deferred-to-worker")
        return 0
    if report is None:
        advise([f"[memory-index] native projection unavailable this run ({reason}); "
                f"the index is unchanged."])
        return 0
    msgs: list[str] = []
    if outcome.startswith("refused-would-drop:"):
        msgs.append(f"[memory-index] the projection would drop row(s) for "
                    f"{outcome.split(':', 1)[1]} — the index is left UNCHANGED rather than "
                    "installing a render that loses them.")
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
        entry, as_of = _arg("--entry"), _arg("--as-of")
        session = _arg("--session") if "--session" in sys.argv else None
        if entry and as_of:
            try:
                return worker(_repo(), session, entry, as_of)
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
    print(json.dumps({"route": "native" if binary else "stand-down",
                      "binary": binary or resolve_bin(), "reason": reason,
                      "unattributable": unattributable(root)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
