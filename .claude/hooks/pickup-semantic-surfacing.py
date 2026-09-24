#!/usr/bin/env python3
"""
PICKUP fire point — session-pickup semantic surfacing (read-only).

Spec: genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md §4b.

Two harness surfaces, one once-per-session gate:

  --event prompt   UserPromptSubmit (primary). Eligible on prompts 1-3 of a session. Fires on
                   pickup-vocabulary regex (incl. /deliver|/shift|/converge|/plan slash pickups;
                   /brainstorm excluded — owns its own FRONT seam). Always stashes early prompts
                   so the fallback net has query material.
  --event tool     PreToolUse on Grep|Glob|Agent (fallback net). Fires iff (a) no injection yet,
                   (b) first search-shaped tool call of the session, (c) still inside the
                   first-3-prompt window (stash exists). Query = stashed prompt + distilled
                   tool pattern.

Engine is the NATIVE semantic provider (governed-discovery station 4, task 4.6), reached through
the one `epr` resolver `_observation.resolve_bin` owns. Once per Claude session the hook opens its
own governed recall session (`epr flow memory recall open --session surfacing-<session> --need
<query>`; the executor requires the open) and asks `epr flow memory recall search --provider
semantic --query <query> --search-scope . --json` — bounded by the recall contract, every candidate
carrying its producer, method and the fold's lag — each injected line prints its producer and
short method CID, and the lag when behind. Freshness is the fold's own lag, read from the search
answer's `fold_lag` (`epr flow memory index status --json` only as the fallback when the search gave
no reading): when it is behind (lag > 0) the hook spawns exactly ONE
detached `epr flow memory index fold --max-files <limits.fold_files_per_run>` (its own session,
output appended to `.eprfs/status/index/fold-spawn.log`, which keeps its last 256 KiB once past
1 MiB) and never waits on it, so the lag
converges across sessions without blocking a prompt; the fold's flock turns a concurrent spawn
into a harmless `busy`. Injection contract (§4b.3): top candidate under the 0.35 cosine floor →
silent no-op; a DEGRADED banner only when the lag is past the measure's declared foldLag limit
(the headline's `index:` line reads the same bound); top-4 hits; recall-hints-not-truth footer.
Always exit 0 — surfacing must never block a prompt or a tool call.

MemPalace is the optional visitor, not the engine: its own `mine` is visitor maintenance and no
longer a headline gate, and nothing here calls it.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

COSINE_FLOOR = 0.35
TOP_K = 4
MAX_PROMPT_WINDOW = 3          # prompts 1..3 are pickup-eligible
QUERY_MAX_CHARS = 300
# The harness allows this hook 15 s; the three bounded calls share one 13 s budget.
HOOK_BUDGET_S = 13.0
STATUS_TIMEOUT_S = 4.0
OPEN_TIMEOUT_S = 8.0
SEARCH_TIMEOUT_S = 8.0
CONTRACT_REL = ".epr-meta/elohim/algorithms/recall-contract.json"
SPAWN_LOG_REL = ".eprfs/status/index/fold-spawn.log"
# Retention: past 1 MiB before an append, the spawn log keeps its last 256 KiB.
SPAWN_LOG_MAX = 1 << 20
SPAWN_LOG_KEEP = 256 << 10

PICKUP_RE = re.compile(
    r"(?:"
    r"where (?:are|were) we"
    r"|pick(?:ing)? up"
    r"|resume"
    r"|continue (?:with|from|where)"
    r"|status of"
    r"|what'?s next"
    r"|where did we leave"
    r"|catch me up"
    r"|state of (?:the|our)"
    r")",
    re.IGNORECASE,
)
SLASH_PICKUP_RE = re.compile(r"^/(deliver|shift|converge|plan)\b", re.IGNORECASE)
SLASH_EXCLUDED_RE = re.compile(r"^/brainstorm\b", re.IGNORECASE)


def project_dir() -> Path:
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env:
        return Path(env)
    # fallback: walk up from this script (.claude/hooks/ → repo root)
    return Path(__file__).resolve().parents[2]


def session_key(payload: dict) -> str:
    key = payload.get("session_id") or os.environ.get("CLAUDE_PICKUP_TEST_KEY") or str(os.getppid())
    return re.sub(r"[^A-Za-z0-9_-]", "_", str(key))[:64]


def state_path(kind: str, key: str) -> Path:
    return Path(f"/tmp/claude-pickup-{kind}-{key}")


def read_payload() -> dict:
    try:
        return json.loads(sys.stdin.read() or "{}")
    except (json.JSONDecodeError, OSError):
        return {}


def bump_prompt_counter(key: str) -> int:
    """Increment and return the 1-based prompt count for this session."""
    p = state_path("prompts", key)
    try:
        count = int(p.read_text().splitlines()[0]) if p.exists() else 0
    except (OSError, ValueError, IndexError):
        count = 0
    count += 1
    try:
        p.write_text(f"{count}\n")
    except OSError:
        pass
    return count


def stash_prompt(key: str, prompt: str) -> None:
    try:
        with state_path("stash", key).open("a", encoding="utf-8") as f:
            f.write(prompt.replace("\n", " ").strip()[:QUERY_MAX_CHARS] + "\n")
    except OSError:
        pass


def read_stash(key: str) -> str:
    try:
        return state_path("stash", key).read_text(encoding="utf-8").replace("\n", " ").strip()
    except OSError:
        return ""


def prompt_count(key: str) -> int:
    try:
        return int(state_path("prompts", key).read_text().splitlines()[0])
    except (OSError, ValueError, IndexError):
        return 0


def _epr_bin() -> str | None:
    """Shared binary resolution — one owner (`_observation.resolve_bin`), imported, never copied."""
    try:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        from _observation import resolve_bin  # noqa: PLC0415
        return resolve_bin()
    except Exception:  # noqa: BLE001 — resolution is best-effort; absence is not an error
        return None


class Deadline:
    """One budget for the whole hook, so three bounded calls cannot sum past the harness timeout."""

    def __init__(self, seconds: float) -> None:
        self.end = time.monotonic() + seconds

    def cap(self, seconds: float) -> float:
        return max(0.0, min(seconds, self.end - time.monotonic()))


def read_json(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


def declared(root: Path) -> tuple[int | None, float | None]:
    """(`limits.fold_files_per_run`, the semantic measure's foldLag limit) — both read from their
    one home (the recall contract, and the measure it names), never restated here."""
    contract = read_json(root / CONTRACT_REL)
    per_run = (contract.get("limits") or {}).get("fold_files_per_run")
    per_run = per_run if isinstance(per_run, int) and per_run > 0 else None
    rel = (((contract.get("ceremony") or {}).get("providers") or {}).get("semantic") or {}).get(
        "measure"
    )
    limit = None
    if isinstance(rel, str) and rel:
        raw = ((read_json(root / rel).get("foldLag") or {}).get("limit"))
        limit = float(raw) if isinstance(raw, (int, float)) else None
    return per_run, limit


def run_epr(binary: str, root: Path, args: list[str], timeout: float) -> dict | None:
    """One bounded `epr` call whose stdout is a JSON object, or None on any failure."""
    if timeout <= 0:
        return None
    try:
        done = subprocess.run(
            [binary, *args, "--root", str(root)],
            capture_output=True, text=True, timeout=timeout, cwd=str(root),
            stdin=subprocess.DEVNULL,
        )
        if done.returncode != 0:
            return None
        value = json.loads(done.stdout)
        return value if isinstance(value, dict) else None
    except (OSError, ValueError, subprocess.SubprocessError):
        return None


def as_lag(value) -> int | None:
    return value if isinstance(value, int) and not isinstance(value, bool) and value >= 0 else None


def status_lag(binary: str, root: Path, deadline: Deadline) -> int | None:
    """The FALLBACK lag reading, from `index status --json` — taken only when the search itself
    gave none (it failed, or returned no candidate carrying `fold_lag`). None when there is no
    fold or no reading."""
    view = run_epr(binary, root, ["flow", "memory", "index", "status", "--json"],
                   deadline.cap(STATUS_TIMEOUT_S))
    return as_lag((view or {}).get("lag"))


def trim_spawn_log(log: Path) -> None:
    """Retention: past SPAWN_LOG_MAX bytes, keep only the last SPAWN_LOG_KEEP (from a line start)."""
    try:
        if log.stat().st_size <= SPAWN_LOG_MAX:
            return
        with log.open("rb") as fh:
            fh.seek(-SPAWN_LOG_KEEP, os.SEEK_END)
            tail = fh.read()
        newline = tail.find(b"\n")
        if newline != -1:
            tail = tail[newline + 1:]
        partial = log.with_name(log.name + ".trim")
        partial.write_bytes(tail)
        os.replace(partial, log)
    except OSError:
        pass


def spawn_fold(binary: str, root: Path, per_run: int) -> None:
    """Exactly one DETACHED fold: its own session, no stdin, output appended to the spawn log.
    Never waited on — the fold's flock makes a concurrent spawn a harmless `busy`."""
    log = root / SPAWN_LOG_REL
    try:
        log.parent.mkdir(parents=True, exist_ok=True)
        trim_spawn_log(log)
        with log.open("ab") as out:
            out.write(f"--- {time.strftime('%Y-%m-%dT%H:%M:%S%z')} surfacing spawns index fold "
                      f"--max-files {per_run}\n".encode())
            out.flush()
            subprocess.Popen(
                [binary, "flow", "memory", "index", "fold", "--max-files", str(per_run),
                 "--root", str(root)],
                cwd=str(root), stdin=subprocess.DEVNULL, stdout=out, stderr=subprocess.STDOUT,
                start_new_session=True, close_fds=True,
            )
    except (OSError, ValueError, subprocess.SubprocessError):
        pass


def short_cid(cid: str) -> str:
    """`bafyreig…rn6e` — the spelling `epr` prints for a short CID."""
    return f"{cid[:8]}…{cid[-4:]}" if len(cid) > 14 else cid


def semantic_search(binary: str, root: Path, key: str, query: str,
                    deadline: Deadline) -> list[dict]:
    """Open this Claude session's own governed recall session, then ask the semantic provider.
    Returns candidates `{where, source, cosine, producer, method, fold_lag}`, best first — the
    producer and method travel with every candidate, because the honesty floor prints them."""
    session = f"surfacing-{key}"
    opened = run_epr(binary, root, ["flow", "memory", "recall", "open", "--session", session,
                                    "--need", query, "--json"], deadline.cap(OPEN_TIMEOUT_S))
    if opened is None:
        return []
    answer = run_epr(binary, root, ["flow", "memory", "recall", "search", "--provider", "semantic",
                                    "--query", query, "--search-scope", ".", "--session", session,
                                    "--json"], deadline.cap(SEARCH_TIMEOUT_S))
    candidates = ((answer or {}).get("retrieval") or {}).get("candidates") or []
    hits: list[dict] = []
    for candidate in candidates:
        if not isinstance(candidate, dict):
            continue
        path = candidate.get("path")
        score = candidate.get("score")
        producer = candidate.get("producer")
        method = candidate.get("method")
        # A candidate that cannot say who produced it under which method is not surfaced.
        if not isinstance(path, str) or not isinstance(score, (int, float)):
            continue
        if not isinstance(producer, str) or not producer or not isinstance(method, str) or not method:
            continue
        section = candidate.get("best_section") or {}
        lines = section.get("lines") if isinstance(section, dict) else None
        title = section.get("title") if isinstance(section, dict) else None
        hits.append({
            "where": title if isinstance(title, str) and title else path,
            "source": f"{path}:{lines}" if isinstance(lines, str) and lines else path,
            "cosine": float(score),
            "producer": producer,
            "method": method,
            "fold_lag": as_lag(candidate.get("fold_lag")),
        })
    hits.sort(key=lambda h: -h["cosine"])
    return hits


def banner(lag: int | None, limit: float | None) -> str:
    """The fold's freshness, labelled — DEGRADED only past the declared bound."""
    if lag is None:
        return "fold lag unknown"
    if limit is not None and lag > limit:
        return f"DEGRADED: fold {lag} files behind, past the declared bound {limit:g}"
    if lag == 0:
        return "fold current"
    return f"fold {lag} files behind"


def hit_line(h: dict) -> str:
    line = (f"  [cosine {h['cosine']:.2f}] {h['where']} · {h['source']} · "
            f"{h['producer']} · method {short_cid(h['method'])}")
    if h["fold_lag"]:
        line += f" · fold {h['fold_lag']} files behind"
    return line


def inject(root: Path, key: str, query: str, via: str) -> None:
    """Search and emit the injection block; sets the once-per-session flag on any attempt.

    Emission is event-aware: on UserPromptSubmit plain stdout reaches model
    context, but on PreToolUse only hookSpecificOutput JSON does (2026-07-02
    review — the tool-path injection was print()-based and never landed).
    """
    try:
        state_path("surfaced", key).touch()
    except OSError:
        pass

    binary = _epr_bin()
    if not binary:
        return
    deadline = Deadline(HOOK_BUDGET_S)
    per_run, limit = declared(root)
    hits = semantic_search(binary, root, key, query, deadline)
    # The lag rides on the answer (every candidate carries the fold's lag at answer time); the
    # separate status call is only the fallback for a search that gave no reading.
    lags = [h["fold_lag"] for h in hits if h["fold_lag"] is not None]
    lag = max(lags) if lags else status_lag(binary, root, deadline)
    # After the search, so the fold's embedding work never competes with the answer being read.
    if lag is not None and lag > 0 and per_run is not None:
        spawn_fold(binary, root, per_run)
    if not hits or hits[0]["cosine"] < COSINE_FLOOR:
        return  # silent no-op — noise is worse than nothing (§4b.3)

    lines = [f"PICKUP SURFACING (semantic recall — {banner(lag, limit)}; via {via})", ""]
    lines += [hit_line(h) for h in hits[:TOP_K]]
    lines += ["", "Recall hints, not truth — verify each source against disk before acting on them."]
    if via == "prompt":
        print("\n".join(lines))
    else:
        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "additionalContext": "\n".join(lines),
            }
        }))


def handle_prompt(payload: dict) -> None:
    key = session_key(payload)
    prompt = (payload.get("prompt") or "").strip()
    if not prompt:
        return
    count = bump_prompt_counter(key)
    if count > MAX_PROMPT_WINDOW:
        return
    stash_prompt(key, prompt)
    if state_path("surfaced", key).exists():
        return
    if SLASH_EXCLUDED_RE.match(prompt):
        return  # /brainstorm owns its own FRONT seam
    if not (SLASH_PICKUP_RE.match(prompt) or PICKUP_RE.search(prompt)):
        return
    query = re.sub(r"^/\w+\s*", "", prompt).strip()[:QUERY_MAX_CHARS] or prompt[:QUERY_MAX_CHARS]
    inject(project_dir(), key, query, via="prompt")


def distilled_tool_query(payload: dict) -> str:
    ti = payload.get("tool_input") or {}
    for field in ("pattern", "prompt", "description", "query", "path"):
        v = ti.get(field)
        if isinstance(v, str) and v.strip():
            return v.strip()[:QUERY_MAX_CHARS]
    return ""


def handle_tool(payload: dict) -> None:
    key = session_key(payload)
    if state_path("surfaced", key).exists():
        return  # (a) an injection already happened
    first_flag = state_path("firstsearch", key)
    if first_flag.exists():
        return  # (b) not the first search-shaped call
    try:
        first_flag.touch()
    except OSError:
        pass
    stash = read_stash(key)
    if not stash or prompt_count(key) > MAX_PROMPT_WINDOW:
        return  # (c) outside the pickup window (or prompt hook never ran)
    query = " ".join(filter(None, [stash, distilled_tool_query(payload)]))[:QUERY_MAX_CHARS]
    inject(project_dir(), key, query, via="first-search net")


def main() -> int:
    event = sys.argv[sys.argv.index("--event") + 1] if "--event" in sys.argv else ""
    payload = read_payload()
    try:
        if event == "prompt":
            handle_prompt(payload)
        elif event == "tool":
            handle_tool(payload)
    except Exception:  # noqa: BLE001 — surfacing must never block the session
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
