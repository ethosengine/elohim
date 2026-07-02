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

Engine is the MemPalace CLI, NOT the MCP (absent in main session — §4b.2): staleness check via
mempalace-currency.py --status --json (stdlib, ms), then `mempalace search` (~3 s, once per
session at most). Injection contract (§4b.3): cosine floor 0.35 → silent no-op; DEGRADED banner
always surfaced when the index is behind the front-link; top-4 hits; recall-hints-not-truth
footer. Always exit 0 — surfacing must never block a prompt or a tool call.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

COSINE_FLOOR = 0.35
TOP_K = 4
MAX_PROMPT_WINDOW = 3          # prompts 1..3 are pickup-eligible
SNIPPET_LINES = 2
SNIPPET_WIDTH = 160
QUERY_MAX_CHARS = 300
SEARCH_TIMEOUT_S = 10

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


def currency_banner(root: Path) -> str:
    script = root / ".claude" / "scripts" / "memory-kit" / "mempalace-currency.py"
    try:
        out = subprocess.run(
            [sys.executable, str(script), "--status", "--json"],
            capture_output=True, text=True, timeout=5,
        ).stdout.strip()
        st = json.loads(out)
        if st.get("stale"):
            return (f"DEGRADED: index {st.get('changed_since_mine', '?')} file(s) behind "
                    f"front-link, last mine {st.get('last_mine', '?')}")
        return "fresh"
    except Exception:  # noqa: BLE001 — currency is advisory; absence of it must not block
        return "currency unknown"


def palace_search(root: Path, query: str) -> list[dict]:
    """Run `mempalace search` and parse entries: wing/room, source, cosine, snippet."""
    try:
        out = subprocess.run(
            ["mempalace", "--palace", str(root / ".mempalace" / "palace"),
             "search", "--results", str(TOP_K), query],
            capture_output=True, text=True, timeout=SEARCH_TIMEOUT_S,
        ).stdout
    except (OSError, subprocess.TimeoutExpired):
        return []

    hits: list[dict] = []
    current: dict | None = None
    in_snippet = False
    entry_re = re.compile(r"^\s*\[(\d+)\]\s+(.+)$")
    for line in out.splitlines():
        m = entry_re.match(line)
        if m:
            current = {"where": m.group(2).strip(), "source": "", "cosine": 0.0, "snippet": []}
            hits.append(current)
            in_snippet = False
            continue
        if current is None:
            continue
        stripped = line.strip()
        if stripped.startswith("Source:"):
            current["source"] = stripped.split("Source:", 1)[1].strip()
        elif stripped.startswith("Match:"):
            cm = re.search(r"cosine=([\d.]+)", stripped)
            if cm:
                current["cosine"] = float(cm.group(1))
            in_snippet = True
        elif in_snippet and stripped.startswith("─"):
            in_snippet = False
        elif in_snippet and stripped and len(current["snippet"]) < SNIPPET_LINES:
            current["snippet"].append(stripped[:SNIPPET_WIDTH])
    seen: set[tuple[str, str]] = set()
    deduped = []
    for h in hits:
        ident = (h["where"], h["source"])
        if h["source"] and ident not in seen:
            seen.add(ident)
            deduped.append(h)
    return deduped


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

    hits = palace_search(root, query)
    if not hits or hits[0]["cosine"] < COSINE_FLOOR:
        return  # silent no-op — noise is worse than nothing (§4b.3)

    banner = currency_banner(root)
    lines = [f"PICKUP SURFACING (semantic recall — {banner}; via {via})", ""]
    for h in hits[:TOP_K]:
        lines.append(f"  [cosine {h['cosine']:.2f}] {h['where']} · {h['source']}")
        for s in h["snippet"]:
            lines.append(f"      {s}")
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
