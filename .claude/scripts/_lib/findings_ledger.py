"""findings_ledger.py — the ONE home for the repo's fingerprinted-findings ledger discipline
(seam-concern-contract plan, tasks P4.2/P4.4).

WHY THIS MODULE EXISTS (bind, don't fork). The measure tier already runs this exact protocol
inline in `.claude/hooks/epr-meta-resolver.py::_handle_measures`: one JSONL line per LIVE
fingerprint, an flock'd read-check-append so two concurrent sessions cannot double-file the same
finding, a NEW fingerprint producing a dispatch directive and an already-filed one producing a
debounced one-line citation, and the row DELETED when the finding drains (never parked as
`status: closed`). The cascade (P4.2) and the calibration ledger (P4.4) need the identical
discipline, so it is extracted here rather than re-derived — three callers is the repo's stated
extraction bar and this is the third.

THE ELDER TWIN IS STILL INLINE. `epr-meta-resolver.py` keeps its own copy of this logic; that hook
is out of this task's write-set, so the duplication is deliberate and recorded rather than hidden.
The shapes are byte-compatible on purpose (same `fp` key, same `first_seen`/`status` fields, same
`.claude/data/epr-meta-advice.json` debounce store with a per-namespace key prefix) so the hook can
be refactored onto this module in a follow-up with no ledger migration. If you change the row shape
here, change it there too or the two ledgers drift.

TWO LEDGER MODES, and the difference is load-bearing:
  • LIVE-SET (`append_if_new`) — one line per live fingerprint; the row is REMOVED when the
    condition it names stops holding (`drain`). Reading the file answers "what is wrong right
    now". The architecture (LoC) ledger and the cascade ledger are this mode.
  • APPEND-ONLY (`append`) — every line is a historical fact; nothing is ever removed or mutated.
    Reading the file answers "what happened". The calibration ledger is this mode, because a
    forecast hit/miss is an event, not a condition (an append-only ledger whose rows get drained
    would silently destroy the very growth-rate measure it exists to compute).
A ledger declares its mode in the caller, not here — this module just refuses to blur them by
offering `drain()` only for the live-set shape.

Best-effort throughout: a ledger append must never break the tool that is filing. Every entry
point swallows its own errors and reports failure via return value, exactly as the hook does.
"""
from __future__ import annotations

import json
import time
from pathlib import Path

try:
    import fcntl
except ImportError:  # pragma: no cover — non-POSIX
    fcntl = None

from _lib import store  # the shared debounce store — same file the resolver hook uses

# The debounce window the resolver hook uses for re-citing an already-filed finding
# (~one working session). Kept identical so both tiers feel the same to an agent.
ADVICE_WINDOW = 4 * 3600
ADVICE_STORE_REL = ".claude/data/epr-meta-advice.json"

_LOCK_TIMEOUT = 0.5
_LOCK_RETRY = 0.02


def utc_now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def _flock(fh) -> bool:
    """Non-blocking exclusive flock with a bounded retry. Returns False on contention — a
    caller must treat that as "another session owns this moment", never as "nothing to file"
    (the hook's own comment: the next edit retries)."""
    if fcntl is None:  # pragma: no cover
        return True
    deadline = time.monotonic() + _LOCK_TIMEOUT
    while time.monotonic() < deadline:
        try:
            fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            return True
        except OSError:
            time.sleep(_LOCK_RETRY)
    return False


def read_rows(path: Path) -> list[dict]:
    """Every parseable JSON object line. A corrupt line is SKIPPED, never fatal — a ledger that
    refuses to load because one line got truncated is a dead instrument, and a dead instrument
    reads as 'nothing to report' (the placement-audit gate-liveness lesson)."""
    p = Path(path)
    if not p.is_file():
        return []
    out: list[dict] = []
    try:
        text = p.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return []
    for ln in text.splitlines():
        ln = ln.strip()
        if not ln:
            continue
        try:
            row = json.loads(ln)
        except Exception:  # noqa: BLE001
            continue
        if isinstance(row, dict):
            out.append(row)
    return out


def live_fingerprints(path: Path, *, fp_key: str = "fp") -> set[str]:
    return {r[fp_key] for r in read_rows(path) if isinstance(r.get(fp_key), str)}


def append_if_new(path: Path, entry: dict, *, fp_key: str = "fp") -> bool | None:
    """LIVE-SET append. Returns True if the row was newly filed, False if its fingerprint was
    already live, and None if the lock could not be taken (caller must NOT interpret None as
    either — it means "unknown, retry later", which is why the hook `continue`s on it).

    The read, the membership check and the append all happen under one lock on the ledger fd
    itself — the same shape as the resolver hook, and the reason two sessions cannot both see
    "not filed" and duplicate a row."""
    p = Path(path)
    fp = entry.get(fp_key)
    if not isinstance(fp, str) or not fp:
        return False
    try:
        p.parent.mkdir(parents=True, exist_ok=True)
        with open(p, "a+", encoding="utf-8", errors="replace") as fh:
            if not _flock(fh):
                return None
            fh.seek(0)
            known = set()
            for ln in fh.read().splitlines():
                try:
                    known.add(json.loads(ln).get(fp_key))
                except Exception:  # noqa: BLE001
                    continue
            if fp in known:
                return False
            fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return True
    except Exception:  # noqa: BLE001 — filing must never break the caller
        return None


def append(path: Path, entry: dict) -> bool:
    """APPEND-ONLY append. No dedupe, no drain — every call adds one historical line."""
    p = Path(path)
    try:
        p.parent.mkdir(parents=True, exist_ok=True)
        with open(p, "a", encoding="utf-8", errors="replace") as fh:
            _flock(fh)  # best-effort ordering; a lost lock costs interleaving, never a lost row
            fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return True
    except Exception:  # noqa: BLE001
        return False


def drain(path: Path, keep_fps: set[str], *, fp_key: str = "fp") -> int:
    """LIVE-SET drain: rewrite the ledger keeping ONLY rows whose fingerprint is still live.
    Returns the number of rows removed. This is what makes the file answer "what is wrong right
    now" — a finding whose condition no longer holds is DELETED, never parked as closed."""
    p = Path(path)
    rows = read_rows(p)
    if not rows:
        return 0
    kept = [r for r in rows if r.get(fp_key) in keep_fps]
    removed = len(rows) - len(kept)
    if removed <= 0:
        return 0
    try:
        with open(p, "r+", encoding="utf-8", errors="replace") as fh:
            if not _flock(fh):
                return 0
            fh.seek(0)
            fh.truncate()
            for r in kept:
                fh.write(json.dumps(r, ensure_ascii=False) + "\n")
    except Exception:  # noqa: BLE001
        return 0
    return removed


def debounced(root: Path, key: str, *, window: float = ADVICE_WINDOW) -> bool:
    """True if `key` was advised within `window`; records the advice time otherwise. Shares the
    resolver hook's advice store so ALL epr-meta debounce state lives in one file (worst case on
    lock contention: one extra advisory, never a suppression)."""
    try:
        state_p = Path(root) / ADVICE_STORE_REL
        now = time.time()
        hit = {"debounced": False}

        def upd(seen):
            seen = seen if isinstance(seen, dict) else {}
            last = seen.get(key, 0)
            if isinstance(last, (int, float)) and (now - last) < window:
                hit["debounced"] = True
                return seen
            seen[key] = now
            return seen

        store.locked_update(state_p, upd, {})
        return hit["debounced"]
    except Exception:  # noqa: BLE001 — store trouble -> advise rather than suppress
        return False
