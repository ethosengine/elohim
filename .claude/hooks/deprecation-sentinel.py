#!/usr/bin/env python3
"""deprecation-sentinel — PostToolUse(Bash) hook.

Watches Bash tool output for deprecation warnings emitted in flight
(e.g. Vitest "DEPRECATED `test.poolOptions` was removed in Vitest 4",
npm "npm warn deprecated", Rust "use of deprecated", Node
DeprecationWarning) against two stores with crisp roles:

  1. Ledger = the EXISTING-POSITIVES CHECK SURFACE (decides firing):
     .claude/data/deprecations.jsonl — one line per LIVE fingerprint
     {ts, fp, line, cmd, status, backlog?}. Status: open → triaged →
     blocked. A fingerprint PRESENT here suppresses dispatch; a
     fingerprint ABSENT fires the dev. FIXED items are DELETED from
     the ledger at close (full memory decomposition — the git commit
     is the record), so a reintroduced deprecation reads as NEW and
     correctly re-fires: regression handling for free.
  2. Canonical backlog = the CLOSE-OF-TRIAGE DECISION SURFACE:
     genesis/data/timeline/backlog/deprecation-*.md — timeline-
     CONVENTIONS-conformant entries holding the live trajectory
     ("Current decision" is the citation line for blocked items).
     Fixed-no-work-left entries are DELETED (rarely graduated to
     timeline/chronicle/ when genuinely meaningful) — everything in
     the backlog has a trajectory or a status, or it's not there.

Behavior:
  * NEW fingerprint  → append to ledger + inject a dispatch directive:
    the session launches the `deprecation-triage` agent (Opus) in the
    BACKGROUND — flag → scope → canonicalize → fix|block — and carries
    on with its current task.
  * KNOWN fingerprint (live: open/triaged/blocked) → once per session,
    inject a one-line deterministic citation of the current decision
    (backlog path + status). No agent dispatch — blocked-and-
    canonicalized items never re-fire automation; the
    deprecation-stasis sweep re-checks blockers deliberately.
  * Command itself mentions deprecation (greps, ledger edits, this
    hook's own tooling) → skip entirely (false-positive guard).

Fail-safe: any internal error exits 0 silently — the sentinel must
never break a session.
"""

import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone

MAX_SCAN_BYTES = 200_000  # bound the regex scan on huge outputs
MAX_NEW_PER_CALL = 5  # bound dispatch-directive noise from one command
MAX_CITED_PER_CALL = 3  # bound re-encounter citation noise
LINE_TRUNC = 300

# Deprecation signatures across the toolchains in this repo
# (Vitest/Vite, npm/pnpm, Node, Angular, Rust, Python, generic).
PATTERNS = re.compile(
    r"(?:"
    r"\bDEPRECATED\b"
    r"|\bDeprecationWarning\b"
    r"|npm warn deprecated"
    r"|\buse of deprecated\b"
    r"|\b(?:is|are|has been|was|were) deprecated\b"
    r"|\bdeprecated (?:and|API|option|in|since)\b"
    r"|\bwill be removed in (?:a future|the next|version|v?\d)"
    r")",
    re.IGNORECASE,
)

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def collect_strings(node, out, budget):
    """Walk the tool_response JSON collecting string values, bounded."""
    if budget[0] <= 0:
        return
    if isinstance(node, str):
        out.append(node[: budget[0]])
        budget[0] -= len(node)
    elif isinstance(node, dict):
        for v in node.values():
            collect_strings(v, out, budget)
    elif isinstance(node, list):
        for v in node:
            collect_strings(v, out, budget)


def fingerprint(line: str) -> str:
    norm = re.sub(r"\s+", " ", line).strip().lower()
    return hashlib.sha256(norm.encode()).hexdigest()[:12]


def session_cited_path(session_id: str) -> str:
    sid = re.sub(r"[^A-Za-z0-9]", "", session_id)[:12] or "nosession"
    return f"/tmp/claude-dep-cited-{sid}"


def load_session_cited(path: str) -> set:
    try:
        with open(path, encoding="utf-8") as fh:
            return set(fh.read().split())
    except OSError:
        return set()


def main() -> None:
    payload = json.load(sys.stdin)
    if payload.get("tool_name") != "Bash":
        return

    command = (payload.get("tool_input") or {}).get("command", "")
    # Guard the grep/echo false-positive class: a command that itself
    # talks about deprecation (searching the codebase for it, reading
    # this ledger, editing this hook) is not a NEW in-flight warning.
    if "deprecat" in command.lower():
        return

    texts: list = []
    collect_strings(payload.get("tool_response"), texts, [MAX_SCAN_BYTES])
    if not texts:
        return

    project = os.environ.get("CLAUDE_PROJECT_DIR", ".")
    ledger = os.path.join(project, ".claude", "data", "deprecations.jsonl")

    known = {}  # fp -> entry
    if os.path.exists(ledger):
        with open(ledger, encoding="utf-8", errors="replace") as fh:
            for raw in fh:
                try:
                    entry = json.loads(raw)
                    known[entry["fp"]] = entry
                except (json.JSONDecodeError, KeyError):
                    continue

    new_entries = []
    reencountered = []  # known, not fixed
    matched_this_call = set()
    for text in texts:
        for line in ANSI.sub("", text).splitlines():
            line = line.strip()
            if not line or not PATTERNS.search(line):
                continue
            fp = fingerprint(line)
            if fp in matched_this_call:
                continue
            matched_this_call.add(fp)
            entry = known.get(fp)
            if entry is None:
                if len(new_entries) < MAX_NEW_PER_CALL:
                    new_entries.append(
                        {
                            "ts": datetime.now(timezone.utc).isoformat(timespec="seconds"),
                            "fp": fp,
                            "line": line[:LINE_TRUNC],
                            "cmd": command[:160],
                            "status": "open",
                        }
                    )
            else:
                # Any ledger presence is a LIVE positive (fixed items are
                # deleted at close, never parked) — cite, don't re-fire.
                reencountered.append(entry)

    context_parts = []
    system_parts = []

    if new_entries:
        os.makedirs(os.path.dirname(ledger), exist_ok=True)
        with open(ledger, "a", encoding="utf-8") as fh:
            for entry in new_entries:
                fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        fps = ", ".join(e["fp"] for e in new_entries)
        lines = " | ".join(f'{e["fp"]}: "{e["line"][:110]}"' for e in new_entries)
        context_parts.append(
            f"[deprecation-sentinel] {len(new_entries)} NEW deprecation warning(s) "
            f"captured to .claude/data/deprecations.jsonl — {lines}. "
            f"DISPATCH NOW (do not derail the current task): launch the "
            f"`deprecation-triage` agent via the Agent tool with "
            f"run_in_background: true and the prompt 'Triage ledger "
            f"fingerprint(s) {fps} per your agent definition "
            f"(.claude/agents/deprecation-triage.md): scope usages, "
            f"canonicalize into genesis/data/timeline/backlog/, fix if "
            f"bounded, else document the blocker and mark blocked.' "
            f"If the Agent tool lacks the deprecation-triage type this "
            f"session, use general-purpose with the same prompt. Then "
            f"continue your current task."
        )
        system_parts.append(f"+{len(new_entries)} new → deprecation-triage dispatch")

    if reencountered:
        # Deterministic backlog citation: once per session per fingerprint.
        cited_path = session_cited_path(str(payload.get("session_id", "")))
        cited = load_session_cited(cited_path)
        fresh = [e for e in reencountered if e["fp"] not in cited][:MAX_CITED_PER_CALL]
        if fresh:
            try:
                with open(cited_path, "a", encoding="utf-8") as fh:
                    for e in fresh:
                        fh.write(e["fp"] + "\n")
            except OSError:
                pass
            cites = "; ".join(
                f'{e["fp"]} status={e.get("status", "open")}'
                + (f' decision={e["backlog"]}' if e.get("backlog") else " (untriaged)")
                for e in fresh
            )
            context_parts.append(
                f"[deprecation-sentinel] known deprecation(s) re-encountered — "
                f"current decision(s): {cites}. No action needed; the "
                f"deprecation-stasis sweep owns re-checks."
            )

    if not context_parts:
        return

    print(
        json.dumps(
            {
                "systemMessage": "deprecation-sentinel: " + "; ".join(system_parts or ["known re-encounter cited"]),
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": " ".join(context_parts),
                },
            }
        )
    )


if __name__ == "__main__":
    try:
        main()
    except Exception:  # noqa: BLE001 — sentinel must never break a session
        pass
    sys.exit(0)
